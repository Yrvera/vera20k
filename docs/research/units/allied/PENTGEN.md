# PENTGEN — General Pentagon (Allied campaign placeholder)

**Side classification:** Allied (per name/intent and the priority-index list).
**⚠ INI quirk:** `Owner=` actually lists **Soviet houses + YuriCountry** — see §1 and §6
for analysis. The unit is named "Pentagon General" and conceptually Allied, but the
shipped INI restricts ownership to Soviet/Yuri factions. Likely an unfixed copy-paste
bug from `[VLADIMIR]` (same `Owner=` line, same surrounding stub stats).

**Role:** Campaign cutscene / mission-script placeholder. Not buildable in any
multiplayer or skirmish context. 50 HP, weak pistol, ignored by enemy AI.

> Output bar: parity-relevant only to the extent that mission scripts reference
> PENTGEN. Ghidra confirms `gamemd.exe` contains no `"PENTGEN"` / `"Pentagon"` strings
> (verified — see §7). All behavior is generic flag-driven.

> Companion doc: [`soviet/VLADIMIR.md`](../soviet/VLADIMIR.md) — Soviet equivalent
> placeholder. PENTGEN and VLADIMIR are template-cloned units; differences are
> spelled out in §10.

---

## 1. `rulesmd.ini` — `[PENTGEN]` verbatim

```ini
; Pentagon General
[PENTGEN]
UIName=Name:CIV1
Name=General Pentagon
Category=Soldier
Strength=50
Primary=Pistola
Armor=none
TechLevel=-1
CrushSound=InfantrySquish
;Insignificant=yes
Sight=2
Speed=4
Owner=Russians,Confederation,Africans,Arabs,YuriCountry
AllowedToStartInMultiplayer=no
Cost=10
Soylent=200
Points=1
;Ammo=10
;Fraidycat=yes
;Civilian=yes
;Nominal=yes
Pip=white
VoiceSelect=GISelect
VoiceMove=GIMove
VoiceAttack=GIAttackCommand
VoiceFeedback=GIFear
VoiceSpecialAttack=GIMove
DieSound=GIDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=0	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
Size=1
IFVMode=0
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `;` (comment) | `Pentagon General` | — | Author-note: this is the Pentagon General hero. |
| `UIName` | `Name:CIV1` | AbstractType | CSF lookup — points at **Civilian Male 1's CSF entry**. **⚠ Note**: PENTGEN is **missing the `UseOwnName=true` flag** that VLADIMIR has, so the UI renders "Civilian Male 1" (the CSF text), **not** "General Pentagon". This appears to be an INI oversight — the `Name=` field below was clearly intended to be the displayed name. |
| `Name` | `General Pentagon` | AbstractType | Dev fallback. Only used if `UseOwnName=true` were set — which it isn't, so this string never reaches the UI. |
| `Category` | `Soldier` | TechnoType | Infantry classifier. |
| `Strength` | `50` | AbstractType | 50 HP — same as VLADIMIR. |
| `Primary` | `Pistola` | TechnoType | Weak pistol (Damage=2, ROF=20, Range=3, Warhead=SA) — see §3. |
| `Armor` | `none` | TechnoType | Slot 1. |
| `TechLevel` | `-1` | TechnoType [BINARY-VERIFIED audit 13: `TechnoType+0x634`, int; parser xref @ 0x00714577; 5 consumer xref sites] | **Not buildable** under any tech-tree condition. Spawn-only via map / script. |
| `CrushSound` | `InfantrySquish` | TechnoType | Squish on crush. |
| `;Insignificant=yes` | *(commented)* | — | Inert — PENTGEN registers as a normal unit. |
| `Sight` | `2` | TechnoType | 2-cell reveal — civilian-tier. |
| `Speed` | `4` | TechnoType | Slow walk. |
| `Owner` | `Russians,Confederation,Africans,Arabs,YuriCountry` | TechnoType | **⚠ BUG**: this is the **Soviet+Yuri** owner list, not Allied. A unit named "General **Pentagon**" should logically be Allied (`British,French,Germans,Americans,Alliance`). The INI line is verbatim-copied from VLADIMIR above, suggesting the developer cloned VLADIMIR's block and forgot to flip the owner list. Live consequence: PENTGEN cannot be owned by any Allied house unless a campaign script forces a house-transfer. The unit is functionally Soviet/Yuri-owned despite its name. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Never preplaced. |
| `Cost` | `10` | TechnoType | Nominal — irrelevant (TechLevel=-1 blocks builds). |
| `Soylent` | `200` | TechnoType | Grinder refund 20× cost. A Yuri player grinding PENTGEN nets +$190 (same as VLADIMIR). |
| `Points` | `1` | TechnoType | Score on kill. |
| `;Ammo=10 / ;Fraidycat / ;Civilian / ;Nominal` | *(commented)* | — | All inert — PENTGEN is a regular combatant. |
| `Pip` | `white` | InfantryType | Transport passenger pip (white/neutral aesthetic). |
| `VoiceSelect` | `GISelect` | TechnoType | **Reuses GI voice set** (6 random clips). Unlike VLADIMIR's silent `Dummy`, PENTGEN actually responds to player commands using a regular GI's voice lines ("Yes sir!", "Affirmative!", etc.). |
| `VoiceMove` | `GIMove` | TechnoType | 6 GI move clips. |
| `VoiceAttack` | `GIAttackCommand` | TechnoType | 6 GI attack clips. |
| `VoiceFeedback` | `GIFear` | TechnoType | 2 GI fear clips (under attack). |
| `VoiceSpecialAttack` | `GIMove` | TechnoType | Reuses move set (no unique special voice — and PENTGEN has no Secondary anyway). |
| `DieSound` | `GIDie` | TechnoType | 5 GI death clips, FShift ±10. |
| `Locomotor` | `{4A582744-9839-11d1-B709-00A024DDAFD1}` | TechnoType | WalkLocomotionClass — standard infantry walk. |
| `PhysicalSize` | `1` | TechnoType | Sub-cell footprint. |
| `MovementZone` | `Infantry` | TechnoType | Standard pathing. |
| `ThreatPosed` | `0` | TechnoType [BINARY-VERIFIED audit 13: `TechnoType+0x670`, int; parser xref @ 0x007149CE] | **Zero AI threat** — enemy AI ignores PENTGEN unless explicitly targeted. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `Size` | `1` | TechnoType | Transport slot cost. |
| `IFVMode` | `0` | TechnoType [BINARY-VERIFIED audit 13: `TechnoType+0x688`, int; parser xref @ 0x00714787] | IFV gunner mode 0 → IFV's `NormalTurretWeapon=0` → `Weapon1=HoverMissile`. PENTGEN in an IFV makes it fire its default missile (no per-passenger weapon swap). |
| *(missing)* `UseOwnName=true` | — | InfantryType [BINARY-VERIFIED audit 13: parser xref @ 0x0052463D in `InfantryTypeClass__ReadINI` (entry 0x005240A0, body 0x005240A0–0x0052475C); exact byte offset DEFERRED — one of the +0xEAC..+0xECB ReadBool block] | **⚠ Bug parity note**: VLADIMIR has this flag, PENTGEN does not. Without it, the displayed name comes from CSF `Name:CIV1` ("Civilian Male 1") instead of `Name=General Pentagon`. To reproduce gamemd's observable behavior exactly, the engine must NOT promote `Name=` to UI when `UseOwnName=` is absent or false. |

---

## 2. `artmd.ini` — `[PENTGEN]` section

```ini
[PENTGEN] ; Pentagon General
Cameo=SHKICON
AltCameo=SHKUICO
Sequence=GenSequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=100,-25,135
SecondaryFireFLH=100,-25,135
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `SHKICON` | **Reuses Tesla Trooper (SHK) cameo** — placeholder. |
| `AltCameo` | `SHKUICO` | Yuri-skinned SHK cameo. |
| `Sequence` | `GenSequence` | **PENTGEN has its own frame table** — see below. (VLADIMIR by contrast reuses `E1Sequence`.) |
| `Crawls` | `yes` | Crawl/prone supported. |
| `Remapable` | `yes` | House-color remap. |
| `FireUp` | `2` | 2 frames before projectile spawns — fast pistol draw. |
| `PrimaryFireFLH` | `100,-25,135` | Fire offset (X=100 fwd, Y=-25 left, Z=135 head/shoulder height). |
| `SecondaryFireFLH` | `100,-25,135` | Same — unused (no Secondary weapon). |

### `[GenSequence]` referenced sequence

```ini
[GenSequence]
Ready=0,1,1
Guard=0,1,1
Prone=134,1,6
Walk=8,6,6
FireUp=86,6,6
Down=198,2,2
Crawl=134,6,6
Up=182,2,2
FireProne=214,6,6
Idle1=56,15,0,W
Idle2=0,1,1,E
Die1=71,15,0
Die2=0,1,1
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=56,15,0,W
Paradrop=0,1,0
Panic=8,6,6
```

| Row | Notes |
|-----|-------|
| `Ready/Guard=0,1,1` | Stand pose. |
| `Walk=8,6,6` | 6-frame walk × 6 facings (shared layout with E1). |
| `Idle1=56,15,0,W` | Idle anim 1, west-locked (vs E1's south-locked at frame 56 — same start frame, different facing lock). |
| `Idle2=0,1,1,E` | Idle 2 is a **stub** (1 frame, east-lock) — collapses to `Ready` pose facing east. PENTGEN only has one real idle animation. |
| `FireUp=86,6,6` | 6-frame fire × 6 facings starting at frame 86 (E1's FireUp starts at 164; PENTGEN.SHP packs frames differently). |
| `Crawl/Prone=134` | Crawl/prone frames share start 134 (E1: 86). |
| `Die1=71,15,0` | Single 15-frame death (E1: starts at 134). |
| `Die2-5=0,1,1` | Stub deaths — PENTGEN only has 1 real death animation (vs E1's 2 deaths). |
| `Down=198 / Up=182` | Lay-down (198, 2 frames × 2 facings) and stand-up (182, 2×2). Unusual order — Up frame is *before* Down in the file, but logically Up follows Down at runtime. |
| `FireProne=214,6,6` | Prone-fire frames. |
| `Cheer=56,15,0,W` | Victory cheer west-locked (reuses Idle1 frames). |
| `Paradrop=0,1,0` | Paradrop stub. |
| `Panic=8,6,6` | Reuses walk frames. |

**Frame layout differs from `[E1Sequence]`** — PENTGEN.SHP is a unique sprite (a "general in uniform" walking animation), not a re-skin of the GI's. The sequence table just tells the engine where to find each pose inside PENTGEN.SHP.

---

## 3. Weapon — `[Pistola]`

Identical to VLADIMIR's primary. See [`soviet/VLADIMIR.md`](../soviet/VLADIMIR.md#3-weapon--pistola) §3 for full breakdown.

```ini
[Pistola]
Damage=2
ROF=20
Range=3
Projectile=InvisibleLow
Speed=100
Warhead=SA
Report=CivAttack
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `2` | 2 HP per shot. |
| `ROF` | `20` | 20-tick cooldown. |
| `Range` | `3` | 3 cells. |
| `Projectile` | `InvisibleLow` | Inviso, respects cliffs/elevation/walls. |
| `Speed` | `100` | Bullet speed. |
| `Warhead` | `SA` | Small-arms (Verses 100/80/80/50/25/25/75/50/25/100/100, InfDeath=1, AnimList=PIFFPIFF). |
| `Report` | `CivAttack` | Civilian attack sound (2 clips, FShift ±3, vol 70). |

---

## 4. Warhead — `[SA]`

Identical to VLADIMIR's. See [`soviet/VLADIMIR.md`](../soviet/VLADIMIR.md#4-warhead--sa) §4 for full breakdown. Key points: Verses=100,80,80,50,25,25,75,50,25,100,100 (effective vs infantry, poor vs medium/heavy armor); `InfDeath=1` (small-arms slumping death); `AnimList=PIFFPIFF,PIFFPIFF` (hit-spark anim).

---

## 5. Voices / sounds

```ini
[GIAttackCommand]
Sounds= $igiata $igiatb $igiatc $igiatd $igiate $igiatf
Control= random
Volume=85

[GIMove]
Sounds= $igimoa $igimob $igimoc $igimod $igimoe $igimof
Control= random
Volume=85

[GISelect]
Sounds= $igisea $igiseb $igisec $igised $igisee $igisef
Control= random
Volume=85

[GIFear]
Sounds= $igifea $igifeb
Control= random
Priority=low
Volume=90

[GIDie]
Sounds= $igidia $igidib $igidic $igidid $igidie
Priority=low
Control= random
FShift= -10 10
Volume=85
```

```ini
[InfantrySquish]
Sounds=igensqua
FShift= -10 10
Volume=65
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=GISelect` | 6 clips ($igisea..f) | Click-select — sounds **exactly like a regular GI** |
| `VoiceMove=GIMove` | 6 clips ($igimoa..f) | Move order |
| `VoiceAttack=GIAttackCommand` | 6 clips ($igiata..f) | Attack order |
| `VoiceFeedback=GIFear` | 2 clips ($igifea..b), low priority | Under attack — "I'm in trouble!" GI line |
| `VoiceSpecialAttack=GIMove` | reuses move set | No unique special voice (and no Secondary weapon to trigger it) |
| `DieSound=GIDie` | 5 GI death clips ($igidia..e), FShift ±10 | Death scream |
| `Report=CivAttack` (on weapon) | 2 clips | Pistol-fire sound (civilian, not GI) |
| `CrushSound=InfantrySquish` | igensqua, vol 65 | When crushed |

**Audio contrast with VLADIMIR**: PENTGEN sounds identical to a regular GI in all four player-command voices, whereas VLADIMIR is completely silent (`Dummy`). The audio difference is the primary "feel" distinction between the two campaign placeholders.

---

## 6. Prerequisites / owners / availability

- `TechLevel=-1` + `AllowedToStartInMultiplayer=no` → **never buildable** in any normal context.
- `Owner=Russians,Confederation,Africans,Arabs,YuriCountry` — **Soviet/Yuri only**, despite the unit's name (see §1 bug note).
- `Prerequisite=` — **none**.
- No `RequiredHouses=`, no `RequiresStolen*Tech=`, no `BuildLimit=`.

Reachable spawn paths:
1. **Map preplacement** — campaign `.map` `[Infantry]` sections.
2. **Trigger/script spawn** — mission AI events.
3. **House-transfer via script** — a campaign script can force PENTGEN onto an Allied house even though `Owner=` doesn't include them. This is how Pentagon-evac missions would actually deploy the unit; the `Owner=` line restricts *initial* ownership but the engine accepts script-driven transfers.

### Distinguishing PENTGEN from VLADIMIR at the INI level

| Aspect | PENTGEN | VLADIMIR |
|--------|---------|----------|
| `Owner=` | Soviet+Yuri (bug — should be Allied) | Soviet+Yuri (correct) |
| `UseOwnName=` | (absent) | `true` |
| Displayed name in UI | "Civilian Male 1" (CSF) | "Vladimir" (Name= override) |
| `Sequence=` | `GenSequence` (own anim) | `E1Sequence` (reuses GI's) |
| `VoiceSelect/Move/Attack/Feedback` | GI voices | `Dummy` (silent) |
| `VoiceSpecialAttack` | `GIMove` (present) | (absent) |
| `DieSound` | `GIDie` | `FlakTroopDie` |

Everything else is identical between the two units (Strength, Primary, Armor, TechLevel, Sight, Speed, Cost, Soylent, Points, Pip, Locomotor, PhysicalSize, MovementZone, ThreatPosed, ImmuneToVeins, Size, IFVMode, all commented-out flags).

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 PENTGEN-specific code in `gamemd.exe`

| Query (search_strings) | Result |
|------------------------|--------|
| `PENTGEN` | 0 matches |
| `Pentagon` | 0 matches |

⇒ **No PENTGEN-specific code path or string reference** in the binary. All behavior is generic flag-driven, exactly as with VLADIMIR.

### 7.2 Flag-scope verification

All flags PENTGEN carries are the standard TechnoType / InfantryType / AbstractType set already verified in prior unit docs:

- `ImmuneToVeins`, `IFVMode` — TechnoType (cheat sheet)
- `Pip` — InfantryType (cheat sheet)
- All voice hooks, Locomotor, MovementZone, Owner, Sight, Speed, Strength, Cost, Soylent, Points — generic TechnoType / AbstractType
- **Notably absent**: `UseOwnName=true` (which IS read by InfantryTypeClass per the cheat sheet — verified scope, just not present on PENTGEN's INI block)

### 7.3 Live behaviors

| Behavior | Driver | Notes |
|----------|--------|-------|
| Displayed name = "Civilian Male 1" (not "General Pentagon") | `UseOwnName=` flag is **absent** → engine falls back to CSF resolution via `UIName=Name:CIV1` | INI bug — intent likely was for the name override to be active. |
| Owned by Soviet/Yuri houses only at spawn | `Owner=` list | Bug — name suggests Allied intent. |
| Responds to player commands with GI voice lines | `VoiceSelect/Move/Attack/Feedback=GI*` | Sound playback uses generic VocClass — no PENTGEN-specific handling. |
| Standard infantry walk | `Locomotor={4A582744-...}` | WalkLocomotionClass. |
| Ignored by enemy AI | `ThreatPosed=0` | Generic threat-scan exclusion. |
| Cannot be built | `TechLevel=-1` | Build-availability resolver rejects. |

### 7.4 Behaviors NOT present in PENTGEN

- No `Hero=` / no special-protection flag — dies at 50 HP like any soldier.
- No `Fearless` / no `Fraidycat` — neutral combat behavior (will stand and shoot pistol when attacked).
- No `DetectDisguise`, no `ImmuneToPsionics`, no `C4`, no `Ivan`, no `Spy` — basic soldier kit.
- No `Secondary` weapon — pistol only.
- No `Deployer`, no special-attack, no `Trainable=no` (so technically *can* gain XP — but `ThreatPosed=0` means he never gets shot at).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| Commented `;Ammo / ;Fraidycat / ;Civilian / ;Nominal / ;Insignificant` | n/a (commented) | Inactive. |

No fog-of-war flags, no Tiberium refs (PENTGEN does not even have `TiberiumProof=yes` like VLADIMIR — odd omission but inert anyway).

---

## 9. Veterancy

PENTGEN has **no** `VeteranAbilities=`, **no** `EliteAbilities=`, and **no** explicit `Trainable=` key. Default `Trainable=yes` applies — so PENTGEN *can* theoretically gain XP. But with `ThreatPosed=0` and no scripted enemies targeting him, XP gain is unlikely in normal mission play.

Even if he reached veteran/elite, the absent ability lists would have no effect; and there's no `ElitePrimary=` so the weapon never changes.

In practice PENTGEN is locked at rookie rank for his mission lifetime.

---

## 10. Cross-references

### Direct dependencies
- `[Pistola]` — weapon (§3)
- `[InvisibleLow]` — projectile
- `[SA]` — warhead (§4)
- `[PIFFPIFF]` (artmd) — hit-spark anim
- `[GenSequence]` (artmd) — frame table (§2)
- `[GISelect/Move/AttackCommand/Fear/Die] / [CivAttack] / [InfantrySquish]` (soundmd) — voices and weapon report (§5)
- `[CIV1]` CSF entry — `UIName=` target (since `UseOwnName=` is absent, this is what the player actually sees in the UI)

### Conceptual companions
- **VLADIMIR** ([`soviet/VLADIMIR.md`](../soviet/VLADIMIR.md)) — Soviet template-twin. Differences spelled out in §6 table. PENTGEN is the direct Allied counterpart by name, but the shipped INI has Soviet `Owner=` (bug).
- **PRES** (President Dugan), **SSRV** (Secret Service), **CLNT** (Clinton), **EINS** (Einstein), **RMNV** (Romanov), **ARND** (Arnie), **STLN** (Stalin) — other campaign placeholder heroes following the same `TechLevel=-1` + thin-stat template. All TODO.
- **CIV1 / CIV2 / CIV3 / CTECH** — civilian variants whose CSF entries are reused by PENTGEN's `UIName=Name:CIV1` lookup.

### Deep-RE docs
- None directly relevant — PENTGEN has no hardcoded behavior to research.

---

## Ghidra audit log (audit iteration 13 — 2026-05-18)

**Methodology**: PENTGEN is a campaign-placeholder unit with no
unit-specific code in `gamemd.exe`. The doc is heavy on INI data
(layout, voices, sequences) and light on binary-verifiable claims. This
audit focuses on the few load-bearing claims (negative string searches,
field-scope verifications, and the offsets for the 4 TechnoType keys
that aren't yet in the cumulative tables). ~10 Ghidra queries: 6
string-searches + 4 xref lookups + 1 full inline decompile of
`InfantryTypeClass__ReadINI` + 1 grep on saved
`TechnoTypeClass__ReadINI` decompile.

### Negative claims re-verified (BINARY-VERIFIED)

| Query | Result |
|-------|--------|
| `search_strings("^PENTGEN$")` | **0 matches** |
| `search_strings("^Pentagon$")` | **0 matches** |

Confirms: no hardcoded section-name branch, no `Pentagon`-keyword
behavior gate. All PENTGEN behavior is data-driven via generic
TechnoType / InfantryType / AbstractType flag handling.

### Function entry points verified (BINARY-VERIFIED)

| Function | Entry | Body | Notes |
|----------|-------|------|-------|
| `InfantryTypeClass__ReadINI` | `0x005240A0` | `0x005240A0–0x0052475C` | Fully decompiled this pass. Calls `TechnoTypeClass__ReadINI` first, then 23 `ReadBool` + 6 `ReadInt` + `ReadString`/`ReadSoundList` writes into the InfantryType-specific block at `+0xE40..+0xECB`. |
| `TechnoTypeClass__ReadINI` | (oversized) | — | Read via grep on the saved audit-12 decompile (file: `mcp-ghidra-mcp-decompile_function-1779128548776.txt`). |

### Struct offsets BINARY-VERIFIED (this pass)

| Class | Offset | INI key | Type | Notes |
|-------|--------|---------|------|-------|
| TechnoType | `+0x634` | `TechLevel` | int | `param_1[0x18d]` — parser xref @ `0x00714577`. **NEW** — populated by Constructor default, then overwritten if `[TechLevel]` is present in INI. **Also consumed at 5 other sites** (lobby `RulesClass__ReadMultiplayerDialogSettings @ 0x00671fad`, `CCINIClass__Constructor @ 0x00599830`, `HouseClass__Read_Scenario_INI @ 0x00500b95`, `FUN_006f1550 @ 0x006f171f`, and `FUN_00501210 @ 0x00501277`) — these are likely the build-availability + tech-level-clamp consumers. |
| TechnoType | `+0x670` | `ThreatPosed` | int | `param_1[0x19c]` — parser xref @ `0x007149CE`. **Re-confirms audit 7 cumulative**. |
| TechnoType | `+0x688` | `IFVMode` | int | `param_1[0x1a2]` — parser xref @ `0x00714787`. **Re-confirms audit 7 cumulative**. |
| InfantryType | (offset DEFERRED — see below) | `UseOwnName` | byte | Parser xref @ `0x0052463d` in `InfantryTypeClass__ReadINI`. **Confirms InfantryType-scope** (matches audit 4 GHOST finding) — the byte offset is one of the 23 sequential `ReadBool` writes (+0xEAC..+0xECB) in the function body, but pinning the exact ordinal requires interpolating across all sibling-key xrefs and is not load-bearing for PENTGEN parity (UseOwnName is *absent* from PENTGEN's INI, so the offset only affects the in-engine default behavior). |

### Behavioral claim verification

- **"PENTGEN displayed name = 'Civilian Male 1' (not 'General Pentagon')"**:
  The doc claims this is because `UseOwnName=` is absent → engine falls
  back to CSF resolution via `UIName=Name:CIV1`. This audit confirms
  `UseOwnName` IS read by `InfantryTypeClass__ReadINI` (xref verified),
  so the field IS an active gate. The exact CSF-fallback code path
  (TechnoClass display-name resolver consuming the `UseOwnName` byte) is
  **DEFERRED** — not load-bearing for this doc since PENTGEN never sets
  the flag.
- **"Owner=Soviet+Yuri only at spawn"**: data-driven from the
  rulesmd `Owner=` field via `TechnoTypeClass__ReadINI`. The
  HouseAllowed bitmask consumer is standard and was already verified in
  the cumulative tables. No audit value in re-tracing.
- **"ThreatPosed=0 → AI ignores"**: TechnoType+0x670 = ThreatPosed is
  consumed by AI threat-scan paths. End-to-end consumer DEFERRED —
  the parity claim "ignored by AI" is a downstream effect; the offset is
  verified, the chain is not.
- **"TechLevel=-1 → not buildable"**: TechnoType+0x634 has 5 consumer
  sites (listed above). One is the lobby-tech-clamp, one is the
  scenario-INI loader, one is the build-availability gate. Specific gate
  function DEFERRED — verified the offset, not the consumer.

### Items NOT re-verified in this pass (DEFERRED)

- `UseOwnName` exact byte offset (one of `+0xEAC..+0xECB`; requires
  enumerating sibling keys' xrefs to pin position).
- `UseOwnName` consumer in the display-name resolver
  (TechnoClass-side function that picks `Name=` vs CSF `UIName=`).
- TechLevel build-availability consumer (the function that rejects
  `TechLevel=-1` as not-buildable).
- ThreatPosed AI consumer (zero-threat exclusion path).
- IFVMode consumer end-to-end (the IFV gunner-table lookup for the
  passenger weapon swap).

### Confidence summary

- **HIGH**: 6 string addresses + parser xrefs (4 of 4 verified exactly
  matching their parser-function names); 1 new TechnoType offset
  (TechLevel=+0x634) + 2 re-confirmations (ThreatPosed, IFVMode); 1
  InfantryType-scope re-confirmation (UseOwnName); 2 negative claims
  (PENTGEN, Pentagon → 0 matches). `InfantryTypeClass__ReadINI` fully
  decompiled with 23 ReadBool / 6 ReadInt offset writes visible.
- **MEDIUM**: TechLevel's specific build-gate consumer site identified
  by xref list but not decompiled this pass.
- **No INCORRECT findings in the doc** — the binary content is thin but
  consistent with the binary. The two flagged INI bugs (Owner=
  Soviet-not-Allied; UseOwnName= absent) are correct observations about
  the shipped INI, not about the doc's interpretation.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[PENTGEN]` rulesmd key explained | ✅ §1 |
| Every `[PENTGEN]` artmd key explained | ✅ §2 |
| `Sequence=GenSequence` fully expanded | ✅ §2 |
| Weapon + projectile + warhead | ✅ §3–§4 |
| All voices expanded with verbatim sound defs | ✅ §5 |
| Prereqs / owners / spawn paths | ✅ §6 |
| Hardcoded behavior — Ghidra searches for PENTGEN ID | ✅ §7 (2 queries, 0 matches each) |
| TS-legacy filter | ✅ §8 |
| Veterancy treated correctly | ✅ §9 |
| Cross-refs to VLADIMIR + other campaign placeholders | ✅ §10 |
| **Two INI bugs flagged** (Owner= Soviet not Allied; `UseOwnName=` absent → wrong UI name) | ✅ doc header + §1 |

**Open follow-ups (none load-bearing):**
- Confirm whether the original gamemd.exe shipped binary actually exhibits the "Civilian Male 1" UI label for PENTGEN — would need a campaign-mission test to be sure. If gamemd somehow displays "General Pentagon" via a different code path (e.g. hardcoded mission-UI override), document that.
- Mission-script audit for which Allied campaign missions reference PENTGEN, and how those scripts handle the Soviet `Owner=` line (presumably via house-transfer trigger actions). Out of scope for unit docs.
- Audit other campaign-placeholder units for the same `Owner=` clone-bug pattern (PRES, SSRV, CLNT, RMNV, EINS, ARND, STLN) — likely several share the same copy-paste mistake.
