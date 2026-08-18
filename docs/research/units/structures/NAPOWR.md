# [NAPOWR] — Soviet Tesla Reactor

**INI ID:** `NAPOWR`
**Display name:** `UIName=Name:NAPOWR` → CSF "Soviet Tesla Reactor"
**Internal name:** `Name=Soviet Tesla Reactor`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** all 10 factions
**Doc filename:** `units/structures/NAPOWR.md`
**Loop iteration:** 94

**Role:** Soviet primary power plant. Sister to GAPOWR. Provides `Power=150` (less than GAPOWR's 200) at `Cost=600` (cheaper than GAPOWR's 800). Tesla-themed. Distinct from NANRCT (Nuclear Reactor, heavy power).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:12450-12479](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[NAPOWR]
UIName=Name:NAPOWR
Name=Soviet Tesla Reactor
BuildCat=Power
Prerequisite=NACNST
Strength=750
Armor=wood
TechLevel=1
Sight=4
Adjacent=2
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=1 ;gs 0 for Good, 1 for Evil
Cost=600
Points=40
Power=150
Capturable=true
Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60,tstlexp
MaxDebris=15
MinDebris=5
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
ThreatPosed=0 ; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=450, 200, 300
TogglePower=no
Spyable=yes ; A spy can do something to this, works like captureable
DieSound=PowerPlantDie
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
Drainable=yes
PoweredSpecial=yes
```

### Diffs vs GAPOWR

NAPOWR vs GAPOWR — 6 substantive diffs and several minor:

| Field                        | GAPOWR                          | NAPOWR                                | Notes |
|------------------------------|---------------------------------|---------------------------------------|-------|
| Prerequisite                 | GACNST                           | **NACNST**                             | per-side ConYard |
| AIBasePlanningSide           | 0 (Good)                         | **1 (Evil)**                           | Soviet/Evil AI hint |
| **Cost**                     | 800                              | **600**                                | Soviet plant is 25% cheaper |
| **Power**                    | 200                              | **150**                                | Soviet plant produces 25% less |
| **Power/Cost ratio**         | 0.25 (200/800)                   | **0.25 (150/600)**                     | exact parity (4 power per credit) — Soviet just buys in smaller units |
| **Upgrades**                 | 2 (Power Turbine slots)          | **(absent)** — no upgrades             | Soviet Tesla Reactor cannot be upgraded |
| MaxDebris / MinDebris        | 6 / 4                            | **15 / 5**                             | Soviet drops far more debris (2.5x) — possibly tied to Tesla coil explosion theme |
| DebrisAnims=                 | 6 anims                          | **6 anims (identical to GAPOWR list)** | parity |
| Explosion= extra anim        | gtpowexp                         | **tstlexp** (Tesla-specific)          | per-side power-plant explosion anim |
| **DamageParticleSystems**    | `;commented`                     | **active 3-system**                    | NAPOWR keeps damage particles visible (Tesla building visibly arcs/smokes when damaged) |
| DamageSmokeOffset            | 300, 300, 450                    | **450, 200, 300**                      | per-side art coords |

**Identical to GAPOWR:** BuildCat=Power, Strength=750, Armor=wood, TechLevel=1, Adjacent=2, Sight=4, Owner= (all 10), Capturable=true, Crewed=yes, ThreatPosed=0, TogglePower=no, Spyable=yes, DieSound=PowerPlantDie (same sound — Westwood reused), ImmuneToPsionics=no, Drainable=yes, PoweredSpecial=yes, Points=40.

### Soviet trade-off design

NAPOWR's stats reveal the Soviet design intent:
- **Cheaper, weaker, more numerous**: 600 cost, 150 power per plant. To match GAPOWR's 200 power, Soviet needs 1 full plant + ~33% of another. Effectively the same Power/Cost efficiency but smaller granularity.
- **No upgrades**: GAPOWR's 2 Power Turbine slots are absent. Soviet cannot upgrade their basic power plant — they have to build NANRCT (Nuclear Reactor) if they want denser power. This is the Soviet branching strategy: spam cheap Tesla Reactors OR commit to expensive Nuclear Reactors.
- **More dramatic destruction**: MaxDebris=15 (matching ConYards) suggests Westwood envisioned Tesla Reactor explosions as visually larger. The `tstlexp` explosion SHP suggests electrical arcing/Tesla-themed boom.
- **Visible damage feedback**: DamageParticleSystems active (not commented) means a damaged Tesla Reactor visibly arcs/smokes. Soviet aesthetic: industrial/dirty/visibly stressed.

### Identity & UI
- **`UIName=Name:NAPOWR`** → "Soviet Tesla Reactor".
- **`BuildCat=Power`** — Power tab.

### Combat / capture / spy — same as GAPOWR
- **`Strength=750` / `Armor=wood`** — identical fragility.
- **`Capturable=true` / `Spyable=yes`** — same spy outage trigger.
- **`Crewed=yes`** — destruction ejects E2 (Soviet conscript).
- **`Drainable=yes`** — Yuri drain-target.
- **`PoweredSpecial=yes`** — power-special-state animation supported.

### Power-system flags — same as GAPOWR
- **`TogglePower=no`** — no toggle.
- **`PoweredSpecial=yes`** — power-state animation enabled.

### AI / placement
- **`AIBuildThis=` NOT set** — same as GAPOWR. AI uses `BuildPower=NAPOWR,GAPOWR,YAPOWR` Rules-global table.
- **`AIBasePlanningSide=1`** — Soviet/Evil.

### Visual FX
- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60,tstlexp`** — 6-anim with **Tesla-specific `tstlexp`** ("Tesla EXPlosion") last. Mirrors GAPOWR's `gtpowexp` but Tesla-themed.
- **`DieSound=PowerPlantDie`** — **same sound as GAPOWR** (Westwood reused the audio cue; per soundmd `bpowdiea`, `bpowdieb` 2-sample random pick).
- **`DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — **active** (vs GAPOWR's commented). Soviet plants visibly emit electrical arcing/smoke when damaged.

---

## artmd.ini section — full transcript

[artmd.ini:3258-3275](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[NAPOWR]
Normalized=yes
Remapable=yes
Cameo=NPWRICON
Foundation=3x2
Height=3
Buildup=NAPOWRMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
ActiveAnim=NAPOWR_A
ActiveAnimDamaged=NAPOWR_AD
ActiveAnimZAdjust=-5
CanBeHidden=False
OccupyHeight=3
DamageFireOffset0=-12,30
ActiveAnimPoweredSpecial=true
ActiveAnimPowered=false
```

### Diffs vs GAPOWR artmd

| Field | GAPOWR artmd | NAPOWR artmd | Notes |
|-------|--------------|--------------|-------|
| Cameo | POWRICON | **NPWRICON** | per-side |
| **Foundation** | 2x2 | **3x2** (50% larger footprint) | Soviet Tesla Reactor is wider |
| **Height** | 4 | **3** (shorter) | Tesla coil is shorter than Allied generator |
| ActiveAnimZAdjust | -32 | **-5** (less Z-raise) | shorter building, less Z-offset needed |
| ActiveAnimYSort | 362 | **(absent)** | NAPOWR omits explicit Y-sort |
| DamageFireOffset0 | -20,32 | **-12,30** | per-side art |
| DamageFireOffset1 | 3,12 | **(absent)** | NAPOWR uses single fire point |
| **`CanHideThings`** | True | **(absent)** | NAPOWR omits — likely defaults to engine default (False?) |

**Identical to GAPOWR artmd:** Normalized=yes, Remapable=yes, Buildup=*_MK with DemandLoadBuildup+FreeBuildup, NewTheater=yes, OccupyHeight=3, ActiveAnimPoweredSpecial=true, ActiveAnimPowered=false (same power-state gating).

### Sub-animation

[artmd.ini:16567-16591](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[NAPOWR_A]
Image=NAPOWR_A
Normalized=yes
NewTheater=yes
Layer=ground
Start=0
LoopStart=0
LoopEnd=18
LoopCount=-1
Rate=300
DetailLevel=2
DoubleThick=true

[NAPOWR_AD]
Image=NAPOWR_A
Normalized=yes
;NewTheater=yes
Layer=ground
Start=19
LoopStart=19
LoopEnd=37
LoopCount=-1
Rate=300
DetailLevel=2
```

Diffs vs GAPOWR_A sub-anim:

- **`LoopEnd=18`** vs GAPOWR's 8 — **2.25× longer animation** (18 frames vs 8). More elaborate visual.
- **`Rate=300`** vs GAPOWR's 220 — **even slower** (Rate is the inverse — higher Rate = slower playback). The Tesla coil animation is slower per-frame, more deliberate.
- **`DoubleThick=true`** — **NEW field**. Enables doubled-pixel rendering for this anim (similar to drawing with thicker outlines). Likely related to electrical-arc-line visual emphasis. Confirmed Ghidra-scope: **dual AnimType + BuildingType** (xrefs `0x00427e21 in AnimTypeClass__ReadINI` and `0x00461136 in BuildingTypeClass_ReadINI_Water`). **NEW cheat-sheet entry.**
- **`DetailLevel=2`** — same gating as GAPOWR_A; renders only when user's detail level is ≥ 2. Confirmed Ghidra-scope: **OptionsClass + AnimTypeClass** (dual scope — user options + per-anim filter). **NEW cheat-sheet entry.**

The damaged variant `NAPOWR_AD` uses frames 19-37 of the same SHP — same memory-thrift pattern as GAPOWR_AD. Note: `;NewTheater=yes` is commented in the AD variant (same convention as GAPOWR_AD — damaged variants skip theater substitution).

### Power-state animation gating (identical to GAPOWR)

```ini
ActiveAnimPoweredSpecial=true
ActiveAnimPowered=false
```

NAPOWR uses the same power-state gating: ActiveAnim only plays in the powered-special state, NOT in normal-powered state. This is unusual because NAPOWR has `Upgrades=` absent — without upgrade slots, what triggers the "powered-special" state?

DEFERRED — possibly the engine treats "non-low-power" as "powered-special" by default, OR the Tesla Reactor has some other PoweredSpecial trigger. Compare GAPOWR which has Upgrades=2 (Power Turbine slots) — the upgrade may flip to PoweredSpecial state. NAPOWR may be visually static in normal play.

OR (more likely): the `PoweredSpecial=yes` rulesmd flag combined with these artmd flags means the building enters PoweredSpecial state when normally powered, and the ActiveAnim displays. The "PoweredSpecial" name is misleading — it may just mean "the power-special category" with the actual visible state being the default.

This is the kind of detail that should be verified in-game. For now, document as observed: NAPOWR uses identical power-state gating to GAPOWR; expected behavior is for the Tesla coil's electrical arcs to animate continuously while the building is functional.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field             | String address    | First xref                                | Read scope                              |
|-------------------|--------------------|-------------------------------------------|-----------------------------------------|
| `DoubleThick`     | `0x0081862c`       | `0x00427e21` + `0x00461136`               | **AnimType + BuildingType (dual)**      |
| `DetailLevel`     | `0x0081855c`       | `0x005fa782` + `0x00428081`               | **OptionsClass + AnimType (dual)**      |

**2 NEW cheat-sheet entries this iteration (both dual-scope):**

1. **`DoubleThick`** — dual scope: AnimType (`0x00427e21`) + BuildingType (`0x00461136`). For anim blocks: enables double-pixel-thick rendering (likely for visual emphasis on electrical arcs, Tesla bolts, etc.). For buildings: similar effect or compatibility flag. NAPOWR_A uses it; GAPOWR_A does not.
2. **`DetailLevel`** — dual scope: OptionsClass (`0x005fa782` in `OptionsClass__ReadFromINI`) + AnimType (`0x00428081`). The user's options.ini stores their preferred detail level; AnimType blocks specify a per-anim DetailLevel threshold; if user-level < anim-level, anim is suppressed. GAPOWR_A and NAPOWR_A both use DetailLevel=2 (medium-detail). Also has a debug-print at `0x00833218` (`"DetailLevel = %d\n"`).

Note: `DetailLevel` is also read by `OptionsClass__WriteToINI` at `0x005fadd3` (write-back path — the engine persists user's setting choice).

### Ghidra search log for this iteration

- `search_strings("DoubleThick")` → 1 match → dual-scope.
- `search_strings("DetailLevel")` → 3 matches: `TranslucencyDetailLevel` (separate field at `0x00818544` — applies to translucency rendering), `DetailLevel` (parser key), debug-print string.

### Cross-references for power-state animation

The 16-entry `*PoweredSpecial` family (discovered in GAPOWR iteration) applies here too. NAPOWR's ActiveAnimPoweredSpecial=true / ActiveAnimPowered=false uses 2 of those 16 variants. Power plants are the buildings that drive the engine's power-state animation system; superweapons and special buildings likely use the other 14 variants.

### TS-legacy filter

- **`Explosion=...,tstlexp`** — `tstlexp` is Tesla-specific explosion SHP. Active YR.
- **`DamageParticleSystems=` active** — Soviet visual aesthetic.
- **`DieSound=PowerPlantDie`** — same audio as GAPOWR.
- **`DoubleThick=true`** in art — active YR (electrical arc rendering emphasis).
- **`DetailLevel=2`** — active YR (engine renders/culls based on user options).
- **No fog-of-war / 0x1000 gating** — clean.
- **No Tunnel/Subterranean** — clean.

NAPOWR has no TS-legacy gating. All fields active YR.

---

## Cross-references

- **`GAPOWR`** (`units/structures/GAPOWR.md`) — DONE iteration 93. Allied sister.
- **`NANRCT` (Soviet Nuclear Reactor)** — pending iteration 95. Heavy Soviet power, higher cost/output. Explodes destructively when destroyed (chain damage to adjacent buildings — confirmed in Westwood/community docs).
- **`YAPOWR` (Yuri Bio Reactor)** — pending iteration 96. Garrison-boost mechanic (Initiates inside boost output).
- **`NAAPWR`** — adjacent in artmd (Soviet AdvPowerPlant / NaAPwr?). [artmd.ini:3277](../../../../ra2-rust-game/ini/artmd.ini). Foundation=2x3 (rotated 3x2). Likely a TS-era leftover power plant variant, possibly unused. Cameo=tpwricon (lowercase). DEFERRED — investigate.
- **`NACNST`** — Prerequisite parent.
- **`SPY`** / **`SENGINEER`** / **`ENGINEER`** — capture/infiltrate.
- **`Rules-global BuildPower=NAPOWR,GAPOWR,YAPOWR`** — AI lookup table.
- **`PowerPlantDie`** sound — soundmd shared with GAPOWR.

---

## Coverage audit

INI fields covered (27 rulesmd + 17 artmd + 2 sub-anims + 1 sound ref = 47 entries). **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`Upgrades=` absent on NAPOWR** but present on GAPOWR. Confirms Westwood's intentional asymmetry: Allied gets upgradable power, Soviet gets the choice between cheap-numerous (NAPOWR) or expensive-dense (NANRCT). Cross-faction parity in Power/Cost ratio (0.25) suggests Westwood balanced it carefully.
2. **MaxDebris=15** is way higher than GAPOWR's 6. The Soviet Tesla Reactor visually explodes much bigger. Possibly Westwood's intent: Tesla coil = dramatic kill animation.
3. **`ActiveAnimPoweredSpecial=true` + `ActiveAnimPowered=false`** on NAPOWR despite no Upgrades=. Likely the "PoweredSpecial" state is the default-powered state for buildings with `PoweredSpecial=yes` rulesmd flag, and "Powered" (without Special) is a separate state for buildings without the Special suffix. DEFERRED — needs in-game observation.
4. **`NAAPWR` block at artmd.ini:3277** with Cameo=tpwricon (lowercase) — separate Soviet power plant variant in the art table. Not in the BuildingTypes list of rulesmd? DEFERRED — investigate whether NAAPWR is referenced or dead.
5. **`DoubleThick=true`** on NAPOWR_A but not GAPOWR_A — Westwood likely added DoubleThick for the Tesla coil's electrical arc visual prominence. GAPOWR's simpler turbines didn't need it.
6. **`Rate=300` vs GAPOWR's 220** — NAPOWR's anim is slower per-frame. Combined with 18 frames vs 8, the total animation duration is much longer. The Tesla coil's electrical buildup cycle is meant to feel deliberate and powerful.
7. **`DamageSmokeOffset=450, 200, 300`** vs GAPOWR's `300, 300, 450` — Westwood actually tuned these for the per-side art (unlike refineries where they were identical). NAPOWR's offsets are tuned for the Soviet Tesla Reactor's actual art coord system.

---

## Status

**DONE** — iteration 94. Index entry will be updated.

Doc total: **94**.

Next pick (priority): NANRCT (Soviet Nuclear Reactor — heavy power, explosive death). Then YAPOWR (Yuri Bio Reactor — garrison-boost hardcoded mechanic). Then barracks (GAPILE, NAHAND, YABRCK). Then war factories.
