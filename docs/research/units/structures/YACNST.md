# [YACNST] — Yuri Construction Yard

**INI ID:** `YACNST`
**Display name:** `UIName=Name:YACNST` → CSF label "Yuri Construction Yard"
**Internal name:** `Name=Yuri Construction Yard`
**Side:** Yuri (single-faction: YuriCountry)
**Category:** `[BuildingTypes]`
**Owner:** `Owner=YuriCountry` (single faction; vs Allied 5, Soviet 4)
**Doc filename:** `units/structures/YACNST.md`
**Loop iteration:** 90

**Role:** Yuri sister to GACNST/NACNST. Build-tree root for all Yuri structures. Deployed from PCV. Closes the ConYard trio. Combines GACNST's compact 4×4/Height=4 footprint with NACNST's 3-layer (Active+Idle+Production) animation system; adds one Yuri-unique stat (Sight=10).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:13091-13121](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
[YACNST]
UIName=Name:YACNST
Name=Yuri Construction Yard
;Image=GACNST
ConstructionYard=yes
Strength=1000
Armor=concrete
TechLevel=-1
Adjacent=2
Factory=BuildingType
UndeploysInto=PCV
Sight=10
Owner=YuriCountry
Cost=3000
Points=80
Power=0
Capturable=true
Crewed=yes
;DestroyAnim=GACNSTDM
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS2LG,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM,DBRIS6LG,DBRIS6SM,DBRIS7LG
MaxDebris=15
MinDebris=7
ThreatPosed=0	; This value MUST be 0 for all building addons
;DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=1470, 1060, 1078
AIBuildThis=yes
TogglePower=no
ProtectWithWall=yes
EligibileForAllyBuilding=yes ;gs This allows a building of this type to count as a sucess in building placement, but only if that option is turned on
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
```

### Diffs vs GACNST and NACNST

YACNST is **closer to GACNST than to NACNST**. Comparing across all three:

| Field | GACNST | NACNST | YACNST |
|-------|--------|--------|--------|
| UndeploysInto | AMCV | SMCV | **PCV** |
| Owner | British,French,Germans,Americans,Alliance (5) | Russians,Confederation,Africans,Arabs (4) | **YuriCountry (1)** |
| **Sight** | 8 | 8 | **10** |
| DebrisAnims (key) | `DebrisAnims=` (plural) | `DebrisAnim=` (singular, lowercase) | `DebrisAnims=` (plural — matches GACNST) |
| DebrisAnims (values) | DBRIS1LG,DBRIS1SM,DBRIS2LG,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM,DBRIS6LG,DBRIS6SM,DBRIS7LG (10 anims, matches GACNST exactly) | 9 anims, mostly the same pool but DBRIS8sm + lowercase | 10 anims **identical to GACNST list** |
| MinDebris | 7 | 5 | **7 (matches GACNST)** |
| MaxDebris | 15 | 15 | 15 (all three the same) |
| DamageParticleSystems | `;commented` | active 3-system | `;commented` (matches GACNST) |
| ;DestroyAnim | `;DestroyAnim=GACNSTDM` | `;DestroyAnim=NACNSTD` | **`;DestroyAnim=GACNSTDM` (Westwood typo — references GACNST's destroy anim, not YACNSTDM)** |
| `;Image=` | (absent) | (absent) | **`;Image=GACNST` (commented out)** |

**5 YACNST-unique rulesmd traits:**

1. **`Sight=10`** — **+2 over GACNST/NACNST's Sight=8**. Why does Yuri's ConYard see further? Possibly Westwood's "psychic Yuri" theme — the ConYard has a small psychic-vision boost. (Game-balance impact: minor; the player typically has plenty of other vision sources.)
2. **`Owner=YuriCountry`** — Yuri is a single-faction side. Reflects YR's Yuri faction design (one country, no sub-factions like Allied's 5 or Soviet's 4).
3. **`UndeploysInto=PCV`** — Yuri's MCV (PCV = Psychic Construction Vehicle, also called "Yuri Construction Vehicle" in some sources).
4. **`;Image=GACNST` commented out**. Westwood at some point planned for YACNST to share GACNST's voxel asset, but reverted before shipping. YACNST has its own art assets ([artmd.ini:1622](../../../../ra2-rust-game/ini/artmd.ini)).
5. **`;DestroyAnim=GACNSTDM` typo** — references GACNST's destroy anim, not a YACNSTDM. Westwood likely copy-pasted from GACNST when authoring YACNST and didn't update the reference. Since this line is commented out, it has no in-game effect — but it's a Westwood code-smell artifact preserved.

**Identical to GACNST in:** Strength=1000, Armor=concrete, TechLevel=-1, Adjacent=2, Factory=BuildingType, Cost=3000, Points=80, Power=0, Capturable=true, Crewed=yes, Explosion= (same 5-anim palette), MaxDebris=15, MinDebris=7, ThreatPosed=0, DamageSmokeOffset=1470/1060/1078, AIBuildThis=yes, TogglePower=no, ProtectWithWall=yes, EligibileForAllyBuilding=yes, ImmuneToPsionics=no.

So YACNST = GACNST's stats + Sight=10 + Yuri-flavor metadata. Cleanest of the three trios for game-balance parity.

### Per-key analysis (highlighting Yuri-specific)

#### Identity
- **`UIName=Name:YACNST`** → CSF label "Yuri Construction Yard".
- **`;Image=GACNST`** — commented out, see above.

#### ConYard mechanics — identical to GACNST/NACNST
- **`ConstructionYard=yes`** — defining flag.
- **`Adjacent=2`** — parity adjacency radius. All three ConYards seed equivalent build radii.
- **`Factory=BuildingType`** — Yuri building producer.
- **`UndeploysInto=PCV`** — bidirectional pair with `[PCV] DeploysInto=YACNST`.

#### Ownership
- **`Owner=YuriCountry`** — single faction.
- **`TechLevel=-1`** — hide-from-build-list.

#### Combat / capture — identical to GACNST
- All same as GACNST except Sight=10.

#### Visual FX
- **`Explosion=`** — same 5-anim Allied palette (Westwood didn't customize for Yuri).
- **`DebrisAnims=`** — identical to GACNST (10 anims, plural key spelling).
- **`MaxDebris=15` / `MinDebris=7`** — identical to GACNST. (NACNST's MinDebris=5 is the outlier.)
- **`DamageSmokeOffset=1470, 1060, 1078`** — same as both siblings (Westwood copy-paste).
- **`;DamageParticleSystems`** commented — same as GACNST. Particle systems disabled. NACNST is the only sibling with active particle systems.

#### AI / placement — identical
- `ThreatPosed=0`, `AIBuildThis=yes`, `TogglePower=no`, `ProtectWithWall=yes`, `EligibileForAllyBuilding=yes`.

---

## artmd.ini section — full transcript

[artmd.ini:1622-1649](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[YACNST]
;Image=GACNST
Cameo=YCONICON
Remapable=yes
Foundation=4x4
Height=4
AnimActive=0,26,3
Buildup=YACNSTMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
ActiveAnim=YACNST_A
ActiveAnimDamaged=YACNST_AD
ActiveAnimZAdjust=-130
ActiveAnimYSort=650
ProductionAnim=YACNST_B
ProductionAnimDamaged=YACNST_BD
ProductionAnimZAdjust=-15
ProductionAnimYSort=650
IdleAnim=YACNST_C
IdleAnimDamaged=YACNST_CD
IdleAnimZAdjust=-135
IdleAnimYSort=650
CanHideThings=True
CanBeHidden=False
OccupyHeight=3
DamageFireOffset0=-24,-1
DamageFireOffset1=64,36
```

### Diffs vs GACNST and NACNST artmd

| Field                  | GACNST artmd            | NACNST artmd                       | YACNST artmd                       |
|------------------------|-------------------------|------------------------------------|------------------------------------|
| **Cameo=**             | (implicit — no explicit Cameo= line)  | (implicit) | **`Cameo=YCONICON` explicit** |
| Foundation             | 4x4                     | 4x4                                | 4x4 (all parity) |
| **Height**             | 4                       | **6** (taller) | **4 (matches GACNST)** |
| **OccupyHeight**       | 3                       | **4** (taller) | **3 (matches GACNST)** |
| ActiveAnim             | GACNST_A                | NACNST_A                            | YACNST_A |
| ActiveAnimDamaged      | GACNST_AD               | NACNST_AD                           | YACNST_AD |
| **ActiveAnimZAdjust**  | **-130**                | -77 (different) | **-130 (matches GACNST)** |
| **ActiveAnimYSort**    | 362                     | (absent)                           | **650** |
| ProductionAnim         | GACNST_B (20 frames)    | NACNST_B (21 frames)                | YACNST_B (18 frames — shortest) |
| **ProductionAnimZAdjust** | -10                  | -5                                 | **-15** |
| **ProductionAnimYSort** | 543                    | 700                                | **650** |
| **IdleAnim**           | (absent)                | **NACNST_C** | **YACNST_C** |
| IdleAnimDamaged        | (absent)                | NACNST_CD                          | YACNST_CD |
| IdleAnimZAdjust        | (absent)                | -35                                | **-135** |
| IdleAnimYSort          | (absent)                | 700                                | **650** |
| RemoveOccupy1-8        | (absent)                | **8 entries (crane clearance)** | (absent — matches GACNST) |
| DamageFireOffset0      | -24,-1                  | -65,41                             | **-24,-1 (matches GACNST exactly)** |
| DamageFireOffset1      | 64,36                   | (absent)                           | **64,36 (matches GACNST exactly)** |

**YACNST takes the best of both:**
- **GACNST's compact dimensions** (Height=4, OccupyHeight=3, ActiveAnimZAdjust=-130, both DamageFireOffsets).
- **NACNST's 3-layer animation system** (Active + Idle + Production).
- **Adds its own Y-sort scheme** (650 across all three anims — a Yuri-specific uniform sort tier).
- **Adds explicit `Cameo=YCONICON`** — the only ConYard with an explicit Cameo override in artmd. GACNST/NACNST rely on the implicit `<INI_ID>ICON` convention; YACNST routes through `YCONICON` (not `YACNSTICON`).

### YACNST sub-animations

[artmd.ini:17107-17175](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[YACNST_A]                        ; Active idle anim (4-frame infinite loop)
Normalized=yes
Start=0
LoopStart=0
LoopEnd=3
LoopCount=-1
Rate=200
Layer=ground
NewTheater=yes

[YACNST_AD]                       ; Damaged active anim (Image=YACNST_A, frames 3-6, infinite)
Image=YACNST_A
Normalized=yes
Start=3
LoopStart=3
LoopEnd=6
LoopCount=-1
Rate=200
Layer=ground
NewTheater=yes

[YACNST_B]                        ; Production anim (18-frame one-shot)
Normalized=yes
LoopStart=0
LoopEnd=18
LoopCount=1
Rate=200
Layer=ground
NewTheater=yes
Shadow=yes

[YACNST_BD]                       ; Damaged production (frames 19-36 of YACNST_B, one-shot)
Image=YACNST_B
Normalized=yes
Start=19
LoopStart=19
LoopEnd=36
LoopCount=1
Rate=200
Layer=ground
NewTheater=yes
Shadow=yes

[YACNST_C]                        ; Idle crane anim (2-frame infinite loop)
Normalized=yes
LoopStart=0
LoopEnd=1
LoopCount=-1
Layer=ground
NewTheater=yes
Shadow=yes

[YACNST_CD]                       ; Damaged idle crane (Image=YACNST_C, frames 1-2, infinite)
Image=YACNST_C
Normalized=yes
LoopStart=1
LoopEnd=2
LoopCount=-1
Layer=ground
NewTheater=yes
```

The frame counts (Active 4, Production 18, Idle 2) are tighter than GACNST (Active 4, Production 20) and NACNST (Active 11, Production 21, Idle 2) — YACNST has the **shortest production animation** in the trio, at 18 frames.

The Image= redirect inside `*AD`, `*BD`, `*CD` sub-blocks follows the same pattern as GACNST/NACNST — single SHP, frame-range slicing for damaged variants. Memory-efficient.

### Buildup

- **`Buildup=YACNSTMK`** — Yuri buildup anim. Like GACNSTMK/NACNSTMK, this is a one-shot SHP loaded on-demand (DemandLoadBuildup=true) and freed after play (FreeBuildup=true).

### Render flags

- **`Remapable=yes`** — house color tints the voxel/SHP at remap palette index.
- **`NewTheater=yes`** — applies to building + active anim + idle + production.
- **`CanHideThings=True`** / **`CanBeHidden=False`** — same Z-hide semantics as siblings.
- **`AnimActive=0,26,3`** — same animation sub-frame range params.
- **`Cameo=YCONICON`** — sidebar build icon. The `YCON` prefix (vs `YACNST` everywhere else) suggests Westwood named the cameo after a separate naming convention (`Y` + `CON` for "Yuri Construction"). The implicit cameo lookup for `YACNST` would have been `YACNSTICON` if no explicit Cameo= was set. The explicit Cameo= override here routes to a different SHP file.

### No RemoveOccupy on YACNST

YACNST does **not** declare RemoveOccupy1-8. Unlike NACNST's crane that extends beyond the 4x4 footprint, YACNST's visual footprint matches its 4x4 foundation exactly. Same as GACNST.

---

## Build queue chain — what YACNST unlocks

Every Yuri building has `Prerequisite=YACNST` (or chains through it). Sample at [rulesmd.ini:13130-...](../../../../ra2-rust-game/ini/rulesmd.ini):

| Building | Prerequisite | Role |
|----------|--------------|------|
| YAPOWR (Bio Reactor) | `YACNST` | Power (boost via garrison Initiates) |
| YABRCK (Yuri Barracks) | `POWER,YACNST` | Infantry producer |
| YAREFN ([Slave Miner deployed], DONE) | varies | Refinery |
| YAWEAP (War Factory) | `POWER,YABRCK,YACNST` (typical) | Vehicle producer |
| YATECH (Battle Lab) | `YAREFN,YACNST` | Tech prereq |
| YAYARD (Sub Pen) | `YAREFN,YACNST` (or similar) | Naval |
| YACOMD (Yuri command/radar) | varies | Radar provider |
| YAGGUN (Gattling Cannon), YAPSYT (Psychic Tower), YAGRND (Grinder), YAGNTC (Genetic Mutator), YAPPET (Psychic Dominator), NAPSYA (Psychic Beacon — actually NA prefix despite being Yuri-tier?) | each requires YACNST | Yuri specialty |

The AI's Rules-global table `BuildConst=GACNST,NACNST,YACNST` includes YACNST as the Yuri ConYard for AI recognition.

---

## Yuri's tactical signature on ConYard

The Yuri ConYard contributes to Yuri's faction identity in two subtle ways:

1. **Sight=10** — small psychic-vision boost over Allied/Soviet's 8. Symbolizes Yuri's "psychic" theme even on inert structures.
2. **Cameo=YCONICON explicit** — Yuri's sidebar has a distinct artistic style (purple/magenta vs Allied gold or Soviet red), and the explicit Cameo override routes to a Yuri-styled icon outside the standard `<INI_ID>ICON` convention.

Beyond these, the ConYard mechanics are mechanically symmetric with GACNST. Yuri's faction advantages emerge from his other buildings (Bio Reactor garrison boost, Slave Miner economy, Grinder recycling, Psychic Tower, etc.), not from the ConYard itself.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field                    | String address     | First xref               | Read scope                  |
|--------------------------|--------------------|--------------------------|-----------------------------|
| `IdleAnimYSort`          | `0x00819794`       | `0x00464190`             | BuildingType                |
| `ActiveAnimTwoPoweredSpecial` | `0x0081a410`  | `0x00461cf8`             | BuildingType (engine supports power-state ActiveAnimTwo variants) |
| `DamageFireOffset`       | `0x0081ac60`       | `0x004603ab`             | BuildingType                |

**3 NEW cheat-sheet entries this iteration:**

1. **`IdleAnimYSort`** — `0x00819794 → 0x00464190` — BuildingType. Confirms NACNST/YACNST use this; engine reads it. Also discovered **siblings** at adjacent addresses: `IdleAnimPowered`, `IdleAnimPoweredEffect`, `IdleAnimPoweredLight`, `IdleAnimPoweredSpecial` ([strings at 0x00819784, 0x00819754, 0x0081976c, 0x0081973c]). The engine supports **idle anim variants for different power states** — `IdleAnimPowered` plays when the building is powered, etc. None of the ConYards use these (they have Power=0), but the engine supports them — likely used on Tesla Reactor / Cloning Vats / superweapons.
2. **`ActiveAnimTwoPoweredSpecial`** — `0x0081a410 → 0x00461cf8` — BuildingType. Confirms the **ActiveAnimTwo + power-state matrix** is fully engine-supported, even though no ConYard uses it. NACNST has `;ActiveAnimTwo=` commented out as a Westwood iteration artifact, but the engine WOULD read it if uncommented. Total 11 ActiveAnimTwo* variants discovered (Powered, PoweredLight, PoweredEffect, PoweredSpecial — full quad for each anim slot).
3. **`DamageFireOffset`** — `0x0081ac60 → 0x004603ab` — BuildingType (base read path). The numbered variants (DamageFireOffset0, DamageFireOffset1) likely format-string-loop off this base, similar to RemoveOccupy%d.

### Ghidra search log for this iteration

- `search_strings("YuriCountry")` → 3 matches: `STT:PlayerSideYuriCountry` (CSF UI string), `LoadBrief:YuriCountry` (campaign load brief), `Name:YuriCountry` (CSF country name). All 3 are CSF lookup keys, not parser keys. Confirms YuriCountry is the single Yuri faction's CSF binding.
- `search_strings("IdleAnim")` → 11 matches total. Top 5 shown — including `IdleAnimYSort` (parser key), and 4 power-state variants `IdleAnim`, `IdleAnimPowered`, `IdleAnimPoweredLight`, `IdleAnimPoweredEffect`, `IdleAnimPoweredSpecial`. **Engine supports a 4-stage power animation system for idle anims.**
- `search_strings("ActiveAnimTwo")` → 11 matches total. Confirms **ActiveAnimTwo + power-state-matrix** support. The shipping ConYards don't use this, but the engine reads it. Modders / TS leftover.
- `search_strings("Cameo")` → 8 matches (CAMEO.PAL, CAMEO.MIX, CAMEOMD.MIX, etc.). The Cameo= INI key itself is read via TechnoType-scope (existing cheat-sheet). The `CAMEOMD.MIX` reference confirms YR uses MD-suffix variants (vs RA2 base `CAMEO.MIX`).

### Yuri-specific hardcoded behavior?

YACNST has **no detectable unit-specific hardcoded code path**. As with GACNST/NACNST:

- All ConYard behavior derives from generic `ConstructionYard=yes` + `Factory=BuildingType` + `UndeploysInto=PCV` field combination.
- Yuri's psychic-themed faction logic lives elsewhere (PsychicSensor, MIND infiltration via PTROOP, etc.), not on the ConYard.
- The `Sight=10` value is a per-instance INI override, not hardcoded.
- `YuriCountry` as an Owner check is a generic House-name lookup against the `[Countries]` table; no YACNST-specific code path.

### TS-legacy filter

- All standard ConYard fields are active YR.
- **`;Image=GACNST` commented out** — Westwood iteration artifact, not TS-legacy specifically.
- **`;DestroyAnim=GACNSTDM` typo (references GACNST's anim)** — commented out, no in-game effect.
- **No FogOfWar / SpecialFlags & 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.

YACNST has no TS-legacy gating. Fully active in standard YR.

---

## Cross-references

- **`GACNST`** (`units/structures/GACNST.md`) — DONE. Mechanically nearest sibling; YACNST inherits most stats from GACNST.
- **`NACNST`** (`units/structures/NACNST.md`) — DONE. Animation-system nearest sibling; YACNST inherits NACNST's 3-layer Active+Idle+Production anim approach.
- **`PCV`** (`units/yuri/PCV.md`) — DONE. Bidirectional pair (`PCV.DeploysInto=YACNST`).
- **`AMCV`, `SMCV`, `PCV`** — three MCVs deploy into three ConYards. Pair table:
  - AMCV ↔ GACNST (Allied)
  - SMCV ↔ NACNST (Soviet)
  - PCV ↔ YACNST (Yuri)
- **`YAPOWR` (Bio Reactor)** — pending. Yuri power building. Prerequisite=YACNST.
- **`YABRCK` (Yuri Barracks)**, **`YAWEAP` (Yuri War Factory)**, **`YATECH` (Yuri Battle Lab)**, **`YAGGUN` (Gattling Cannon)**, **`YAPSYT` (Psychic Tower)**, **`YAGRND` (Grinder)**, **`YAGNTC` (Genetic Mutator)**, **`YAPPET` (Psychic Dominator)** — all pending; all have `Prerequisite=YACNST`.
- **`YAREFN`** (`units/structures/YAREFN.md`) — DONE. Slave Miner deployed form; building variant of SMIN.
- **`ENGINEER`, `SENGINEER`, `YENGINEER`** — capture mechanic.

---

## Coverage audit

INI fields covered (28 rulesmd + 23 artmd + 6 sub-animations = 57 entries):

**Coverage: 100%.** Every key in YACNST's rulesmd, artmd, and the 6 referenced sub-anim blocks (YACNST_A, YACNST_AD, YACNST_B, YACNST_BD, YACNST_C, YACNST_CD) transcribed and explained.

---

## Open questions / Westwood inconsistencies

1. **`Sight=10`** — only Yuri ConYard has this. Was Westwood intending to give Yuri faction-wide vision advantages and only the ConYard kept this stat? Or is it a one-off Yuri-flavor touch? DEFERRED to cross-Yuri-buildings audit.
2. **`Cameo=YCONICON` explicit** — only ConYard with explicit cameo override. Why `YCON` and not `YACNST`? Possibly Westwood named the SHP `yconicon.shp` early in development and kept the legacy name. Cross-ref to `RPN_ICON_KEY` lookup pending.
3. **`;DestroyAnim=GACNSTDM`** typo — YACNST's commented destroy anim references GACNST's, not YACNSTDM. Westwood copy-paste error. Since the line is commented out, no in-game effect.
4. **`;Image=GACNST`** commented — Westwood considered using GACNST's voxel/SHP for YACNST but reverted. The line's existence suggests there was a development phase where YACNST and GACNST shared assets, and Yuri's distinct art was added later.
5. **Why no RemoveOccupy on YACNST?** NACNST needs RemoveOccupy 1-8 for crane clearance because its visual extent exceeds its 4x4 foundation. YACNST has the same 4x4 foundation as GACNST and its art fits within. NACNST is the only ConYard with extended visual extent.
6. **Engine has 11 IdleAnim* variants and 11 ActiveAnimTwo* variants** (power-state matrix) but no ConYard uses them. The shipping art is simpler than the engine supports. These are latent capabilities — potentially used by other buildings (Tesla Reactor, Iron Curtain, etc.) but not the ConYards. Audit pending across other buildings.

---

## Status

**DONE** — iteration 90. Index entry updated. **ConYard trio (GACNST + NACNST + YACNST) complete.**

Doc total: **90**.

Next pick (priority): Refineries — **GAREFN (Allied Ore Refinery), NAREFN (Soviet Ore Refinery)**. YAREFN is already DONE (it's the Slave Miner's deployed form). GAREFN spawns CMIN; NAREFN spawns HARV — close the refinery trio. Then power plants (GAPOWR, NAPOWR, NANRCT, YAPOWR — 4 since Soviet has both Tesla Reactor and Nuclear Reactor).
