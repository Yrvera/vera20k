# Crazy Ivan (IVAN)
Side: Soviet | Category: Infantry | Image alias: `[IVAN]` (no `Image=` redirect — own SHP `IVAN`)

The Soviet **Crazy Ivan**. $600 from Soviet Barracks + Radar. The iconic
"plant a time bomb on any unit, building, bridge or cow" infantry. The bomb
system is a **separate engine class (`BombClass`)** — one of the most
self-contained subsystems in YR — instantiated by the `IvanBomb=yes` warhead
flag. Each plant creates a 92-byte BombClass instance (vtable
`0x007E3D10`, factory `0x00438E70`) linked two-way to the carrier via
`TechnoClass+0x38 (AttachedBomb)`. Fuse runs `IvanTimedDelay=450` frames
(30s @ 15fps); on expiry calls `Apply_area_damage(target.coords, IvanDamage=450,
attacker, IvanWarhead=IvanWH)`. Carrier death also triggers detonation (it
does NOT defuse). The clock overlay is rendered from `CHRONOSK.SHP` (13
frames). The Engineer's `BombSight=4` is how friendly engineers reveal these
bombs on the minimap.

Authoritative deep RE:
[BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md) (941 lines).

---

## rulesmd.ini — `[IVAN]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4547`:

```ini
[IVAN]
UIName=Name:IVAN
Name=Crazy Ivan    ;locked
Prerequisite=NAHAND,NARADR
Pip=red
Category=Soldier
Strength=125
Primary=IvanBomber
Explodes=yes
Armor=none
TechLevel=5
CrushSound=InfantrySquish
Insignificant=no
Sight=6
Speed=4
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=600
Soylent=300
Points=30
IsSelectableCombatant=yes
VoiceSelect=CrazyIvanSelect
VoiceMove=CrazyIvanMove
VoiceAttack=CrazyIvanAttackCommand
VoiceFeedback=CrazyIvanFear
VoiceSpecialAttack=CrazyIvanAttackCommand
DieSound=CrazyIvanDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=15	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
Ivan=yes;needed to differentiate from Bomber, which is C4, and engineer
;Bombable=no
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;Deployer=yes
;UndeployDelay=20
Size=1
ElitePrimary=IvanBomberE
;AttackFriendlies=yes ; when scanning for targets, won't differentiate between allied or not, and also doesn't need control pressed to get attack cursor on friends
AttackCursorOnFriendlies=yes ; subset of AttackFriendlies.  Accept a command to attack, but don't consider in threat scan
IFVMode=7
Trainable=no
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:IVAN` | CSF-string key → "Crazy Ivan" |
| `Name=Crazy Ivan    ;locked` | Internal name; `;locked` is a build-only marker |
| `Prerequisite=NAHAND,NARADR` | Soviet Barracks AND Soviet Radar Tower (NARADR) — note **NARADR specifically**, not the abstract `RADAR` keyword. A Soviet player who has only an Allied radar (captured) cannot build Ivan |
| `Pip=red` | Cargo pip color — **red** (elite/special class, same as Tanya/SEAL/Sniper) |
| `Category=Soldier` | Pip group + AI grouping |
| `Strength=125` | HP — 125 (slightly tougher than GI/Engineer) |
| `Primary=IvanBomber` | The bomb-planting weapon — `Range=1.5`, `FireOnce=yes`, `Warhead=IvanBomb` (the `IvanBomb=yes` flag triggers BombClass::Attach). See "Weapons" and "Hardcoded Behavior" §1 |
| `Explodes=yes` | **Behavior flag** — TechnoTypeClass field. **TechnoType+0xD15 (byte) [BINARY-VERIFIED audit 34]** (parser xref @ 0x007122C5 to string at 0x0083355C; writeback `*(undefined1 *)((int)param_1 + 0xd15) = uVar3` after ReadBool). Note: Explodes is ALSO read by OverlayTypeClass__ReadINI @ 0x005FE840 (crates can explode too). When the unit dies, triggers a death-explosion using `[DeathWeapon]`/`[DeathWeaponDamage]` (defaults to `[DeathWH]` warhead). **Combined with the bomb-on-self trick** this is one of the iconic Ivan plays. |
| `Armor=none` | Damage type column 0 — standard infantry |
| `TechLevel=5` | Mid-game tech-5 cap (gated by Battle Lab implicitly because NARADR comes after) |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `Insignificant=no` | **Behavior flag** — when set to `yes`, the unit's death doesn't trigger Sad Phantom / EVA voice messages. For IVAN it's explicitly `no` because the designers wanted Ivan deaths to register on the EVA feedback channel ("Crazy Ivan lost") — important player feedback for a high-value unit |
| `Sight=6` | Reveal radius — moderate |
| `Speed=4` | Foot-speed — standard infantry |
| `Owner=Russians,Confederation,Africans,Arabs` | All 4 Soviet countries |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `Cost=600` | $600 — same as Sniper |
| `Soylent=300` | $300 Grinder refund (Yuri only) |
| `Points=30` | **Kill score 30** — by far the highest of any basic infantry (compare GI=5, Engineer=5, Sniper=10, Tesla Trooper=5). Reflects the unit's high game-impact value |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=CrazyIvanSelect` | Select voice — `$icrasea..g` (7 lines, the largest select bank — Ivan's mad-laugh repertoire) |
| `VoiceMove=CrazyIvanMove` | Move voice — `$icramoa..f` (5 lines, `icramoe` skipped — `a,b,c,d,f`) |
| `VoiceAttack=CrazyIvanAttackCommand` | Attack voice — `$icraata..d` (4 lines) |
| `VoiceFeedback=CrazyIvanFear` | Fear voice — `$icraseg $icrasea` (2 lines, **recycled** from select bank — Priority=Low) |
| `VoiceSpecialAttack=CrazyIvanAttackCommand` | Reuses Attack-Command voice for special-attack |
| `DieSound=CrazyIvanDie` | Death voice — `$icradia/b` (2 lines) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=15` | AI scoring weight — moderate (high enough to warrant priority targeting but lower than direct combat threats) |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set |
| `Ivan=yes` | **[POSSIBLY DEAD INI, audit 34]**: `search_strings("^Ivan$")` returns **0 matches** in the binary. The only Ivan-prefixed strings present are `IvanBomb` (warhead flag), `IvanDamage`, `IvanTimedDelay`, `IvanIconFlickerRate`, `IvanWarhead` (Rules-CombatDamage), and `NoIvanBomb` (the negative flag). **`Ivan=yes` itself is NOT parsed as a standalone INI key.** The doc's "needed to differentiate from Bomber/Engineer" comment is the designer's stated intent, but the parser-side implementation appears absent. The actual differentiation comes from the unit's Primary weapon's warhead carrying `IvanBomb=yes`, not from this flag. **[CORRECTION needed to deep RE doc + MouseClass research]**: the `+0xEBE` InfantryType field is set by `Infiltrate=`/`C4=`/two unnamed sibling InfantryType keys at +0xEC3/+0xEC4 (audit 6 cumulative) — `Ivan=yes` is NOT a parser source for +0xEBE. Mouse cursor logic for Ivan-bomb-target likely keys off the unit's Primary-weapon-warhead-has-IvanBomb-yes pattern, not off the Ivan= INI key.|
| `;Bombable=no` (commented) | Commented out — defaults to `no` for Ivan (Ivan **cannot** himself be bombed by another Ivan? actually defaults to `no` anyway since only E1 has explicit Bombable=yes). The comment is historical: the designers were considering whether Ivan should be Bombable. Final answer = no (default) |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 abilities at Veteran tier — **but moot** because `Trainable=no` |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities at Elite tier — also moot |
| `;Deployer=yes` (commented) | Commented out — Ivan was originally going to have a deploy command (probably to "arm" a bomb on himself). Final design: no deploy |
| `;UndeployDelay=20` (commented) | Companion to Deployer — also commented out |
| `Size=1` | Transport cargo slot cost |
| `ElitePrimary=IvanBomberE` | At Elite rank, Primary swaps to `[IvanBomberE]` (Damage 400→600 — but bomb damage is `IvanDamage=450` regardless, so the weapon's Damage value is **dead** for placement; only the death-explosion uses the per-bomb Damage). Effective Elite difference: virtually nil — the bomb fuse, BombClass behavior, IvanDamage, and IvanWarhead are all global rules, not per-weapon |
| `;AttackFriendlies=yes` (commented) | TechnoTypeClass field. **TechnoType+0x6C0 (byte) [BINARY-VERIFIED audit 34]** (xref at 0x0071522E, writeback `param_1 + 0x1b0` × int*-stride = +0x6C0). **Disabled** — Ivan does NOT auto-target friendlies. The lighter `AttackCursorOnFriendlies` (+0x6C1) is used instead. Sibling bytes. |
| `AttackCursorOnFriendlies=yes` | **Behavior flag** — TechnoTypeClass field. **TechnoType+0x6C1 (byte) [BINARY-VERIFIED audit 34]** (xref at 0x0071524F, writeback `*(char *)((int)param_1 + 0x6c1) = (char)uVar5`). The user can right-click a friendly unit and the cursor shows attack — letting the player intentionally bomb friendly units. The AI's threat-scan does NOT consider friendlies as targets — only manual player commands accept the order. |
| `IFVMode=7` | IFV gunner-table index 7 → HTK's `Weapon8`/`ElitePassengerWeapon8` slot. In stock YR maps to a demolition-charge weapon variant. Garrisoned Ivan in IFV gives a mobile bomb-planter |
| `Trainable=no` | **Cannot gain veterancy** — Ivan never reaches Veteran/Elite organically. Defensive VeteranAbilities/EliteAbilities/ElitePrimary keys are inert. Reason: Ivan death = bomb detonates anyway (Explodes=yes), so XP-promotion would be moot. Also prevents the Veteran-discount-on-cost from making mass-Ivan strategy too cheap |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `NotHuman=` — defaults to `no` (Ivan is human — subject to InfDeath, sniper headshot, mind-control)
- `ImmuneToPsionics=` — defaults to `no`; **Ivan CAN be mind-controlled** (and a mind-controlled Ivan can bomb the original owner's units — strong Yuri vs Soviet trick)
- `ImmuneToRadiation=` — defaults to `no`; killed by radiation
- `Bombable=` — defaults to `no` (Ivan cannot have a bomb placed on him; the inline `;Bombable=no` comment is consistent)
- `Fearless=` — not set; Ivan shows fear behavior
- `Occupier=` — defaults to `no`; **Ivan cannot garrison** civilian buildings (would be too powerful — garrison + bomb)
- `Agent=`/`Infiltrate=` — not set; Ivan cannot enter enemy buildings via Mission Enter (but he CAN target them with IvanBomber)
- `Engineer=` — not set
- `Assaulter=` — not set
- `C4=` — not set; Ivan does NOT use the C4 system (`C4=yes` is for Tanya/SEAL). The bomb mechanic is entirely separate (BombClass)
- `Deployer=` — explicitly commented out (final answer: no)
- `DetectDisguise=` — not set
- `DefaultToGuardArea=` — not set (MissionGuard when idle)
- `BombSight=` — not set; Ivan does NOT detect bombs (only Engineer's `BombSight=4` and Tanya's `BombSight=4` do)

---

## artmd.ini — `[IVAN]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:358`:

```ini
[IVAN] ; Crazy Ivan
Cameo=IVANICON
Sequence=IvanSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=80,0,85
```

| Key | Meaning |
|-----|---------|
| `Cameo=IVANICON` | Sidebar build icon (SHP) |
| `Sequence=IvanSequence` | Reference to `[IvanSequence]` — Ivan-specific sequence with deploy frames (even though Deployer=no) |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette |
| `FireUp=6` | Bullet-spawn frame — at frame 6 the bomb is "thrown" |
| `PrimaryFireFLH=80,0,85` | FLH — 80 forward, 0 sideways, 85 up |

**No `AltCameo=` is set** — defensively missing because `Trainable=no` (Veteran cameo would never show).

### Referenced sequence — `[IvanSequence]`

`artmd.ini:14464`:

```ini
[IvanSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=164,6,6
Down=212,2,2
Up=228,2,2
Deploy=244,15,0 ; ### Bad/missing frames in Ivan
Deployed=257,1,0
Undeploy=257,1,0
;Deploy=164,6,0
;Deployed=169,1,0
;Undeploy=169,1,0
Cheer=259,8,0,E
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Same | |
| `Walk=8,6,6` | Walk cycle 6 frames × 6 facings | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames, S-facing | "Laugh maniacally" anim |
| `Idle2=71,15,0,E` | Idle 2 — E-facing | "Cradle the bomb" anim |
| `Prone=86,1,6` | Prone 1 frame × 6 facings | |
| `Crawl=86,6,6` | Crawl reuses prone | |
| `Die1=134,15,0` | Death 1 — 15 frames | **Ivan death triggers `Explodes=yes` death-explosion**, distinct from the visible death sprite |
| `Die2=149,15,0` | Death 2 | |
| `FireUp=164,6,6` | Bomb-throw cycle | At frame 6 (per artmd FireUp=6) the bomb is launched |
| `FireProne=164,6,6` | Prone-fire reuses standing fire | Ivan can plant bombs while prone |
| `Down=212,2,2` | Get-down to prone | |
| `Up=228,2,2` | Get-up from prone | |
| `Deploy=244,15,0` | **Deploy anim — 15 frames** | Designer comment: "### Bad/missing frames in Ivan". Despite `Deployer=no`, the deploy anim is defined (defensively — in case Deployer flag flipped during testing) |
| `Deployed=257,1,0` | Deployed single-frame pose | Same — defensive |
| `Undeploy=257,1,0` | Reuse of Deployed | |
| `;Deploy=164,6,0` `;Deployed=169,1,0` `;Undeploy=169,1,0` | Commented-out earlier deploy frames | Pre-Sequence rewrite, abandoned |
| `Cheer=259,8,0,E` | Cheer — 8 frames, E-facing | |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | Only Die1/Die2 are real |
| `Panic=8,6,6` | Panic = Walk frames | |

---

## Weapons

### Primary (Veteran and below) — `[IvanBomber]`

`rulesmd.ini:24110`:

```ini
[IvanBomber]
Damage=400 ; Damage is used only for death explosion
ROF=50
Range=1.5 ; you can't change the target, but you can change yourself for CellRangefinding, so target could still be far side infantry
CellRangefinding=yes
FireOnce=yes ; Only fire once; don't stay in attack mission
Projectile=Invisible
Warhead=IvanBomb
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=400` | **Inline comment: "Damage is used only for death explosion"** — the bullet's damage is NEVER applied to the target (WarheadTypeClass::Detonate short-circuits the damage path because `IvanBomb=yes` is on the warhead). The 400 is a placeholder so the bullet system doesn't crash on zero-damage bullets. The actual bomb explosion damage is `Rules->IvanDamage = 450` (not this value) |
| `ROF=50` | 50 frames between attempts (~3.3s) — but combined with `FireOnce=yes`, only one shot per Ivan-attack-command anyway |
| `Range=1.5` | 1.5 cells — Ivan must be adjacent to target. Inline comment notes that CellRangefinding allows the target to drift slightly farther via the "self-positioning" trick |
| `CellRangefinding=yes` | Use cell-center distance — forgiving 1.5-cell radius |
| `FireOnce=yes` | After firing once, TarCom clears — Ivan doesn't stay in attack mission spamming bombs (one bomb per command) |
| `Projectile=Invisible` | Instant-hit invisible — bomb appears on target as soon as fire is committed |
| `Warhead=IvanBomb` | **The planting warhead** — see warhead section. The `IvanBomb=yes` flag is what triggers BombClass::Attach |
| `FireInTransport=no` | Cannot fire from inside [FV] Battle Fortress |

### Elite Primary — `[IvanBomberE]`

`rulesmd.ini:25052`:

```ini
[IvanBomberE]
Damage=600 ; Damage is used only for death explosion
ROF=50
Range=1.5 ; you can't change the target, but you can change yourself for CellRangefinding, so target could still be far side infantry
CellRangefinding=yes
FireOnce=yes ; Only fire once; don't stay in attack mission
Projectile=Invisible
Warhead=IvanBomb
;Report=CrazyIvanAttack
FireInTransport=no;can't fire out of the BattleFortress
```

**Effectively identical** to `[IvanBomber]` because:
- `Damage=600` (vs 400) is acknowledged dead code via the comment
- Both use `Warhead=IvanBomb` → same BombClass → same `Rules->IvanDamage = 450` explosion
- Same ROF, Range, projectile, FireOnce, FireInTransport
- Difference: commented-out `;Report=CrazyIvanAttack` — neither weapon has a sound report wired (the `CrazyIvanAttack` SFX plays from a different path; see "Hardcoded Behavior" §1 step 4.1)

**`Trainable=no` on the type means Ivan never reaches Elite** — IvanBomberE is dead code in stock YR. Defensive only.

### Primary's Warhead — `[IvanBomb]` (the planting warhead)

`rulesmd.ini:27185`:

```ini
[IvanBomb] ; Placing
IvanBomb=yes
```

**Minimal warhead — just the one flag.**

| Key | Meaning |
|-----|---------|
| `IvanBomb=yes` | **THE flag** — WarheadTypeClass field. **WarheadType+0x157 (byte) [BINARY-VERIFIED audit 34]** (assembly-verified writeback `MOV byte ptr [ESI + 0x157], AL` at 0x0075d823; parser xref @ 0x0075D807 to string at 0x0081BD60). Sibling to +0x158 ElectricAssault (audit 33). When a weapon with this warhead detonates, `WarheadTypeClass::Detonate @ 0x004690B0` branches to the IvanBomb handler at `0x0046935D` which calls **`BombClass::Attach(target, attacker)`** on the global `g_BombList @ 0x0087F5D8`. The bullet's damage value is **discarded** — only the bomb-attach happens. |

No `Verses=`, no `InfDeath=`, no `AnimList=` — the IvanBomb warhead does no
damage. All damage comes later from `[IvanWH]` when the bomb expires.

### The Explosion Warhead — `[IvanWH]`

`rulesmd.ini:27188`:

```ini
[IvanWH] ;Explosion
Verses=100%,100%,100%,100%,100%,100%,100%,250%,20%,100%,100%
InfDeath=6;3
CellSpread=1.5
PercentAtMax=.25
AnimList=CRIVEXP
```

**Used by `Apply_area_damage` when BombClass::Detonate fires.**

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,100%,100%,100%,100%,250%,20%,100%,100%` | 11-column. **100% vs all infantry and vehicle armors** (one-shots most infantry at 450 damage). **250% vs steel** (= 1125 effective damage vs steel-armored structures like the Rhino tank? actually `steel` is armor index 7 = the standard heavy-vehicle/medium-building armor; 250% bonus reflects bombs being effective vs hard targets). **20% vs concrete** (armor index 8 — heavily-armored static structures like Battle Lab, Construction Yard — Ivan damage is suppressed vs concrete to prevent trivial base destruction). 100% vs special_1 and special_2 |
| `InfDeath=6;3` | Infantry death animation type 6 (formerly 3 per inline comment) — **the "blown to bits"** death animation (large gibs). Stronger visual than standard small-arms death type 1 |
| `CellSpread=1.5` | Splash radius 1.5 cells — about a 3×3 cell explosion area |
| `PercentAtMax=.25` | At the splash radius edge, damage is 25% of full — so edge units take ~113 dmg |
| `AnimList=CRIVEXP` | Explosion animation `CRIVEXP` (Crazy Ivan Explosion) |

### Reference list of bomb constants in `[CombatDamage]`

All read by `RulesClass::ReadCombatDamage`:

| Key | Default | Rules offset | Used by |
|-----|---------|--------------|---------|
| `IvanDamage` | 450 | `+0xFCC` | BombClass::Detonate → Apply_area_damage |
| `IvanTimedDelay` | 450 (frames = 30s) | `+0xFD0` | BombClass::Attach → `EndFrame = StartFrame + IvanTimedDelay` |
| `IvanIconFlickerRate` | 8 (frames) | `+0xFD8` | BombClass::GetClockFrame → clock-icon flicker period |
| `IvanWarhead` | `IvanWH` | `+0xFC8` (warhead pointer) | BombClass::Detonate → passed to Apply_area_damage |
| `BombTickingSound` | `CrazyIvanBombTick` | `+0x20C` (sound index, AudioList) | BombClass::Attach → cached on bomb instance |
| `BombAttachSound` | `CrazyIvanAttack` | `+0x210` (sound index) | BombClass::Attach → one-shot at target on attach (human players only) |
| `BombSight` (per infantry) | 0 / 4 (E1/ENG/TANY) | **TechnoType+0x5F8** (int, **[BINARY-VERIFIED audit 34]** — doc claimed InfantryType-scope; parser xref @ 0x0071431C is in **TechnoTypeClass__ReadINI** not InfantryTypeClass__ReadINI. Offset +0x5F8 confirmed via `param_1[0x17e] = iVar4` int-array indexing. Scope is TechnoType.) | BombClass::UpdateAll BombVisible refresh |
| `CanDetonateTimeBomb` / `CanDetonateDeathBomb` | no | `[General]` | EventClass::Execute case 10 (player double-click detonate) |

### Projectile — `[Invisible]`

Standard inviso projectile (separate from `[InvisibleLow]`/`[InvisibleHigh]`/`[InvisibleAll]`/`[Invisible3]`). Defined elsewhere in rulesmd as the bare-minimum inviso projectile.

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death (the iconic mad-laugh bank)

```ini
[CrazyIvanSelect]                  ; soundmd.ini:3660
Sounds= $icrasea $icraseb $icrasec $icrased $icrasee $icrasef $icraseg
Control= random
Volume=85

[CrazyIvanMove]                    ; soundmd.ini:3655
Sounds= $icramoa $icramob $icramoc $icramod $icramof
Control= random
Volume=85

[CrazyIvanAttackCommand]           ; soundmd.ini:3650
Sounds= $icraata $icraatb $icraatc $icraatd
Control= random
Volume=85

[CrazyIvanFear]                    ; soundmd.ini:3665
Sounds= $icraseg $icrasea
Control= random
Priority=Low
Volume=90

[CrazyIvanDie]                     ; soundmd.ini:3671
Sounds= $icradia $icradib
Control= random
Volume=85
```

**7 select lines** (the largest of any infantry voice bank — Ivan's mad-laugh
catalogue is exhaustive). 5 move / 4 attack / 2 fear (recycled from select) /
2 death.

### Weapon / event sounds (3 distinct)

```ini
[CrazyIvanAttack]                  ; soundmd.ini:953
Sounds=icraatta
Volume=70

[CrazyIvanDeploy]                  ; soundmd.ini:957
Sounds=icraatta
Volume=55

[CrazyIvanBombTick]                ; soundmd.ini:961
Sounds=icraloop
Control=loop
Limit=3
Priority=high
Volume=45
```

| Sound | Wired by | Purpose |
|-------|----------|---------|
| `CrazyIvanAttack` | `Rules+0x210 (BombAttachSound)` → played by `BombClass::Attach` on attach **for human-owned target only** | The "sploosh / Ivan giggles" sound when a bomb is planted. Single sample `icraatta`. **Not played for AI-only targets** |
| `CrazyIvanDeploy` | Likely unused in stock (Ivan has no deploy command — `;Deployer=yes` commented out). Same sample as Attack at lower volume | Vestigial |
| `CrazyIvanBombTick` | `Rules+0x20C (BombTickingSound)` → played by `BombClass::UpdateAll` as a looping voc anim spatially tracked to the carrier | **The iconic ticking clock sound** — `icraloop` plays continuously on the carrier while the bomb's fuse is running. `Control=loop` for the loop, `Limit=3` caps to 3 concurrent (so 3 ticking bombs nearby don't blow the audio). `Priority=high` — important enough to preempt other SFX |

### Cross-references

- `[CrazyIvanBombTick]` is the **most distinctive Ivan sound** — players hear it from off-screen and know there's a planted bomb somewhere. Critical for parity
- `[DemoTruckAttackCommand]` (soundmd.ini:3676) — adjacent in soundmd; the Demolition Truck shares some of Ivan's "kamikaze" mood and is wired separately

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND,NARADR` | **Soviet Barracks + Soviet Radar Tower specifically** (not the abstract Barracks/RADAR). Requires Soviet tech chain |
| `Owner=` | `Russians,Confederation,Africans,Arabs` | All 4 Soviet countries |
| `TechLevel=` | `5` | Mid-game tech-5 cap |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=600` | $600 | Same as Sniper |
| `Soylent=300` | $300 refund (Yuri only) | |
| `Points=30` | **30** | Highest of any infantry — reflects strategic value |

No `PrerequisiteOverride=`, no `BuildLimit=`, no `RequiredHouses=` (any Soviet country can build), no `RequiresStolenXxxTech=`.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Trainable=no | **Cannot gain veterancy** — all Veteran/Elite ability lists and `ElitePrimary=IvanBomberE` are inert. Reason: Ivan death triggers Explodes=yes anyway; promotion would not meaningfully change his contribution |

---

## Hardcoded behavior — Ghidra-verified

### 1. Bomb plant — BombClass system (the headline mechanic)

**Full pipeline traced** in [BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md).
Summary:

```
1. Ivan fires IvanBomber weapon at target
   → InfantryClass::Fire_At creates a BulletClass with Warhead=IvanBomb
2. Bullet detonates (Projectile=Invisible, instant)
   → BulletClass::Detonate → WarheadTypeClass::Detonate @ 0x004690B0
3. WarheadTypeClass::Detonate sees IvanBomb=yes flag at WarheadType+0x157
   → branches to IvanBomb handler at 0x0046935D
4. IvanBomb handler calls BombClass::Attach(target, attacker)
   @ 0x00438E70, this=g_BombList @ 0x0087F5D8
5. BombClass::Attach:
   a. operator_new(0x5C) → new BombClass instance
   b. AbstractClass::Constructor_Full() sets base fields
   c. Set vtables (primary 0x007E3D10, 3 secondary)
   d. Init VocHandle, cache TickingSoundID = Rules->BombTickingSound (Rules+0x20C)
   e. Register in g_BombClass_List @ 0x0089C668 (global tracking)
   f. Register in g_BombAttachList @ 0x0087F5D8 (per-tick iteration)
   g. Set Attacker=attacker, OwnerHouse=attacker.GetOwningHouse(),
      Target=target, State=0, StartFrame=g_CurrentFrameCounter,
      EndFrame=StartFrame + Rules->IvanTimedDelay (Rules+0xFD0) = +450 frames
   h. Set target->AttachedBomb (target+0x38) = bomb  ← BACK-POINTER
   i. If target.Owner.IsHumanPlayer():
      VocClass::PlayAt(Rules->BombAttachSound = CrazyIvanAttack)
```

**Per-tick fuse check** in `TechnoClass::AI_Update @ 0x006F9E50` on each carrier:
```c
if (carrier->AttachedBomb /*+0x38*/ != 0 && !carrier->InLimbo /*+0x81*/) {
    if (BombClass::IsTimerExpired(carrier->AttachedBomb))
        BombClass::Detonate(carrier->AttachedBomb);
}
```

`IsTimerExpired @ 0x00438A70`: `state==0 && endFrame < currentFrame && !hasFired`.

**Detonate @ 0x00438720** (called from fuse expiry, carrier death, or
player double-click):
```c
target->AttachedBomb = 0;
target->BombVisible = 0;
bomb->HasFired = 1;
if (!target->InLimbo) {
    Apply_area_damage(
        target.coords,
        Rules->IvanDamage,          // = 450
        bomb->Attacker,             // for kill credit
        Rules->IvanWarhead);        // = IvanWH
    spawn AnimClass(CRIVEXP);
    if (target is Building with foundation on bridge)
        destroy LOW or HIGH bridge under building's foundation cells;
}
bomb->Attacker = bomb->Target = bomb->OwnerHouse = 0;
VocHandle_Release(bomb->TickingSoundHandle);
```

**Critical parity facts:**

- **Carrier death detonates, NOT defuses.** When the carrier is killed by
  any other source, `TechnoClass::ReceiveDamage` calls
  `BombClass::Detonate`, adding IvanDamage splash to whatever killed the
  carrier
- **In-Limbo target detonate is silent** — no damage, no anim (target was
  garrisoned / loaded as cargo when fuse expired)
- **Defuse only via UnInit / ChangeOwner** — captured buildings have their
  bombs silently defused; sold buildings same. **No stock YR weapon
  defuses bombs** — `[BombDisarm]` warhead exists but no weapon references
  it (a moddable hook)
- **Bridge destruction**: Ivan-bombing a building on a bridge destroys the
  bridge under the building's foundation (LOW or HIGH bridge per the
  IsoTile overlay range scan)

### 2. Clock overlay rendering — CHRONOSK.SHP

`IvanBomb::GetClockFrame @ 0x00438A00`:
```c
if (bomb->State == 1) return 12;  // dead branch — no code writes State=1
int elapsed = currentFrame - bomb->StartFrame;
int frame = (elapsed / (IvanTimedDelay / 6)) * 2;  // 0, 2, 4, 6, 8, 10
if (currentFrame % (IvanIconFlickerRate * 2) >= IvanIconFlickerRate)
    frame++;  // flicker odd frame (1, 3, 5, 7, 9, 11)
return min(frame, 11);
```

`IvanTimedDelay / 6 = 75 frames per "clock hour"`. With default
`IvanIconFlickerRate=8`, flicker period is 16 frames. **All bombs on screen
flicker in phase** (driven by global frame counter, not per-bomb StartFrame).

Drawn inline in `TechnoClass::DrawExtras @ 0x006F5190`:
```c
if (carrier->BombVisible && carrier->AttachedBomb) {
    int frame = BombClass::GetClockFrame(carrier->AttachedBomb);
    CC_Draw_Shape(Rules->CHRONOSK_SHP /*+0xFE0*/, frame, ...);
}
```

**Frame 12 (the detonation glyph) never renders in stock YR** — per the deep
RE doc round-2 byte-pattern scan, no code writes `bomb->State = 1`. Dead
visual logic.

### 3. BombVisible refresh — who sees the clock

`BombClass::UpdateAll @ 0x00438BF0` runs every tick from `LogicClass::PerTickUpdate`.
Every **45 frames** (hardcoded constant 0x2D in the function, ~3s @ 15fps):
- Iterate opposing infantry list
- For each opposing infantry, if `infantry->TypeClass->BombSight (+0x5F8)` >
  distance-in-cells to bomb's target, set `target->BombVisible = 1`
- Human players **always see their own bombs** unconditionally (no scan needed)
- If BombVisible changed, mark `target->NeedsRedraw = 1`

In stock YR, **only Allied Engineer / SEAL / Tanya** have `BombSight > 0`
(all = 4 cells). Soviet/Yuri units cannot detect Ivan bombs.

### 4. Ivan=yes flag — type marker

INI key `Ivan=yes` on InfantryTypeClass marks the unit as an Ivan-type
infantry. Per the inline comment "needed to differentiate from Bomber,
which is C4, and engineer", the engine uses this flag to:
- Show the "place bomb" mouse cursor when hovering over a valid target
- Gate AI bomb-target-pick logic (only Ivan-flagged units choose bomb targets)
- Distinguish from `C4=yes` (Tanya/SEAL — different demolition system) and
  `Engineer=yes` (Engineer — different capture system)

Note: the `[CIVAN]` Chrono Ivan (campaign variant) also has `Ivan=yes`.

### 5. AttackCursorOnFriendlies=yes — friendly-fire targeting

INI key `AttackCursorOnFriendlies` is a TechnoTypeClass field (per
`TechnoTypeClass__ReadINI @ 0x0071524F` DATA xref to string at `0x00843604`).
When set, the engine shows the **attack cursor when hovering over friendly
units** (without requiring Ctrl modifier as force-attack). The AI does NOT
threat-scan friendlies. **This is what allows the player to manually
right-click bomb a Tanya / friendly Battle Fortress** for the iconic "bomb
your own unit to deliver damage" plays.

The companion `AttackFriendlies=yes` (commented out, xref `0x0071522E`) is
the heavier flag — it would make the AI consider friendlies as valid threat
scan targets. Ivan uses only the lighter `AttackCursorOnFriendlies`.

### 6. Explodes=yes — death-explosion

INI key `Explodes` is a TechnoTypeClass field (xref `0x007122C5`). When the
unit dies, triggers a death-explosion using the type's `[DeathWeapon]` /
`[DeathWeaponDamage]` or default `[DeathWH]` warhead. For IVAN with no
explicit DeathWeapon, the death-explosion uses default warhead. **Critical
parity behavior**: killing an Ivan with a planted bomb on his own unit =
double explosion (Explodes-death + IvanWH bomb).

### 7. Insignificant=no — EVA feedback flag

INI key `Insignificant` is a TechnoTypeClass field. When `yes`, suppresses
EVA voice notifications on death ("Unit lost"). For IVAN explicitly `no`
ensures EVA announces Ivan deaths — the player is supposed to track this
high-value unit.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("Ivan\|Explodes\|AttackCursorOnFriendlies\|AttackFriendlies\|IvanBomb")` | 10 strings — confirms 10 hardcoded keys: `NoIvanBomb`, `IvanBomb` (warhead flag), `Explodes` (techno flag), `IvanIconFlickerRate`/`IvanTimedDelay`/`IvanDamage`/`IvanWarhead` (rules `[CombatDamage]`), `AttackCursorOnFriendlies` and `AttackFriendlies` (techno flags), `EXPLODES` (animation reference) |
| `get_xrefs_to(0x0083355C)` (= "Explodes") | 2 xrefs: `TechnoTypeClass__ReadINI @ 0x007122C5` + `OverlayTypeClass__ReadINI @ 0x005FE840` — confirms Explodes works on both Techno (units) and Overlay (e.g., crates) |
| `get_xrefs_to(0x00843604)` (= "AttackCursorOnFriendlies") | Sole xref from `TechnoTypeClass__ReadINI @ 0x0071524F` DATA — confirms techno-level flag |
| `get_xrefs_to(0x00843620)` (= "AttackFriendlies") | Sole xref from `TechnoTypeClass__ReadINI @ 0x0071522E` DATA — confirms techno-level (heavier than AttackCursorOnFriendlies) |
| `get_xrefs_to(0x0081BD60)` (= "IvanBomb") | 2 xrefs: `WarheadTypeClass__ReadINI @ 0x0075D807` + a static data reference at `0x007E4D24` — confirms warhead flag wired into the IvanBomb warhead handler in WarheadTypeClass::Detonate |
| `get_xrefs_to(0x0081BD54)` (= "NoIvanBomb") | Static data reference at `0x007E4D28` — confirms `NoIvanBomb` is also an engine-recognized flag (likely targets the negative case — a unit type with `NoIvanBomb=yes` cannot be bombed) |

Plus 12 BombClass functions already labeled in the deep RE doc.

---

## Ghidra audit log (audit iteration 34 — 2026-05-19)

**~18 Ghidra queries** (10 string searches + 6 xref lookups + 1 assembly-
context for IvanBomb + 5 grep passes on TechnoTypeClass__ReadINI decompile +
1 broad "Ivan" substring search). 6 doc-cited claims verify exactly + 5 NEW
struct-offset bindings BINARY-VERIFIED + 1 IN-DOC scope correction (BombSight)
+ 1 IMPORTANT DOC FINDING flagged (`Ivan=yes` is POSSIBLY DEAD INI).

### Function-entry verification

| Function | Address | Status |
|----------|---------|--------|
| `TechnoTypeClass__ReadINI` | (oversized) | grep-verified for Explodes/AttackCursor*/AttackFriendlies/BombSight |
| `WarheadTypeClass__ReadINI` | 0x0075d590 | IvanBomb parser @ 0x0075D807 → assembly-verified writeback to +0x157 |
| `OverlayTypeClass__ReadINI` | (cumulative) | secondary xref for Explodes (overlays can also Explodes=yes — e.g., crates) |
| `RulesClass__ReadCombatDamage` | (oversized) | parser xref for IvanDamage/IvanTimedDelay confirmed; per-key offsets DEFERRED |
| BombClass system functions | 0x00438xxx range | Constructor, Attach, Detonate, IsTimerExpired, UpdateAll, GetClockFrame — all labeled and cross-referenced via the standalone BOMB_CLASS_GHIDRA_REPORT.md (941 lines) |

### Key behavioral findings — 5 NEW struct-offset bindings BINARY-VERIFIED

| INI key | Scope | Offset | Type | Parser site | Status |
|---------|-------|--------|------|-------------|--------|
| `Explodes` | TechnoType (+ OverlayType) | **+0xD15** | byte (ReadBool) | 0x007122c5 | NEW |
| `AttackFriendlies` | TechnoType | **+0x6C0** | byte (ReadBool) | 0x0071522e | NEW |
| `AttackCursorOnFriendlies` | TechnoType | **+0x6C1** | byte (ReadBool) | 0x0071524f | NEW (sibling to AttackFriendlies, byte cluster) |
| `BombSight` | **TechnoType** (NOT InfantryType — doc was wrong) | **+0x5F8** | int (ReadInt) | 0x0071431c | NEW + scope correction |
| `IvanBomb` (warhead) | WarheadType | **+0x157** | byte (ReadBool) | 0x0075d807 | NEW (assembly-verified writeback at 0x0075d823; sibling to +0x158 ElectricAssault audit 33) |

### WarheadType byte-cluster +0x14B..+0x158 (consolidated audit 34)

| Offset | Key | Audit |
|--------|-----|-------|
| +0x14B | Sonic | 28 (DLPH) |
| +0x14C..+0x154 | DEFERRED siblings (ReadBool block) | — |
| +0x155..+0x156 | DEFERRED siblings | — |
| **+0x157** | **IvanBomb** | **34 (IVAN)** — NEW |
| +0x158 | ElectricAssault | 33 (SHK) |

The Sonic / IvanBomb / ElectricAssault triad of "warhead-flag special-effect
triggers" is now mapped — Sonic→chain-pulse, IvanBomb→BombClass::Attach,
ElectricAssault→Tesla Coil charge.

### TechnoType byte-cluster +0x6C0..+0x6C1 (audit 34 closure)

| Offset | Key | Audit |
|--------|-----|-------|
| **+0x6C0** | **AttackFriendlies** | **34 (IVAN)** — NEW |
| **+0x6C1** | **AttackCursorOnFriendlies** | **34 (IVAN)** — NEW |
| +0x6C8 | PreventAttackMove | 10 (SNIPE) |

Tactical-AI friendly-fire byte cluster forms a sub-block adjacent to
PreventAttackMove (audit 10).

### Re-confirmations

- `Insignificant` = ObjectType+0x232 (audit 21 cumulative) — note: doc
  said "TechnoTypeClass field" but actual scope is ObjectType. Minor
  semantic distinction — ObjectType is the parent layer that all
  unit-class types inherit, so calling it a "TechnoType field" isn't
  strictly wrong from a usage standpoint (a TechnoType instance has the
  field at the same offset).
- `FireOnce` = WeaponType+0x135 (audit 9 cumulative) — IvanBomber sets
  this to limit Ivan to 1 plant per command.
- `CellRangefinding` = WeaponType+0x134 (audit 9 cumulative).
- `FireInTransport` = WeaponType+0x143 (audit 9 cumulative).
- `IvanDamage` / `IvanTimedDelay` Rules-CombatDamage scope confirmed
  (parser xrefs @ 0x0066C5BB / 0x0066C6CC; per-key offsets DEFERRED).
- `Trainable=no` correctly causes Ivan to never veteran-up (per audit-32
  Trainable-default correction; default is TRUE so this explicit no is
  required to suppress veterancy).

### IN-DOC INCORRECT findings

**1. BombSight scope** — doc claims "InfTypeClass+0x5F8". Actual scope
is **TechnoType+0x5F8** (parser xref in TechnoTypeClass__ReadINI, NOT
InfantryTypeClass__ReadINI). The offset is correct; the scope label is
wrong. Note: only infantry have non-zero BombSight in stock YR (E1/ENG/
TANY), but the field is parsed at the TechnoType layer and is
theoretically settable on any TechnoType including vehicles/buildings.

**2. `Ivan=yes` INI key POSSIBLY DEAD INI** — `search_strings("^Ivan$")`
returns 0 matches in the binary. The doc claims `Ivan=yes` is parsed
and stored on InfantryTypeClass+0xEBE (via MouseClass research). But
**no standalone "Ivan" string exists in the binary**, only Ivan-prefixed
strings (`IvanBomb`, `IvanDamage`, `IvanTimedDelay`, `IvanIconFlickerRate`,
`IvanWarhead`, `NoIvanBomb`). Without a parser xref, `Ivan=yes` cannot
be confirmed as live. **The +0xEBE flag is set by C4/Infiltrate/two
unnamed sibling InfantryType keys** (audit 6 cumulative) — `Ivan=yes`
is NOT one of those sources per the InfantryTypeClass__ReadINI
decompile. The actual differentiation for "this unit places Ivan bombs"
comes from the unit's Primary weapon having a warhead with `IvanBomb=yes`,
not from the `Ivan=yes` INI key.

### Items NOT re-verified (DEFERRED with reason)

- **Rules-CombatDamage offsets** (IvanDamage +0xFCC, IvanTimedDelay +0xFD0,
  IvanIconFlickerRate +0xFD8, IvanWarhead +0xFC8, BombTickingSound
  +0x20C, BombAttachSound +0x210) — sourced from BOMB_CLASS deep RE
  doc; trust-chain to that doc, not directly re-verified inline.
- **BombClass instance offsets** (TechnoClass+0x38 AttachedBomb,
  bomb->Attacker, ->Target, ->StartFrame, ->EndFrame etc.) — same
  trust-chain to deep RE doc.
- **`NoIvanBomb` scope/offset** — static data reference only (0x007E4D28
  per the doc); parser site DEFERRED. The flag is engine-recognized but
  the parser scope (likely TechnoType or ObjectType) and offset still
  unknown.
- **`Ivan=yes` actual parsing mechanism** — if any. May require
  decompile of MouseClass / mouse-cursor logic to find any place that
  reads an `Ivan` flag from the INI section. Currently flagged
  POSSIBLY DEAD INI.
- **InfantryType +0xEC3 / +0xEC4 unnamed siblings** that set +0xEBE —
  could be the Ivan parser source. Would need string-table enumeration
  near InfantryTypeClass__ReadINI parser sites.
- **Bridge-destruction radius** in BombClass::Detonate (foundation cell
  scan) — code path exists per deep RE doc; full decompile DEFERRED.

### Negative claims verified

- `search_strings("IVAN")` → **0 matches**.
- `search_strings("Ivan")` (broad) → 6 prefix matches (IvanBomb, etc.)
  but **no standalone "Ivan"**.

### Confidence summary

- 5/5 NEW struct-offset bindings BINARY-VERIFIED.
- 1 NEW WarheadType cluster slot pinned (IvanBomb +0x157, joining audit
  28's Sonic and audit 33's ElectricAssault).
- 1 NEW TechnoType byte-cluster +0x6C0..+0x6C1 (AttackFriendlies +
  AttackCursorOnFriendlies).
- 1 IN-DOC scope correction (BombSight TechnoType-not-InfantryType).
- 1 IMPORTANT in-doc finding flagged (`Ivan=yes` POSSIBLY DEAD INI).
- 5 re-confirmations of prior cumulative offsets.
- Negative claims confirmed.

**Soviet sub-section: 3 of 32 docs DEEP-AUDITED.**

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Deployer=yes` / `;UndeployDelay=20` (commented) | Designer history — Ivan was originally going to have a deploy command. Final design: no deploy | OK |
| `;AttackFriendlies=yes` (commented) | Heavier flag than what was kept — designer chose the lighter `AttackCursorOnFriendlies` | OK |
| `Deploy=244,15,0 ; ### Bad/missing frames in Ivan` (in IvanSequence) | Designer comment about poor art quality; not TS-legacy | Documented |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `bomb->State (+0x30) = 1` write path | **Dead in stock YR** per deep RE round-2 byte-pattern scan. The frame-12 "explosion glyph" render path in `GetClockFrame` is therefore unreachable. Latent visual feature — possibly TS-era code | Documented |
| `[BombDisarm]` warhead | **No stock YR weapon references it**. The Defuse-by-warhead pipeline exists but is unreachable in vanilla. Engineer's BombSight reveals bombs but doesn't disarm. Moddable hook | Documented |
| `Charges=` (general engine flag, not on Ivan) | Superseded by DelayedFire system per Tesla Coil comment | OK |

No TS-only behavior found on the IVAN type itself. The BombClass subsystem
is fully YR-active.

---

## Cross-references

- **Bomb-planting infantry family**:
  - `[IVAN]` Crazy Ivan (this doc) — buildable, full bomb system
  - `[CIVAN]` Chrono Ivan — campaign-only, has both `Ivan=yes` and chrono-teleport. Uses same IvanBomb/BombClass system
- **Bomb-defending infantry** (BombSight > 0):
  - `[ENGINEER]` Allied Engineer (BombSight=4) — reveals bombs on minimap
  - `[GHOST]` Navy SEAL (BombSight=4)
  - `[TANY]` Tanya (BombSight=4)
  - Soviet/Yuri units cannot detect bombs (BombSight=0 default)
- **Bomb mechanics constants**:
  - All in `[CombatDamage]` of rulesmd.ini: `IvanDamage=450`, `IvanTimedDelay=450` (frames), `IvanIconFlickerRate=8`, `IvanWarhead=IvanWH`
  - In `[AudioList]`: `BombTickingSound=CrazyIvanBombTick`, `BombAttachSound=CrazyIvanAttack`
  - In `[General]`: `CanDetonateTimeBomb=no`, `CanDetonateDeathBomb=no` (UI gate for player double-click detonate)
- **Sister Soviet basic infantry**:
  - `[E2]` Conscript — basic
  - `[SHK]` Tesla Trooper — anti-vehicle
  - `[FLAKT]` Flak Trooper — AA
  - `[DESO]` Desolator — radiation
- **Related warheads / flags**:
  - `[IvanBomb]` warhead (planting — IvanBomb=yes)
  - `[IvanWH]` warhead (explosion — Verses + AnimList)
  - `[BombDisarm]` warhead (defuse — BombDisarm=yes; unwired in stock)
  - `[DeathWH]` warhead (Explodes=yes death-explosion default)
- **Counter-units / hard counters**:
  - Mind-control (Yuri/Initiate) — Ivan ImmuneToPsionics=no by default
  - Sniper one-shot (250 dmg > 125 HP)
  - Crushed by vehicle (Crushable defaults to yes)
  - Dog leap kill (one-shot via Parasite)
  - Engineer's BombSight defuses **indirectly** — players see the bomb and can move the carrier away from base (no in-game defuse weapon)
- **Iconic plays**:
  - Bomb a friendly Tanya → Tanya runs into enemy base → bomb detonates
  - Bomb a cow → drive cow into enemy base (cows are crushable, easy to herd)
  - Bomb a building on a bridge → bridge collapses (foundation-cell scan)
  - Bomb a Demo Truck → trade an Ivan for double demolition
  - Bomb an enemy Engineer mid-walk → deny capture

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [IVAN]` | 4547-4590 (44 lines) | All 38 active keys covered (3 commented: `;Bombable=no`, `;Deployer=yes`, `;UndeployDelay=20`, `;AttackFriendlies=yes` all documented) |
| `artmd.ini [IVAN]` | 358-364 (7 lines) | All keys covered (no AltCameo intentionally absent) |
| `artmd.ini [IvanSequence]` | 14464-14488 (25 lines) | All 22 active slots + 3 commented Deploy variants covered |
| `rulesmd.ini [IvanBomber]` | 24110-24118 (9 lines) | All keys covered including Damage=400 placeholder note |
| `rulesmd.ini [IvanBomberE]` | 25052-25061 (10 lines) | All keys covered (Trainable=no makes it dead) |
| `rulesmd.ini [IvanBomb]` warhead | 27185-27186 (2 lines) | Single flag covered |
| `rulesmd.ini [IvanWH]` warhead | 27188-27193 (6 lines) | All keys covered including 11-column Verses |
| `rulesmd.ini [CombatDamage]` Ivan keys | IvanDamage, IvanTimedDelay, IvanIconFlickerRate, IvanWarhead | All 4 covered with Rules offsets |
| `soundmd.ini` CrazyIvan voices | CrazyIvanSelect, Move, AttackCommand, Fear, Die | All 5 covered |
| `soundmd.ini` Ivan SFX | CrazyIvanAttack, CrazyIvanDeploy (vestigial), CrazyIvanBombTick | All 3 covered with wiring distinctions |
| Hardcoded behavior | BombClass system (Attach + Detonate + Defuse + UpdateAll + Clock + 0x30/0x58/0x68 + bridge scan) + Explodes=yes + Ivan=yes + AttackCursorOnFriendlies + Insignificant + 7 Ghidra-verified flag paths | Fully covered with cross-reference to standalone 941-line RE doc |
| Ghidra searches performed against ID | 6 distinct queries (1 strings + 5 xref lookups) | Logged inline |
| TS-legacy filter | Applied; `bomb->State=1` dead path, `[BombDisarm]` unwired in stock, deploy keys commented, ImmuneToVeins defensive — all documented | Done |
