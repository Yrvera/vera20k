# Bridge Parity Scan — Facet: zone-connectivity (cell/zone connectivity & endpoint records)

Scope: `BridgeEndpointRecord`, `compute_bridge_endpoints`, `refresh_endpoint_active_flags`,
group BFS in `src/sim/bridge_state/mod.rs`; `refresh_bridge_zones_if_dirty` in
`src/sim/world/bridge_orchestrator.rs`. gamemd side: `ComputeBridgeZones@0x0056d6e0`,
`FindBridgeRecord@0x0056da10`, `InvalidateBridgeZones@0x0056dae0`,
`ValidateBridgeZones@0x0056db70`, `UpdateBridgeZonesHelper@0x0056c510`,
`AddBridgeZoneEdges@0x005851b0`, `RemoveBridgeZoneEdges@0x00584e50`.

All seven anchor addresses re-confirmed live this session (get_function_by_address /
decompile_function each resolved to the named MapClass function). Docs in
`02-cell-state-layering-zones/` match the binary on the 16-byte layout (high=0/low=1 kind,
+0x08 intact byte, FindBridgeRecord kind!=0 skip, all-active loop ignores kind).

---

### D1: Endpoint deactivation is permanent — engineer/CABHUT bridge repair never re-activates the zone record

- Rust now: `refresh_endpoint_active_flags` (mod.rs:1587–1603) only ever sets
  `record.active = false`. Its own docstring states "Deactivation is one-way (no
  re-activation)." The only caller is `refresh_bridge_zones_if_dirty`
  (bridge_orchestrator.rs:1135–1150), and the engineer/hut **repair** path
  (`repair_bridge_from_engineer_scan` via world_orders.rs:379, `RepairOutcome.zones_dirty`)
  rebuilds the PathGrid + zone grid but there is no code anywhere that flips an
  `endpoint_records[*].active` back to `true`. Grep confirms: the only write to
  `.active` outside construction is the `false` assignment at mod.rs:1600.
- gamemd: repair re-activates the record. `ProcessBridgeDestruction_High@0x00573540`
  (the restore/repair walker; despite the name it handles the repair branch reached via
  `RepairBridge_High@0x0057f440`, whose only caller is this function) calls
  `MapClass__ValidateBridgeZones(&local_40)` at two sites. `ValidateBridgeZones@0x0056db70`
  loops `FindBridgeRecord(coord, tol=3, start)` over high records and, for each record with
  `+0x08 == 0`, sets `+0x08 = 1` and calls `AddBridgeZoneEdges(record)` — re-inserting the
  directed zone edges. If any newly-validated record is not yet reachable it sets the
  return flag, and the caller then runs `UpdateBridgeZonesHelper` (full rebuild). So a
  repaired high bridge becomes a connected zone edge again.
- Fixture: high bridge group_id=1, record endpoints (0,0)/(4,0), `active=true`. Tank shell
  collapses body cell (2,0) → `Destroyed`; `refresh_endpoint_active_flags` flips
  `records[0].active=false` (test `refresh_endpoint_active_flags_deactivates_on_first_destroyed_cell`
  at tests.rs:430 proves this). Engineer enters CABHUT, repair walker sets cells (1,0)/(2,0)/(3,0)
  back to `Healthy` (overlay 0xCD…). gamemd: `ValidateBridgeZones` flips the record's +0x08
  back to 1 and `AddBridgeZoneEdges` restores ground↔ground edges → cross-bridge pathing
  works again. Rust: `records[0].active` stays `false` forever; even though
  `from_resolved_terrain_with_bridges` re-reads cell walkability, the zone-edge filter
  `bridge_record_matches` (zone_build.rs:64–70) requires `record.active`, so the repaired
  bridge is never re-added as a long-range zone edge.
- Player sees: after repairing a bridge, units' long-range A* routing across that bridge can
  remain broken (units detour the long way or refuse to cross) until a full map zone rebuild
  happens to recreate edges from terrain. Triggers in any skirmish where a player has an
  engineer + CABHUT and repairs a previously-collapsed bridge — a normal mid/late-game move.
- Severity: HIGH (player-visible pathing regression on a deliberate, common repair action)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00573540` (two `MapClass__ValidateBridgeZones` calls
  + conditional `MapClass__UpdateBridgeZonesHelper`); `decompile_function 0x0056db70`
  (`+0x08 = 1`, `AddBridgeZoneEdges`); `get_function_callers 0x0057f440` → only
  `ProcessBridgeDestruction_High`.

---

### D2: Deactivation granularity is whole-group, not per-record geometric (tolerance-3) match

- Rust now: `refresh_endpoint_active_flags` (mod.rs:1587–1603) collects every
  `bridge_group_id` that has ANY `Destroyed` cell, then deactivates **every** endpoint
  record whose `record.group_id` is in that set. Group = the BFS-connected blob of all
  bridge-deck cells (Pass 1, mod.rs:564–624) joined by 4-cardinal adjacency, regardless of
  how long the bridge is or how far the impact was from a given record's endpoints.
- gamemd: `InvalidateBridgeZones@0x0056dae0` deactivates records selected by
  `FindBridgeRecord(impactCoord, tol=3, start)` — only records whose endpoint-line passes
  within Manhattan distance 3 of the damage coordinate (geometric, per-record), and only
  high records (`+0x0C == 0`). It flips just those records' `+0x08` to 0 and calls
  `RemoveBridgeZoneEdges` on each. Records on the same `MapClass` array that are far from the
  impact are left intact=1 and keep their zone edges.
- Fixture: a long N-S high bridge spanning Y=0..30 in one X column. gamemd's
  `ComputeBridgeZones` emits ONE record per bridge tile run (endpoint_a = each bridge cell's
  own coord, endpoint_b = far end), so a long bridge has many records sharing the geometry;
  a hit at (x, 5) invalidates only records whose endpoint line is within 3 cells of (x,5).
  Rust models the whole bridge as a single group with (typically) a single endpoint record,
  so the first `Destroyed` cell anywhere along the span deactivates the entire span's single
  record. For the common single-record case the observable end state (whole bridge severed
  on first collapse) is the same, but the record *count and selection mechanism differ*; on
  a map where one BFS group fuses two physically distinct bridges (e.g. an X/Y junction of
  decks sharing a cell), Rust would deactivate both arms on a hit to one arm, whereas gamemd
  invalidates only the arm within tolerance 3.
- Player sees: on bridge layouts where two spans share a connected deck blob, destroying one
  span can incorrectly mark the other span impassable for routing in Rust. Rare on stock YR
  maps (most are isolated linear bridges); fires only on multi-arm/junction bridge geometry.
- Severity: LOW (correct on the common linear bridge; diverges only on shared-deck junctions,
  uncommon in stock skirmish maps)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0056dae0` (`FindBridgeRecord(...,3,0)` loop +
  per-record `RemoveBridgeZoneEdges`); `decompile_function 0x0056da10` (geometric
  vertical/horizontal between-endpoints + `abs(delta) <= param_3` match).

---

### D3: Low-bridge (tube) records are never deactivated on collapse

- Rust now: `refresh_endpoint_active_flags` keys deactivation off `cell.bridge_group_id`
  (mod.rs:1592) and the per-cell `damage_state == Destroyed`. Low-bridge tube endpoint
  records are built in `compute_low_bridge_tube_endpoints` (mod.rs:1720–1772) with
  `bridge_kind = Low` and a `group_id` taken from the runtime cell's `bridge_group_id`. But
  low-bridge collapse in the dispatcher writes `damage_state` on the low-overlay cells, and
  the record→group linkage for tube records depends on the runtime cell having had a
  `bridge_group_id` from Pass-1 deck BFS. Low-bridge tube cells are not deck cells
  (`has_bridge_deck=false`, registered as bridgeheads with `bridge_group_id = None` in Pass
  4, mod.rs:739–751), so a low tube record's `group_id` frequently has no `Destroyed`-cell
  member to match → the low record stays `active = true` after the tube/low bridge is
  destroyed.
- gamemd: low collapse uses the same `InvalidateBridgeZones`/`UpdateBridgeZonesHelper` chain;
  however the all-active zone rebuild loop in `UpdateBridgeZonesHelper@0x0056c510` includes
  every record with `+0x08 != 0` **regardless of kind** (it does NOT test +0x0C). So a low
  record that is still intact=1 keeps contributing its endpoint cluster pair. The mechanism
  by which a destroyed low bridge stops contributing is the per-cell zone-type/cluster
  changing under it (tube cell no longer LandType 10 / passable), which is recomputed during
  the rebuild — the record stays in the array but its endpoints resolve to non-connected
  clusters. (See LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT §3.7: "normal low collapse
  does not delete TubeClass records.")
- Fixture: low bridge tube run X=0..4 at Y=0, one Low record (0,0)/(4,0), group from a
  Pass-1 deck BFS member. Destroy the low overlay span. If the low cells were registered as
  bridgeheads (group_id None) the Rust group set never contains the record's group → record
  stays `active=true` → zone_build keeps adding the cross-tube edge even though the tube is
  gone. gamemd severs the connection because the underlying cell zone-type/cluster recompute
  makes the endpoint clusters unreachable.
- Player sees: amphibious/infantry units may still be routed across a destroyed low bridge in
  Rust. Frequency depends on whether maps with destroyable low bridges + collapse occur;
  low-bridge collapse is itself uncommon in stock skirmish, so low frequency, but the
  resulting "ghost connectivity" is a clear correctness defect when it does occur.
- Severity: MED (correctness defect — routing over a destroyed crossing — but on the
  lower-frequency low-bridge collapse path)
- Confidence: LIKELY-DRIFT (the Rust group→record linkage for low tube records is
  fragile and the deactivation path is built around deck-cell damage_state; needs a
  fixture run to confirm a specific map produces the stale `active=true`)
- Verify-call: `decompile_function 0x0056c510` (all-active loop tests only `+8 != 0`, no
  `+0x0C` test); doc `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md` §3.7.

---

### D4: Endpoint pair geometry uses max-Manhattan ground-neighbor heuristic, not the binary's perpendicular-walk + opposite-step

- Rust now: `compute_bridge_endpoints` (mod.rs:1648–1718) collects all ground cells
  cardinally adjacent to the bridge group, then picks the **pair with maximum Manhattan
  distance** as endpoint_a/endpoint_b. Activeness, group, kind set from there.
- gamemd: `ComputeBridgeZones@0x0056d6e0` derives endpoints structurally: endpoint_a =
  the *bridge cell's own* MapCoord (`*puVar9 = uVar2` where uVar2 = this cell's MapCoord_X);
  endpoint_b = the coord of the cell one opposite-direction step past the far bridge end
  (`Pathfinding_update_continued(iVar7-4 & 7)` then read its `+0x24`). The walk direction
  comes from per-tile orientation tables `DAT_0082A734`/`DAT_0082A774`. The endpoints are
  thus the two ground/landing coords aligned with the bridge's physical orientation axis,
  one record per qualifying bridge tile — NOT the geometrically-farthest pair of any adjacent
  ground cells.
- Fixture: an L-shaped or diagonally-fringed bridge whose adjacent ground cells include a
  protruding ground cell off-axis. gamemd picks the two on-axis landing cells via the
  orientation walk; Rust's max-Manhattan picks the off-axis protrusion if it is farther apart
  in Manhattan terms, yielding a different endpoint pair → different zone clusters joined.
  For a clean straight bridge with symmetric landings the two methods coincide (test
  `bridge_endpoints_detected` passes with (0,0)/(4,0)), so happy-path is equal; the
  divergence is at irregular bridge fringes.
- Player sees: on irregular bridges the wrong two ground zones get joined, so cross-bridge
  routing can connect/sever the wrong sides. Frequency low on stock symmetric bridges.
- Severity: LOW (coincides on straight/symmetric bridges; diverges on irregular fringe
  geometry, uncommon in stock maps)
- Confidence: PROVEN-DRIFT (different selection algorithm; equality only proven for the
  symmetric happy path, not across all bridge shapes)
- Verify-call: `decompile_function 0x0056d6e0` (endpoint_a = this cell MapCoord at 0x56D9AE;
  endpoint_b from opposite-direction step `Pathfinding_update_continued(iVar7-4 & 7)` read
  `+0x24`); doc BRIDGE_ZONE_LIFECYCLE §1.1.

---

### D5: One record per group, vs one record per bridge tile (record count / FindBridgeRecord index space)

- Rust now: `compute_bridge_endpoints` pushes ONE `BridgeEndpointRecord` per BFS group
  (mod.rs:1657–1707) (plus low records one per accepted low-cell-with-opposite-neighbors,
  de-duplicated by ordered endpoint key, mod.rs:1759–1762).
- gamemd: `ComputeBridgeZones` pushes a record for **every** bridge tile whose height
  matches the orientation table and whose perpendicular walk finds the far end
  (`*(int *)(param_1+0x60) += 1` per accepted cell). A 5-cell-long, 3-wide high bridge
  therefore produces many records (each body tile that passes the height/walk gate), all
  high (+0x0C=0), with overlapping geometry. `FindBridgeRecord` then matches any of them
  within tolerance 3 of an impact.
- Fixture: 3-wide × 5-long high bridge. gamemd: up to ~15 records (one per qualifying tile);
  Rust: 1 record for the whole group. `Invalidate`/`Validate` in gamemd flip the +0x08 of
  *each* matched record independently; the all-active rebuild folds each into the zone graph,
  but because they share endpoints the net connected-cluster result is the same as a single
  edge. The observable zone connectivity (which two ground clusters are joined) is identical
  for a single physical bridge, so this is internal *unless* combined with D2 (geometric
  per-record invalidation differs) — listed separately because the record count itself is a
  state-layout difference and the per-record intact toggling is the mechanism D1/D2 depend
  on.
- Player sees: nothing directly from the count alone on a single bridge; matters only as the
  substrate for D1/D2 divergences.
- Severity: LOW (state-layout difference; observable only via D1/D2 interactions)
- Confidence: PROVEN-DRIFT (record cardinality differs by construction)
- Verify-call: `decompile_function 0x0056d6e0` (per-cell `*(param_1+0x60)+=1` push inside
  the CellIterator loop, no per-group dedupe).

---

## PARITY-CONFIRMED

- **High=0 / Low=1 kind encoding.** Binary: high path writes `puVar9[3] = 0`
  (`ComputeBridgeZones` 0x56D9BF), low path writes `puVar9[3] = 1` (0x56D7C8). Rust:
  `BridgeRecordKind::High`/`Low`, `is_high()` returns `bridge_kind == High`
  (mod.rs:495–529). Matches. (decompile_function 0x0056d6e0)
- **16-byte record layout semantics.** +0x00 endpoint_a, +0x04 endpoint_b, +0x08 intact,
  +0x0C kind. Rust mirrors with `endpoint_a`, `endpoint_b`, `active` (bool intact),
  `bridge_kind`. Field meanings match the verified writers. (decompile_function 0x0056d6e0,
  doc §1.1)
- **FindBridgeRecord skips non-zero kinds (high-only).** Binary `if (puVar5[3] == 0)` gate
  (0x56DA3A). Rust models this with `BridgeRecordFilter::HighActiveOnly → record.is_high()`
  (zone_build.rs:60–69). Matches the high-only lookup semantic. (decompile_function 0x0056da10)
- **All-active zone rebuild ignores kind.** Binary `UpdateBridgeZonesHelper` all-active loop
  tests only `*(char*)(record+8) != '\0'`, never +0x0C, so intact low records participate.
  Rust `BridgeRecordFilter::AllActive → true` (only `record.active`), so low records
  participate (zone_build.rs:57–67). Matches. (decompile_function 0x0056c510)
- **Records never removed — only intact toggled.** Binary keeps the array for map lifetime;
  Invalidate/Validate flip +0x08. Rust keeps `endpoint_records` Vec for map lifetime and
  flips `.active` (mod.rs:540, 1598–1602). Matches the persistence model (modulo D1's missing
  re-activation). (decompile_function 0x0056dae0 / 0x0056db70; doc §2.3)
- **Group BFS uses 4-cardinal adjacency.** Rust Pass-1 BFS uses `cardinal_neighbors`
  (mod.rs:613–619). gamemd has no "group" object (it uses per-tile records), so this is a
  Rust-native grouping with no binary counterpart to diverge from; its only observable use
  is endpoint-record construction, covered by D4/D5.
- **Low record requires opposite-neighbor low-bridge pattern + valid tube.** Rust
  `has_opposite_low_bridge_tube_neighbors` (E/W or N/S) + `tube_at_cell`
  (mod.rs:1729–1746) mirrors `ComputeBridgeZones` low branch (E,W then S,N probe via
  `Pathfinding_update_continued` + `GetTubeAtCell` + `IsLowBridgeCell`). Pattern matches.
  (decompile_function 0x0056d6e0 low branch; doc LOW_BRIDGE_ZONE_PRECHECK §3.3)
- **zones_dirty → refresh + PathGrid + zone-grid rebuild ordering.** Orchestrator runs
  `refresh_endpoint_active_flags` → `from_resolved_terrain_with_bridges` → `rebuild_zone_grid`
  (bridge_orchestrator.rs:1135–1150), matching the binary's Invalidate(remove edges) →
  UpdateBridgeZonesHelper(full rebuild) ordering for the deactivation direction.

## UNCHECKED

- **Exact `is_intact=0` at-map-load case** (gamemd sets +0x08=0 when the bridge body is
  structurally broken at load: `if ((Flags & 0x100)==0) uStack_31 = 0`). Rust always
  constructs records with `active = true` and derives initial damage from
  `initial_bridge_damage_state`. Whether a map authored with a pre-broken bridge body yields
  a load-time inactive record in Rust was not traced — would need a fixture with an
  author-damaged mid-span. Could be a 6th disparity (load-time active flag) but unconfirmed.
- **`AddBridgeZoneEdges`/`RemoveBridgeZoneEdges` per-direction edge insertion detail.** The
  binary inserts/removes directed edges across 3 hierarchy levels (stride 0x18) using
  orientation table `DAT_0082A944` to add two perpendicular off-axis edge pairs. Rust's
  `zone_build` injects bridge adjacency differently (full zone rebuild from records, not
  incremental directed-edge insert/remove). Whether the *resulting* connected components are
  identical for all bridge orientations was not exhaustively verified — only the
  active/kind filter that feeds the rebuild was checked. The incremental-vs-rebuild
  divergence is documented as a known partial in BRIDGE_ZONE_INCREMENTAL_REFRESH (deferred).
- **Whether `repair_bridge_from_engineer_scan` sets `RepairOutcome.zones_dirty` AND whether
  the world tick actually re-runs a full zone rebuild after repair** (world_orders.rs:379).
  D1 establishes there is no re-activation of `.active`; not separately confirmed whether the
  post-repair PathGrid rebuild alone (without record re-activation) recreates long-range
  edges from terrain — if `from_resolved_terrain_with_bridges` recreates bridge walkability
  but `zone_build` gates the long-range zone edge on `record.active`, the edge stays missing
  (the basis for D1's HIGH severity), but a runtime fixture run would make this conclusive.
