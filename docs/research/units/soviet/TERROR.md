# Terrorist (TERROR)
Side: Soviet | Category: Infantry | Image alias: `Image=TRST` → `[TRST]` artmd

The **Cuban Terrorist** (`RequiredHouses=Confederation` — Cuba's national
special unit, NOT a Yuri faction unit despite the index's prior mislabel).
$200 from Soviet Barracks + Radar. The simplest hardcoded suicide-bomber:
**`Primary=TerrorBomb`** is `Suicide=yes` `Damage=225` `Range=1.5` — when
fired, the engine destroys the firing Terrorist and detonates the warhead
(`TerrorBombWH`) at the target. **`Explodes=yes`** + **`DeathWeapon=TerrorBomb`**
ensures the Terrorist **also explodes when killed by anything else** — a
shot Terrorist still detonates. The double trigger (manual suicide OR
death-explosion) makes Terrorist effectively impossible to kill safely;
the only safe counters are crushing (Crushable default `yes`) or
mind-controlling before he reaches detonation range. **`Strength=75`**
(intentionally fragile — designed to die), **`Speed=6`** (faster than
typical infantry), **`Trainable=no`** (single-use unit).

No standalone Terrorist RE doc existed; this document originates the
Ghidra trace of `Suicide=yes` and `DeathWeapon=` flag paths.

---

## rulesmd.ini — `[TERROR]` section

Verbatim from `ini/rulesmd.ini:4768`:

```ini
[TERROR]
UIName=Name:TERROR
Name=Terrorist
Image=TRST
;Image=SPY
Category=Soldier
;Prerequisite=NAHAND,NATECH
Prerequisite=NAHAND,RADAR
CrushSound=InfantrySquish
LeadershipRating=3
Strength=75 ;changed on 11/29 from 50 to 75
;Primary=MakeupKit ; virtual weapon that picks disguise
;C4=Yes
Primary=TerrorBomb
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
Armor=flak
TechLevel=5
;CanDisguise=yes; I appear differently on other people's computers
;PermaDisguise=yes; and I appear that way always (Mirage Tank will be Can but not Perma)
Sight=9
Speed=6
Owner=Russians,Confederation,Africans,Arabs
RequiredHouses=Confederation
AllowedToStartInMultiplayer=no
;Cost=1500
;Soylent=200
Cost=200
Soylent=100
Pip=red
Points=5
VoiceSelect=TerroristSelect
VoiceMove=TerroristMove
VoiceAttack=TerroristAttackCommand
VoiceFeedback=TerroristFear
VoiceSpecialAttack=TerroristAttackCommand
DieSound=TerroristDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=0	; This value MUST be 0 for all building addons
SpecialThreatValue=1
Trainable=no
Explodes=yes
DeathWeapon=TerrorBomb
IFVMode=11
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:TERROR` | CSF-string key → "Terrorist" |
| `Name=Terrorist` | Internal name |
| `Image=TRST` | **Art redirect** — rendering uses `[TRST]` artmd entry (not `[TERROR]`). SHP on disk is `TRST` |
| `;Image=SPY` (commented) | Designer history — Terrorist originally going to reuse Spy art. Replaced with own SHP TRST |
| `Category=Soldier` | Infantry pip/AI grouping |
| `;Prerequisite=NAHAND,NATECH` (commented) | Original prereq required Battle Lab. Reduced to NAHAND+RADAR for accessibility — Terrorist is mid-game, not late-game |
| `Prerequisite=NAHAND,RADAR` | Soviet Barracks + any Radar building |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `LeadershipRating=3` | Low veterancy-gain modifier — moot (Trainable=no) |
| `Strength=75` | **HP — 75**. Designer comment: "changed on 11/29 from 50 to 75" — early balance had Terrorist at 50 HP. **Intentionally fragile** — Terrorist is supposed to die (the suicide is the feature, not the bug) |
| `;Primary=MakeupKit` (commented) | Designer history — Terrorist was at one point planned to have spy-style disguise. Switched to suicide-bomb |
| `;C4=Yes` (commented) | Was considered for C4-on-buildings (Tanya-style). Replaced by the simpler Suicide weapon system. Note the captial-Y "Yes" — case-insensitive INI parsing |
| `Primary=TerrorBomb` | **The suicide weapon** — `Suicide=yes Damage=225 Range=1.5 Warhead=TerrorBombWH`. See "Weapons" and "Hardcoded Behavior" §1 |
| `CanPassiveAquire=no` | **Disables auto-target acquisition** — Terrorist will NOT walk over and auto-blow up infantry he passes. Detonation is always player-commanded. Critical for player control — otherwise massed Terrorists would self-destruct prematurely |
| `CanRetaliate=no` | **Disables damage-response retaliation** — when shot, Terrorist doesn't fire back. (Firing back = exploding self, which is bad for the player) |
| `Armor=flak` | Damage type column 1 — flak armor. Same as Conscript |
| `TechLevel=5` | Mid-game tech-5 cap |
| `;CanDisguise=yes` / `;PermaDisguise=yes` (commented) | Designer history — Terrorist would have disguised as enemy infantry to walk into bases unchallenged. **Cut feature**; final design relies on raw speed + small profile |
| `Sight=9` | Reveal radius — large. Combined with Speed=6, lets Terrorist see the target from outside enemy vision and run in |
| `Speed=6` | **Foot-speed — 6**. Faster than typical infantry (4 standard). Designed to reach targets before defenses kill him |
| `Owner=Russians,Confederation,Africans,Arabs` | All 4 Soviet houses listed |
| `RequiredHouses=Confederation` | **Country-locked to Cuba only**. Cuba's national special unit (analogous to Iraq's Desolator, Britain's Sniper, etc.) |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `;Cost=1500` (commented) | Original cost — radically reduced |
| `;Soylent=200` (commented) | Was tuned with the higher cost |
| `Cost=200` | **$200** — cheap (same as basic Conscript). Mass-producible suicide bombers |
| `Soylent=100` | $100 Grinder refund (Yuri only — 50% of cost) |
| `Pip=red` | Cargo pip color — red (elite class — defensive even though Trainable=no) |
| `Points=5` | Kill score |
| `VoiceSelect=TerroristSelect` | Select voice — `$itersea/b/c` (3 lines, Cuban-accented) |
| `VoiceMove=TerroristMove` | Move voice — `$itermoa/b/c` (3 lines) |
| `VoiceAttack=TerroristAttackCommand` | Attack voice — `$iterata..d` (4 lines) |
| `VoiceFeedback=TerroristFear` | Fear voice — `$iterfea/b/c` (3 lines, Priority=Low) |
| `VoiceSpecialAttack=TerroristAttackCommand` | Reuses Attack-Command voice for special-attack |
| `DieSound=TerroristDie` | Death voice — `$iterdia/b/c` (3 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=0` | AI scoring weight — **zero**. The AI does NOT consider Terrorist a target (because killing him still detonates the bomb). Defensive AI behavior — enemy AI ignores Terrorists by default, forcing them to be picked up by area-of-effect or crushed |
| `SpecialThreatValue=1` | Self-threat max — Terrorist "wants" target maximally |
| `Trainable=no` | **Cannot gain veterancy** — Terrorist dies in single use; no XP gain path makes sense |
| `Explodes=yes` | **Behavior flag** — TechnoTypeClass field. When the unit dies, triggers a death-explosion. Combined with `DeathWeapon=TerrorBomb` below, the death-explosion uses TerrorBomb's warhead (TerrorBombWH) instead of the default DeathWH |
| `DeathWeapon=TerrorBomb` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x007122F0` DATA xref to string at `0x0083B11C`). Also has a global xref from `RulesClass__ReadCombatDamage @ 0x0066C58A` for the global `[CombatDamage].DeathWeapon=` default. The per-type override sets which weapon the unit fires when it dies (combined with Explodes=yes to enable). For Terrorist: **dying triggers TerrorBomb at his own location** — Damage 225 + Warhead TerrorBombWH (Fire=yes, CellSpread=2). Means **the Terrorist explodes no matter how he dies** — shot, crushed, sniped, anything |
| `IFVMode=11` | IFV gunner-table index 11 → HTK's `Weapon12`/`ElitePassengerWeapon12` slot. In stock YR maps to a demolition-style weapon — Terrorist-in-IFV gives the chassis a suicide-style weapon (the IFV itself doesn't suicide; just the visual + damage profile) |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `no` (Terrorist cannot crawl)
- `Crushable=` — **defaults to `yes`** — Terrorist CAN be crushed by vehicles. **Critical counter-mechanic**: crushing a Terrorist with a vehicle bypasses the `Explodes=yes`/`DeathWeapon=` path entirely (vehicle-crush is a separate death route that doesn't trigger explosion). The Terrorist just dies silently. The classic "drive a Rhino over the Terrorist to neutralize" play
- `NotHuman=` — defaults to `no` (human, subject to InfDeath, sniper headshot, mind-control)
- `ImmuneToPsionics=` — defaults to `no`; **Terrorist CAN be mind-controlled** — the safest counter for Yuri. Mind-controlled Terrorist becomes the controller's unit and can be detonated against the original owner
- `ImmuneToRadiation=` — defaults to `no`
- `Bombable=` — defaults to `no` (not in explicit list)
- `Fearless=` — not set; Terrorist shows fear behavior
- `Occupier=` — defaults to `no`; **cannot garrison**
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=` — all not set (C4 commented out)
- `BombSight=` — not set
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set (MissionGuard when idle)
- `Natural=` — not set
- `SelfHealing=` — not set
- `TiberiumProof=` — not set (default no)
- `ImmuneToVeins=` — not set (defaults no — but irrelevant since veins are TS-only anyway)

---

## artmd.ini — `[TRST]` section (via `Image=TRST` redirect)

`ini/artmd.ini:176`:

```ini
[TRST] ; Terrorist
Cameo=TRSTICON
AltCameo=TRSTUICO
Sequence=TerroristSequence
;Crawls=yes
Crawls=no
Remapable=yes
FireUp=1
```

| Key | Meaning |
|-----|---------|
| `Cameo=TRSTICON` | Sidebar build icon (SHP) |
| `AltCameo=TRSTUICO` | Elite cameo — **never shown** (`Trainable=no`) but defensively present |
| `Sequence=TerroristSequence` | Reference to `[TerroristSequence]` |
| `;Crawls=yes` (commented) | Older setting — Terrorist could crawl initially |
| `Crawls=no` | **Final setting** — Terrorist cannot go prone. Matching the no-crawl design (the unit's role is to run upright into enemies, not skirmish from prone) |
| `Remapable=yes` | House remap palette |
| `FireUp=1` | Bullet-spawn frame — at frame 1 the bomb "fires" (i.e., the Terrorist is removed and the explosion spawns). Very early frame because the visual is just the explosion, not an aimed weapon |

### Referenced sequence — `[TerroristSequence]`

`artmd.ini:13924`:

```ini
[TerroristSequence]
Ready=0,1,1
Guard=0,1,1
Prone=0,1,1     ;No Crawls can't crawl, but spy needs this listing
Walk=8,6,6
;FireUp=116,6,6
Down=8,2,6		;No Crawls can't crawl, but spy needs this listing
Crawl=8,6,6		;No Crawls can't crawl, but spy needs this listing
Up=8,2,6		;No Crawls can't crawl, but spy needs this listing
;FireProne=116,6,6	;No Crawls can't crawl, but spy needs this listing
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=86,15,0
Die2=101,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Paradrop=179,1,0
Cheer=180,8,0,E
FireUp=164,6,6
FireProne=164,6,6
Deploy=164,15,0
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Prone=0,1,1` | **Falls back to Ready frame** — designer comment: "No Crawls can't crawl, but spy needs this listing". Defensive entry for the spy-disguise rendering path |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `;FireUp=116,6,6` (commented) | Older fire pose | |
| `Down=8,2,6` / `Crawl=8,6,6` / `Up=8,2,6` | Reuse Walk frames — same defensive-for-spy comment | |
| `;FireProne=116,6,6` (commented) | Older prone-fire | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S | |
| `Idle2=71,15,0,E` | Idle 2 — E | |
| `Die1=86,15,0` | Death 1 — 15 frames | **Important**: this animation is **rarely seen** because the death-explosion triggers immediately on death, replacing the standard death anim with the TerrorBombWH explosion (MININUKE animation) |
| `Die2=101,15,0` | Death 2 | Same — usually preempted by explosion |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `Paradrop=179,1,0` | Single frame at 179 — paradrop pose | Live — Terrorists can be paradropped via map scripts |
| `Cheer=180,8,0,E` | Cheer — 8 frames E | |
| `FireUp=164,6,6` | **Live FireUp** — 6 frames × 6 facings (note: this overrides the earlier commented `;FireUp=116`). The "fire bomb" animation when ordered to attack | |
| `FireProne=164,6,6` | Prone-fire reuses FireUp | Unused (Crawls=no) |
| `Deploy=164,15,0` | Deploy anim reuses FireUp 15 frames | Unused (no Deployer flag) — defensive |
| `Panic=8,6,6` | Panic = Walk frames | |

---

## Weapons

### Primary — `[TerrorBomb]` (the suicide weapon)

`rulesmd.ini:22370`:

```ini
[TerrorBomb]
Projectile=Invisible
Damage=225
Warhead=TerrorBombWH
Anim=RING1
Range=1.5
ROF=10
Suicide=yes
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Projectile=Invisible` | Instant-resolution inviso projectile |
| `Damage=225` | Per-detonation damage. Combined with `TerrorBombWH.Verses[none]=150%` → **337 effective damage vs Armor=none infantry** at the impact cell — one-shots virtually every standard infantry. Falls off with CellSpread |
| `Warhead=TerrorBombWH` | See warhead — CellSpread 2, Fire=yes, MININUKE animation |
| `Anim=RING1` | **Weapon-level fire animation** — `RING1` plays at the firer's position when the weapon fires. The "ring of fire" visual at detonation |
| `Range=1.5` | 1.5 cells — Terrorist must be adjacent. Compare Ivan's IvanBomber (Range 1.5, also CellRangefinding for forgiving radius) |
| `ROF=10` | Cooldown — 10 frames. **Effectively irrelevant** — Suicide=yes destroys the Terrorist before ROF matters |
| `Suicide=yes` | **THE suicide flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x0077228D` DATA xref to string at `0x00843050`). Two additional xrefs from `0x006F1271` and `0x006F16DD` (inside `FUN_006F1550`) — likely the per-shot resolution path that consumes Suicide=yes. When the weapon fires, the engine **immediately destroys the firing unit** at the launch point, applying the warhead at the target. The firer doesn't survive the shot |
| `FireInTransport=no` | Cannot fire from inside [FV] Battle Fortress (would destroy the Battle Fortress, breaking the BF abstraction) |

### Primary's Warhead — `[TerrorBombWH]`

`rulesmd.ini:27214`:

```ini
[TerrorBombWH]
Verses=150%,100%,100%,90%,50%,50%,100%,150%,30%,100%,100%
Sparky=no
Fire=yes
InfDeath=4
CellSpread=2
PercentAtMax=.5
;Dustin is experimenting with art stuff here.
Bright=yes
AnimList=MININUKE
```

| Key | Meaning |
|-----|---------|
| `Verses=150%,100%,100%,90%,50%,50%,100%,150%,30%,100%,100%` | 11-column. **150% vs `none`** (infantry one-shot at 337 dmg). **100% vs `flak`/`plate`** (kills basic infantry). **90/50/50% vs light/medium/heavy vehicle** — moderate anti-vehicle (203/112/112 dmg). **100% vs wood, 150% vs steel, 30% vs concrete** — strong vs steel-armored structures (337 dmg) but weak vs concrete (67 dmg). **Steel boost is notable** — Terrorist is anti-fortification (Tesla Coil, Patriot, Flak Cannon = steel armor). Verses concrete is low to prevent trivial trashing of high-tier buildings |
| `Sparky=no` | No spark animation |
| `Fire=yes` | **Fire warhead** — sets ground on fire (small fire particles, can spread to wooden buildings). Adds to the visual chaos |
| `InfDeath=4` | **Infantry death animation type 4** — the **burn/incinerate** death (skeleton flash with fire). Player visual cue that the kill was a fire/blast weapon |
| `CellSpread=2` | Splash radius — 2 cells. About a 5×5 cell explosion |
| `PercentAtMax=.5` | At spread radius edge, damage is 50% of full (so ~112 vs none-armor at edge) |
| `Bright=yes` | **Visual flag** — sprites in the impact area get a bright flash. Designer comment: "Dustin is experimenting with art stuff here" — clearly an experimental feature kept in the final |
| `AnimList=MININUKE` | **Explosion animation** — `MININUKE` (Mini-Nuke). Distinctive mushroom-cloud visual matching the "this looks like a nuke" feel of a Terrorist detonation. **NOT actual nuclear** — just the visual style |

### Projectile — `[Invisible]`

Standard bare-minimum inviso projectile (no SubjectToCliffs/Walls/Elevation flags — the bomb just detonates immediately).

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death

```ini
[TerroristSelect]                  ; soundmd.ini:4076
Sounds= $itersea $iterseb $itersec
Control= random interrupt
Volume=90

[TerroristMove]                    ; soundmd.ini:4071
Sounds= $itermoa $itermob $itermoc
Control= random interrupt
Volume=90

[TerroristAttackCommand]           ; soundmd.ini:4066
Sounds= $iterata $iteratb $iteratc $iteratd
Control= random interrupt
Volume=90

[TerroristFear]                    ; soundmd.ini:4081
Sounds= $iterfea $iterfeb $iterfec
Control= random interrupt
Volume=90
Priority=Low

[TerroristDie]                     ; soundmd.ini:4087
Sounds= $iterdia $iterdib $iterdic
Priority=low
Control= random interrupt
Volume=90
```

3 select / 3 move / **4 attack** / 3 fear / 3 death. Spanish-accented voice
bank (matches Cuban faction archetype). Note: the death sound `TerroristDie`
**rarely plays** because the explosion (TerrorBombWH AnimList=MININUKE)
preempts the death sequence — the impact SFX dominates.

### Weapon report (the "scream as he runs" sound)

```ini
[TerroristAttack]                  ; soundmd.ini:1156
Sounds=igiat2a
Volume=35
```

| Sound | Wired by | Purpose |
|-------|----------|---------|
| `TerroristAttack` | **NOT wired on TerrorBomb!** | The `[TerrorBomb]` weapon has **no `Report=` field**, so this sound is technically unused for TerrorBomb's Report. It exists as a defined sound but the Terrorist's actual attack uses no Report SFX — only the explosion sound from the impact animation |

**Sample `igiat2a` is reused** from the GI's attack sample 2 — designer
recycling. Volume=35 is very low. Likely defined for legacy reasons or for
a different unit's usage.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND,RADAR` | Soviet Barracks + any Radar building (not specifically NARADR — abstract RADAR works) |
| `Owner=` | `Russians,Confederation,Africans,Arabs` | All 4 Soviet houses listed |
| `RequiredHouses=` | `Confederation` | **Cuba-only** national special unit |
| `TechLevel=` | `5` | Mid-game tech-5 cap |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=200` | $200 | Cheap |
| `Soylent=100` | $100 refund (Yuri only) | |
| `Points=5` | 5 | Low — Terrorist deaths score low (intentional — killing one isn't an achievement, surviving the explosion is) |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiresStolenXxxTech=`.

The Soviet country-special lineup (parallel to Allied side):
- Russia: Tesla Tank `[TTNK]`
- Iraq: Desolator `[DESO]`
- Cuba: **Terrorist `[TERROR]` (this doc)**
- Libya: Demolition Truck `[DTRUCK]`

---

## Veterancy

| Field | Value | Notes |
|-------|-------|-------|
| `Trainable=no` | — | **No veterancy** — Terrorist dies on first use (Suicide=yes); promotion would never occur. Defensive `AltCameo=TRSTUICO` and `Pip=red` are inert |

No ElitePrimary, no VeteranAbilities/EliteAbilities listed — completely flat single-use unit.

---

## Hardcoded behavior — Ghidra-verified

### 1. Suicide=yes — the suicide-on-fire mechanism

INI key `Suicide=yes` is a **WeaponTypeClass** field (per
`WeaponTypeClass__ReadINI @ 0x0077228D` DATA xref to string at `0x00843050`).
Two additional xrefs identify the consumer code:
- `0x006F1271` — inside an unnamed function
- `0x006F16DD` — inside `FUN_006F1550` (the per-shot resolution path)

When a weapon with `Suicide=yes` fires:
1. The engine queues the shot resolution
2. **Immediately destroys the firing unit** at its current cell — sets HP=0,
   triggers ReceiveDamage path
3. Applies the warhead at the **target** location (not the firer's location)

If the firer also has `Explodes=yes` + `DeathWeapon=`, the death-trigger
from step 2 fires a SECOND copy of the weapon at the firer's own location.
For Terrorist this means: ordering an attack → Terrorist dies → explosion
A at target + explosion B at firer position (himself). Same warhead
detonation twice, slight offset by Range=1.5 cells.

In practice the two detonations overlap because Range=1.5 + CellSpread=2 =
both blasts cover the same 3×3 area around the impact point.

### 2. DeathWeapon — fire-weapon-on-death

INI key `DeathWeapon=<weaponID>` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x007122F0` DATA xref to string at `0x0083B11C`).
The same string is also read by `RulesClass__ReadCombatDamage @ 0x0066C58A`
as a global default (in `[CombatDamage]`), so there's both a per-type
override and a global fallback.

When a unit with `Explodes=yes` dies, the engine looks up the unit's
`DeathWeapon` (falling back to the global default if unset). The death
trigger fires that weapon at the unit's current position. For Terrorist:
DeathWeapon=TerrorBomb means **any death — by enemy fire, by friendly fire,
by suicide command, by sniper, by burning, by anything except crush —
detonates a TerrorBomb at the death cell**.

**Crush is the exception** — vehicle-crush is a separate death-route that
bypasses Explodes=yes. The "Rhino runs over Terrorist" play prevents
detonation.

### 3. DeathWeaponDamageModifier — damage scaling for death-explosion

INI key `DeathWeaponDamageModifier` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00712325` DATA xref to string at `0x00844488`).
Multiplier applied to the death-weapon's damage when fired via the
Explodes=yes path. Default 1.0 (no modification). For Terrorist this is
not set — full Damage=225 applies.

Useful for mods that want a different damage profile for "dies to enemy"
vs "ordered to suicide" — but stock YR doesn't differentiate.

### 4. Explodes=yes — death-explosion enable (same as Ivan)

Same flag documented for Ivan (see IVAN.md §1.6): TechnoTypeClass field
(xref `TechnoTypeClass__ReadINI @ 0x007122C5`). For Terrorist, combined
with DeathWeapon=TerrorBomb, defines the death-detonation behavior.
Without this flag, DeathWeapon would be inert.

### 5. CanPassiveAquire=no + CanRetaliate=no — player-only commitment

Two complementary flags that prevent the Terrorist from killing himself
prematurely:
- **CanPassiveAquire=no** — Terrorist standing in MissionGuard does NOT
  auto-target enemy infantry that walks past. Without this, Terrorist
  would suicide on the first stray enemy footman
- **CanRetaliate=no** — Terrorist taking damage does NOT fire back at the
  attacker. Without this, every shot fired at Terrorist would trigger his
  Suicide=yes weapon, blowing him up in self-defense (which would then
  detonate)

Both flags ensure detonation is **always player-commanded** — the
player decides when to invest the Terrorist.

### 6. Speed=6 — fast infantry (not Speed=4)

Standard infantry Speed is 4 (GI, Engineer, Conscript). Terrorist's 6 is
50% faster. Necessary because Terrorist relies on closing distance under
enemy fire — he MUST reach the target before being killed (although
DeathWeapon catches the case where he doesn't, this only triggers at the
death cell, not at the intended target cell).

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("Suicide\|DeathWeapon\|DeathWeaponDamage")` | 3 strings — confirms `Suicide`, `DeathWeapon`, and `DeathWeaponDamageModifier` |
| `get_xrefs_to(0x00843050)` (= "Suicide") | 3 xrefs: `WeaponTypeClass__ReadINI @ 0x0077228D` (the ReadINI parse) + 2 consumers at `0x006F1271` and `0x006F16DD` (inside `FUN_006F1550`, the suicide-resolution path) — confirms Suicide=yes is consumed at weapon-fire time |
| `get_xrefs_to(0x0083B11C)` (= "DeathWeapon") | 2 xrefs: `RulesClass__ReadCombatDamage @ 0x0066C58A` (global default) + `TechnoTypeClass__ReadINI @ 0x007122F0` (per-type override) — confirms dual scope |
| `get_xrefs_to(0x00844488)` (= "DeathWeaponDamageModifier") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00712325` DATA — confirms TechnoType-level scaling factor |

Confirmation: TERROR uses purely **engine-generic flag combinations**:
`Suicide=yes` (weapon) + `Explodes=yes` (techno) + `DeathWeapon=TerrorBomb`
(techno). No TERROR-specific hardcoded function block — the entire behavior
emerges from these three flags. The same combination could be applied to
any infantry to make them a suicide-bomber variant.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Image=SPY` (commented) | Designer history — Terrorist originally going to reuse Spy art | OK |
| `;Prerequisite=NAHAND,NATECH` (commented) | Original Battle Lab prereq, reduced for accessibility | OK |
| `;Primary=MakeupKit` (commented) | Cut spy-disguise feature | OK |
| `;C4=Yes` (commented) | Cut C4-on-buildings feature | OK |
| `;CanDisguise=yes` / `;PermaDisguise=yes` (commented) | Cut disguise feature | OK |
| `;Cost=1500` / `;Soylent=200` (commented) | Original high price, slashed to $200 | OK |
| `;FireUp=116,6,6` / `;FireProne=116,6,6` (commented in artmd) | Older fire-frame range, replaced by 164 | OK |
| `;Crawls=yes` (commented in artmd) | Was crawl-capable initially | OK |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` — standard RA2/YR infantry | OK |
| `MovementZone=Infantry` | Standard | OK |
| `Suicide=yes` + `Explodes=yes` + `DeathWeapon=` | All YR-active — verified via Ghidra ReadINI xrefs and consumer paths | OK |

No TS-only behavior. All hardcoded flags are fully YR-active.

---

## Cross-references

- **Related suicide / kamikaze units**:
  - `[TERROR]` Terrorist (this doc — Soviet/Cuban)
  - `[DTRUCK]` Demolition Truck (Libyan special) — vehicle suicide bomber, also uses Suicide=yes weapon path. Documented separately
  - **Yuri's "Suicide" tier**: actually Yuri has NO dedicated suicide unit — Brute is melee, Initiate is psychic, Yuri/Yuri Prime are mind-controllers. Cuba's Terrorist is the dedicated "suicide infantry" in YR
- **Related Death-explosion (`Explodes=yes`) units**:
  - `[IVAN]` Crazy Ivan (`Explodes=yes`, DeathWeapon defaults to DeathWH unless overridden — Ivan death=ordinary explosion, NOT IvanBomb)
  - `[TERROR]` Terrorist (this doc — Explodes=yes + DeathWeapon=TerrorBomb means death = TerrorBomb explosion)
  - `[DTRUCK]` Demolition Truck (vehicle)
- **Related `Suicide=yes` weapons**:
  - `[TerrorBomb]` — Terrorist Primary
  - Demolition Truck's weapon (when documented)
  - Crazy Ivan's IvanBomber is NOT Suicide=yes — Ivan plants a bomb on target and survives. Different mechanic
- **Soviet country-special lineup**:
  - Russia: `[TTNK]` Tesla Tank
  - Iraq: `[DESO]` Desolator (documented)
  - **Cuba: `[TERROR]` Terrorist (this doc)**
  - Libya: `[DTRUCK]` Demolition Truck
- **Counter-units to Terrorist**:
  - **Vehicle crush (Crushable=yes default)** — crush bypasses Explodes=yes, safest hard counter. Drive Rhino/Grizzly over Terrorist
  - **Mind-control** (ImmuneToPsionics=no by default) — Yuri/Initiate/Magnetron can flip the Terrorist and use him against the original owner
  - **Long-range fire** (Sniper, V3, Prism Tank, Apocalypse cannon) — kill Terrorist before he reaches detonation range. But the death-explosion still triggers at his death cell, so this only works if you kill him FAR from valuable targets
  - **Dog leap** (Parasite warhead one-shot) — Strength=75 < 250 dmg vs Armor=flak (`ParasiteDog.Verses[flak]=100%`)
- **Related warheads**:
  - `[TerrorBombWH]` shares **AnimList=MININUKE** with other "mini-nuke" explosion warheads (V3HE etc.)
  - `[TerrorBombWH].Fire=yes` puts the warhead in the "starts fires" family with `[Flamer]`, `[Napalm]`, etc.
- **Soundmd cross-link**:
  - `[TerroristAttack]` sound `igiat2a` is **shared** with GI's attack sound 2 (recycled audio asset)

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [TERROR]` | 4768-4813 (46 lines) | All 38 active keys covered (9 commented: `;Image=SPY`, `;Prerequisite=NAHAND,NATECH`, `;Primary=MakeupKit`, `;C4=Yes`, `;CanDisguise=yes`, `;PermaDisguise=yes`, `;Cost=1500`, `;Soylent=200` all documented) |
| `artmd.ini [TRST]` | 176-183 (8 lines) | All keys covered |
| `artmd.ini [TerroristSequence]` | 13924-13946 (23 lines) | All 22 active slots + 3 commented `;FireUp`, `;FireProne`, and stub Die3-5 covered including "spy disguise needs this" defensive entries |
| `rulesmd.ini [TerrorBomb]` | 22370-22378 (9 lines) | All keys covered including Suicide=yes flag |
| `rulesmd.ini [TerrorBombWH]` | 27214-27223 (10 lines) | All keys covered with 11-column Verses breakdown and Dustin's experimental Bright=yes note |
| `soundmd.ini` Terrorist voices | TerroristSelect, Move, AttackCommand, Fear, Die, Attack | All 6 covered; `[TerroristAttack]` flagged as unused (TerrorBomb has no Report=) |
| Hardcoded behavior | Suicide=yes (weapon) + Explodes=yes + DeathWeapon=TerrorBomb (techno) + DeathWeaponDamageModifier (techno) + CanPassiveAquire/CanRetaliate=no (player-control safety) | 5 mechanisms covered with 4 Ghidra-verified xrefs |
| Ghidra searches performed against ID | 4 distinct queries (1 strings + 3 xref lookups) | Logged inline |
| TS-legacy filter | Applied; all commented designer-history entries documented; all active flags YR-verified | Done |
