# Hierarchical vs Regular Pathfinding — System MAP + Core Verification (Ghidra)

**Date:** 2026-07-19
**Investigation mode:** system-MAP + live core-structure re-verification (this is a
consolidation/verification pass over an already-deeply-decoded corpus, not a fresh decode).
**Program:** gamemd.exe (RA2 Yuri's Revenge), image base 0x00400000, Ghidra project `testProsjekt`.
**Authority:** binary → Ghidra → docs. Every address below was re-decompiled/read live this session
unless tagged `(doc)`.

## Scope & relationship to existing research

This system is one of the most thoroughly decoded in `docs/research/` (81 docs under the
`pathfinding` map; the `/decode-system pathfinding` run produced 34 per-symbol PROOFED decodes plus
`pathfinding/_system.md` and `pathfinding/_parity.md`). This report does **not** redo that work — it
**maps** the regular-vs-hierarchical split the Rust port flags as approximate
(`src/sim/pathfinding/mod.rs:6-9`, `zone_search.rs:11-13`, `zone_build.rs:243-245`, `zone_map.rs:46`),
**re-verifies the load-bearing core structures live**, surfaces one **stale-corpus contradiction it
found**, and hands off a prioritized Rust gap list. Where a claim is only doc-sourced it is tagged
`(doc: <FILE>)`; verify before treating as load-bearing.

Anchors (all live-verified this session via `decompile_function`/`read_memory`):
- `AStar_pathfind_search @ 0x0042C900` — orchestrator (regular⇄hierarchical dispatch + retry loop)
- `Zone_precheck @ 0x0042C290` — hierarchical zone-graph Dijkstra
- `AStar_main_loop @ 0x00429A90` — regular per-cell A*
- `MapClass__Can_Reach_Zone @ 0x0056D100` — reachability gate 1 (zone-ID equality)
- `MapClass__GetZoneID @ 0x0056D230` — per-cell zone-ID resolution (two-level nodeIndex indirection)
- `g_PassabilityMatrix @ 0x0082A594` — 13×8 i32 MovementZone × reduced-zone-type table

---

## 1. The two entrypoints — regular (cell A*) vs hierarchical (zone corridor)

Verdict: **they are not two separate functions you pick between — they are one orchestrator
(`AStar_pathfind_search`) that composes a coarse zone corridor with a fine cell search.** The
"regular" pathfinder is `AStar_main_loop` (per-cell A*); the "hierarchical" pathfinder is
`Zone_precheck` (per-zone Dijkstra). `AStar_pathfind_search` runs the hierarchical pass first to stamp
a corridor, then always runs the cell pass constrained to that corridor.

Verified control flow of `AStar_pathfind_search @ 0x0042C900` (decompile this session):

1. `*(this+0x38) = 1` (set hierarchy-valid flag), `PathfinderClass__Reset()`, then clears **3**
   per-search edge-exclusion vectors: `piVar6 = this+0x74; iVar7=3; do { (*vtable[0xc])(); piVar6 += 6; } while(--iVar7)`
   (three 24-byte `PathfinderHeapVec` at +0x74/+0x8C/+0xA4).
2. `*(this+0x3c) = param_8` (urgency). Resolve both endpoint cells and both endpoint **zone IDs**:
   `iStack_14 = GetZoneID(src, mz, ...)`, `iVar3 = GetZoneID(dst, mz, ...)`. `mz` comes from
   `param_7`; if `param_7 == 0xFFFFFFFF` it is read from the mover's `TechnoTypeClass+0x5b4`
   (the MovementZone field) via `vtable[0x84]`.
3. Bridge-snap both endpoints (`MapClass__ResolvePathCoord_BridgeAware`).
4. Decide whether to run the hierarchical pass — `param_8` (a bool, call it `allowHS`) is computed
   from: mover flag `+0xc94` clear, `this+0x3d5` set, `vtable[800]` false, and both snapped endpoints
   in-playfield.
5. **The regular-vs-hierarchical decision (verified):**
   - `if (srcZone == dstZone)` (`iStack_14 == iVar3`): if `allowHS`, run `Zone_precheck`. If precheck
     **fails**, log `"Hierarchical findpath failure"` and **clear `allowHS`** — i.e. fall through to a
     *flat* (unconstrained) cell A*. Same-zone precheck failure never aborts.
   - `else if (allowHS)` (cross-zone, hierarchy enabled): **`return 0` immediately** — cross-zone with
     hierarchy on is handled entirely by the corridor; if `Zone_precheck` inside the retry loop can't
     find a corridor the search returns no-path *before* cell A*.
     (Note the asymmetry: same-zone precheck failure degrades to flat A*; cross-zone hierarchy is a
     hard gate. This is H12 in the helpers study, now re-confirmed from the orchestrator body.)
6. **Retry budget:** `iStack_14 = (-(uint)(param_6 != -1) & 0xFFFFFFFC) + 5`. So the total A* attempt
   count is **5 when the caller passes the default limit `param_6 == -1`, else 1** (`0xFFFFFFFC+5 = 1`).
   Ordinary foot pathing passes -1 (doc: `PATHFINDING_FAILED_ASTAR_RETRY_SYSTEM_MODEL_SYNTHESIS.md`).
7. **Retry loop:** call `AStar_main_loop`. On failure (`ret==0`) with `allowHS` still set:
   `UpdateHierarchicalEdges(this)` (append the failing zone edge(s) to the exclusion vectors) →
   `Reset()` → re-read `allowHS = (*(this+0x38) != 0)` → if attempts exhausted return, else re-run
   `Zone_precheck` (which now excludes the appended edges) and loop. If `Zone_precheck` returns 0 on a
   retry, return the last result.

So the composition is: **hierarchical Dijkstra picks a chain of zones and stamps a per-cell marker
array; regular cell A* refines within the marked corridor; failure invalidates a corridor edge and
retries a new corridor.** There is a distance-based *implicit* threshold only in the sense that
same-zone short hops still run the corridor pass (which trivially succeeds as a 1-zone chain) — there
is **no explicit A-to-B distance cutoff** choosing "regular vs hierarchical." The selector is
**zone identity + the `allowHS` gate**, not range.

---

## 2. Zone structure — build, storage, connectivity, MovementZone partition, bridges

### 2.1 Per-cell zone-ID storage is a TWO-LEVEL indirection (confirms the Rust TODO)

`MapClass__GetZoneID @ 0x0056D230` (decompile this session). Ignoring the bridge-redirect prologue, the
return is:

```
linearCellIdx = (MapClass+0xf8 + 1 + MapClass+0xf4) * cellY + cellX      // clamped to [0, MapClass+0x6c)
nodeIndex     = *(u16*)( *(MapClass+0x68) + 2 + linearCellIdx*4 )         // per-cell nodeIndex table
zoneID        = *(u16*)( *(MapClass+0x18 + movementZone*4) + nodeIndex*2 ) // per-MovementZone zoneId-by-nodeIndex
```

This is exactly what `zone_map.rs:46-48` TODO(RE) says the port does *not* replicate: a cell does not
store its zone ID directly. It stores a **nodeIndex** (shared across all MovementZones, at
`MapClass+0x68`, 4-byte stride, the u16 at offset +2), and each MovementZone owns its own
**`zoneIdByNodeIndex` array** (`MapClass+0x18[mz]`). Two cells reachable to a *tracked* unit but not to
an *amphibious* one share a nodeIndex but map to different zone IDs per column. The Rust port instead
keeps a direct `zone_ids: Vec<ZoneId>` per cell per MovementZone (`zone_map.rs:44`) — a denormalized
equivalent, correct in result but structurally different (matters only if a system relies on nodeIndex
identity across zones, e.g. incremental bridge rebuild).

### 2.2 Bridge-layer remap

The `GetZoneID` prologue is the bridge remap: if the cell flag `+0x140 & 0x100` (bridge) is set, it
calls `MapClass__FindBridgeRecord`; if the bridge record's intact byte (`+0x08`/`psVar7[4]`) is 0
(destroyed), it walks perpendicular (`MapCoord_StepByDir_GetCell`, step dir chosen by
`sVar1 != sVar2`) until it leaves bridge cells, then if the landing cell is a real bridge tile and
`LandType != 3` it uses the *far* endpoint (`psVar7 += 2`). Net effect: a bridge-deck cell's zone query
is redirected to a **ground endpoint** so bridge decks join the ground connectivity graph correctly.
The Rust port models this with `zone_map.rs:52 bridge_redirect: Option<Vec<Option<(u16,u16)>>>` — a
per-cell "return this ground endpoint's zone instead" table, explicitly citing `0x0056d230`. This is
the "bridge-layer remap … still pending" item in `mod.rs:7`; the redirect exists but the destroyed-bridge
perpendicular-walk and the `LandType != 3` far-endpoint rule are the parts to verify against the Rust
`bridge_redirect` construction.

### 2.3 Zone graph — 3 hierarchy levels, per-MovementZone

`Zone_precheck @ 0x0042C290` (decompile this session) iterates `local_38` from **2 down to 0** — three
hierarchy levels (level 0 = finest cell-zone; levels 1,2 = coarse super-zones). Per level it reads:
- Per-cell zone id for this level: `*(short*)(DAT_0087f858 + cellNodeIdx*10 + level*2)` — the per-cell
  tuple at `DAT_0087f858` is **10 bytes wide** (word0 = level-0 zone, +2 = level-1, +4 = level-2, plus
  more). This is the shared zone-index table.
- The zone adjacency graph for this level: `DAT_0087f878 + level*0x18` is the base; each zone record is
  **0x24 bytes**, neighbor list pointer at record `+4`, neighbor count at record `+0x10`, a
  representative/reduced-zone-type at record `+0x1c`, and a small field at `+0x18`. (`DAT_0087f890`
  level-1 / `DAT_0087f8a8` level-2 in the helpers-study inventory are `+0x18`/`+0x30` into this base.)

So the connectivity graph is an explicit adjacency list per hierarchy level, keyed by zone id, stored
outside the PathfinderClass (globals at `0x0087f858`/`0x0087f878`), rebuilt by the map-zone builder.

### 2.4 MovementZone partition & the passability matrix

`g_PassabilityMatrix @ 0x0082A594` — read live this session (416 bytes = 13 rows × 8 i32 columns,
row stride 0x20). Value `1` = passable; any other value fails the `== 1` test in `Zone_precheck`
(`(&g_PassabilityMatrix)[movementZone*8 + reducedZoneType] == 1`). Verified rows (MovementZone → 8
reduced-zone-type columns):

```
MZ 0 : 1 2 2 2 2 2 2 3      MZ 7 : 1 2 2 2 2 1 2 3
MZ 1 : 1 1 2 2 2 2 2 3      MZ 8 : 1 1 1 2 2 1 2 3
MZ 2 : 1 1 1 2 2 2 2 3      MZ 9 : 1 1 1 1 1 1 1 3   (Fly — all terrain columns pass)
MZ 3 : 1 1 1 1 1 1 2 3      MZ10 : 2 2 2 2 1 2 2 3
MZ 4 : 1 1 2 1 1 2 2 3      MZ11 : 2 2 2 1 1 2 2 3
MZ 5 : 1 2 2 1 1 2 2 3      MZ12 : 1 1 1 2 2 2 2 3
MZ 6 : 1 1 1 2 2 2 1 3
```

Column 7 is `3` for every row (the "out-of-bounds/rejected" zone-type). This is the same matrix in
`ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; the **13 rows are the MovementZone enum**
(the `TechnoTypeClass+0x5b4` value 0–12), the **8 columns are the reduced zone-type** written to
`CellClass+0x4C` by `CellClass__RecalcZoneType @ 0x00483c80` (doc). The partition is: a cell is in a
MovementZone's connectivity iff `matrix[mz][cell.zoneType] == 1`; the flood-fill that assigns
node/zone IDs only links cells that pass this test for that MovementZone.

---

## 3. Core A* / Dijkstra internals (regular loop) — verified

`AStar_main_loop @ 0x00429A90` (full decompile this session). Key mechanics:

- **Open set:** binary min-heap at `this+0x14` (sift-up on push, sift-down on pop). **Closed/g-cost:**
  *dual* arrays — ground list at `this+0x18` (closed epoch) / `this+0x24` (g-cost), bridge list at
  `this+0x1c` / `this+0x20`. Which pair a cell uses is chosen by height: `if (cell+0x11b < this+0x30)`
  → bridge pair, else ground pair. Confirms the port's ground/bridge dual-closed-list design
  (`core.rs:1-5`).
- **Neighbor set:** 9 directions per pop — `iStack_44` 0..8; dirs 0–7 use
  `g_CellNeighborOffsets_8Dir` (CellClass* pointer offsets) and `g_PathfinderLinearNeighborOffsets8`
  (linear index deltas); **dir 8 = tube edge** (via `g_TubeArray[cell+0x116]+0x28`), TS-legacy, inert
  in stock YR.
- **Hierarchy marker gate (level 0):** for each neighbor, `zoneL0 = *(short*)(DAT_0087f858 + nodeIdx*10)`
  (word0), then check `*(this+0x40)[zoneL0] == this+0x28` (epoch). If off-marker: the near-height
  branch is taken **unless** `(cell+0x122 == 0) && (param_7 != 0)` → `goto skip` (prune). I.e. an
  off-corridor cell is pruned only when its 8-neighbor blocker refcount (`+0x122`) is 0 **and** the
  hierarchy flag (`param_7`) is set. This is the exact `+0x122` off-marker exception (H10/H11), verified.
- **Passability / legality predicate:** the neighbor's legality code comes from a **vtable call**
  `(*vtable[0x1ac])(cell, dir, height, parentCell, …)` = `Can_Enter_Cell` (Unit `0x0073F0A0`,
  Infantry `0x0051BF90`). It returns a code 0–7; **`if (code < 7)` the neighbor is expanded**, `>= 7`
  rejects. Crusher override: if the crusher flag (`+0xc94`) is set (`bVar10`) then `if (code < 7) code = 0`.
  This is why `core.rs:7-10` says "stock neighbor predicate is richer than the grid-level checks" — the
  binary asks a full entity/terrain/occupancy validator per neighbor, returning a graded 0–7 code, not a
  boolean walkable bit. Codes 4/5/6 are **soft** (expanded at high cost, not hard-blocked).
- **Edge cost → g-cost:** `edge = AStar_compute_edge_cost(parent, neighbor, isGround, code, …)` (base
  table `0x0081870C = {1,1000,1,1,60,20,8,10000}`, code-2 10-hop prediction, ×4 `0x40000` marker,
  bridge-flank ×1/×2/×10). Then `g = edge * *(float*)(this+0x04) + tiebreak[dir]` where the tiebreak
  table is `0x0081872C = {.001,.005,.002,.006,.003,.007,.004,.008, 0.0}` (cardinals < diagonals →
  cardinal-first tie resolution). `this+0x04` is the per-search **cost multiplier**.
- **Re-relaxation tolerance:** a closed cell is reopened when `g_new < parent_g + 1.009` (the constant
  shown by the decompiler at the two comparison sites; the memory constant `_DAT_007e37c0 ≈ 1.0009…`).
  The cell-A* per-step cost is **uniform** — diagonal preference comes only from the dir epsilon, not
  from a distance/diagonal upcharge (matches `core.rs:100-103`).
- **Iteration cap:** `if (param_6 < 0) param_6 = 0xFFF7;` → default **65527**. The loop breaks when
  `param_6 <= local_34`. A *separate* success guard requires `local_34 != 10000 && local_34 != param_6`
  before reconstruct — i.e. `10000` is an exact-equality reconstruction-failure sentinel, not the loop
  cap. **See §6 correction: the "10000 cap" claim in `_system.md` is stale.**
- **Success tail:** `AStar_reconstruct_path` → `Path_smooth_corners` → `Path_optimize_straight_segments`,
  then `UpdateBridgePassability` restore. Heuristic in `AStar_create_node` is `Sqrt_Approx(dx²+dy²)`
  (Euclidean) — matches the Rust `euclidean_heuristic` (helpers study §4, verified).

### 3.2 Hierarchical Dijkstra cost (Zone_precheck) — verified

Per candidate zone-edge, `Zone_precheck` accumulates:
`cost = zoneBaseCost[reducedType] + parent_g + ftol(slope) + diagStep`, where
- `zoneBaseCost` = table `0x007E3794 = {1.0,0.0,0.0,1.0,1.0,0.0,1.0,1.0}` (indexed by the zone record's
  reduced type at record `+0x1c`) — a **different table** from the cell-A* cost table `0x0081870C`.
- `diagStep` = `0.001` if the edge's diagonal byte (`local_3c[1]`) is nonzero, else `0.0`
  (`_DAT_007e3818`/`_DAT_007e2800`).
- `slope` = `ftol(Zone_Estimate_Slope_Cost(ctx, level, cur, next))` added **only** when the mover is
  non-null and `Get_Slope_Speed_Factor() > 1e-05` (`(float10)1e-05 < fVar26`, verified inline). Level 2
  skips the current-zone marker guard (`if (local_38==2) local_2c=0`).
- Passability gate identical to cell A*: `g_PassabilityMatrix[movementZone*8 + reducedType] == 1`.
This is a **pure uniform-cost Dijkstra with NO distance heuristic** — the corridor is shaped only by
zone base cost + slope + a 0.001 diagonal tiebreak. Tie order is min-heap by accumulated cost;
insertion order breaks equal costs (zone id is not a tie key). The chosen chain is written to
`this+0xBC + level*1000` with count at `this+0xC74 + level*4`, and the marker array
`this+0x40 + level*4` is stamped with the epoch at every chosen-chain zone + both endpoints (this is
what the cell loop's level-0 gate reads).

---

## 4. Unreachable detection — zone-disconnected vs blocked

Two distinct rejection layers, verified:

1. **Zone-disconnected (cheap, pre-A*):** `MapClass__Can_Reach_Zone @ 0x0056D100`. If
   `movementZone == -1` → always reachable (no zone constraint). Otherwise, after in-playfield/diamond
   bounds handling, the decision is literally `return GetZoneID(dst) == GetZoneID(src)` — **same
   level-0 zone id ⇒ reachable, different ⇒ unreachable.** This is the O(1) island test: a land unit
   ordered to a disconnected island fails here without any cell search. Rust models this as "different
   zone ⇒ return None immediately" (`zone_search.rs:5-6`), which is the right shape but keyed on the
   port's direct zone_ids rather than the two-level nodeIndex lookup.
   Note: `Can_Reach_Zone` is a *sibling* gate (used by order validation / `CellRect` checks); inside the
   pathfinder itself the equivalent is the `srcZone == dstZone` branch in `AStar_pathfind_search` (§1.5)
   plus the cross-zone hierarchy hard-gate.
2. **Corridor-unreachable (hierarchical):** cross-zone with `allowHS` set — if `Zone_precheck` cannot
   produce a zone chain (returns 0), `AStar_pathfind_search` returns no-path **before** running cell A*
   (the `else if (allowHS) return 0` branch + the retry-loop `Zone_precheck==0 ⇒ return` guard).
3. **Blocked-but-connected (expensive, cell A*):** same-zone (or hierarchy-degraded) searches run
   `AStar_main_loop`. "Unreachable" here means the open set empties or the iteration cap (65527) is hit
   without popping the goal → `ret==0`, driving the retry loop (edge exclusion + re-precheck). After the
   attempt budget (5 default) is exhausted the caller stops the unit / scatters. So a destination that is
   *terrain-connected but fully walled by units* is discovered as unreachable only after the full retry
   budget, whereas a *different-zone island* is rejected instantly by the zone test.

---

## 5. Corrections & contradictions found

- **`_system.md` "iteration cap 10000 vs 65527 = most significant parity bug (row 28)" is STALE/WRONG.**
  Live `AStar_main_loop @ 0x00429A90`: `if (param_6 < 0) param_6 = 0xFFF7` → default loop cap **65527**.
  The Rust `MAX_SEARCH_NODES = 65_527` (`core.rs:109`, comment "Original engine uses 65,527 (0xFFF7)")
  **MATCHES**. `10000` is only an exact-equality reconstruction-failure sentinel (`local_34 != 10000`),
  not the loop bound. `PATHFINDING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md §4` already carries the
  corrected reading ("node cap matches 0xFFF7"); `pathfinding/_system.md:310` and its parity row 28 /
  hazard #1 were not updated and should be demoted. (Evidence: `decompile_function 0x00429A90`.)
- **`fn-astar_main_loop.md`/`_system.md` "10000-node cap" phrasing** should be reworded to
  "default 65527 loop cap; 10000 is a separate reconstruct sentinel" to avoid re-introducing the wrong
  parity row.

Everything else I verified this session (retry budget, exclusion-vector clear, zone-ID equality gate,
two-level nodeIndex storage, 13×8 passability matrix values, uniform-cost corridor Dijkstra, level-0
marker gate + `+0x122` exception, dual closed lists, Euclidean heuristic) is **consistent** with the
existing `ghidra/verified` corpus.

---

## 6. Rust Implementation Handoff

The Rust A* **spine** (`core.rs::astar_search`) is faithful: dual ground/bridge closed lists, node cap
65527 (correct — see §5), Euclidean heuristic, 24-step segments, dir-tiebreak epsilons, level-0
`HierarchyGate` + `BlockerNeighborCounts` (`+0x122` exception modeled). The gaps are in **how the
corridor is chosen (`zone_search.rs`), how zones are stored/built (`zone_map.rs`/`zone_build.rs`), and
the entity-cost softness** — the four `mod.rs:6-9` TODO items.

### What the Rust corridor-Dijkstra approximation gets wrong vs stock

| # | Rust surface | Stock behavior (verified) | Drift | Player-visible when |
|---|---|---|---|---|
| G1 | `zone_search.rs:626 find_zone_corridor` uses **centroid-Manhattan edge cost** (a distance pull) | `Zone_precheck` is **pure uniform-cost Dijkstra, NO distance heuristic**; cost = `zoneBaseCost[0x7e3794] + parent_g + ftol(slope) + 0.001·diag` | Rust injects a distance estimate the binary lacks AND omits per-zone base cost + slope + 0.001 diagonal tiebreak → **different corridor choice** among equal-length routes | Long cross-zone move around terrain: Rust may pick a geometrically-straighter corridor where gamemd picks a lower-base-cost one |
| G2 | `zone_search.rs` corridor = `AStarOptions::corridor` BTreeSet + `expand_corridor()` one-ring widening | Binary stamps a **per-zone marker array** (`this+0x40[zoneL0]==epoch`) consumed by the cell loop's level-0 gate; **`expand_corridor` is not a binary behavior** (helpers study OQ-7) | The one-ring widening is an invented relaxation; the marker path exists (`HierarchyGate`) but runs in parallel with the legacy corridor set | Corridor-edge cells: Rust admits a 1-ring halo gamemd would prune (or vice-versa), changing detour shape near chokes |
| G3 | Zone slope cost — **absent** | `Zone_precheck` adds `ftol(Zone_Estimate_Slope_Cost·factor)` per edge when mover factor > 1e-05 | Entire helper missing → sloped maps route without slope deprioritization | Only for movers with `ThreatAvoidanceCoefficient>0` (some harvesters) on sloped terrain — narrow |
| G4 | `zone_map.rs:44` stores **direct `zone_ids` per cell** per MovementZone | Cell → `MapClass+0x68` nodeIndex → `MapClass+0x18[mz]` zoneId (§2.1) | Structurally different; result-equivalent for reachability but nodeIndex identity across MovementZones is lost | Only if incremental bridge/terrain rebuild relies on shared nodeIndex — not in normal play |
| G5 | `zone_build.rs:227 flood_fill_node_index` = 8-neighbor + class + `abs(Δground_level) < 2` | Binary flood-fill links cells passing `matrix[mz][zoneType]==1`; node/zone assignment via `RecalcZoneType` priority cascade | Rust's height-delta gate is a proxy; edge cells (railroad, terrain-objects RTTI 0x24, gate/wall buildings) may get a different reduced-zone-type → different zone boundaries | Runtime terrain change (bridge destroy) re-derives zones; static WAE maps mask most of it |
| G6 | `core.rs:1184-1203` folds codes 1/4 into **hard** `entity_blocks` BTreeSets | Codes 4/5/6 are **soft** — `if(code<7)` expands at table cost (60/20/8); only `>=7` rejects | Rust hard-blocks code-4-equivalents → a unit boxed by stationary friendlies reports "no path" where gamemd finds a high-cost path-through | Dense-traffic / surrounded-unit scenarios — common |
| G7 | `core.rs:118 CLIFF_COST_MULTIPLIER` triggers on **inter-cell height delta** | ×4 triggers on `cell+0x140 & 0x40000` — a **dynamically XOR-toggled bridge-peer marker** set by `UpdateBridgePassability`, not static cliff terrain | Wrong trigger condition AND the whole `UpdateBridgePassability` peer-marking lifecycle is missing | Bridge approaches + multi-unit traffic biasing — frequent on bridge maps |
| G8 | Cross-zone hierarchy fall-through partial (`can_reach_same_or_zoned`, `can_use_reduced_zone_precheck`) | Same-zone precheck-fail ⇒ **clear hierarchy, run flat A***; cross-zone hierarchy-on ⇒ **return no-path before cell A*** (§1.5) | If Rust doesn't reproduce the asymmetry, unreachable cross-zone targets burn all 5 retries in corridor mode where gamemd hard-rejects early (or vice-versa) | Ordering a unit to an unreachable cross-zone island — responsiveness/stop timing |

### Decodable NOW vs needs deeper work

- **Decodable now (numeric/structural, verified addresses in hand):**
  - G1 corridor cost formula — swap centroid-Manhattan for `zoneBaseCost[0x7e3794]` table + 0.001
    diagonal + uniform parent_g (drop the distance pull). Table bytes verified.
  - G6 soft codes 4/5/6 — change hard `entity_blocks` to soft edge costs 60/20/8 with `code>=7` reject.
    Cost table `0x0081870C` verified bit-exact.
  - `_system.md`/parity row-28 correction — mechanical doc edit (§5).
  - G4/G5 zone storage — the two-level nodeIndex model (`0x0056D230`) and the flood-fill legality
    (`matrix[mz][zoneType]==1`) are both verified; a faithful rebuild is decodable, though large.
- **Needs deeper work before authoritative:**
  - G3 slope cost — `Zone_Estimate_Slope_Cost @ 0x00585f40` level-1/2 formulas are doc-sourced; the
    slope-context writer lifecycle (`Foot+0x21C`, `+0x57E4`/`+0x59F0`) is a deferred open question
    (helpers study P0 gate). Do not make slope authoritative until that writer is verified.
  - G7 `UpdateBridgePassability` peer-marking (`0x0042ACF0`) + `FindNearbyBridgePeer` 5×5 fallback
    (`0x0042B080`) — a coupled bridge subsystem; the `0x40000` marker cannot be ported independently.
    `object+0x674` subobject identity is unresolved (OQ-16).
  - G2 marker-array handoff — retire `expand_corridor`; make the `HierarchyGate` marker path the sole
    corridor mechanism. Requires the corridor-stamp (`this+0x40` epoch array) to be the authoritative
    output of the Rust `Zone_precheck` equivalent, not a parallel BTreeSet.

### Acceptance scenarios (long-range path-shape that currently differs)

1. **Cross-zone equal-length corridor pick (G1/G2):** two zone chains of equal cell-length between a
   land unit and a target separated by a wall gap. gamemd picks the chain with lower summed
   `zoneBaseCost + 0.001·diag`; Rust picks the geometrically straighter (centroid-Manhattan) chain.
   Acceptance: identical chosen zone sequence for a fixture with `zoneType`-varied corridors.
2. **Surrounded unit path-through (G6):** a Grizzly ringed by stationary friendly units, target one
   cell outside the ring. gamemd returns a high-cost path threading between friendlies (code-6 = 8.0
   soft); Rust returns None (hard block) → unit stops. Acceptance: Rust yields a path with the same
   step count at code-6 cost.
3. **Unreachable island stop timing (G8):** land unit ordered to a disconnected island. gamemd rejects
   at the zone test / cross-zone hard-gate (near-instant stop, no retries); if Rust runs corridor
   retries it stops several ticks later. Acceptance: no cell-A* attempt is made when src/dst level-0
   zones differ and hierarchy is on.
4. **Bridge-approach biasing (G7):** two units cross a bridge in traffic. gamemd's `UpdateBridgePassability`
   toggles `0x40000` on peer-path cells so the second unit's A* biases ×4 away; Rust penalizes static
   height-delta cells instead → different spread. Acceptance: second unit's path avoids the first's
   marked cells, not the ramp cells.

### Remaining uncertainty (verified vs pending)

- **Verified this session:** orchestrator dispatch/retry (`0x0042C900`), corridor Dijkstra structure &
  cost table (`0x0042C290`, `0x007E3794`), regular-loop neighbor/marker/cost mechanics (`0x00429A90`),
  reachability gate (`0x0056D100`), two-level zone storage (`0x0056D230`), passability matrix values
  (`0x0082A594`).
- **Doc-sourced (not re-decompiled this session — verify before load-bearing):** `RecalcZoneType`
  priority cascade (H8), `Zone_Estimate_Slope_Cost` level-1/2 formulas (H15), `UpdateBridgePassability`
  /`FindNearbyBridgePeer` bridge-marker lifecycle (R6/H17), `InvalidateZoneEdge` common-neighbor append
  model, `UpdateHierarchicalEdges` failed-A* edge selection. These have dedicated `ghidra/verified`
  docs in `docs/research/pathfinding/`; extend those, do not redo.
- **Open in the corpus:** slope-context writer lifecycle (`Foot+0x21C`); `object+0x674` subobject
  identity for the bridge-peer footprint predicate.

