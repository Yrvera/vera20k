# Pathfinding Helpers — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics.
**Scope:** the *standalone support helpers* the A* search depends on — **not** the A* core itself
(Euclidean-heuristic / dual-closed-list spine, well-implemented in `core.rs` — heuristic VERIFIED matching, Pass 2) and **not** the
`Can_Enter_Cell` cell-validation predicates (a **sibling study**; this doc references the seam
but does not own it). The helpers in scope: `CellClass::RecalcZoneType` (zone-type assignment),
the zone-passability matrix consumption, the A* **edge-cost** helper (`AStar_compute_edge_cost`,
entity-aware via the `Can_Enter_Cell` return-code table + code-2 prediction), the **level-0 marker
gate** in `AStar_main_loop`, `Zone_Estimate_Slope_Cost` (level-aware zone slope cost), the
`UpdateBridgePassability` **alt-object fallback** scan (`FUN_0042B080` = `FindNearbyBridgePeer`),
and the zone-corridor / expand-corridor handoff.

**Provenance / confidence posture:** This is a SYNTHESIS of an already-decoded corpus (the
pathfinding research family is among the most thoroughly verified in `docs/research/`). The
following load-bearing facts were **live-verified this session** via Ghidra MCP:
- `CellClass__RecalcZoneType` is the function at `0x00483c80` (verified via `get_function_by_address 0x00483c80`).
- `AStar_compute_edge_cost @ 0x00429830`, `AStar_main_loop @ 0x00429a90`, `Zone_precheck @ 0x0042c290`,
  `Zone_Estimate_Slope_Cost @ 0x00585f40` — all function identities confirmed (`get_function_by_address` each).
- `FUN_0042B080` carries the live Ghidra label `PathfinderClass__FindNearbyBridgePeer` (verified via `get_function_by_address 0x0042b080`).
- The 8-entry A* base cost table at `0x0081870c` reads bit-exact `{1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0}`
  (verified via `read_memory 0x0081870c` len 32: `0000803f 00007a44 0000803f 0000803f 00007042 0000a041 00000041 00401c46`).
- The 9-entry direction tiebreaker table at `0x0081872c` reads bit-exact
  `{0.001, 0.005, 0.002, 0.006, 0.003, 0.007, 0.004, 0.008, 0.0}` (verified via `read_memory 0x0081872c` len 36).

Everything else is **doc-sourced** from the cited reports (their own confidence is `ghidra/verified`
but was **not** re-decompiled this session); each such fact is tagged `(doc-confidence: <FILE>)`.
**Default verdict for any unproven equivalence is DRIFT** — there is no internal-only escape hatch for
active gameplay/path-shape/ordering. The §9 ledger separates verified-this-session from doc-only.

**Companion:** the in-flight engine-substrate program. Master TODO:
`docs/plans/2026-05-29-core-engine-substrate-todo.md`. **Item #7 (Map/cell substrate)** overlaps this
study directly — the helpers here *consume* the cell substrate (occupancy lists, blocker-neighbor
refcount, zone records, bridge flags). This study **slots into** that program; it does not invent a
parallel architecture. Precedent format mirrored: `FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.

---

## Executive Summary

**Verdict: the A* *spine* is in good shape, but every *helper* that shapes the path is an
approximation, and three of them produce player-visible path-shape drift today.** The Rust port
faithfully reproduces the entity-aware edge-cost table values (codes 0–7, the code-2 10-hop prediction
walk, the direction tiebreaker ratio) — those numerics are close. (The ×4 multiplier value is right but
its TRIGGER is wrong — see H4 correction below.) The gaps are:
(1) **zone slope cost is entirely absent** — `Zone_Estimate_Slope_Cost` is not ported, so on sloped
maps Rust's coarse corridor picks routes the original would deprioritize; (2) the **hierarchical
marker gate** is replaced by a `expand_corridor()` one-ring widening that is explicitly *not* a
binary behavior and lacks the `CellClass+0x122` blocker-neighbor off-marker exception, so corridor
pruning differs; (3) the **`FindNearbyBridgePeer` 5×5 fallback** and the bridge-flank diagonal cost
multipliers are decoded-but-inert (`#[allow(dead_code)]`), so bridge-approach steering is missing.
Smaller drifts: the corridor edge cost uses **centroid Manhattan with a distance pull**, where the
binary's `Zone_precheck` corridor is a **pure uniform-cost Dijkstra with NO heuristic** (zone base-cost
table `0x007e3794` + slope + 0.001 diagonal step); `RecalcZoneType`'s priority cascade is replaced by a
passability-matrix lookup that may misclassify building/railroad edge cells. **NOT a drift (corrected
Pass 2, 2026-06-04):** Rust's cell-A* **Euclidean** heuristic (`euclidean_heuristic`) MATCHES the binary —
`AStar_create_node` computes `Sqrt_Approx(dx²+dy²)`, genuinely Euclidean; the earlier "corpus describes
octile" framing was the error. The cliff ×4 trigger IS a drift, however: the binary's `0x40000` flag is a
dynamically-toggled bridge-peer marker (set by `UpdateBridgePassability`), not the inter-cell height delta
Rust uses. The proposed
replacement is an additive, shadow-first **pathfinding-support service** (`PathSupport`) that owns
the four helper families (zone-type classification, edge-cost evaluation, slope-cost estimation,
bridge-peer fallback) as pure functions over a borrowed *cell substrate* (the sibling study's
`Can_Enter_Cell` + occupancy surfaces), wired into A* through the existing `AStarOptions` seam.
Rollout follows the proven Mission/Radio rhythm — shadow → invert hash-invariant → drop shadow
asserts → authoritative → `SNAPSHOT_VERSION` bump → parity harness — gated by a P0 research
checkpoint that must resolve the still-deferred slope-context (`Foot+0x21C` `+0x57E4`/`+0x59F0`)
writer lifecycle before any slope math becomes authoritative.

---

## Table of Contents

- §1. Verified active-YR responsibilities of the helper family
- §2. Full inventory (functions, globals, tables, vtable seams, TS-legacy paths)
- §3. Active-YR vs inactive/legacy (TS) split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements H1–H18)
- §6. Rust-native replacement boundary (`PathSupport` service)
- §7. Old ad hoc Rust logic to retire/fold
- §8. Migration slices + acceptance tests (P0–P7)
- §9. Sources & Verification Ledger
- §10. Pass 2 — Expansion (2026-06-04): new consumers, globals, slots, edge cases, cross-family folds

---

## 1. Verified active-YR responsibilities of the helper family

This is what the helper family **owns** in a normal YR skirmish — the player-observable contract a
Rust replacement must reproduce. Each row is the *behavior*, not the C++ structure.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **Per-cell zone-type classification** (`cell+0x4C`, values 0–7): the column index into the passability matrix, assigned by a priority cascade over overlay flags, terrain Wheel-speed, LandType, terrain-objects, and buildings. Recomputed on every terrain/overlay change. | YES | `CellClass__RecalcZoneType @ 0x00483c80` (live-verified identity this session); cascade doc-sourced (`PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` §2.5) |
| R2 | **A* edge cost** for one neighbor expansion: looks up the `Can_Enter_Cell` return code (0–7) in the base cost table, applies the code-2 friendly-mover prediction (1.0/4.0/1000.0), the cliff-ramp ×4 multiplier (`cell+0x140 & 0x40000`), and the bridge-diagonal flank multipliers (1.0/2.0/10.0). | YES | `AStar_compute_edge_cost @ 0x00429830` (live id); table `0x0081870c` (live bytes); `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` §§3–4 |
| R3 | **A* g-cost assembly**: `edge_cost × PathfinderClass+0x04 (cost_multiplier) + DirectionEpsilon[dir]`, where the epsilon table breaks ties with a slight cardinal preference. | YES | `0x0081872c` (live bytes: cardinals 0.001–0.004 < diagonals 0.005–0.008); `PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md` §5 (doc) |
| R4 | **Hierarchical level-0 marker gate**: in `AStar_main_loop`, a candidate neighbor's level-0 zone is checked against `PathfinderClass+0x40` (chosen-zone marker array stamped by `Zone_precheck`) *before* the `Can_Enter_Cell` call; off-marker cells are pruned **unless** `CellClass+0x122 != 0` (8-neighbor blocker refcount). | YES | `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md` §3 (doc); `AStar_main_loop @ 0x00429a90` & `Zone_precheck @ 0x0042c290` (live ids) |
| R5 | **Zone-level slope cost**: `Zone_precheck` adds `ftol(Zone_Estimate_Slope_Cost(ctx, level, cur, next) × mover_factor)` to candidate edge cost, gated on `mover_factor > ~1e-5`; level 0 → 0, level 1 → neighbor-rep lookup, level 2 → directional corner-min half-sum. | YES (when mover factor > threshold) | `Zone_Estimate_Slope_Cost @ 0x00585f40` (live id); formula `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md` §3 (doc) |
| R6 | **Bridge-peer fallback scan** (`FindNearbyBridgePeer`): when `UpdateBridgePassability` selects a probe cell whose object list is null, scan a 5×5 square, choose each candidate's ground/bridge list by `(structural bridge bit) && abs(level − requested_height) > 2`, return the first object whose attached subobject footprint accepts the original center point at the requested height. | YES / Conditional | `FindNearbyBridgePeer @ 0x0042b080` (live label this session); `PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md` (doc) |
| R7 | **Neighbor-cell utilities**: `Pathfinding_update_continued` (get adjacent cell by dir 0–7 via `g_DirectionOffsets`); `Path_walk_directions_to_cell` (replay a direction-step array to a destination cell, with dir-8 tube/tunnel teleport). Used to replay an existing partial path for A* continuation. | YES | `PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` §§2.2–2.3 (doc); `g_DirectionOffsets @ 0x0089f688` |
| R8 | **Zone-corridor handoff**: `Zone_precheck` runs a Dijkstra over the hierarchical zone graph (slope-aware edge cost), stores the chosen chain (`Pathfinder+0xBC`, count `+0xC74`) and stamps the marker array consumed by R4. Same-zone precheck failure clears hierarchy and still runs cell A*; cross-zone hierarchy failure aborts before cell A*. | YES | `ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md` §3.4 (doc) |

These helpers chain as: terrain change → `RecalcAttributes` → `RecalcZoneType` (R1) establishes
`cell+0x4C`. Path request → `Run_AStar` → `Path_walk_directions_to_cell` (R7) replays partial path
→ `AStar_pathfind_search` → `Zone_precheck` (R8: corridor Dijkstra + slope R5 + marker stamp) →
`AStar_main_loop` (R4 marker gate per neighbor → `Can_Enter_Cell` → R2 edge cost → R3 g-cost).

---

## 2. Full inventory

### 2a. Helper functions (with addresses)

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| `CellClass__RecalcZoneType` | `0x00483c80` | Assign `cell+0x4C` zone type 0–7 via priority cascade | YES | **live id this session** (`get_function_by_address`) |
| `CellClass::RecalcAttributes` | `0x0047d2b0` | Master cell recalc; calls LAT fixup + RecalcZoneType + CliffBackImpassability | YES | doc (`PATHFINDING_STANDALONE_FUNCTIONS` §2.6) |
| `CellClass__ApplyLAT_and_SlopeFixup` | `0x0047ca80` | LAT terrain-blend auto-transition + slope/ramp sub-variant fixup (visual + slope index) | YES (runtime terrain mods only) | doc (§2.1) |
| `Pathfinding_update_continued` (get-neighbor) | `0x00481810` | `cell + dir(0–7)` → neighbor `CellClass*` via `g_DirectionOffsets`; dir≥8 returns self | YES (47+ callers) | doc (§2.2) |
| `Path_walk_directions_to_cell` | `0x00429780` | Replay direction-step array → destination cell; dir-8 = tube teleport via `g_TubeArray[idx]+0x28` | YES | doc (§2.3) |
| `AStar_compute_edge_cost` | `0x00429830` | Per-neighbor edge cost: table lookup + code-2 prediction + cliff/bridge multipliers | YES | **live id this session** |
| `AStar_main_loop` | `0x00429a90` | Core A* loop; hosts R4 marker gate + `Can_Enter_Cell` dispatch + R3 g-cost assembly | YES | **live id this session** |
| `Zone_precheck` | `0x0042c290` | Hierarchical corridor Dijkstra; stamps marker array; calls slope helper | YES | **live id this session** |
| `Zone_Estimate_Slope_Cost` | `0x00585f40` | Level-aware (0/1/2) zone-edge slope estimate (int) | YES (gated) | **live id this session** |
| `PathfinderClass__FindNearbyBridgePeer` (`FUN_0042B080`) | `0x0042b080` | 5×5 first-eligible-object fallback for `UpdateBridgePassability` empty probe list | YES / Conditional | **live label this session** |
| `PathfinderClass::UpdateBridgePassability` | `0x0042acf0` | Sole caller of R6; writes the `0x40000` peer-path marker layer | YES / Conditional | doc (`PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080`) |
| `FootClass::Run_AStar` | `0x004cbba0` | A* wrapper; replays partial path then searches | YES | doc |
| `AStar_pathfind_search` | `0x0042c900` | Hierarchy wrapper: same-zone/cross-zone precheck dispatch | YES | doc |
| `Math__ftol` | `0x007c5f00` | Truncate-toward-zero float→int (control word `0x0E7F`), used to fold slope cost | YES | doc (`ZONE_ESTIMATE_SLOPE_COST_PARITY` §3.2) |
| `FootClass__Get_Slope_Speed_Factor` | `0x004dc760` | Returns `Foot+0x530` (or `1.0` via linked-object exemption) = slope mover factor | YES | doc (`ZONE_ESTIMATE_SLOPE_COST_PARITY` §3.6) |

### 2b. Static tables & globals (load-bearing)

| Item | Address | Shape | Verdict / evidence |
|---|---|---|---|
| A* base cost table | `0x0081870c` | 8 × f32 = `{1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0}` | **live bytes this session** — bit-exact |
| Direction tiebreaker | `0x0081872c` | 9 × f32 = `{.001,.005,.002,.006,.003,.007,.004,.008, 0.0}` | **live bytes this session** — bit-exact |
| Cliff-ramp multiplier | `0x007e37bc` | f32 `4.0` | doc (`PATHFINDING_STANDALONE_FUNCTIONS` §3) |
| Bridge flank: both | `0x007e37b4` | f32 `2.0` | doc |
| Bridge flank: one | `0x007e2ac8` | f32 `1.0` | doc |
| Bridge flank: neither (non-bridge diagonal) | `0x007e37b8` | f32 `10.0` | doc |
| CellArray neighbor offsets (8 dir, map width 512) | `0x007e3774` | 8 × i32 `{-512,-511,+1,+513,+512,+511,-1,-513}` | doc |
| `g_DirectionOffsets` | `0x0089f688` | 8 × (i16 dx, i16 dy) | doc |
| Bridge flanking cell-offset tables | `0x007e3710` / `0x007e3730` | per-bridge-flag selector | doc (geometry MEDIUM confidence) |
| Dir-from-delta table | `0x007e3760` | maps `(dy*3+dx)` → dir index | doc |
| Slope mover-factor threshold | `0x007e3810` | f32 raw bytes `f1 68 e3 88` (= a near-zero/tiny-magnitude float, NOT `~1e-5`) | **live bytes this session** (`read_memory 0x007e3810`); the gate is `threshold < Get_Slope_Speed_Factor` in `Zone_precheck` (`decompile_function 0x0042c290`). Operative semantics: any positive mover factor passes. The "~1e-5" figure in the slope-parity doc is a mischaracterization — verify the exact intended constant before any authoritative slope flip. |
| Slope corner offset table | `0x00abd460` | 4 × (i16 dx, i16 dy) | doc |
| Slope dir-corner tables | `0x0082a984` / `0x0082a9c4` | 2 corners/dir (src/dst) | doc |
| Level-1 zone graph | `DAT_0087f890` | stride `0x24`, rep at `+0x20` | doc |
| Level-2 zone graph | `DAT_0087f8a8` | stride `0x24`, rep at `+0x20` | doc |
| Zone-index table | `DAT_0087f858` | 10-byte per-cell tuple; word0 = level-0 zone id | doc |
| Passability matrix | `0x0082a594` | MovementZone rows × 8 zone columns; **row stride = 0x20 (32 bytes = 8 × i32), entries are i32 (compared `== 1`)** | doc (`ZONE_PASSABILITY_VERIFIED.md`); stride/element-size VERIFIED 2026-06-04 (disassemble_function 0x0042c290: `SHL EAX,0x5; ADD EAX,0x82a594` then `CMP [ECX+EDX*4],1`) |
| **Zone-corridor base-cost table** | `0x007e3794` | 8 × f32 = `{1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0}` (per zone-type 0–7) | **VERIFIED 2026-06-04** (read_memory 0x007e3794 → `0000803f 00000000 00000000 0000803f 0000803f 00000000 0000803f 0000803f`). Distinct from the cell-A* table `0x0081870c`. Indexed by `iVar18` (zone passability code) in `Zone_precheck`. NEW — doc previously omitted. |
| **Corridor diagonal-step cost** | `0x007e3818` | double `0.001` | **VERIFIED 2026-06-04** (read_memory 0x007e3818 → `fca9f1d24d62503f`). Added per diagonal zone step in `Zone_precheck`. NEW. |
| **Corridor cardinal-step cost** | `0x007e2800` | double `0.0` | **VERIFIED 2026-06-04** (read_memory 0x007e2800 → all zeros). Added per cardinal zone step. NEW. |
| **Cell-A* per-step re-relax cost** | `0x007e37c0` | double `1.0009...` (≈1.001) | **VERIFIED 2026-06-04** (read_memory 0x007e37c0 → `be9f1a2fdd24f03f`). `parent_g + 1.001` in `AStar_main_loop` node re-relaxation. NEW — the cell-A* uniform step cost. |
| Wheel-speed terrain table | `0x0089ea48` | `LandType*36` stride, Wheel column | doc (`PATHFINDING_STANDALONE_FUNCTIONS` §2.5, corrected 2026-06-01) |

### 2c. Cell / pathfinder struct fields the helpers read (NOT owned here — see seam §6.0)

| Owner | Offset | Field | Used by |
|---|---|---|---|
| CellClass | `+0x4C` | ZoneType (0–7) | R1 writes, R2/R4 read |
| CellClass | `+0x122` | 8-neighbor blocker refcount | R4 off-marker exception |
| CellClass | `+0x124` | bridge-peer-marker eligibility gate (NEW 2026-06-04) | `UpdateBridgePassability` 5×5 `0x40000` XOR loop (decompile_function 0x0042acf0) — only cells with `+0x124 != 0` get the cliff-ramp flag toggled |
| CellClass | `+0x140` | Flags (`0x100` bridge, `0x40000` **bridge-peer-path / approach marker — dynamically XOR-toggled by `UpdateBridgePassability`, NOT static cliff**, `0x800` bridge flank-table selector) | R2 (H4 ×4), R6 |
| CellClass | `+0xE4` / `+0xE8` | ground / bridge object-list head | R2 code-2 walk, R6 scan |
| CellClass | `+0x11B` | cell level | R6, height compare |
| CellClass | `+0x116` | TubeIndex (-1 = none) | R7 dir-8 |
| PathfinderClass | `+0x04` | cost_multiplier (R3) | R3 |
| PathfinderClass | `+0x3C` | urgency (0/1/2) | R2 code-2 |
| PathfinderClass | `+0x40` | level-0 chosen-zone marker array | R4 |
| PathfinderClass | `+0x28` | epoch/stamp | R4, R8 |
| PathfinderClass | `+0xBC` / `+0xC74` | chosen chain / count | R8 (retry/diag) |
| FootClass | `+0x530` | slope mover factor (← `TechnoTypeClass+0x2F0` `ThreatAvoidanceCoefficient`) | R5 |
| FootClass | `+0x21C` | slope-cost context ptr (`+0x57E4` L1 array, `+0x59F0` 130-wide L2 grid) | R5 |
| object | `+0x14 bit 2` | active/eligible bit | R2, R6 |
| object | `+0x674` | attached subobject (vtable `+0xA0` = footprint accept predicate) | R6 |
| object | `+0x578` (double) | velocity (==0 → use path_queue dir) | R2 code-2 |
| object | `+0x5E0` | path_queue[0] direction | R2 code-2 |

### 2d. vtable / COM seams (referenced, owned by sibling study)

- `vtable+0x1AC` = `Can_Enter_Cell` (Unit `0x0073f0a0`, Infantry `0x0051bf90`) — returns the 0–7 code that R2's
  cost table is indexed by. **This is the seam with the sibling cell-validation study.** R2 consumes its return
  value; it does not define it.
- `vtable+0xA0` on `object+0x674` = footprint/point-accept predicate used by R6.
- `vtable+0x1B8` = `GetCellCoords` used by R2's code-2 walk.

---

## 3. Active-YR vs inactive/legacy (TS) split

| ACTIVE in standard YR skirmish | INACTIVE / LEGACY (TS) / dormant — do NOT design substrate around |
|---|---|
| R1 zone-type cascade (`cell+0x4C`), recomputed on terrain change. | **Tunnel / subterranean** (`feedback_no_tunnel_subterranean`). `Path_walk_directions_to_cell` dir-8 reads `g_TubeArray[cell+0x116]+0x28` — the **tube/tunnel teleport** path. RA2/YR maps have no subterranean units; skip the tube-teleport leg of R7 in the substrate. (Note: `CellClass::RecalcAttributes` LandType==10 "Tunnel detection" creating a TubeClass is the same TS-legacy family.) |
| R2 edge cost: full 0–7 table, code-2 prediction, cliff ×4, **bridge-diagonal flank** multipliers (bridges ARE live in YR). | **Fog-of-war darkening of `cell+0x122`** — the OLD doc text called `cell+0x122` "fog of war"; it is NOT. It is the 8-neighbor blocker refcount (verified, `CELL_0x122_CAN_ENTER_CELL_SEMANTIC`). Standard YR fog-darkening is off (`feedback`). Do not treat the R4 off-marker exception as fog. |
| R3 g-cost assembly (cost_mult + epsilon tiebreaker). | `Pathfinder+0x3C == 2` (destroyer) override of code-2 to 1000.0 — this IS live (urgency-2), not TS; listed here only to flag it must not be confused with a dead branch. |
| R4 marker gate **with** `cell+0x122` exception (hierarchy flag from `AStar_pathfind_search`). | The Rust `expand_corridor()` one-ring widening — explicitly **not** a binary behavior (doc OQ-7); it is a Rust approximation, the thing we are replacing, not a thing to preserve. |
| R5 slope cost — live when the mover's `ThreatAvoidanceCoefficient` (`Foot+0x530`) > ~1e-5. Stock harvester variants set it (1 or .65). | Default-constructor `Foot+0x530 == 0` for units not overriding `ThreatAvoidanceCoefficient` → slope contributes **zero** for those movers. (Not legacy — just means slope is mover-conditional; flat/no-factor synthetic tests may defer it, per doc.) |
| R6 `FindNearbyBridgePeer` — live when `Pathfinder+0x3C != 0` and probe list is null (bridges). | `FindNearbyBridgePeer`'s `object+0x674` subobject identity (vtable `+0xA0`) is UNRESOLVED in the corpus (deferred OQ-16). Not legacy; just unverified — flag, do not invent. |
| R7 get-neighbor (dir 0–7) + `g_DirectionOffsets`. | `CellClass__ApplyLAT_and_SlopeFixup` (R-adjacent): LAT terrain-blend is needed only for **runtime terrain modification** (bridge destruction/construction). Rust loads pre-resolved WAE maps, so the visual LAT pass is not needed for static maps — but the **slope-index fixup** it performs feeds R1/R5, so the slope-index output is in-scope even if the tile-blend output is not. |
| R8 zone-corridor Dijkstra (slope-aware), marker stamp, hierarchy fall-through. | — |

---

## 4. Comparison against the current Rust architecture

Files: `src/sim/pathfinding/{core,zone_search,passability,zone_build,terrain_cost,terrain_speed,cell_entry,zone_hierarchy,zone_incremental,zone_map}.rs`,
`src/sim/movement/{group_destination,movement_reservation}.rs`. Status legend: ✅ close, ⚠️ partial/approx, ❌ missing.

| Helper | Rust location | Status | Detail / drift |
|---|---|---|---|
| **A* spine** (Euclidean h / dual-closed-list, 24-step segments, MAX_SEARCH_NODES) | `core.rs::astar_search` (821), `MAX_SEARCH_NODES=65_527` (109), `MAX_PATH_SEGMENT_STEPS=24` (114) | ✅ | Dual ground/bridge closed lists, node cap matches `0xFFF7`. **CORRECTED 2026-06-04 — heuristic is NOT drift.** `AStar_create_node @ 0x0042a460` (decompile_function) computes `h = Sqrt_Approx(dx*dx + dy*dy)` over abs goal-deltas, where `Sqrt_Approx @ 0x004cac40` (decompile_function) is a genuine table-based square-root → the binary cell-A* heuristic is **Euclidean**. Rust `euclidean_heuristic` (core.rs:2079, `sqrt(dx²+dy²)×1000`) **MATCHES**. The old "corpus/brief describe octile" framing was the error, not the Rust. The per-step re-relax cost in `AStar_main_loop` is `parent_g + _DAT_007e37c0` where `_DAT_007e37c0 = 1.0009...` (read_memory 0x007e37c0 = `be 9f 1a 2f dd 24 f0 3f`); cell A* uses NO diagonal upcharge on the step itself (uniform), diagonal preference comes only from the DIR-tiebreaker epsilon. Verify Rust's STEP_COST scaling matches the 1.001 step constant before declaring full parity. |
| **R1 RecalcZoneType** | `zone_build.rs` (passability-matrix-based zone flood fill); no direct cascade | ⚠️ | Rust assigns zones via passability-matrix checks in `zone_build.rs`, **not** the binary's first-match priority cascade (overlay Crushable → Wall → Wheel-speed==0 → IsARock → Water → Beach → terrain-object → building). Edge cells (railroad, terrain-objects RTTI 0x24, gates) may be classified differently. Pre-resolved maps mask most of this, but runtime bridge destroy/build re-derives zones. |
| **R2 edge cost: table + code-2** | `core.rs`: `CODE5_MULT_ENEMY=20` (131), `CODE6_MULT_STATIONARY_ALLY=8` (134), `CODE2_MULT_*` (122–124), `compute_code2_multiplier` (2099), applied at core.rs:1274–1292 | ✅ (mostly) | Codes 2/5/6 and the 10-hop code-2 chain walk are ported and match the verified constants. **Mechanism drift (not value drift):** Rust precomputes a `LayeredEntityBlockMap` (denormalized blocker→next-cell) instead of calling `Can_Enter_Cell` per neighbor; codes 1 (crushable=1000) and 4 (occupied-friendly=60) are folded into hard `entity_blocks` BTreeSets rather than soft costs (core.rs:1184–1203). Per `ASTAR_ENTITY_COST_INTEGRATION` §7.4, codes 4/5/6 should be **soft** (path-through at high cost), enemies 20.0 fight-through — Rust hard-blocks code-4-equivalents. DRIFT: a unit boxed by stationary friendlies should still find a high-cost path-through, not "no path". |
| **R2 cliff ×4** | `core.rs::CLIFF_COST_MULTIPLIER=4` (118), applied at 1270–1272 | ⚠️ | Rust multiplies ×4 on **height change between cells** (`current.height != neighbor_height`); binary multiplies ×4 on the `cell+0x140 & 0x40000` **cliff-ramp flag**. Different trigger condition → different cells penalized. DRIFT. |
| **R2 bridge-flank multipliers** | `core.rs`: `BRIDGE_FLANK_*` (145–147), `bridge_flank_multiplier` (2147), `apply_bridge_flank_cost` (2158) — both `#[allow(dead_code)]` | ❌ | Numerics pinned (10/1/2) but **inert** — explicitly blocked "until `PathfinderClass+0x01` lifecycle is verified" (core.rs:143). Diagonal bridge crossings are not penalized. |
| **R3 g-cost: cost_mult + epsilon** | `core.rs::DIR_TIEBREAK` (371), `STEP_COST=1000` (103) | ⚠️ | `STEP_COST=1000` chosen so DIR_TIEBREAK sits at 0.001–0.008 of base — matches the binary ratio. **No `PathfinderClass+0x04` cost_multiplier** equivalent (R3's per-search speed factor). Currently A* has no per-mover cost scalar; uniform. DRIFT for movers where the binary scales all costs. |
| **R4 marker gate + cell+0x122** | `core.rs::HierarchyGate` (353), `BlockerNeighborCounts` (245), gate applied at core.rs:1208–1214; corridor fallback `zone_search.rs::expand_corridor` (794) | ⚠️ | Rust HAS a binary-style `HierarchyGate` + `BlockerNeighborCounts` (the off-marker `+0x122` exception is modeled!) AND still has the legacy `expand_corridor()` one-ring widening + `AStarOptions::corridor` BTreeSet path (core.rs:1216–1224). Two parallel mechanisms; the corridor-set path is the approximation to retire. |
| **R5 slope cost** | — | ❌ | **Entirely absent.** `zone_search.rs::find_zone_corridor` uses centroid Manhattan (zone_search.rs:677); no mover factor, no slope context, no `Zone_Estimate_Slope_Cost`. `terrain_speed.rs` has runtime `SlopeClimb`/`SlopeDescend` — that is movement *execution*, NOT the `Zone_precheck` slope-cost pipeline (explicitly distinct, doc Negative Fact). Biggest missing helper. |
| **R6 FindNearbyBridgePeer** | — | ❌ | No 5×5 first-object fallback. `cell_entry.rs` has `object_list_layer`/`occupancy_bits_layer` concepts (the input substrate) but no probe-fallback scan. |
| **R7 get-neighbor + walk-dirs** | `core.rs::NEIGHBORS` (388), `explicit_tube_edge` (677), `find_path` continuation | ✅ (sans tube) | 8-dir neighbor table present. Tube/tunnel dir-8 is modeled (`explicit_tube_edge`, core.rs:1330–1369) — but per §3 this is TS-legacy and should be inert in standard YR. |
| **R8 zone-corridor Dijkstra** | `zone_search.rs::find_zone_corridor` (626), `ZoneQueueEntry` insertion-order ties (599) | ⚠️ | Dijkstra present with stable insertion-order ties (matches the "adjacency discovery order, ZoneId not a tie key" contract). **DRIFT (now sharper):** the binary corridor is a **pure uniform-cost Dijkstra with NO distance heuristic** — `Zone_precheck @ 0x0042c290` (disassemble_function) accumulates `cost = zoneBaseCost[zoneType] + parent_g + ftol(slope) + diagStep` where (a) `zoneBaseCost` is the 8-entry table at `0x007e3794` = `{1.0,0.0,0.0,1.0,1.0,0.0,1.0,1.0}` (read_memory 0x007e3794, bit-exact; this is a DIFFERENT table from the cell-A* cost table at `0x0081870c`), (b) `diagStep` adds `_DAT_007e3818 = 0.001` for a diagonal zone step and `_DAT_007e2800 = 0.0` for cardinal (read_memory both, bit-exact), (c) slope is added when the gate passes. Rust's centroid-Manhattan edge cost is therefore drift on TWO axes: it injects a distance estimate the binary does not have AND omits the per-zone base cost + slope + 0.001 diagonal tiebreaker. No marker-stamp handoff (uses `expand_corridor` instead). Same-zone vs cross-zone hierarchy fall-through partially present (`can_reach_same_or_zoned`, `can_use_reduced_zone_precheck`). |
| **Terrain cost grid** | `terrain_cost.rs::TerrainCostGrid` (33), `cost_at` returns 0/60/75/90/100/120 | ⚠️ | Rust bakes a per-SpeedType `100/cost` step scalar (core.rs:1263–1267). The binary's cost table is **occupation-coded (0–7)**, not terrain-speed-coded — terrain speed enters via the *movement execution* speed, not A* edge cost in the same way. Two different cost models coexist; needs reconciliation (the binary A* cost is occupation+cliff+bridge, terrain speed is a separate locomotor concern). |

**Where logic is scattered ad hoc:** the A* cost shaping is spread across `core.rs` (codes 2/5/6,
cliff, marker, bridge-flank), `terrain_cost.rs` (per-SpeedType speed scalar — a *different* cost
axis), and `zone_search.rs` (corridor Dijkstra + expand). There is no single "edge cost" owner that
mirrors `AStar_compute_edge_cost`; the binary computes one cost per neighbor in one function, Rust
computes it in three places with two different cost philosophies (occupation-coded vs speed-scaled).

---

## 5. gamemd-native behavior contract (testable statements H1–H18)

The exact observable semantics any Rust replacement must reproduce. Each is independently testable.
Default verdict on any unproven Rust deviation = DRIFT.

**Edge cost (R2/R3):**

- **H1.** The A* base cost for a neighbor is `costtable[Can_Enter_Cell_code]` where the table is bit-exact
  `[0]=1.0, [1]=1000.0, [2]=1.0(base), [3]=1.0, [4]=60.0, [5]=20.0, [6]=8.0, [7]=10000.0`.
  Codes ≥7 reject the neighbor (not expanded). (live bytes `0x0081870c`)
- **H2.** Code 2 (moving friendly) cost is **dynamic**: urgency 0 → walk the first blocker's predicted trajectory
  up to **10** cells (read each blocker's velocity: if `vel==0` use `path_queue[0]` dir, else derive dir from rate
  timer `(RateTimer>>12 +1)>>1 & 7`); if an empty cell or a `path_queue[0]==-1` blocker is found → **1.0**; if an
  inactive object (bit-2 clear) or the full 10 hops without clearing → **4.0**. Urgency 1 → **4.0** flat (no walk).
  Urgency 2 → **1000.0** flat. (`ASTAR_ENTITY_COST_INTEGRATION` §4, doc)
- **H3.** Crusher units (`TechnoTypeClass Crusher`): any `Can_Enter_Cell` code `< 7` is forced to `0` before cost
  lookup → Crushers ignore all occupancy and only check terrain passability. (doc §2.2)
- **H4.** If `cell+0x140 & 0x40000` set, edge cost `×= 4.0` (decompile_function 0x00429830, `_g_BridgeApproach_CostMult_4_0`;
  table `0x007e37bc`). **CORRECTED 2026-06-04 — the `0x40000` flag is NOT static cliff terrain; it is a dynamically
  XOR-toggled bridge-peer-path marker.** `PathfinderClass__UpdateBridgePassability @ 0x0042acf0` (decompile_function)
  toggles `cell+0x140 ^= 0x40000` along the chosen bridge-peer's path-queue cells AND across a 5×5 block around the
  probe cell (gated by `cell+0x124 != 0`), then `AStar_main_loop` calls `UpdateBridgePassability` at search entry and
  exit. So the ×4 multiplier penalizes cells the bridge-passability pass marked as "approach/under a peer bridge,"
  not raw terrain cliffs. **Rust's CLIFF_COST_MULTIPLIER trigger (inter-cell height delta, core.rs:1270) is therefore
  doubly wrong:** wrong trigger condition AND missing the entire `UpdateBridgePassability` marker lifecycle that sets
  the flag. This makes H5/H4/R6 a single coupled bridge subsystem — they cannot be ported independently of
  `UpdateBridgePassability`.
- **H5.** Bridge diagonal: only when `bridge_flag != 0 && Pathfinder+0x01 != 0`; flanking-cell bridge bits select
  `×2.0` (both), `×1.0` (one), `×10.0` (neither). Flank-offset table chosen by `dest+0x140 & 0x800`. (doc)
- **H6.** Final g-cost per neighbor = `edge_cost × (search_ctx+0x04 cost_multiplier) + DirectionEpsilon[dir]`,
  epsilon bit-exact `[N=.001, NE=.005, E=.002, SE=.006, S=.003, SW=.007, W=.004, NW=.008, tube=0.0]` — cardinals
  strictly < their adjacent diagonals, so ties resolve to a cardinal-first expansion order. (live bytes `0x0081872c`,
  get_xrefs_to 0x0081872c → sole reader is `AStar_main_loop @ 0x00429f96`.) **Attribution corrected 2026-06-04:**
  the `+0x04` cost_multiplier and `+0x3c` urgency are fields of the **A* search-context `this`** (first arg of
  `AStar_main_loop`/`AStar_compute_edge_cost`), NOT a separate PathfinderClass — verified by the call-setup
  `MOV ECX,ESI` at `0x00429f88` (read_memory 0x00429f60) where ESI is the preserved search-context pointer. The
  g-cost FMUL is `fVar25 * *(float*)(this+4) + tiebreak[dir]` (decompile_function 0x00429a90).
- **H7.** Codes 4 (occupied-friendly 60.0), 5 (enemy 20.0), 6 (stationary-friendly 8.0) are **soft** — the cell is
  still expandable at that cost (path-through), NOT hard-blocked. Friendly stationary (8.0) is the cheapest detour;
  enemy (20.0) is fight-through-able; occupied-friendly (60.0) strongly avoided but reachable. **VERIFIED live
  2026-06-04** (decompile_function 0x00429a90): the Can_Enter_Cell return `iVar17` from `vtable+0x1ac` gates node
  creation with `if (iVar17 < 7) { ...AStar_create_node... }`; only `>= 7` rejects the neighbor. Codes 4/5/6 all
  pass the `< 7` test and are expanded at their table cost. The Rust hard-block of code-4/5/6-equivalents via
  `entity_blocks`/`bridge_blocks` BTreeSets (core.rs:1184–1203) is therefore a confirmed DRIFT.

**Zone-type classification (R1):**

- **H8.** `RecalcZoneType` assigns `cell+0x4C` by **first-match** priority: (1) out-of-bounds→7; (2) overlay inherited
  `Crushable=`(`+0x22D`)→1; (3) overlay `Wall=`(`+0x2A8`)→2; (4) overlay Wheel-speed==0 (table `0x0089ea48 +
  LandType*36`)→6; (5) overlay `IsARock=`(`+0x2B5`)→6; (6) LandType==2 Water→4; (7) LandType==6 Beach→3; (8) Wheel
  speed ≤ threshold→6; (9) terrain-object RTTI 0x24 (`+0x2A8`/`+0x2AC` vs 7)→2 or 5; (10) building wall/gate→6;
  (11) default→0. (doc §2.5, corrected 2026-06-01)

**Hierarchy / corridor (R4/R8):**

- **H9.** `Zone_precheck` stamps `Pathfinder+0x40+level*4` chosen-zone marker array with epoch `+0x28` at both
  endpoints and every chosen-chain zone; stores chain at `+0xBC+level*1000`, count at `+0xC74+level*4`. (doc)
- **H10.** `AStar_main_loop` gates each neighbor by its **level-0** zone (`DAT_0087f858` word0) vs
  `Pathfinder+0x40[level0_zone] == epoch`. If marked → normal cost path. (doc)
- **H11.** Off-marker neighbor (normal/near-height branch): accepted iff `cell+0x122 != 0` (8-neighbor blocker
  refcount); pruned only when `+0x122 == 0` AND hierarchy flag set. If hierarchy flag clear, off-marker zero-refcount
  cells are still accepted. (doc §3.3)
- **H12.** Same-zone `Zone_precheck` failure → clear hierarchy flag, still run cell A* (unrestricted). Cross-zone
  hierarchy enabled+failed → return no path **before** cell A*. (doc §3.4)
- **H13.** Corridor Dijkstra ties resolve by **adjacency-discovery (insertion) order**; `ZoneId` is not a tie key.
  (matches Rust `ZoneQueueEntry` sequence field; doc)

**Slope cost (R5):**

- **H14.** Slope contributes to a `Zone_precheck` candidate edge only when the mover object is non-null AND
  `FootClass__Get_Slope_Speed_Factor(obj) > threshold@0x007e3810` (raw bytes `f1 68 e3 88`, a tiny-magnitude
  float — NOT `~1e-5`; effectively "any positive factor passes"). Verified live this session: the gate is
  `(float10)_DAT_007e3810 < fVar27` and the context ptr is `*(param_5 + 0x21c)` passed to `Zone_Estimate_Slope_Cost`
  then `Math__ftol` (`decompile_function 0x0042c290`). The factor is `Foot+0x530` (= `TechnoTypeClass+0x2F0`
  `ThreatAvoidanceCoefficient`) unless a linked-object exemption returns 1.0. (doc)
- **H15.** Contribution = `Math__ftol(Zone_Estimate_Slope_Cost(ctx, level, cur, next) × factor)` added as integer-derived
  cost; `ftol` is truncate-toward-zero (≡ floor for non-negative). Level 0 → 0. Level 1 → `ctx[0x57E4/4 + neighbor_rep]`
  (ignores current zone). Level 2 → adjusted-coarse-coord directional two-corner **min** at each endpoint, then
  `(min_src + min_dst) >> 1` (arithmetic SAR). (doc §3.3–3.5)

**Neighbor utilities (R7):**

- **H16.** Neighbor lookup: dir 0–7 add `g_DirectionOffsets[dir]` (N=(0,−1), NE=(+1,−1), E=(+1,0), SE=(+1,+1),
  S=(0,+1), SW=(−1,+1), W=(−1,0), NW=(−1,−1)); CellArray offset = `dir_offset_table[dir]` with map-width 512. (doc)

**Bridge-peer fallback (R6):**

- **H17.** When `UpdateBridgePassability`'s selected probe list (`cell+0xE4`/`+0xE8`) is null, `FindNearbyBridgePeer`
  scans dx,dy ∈ [−2,+2] (inner = X), returning the **first** object (scan order, then `object+0x30` list order) whose
  `object+0x674` subobject vtable `+0xA0` accepts the **original center** point (`x*256+128, y*256+128, height×scale`),
  filtered by `object+0x14 bit 2`. Per-candidate list choice: bridge list iff structural-bridge `&& abs(candidate.level
  − requested_height) > 2` (note: **>2**, stricter than the caller's >3, and ignores `Foot+0x8C`). Returns null cleanly
  if all 25 cells exhaust. (doc; live label `FindNearbyBridgePeer`)
- **H18.** R6 builds/caches nothing — it returns one `ObjectClass*` or null; the caller resumes its peer scan from that
  object via `object+0x30`. (doc Negative Facts)

---

## 6. Rust-native replacement boundary — the `PathSupport` service

### 6.0 The seam with the sibling cell-validation study (explicit ownership split)

| Concern | Owner | Why |
|---|---|---|
| `Can_Enter_Cell` predicate → returns code 0–7 | **Sibling cell-validation study** | It is the cell/occupancy validator; produces the *input* code. |
| Occupancy lists (`cell+0xE4`/`+0xE8`), blocker-neighbor refcount (`cell+0x122`), cell flags, bridge state | **Map/cell substrate (master TODO #7)** | The substrate this service borrows; not duplicated here. |
| **Edge cost** given a code + cell flags (table, code-2 walk, cliff, bridge-flank) | **`PathSupport` (this study)** | R2/R3 — the *consumer* of the code. |
| **Zone-type classification** (`cell+0x4C` cascade) | **`PathSupport`** | R1 — feeds the passability matrix column. |
| **Zone slope cost** (`Zone_Estimate_Slope_Cost` + gate) | **`PathSupport`** | R5. |
| **Marker gate** (level-0 marker array consume + `+0x122` exception) | **`PathSupport`** (gate logic) over substrate-provided `+0x122` | R4 — gate is path logic; refcount is substrate data. |
| **Bridge-peer fallback** scan | **`PathSupport`** | R6 — a path-support scan over substrate lists. |

Rule: `PathSupport` is a set of **pure functions over borrowed substrate references** — it never owns
occupancy or cell state, never calls `render/ui/audio/net`. It plugs into A* through the existing
`AStarOptions` seam in `core.rs`.

### 6.1 Module layout

```
src/sim/pathfinding/
  support/
    mod.rs            // PathSupport facade; re-exports
    edge_cost.rs      // R2/R3: cost table, code-2 prediction, cliff, bridge-flank, g-cost assembly
    zone_class.rs     // R1: RecalcZoneType priority cascade → ZoneType(0..=7)
    slope_cost.rs     // R5: zone-level slope estimate + ftol gate (P0-gated)
    marker_gate.rs    // R4: level-0 marker consume + cell+0x122 off-marker exception
    bridge_peer.rs    // R6: FindNearbyBridgePeer 5x5 first-object fallback
  // core.rs unchanged owner of the A* spine; consumes support:: via AStarOptions
```
All `pub(crate)`; `support/` is internal to `pathfinding`. Lives entirely under `sim/` (layering honored).

### 6.2 Type & signature sketch (illustrative; fixed-point per CLAUDE.md)

```rust
// edge_cost.rs — R2/R3. All costs are fixed-point i32 in STEP_COST units (1000 = 1.0),
// so the 0.001..0.008 epsilon maps to 1..8 exactly (already the core.rs convention).
pub(crate) struct EdgeCostInputs<'a> {
    pub can_enter_code: u8,         // 0..=7, FROM the sibling Can_Enter_Cell validator
    pub cliff_ramp: bool,           // cell+0x140 & 0x40000
    pub urgency: u8,                // Pathfinder+0x3C
    pub mover_is_crusher: bool,
    pub dir: u8,                    // 0..=8 for epsilon
    pub cost_multiplier_q: i32,     // Pathfinder+0x04, fixed-point (1000 = ×1.0)
    pub blockers: &'a LayeredEntityBlockMap, // code-2 chain walk source
    pub bridge_flank: Option<BridgeFlankState>, // H5 inputs, None if not bridge-aware
}
pub(crate) fn edge_g_cost(inp: &EdgeCostInputs, neighbor: (u16,u16), layer: MovementLayer) -> Option<i32>;
// None == code>=7 (reject). Folds H1..H6 in ONE place, mirroring AStar_compute_edge_cost.

// zone_class.rs — R1
pub(crate) fn recalc_zone_type(cell: &CellFacts, overlay: Option<&OverlayFacts>) -> u8; // 0..=7, H8 cascade

// slope_cost.rs — R5 (P0-gated until slope-context writer is verified)
pub(crate) fn zone_slope_cost(ctx: &SlopeContext, level: u8, cur: ZoneId, next: ZoneId) -> i32; // H15
pub(crate) fn slope_contribution(raw: i32, mover_factor_q: i32) -> i32; // ftol(raw * factor), H14/H15

// marker_gate.rs — R4 (largely already present as HierarchyGate/BlockerNeighborCounts)
pub(crate) fn marker_allows(level0_zone: ZoneId, gate: &HierarchyGate, blocker_refcount: u8,
                            hierarchy_enabled: bool, near_height: bool) -> bool; // H10/H11

// bridge_peer.rs — R6
pub(crate) fn find_nearby_bridge_peer(center: (u16,u16), requested_height: i32,
                                      lists: &dyn CellObjectLists) -> Option<ObjId>; // H17/H18
```

### 6.3 Ownership & wiring

- `astar_search` keeps owning the open/closed sets and the workspace. It calls `edge_g_cost()` exactly
  once per neighbor (replacing the three scattered cost sites at core.rs:1262–1294), passing the
  `can_enter_code` it received from the validator. **One cost owner**, mirroring the binary.
- `Zone_precheck`-equivalent (`zone_search.rs`) calls `zone_slope_cost` + `slope_contribution` inside the
  corridor Dijkstra and stamps the marker array; `expand_corridor` is deleted (P5).
- `recalc_zone_type` is invoked by the map/cell substrate's terrain-recalc path (substrate #7 owns *when*;
  `PathSupport` owns *what value*).
- `find_nearby_bridge_peer` is invoked only by the bridge-passability marker generator (deferred slice P7).

### 6.4 Scale (30-player / 20k-unit target)

The denormalized `LayeredEntityBlockMap` is already `BTreeMap`-keyed (deterministic). `PathSupport`
functions are pure and allocation-free on the hot path (the code-2 walk is bounded to 10 hops; the
5×5 scan to 25 cells). No per-player fixed-size arrays are introduced.

---

## 7. Old ad hoc Rust logic to RETIRE/fold into `PathSupport`

| Item (file:symbol) | Action | Reason |
|---|---|---|
| `zone_search.rs::expand_corridor` (794) + `AStarOptions::corridor` BTreeSet path (`core.rs:716`, applied 1216–1224) | **RETIRE** | One-ring widening is explicitly NOT a binary behavior (doc OQ-7). Replace with the `HierarchyGate` marker path (already present) once it is authoritative. |
| Triple-located edge cost: `core.rs` code-2/5/6 block (1274–1292), cliff (1270–1272), marker (1294), bridge-flank (`bridge_flank_multiplier`/`apply_bridge_flank_cost`, 2147/2158) | **FOLD** into `edge_cost.rs::edge_g_cost` | Binary computes one cost per neighbor in one function. Unify; also un-`dead_code` the bridge-flank path. |
| `core.rs` hard-block of code-4-equivalents via `entity_blocks`/`bridge_blocks` BTreeSets (1184–1203) | **REVISE** | H7: occupied-friendly (60.0) and friendly-stationary (8.0)/enemy (20.0) must be **soft** path-through costs, not hard blocks. Keep only buildings (code 7) hard-blocked. |
| `core.rs::CLIFF_COST_MULTIPLIER` trigger = inter-cell height change (1270) | **REVISE** | H4: trigger is the `0x40000` cliff-ramp flag, not raw height delta. Move flag into `CellFacts`. |
| `core.rs::euclidean_heuristic` (2079) | **KEEP (verified correct 2026-06-04)** | NOT drift — the binary cell-A* heuristic IS Euclidean (`Sqrt_Approx(dx²+dy²)` in `AStar_create_node 0x0042a420`). Leave as-is. Only audit that the `×1000` STEP_COST scaling lines up with the binary's `1.001` per-step constant (`_DAT_007e37c0`); no heuristic-shape change. |
| `zone_search.rs::find_zone_corridor` centroid-Manhattan edge cost (677) | **REVISE (heavier than first stated)** | Replace centroid-Manhattan with the binary's **pure uniform-cost Dijkstra**: edge weight = `zoneBaseCost[zoneType]` (table `0x007e3794` `{1,0,0,1,1,0,1,1}`) + `ftol(slope)` (P0-gated, H15) + `0.001` diagonal / `0.0` cardinal step (`0x007e3818`/`0x007e2800`). Remove the distance estimate entirely — the corridor tier has NO heuristic. Keep the insertion-order tie behavior (already correct, H13). |
| `terrain_cost.rs::TerrainCostGrid` as an A* edge-cost axis (`core.rs:1263–1267` `100/cost`) | **RECONCILE** | The binary A* edge cost is occupation/cliff/bridge-coded, NOT terrain-speed-coded. Terrain speed belongs to locomotor *movement execution*. Decide one cost model; do not double-count. |
| `core.rs::explicit_tube_edge` dir-8 tube path (677, 1330–1369) | **GATE OFF** | TS-legacy tube/tunnel (`feedback_no_tunnel_subterranean`). Keep inert in standard YR. |

---

## 8. Migration SLICES + ACCEPTANCE TESTS

Shadow-first, dependency-ordered, each independently shippable. Mirrors the Mission/Radio rhythm:
shadow → invert hash-invariant → drop shadow asserts → authoritative → `SNAPSHOT_VERSION` bump →
parity harness. **P0 is a BLOCKING research gate.**

- **P0 — RESEARCH GATE (BLOCKING, no code).** Re-decompile the slope-context writer lifecycle:
  who fills `Foot+0x21C → +0x57E4` (L1 array) and `+0x59F0` (130-wide L2 grid), and the level-2 coarse-coord
  adjustment math, before any slope cost (R5/H14/H15) becomes authoritative. Also resolve `Pathfinder+0x01`
  (bridge-aware flag) lifecycle to un-block the bridge-flank multipliers (H5). Deliverable: a verified
  addendum to `ZONE_ESTIMATE_SLOPE_COST_PARITY` covering the writer side. **Until P0 closes, slices touching
  R5/H5 ship as shadow-only and may not flip authoritative.**

- **P1 — `edge_cost.rs` extraction (shadow).** Create `support::edge_cost::edge_g_cost` reproducing the
  current three scattered cost sites EXACTLY (no behavior change). Add a shadow assert that the unified
  cost equals the legacy inline cost for every neighbor.
  - **Test `pathsupport_edge_cost_shadow_matches_legacy_inline`**: over a fixture map with code-2/5/6
    blockers, cliff cells, and a bridge, assert unified == legacy per neighbor; state hash unchanged.

- **P2 — Cost-table & soft-block correction (authoritative behind shadow).** Apply H1/H7: make codes 4/5/6
  soft path-through costs (60/20/8), keep only buildings hard-blocked; fix H4 cliff trigger to the `0x40000`
  flag. Invert the shadow assert (legacy is now the shadow).
  - **Test `pathsupport_boxed_by_stationary_allies_finds_high_cost_path`**: a unit fully ringed by stationary
    friendlies still returns a path (cost ≈ 8.0×perimeter), not `None`.
  - **Test `pathsupport_cliff_ramp_flag_costs_4x_not_height_delta`**: a flat-height cell with the `0x40000`
    flag costs ×4; a height-change cell WITHOUT the flag does not.

- **P3 — g-cost reconciliation (heuristic confirmed parity, NOT a fix).** Resolve H6 cost_multiplier and the
  corridor-tier cost model. The cell-A* Euclidean heuristic is VERIFIED-matching (2026-06-04) — do NOT change it;
  only confirm the STEP_COST/×1000 scaling vs the binary `1.001` per-step constant. The real P3 work is the
  **corridor-tier** swap: replace centroid-Manhattan with uniform-cost Dijkstra (zone base table `0x007e3794` +
  0.001 diagonal step), no distance estimate.
  - **Test `pathsupport_equal_cost_grid_expands_cardinal_first`**: on a uniform-cost open grid, the tiebreaker
    epsilon yields the cardinal-first expansion order the binary produces; path is deterministic and reproducible.
  - **Test `corridor_dijkstra_has_no_distance_heuristic`**: two zone chains of equal hop-count-weighted base cost
    expand purely by accumulated zone base cost (+0.001 diag), with NO Euclidean/Manhattan pull toward the goal.

- **P4 — `zone_class.rs` cascade (shadow then authoritative).** Replace the passability-matrix zone derivation in
  `zone_build.rs` with the H8 first-match cascade for runtime terrain recalc.
  - **Test `pathsupport_zone_type_cascade_matches_priority_order`**: synthetic cells (overlay Crushable, Wall,
    Wheel-speed-0, IsARock, Water, Beach, terrain-object, building-gate) each classify to the H8 expected value;
    first-match wins on multi-condition cells.

- **P5 — Marker-gate authoritative; retire `expand_corridor`.** Make `HierarchyGate`+`BlockerNeighborCounts`
  the sole corridor mechanism (H10/H11/H12); delete `expand_corridor` and the corridor BTreeSet path.
  - **Test `astar_hierarchy_rejects_unmarked_one_ring_zone_without_blocker_exception`** (from doc): tempting
    one-ring zone not level-0 marked is rejected unless `+0x122 != 0`.
  - **Test `astar_hierarchy_allows_off_marker_cell_with_blocker_neighbor_count`** (from doc): off-marker cell
    with refcount 1 allowed; identical cell with refcount 0 pruned.
  - **Test `zone_precheck_same_zone_failure_clears_hierarchy_before_astar`** (from doc): same-zone forced miss
    runs unrestricted A*; cross-zone forced miss returns None before cell A*.

- **P6 — Slope cost (authoritative ONLY if P0 closed).** Wire `slope_cost.rs` into the corridor Dijkstra under
  the H14 gate; `ftol(raw × factor)` per H15.
  - **Test `zone_precheck_applies_slope_cost_only_above_factor_threshold`** (from doc): mover factor ≤ 1e-5 →
    zero slope contribution; factor above → contribution added.
  - **Test `zone_slope_level1_uses_neighbor_representative_not_source_zone`** (from doc).
  - **Test `zone_slope_level2_uses_directional_corner_min_average`** (from doc).

- **P7 — Bridge-peer fallback + bridge-flank cost (authoritative ONLY if P0/`+0x01` closed).** Implement
  `find_nearby_bridge_peer` (H17/H18) and un-block the bridge-flank multipliers (H5).
  - **Test `bridge_marker_fallback_uses_first_nearby_accepted_object`** (from doc).
  - **Test `bridge_marker_fallback_height_gap_two_ground_three_bridge`** (from doc): diff 2 → ground list, diff
    3 → bridge list.
  - **Test `bridge_marker_fallback_none_preserves_no_peer_urgency_path`** (from doc).
  - **Test `pathsupport_bridge_diagonal_flank_2_1_10`**: both-flank ×2, one-flank ×1, neither ×10.

**Post-P7:** bump `SNAPSHOT_VERSION` once all authoritative flips land (slope + edge-cost are hash-relevant
since they change path shape → unit positions → state hash); add a deterministic-replay parity harness entry
(seeded skirmish, baseline path-set hash) mirroring the Factory/House Slice-8 global parity harness.

---

## 9. Sources & Verification Ledger

### Live-verified THIS session (Ghidra MCP, 2026-06-04)
- `get_function_by_address 0x00483c80` → `CellClass__RecalcZoneType` (body `0x00483c80–0x00483e28`).
- `get_function_by_address 0x00429830` → `AStar_compute_edge_cost`.
- `get_function_by_address 0x00429a90` → `AStar_main_loop`.
- `get_function_by_address 0x0042c290` → `Zone_precheck`.
- `get_function_by_address 0x00585f40` → `Zone_Estimate_Slope_Cost`.
- `get_function_by_address 0x0042b080` → `PathfinderClass__FindNearbyBridgePeer` (the live label for `FUN_0042B080`).
- `read_memory 0x0081870c` len 32 → A* base cost table `{1.0,1000.0,1.0,1.0,60.0,20.0,8.0,10000.0}` (bit-exact).
- `read_memory 0x0081872c` len 36 → direction tiebreaker `{.001,.005,.002,.006,.003,.007,.004,.008,0.0}` (bit-exact).

### Doc-sourced (cited; NOT re-decompiled this session — confidence = each doc's own `ghidra/verified`)
- `docs/research/pathfinding/PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` — R1 cascade, R7, cost/tiebreak tables, Rust status §6.
- `docs/research/pathfinding/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` — R2/R3, code-2 prediction (H2), urgency, soft-block H7, crusher H3.
- `docs/research/pathfinding/ASTAR_MAIN_LOOP_LEVEL0_MARKER_GATE_GHIDRA_REPORT.md` — R4/R8 (H9–H13), `cell+0x122` exception.
- `docs/research/ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md` — R5 (H14/H15), `ftol`, mover factor.
- `docs/research/ZONE_ESTIMATE_SLOPE_COST_GHIDRA_REPORT.md` — slope helper formula (corroborating).
- `docs/research/pathfinding/PATHFINDER_ALT_OBJECT_LIST_FUN_0042B080_GHIDRA_REPORT.md` — R6 (H17/H18).
- `docs/research/pathfinding/PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md` — `Pathfinder+0x04` cost_multiplier (H6).
- `docs/research/CELL_0x122_CAN_ENTER_CELL_SEMANTIC_GHIDRA_REPORT.md` — `cell+0x122` = blocker-neighbor refcount (NOT fog).
- `docs/research/SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md` — `Foot+0x530` / `ThreatAvoidanceCoefficient`.

### Rust files read this session (for §4/§7)
- `src/sim/pathfinding/{mod,core,zone_search,terrain_cost}.rs` (full or signature-level), `passability.rs` / `cell_entry.rs` / `zone_build.rs` (surveyed), `src/sim/movement/{group_destination,movement_reservation}.rs`.

### Live-verified THIS session (Ghidra MCP, 2026-06-04 — Pass 2 expansion)
- `decompile_function 0x0042c290` (Zone_precheck) + `disassemble_function 0x0042c290` — slope gate, ftol fold, marker stamp, AND the corridor cost assembly `zoneBaseCost[0x7e3794] + parent_g + ftol(slope) + diagStep`. Corridor is uniform-cost Dijkstra, NO heuristic.
- `decompile_function 0x00429a90` (AStar_main_loop) — codes 4/5/6 SOFT (`iVar17 < 7` expands), g-cost `edge×(this+4)+epsilon[dir]`, cliff `0x40000` read.
- `decompile_function 0x00429830` (AStar_compute_edge_cost) — confirmed code-table lookup, code-2 walk, `0x40000` ×4, bridge flank. `param_1` = search-context (read_memory 0x00429f60 → `MOV ECX,ESI` call setup).
- `decompile_function 0x0042a460` (AStar_create_node) + `decompile_function 0x004cac40` (Sqrt_Approx) — cell-A* heuristic is genuine **Euclidean** `Sqrt_Approx(dx²+dy²)`. Rust `euclidean_heuristic` MATCHES.
- `decompile_function 0x0042acf0` (UpdateBridgePassability) — `0x40000` is a dynamically-XOR-toggled bridge-peer-path marker gated by `cell+0x124`, NOT static cliff terrain.
- `decompile_function 0x0042b080` (FindNearbyBridgePeer) — H17 5×5 scan, list choice `abs(level−req)>=3→bridge`, predicate `(obj+0x674 vtable+0xa0)(subobj, x*256+128, y*256+128, req*0x89c2d8)`, filter `obj+0x14 bit2`. Matches H17.
- `decompile_function 0x0042d170` (EstimateZoneCost) + `0x0042c900` (AStar_pathfind_search) — Zone_precheck's mover arg (`param_5`, whose `+0x21c` is the slope ctx) = the FootClass `this`. Both callers pass the mover through.
- `decompile_function 0x004cbba0` (Run_AStar) — passes FootClass `this` as mover; called by `Find_Path 0x004d3920`.
- read_memory: `0x007e3794` zone base table `{1,0,0,1,1,0,1,1}`; `0x007e3818` = 0.001 (diag); `0x007e2800` = 0.0 (cardinal); `0x007e37c0` = 1.001 (cell step). All bit-exact.

### UNCHECKED / blocking-gate items (post-Pass-2)
- **RESOLVED → VERIFIED (was DRIFT):** Euclidean heuristic — the binary cell-A* IS Euclidean (`Sqrt_Approx`), Rust matches. Removed from drift list.
- **RESOLVED → VERIFIED:** codes 4/5/6 are SOFT (`iVar17 < 7` expands) — Rust hard-block is confirmed DRIFT.
- **RESOLVED → VERIFIED:** corridor tier is pure uniform-cost Dijkstra (no heuristic) with zone base table `0x007e3794`; Rust centroid-Manhattan is drift on two axes.
- **CORRECTED:** `cell+0x140 & 0x40000` = dynamic bridge-peer marker (toggled by `UpdateBridgePassability`, gated by `cell+0x124`), not static cliff terrain. H4/H5/R6 are one coupled bridge subsystem.
- **UNCHECKED:** whether Rust's terrain-speed A* cost axis double-counts vs the binary's occupation-coded cost (§7 RECONCILE).
- **BLOCKING (P0) — STILL OPEN, traced but unresolved this run:** slope-context (`Foot+0x21C` → `+0x57E4`/`+0x59F0`) **writer** lifecycle. `Foot+0x21c` is a pointer (on FootClass) to a large slope-context block (offsets +0x57E4/+0x59F0 ⇒ ~23KB, so a separate per-map/per-unit workspace, not the FootClass body). No literal `mov [reg+0x21c], reg` store exists (search_byte_patterns `89 ?? 1c 02 00 00` → only `0x00669a54`, which is a RulesClass field, not Foot). The block is populated through computed-base loops, not immediate-offset stores. **Next query:** `get_field_access_context` on a FootClass instance addr for offset 0x21c, OR decompile the FootClass constructor / `ZoneMap` build path that allocates the `+0x59f0` 130-wide grid (the `e4 57 00 00` / `f0 59 00 00` byte-pattern hits at `0x586xxx` are all READS inside `Zone_Estimate_Slope_Cost`; the writer is elsewhere — try xrefs to the allocation of a ≥0x59f0+0x82*0x82*4 byte block).
- **BLOCKING (P0/P7):** `search_ctx+0x01` bridge-aware-flag lifecycle (un-blocks H5 bridge-flank multipliers). Read in `AStar_compute_edge_cost` as `*(char*)(param_1+1)`; the `+0x01` byte store did not surface via `c6 41 01`/`88 41 01` patterns inside pathfinding functions. Still UNCHECKED — next query: `get_field_access_context` on the A* search-context struct at offset 1, or trace the `AStar_main_loop` `this` init in `Find_Path 0x004d3920`.
- **UNRESOLVED (do not invent):** `object+0x674` subobject class identity / vtable `+0xA0` predicate name — model as "footprint accepts point at height" until named.
- **DOC-ONLY (not re-verified this session):** slope tables `0x0082a984`/`0x0082a9c4`/`0x00abd460`, zone graphs `DAT_0087f890`/`DAT_0087f8a8`, Wheel-speed table `0x0089ea48`. (Passability matrix `0x0082a594` stride/element-size now VERIFIED.)

---

## Adversarial review (2026-06-04) — what was re-verified and what remains

**Re-verified live this session (all PASS, bit/identity-exact):**
- All six function identities (`0x00483c80`, `0x00429830`, `0x00429a90`, `0x0042c290`, `0x00585f40`, `0x0042b080`) and `FootClass__Get_Slope_Speed_Factor 0x004dc760` — `get_function_by_address` each.
- A* base cost table `0x0081870c` and direction tiebreaker `0x0081872c` — `read_memory`, decoded bit-exact to the doc's values.
- Bridge/cliff multipliers `0x007e37bc`=4.0, `0x007e37b4`=2.0, `0x007e2ac8`=1.0, `0x007e37b8`=10.0 — `read_memory` (previously DOC-ONLY, now **promoted to VERIFIED**).
- CellArray neighbor offsets `0x007e3774` = `{-512,-511,+1,+513,+512,+511,-1,-513}` — `read_memory`, bit-exact (previously DOC-ONLY, now **VERIFIED**; this is the operative `g_CellNeighborOffsets_8Dir` used in `AStar_main_loop`).
- **H11 (`cell+0x122` off-marker exception) confirmed in the binary**: `decompile_function 0x00429a90` shows `if ((*(char *)(iVar16 + 0x122) == '\0') && (param_7 != '\0')) goto <skip neighbor>;` — pruned only when refcount==0 AND hierarchy flag set, exactly as H11 states.
- **H7 (codes 4/5/6 SOFT) confirmed**: `AStar_main_loop` expands every neighbor whose `Can_Enter_Cell` code (`vtable+0x1ac`) is `< 7`; only `>= 7` rejects. Codes 4/5/6 are expanded at cost — the Rust hard-block (`entity_blocks` BTreeSets, core.rs:1184–1203) is a genuine DRIFT.
- **R5 plumbing confirmed**: `Zone_precheck` reads `*(param_5 + 0x21c)` as the slope context, gates on `threshold@0x007e3810 < Get_Slope_Speed_Factor`, calls `Zone_Estimate_Slope_Cost` then `Math__ftol`, and stamps chain at `+0xbc + level*1000` / count `+0xc74 + level*4` — matches H9/H14/H15.
- Rust retire-list refs spot-checked and accurate: `euclidean_heuristic` (core.rs:2079, `sqrt(dx²+dy²)*1000`), bridge-flank `#[allow(dead_code)]` (core.rs:2146/2157) with the "PathfinderClass+0x01 lifecycle" block comment (core.rs:143), `find_zone_corridor` Manhattan-between-zone-centers (zone_search.rs:677), `expand_corridor` (zone_search.rs:794).

**Corrections applied this pass:**
- Slope threshold `0x007e3810` was characterized as "f32 ~1e-5" — the raw bytes are `f1 68 e3 88` (a tiny-magnitude float, NOT 1e-5). Corrected in §2b and H14 with the verified bytes + `Zone_precheck` gate citation. Behavior (any positive factor passes) is unaffected, but the literal figure was wrong.

**Reviewer follow-ups (uncertain — do NOT treat as resolved):**
- `g_DirectionOffsets @ 0x0089f688`: a static `read_memory` returned all-zeros (uninitialized at rest / wrong slot / runtime-populated). The operative neighbor table in `AStar_main_loop` is `g_CellNeighborOffsets_8Dir @ 0x007e3774` (verified). Re-resolve `g_DirectionOffsets`'s true address/role before H16's dx/dy table becomes load-bearing; it is currently UNVERIFIED, not VERIFIED.
- The exact intended value of the slope threshold constant (vs. its raw tiny-magnitude bytes) should be settled in the P0 slope-context addendum — it does not block the "positive factor passes" semantics but should not be quoted as 1e-5 anywhere downstream.

**Program-fit check (PASS):** `PathSupport` lives entirely under `sim/` (no `render/ui/audio/net` dependency), slots into substrate master-TODO #7 rather than competing, keeps `advance_tick` phase order unchanged, defers the `SNAPSHOT_VERSION` bump to post-P7 once path-shape-affecting flips land (correctly identified as hash-relevant), and follows the Mission/Radio shadow→invert→authoritative→parity rhythm. No TS-legacy path is designed into the substrate (tube/tunnel dir-8 and LAT tile-blend both correctly gated OFF; `cell+0x122` correctly identified as blocker-refcount, not fog).

---

## 10. Pass 2 — Expansion (2026-06-04)

A systematic sweep of consumers, globals, tables, and edge cases the time-boxed Pass 1 missed. Each item is
tagged **VERIFIED** (live this run) / **DOC-ONLY** / **UNCHECKED**.

### 10.1 Newly-found CONSUMERS of the helper family (get_function_callers / get_xrefs_to)

| Function | Address | Relation | Status |
|---|---|---|---|
| `FootClass__Find_Path` | `0x004d3920` | **Top-level pathfinding entry** — sole caller of `Run_AStar 0x004cbba0`. The doc listed `Run_AStar` as the wrapper but missed its single caller. | VERIFIED (get_function_callers 0x004cbba0) |
| `DriveLocomotionClass__Process_Movement` | `0x004b2630` | Calls `Find_Path 0x004d3920` (mover requests a path during movement execution). | VERIFIED (get_function_callers 0x004d3920) |
| `ShipLocomotionClass__Process_Movement` | `0x006a1c80` | Calls `Find_Path`. | VERIFIED |
| `WalkLocomotionClass__ProcessMovement` | `0x0075aec0` | Calls `Find_Path`. | VERIFIED |
| `FUN_005164d0`, `FUN_005b01c0` | — | Additional `Find_Path` callers (un-named; likely teleport/special-move locomotors). | VERIFIED (callers list); identities UNCHECKED |
| `ZoneMap__FindBestCompatibleMovementZone` | `0x00588a11` | **Reads the passability matrix `0x0082a594`** — a matrix consumer the doc did not list (doc only attributed the matrix to R1/A*). | VERIFIED (get_xrefs_to 0x0082a594) |
| `MapClass__UpdateBridgeZonesHelper` | `0x0056c997` | Reads the passability matrix during bridge-zone recompute. | VERIFIED |
| `ZoneMap__FloodFillReachableZones` | `0x005841c9` | Reads the passability matrix during zone flood-fill (the R1 zone-build path). | VERIFIED |

**Contract impact:** R1's zone classification feeds at least three more matrix consumers than the doc's "passability
matrix column" framing — the matrix is a shared zone-reachability surface for zone-build, bridge-zone recompute, and
best-compatible-zone selection, not just the A* edge tier. The `PathSupport` seam (§6.0) should expose the matrix as
substrate-owned read surface for all of these, not duplicate it per consumer.

### 10.2 Newly-found GLOBALS / TABLES (get_xrefs_from + read_memory)

| Item | Address | Shape / value | Where used | Status |
|---|---|---|---|---|
| Zone-corridor base-cost table | `0x007e3794` | 8 × f32 `{1,0,0,1,1,0,1,1}` | `Zone_precheck` corridor edge weight | **VERIFIED** (read_memory) |
| Corridor diagonal-step cost | `0x007e3818` | double `0.001` | `Zone_precheck` per-diag step | **VERIFIED** |
| Corridor cardinal-step cost | `0x007e2800` | double `0.0` | `Zone_precheck` per-cardinal step | **VERIFIED** |
| Cell-A* per-step re-relax cost | `0x007e37c0` | double `1.001` | `AStar_main_loop` node re-relax | **VERIFIED** |
| Height→lepton scale | `0x0089c2d8` | i32 (map height multiplier) | `FindNearbyBridgePeer` predicate arg, `UpdateBridgePassability` | DOC-ADJACENT (used as `req*0x89c2d8`); exact value UNCHECKED |
| `_DAT_007e4900` (Sqrt_Approx neg-input scale), `DAT_008650bc` (sqrt mantissa LUT base) | `0x007e4900` / `0x008650bc` | sqrt approximation internals | `Sqrt_Approx 0x004cac40` | VERIFIED (present in decompile); not load-bearing for parity (Rust uses real sqrt) |

### 10.3 SLOTS / vtable seams catalogued this run

- `vtable+0x1ac` = `Can_Enter_Cell` — confirmed the gate is `iVar17 < 7` in `AStar_main_loop` (already in §2d; behavior now bit-verified).
- `vtable+0x84` = `Get TechnoTypeClass` (returns type ptr; `+0xc94` crusher-ish flag read in main_loop, `+0x5b4` default zone read in pathfind_search). NEW slot not in §2d.
- `vtable+0x2c` = `What_Am_I` / RTTI-id (returns 1=Unit, 2=?, 0xf=? — gates the code-2 prediction `bVar10` and the `== 1 && +0xe0c` branch). NEW slot.
- `vtable+0x1b8` = `Get cell coords` (code-2 walk + `UpdateBridgePassability` probe). Already in §2d (`GetCellCoords`); confirmed.
- `vtable+0x1bc` = secondary-cell/center accessor used in the code-2 bridge height compare. NEW slot.
- `vtable+0x4c` = path-setup (called at `Run_AStar` entry). NEW slot.
- `vtable+0xa0` on `obj+0x674` = footprint point-accept predicate (R6). Confirmed; subobject class still UNRESOLVED.

### 10.4 EDGE CASES / TS-legacy separated this run

- **Hierarchical-findpath debug-log strings** (`s_Hierarchical_findpath_failure...`, `s_Regular_findpath_failure...`, `s_Warning__A__without_HS...`) in `AStar_pathfind_search` — these are `Register_heap_pool`/debug-log calls on failure paths; **not gameplay**, do not model. Active in YR but produce no player-observable output.
- **`vtable+0x2c == 0xf` + `param_4[0x1b0]+0xd94` branch** in `AStar_pathfind_search` (the `What_Am_I==0xf` special) calls a COM-style `QueryInterface (0x80004003 assert)` path. Reachability in standard YR skirmish is **UNCHECKED** — flag, do not model speculatively (likely a special unit type; could be TS-legacy). Next query: resolve `What_Am_I` enum 0xf identity.
- **The retry loop** in `AStar_pathfind_search` (up to 5 attempts: `iStack_14 = 5` cross-zone / `1` same, decrementing via `UpdateHierarchicalEdges` + `Reset` + re-`Zone_precheck`) — this hierarchy-edge-invalidation retry is live in YR and **not yet modeled in the Rust corridor fall-through**. The doc's H12 covers same/cross-zone failure but not the multi-attempt `UpdateHierarchicalEdges` retry. NEW partial-coverage gap (DOC-ONLY for the retry-count; verified the loop exists via decompile_function 0x0042c900).

### 10.5 Cross-family shared-gate folds (from Phase-1 findings, applied to PathSupport)

- **ftol = truncate-toward-zero (CW 0x0E7F, `Math__ftol 0x007c5f00`).** Applies directly to H15's slope fold:
  `local_58 = Math__ftol()` after `FILD slope; FMUL [slope_factor]` (disassemble_function 0x0042c290 at `0x0042c589..0x0042c591`). Rust `slope_contribution` must use **truncate toward zero** (`fixed`'s default `.to_num::<i32>()`), NEVER `.round()`. The multiply `raw_slope × mover_factor` stays in the wider type and truncates once, at the same boundary gamemd does (immediately after the FMUL). This is the only ftol in the corridor cost path — the float zone base cost and 0.001 diag are summed in float10 and stored as `(int)` only at the final node f-cost, matching the cross-family "truncate at sub-step boundaries, not per-multiply" rule.
- **ReadDouble f32-narrowing boundary.** The mover slope factor `Foot+0x530` ← `TechnoTypeClass+0x2F0` `ThreatAvoidanceCoefficient` is a `ReadDouble`/`ReadFloat`-class INI scalar → carries **f32 mantissa precision** (per the cross-family ReadDouble finding: `sscanf "%f"` then widen). Rust must parse `ThreatAvoidanceCoefficient` through f32 first, then widen, before it becomes the slope mover factor — do NOT parse direct to f64/SimFixed (would retain precision gamemd discards). The slope gate `threshold@0x007e3810 < factor` is a double compare, but the factor itself is f32-precision.

### 10.6 Burden-of-proof re-flag of this doc's own remaining "equivalent" claims

Re-applying DRIFT-default to claims this doc carried without proof:
- **§4 "R8 insertion-order ties (already correct, H13)"** — DOC-ONLY that Rust's `ZoneQueueEntry` sequence field bit-matches the binary's min-heap sift order. The binary uses a **binary min-heap with sift-up/sift-down** (`MinHeap__SiftDown 0x0042dca0`), not a FIFO insertion queue. Equal-f-cost tie resolution between a heap and an insertion-order queue is **NOT proven equivalent** — re-flag as **DRIFT/UNCHECKED** pending a boundary test on ≥3 equal-cost candidates. (The heap pop order for equal keys depends on insertion position in the heap array, which is not the same as discovery order.)
- **§4 "R7 8-dir neighbor table present ✅"** — the operative table is `g_CellNeighborOffsets_8Dir 0x007e3774` (VERIFIED bytes), but the Rust `NEIGHBORS` ordering vs the binary's `iStack_44` 0–7 loop order (which interleaves with the `0x40000`/zone checks per index) was not bit-compared. Likely fine but DOC-ONLY — confirm the index→(dx,dy) mapping order matches before P5.
