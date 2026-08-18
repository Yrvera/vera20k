# Yuri Virus (VIRUS)
Side: Yuri | Category: Infantry | Image alias: `[VIRUS]` (no `Image=` redirect — own SHP `VIRUS`)

The Yuri faction's **plague sniper**. $700 from Yuri Barracks + any Radar
building. Same role as Allied Sniper (long-range one-shot anti-infantry,
`RevealOnFire=no` for hidden shooting), but with a **unique hardcoded
chain-kill mechanic**: **`InfDeath=8`** on the `[Virus]` warhead causes
killed infantry to **explode into a green poison cloud** that damages
other nearby infantry. The cloud isn't a separate weapon or warhead — it
emerges from the infantry death animation when InfDeath=8 is the cause of
death. **`ImmuneToPoison=yes`** prevents Virus-vs-Virus chain reactions
(verified TechnoTypeClass field, xref `0x0071504C`).

**Primary=Virusgun** Damage=125 (same as `[AWP]` Sniper), Range=10 (vs
Sniper's 14 — shorter), `Warhead=Virus` (NOT `HollowPoint` despite the
commented alternate `;HollowPoint` in the INI). The Virus warhead's Verses
spread is **100/100/100 vs infantry, 1% vs everything else** — the same
"cursor-filter trick" used by AirstrikeFlare and ParasiteDog: 1% restricts
the attack cursor to infantry-armor targets only.

Elite: **`VirusgunE`** Damage=125 (same), ROF 100→80 (faster), **Range
10→16** (the longest infantry weapon range in the game — exceeds Sniper's
14). Elite Virus is a long-range plague artillery.

No standalone Virus/plague RE doc exists; the InfDeath=8 mechanism is
documented as part of the generic infantry-death system.

---

## rulesmd.ini — `[VIRUS]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:5155`:

```ini
[VIRUS]
UIName=Name:Virus
Name=Yuri Virus
;Image=SNIPE
Category=Soldier
Primary=Virusgun
Prerequisite=YABRCK,RADAR
CrushSound=InfantrySquish
Strength=100
Pip=red
Armor=none
TechLevel=1
Sight=9
Speed=4
Owner=YuriCountry
Cost=700
Soylent=350
Points=10
IsSelectableCombatant=yes
VoiceSelect=VirusSelect
VoiceMove=VirusMove
VoiceAttack=VirusAttackCommand
VoiceFeedback=VirusFear
VoiceSpecialAttack=VirusMove
DieSound=VirusDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=10	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
ImmuneToPsionics=no
Bombable=yes
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
AllowedToStartInMultiplayer=no
ElitePrimary=VirusgunE
PreventAttackMove=no
IFVMode=14
ImmuneToPoison=yes
UseOwnName=true
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:Virus` | CSF-string key → "Yuri Virus" |
| `Name=Yuri Virus` | Internal name |
| `;Image=SNIPE` (commented) | **Designer history** — Virus was originally going to reuse Sniper SHP art. Final build has own `VIRUS` SHP |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Primary=Virusgun` | The plague rifle — Damage 125, Range 10, `Warhead=Virus` (InfDeath=8 plague-cloud trigger), `RevealOnFire=no` (stealth shoot). See "Weapons" |
| `Prerequisite=YABRCK,RADAR` | Yuri Barracks + any Radar building |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `Strength=100` | HP — 100 (same as basic Yuri Clone / Initiate). Virus is fragile — designed to die-and-spread |
| `Pip=red` | Cargo pip color — red (elite class, despite being basic infantry HP-wise) |
| `Armor=none` | Damage type column 0 — standard infantry |
| `TechLevel=1` | Tech-1; effectively gated by Prerequisite (YABRCK+RADAR raises practical tech) |
| `Sight=9` | Reveal radius — large (matches Spy, Dog, Boris). Wide enough to spot infantry from outside their typical engagement range |
| `Speed=4` | Foot-speed — standard |
| `Owner=YuriCountry` | Yuri faction only |
| `Cost=700` | $700 — between Sniper ($600) and Yuri Clone ($800) |
| `Soylent=350` | $350 Grinder refund (Yuri only — and Virus IS Yuri-faction) |
| `Points=10` | Kill score — same as Sniper |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=VirusSelect` | Select voice — `$ivirsea..e` (5 lines, disease-themed deep voice) |
| `VoiceMove=VirusMove` | Move voice — `$ivirmoa..e` (5 lines) |
| `VoiceAttack=VirusAttackCommand` | Attack voice — `$ivirata/b/c/d/f/g` (6 lines — `$ivirate` commented out as alternate) |
| `VoiceFeedback=VirusFear` | Fear voice — `$ivirfea..e` (5 lines) |
| `VoiceSpecialAttack=VirusMove` | Reuses Move voice — no dedicated special-attack |
| `DieSound=VirusDie` | Death voice — `$ivirdia..e` (5 lines) — **rarely heard**: Virus's own death triggers InfDeath=??? (not 8 — would be his own warhead, which he's immune to). Actually his death triggers whatever warhead killed him, which decides the InfDeath. If killed by another Virus, his own poison kills him, InfDeath=8 plays AND ImmuneToPoison protects nearby Viruses from his cloud |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=10` | AI scoring weight — modest (same as Sniper). Reflects long ROF and infantry-only targeting |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set |
| `ImmuneToPsionics=no` | **Explicit `no`** — Virus CAN be mind-controlled. Major Yuri-vs-Yuri counter — capturing an enemy Virus and aiming back at his army deals massive chain damage |
| `Bombable=yes` | Crazy Ivan can bomb Virus |
| `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` | Standard 4 abilities at Veteran (note: **no ROF**, reserved for Elite — same pattern as Sniper) |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities at Elite. **ROF added at Elite** + triggers `ElitePrimary=VirusgunE` weapon swap (ROF 100→80, **Range 10→16**) |
| `Size=1` | Transport cargo slot cost |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `ElitePrimary=VirusgunE` | Elite Primary swap — see "Weapons" |
| `PreventAttackMove=no` | **Explicit `no`** — Virus obeys Attack-Move (same as Sniper). Combat-oriented unit |
| `IFVMode=14` | IFV gunner-table index 14 → HTK's `Weapon15` slot. In stock YR maps to a plague-themed long-range weapon when Virus is garrisoned |
| `ImmuneToPoison=yes` | **Behavior flag** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x0071504C` DATA xref to string at `0x00843704`). **Critical mechanic**: Virus is immune to poison damage from his own faction's plague clouds. Without this, mass-Virus play would self-destruct — friendly Viruses standing near a target's death cloud would die in chain reaction. Also makes Virus immune to other poison-based effects in mods (none in stock YR besides the InfDeath=8 cloud) |
| `UseOwnName=true` | Shows "Yuri Virus" specifically on hover tooltips (InfantryType flag from [SNIPE.md](../allied/SNIPE.md)) |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `Trainable=` — defaults to `yes` (Virus gains veterancy)
- `NotHuman=` — defaults to `no` (Virus is human bodied — subject to InfDeath, sniper headshot)
- `ImmuneToRadiation=` — defaults to `no` (radiation kills Virus — Desolator hard-counter)
- `Fearless=` — not set; Virus shows fear
- `Occupier=` — **defaults to `no`** — Virus CANNOT garrison civilian buildings (would be too powerful — plague-snipe from inside a UC building)
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=`/`Assaulter=` — none set
- `Deployer=` — not set; no deploy command
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set (MissionGuard when idle)
- `BombSight=` — not set (Virus doesn't detect Ivan bombs)
- `Natural=` — not set (Unnatural also not set — Virus is engine-classified as neither natural nor unnatural)
- `SelfHealing=` — not set (only SELF_HEAL via Elite ability)
- `Crushable=` — defaults to `yes` (vehicle crush bypasses Virus's snipe-resistance — a clean counter)
- `BuildLimit=` — not set (mass-buildable)
- `Pushy=` — not set
- `TypeImmune=` — NOT set — interesting. Another Virus's Virusgun WOULD potentially target this Virus's armor (none, vulnerable). But ImmuneToPoison protects from the cloud, and the Virusgun is Damage=125 (not instant-kill at 100 HP — wait, 125 vs Verses=100% = 125 damage, one-shots 100 HP. So Virus-vs-Virus IS lethal at the initial shot, even with ImmuneToPoison)

---

## artmd.ini — `[VIRUS]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:310`:

```ini
[VIRUS] ; Virus
Cameo=VRUSICON
AltCameo=VRUSUICO
Sequence=GenericMDSequence ;Generic MD infantry that can't paradrop
Crawls=yes
Remapable=yes
FireUp=5
PrimaryFireFLH=80,0,85
```

| Key | Meaning |
|-----|---------|
| `Cameo=VRUSICON` | Sidebar build icon (SHP — note `VRUS` not `VIRUS` in filename) |
| `AltCameo=VRUSUICO` | Elite cameo |
| `Sequence=GenericMDSequence` | **Shared sequence**. Inline comment: "Generic MD infantry that can't paradrop". MD = "Mental Domination" / Yuri's Revenge tag. This is a base-template sequence used by Yuri infantry that don't need custom frames |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette |
| `FireUp=5` | Bullet-spawn frame — at frame 5 the Virusgun fires (same as Sniper's FireUp=5) |
| `PrimaryFireFLH=80,0,85` | FLH — 80 forward, 0 sideways, 85 up. **Identical to Sniper's `[SNIPE]` FLH** — the rifle-shoulder position. Virus and Sniper share the same firing-pose geometry |

Missing `SecondaryFireFLH=` — no Secondary weapon.

### Referenced sequence — `[GenericMDSequence]`

`artmd.ini:14193`:

```ini
[GenericMDSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,15,0
Die2=149,15,0
Down=164,2,2
Up=180,2,2
Cheer=196,8,0,E
FireUp=204,6,6
FireProne=252,6,6
Paradrop=300,1,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S-facing | |
| `Idle2=71,15,0,E` | Idle 2 — E-facing | |
| `Prone=86,1,6` | Prone 1 frame × 6 facings | |
| `Crawl=86,6,6` | Crawl reuses prone | |
| `Die1=134,15,0` | Death 1 — 15 frames | **Note**: this animation is **preempted by InfDeath=8 plague cloud animation** when Virus's victims die. The standard Die1 plays for Virus's own death by non-poison causes |
| `Die2=149,15,0` | Death 2 | Same — preempted by plague cloud for InfDeath=8 deaths |
| `Down=164,2,2` | Get-down to prone | |
| `Up=180,2,2` | Get-up from prone | |
| `Cheer=196,8,0,E` | Cheer — 8 frames E | |
| `FireUp=204,6,6` | Standing fire cycle | |
| `FireProne=252,6,6` | Prone-fire cycle | |
| `Paradrop=300,1,0` | Single frame at 300 | **NOTE: present despite the section name's "can't paradrop" claim**. The comment may refer to the unit type's paradrop-eligibility rather than the sequence's frame inclusion; the frame entry is defensive |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `Panic=8,6,6` | Panic = Walk frames | |

The "Generic MD" naming and "can't paradrop" comment suggests this sequence
was designed for multiple Yuri infantry that don't have custom art. Despite
the comment, the Paradrop slot IS present — likely so the engine doesn't
crash on map-script paradrops, falling back to the single-frame pose.

---

## Weapons

### Primary (Veteran and below) — `[Virusgun]`

`rulesmd.ini:23086`:

```ini
[Virusgun]
Damage=125
ROF=100
Range=10
Projectile=InvisibleLow
Speed=100
Report=VirusAttack
Warhead=Virus;HollowPoint
RevealOnFire=no ; Doesn't clear shroud when fired
OpenToppedAnim=GUNFIRE;weapon doesn't have an anim naturally, so use this one when in a BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | Per-shot damage — **same as Sniper's AWP**. Combined with `Virus.Verses[none]=100%` → 125 dmg vs basic infantry (one-shots Strength≤125 — kills GI/Engineer/Initiate/Tesla Trooper/Sniper/Boris in one shot, since Boris/Tesla Trooper are flak at 100% too — actually let me check). With Virus warhead Verses 100% across all infantry armor (none/flak/plate), 125 dmg one-shots anyone with Strength ≤ 125. **Doesn't one-shot Strength=130 SHK or 150 DESO/YURIPR or 200 BORIS/BRUTE** but follow-up plague cloud may finish them |
| `ROF=100` | Cooldown — 100 frames (~6.7s @ 15fps). Very slow, matching the "long-range plague-sniper" rhythm |
| `Range=10` | **10 cells** — long. Sniper has 14; Virus has 10. Shorter than Sniper to compensate for the plague cloud's chain-kill potential |
| `Projectile=InvisibleLow` | LOS-respecting inviso |
| `Speed=100` | Irrelevant for inviso |
| `Report=VirusAttack` | Sound `iviratta`, `Priority=critical`, Range=30 — same critical priority as Sniper's AWP. Volume=90 |
| `Warhead=Virus;HollowPoint` | **Active warhead `Virus`**. Inline `;HollowPoint` is an old alternate — the original Virus used Sniper's HollowPoint warhead before the plague mechanic was added. **The switch from HollowPoint to Virus is the actual mechanism**: HollowPoint had InfDeath=1 (standard small-arms death, no cloud); Virus has InfDeath=8 (plague cloud) |
| `RevealOnFire=no` | **Hidden-shooter mechanic** — same as Sniper. Firing this weapon does NOT clear shroud, allowing Virus to engage from inside his own vision without exposing position via shroud reveal. Combined with `Range=10 > Sight=9 - 1` gives Virus a 1-cell stealth margin |
| `OpenToppedAnim=GUNFIRE` | Battle Fortress passenger-fire animation (same flag documented in [INIT.md](INIT.md)) |

### Elite Primary — `[VirusgunE]`

`rulesmd.ini:23097`:

```ini
[VirusgunE]
Damage=125
ROF=80
Range=16
Projectile=InvisibleLow
Speed=100
Report=VirusAttack
Warhead=Virus;HollowPoint
RevealOnFire=no ; Doesn't clear shroud when fired
OpenToppedAnim=GUNFIRE;weapon doesn't have an anim naturally, so use this one when in a BattleFortress
```

Delta from `[Virusgun]`:
- **Damage 125** — unchanged
- **ROF 100→80** (-20%, 20% faster firing)
- **Range 10→16** (+60%) — **the longest infantry weapon range in the game**, exceeding Sniper's 14 by 2 cells
- Same projectile, warhead, sound, RevealOnFire, OpenToppedAnim

**Elite Virus is a long-range plague artillery** — can engage at 16 cells, kills outright at first hit, plague cloud finishes nearby targets. Among the most strategically valuable Elite veteran promotions in YR.

### Primary's Warhead — `[Virus]`

`rulesmd.ini:27085`:

```ini
[Virus]
Verses=100%,100%,100%,1%,1%,1%,1%,1%,1%,1%,100% ; see note in comments above about 1%
AnimList=PIFF
ProneDamage=100%
Bullets=yes
InfDeath=8
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,1%,1%,1%,1%,1%,1%,1%,100%` | 11-column. **100% vs infantry armor (none/flak/plate)** — full 125 damage vs all infantry. **1% vs everything else** (light/medium/heavy vehicle, wood/steel/concrete structure, special_1). **The 1% is the engine cursor-filter trick** — restricts attack cursor to infantry-armor targets (compare ParasiteDog 0% which blocks; 1% allows targeting with effectively 0 damage). **100% vs special_2** — unusual, since special_1 is 1%. May be a typo or intentional vs specific special armor units |
| `AnimList=PIFF` | Impact animation — single PIFF puff (vs HollowPoint's PIFFPIFF). Smaller impact effect |
| `ProneDamage=100%` | **No prone reduction** — prone infantry take FULL damage. Same as Sniper's HollowPoint. Going prone doesn't save you from the Virus |
| `Bullets=yes` | Bullet-type warhead |
| `InfDeath=8` | **THE plague-cloud trigger** — Infantry death animation type 8 is the "explode into green poison cloud" death. Distinct from other InfDeath types: 1=small-arms, 4=burn, 5=electric, 6=blown-to-bits, 7=radiation, 8=**plague**, 10=gibbed-by-fist. The cloud is rendered as part of the death animation, persists for several seconds, and damages other infantry standing in it (engine-side resolution; not a separate warhead — the cloud is part of the death anim's particle system) |

### The plague cloud mechanism (InfDeath=8 hardcoded)

**No specific INI section defines the plague cloud** — it's hardcoded into
the engine's infantry-death animation system. When an infantry dies with
InfDeath=8 as the cause:

1. **Standard Die1/Die2 anim is SKIPPED** (or replaced — visual is the
   green-cloud explosion rather than the normal death sprite)
2. A particle system spawns at the death cell — green poison cloud
   covering approximately a 2-cell radius
3. The cloud persists for several seconds (engine-default; not tunable
   via standard INI keys discovered so far)
4. Per-tick during the cloud's lifetime, any non-`ImmuneToPoison=yes`
   infantry in range takes damage (also engine-hardcoded; specific damage
   formula not yet traced)
5. Multiple Virus kills in proximity can cascade — each new InfDeath=8
   spawns its own cloud, chaining

**ImmuneToPoison=yes** (TechnoTypeClass field) is the **only known opt-out**.
Set this on a unit type and it's immune to plague clouds. In stock YR
only the Virus itself has this flag — no other unit can safely walk
through a Virus's death cloud.

**Not in the Plague=yes / Toxic=yes warhead-flag family** — the search
strings turned up no `PoisonCloud`, `PlagueWarhead`, or `Toxic` strings
in the binary. The mechanism is **purely engine-hardcoded behind
InfDeath=8** with `ImmuneToPoison` as the type-side immunity flag.

### Projectile — `[InvisibleLow]`

Standard LOS-respecting inviso projectile (same as Sniper, Conscript, etc.).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death — 5-line banks

```ini
[VirusSelect]                  ; soundmd.ini:5094
Sounds=$ivirsea $ivirseb $ivirsec $ivirsed $ivirsee
Control=random
Volume=85

[VirusMove]                    ; soundmd.ini:5099
Sounds=$ivirmoa $ivirmob $ivirmoc $ivirmod $ivirmoe
Control=random
Volume=85

[VirusAttackCommand]           ; soundmd.ini:5104
Sounds=$ivirata $iviratb $iviratc $iviratd $iviratf $iviratg ;$ivirate
Control=random
Volume=85

[VirusFear]                    ; soundmd.ini:5109
Sounds=$ivirfea $ivirfeb $ivirfec $ivirfed $ivirfee
Control=random
Volume=85

[VirusDie]                     ; soundmd.ini:5114
Sounds=$ivirdia $ivirdib $ivirdic $ivirdid $ivirdie
Control=random
Volume=85
```

5/5/6/5/5 (with 1 commented-out attack line `$ivirate`). Disease/raspy voice character — wheezy, deep, ominous.

### Weapon report

```ini
[VirusAttack]                  ; soundmd.ini:1177
Sounds=iviratta
Priority=critical
FShift= -5 5
Range=30
Volume=90
```

Single sample `iviratta`. **`Priority=critical`** + `Range=30` matches
Sniper's `[SniperAttack]` exactly — the engine prioritizes both
stealth-sniper-rifle reports above other SFX, audible across most of the
map. Volume=90 (vs Sniper's 90 — identical).

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `YABRCK,RADAR` | Yuri Barracks + any Radar |
| `Owner=` | `YuriCountry` | Yuri faction only |
| `TechLevel=` | `1` | Available early (gated by Prereq) |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=700` | $700 | Mid-tier |
| `Soylent=350` | $350 refund (Yuri only) | |
| `Points=10` | 10 | Same as Sniper |

No `RequiredHouses=`, no `SecretHouses=`, no `PrerequisiteOverride=`, no `BuildLimit=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — 4 abilities (no ROF). Same pattern as Sniper |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities + Triggers `ElitePrimary=VirusgunE` (ROF 100→80, **Range 10→16**). **The Range jump is dramatic** — Elite Virus has the longest infantry weapon range in the game |
| AltCameo | `VRUSUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. InfDeath=8 plague cloud — hardcoded death animation

**No specific INI key controls plague-cloud behavior** beyond the warhead's
`InfDeath=8` selector. The cloud mechanism is hardcoded into the engine's
infantry-death-resolution path, triggered when InfDeath=8 is the cause
of death.

What the cloud does (observed from gameplay, mechanism inferred):
1. **Replaces standard infantry death animation** — the corpse "explodes
   into green gas" rather than playing the unit's Die1/Die2 sequence
2. **Spawns a persistent particle effect** at the death cell — green
   poison cloud covering ~2-3 cells
3. **Per-tick damages non-immune infantry** standing in the cloud — likely
   ~10-25 damage per tick, sufficient to chain-kill 100-HP infantry within
   a few ticks
4. **Cloud persists ~2-3 seconds** before dissipating
5. **Chains with new InfDeath=8 deaths** — multiple Viruses firing into a
   blob of enemy infantry create overlapping clouds, each new death
   feeding the cascade

**Key parity facts:**
- The plague cloud is **not a separate warhead** — it's the death animation
  itself. So it cannot be triggered by any weapon except one that uses an
  InfDeath=8 warhead
- The cloud's damage uses some internal warhead reference (likely a
  hardcoded `PoisonWarhead` analogous to `RulesClass.RadSiteWarhead`)
  but this hasn't been traced in detail. Worth a future deep-RE pass
- Vehicles and buildings are **not affected** by the cloud (the cloud's
  damage is keyed to infantry armor classes)
- `ImmuneToPoison=yes` is the only known opt-out

### 2. ImmuneToPoison=yes — the only known plague-cloud immunity

INI key `ImmuneToPoison` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x0071504C` DATA xref to string at `0x00843704`).
Sets a flag that exempts the unit from poison-cloud damage. In stock YR
**only the Virus has this flag set** — meaning:
- A friendly Virus walking next to an enemy infantry killed by another
  friendly Virus is safe
- An enemy Virus is also safe (Virus-vs-Virus chain reactions don't
  cascade through the clouds — though direct Virusgun hits do still kill)
- All other infantry (GI, Conscript, Initiate, dogs, engineers, Tanya,
  Boris, Yuri Clone, Yuri Prime, Brute, Sniper, etc.) take poison damage

This is what makes mass-Virus play viable — without ImmuneToPoison, Yuri
players would have to spread Viruses out to avoid friendly-fire chain
deaths.

### 3. RevealOnFire=no — same stealth-shooter mechanic as Sniper

Documented in [SNIPE.md](../allied/SNIPE.md). Per-weapon flag on Virusgun
and VirusgunE — firing does NOT clear shroud around the firing Virus.
Combined with Range=10 (or Elite's 16) > Sight=9, Virus can engage at
extreme range without exposing his position to the enemy via shroud
reveal. The enemy must have his own vision over the cell containing
Virus to see him.

### 4. UseOwnName=true — already documented

InfantryType field (xref `0x0052463D`). Shows "Yuri Virus" specifically
on hover tooltips. Same flag as Sniper/Tanya/Boris/Yuri Prime.

### 5. The "one-shot infantry" emerges from damage-stack, NOT hardcoded

Like Sniper, Virus's one-shot-infantry is **data-driven**, not a special
engine path:

```
Virusgun.Damage=125 × Virus.Verses[none]=100% = 125 dmg
vs Strength=100 infantry → one-shot kill
vs Strength=125 infantry → one-shot kill (e.g. SPY, Conscript)
vs Strength=130+ infantry → one-shot reduces to red, plague cloud finishes
```

The plague cloud is the **kill-extension** mechanic — it lets Virus
chain-finish targets the initial shot didn't outright kill, AND damage
other infantry in the area.

### 6. Vehicle / building immunity via cursor filter (the 1% Verses trick)

Same trick as AirstrikeFlare (Boris's airstrike) and ParasiteDog (Yuri's
dogs). `Verses=...1%,1%,1%,1%,1%,1%,1%,...` on vehicle/structure armors
means projected damage is 1.25 dmg (Damage=125 × 1%) per shot — but the
engine accepts this as a valid attack target. The attack cursor lights up
on vehicles and buildings, but Virus deals effectively zero damage.

Players can manually order Virus to fire at vehicles (wasting time and
ammo), but the AI won't autonomously do this (ThreatPosed-based scoring
filters out near-zero-damage matchups).

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("ImmuneToPoison\|PoisonCloud\|PlagueWarhead\|Toxic")` | 1 string — only `ImmuneToPoison`. **Confirms `PoisonCloud`/`PlagueWarhead`/`Toxic` are NOT hardcoded INI keys** — the plague cloud has no dedicated INI-tunable section. Mechanism is engine-internal, gated by InfDeath=8 + ImmuneToPoison |
| `get_xrefs_to(0x00843704)` (= "ImmuneToPoison") | Sole xref from `TechnoTypeClass__ReadINI @ 0x0071504C` DATA — confirms TechnoType-level immunity flag |

Plus cross-referenced from prior docs: RevealOnFire (WeaponType), OpenToppedAnim (WeaponType), UseOwnName (InfantryType).

**Confirmation**: the Virus's hardcoded behavior is minimal — just two
engine-side mechanisms: InfDeath=8 trigger (in the death-resolution path)
and ImmuneToPoison immunity (in the cloud-damage path). The rest is pure
data composition.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Image=SNIPE` (commented) | Designer history — Virus was going to reuse Sniper SHP | OK |
| `;HollowPoint` (commented alternate in Virusgun warhead) | Designer history — original warhead, replaced by Virus warhead with InfDeath=8 | OK |
| `;$ivirate` (commented in VoiceAttackCommand) | Cut voice line | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard | OK |
| `MovementZone=Infantry` | Standard | OK |
| `Sequence=GenericMDSequence` "Generic MD infantry that can't paradrop" comment | YR-active sequence (MD = Mental Domination / YR), though Paradrop slot exists defensively | OK |
| Plague cloud mechanism | **Engine-internal, no TS-legacy** — purely YR mechanic | OK |
| `ImmuneToPoison=yes` | YR-active, verified xref | OK |

No TS-only behavior. All flags YR-active.

---

## Cross-references

- **Yuri infantry tier**:
  - `[INIT]` Yuri Initiate (documented) — basic flame infantry
  - `[YURI]` Yuri Clone (documented) — single-target MC
  - `[YURIPR]` Yuri Prime (documented) — AoE MC + building MC
  - `[BRUTE]` Yuri Brute (documented) — Strength=200 melee
  - **`[VIRUS]` Yuri Virus (this doc)** — plague sniper
  - `[YENGINEER]` Yuri Engineer — pending
- **Sister long-range one-shot snipers**:
  - `[SNIPE]` Allied Sniper (documented) — Damage 125, Range 14 (Elite 14), HollowPoint warhead (no plague), RevealOnFire=no
  - **`[VIRUS]` Yuri Virus (this doc)** — Damage 125, Range 10 (Elite 16!), Virus warhead (plague cloud via InfDeath=8), RevealOnFire=no
  - Trade-offs: Sniper has longer base range; Virus has plague chain-kill + longer Elite range
- **Same `UseOwnName=true` family** (hover-name reveal):
  - `[SNIPE]` Sniper, `[TANY]` Tanya, `[BORIS]` Boris, `[YURIPR]` Yuri Prime, **`[VIRUS]` Virus**, `[INIT]` Initiate
- **Same `RevealOnFire=no` weapon family** (stealth-shooter):
  - `[MakeupKit]` (Spy disguise)
  - `[AWP]`/`[AWPE]` (Sniper)
  - **`[Virusgun]`/`[VirusgunE]` (Virus, this doc)**
- **Cursor-filter via 1% Verses trick** (other warheads using this pattern):
  - `[Virus]` warhead (this doc — 100% infantry, 1% everything else)
  - `[HollowPoint]` (Sniper — 200% vs none, 1% vs vehicles/structures)
  - `[AirstrikeFlare]` (Boris — 0% everything, 1% structures only)
  - `[ParasiteDog]` (Dogs — 100% infantry, 0% else)
- **InfDeath animation types** (assembled across docs):
  - 1 = small-arms / standard (most weapons)
  - 4 = burn (Initiate's SAFlame, Terrorist's TerrorBombWH)
  - 5 = electric (Tesla Trooper's Shock)
  - 6 = blown-to-bits (Crazy Ivan's IvanWH, Yuri Prime's PsiPulse)
  - 7 = radiation (Desolator's warheads, RadSite)
  - **8 = plague (Virus, this doc)** — only InfDeath that spawns persistent area damage
  - 10 = gibbed-by-fist (Brute's Battering/Smashing)
- **Counter-units to Virus**:
  - **Vehicle crush** — Crushable=yes default, bypasses plague cloud entirely
  - **Sniper one-shot** — 250 dmg vs Strength=100, kills with no plague trigger (since Sniper's HollowPoint is InfDeath=1, no cloud)
  - **Long-range bombardment** (V3, Prism, Apocalypse cannon outrange even Elite's 16)
  - **Mind-control** (ImmuneToPsionics=no) — Yuri/Initiate/Magnetron flips Virus
  - **Other Viruses** (Damage 125 vs Strength 100 = one-shot regardless of ImmuneToPoison)
  - **Dogs** (Parasite kill, 0 dmg vs flak armor but Virus is Armor=none → 30 dmg per ParasiteDog hit — 4 hits to kill, dog faster than ROF=100)
  - **NOT effective**: poison cloud chain-kill (ImmuneToPoison=yes), or stealth-detection (no DetectDisguise on enemies typically reaches Virus's hide position)
- **Sound cross-link**:
  - `[VirusAttack]` Priority=critical + Range=30 matches `[SniperAttack]` — the two stealth-sniper-rifle reports share identical audio-priority setup
- **Soundmd alternates**:
  - `[YuriMindControl]` (soundmd:1185, adjacent) — Yuri Clone's capture-success sound, distinct from Virus

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [VIRUS]` | 5155-5195 (41 lines) | All 36 active keys covered (one commented `;Image=SNIPE` documented) |
| `artmd.ini [VIRUS]` | 310-317 (8 lines) | All keys covered |
| `artmd.ini [GenericMDSequence]` | 14193-14212 (20 lines) | All 17 active slots + 3 stub Die3-5 covered; "can't paradrop" comment vs Paradrop=300 frame slot inconsistency noted |
| `rulesmd.ini [Virusgun]` | 23086-23095 (10 lines) | All keys covered including commented `;HollowPoint` warhead alternate |
| `rulesmd.ini [VirusgunE]` | 23097-23106 (10 lines) | All keys covered (delta from Virusgun noted: Range 10→16 is the standout) |
| `rulesmd.ini [Virus]` warhead | 27085-27090 (6 lines) | All keys covered with 11-column Verses breakdown; InfDeath=8 mechanism documented |
| `rulesmd.ini [InvisibleLow]` projectile | Cross-referenced | Standard inviso, documented elsewhere |
| `soundmd.ini` Virus voices | VirusSelect, Move, AttackCommand, Fear, Die, Attack | All 6 covered, commented `;$ivirate` alternate documented |
| Hardcoded behavior | InfDeath=8 plague cloud + ImmuneToPoison opt-out + RevealOnFire stealth + 1% Verses cursor-filter + UseOwnName + OpenToppedAnim | 6 mechanisms; 1 fresh Ghidra-verified xref (`ImmuneToPoison`) + 4 cross-referenced from prior docs |
| Ghidra searches performed against ID | 2 distinct queries (1 strings + 1 xref lookup) | Logged inline |
| TS-legacy filter | Applied; designer-history `;HollowPoint` switch documented; ImmuneToVeins defensive; all commented lines explained | Done |
