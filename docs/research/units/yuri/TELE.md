# TELE — Magnetron (Yuri Tier-3 Anti-Vehicle Levitator)

**INI ID:** `TELE` (the art-side comment says "Telekenetic Tank" — early development name).
**Display Name:** `Magnetron` (`UIName=Name:Magnetron`)
**Side:** Yuri (`Owner=YuriCountry`)
**Category:** Vehicle / AFV
**Cameo:** `TELEICON` / `TELEUICO` (AltCameo)
**Voxel:** yes (with `TurretOffset=-100`)

The Magnetron is Yuri's tier-3 anti-vehicle specialist. Its primary weapon
fires the `LocomotorBeam` warhead, which **temporarily replaces the target
vehicle's locomotor with a Jumpjet locomotor**, lifting the vehicle off the
ground and rendering it immobile (and unable to fire). Secondary weapon is an
anti-building shake that does real damage. The effect is one of the most
distinctive and disruptive mechanisms in YR.

---

## INDEX CORRECTIONS — IMPORTANT

During this doc's research, **two stale index entries were corrected**:

1. **`[UTNK]` is NOT the Magnetron.** UTNK is a vestigial test placeholder
   (`Name=ZZZ Not Used`, Soviet `Owner=Russians,Confederation,Africans,Arabs`,
   `TechLevel=-1`, `Image=HTNK`, `Primary=Comet`, `MovementZone=Destroyer`).
   It's a dead INI entry, possibly an early-development clone of the Magnetron
   slot that got repurposed as a test stub. Index now marked SKIP-DUPLICATE.
2. **`[TELE]` IS the Magnetron.** The index labeled it "Chrono trooper transport?"
   — entirely wrong. The MAGNETRON_SYSTEM_GHIDRA_REPORT.md explicitly states the
   Magnetron's unit ID is `[TELE]`, and the rulesmd block at line 8586 has
   `Name=Magnetron`. Index now corrected.

> **Cross-references — do not re-derive:**
> - [`MAGNETRON_SYSTEM_GHIDRA_REPORT.md`](../../MAGNETRON_SYSTEM_GHIDRA_REPORT.md) (564 lines) — full Detonate path, WarheadTypeClass offset table, IsLocomotor / Locomotor GUID dispatch chain, BulletClass field accesses, Apply_area_damage flow, piggyback-locomotor swap mechanism. **All deep RE is in this doc; cross-reference rather than re-derive.**
> - [`JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) (900 lines) — the locomotor that gets piggybacked onto the lifted target.
> - [`CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md`](../../CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md) — same `IsLocomotor=yes` mechanism but with TeleportLocomotion CLSID instead of Jumpjet.
> - [`DISK.md`](./DISK.md) — sibling Yuri tier-3 with its own dedicated sim class (DiskLaserClass).
> - [`MIND.md`](./MIND.md) — sibling Yuri tier-3 with mind-control weapon.

> **TS-legacy filter:** `;Image=V3`, `;TargetLaser=yes`, `;yes` on Crusher are INI comments (drafts). The `Locomotor={4A582741-...}` = DriveLocomotionClass is the standard ground-vehicle locomotor (TS legacy GUID retained). `IsLocomotor=yes` + `Locomotor=` mechanism on the LocomotorBeam warhead is **live YR**, not TS — it's the engine for both Chronosphere (TeleportLoc) and Magnetron (JumpjetLoc). Confirmed live by MAGNETRON_SYSTEM doc.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
; Magnetron
[TELE]
UIName=Name:Magnetron
Name=Magnetron
;Image=V3
Prerequisite=YAWEAP,NAPSIS
Primary=MagneticBeam
Secondary=MagneShake
Strength=150
Category=AFV
Armor=light
Turret=yes
IsTilter=yes
;TargetLaser=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=10
Speed=5
CrateGoodie=no
Crusher=no;yes
Owner=YuriCountry
Cost=1000
Soylent=1000
Points=25
ROT=5
AllowedToStartInMultiplayer=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MagnetronSelect
VoiceMove=MagnetronMove
VoiceAttack=MagnetronAttackCommand
VoiceSecondaryWeaponAttack=MagnetronMagneShakeVoice
DieSound=GenVehicleDie
MoveSound=MagnetronMoveStart
CrushSound=TankCrush
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Destroyer
ThreatPosed=40	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
DamageSmokeOffset=100, 100, 275
Weight=3.5
VeteranAbilities=STRONGER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,ROF
Accelerates=false
ZFudgeColumn=8
ZFudgeTunnel=13
Size=6
OpportunityFire=yes
ElitePrimary=MagneticBeamE
Bunkerable=no; Units default to yes, others default to no
CanPassiveAquire=no ; Won't try to pick up own targets ;GEF don't want him accidently attacking your own guys
```

### 1.1 Key-by-key explanation

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:Magnetron` | string | AbstractTypeClass | CSF lookup. |
| `Name=Magnetron` | string | AbstractTypeClass | English fallback. |
| `;Image=V3` | (commented) | — | Disabled. Would have used V3 voxel as visual placeholder. The active artmd `[TELE]` block specifies its own voxel. |
| `Prerequisite=YAWEAP,NAPSIS` | building list | TechnoTypeClass | **Yuri War Factory + Psychic Sensor.** Notable: NAPSIS is the Psychic Sensor (originally Soviet-only in RA2, made available to Yuri in YR). The Magnetron is gated behind tier-2/3 psychic tech. |
| `Primary=MagneticBeam` | weapon | TechnoTypeClass | The locomotor-hijack beam. See §3.1. |
| `Secondary=MagneShake` | weapon | TechnoTypeClass | Anti-building physical-damage shake. See §3.2. |
| `Strength=150` | hp | TechnoTypeClass | **Only 150 HP** — extremely fragile (compare DISK 600, MIND 500). Glass-cannon design — the Magnetron must be kept at range. |
| `Category=AFV` | enum | TechnoTypeClass | AFV category. |
| `Armor=light` | enum | TechnoTypeClass | Light armor — vulnerable to almost everything. |
| `Turret=yes` | bool | UnitTypeClass | Has a rotating turret. |
| `IsTilter=yes` | bool | UnitType @ 0x00747712 | Body tilts when changing direction. |
| `;TargetLaser=yes` | (commented) | — | Would have shown a targeting laser (red dot on target). Disabled. |
| `TooBigToFitUnderBridge=true` | bool | UnitType @ 0x0074774e | Cannot path under bridges. |
| `TechLevel=2` | int | TechnoTypeClass | TechLevel 2 — same as DISK/MIND; actual gating via Prerequisite (Battle Lab tier). |
| `Sight=10` | cells | TechnoTypeClass | **10-cell sight** — longest of any vehicle (tied with MIND); essential for the 12-cell weapon range. |
| `Speed=5` | int | TechnoTypeClass | Moderate ground speed. |
| `CrateGoodie=no` | bool | UnitType @ 0x00747658 | No crate pop. |
| `Crusher=no;yes` | bool | TechnoTypeClass | No crush. `;yes` draft. |
| `Owner=YuriCountry` | country list | TechnoTypeClass | **Yuri only**. |
| `Cost=1000` | credits | TechnoTypeClass | Tier-3 cost (cheaper than DISK 1750, on par with MIND ~1500). |
| `Soylent=1000` | credits | TechnoTypeClass | Full recycle value. |
| `Points=25` | int | TechnoTypeClass | Score on kill. |
| `ROT=5` | int | TechnoTypeClass | Moderate turret turn rate. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Combat unit. |
| `Explosion=...` | anim list | TechnoTypeClass | Standard 5-anim destruction set. |
| `VoiceSelect=MagnetronSelect` | sound | TechnoTypeClass | Unique Magnetron select voice (sound:4685). |
| `VoiceMove=MagnetronMove` | sound | TechnoTypeClass | Unique move voice (sound:4690). |
| `VoiceAttack=MagnetronAttackCommand` | sound | TechnoTypeClass | Unique attack voice (sound:4695). |
| `VoiceSecondaryWeaponAttack=MagnetronMagneShakeVoice` | sound | TechnoType @ 0x00844038 | **Separate voice for Secondary weapon** (sound:4700) — playerheard cue distinguishes "lifting unit" (Primary) from "shaking building" (Secondary). Mirrors DISK's pattern. |
| `DieSound=GenVehicleDie` | sound | TechnoTypeClass | Standard vehicle death (`gendiea` clip). |
| `MoveSound=MagnetronMoveStart` | sound | TechnoTypeClass | One-shot engine start sound (sound:5238). |
| `CrushSound=TankCrush` | sound | TechnoTypeClass | Crush sound (irrelevant — Crusher=no). |
| `Maxdebris=3` | int | TechnoTypeClass | Up to 3 debris pieces on destruction. |
| `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` | CLSID | TechnoTypeClass | **DriveLocomotionClass** (standard ground vehicle) — note this is the locomotor of the *Magnetron itself*, NOT the locomotor it grants to victims (that one is on the LocomotorBeam warhead, see §3.3). |
| `MovementZone=Destroyer` | enum | TechnoTypeClass | Ground MBT-class pathfinding zone. |
| `ThreatPosed=40` | int | TechnoTypeClass | High AI threat weight (40 — vs DISK's 20). The AI prioritizes destroying Magnetrons because they neutralize entire tank columns. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `DamageSmokeOffset=100, 100, 275` | x,y,z leptons | TechnoType @ 0x00713e25 | Smoke emission offset. |
| `Weight=3.5` | float | TechnoTypeClass | Fractional weight (uncommon). |
| `VeteranAbilities=STRONGER,SIGHT,FASTER` | ability list | TechnoTypeClass | **Veteran: +HP, +sight, +speed. Notably NO FIREPOWER and NO ROF** (the LocomotorBeam doesn't deal real damage, so FIREPOWER would have no effect — design choice acknowledges this). |
| `EliteAbilities=SELF_HEAL,STRONGER,ROF` | ability list | TechnoTypeClass | **Elite: passive self-heal, +HP, +ROF (faster lifts). Still no FIREPOWER.** ROF improves the cooldown between locomotor-swap attempts. |
| `Accelerates=false` | bool | TechnoTypeClass | Constant speed — no ramp-up. |
| `ZFudgeColumn=8` / `ZFudgeTunnel=13` | int | TechnoTypeClass | Z-buffer render tweaks vs columns/tunnels. |
| `Size=6` | int | TechnoTypeClass | Transport-cost. |
| `OpportunityFire=yes` | bool | TechnoType @ 0x0071483d | Engages opportunistic targets during movement. |
| `ElitePrimary=MagneticBeamE` | weapon | TechnoType @ 0x00712a32 | Elite-rank Primary swap to `[MagneticBeamE]` (10000 dmg vs 5000 — same locomotor effect, double the raw damage number — but since LocomotorBeam doesn't apply damage normally, this is effectively cosmetic too). |
| `Bunkerable=no` | bool | TechnoType @ 0x0071500a (NEW) | **Cannot be garrisoned into a structure.** Verified TechnoType scope. The inline comment is important: "Units default to yes, others default to no" — meaning *infantry* default to bunkerable, vehicles default to not, but the Magnetron explicitly opts out anyway (defense against being garrisoned in some moddable way). |
| `CanPassiveAquire=no` | bool | TechnoType @ 0x00714473 | **Will NOT auto-acquire targets.** Inline comment is the key: "don't want him accidently attacking your own guys" — because the Magnetron's lift effect can affect friendly units in some edge cases? Actually, more likely because passive-acquire could ruin the player's setup (lifting the wrong tank in a battle, blocking line-of-fire). The player must explicitly order each attack. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[TELE] ; Telekenetic Tank
Cameo=TELEICON
AltCameo=TELEUICO
Voxel=yes
TurretOffset=-100
Remapable=yes
PrimaryFireFLH=85,0,130;50,0,100
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=TELEICON` | SHP | Standard cameo. |
| `AltCameo=TELEUICO` | SHP | Alternate "unbuildable" cameo. |
| `Voxel=yes` | bool | `tele.vxl` + `.hva` voxel render. |
| `TurretOffset=-100` | int | **Turret rendered 100 leptons backward from body center** — affects the visible turret position on the voxel chassis. |
| `Remapable=yes` | bool | House-color tinted (Yuri purple). |
| `PrimaryFireFLH=85,0,130;50,0,100` | x,y,z leptons | **TWO FLH values separated by `;`** — the engine uses the first for the *main* firing position (85 forward, 0 sideways, 130 up = top of the upraised turret-coil) and the second for an alternate (50,0,100). The `;` here is a multi-value separator, NOT a comment (since the second value starts with another number). Used for multi-frame or per-burst FLH selection. |

> The "Telekenetic Tank" art-side comment is a development-era name. The unit was repurposed/renamed to Magnetron before shipping.

---

## 3. Weapons

### 3.1 `[MagneticBeam]` — primary (locomotor lift)

```ini
[MagneticBeam]
Damage=5000
ROF=20
Range=12
MinimumRange=3
Speed=100
Projectile=InvisibleHigh
Warhead=LocomotorBeam
Report=MagnetronAttack
;IsRadBeam=yes
IsMagBeam=yes
```

| Key | Effect |
|-----|--------|
| `Damage=5000` | **HUGE damage value — but does NOT apply to the target in normal damage terms.** The LocomotorBeam warhead's `Verses=` excludes infantry and structures (0 % for armor classes 0,1,2,6,7,8,10). For vehicles (classes 3,4,5,9), the warhead has 100 % vs damage application — but the actual mechanism is the locomotor swap, NOT raw HP reduction. The 5000 figure is essentially internal-bookkeeping to ensure the warhead trigger fires (some game logic gates on damage > 0). |
| `ROF=20` | **Very fast ROF (~1.3 sec)** — relative to the 5-second lift duration, the Magnetron can re-engage almost immediately after a target lands. |
| `Range=12` | 12-cell range — among the longest in the game (only V3 with 18 and Dread/Carrier with 25 exceed it). |
| `MinimumRange=3` | Cannot engage targets closer than 3 cells (the beam needs distance to "form"). |
| `Speed=100` | Beam moves "fast" (instantaneous visual). |
| `Projectile=InvisibleHigh` | Bookkeeping invisible projectile. |
| `Warhead=LocomotorBeam` | The actual locomotor-swap warhead. See §3.3. |
| `Report=MagnetronAttack` | Attack sound (sound:1670). |
| `;IsRadBeam=yes` | (commented) — would have routed to RadBeamClass; disabled. |
| `IsMagBeam=yes` | **WeaponType @ 0x0077223f? No — verified at 0x007728f0 (NEW THIS DOC).** Hardcoded flag — when set, the engine renders the beam in `[General] MagnaBeamColor=255,200,255` (pink-purple) using a dedicated magnetic-beam visualization (likely a variant of LaserDrawClass). |

### 3.2 `[MagneShake]` — secondary (anti-building)

```ini
[MagneShake]
Damage=80
ROF=110
Range=10
Projectile=InvisibleHigh
Spread=2
Speed=40
Report=MagnetronMagneShake
Warhead=MagneShakeWH
;Bright=yes
;LaserInnerColor = 216,0,184
;LaserOuterColor = 80,0,88
;LaserOuterSpread= 0,0,0
;LaserDuration = 15
;IsLaser=true	; this flag tells the game to use the special laser draw effect
IsMagBeam=yes
```

| Key | Effect |
|-----|--------|
| `Damage=80` | 80 base damage. |
| `ROF=110` | ~7.3 sec between shots. Slower than Primary because each MagneShake is a more deliberate building-attack. |
| `Range=10` | 10-cell range (2 cells less than Primary). |
| `Projectile=InvisibleHigh` | Bookkeeping invisible projectile. |
| `Spread=2` | Spread factor (legacy field; probably affects damage falloff). |
| `Speed=40` | Beam speed. |
| `Report=MagnetronMagneShake` | Attack sound (sound:5704). |
| `Warhead=MagneShakeWH` | Anti-building shake warhead. See §3.4. |
| `;Bright=yes`, `;LaserInnerColor=...`, `;LaserOuterColor=...`, `;LaserOuterSpread=...`, `;LaserDuration=...`, `;IsLaser=true` | (all commented) — would have rendered as a standard laser; disabled in favor of MagBeam routing. |
| `IsMagBeam=yes` | Routes to magnetic-beam visual (same as Primary). |

### 3.3 `[LocomotorBeam]` warhead — THE locomotor-swap mechanism

```ini
[LocomotorBeam]
;GEF can only grab units and terror drones
Verses=0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,0%
;Verses=100%,0%,20%,10%,0%
;InfDeath=5
IsLocomotor=yes
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}
;Spread=0
```

| Key | Effect |
|-----|--------|
| `;GEF can only grab units and terror drones` | (comment) — author's note explaining the Verses gate. |
| `Verses=0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,0%` | **Critical targeting gate:** 100 % vs armor classes 3 (light vehicle), 4 (medium vehicle), 5 (heavy vehicle), 9 (special_1 — terror drone). 0 % vs everything else (infantry: 0/1/2, structures: 6/7/8, special_2: 10). **The warhead only "affects" ground vehicles** (and terror drones via special_1). |
| `;Verses=...` | (commented) — alternate older Verses curve, disabled. |
| `;InfDeath=5` | (commented) — would have been an infantry-death animation, but since infantry are immune (Verses 0 %) it's irrelevant. |
| `IsLocomotor=yes` | **WarheadType @ 0x14B (per MAGNETRON_SYSTEM_GHIDRA_REPORT §2.1). Verified at 0x0075d86b xref to 0x00847d3c.** The critical flag. When set, the warhead's Detonate path piggybacks the `Locomotor=` GUID onto the target's TechnoClass, replacing their normal locomotor for a fixed duration. Mechanism is shared with Chronosphere (same flag, but Chronosphere uses TeleportLocomotion CLSID). |
| `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}` | **WarheadType +0x15C..+0x16B (16-byte GUID, per MAGNETRON_SYSTEM_GHIDRA_REPORT §2.1).** Read via `CCINIClass::ReadCLSID`. **`{92612C46-...}` = JumpjetLocomotionClass.** When the LocomotorBeam hits a vehicle, the engine swaps the vehicle's locomotor to Jumpjet — the vehicle lifts off the ground per Jumpjet's normal flight rules. |
| `;Spread=0` | (commented) — would have made it a point-target warhead; disabled. |

**Full hardcoded mechanism (see MAGNETRON_SYSTEM_GHIDRA_REPORT for details):**

1. WeaponType `MagneticBeam.IsMagBeam=yes` selects the magnetic-beam visual rendering (color = `[General] MagnaBeamColor`).
2. WeaponType triggers BulletClass to detonate `Warhead=LocomotorBeam` on the target.
3. WarheadType `LocomotorBeam.Verses=` gates to vehicles+terror-drones only.
4. WarheadType `LocomotorBeam.IsLocomotor=yes` triggers the locomotor-swap branch in `WarheadTypeClass__Detonate` @ `0x004690B0`.
5. The `Locomotor={92612C46-...}` GUID is read from WarheadType+0x15C and piggybacked onto the target TechnoClass.
6. Target vehicle: normal locomotor saved, Jumpjet locomotor activated. Jumpjet logic immediately starts lifting the target into the air.
7. Target cannot move or fire (Jumpjet auto-flight controls it, not the player).
8. Lift duration (the end-of-effect teardown) is governed by a piggyback-release timer on the target — per MAGNETRON_SYSTEM doc this part of the implementation is MEDIUM-confidence and not fully traced; typically observed at ~5-7 sec in standard gameplay.
9. When timer expires: original locomotor restored, vehicle drops to ground, taking falling damage based on JumpjetCrash rate + FallingDamageMultiplier (Rules global from CombatDamage section).

### 3.4 `[MagneShakeWH]` warhead — anti-building

```ini
[MagneShakeWH]
Verses=0%,0%,0%,0%,100%,0%,100%,100%,100%,0%,0%
Bullets=yes
```

- `Verses=0/0/0/0/100/0/100/100/100/0/0` — **Zero damage vs infantry (0/1/2), light/heavy vehicles (3/5), special (9/10). 100 % vs medium vehicles (4), wood/steel/concrete structures (6/7/8).** This curve gates the secondary specifically to:
  - **Buildings** (the intended targets — anti-base utility)
  - **Medium vehicles** (an unintended-but-design-acceptable side effect — armor class 4 includes some armored vehicles)
  - Inline note: `;GEF Needs to be able to hit the deployed Slave Miner as well` (visible in adjacent INI) — the Slave Miner is medium-armor when deployed, hence the 100 % vs armor class 4 entry.
- `Bullets=yes` — counts as bullet damage (shared with AntiB, NukeB — utility-warhead category).
- No CellSpread (single-target).

### 3.5 `[MagneticBeamE]` — elite primary swap

```ini
[MagneticBeamE]
Damage=10000
ROF=20
Range=12
MinimumRange=3
Speed=100
Projectile=InvisibleHigh
Warhead=LocomotorBeam
Report=MagnetronAttack
IsMagBeam=yes
```

**Functionally identical to `[MagneticBeam]` except `Damage=10000` (vs 5000)**. Same range, ROF, warhead, locomotor effect. Since the warhead's actual mechanism is the locomotor swap (not damage application), the damage doubling has no in-game effect on the lift target. The damage value matters only insofar as it determines whether the warhead fires at all (must be > 0 for some gating checks).

> **Elite swap is essentially cosmetic** — the lift effect duration, range, and target gate are all unchanged. Like DISK's DiskLaserE pattern, the elite weapon block exists for engine compatibility but offers no mechanical upgrade beyond the standard FIREPOWER/ROF ability scalars.

### 3.6 Projectile

`Projectile=InvisibleHigh` — standard bookkeeping invisible projectile (no visual, no real motion). The visible "magnetic beam" is rendered by the IsMagBeam=yes flag's hardcoded routing, not by the projectile.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `MagnetronSelect` | sound:4685 | unique select voice |
| `VoiceMove` | `MagnetronMove` | sound:4690 | unique move voice |
| `VoiceAttack` | `MagnetronAttackCommand` | sound:4695 | unique primary-attack voice |
| `VoiceSecondaryWeaponAttack` | `MagnetronMagneShakeVoice` | sound:4700 | unique secondary-attack voice (anti-building) |
| `VoiceFeedback` | (not set — defaults handled by engine) | — | — |
| `DieSound` | `GenVehicleDie` | sound:1961 | generic vehicle death |
| `MoveSound` | `MagnetronMoveStart` | sound:5238 | one-shot engine start |
| `CrushSound` | `TankCrush` | sound:5472 | generic tank crush (irrelevant — Crusher=no) |
| `MagneticBeam Report` | `MagnetronAttack` | sound:1670 | primary attack sound |
| `MagneShake Report` | `MagnetronMagneShake` | sound:5704 | secondary attack sound |

**Five Magnetron-unique sound entries** (Select/Move/AttackCommand/MagneShakeVoice/Attack + MagneShake). The Magnetron has nearly as much custom audio as the Floating Disc, reinforcing its distinctive role.

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `YuriCountry` only.
- **Prerequisite:** `YAWEAP,NAPSIS` — Yuri War Factory + Psychic Sensor (NAPSIS is the Soviet-origin Psychic Sensor structure, made cross-faction in YR for Yuri).
- **TechLevel:** 2 (low; gating via Prerequisite).
- **Cost:** 1000 — relatively cheap for a tier-3 unit (compare DISK 1750, MIND ~1500).
- `AllowedToStartInMultiplayer=no` → not pre-built.
- `CrateGoodie=no` → not from crates.

---

## 6. Veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base — MagneticBeam (5000 dmg → no real damage, lifts vehicles), HP=150, Sight=10, Speed=5, ROT=5. |
| Veteran | `STRONGER,SIGHT,FASTER` — +HP, +sight, +speed. **No FIREPOWER/ROF.** |
| Elite | `SELF_HEAL,STRONGER,ROF` + `ElitePrimary=MagneticBeamE` swap — passive auto-heal, +HP, **+ROF (faster lift cooldown)**, swap to MagneticBeamE (10000 dmg — same effect). |

> Notable: VeteranAbilities omits FIREPOWER because the LocomotorBeam doesn't deal damage in the normal sense. Elite gets ROF which DOES matter (faster successive lifts).

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan

- `search_strings "Magnetron"` not run (would mostly catch the INI key strings themselves).
- `search_strings "IsMagBeam"` → 1 match @ 0x0084928c → WeaponTypeClass__ReadINI @ 0x007728f0. **NEW field-scope verification:** IsMagBeam @ WeaponType 0x007728f0.
- `search_strings "IsLocomotor"` → 1 match @ 0x00847d3c → WarheadTypeClass__ReadINI @ 0x0075d86b. **NEW field-scope verification:** IsLocomotor @ WarheadType 0x0075d86b.
- `search_strings "MagnaBeamColor"` → 1 match @ 0x0083a1e0 → RulesClass__ReadAudioVisual @ 0x0066b7de. Confirmed Rules-global, read once at INI load. Used by the IsMagBeam visual renderer.
- `search_strings "Bunkerable"` → 1 match @ 0x0084371c → TechnoTypeClass__ReadINI @ 0x0071500a. **NEW field-scope verification:** Bunkerable @ TechnoType 0x0071500a.

### 7.2 Verified field scopes (new this doc)

| Field | Scope | Address |
|-------|-------|---------|
| `IsMagBeam=yes` (Weapon) | WeaponType | **0x007728f0** (NEW) |
| `IsLocomotor=yes` (Warhead) | WarheadType | **0x0075d86b** (NEW) |
| `MagnaBeamColor=R,G,B` (Rules) | Rules global | **0x0066b7de** (NEW) |
| `Bunkerable=no` | TechnoType | **0x0071500a** (NEW) |
| `IsTilter=yes` | UnitType | 0x00747712 (verified in DISK doc) |
| `Locomotor=` (Warhead GUID) | WarheadType +0x15C..+0x16B (16-byte) | per MAGNETRON_SYSTEM doc §2.1 |
| `IsLocomotor=yes` byte offset | WarheadType +0x15B | per MAGNETRON_SYSTEM doc §2.1 |
| `VoiceSecondaryWeaponAttack` | TechnoType | 0x00844038 (cheat sheet) |
| `OpportunityFire=yes` | TechnoType | 0x0071483d |
| `CanPassiveAquire=no` | TechnoType | 0x00714473 |
| `ElitePrimary=MagneticBeamE` | TechnoType | 0x00712a32 |
| `TooBigToFitUnderBridge=true` | UnitType | 0x0074774e |
| `CrateGoodie=no` | UnitType | 0x00747658 |
| `DamageSmokeOffset` | TechnoType | 0x00713e25 |

### 7.3 LocomotorBeam dispatch chain (cross-ref summary)

From [`MAGNETRON_SYSTEM_GHIDRA_REPORT.md`](../../MAGNETRON_SYSTEM_GHIDRA_REPORT.md):

1. **WeaponType::ReadINI @ 0x00772080** reads `IsMagBeam` into `WeaponType+0x15C` (bool).
2. **WarheadType::ReadINI @ 0x0075D590** reads `IsLocomotor` into `WarheadType+0x15B` (bool) and `Locomotor=` GUID into `WarheadType+0x15C..+0x16B` (16-byte CLSID via `CCINIClass::ReadCLSID`).
3. **WarheadType::Detonate @ 0x004690B0** is called on bullet impact:
   - If `WarheadType+0x15B (IsLocomotor)` is set → enter locomotor-swap branch.
   - Read `WarheadType+0x15C..+0x16B (Locomotor GUID)` → resolve to LocomotorClass via COM/factory lookup.
   - Apply via `Apply_area_damage` @ `0x00489280` with the per-cell or per-target piggyback effect.
4. **TechnoClass::PerformDeploy @ 0x00710000** is the receiver-side function — the target's locomotor is swapped here (or via a related callback).
5. **Verses gate** applied at the area-damage step: targets with Verses=0% are skipped (gate that restricts to vehicle armor classes).
6. **Release timing** (MEDIUM-confidence per MAGNETRON_SYSTEM doc) — a timer on the target TechnoClass eventually restores the original locomotor; the target drops to ground and takes falling damage per `Rules.FallingDamageMultiplier` (CombatDamage block, Rules+0xf64).

### 7.4 Magnetron vs Chronosphere — same engine, different CLSID

Both use the `IsLocomotor=yes` mechanism, but swap to different locomotors:

| | Magnetron LocomotorBeam | Chronosphere ChronoWarp |
|---|---|---|
| `IsLocomotor=yes` | yes | yes |
| `Locomotor=` GUID | `{92612C46-...}` (Jumpjet) | `{4A582747-...}` (Teleport) |
| Effect on target | Lifts target off ground (Jumpjet flight) | Teleports target to destination |
| Verses gate | Vehicles + Terror Drones only | Different gating (often all-ground or even infantry) |
| Duration | ~5-7 sec, then drops | Instantaneous (teleport completes) |
| Source | Unit weapon (Magnetron) | Superweapon (Chronosphere structure) |

> **One generic mechanism (`IsLocomotor=yes`) supports both abilities — a clean engine design.** Cross-reference CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT for the Chronosphere side.

### 7.5 Why CanPassiveAquire=no

The Magnetron's inline comment is unique: "Won't try to pick up own targets ;GEF don't want him accidently attacking your own guys". This is *not* the same reason as V3/Dread/Carrier (which set `CanPassiveAquire=no` because they're long-range siege and shouldn't fire on random nearby threats).

For the Magnetron, the concern is: lifting friendly units could happen if passive-acquire accidentally targets a friendly vehicle (e.g., if the Magnetron is mind-controlled by an enemy Yuri). More importantly, **the auto-engage could pick up the *wrong* enemy tank in a complex battle**, lifting one tank when the player wanted another disabled. By forcing manual targeting, the Magnetron preserves the player's tactical choice.

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| Locomotor `{4A582741-...}` (own) = DriveLocomotionClass | Live in YR. |
| Locomotor `{92612C46-...}` on warhead = JumpjetLocomotionClass | Live in YR. |
| `IsMagBeam=yes` on weapon | Live YR (Magnetron-exclusive routing). |
| `IsLocomotor=yes` on warhead | Live YR (Magnetron + Chronosphere). |
| `Bunkerable=no` | Live YR. |
| `;Image=V3`, `;TargetLaser=yes`, `;yes` Crusher | INI comments — drafts. |
| `;IsRadBeam=yes`, `;IsLaser=true`, `;Bright=yes`, `;LaserInnerColor=...` etc. on weapons | INI comments — disabled in favor of IsMagBeam routing. |
| `;Verses=...` alternate on LocomotorBeam | INI comment — old curve. |
| `;InfDeath=5` on LocomotorBeam | INI comment — irrelevant (infantry immune). |
| `;Spread=0` on LocomotorBeam | INI comment. |
| Fog-of-war 0x1000 gate | Not on TELE. |
| ImmuneToVeins / Subterranean / Tunneling | Not on TELE. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| Index corrections (UTNK SKIP-DUPLICATE; TELE = Magnetron) | ✅ at top |
| rulesmd `[TELE]` — every key | ✅ §1 (48 keys including 5 commented drafts) |
| artmd `[TELE]` — every key | ✅ §2 (6 keys; FLH multi-value `;`-separated explained) |
| `[MagneticBeam]` weapon | ✅ §3.1 |
| `[MagneShake]` weapon | ✅ §3.2 (with 6 commented Laser*/Bright keys) |
| `[LocomotorBeam]` warhead | ✅ §3.3 + full mechanism in §7.3 |
| `[MagneShakeWH]` warhead | ✅ §3.4 |
| `[MagneticBeamE]` (elite swap) | ✅ §3.5 |
| Projectile | ✅ §3.6 |
| Voices / sounds (10 bindings) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy (including no-FIREPOWER explanation) | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (**4 NEW field-scope verifications added to cheat sheet**: IsMagBeam @ 0x007728f0 WeaponType, IsLocomotor @ 0x0075d86b WarheadType, MagnaBeamColor @ 0x0066b7de Rules, Bunkerable @ 0x0071500a TechnoType; full dispatch chain cross-ref to MAGNETRON_SYSTEM doc; Magnetron-vs-Chronosphere comparison) |
| TS-legacy filter | ✅ §8 |
| Cross-references (MAGNETRON_SYSTEM, JUMPJET_LOCOMOTION, CHRONOSPHERE_SUPERWEAPON, DISK, MIND) | ✅ at top + inline |
| Index corrections logged in doc and in INDEX_UNITS.md | ✅ |

---

## 10. Quick implementer summary

To make a Magnetron-equivalent:

1. **Render** — voxel + HVA with `TurretOffset=-100`; FLH at 85,0,130 (top of upraised coil).
2. **Movement** — DriveLocomotionClass (ground, MovementZone=Destroyer); TooBigToFitUnderBridge gate; not Bunkerable; CanPassiveAquire=no (manual targeting only).
3. **Primary attack (MagneticBeam → LocomotorBeam → Jumpjet swap)** —
   - Weapon has `IsMagBeam=yes` flag at WeaponType+0x15C (visual: render beam in `Rules.MagnaBeamColor`).
   - Warhead has `IsLocomotor=yes` flag at WarheadType+0x15B AND `Locomotor=` GUID at WarheadType+0x15C..+0x16B (16-byte CLSID).
   - At detonate: read GUID, resolve to LocomotorClass via factory, piggyback onto target TechnoClass.
   - Target's normal locomotor is saved; new locomotor (Jumpjet here) takes over for ~5-7 sec.
   - Target lifts off ground, becomes immobile/non-firing.
   - At end of duration: restore original locomotor, target drops, take falling damage per Rules.FallingDamageMultiplier.
   - Verses curve on LocomotorBeam gates to ground vehicles + terror drones only (armor classes 3/4/5/9).
4. **Secondary attack (MagneShake → MagneShakeWH)** —
   - 80 base damage, 10-cell range.
   - Verses curve gates to medium vehicles + structures only.
   - Standard direct-fire WeaponType behavior.
   - Uses same IsMagBeam=yes visual rendering.
5. **Audio** — Magnetron-unique voice set (Select/Move/AttackCommand/MagneShakeVoice); separate VoiceSecondaryWeaponAttack for the building-shake mode.
6. **Veterancy** — Vet: STRONGER/SIGHT/FASTER (no FIREPOWER/ROF since damage is irrelevant). Elite: adds SELF_HEAL + ROF + ElitePrimary swap (cosmetic).
7. **AI flags** — High ThreatPosed=40 (AI prioritizes killing Magnetrons); CanPassiveAquire=no forces deliberate use.
8. **Build gate** — YAWEAP+NAPSIS prerequisites; YuriCountry only.

The Magnetron requires the same `IsLocomotor=yes` infrastructure as the Chronosphere — a single shared engine path that reads a GUID, resolves it to a Locomotor factory, and piggybacks the new locomotor onto target vehicles. This is one of the cleanest piece of generic-engine design in YR: two completely different player-facing abilities (Chronoport vehicles vs Magnetic-lift vehicles) share the same code path, differentiated only by which CLSID the warhead specifies.
