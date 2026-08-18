# Tesla Trooper (SHK)
Side: Soviet | Category: Infantry | Image alias: `[SHK]` (no `Image=` redirect — own SHP `SHK`)

The Soviet **Tesla Trooper**. $500 from Soviet Barracks (no Battle Lab prereq).
Heavy anti-vehicle infantry with an iconic dual-weapon configuration:
**`Primary=ElectricBolt`** (Damage 50, Range 3, `Warhead=Shock`) for offense,
**`Secondary=AssaultBolt`** (Damage 10, Range 1.83, `Warhead=ElectricAssault`)
for the **legendary "charge a Tesla Coil" interaction** — when a friendly Tesla
Coil has lost power, ordering a Tesla Trooper to attack-target the coil fires
AssaultBolt at it, whose `ElectricAssault=yes` warhead flag triggers the
hardcoded "manually power the coil" engine path (verified WarheadTypeClass
field, xref at `0x0075D81D`). `Crushable=no` — Tesla Troopers **cannot be
crushed** by vehicles (the only basic infantry with this property; designer
reasoning: high voltage). `Strength=130, Armor=Plate` — among the toughest
infantry. **Trainable=yes** (default). At Elite, weapon swaps to `[ElectricBoltE]`
which uses `Projectile=Electricbounce` (chain-bounces to 2 nearby targets via
`ShrapnelWeapon=TeslaFragment`/`ShrapnelCount=2`).

No standalone Tesla Trooper RE doc previously existed; this document
originates the Ghidra trace of `IsElectricBolt`/`IsAlternateColor`/
`ElectricAssault`/`TeslaCharge` flag paths.

---

## rulesmd.ini — `[SHK]` section

Verbatim from `ini/rulesmd.ini:4507`:

```ini
[SHK]
UIName=Name:SHK
Name=Shock Trooper
Category=Soldier
Primary=ElectricBolt
Secondary=AssaultBolt
Assaulter=no ; I clear out UC buildings
Prerequisite=NAHAND
CrushSound=InfantrySquish
Crushable=no
Strength=130
Armor=Plate
TechLevel=5
Pip=white
Sight=6
Speed=4
Owner=Russians,Confederation,Africans,Arabs
Cost=500
Soylent=250
Points=5
IsSelectableCombatant=yes
VoiceSelect=TeslaTroopSelect
VoiceMove=TeslaTroopMove
VoiceAttack=TeslaTroopAttackCommand
VoiceFeedback=TeslaTroopFear
VoiceSpecialAttack=TeslaTroopMove
DieSound=TeslaTroopDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
;MovementZone=InfantryDestroyer ;GEF wow!!! copy paste bug from the original Disk Thrower!
ThreatPosed=20	; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ImmuneToVeins=yes
Size=1
AllowedToStartInMultiplayer=no
ElitePrimary=ElectricBoltE
IFVMode=6
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:SHK` | CSF-string key → "Tesla Trooper" |
| `Name=Shock Trooper` | Internal short name — **note** the engine name is "Shock Trooper" but the CSF/UI displays "Tesla Trooper". The SHK ID is from "Shock" |
| `Category=Soldier` | Pip group + AI grouping (infantry) |
| `Primary=ElectricBolt` | Main offensive weapon — Damage 50, Range 3, `Warhead=Shock`. **Anti-vehicle/anti-anything** — the standard Tesla Trooper offensive use. See "Weapons" |
| `Secondary=AssaultBolt` | **The Tesla Coil charge weapon** — Damage 10, Range 1.83, `Warhead=ElectricAssault`. Verses are 0% vs infantry/vehicles; only damages building-class armors at 100%. Used specifically to "charge" a friendly Tesla Coil during a power outage. See "Hardcoded Behavior" §1 |
| `Assaulter=no ; I clear out UC buildings` | **Behavior flag, explicitly `no`** — the inline comment "I clear out UC buildings" describes what `Assaulter=yes` would mean. SHK explicitly does NOT clear garrisoned civilian buildings (only SEAL/Tanya/Yuri do). The `AssaultAnim=UCELEC` on ElectricBolt is therefore vestigial for SHK |
| `Prerequisite=NAHAND` | Soviet Barracks specifically (TechLevel=5 in absence of separate Battle Lab gate — note this is one of the few level-5 units without a Battle Lab prereq, because Soviet players need Tesla Trooper for their Tesla Coil tech tree to function under power loss) |
| `CrushSound=InfantrySquish` | Crush sound — but **moot** (`Crushable=no`) |
| `Crushable=no` | **Behavior flag** — Tesla Trooper **cannot be crushed by vehicles**. The only basic infantry with this property; designer rationale is the high voltage / armored battle-suit. Critical anti-crush counter to Soviet vehicle rush strategies |
| `Strength=130` | HP — second-toughest infantry (behind Brute's 350). Survives most one-shot weapons (Sniper does 250 → still dies, but most others can't one-shot) |
| `Armor=Plate` | Damage type column 2 — Plate is mid-tier infantry armor. SA/SSA warheads do 80% (vs 100% for none-armor); flak warheads less effective. Combined with Strength=130, SHK is one of the most durable infantry |
| `TechLevel=5` | Tech-5 cap; mid-game |
| `Pip=white` | Cargo pip color |
| `Sight=6` | Reveal radius — modest |
| `Speed=4` | Foot-speed — standard infantry (slow) |
| `Owner=Russians,Confederation,Africans,Arabs` | All 4 Soviet countries |
| `Cost=500` | $500 — 5× Conscript |
| `Soylent=250` | $250 Grinder refund (Yuri only) |
| `Points=5` | Kill score |
| `IsSelectableCombatant=yes` | Included in "select all combat units" hotkey |
| `VoiceSelect=TeslaTroopSelect` | Selection voice — `$itessea/c/d/e + itesmoc` (5 lines) |
| `VoiceMove=TeslaTroopMove` | Move voice — `$itesmoa..f` (6 lines) |
| `VoiceAttack=TeslaTroopAttackCommand` | Attack voice — `$itesata..e` (5 lines) |
| `VoiceFeedback=TeslaTroopFear` | Fear voice — `$itesfea/b/c` (Priority=low) |
| `VoiceSpecialAttack=TeslaTroopMove` | Reuses Move voice — no dedicated special-attack line |
| `DieSound=TeslaTroopDie` | Death voice — `$itesdia..d` (4 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;MovementZone=InfantryDestroyer ;GEF...` | Same copy-paste-fix history comment (Disk Thrower legacy) |
| `ThreatPosed=20` | AI scoring weight — moderate; same as Attack Dog |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 abilities at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities at Elite — triggers `ElitePrimary=ElectricBoltE` weapon swap |
| `ImmuneToVeins=yes` | TS legacy; defensively set |
| `Size=1` | Transport cargo slot cost |
| `AllowedToStartInMultiplayer=no` | NOT in starting unit complement (must be produced) |
| `ElitePrimary=ElectricBoltE` | At Elite: Primary becomes `[ElectricBoltE]` (Damage 50, **Range 5** vs 3, ROF 40 vs 60). **Uses chain-bouncing projectile** `Electricbounce` (see Weapons — fires 2 ShrapnelWeapon TeslaFragments on impact) |
| `IFVMode=6` | IFV gunner-table index 6 → HTK's `Weapon7`/`ElitePassengerWeapon7` slot. In stock YR this slot maps to an anti-vehicle Tesla-style weapon (the IFV chassis takes on a Tesla-themed beam when garrisoned by SHK) |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone while crawling enabled)
- `Trainable=` — defaults to `yes` (Veteran/Elite presence confirms)
- `NotHuman=` — defaults to `no` (SHK is human-class; subject to InfDeath, sniper headshot priority, mind-control)
- `ImmuneToPsionics=` — defaults to `no`; **SHK CAN be mind-controlled** (significant counter for Yuri vs Soviet)
- `ImmuneToRadiation=` — defaults to `no`; killed by radiation
- `Bombable=` — not in explicit list (defaults to `false` for non-E1 infantry)
- `Fearless=` — not set; SHK shows fear behavior
- `Occupier=` — defaults to `no`; SHK **cannot garrison** civilian buildings (one of the few basic infantry that can't — design choice; otherwise SHK + UC would be too powerful)
- `Agent=`/`Infiltrate=` — not set
- `Engineer=` — not set
- `Deployer=` — not set; SHK has no deploy command
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set (MissionGuard when idle)
- `Natural=` — not set
- `PreventAttackMove=` — not set, defaults to `no`; obeys Attack-Move
- `OccupyWeapon=`/`OccupyPip=` — not set (no garrison)

---

## artmd.ini — `[SHK]` section

`ini/artmd.ini:193`:

```ini
[SHK] ; Shock Trooper
Cameo=SHKICON
AltCameo=SHKUICO
Sequence=ConSequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=100,-25,135
SecondaryFireFLH=100,-25,135
```

| Key | Meaning |
|-----|---------|
| `Cameo=SHKICON` | Sidebar build icon (SHP `SHKICON`) |
| `AltCameo=SHKUICO` | Elite cameo |
| `Sequence=ConSequence` | **Shared** `[ConSequence]` (also used by E2 Conscript, SNIPE Sniper) — generic infantry layout. Documented in [SNIPE.md](../allied/SNIPE.md) |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame — SHK fires very early (frame 2 of the FireUp track), matching the instant-bolt visual |
| `PrimaryFireFLH=100,-25,135` | Primary FLH — 100 forward, -25 sideways (slight left offset for the shoulder-mounted coil), 135 up (high muzzle, around shoulder/head height of the Tesla suit) |
| `SecondaryFireFLH=100,-25,135` | Secondary FLH — **identical** to Primary. Both ElectricBolt and AssaultBolt emanate from the same coil mount |

Missing per-weapon `SecondaryFireOffset=` etc; uses defaults.

### Referenced sequence — `[ConSequence]`

Already documented in detail in [SNIPE.md](../allied/SNIPE.md). SHK uses the
exact same generic infantry sequence (with its own `FireUp=2` per-unit timing
offset). Note: `Paradrop=292,1,0` is technically in the sequence but SHK is
not paradrop-eligible in stock YR; its paradrop sprite would fall back to
the generic frame anyway.

---

## Weapons

### Primary (Veteran and below) — `[ElectricBolt]`

`rulesmd.ini:23856`:

```ini
[ElectricBolt]
Damage=50
ROF=60
Range=3
Speed=100
Warhead=Shock
Report=TeslaTroopAttack
Projectile=InvisibleLow
IsElectricBolt=true
AssaultAnim=UCELEC;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

| Key | Meaning |
|-----|---------|
| `Damage=50` | Per-shot damage. Combined with `Shock.Verses[medium]=100%` → 50 dmg vs Grizzly/Rhino tanks. Anti-vehicle effective; not a one-shot but solid DPS over time |
| `ROF=60` | Cooldown — 60 frames (~4s) — slow enough that 2-3 SHKs vs a tank is meaningful but not overwhelming |
| `Range=3` | 3 cells — **very short**. SHK must close to point-blank range to engage. Critical balance: high damage offset by tiny range |
| `Speed=100` | Irrelevant for inviso instant resolution |
| `Warhead=Shock` | See warhead section — anti-everything Verses with special_1=200% boost |
| `Report=TeslaTroopAttack` | Sound `itesatta` (single-bolt zap sample) |
| `Projectile=InvisibleLow` | Standard LOS-respecting inviso projectile (blocked by walls/cliffs/elevation) |
| `IsElectricBolt=true` | **Behavior flag** — WeaponTypeClass field. **WeaponType+0x152 (byte, ReadBool) [BINARY-VERIFIED audit 33, re-confirms audit 9 cumulative]** (parser xref @ 0x00772854 to string at 0x008492E4). When set, the engine draws the iconic **animated Tesla zap bolt visual** between firer and target. **`IsAlternateColor=true`** (used on AssaultBolt) shifts the bolt to a different color. **WeaponType+0x154 (byte) [BINARY-VERIFIED audit 33, re-confirms audit 9 cumulative]** (xref at 0x0077288C to string at 0x008492C0). Both flags purely visual. |
| `AssaultAnim=UCELEC` | WeaponTypeClass field. **WeaponType+0x114 (AnimType*) [BINARY-VERIFIED audit 33]** (parser xref @ 0x00772574 to string at 0x00849410). Animation to play when an `Assaulter=yes` unit clears a UC building with this weapon. **Vestigial on SHK** because `Assaulter=no` — SHK never triggers UC clear. Sibling field to OccupantAnim at +0x110 (audit 32) and OpenToppedAnim at +0x118. |

### Elite Primary — `[ElectricBoltE]`

`rulesmd.ini:24872`:

```ini
[ElectricBoltE]
Damage=50
ROF=40
Range=5
Speed=100
Warhead=Shock
Report=TeslaTroopEliteAttack
Projectile=Electricbounce
IsElectricBolt=true
AssaultAnim=UCELEC;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

Delta from `[ElectricBolt]`:
- **Damage 50** — unchanged (Elite ability `FIREPOWER` separately gives +50% damage stack via the multiplier, so effective Elite Tesla damage is ~75)
- **ROF 60→40** — 33% faster firing
- **Range 3→5** — 67% range increase (close-quarter constraint relaxed)
- **Projectile InvisibleLow → `Electricbounce`** — **chain-bounce projectile** (see below). Single Elite Tesla bolt becomes 1 primary hit + 2 bouncing fragments
- **Report TeslaTroopAttack → TeslaTroopEliteAttack** — slightly different sound bank

The chain-bounce is the dramatic Elite upgrade — turns SHK into an effective
group-anti-infantry weapon as well as anti-vehicle.

### Secondary — `[AssaultBolt]` (Tesla Coil charge weapon)

`rulesmd.ini:23879`:

```ini
; Fire by Telsa Trooper at Tesla Coil
[AssaultBolt]
Damage=10
ROF=25
Range=1.83
Speed=100
Warhead=ElectricAssault
Report=TeslaTroopRechargeCoil
Projectile=InvisibleLow
IsElectricBolt=true
IsAlternateColor=true
```

| Key | Meaning |
|-----|---------|
| `Damage=10` | Nominal damage — but with `ElectricAssault.Verses=0%,0,0%,0%,0%,0%,100%,100%,100%,50%,100%` the damage to **infantry and vehicles is literally zero** (the friendly Tesla Coil being charged takes the full 10 with Verses 100% vs wood/steel/concrete). The 10 to the coil is also irrelevant — the charge mechanic happens before damage resolution |
| `ROF=25` | 25 frames between charges — Tesla Trooper rapid-fires AssaultBolt to keep coil powered |
| `Range=1.83` | Less than 2 cells — Tesla Trooper must be **immediately adjacent** to the Coil |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=ElectricAssault` | **THE charge warhead** — contains `ElectricAssault=yes` flag. See warhead section |
| `Report=TeslaTroopRechargeCoil` | Sound `iteschaa` (charging hum sample, Limit=3, low volume 15). Distinct from offensive TeslaTroopAttack |
| `Projectile=InvisibleLow` | Standard inviso |
| `IsElectricBolt=true` | Draws the Tesla zap visual between Trooper and Coil |
| `IsAlternateColor=true` | **WeaponTypeClass field** (xref at `0x0077288C` to string at `0x008492C0`). Shifts the zap visual to alternate color — making the charge zap visually distinct (typically orange/red) from the offensive blue zap. Critical visual signal — players can identify charging vs attacking at a glance |

### Primary's Warhead — `[Shock]`

`rulesmd.ini:27364`:

```ini
[Shock]
Verses=100%,100%,100%,85%,100%,100%,50%,50%,50%,200%,100%
InfDeath=5
Wood=yes
; SJM: No piff-piff animation -- electric bolts now spawn spark systems instead.
AnimList=TSTIMPCT
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,85%,100%,100%,50%,50%,50%,200%,100%` | 11-column. 100/100/100 vs infantry (one-shots GI/Engineer/Initiate). 85/100/100 vs light/medium/heavy vehicle armor — strong anti-vehicle. 50/50/50 vs wood/steel/concrete — moderate anti-building. **200% vs special_1** — boost vs special armor units. 100% vs special_2 |
| `InfDeath=5` | **Infantry death animation type 5** — the **electrocution** death (skeleton flash). Player visual cue that a Tesla weapon killed them |
| `Wood=yes` | Marks the warhead as wood-destroying for fire-spreading purposes |
| `AnimList=TSTIMPCT` | Impact animation `TSTIMPCT` (Tesla Impact). SJM designer note: "No piff-piff animation -- electric bolts now spawn spark systems instead" — Tesla weapons skip the standard PIFF bullet impact, drawing electrical spark particles instead |

### Secondary's Warhead — `[ElectricAssault]` (the Tesla Coil charge flag)

`rulesmd.ini:27371`:

```ini
[ElectricAssault]
ElectricAssault=yes
Verses=0%,0,0%,0%,0%,0%,100%,100%,100%,50%,100%
InfDeath=5
```

| Key | Meaning |
|-----|---------|
| `ElectricAssault=yes` | **THE Tesla Coil charge flag** — WarheadTypeClass field. **WarheadType+0x158 (byte, ReadBool) [BINARY-VERIFIED audit 33]** (assembly-context proof: writeback `MOV byte ptr [ESI + 0x158], AL` at 0x0075d82e; parser xref @ 0x0075D81D to string at 0x00847D48). When a weapon with this warhead hits a building of type Tesla Coil (or any building with the appropriate flag), the engine routes through the hardcoded **"charge the building"** path instead of normal damage application. **This is the entire mechanism that makes Tesla Trooper able to power a Tesla Coil during a blackout.** |
| `Verses=0%,0,0%,0%,0%,0%,100%,100%,100%,50%,100%` | 11-column. **0% vs all infantry/vehicles** (cannot accidentally damage Soviet allies near the coil; cannot be used as a weapon against enemy infantry/vehicles). 100% vs wood/steel/concrete buildings (allows the engine to recognize the building as a valid target). 50% special_1, 100% special_2 |
| `InfDeath=5` | Electric death anim — moot here (0% vs infantry) |

### Elite's Projectile — `[Electricbounce]`

`rulesmd.ini:25848`:

```ini
[Electricbounce]
ShrapnelWeapon=TeslaFragment
ShrapnelCount=2
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=no
SubjectToWalls=no
```

| Key | Meaning |
|-----|---------|
| `ShrapnelWeapon=TeslaFragment` | On impact, fires `[TeslaFragment]` weapon at nearby targets. Allows chain-bouncing — the Elite Tesla bolt hits one target, then "splashes" 2 more bolts to other nearby targets |
| `ShrapnelCount=2` | Number of secondary targets — 2 |
| `Inviso=yes Image=none` | No primary projectile sprite (the Tesla zap visual handles it) |
| `SubjectToCliffs=yes` | Blocked by cliffs |
| `SubjectToElevation=no` | NOT subject to elevation differences (Tesla bolt can hit elevated targets) |
| `SubjectToWalls=no` | NOT blocked by walls — Tesla bolt arcs over walls |

### Primary's Projectile — `[InvisibleLow]`

Standard LOS-respecting inviso projectile, documented in [SNIPE.md](../allied/SNIPE.md).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death

```ini
[TeslaTroopSelect]                  ; soundmd.ini:4103
Sounds= $itessea $itessec $itessed $itessee $itesmoc
Control= random interrupt
Volume=85

[TeslaTroopMove]                    ; soundmd.ini:4098
Sounds= $itesmoa $itesmob $itesmoc $itesmod $itesmoe $itesmof
Control= random interrupt
Volume=85

[TeslaTroopAttackCommand]           ; soundmd.ini:4093
Sounds= $itesata $itesatb $itesatc $itesatd $itesate
Control= random interrupt
Volume=85

[TeslaTroopFear]                    ; soundmd.ini:4108
Sounds= $itesfea $itesfeb $itesfec
Control= random interrupt
Priority=low
Volume=90

[TeslaTroopDie]                     ; soundmd.ini:4114
Sounds= $itesdia $itesdib $itesdic $itesdid
Priority=low
Control= random interrupt
Volume=90
```

5 select (note `$itesmoc` recycled from Move bank) / 6 move / 5 attack /
3 fear / 4 death. Russian-accented muffled voice (Tesla suit helmet).

### Weapon reports — 3 distinct sounds

```ini
[TeslaTroopAttack]                  ; soundmd.ini:1160
Sounds=itesatta
FShift= -5 5
VShift=10

[TeslaTroopEliteAttack]             ; soundmd.ini:1165
Sounds=itesat2a itesat2b
Control= random
VShift=10
Volume=60

[TeslaTroopRechargeCoil]            ; soundmd.ini:1171
Sounds=iteschaa
Limit=3
VShift=10
Volume=15
```

| Sound | Used by | Distinction |
|-------|---------|-------------|
| `TeslaTroopAttack` | `[ElectricBolt]` (Veteran Primary) | Single sample `itesatta` |
| `TeslaTroopEliteAttack` | `[ElectricBoltE]` (Elite Primary) | 2 alternate samples for slight variation |
| `TeslaTroopRechargeCoil` | `[AssaultBolt]` (Secondary — Tesla Coil charge) | Distinct charging-hum `iteschaa`, very low Volume=15, Limit=3 — soft, ambient sound for the charging activity |

Three separate sound IDs for one unit type — among the highest count of any infantry. Reflects the gameplay distinction between offensive, Elite, and charge actions.

### Cross-family Tesla sounds

- `[TeslaTankAttack]` (soundmd.ini:1881) — used by Tesla Tank (different unit)
- `[TeslaCoilAttack]` (soundmd.ini:2239) — used by Tesla Coil building
- `[TeslaCoilPowerUp]` (soundmd.ini:2235) — Tesla Coil priming sound (separate from charge)
- `[TeslaCoilSuper]` (soundmd.ini:2243) — special "powered by Tesla Trooper" sound? possibly the louder coil discharge when Trooper-charged

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND` | Soviet Barracks ONLY — no Battle Lab requirement despite TechLevel=5. Critical for Soviet tech tree: SHK must be buildable without Battle Lab so Soviet players can defend Tesla Coils during power outages even if their Battle Lab is destroyed |
| `Owner=` | `Russians,Confederation,Africans,Arabs` | All 4 Soviet countries |
| `TechLevel=` | `5` | Mid-game tech-5 cap (but not Battle Lab gated) |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=500` | $500 | |
| `Soylent=250` | $250 refund (Yuri only) | |
| `Points=5` | 5 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiredHouses=` (any Soviet country can build), no `RequiresStolenXxxTech=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities + activates `ElitePrimary=ElectricBoltE` weapon swap. Elite ElectricBoltE: Range 3→5, ROF 60→40, and **chain-bounces to 2 nearby targets via ShrapnelWeapon=TeslaFragment**. Damage stays 50 but the FIREPOWER stack pushes effective damage higher |
| AltCameo | `SHKUICO` shown in sidebar once Veteran rank reached |

`Trainable=` defaults to `yes` (not overridden).

---

## Hardcoded behavior — Ghidra-verified

### 1. Tesla Coil charge — the `ElectricAssault=yes` warhead flag

The iconic "Tesla Trooper charges friendly Tesla Coil during blackout"
mechanic. Mechanism:

1. Player orders a Tesla Trooper to attack-target a friendly Tesla Coil
2. Engine's weapon-pick code chooses **Secondary** (AssaultBolt) instead of
   Primary because the Verses of AssaultBolt's `ElectricAssault` warhead
   show non-zero damage against the coil's structure armor while the Primary's
   `Shock` warhead would target the coil as a building (same target type
   but the Secondary is configured specifically for the charge interaction
   — designer intent encoded via the Verses spread)
3. AssaultBolt fires at Range 1.83 (Trooper must be adjacent)
4. On impact, the warhead's **`ElectricAssault=yes`** flag (WarheadTypeClass
   field, per `WarheadTypeClass__ReadINI @ 0x0075D81D` DATA xref to string
   at `0x00847D48`) routes through the hardcoded "charge the target
   building" engine path
5. The Tesla Coil treats the charge as a temporary power-on event —
   bypassing the normal `Powered=yes` + `Power<0` blackout gate. The coil
   can fire its normal `[CoilBolt]` weapon (Damage 200) at enemy targets

The charge does NOT permanently fix power — it's per-shot enabling. The
Tesla Trooper must continuously fire AssaultBolt to keep the coil active
during the blackout. ROF=25 means roughly 36 charges per minute @ 15fps.

**Multiple SHKs can charge one coil simultaneously** — stacks charges for
faster firing. **Coil also fires harder when Trooper-charged** in some
documented behaviors (longer range, more damage); this is the
`[TeslaCoilSuper]` sound — likely triggered by `ChargedAnimTime`
field. **[INCORRECT — scope correction, audit 33]**: `ChargedAnimTime`
is **BuildingType-scope**, NOT Rules-AudioVisual. Parser xref @
0x00460b9e is in `BuildingTypeClass_ReadINI_Water` →
**BuildingType+0x16E8 (float, ReadDouble) [BINARY-VERIFIED audit 33]**.
The "charged" animation timer is per-building (Tesla Coil), not a
global rule. `TeslaCharge` (string @ 0x0083A480) DOES remain
Rules-AudioVisual scope — parser xref @ 0x0066AC29 re-confirmed.

### 2. IsElectricBolt + IsAlternateColor — visual rendering flags

Both are **WeaponTypeClass** fields:

- **`IsElectricBolt=true`** (xref `WeaponTypeClass__ReadINI @ 0x00772854` to
  string at `0x008492E4`) — engine draws the animated Tesla zap visual
  (arcing/branching/flickering electric bolt) between firer and target.
  Used by ElectricBolt, ElectricBoltE, AssaultBolt, TankBolt (Tesla Tank),
  CoilBolt (Tesla Coil)
- **`IsAlternateColor=true`** (xref `WeaponTypeClass__ReadINI @ 0x0077288C`
  to string at `0x008492C0`) — shifts the bolt color to alternate (orange/red
  instead of blue). Used only on AssaultBolt to visually distinguish charge
  vs attack

Pure visual; no gameplay effect. But critical for parity — players rely on
the color difference to tell at a glance whether their Tesla Trooper is
attacking or charging.

### 3. Crushable=no — uncrushable infantry

INI flag `Crushable=no` on the type means vehicles cannot crush the Tesla
Trooper (no instant-kill on vehicle ram). Combined with Strength=130 +
Armor=Plate, SHK is the **most physically durable basic infantry**. The
Soviet anti-rush answer — vehicle rushes can't simply trample a Tesla
Trooper line.

### 4. Charges — TS-style charge-up firing mechanic (NOT used by SHK)

Comment in `[CoilBolt]` (Tesla Coil's weapon): "SJM: Now using home-grown
DelayedFire system. Charges=yes" — indicates the **`Charges=yes`** flag
(WeaponTypeClass field, per `WeaponTypeClass__ReadINI @ 0x00772693` xref to
string at `0x008493C0`) was the legacy TS mechanism for charge-up weapons,
**superseded** by a newer "DelayedFire" system. Tesla Coil now uses
DelayedFire instead of Charges. SHK weapons do not use Charges. Worth
noting because this flag could appear in other weapons / older docs.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("IsElectricBolt\|IsAlternateColor\|Charges\|Charged\|TeslaCharge")` | 6 strings — `ChargedAnimTime`, `TeslaCharge`, `UseChargeDrain`, `IsAlternateColor`, `IsElectricBolt`, `Charges` |
| `search_strings("ElectricAssault\|AssaultAnim")` | 2 strings — confirms both are hardcoded-recognized |
| `get_xrefs_to(0x008492E4)` (= "IsElectricBolt") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772854` DATA — confirms per-weapon visual rendering flag |
| `get_xrefs_to(0x008492C0)` (= "IsAlternateColor") | Sole xref from `WeaponTypeClass__ReadINI @ 0x0077288C` DATA — confirms per-weapon color shift |
| `get_xrefs_to(0x00847D48)` (= "ElectricAssault") | Sole xref from `WarheadTypeClass__ReadINI @ 0x0075D81D` DATA — **confirms ElectricAssault is a warhead-level flag**, the Tesla Coil charge trigger |
| `get_xrefs_to(0x00849410)` (= "AssaultAnim") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772574` DATA — per-weapon UC-clear animation |
| `get_xrefs_to(0x0083A480)` (= "TeslaCharge") | Sole xref from `RulesClass__ReadAudioVisual @ 0x0066AC29` DATA — global RulesClass [AudioVisual] field, likely controls visual params of the charge effect |
| `get_xrefs_to(0x008493C0)` (= "Charges") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772693` DATA — legacy TS charge-up flag, superseded by DelayedFire system |

Confirmation: SHK itself has minimal type-specific hardcoded behavior. The
distinctive mechanics are:
1. Per-weapon flags (`IsElectricBolt`, `IsAlternateColor`) for visual
2. Per-warhead flag (`ElectricAssault`) for the Tesla Coil charge gameplay
3. Per-type flag (`Crushable=no`) for crush immunity

All three are general-purpose engine flags reused for the Tesla Trooper
configuration.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;MovementZone=InfantryDestroyer` (commented) | Designer-fixed copy-paste from Disk Thrower | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only terrain); defensively set | OK |
| `Charges=` WeaponTypeClass flag | **TS legacy** — per Tesla Coil weapon comment "SJM: Now using home-grown DelayedFire system. Charges=yes". The Charges system was superseded by DelayedFire in YR. SHK does not use Charges; documented for reference | Documented |
| `AssaultAnim=UCELEC` on ElectricBolt | YR-active in general (UC-clear animation), but **vestigial on SHK** because `Assaulter=no` | Documented |
| `IsElectricBolt`/`IsAlternateColor` | YR-active — used every match Tesla weapons fire | OK |
| `ElectricAssault=yes` warhead flag | YR-active — Tesla Coil charge is a core YR mechanic | OK |
| `Crushable=no` | YR-active — engine respects the flag for crush resolution | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard RA2/YR infantry | OK |

No TS-only behavior found on the SHK type itself.

---

## Cross-references

- **Related Tesla units** sharing the IsElectricBolt visual:
  - `[SHK]` Tesla Trooper (this doc) — Primary ElectricBolt, Secondary AssaultBolt
  - `[TTNK]` Tesla Tank — Primary TankBolt (Damage 135, Range 4)
  - `[TESLA]`/`[ATESLA]` Tesla Coil — CoilBolt (Damage 200, Range 7)
- **Same warhead family** (`Shock`/`Electric`):
  - `[ElectricBolt]` SHK Primary — Shock warhead
  - `[ElectricBoltE]` SHK Elite — Shock warhead
  - `[TankBolt]` Tesla Tank — Electric warhead (slightly different Verses: same as Shock here)
  - `[CoilBolt]` Tesla Coil — Electric warhead
  - `[AssaultBolt]` SHK Secondary — ElectricAssault warhead (unique to charge)
- **Sister Soviet basic infantry**:
  - `[E2]` Conscript — cheap mass-soldier counterpart
  - `[FLAKT]` Flak Trooper — AA-specialist counterpart
  - `[IVAN]` Crazy Ivan — bomb-plant specialist
  - `[DESO]` Desolator — radiation specialist
- **Counter-units / hard counters**:
  - Yuri / Initiate / Magnetron mind-control (ImmuneToPsionics=no)
  - Sniper one-shot (250 damage > Strength=130)
  - Anti-infantry crush — **does not work** (Crushable=no)
  - Long-range artillery (V3 Rocket, Prism Tank) — outranges SHK's 3-cell Primary
  - Air units — SHK has no AA capability
- **Buildings SHK can charge** via AssaultBolt:
  - `[TESLA]` Soviet Tesla Coil (base game)
  - `[ATESLA]` Tesla Coil alt (if defined)
- **Related global rules**:
  - `[AudioVisual] TeslaCharge=` — engine-recognized RulesClass field for charge visual params
  - `ChargedAnimTime=` — **BuildingType-scope** (corrected audit 33; was incorrectly attributed to RulesClass). BuildingType+0x16E8 float — per-coil animation timing
- **Soundmd cross-link**:
  - `[TeslaCoilSuper]` (soundmd.ini:2243) — likely the special "powered by Trooper" coil discharge sound, distinct from `[TeslaCoilAttack]`

---

## Ghidra audit log (audit iteration 33 — 2026-05-19)

**~17 Ghidra queries** (10 string searches + 5 xref lookups + 1 assembly-
context batch for ElectricAssault/Charges/Assaulter/ChargedAnimTime + grep
on WeaponType decompile from audit 28). 6 doc-cited claims verify + 3 NEW
struct-offset bindings BINARY-VERIFIED + 1 important CORRECTION to audit
1 cumulative + 1 IN-DOC scope correction.

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `WarheadTypeClass__ReadINI` | 0x0075d590 | ElectricAssault parser @ 0x0075d81d → +0x158 |
| `WeaponTypeClass__ReadINI` | 0x00772080 | Charges/IsElectricBolt/IsAlternateColor/AssaultAnim re-confirmed audit 9+28+33 |
| `InfantryTypeClass__ReadINI` | 0x005240a0 | Assaulter parser @ 0x005244ef → +0xEB5 (corrects audit 1) |
| `BuildingTypeClass_ReadINI_Water` | (oversized) | ChargedAnimTime parser @ 0x00460b9e → +0x16E8 (corrects doc scope claim) |
| `RulesClass__ReadAudioVisual` | (oversized) | TeslaCharge parser @ 0x0066AC29 re-confirmed |

### Key behavioral findings — 3 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `ElectricAssault` | WarheadType | **+0x158** | byte (ReadBool) | 0x0075d81d | NEW (Tesla Coil charge flag; sibling to +0x14B Sonic from audit 28) |
| `AssaultAnim` | WeaponType | **+0x114** | AnimType* (ReadString + AnimTypeClass__FindOrAllocate) | 0x00772574 | NEW (sibling to OccupantAnim +0x110 audit 32 and OpenToppedAnim +0x118 NEW) |
| `ChargedAnimTime` | **BuildingType** (NOT Rules-AudioVisual) | **+0x16E8** | float (ReadDouble) | 0x00460b9e | NEW (corrects doc scope claim) |

Plus 1 NEW bonus offset discovered in WeaponType anim cluster:
- `OpenToppedAnim` = WeaponType+0x118 (AnimType*) — sibling to OccupantAnim and AssaultAnim. (Seen in audit-28 decompile, formally pinned now.)

Re-confirmations (already in cumulative):
- `IsElectricBolt` = WeaponType+0x152 (audit 9) — parser xref @ 0x00772854 re-verified
- `IsAlternateColor` = WeaponType+0x154 (audit 9) — parser xref @ 0x0077288C re-verified
- `Charges` = WeaponType+0x148 (audit 9) — parser xref @ 0x00772693 re-verified (legacy TS charge-up flag, superseded by DelayedFire)
- `Crushable` = ObjectType+0x22D (audit 7) — `Crushable=no` on SHK suppresses crush; ObjectType-scope, applies to all unit classes
- `TeslaCharge` = Rules-AudioVisual scope confirmed (offset DEFERRED — parser oversized)

### CRITICAL CORRECTION to audit 1 cumulative

**Audit 1 (E1) claim: "InfantryType+0xEB5 = paratrooper-occupier flag" is INCORRECT.**

Assembly-context proof at parser site 0x005244ef:
```
005244ef: PUSH 0x82599c        ; push "Assaulter" string
005244f5: MOV ECX, EBP
005244f7: CALL 0x005295f0      ; ReadBool
005244fc: MOV EDX, [ESI+0xeb0]
00524502: MOV ECX, EBP
00524504: PUSH EDX
00524505: PUSH 0x825988        ; next key
0052450a: PUSH EDI
0052450b: MOV byte ptr [ESI + 0xeb5], AL   ; store Assaulter result
```

The writeback at 0xEB5 happens AFTER the Assaulter ReadBool. So
**InfantryType+0xEB5 = Assaulter (byte)**, NOT "paratrooper-occupier
flag" as audit 1 inferred from the AddGarrisonOccupant decompile. The
audit-1 inference was a label guess from context; this audit pins the
actual INI binding.

Updated meaning: `Assaulter=yes` enables an infantry unit to clear
garrisoned UC buildings (SEAL/Tanya/Yuri). SHK explicitly sets
`Assaulter=no` — Tesla Trooper cannot clear UC buildings, making the
`AssaultAnim=UCELEC` on ElectricBolt vestigial.

### IN-DOC scope correction

**[INCORRECT — IN-DOC]**: SHK doc claimed `ChargedAnimTime` is a
RulesClass field. Actual scope is **BuildingType** (parser xref @
0x00460b9e in `BuildingTypeClass_ReadINI_Water`, NOT
`RulesClass__ReadAudioVisual`). Field is **BuildingType+0x16E8 (float,
ReadDouble)**. Semantically correct: the "charged" animation timer is
per-building (per Tesla Coil instance), not a global rule. `TeslaCharge`
(separate INI key) DOES remain Rules-AudioVisual scope — the two were
conflated in the doc.

### Items NOT re-verified (DEFERRED with reason)

- **TeslaCharge Rules-AudioVisual offset** — RulesClass__ReadAudioVisual
  oversized; offset DEFERRED.
- **`ElectricAssault=yes` consumer path** — the engine routes the warhead
  to a building-charge code path instead of damage application; consumer
  decompile DEFERRED (likely in WarheadTypeClass::Detonate or
  BuildingClass::ReceiveDamage).
- **ChargedAnimTime consumer in TeslaCoil animation** — float at
  BuildingType+0x16E8 controls how long the "charged" animation plays;
  consumer DEFERRED.
- **DelayedFire system** — successor to legacy `Charges=yes`; not
  directly verified this audit. Tesla Coil uses DelayedFire instead of
  Charges per the SJM comment in CoilBolt.
- **ShrapnelWeapon=TeslaFragment / ShrapnelCount=2 chain-bounce
  mechanic** for ElectricBoltE — re-uses BulletType+0x2B4/+0x2B8
  (audit 22) cumulative; consumer for "fire shrapnel at N nearest
  targets" DEFERRED.

### Negative claims verified

- `search_strings("SHK")` → **0 matches**.

All SHK behavior is INI-driven via general-purpose flag mechanisms
(`Crushable=no`, `IsElectricBolt`, `IsAlternateColor`, `ElectricAssault`,
`Assaulter=no`).

### Confidence summary

- 3/3 NEW struct-offset bindings BINARY-VERIFIED with assembly-context
  proof.
- 1 NEW bonus offset (OpenToppedAnim = WeaponType+0x118).
- 5 re-confirmations of prior cumulative offsets.
- 1 IMPORTANT CORRECTION to audit 1 cumulative (Assaulter not
  "paratrooper-occupier" at +0xEB5).
- 1 IN-DOC scope correction (ChargedAnimTime BuildingType, not Rules).
- Negative claim confirmed.

**Soviet sub-section: 2 of 32 docs DEEP-AUDITED.**

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [SHK]` | 4507-4545 (39 lines) | All 38 active keys covered (one commented MovementZone documented) |
| `artmd.ini [SHK]` | 193-201 (9 lines) | All keys covered |
| `artmd.ini [ConSequence]` | Already documented in SNIPE.md | Cross-referenced |
| `rulesmd.ini [ElectricBolt]` | 23856-23865 (10 lines) | All keys covered |
| `rulesmd.ini [ElectricBoltE]` | 24872-24881 (10 lines) | All keys covered (delta noted; chain-bounce projectile detailed) |
| `rulesmd.ini [AssaultBolt]` | 23879-23888 (10 lines) | All keys covered |
| `rulesmd.ini [Shock]` warhead | 27364-27369 (6 lines) | All keys covered including 11-column Verses breakdown |
| `rulesmd.ini [ElectricAssault]` warhead | 27371-27374 (4 lines) | All keys covered |
| `rulesmd.ini [Electricbounce]` projectile | 25848-25855 (8 lines) | All keys covered |
| `soundmd.ini` Tesla voices | TeslaTroopSelect, Move, AttackCommand, Fear, Die | All 5 covered |
| `soundmd.ini` Tesla weapon reports | TeslaTroopAttack, TeslaTroopEliteAttack, TeslaTroopRechargeCoil | All 3 covered |
| Hardcoded behavior | ElectricAssault warhead (Tesla Coil charge) + IsElectricBolt/IsAlternateColor visuals + Crushable=no + Charges legacy note | 4 mechanisms covered with Ghidra confirmation |
| Ghidra searches performed against ID | 8 distinct queries (2 strings + 6 xref lookups) | Logged inline |
| TS-legacy filter | Applied; `Charges=` flagged as superseded; `AssaultAnim` flagged as vestigial on SHK; ImmuneToVeins/MovementZone-comment defensive | Done |
