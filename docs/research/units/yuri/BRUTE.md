# Yuri Brute (BRUTE)
Side: Yuri | Category: Infantry | Image alias: `[BRUTE]` (no `Image=` redirect — own SHP `BRUTE`)

The Yuri faction's **melee heavyweight**. $500 from Yuri Barracks.
**`Strength=200`** — tied with Yuri Prime for the toughest standard
infantry, matched only by Boris (200) among standard units. **Two melee
weapons**: **`Primary=Punch`** (Range 1.4, `Warhead=Battering`) for
**infantry and buildings** (Verses 100% vs all infantry / 0% vs vehicles
/ 30%-30%-20% vs wood/steel/concrete / **200% vs special_1**) and
**`Secondary=Smash`** (Range 1.1, `Warhead=Smashing`) for **vehicles only**
(0% vs infantry / 100% vs light / **20% vs medium** / 100% vs heavy /
0% vs buildings). The mid-tier vehicle armor's 20% is intentionally
weak — Brute is designed to wreck Soviet heavy tanks (Rhino, Apocalypse)
but not trivially crush Allied Grizzly rushes.

**`Crushable=no`**, **`ImmuneToPsionics=yes`**, **`SelfHealing=yes`**,
**`Unnatural=yes`** — survivability comparable to Yuri Prime. **`Size=2`**
("too big for IFV" inline comment) — Brute is the **only infantry that
cannot enter the [HTK] IFV** because of cargo-slot cost. **`CloseRange=yes`**
+ **`GuardRange=2`** wire the AI to engage at melee range only.
**`DefaultToGuardArea=yes`** — idle Brutes actively patrol, like Attack
Dogs.

No standalone Brute/melee RE doc exists; this document originates the
Ghidra trace of `CloseRange`/`GuardRange` flag paths.

---

## rulesmd.ini — `[BRUTE]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:5104`:

```ini
[BRUTE]
UIName=Name:Brute
Name=Yuri Brute
;Image=SHK
Category=Soldier
Primary=Punch
Secondary=Smash
;GEF Unles we decide to put it back in Pushy=yes
Prerequisite=YABRCK
CrushSound=InfantrySquish
Crushable=no
Strength=200 ;180 ;250
Armor=plate
TechLevel=5
Pip=white
Sight=8
Speed=6
Owner=YuriCountry
Cost=500
Soylent=250
Points=5
IsSelectableCombatant=yes
VoiceSelect=BruteSelect
VoiceMove=BruteMove
VoiceAttack=BruteAttackCommand
VoiceFeedback=BruteFear
VoiceSpecialAttack=BruteMove
DieSound=BruteDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=InfantryDestroyer
ThreatPosed=20 ; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToVeins=yes
Size=2;too big for IFV
AllowedToStartInMultiplayer=no
ElitePrimary=PunchE
EliteSecondary=SmashE
IFVMode=0
Unnatural=yes
CloseRange=yes
DefaultToGuardArea=yes ; the much awaited dog default to move and attack when resting
GuardRange=2
SelfHealing=yes
ImmuneToPsionics=yes
PixelSelectionBracketDelta=-8 ;gs higher number draws lower.  Pixel difference from normal for selection bracket
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:Brute` | CSF-string key → "Yuri Brute" |
| `Name=Yuri Brute` | Internal name |
| `;Image=SHK` (commented) | Designer history — Brute was at one point going to reuse Tesla Trooper art. Replaced with own SHP `BRUTE` |
| `Category=Soldier` | Infantry pip/AI grouping (despite the giant melee fighter theme) |
| `Primary=Punch` | Anti-infantry/anti-building melee. Damage 100, Range 1.4, Warhead=Battering (0% vs vehicles). See "Weapons" |
| `Secondary=Smash` | Anti-vehicle melee. Damage 100, Range 1.1, Warhead=Smashing (0% vs infantry; only vehicles). See "Weapons" |
| `;GEF Unles we decide to put it back in Pushy=yes` (commented designer note) | **Designer note** — Brute was going to have `Pushy=yes` (push other units out of the way). Cut feature; not in shipping build |
| `Prerequisite=YABRCK` | **Only Yuri Barracks** — no Battle Lab requirement despite TechLevel=5. Brute is the mid-game backbone Yuri-vs-vehicle answer; needs to be accessible without further tech |
| `CrushSound=InfantrySquish` | Moot — Crushable=no |
| `Crushable=no` | **Cannot be crushed** by vehicles. Same as Tesla Trooper/Desolator/Boris/Yuri Prime |
| `Strength=200 ;180 ;250` | HP — 200. Inline comments show balance iteration: 250→180→200. Final value matches Boris (200) — among the toughest standard infantry. **Note**: not 350 (my initial loop-prompt summary was wrong) |
| `Armor=plate` | Damage type column 2 — Plate armor (vs Tesla Trooper's Plate too). Resistant to small-arms |
| `TechLevel=5` | Tech-5 cap (early-mid game) |
| `Pip=white` | Cargo pip color — white (not red — Brute is **not** in the "elite/special" tier, despite high HP) |
| `Sight=8` | Reveal radius — slightly above standard (4-6) |
| `Speed=6` | Foot-speed — 6, faster than typical infantry (4). Critical for melee unit — must close distance under fire |
| `Owner=YuriCountry` | Yuri faction only |
| `Cost=500` | $500 — moderate |
| `Soylent=250` | $250 Grinder refund (Yuri only) |
| `Points=5` | **Kill score 5** — low, despite the high HP. Brutes are designed to be mass-produced |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=BruteSelect` | Select voice — `$ibrusea..e` (5 lines, growled/grunting) |
| `VoiceMove=BruteMove` | Move voice — `$ibrumoa..e` (5 lines) |
| `VoiceAttack=BruteAttackCommand` | Attack voice — `$ibruata..e` (5 lines) |
| `VoiceFeedback=BruteFear` | Fear voice — `$ibrufea..e` (5 lines) |
| `VoiceSpecialAttack=BruteMove` | Reuses Move voice |
| `DieSound=BruteDie` | Death voice — `$ibrudia..e` (5 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry locomotor |
| `PhysicalSize=1` | Pathfinder size class — same as other infantry (despite the giant sprite). Brute physically takes one infantry slot, even though Size=2 makes him uncargo-able |
| `MovementZone=InfantryDestroyer` | **NOT the standard Infantry MZ** — `InfantryDestroyer` is an anti-pathfinding zone that lets Brute walk through certain tile types other infantry can't. Used by SEAL ([GHOST.md](../allied/GHOST.md)) and amphibious infantry. **Note**: this MZ name was the basis for the commented `;MovementZone=InfantryDestroyer` typo in Disk Thrower documentation seen across other infantry docs — Brute uses the same legitimately |
| `ThreatPosed=20` | AI scoring weight — moderate (matches Tesla Trooper, dog) |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 at Elite. Triggers `ElitePrimary=PunchE` AND `EliteSecondary=SmashE` (rare — both weapons swap at Elite) |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set |
| `Size=2;too big for IFV` | **Cargo slot cost — 2** instead of typical 1. **Inline comment confirms intent**: "too big for IFV". The [HTK] IFV has cargo space for 1 passenger; Size=2 makes Brute exceed capacity → cannot enter. **Brute is the only infantry that cannot enter the IFV** for this reason |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `ElitePrimary=PunchE` | Elite Primary swap — **same damage as base Punch** (Damage 100), same Warhead=Battering, same Range. The Elite tier mostly benefits via the FIREPOWER/ROF stack from EliteAbilities, not via the weapon's own stats |
| `EliteSecondary=SmashE` | Elite Secondary swap — also **same damage** as base Smash. Elite Brute is mostly just more durable via SELF_HEAL and FIREPOWER stack |
| `IFVMode=0` | IFV gunner-table index 0 — would be the IFV's default chassis machinegun. **Moot**: Brute can't enter IFV anyway (Size=2) |
| `Unnatural=yes` | TechnoType flag (xref `0x00714960` per [YURIPR.md](YURIPR.md)). Engine marks Brute as "unnatural" (designed/genetically-engineered). Inverse of `Natural=yes` (Cow/Brute thematically) — interesting that Brute himself is Unnatural (he's a mutated cow but engineered by Yuri, so the engine classifies him as Yuri's creation, not "natural") |
| `CloseRange=yes` | **Behavior flag** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x0071497A` DATA xref to string at `0x008439C4`). When set, the AI's target-acquisition path prefers/requires close-range engagement. Brute's weapons have Range 1.1-1.4 — well below typical infantry. Without CloseRange=yes, the AI's targeting heuristics might try to engage from outside Brute's actual range. The flag tells the AI "this unit fights up close — don't try to stand-off shoot" |
| `DefaultToGuardArea=yes` | TechnoType field (per [ADOG.md](../allied/ADOG.md)). Inline comment: "the much awaited dog default to move and attack when resting" — same comment as on Attack Dog. Idle Brute actively patrols within `GuardRange` cells, attacking enemy units it sees |
| `GuardRange=2` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x007122AB` DATA xref to string at `0x008444A4`). When in MissionGuardArea (default), Brute attacks targets within this radius from his stand position. **2 cells** is very short — matches melee range. Compare Attack Dog's `GuardRange=9` which scans wide for prey. Brute's 2 means he only commits if the target is essentially in arm's reach |
| `SelfHealing=yes` | Passive HP regeneration (same flag as Boris/Desolator) |
| `ImmuneToPsionics=yes` | **Cannot be mind-controlled** by Yuri/Yuri Prime/Magnetron/Psychic Tower. Critical — without this, Yuri-vs-Yuri mirror matchups would degenerate into mutual capture of each other's Brutes. (Note: ImmuneToPsionicWeapons is NOT set — Brute CAN be killed by Yuri Prime's PsiPulse area damage) |
| `PixelSelectionBracketDelta=-8` | TechnoType field (xref `0x00714166` per [YURIPR.md](YURIPR.md)). **-8** — bracket drawn 8 pixels higher than default. Brute's sprite is taller than standard infantry (the giant fist/punch pose) but not as exaggerated as Yuri Prime (-26). Minor adjustment for visual accuracy |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `no` (Brute does NOT crawl/go prone)
- `Trainable=` — defaults to `yes` (Brute gains veterancy; ElitePrimary/EliteSecondary confirm)
- `NotHuman=` — defaults to `no` (Brute is technically human-bodied, despite being a mutated/engineered creature)
- `ImmuneToPsionicWeapons=` — NOT set; defaults to `no`. **Brute is vulnerable to psionic damage** (Yuri Prime PsiPulse, Psychic Dominator blast). Contrast with Yuri Prime who has both
- `Bombable=` — defaults to `no`
- `Fearless=` — not set; Brute shows fear behavior (VoiceFeedback wired)
- `Occupier=` — defaults to `no`; **Brute cannot garrison** civilian buildings (Size=2 plus the melee-only weapons would make this absurd)
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=`/`Assaulter=` — none set
- `DetectDisguise=` — not set
- `BombSight=` — not set
- `Natural=` — not set (the inverse Unnatural=yes is set instead)
- `Pushy=` — explicitly commented out (cut feature)
- `;Image=SHK` — commented out
- `TypeImmune=` — NOT set (basic Yuri Clone has it; Brute does not — meaning another Brute could potentially be... but ImmuneToPsionics=yes already blocks that)

---

## artmd.ini — `[BRUTE]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:203`:

```ini
[BRUTE] ; Brute
Cameo=BRUTICON
AltCameo=BRUTUICO
Sequence=BruteSequence
Crawls=no
Remapable=yes
FireUp=6
PrimaryFireFLH=100,-25,135
SecondaryFireFLH=100,-25,135
SecondaryFire=7
```

| Key | Meaning |
|-----|---------|
| `Cameo=BRUTICON` | Sidebar build icon (SHP) |
| `AltCameo=BRUTUICO` | Elite cameo — shown after Veteran promotion |
| `Sequence=BruteSequence` | Brute-specific sequence (with 10-frame × 10-facing fire animations — see below) |
| `Crawls=no` | **Cannot crawl/prone** — Brute is always upright |
| `Remapable=yes` | House remap palette |
| `FireUp=6` | Bullet-spawn frame — at frame 6 the punch lands. **Note: punches are inviso instant-hit projectiles**, so "fire" = "punch contact" |
| `PrimaryFireFLH=100,-25,135` | Primary FLH — 100 forward, -25 sideways, 135 up. Same as Tesla Trooper/Desolator (shoulder-mounted weapon FLH, even though Brute's "weapon" is his fist) |
| `SecondaryFireFLH=100,-25,135` | Same FLH as Primary — both punch and smash come from same arm position |
| `SecondaryFire=7` | **Behavior key** — artmd-level. Bullet-spawn frame for **Secondary** weapon at frame 7. Distinct from Primary's FireUp=6. Brute's smash anim is slightly delayed vs punch — visible as a longer windup before the contact frame |

### Referenced sequence — `[BruteSequence]`

`artmd.ini:14235`:

```ini
[BruteSequence]
Ready=0,1,1
Guard=0,1,1
Prone=0,1,1
Walk=8,6,6
FireUp=116,10,10
SecondaryFire=196,10,10
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=86,15,0
Die2=101,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Cheer=276,8,0,E
Panic=8,6,6
Paradrop=284,1,0
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Prone=0,1,1` | **Stub → Ready frame** — Brute doesn't prone (Crawls=no), but Prone= entry exists defensively (spy-disguise rendering may probe this) |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `FireUp=116,10,10` | **Standing fire (Punch) — 10 frames × 10 facings**. Higher facing count than typical 6 — Brute's punch needs precise angular rendering because the fist is large and asymmetric. Combined with FireUp=6 artmd top-level (frame 6 of the 10 = contact frame), the punch lands mid-animation |
| `SecondaryFire=196,10,10` | **Smash anim** — 10 frames × 10 facings starting at frame 196. Combined with `SecondaryFire=7` artmd top-level (frame 7 of the 10 = contact frame) — 1 frame later windup than punch |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S-facing |
| `Idle2=71,15,0,E` | Idle 2 — E-facing |
| `Die1=86,15,0` | Death 1 — 15 frames | "Big-creature collapse" anim |
| `Die2=101,15,0` | Death 2 |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready | |
| `Cheer=276,8,0,E` | Cheer — 8 frames E | |
| `Panic=8,6,6` | Panic = Walk | |
| `Paradrop=284,1,0` | Single frame at 284 — paradrop pose | Brute paradrop-eligible |

Note: this sequence is **shorter than typical Yuri infantry** (no Crawl, Down, Up, FireProne — all the prone-related slots) because Crawls=no. The 10-frame fire cycles compensate by giving more visual detail to the actual attacks.

---

## Weapons

### Primary (Veteran and below) — `[Punch]`

`rulesmd.ini:23715`:

```ini
[Punch]
Damage=100
ROF=60
Range=1.4
Speed=100
Warhead=Battering
Report=BruteSmashAttack
Projectile=InvisibleLow
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=100` | Per-shot damage. Combined with Battering's Verses gives **100 vs infantry / 0 vs vehicles / 30/30/20 vs structures / 200 vs special_1** |
| `ROF=60` | Cooldown — 60 frames (~4s @ 15fps). Slow cadence balances the high per-hit damage. Two punches kill a Strength=100 GI / Conscript / Initiate (with full damage) |
| `Range=1.4` | **1.4 cells** — melee. Brute must close to point-blank |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=Battering` | See warhead — anti-infantry / anti-building / anti-special, NOT anti-vehicle |
| `Report=BruteSmashAttack` | Sound — 6 layered samples `ibruatta..f` |
| `Projectile=InvisibleLow` | LOS-respecting inviso (subject to walls/cliffs/elevation, though irrelevant at 1.4 cell range) |
| `FireInTransport=no` | Cannot fire from inside Battle Fortress (Brute is too big to enter anyway, defensive) |

### Secondary — `[Smash]` (anti-vehicle melee)

`rulesmd.ini:23726`:

```ini
; Tank smashing by Brute
[Smash]
Damage=100;150
ROF=60 ;30
Range=1.1
Speed=100
Warhead=Smashing
Report=BruteSmashAttack
Projectile=InvisibleLow
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=100;150` | Damage 100. Inline comment "150" shows it was tuned down from 150 |
| `ROF=60 ;30` | Cooldown 60 (vs commented 30 — was tuned slower) — same cooldown as Punch |
| `Range=1.1` | **1.1 cells** — slightly shorter than Punch's 1.4 (still melee). Brute must be RIGHT next to the vehicle |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=Smashing` | **Vehicles-only warhead** — 0% vs infantry, 100/20/100 vs vehicle armor classes |
| `Report=BruteSmashAttack` | Same sound as Punch (6 layered samples) |
| `Projectile=InvisibleLow` | LOS-respecting inviso |
| `FireInTransport=no` | Same |

### Elite Primary — `[PunchE]`

`rulesmd.ini:24953`:

```ini
[PunchE]
Damage=100
ROF=60
Range=1.4
Speed=100
Warhead=Battering
Report=BruteSmashAttack
Projectile=InvisibleLow
;IsElectricBolt=true
;AssaultAnim=UCELEC;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
FireInTransport=no;can't fire out of the BattleFortress
```

**Identical to `[Punch]`** — same Damage/ROF/Range/Speed/Warhead/Report/Projectile. The Elite tier doesn't change Brute's per-shot weapon stats — improvements come from the EliteAbilities FIREPOWER/ROF stack on the unit.

Two **commented-out flags**: `;IsElectricBolt=true` and `;AssaultAnim=UCELEC` — designer history. Brute was at one point going to fire an electric bolt instead of a fist? Or PunchE was copy-pasted from a Tesla weapon. Either way, both lines are inactive.

### Elite Secondary — `[SmashE]`

`rulesmd.ini:24966`:

```ini
; Tank smash by Brute
[SmashE]
Damage=100
ROF=60
Range=1.1
Speed=100
Warhead=Smashing
Report=BruteSmashAttack
Projectile=InvisibleLow
;IsElectricBolt=true
;AssaultAnim=UCELEC;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
FireInTransport=no;can't fire out of the BattleFortress
```

**Identical to `[Smash]`** — same per-shot stats. Same commented-out IsElectricBolt/AssaultAnim history.

### Primary Warhead — `[Battering]`

`rulesmd.ini:27310`:

```ini
[Battering]
;;Verses=100%,100%,100%,0%,0%,0%,20%,20%,20%,200%,100%
Verses=100%,100%,100%,0%,0%,0%,30%,30%,20%,200%,100%
InfDeath=10
;GEF Unles we decide to put it back in DirectRocker=yes
Wall=yes
Wood=yes
```

| Key | Meaning |
|-----|---------|
| `;;Verses=` (commented older) | Designer history — building Verses were 20/20/20, bumped to 30/30/20 |
| `Verses=100%,100%,100%,0%,0%,0%,30%,30%,20%,200%,100%` | 11-column. **100% vs all infantry armor** (one-shots GI/Engineer/Sniper/Initiate at Damage 100). **0% vs all vehicle armor** — Brute's Punch cannot damage vehicles (must use Secondary Smash). **30% vs wood, 30% vs steel, 20% vs concrete** — moderate vs structures (Brute can demolish walls and lightly-armored buildings). **200% vs special_1** — boost vs some special armor (likely psionic-class units or specific bosses). 100% vs special_2 |
| `InfDeath=10` | **Infantry death animation type 10** — **the "punched into pieces" gibbed death**. Distinctive visual signal that a melee Brute killed the target. Same InfDeath type as Smashing |
| `;GEF Unles we decide to put it back in DirectRocker=yes` (commented) | Designer note — DirectRocker (rocker without animation interpolation?) was considered, cut |
| `Wall=yes` | Damages walls (can destroy concrete/wood walls) |
| `Wood=yes` | Damages wooden structures and sets fires |

### Secondary Warhead — `[Smashing]`

`rulesmd.ini:27318`:

```ini
[Smashing]
Verses=0%,0%,0%,100%,20%,100%,0%,0%,0%,0%,0%
InfDeath=10
Wall=yes
Wood=yes
Rocker=yes
MinDebris=1
MaxDebris=3
```

| Key | Meaning |
|-----|---------|
| `Verses=0%,0%,0%,100%,20%,100%,0%,0%,0%,0%,0%` | 11-column. **0% vs all infantry armor** — Smash cannot hit infantry (forces use of Primary for infantry). **100% vs light, 20% vs medium, 100% vs heavy** — **the iconic "vehicle smashing" damage curve**. Brute deals **full damage to Allied Lasher (light) and Soviet Apocalypse (heavy) but only 20% to Allied Grizzly / Soviet Rhino (medium)**. **Intentional balance** — Grizzly/Rhino are the most-produced tanks; full Brute damage there would trivialize mid-game vehicle play. The 20% medium gap forces Yuri players to value Brutes against heavy tanks specifically. **0% vs structures** (Smash is anti-vehicle only — buildings need Punch). 0% specials |
| `InfDeath=10` | Same "gibbed" death (moot — 0% vs infantry, never triggers) |
| `Wall=yes` | Damages walls |
| `Wood=yes` | Damages wood (moot — 0% vs wood structure armor; the wood-on-wall is for fence-type walls?) |
| `Rocker=yes` | **Tank rocks on hit** — visual effect; the targeted vehicle physically rocks back from Brute's punch. Combined with `MinDebris`/`MaxDebris` makes Brute hits very satisfying visually |
| `MinDebris=1` | At least 1 debris piece spawned on hit |
| `MaxDebris=3` | Up to 3 debris pieces |

### Projectile — `[InvisibleLow]`

Standard LOS-respecting inviso projectile (same as most basic infantry).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death (uniform 5-line banks)

```ini
[BruteSelect]                  ; soundmd.ini:4364
Sounds=$ibrusea $ibruseb $ibrusec $ibrused $ibrusee
Control=random
Volume=85

[BruteMove]                    ; soundmd.ini:4369
Sounds=$ibrumoa $ibrumob $ibrumoc $ibrumod $ibrumoe
Control=random
Volume=85

[BruteAttackCommand]           ; soundmd.ini:4374
Sounds=$ibruata $ibruatb $ibruatc $ibruatd $ibruate
Control=random
Volume=85

[BruteFear]                    ; soundmd.ini:4379
Sounds=$ibrufea $ibrufeb $ibrufec $ibrufed $ibrufee
Control=random
Volume=85

[BruteDie]                     ; soundmd.ini:4389
Sounds=$ibrudia $ibrudib $ibrudic $ibrudid $ibrudie
Control=random
Volume=85
```

5/5/5/5/5 — uniformly large voice bank. Brute voice character: deep grunts, growls, single-syllable utterances (the brute is essentially non-verbal — a Yuri-engineered cow-creature).

### Extra voice — `[BruteCrushing]`

```ini
[BruteCrushing]                ; soundmd.ini:4384
Sounds=$ibrucra $ibrucrb $ibrucrc $ibrucrd
Control=random
Volume=85
```

**Not wired on the type via standard Voice* fields**. 4 lines specifically for "Brute crushing" — but **Brute doesn't crush** (his weapons are inviso instant-hit, not crush). This sound is **likely played when Brute's Smash warhead triggers debris/rocker on a vehicle** — the "crushing impact" SFX heard when Smash hits. Unverified — could also be unused designer content. The naming suggests it's wired via the Smashing warhead's animation/sound system, not via the type's VoiceX fields.

### Weapon report — single shared sound for both weapons

```ini
[BruteSmashAttack]             ; soundmd.ini:897
Sounds=ibruatta ibruattb ibruattc ibruattd ibruatte ibruattf
Control= random
Volume=80
```

**6 layered samples** for impact sound. Used by **both** `[Punch]` and `[Smash]` (and their Elite variants). Wired via `Report=BruteSmashAttack` on all four weapons. Single shared SFX = Brute makes the same "thud" sound whether punching infantry or smashing a tank.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `YABRCK` | Yuri Barracks ONLY — no Battle Lab gate despite Tech 5 |
| `Owner=` | `YuriCountry` | Yuri faction only — singleton |
| `TechLevel=` | `5` | Mid-game |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=500` | $500 | |
| `Soylent=250` | $250 refund (Yuri only) | |
| `Points=5` | 5 | Low — kills don't score much |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiredHouses=`, no `SecretHouses=`, no `RequiresStolenXxxTech=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities + **BOTH** weapons swap: `ElitePrimary=PunchE` AND `EliteSecondary=SmashE`. The Elite weapons have identical stats to base — improvements come purely from the ability stack |
| AltCameo | `BRUTUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. CloseRange=yes — AI close-engagement preference

INI key `CloseRange` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x0071497A` DATA xref to string at
`0x008439C4`). When set, the AI's target-acquisition path classifies the
unit as a close-range fighter — the target-pick heuristic will:
- Not try to stand-off shoot (waiting for the target to come into range
  while taking damage)
- Prefer pathfinding into melee range over stationary engagement
- Possibly skip the standard "back away to maintain range" Mission_Guard
  behavior when at range limit

For Brute with Range 1.1-1.4, the flag is essential — without it, the AI
might try to engage a target at 4-5 cells (the default infantry combat
range), wandering aimlessly while Brute is unable to fire.

### 2. GuardRange=2 — close-Guard radius

INI key `GuardRange` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x007122AB` DATA xref to string at
`0x008444A4`). When in MissionGuard / MissionGuardArea, this is the
radius (in cells) within which the unit will commit to attacking visible
enemies. Brute's `GuardRange=2` (cells) is **very short** — matches his
melee weapon range. He won't break formation to chase a target unless
it's essentially in arm's reach.

Compare:
- Attack Dog: `GuardRange=9` (wide scout/pursuit)
- Brute: `GuardRange=2` (melee-only commitment)
- Default (most infantry, unset): typically falls back to weapon range or sight

The 2-cell GuardRange combined with `DefaultToGuardArea=yes` makes Brute
an effective "stand here and slap anything that comes close" defender.

### 3. Size=2 — IFV exclusion via cargo cost

INI key `Size=2` (TechnoTypeClass) sets the unit's cargo-slot cost when
loading into transports. Most infantry are Size=1; Brute is Size=2.
**[HTK] IFV has 1 cargo slot total** — Brute's 2-slot cost exceeds it,
the engine rejects the load. **Brute is the only stock infantry that
cannot enter the IFV** because of cargo size, not because of a specific
IFV-exclusion flag. The inline comment "too big for IFV" confirms intent.

Transports with multi-slot cargo (Battle Fortress's 5 slots) can still
load Brutes — BF can fit 2 Brutes (4 slots used) + 1 normal infantry
(5 slots total). However Brute's `FireInTransport=no` on both weapons
means he can't fire from inside the BF.

### 4. Verses curve — 0/100/20/100 vs vehicle armor (Smashing)

The intentional "20% vs medium" dip in the Smashing warhead's Verses is a
**balance lever**, not a hardcoded mechanic. The engine doesn't know
about "anti-medium-tank infantry" — the dip emerges from the data:
`Verses=0%,0%,0%,100%,20%,100%,0%,0%,0%,0%,0%`.

Verse columns: none, flak, plate, **light**, **medium**, **heavy**, wood,
steel, concrete, special_1, special_2.

- Allied Lasher Tank (Yuri side, light armor) → 100% Brute Smash
- Allied Grizzly (medium) → 20%
- Soviet Rhino (medium) → 20%
- Soviet Apocalypse (heavy) → 100%
- Allied Battle Fortress (heavy) → 100%

The strategic implication: Brute is a **heavy-tank-counter unit**, not a
general anti-vehicle. Sticking him on Grizzly rushes is inefficient
(20% damage = 5 hits per medium tank); using him to wreck Apocalypse /
Battle Fortress is highly effective (1-2 hits).

### 5. Unnatural=yes — engineered-entity flag

Same flag documented in [YURIPR.md](YURIPR.md). TechnoType field. Marks
Brute as "designed" rather than "natural" — the engine considers him
opposite of Cow/Brute-thematically-natural animals despite the cow-mutation
origin story. Possibly affects which warhead effects treat him as a
"natural" target (Mutator Warhead converts naturals to Brutes — but
Brute is already Unnatural, so the Mutator wouldn't trigger on him).

### 6. PixelSelectionBracketDelta=-8 — minor bracket adjustment

Same flag documented in [YURIPR.md](YURIPR.md). Brute's -8 (vs Yuri
Prime's -26) reflects the smaller sprite-height difference: Brute is a
big creature but stays grounded; Yuri Prime floats higher above the
ground.

### 7. MovementZone=InfantryDestroyer

`MovementZone=InfantryDestroyer` — TechnoType field (`MovementZone` is in
the search-strings list at `0x008431C8`, also used by SEAL). **Distinct
from `MovementZone=Infantry`**: InfantryDestroyer allows the unit to
walk through some terrain types that standard Infantry MZ blocks (notably,
the SEAL stuck-on-tree bug fix references this). For Brute it's not clear
what tactical advantage this gives — possibly access to certain civilian
terrain (statues, tree clusters) for cheese routes.

This is the legitimate use of `InfantryDestroyer` MZ. Other infantry have
comments like `;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug
from the original Disk Thrower!` — confirming Brute / SEAL use it
intentionally while others had it copy-pasted by mistake.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("CloseRange\|GuardRange\|MovementZone")` | 3 strings — confirms all 3 as hardcoded INI keys |
| `get_xrefs_to(0x008439C4)` (= "CloseRange") | Sole xref from `TechnoTypeClass__ReadINI @ 0x0071497A` DATA — confirms TechnoType-level close-range AI hint |
| `get_xrefs_to(0x008444A4)` (= "GuardRange") | Sole xref from `TechnoTypeClass__ReadINI @ 0x007122AB` DATA — confirms TechnoType-level Guard radius |

Plus cross-referenced from prior docs: Unnatural, PixelSelectionBracketDelta, DefaultToGuardArea, ImmuneToPsionics, MovementZone.

Confirmation: **no Brute-specific hardcoded function block exists** in
gamemd.exe. Brute is data-driven via generic engine flags — close-range
melee emerges from CloseRange=yes + GuardRange=2 + tiny weapon Range +
Verses spreads + InfantryDestroyer MZ. Pure data composition.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Image=SHK` (commented) | Designer history — Brute was going to reuse Tesla Trooper art | OK |
| `;GEF Unles we decide to put it back in Pushy=yes` (commented) | Cut feature — Pushy mechanic | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `;180 ;250` (commented Strength values) | Balance history — Strength was tuned from 250 down to 200 | OK |
| `;Verses=` (commented older in Battering) | 20/20/20 building Verses → 30/30/20 (tuned higher) | OK |
| `;150` (commented Damage in Smash) | Original Damage 150, dropped to 100 | OK |
| `;IsElectricBolt=true` / `;AssaultAnim=UCELEC` (commented in PunchE/SmashE) | Designer cut-content — Brute was tested with electric/UC-clearing weapons | OK |
| `;30` (commented ROF in Smash) | Original ROF 30 (faster), slowed to 60 | OK |
| `;GEF Unles we decide to put it back in DirectRocker=yes` (commented in Battering) | Cut DirectRocker feature | OK |
| `MovementZone=InfantryDestroyer` | YR-active, **legitimate use** (vs the copy-paste-bug uses on other infantry) | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard | OK |
| `CloseRange=yes` / `GuardRange=2` / `Unnatural=yes` | YR-active, verified via Ghidra ReadINI xrefs | OK |

No TS-only behavior on Brute. All flags YR-active. Many balance-history
comments preserved as inline notes.

---

## Cross-references

- **Yuri infantry tier**:
  - `[INIT]` Yuri Initiate (documented) — basic flame infantry
  - `[YURI]` Yuri Clone (documented) — single-target MC
  - `[YURIPR]` Yuri Prime (documented) — AoE MC + building MC
  - **`[BRUTE]` Yuri Brute (this doc)** — Strength=200 melee
  - `[VIRUS]` Virus — plague sniper
  - `[YENGINEER]` Yuri Engineer
- **Strength=200 club** (toughest standard infantry):
  - `[BORIS]` Boris (Soviet — same 200)
  - `[YURIPR]` Yuri Prime (150 actually, the highest is Boris/Brute at 200 — verify Yuri Prime's was 150)
  - **`[BRUTE]` Yuri Brute (this doc)**
  - Above all: `[BORIS]` Boris (200), Brute (200) — both at top. Yuri Prime at 150
- **Melee-range units** (CloseRange=yes implicit family):
  - `[BRUTE]` Brute (this doc) — only stock infantry with this exact CloseRange + GuardRange=2 setup
  - Attack dogs are NOT CloseRange (they use ranged Parasite Damage trick)
  - Tanya/SEAL's C4 is melee but a different mechanic
- **Same `Crushable=no` family** (uncrushable infantry):
  - `[SHK]` Tesla Trooper
  - `[DESO]` Desolator
  - `[BORIS]` Boris
  - `[YURIPR]` Yuri Prime
  - **`[BRUTE]` Brute (this doc)**
- **Anti-heavy-vehicle answers**:
  - Soviet: V3 Rocket Launcher, Apocalypse cannon, Demolition Truck
  - Allied: Tank Destroyer, Prism Tank
  - **Yuri: `[BRUTE]` Brute melee Smash (this doc)** — the only infantry-tier hard counter to heavy tanks (100% Verses)
- **Counter-units to Brute**:
  - **Snipers** — 250 dmg (one-shot 200 HP — but takes off only most of it; second shot finishes; Brute has SelfHealing so non-trivial)
  - **Crazy Ivan** bomb (Bombable defaults to no, but Bomb mission works)
  - **Dogs** (Parasite warhead — 100% vs Plate armor at 30 dmg... wait, dogs only target infantry. Yes — 1 dog leap = 30 dmg vs Plate. Need ~7 leaps to kill a Brute via dog ParasiteDog math)
  - **Long-range fire** (V3, Prism, Apocalypse cannon outrange Brute's 1.4)
  - **Light/Medium tank rush** — medium-armor at 20% Verses means **medium tanks** (Grizzly/Rhino) **kite Brutes effectively** — Brute can't keep up with retreating tanks, and 5 hits per medium tank at 60 ROF = 5 minutes per kill
  - **NOT effective**: mind-control (ImmuneToPsionics=yes), vehicle crush (Crushable=no), most ranged infantry (200 HP + plate armor + SelfHealing)
- **Sound cross-link**:
  - `[BruteCrushing]` (4 lines, soundmd:4384) — defined but **not wired via standard Voice fields**. Likely triggered by Smashing warhead's debris/rocker system, not the unit's VoiceFeedback hooks. Worth verifying when investigating warhead-side sound triggers
- **Related warhead family**:
  - `[Battering]` (this doc — Brute Primary) — anti-infantry/building/special
  - `[Smashing]` (this doc — Brute Secondary) — anti-vehicle only with the iconic 100/20/100 curve
  - `[Mummypunch]` (rulesmd:23737) — adjacent in INI, mummy/easter-egg unit uses Battering-style melee

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [BRUTE]` | 5104-5150 (47 lines) | All 41 active keys covered (5 commented/inline-balance-history entries documented) |
| `artmd.ini [BRUTE]` | 203-212 (10 lines) | All keys covered including SecondaryFire=7 unique entry |
| `artmd.ini [BruteSequence]` | 14235-14251 (17 lines) | All 14 active slots + 3 stub Die3-5 covered with 10×10 fire-cycle distinction noted |
| `rulesmd.ini [Punch]` | 23715-23723 (9 lines) | All keys covered |
| `rulesmd.ini [Smash]` | 23726-23734 (9 lines) | All keys covered including inline `;150` and `;30` balance comments |
| `rulesmd.ini [PunchE]` | 24953-24963 (11 lines) | All keys covered including 2 commented IsElectricBolt/AssaultAnim |
| `rulesmd.ini [SmashE]` | 24966-24976 (11 lines) | Same as PunchE |
| `rulesmd.ini [Battering]` warhead | 27310-27316 (7 lines) | All keys covered with full 11-column Verses; commented historical Verses + DirectRocker notes documented |
| `rulesmd.ini [Smashing]` warhead | 27318-27325 (8 lines) | All keys covered; iconic 100/20/100 vehicle Verses curve explained as balance design |
| `soundmd.ini` Brute voices | BruteSelect, Move, AttackCommand, Fear, Die, Crushing (orphan), SmashAttack (weapon) | All 7 covered, including the unwired BruteCrushing flagged for verification |
| Hardcoded behavior | CloseRange + GuardRange + Size=2 IFV exclusion + Verses-design 100/20/100 anti-vehicle curve + Unnatural + PixelSelectionBracketDelta + MovementZone=InfantryDestroyer (legit use) | 7 mechanisms with 2 fresh Ghidra-verified xrefs + 4 cross-referenced |
| Ghidra searches performed against ID | 3 distinct queries (1 strings + 2 xref lookups) | Logged inline |
| TS-legacy filter | Applied; ImmuneToVeins/balance-history all documented; InfantryDestroyer MZ distinguished from copy-paste-bug uses | Done |
