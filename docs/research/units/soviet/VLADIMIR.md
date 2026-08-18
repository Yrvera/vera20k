# VLADIMIR — Vladimir (Soviet campaign placeholder)

**Side classification:** Soviet (Owner=Soviet+YuriCountry, no other gates).
**Role:** Campaign cutscene / mission-script placeholder. Not buildable in any
multiplayer or skirmish context. Ultra-thin gameplay surface (50 HP, weak pistol,
no AI threat, silent voice set).

> Output bar: this is a placeholder/stub unit. The "indistinguishable from gamemd"
> bar applies — but the surface area is minimal because the unit barely participates
> in regular gameplay. Documented in full nonetheless.

> Ghidra confirms `gamemd.exe` contains **no** `"VLADIMIR"` / `"Vladimir"` / `"Name:VLADIMIR"`
> strings (verified — see §7). All behavior is generic flag-driven.

---

## 1. `rulesmd.ini` — `[VLADIMIR]` verbatim

```ini
[VLADIMIR]
UIName=Name:CIV1
Name=Vladimir
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
VoiceSelect=Dummy
VoiceMove=Dummy
VoiceAttack=Dummy
VoiceFeedback=Dummy
DieSound=FlakTroopDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=0	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
Size=1
IFVMode=0
UseOwnName=true
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:CIV1` | AbstractType | CSF lookup — **reuses Civilian Male 1's CSF label**. With `UseOwnName=true` below, this is overridden — see that key. |
| `Name` | `Vladimir` | AbstractType | Dev fallback. With `UseOwnName=yes`, this **becomes** the displayed name in the UI (the `UIName` CSF lookup is bypassed). |
| `Category` | `Soldier` | TechnoType | Infantry classifier; AI targeting group. |
| `Strength` | `50` | AbstractType | 50 HP — half of a basic GI. One-shot for almost any infantry weapon. |
| `Primary` | `Pistola` | TechnoType | Weak pistol (Damage=2, ROF=20, Range=3, Warhead=SA, Report=CivAttack). Same weapon as civilian SHP infantry use. |
| `Armor` | `none` | TechnoType | Verses-slot 1. |
| `TechLevel` | `-1` | TechnoType | **Not buildable** under any tech-tree condition. The unit can only appear via mission scripting, map preplacement, or trigger-spawn. |
| `CrushSound` | `InfantrySquish` | TechnoType | Squish sound when crushed. |
| `;Insignificant=yes` | *(commented)* | — | Commented out — the unit IS counted as a unit (registers in player count, can be selected, etc.). |
| `Sight` | `2` | TechnoType | 2-cell reveal radius — extremely short, civilian-tier. |
| `Speed` | `4` | TechnoType | Slow infantry pace. |
| `Owner` | `Russians,Confederation,Africans,Arabs,YuriCountry` | TechnoType | Soviet houses + Yuri. Cannot be owned by Allied countries. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Belt-and-braces with `TechLevel=-1` — never appears as starting unit. |
| `Cost` | `10` | TechnoType | Nominal cost (irrelevant since TechLevel=-1 prevents building). |
| `Soylent` | `200` | TechnoType | Grinder refund **20× build cost** — the Grinder gives back $200 for a $10 unit. (This is asymmetric on purpose for placeholder/mission units, but if a Yuri player ever captures and grinds a VLADIMIR they net +$190.) |
| `Points` | `1` | TechnoType | Score on kill. |
| `;Ammo=10` | *(commented)* | — | Inert. Would have given the pistol limited ammo. |
| `;Fraidycat=yes` | *(commented)* | — | Inert. `Fraidycat=yes` on civilians causes them to flee when shot at; commented out for VLADIMIR. |
| `;Civilian=yes` | *(commented)* | — | Inert. `Civilian=yes` marks the unit as a civilian/neutral (changes targeting, scoring, AI handling). Commented — VLADIMIR is **not** flagged as civilian despite being campaign-only. |
| `;Nominal=yes` | *(commented)* | — | Inert. `Nominal=yes` hides the unit from various combat reports/scoring. |
| `Pip` | `white` | InfantryType | Passenger pip colour in transports (white = civilian/neutral aesthetic). |
| `VoiceSelect/Move/Attack/Feedback` | `Dummy` | TechnoType | All four voice hooks point to `[Dummy]` which has `Volume=0` — the unit is **completely silent** when selected, ordered, or under attack. Only `DieSound` produces audio. |
| `DieSound` | `FlakTroopDie` | TechnoType | Death sound — 5 random Flak-Trooper death clips. Notable: a Soviet hero shares Flak Trooper death lines (not a unique Vladimir death scream). |
| `Locomotor` | `{4A582744-9839-11d1-B709-00A024DDAFD1}` | TechnoType | `WalkLocomotionClass` (standard infantry walk). |
| `PhysicalSize` | `1` | TechnoType | Sub-cell footprint. |
| `MovementZone` | `Infantry` | TechnoType | Standard infantry pathing. |
| `ThreatPosed` | `0` | TechnoType | **Zero AI threat** — enemy AI completely ignores VLADIMIR as a target (will be hit only via splash/AoE or explicit targeting). |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** — dormant. |
| `Size` | `1` | TechnoType | Transport-slot cost. |
| `IFVMode` | `0` | TechnoType | When boarded into Allied IFV: `IFVMode=0` → IFV's `NormalTurretWeapon=0` → `Weapon1=HoverMissile`. VLADIMIR in an IFV fires the IFV's default missile (same as no-passenger or generic-civilian passenger). |
| `UseOwnName` | `true` | **InfantryType** (cheat sheet — InfantryTypeClass__ReadINI 0x00524xxx range) | The displayed sidebar/cursor name uses `Name=Vladimir` rather than the CSF entry for `Name:CIV1` — so the player sees "Vladimir" instead of "Civilian Male 1". This is the only INI mechanism that differentiates VLADIMIR's identity from a generic civilian male. |

---

## 2. `artmd.ini` — `[VLADIMIR]` section

```ini
[VLADIMIR] ; Vladimir
Cameo=SHKICON
AltCameo=SHKUICO
Sequence=E1Sequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=100,-25,135
SecondaryFireFLH=100,-25,135
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `SHKICON` | **Reuses Tesla Trooper (SHK) cameo** — VLADIMIR has no dedicated cameo SHP. Visible if the unit somehow appears in the sidebar (e.g. via mission-script reveal). |
| `AltCameo` | `SHKUICO` | Yuri-skinned Tesla Trooper cameo (same reuse). |
| `Sequence` | `E1Sequence` | **Reuses GI (E1) infantry frame table** — VLADIMIR has no dedicated animation. The sprite file `VLADIMIR.SHP` is the unique art, but the frame layout is identical to E1's. |
| `Crawls` | `yes` | Has crawl/prone anims (inherited from E1 sequence). |
| `Remapable` | `yes` | House-color remap. |
| `FireUp` | `2` | Frames into FireUp before projectile spawns. Note: PENTGEN also has `FireUp=2`; standard infantry like GGI uses `FireUp=6`. VLADIMIR's pistol fires earlier in the animation. |
| `PrimaryFireFLH` | `100,-25,135` | Firing offset (X=100, Y=−25 left, Z=135 head-height — high-shoulder pistol pose). |
| `SecondaryFireFLH` | `100,-25,135` | Same — but VLADIMIR has no `Secondary` weapon, so this is unused. |

### Referenced `[E1Sequence]` (full frame table — reused verbatim from GI)

```ini
[E1Sequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=164,6,6
Down=260,2,2
Crawl=86,6,6
Up=276,2,2
FireProne=212,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=134,15,0
Die2=149,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=56,15,0,S
Paradrop=0,1,0
Panic=8,6,6
```

Same layout as documented in `allied/E1.md` — VLADIMIR's sprite frames are
arranged at the same offsets as the GI's. The actual pixel art is unique
(`VLADIMIR.SHP`), but each row's `start_frame` matches a corresponding pose
in the GI SHP.

---

## 3. Weapon — `[Pistola]`

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
| `Damage` | `2` | 2 HP per shot — kills basic infantry in ~50 ROF cycles. |
| `ROF` | `20` | 20-tick cooldown (≈1.2 s). |
| `Range` | `3` | 3-cell range — short. |
| `Projectile` | `InvisibleLow` | Inviso projectile that respects cliffs/elevation/walls. See §3.1. |
| `Speed` | `100` | Bullet speed (inviso). |
| `Warhead` | `SA` | Small-Arms warhead — see §4. |
| `Report` | `CivAttack` | Civilian attack sound — 2 clips (`icivatta`, `icivattb`), FShift ±3, vol 70. |

### 3.1 `[InvisibleLow]` projectile

```ini
[InvisibleLow]
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=yes
SubjectToWalls=yes
```

- `Inviso/Image=none` — no visible bullet.
- `SubjectToCliffs/Elevation/Walls=yes` — respects terrain blocks.
- INI comment in nearby code: "Use this for weapons that are invisble but are subject to cliffs, elevation and walls."

---

## 4. Warhead — `[SA]`

```ini
[SA]
;DB Changed how Plate interacts with this warhead on 6/6. See also AP warhead.
;Verses=100%,80%,70%,50%,25%,25%,75%,50%,25%,100%,100%
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=70%
```

| Key | Value | Effect |
|-----|-------|--------|
| `Verses` | `100,80,80,50,25,25,75,50,25,100,100` | Small-arms damage curve: 100% vs `none` infantry, 80% vs flak/plate, 50% vs light, 25% vs medium/heavy (very poor vs tanks), 75% vs wood (effective vs civilian buildings), 50% vs steel, 25% vs concrete, 100% vs both special slots. |
| Commented `Verses` | (old slot-3 = 70%) | INI history-note: plate armor's response was tweaked. |
| `InfDeath` | `1` | Small-arms infantry death anim (standard slumping death — InfDeath table compiled across docs: 1=small-arms/standard). |
| `AnimList` | `PIFFPIFF,PIFFPIFF` | Hit-spark animation (listed twice — gamemd picks randomly, but both entries are identical, so it always plays PIFFPIFF). |
| `Bullets` | `yes` | Marks the warhead as bullet-impact (vs explosive) — used by sound/AI categorisation. |
| `ProneDamage` | `70%` | Infantry in prone state take 70% damage (only 30% reduction; pistol is not heavily affected by suppression). |

VLADIMIR shooting a `Strength=100, Armor=none` GI: `2 × 100% × 1.0 = 2 dmg/shot` × ROF 20 = 0.1 DPS (raw). Kills GI in ~50 shots × 1.2 s = ≈60 s. Functionally non-threatening to anything.

---

## 5. Voices / sounds

```ini
[Dummy]
Volume=0		; no sound
```

```ini
[FlakTroopDie]
Sounds= $ifladia $ifladib $ifladic $ifladid $ifladie
Priority=low
Control= random
```

```ini
[CivAttack]
Sounds= icivatta icivattb
Control= random
FShift= -3 3
volume=70
```

```ini
[InfantrySquish]
Sounds=igensqua
FShift= -10 10
Volume=65
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=Dummy` | Volume=0 silent | Silent on click-select |
| `VoiceMove=Dummy` | Volume=0 silent | Silent on move order |
| `VoiceAttack=Dummy` | Volume=0 silent | Silent on attack order |
| `VoiceFeedback=Dummy` | Volume=0 silent | Silent under attack |
| `DieSound=FlakTroopDie` | 5 clips, low priority | Death scream — reuses Flak Trooper's death voice |
| `Report=CivAttack` (on weapon) | 2 clips, FShift ±3, vol 70 | Pistol-fire sound |
| `CrushSound=InfantrySquish` | `igensqua`, vol 65 | When crushed |

Net audio profile: VLADIMIR is silent except for pistol shots, the squish-on-crush, and a Flak-Trooper-style death scream. The "Dummy" voice pattern is shared with other civilian-class stubs (CIV1/CIV2/CIV3) and removes the "yes sir / on my way" responses that named heroes would normally have.

---

## 6. Prerequisites / owners / availability

- `TechLevel=-1` + `AllowedToStartInMultiplayer=no` → **never buildable** in any normal context.
- `Owner=Russians,Confederation,Africans,Arabs,YuriCountry` — Soviet houses + Yuri can possess (if mission-spawned or captured).
- `Prerequisite=` — **none**; consistent with placeholder/mission-spawn role.
- No `RequiredHouses=`, no `RequiresStolen*Tech=`, no `BuildLimit=`.

VLADIMIR's only reachable spawn paths are:
1. **Map preplacement** — placed in a `.map` file's `[Units]` or `[Infantry]` section.
2. **Trigger/script spawn** — created via mission AI events (e.g. campaign script triggers).
3. **Crate goodie** — `CrateGoodie` flag is **not set** on VLADIMIR (only on vehicles with that flag), so this path doesn't apply.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 VLADIMIR-specific code in `gamemd.exe`

| Query (search_strings) | Result |
|------------------------|--------|
| `VLADIMIR` | 0 matches |
| `Vladimir` | 0 matches |
| `Name:VLADIMIR` | 0 matches |
| `Pistola` (weapon name) | 0 matches |

⇒ **No VLADIMIR-specific code path or string reference** in the binary. All behavior is driven by generic TechnoType/InfantryType/WeaponType/WarheadType flag handling.

The CSF key the UI shows is `Name:CIV1` (Civilian Male 1) — but because `UseOwnName=true`, the rendered label becomes "Vladimir" from the INI `Name=` field instead.

### 7.2 Flag-scope verification (live flags VLADIMIR carries)

| Key | Scope | Reference |
|-----|-------|-----------|
| `UseOwnName` | InfantryType | Cheat sheet — InfantryTypeClass__ReadINI 0x00524xxx range |
| `ImmuneToVeins` | TechnoType | Cheat sheet |
| `IFVMode` | TechnoType | 0x00714787 (verified prior iterations) |
| All other flags (Strength, Speed, Sight, Cost, Soylent, Points, Pip, Locomotor, MovementZone, ThreatPosed, Size, voice hooks) | TechnoType / AbstractType | Generic readers |

### 7.3 Live behaviors

| Behavior | Driver | Notes |
|----------|--------|-------|
| Displayed name "Vladimir" not "Civilian Male 1" | `UseOwnName=yes` overrides CSF lookup with `Name=` value | Only INI-level customization of VLADIMIR's identity. |
| Completely silent on player commands | `VoiceSelect/Move/Attack/Feedback=Dummy` with `[Dummy] Volume=0` | Generic dummy-voice pattern; no special engine handling. |
| Death scream uses Flak Trooper voice | `DieSound=FlakTroopDie` | Asset reuse — no unique Vladimir death clip exists. |
| Ignored by enemy AI auto-targeting | `ThreatPosed=0` | AI threat-scan skips zero-threat units when picking targets. |
| Cannot be built | `TechLevel=-1` combined with absent `Prerequisite=` | Build-availability resolver rejects. |
| Standard infantry walk | `Locomotor={4A582744-...}` | WalkLocomotionClass. |

### 7.4 Behaviors NOT present in VLADIMIR

- No `Hero=yes` / no special-protection flag — VLADIMIR dies as easily as any 50-HP unit.
- No `Insignificant`, no `Nominal`, no `Civilian` (all commented out) — VLADIMIR is a *named* combat unit despite its placeholder role.
- No `Fraidycat` — does not flee when shot at; will stand and try to fight with the pistol.
- No `DetectDisguise`, no `ImmuneToPsionics`, no `C4`, no `Ivan` flag — just a basic soldier.
- No `Secondary` weapon — only the pistol.
- No `Deployer`, no special-attack.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| Commented `;Ammo / ;Fraidycat / ;Civilian / ;Nominal / ;Insignificant` | n/a (commented) | Inactive. |

No fog-of-war flags, no veinhole references, no Insignificant gating, no Hospital, no tunnel/subterranean refs.

---

## 9. Veterancy

VLADIMIR has **no** `VeteranAbilities=` or `EliteAbilities=` keys, and **no** `Trainable=`
key (default is `yes`, but with `ThreatPosed=0` he never gains XP from combat anyway —
nothing attacks him voluntarily, and his pistol rarely kills anything to gain offensive XP).

No `ElitePrimary=` either — would use `Primary=Pistola` regardless of rank.

In practice VLADIMIR is locked at rookie rank for his entire mission lifetime.

---

## 10. Cross-references

### Direct dependencies
- `[Pistola]` — weapon (§3)
- `[InvisibleLow]` — projectile (§3.1)
- `[SA]` — warhead (§4)
- `[PIFFPIFF]` (artmd) — hit-spark anim
- `[E1Sequence]` (artmd) — frame table (reused from GI)
- `[Dummy] / [FlakTroopDie] / [CivAttack] / [InfantrySquish]` (soundmd) — sounds (§5)
- `[CIV1]` CSF entry — `UIName=` target (overridden by `UseOwnName=true`, but still resolved for compat)

### Conceptual companions
- **PENTGEN** (`allied/PENTGEN.md` — TODO) — Allied campaign equivalent. Same stats (50 HP, Pistola, TechLevel=-1, Cost=10) but uses **GI voices** (not Dummy), `Sequence=GenSequence` (its own anim), and Allied `Owner=`. Direct mirror of VLADIMIR.
- **PRES** (`civilian/PRES.md` — TODO) — President Dugan; same pattern of named placeholder.
- **CIV1 / CIV2 / CIV3** (`civilian/CIV1.md` etc. — TODO) — basic civilian male/female with the same Dummy voice pattern and similar stub stats. VLADIMIR's `UIName=Name:CIV1` points at the same CSF entry; only `UseOwnName=true` differentiates.

### Deep-RE docs
- None directly relevant — VLADIMIR has no hardcoded behavior to research.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[VLADIMIR]` rulesmd key explained | ✅ §1 |
| Every `[VLADIMIR]` artmd key explained | ✅ §2 |
| Reused `[E1Sequence]` cross-referenced | ✅ §2 |
| Weapon + projectile + warhead | ✅ §3–§4 |
| All voices expanded (including `[Dummy]` silence) | ✅ §5 |
| Prereqs / owners / spawn paths | ✅ §6 |
| Hardcoded behavior — Ghidra searches for VLADIMIR ID | ✅ §7 (four queries, all 0 matches) |
| TS-legacy filter | ✅ §8 |
| Veterancy treated correctly (no keys = locked rookie) | ✅ §9 |
| Cross-refs to companion campaign stubs | ✅ §10 |

**Open follow-ups (none load-bearing):**
- Cross-check `UseOwnName` precedence vs `UIName` CSF lookup by decompiling the InfantryType name-resolution path. Not critical for parity since visible result ("Vladimir" appears) is consistent with INI intent.
- Mission-script audit — which campaign maps actually preplace VLADIMIR. Belongs in a campaign-script analysis, not a unit doc.
