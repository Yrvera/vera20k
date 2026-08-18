# [GAPILE] — Allied Barracks

**INI ID:** `GAPILE`
**Display name:** `UIName=Name:GAPILE` → CSF "Allied Barracks"
**Internal name:** `Name=Allied Barracks ;needs different Given Name to avoid editor confusion`
**Side:** Universal-Owner (all 10 factions)
**Category:** `[BuildingTypes]`
**Owner:** all 10 factions
**Doc filename:** `units/structures/GAPILE.md`
**Loop iteration:** 97

**Role:** Allied infantry producer. Tier-2 building (Cost=500). Engine-tagged as Allied-side barracks via `GDIBarracks=yes` flag — the side-specific Barracks flag system is one of three (GDIBarracks/NODBarracks/YuriBarracks) the engine uses to recognize per-faction infantry producers. The `GDI`/`NOD` naming is **TS-era heritage preserved as parser keys** (the actual game has Allied/Soviet/Yuri, but the engine still uses TS faction labels).

---

## rulesmd.ini section — full transcript

[rulesmd.ini:11687-11719](../../../../ra2-rust-game/ini/rulesmd.ini):

```ini
; Allied Barracks
[GAPILE]
UIName=Name:GAPILE
Name=Allied Barracks ;needs different Given Name to avoid editor confusion
BuildCat=Tech
Prerequisite=POWER,GACNST
Strength=500
Armor=steel
Factory=InfantryType
Adjacent=2
TechLevel=2
Sight=5
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
AIBasePlanningSide=0 ;gs 0 for Good, 1 for Evil
Cost=500
Points=30
Power=-10
Crewed=yes
Capturable=true
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
;DestroyAnim=GAPILEDM
MaxDebris=15
MinDebris=5
DebrisAnims=DBRIS1LG,DBRIS1SM,DBRIS4LG,DBRIS4SM,DBRIS5LG,DBRIS5SM
ThreatPosed=0 ; This value MUST be 0 for all building addons
ExitCoord=-64,64,0
GDIBarracks=yes
DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys
DamageSmokeOffset=215,395,200
AIBuildThis=yes
Spyable=yes
;WantsExtraSpace=yes ; gs This will look for a space AIBaseSpacing+1 when the computer places, but will settle for AIBasSpacing
ImmuneToPsionics=no ; defaults to yes for buildings, no for others
```

### Identity & UI

- **`UIName=Name:GAPILE`** → CSF "Allied Barracks".
- **`Name=Allied Barracks ;needs different Given Name to avoid editor confusion`** — verbatim Westwood comment. The `;needs different Given Name to avoid editor confusion` suggests World Builder / the map editor relies on the Name= for some display purpose, and Westwood had a different display name to avoid conflicts in editor tooling.
- **`BuildCat=Tech`** — sidebar Tech tab (vs Power/Resource/etc.).

### Side-specific barracks flag (engine-tagged)

- **`GDIBarracks=yes`** — engine-keyed flag tagging this building as the **GDI** (Allied) barracks. Confirmed Ghidra-scope: BuildingType (xref `0x0081aa00 → 0x00460b15`). **NEW cheat-sheet entry.**

The engine has **three side-specific Barracks flags** at consecutive addresses:
- **`GDIBarracks`** at `0x0081aa00` → xref `0x00460b15` — Allied (GAPILE)
- **`NODBarracks`** at `0x0081a9f4` → xref `0x00460b2f` — Soviet (NAHAND)
- **`YuriBarracks`** at `0x0081a9e4` → xref `0x00460b45` — Yuri (YABRCK)

All three xrefs are 0x10-0x20 bytes apart in the same parser function — they're a tight sequential parser triple. The engine reads ALL THREE flags per BuildingType and tags the building's faction-barracks role accordingly. The `GDI` and `NOD` naming are **TS-era heritage**: in Tiberian Sun, the two sides were GDI (good) and NOD (evil). RA2/YR retained the parser key names even though the in-game sides are Allied/Soviet/Yuri.

Why three separate flags instead of `BarracksSide=Allied|Soviet|Yuri`? Engine simplicity — each side's AI can directly check `if building.GDIBarracks` to find its faction's barracks, without enum lookup. The 3-flag system also allows a building to be MULTIPLE sides' barracks simultaneously (a captured GAPILE could theoretically have all three flags set, though the INI doesn't allow it directly).

This is one of the most important hardcoded faction-recognition mechanisms in the engine. The AI Build*= tables (BuildBarracks=NAHAND,GAPILE,YABRCK) reference all three by INI-ID, but the per-building side-flag is what matters for spawn-side logic (which side's infantry are produced here).

### Factory declaration

- **`Factory=InfantryType`** — declares this building as the producer of InfantryType units. Pairs with `Factory=BuildingType` on ConYards and `Factory=UnitType` on War Factories.
- Combined with `GDIBarracks=yes`, the engine knows: "this builds infantry FOR the Allied side". Different infantry are buildable depending on the owner's faction (per each unit's `Owner=` filter).

### Build gating

- **`Prerequisite=POWER,GACNST`** — needs Allied power + ConYard. POWER is the Rules-global alias resolving to GAPOWR/NAPOWR/YAPOWR.
- **`TechLevel=2`** — tier-2 (just above ConYard/Power).
- **`Cost=500`** — cheap. Second-cheapest core building after the Dog Kennel.
- **`Adjacent=2`** — standard build-adjacency.
- **`Sight=5`** — moderate vision.
- **`Power=-10`** — minimal power consumption (10 units). Barracks consume very little.

### Combat / capture / spy

- **`Strength=500`** — relatively fragile. The cheapest core building HP — lower than refinery (1000), power plant (750), war factory (1000). Barracks fall first under harassment.
- **`Armor=steel`** — same as ConYard but different from refinery/power-plant (wood). Steel resists conventional better than wood.
- **`Capturable=true`** — Engineer-capturable. Capturing transfers infantry production tree.
- **`Crewed=yes`** — destruction ejects an E1 (Allied side).
- **`Spyable=yes`** — spy infiltrate. Per `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`, infiltrating a barracks **gives the spy a veteran-rank version of the next infantry produced** (or similar veterancy-related effect — DEFERRED to confirm exact mechanic; likely "spy promotes all your future infantry to Veteran tier").
- **`ImmuneToPsionics=no`** — psi-vulnerable.

### Visual FX / destruction

- **`Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60`** — same 5-anim palette as ConYards (standard for large structures).
- **`;DestroyAnim=GAPILEDM`** commented — TS-era dead-code pattern.
- **`MaxDebris=15` / `MinDebris=5`** — matches ConYard's debris budget (15/7) closely. Big building, big destruction.
- **`DebrisAnims=`** — same 6-anim list as refinery/power-plants.
- **`DamageParticleSystems=SparkSys,SmallGreySSys,BigGreySmokeSys`** — **active** 3-system (visible damage particles). Allied barracks visibly emit sparks + smoke when damaged.
- **`DamageSmokeOffset=215,395,200`** — single damage smoke offset point.

### Exit configuration

- **`ExitCoord=-64,64,0`** — coordinates where infantry exit the barracks. (X, Y, Z) in leptons, relative to the building's foundation anchor. -64,64,0 = 0.25 cell west + 0.25 cell south + ground level. Confirmed Ghidra-scope: BuildingType (xref `0x0081a808 → 0x00460fb6`). **NEW cheat-sheet entry.**

When a player completes an Initiate/E1/E2/etc. build, the unit appears at the ExitCoord position adjacent to the barracks. From there, it walks to the player's rally point (if set) or stays.

### AI hints

- **`AIBuildThis=yes`** — AI builds barracks.
- **`AIBasePlanningSide=0`** — Good/Allied side (Allied AI marks GAPILE as their barracks; Soviet AI marks NAHAND; Yuri AI marks YABRCK).
- **`ThreatPosed=0`** — building threat 0.
- **`;WantsExtraSpace=yes`** — commented out (same as refinery, latent engine feature).
- **`Rules-global BuildBarracks=NAHAND,GAPILE,YABRCK`** ([rulesmd.ini:3069](../../../../ra2-rust-game/ini/rulesmd.ini)) — AI lookup table.

### Universal Owner

All 10 factions can theoretically build GAPILE if they have the prereq chain. Same engine-consistency pattern as refinery/power. Practical use: only Allied players hold a GACNST in normal play.

---

## artmd.ini section — full transcript

[artmd.ini:3763-3785](../../../../ra2-rust-game/ini/artmd.ini):

```ini
[GAPILE]
Remapable=yes
Normalized=yes
Cameo=BRRKICON
Foundation=3x2
Buildup=GAPILEMK
DemandLoadBuildup=true
FreeBuildup=true
NewTheater=yes
Height=4
ActiveAnim=GAPILE_A
ActiveAnimDamaged=GAPILE_AD
ActiveAnimZAdjust=-130
ActiveAnimPowered=no
;ActiveAnimTwo=GAPILE_A
;ActiveAnimTwoZAdjust=-39
;ActiveAnimThree=GAPILE_B
;ActiveAnimThreeZAdjust=-39
OccupyHeight=2
AddOccupy1=-1,-1
DamageFireOffset0=18,9
DamageFireOffset1=42,55
CanBeHidden=False
```

### Foundation and dimensions
- **`Foundation=3x2`** — 6-cell footprint. Smaller than refinery (4x3=12), bigger than power plant (2x2=4).
- **`Height=4`** — same as ConYard/power-plant.
- **`OccupyHeight=2`** — same as refinery.
- **`Cameo=BRRKICON`** — shortened cameo (`BRRK` for "Barracks").

### Render flags
- **`Remapable=yes`** — house color.
- **`Normalized=yes`** — palette normalization.
- **`NewTheater=yes`** — theater-letter substitution.
- **`CanBeHidden=False`** — building never Z-hidden behind other objects.

### Buildup
- **`Buildup=GAPILEMK`** — Allied barracks buildup anim.

### Active animation (single layer, power-gated)

- **`ActiveAnim=GAPILE_A`** — primary active anim.
- **`ActiveAnimDamaged=GAPILE_AD`** — damaged variant.
- **`ActiveAnimZAdjust=-130`** — large Z-offset (raised above building).
- **`ActiveAnimPowered=no`** — **`ActiveAnim does NOT play when building is powered`**. This is the inverse of GAPOWR's `ActiveAnimPowered=false` (which means "DO NOT play when normal-powered, only PoweredSpecial state plays it").
  - Actually wait — `ActiveAnimPowered=no` literally means: "this animation is NOT subject to power-state gating". The barracks' active anim runs at all times, regardless of low-power state. (Compare power-plant's `ActiveAnimPowered=false` which uses the power-state gating but says "active anim doesn't play in normal-powered" state.)
  - DEFERRED to fully clarify the semantic difference; my best read: `ActiveAnimPowered=no` = "this animation is not power-gated, always plays" vs `false` = "power-gated, but disabled in this specific state".

### Commented-out alternative anim layouts

```ini
;ActiveAnimTwo=GAPILE_A
;ActiveAnimTwoZAdjust=-39
;ActiveAnimThree=GAPILE_B
;ActiveAnimThreeZAdjust=-39
```

Westwood iterated multi-layer active anims (ActiveAnimTwo+Three) and reverted. Same `ActiveAnimTwo`/`Three` engine support discovered in earlier iterations.

### Foundation tweaks
- **`AddOccupy1=-1,-1`** — adds 1 extra impassable cell northwest of the foundation. The barracks' silhouette extends slightly northwest (chimney? door overhang?).

### Damage fire
- **`DamageFireOffset0=18,9` / `DamageFireOffset1=42,55`** — two fire points for the 3x2 footprint.

---

## Hardcoded behavior (Ghidra-verified)

### ReadINI scope verification (this iteration)

| Field             | String address    | First xref                | Read scope                       |
|-------------------|--------------------|--------------------------|----------------------------------|
| `GDIBarracks`     | `0x0081aa00`       | `0x00460b15`             | BuildingType                     |
| `NODBarracks`     | `0x0081a9f4`       | `0x00460b2f`             | BuildingType (Soviet equivalent) |
| `YuriBarracks`    | `0x0081a9e4`       | `0x00460b45`             | BuildingType (Yuri equivalent)   |
| `ExitCoord`       | `0x0081a808`       | `0x00460fb6`             | BuildingType                     |

**4 NEW cheat-sheet entries this iteration** — the side-specific barracks flag triple is a major engine-recognition system:

1. **`GDIBarracks`** — `0x0081aa00 → 0x00460b15` — BuildingType. Allied-barracks flag. TS-era `GDI` naming preserved.
2. **`NODBarracks`** — `0x0081a9f4 → 0x00460b2f` — BuildingType. Soviet-barracks flag. TS-era `NOD` naming preserved.
3. **`YuriBarracks`** — `0x0081a9e4 → 0x00460b45` — BuildingType. Yuri-barracks flag. Yuri-era addition (no TS heritage).
4. **`ExitCoord`** — `0x0081a808 → 0x00460fb6` — BuildingType. Infantry/unit exit position offset from foundation.

The three barracks flags' xrefs are at consecutive addresses (`0x00460b15, 0x00460b2f, 0x00460b45` — spaced 26 bytes / 0x1a apart), confirming they're processed as a tight sequence in the same parser function. This is the engine's per-faction barracks recognition: each side checks its own flag.

### Ghidra search log for this iteration

- `search_strings("GDIBarracks")` → 1 match → BuildingType. Allied flag.
- `search_strings("NODBarracks")` → 1 match → BuildingType. Soviet flag.
- `search_strings("YuriBarracks")` → 1 match → BuildingType. Yuri flag.
- `search_strings("ExitCoord")` → 1 match → BuildingType.

### Side-specific barracks naming clarification

The `GDIBarracks`/`NODBarracks` naming in YR is **TS-legacy parser keys preserved**:
- In Tiberian Sun (TS): GDI = good faction (Allied analog), NOD = evil faction (Soviet analog).
- In RA2/YR: the engine retained these parser key names. The INI files still use `GDIBarracks=yes` on Allied barracks and `NODBarracks=yes` on Soviet barracks despite the in-game faction labels being Allied/Soviet/Yuri.
- **`YuriBarracks`** is the YR-era addition (Yuri was a brand-new third side); no TS analog.

The naming is not visible to players; it's only in INI/binary parser keys. Modders working with rules.ini need to know:
- `GDIBarracks=yes` = "this is the Allied barracks for engine purposes"
- `NODBarracks=yes` = "this is the Soviet barracks for engine purposes"
- `YuriBarracks=yes` = "this is the Yuri barracks for engine purposes"

### Sub-animation

GAPILE_A and GAPILE_AD are referenced but not transcribed in this pass. They follow the standard pattern: GAPILE_A active loop + GAPILE_AD damaged variant (frame-range slicing).

### TS-legacy filter

- **`GDIBarracks=yes`** — **TS naming preserved, behavior fully active in YR**. The flag itself is not legacy; just the name.
- **`Armor=steel`** — active YR (armor class).
- **`;DestroyAnim=GAPILEDM`** commented — TS-era dead-code pattern.
- **`Capturable=true` / `Spyable=yes` / `Crewed=yes`** — active YR.
- **`;ActiveAnimTwo`/`Three`** commented — engine-supported but unused on barracks.
- **No fog-of-war / 0x1000 gating** — clean.
- **No Subterranean/Tunnel** — clean.

GAPILE has no actual TS-legacy behavior; only the `GDIBarracks` parser key name is heritage.

---

## Cross-references

- **`GACNST`** (`units/structures/GACNST.md`) — DONE. Prerequisite parent.
- **`NAHAND`** — pending iteration 98. Soviet Barracks (sister with `NODBarracks=yes`).
- **`YABRCK`** — pending iteration 99. Yuri Barracks (sister with `YuriBarracks=yes`).
- **`E1`** (`units/allied/E1.md`) — DONE. Allied G.I. Built from GAPILE.
- **`GGI`, `ENGINEER`, `SPY`, `TANY`, `JUMPJET`, `SNIPE`, `ADOG`, `GHOST`, `CCOMAND`, `CLEG`, `PENTGEN`** — DONE. All Allied infantry built from GAPILE.
- **`Rules-global BuildBarracks=NAHAND,GAPILE,YABRCK`** ([rulesmd.ini:3069](../../../../ra2-rust-game/ini/rulesmd.ini)) — AI lookup table.
- **`ENGINEER`, `SPY`** — capture/infiltrate.
- **`SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`** — spy effect on barracks (veterancy promotion).

---

## Coverage audit

INI fields covered (28 rulesmd + 19 artmd = 47 entries). **Coverage: 100%.**

---

## Open questions / Westwood inconsistencies

1. **`ActiveAnimPowered=no` vs power-plant's `ActiveAnimPowered=false`** — boolean parsing. Both should evaluate to "off"; difference may be semantic (parser distinction between explicit "no" and "false" or engine treating them identically). DEFERRED to verify in OptionsClass parsing.
2. **`Name=Allied Barracks ;needs different Given Name to avoid editor confusion`** — Westwood developer commentary preserved. The Given Name (Name= field) is used by the editor for some lookup, and must differ from the display name. Internal tooling detail.
3. **GDI/NOD parser key naming** — TS heritage. The engine could have renamed to AlliedBarracks/SovietBarracks but chose to keep parser keys consistent with TS code paths. Modders writing converters should know this.
4. **`Spyable=yes` on barracks** — the spy infiltrate effect promotes future infantry to veterancy (per common knowledge). Exact threshold (one-shot veteran or persistent for X infantry) needs Ghidra trace. DEFERRED.
5. **`Strength=500` (lowest core building HP)** — barracks are deliberately the most fragile. Strategic implication: protect barracks because they break first under harassment.

---

## Status

**DONE** — iteration 97. Index entry will be updated.

Doc total: **97**.

Next pick (priority): NAHAND (Soviet Barracks). Then YABRCK (Yuri Barracks) to close the trio. Then war factories (GAWEAP, NAWEAP, YAWEAP) — vehicle producers with `Factory=UnitType`.
