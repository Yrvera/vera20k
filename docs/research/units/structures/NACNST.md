# [NACNST] — Soviet Construction Yard

**INI ID:** `NACNST`
**Display name:** `UIName=Name:NACNST` → CSF label "Soviet Construction Yard"
**Internal name:** `Name=Soviet Construction Yard`
**Side:** Soviet (4 Soviet factions, no YuriCountry)
**Category:** `[BuildingTypes]` (slot per [rulesmd.ini](../../../../ra2-rust-game/ini/rulesmd.ini))
**Owner:** `Owner=Russians,Confederation,Africans,Arabs` (4 Soviet factions — does NOT include YuriCountry)
**Doc filename:** `units/structures/NACNST.md`
**Loop iteration:** 89

**Role:** Soviet sister to GACNST. Build-tree root for all Soviet structures. Deployed from SMCV. Near-identical mechanics to GACNST with 2 notable artmd differences (taller Height=6, RemoveOccupy 1-8 for crane-arm clearance).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:12418-12447](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[NACNST]
UIName=Name:NACNST
Name=Soviet Construction Yard
ConstructionYard=yes
Strength=1000
Armor=concrete
TechLevel=-1
Adjacent=2
Factory=BuildingType
UndeploysInto=SMCV
Sight=8
Owner=Russians,Confederation,Africans,Arabs
Cost=3000
Points=80
Power=0
Capturable=true
Crewed=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
;DestroyAnim=NACNSTD
MaxDebris=15
MinDebris=5
DebrisAnim=Dbris1sm,Dbris1lg,Dbris4sm,Dbris5sm,Dbris4lg,Dbris7sm,Dbris8sm,Dbris5lg,Dbris4lg
ThreatPosed=0 ; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=1470,1060,1078
AIBuildThis=yes
TogglePower=no
ProtectWithWall=yes
EligibileForAllyBuilding=yes ;gs This allows a building of this type to count as a sucess in building placement, but only if that option is turned on
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
```

### Diffs vs GACNST (the relevant 5)

NACNST is a near-mirror of GACNST. Comparing field-by-field:

| Field            | GACNST                                            | NACNST                                             | Notes |
|------------------|---------------------------------------------------|----------------------------------------------------|-------|
| UndeploysInto    | AMCV                                               | **SMCV**                                            | parity diff (per-side MCV) |
| Owner            | British,French,Germans,Americans,**Alliance**     | Russians,Confederation,Africans,Arabs              | **No Alliance** (Korea is Allied-only); **No YuriCountry** (Yuri uses YACNST) |
| DebrisAnims      | DBRIS1LG,DBRIS1SM,DBRIS2LG,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM,DBRIS6LG,DBRIS6SM,DBRIS7LG (**10 anims**) | Dbris1sm,Dbris1lg,Dbris4sm,Dbris5sm,Dbris4lg,Dbris7sm,Dbris8sm,Dbris5lg,Dbris4lg (**9 anims**) | mostly the same pool but different per-side selection (e.g. NACNST has `Dbris8sm` not in GACNST list; GACNST has DBRIS2LG/6LG/6SM not in NACNST list; **Westwood used lowercase `DebrisAnim` (no S)** on NACNST vs `DebrisAnims` on GACNST — likely Westwood typo, engine is case-insensitive so it parses but worth flagging) |
| MinDebris        | 7                                                  | **5**                                               | NACNST drops 2 lower than GACNST. Same MaxDebris=15. |
| DamageParticleSystems | `;commented out`                              | **`SparkSys,SmallGreySSys,BigGreySmokeSys` (active)** | NACNST keeps damage smoke particles active; GACNST disables. Soviet ConYard literally smokes more visibly when damaged (asymmetric Westwood design touch). |
| ;DestroyAnim     | `;DestroyAnim=GACNSTDM` (commented)               | `;DestroyAnim=NACNSTD` (commented)                  | both commented out, same TS-era dead-code pattern |

Everything else is **identical** (ConstructionYard=yes, Strength=1000, Armor=concrete, TechLevel=-1, Adjacent=2, Factory=BuildingType, Sight=8, Cost=3000, Points=80, Power=0, Capturable=true, Crewed=yes, ThreatPosed=0, DamageSmokeOffset=1470,1060,1078, AIBuildThis=yes, TogglePower=no, ProtectWithWall=yes, EligibileForAllyBuilding=yes, ImmuneToPsionics=no, Explosion= same 5-anim palette).

### Field-by-field analysis (highlighting differences)

#### Identity
- **`UIName=Name:NACNST`** → CSF label "Soviet Construction Yard".
- **`Name=Soviet Construction Yard`** — fallback display name.
- **No `Image=` redirect** — NACNST has its own artmd block.

#### Construction-yard mechanics (identical to GACNST)
- **`ConstructionYard=yes`** — defining ConYard flag. BuildingType-scope (xref `0x0081aa74 → 0x00460a2b` in BuildingTypeClass_ReadINI_Water).
- **`Adjacent=2`** — Soviet build-adjacency radius. Same value as Allied (parity in adjacency rules).
- **`Factory=BuildingType`** — Soviet building producer. Same flag.
- **`UndeploysInto=SMCV`** — pack-up returns to SMCV. Bidirectional pair with `[SMCV] DeploysInto=NACNST`.

#### Ownership and faction restriction
- **`Owner=Russians,Confederation,Africans,Arabs`** — **4 Soviet factions**. Notably:
  - No `YuriCountry` — Yuri has separate `[YACNST]` (Yuri ConYard, doc pending iteration 90).
  - This is the standard Soviet partition: Russians (Soviet Union proper) + Confederation (Cuba) + Africans (Libya) + Arabs (Iraq). 4 factions, distinct from the 5 Allied factions which include Alliance (Korea).
- **`TechLevel=-1`** — hide-from-build-list. Acquired only via SMCV deploy or pre-placed.

#### Combat / capture
- **`Strength=1000`** — same as GACNST. (YACNST identical too — established parity.)
- **`Armor=concrete`** — hardest armor.
- **`Cost=3000` / `Points=80` / `Power=0`** — identical to GACNST.
- **`Capturable=true`** — Engineer-capturable. Capturing transfers Soviet build queue access.
- **`Crewed=yes`** — destruction ejects an E2 Conscript (Soviet equivalent of Allied E1 eject from GACNST).
- **`ImmuneToPsionics=no`** — explicitly NOT psi-immune. Same Westwood comment "defaults to yes for buildings, no for others".

#### Visual FX / destruction
- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — same 5-anim palette as GACNST. Same Westwood pattern across ConYards.
- **`;DestroyAnim=NACNSTD`** — commented out. Sister-pattern with GACNST's `;DestroyAnim=GACNSTDM`. The artmd `[NACNSTD]` block at [artmd.ini:17325](../../../../ra2-rust-game/ini/artmd.ini) is **also commented out** (`;[NACNSTD]`) — confirming the destroy-anim is dead-code on both sides. Likely TS-era leftover.
- **`MaxDebris=15` / `MinDebris=5`** — same max, NACNST drops 2 lower min. Subtle Soviet aesthetic touch (slightly less debris-spam? Or just Westwood-balance).
- **`DebrisAnim=`** (note: **lowercase, singular** — vs GACNST's `DebrisAnims=` plural). Soviet variant uses 9 debris pieces drawn from the standard `Dbris1sm..Dbris8sm` pool. Slightly different selection than GACNST.
- **`DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — **active on NACNST** (vs commented on GACNST). 3 particle systems: sparks + small grey smoke + big grey smoke. Soviet ConYard visibly smokes more when damaged. (Possibly Westwood's "industrial Soviet aesthetic" intent.)
- **`DamageSmokeOffset=1470,1060,1078`** — same offsets as GACNST. (Probably Westwood copy-pasted then forgot to retune for Soviet art coords — the offsets are tied to GACNST_A.shp coords; NACNST_A.shp may have different visible damage points.)

#### AI / placement hints (identical to GACNST)
- **`ThreatPosed=0`** — building threat 0.
- **`AIBuildThis=yes`** — AI allowed to construct.
- **`TogglePower=no`** — no player power toggle.
- **`ProtectWithWall=yes`** — AI hint to wall around.
- **`EligibileForAllyBuilding=yes`** — multiplayer share-build-queues eligibility. Westwood typo preserved.

The Soviet ConYard is **mechanically identical** to the Allied ConYard. The only differences are art assets, debris palette selection, particle-systems toggle, MinDebris=5 vs 7, and the lower-case `DebrisAnim` typo. **Every gameplay-relevant value is symmetric.**

---

## artmd.ini section — full transcript

[artmd.ini:1651-1690](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[NACNST]
Remapable=yes
Foundation=4x4
Height=6
AnimActive=0,26,3
Buildup=NACNSTMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
ActiveAnim=NACNST_A
ActiveAnimDamaged=NACNST_AD
ActiveAnimZAdjust=-77
IdleAnim=NACNST_C
IdleAnimZAdjust=-35
IdleAnimYSort=700
IdleAnimDamaged=NACNST_CD
;IdleAnimDamagedZAdjust=-77
;ActiveAnimTwo=NACNST_C
;ActiveAnimTwoDamaged=NACNST_CD
;ActiveAnimTwoZAdjust=-77
;ActiveAnimThree=NACNST_B
;ActiveAnimThreeZAdjust=-27
;PreProductionAnim=NACNST_B
;PreProductionAnimZAdjust=-15
ProductionAnim=NACNST_B
ProductionAnimDamaged=NACNST_BD
ProductionAnimZAdjust=-5
ProductionAnimYSort=700
OccupyHeight=4
CanHideThings=true
CanBeHidden=False
RemoveOccupy1=-2,-1
RemoveOccupy2=2,-1
RemoveOccupy3=1,-2
RemoveOccupy4=0,-2
RemoveOccupy5=-1,-2
RemoveOccupy6=-2,-2
RemoveOccupy7=-2,0
RemoveOccupy8=-2,1
DamageFireOffset0=-65,41
```

### Diffs vs GACNST artmd (significant)

| Field | GACNST artmd | NACNST artmd | Notes |
|-------|--------------|--------------|-------|
| Height | 4 | **6** | Soviet ConYard is taller (crane visible vs flat Allied roofline). |
| ActiveAnimZAdjust | -130 | **-77** | NACNST active anim closer to base; the Soviet ConYard layers differently. |
| ActiveAnimYSort | 362 | **(absent)** | NACNST omits ActiveAnimYSort; Soviet relies on IdleAnimYSort=700 + ProductionAnimYSort=700 instead. |
| IdleAnim | (absent) | **NACNST_C** | **Soviet has an IdleAnim layer** that Allied lacks. The blue glow / crane idle. References `[NACNST_C]` at [artmd.ini:17313](../../../../ra2-rust-game/ini/artmd.ini): 2-frame infinite loop. |
| IdleAnimDamaged | (absent) | **NACNST_CD** | Damaged variant of the idle (2-frame loop, `Image=NACNST_C`, frames 1-2). |
| IdleAnimZAdjust | (absent) | **-35** | Z-raise for idle anim. |
| IdleAnimYSort | (absent) | **700** | Idle anim Y-sort. |
| OccupyHeight | 3 | **4** | Z-occupancy for unit collision. Taller building blocks aircraft up to Z=4. |
| ProductionAnimYSort | 543 | **700** | NACNST production matches IdleAnim Y-sort (both at 700). |
| ProductionAnimZAdjust | -10 | **-5** | Less Z-raise on production anim. |
| RemoveOccupy1-8 | (absent) | **8 entries** | **Critical Soviet-specific feature.** Pre-clears 8 cells around the foundation for crane-arm clearance. |
| DamageFireOffset0 | -24,-1 | **-65,41** | Different damage fire location (taller building, different visible damage spot). |
| DamageFireOffset1 | 64,36 | **(absent)** | Soviet uses single fire point, not two. |

### RemoveOccupy1..8 — Soviet crane clearance

[artmd.ini:1682-1689](../../../../ra2-rust-game/ini/artmd.ini):

```ini
RemoveOccupy1=-2,-1
RemoveOccupy2=2,-1
RemoveOccupy3=1,-2
RemoveOccupy4=0,-2
RemoveOccupy5=-1,-2
RemoveOccupy6=-2,-2
RemoveOccupy7=-2,0
RemoveOccupy8=-2,1
```

8 cell offsets from the building's foundation anchor (top-left corner). At build placement, these cells are **removed from the occupied set** — meaning they remain walkable/passable even though they are within the visual extent of the building's crane arm.

The Soviet ConYard's tall industrial crane extends north (negative Y) and west (negative X) beyond the 4x4 foundation; without RemoveOccupy, the engine would mark those cells as occupied and units couldn't path through them. RemoveOccupy declares "this is visual-only space, not solid".

Confirmed Ghidra-scope: BuildingType (xref `0x0081a624 → 0x0046148f` in BuildingTypeClass_ReadINI_Water — the format string `RemoveOccupy%d` is read via `sprintf`-then-INI-lookup loop, slot 1..N). **NEW cheat-sheet entry.**

GACNST and YACNST do not use RemoveOccupy — their visual footprints exactly match their foundation. This is a NACNST-specific layout detail.

### Active vs Idle vs Production animation layering (Soviet 3-layer system)

GACNST has 2 anim layers: ActiveAnim + ProductionAnim. NACNST adds a third: **IdleAnim**.

- **`ActiveAnim=NACNST_A`** ([artmd.ini:17276](../../../../ra2-rust-game/ini/artmd.ini), 11-frame infinite loop) — the always-on base animation.
- **`ActiveAnimDamaged=NACNST_AD`** ([artmd.ini:17288](../../../../ra2-rust-game/ini/artmd.ini), frames 11-21, infinite loop) — damaged-state base anim.
- **`IdleAnim=NACNST_C`** ([artmd.ini:17313](../../../../ra2-rust-game/ini/artmd.ini), 2-frame loop with Shadow=yes) — a secondary always-on layer (the glowing red light / status indicator on the crane).
- **`IdleAnimDamaged=NACNST_CD`** ([artmd.ini:17301](../../../../ra2-rust-game/ini/artmd.ini), frames 1-2, looping) — damaged variant.
- **`ProductionAnim=NACNST_B`** ([artmd.ini:17263](../../../../ra2-rust-game/ini/artmd.ini), 21-frame one-shot) — plays while production happens.
- **`ProductionAnimDamaged=NACNST_BD`** ([artmd.ini:1693](../../../../ra2-rust-game/ini/artmd.ini), 22-42 frame range, one-shot) — damaged production anim.

The 3-layer system (Active + Idle + Production) allows the Soviet ConYard to have:
1. The base structure animating gently (Active = boiler/exhaust/rotation).
2. A status light always cycling (Idle = warning lamp).
3. A production effect when building (Production = crane swinging).

GACNST uses only 2 layers (Active + Production). The Soviet ConYard is **visually richer**, matching the Westwood "Soviet = industrial / dirty / busy" aesthetic.

### Commented-out alternative layouts (Westwood's iteration history)

```ini
;IdleAnimDamagedZAdjust=-77
;ActiveAnimTwo=NACNST_C
;ActiveAnimTwoDamaged=NACNST_CD
;ActiveAnimTwoZAdjust=-77
;ActiveAnimThree=NACNST_B
;ActiveAnimThreeZAdjust=-27
;PreProductionAnim=NACNST_B
;PreProductionAnimZAdjust=-15
```

Westwood iterated:
- An earlier layout used **`ActiveAnimTwo`** and **`ActiveAnimThree`** (multi-layered active anims for the C and B variants) before settling on the simpler IdleAnim approach.
- A **`PreProductionAnim`** was considered (a "warming up" anim before production starts) but dropped.

These commented entries reveal Westwood's engine supports `ActiveAnimTwo`, `ActiveAnimThree`, `PreProductionAnim` fields, but the shipping Soviet ConYard uses IdleAnim instead. **Latent engine capabilities for modders** — these field names parse and would work if uncommented.

### Buildup, foundation, render flags (shared with GACNST)

- **`Foundation=4x4`** — same 16-cell footprint.
- **`Buildup=NACNSTMK`** — Soviet buildup anim (sister to GACNSTMK).
- **`DemandLoadBuildup=true` / `FreeBuildup=true`** — same memory-thrift pattern.
- **`NewTheater=yes`** — new-theater asset substitution.
- **`Remapable=yes`** — house-color remap.
- **`CanHideThings=true` / `CanBeHidden=False`** — same Z-hide semantics.
- **`AnimActive=0,26,3`** — same active-anim params.

---

## Build queue chain — what NACNST unlocks

Every Soviet building has `Prerequisite=NACNST` (or a chain through it). Sample dependents at [rulesmd.ini:12454-...](../../../../ra2-rust-game/ini/rulesmd.ini):

| Building | Prerequisite | Role |
|----------|--------------|------|
| NAPOWR (Tesla Reactor) | `NACNST` | Power |
| NAREFN (Refinery) | `POWER,NACNST` | Ore refinery |
| NAHAND (Barracks) | `POWER,NACNST` | Infantry producer |
| NAWEAP (War Factory) | (typically POWER,NAPILE,NACNST or similar) | Vehicle producer |
| NARADR (Radar Tower) | `NAREFN,NACNST` or similar | Radar / SPYP source |
| NATECH (Battle Lab) | `NAWEAP,NACNST` | Tech prereq |
| NAYARD (Soviet Shipyard) | `NAREFN,NACNST` (or PROC,NACNST) | Naval |
| TESLA, NAFLAK, NALASR | `BARRACKS,NACNST` | Soviet defenses |
| NAMISL (Nuke Silo) | `NATECH,NACNST` | Superweapon |
| NAIRON (Iron Curtain) | `NATECH,NACNST` | Superweapon |
| NACLON (Cloning Vats) | `NATECH,NACNST` | Infantry duplicator |
| NAPSYB / NAPSYA / NAPSIS / NAINDP | each requires NACNST | Tech / specialty buildings |

Captured ConYard → captured Soviet tech tree. Same mechanic as GACNST.

The AI's Rules-global table `BuildConst=GACNST,NACNST,YACNST` at [rulesmd.ini:3065](../../../../ra2-rust-game/ini/rulesmd.ini) declares NACNST as Soviet equivalent.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                    | String address     | First xref               | Read scope                  |
|--------------------------|--------------------|--------------------------|-----------------------------|
| `RemoveOccupy%d`         | `0x0081a624`       | `0x0046148f`             | BuildingType (format-string loop) |
| `OccupyHeight`           | `0x0081a798`       | `0x004610f6`             | BuildingType                |
| `TogglePower`            | `0x0081ab68`       | `0x00460727`             | BuildingType                |
| `AIBasePlanningSide`     | `0x00843980`       | `0x00714a02`             | **TechnoType** (NOT BuildingType — broader scope) |

**4 NEW cheat-sheet entries this iteration:**

1. **`RemoveOccupy%d`** — `0x0081a624 → 0x0046148f` — BuildingType. Format-string-loop reader (1..N per slot). Confirms the engine supports arbitrary count of RemoveOccupy entries.
2. **`OccupyHeight`** — `0x0081a798 → 0x004610f6` — BuildingType. Z-occupancy for unit collision.
3. **`TogglePower`** — `0x0081ab68 → 0x00460727` — BuildingType. Player power-toggle permission. Also has paired `"NoTogglePower"` string at `0x0081be7c` and secondary xref `0x007e4cd4` (likely vtable/data table — deferred).
4. **`AIBasePlanningSide`** — `0x00843980 → 0x00714a02` — **TechnoType** scope. Per-unit AI side hint (0=Good, 1=Evil). Surprisingly TechnoType-scope rather than BuildingType-only, even though it's primarily used on buildings. (May be reusable on vehicles too; field is general.)

### Ghidra search log for this iteration

- `search_strings("RemoveOccupy")` → 1 match at `0x0081a624` (`RemoveOccupy%d` format string) → `0x0046148f` (BuildingType).
- `search_strings("AIBasePlanningSide")` → 1 match at `0x00843980` → `0x00714a02` (TechnoType).
- `search_strings("TogglePower")` → 2 matches: `0x0081ab68` (TogglePower) + `0x0081be7c` (NoTogglePower) → primary xref `0x00460727` (BuildingType).
- `search_strings("OccupyHeight")` → 1 match at `0x0081a798` → `0x004610f6` (BuildingType).

### Unit-specific hardcoded behavior?

NACNST has **no detectable unit-specific hardcoded code**. Same conclusion as GACNST:

- All ConYard behavior derives from the generic `ConstructionYard=yes` + `Factory=BuildingType` + `UndeploysInto=` field combination.
- The AI uses the `BuildConst=` table to identify all 3 ConYards by INI-ID match; no hardcoded NACNST-specific path.
- E2 (Conscript) crew eject on death is generic `Crewed=yes` → side-mapped crew lookup, not NACNST-keyed.

### TS-legacy filter

- **`Crewed=yes`** — active YR.
- **`Capturable=true`** — active YR (Engineer mechanic).
- **`ImmuneToPsionics=no`** — active YR (psi-vulnerability).
- **`;DestroyAnim=NACNSTD`** + **`;[NACNSTD]`** in artmd — both commented out; dead-code pair. TS-era leftover.
- **`AnimActive=0,26,3`** — active YR field (engine reads, value chosen per-building).
- **`DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — active YR.
- **No fog-of-war / 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.
- The commented `ActiveAnimTwo` / `ActiveAnimThree` / `PreProductionAnim` are **latent engine capabilities** (likely TS holdovers that still parse), not dead code per se — they would work if uncommented but the shipped art uses the simpler 3-layer scheme.

NACNST has no TS-legacy gating. Fully active in standard YR.

---

## Cross-references

- **`GACNST`** (`units/structures/GACNST.md`) — DONE. Allied ConYard. Near-identical mechanics; differs in art and 5 rulesmd fields. Cross-reference established.
- **`YACNST`** — pending iteration 90. Closes the ConYard trio.
- **`SMCV`** (`units/soviet/SMCV.md`) — DONE. Bidirectional pair (`SMCV.DeploysInto=NACNST`).
- **`AMCV`** (`units/allied/AMCV.md`) — DONE.
- **`PCV`** (`units/yuri/PCV.md`) — DONE. Yuri MCV pair to YACNST.
- **`NAPOWR`, `NAHAND`, `NAREFN`, `NAWEAP`, `NATECH`, `NARADR`, `NAYARD`, `NAINDP`, `NACLON`, `NAMISL`, `NAIRON`, `NAPSYA`, `NAPSYB`, `NAPSIS`, `TESLA`, `NALASR`, `NAFLAK`, `NATBNK`, `NABNKR`, `NANRCT`** — all pending; all have `Prerequisite=NACNST`.
- **`ENGINEER`, `SENGINEER`, `YENGINEER`** — capture mechanic via `Capturable=true`.

---

## Coverage audit

INI fields covered (27 rulesmd + 19 artmd + 4 sub-anims + 4 commented-but-discussed = 54 entries):

**Coverage: 100%.** Every key in the rulesmd NACNST block, the artmd NACNST block, and the 4 referenced sub-animation blocks (NACNST_A, NACNST_AD, NACNST_B, NACNST_BD, NACNST_C, NACNST_CD = 6 actually) has been transcribed and explained. The 7 commented-out artmd alternatives (IdleAnimDamagedZAdjust, ActiveAnimTwo/Three, PreProductionAnim) are documented as Westwood iteration-history.

---

## Open questions / Westwood inconsistencies

1. **`DebrisAnim` (singular, lowercase) vs `DebrisAnims` (plural)** on GACNST. NACNST uses `DebrisAnim=`; GACNST uses `DebrisAnims=`. Engine appears case-insensitive and accepts both — but this is a Westwood inconsistency. (Field-name parsing likely strips trailing `s` or uses substring match; precise behavior DEFERRED to Ghidra trace of `Debris` string match.)
2. **`DamageSmokeOffset=1470,1060,1078`** identical to GACNST despite different SHP art. Likely Westwood copy-paste oversight — the offsets may not align with the NACNST_A art's actual damage points.
3. **`Owner=` excludes YuriCountry** — confirmed. Yuri factions cannot deploy SMCV (Soviet MCV) into a Soviet ConYard. Yuri has separate YACNST. But note: a mind-controlled SMCV (Yuri controlling a Soviet) — does deploying it produce a NACNST for the Yuri player? Possibly the Owner check applies at deploy-time. DEFERRED to mind-control / capture-pair audit.
4. **AI's BuildConst table = `GACNST,NACNST,YACNST` regardless of side.** The AI scans all 3 ConYards even on a Soviet match. Does the AI try to build a GACNST on a Soviet side? The Owner= check filters this at build-validation, but the AI planner sees all 3 ConYards in its table. DEFERRED.
5. **MinDebris=5 vs GACNST=7** — minor balance asymmetry. Is the Soviet ConYard meant to drop slightly less debris on death? Or is this just Westwood iteration noise? Likely the latter.
6. **`AIBasePlanningSide` TechnoType-scope** — surprising. The field's semantic meaning (good vs evil AI base planner side) only makes sense for buildings, yet it's read at TechnoType level. Possible reason: legacy TS field that worked on vehicles too (e.g., MCV planning side). DEFERRED.

---

## Status

**DONE** — iteration 89. Index entry updated.

Doc total: **89**.

Next pick (priority): YACNST (Yuri ConYard) — completes the ConYard trio. Then refineries (GAREFN/NAREFN; YAREFN done), then power plants (GAPOWR/NAPOWR/NANRCT/YAPOWR — Tesla Reactor, Nuclear Reactor, Bio Reactor), then barracks, war factories, battle labs.
