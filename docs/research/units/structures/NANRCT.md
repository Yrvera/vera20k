# [NANRCT] — Soviet Nuclear Reactor

**INI ID:** `NANRCT`
**Display name:** `UIName=Name:NANRCT` → CSF "Soviet Nuclear Reactor"
**Internal name:** `Name=Soviet Nuclear Reactor`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** all 10 factions
**Doc filename:** `units/structures/NANRCT.md`
**Loop iteration:** 95

**Role:** Soviet heavy power plant. Tier-9 (very late game). Provides `Power=2000` — **10× NAPOWR's 150 power per plant**. Smallest power-per-cost ratio in the game (2 power per credit vs NAPOWR's 0.25). Distinctive: `Explodes=yes` + `DeathWeapon=NukePayload` — destruction triggers a small nuclear blast (anti-base-cluster discouragement). Uses `LowPower`/`LowPowerDamaged` animation states (first documented building using low-power anim variants).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:12737-12767](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[NANRCT]
UIName=Name:NANRCT
Name=Soviet Nuclear Reactor
BuildCat=Power
Strength=1000
Armor=concrete
TechLevel=9
Prerequisite=NATECH,NACNST
Adjacent=2
Sight=5
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=1 ;gs 0 for Good, 1 for Evil
Cost=1000
Points=30
Power=2000
Crewed=yes
Capturable=true
DamageSmokeOffset=410, 100, 165
MaxDebris=15
MinDebris=5
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
DamageParticleSystems=SmallGreySSys,BigGreySmokeSys
Powered=no
IsImmuneToRadiation=yes
Explodes=yes
DeathWeapon=NukePayload ; NUKE
DeathWeaponDamageModifier=0.5
Spyable=yes
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
Drainable=yes
PoweredSpecial=yes
```

### Diffs vs NAPOWR (the relevant 10)

NANRCT is the Soviet heavy power-plant alternative. Comparing field-by-field to NAPOWR:

| Field | NAPOWR | NANRCT | Notes |
|-------|--------|--------|-------|
| **TechLevel** | 1 | **9** | very late tech tier |
| **Prerequisite** | NACNST | **NATECH,NACNST** | needs Battle Lab |
| **Cost** | 600 | **1000** | 67% more expensive |
| **Power** | 150 | **2000** | **13.3× output** |
| **Power/Cost ratio** | 0.25 | **2.0** | 8× better efficiency at this tier |
| **Strength** | 750 | **1000** | tougher (same as ConYard) |
| **Armor** | wood | **concrete** | hardest armor class |
| **Sight** | 4 | **5** | slightly better vision |
| **Points** | 40 | **30** | actually LESS score-on-kill (Soviet design balance — encourages destroying it for the nuclear blast bonus, not the score) |
| **MaxDebris / MinDebris** | 15 / 5 | 15 / 5 | identical |
| **DebrisAnims=** | same set | same set | identical |
| **DamageParticleSystems=** | `SparkSys,SmallGreySSys,BigGreySmokeSys` (3 systems) | **`SmallGreySSys,BigGreySmokeSys` (2 systems — no SparkSys)** | NANRCT's damage doesn't spark; instead nuclear-themed smoke. |
| **Powered=** | (absent — implicit yes) | **`Powered=no`** | **Critical**: NANRCT does NOT need power to operate. It IS the power source. |
| **IsImmuneToRadiation** | (absent) | **`IsImmuneToRadiation=yes`** (but **Westwood typo — see Ghidra findings**) |
| **Explodes** | (absent) | **`Explodes=yes`** | NUCLEAR DEATH MECHANIC |
| **DeathWeapon** | (absent) | **`DeathWeapon=NukePayload`** | small nuclear blast on destruction (Damage=600, Range=30, NUKE warhead, RadLevel=500 leaves radiation field) |
| **DeathWeaponDamageModifier** | (absent) | **`0.5`** | nukes hit at 50% damage (300 effective dmg, not 600) |
| **Explosion=** | 6-anim with tstlexp | **(absent — uses engine default)** | NANRCT doesn't list explosion anims; relies on DeathWeapon's NUKE warhead detonation visuals |

**Identical to NAPOWR:** BuildCat=Power, Adjacent=2, Owner= (all 10), AIBasePlanningSide=1, Crewed=yes, Capturable=true, Spyable=yes, ImmuneToPsionics=no, Drainable=yes, PoweredSpecial=yes, TogglePower= absent, ThreatPosed= absent (defaults to 0).

### Soviet design: "Build NANRCT or spam NAPOWR"

The economics:
- **5× NAPOWR = 750 power for 3000 credits** (5×600=3000, 5×150=750)
- **1× NANRCT = 2000 power for 1000 credits** at TechLevel=9

So NANRCT is **6.7× more power-efficient per credit** than NAPOWR — but it requires Battle Lab and explodes catastrophically when destroyed. Soviet faction strategy:
- Early/mid game: 4-6 NAPOWRs for power.
- Late game (after NATECH built): 1-2 NANRCTs replace all of them.
- Risk: NANRCT's nuclear death is a single-point-of-failure economic catastrophe.

### Identity & UI

- **`UIName=Name:NANRCT`** → "Soviet Nuclear Reactor".
- **`Name=Soviet Nuclear Reactor`** — fallback.
- **`BuildCat=Power`** — Power tab.

### Power-system flags

- **`Power=2000`** — the highest per-building power in the game.
- **`Powered=no`** — **the building does NOT consume power**. (Note: this is the inverse of typical buildings. A power plant being `Powered=no` means it doesn't shut off when low-power because it IS the power.) The default for buildings is `Powered=yes` (they consume power and degrade when low-power). Confirmed Ghidra-scope: **`Powered`** is BuildingType-scope (search returns 65 power-related strings; the core `Powered` parser key is in TechnoType range but applies to buildings).
- **`PoweredSpecial=yes`** — power-special-state animation supported.
- **`Drainable=yes`** — Yuri drain-target.

### Tech / build gating

- **`TechLevel=9`** — late game. Visible only after extensive tech tree progression.
- **`Prerequisite=NATECH,NACNST`** — needs Soviet Battle Lab + ConYard.
- **`Cost=1000`** — substantial but pays back massively in power.

### Combat / capture / spy

- **`Strength=1000` / `Armor=concrete`** — toughest power plant.
- **`Capturable=true`** — Engineer-capturable. **Capturing transfers 2000 power** — game-changing.
- **`Spyable=yes`** — spy infiltrate. Per `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`, the infiltrate effect is the same as other power plants (temporary base-wide power outage), but with NANRCT's massive power contribution, the outage is far more devastating.
- **`Crewed=yes`** — destruction ejects E2.
- **`IsImmuneToRadiation=yes`** ← **WESTWOOD TYPO / DEAD-FIELD**:
  - The engine reads the field **`ImmuneToRadiation`** (without the "Is" prefix) — confirmed Ghidra-scope: TechnoType (xref `0x00843854 → 0x00714d53`). **NEW cheat-sheet entry.**
  - The string `IsImmuneToRadiation` returns **0 matches** in the binary. The "Is" prefix variant is NOT a parser key.
  - Therefore: **NANRCT's `IsImmuneToRadiation=yes` line is silently ignored by the engine**. The reactor is NOT actually immune to radiation if a Soviet Desolator deploys nearby (or another nuke detonates near it).
  - This is a Westwood disparity: visible INI intent is for Nuclear Reactor to be radiation-immune, but the engine does NOT honor it. **The correct INI key would be `ImmuneToRadiation=yes`** without the "Is" prefix.
  - Implication for parity: when reimplementing, the field `IsImmuneToRadiation` should be **ignored**, matching gamemd.exe behavior. Do NOT make NANRCT immune to radiation based on the typo.

### Death — the nuclear blast mechanic

- **`Explodes=yes`** — when destroyed, the building triggers its `DeathWeapon` instead of a normal Explosion= anim. Confirmed Ghidra-scope: **dual TechnoType + OverlayType** (xrefs `0x007122c5 + 0x005fe840`) — `Explodes` works on both units AND overlay tiles. NEW cheat-sheet entry.
- **`DeathWeapon=NukePayload`** — the weapon fired on death. References `[NukePayload]` at [rulesmd.ini:24017](../../../../ra2-rust-game/ini/rulesmd.ini):
  ```ini
  [NukePayload]
  Damage=600
  Range=30
  Projectile=GiantNukeDown  ;MultiMissile
  Speed=10
  RadLevel=500
  Warhead=NUKE
  Report=
  ```
  - Damage=600 (modified by 0.5 = effective 300 dmg per cell within Range=30).
  - Range=30 — huge AoE radius (much larger than any combat weapon).
  - Projectile=GiantNukeDown — the falling-nuke missile visual (cross-ref to NUKE_SUPERWEAPON for projectile details).
  - **RadLevel=500** — leaves a radiation field at the impact site. Persistent ground damage area.
  - Warhead=NUKE — the same warhead the Nuclear Missile Silo (NAMISL) superweapon uses.
- **`DeathWeaponDamageModifier=0.5`** — halves the damage. Mitigates the catastrophic effect; without it, a destroyed NANRCT would deal 600 damage at range 30, instantly killing every nearby building. At 0.5, it's "still really bad" but survivable for hardier structures.

The strategic implication: **clustering buildings near NANRCT is a major risk**. A late-game push that destroys an enemy NANRCT can deal massive chain damage to their adjacent base.

### Visual FX
- **`DamageSmokeOffset=410, 100, 165`** — same as refinery (Westwood reused offsets).
- **`MaxDebris=15` / `MinDebris=5`** — same as NAPOWR.
- **`DebrisAnims=`** — same 6-anim list.
- **`DamageParticleSystems=SmallGreySSys,BigGreySmokeSys`** — 2 systems (no SparkSys). Nuclear reactor's damage doesn't visually spark (no electrical arcs like Tesla Reactor); instead just smoke.
- **No `Explosion=`** — the standard 5-anim Explosion is not listed. NANRCT relies entirely on the NukePayload DeathWeapon for destruction visuals.

### AI hints
- **`AIBuildThis=`** NOT set — same convention as other power plants (uses Rules-global BuildPower table).
- **`AIBasePlanningSide=1`** — Evil/Soviet.

---

## artmd.ini section — full transcript

[artmd.ini:3291-3317](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[NANRCT]
Normalized=yes
Remapable=yes
Cameo=NRCTICON
Foundation=4x4
Height=4
Buildup=NANRCTMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
ActiveAnim=NANRCT_A
ActiveAnimDamaged=NANRCT_AD
ActiveAnimZAdjust=-10
CanBeHidden=False
OccupyHeight=3
RemoveOccupy1=0,-1
RemoveOccupy2=-1,1
RemoveOccupy3=-1,2
RemoveOccupy4=0,2
RemoveOccupy5=0,3
DamageFireOffset0=72,15
DamageFireOffset1=-44,71
ActiveAnimPoweredSpecial=true
ActiveAnimPowered=false
LowPower=NANRCT_P
LowPowerDamaged=NANRCT_PD
LowPowerPowered=false
```

### Diffs vs NAPOWR artmd

| Field | NAPOWR artmd | NANRCT artmd | Notes |
|-------|--------------|--------------|-------|
| Cameo | NPWRICON | **NRCTICON** | per-side cameo |
| Foundation | 3x2 | **4x4** | 2.67× larger footprint (16 cells vs 6) |
| Height | 3 | 4 (taller) | |
| ActiveAnimZAdjust | -5 | **-10** | bigger building, more Z-offset |
| RemoveOccupy slots | (none) | **5 slots** | non-rectangular cooling tower extension |
| DamageFireOffset0 | -12,30 | **72,15** | per-side art |
| DamageFireOffset1 | (absent) | **-44,71** | second fire point (taller building, two visible damage zones) |
| **LowPower anim** | (absent) | **`LowPower=NANRCT_P` + `LowPowerDamaged=NANRCT_PD` + `LowPowerPowered=false`** | **first building to use low-power animation variants** |

### LowPower animation system — first documented usage

```ini
LowPower=NANRCT_P
LowPowerDamaged=NANRCT_PD
LowPowerPowered=false
```

NANRCT is the **first building documented using the low-power animation system**:

- **`LowPower=NANRCT_P`** — animation to play when the player's power is in the LOW state (net power negative but not critically so). References `[NANRCT_P]` at [artmd.ini:4276](../../../../ra2-rust-game/ini/artmd.ini): 1-frame loop, DetailLevel=2, DoubleThick=true, Image=NANRCT_A (frames 0-1). Shows a static frame of the reactor in idle state.
- **`LowPowerDamaged=NANRCT_PD`** — damaged variant of the low-power anim. References `[NANRCT_PD]` at [artmd.ini:4290](../../../../ra2-rust-game/ini/artmd.ini): same SHP, frames 11-12, 1-frame infinite loop.
- **`LowPowerPowered=false`** — disables the LowPower anim during normal-powered state (verbose: only play LowPower anim when actually in low-power state).

Confirmed Ghidra-scope:
- **`LowPower`** is part of the 65-string Powered family (search `Powered` → 65 matches).
- **`LowPowerDamaged`** → `0x00819a28 → 0x0046384a` BuildingType. **NEW cheat-sheet entry.**
- **`SuperLowPowerDamaged`** → `0x0081995c` (also exists but unused by NANRCT).

The visual semantic: NANRCT's reactor core appears to dim or glow differently when the base loses power. This is significant feedback because the nuclear reactor's massive output makes its low-power state visible to both player and opponent.

### Foundation & RemoveOccupy

- **`Foundation=4x4`** — same as ConYard. NANRCT is one of the larger buildings.
- **`OccupyHeight=3`** — moderate Z-occupancy.
- **`RemoveOccupy1=0,-1, RemoveOccupy2=-1,1, RemoveOccupy3=-1,2, RemoveOccupy4=0,2, RemoveOccupy5=0,3`** — 5 cells of non-foundation passability. The Nuclear Reactor's cooling tower extends north (negative Y) and south (positive Y) beyond the 4x4 base; these RemoveOccupy entries mark visual-extension cells as passable so units can walk under/past the cooling tower.

### Sub-animations (4 blocks)

```ini
[NANRCT_A]                ; Active (10-frame infinite loop, DoubleThick=true)
LoopEnd=10
Rate=220
DoubleThick=true

[NANRCT_AD]               ; Damaged Active (frames 11-21, infinite, DoubleThick=true)
Start=11
LoopEnd=21
Rate=220
DoubleThick=true

[NANRCT_P]                ; LowPower (1-frame infinite, DoubleThick=true)
LoopEnd=1
DoubleThick=true

[NANRCT_PD]               ; LowPower Damaged (frames 11-12, infinite, DoubleThick=true)
Start=11
LoopEnd=12
DoubleThick=true
```

All four anims use `DoubleThick=true` and `DetailLevel=2`. The Nuclear Reactor's core visuals (radioactive glow lines, electrical arcing) need the double-thick rendering for emphasis at low detail levels.

### Buildup
- **`Buildup=NANRCTMK`** — Soviet Nuclear Reactor buildup.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                  | String address    | First xref                              | Read scope                       |
|------------------------|--------------------|-----------------------------------------|----------------------------------|
| `Explodes`             | `0x0083355c`       | `0x007122c5` + `0x005fe840`             | **TechnoType + OverlayType (dual)** |
| `ImmuneToRadiation`    | `0x00843854`       | `0x00714d53`                            | TechnoType                       |
| `LowPowerDamaged`      | `0x00819a28`       | `0x0046384a`                            | BuildingType                     |
| `IsImmuneToRadiation`  | **(not in binary)** | **NO XREFS — dead text**                | **WESTWOOD TYPO**                |

**3 NEW cheat-sheet entries + 1 dead-field discovery:**

1. **`Explodes`** — dual scope: TechnoType (`0x007122c5`) + OverlayType (`0x005fe840`). For units/buildings: triggers DeathWeapon. For overlays: triggers explosion on overlay damage (e.g., tiberium pile explodes when shot, oil derrick, etc.). NEW cheat-sheet entry.
2. **`ImmuneToRadiation`** — `0x00843854 → 0x00714d53` — TechnoType. The CORRECT field name for radiation immunity. **Cross-ref**: ROBO Robot Tank uses `ImmuneToRadiation=yes` (per ROBO doc cheat-sheet listing); NANRCT uses the typo'd `IsImmuneToRadiation=yes` (silently ignored). NEW cheat-sheet entry.
3. **`LowPowerDamaged`** — `0x00819a28 → 0x0046384a` — BuildingType. Low-power-state damaged animation reference. Part of the 65-string Powered family. NEW cheat-sheet entry.

**Dead-field discovery:**

- **`IsImmuneToRadiation`** — the field name used in NANRCT's INI block returns **0 matches** in the binary's string table. The engine does NOT read this as a parser key. **The line `IsImmuneToRadiation=yes` in NANRCT's INI is dead text — Westwood typo.**
- **Disparity for parity bar**: The Soviet Nuclear Reactor is NOT actually immune to radiation in gamemd.exe (despite the INI line). Reimplementations must ignore `IsImmuneToRadiation` to match gamemd behavior. The correct INI key would be `ImmuneToRadiation=yes`.

### Ghidra search log for this iteration

- `search_strings("Explodes")` → 2 matches: `Explodes` (parser key) + `EXPLODES` (uppercase variant at `0x00846448` — likely debug string or comment). Dual-scope (TechnoType + OverlayType).
- `search_strings("IsImmuneToRadiation")` → 0 matches. **Dead field.**
- `search_strings("ImmuneToRadiation")` → 1 match → TechnoType.
- `search_strings("LowPowerDamaged")` → 1 match → BuildingType. Sibling: `SuperLowPowerDamaged` at `0x0081995c`.
- `search_strings("DeathWeapon")` → 2 matches: `DeathWeapon` + `DeathWeaponDamageModifier`. Existing cheat-sheet entries.
- `search_strings("Powered")` → 65 matches. Including EVA voice `EVA_EnemyBasePoweredDown` (low-power EVA notification — engine has voice lines for power events).

### Nuclear blast cross-reference

The `NukePayload` weapon → `NUKE` warhead chain → cross-ref to `NUKE_SUPERWEAPON_GHIDRA_REPORT.md`. The NUKE warhead is the same one used by NAMISL (Nuclear Missile Silo superweapon). NANRCT's death weapon uses it at 50% damage (DeathWeaponDamageModifier=0.5) but full RadLevel=500. So a destroyed reactor produces:
- Damage 300 (cell-center) falling off with CellSpread
- RadLevel=500 radiation field at impact site (persistent ground damage area)
- Visual: GiantNukeDown projectile + NUKE warhead's AnimList (likely NukeAnim or similar mushroom cloud)

This is a "mini-nuke" — significantly less than the superweapon's full nuke, but still devastating for cluster-based base layouts.

### Cross-faction comparison: only Soviet has explosive power plants

| Side | Basic Power | Heavy Power | Explosive on Death? |
|------|-------------|-------------|---------------------|
| Allied | GAPOWR (Power=200, Upgrades=2) | (no heavy alternative — uses upgrades) | No |
| Soviet | NAPOWR (Power=150) | **NANRCT (Power=2000, Explodes=yes)** | **YES — nuclear blast** |
| Yuri | YAPOWR (Power=150, garrison-boost) | (no heavy alternative — uses garrison) | No |

Soviet is the only faction whose late-game heavy power plant explodes catastrophically. Allied chose upgrade-density (incremental safety); Yuri chose garrison-boost (active management); Soviet chose raw output with explosion risk.

### TS-legacy filter

- **`Powered=no`** — active YR. The "power source doesn't consume power" pattern.
- **`Explodes=yes` + `DeathWeapon=NukePayload`** — active YR.
- **`DeathWeaponDamageModifier=0.5`** — active YR (existing cheat-sheet).
- **`LowPower=` / `LowPowerDamaged=` / `LowPowerPowered=false`** — active YR (engine supports rich power-state animations).
- **`DoubleThick=true`** in all 4 sub-anims — active YR.
- **`IsImmuneToRadiation=yes`** — **Westwood typo, parsed but ignored**. NOT a TS-legacy issue; it's a developer mistake.
- **No fog-of-war / 0x1000 gating** — clean.
- **No Tunnel/Subterranean** — clean.

NANRCT has one notable Westwood disparity (IsImmuneToRadiation typo) but is otherwise clean of TS-legacy issues.

---

## Cross-references

- **`NAPOWR`** (`units/structures/NAPOWR.md`) — DONE iteration 94. Basic Soviet power plant sibling.
- **`GAPOWR`** (`units/structures/GAPOWR.md`) — DONE iteration 93. Allied power, no explosion mechanic.
- **`YAPOWR`** — pending iteration 96. Yuri Bio Reactor with garrison-boost.
- **`NATECH`** — pending. Soviet Battle Lab; prerequisite for NANRCT.
- **`NAMISL`** (Nuclear Missile Silo) — pending. Uses the same NUKE warhead, but with full damage and a much larger area (superweapon scale).
- **`NUKE_SUPERWEAPON_GHIDRA_REPORT.md`** — deep-RE doc for the NUKE warhead behavior, GiantNukeDown projectile, RadLevel mechanic. Cross-ref.
- **`SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`** — spy infiltrate causes power outage; with NANRCT's 2000 power, the outage is devastating.
- **`DESO` (Desolator)** — radiation source. With `IsImmuneToRadiation=yes` being a Westwood typo, NANRCT is NOT actually immune to Desolator radiation (per Ghidra binding).
- **`ROBO` (Robot Tank)** — uses the CORRECT `ImmuneToRadiation=yes` field. Confirmed working in cheat-sheet.

---

## Coverage audit

INI fields covered (28 rulesmd + 17 artmd + 4 sub-anims = 49 entries). **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`IsImmuneToRadiation=yes` is a Westwood typo.** The engine reads `ImmuneToRadiation` (no Is prefix). Confirmed via Ghidra: no string `IsImmuneToRadiation` in the binary. **NANRCT is NOT actually radiation-immune in gamemd.exe.** Significant parity finding.
2. **`Explodes=yes` works on OverlayType too** — discovered via dual-xref (TechnoType + OverlayType). Tiberium piles and other overlays can use `Explodes` to detonate when destroyed. Not relevant to NANRCT but documented for engine completeness.
3. **`DeathWeaponDamageModifier=0.5`** — Westwood deliberately reduced the death-explosion damage. Without this, the nuclear blast would be a superweapon-level event on building destruction. The 0.5 modifier makes it survivable but still dangerous.
4. **`Powered=no`** — explicit. Confirms the power-source-doesn't-consume-power pattern is opt-in via this flag. Other buildings default to `Powered=yes`.
5. **`Points=30`** is LESS than NAPOWR's 40. Westwood deliberately reduced score-on-kill for NANRCT, possibly because destroying it gives the additional "nuclear blast damage to surrounding buildings" reward (extra collateral). Score is intentionally low to balance the strategic value.
6. **65-string Powered family in binary** — engine has very rich power-state animation matrix. NANRCT uses 3 variants (ActiveAnimPoweredSpecial, ActiveAnimPowered, LowPower, LowPowerDamaged, LowPowerPowered). The other ~60 strings cover IdleAnim×PowerState, ActiveAnimTwo×PowerState, ProductionAnim×PowerState, etc. Most are latent (unused by shipping art) but engine-supported.
7. **`EVA_EnemyBasePoweredDown`** voice line — engine has dedicated EVA announcement for enemy power outage. Confirms the engine watches power state changes and triggers voice events.

---

## Status

**DONE** — iteration 95. Index entry will be updated.

Doc total: **95**.

Next pick (priority): YAPOWR (Yuri Bio Reactor) — closes power plant quartet (GAPOWR + NAPOWR + NANRCT + YAPOWR). YAPOWR has the garrison-boost mechanic: Initiates inside the Bio Reactor multiply its power output. This is a unit-specific hardcoded behavior that should be verified via Ghidra.
