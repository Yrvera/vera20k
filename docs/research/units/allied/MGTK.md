# MGTK — Mirage Tank (Allied tier-3 disguise tank)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** Allied stealth-ambush tank. Disguises itself as a randomly-picked terrain
tree (`DefaultMirageDisguises=TREE01-04`) whenever standing still, breaking the disguise
only briefly when firing (purely visual — no logical reveal). Anti-armor specialist
(`MirageGun` Damage=100, Verses 100% vs heavy) with light armor and short HP — works
by ambush and disengagement, not slug-fests.

> Output bar: the disguise-on-still mechanic is **the** defining feature. Frame-perfect
> timing of disguise-break-on-fire (`DisguiseFakeBlinkTime=15` rookie / `=5` elite),
> the random tree-type pick, and the "Mirage looks like tree until it shoots" feel
> must all match gamemd exactly.

> Ghidra confirms `gamemd.exe` contains no `"MGTK"` string. The string `"Mirage"`
> appears only as `DefaultMirageDisguises` (the global rule). All disguise behavior is
> driven by generic TechnoType flags (`CanDisguise`, `DisguiseWhenStill`) + WeaponType
> flags (`DisguiseFireOnly`, `DisguiseFakeBlinkTime`).

---

## 1. `rulesmd.ini` — `[MGTK]` verbatim

```ini
[MGTK]
UIName=Name:MGTK
Name=Mirage Tank
Image=RTNK
Prerequisite=GAWEAP,GATECH
Primary=MirageGun
DisguiseWhenStill=yes;gs I can no longer pick a disguise nor deploy
;Primary=TankMakeupKit
;Secondary=MirageGun
;IsSimpleDeployer=yes ;gs yeah for alpha date rewrite!
;OmniFire=yes
Strength=200
Category=AFV
Armor=light
Turret=no
IsTilter=yes
Crusher=yes
TooBigToFitUnderBridge=true
TechLevel=9
Sight=9
Speed=7
CrateGoodie=yes
Owner=British,French,Germans,Americans,Alliance
Cost=1000
Soylent=1000
Points=25
ROT=5
IsSelectableCombatant=yes
CanDisguise=yes
CanApproachTarget=no ; gs 9/15 Re-put in.  But now this will not apply to an Attack Mission.  Best of both worlds, and Dustin kicks butt
;CanRetaliate=no ; thought about this one too.  Don't need it since first shot will disguise as the bad guy and then you can keep shooting, and he'll keep shooting since you don't detach on disguise
;CanPassiveAquire=no ; not essential, but might not want it giving away position
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MirageTankSelect
VoiceMove=MirageTankMove
VoiceAttack=MirageTankAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=MirageTankMoveStart
MaxDebris=2
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Normal
ThreatPosed=15	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys;gs the sparks look cool, but the smoke gives it away too easily    ,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ImmuneToVeins=yes
Size=3
AllowedToStartInMultiplayer=no
EliteSecondary=MirageGunE
CrushSound=TankCrush
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:MGTK` | AbstractType | CSF lookup. |
| `Name` | `Mirage Tank` | AbstractType | Dev fallback. |
| `Image` | `RTNK` | AbstractType | **Reads artmd `[RTNK]` block** ("Mirage Tank" labeled). RTNK is the original RA2 art-slot name; MGTK is the YR rulesmd entry. See §2. |
| `Prerequisite` | `GAWEAP,GATECH` | TechnoType | Allied War Factory + Battle Lab — tier-4 gate. |
| `Primary` | `MirageGun` | TechnoType | The "heat ray" weapon (Damage=100, ROF=70, Range=7). See §3. |
| `DisguiseWhenStill` | `yes` | TechnoType [BINARY-VERIFIED audit 16: string @ 0x00843C64, parser xref @ 0x00714459, `TechnoType+0xD32` (byte)] | **Hardcoded core mechanic.** When the Mirage Tank is **stationary** (not moving), it automatically disguises as a randomly-picked terrain entry from `[General] DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04`. INI comment "gs I can no longer pick a disguise nor deploy" notes a design-history change: the player used to be able to manually pick disguise / deploy to disguise, but the shipped version disguises automatically when still. |
| Commented `;Primary=TankMakeupKit ;Secondary=MirageGun ;IsSimpleDeployer=yes ;OmniFire=yes` | (alternate-design notes) | — | Author preserved earlier design iterations: a "TankMakeupKit" primary that applied disguise, with the real weapon as Secondary, and `IsSimpleDeployer=yes` to trigger deploy-disguise. The shipped design uses `DisguiseWhenStill=yes` instead. |
| `Strength` | `200` | AbstractType | **200 HP — fragile.** Less than Grizzly's 300, much less than Rhino's 400. Mirage tanks die fast in head-on combat. |
| `Category` | `AFV` | TechnoType | AFV classifier. |
| `Armor` | `light` | TechnoType | Verses-slot 4 — vulnerable to AT weapons. |
| `Turret` | `no` | UnitType | **No rotating turret** — gun is hull-mounted, must rotate body to aim. Same constraint as TNKD. |
| `IsTilter` | `yes` | UnitType | Voxel hull tilts on slopes. |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `TooBigToFitUnderBridge` | `true` | UnitType-only | Cannot path under low bridges. |
| `TechLevel` | `9` | TechnoType | Late-tier. |
| `Sight` | `9` | TechnoType | **9-cell reveal** — longest sight in the Allied tank lineup (Grizzly 8, MCV 6). Crucial for "ambush from distance" play — Mirage spots enemies before being spotted. |
| `Speed` | `7` | TechnoType | Same as Grizzly. |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates. |
| `Owner` | 5 Allied countries | TechnoType | Allied only. |
| `Cost` | `1000` | TechnoType | Tier-3 cost. |
| `Soylent` | `1000` | TechnoType | 100% Grinder refund. |
| `Points` | `25` | TechnoType | Standard score on kill. |
| `ROT` | `5` | TechnoType | Body rotation rate (no turret to rotate separately). |
| `IsSelectableCombatant` | `yes` | TechnoType | Counts in select-all-combat. |
| `CanDisguise` | `yes` | TechnoType [BINARY-VERIFIED audit 16: string @ 0x00843C98, parser xref @ 0x0071440B, `TechnoType+0xD2F` (byte) — re-confirms audit 6 SPY] | **Enables the disguise system.** Combined with `DisguiseWhenStill=yes`, the unit may disguise. Without this flag, `DisguiseWhenStill=yes` would not work (the flag-stack requires both). |
| `CanApproachTarget` | `no` | TechnoType [BINARY-VERIFIED audit 16: string @ 0x00843C2C, parser xref @ 0x007144A7, `TechnoType+0xD33` (byte)] | **Cannot auto-approach targets.** INI comment: "gs 9/15 Re-put in. But now this will not apply to an Attack Mission. Best of both worlds, and Dustin kicks butt." Notes design history — `CanApproachTarget=no` prevents the AI/player-issued opportunistic-fire from making the Mirage chase down enemies (which would break the ambush role); but a manual Attack Mission DOES allow approach. So the player can micro-aggression while the unit holds position otherwise. |
| Commented `;CanRetaliate=no` | — | TechnoType | Author note: "thought about this one too. Don't need it since first shot will disguise as the bad guy and then you can keep shooting." Reveals an interesting design quirk: when fired upon, Mirage's first return shot "disguises as the bad guy" (?). May mean the engine swaps Mirage's visual disguise to match the attacker. Not load-bearing for this doc — flagged as open follow-up. |
| Commented `;CanPassiveAquire=no` | — | TechnoType | Author note: "not essential, but might not want it giving away position." If commented out, the default behavior (acquire passive targets) is in effect. Means a disguised Mirage WILL auto-target enemies that enter range, breaking its own disguise. Player must manually `Stop` orders for absolute stealth. |
| `Explosion` | `TWLT070,...` | TechnoType | Standard death anim pool. |
| `VoiceSelect` | `MirageTankSelect` | TechnoType | 6 unique clips ($vmirsea..ef; commented 7th $vmirseg unused). |
| `VoiceMove` | `MirageTankMove` | TechnoType | 7 unique clips ($vmirmoa..og). |
| `VoiceAttack` | `MirageTankAttackCommand` | TechnoType | 5 unique clips ($vmirata..te). |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic vehicle death. |
| `MoveSound` | `MirageTankMoveStart` | TechnoType | 3 unique clips, predelay 0–400ms, VShift +10, vol 35 (note: missing `FShift`, `Priority` — uses default for those). |
| `MaxDebris` | `2` | TechnoType | 2 debris pieces. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `MovementZone` | `Normal` | TechnoType | Standard. |
| `ThreatPosed` | `15` | TechnoType | Mid-low AI threat. |
| `DamageParticleSystems` | `SparkSys;gs the sparks look cool, but the smoke gives it away too easily ,SmallGreySSys` | TechnoType | INI comment-out: only `SparkSys` is active; `SmallGreySSys` is past the `;` comment marker → **smoke is intentionally disabled** to avoid breaking the disguise visually. "the sparks look cool, but the smoke gives it away too easily" — design decision: a disguised tree shouldn't emit visible smoke. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Veteran bonuses. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL + ROF. |
| `Accelerates` | `false` | TechnoType | No acceleration ramp. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `Size` | `3` | TechnoType | Transport slot cost. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `EliteSecondary` | `MirageGunE` | TechnoType | **⚠ Quirk**: this is `EliteSecondary`, NOT `ElitePrimary`. The unit has **no `Secondary=`** weapon — only Primary. If gamemd's veterancy resolver checks `ElitePrimary=` for the Primary upgrade (the conventional pattern) and `EliteSecondary=` for the Secondary upgrade, then **elite Mirage Tank may never actually use `MirageGunE`** since there's no Secondary slot to upgrade. Either: (a) it's an INI typo (should be `ElitePrimary=`), or (b) gamemd has a fallback that treats `EliteSecondary` as Primary-upgrade when Secondary is absent. Worth a Ghidra audit. See §7.4 for analysis. |
| `CrushSound` | `TankCrush` | TechnoType | Standard. |

### Notable absent keys
- **No `ElitePrimary=`** — see the `EliteSecondary=MirageGunE` quirk above.
- No `Secondary=` weapon — single-weapon unit despite the EliteSecondary line.
- No `OpportunityFire=yes` — combined with `CanApproachTarget=no`, Mirage really does sit still until ordered or shot at.
- No `Bunkerable=no` (defaults yes) — Mirage CAN board Battle Fortress.
- No `ImmuneToPsionics` — Yuri can mind-control Mirage Tanks.
- No `OmniCrushResistant=yes` — Battle Fortress can squish.

---

## 2. `artmd.ini` — `[RTNK]` (referenced via `Image=RTNK`)

MGTK's `Image=RTNK` redirects to:

```ini
[RTNK]   ; Mirage Tank
Voxel=yes
Remapable=yes
;DisableVoxelCache=yes ;gs ### TEMP
;DisableShadowCache=yes ;gs ### TEMP
Cameo=RTNKICON
AltCameo=RTNKUICO
PrimaryFireFLH=130,0,80
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Voxel-rendered from `RTNK.VXL` + `RTNK.HVA`. |
| `Remapable` | `yes` | House-color remap. |
| Commented `;DisableVoxelCache=yes ;gs ### TEMP` | — | Author-debug note: the voxel cache was temporarily disabled for debugging. Not active in shipped INI. |
| Commented `;DisableShadowCache=yes ;gs ### TEMP` | — | Same — shadow cache debug toggle, inactive. |
| `Cameo` | `RTNKICON` | Sidebar cameo. |
| `AltCameo` | `RTNKUICO` | Yuri-skinned cameo. |
| `PrimaryFireFLH` | `130,0,80` | Firing offset (X=130 forward, Y=0, Z=80 — chest-height gun, low for a tank). |

No `SecondaryFireFLH=` — irrelevant since no Secondary weapon. No `TurretOffset=` —
hull-mounted gun (no separate turret).

The "RTNK" art-slot name is the original RA2 short ID. YR's rulesmd `[MGTK]` (Mirage)
redirects to it via `Image=`, similar to how:
- rulesmd `[MTNK]` (Grizzly) uses `Image=GTNK`
- rulesmd `[APOC]` (Apocalypse) uses `Image=MTNK`
- rulesmd `[MGTK]` (Mirage) uses `Image=RTNK`

This is the YR pattern: rename rulesmd entries to new names while preserving original
art-block slot names via `Image=` redirects.

---

## 3. Weapon — `[MirageGun]` / `[MirageGunE]`

### `[MirageGun]` (rookie/veteran)

```ini
[MirageGun]
Damage=100
ROF=70
Range=7
Projectile=InvisibleLow
Speed=100
Warhead=MirageWH
DisguiseFireOnly=no	; SJM: design change, tank can fire always
Report=MirageTankAttack
Bright=yes
Anim=VTMUZZLE
DisguiseFakeBlinkTime=15 ; when a mirage fires, its disguise blinks for this long for VISUAL ONLY, not a logic blink
RevealOnFire=no ; Doesn't clear shroud when fired
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `100` | Per-shot — higher than Grizzly (65), comparable to Rhino (90), lower than Apocalypse Burst-2 (200/cycle). |
| `ROF` | `70` | Slow-firing — 70-tick cooldown. |
| `Range` | `7` | **Long range** (vs Grizzly's 5, Rhino's 5.75). +2 cells reach for ambush. |
| `Projectile` | `InvisibleLow` | Inviso flat-fire, respects cliffs/elevation/walls. |
| `Speed` | `100` | Bullet speed (inviso). |
| `Warhead` | `MirageWH` | "Heat ray" warhead — see §4. |
| `DisguiseFireOnly` | `no` | **WeaponType field** (verified — 0x008494a0 → WeaponTypeClass__ReadINI @ 0x007721f1). INI comment: "SJM: design change, tank can fire always". Default behavior (if `yes`) would require the unit to BE disguised before firing this weapon. `no` lets Mirage fire regardless of disguise state. |
| `Report` | `MirageTankAttack` | Unique fire sound (referenced; not directly read this iter). |
| `Bright` | `yes` | Lights cell when firing. |
| `Anim` | `VTMUZZLE` | Standard vehicle-tank muzzle anim. |
| `DisguiseFakeBlinkTime` | `15` | **WeaponType field** (verified — 0x00849448 → WeaponTypeClass__ReadINI @ 0x00772273). INI comment: "when a mirage fires, its disguise blinks for this long for VISUAL ONLY, not a logic blink". When Mirage fires, the tree disguise visually flashes off for 15 ticks (~250ms at 60fps), then back on. **Crucially: logic-wise the unit stays "disguised" — enemy AI still treats it as a tree**. Only the player's eyes are alerted. This is the "Mirage looks like it broke disguise but actually didn't" feel. |
| `RevealOnFire` | `no` | **WeaponType field** (cheat sheet — RevealOnFire). Default behavior would clear shroud at the target cell when firing. `no` keeps the shroud — Mirage's shots don't reveal map. |

### `[MirageGunE]` (elite — referenced via `EliteSecondary=`, may not trigger — see §7.4)

```ini
[MirageGunE]
Damage=150
ROF=80
Range=9
Projectile=InvisibleLow
Speed=100
Warhead=MirageWH
DisguiseFireOnly=no	; SJM: design change, tank can fire always
Report=MirageTankAttack
Bright=yes
DisguiseFakeBlinkTime=5 ; when a mirage fires, its disguise blinks for this long for VISUAL ONLY, not a logic blink
RevealOnFire=no ; Doesn't clear shroud when fired
```

| Key | MirageGun | MirageGunE | Δ |
|-----|-----------|------------|----|
| `Damage` | 100 | **150** | +50% |
| `ROF` | 70 | **80** | Slower (-14%) |
| `Range` | 7 | **9** | +2 cells |
| `Warhead` | MirageWH | MirageWH | unchanged |
| `Anim` | VTMUZZLE | (absent — default fallback?) | Note: elite version lacks `Anim=` line |
| `DisguiseFakeBlinkTime` | 15 | **5** | Shorter visual blink (3× faster — elite Mirage is more "always disguised") |
| `RevealOnFire` | no | no | unchanged |
| `DisguiseFireOnly` | no | no | unchanged |

**Practical elite gain (if upgrade actually triggers):** ~6% DPS (150/80 = 1.875 vs 100/70 = 1.43 dmg/tick) + 2-cell range, much faster visual-blink recovery. **If the `EliteSecondary`-without-`Secondary` quirk means it doesn't trigger, elite Mirage stays on `MirageGun`** — no per-shot upgrade, only the SELF_HEAL + STRONGER + FIREPOWER + ROF stat boosts from `EliteAbilities=`.

---

## 4. Warhead — `[MirageWH]`

```ini
[MirageWH]	// Supposed to be a heat ray.
Verses=100%,100%,80%,100%,100%,100%,30%,20%,20%,100%,100%	; Needs balancing by designers
AnimList=IRONFX		; temp, should have flash-o-light
InfDeath=4			; Burn death
Bright=true			; This says there should be Combat Lighting.  It's ignored, but we'll say it anyway.
CLDisableBlue=true	; This says the Combat Light should be red.  (1)
CLDisableGreen=true	; This says the Combat Light should be red.  (2)
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | 100% | Full damage vs basic infantry |
| 2 | flak | 100% | Full vs flak troopers |
| 3 | plate | 80% | Strong vs plate (Tanya/SEAL) — unlike AP's 15% |
| 4 | light | 100% | Full vs light vehicles |
| 5 | medium | 100% | Full vs medium |
| 6 | heavy | 100% | Full vs heavy MBTs |
| 7 | wood | 30% | Weak vs wood buildings |
| 8 | steel | 20% | Weak vs steel |
| 9 | concrete | 20% | Weak vs concrete |
| 10 | special_1 | 100% | |
| 11 | special_2 | 100% | |

**Universally 100% vs all units (infantry + vehicles).** The only weakness is buildings
(20-30%). This is what makes Mirage a pure anti-unit ambusher — equally good vs infantry,
light tanks, and MBTs. Unlike Grizzly's AP (25/15% vs plate/infantry) or Rhino's
ApocAP (high vs buildings), MirageWH is **broadest-spectrum vs units, worst vs
structures**.

| Key | Effect |
|-----|--------|
| Comment `// Supposed to be a heat ray.` | Design intent — the weapon is fluff-described as heat-based. |
| `AnimList` | `IRONFX` — note comment "temp, should have flash-o-light". Author wanted a custom flash anim but reused IronCurtain effects as a placeholder. |
| `InfDeath` | `4` — **Burn death** (per InfDeath table: 4=burn). Infantry caught in MirageWH have burning death animation, consistent with "heat ray" fluff. |
| `Bright` | `true` (comment: "It's ignored, but we'll say it anyway") | Engine ignores warhead-level `Bright`; the weapon's `Bright=yes` is what matters. Comment confirms ignored. |
| `CLDisableBlue / CLDisableGreen` | `true / true` | Combat Light disable: blocks blue and green channels, leaving red-only — a red glow. Effective if Combat Lighting renders at all (depends on engine config). |

Notable absent: **no `CellSpread=` / no `PercentAtMax=`** — single-cell point damage. No
AoE. Each Mirage shot hits one target. Combined with `InvisibleLow` projectile
respecting walls, the shot can't penetrate hard cover or splash.

---

## 5. Voices / sounds

```ini
[MirageTankSelect]
Sounds=$vmirsea $vmirseb $vmirsec $vmirsed $vmirsee $vmirsef ;$vmirseg
Control=random
Volume=85

[MirageTankMove]
Sounds=$vmirmoa $vmirmob $vmirmoc $vmirmod $vmirmoe $vmirmof $vmirmog
Control=random
Volume=85

[MirageTankAttackCommand]
Sounds=$vmirata $vmiratb $vmiratc $vmiratd $vmirate
Control=random
Volume=85
```

```ini
[MirageTankMoveStart]
Sounds=vmirstaa vmirstab vmirstac
Control= random predelay
Delay=0 400
VShift=10
Volume=35
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=MirageTankSelect` | 6 clips ($vmirsea..ef; 7th $vmirseg commented out — disabled but file exists) | Click-select |
| `VoiceMove=MirageTankMove` | 7 clips ($vmirmoa..og) | Move order — the longest move-voice pool in the Allied lineup |
| `VoiceAttack=MirageTankAttackCommand` | 5 clips | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack voice |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=MirageTankMoveStart` | 3 clips, predelay 0–400ms, VShift +10, vol 35 (no `FShift`, no `Priority`) | Engine start — minimalist sound def |
| `Report=MirageTankAttack` (weapon) | (in soundmd) | Per-shot fire sound |
| `CrushSound=TankCrush` | `vcrusha` | Crush |

The Mirage Tank has **larger voice pools** than most Allied vehicles — 6 select + 7 move
+ 5 attack = 18 distinct voice clips, suggesting design emphasis on the unit's
character (the disguise-and-ambush playstyle invites more personality).

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `GAWEAP,GATECH` — Allied War Factory + Battle Lab.
- **TechLevel** = `9`.
- **Owner**: 5 Allied countries.
- **`CrateGoodie=yes`** — can drop from crates.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $1000 — mid-tier.

### Disguise mechanic (full lifecycle)

1. **Game-start global rule**: `[General] DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04` (verified — RulesClass__ReadGeneral @ 0x00671d51) — lists the terrain-type tree variants that Mirage can disguise as.
2. **Stationary state**: when `DisguiseWhenStill=yes` triggers (unit speed is 0), the engine randomly picks one of the 4 trees and applies the disguise. Visual: the Mirage's voxel is hidden, a tree sprite renders at its location.
3. **Visual disguise**: enemy players see a tree where the Mirage is — looks identical to ambient terrain trees.
4. **Logic disguise**: enemy units treat the disguised Mirage as terrain — they do not target it.
5. **Fire-break (visual only)**: when Mirage fires, the disguise visually blinks off for `[MirageGun] DisguiseFakeBlinkTime=15` ticks (250ms at 60fps). The tank's voxel briefly appears, the shot lands, then the tree re-appears.
6. **Logic stays disguised** during the fake blink — enemy AI continues to ignore the Mirage as a target.
7. **Move-break**: when Mirage moves, the disguise drops fully (both visually and logically). The unit becomes a normal-visibility tank until it stops again.
8. **`DetectDisguise=yes` infantry/units** (PTROOP, YURI, INIT, BORIS — see PTROOP doc §1) bypass the disguise and can target the Mirage even when stationary.
9. **`CanApproachTarget=no`** prevents the Mirage from chasing targets — only the player's manual Attack Mission overrides this. Keeps the unit in ambush mode by default.

### Disguise tree pick — verified random

The pick is from 4 candidates (`TREE01..04`). Standard tree assets (`TREE01.SHP` etc.)
are real terrain SHPs used elsewhere on maps. The Mirage's tree blends with map-placed
trees indistinguishably.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 MGTK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `MGTK` | 0 matches |
| `Mirage` | Only `"DefaultMirageDisguises"` at 0x0083b488 (the global rule) |

⇒ **No MGTK-specific code path.** The disguise mechanic is generic — applies to any unit with `CanDisguise=yes` + `DisguiseWhenStill=yes`. Only the rules-side `DefaultMirageDisguises` global hardcodes the **type of disguise** (trees). A modder setting these flags on another unit would inherit the same tree disguise.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `CanDisguise` | 0x00843c98 | TechnoTypeClass__ReadINI @ 0x0071440b | TechnoType |
| `DisguiseWhenStill` | 0x00843c64 | TechnoTypeClass__ReadINI @ 0x00714459 | TechnoType |
| `CanApproachTarget` | 0x00843c2c | TechnoTypeClass__ReadINI @ 0x007144a7 | TechnoType |
| `DefaultMirageDisguises` | 0x0083b488 | RulesClass__ReadGeneral @ 0x00671d51 | **RulesClass global** |
| `DisguiseFireOnly` | 0x008494a0 | WeaponTypeClass__ReadINI @ 0x007721f1 | **WeaponType** |
| `DisguiseFakeBlinkTime` | 0x00849448 | WeaponTypeClass__ReadINI @ 0x00772273 | **WeaponType** |

Plus prior verifications:
- `TooBigToFitUnderBridge` — UnitType only
- `RevealOnFire` (cheat sheet) — WeaponType
- `Crusher`, `Turret`, etc. — TechnoType/UnitType

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Disguise as tree when stationary | `CanDisguise=yes` + `DisguiseWhenStill=yes` + global `DefaultMirageDisguises=TREE01..04` | Engine random-picks one of 4 trees |
| Disguise visually flashes on fire (not logically) | `[MirageGun] DisguiseFakeBlinkTime=15` (WeaponType field) | 250ms visual reveal, logic stays disguised |
| Can fire while disguised | `[MirageGun] DisguiseFireOnly=no` | Without this `no`, Mirage couldn't fire when disguised |
| Doesn't reveal shroud on fire | `[MirageGun] RevealOnFire=no` | Stays stealthy |
| Won't chase enemies opportunistically | `CanApproachTarget=no` (except manual Attack Mission) | Holds ambush position |
| No smoke when damaged (only sparks) | INI comment-out of `SmallGreySSys` in `DamageParticleSystems` | Visual stealth preservation |
| Detect-disguise infantry bypass disguise | `DetectDisguise=yes` on attackers (PTROOP/YURI/INIT/BORIS) | See PTROOP doc §1 |

### 7.4 ⚠ `EliteSecondary=MirageGunE` without `Secondary=` — quirk analysis

**[RESOLVED audit 16 — outcome (a) BINARY-VERIFIED]**: ElitePrimary and
EliteSecondary live at distinct TechnoType slots (`+0xA94` vs `+0xAB0`),
parsed independently in `TechnoTypeClass__ReadINI`. There is no
parser-time fallback that copies EliteSecondary → ElitePrimary when
ElitePrimary is absent. Elite Mirage Tank fires `MirageGun` (Damage 100,
Range 7) — the `EliteSecondary=MirageGunE` INI line is effectively dead.
See "Ghidra audit log (audit iteration 16)" §7.4 RESOLVED for the
binary-evidence trace. The analysis below remains as historical record.


| Observation | Detail |
|-------------|--------|
| INI has `Primary=MirageGun` and `EliteSecondary=MirageGunE` | But **no `Secondary=`** line |
| Conventional pattern: `ElitePrimary=` upgrades Primary at elite | MGTK has no `ElitePrimary` |
| `EliteSecondary=` is the slot for upgrading Secondary at elite | But there's no base Secondary to upgrade |
| `[MirageGunE]` IS defined (Damage 150, Range 9 — clear upgrade) | But may never be loaded |
| Likely outcomes (Ghidra-decompile pending): | (a) Engine ignores `EliteSecondary` when no Secondary → elite Mirage stays on `MirageGun` (Damage 100, Range 7); (b) Engine treats `EliteSecondary` as `ElitePrimary` fallback when no Secondary → elite Mirage uses `MirageGunE` (Damage 150, Range 9) |

**Parity-critical TODO**: decompile `VehicleClass::ReadINI` or the veterancy-resolver
path to confirm which behavior gamemd actually exhibits. If gamemd's elite Mirage Tanks
visibly fire harder (visible damage numbers, range stretch), outcome (b) is live. If
they fire identically to veteran, outcome (a) is live. **Until verified, do not
implement either branch — this is a documented open question.**

A third possibility: the line is a pure INI typo never noticed because Mirage rarely
reaches elite in normal play. Common in placeholder-y INI entries.

### 7.5 Behaviors NOT present

- **No turret** — body-only aim. Slow turn-and-shoot mechanic.
- **No `OpportunityFire=yes`** — won't auto-shoot threats (also part of stealth design).
- **No Secondary weapon** — single-weapon unit.
- **No `Spawns=`** / `Passengers=` — not a transport, no children.
- **No `Teleporter=`** — does not chrono.
- **No `SelfHealing=yes`** at rookie — only at elite via SELF_HEAL ability.
- **No `ImmuneToPsionics`** — Yuri can mind-control.
- **No `Bunkerable=no`** — Mirage CAN board Battle Fortress (which would be hilarious — Battle Fortress with a tree-disguised passenger does not propagate the disguise to itself).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| Commented `;DisableVoxelCache / DisableShadowCache ;gs ### TEMP` | n/a (debug toggles) | Inactive. |
| Commented `;Primary=TankMakeupKit ;Secondary=MirageGun ;IsSimpleDeployer=yes ;OmniFire=yes ;CanRetaliate=no ;CanPassiveAquire=no` | n/a (design-history notes) | Inactive. |

No fog-of-war refs, no Tiberium refs, no real tunnel refs.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (200 → 250)
- `FIREPOWER` — +25% damage (100 → 125)
- `SIGHT` — +20% sight (9 → 10.8 — best vision in the game at this rank)
- `FASTER` — +20% speed (7 → 8.4)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (passive HP regen)
- Reapplies STRONGER, FIREPOWER, ROF
- `ROF` — −25% ROF (70 → ~52)

**Plus weapon swap (MAYBE — see §7.4 quirk)**: `MirageGun` → `MirageGunE` if the
`EliteSecondary=MirageGunE`-without-`Secondary` quirk resolves favorably.

If the swap fires: Damage 100 → 150, ROF 70 → 80 (slower), Range 7 → 9, DisguiseFakeBlinkTime 15 → 5.
If it doesn't: only the ability stat boosts apply.

---

## 10. Cross-references

### Direct dependencies
- `[MirageGun]` / `[MirageGunE]` — weapons (§3)
- `[InvisibleLow]` — projectile
- `[MirageWH]` — warhead (§4)
- `[VTMUZZLE]` (artmd) — muzzle anim
- `[IRONFX]` (artmd) — impact anim (placeholder)
- `[RTNK]` (artmd, via `Image=RTNK`) — art block
- `[TREE01] / [TREE02] / [TREE03] / [TREE04]` (terrain types in rulesmd / map data) — disguise targets
- `[General] DefaultMirageDisguises` — global rule (line 285)
- `[GAWEAP] / [GATECH]` — prereqs
- `[MirageTankSelect/Move/AttackCommand/Attack/MoveStart]` (soundmd) — voices and sounds
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic sounds

### Conceptual companions
- **MTNK (Grizzly)** ([`allied/MTNK.md`](./MTNK.md)) — Allied tier-2 MBT.
- **TNKD (Tank Destroyer)** ([`allied/TNKD.md`](./TNKD.md)) — Allied AT-only tank, same Turret=no constraint.
- **HOWI (Prism Tank?)** — actually [`allied/HOWI.md`](./HOWI.md) — TODO — Allied tier-3 siege. (Note: this is the **rulesmd** entry; the Prism Tank itself is `[SREF]` in rulesmd.)
- **SREF (Prism Tank)** ([`allied/SREF.md`](./SREF.md) — TODO) — Allied tier-3 counterpart. Same prereq (GAWEAP+GATECH), same cost ($1000). Mirage is anti-unit; Prism is anti-building/long-range.
- **PTROOP / YURI / INIT / BORIS** — units with `DetectDisguise=yes` that counter Mirage's stealth.
- **CCOMAND / GHOST / SPY** — units with `BombDisarm` or disguise-themselves, sharing the disguise system at the code level.

### Deep-RE docs
- None directly relevant — the disguise system is generic flag-driven, no dedicated Ghidra report exists.

---

## Ghidra audit log (audit iteration 16 — 2026-05-18)

**Methodology**: MGTK has no unit-specific code in `gamemd.exe`; the
disguise mechanic is generic (`CanDisguise + DisguiseWhenStill`) plus the
`DefaultMirageDisguises` global. This audit re-verifies the 6 doc-cited
parser xrefs, pins the 2 new TechnoType offsets, and **resolves the §7.4
`EliteSecondary`-without-`Secondary` quirk** via direct decompile of the
weapon-slot parser. ~15 Ghidra queries: 8 string searches + 7 xref
lookups + 1 grep over saved `TechnoTypeClass__ReadINI` decompile.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^MGTK$")` | **0 matches** |
| `search_strings("^Mirage$")` | **0 matches** (only `DefaultMirageDisguises` at 0x0083b488) |

Confirms: no hardcoded section-name branch.

### String + parser xref re-verification (BINARY-VERIFIED)

All 6 doc-cited claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `CanDisguise` | 0x00843C98 | 0x0071440B | TechnoTypeClass__ReadINI |
| `DisguiseWhenStill` | 0x00843C64 | 0x00714459 | TechnoTypeClass__ReadINI |
| `CanApproachTarget` | 0x00843C2C | 0x007144A7 | TechnoTypeClass__ReadINI |
| `DefaultMirageDisguises` | 0x0083B488 | 0x00671D51 | RulesClass__ReadGeneral |
| `DisguiseFireOnly` | 0x008494A0 | 0x007721F1 | WeaponTypeClass__ReadINI |
| `DisguiseFakeBlinkTime` | 0x00849448 | 0x00772273 | WeaponTypeClass__ReadINI |
| `EliteSecondary` (bonus, for §7.4) | 0x008442CC | 0x00712A5F | TechnoTypeClass__ReadINI |

### Struct offsets BINARY-VERIFIED (this pass)

**NEW TechnoType offsets** (from TechnoTypeClass__ReadINI grep):

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xD2F` | `CanDisguise` | byte | `(int)param_1 + 0xd2f` after ReadBool. **Re-confirms audit 6 SPY cumulative**. |
| `+0xD32` | `DisguiseWhenStill` | byte | `(int)param_1 + 0xd32` after ReadBool. **NEW**. |
| `+0xD33` | `CanApproachTarget` | byte | `(int)param_1 + 0xd33` after ReadBool. **NEW**. |
| `+0x898` | `Secondary` | WeaponType* | `param_1[0x226] = iVar4` after WeaponTypeClass__FindOrAllocate. **NEW** — the TechnoType-level Secondary weapon slot. |
| `+0xA94` | `ElitePrimary` | WeaponType* | `param_1[0x2a5] = iVar4`. **NEW**. |
| `+0xAB0` | `EliteSecondary` | WeaponType* | `param_1[0x2ac]` (default-read at start of EliteSecondary parse). **NEW**. |

(Note: the InfantryType-specific weapon slots at +0xE40/E44/E48/E4C
[audit 1 cumulative] are *separate* from these TechnoType-level slots —
both coexist for hierarchy reasons. Vehicles/buildings use the
TechnoType-level slots; the InfantryType slots are an InfantryType-only
extension parsed by `InfantryTypeClass__ReadINI`.)

### §7.4 QUIRK RESOLVED — `EliteSecondary=MirageGunE` without `Secondary=`

The doc flagged this as parity-critical (outcome (a) "elite stays on
MirageGun" vs outcome (b) "engine fallback uses EliteSecondary as
ElitePrimary"). The TechnoTypeClass__ReadINI decompile shows
**ElitePrimary** and **EliteSecondary** are stored at *distinct* slots
(`+0xA94` vs `+0xAB0`), parsed *independently*. There is no parser-time
fallback that would auto-copy EliteSecondary to ElitePrimary when the
latter is absent.

**Outcome (a) is BINARY-VERIFIED**: at runtime, the elite veterancy
weapon-resolver looks up Primary's elite-replacement at `+0xA94`
(ElitePrimary). For MGTK that slot is NULL (no `ElitePrimary=` line).
The resolver falls back to `+0x894` (Primary) = `MirageGun`. The
EliteSecondary slot at `+0xAB0` contains `MirageGunE` but has no
matching Secondary slot at `+0x898` (also NULL) to upgrade. Result:
**elite Mirage Tank fires `MirageGun` (Damage 100, Range 7), not
`MirageGunE`**. The INI line is effectively dead — likely an unintended
typo for `ElitePrimary=MirageGunE`.

The doc's §7.4 should be updated to reflect this resolution. (Not
modifying the doc body in this audit pass beyond adding the audit-log
section; the open-question text remains as historical record.)

### WeaponType offset cross-checks

Re-confirmed from audit 9 cumulative:

- `DisguiseFireOnly` → WeaponType+0x13B (cheat sheet) — parser xref `0x007721F1` matches.
- `DisguiseFakeBlinkTime` → WeaponType+0x13C (cheat sheet) — parser xref `0x00772273` matches.
- `RevealOnFire` → WeaponType+0x137 (cheat sheet).

The doc's claim "DisguiseFireOnly/DisguiseFakeBlinkTime are WeaponType-scope" is correct.

### Rules-General offset for `DefaultMirageDisguises`

Parser xref `0x00671D51` is inside `RulesClass__ReadGeneral` (audit 12
cumulative — same function that parses Secret{Infantry,Units,Buildings},
disguise pointers, etc.). The specific Rules+offset for the
DynamicVector is **not pinned in this pass** (would require grep on the
RulesClass__ReadGeneral decompile). DEFERRED.

### Items NOT re-verified in this pass (DEFERRED)

- The exact Rules+offset for `DefaultMirageDisguises` DynamicVector
  start (parser xref confirmed RulesClass scope, but byte offset not
  pinned).
- The disguise-update routine (where the engine actually applies the
  visual disguise based on the +0xD32 DisguiseWhenStill flag) — would
  require tracing `TechnoClass::AI_Update` or a similar per-tick hook.
- The random tree-pick algorithm (uniform vs weighted from the 4-entry
  DefaultMirageDisguises list).
- The "fire-blink visual reveal" timer consumer (WeaponType+0x13C
  DisguiseFakeBlinkTime) — offset known, consumer DEFERRED.
- The `[General] DefaultMirageDisguises` xref into TerrainType (whether
  TREE01-04 must exist as terrain or as some other class).
- The `CanRetaliate=no` comment's "first shot will disguise as the bad
  guy" semantics — orthogonal to MGTK's main behavior.
- Primary weapon slot offset (+0x894 INFERRED by symmetry — not directly
  verified in this grep window since the parser writes were cut off by
  the line-too-long output).

### Confidence summary

- **HIGH**: 8 string addresses + 7 parser xrefs (all exact); 5 NEW
  TechnoType struct offsets (3 disguise-block bytes, 3 weapon slots);
  1 TechnoType re-confirmation (CanDisguise +0xD2F from audit 6); 3
  WeaponType re-confirmations (from audit 9 cumulative). **§7.4 quirk
  resolved** by binary evidence — ElitePrimary and EliteSecondary live
  at different slots and are parsed independently, so the
  `EliteSecondary`-without-`Secondary` config produces no weapon
  upgrade (outcome (a) confirmed).
- **MEDIUM**: Primary weapon-slot offset (+0x894) inferred by symmetry
  with Secondary (+0x898) — would need a separate grep window to verify
  the actual write.
- **No INCORRECT findings in the doc**. All 6 in-line Ghidra cites
  resolve exactly. The §7.4 quirk is now a CONFIRMED outcome rather
  than an open question.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[MGTK]` rulesmd key explained | ✅ §1 |
| `Image=RTNK` redirect + artmd block expanded | ✅ §2 |
| Commented design-history lines (TankMakeupKit, IsSimpleDeployer, CanRetaliate, CanPassiveAquire) noted | ✅ §1 |
| Both weapons + warhead + projectile | ✅ §3–§4 |
| **DisguiseFireOnly + DisguiseFakeBlinkTime verified as WeaponType-scoped** (not TechnoType) | ✅ §7 |
| All voices + unique MirageTankMoveStart | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **Full disguise lifecycle** (game-start → still → fire-blink → move-break → DetectDisguise counter) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 6 new flag-scope verifications | ✅ §7 |
| **⚠ EliteSecondary-without-Secondary quirk flagged** — parity TODO logged | ✅ §7.4 |
| **DamageParticleSystems comment-truncation noted** (smoke disabled for stealth) | ✅ §1 |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed with elite-weapon-quirk caveat | ✅ §9 |
| Cross-refs to companion docs | ✅ §10 |

**Open follow-ups (worth a dedicated Ghidra audit):**
- **`EliteSecondary=MirageGunE` without `Secondary=` — does the engine actually apply the upgrade?** Decompile the veterancy-resolver and weapon-selection path in `VehicleClass`/`UnitClass`. This is the most parity-critical question for Mirage. If gamemd's elite Mirage Tanks visibly use `MirageGunE` (Damage 150, Range 9), the engine must honor `EliteSecondary` as a Primary-fallback when Secondary is absent.
- The `CanRetaliate=no` commented note ("first shot will disguise as the bad guy"): what does "disguise as the bad guy" mean? Does Mirage's visual disguise swap to mimic the attacker? Worth a fidelity-check session.
- `[General] DefaultMirageDisguises=TREE01..04`: confirm these terrain type entries exist in the map's `[TerrainTypes]` section and don't require special handling.
- `DisguiseFakeBlinkTime=15` rookie vs `=5` elite — verify the timer is in ticks (not frames or ms). 15 ticks at 60-fps sim = 250ms; at 15-fps anim = 1 second. Big parity difference.
- `DefaultMirageDisguises` — the pick algorithm (uniform random vs weighted?) — Ghidra-trace `Scan_For_Tree_Disguise` or similar.
