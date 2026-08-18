# Zone-Incremental Algorithm Divergence — Ghidra vs Rust (2026-04-24)

Compares gamemd's per-cell `AssignOrphanedCellZone` /
`MergeAdjacentCellZone` incremental zone repair against Rust's
bbox-based `zone_incremental::try_incremental_update`. Identifies
specific scenarios where the two diverge and what that implies for
parity.

**Confidence:** HIGH — both algorithms freshly re-read.

**Active in YR:** Yes — triggered on building placement/removal,
bridge destruction/repair, and other cell-passability changes.

---

## 1. The two algorithms, side by side

### Gamemd — per-cell incremental

Two entry points for single-cell topology changes:

**AssignOrphanedCellZone** (`0x56D460`) — called when a cell
transitions from impassable/unknown into "needs a zone":
```
1. Read cell's zone_cell_data[cell] at MapClass+0x68:
     (u8 zone_type, u8 height, u16 cluster_id)
   stride = map.size_width + 1 + map.size_height
   Neighbor offsets (in zone_cell_data entries):
     [-stride, 1-stride, 1, 1+stride, stride, stride-1, -1, -1-stride]

2. If cell.zone_type == 7 (impassable sentinel): return (no zone needed)

3. For each of 8 neighbors (scanned in order):
     if neighbor.zone_type == 0 (also unassigned):
         # Count how many of the 8 neighbors have DIFFERENT zone IDs.
         # Uses zone_ids[0][cluster_id] — first MovementZone's mapping.
         conflict_count = 0
         ref_zone = 0
         for each of 8 neighbors:
             neighbor_zone = zone_ids[0][neighbor.cluster_id]
             if neighbor_zone != ref_zone AND neighbor.zone_type != 7:
                 conflict_count++
                 ref_zone = neighbor_zone
         if conflict_count < 4 AND cell.zone_type == 0:
             # Low topology complexity → safe to adopt
             cell.cluster_id = neighbor.cluster_id
             return
         # Otherwise: fall through to full rebuild
         break

4. No neighbor matched OR too many conflicts:
     MapClass::UpdateBridgeZonesHelper (full rebuild)
```

**MergeAdjacentCellZone** (`0x56D5A0`) — called when a cell's
type matches a neighbor's type (should merge):
```
Same as AssignOrphaned but the inner match is:
     if neighbor.zone_type == cell.zone_type:
         ... same conflict count ...
         if conflict_count < 4:
             cell.cluster_id = neighbor.cluster_id
             return
         break
```

**Key properties:**
- Both are **single-cell** operations. One call per changed cell.
- Adoption only touches the target cell's `cluster_id` (2 bytes in
  zone_cell_data). Zero other writes.
- The 13 `zone_ids[MovementZone][cluster_id]` arrays are **not**
  updated. They retain whatever mapping the prior full rebuild
  computed. The new cell inherits that mapping implicitly by
  adopting the neighbor's cluster_id.
- `UpdateBridgeZonesHelper` is the fallback: re-runs flood-fill
  across the entire map, reallocates all 13 zone_ids arrays,
  rebuilds the connection hash.

### Rust — per-batch bbox-reflood

`zone_incremental::try_incremental_update` in
[src/sim/pathfinding/zone_incremental.rs](../ra2-rust-game/src/sim/pathfinding/zone_incremental.rs):

```
Entry: list of changed cells (batch, not per-cell).

1. Bail out to full rebuild if:
     - changed_cells > 200, OR
     - resolved_terrain exists (terrain-aware zoning active), OR
     - any zone_map has zone_count >= 60_000 (u16 exhaustion).

2. Compute bbox of changed cells + 2-cell padding.

3. For each ground MovementZone (iterated independently):
     a. Collect affected_ground = { zid : zone_map[cell].zone_id
        for cell in bbox and zone_id != INVALID }
     b. If affected_ground empty AND no cell newly passable: continue.
     c. Clear ALL cells whose zone_id ∈ affected_ground, map-wide.
     d. Re-flood-fill from zone_count+1 across the whole map,
        assigning fresh zone_ids to cleared+passable cells.
     e. Rebuild extract_adjacency(ground, width, height, new_count).
     f. Inject bridge adjacency if MZ can use bridges.
     g. Rebuild compute_zone_info().
     h. Rebuild SuperZoneMap::from_adjacency().
     i. Rebuild bridge_redirect.

4. True → success; false → full rebuild.
```

**Key properties:**
- **Batch** — all changed cells processed together, once per
  MovementZone.
- The `affected_ground` set is pulled from the bbox but the *clear*
  extends map-wide: any zone whose ID appears in the bbox is cleared
  EVERYWHERE, then refloodfilled from scratch. So unaffected
  topology far from the change may still get re-numbered.
- Adjacency, zone-info, and super-zones are rebuilt per MZ.
- Bridge redirect is rebuilt from scratch each time.

---

## 2. Divergence catalog

### Divergence A — Granularity

| | Gamemd | Rust |
|---|-------|------|
| Unit of work | 1 cell | N changed cells (batch) |
| Call site | One call per changed cell | One call per batch |
| Per-MZ iteration | None (cluster_id shared across MZs) | 13 times per call (one per MZ) |

**Implication:** When demolishing a large building (say 16 cells),
gamemd invokes Assign/Merge up to 16 times, each doing O(1) work
plus 8 neighbor reads, possibly falling through to a full rebuild
on the first cell that fails. Rust gathers all 16 into one call,
computes one bbox, clears affected zones once per MZ, and does 13×
the per-MZ work in one pass.

**Parity impact:** The *timing* of rebuilds differs. Gamemd may do
a full rebuild mid-batch as soon as one cell triggers it; Rust
batches evenly. Observable effect is frame-time jitter profile, not
gameplay state.

### Divergence B — Fallback trigger

| | Gamemd | Rust |
|---|-------|------|
| Trigger | `conflict_count ≥ 4` among 8 neighbors | `len(changed_cells) > 200` |
| Based on | Local topology | Total change count |
| Can adopt a single cell in a high-conflict zone? | No — falls back | Yes — handles in bbox |
| Handles 500-cell mass demolish? | Full rebuild (conflicts trip early) | Full rebuild (threshold) |

**Implication:** A single cell change in a highly fragmented area
(e.g., dense urban map with many small paths meeting at a point)
will trigger gamemd's full rebuild but not Rust's. Conversely, 150
scattered single-cell changes in open terrain are incremental in
Rust but cause 150 separate gamemd incremental calls, each
potentially succeeding cheaply.

**Parity impact:** Different full-rebuild cadence. In a
fortification-heavy mid/late game, gamemd hits full rebuilds more
often; Rust glides until the threshold is hit.

### Divergence C — Scope of in-place edit

| | Gamemd | Rust |
|---|-------|------|
| Cells touched per successful incremental | 1 (just cluster_id) | All cells whose zone_id is in `affected_ground` |
| Zone-id map updated? | No (uses stale mapping) | Yes, refloodfilled |
| Adjacency updated? | No | Yes |
| Super-zones updated? | N/A (no super-zones) | Yes |
| Bridge redirect? | N/A | Yes |

**This is the biggest semantic divergence.** In gamemd, an
incremental success leaves the `zone_ids[MZ][cluster_id]` mapping
unchanged. The cell inherits the neighbor's cluster, which already
mapped to a valid zone ID via the last full rebuild. So pathfinding
queries return coherent results.

In Rust, every successful incremental *also* rebuilds adjacency and
super-zones for the affected MZs. This is more work but guarantees
all downstream data structures are in sync.

**Parity impact:** Gamemd pathfinding may (in theory) cache zone
reachability answers from before a cell change. Because the
zone_ids mapping stays the same, those cached answers remain valid
until the next full rebuild. Rust invalidates more aggressively.

Neither approach is "wrong"; they're different correctness↔cost
tradeoffs.

### Divergence D — Conflict heuristic

Gamemd counts "distinct zone IDs among the 8 neighbors" using only
MovementZone 0's mapping, and fails if ≥4. Rust has no equivalent
per-cell heuristic — its threshold is purely batch-size.

**Implication:** Rust might succeed on a single-cell change where
gamemd falls back. In practice this is usually fine — Rust's bbox
clear-and-reflood is correct either way. But the *pattern* of which
changes trigger full rebuilds will differ.

**Concrete scenario:**
- A cell sits at the junction of 5 distinct zones (unusual but
  possible in a maze-like map).
- One of its 4 cardinal neighbors becomes passable.
- Gamemd: `conflict_count = 5 ≥ 4` → full rebuild.
- Rust: 1 cell changed, within threshold → bbox reflood succeeds.

Both produce correct zones after the update. Gamemd does more work;
Rust does less. No observable gameplay difference unless the two
different output zone-ID assignments happen to affect AI pathing
tie-breakers (very unlikely).

### Divergence E — MovementZone scope

Gamemd's incremental touches only `cluster_id` in zone_cell_data —
which is *shared* by all 13 MovementZones. The per-MZ mapping in
`zone_ids[MZ][cluster_id]` is pre-computed and reused.

Rust iterates each ground MovementZone independently, rebuilding
per-MZ zone IDs. Different MZs may end up with different zone-ID
shapes around the change.

**Parity impact:** In gamemd, if cluster_id 7 maps to zone 12 for
MZ=Normal and zone 15 for MZ=Amphibious, adopting neighbor's
cluster 7 means the new cell is part of zone 12 (Normal) AND zone
15 (Amphibious) simultaneously. In Rust, those are separate
decisions per MZ — the new cell might end up in zone 12 of Normal
but a completely different zone ID in Amphibious.

This is fine as long as pathfinding consumers query per-MZ, which
Rust does. But it means Rust's zone IDs aren't globally consistent
across MZs in the way gamemd's cluster model is.

### Divergence F — Iteration order within a batch

Gamemd processes cells in call order (whatever order the higher
layer emits change events). Rust sorts cells into a bbox and
floods in row-major scan order.

**Implication:** Deterministic outputs require a consistent order.
Both approaches are deterministic *within* their model. But if you
replay a gamemd session on the Rust engine, zone ID numerical
values may differ (same connectivity, different labels). If any
persistent structure serializes zone IDs (save files, replay data),
it would fail to round-trip across engines — **but we already know
the Rust engine uses its own save format, so this is only a problem
if we try to hot-swap a gamemd savegame in Rust.**

---

## 3. Scenarios that exercise the divergence

1. **Dense urban single-cell demolish:** Sell a single wall cell in
   a fortified base with many adjacent small passages. Gamemd likely
   hits the 4-conflict fallback → full rebuild. Rust sails through.

2. **Batch 50-cell demolish:** Destroy a 5×10 building. Gamemd: 50
   sequential Assign calls, each re-checking 8 neighbors, possibly
   hitting rebuild early. Rust: one bbox-reflood per MZ, likely all
   ≤13 succeed.

3. **Bridge destruction:** Bridge collapse fires
   `MapClass::UpdateBridgeZonesHelper` directly (see original
   MapClass report §3). Both engines likely do full rebuilds here.
   No divergence.

4. **Sequential builder builds 200 walls over 5 minutes:** 200
   single-cell changes, trickled. Gamemd: 200 independent Assign
   calls, each cheap but summing to real cost. Rust: each batch
   (however the app layer batches) goes through `update_category`
   per MZ. Whichever engine batches less pays more.

5. **Thin-corridor flip:** A corridor one cell wide becomes blocked,
   then unblocked within 1 second. Gamemd: probably two full
   rebuilds (conflicts trip easily in narrow passes). Rust: two
   bbox-reflood batches. Final state identical.

---

## 4. Correctness verdict

Both algorithms produce **correct** zone assignments. They differ
in:
- *When* full rebuilds occur
- *How many* cells get re-numbered during a partial update
- *Which* data structures get refreshed synchronously vs lazily

For a playing user, the primary observable is frame-time jitter
pattern during heavy building/demolish activity. Zone-based
pathfinding answers should be equivalent after the dust settles.

**The potentially visible edge case:** between a gamemd incremental
success and the next full rebuild, pathfinding in gamemd uses the
stale `zone_ids` mapping (which still works because cluster
adoption preserves connectivity). In Rust, each incremental updates
zone_ids+adjacency+super-zones. If the AI queries
"is-zone-A-reachable-from-zone-B" in a tiny window between an
incremental and a full rebuild, gamemd and Rust may return
different answers *in theory*. In practice, both should be correct
(gamemd because the cluster mapping is preserved; Rust because it
rebuilds).

---

## 5. Recommendations (research only, no code changes)

1. **Leave the current Rust behavior.** It's more consistent and the
   parity risk is low. The algorithm divergence won't manifest as a
   player-visible gameplay difference.

2. **If micro-optimizing becomes important**, the gamemd approach
   offers a potential fast-path for isolated single-cell changes:
   skip the bbox flood and just patch the one cell's zone ID. But
   that requires also tracking adjacency-edge deltas, which gamemd
   doesn't (it has no super-zones), so the gamemd shortcut
   doesn't translate directly to Rust.

3. **Deterministic replay:** if cross-engine replay is ever a goal
   (unlikely for this project), zone IDs would need to be mapped
   through an equivalence class, not compared directly.

4. **Future audit target:** trace which Rust sim callers invoke
   incremental updates, and whether they batch or call per-cell. If
   per-cell, the batch-amortization benefit of Rust's algorithm is
   being left on the table.

---

## Sources

### Newly decompiled / re-read
- `0x56D460` `MapClass::AssignOrphanedCellZone`
- `0x56D5A0` `MapClass::MergeAdjacentCellZone`
- `0x56C510` `MapClass::UpdateBridgeZonesHelper` (context)

### Rust files read
- [src/sim/pathfinding/zone_incremental.rs](../ra2-rust-game/src/sim/pathfinding/zone_incremental.rs) — full read

### Doc files referenced
- `MAPCLASS_GHIDRA_REPORT.md` (zone cell data layout, cluster_id
  semantics)
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` (original claim that these
  are "the fast path")
- `ZONE_PASSABILITY_VERIFIED.md` (MovementZone model)
