# LandType / SpeedType / Passability Tables — Substrate Study

**Date:** 2026-06-04
**Scope:** The three-table family that gates ground/naval movement in YR:
(1) `g_SpeedType_LandType_Table` (float[12 LandType × 9]) — per-cell terrain entry gate + speed multiplier;
(2) `g_PassabilityMatrix` (int[13 MovementZone × 8 ZoneType]) — macro zone-reachability gate;
(3) `CellClass::RecalcZoneType` (cell+0x4C) — the per-cell reduced-ZoneType column selector that feeds (2).
Plus the support tables: LandType / SpeedType name tables, `EdgeCostBaseTable`, and the parse-time constants.

**Authority order:** binary → Ghidra → docs. Every binary fact below cites the exact Ghidra MCP call used to
establish it *this session* or in cited stage-1 work. Confidence is per-claim.
**Burden of proof:** default verdict for any Rust↔gamemd difference is **DRIFT** unless proven bit-identical
across the input space (incl. boundaries). No INTERNAL-ONLY escape hatch for movement behavior.

**Top-line verdict:** the Rust port reproduces the **passability matrix** (table 2) bit-exactly, and the
**reduced-ZoneType derivation** (table 3) closely, but it does **NOT** reproduce gamemd's movement-cost model.
gamemd's A* edge cost is keyed *only* by the Can_Enter_Cell return code (0..7), never by terrain speed; the Rust
A* folds the per-terrain speed multiplier into the search weight (`base_cost * 100 / terrain_cost`). This is a
behavioral DRIFT that changes which route the planner picks across rough/ice/road terrain in every match.

---

## (1) Active-YR responsibilities

This family governs **whether and how fast any ground/naval unit can occupy a cell** — the most player-visible
movement constraint in a skirmish. Three distinct tables, three distinct jobs, all live in stock YR:

- **`g_SpeedType_LandType_Table` (float[12 LandType × 9]) @ 0x0089EA40** — terrain *speed multiplier* AND the hard
  *terrain-passability gate*. `0.0` = "this SpeedType physically cannot enter this terrain" (the wall that stops a
  tank entering deep water / impassable rock). `<1.0` slows the unit on that terrain. Player-visible: tanks
  refusing to drive into water/cliffs, infantry crawling on rough/tiberium, hovercraft skimming water at full
  speed, the cursor "no-move" over impassable terrain.
- **`g_PassabilityMatrix` (int[13 MovementZone × 8 ZoneType]) @ 0x0082A594** — the *zone-reachability / macro-routing*
  gate. Decides which reduced cell-class a given MovementZone can flood-fill through, building the zone graph that
  lets the engine *instantly reject* impossible orders (ship → land = no path, no search). Player-visible: ships
  confined to their water body, amphibious units crossing beaches, the instant "can't go there".
- **`CellClass::RecalcZoneType` (cell+0x4C)** — the per-cell *column selector* into the matrix, recomputed whenever
  a cell's overlay/terrain/building changes. Player-visible indirectly: build a wall → corridor blocks; sell a
  building → corridor reopens.

The float table and the matrix are **not redundant**: the float table is the per-cell entry gate keyed by
**SpeedType**; the matrix is the macro zone gate keyed by **MovementZone**. A unit carries both fields
independently (`SpeedType=` → `TechnoTypeClass+0x67C`; `MovementZone=` → `TechnoTypeClass+0x5B4`).

A fourth, distinct table — **`EdgeCostBaseTable` @ 0x0081870C** — is keyed by the Can_Enter_Cell *return code*,
NOT by terrain, and is the *only* per-step weight gamemd's A* applies. This is the crux of the cost-model DRIFT
in §4.

---

## (2) Full inventory (each fact Ghidra-cited)

### Static tables / globals

| Symbol | Address | Shape | Source-of-truth note |
|--------|---------|-------|----------------------|
| `g_SpeedType_LandType_Table` | 0x0089EA40 | float[12][9], row stride 0x24 | All-zero at static time (`read_memory 0x0089EA40 len 36` → all 0x00 this session); runtime-filled by rules parser. |
| `g_PassabilityMatrix` | 0x0082A594 | int32[13][8], 416 bytes | Static, baked. Dumped values below (stage-1 `read_memory 0x0082A594 len 416`). |
| LandType name table | 0x0081DA28 | ptr[12] | Clear,Road,Water,Rock,Wall,Tiberium,Beach,Rough,Ice,Railroad,Tunnel,Weeds. |
| LandType string table (parser input) | 0x00839D68 | ptr[12] | Byte-identical to 0x0081DA28 (stage-1). These ARE the `[Section]` names the parser reads. |
| `g_SpeedTypeNameTable` | 0x0081DA58 | ptr[8] | Foot,Track,Wheel,Hover,Winged,Float,Amphibious,FloatBeach. |
| `g_AStar_EdgeCost_BaseTable` (= `EdgeCostBaseTable`) | 0x0081870C | float[8] | Indexed by Can_Enter_Cell **return code**, NOT terrain. Re-verified this session (below). |
| Impassable sentinel | 0x007E1748 | 0.0f | Used by `== 0.0` impassable test. |
| Speed cap | 0x007E1718 | 1.0 (double) | Parser caps every INI speed at 1.0. Re-confirmed stage-1: 0x3FF0000000000000. |
| RecalcZoneType threshold | 0x007E3808 | 0.01 (double) | `<= 0.01` base-LandType impassable test. |

**`g_AStar_EdgeCost_BaseTable` @ 0x0081870C** — re-verified this session
(`read_memory 0x0081870C len 32` → bytes `00 00 80 3f | 00 00 7a 44 | 00 00 80 3f | 00 00 80 3f | 00 00 70 42 | 00 00 a0 41 | 00 00 00 41 | 00 40 1c 46`):
| Code | Meaning (Can_Enter_Cell) | Value |
|------|--------------------------|-------|
| 0 | Clear / passable | **1.0** |
| 1 | Crushable | **1000.0** |
| 2 | Temporary block (friendly moving) | **1.0** (then dynamic ×, see §5) |
| 3 | Scatter-required | **1.0** |
| 4 | Friendly wall | **60.0** |
| 5 | Occupied enemy | **20.0** |
| 6 | Friendly stationary | **8.0** |
| 7 | Impassable | **10000.0** |

The *only* xref to 0x0081870C is `AStar_compute_edge_cost @ 0x00429848`
(`get_xrefs_to 0x0081870C` this session → "From 00429848 in AStar_compute_edge_cost [DATA]"). So this table is
consulted exactly once, by the A* edge-cost function, indexed by return code — never by LandType, never by
SpeedType, never scaled by terrain speed.

**`g_PassabilityMatrix` @ 0x0082A594** (rows = MovementZone 0..12, cols = reduced ZoneType 0..7; only `1`=passable;
stage-1 `read_memory 0x0082A594 len 416`):

```
                       Gnd Crs Wal Bch Wtr Bld Imp Out
0  Normal               1   2   2   2   2   2   2   3
1  Crusher              1   1   2   2   2   2   2   3
2  Destroyer            1   1   1   2   2   2   2   3
3  AmphibiousDestroyer  1   1   1   1   1   1   2   3
4  AmphibiousCrusher    1   1   2   1   1   2   2   3
5  Amphibious           1   2   2   1   1   2   2   3
6  Subterranean         1   1   1   2   2   2   1   3
7  Infantry             1   2   2   2   2   1   2   3
8  InfantryDestroyer    1   1   1   2   2   1   2   3
9  Fly                  1   1   1   1   1   1   1   3
10 Water                2   2   2   2   1   2   2   3
11 WaterBeach           2   2   2   1   1   2   2   3
12 CrusherAll           1   1   1   2   2   2   2   3
```
Column 7 (Out / OOB sentinel) is `3` for all rows — universally impassable. Row 12 == Row 2 byte-for-byte.

### Initializer / parser / consumer functions (stage-1 cited)

- `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` — fills the float table; 12 LandType sections × 7 INI keys
  (Foot/Track/Wheel/Hover/Float/Amphibious/FloatBeach), caps each at 1.0, hardcodes Winged=1.0
  (`0x00674148`), reads Buildable bool into the 9th-slot low byte. Called by `RulesClass__Process @ 0x00668BF0`.
- `SpeedType__FromName @ 0x0048DFF0` — linear case-insensitive scan of the 8-entry name table, first match wins,
  `-1` on null/no-match. Stored to `TechnoTypeClass+0x67C`.
- `UnitClass__Can_Enter_Cell @ 0x0073F0A0` — primary float-table passability consumer:
  `(&table)[cell.LandType * 9 + TypeClass.SpeedType] == 0.0f → return 7`. Virtual at vtable+0x1AC.
- `CellClass__RecalcZoneType @ 0x00483C80` — writes `cell+0x4C` (0..7); reads float-table **Wheel column**
  (`FLD [0x89EA48 + LandType*36]`, col 2) for impassable tests. Reached via `RecalcAttributes @ 0x0047D2B0`
  (38 callers: map init, overlay/wall change, building place/sell, bridge destroy/repair).
- `CellClass__CheckCellPassability @ 0x004834A0` — float-table + zone-id consumer; `[SpeedType + LandType*9] == 0.0f`.
- `AStar_compute_edge_cost @ 0x00429830` — the A* per-step cost; re-decompiled this session (see §4). Takes the
  Can_Enter_Cell return code as `param_5`, indexes `EdgeCostBaseTable[code]`, then applies dynamic blocker/bridge
  multipliers. **No terrain-speed input.**
- Speed-multiplier consumers (NOT passability): `ShipLocomotionClass__Process_Movement @ 0x006A32F2`,
  `DriveLocomotionClass__Process_Movement @ 0x004B3CA3`, et al. — these read `[LandType*9 + SpeedType]` at
  runtime to scale the unit's actual movement speed.

### INI field mapping

- `MovementZone=` → `TechnoTypeClass+0x5B4` (matrix row, directly parsed; the "computed combination" claim in the
  TIMER_CLASSES doc is WRONG per stage-1).
- `SpeedType=` → `TechnoTypeClass+0x67C` (float-table column).
- Independent fields; neither is derived from the other.

### INI data (in-repo `ini/rulesmd.ini`, `[Clear]`..`[Tunnel]` @ L30191–30330, this session)

The 12 parsed sections and their per-SpeedType percentages (these are the *input* the parser caps at 1.0 and
writes into the float table). Note `[Ice]` and `[Weeds]` exist in INI; the verified parser section list also
includes `[Weeds]` (12th). `[Ice]` is present in INI but is LandType 8 in the name table:

| Section | Foot | Track | Wheel | Float | Hover | Amphibious | FloatBeach | Buildable |
|---------|------|-------|-------|-------|-------|------------|------------|-----------|
| Clear | 100 | 100 | 100 | 0 | 50 | 80 | 0 | yes |
| Rough | 100 | 100 | 100 | 0 | 50 | 80 | 0 | yes |
| Road | 100 | 100 | 100 | 0 | 75 | 100 | 0 | yes |
| Water | 0 | 0 | 0 | 100 | 100 | 100 | 100 | no |
| Rock | 0 | 0 | 0 | 0 | 0 | 0 | 0 | no |
| Wall | 0 | 0 | 0 | 0 | 0 | 0 | 0 | no |
| Tiberium | 90 | 70 | 50 | 0 | 50 | 50 | 0 | no |
| Weeds | 50 | 70 | 50 | 0 | 100 | 50 | 0 | no |
| Beach | 0 | 0 | 0 | 0 | 75 | 60 | 100 | no |
| Ice | 50 | 80 | 50 | 0 | 100 | 50 | 0 | no |
| Railroad | 90 | 100 | 50 | 0 | 100 | 50 | 0 | no |
| Tunnel | 100 | 100 | 100 | 0 | 100 | 100 | 0 | no |

**Note on Rough:** stock `[Rough]` is 100/100/100 — i.e. rough terrain does **not** slow tracked/wheeled units in
stock YR (the slowdown lives in different terrain like Tiberium/Ice). The Rust `COST_ROUGH=75` and the
`classify_terrain_cost` rough penalties are not drawn from the INI table and contradict it (see §4 DRIFT-R1).

---

## (3) Active vs legacy/dormant TS split

- **Float table (all 12 rows):** LIVE. Filled every rules load, read by every move attempt and zone recalc.
  Not flag-gated, reachable in skirmish, visible. **ACTIVE machinery.**
- **Dormant CONTENT rows:** LandType 5 (Tiberium) and 11 (Weeds) are Tiberium-economy terrain — RA2/YR uses ore
  *overlays*, not Tiberium terrain *cells*; rows parsed but rarely classify a real YR cell. LandType 10 (Tunnel)
  is subterranean terrain — TS-legacy (skip per project policy). A Rust replacement must still allocate/parse all
  12 rows (parser is hard-bounded to 12) but rows 5/10/11 are dormant content.
- **Matrix rows:** Row 6 (Subterranean) = TS-legacy MovementZone (subterranean not in YR) — row exists/read by the
  zone builder but no stock YR unit uses it. Row 9 (Fly) is conditionally active. Rows 0–5,7,8,10,11,12 = ACTIVE.
- **Winged column (SpeedType 4):** hardcoded 1.0, never read from INI (`0x00674148`). Present-but-constant; inert
  for the ground passability gate (aircraft don't use this gate).
- **No SpecialFlags / FogOfWar gating** on any of these reads — unconditional movement infrastructure.

---

## (4) Compare vs current Rust — table-by-table, helper-by-helper

### 4.1 Passability matrix — MATCH (proven)

`MOVEMENT_ZONE_PASSABILITY` (passability.rs:115–143) is **byte-for-byte identical** to the gamemd dump, including
the OOB sentinel column (`3`) and the Subterranean/Fly exceptions. Proven by the in-file test
`matrix_matches_verified_native_dump` (passability.rs:208–227) which asserts equality against an inlined copy of
the verified dump. **VERDICT: MATCH.** Row indexing (`MovementZone::matrix_row()` = direct `self as usize`,
locomotor_type.rs:303–309) matches gamemd's `matrix[MovementZone*8 + ZoneType]`. **No DRIFT.**

One residual nuance: passability.rs's module doc and the legacy `LandType` enum (passability.rs:40–49) keep an
8-bucket compatibility `LandType` (Clear,Road,Rough,Beach,Water,Tiberium,Railroad,Rock) that is NEITHER the 12-row
gamemd LandType nor the 8-column reduced ZoneType. `is_passable_for_speed_type` (passability.rs:173–179) indexes
the matrix with these buckets via `zone_layer_for_speed_type` — a Rust-side remap with no gamemd counterpart.
gamemd never indexes the matrix by SpeedType or by an 8-value LandType. This helper is a port-side invention used
only by fallback paths (terrain_cost.rs:77, cell_entry.rs:187). **VERDICT: DRIFT (structural)** — see DRIFT-M1; it
happens to produce plausible results for the common cases the callers hit, but the mapping
`zone_layer_for_speed_type` (Foot→2, Track→2, Wheel→1, Float/Hover/Winged→9, FloatBeach→4, Amphibious→3) is NOT a
gamemd table and is unproven across the input space.

### 4.2 Reduced-ZoneType derivation (RecalcZoneType) — CLOSE, two unproven thresholds

Rust derives `zone_type` in `resolved_terrain.rs:568–593` with a priority chain that tracks gamemd's
`RecalcZoneType @ 0x00483C80` decision tree well:

| gamemd step (0x00483C80) | gamemd result | Rust equivalent (resolved_terrain.rs) | Status |
|--------------------------|---------------|---------------------------------------|--------|
| overlay+0x22D (crushable) | 1 Road/Crushable | `overlay_effects.is_crushable → ROAD(1)` :568 | MATCH (semantic) |
| overlay+0x2A8 (wall) | 2 Wall | `is_wall → WALL(2)` :570 | MATCH |
| `floatTable[ovl.LandType*9 + 2] == 0.0f` | 6 Impassable | `overlay_land_wheel_speed_zero → IMPASSABLE(6)` :572 | MATCH (exact-zero, RESOLVED — see DRIFT-Z1) |
| overlay+0x2B5 (gate) | 6 Gate | `is_gate → IMPASSABLE(6)` :577 | MATCH |
| overlay+0x2B4 (rubble) | → default 0 | `is_rubble → GROUND(0)` :575 | MATCH |
| base LandType == 2 (Water) | 4 Water | `is_water → WATER(4)` :579 | MATCH |
| base LandType == 6 (Beach) | 3 Beach | `land_type == Beach → BEACH(3)` :581 | MATCH |
| `floatTable[LandType*9 + 2] <= 0.01` (double) | 6 Impassable | `wheel_speed_at_or_below_one_percent(<=1)` :585 | MATCH (algebraically) |
| object scan (building) | 5/6 | `terrain_object_blocks → BUILDING(5)` :589 | PARTIAL (object-list order untested) |
| default | 0 Ground | `GROUND(0)` :592 | MATCH |

- **DRIFT-Z1 (overlay impassable threshold) — RESOLVED to MATCH (adversarial re-check 2026-06-04):** gamemd's
  *overlay* path uses exact `== 0.0f` (single), so only a literal 0% Wheel speed marks the overlay impassable
  (re-verified: `decompile_function 0x00483C80` this session — overlay branch is
  `(float)(&DAT_0089ea48)[*(int *)(iVar3 + 0x298) * 9] == FLOAT_007e1748`, exact `== 0.0f` on the Wheel column
  +8/idx2). The Rust helper feeding step :572 is the bool field `overlay_effects.overlay_land_wheel_speed_zero`,
  sourced from `OverlayFlags.land_wheel_speed_zero` (overlay_types.rs:96,220–223), which is computed by
  `section_wheel_speed_is_exact_zero` (overlay_types.rs:410–416) — it parses the overlay Land= section's `Wheel`
  value and compares `== Some(0.0)`. **Exact-zero, NOT a `<= 1` reuse.** A 1% overlay Wheel speed does NOT
  classify Impassable on the overlay path. **VERDICT: MATCH.** (The base path at :585 correctly uses `<= 1`, which
  IS the integer-percent equivalent of gamemd's `<= 0.01` on a 0.0–1.0 float — those two agree, separately
  re-verified `decompile_function 0x00483C80`: base branch is `<= (float)_g_ImpassableSpeedThreshold_0_01`.)
- **Wheel column is correct:** Rust uses Wheel speed (`speed_costs.wheel`) for both impassable tests, matching the
  binary's col-2/+8-byte read. The prior NAVAL_ZONE doc's "col 0 / Foot" claim was WRONG; Rust does not replicate
  that error. **No DRIFT on column choice.**
- **PARTIAL (object-list order):** gamemd's step 5 walks the cell's FirstObject chain with type/owner conditions;
  Rust collapses this to a single `terrain_object_blocks` boolean. Object-list iteration order is gameplay-relevant
  per the function's own audit comment. **DRIFT-Z2 (object branch fidelity), low trigger frequency** — fires only
  when a cell holds a building/terrain object with the specific type-6/0x24 flags; most cells have none.

### 4.3 Float speed/passability table — NOT reproduced as a table; MULTIPLE DRIFTs

gamemd stores **one** runtime-filled `float[12][9]` table that serves BOTH the passability gate (`== 0.0`) AND the
runtime speed multiplier (`[LandType*9 + SpeedType]`, capped 1.0). Rust has **no equivalent single table.** It
splits the responsibility three ways, none of which is the gamemd table:

1. **`SpeedCostProfile` (terrain_rules.rs:37–78)** — per-section INI percentages (0–255, `Option<u8>`), parsed
   from the same `[Clear]`..`[Tunnel]` sections. This is the closest analogue to the gamemd float table's *source
   data*, but stored as integer percent, not the capped 0.0–1.0 float, and keyed by an 8-bucket LandType /
   section-name map (terrain_rules.rs:146–163), not the 12-row gamemd LandType.
2. **`TerrainCostGrid` (terrain_cost.rs)** — a per-SpeedType *integer cost grid* (0 / 75 / 100) used as A* search
   weight. gamemd has no per-terrain A* weight at all (see §4.4).
3. **`terrain_speed.rs::compute_cell_speed_modifier`** — the runtime per-cell speed multiplier, which IS the
   correct home for the gamemd float-table speed value, but it (a) applies extra slope/crowd factors gamemd
   applies elsewhere or not at all, and (b) clamps to [0.3, 1.2] rather than gamemd's [parsed, 1.0].

DRIFTs in this area:

- **DRIFT-F1 (no caps-1.0 float table; integer percent instead).** gamemd caps every parsed speed at exactly 1.0
  (double compare vs 0x007E1718) and stores a float. Rust stores raw INI percent in `Option<u8>` and only clamps
  to [0,255] at parse (terrain_rules.rs:353–357), then clamps to ≤100 at *use* (terrain_rules.rs:71). Boundary:
  an INI value of 120 → gamemd stores 1.0 → speed ×1.0; Rust stores 120 → at use clamps to 100 → ×1.0. Same
  output for the speed read, but the A* cost path (terrain_cost.rs:69, `cost_for_speed_type` returns the raw 120
  unclamped) carries an *uncapped* 120 into the cost grid, where `100/120` makes that cell *cheaper* than clear —
  a behavior gamemd cannot produce because gamemd has no such cost path. **VERDICT: DRIFT.**
- **DRIFT-F2 (0%→50% boost location).** gamemd's runtime speed read has no "0%→50%" rule in the float table — a
  0.0 entry means *impassable* (the gate returns code 7), not "move at 50%". Rust's
  `speed_multiplier_for` boosts `Some(0) → SIM_HALF` (terrain_rules.rs:70, terrain_speed.rs:22). For a cell that
  is *passable* but parsed 0% this would be a half-speed crawl; but in stock data 0% always co-occurs with the
  impassable gate, so the boost should never apply to a reachable cell. The rule is therefore either dead or, if
  it ever fires, a DRIFT (gamemd would have blocked the cell). **VERDICT: DRIFT (unproven-dead) — needs a caller
  trace proving no passable cell ever reaches `Some(0)`.**
- **DRIFT-F3 (passability gate uses `> 0`, gamemd uses `== 0.0`).** Rust `speed_type_allows_cell`
  (cell_entry.rs:183–188) returns `cost > 0` as passable. gamemd's gate is `!= 0.0` passable / `== 0.0` blocked.
  For integer percent these agree at the boundary (0 blocks, ≥1 passes). gamemd has no lower clamp, so a *negative*
  speed in INI would read as passable-but-negative in gamemd; Rust clamps negatives to 0 at parse
  (terrain_rules.rs:356, `clamp(0,255)`), turning a hypothetical negative into *blocked*. Stock INI has no
  negatives, so trigger frequency is zero in stock play, but it IS a divergence at the boundary. **VERDICT: DRIFT
  (boundary, zero stock frequency).**

### 4.4 A* edge cost — the headline DRIFT

**gamemd:** `AStar_compute_edge_cost @ 0x00429830` (re-decompiled this session) computes the per-step cost as
`EdgeCostBaseTable[return_code]`, then multiplies by dynamic blocker-chain / bridge factors. The base for code 0
(Clear) is **1.0 regardless of the cell's terrain speed.** A Grizzly crossing Ice (Track=80%), Road (100%), and
Clear (100%) pays the **same** A* step cost on each — terrain speed never enters the route search. (Verified:
the only data xref to `EdgeCostBaseTable` 0x0081870C is this function; `param_5` is the return code, not a speed.)

**Rust:** `astar` (core.rs:1263–1267) computes `step_cost = base_cost * 100 / terrain_cost`, where `terrain_cost`
is the per-SpeedType `TerrainCostGrid` value (terrain_cost.rs). So a cell at 75% costs `1000*100/75 = 1333` vs a
100% cell's `1000`. **The Rust planner actively routes around slow terrain; gamemd does not.** This changes the
chosen path whenever slow and fast terrain offer comparable-length routes — every match with rough/ice/tiberium.

- **DRIFT-A1 (terrain speed as A* weight).** Severity HIGH. Trigger frequency: every path search that crosses any
  cell whose speed multiplier ≠ 100% (Tiberium, Ice, Railroad-wheel, Beach for hover, etc.) — i.e. effectively
  every skirmish with ore fields or mixed terrain. Player-visible: units take visibly different routes than
  gamemd (detour around ore/ice the original drives straight through).
- **DRIFT-A2 (cliff multiplier).** Rust multiplies step cost ×4 on a height transition (core.rs:118,1270). gamemd's
  edge-cost function applies no generic height/cliff multiplier in `0x00429830`; its terrain "cost" comes from the
  return code only (impassable cliffs return code 7 = 10000.0; passable ramps return 0 = 1.0). **VERDICT: DRIFT** —
  the ×4 height penalty is a port invention. (Ramps gamemd treats as ordinary code-0 cells.)
- **MATCH (return-code multipliers).** Rust *does* correctly model the return-code side: `CODE5_MULT_ENEMY=20`,
  `CODE6_MULT_STATIONARY_ALLY=8`, code-2 chain walk with ×4 jam / ×1000 route-around, and the bridge flank
  multipliers (core.rs:122–147, 1278–1289) all match `EdgeCostBaseTable[4..6]` and the dynamic branches in
  `0x00429830`. **No DRIFT on the code-keyed costs.** The DRIFT is purely the *extra* terrain-speed factor layered
  on top. (Note: Rust's code-0/1/3/7 base is the implicit `STEP_COST=1000` / impassable-skip, which is the 1.0 /
  10000.0 equivalents scaled by 1000; code 1 Crushable=1000.0 maps to gamemd routing *around* crushables unless
  the mover is a crusher, which Rust handles via the crusher exemption at core.rs:1275.)

### 4.5 Name tables / enums — MATCH

- `SpeedType` enum (locomotor_type.rs:130–147) order Foot,Track,Wheel,Hover,Winged,Float,Amphibious,FloatBeach
  matches `g_SpeedTypeNameTable @ 0x0081DA58`. **MATCH.**
- `SpeedType::from_ini` (locomotor_type.rs:169) is case-insensitive first-match — matches `SpeedType__FromName`.
  One DRIFT: gamemd returns **-1** on no-match; Rust defaults to **Track** (locomotor_type.rs:179–182).
  **DRIFT-N1 (no-match fallback): low frequency** (only fires on a typo'd/modded `SpeedType=`); gamemd's -1 means
  "no terrain restriction" downstream, Track means "tank rules" — different behavior for a bad key.
- `MovementZone` enum + `from_ini` correctly preserves -1 (Invalid) on no-match (locomotor_type.rs:291–298),
  matching gamemd. **MATCH.**

---

## (5) Gamemd-native behavior contract (exact input→output a Rust port must reproduce)

**A. Float table indexing.** Element addr = `0x89EA40 + (LandType*9 + SpeedType)*4`. **Row stride = 9 floats;
LandType is OUTER, SpeedType INNER.** Impassable test = exact `== 0.0f` (not `<=`, not epsilon). Speed values
capped at 1.0 at parse (INI 1.0→1.0, 1.5→1.0, 0.0→0.0; negatives kept verbatim — no lower clamp). Winged column
(4) always 1.0. Parser iterates exactly 12 sections in fixed order
(Clear,Road,Water,Rock,Wall,Tiberium,Beach,Rough,Ice,Railroad,Tunnel,Weeds), 7 keys each; missing section → row
left at CCINI default 1.0. **DRIFT RISK:** any `[SpeedType*N + LandType]` indexing, stride ≠ 9, or `<=`/epsilon
gate is wrong.

**B. Passability matrix indexing.** Element = `matrix[MovementZone*8 + ZoneType]` int32; MovementZone (0..12) row,
reduced ZoneType (0..7) column. **Only `1` is passable; `2` and `3` both block.** Column 7 = `3` always. This is
NOT SpeedType×LandType.

**C. ZoneType derivation (cell+0x4C), first-match wins:** (1) not-in-playfield → 7; (2) overlay present:
crushable→1, wall→2, `floatTable[ovl.LandType*9 + 2] == 0.0f`→6, gate→6, rubble→default; (3) base LandType==2→4
(Water), ==6→3 (Beach); (4) `floatTable[LandType*9 + 2] <= 0.01` (double)→6; (5) object scan → 5/6; (6) default→0.
**Reads the Wheel column (index 2 / +8 bytes), NOT Foot. Two thresholds: overlay `== 0.0f` single, base `<= 0.01`
double.**

**D. SpeedType::FromName / MovementZone parse:** case-insensitive first-match → index; null/no-match → -1, stored
unclamped. -1 MovementZone = "no zone restriction" downstream.

**E. Reachability ordering (layered AND):** (1) zone-id equality precheck via matrix-built zone maps (instant
reject if src/dst zone differ); (2) per-cell `Can_Enter_Cell` (float-table `== 0.0` gate + occupancy + ownership)
→ code 0..7; (3) A* edge cost = `EdgeCostBaseTable[code]` × dynamic blocker/bridge factors — **terrain speed is
NOT a factor.** Terrain speed is applied separately at *runtime* movement by the locomotor reading
`floatTable[LandType*9 + SpeedType]`. Tie-break: Can_Enter_Cell scans the occupant list from head (cell+0xE4/0xE8)
— list order is gameplay-relevant.

---

## (6) Designed Rust-native substrate boundary

**One verified, pure, read-only, deterministic service:** `MovementTables` — the single owner of this family's
*static and rules-derived* data, with NO map/entity state. Lives at `src/rules/movement_tables.rs` (rules/ layer:
built from INI + baked constants; consumed by sim/ and never the reverse — respects the layering invariant since
rules/ has no sim/render/ui deps).

**Why rules/ not sim/:** the float speed table and the LandType↔SpeedType cost matrix are rules-data (parsed from
`rules(md).ini` + the baked passability matrix), exactly like `TerrainRules`/`SpeedType` already live in rules/.
The matrix is a compile-time constant; the speed table is INI-derived. sim/ holds the *cell state* (ResolvedTerrain
zone_type, occupancy) and *queries* the service. Keeping the tables in rules/ lets headless servers and the
zone-builder share one authority.

### API surface (signatures)

```text
// src/rules/movement_tables.rs   (rules/ layer; pure data; no &mut, no globals)

pub struct MovementTables {
    // Float[12 LandType][8 SpeedType] capped 0.0..=1.0, exact gamemd layout & cap.
    // Stored as SimFixed (fixed-point) for lockstep determinism — NOT f32/f64.
    speed: [[SimFixed; 8]; 12],
    // Buildable bit per LandType (9th slot low byte in gamemd).
    buildable: [bool; 12],
}

impl MovementTables {
    // Construction: INI-parsed for `speed`/`buildable` (12 sections × 7 keys,
    // Winged hardcoded 1.0, cap at 1.0); matrix is an associated const.
    pub fn from_ini(ini: &IniFile) -> Self;

    // (A) passability gate — exact `== 0.0` semantics, returns the gate boolean.
    pub fn is_terrain_passable(&self, land: LandType12, speed: SpeedType) -> bool;
    // (A) runtime speed multiplier — the capped fixed-point value (0.0..=1.0).
    pub fn speed_multiplier(&self, land: LandType12, speed: SpeedType) -> SimFixed;
    pub fn is_buildable(&self, land: LandType12) -> bool;

    // (B) macro zone gate — const matrix, `== 1` passable.
    pub const PASSABILITY: [[u8; 8]; 13]; // = the verified dump
    pub fn zone_passable(mz: MovementZone, zt: ReducedZoneType) -> bool;

    // (E) A* edge base cost — keyed by Can_Enter_Cell return code ONLY.
    pub const EDGE_COST_BASE: [SimFixed; 8]; // 1,1000,1,1,60,20,8,10000
    pub fn edge_base_cost(code: u8) -> SimFixed;
}

// New canonical 12-row LandType (replaces the 8-bucket compat enum):
pub enum LandType12 { Clear, Road, Water, Rock, Wall, Tiberium, Beach,
                      Rough, Ice, Railroad, Tunnel, Weeds }  // exact gamemd order
pub enum ReducedZoneType { Ground, Crushable, Wall, Beach, Water, Building,
                           Impassable, Outside }  // the 8 matrix columns
```

**Data ownership:** `MovementTables` is built once at rules load and stored in the rules bundle; `World`/sim holds
a `&MovementTables` (or `Arc`) for queries. The reduced-ZoneType *derivation* stays in sim/map (it needs per-cell
overlay/object state) but calls `MovementTables::is_terrain_passable(.., Wheel)` for its two impassable tests
instead of the bespoke `wheel_speed_at_or_below_one_percent` helpers.

**Construction source:** matrix + edge-cost = baked `const` (from the verified dumps, with a startup test asserting
equality to the embedded gamemd bytes). Speed/buildable = INI-parsed (12 sections, fixed order, cap 1.0). No
map-derived state in this service.

**Determinism:** all values `SimFixed` (no f32/f64 in sim math); tables immutable after construction; `const`
matrix iteration is fixed-order. Pure functions, no interior mutability, no RNG, no globals — replay/lockstep-safe.

**Crucial behavioral correction the boundary enforces:** the A* search consumes ONLY `edge_base_cost(code)` (×
dynamic blocker/bridge factors), and the **runtime locomotor** consumes `speed_multiplier(land, speed)`. The
service makes it structurally impossible to feed terrain speed into the A* weight (no `TerrainCostGrid`-as-A*-weight
API), retiring DRIFT-A1 by construction.

---

## (7) Retire list (what the new service replaces)

| Ad-hoc / duplicated / approximated item | Location | Why retired |
|------------------------------------------|----------|-------------|
| 8-bucket compat `LandType` enum | passability.rs:40–49 | Replaced by canonical 12-row `LandType12`. Not gamemd's 12 LandTypes nor the 8 reduced ZoneTypes. |
| `tmp_terrain_to_land_type` (8-bucket) | passability.rs:80–93 | Folds into LandType12 mapping; keep TMP→LandType but to the 12-row enum. |
| `zone_layer_for_speed_type` (SpeedType→matrix row) | passability.rs:149–160 | Port-invented remap with no gamemd table; matrix is keyed by MovementZone only. |
| `is_passable_for_speed_type` | passability.rs:173–179 | Bypasses both gamemd gates; replace callers with `is_terrain_passable` (float gate) + `zone_passable` (matrix). |
| `TerrainCostGrid` as **A* weight** | terrain_cost.rs (whole) | The `100/cost` A* weighting is DRIFT-A1; A* must use `edge_base_cost(code)` only. Grid may survive ONLY as a binary passable/blocked mask if needed, never as a cost. |
| `COST_ROUGH=75` + `classify_terrain_cost` rough penalties | terrain_cost.rs:23,122–172 | Not from INI ([Rough]=100/100/100); fabricated penalties (90/75/60). DRIFT-R1. |
| `CLIFF_COST_MULTIPLIER` ×4 | core.rs:118,1270–1272 | Port-invented height penalty; gamemd has no generic cliff cost in the edge function. DRIFT-A2. |
| `base_cost * 100 / terrain_cost` step | core.rs:1263–1267 | The terrain-speed-as-search-weight DRIFT. Replace with `edge_base_cost(code)`. |
| `wheel_speed_at_or_below_one_percent` / `overlay_land_wheel_speed_zero` | resolved_terrain.rs:1570; overlay path :572 | Replace with `MovementTables::is_terrain_passable(land, Wheel)` honoring the two distinct thresholds (overlay `==0`, base `<=0.01`). |
| `SpeedCostProfile` integer-percent storage | terrain_rules.rs:37–78 | Source data folds into `MovementTables.speed` (capped SimFixed). Profile may remain a thin INI-parse intermediate. |
| `MOVEMENT_ZONE_PASSABILITY` const + dup test copy | passability.rs:115–143, 208–222 | Moves to `MovementTables::PASSABILITY`; single source, single equality test. |
| Speed-cap-at-use clamp | terrain_rules.rs:71 | Cap must move to parse-time (gamemd caps once, stores 1.0) so the cost path can't see uncapped values. DRIFT-F1. |
| `0% → 50%` speed boost | terrain_rules.rs:70; terrain_speed.rs:22 | Not a gamemd float-table rule; 0.0 means impassable. DRIFT-F2. |

**Explicit duplications:** the verified 13×8 matrix is currently written **twice** (passability.rs:115 and the
inlined test copy at :208) — consolidate to one. The per-section speed data is read in **two** parallel shapes
(`SpeedCostProfile` for runtime speed AND `TerrainCostGrid` for A* weight) — the A* shape is the one to delete.

---

## (8) Migration slices + acceptance tests

Ordered, each independently shippable. Pure-data-parity slices first (no behavior change risk), then the one
genuinely behavioral slice (A* cost model), gated behind exact-output tests.

**Slice 1 (pure data) — Stand up `MovementTables` with baked matrix + edge-cost consts.**
- Add `MovementTables::PASSABILITY` and `EDGE_COST_BASE` consts; move the matrix off passability.rs.
- *Acceptance:* `test_matrix_bit_identical` — assert `PASSABILITY` equals the 13×8 dump byte-for-byte (all 104
  cells, including col-7 sentinel = 3 and row12==row2). `test_edge_cost_bit_identical` — assert
  `EDGE_COST_BASE == [1.0,1000.0,1.0,1.0,60.0,20.0,8.0,10000.0]` (all 8 codes, exact). Input space: full table.

**Slice 2 (pure data) — INI-parse `speed[12][8]` + `buildable[12]` with gamemd cap/order.**
- Parse the 12 sections in fixed order; cap at 1.0 at parse; Winged hardcoded 1.0.
- *Acceptance:* `test_speed_table_matches_ini` — for all 12 LandTypes × 8 SpeedTypes, assert the parsed capped
  value equals the stock `rulesmd.ini` percentage/100 capped to 1.0 (use the §2 table as the oracle; e.g.
  Tiberium×Track = 0.70, Water×Track = 0.0, Road×Amphibious = 1.0, Winged column all = 1.0). Boundaries:
  an injected INI value of 0% → 0.0, 100% → 1.0, 150% → 1.0 (capped), missing section → 1.0 default, negative →
  kept verbatim (document the gamemd no-lower-clamp; decide explicitly whether to match or guard).
- *Acceptance:* `test_buildable_from_ini` — Buildable bit per LandType matches stock (Clear/Road/Rough = true,
  rest = false).

**Slice 3 (pure data) — Replace passability helpers with the two gamemd gates.**
- `is_terrain_passable(land,speed)` = `speed_multiplier == 0.0` → false (exact-zero gate); `zone_passable(mz,zt)`
  = `PASSABILITY[mz][zt] == 1`.
- *Acceptance:* `test_terrain_gate_exact_zero` — `is_terrain_passable` is true for any value ≥ smallest non-zero
  fixed step and false ONLY for exactly 0.0 (boundary: a 1% entry passes). `test_zone_gate_only_one_passes` — for
  all 13×8, `zone_passable` is true iff the dumped value is 1 (2 and 3 both block). Retire
  `is_passable_for_speed_type` / `zone_layer_for_speed_type`; assert no remaining callers.

**Slice 4 (pure data) — Route RecalcZoneType impassable tests through `MovementTables`.**
- Base path uses `is_terrain_passable(land, Wheel)`'s underlying `<= 0.01`-equivalent; overlay path uses exact
  `== 0.0` Wheel. (DRIFT-Z1 RESOLVED 2026-06-04: the current overlay helper `section_wheel_speed_is_exact_zero`
  is already exact-zero — no fix needed, only carry the exact-zero semantics into the consolidated API + lock it
  with the regression test below.)
- *Acceptance:* `test_recalc_zonetype_wheel_thresholds` — drive the §5-C decision tree on a fixture covering each
  branch: overlay-crushable→1, overlay-wall→2, overlay-Wheel-0%→6, overlay-Wheel-1%→NOT 6 (proves overlay exact-
  zero), base-Water→4, base-Beach→3, base-Wheel-0%→6, base-Wheel-1%→6 (proves base `<=1`), base-Wheel-2%→NOT 6,
  default→0. Expected outputs are the gamemd ZoneType codes. This is the boundary test that separates the two
  thresholds.

**Slice 5 (BEHAVIORAL — the headline fix) — A* edge cost keyed by return code only.**
- Replace `base_cost * 100 / terrain_cost` (core.rs:1266) and the ×4 cliff multiplier with
  `STEP_COST * edge_base_cost(code)` (scaled to the integer fixed point), keeping the existing correct dynamic
  code-2/5/6 + bridge multipliers. Delete `TerrainCostGrid` as an A* input; A* sees only Can_Enter_Cell codes.
  Terrain speed continues to be applied at runtime by `terrain_speed.rs` (unchanged passable/blocked-only feed).
- *Acceptance (exact-output route tests):*
  - `test_astar_ignores_terrain_speed` — on a fixture with two equal-length routes, one over a 70% Tiberium strip
    and one over 100% Clear, the chosen path and its total cost are **identical** to the all-Clear baseline (i.e.
    terrain speed did not shift the route). Expected: gamemd picks the geometrically-tie-broken route, not the
    "faster terrain" one.
  - `test_astar_edge_cost_table` — assert per-step cost for each Can_Enter_Cell code equals
    `STEP_COST * EDGE_COST_BASE[code]` for codes 0,3,7 and the documented dynamic results for 1/2/4/5/6 (crusher
    exempt for 1; ×20 enemy, ×8 stationary, code-2 chain ×4/×1000). Input space: all 8 codes + crusher/non-crusher
    + the three code-2 urgencies.
  - `test_no_cliff_cost_penalty` — a passable ramp cell costs the same as flat clear (proves DRIFT-A2 retired).
- *Risk note:* this is the only slice that changes observable routing; ship behind the exact-output tests above
  and a before/after route diff on a stock map. Keep `TerrainCostGrid` deletion in the same slice so no caller can
  re-introduce the speed weight.

**Pure-data vs stateful split:** Slices 1–4 are pure-data parity (no routing change; only data-source consolidation
and threshold correctness) and can ship in any order after Slice 1. Slice 5 is the single stateful/behavioral
change and must land last, gated by exact-output route tests.

---

## Anchors & Evidence

| Address / symbol | Ghidra call cited (session unless noted) | Doc cross-ref |
|------------------|------------------------------------------|---------------|
| 0x00429830 `AStar_compute_edge_cost` | `decompile_function 0x00429830` (this session) | §4.4, §5-E |
| 0x0081870C `EdgeCostBaseTable` values | `read_memory 0x0081870C len 32` (this session) | §2, §4.4 |
| 0x0081870C single xref = 0x00429848 | `get_xrefs_to 0x0081870C` (this session) | §2, §4.4 |
| 0x0089EA40 float table (all-zero static) | `read_memory 0x0089EA40 len 36` (this session) | §2, §4.3 |
| 0x0082A594 matrix 13×8 | `read_memory 0x0082A594 len 416` (stage-1) | §2, §4.1 |
| 0x007E1718 = 1.0 cap | `read_memory 0x007E1718 len 8` (stage-1) | §5-A |
| 0x007E1748 = 0.0 sentinel | `read_memory 0x007E1748` (stage-1) | §5-A |
| 0x007E3808 = 0.01 RecalcZone threshold | `read_memory 0x007E3808 len 8` (stage-1) | §5-C |
| 0x00674000 `ReadSpeedTypeLandTypeTable` | `disassemble_function 0x00674000` (stage-1) | §2 |
| 0x0048DFF0 `SpeedType__FromName` | `decompile_function 0x0048DFF0` (stage-1) | §4.5 |
| 0x0073F0A0 `Can_Enter_Cell` | `decompile_function 0x0073F0A0` (stage-1) | §5-A |
| 0x00483C80 `RecalcZoneType` | `decompile_function`/`disassemble_function 0x00483C80` (stage-1) | §4.2, §5-C |
| 0x0081DA28 / 0x0081DA58 / 0x00839D68 name tables | `read_memory` (stage-1) | §2 |
| ini/rulesmd.ini [Clear]..[Tunnel] L30191–30330 | Read (this session) | §2 |

**Unverified-by-me this session (high-confidence, doc/stage-1-sourced — treat as DRIFT until bit-tested):** the
four matrix-reader bodies (Zone_precheck/UpdateBridgeZonesHelper/FloodFillReachableZones/
FindBestCompatibleMovementZone) internal logic; the `Can_Reach_Zone`/`GetZoneID` two-level indirection;
the runtime ShipLoco/DriveLoco speed-read stride (cited from stage-1 disassembly, not re-run here). The Rust
`overlay_land_wheel_speed_zero` helper body **WAS read in the 2026-06-04 adversarial re-check** (overlay_types.rs:410–416,
`section_wheel_speed_is_exact_zero` = `Wheel == Some(0.0)`) — **DRIFT-Z1 is now RESOLVED to MATCH** (exact-zero, not
`<=1`). See the Verification Log.

---

## DRIFT Ledger

| Rust file:line | Current | gamemd-correct | Severity + trigger-frequency |
|----------------|---------|----------------|------------------------------|
| core.rs:1263–1267 | `step_cost = base_cost*100/terrain_cost` (terrain speed weights A*) | A* step = `EdgeCostBaseTable[code]` only; terrain speed never weights search | **HIGH** — fires on every path search crossing any ≠100% cell (ore/ice/tiberium); units detour where gamemd drives straight. |
| core.rs:118,1270–1272 | `CLIFF_COST_MULTIPLIER` ×4 on height change | no generic height cost in edge function; ramps are code-0 (1.0), cliffs are code-7 (impassable) | **MEDIUM** — fires on any path near ramps/level changes; alters cost near hills every such map. |
| terrain_cost.rs:23,122–172 | `COST_ROUGH=75` + fabricated rough penalties (90/75/60) | stock `[Rough]`=100/100/100; no rough slowdown | **MEDIUM** — whole `TerrainCostGrid` is the A* weight being retired; fabricated values diverge wherever they apply. |
| terrain_rules.rs:71 | speed cap applied at *use* (`min(100)`) | cap applied once at parse (store ≤1.0 float) | **MEDIUM** — cost path reads uncapped >100% (terrain_cost.rs:69) making >100% cells cheaper than clear; fires only for modded >100% INI, but unbounded when it does. |
| terrain_rules.rs:70; terrain_speed.rs:22 | `Some(0) → 50%` speed boost | 0.0 = impassable (gate returns code 7); no boost | **LOW** — should be dead (0% co-occurs with the impassable gate in stock data); if ever reached on a passable cell it is a half-speed DRIFT. Needs caller trace to confirm dead. |
| cell_entry.rs:183–188 | gate uses `cost > 0` (and clamps neg→blocked at parse) | gate uses `!= 0.0`; no lower clamp (neg kept passable) | **LOW** — boundary-only; zero frequency in stock INI (no negatives), diverges only for negative/modded values. |
| passability.rs:40–49,80–93,149–179 | 8-bucket `LandType` + `zone_layer_for_speed_type` + `is_passable_for_speed_type` | matrix keyed by MovementZone only; gamemd LandType has 12 rows; no SpeedType→row table | **MEDIUM (structural)** — the remap is port-invented and unproven across inputs; fires whenever fallback paths (terrain_cost.rs:77, cell_entry.rs:187) hit a cell without INI speed data. |
| resolved_terrain.rs:572 (overlay path) | `overlay_land_wheel_speed_zero` (bool from `section_wheel_speed_is_exact_zero`, overlay_types.rs:410–416, `== Some(0.0)`) | overlay impassable test = exact `== 0.0f` (single) | **RESOLVED → MATCH (2026-06-04)** — helper is exact-zero, NOT `<=1`; a 1% overlay Wheel speed does NOT classify Impassable. No DRIFT. |
| resolved_terrain.rs:589 | `terrain_object_blocks → BUILDING(5)` (single bool) | object-list scan with type-6/0x24 + owner conditions → 5/6, list-order-sensitive | **LOW** — fires only on cells holding the specific building/terrain objects; list order untested. |
| locomotor_type.rs:179–182 | `SpeedType::from_ini` no-match → Track | no-match → -1 (no terrain restriction downstream) | **LOW** — fires only on a typo'd/modded `SpeedType=`; Track ≠ "unrestricted". |

**No-DRIFT (proven MATCH):** `MOVEMENT_ZONE_PASSABILITY` (passability.rs:115–143) — bit-identical to dump,
test-proven; `MovementZone::matrix_row` direct index; SpeedType/MovementZone enum order + name tables; the
return-code-keyed A* multipliers (CODE5=20, CODE6=8, code-2 ×4/×1000, bridge flanks); RecalcZoneType Wheel-column
choice and the base-path `<= 1` integer-percent threshold (algebraically equal to gamemd `<= 0.01` on 0.0–1.0);
and (RESOLVED 2026-06-04) the RecalcZoneType **overlay** impassable test — Rust `section_wheel_speed_is_exact_zero`
(`== Some(0.0)`) matches gamemd's exact `== 0.0f` overlay branch (formerly DRIFT-Z1).

---

## Verification Log (adversarial re-check, 2026-06-04)

Method: assume each load-bearing claim is wrong; re-verify LIVE in Ghidra this session; default DRIFT/UNVERIFIED
if not proven. Read-only Ghidra MCP + Rust source reads. No cargo, no emulate_function.

| # | Claim re-checked | Verdict | Evidence (Ghidra MCP call / source, this session) |
|---|------------------|---------|---------------------------------------------------|
| 1 | `EdgeCostBaseTable @ 0x0081870C` = float[8] [1.0,1000.0,1.0,1.0,60.0,20.0,8.0,10000.0] | **VERIFIED** | `read_memory 0x0081870C len 32` → `00 00 80 3f / 00 00 7a 44 / 00 00 80 3f / 00 00 80 3f / 00 00 70 42 / 00 00 a0 41 / 00 00 00 41 / 00 40 1c 46` = exactly those 8 floats. |
| 2 | Single xref to 0x0081870C = `AStar_compute_edge_cost @ 0x00429848` (table is consulted once, by code) | **VERIFIED** | `get_xrefs_to 0x0081870C` → "From 00429848 in AStar_compute_edge_cost [DATA]" (sole xref). |
| 3 | A* edge cost is keyed ONLY by the Can_Enter_Cell return code; terrain speed never enters the search (DRIFT-A1 basis) | **VERIFIED** | `decompile_function 0x00429830` → `param_5 = *(float*)(&g_AStar_EdgeCost_BaseTable + (int)param_5*4)`; only further modifiers are code-2 blocker prediction (×4.0/×1000.0), bridge-approach ×4.0 (`0x140 & 0x40000`), diagonal-bridge ×10.0/×2.0. No terrain/LandType/SpeedType input. param_5-in = code (`== 2.8026e-45` = float bits of int 2). |
| 4 | A* edge-cost function has a single caller (consumed only by the search) | **VERIFIED** | `get_function_callers 0x00429830` → `AStar_main_loop @ 00429a90` (sole caller). |
| 5 | `g_PassabilityMatrix @ 0x0082A594` = the 13×8 int32 dump in §2 (incl. col-7 sentinel = 3, row12 == row2) | **VERIFIED** | `read_memory 0x0082A594 len 416` → row0 `1,2,2,2,2,2,2,3`; row12 `1,1,1,2,2,2,2,3` (== row2 Destroyer); col 7 = 3 every row. Byte-for-byte matches §2. |
| 6 | Float table `0x0089EA40` all-zero at static time (runtime-filled) | **VERIFIED** | `read_memory 0x0089EA40 len 36` → all `00`. |
| 7 | Speed cap constant `0x007E1718` = 1.0 (double) | **VERIFIED** | `read_memory 0x007E1718 len 8` → `00 00 00 00 00 00 f0 3f` = 0x3FF0000000000000 = 1.0. |
| 8 | RecalcZone base threshold `0x007E3808` = 0.01 (double) | **VERIFIED** | `read_memory 0x007E3808 len 8` → `7b 14 ae 47 e1 7a 84 3f` = 0x3F847AE147AE147B = 0.01. |
| 9 | Impassable sentinel `0x007E1748` = 0.0f | **VERIFIED** | `read_memory 0x007E1748 len 4` → `00 00 00 00`. |
| 10 | `RecalcZoneType @ 0x00483C80` reads the **Wheel column** (idx 2 / +8 bytes), stride 9; overlay test = exact `== 0.0f`, base test = `<= 0.01` | **VERIFIED** | `decompile_function 0x00483C80` → both reads use `(&DAT_0089ea48)[LandType*9]` (0x0089EA48 = base+8 = Wheel col). Overlay: `== FLOAT_007e1748` (exact 0.0). Base: `<= _g_ImpassableSpeedThreshold_0_01`. Branch order matches §5-C (OOB→7, crushable→1, wall→2, ovl-Wheel==0→6, gate→6, rubble→default, Water→4, Beach→3, base-Wheel<=0.01→6, object 6/0x24→5/6, default→0). |
| 11 | `SpeedType__FromName @ 0x0048DFF0` = case-insensitive first-match over 8-entry table, -1 on null/no-match | **VERIFIED** | `decompile_function 0x0048DFF0` → loop over `&g_SpeedTypeNameTable` while `< 0x81da78` (8 ptrs), returns match index, else -1; null param → -1. |
| 12 | `ReadSpeedTypeLandTypeTable @ 0x00674000` iterates exactly 12 sections, stride 9 floats, caps each at 1.0, hardcodes Winged=1.0 at 0x00674148, Buildable into 9th slot | **VERIFIED** | `disassemble_function 0x00674000` → `MOV EBX,0x89ea44`; per-key `FCOMP [0x007e1718]` cap; `MOV [EBX+0xc],0x3f800000` at **0x00674148** (= idx4 Winged = 1.0f); `MOV [EBX+0x1c],AL` Buildable; `ADD EBX,0x24` (36 = 9 floats); `CMP EBX,0x89ebf4 / JL` ⇒ (0x89EBF4−0x89EA44)/36 = 12 iterations. |
| 13 | Parser reads section names from `g_LandTypeStringTable @ 0x00839D68`; LandType/parser order = Clear,Road,Water,Rock,... | **VERIFIED** | `disassemble_function 0x00674000` → `MOV ESI,0x839d68`. `read_memory 0x00839D68` ptr[0..3] → 0x0081DC1C="Clear", 0x0081DC14="Road", 0x0081BAE8="Water", 0x0081DC0C="Rock" (confirms §5-A parser order, distinct from §2 INI-file listing order). |
| 14 | Column layout idx0=Foot…idx7=FloatBeach, idx8=Buildable (vs name table) | **VERIFIED** | `read_memory 0x0081dbd4` → "Foot" (the `pfVar4[-1]`/idx0 key); decompile write offsets map idx0 Foot, idx1 Track, idx2 Wheel, idx3 Hover, idx4 Winged(1.0), idx5 Float, idx6 Amphibious, idx7 FloatBeach, idx8 Buildable. |
| 15 | Stock INI `[Rough]` = 100/100/100 Foot/Track/Wheel (DRIFT-R1 basis) + full §2 table | **VERIFIED** | `ini/rulesmd.ini` L30212–30220 `[Rough]` Foot=100% Track=100% Wheel=100%; all 12 sections (L30191–30331) match the §2 per-SpeedType table exactly. |
| 16 | DRIFT-Z1: Rust `overlay_land_wheel_speed_zero` is exact-zero (matches gamemd overlay `== 0.0f`), NOT a `<=1` reuse | **WRONG (claim was DRIFT; now MATCH)** — corrected in §4.2, ledger, §6, footer | Rust `section_wheel_speed_is_exact_zero` (overlay_types.rs:410–416) parses overlay Land=`Wheel` and tests `== Some(0.0)`; field flows overlay_types.rs:220–247 → resolved_terrain.rs:1505/1533 → :572. Exact-zero confirmed; the doc's "DRIFT until proven exact-zero" assumed-wrong default is refuted. |

**Net:** 14 VERIFIED, 1 WRONG-corrected (DRIFT-Z1 → MATCH), 0 UNVERIFIABLE among the claims sampled this session.

**Stage-2 recommendation impact:** the only invalidated item is **DRIFT-Z1**. Slice 4's acceptance sub-case
"overlay-Wheel-1%→NOT 6 (proves overlay exact-zero)" is **already satisfied by current code** — Slice 4 still
has value (routing both impassable tests through one `MovementTables` API + the base-path threshold check), but
the "first read the overlay helper body and fix DRIFT-Z1 if it is not exact-zero" instruction is now moot: no fix
needed, only a regression test. All other slices (1,2,3,5) and the headline DRIFT-A1/A2/R1/F1 findings stand
fully re-verified.
