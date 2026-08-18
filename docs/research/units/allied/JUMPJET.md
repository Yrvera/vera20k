# Rocketeer (JUMPJET)
Side: Allied | Category: Infantry | Image alias: `Image=ROCK` → `[ROCK]` artmd

The Allied **Rocketeer**. $600 from the Barracks (needs Radar). An infantry-type
unit with a 6-phase **JumpjetLocomotionClass** flight locomotor — takeoff,
liftoff/ascend, decelerating cruise, long-range horizontal cruise, descend/land,
emergency abort. The only Allied skirmish infantry that flies. Fires the
high-rate `[20mm]` cannon, intended primarily as anti-air (gatling cycle at
Elite via `[20mmE]`). Considered an Aircraft for AI threat (`ConsideredAircraft=yes`)
but produced from the Barracks queue and counted as Infantry for cap/army
limits.

Hardcoded locomotor depth is enormous — see authoritative deep RE in
[JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
and the bridge-interaction follow-ups
[BRIDGE_JUMPJET_ABORT_FLAG_WRITERS_GHIDRA_REPORT.md](../../BRIDGE_JUMPJET_ABORT_FLAG_WRITERS_GHIDRA_REPORT.md)
and
[BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md](../../BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md).

---

## rulesmd.ini — `[JUMPJET]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:3916`:

```ini
[JUMPJET]
UIName=Name:JUMPJET
Name=Rocketeer
Image=ROCK
Category=Soldier
JumpJet=yes
Primary=20mm
Prerequisite=GAPILE,RADAR
Crushable=yes
Strength=125
Fearless=yes
;OpportunityFire=yes ;gs Doesn't work because will fly backwards to point towards it and never shoot
Armor=none
TechLevel=3
;Sight=6
Sight=8
Pip=white
Speed=9
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=600
Soylent=300
Points=15
IsSelectableCombatant=yes
VoiceSelect=RocketeerSelect
VoiceMove=RocketeerMove
VoiceAttack=RocketeerAttackCommand
VoiceFeedback=RocketeerFear
VoiceSpecialAttack=RocketeerMove
DieSound=
CrashingSound=RocketeerDie
ImpactLandSound=RocketeerCrash
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}
PhysicalSize=1
MovementZone=Fly		; This needs to be None, like aircraft
ConsideredAircraft=yes
ThreatPosed=15	; This value MUST be 0 for all building addons
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
JumpjetSpeed=30 ;params not defined use defaults (old globals way up top called Jumpjet controls)
JumpjetClimb=20 ;HY increase climb speed; used to be 10
JumpjetCrash=25 ; Climb, but down
JumpJetAccel=10
JumpJetTurnRate=10
JumpjetHeight=500
JumpjetWobbles=.01
JumpjetDeviation=1
JumpjetNoWobbles=yes ; the wobbling is in the Hover sequence of the art, instead of being programmer art/ sine wave
Size=1
SpeedType=Hover
HoverAttack=yes
Crashable=yes
BalloonHover=yes ; ie never land
MoveSound=RocketeerMoveLoop
ElitePrimary=20mmE
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:JUMPJET` | CSF-string key → "Rocketeer" |
| `Name=Rocketeer` | Internal short name |
| `Image=ROCK` | **Art redirect** — rendering uses `[ROCK]` artmd entry (not `[JUMPJET]`, which doesn't exist in artmd). The Rocketeer SHP is named ROCK on disk |
| `Category=Soldier` | Pip group + AI grouping. Despite Aircraft semantics, it's a Soldier for queue/cap/recycle |
| `JumpJet=yes` | **Behavior flag** — `TechnoTypeClass+0x3E0` bit (per `TechnoTypeClass__ReadINI` xref at `0x007151EC`). Marks the type as jumpjet-capable; enables jumpjet-only mission paths and the Hover/Fly sequence selection |
| `Primary=20mm` | Standard 20mm cannon — `Damage=25`, `ROF=30`, `Range=5`, `Warhead=SSA`, `Report=RocketeerAttack`. AA + AG via the `[Invisible3]` projectile |
| `Prerequisite=GAPILE,RADAR` | Both Allied Barracks AND any building with `Radar=yes` are required (resolves to GAPILE + GAAIRC / GASPYSAT) |
| `Crushable=yes` | Can be crushed by ground vehicles **when landed** — note: flight altitude generally protects from crushing |
| `Strength=125` | HP — 25% more than GI |
| `Fearless=yes` | **Suppresses fear voice / panic behavior** — Rocketeer never plays `Panic` sequence or breaks formation under fire (combined with the `Panic=` mapping to Walk frames, this means the unit visually never shows fear regardless) |
| `;OpportunityFire=yes` (commented) | Designer comment explains why: "Doesn't work because [the unit] will fly backwards to point towards [the target] and never shoot." Engine quirk — jumpjet facing-vs-firing-arc disagreement breaks opportunity fire |
| `Armor=none` | Damage type column 0 — standard infantry armor |
| `TechLevel=3` | Buildable at tech-level 3+; available early-mid game |
| `;Sight=6` then `Sight=8` | Commented earlier value (6) overridden to 8 — wider scout radius matching its mobility |
| `Pip=white` | Cargo-passenger pip color when loaded in transport |
| `Speed=9` | **Horizontal foot-speed** — but JumpjetSpeed (below) is what actually governs in-flight velocity. Speed=9 is high among infantry; matches a fast vehicle |
| `Owner=British,French,Germans,Americans,Alliance` | Allied countries only |
| `AllowedToStartInMultiplayer=no` | Cannot appear in starting unit complement |
| `Cost=600` | $600 — pricier than basic infantry, much cheaper than vehicles |
| `Soylent=300` | Grinder refund (Yuri only) |
| `Points=15` | Kill score |
| `IsSelectableCombatant=yes` | Included in "select all combat units" hotkey |
| `VoiceSelect=RocketeerSelect` | Selection voice bank — `$irocsea/b/c/d/e/f/g` |
| `VoiceMove=RocketeerMove` | Move voice — `$irocmoa..f` |
| `VoiceAttack=RocketeerAttackCommand` | Attack-order voice — `$irocata..e` |
| `VoiceFeedback=RocketeerFear` | Fear voice — `$irocfea..c` (priority=low) — **rarely heard** since `Fearless=yes` |
| `VoiceSpecialAttack=RocketeerMove` | **Reuses Move voice** for special-attack — no specific special-attack line; Rocketeer has no SW |
| `DieSound=` | **Empty** — no death sound on the unit itself. Death audio comes from CrashingSound (during fall) and ImpactLandSound (on terminal impact). This split distinguishes "shot in midair → tumbling" from "hitting the ground" |
| `CrashingSound=RocketeerDie` | Sound played when shot down — `irocdiea` plays during the fall (jumpjet state-5 abort path) |
| `ImpactLandSound=RocketeerCrash` | Sound played when impact hits the ground — `iroccraa/iroccrab` random pick |
| `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` | **JumpjetLocomotionClass** CLSID. Constructor at `0x0054AC40`; vtable at `0x007ECD68`. Six-state machine: 0=Grounded, 1=Liftoff, 2=Decelerating-cruise, 3=Long-range cruise, 4=Descend/land, 5=Emergency abort (6 = sentinel/terminal). See deep RE doc |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Fly` | **Engine designer comment: "This needs to be None, like aircraft"** — kept as Fly historically; pathfinder treats this MZ as nearly-unrestricted. The `Fly` MovementZone bypasses ground occupancy for in-flight movement |
| `ConsideredAircraft=yes` | **Behavior flag** — AI targeting groups this with aircraft, so AA units (Flak Cannon, Patriot, etc.) will engage it as a flyer rather than as ground infantry. Critical for play balance |
| `ThreatPosed=15` | AI scoring weight; medium — Rocketeer is real threat but not top-priority |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | At Veteran rank: +25% strength, +25% firepower, +20% ROF, +1 sight, +20% speed |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | At Elite rank: passive HP regen, +50% strength stack, +50% firepower stack, +40% ROF stack. NOTE: no FASTER on Elite tier (cap) |
| `JumpjetSpeed=30` | **Per-unit override of `[JumpjetControls].Speed`**. RulesClass+0x410 default is 14; Rocketeer raises to 30 (cruise velocity in lepton/tick units). Comment: "params not defined use defaults (old globals way up top)" — i.e., any key absent here falls back to `[JumpjetControls]` |
| `JumpjetClimb=20` | Override of `[JumpjetControls].Climb` (default 5). Comment: "HY increase climb speed; used to be 10" — designer note. Vertical velocity going up |
| `JumpjetCrash=25` | "Climb, but down" — vertical velocity when falling/landing |
| `JumpJetAccel=10` | Override of `[JumpjetControls].Acceleration` (default 2). Speed-ramp rate |
| `JumpJetTurnRate=10` | Override of `[JumpjetControls].TurnRate` (default 4). Faster turning than the global default |
| `JumpjetHeight=500` | Override of `[JumpjetControls].CruiseHeight` (default 500 — same value). Cached at locomotor instance+0x2C (verified in RE doc §6 / R3.6); used by `In_Which_Layer` (instance+0x2C) to decide z-sort layer 3 vs 4, and by State 0→State 1 as the climb-target altitude (copied to instance+0x80) |
| `JumpjetWobbles=.01` | Override of `[JumpjetControls].WobblesPerSecond` (default .15). Extremely low — combined with `JumpjetNoWobbles=yes` the wobble math is effectively disabled |
| `JumpjetDeviation=1` | Override of `[JumpjetControls].WobbleDeviation` (default 40). Amplitude minimal |
| `JumpjetNoWobbles=yes` | **Behavior flag** — disables programmer-generated sinusoidal wobble. Comment: "the wobbling is in the Hover sequence of the art, instead of being programmer art/ sine wave" — meaning the visual hover bob comes from the art sequence (`[RocketeerSequence].Hover`) rather than runtime sine. **Important for parity**: the wobble we see in-game is an animated frame cycle, not a per-frame coord offset |
| `Size=1` | Transport cargo slot cost |
| `SpeedType=Hover` | Speed lookup table column — uses Hover row of the `TerrainClass` `[Speeds]` matrix. Determines per-cell speed multiplier (e.g., 100% on most terrain since Hover ignores ground type) |
| `HoverAttack=yes` | **Behavior flag** — unit can fire while in flight without first landing. Standard aircraft (FlyLocomotion) generally must complete an attack run; HoverAttack=yes lets the Rocketeer fire while loitering, like a chopper. Engine quirk noted in `OpportunityFire` comment: facing pulls toward target which interrupts firing |
| `Crashable=yes` | When killed, plays the crash sequence (Tumble + AirDeathStart/Falling/Finish from artmd) rather than a flat "infantry dies" sprite swap. State machine enters state 5 (emergency abort) for the fall |
| `BalloonHover=yes` | **Behavior flag** ("ie never land") — type-class flag set by `TechnoTypeClass__ReadINI` (xref at `0x00714D95`). Causes the unit to **never voluntarily descend to ground** between moves; it hovers in place at CruiseHeight when idle. State 0 (Grounded) is essentially never reached after first liftoff. Key for parity: a Rocketeer with no orders does **not** land; it floats |
| `MoveSound=RocketeerMoveLoop` | Looping engine SFX while moving — 5 layered samples (`iroclo1 iroclo2a iroclo2b iroclo2c iroclo3`), max 3 concurrent (`Limit=3`) |
| `ElitePrimary=20mmE` | At Elite rank, Primary is swapped to `[20mmE]` — same damage / range / warhead but ROF drops from 30 to 5 frames (6× firing rate). This is the standard "elite Rocketeer is a gatling AA platform" behavior |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `no` (Rocketeer never crawls; cannot go prone)
- `Bombable=` — defaults to `false`; absent from explicit list. Crazy Ivan can still attach a bomb via Bomb mission, but flight altitude usually prevents reach
- `ImmuneToVeins=` — not set; flying anyway, veins are TS-only
- `ImmuneToPsionics=` — defaults to `no`; **Rocketeer can be mind-controlled** (drops out of sky? — interaction with locomotor is via Piggyback; mind-control places ownership change but the locomotor remains JumpjetLoco)
- `Deployer=` — defaults to `no`
- `Occupier=` — defaults to `no` — Rocketeer cannot garrison civilian buildings
- `Assaulter=` — defaults to `no`
- `C4=` — not set; Rocketeer has no demo charge
- `Agent=`/`Infiltrate=` — not set; Rocketeer cannot infiltrate
- `Spawned=` — defaults to `no` (Rocketeer is a buildable unit, not spawn-only)
- `Trainable=` — **not set, defaults to `yes`** — Rocketeer DOES gain veterancy (note presence of `VeteranAbilities`/`EliteAbilities`/`ElitePrimary` confirms this)
- `IFVMode=` — not set; **Rocketeer cannot enter [HTK] IFV** (the IFV gunner table requires `IFVMode=N` to register a passenger swap)
- `Naval=` — not set
- `PreventAttackMove=` — not set; Rocketeer obeys Attack-Move (with the OpportunityFire caveat in the commented-out line above)
- `CanPassiveAquire=` — not set, defaults to `yes`
- `CanRetaliate=` — not set, defaults to `yes` — Rocketeer fires back when shot

---

## artmd.ini — `[ROCK]` section (via `Image=ROCK` redirect)

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:112`:

```ini
[ROCK] ; Rocketeer
Cameo=JJETICON
AltCameo=JJETUICO
Sequence=RocketeerSequence
Crawls=no
Remapable=yes
FireUp=2
PrimaryFireFLH=100,0,120
```

| Key | Meaning |
|-----|---------|
| `Cameo=JJETICON` | Sidebar build icon (SHP `JJETICON`) — **note** filename uses "JJET" not "ROCK" (cameo art is keyed to the original `JUMPJET` short-name) |
| `AltCameo=JJETUICO` | Elite cameo (suffix UICO = Upgraded Icon). Shown after Veteran promotion |
| `Sequence=RocketeerSequence` | Reference to `[RocketeerSequence]` block (frame layout) |
| `Crawls=no` | **Cannot crawl/go prone** — sets the prone-disabled flag on the type. Rocketeer is always upright (or in flight) |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame within the FireUp track — fire happens at frame 2 of the firing animation (matches the `[RocketeerSequence].FireUp=164,6,6` cycle and the `[RocketeerSequence].FireFly=370,6,6` cycle) |
| `PrimaryFireFLH=100,0,120` | **Fire-Launch-Height** for `Primary=20mm` — 100 leptons forward, 0 sideways, 120 leptons up. Z component 120 puts the muzzle flash high relative to the sprite — appropriate for a shoulder-mounted/under-arm 20mm cannon held by a flying unit |

Missing `SecondaryFireFLH=` because Rocketeer has no Secondary weapon.

Note: the standard Soviet Lunar Troops `[LUNR]` immediately below this section
also uses `Sequence=RocketeerSequence` (re-uses the same flight-poses art frames)
with `PrimaryFireFLH=75,-50,85` — confirms that the RocketeerSequence layout is
**shared by all jumpjet-style infantry**, with per-unit FLH being the only
customization needed for the firing pose.

### Referenced sequence — `[RocketeerSequence]`

`artmd.ini:14557`:

```ini
[RocketeerSequence]
Ready=0,1,1
Guard=0,1,1
;Ready=292,6,6
;Guard=292,6,6
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
Die3=0,0,0
Die4=0,0,0
Die5=0,0,0
Fly=292,6,6
Hover=292,6,6
FireFly=370,6,6
Tumble=340,15,0
AirDeathStart=340,8,0;gs a split of Tumble, which is unused
AirDeathFalling=348,1,0
AirDeathFinish=349,6,0
Paradrop=418,1,0
Cheer=419,8,0,E
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle (1 frame × 1 facing) | Grounded idle — visible only between liftoff and the first hover, basically transient |
| `Guard=0,1,1` | Guard mission idle | Same |
| `;Ready=292,6,6` / `;Guard=292,6,6` | Commented out | Original plan was to use the Hover-flight frames as idle; replaced with the standing-on-ground pose |
| `Prone=86,1,6` | Prone single frame × 6 facings | **Unreachable** in practice — `Crawls=no` means Rocketeer never goes prone |
| `Walk=8,6,6` | Walk cycle (6 frames × 6 facings) | Reused for `Panic=` too. Used very rarely — Rocketeer auto-lifts off |
| `FireUp=164,6,6` | Standing fire cycle | Used only when Rocketeer fires while grounded |
| `Down=260,2,2` | Get-down to prone (2 frames × 2 facings) | Unused (no prone) |
| `Crawl=86,6,6` | Crawl cycle reuses prone | Unused |
| `Up=276,2,2` | Get-up from prone | Unused |
| `FireProne=212,6,6` | Prone-fire cycle | Unused |
| `Idle1=56,15,0,S` | Idle anim 1 — 15 frames, S-facing | Plays only when truly grounded — rarely seen due to `BalloonHover=yes` |
| `Idle2=71,15,0,E` | Idle anim 2 — 15 frames, E-facing | Same |
| `Die1=134,15,0` | Death anim 1 — 15 frames, omnidirectional | Plays on land-based death |
| `Die2=149,15,0` | Death anim 2 | Same |
| `Die3=0,0,0` `Die4=0,0,0` `Die5=0,0,0` | **Frame-count 0 entries** — engine treats these as "no animation" rather than fallback-to-Ready. Effectively disables death variants 3-5 | |
| `Fly=292,6,6` | **Flight pose** (6 frames × 6 facings) — horizontal flight animation. Triggered by `FootClass::Locomotion_AI` CLSID match when the unit is moving | Per JUMPJET RE R4.1: artmd index 0x18 (24) |
| `Hover=292,6,6` | **Hover pose** — same frame block as Fly (visual difference negligible) — shown when idle in mid-air. Per RE R4.1: artmd index 0x17 (23) | The "hover bob" effect comes from this 6-frame loop, not from runtime wobble math (`JumpjetNoWobbles=yes` reinforces) |
| `FireFly=370,6,6` | **Fire-while-flying cycle** (6 frames × 6 facings) — shown when firing in mid-air (HoverAttack=yes) | This is the most-seen Rocketeer firing animation |
| `Tumble=340,15,0` | Mid-air death tumble — 15 frames | Designer comment: "a split of Tumble, which is unused" — meaning the next three entries are derived from this and the Tumble entry itself is technically not played as a single block |
| `AirDeathStart=340,8,0` | First 8 frames of Tumble — start of fall | Plays when Rocketeer is shot down (state 5 emergency-abort entry) |
| `AirDeathFalling=348,1,0` | Single frame at 348 — falling pose | Loops during the fall |
| `AirDeathFinish=349,6,0` | 6 frames at 349 — impact spasm | Plays on ground impact, before sprite swap to dead |
| `Paradrop=418,1,0` | Single frame at 418 — **paradrop pose** | Per JUMPJET RE R4.1: sequence index 0x21 (33). The dropped-from-cargo-plane pose; **Rocketeer normally isn't paradropped** (the artmd line exists defensively / for special-case use) |
| `Cheer=419,8,0,E` | Cheer animation — 8 frames, E-facing |  |
| `Panic=8,6,6` | Panic = reuse of Walk frames | Mostly unreachable due to `Fearless=yes` |

---

## Weapons

### Primary (Veteran and below) — `[20mm]`

`rulesmd.ini:22976`:

```ini
[20mm]
Damage=25
ROF=30
Range=5
Projectile=Invisible3
Speed=100
Warhead=SSA
Report=RocketeerAttack
```

| Key | Meaning |
|-----|---------|
| `Damage=25` | Per-shot damage |
| `ROF=30` | Cooldown between shots — 30 frames = 2 seconds @ 15 fps |
| `Range=5` | 5 cells (typical infantry weapon range) |
| `Projectile=Invisible3` | `Inviso=yes Image=none AA=yes AG=yes` — instant-hit invisible bullet, can target both air and ground |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=SSA` | Small-Arms warhead with infantry-killing verses + ProneDamage=80% + Bullets=yes |
| `Report=RocketeerAttack` | Sound `irocatta` (single-burst gunshot) |

### Elite Primary — `[20mmE]`

`rulesmd.ini:24824`:

```ini
[20mmE]
Damage=25
ROF=5
Range=5
Projectile=Invisible3
Speed=100
Warhead=SSA
Report=RocketeerAttack
```

Identical to `[20mm]` except **`ROF=5` instead of 30** — 6× faster firing rate.
Activated via `ElitePrimary=20mmE` once unit reaches Elite veterancy. Same
damage, same warhead, same range — pure DPS bump.

### Primary's Warhead — `[SSA]`

`rulesmd.ini:26509`:

```ini
[SSA]
;DB Changed how Plate interacts with this warhead on 6/6. See also AP warhead.
;Verses=100%,100%,70%,60%,40%,40%,75%,50%,25%,100%,100%
Verses=100%,100%,100%,60%,40%,40%,75%,50%,25%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
;Bright=yes
Bullets=yes
ProneDamage=80%
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,60%,40%,40%,75%,50%,25%,100%,100%` | 11-column armor table (`none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2`). Excellent vs infantry (100/100/100), weak-to-mid vs vehicles (60/40/40), specialty rules vs structures. Designer-history comment shows Plate was 70% then bumped to 100% |
| `InfDeath=1` | **Infantry death animation type 1** — "small arms" — infantry plays the standard shot-down anim when killed by this warhead |
| `AnimList=PIFFPIFF,PIFFPIFF` | Impact animation list — `PIFFPIFF` plays at the impact point. Listed twice (engine random-pick from list, two slots = always PIFFPIFF) |
| `Bullets=yes` | Marks the warhead as bullet-type for engine purposes (e.g., minigun behavior, AI threat classification) |
| `ProneDamage=80%` | If target is prone infantry, damage is multiplied by 80% — prone reduces SSA exposure by 20% |

### Projectile — `[Invisible3]`

`rulesmd.ini:25359`:

```ini
[Invisible3]
Inviso=yes
Image=none
AA=yes
AG=yes
```

Standard no-sprite instant-hit projectile. AA+AG enables the Rocketeer to fire
at both air and ground targets (its primary intended role is anti-air but the
weapon can engage anything).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear

```ini
[RocketeerSelect]                  ; soundmd.ini:3913
Sounds= $irocsea $irocseb $irocsec $irocsed $irocsee $irocsef $irocseg
Control= random interrupt

[RocketeerMove]                    ; soundmd.ini:3909
Sounds= $irocmoa $irocmob $irocmoc $irocmod $irocmoe $irocmof
Control= random interrupt

[RocketeerAttackCommand]           ; soundmd.ini:3905
Sounds= $irocata $irocatb $irocatc $irocatd $irocate
Control= random

[RocketeerFear]                    ; soundmd.ini:3917
Sounds= $irocfea $irocfeb $irocfec
Control= random interrupt
Priority=low
```

Seven select / six move / five attack / three fear lines — wider voice bank
than most infantry. `Priority=low` on fear means other voices (move, attack,
crash) will preempt it. Combined with `Fearless=yes` on the type, fear lines
are nearly never actually heard.

### Weapon report

```ini
[RocketeerAttack]                  ; soundmd.ini:1082
Sounds=irocatta
FShift= -10 10
Control= random interrupt
VShift=10
Volume=70
```

Single-sample `irocatta` for both `[20mm]` and `[20mmE]` reports — Elite's
6× ROF causes the sound to fire 6× as often (no separate Elite sample).
`FShift=-10 10` randomizes pitch ±10%, `VShift=10` randomizes volume ±10%.

### Death sounds (split: in-air vs ground)

```ini
[RocketeerDie]                     ; soundmd.ini:1090 — wired via CrashingSound=
Sounds= irocdiea ;$irocdia $irocdib $irocdic
Priority=low
Control=
FShift= -5 5
Volume=70

[RocketeerCrash]                   ; soundmd.ini:1106 — wired via ImpactLandSound=
Sounds=iroccraa iroccrab
Control=random
FShift=-10 10
```

`RocketeerDie` plays during the fall (locomotor state 5 abort path).
`RocketeerCrash` plays on ground impact at the end of the AirDeathFinish
animation. The commented-out `$irocdia/b/c` are voice-banked alternates that
weren't used in retail; only the SFX `irocdiea` ships. Note the **`DieSound=`
key is empty** on the type — no on-death speech, the audio is purely the
crash SFX chain.

### Engine loop

```ini
[RocketeerMoveLoop]                ; soundmd.ini:1097
Sounds=iroclo1 iroclo2a iroclo2b iroclo2c iroclo3
Control= loop random all decay attack
FShift= -5 5
Priority=Low
Limit=3
VShift=10
Volume=25
```

Five engine samples mixed in a `loop random all` mode (continuously crossfades
all five). `decay attack` adds ASR-style envelope. `Limit=3` allows up to 3
concurrent voices (so 3 Rocketeers nearby all loop independently before cap).
`Priority=Low` ensures combat/voice SFX duck under it.

### Not wired (referenced but absent on the type)

- `[RocketeerSpecialAttack]` — does not exist (Rocketeer has no special attack;
  `VoiceSpecialAttack=` reuses `RocketeerMove`)

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `GAPILE,RADAR` | Both Allied Barracks AND any building with `Radar=yes` (resolves to Airforce Command HQ `GAAIRC` typically) |
| `Owner=` | `British,French,Germans,Americans,Alliance` | Allied countries only; no `ForbiddenHouses=` since `Owner=` already excludes Soviet/Yuri |
| `TechLevel=` | `3` | Skirmish/MP techlevel cap; available from tech 3 |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=600` | $600 | |
| `Soylent=300` | $300 refund | Grinder (Yuri) only |
| `Points=15` | 15 | Kill-score contribution |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiresStolenXxxTech=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — applies standard veterancy multipliers from `[CombatDamage]` (STRONGER and FIREPOWER scale by `VeteranCombat` value; ROF and FASTER by `VeteranSpeed`; SIGHT adds +1) |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — adds passive HP regen via SELF_HEAL; stacks higher multipliers from `EliteCombat`. No FASTER on Elite (capped). **Triggers `ElitePrimary=20mmE` weapon swap** — 6× faster ROF (5 frames vs 30) |
| AltCameo | `JJETUICO` shown in sidebar once Veteran rank reached |

XP gain follows normal rules; `Trainable=` defaults to `yes` (not overridden).

---

## Hardcoded behavior — Ghidra-verified

### 1. JumpjetLocomotionClass — 6-state flight machine [BINARY-VERIFIED audit 8]

Locomotor CLSID `{92612C46-F71F-11D1-AC9F-006008055BB5}` resolves to
**JumpjetLocomotionClass** constructed at `0x0054AC40` [BINARY-VERIFIED —
Ghidra-labeled, header comment confirms CLSID + "Used by 9 units"]. The
class is **multi-interface COM-style** with three vtables initialized in the
constructor:

```c
*param_1     = &JumpjetLocomotionClass__IUnknown_vtable;     // +0x0
param_1[1]   = &JumpjetLocomotionClass__ILocomotion_vtable;  // +0x4
param_1[6]   = &JumpjetLocomotionClass__IPiggyback_vtable;   // +0x18
```

This is the same class used by Siege Chopper, Hornet, and other hover-style
fliers — but **NOT** dropped infantry (parachuting uses no locomotor; see
JUMPJET RE doc R3-R4).

Per-tick `Process` (FUN_0054AEC0 — unlabeled but body-verified as the
state-machine dispatcher) dispatches to one of six state handlers based on
the state field (instance+0x50 in IUnknown raw view):

```c
switch (param_1[0x13]) {   // ILocomotion-view → IUnknown_this + 0x50
  case 0: FUN_0054b980(); break;
  case 1: FUN_0054ba30(); break;
  case 2: FUN_0054bd30(); break;
  case 3: FUN_0054bff0(); break;
  case 4: FUN_0054c550(); break;
  case 5: FUN_0054ca90(); break;
}
```

| State | Handler | Body | Role |
|-------|---------|------|------|
| 0 — Grounded | `0x0054B980` | 0xAE bytes | On ground, awaiting target. Zeros velocity (instance+0x70/+0x74/+0x78/+0x7C); copies `instance+0x2C` (CruiseHeight cache from `JumpjetHeight=500`) → `instance+0x80` (climb target). Sets `instance+0x50 = 1` (transition to state 1) [BINARY-VERIFIED audit 8] |
| 1 — Liftoff/Ascend | `0x0054BA30` | 0x2FC bytes | Rises to CruiseHeight. Pathfinder retry on cell-occupancy; transitions to state 2 (decel cruise) or 3 (long-range cruise). [BODY DEFERRED, entry verified] |
| 2 — Decelerating cruise | `0x0054BD30` | 0x2B4 bytes | Approach destination; hand off to state 3 for long range, state 4 to land. [BODY DEFERRED, entry verified] |
| 3 — Long-range horizontal cruise | `0x0054BFF0` | 0x55B bytes (biggest of the cruise handlers) | Speed ramps with remaining distance. [BODY DEFERRED, entry verified] |
| 4 — Descend/land | `0x0054C550` | 0x533 bytes | Altitude bleeds off; on altitude==0 finalize landing, set state→0. For Rocketeer's `BalloonHover=yes`, this state is generally not entered between orders. [BODY DEFERRED, entry verified] |
| 5 — Emergency abort | `0x0054CA90` | 0x624 bytes (biggest, matches abort/crash complexity) | Triggered when destination invalidates mid-flight or unit takes lethal damage. Plays `CrashingSound`; for `Crashable=yes` triggers the AirDeathStart/Falling/Finish sequence. [BODY DEFERRED, entry verified] |
| 6 — Terminal | (no handler) | — | Sentinel — Process case-6 short-circuits ✅ |

Per-unit override fields (`JumpjetSpeed`/`Climb`/`Crash`/`Accel`/`TurnRate`/`Height`/`Wobbles`/`Deviation`)
overlay the global `[JumpjetControls]` defaults at RulesClass+0x40C..+0x438.
Constructor copies CruiseHeight (`JumpjetHeight=500`) into instance+0x2C; this
single byte-aligned cache is what drives:
- State 0 → 1 climb target (instance+0x80)
- `In_Which_Layer` (vtable slot 29, `0x0054B8D0`) z-sort: altitude < instance+0x2C → layer 3 (Top_Low); altitude ≥ instance+0x2C → layer 4 (Top_High); altitude 0 or invisible → layer 2 (Ground)

### 2. BalloonHover=yes — "never land" semantics

INI key `BalloonHover` is read by `TechnoTypeClass__ReadINI` at `0x00714D95`
(verified via xref to string at `0x00843838`). Sets a type-class flag bit
that, when checked by Process / state 4, prevents the auto-descend
sequence: idle Rocketeer floats at CruiseHeight indefinitely instead of
landing between orders. **Critical for visual parity** — gamemd Rocketeers do
not land at all in normal play.

### 3. ConsideredAircraft=yes — AI threat routing

INI flag that makes the AI's threat / weapon-pick logic treat this unit as
an aircraft for target-acquisition purposes. AA weapons (Flak Cannon's
`AAFireOnly` arc, Patriot SAM lock-on, Aegis Cruiser lock) engage Rocketeer
as a flyer rather than as ground infantry. Without this flag, AA defenses
would ignore the Rocketeer and ground anti-infantry weapons would target it
— breaking the AA-vs-Rocketeer game-balance contract.

### 4. HoverAttack=yes — fire-while-flying

INI flag enabling the Rocketeer to fire its weapon while loitering in
mid-air. Standard FlyLocomotion aircraft (Black Eagle, Harrier, Kirov) must
typically perform an attack run — descend, point at target, fire, climb away.
HoverAttack=yes bypasses the attack-run requirement; the unit fires from a
stationary hover. Visual is the `FireFly=370,6,6` artmd sequence.

The commented-out `;OpportunityFire=yes` reflects an engine quirk: the
jumpjet locomotor turn-toward-target logic for opportunity-fire ends up
rotating the unit backwards through its firing arc (because the chase
heading lags behind facing), so opportunity-fire is intentionally disabled.

### 5. Fly / Hover / FireFly sequence selection via locomotor CLSID match

Per JUMPJET RE R3.3 / R4.1: `FootClass::Locomotion_AI @ 0x00520F40` checks
the unit's primary locomotor against the JumpjetLoco CLSID. On match, it
dispatches sequence 0x17 (`Hover`) when stationary and 0x18 (`Fly`) when
moving. The `FireFly` sequence is selected separately when the unit fires
while in-flight. **This match is universal across jumpjet-locomotor units**
— so Rocketeer, Siege Chopper, Hornet, and Soviet Lunar Troops (whose art
shares `RocketeerSequence`) all use the same flight-pose selection logic.

### 6. CrashingSound vs ImpactLandSound split

Two distinct sound hooks for the death sequence:
- `CrashingSound=RocketeerDie` — played at the **start** of the fall (state
  5 entry) when the Rocketeer is shot down. Sound `irocdiea`
- `ImpactLandSound=RocketeerCrash` — played at the **terminal impact** with
  the ground after the AirDeathFinish animation. Sound `iroccraa/b`

The `DieSound=` field is intentionally empty — flight-capable Crashable
units use the in-air / impact pair instead of a single instant death sound.

### 7. JumpJet=yes — type flag

INI key `JumpJet` (note camel-case JumpJet, distinct from `Jumpjet*` settings)
read by `TechnoTypeClass__ReadINI` at `0x007151EC` (verified via xref to
string at `0x00843640`). Sets a type bit that enables jumpjet-only mission
paths and the Fly/Hover sequence eligibility — required to actually use the
JumpjetLocomotionClass meaningfully.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("JumpJet\|JumpjetSpeed\|... \|BalloonHover")` | 16 strings — confirms every INI key on the type is hardcoded-recognized: `JumpJet`, `JumpjetSpeed/Climb/Crash/Accel/Height/Wobbles/Deviation/NoWobbles/TurnRate`, `JumpjetControls`, `BalloonHover`, `TiltCrashJumpjet`, plus class RTTI strings `.?AVJumpjetLocomotionClass@@` and `.?AV?$TClassFactory@VJumpjetLocomotionClass@@@@` |
| `search_functions_enhanced(name_pattern="Jumpjet\|Hover\|BalloonHover")` | 8 hits: 3 `JumpjetLocomotionClass__Constructor` entries (ctor 0x0054AC40 / destructor 0x0054AD00 / scalar-deleting-destructor 0x0054DFA0), `RulesClass__ReadJumpjetControls @ 0x006743D0`, and 4 `HoverLocomotionClass__*` functions (a separate locomotor class used by hover **vehicles** like Disk/Hover MLRS — **not** Rocketeer; Rocketeer uses JumpjetLoco) |
| `get_xrefs_to(0x00843838)` (= "BalloonHover" string) | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714D95` (DATA reference) — confirms BalloonHover is a TechnoTypeClass flag, parsed once at INI load |
| `get_xrefs_to(0x00843640)` (= "JumpJet" string) | Sole xref from `TechnoTypeClass__ReadINI @ 0x007151EC` (DATA reference) — confirms JumpJet=yes is a TechnoTypeClass flag, same parse path |

**HoverLocomotionClass vs JumpjetLocomotionClass clarification:** Despite both
appearing in the function search, these are **different classes** for
different unit categories. Rocketeer (JUMPJET) uses JumpjetLocomotion. The
HoverLocomotion family is used by hover-type vehicles (e.g., Disk, Robot
Tank, Hover MLRS) — separate state machine, separate constructor. The
`SpeedType=Hover` on `[JUMPJET]` refers to the **terrain-speed-table column**
of the same name, not to HoverLocomotionClass.

---

## Ghidra audit log (audit iteration 8 — 2026-05-18)

Independent re-verification pass against gamemd.exe. ~14 function entry-point
verifications + decompiles spanning the JumpjetLocomotionClass constructor,
state-machine dispatcher, state-0 handler, In_Which_Layer, RulesClass
JumpjetControls reader, and FootClass::Locomotion_AI (the CLSID-match
sequence selector). All key INI keys verified by xref.

### Function entry points re-verified

| Doc claim | Verified at exact address |
|-----------|---------------------------|
| `JumpjetLocomotionClass::Constructor @ 0x0054AC40` | ✅ exact (Ghidra-labeled, body 0x0054ac40–0x0054acf8). Header comment confirms CLSID `{92612C46-F71F-11D1-AC9F-006008055BB5}` and "Used by 9 units" |
| `JumpjetLocomotionClass::Constructor (2nd) @ 0x0054AD00` | ✅ exact (Ghidra labels it as a Constructor too — body 0x0054ad00–0x0054ad2f, only 47 bytes — likely the no-arg / placement variant rather than a "destructor" as the doc claimed; doc claim of "destructor at 0x0054AD00" is [INFERRED / LIKELY-WRONG] — needs body decompile to confirm) |
| `JumpjetLocomotionClass scalar-deleting-destructor @ 0x0054DFA0` | NOT verified this pass — DEFERRED |
| `Process @ 0x0054AEC0` | ✅ exact (body 0x0054aec0–0x0054b19b, contains the `switch (state) { case 0..5 }` dispatch as documented) |
| `State 0 — Grounded @ 0x0054B980` | ✅ exact (body 0xAE bytes — caches +0x2C→+0x80, sets state=1 ✅) |
| `State 1 — Liftoff @ 0x0054BA30` | ✅ exact (body 0x2FC bytes) |
| `State 2 — Decel cruise @ 0x0054BD30` | ✅ exact (body 0x2B4 bytes) |
| `State 3 — Long-range cruise @ 0x0054BFF0` | ✅ exact (body 0x55B bytes) |
| `State 4 — Descend/land @ 0x0054C550` | ✅ exact (body 0x533 bytes) |
| `State 5 — Emergency abort @ 0x0054CA90` | ✅ exact (body 0x624 bytes) |
| `In_Which_Layer @ 0x0054B8D0` | ✅ exact (body 0xAC bytes — z-sort layer decision based on altitude vs cached field at instance+0x28 from ILocomotion view = +0x2C IUnknown raw) |
| `RulesClass::ReadJumpjetControls @ 0x006743D0` | ✅ exact (Ghidra-labeled, body 0x121 bytes — full `[JumpjetControls]` defaults reader) |
| `FootClass::Locomotion_AI @ 0x00520F40` | ✅ exact (Ghidra-labeled, body 0x3D2 bytes — contains the JumpJet flag check + sequence dispatch) |

### Constructor body — vtable layout BINARY-VERIFIED

```c
JumpjetLocomotionClass::Constructor(this) {
    LocomotionClass::Constructor();   // parent ctor
    this[0x10] = g_NullCoord_Jumpjet_X;
    this[0x11] = g_NullCoord_Jumpjet_Y;
    this[0x12] = g_NullCoord_Jumpjet_Z;
    *(byte*)(this + 0x13) = 0;        // byte at 0x4C: state byte (low byte)
    this[0x14] = 0;                   // 0x50: state field (int)
    FUN_004c91e0(g_RulesClass_Instance + 0x40c);  // initialize from Rules.TurnRate
    this[0x1c]..this[0x25] = 0;       // zero various state fields
    *(this+0)  = &JumpjetLocomotionClass__IUnknown_vtable;
    *(this+1)  = &JumpjetLocomotionClass__ILocomotion_vtable;
    *(this+6)  = &JumpjetLocomotionClass__IPiggyback_vtable;
    RateTimer__Set(&local_4 = 0x4000);    // some hover-related rate
    FacingClass__UpdateFacing(0x4000);
    return this;
}
```

Three vtables at +0x0 / +0x4 / +0x18 confirm the COM multi-interface layout
(IUnknown / ILocomotion / IPiggyback). [BINARY-VERIFIED audit 8]

### State 0 handler — confirms instance offsets +0x2C, +0x50, +0x80

```c
FUN_0054B980(this) {
    if (vtable+0x10(this+4) /* ILocomotion */) {
        vtable+0x544(0, 0x3ff00000);  // 1.0 as double — facing reset
        RateTimer__Current(...);
        FacingClass__UpdateFacing(...);
        this[0x70] = 0; this[0x74] = 0; this[0x78] = 0; this[0x7C] = 0;  // zero velocity
        this[0x80] = this[0x2C];   // CACHE CruiseHeight → climb target ✅
        if (!FUN_0053a130()) {     // some "in-motion" check
            psVar3 = vtable+0x2f4(...);  // get target coords
            if (target == NullCoord_X/Y_globals) {
                FUN_004134a0(this[0xC]);   // owner cleanup
            }
            this[0x50] = 1;        // STATE → 1 (Liftoff)  ✅
        }
    }
}
```

This BINARY-VERIFIES the doc claims:
- **Locomotor instance +0x2C = CruiseHeight cache** ✅
- **Locomotor instance +0x50 = state field** (with the 4-byte ILocomotion-view shift accounted for: Process reads `param_1[0x13]` = byte 0x4C from ILocomotion this, which = IUnknown_this + 0x50)
- **Locomotor instance +0x80 = climb target altitude** ✅

The "instance offsets shift by 4 bytes between IUnknown-raw view and
ILocomotion view" is a consequence of the multi-interface vtable layout
(ILocomotion vtable lives at IUnknown+0x4, so an ILocomotion-typed `this`
sees fields at offsets 4 lower than the IUnknown-typed `this`). Ghidra
mixes the views across functions, which can look like contradictory offsets
but is consistent once the interface base is fixed.

### FootClass::Locomotion_AI — sequence dispatch BINARY-VERIFIED

The function reads the JumpJet flag at TechnoTypeClass+0xD94, performs a
GUID comparison against `DAT_007E9AC0` (the JumpjetLocomotionClass CLSID
constant), and dispatches:

```c
if (CLSID_MATCHES_JUMPJET) {
    if (*(byte *)((int)this + 0x68d) != 0) return;     // skip-anim flag
    if (*(double *)(this + 0x15e) <= velocity_threshold) {
        vtable+0x558(0x17, 0, 0);   // sequence 0x17 = Hover  ✅
    } else {
        vtable+0x558(0x18, 0, 0);   // sequence 0x18 = Fly    ✅
    }
    return;
}
// Non-jumpjet path:
if (*(byte *)((int)this + 0x6db) == 0) {
    vtable+0x558(3, 0, 0);   // sequence 3 = Walk
} else {
    vtable+0x558(6, 0, 0);   // sequence 6 = Crawl
}
```

This BINARY-VERIFIES:
- **TechnoTypeClass+0xD94 = JumpJet flag (byte)** (gate for entering the
  jumpjet sequence branch)
- **Sequence 0x17 = Hover** (when velocity ≤ threshold)
- **Sequence 0x18 = Fly** (when velocity > threshold)
- **CLSID match via byte-array compare** against the GUID constant at
  `DAT_007E9AC0` — confirms the per-unit locomotor identity check happens
  through the IPiggyback `QueryInterface`-like virtual at `param_1[0x19d]`

(The `FireFly=370,6,6` sequence — used when firing in flight — is NOT
selected in Locomotion_AI; it's picked elsewhere in the rendering pipeline
when both "in flight" and "firing" conditions hold.)

### RulesClass JumpjetControls reader — defaults BINARY-VERIFIED

`RulesClass__ReadJumpjetControls @ 0x006743D0` reads the `[JumpjetControls]`
section and stores defaults at:

| Rules offset | Field | Type | INI key |
|--------------|-------|------|---------|
| +0x40C | TurnRate (int) | int | `TurnRate=` |
| +0x410 | Speed (int) | int | `Speed=` (default per doc 14) |
| +0x418 | Climb (double, 8 bytes) | double | `Climb=` |
| +0x420 | CruiseHeight (int) | int | `CruiseHeight=` |
| +0x428 | Acceleration (double) | double | `Acceleration=` |
| +0x430 | WobblesPerSecond (double) | double | `WobblesPerSecond=` |
| +0x438 | WobbleDeviation (int) | int | `WobbleDeviation=` |

All BINARY-VERIFIED ✅. Doc claim of `[JumpjetControls]` defaults block at
RulesClass+0x40C..+0x438 is exact.

### TechnoTypeClass offsets BINARY-VERIFIED (this audit)

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| +0x390 | HoverAttack (byte) | `HoverAttack=` | Read via `(char)param_1[0xE4]` |
| +0xD6A | BalloonHover (byte) | `BalloonHover=` | Read via `*(byte*)((int)param_1 + 0xd6a)` |
| +0xD70 | JumpjetSpeed (int) | `JumpjetSpeed=` | Per-unit override |
| +0xD74 | JumpjetClimb (float-as-int) | `JumpjetClimb=` | Per-unit override |
| +0xD78 | JumpjetCrash (float-as-int) | `JumpjetCrash=` | Per-unit override |
| +0xD7C ± | JumpjetHeight / +adjacent params | `JumpjetHeight=`, `JumpJetAccel=`, `JumpJetTurnRate=`, `JumpjetWobbles=`, `JumpjetDeviation=` | Block of per-unit overrides — exact per-key offset within +0xD7C..+0xD90 not pinned individually this pass (DEFERRED — would require disassembly of CCINIClass calls to map by call order) |
| +0xD94 | JumpJet (byte) | `JumpJet=` | Type flag enabling jumpjet locomotor paths — read by FootClass::Locomotion_AI as the gate for sequence 0x17/0x18 dispatch |
| +0xD95 | Crashable (byte) | `Crashable=` | **NOT the same as ObjectType+0x22D Crushable** — Crashable is the "play crash sequence on death" flag for jumpjet/aircraft units; Crushable (audit 7) is the "can be crushed by vehicles" flag |
| +0xD96 | ConsideredAircraft (byte) | `ConsideredAircraft=` | AA-targeting routing flag |

### InfantryTypeClass offsets BINARY-VERIFIED (this audit)

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| (in flag chain) | Fearless (byte) | `Fearless=` | InfantryType-scope confirmed via xref @ 0x00524469. Exact offset within the InfantryType bool-chain DEFERRED (similar to Crawls/Engineer/Occupier — somewhere in +0xEBC..+0xECB range) |

### Parser-scope verifications (this audit, via INI key xrefs)

| INI key | Reader xref | Scope |
|---------|-------------|-------|
| `BalloonHover` | `TechnoTypeClass__ReadINI` @ 0x00714D95 | **TechnoType** ✅ |
| `JumpJet` | `TechnoTypeClass__ReadINI` @ 0x007151EC | **TechnoType** ✅ |
| `ConsideredAircraft` | `TechnoTypeClass__ReadINI` @ 0x00714FE9 | **TechnoType** ✅ |
| `HoverAttack` | `TechnoTypeClass__ReadINI` @ 0x0071255A | **TechnoType** ✅ |
| `Crashable` | `TechnoTypeClass__ReadINI` @ 0x0071520D | **TechnoType** ✅ |
| `JumpjetNoWobbles` | `TechnoTypeClass__ReadINI` @ 0x007151AC | **TechnoType** ✅ |
| `JumpjetHeight` | `TechnoTypeClass__ReadINI` @ 0x0071513F | **TechnoType** ✅ |
| `Fearless` | `InfantryTypeClass__ReadINI` @ 0x00524469 | **InfantryType** ✅ |

### Items NOT re-verified this pass (DEFERRED)

- **State 1–5 handler bodies** — entry-point addresses + body sizes verified,
  but the per-state logic (pathfinder retry in State 1, speed ramps in
  State 3, altitude integrator in State 4, crash sequence in State 5) was
  NOT independently decompiled this audit. The doc references the deep RE
  doc `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` for these; this audit
  pass confirms the entry points are correct but DOES NOT re-verify the
  body claims (e.g., "<0x14 leptons → state 4", "wobble math via
  DAT_007E44E8").
- **Scalar-deleting destructor @ 0x0054DFA0** — not verified.
- **`0x0054AD00` "destructor" claim** — Ghidra labels this function as a
  Constructor (47-byte body). The doc's "destructor at 0x0054AD00" claim
  is likely [INCORRECT] but full disambiguation requires decompiling the
  body (DEFERRED).
- **JumpJet locomotor instance field `+0x80` "climb target altitude"** —
  the value is set in State 0 to +0x2C (CruiseHeight cache), but the
  consumer that reads +0x80 to drive the actual climb integrator is in
  State 1 and was not decompiled.
- **BalloonHover consumer** — the byte is stored at +0xD6A but the runtime
  check that gates "never land" behavior (presumably in State 4 entry or
  Process step 8) was not decompiled.
- **Per-unit Jumpjet* offset pinning** (JumpjetClimb vs JumpjetHeight vs
  JumpJetAccel etc. within +0xD70..+0xD90) — only Speed/Climb/Crash exact
  offsets identified; the rest are in this block but exact per-key offset
  not pinned.
- **AirDeathStart/Falling/Finish sequence selection** — claimed to be
  driven by State 5; consumer not decompiled.

### Confidence summary

**HIGH** for: all 13 function entry-point addresses, Constructor body
(multi-interface vtable layout), Process state-machine dispatcher,
State 0 handler (CruiseHeight cache → climb target → state→1 transition),
RulesClass JumpjetControls offsets, FootClass::Locomotion_AI sequence
dispatch, and all 8 parser-scope verifications.

**MEDIUM** for the locomotor instance offsets — verified +0x2C, +0x50,
+0x80, +0xC owner via State 0 and Process, but the 4-byte
ILocomotion-view-vs-IUnknown-raw-view offset shift means cross-function
offset numbers must be normalized to one view before comparing. The doc's
offsets are all stated as "instance+0xN" — when reading Ghidra output,
note which interface the function is invoked through.

**LOW** for State 1–5 body claims (deep RE doc not re-verified end-to-end
this pass); the `0x0054AD00 = destructor` claim contradicted by Ghidra
label (it's labeled Constructor).

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `MovementZone=Fly` with designer comment "This needs to be None, like aircraft" | **Still active in YR** — Fly MZ is consumed by the pathfinder | OK; the comment is historical regret, not a TS-only path |
| `TiltCrashJumpjet=yes` (on other types, not JUMPJET itself) | YR-active for Siege Chopper. Not set on Rocketeer | OK |
| `Locomotor={92612C46-...}` (JumpjetLocoClass) | All 6 state handlers run in normal YR skirmish | OK |
| Fog-of-war reveal branch in `Process` step 8 | **TS-legacy** — gated on `SpecialFlags & 0x1000` (FogOfWar) which defaults OFF in YR per CLAUDE.md. Dormant in retail | Will not affect parity |
| Wobble math (`JumpjetWobbles`/`JumpjetDeviation`) | **YR-active but disabled by per-unit `JumpjetNoWobbles=yes`** on Rocketeer — the runtime sine-wave is skipped; visual hover comes from the artmd Hover frame loop. Both code paths are present, the data-driven flag selects |
| `;OpportunityFire=yes` commented-out | Engine quirk noted by designer; not TS-specific. The comment documents WHY it's disabled (face-target rotation interrupts firing) | OK |

No purely-TS-only behavior found on the JUMPJET type itself.

---

## Cross-references

- **Related units** sharing the JumpjetLocomotion CLSID (per JUMPJET RE §1):
  - `[ROCK]` / `[JUMPJET]` — Rocketeer (this doc)
  - `[SCHP]` Siege Chopper (Yuri) — air/ground transformable
  - `[HORV]` Hornet — released by Aircraft Carrier
  - Plus ~6 more dormant/special-purpose RA2-base entries
- **Related units sharing the `RocketeerSequence` art** (per artmd `Sequence=`):
  - `[ROCK]` Rocketeer
  - `[LUNR]` Soviet Lunar Troops (campaign easter egg) — same art, different FLH
- **Related rules sections**:
  - `[JumpjetControls]` — global defaults at `rulesmd.ini:571` (overridden per-unit on JUMPJET)
  - `[CombatDamage]` — Veteran/Elite ability multipliers
  - `[General]` — global combat constants
- **Counter-units / interactions**:
  - **AA defenses**: Flak Cannon `[NAFLAK]`, Patriot Missile `[GAPILL]`-variant?, Aegis Cruiser `[AEGIS]`, Gattling Cannon `[YAGGUN]`. All engage Rocketeer because `ConsideredAircraft=yes`
  - **Anti-infantry weapons**: G.I. `[E1]`, Initiate `[INIT]`, etc. will NOT auto-target flying Rocketeer (ConsideredAircraft routes it away from anti-infantry priority)
- **Soundmd cross-links**:
  - `RocketeerMoveLoop` — also used as ambient sound for the Soviet Lunar Troops `[LUNR]` (via Image=LUNR and shared sequence)

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [JUMPJET]` | 3916-3971 (56 lines) | All 50 set keys covered (one commented `;OpportunityFire`, one commented `;Sight=6` noted) |
| `artmd.ini [ROCK]` | 112-119 (8 lines) | All keys covered |
| `artmd.ini [RocketeerSequence]` | 14557-14586 (30 lines) | All 25 slots covered including 2 commented variants |
| `rulesmd.ini [JumpjetControls]` | 571-578 (8 lines) | All 7 default keys covered (cross-referenced as fallback) |
| `rulesmd.ini [20mm]` | 22976-22983 (8 lines) | All keys covered |
| `rulesmd.ini [20mmE]` | 24824-24831 (8 lines) | All keys covered |
| `rulesmd.ini [SSA]` | 26509-26517 (9 lines) | All keys covered (incl. commented history line) |
| `rulesmd.ini [Invisible3]` | 25359-25363 (5 lines) | All keys covered |
| `soundmd.ini` Rocketeer voices | RocketeerSelect, Move, AttackCommand, Fear, Attack, MoveLoop, Die, Crash | All 8 covered |
| Hardcoded behavior | JumpjetLocomotion 6-state machine + BalloonHover + ConsideredAircraft + HoverAttack + Fly/Hover/FireFly selection + CrashingSound/ImpactLandSound split + JumpJet type flag | Covered with deep RE doc referenced |
| Ghidra searches performed against ID | 4 distinct queries (1 strings + 1 function search + 2 xref lookups) | Logged inline |
| TS-legacy filter | Applied; FogOfWar branch flagged as dormant; wobble math noted as data-disabled | Done |
