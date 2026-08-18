# Bridge Parity Scan — Adversarial Verdicts: zone-connectivity

Auditor stance: refute-until-proven. Each gamemd function re-decompiled live this session;
each cited Rust line re-read live. Verdicts below.

Live re-decompilations performed:
- `ProcessBridgeDestruction_High @ 0x00573540` — resolved to named fn; two
  `MapClass__ValidateBridgeZones(&local_40)` calls, each followed by
  `if ((char)param_2 != 0) MapClass__UpdateBridgeZonesHelper();`.
- `MapClass__ValidateBridgeZones @ 0x0056db70` — `FindBridgeRecord(coord,3,0)` loop; for each
  record with `+0x08 == 0` sets `+0x08 = 1`, calls `AddBridgeZoneEdges`, then `Can_Reach_Zone`.
- `get_function_callers 0x0057f440` → only `ProcessBridgeDestruction_High @ 0x00573540`.
- `MapClass__InvalidateBridgeZones @ 0x0056dae0` — `FindBridgeRecord(coord,3,0)` loop;
  per matched record `RemoveBridgeZoneEdges` + `+0x08 = 0`. (high-only via FindBridgeRecord.)
- `MapClass__FindBridgeRecord @ 0x0056da10` — `if (puVar5[3] == 0)` kind-skip; vertical
  (same X) Y-between + |ΔX| ≤ param_3; horizontal X-between + |ΔY| ≤ param_3. Geometric tol=3.
- `MapClass__ComputeBridgeZones @ 0x0056d6e0` — per-cell push; high branch `*puVar9 = uVar2`
  (uVar2 = this cell's MapCoord), `puVar9[1] = uVar1` from `Pathfinding_update_continued(iVar7-4 & 7)+0x24`,
  `puVar9[3]=0`; low branch `puVar9[3]=1`. `*(param_1+0x60)+=1` each accepted cell, no dedupe.
- `MapClass__UpdateBridgeZonesHelper @ 0x0056c510` — record loop tests only
  `*(char*)(MapClass+0x54 + 8 + i*0x10) != 0` (intact byte); never reads +0x0C.

Rust re-reads:
- `refresh_endpoint_active_flags` mod.rs:1587-1603 — deactivate-only; only `.active` write
  outside construction is `record.active = false` at :1600.
- `refresh_bridge_zones_if_dirty` bridge_orchestrator.rs:1135-1150 — refresh(deactivate) →
  `PathGrid::from_resolved_terrain_with_bridges` → `rebuild_zone_grid`; no re-activation.
- `compute_bridge_endpoints` mod.rs:1648-1718 — one record per group, max-Manhattan pair.
- `compute_low_bridge_tube_endpoints` mod.rs:1720-1772 — group_id from runtime cell.
- Pass-1/Pass-4 mod.rs:564-624 / 713-752 — deck cells get `bridge_group_id=Some`, Pass-4
  bridgeheads/low cells get `bridge_group_id=None`.
- `bridge_record_matches` zone_build.rs:64-70 — gates on `record.active`.
- `bridge_state` built once at app_init_helpers.rs:368; never reconstructed at runtime.

---

## D1 — VERDICT=REAL

Bridge repair never re-activates the zone endpoint record.

gamemd reading holds live: `ProcessBridgeDestruction_High @ 0x00573540` (the repair/restore
walker; its `MapClass__RepairBridge_High @ 0x0057f440` callee has only this one caller, confirmed
via `get_function_callers`) calls `MapClass__ValidateBridgeZones(&local_40)` at two sites, each
followed by `if ((char)param_2 != 0) MapClass__UpdateBridgeZonesHelper()`. `ValidateBridgeZones
@ 0x0056db70` walks `FindBridgeRecord(coord,3,0)` and, for each record with `+0x08 == 0`, sets
`+0x08 = 1` and calls `MapClass__AddBridgeZoneEdges` — re-inserting the directed zone edges.

Rust holds: `refresh_endpoint_active_flags` (mod.rs:1587-1603) only ever writes `.active = false`
(:1600); its docstring says "Deactivation is one-way (no re-activation)." The repair path's sole
zone-touching code is `refresh_bridge_zones_if_dirty` (bridge_orchestrator.rs:1135-1150), which
calls the deactivate-only refresh, rebuilds the PathGrid, and reruns `rebuild_zone_grid`. No code
sets `endpoint_records[*].active = true`. `bridge_state` is built once (app_init_helpers.rs:368)
and never reconstructed, so a record flipped to `false` by an earlier collapse stays `false` after
repair. `bridge_record_matches` (zone_build.rs:65) gates the long-range zone edge on
`record.active`, so the repaired bridge's cross-zone edge is never re-added even though
`from_resolved_terrain_with_bridges` restores per-cell walkability.

Corrected delta: Rust = repair restores cell walkability but leaves `record.active=false`
permanently (long-range A* edge missing) -> gamemd = repair runs ValidateBridgeZones which sets
record `+0x08=1` and AddBridgeZoneEdges (long-range edge restored), with a conditional full
UpdateBridgeZonesHelper rebuild if any validated record is not yet reachable.

---

## D2 — VERDICT=REAL

Deactivation granularity is whole-group, not per-record geometric tol-3.

gamemd reading holds live: `InvalidateBridgeZones @ 0x0056dae0` selects records via
`FindBridgeRecord(impact,3,0)` and flips just those records' `+0x08` to 0 + `RemoveBridgeZoneEdges`.
`FindBridgeRecord @ 0x0056da10` is per-record geometric: vertical case requires the impact Y
between the two endpoints AND |impact.X - endpoint.X| ≤ 3; horizontal case mirror. The `puVar5[3]
== 0` gate confirms high-only selection. Records far from the impact stay intact=1.

Rust holds: `refresh_endpoint_active_flags` (mod.rs:1588-1602) collects every `bridge_group_id`
that has ANY `Destroyed` cell, then deactivates EVERY record whose `group_id` is in that set —
group = 4-cardinal BFS blob (Pass 1, mod.rs:564-624). No geometric tol-3 / per-record line test.

Output divergence is real but layout-dependent: on the common single-linear-bridge case (one BFS
group, one record) both sever the whole span on first collapse — identical. They diverge when one
BFS group fuses two physically distinct spans (shared deck cell / junction): Rust deactivates both
arms on a hit to one arm; gamemd invalidates only records within tol-3 of the impact. Finder's LOW
severity (rare on stock linear maps) stands.

Corrected delta: Rust = deactivate all records sharing the impacted cell's BFS group -> gamemd =
deactivate only high records whose endpoint line passes within Manhattan-3 of the impact coord.

---

## D3 — VERDICT=UNCERTAIN

Low-bridge (tube) records never deactivated on collapse.

gamemd side I CAN confirm: `UpdateBridgeZonesHelper @ 0x0056c510` record loop tests only
`*(char*)(MapClass+0x54 + 8 + i*0x10) != 0` (the +0x08 intact byte) — it never reads +0x0C, so an
intact low record (kind=1) is folded into the rebuild. So gamemd does NOT delete/skip low records
by kind; severing a destroyed low bridge relies on the underlying cell zone-type recompute making
the record's endpoints resolve to non-connected clusters — exactly as the finder claims. (I did
not independently re-verify `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT §3.7`; the per-cell
zone-recompute mechanism that severs the connection was not re-decompiled this session.)

Rust side I CAN confirm: low tube records get `group_id` from the runtime cell's `bridge_group_id`
(mod.rs:1740-1746), and the record is only built when that is `Some` (`let Some(group_id) = … else
continue`). Pass-4 registers non-deck low/bridgehead cells with `bridge_group_id=None`
(mod.rs:743). `refresh_endpoint_active_flags` keys deactivation on cell `damage_state==Destroyed`
joined by `bridge_group_id` (mod.rs:1591-1599). So a low record whose group_id came from a Pass-1
deck member would only deactivate if THAT deck group has a Destroyed cell; a low record built off a
`None`-group cell is skipped at construction (no record exists).

Why UNCERTAIN not REAL: the finder's own confidence is LIKELY-DRIFT and explicitly says "needs a
fixture run to confirm a specific map produces the stale active=true." The drift requires that on a
real map a low tube record (a) gets built with a deck group_id that (b) never receives a Destroyed
cell when the low overlay collapses. That two-step linkage was not demonstrated on a concrete
fixture, and gamemd's severing mechanism (cell zone-type recompute under the intact record) was not
re-decompiled. Both unproven sides => UNCERTAIN, not REAL. Finder mislabeled this PROVEN/LIKELY in
the summary line; the body's LIKELY-DRIFT is the honest level, and it falls short of the burden of
proof for REAL.

To upgrade to REAL: decompile the low collapse cell-zone-type recompute path and run a Rust fixture
showing a destroyed low bridge whose record stays `active=true` AND whose endpoints still resolve to
the same zone.

---

## D4 — VERDICT=REAL

Endpoint pair uses max-Manhattan ground-neighbor heuristic, not the binary's structural
own-coord + opposite-step.

gamemd reading holds live: in `ComputeBridgeZones @ 0x0056d6e0`, the accepted high cell pushes
`*puVar9 = uVar2` where `uVar2 = *(undefined4*)&this->MapCoord_X` (endpoint_a = the bridge cell's
OWN MapCoord), and `puVar9[1] = uVar1` where `uVar1 = *(int*)(iVar7+0x24)` with
`iVar7 = Pathfinding_update_continued(iVar7 - 4U & 7)` (endpoint_b = coord one opposite-direction
step past the far end). Walk direction comes from per-tile orientation tables (DAT_0082a734 /
DAT_0082a774). Endpoints are structurally aligned to the bridge's physical axis.

Rust holds: `compute_bridge_endpoints` (mod.rs:1648-1718) collects all ground cells cardinally
adjacent to the BFS group, then picks the pair with maximum Manhattan distance (mod.rs:1683-1699).
Different algorithm. On a clean straight symmetric bridge the two methods coincide (test
`bridge_endpoints_detected` (0,0)/(4,0)); they diverge on irregular fringes where an off-axis
adjacent ground cell is the farthest-apart pair. Finder's LOW severity stands.

Corrected delta: Rust = endpoint pair = max-Manhattan over all ground cells adjacent to the BFS
group -> gamemd = endpoint_a = bridge cell's own MapCoord, endpoint_b = coord one opposite-direction
orientation step past the far bridge end (per-tile tables), one record per qualifying bridge tile.

---

## D5 — VERDICT=REAL

One record per BFS group vs one record per bridge tile.

gamemd reading holds live: `ComputeBridgeZones @ 0x0056d6e0` increments `*(int*)(param_1+0x60) += 1`
and writes a fresh 16-byte record for EVERY accepted bridge tile inside the CellIterator loop, with
no per-group dedupe. A multi-tile high bridge therefore yields many records sharing geometry.

Rust holds: `compute_bridge_endpoints` pushes exactly ONE record per BFS group (mod.rs:1701-1707),
plus low records de-duplicated by ordered endpoint key (mod.rs:1759-1762).

The record cardinality differs by construction. The finder correctly classifies the standalone
observable impact as nil for a single bridge (shared endpoints collapse to the same connected
cluster) — REAL as a state-layout difference, and the substrate that D1/D2 depend on. Keeping the
finder's framing: REAL but LOW, observable only via D1/D2 interactions.

Corrected delta: Rust = 1 endpoint record per BFS group (+ deduped low records) -> gamemd = 1 record
per qualifying bridge tile (per-cell push, no dedupe), each independently intact-toggled by
Invalidate/Validate.

---

## MISS (new disparities the finder did not raise as a numbered D)

- MISS [LOW/LIKELY-DRIFT]: `ValidateBridgeZones @ 0x0056db70` (and `InvalidateBridgeZones
  @ 0x0056dae0`) call `MapClass__ComputeBridgeZones()` to (re)build the record array when
  `FindBridgeRecord` returns -1 on first try — i.e. gamemd lazily reconstructs the per-tile record
  set if it is empty/stale. Rust builds `endpoint_records` ONCE at init
  (app_init_helpers.rs:368 -> `from_resolved_terrain`) and never reconstructs them at runtime. This
  is the deeper root of D1: gamemd's record set is self-healing (Compute on demand), Rust's is
  frozen. Worth surfacing separately because even if D1's re-activation were patched, a Rust record
  that was never created (e.g. a bridge first formed/altered at runtime) would still be missing,
  whereas gamemd would Compute it on the next Validate/Invalidate. (Verify: `decompile_function
  0x0056db70` / `0x0056dae0` — both have the `if (iVar==-1){ ComputeBridgeZones(); FindBridgeRecord
  again; }` reconstruct guard.)

- MISS [INFO]: `ValidateBridgeZones` also calls `MapClass__Can_Reach_Zone(record, record+4, …)`
  after each re-activation and sets the return flag (driving the conditional UpdateBridgeZonesHelper
  full rebuild) only when the endpoints are NOT yet reachable. The finder's D1 mentions the
  conditional rebuild but not that the trigger is a reachability test on the just-validated record;
  the Rust orchestrator unconditionally reruns `rebuild_zone_grid` whenever `zones_dirty` (no
  reachability short-circuit). Output-equivalent for the deactivate direction (always rebuild is a
  superset), so INFO not DRIFT — but the gating condition differs and is worth noting for the repair
  direction once D1 is fixed. (Verify: `decompile_function 0x0056db70` — `Can_Reach_Zone` call +
  `param_2._0_1_ = 1` only when it returns 0.)
