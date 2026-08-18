# Tube-Pathfinding Parity Scan — Low-Bridge TubeClass & Bridge-Aware Pathfinding

Facet: tube-pathfinding. Scope: low-bridge TubeClass identity/lifecycle, tube movement
stepping, dual-layer A* passability for on/under bridge, direction-8 tube jump in path
walking, path tie-break after low-bridge collapse.

Authority: live Ghidra decompiles of gamemd.exe (re-confirmed anchors below) over docs.

Anchors re-confirmed live this session:
- `get_function_by_address 0x00484ab0` → `CellClass__IsLowBridgeCell` (body 484ab0-484ada).
- `get_function_by_address 0x00484f20` → `CellClass__GetTubeAtCell` (body 484f20-484f45).
- `get_function_by_address 0x0042acf0` → `PathfinderClass__UpdateBridgePassability` (body 0042acf0-0042b072).
- `decompile_function 0x007359f0` → `UnitClass__TubeMovement`.
- `decompile_function 0x00727fd0` → `TubeClass__Constructor`.
- `decompile_function 0x0042b080` → `PathfinderClass__FindNearbyBridgePeer`.
- `decompile_function 0x00583180` → `MapClass__ResolvePathCoord_BridgeAware`.
- `read_memory 0x0081CC20` (16 bytes) → dwords `02 00 00 00 04 00 00 00 06 00 00 00 00 00 00 00` = `[2,4,6,0]`.

Rust source read: `src/sim/movement/tube_movement.rs`, `src/map/tube_facts.rs`,
`src/map/resolved_terrain.rs` (tube build + step_coord_by_direction + predicate),
`src/sim/pathfinding/core.rs` (A* tube edge + bridge traversal), `src/util/direction.rs`,
`src/sim/movement/movement_tick.rs` (tube wiring).

---

### D1: Low-bridge tube movement uses an instant 1-cell-per-tick teleport, not speed-gated interpolation

- Rust now: `tick_low_bridge_tube_movement` (`src/sim/movement/tube_movement.rs:219-269`)
  advances exactly one full cell per sim tick via `move_entity_to_cell` (line 260), which
  hard-snaps `position.rx/ry` to the next cell and `sub_x/sub_y` to `CELL_CENTER_LEPTON`
  (lines 329-332). There is **no per-tick speed/budget**: a unit covers one cell per tick
  regardless of its `Speed=`. The unused alternate path `tick_unit_tube_payload`
  (lines 100-137) does carry a `budget` and interpolates, but it is dead in production —
  `DriveTubePayload`/`active_tube` is never set outside tests (grep: `active_tube` only
  assigned in `components.rs` tests and read at `drive_locomotion.rs:35`).
- gamemd: `UnitClass__TubeMovement @ 0x007359f0` is called every tick from
  `UnitClass__AI @ 0x007363b0` while `(char)unit+0x684 >= 0`. Per tick it computes the
  total remaining distance entry→target via `CoordStruct__Distance3D()` (`uStack_68._4_4_`),
  computes the unit's per-tick move amount (`iVar6 = Math__ftol()` after the speed/facing
  calc), and: `if (iVar6 < (int)uStack_68)` it moves a **fractional interpolated** world-coord
  step (Cos/Sin of facing × move-amount) and `return`s **without** incrementing the path step
  byte `+0x685`. Only when the per-tick amount reaches/exceeds the remaining distance does it
  bump `+0x685` and advance to the next tube path cell. The unit visibly slides across the
  tube span at its own speed over multiple ticks.
- Fixture: explicit tube entry (0,0) exit (3,0), 3 steps, on a low bridge. gamemd: a unit with
  Speed≈3 takes ~85 ticks/cell, smoothly sliding cell-center to cell-center, position
  interpolated each tick. Rust: unit jumps (0,0)→(1,0) tick 1, →(2,0) tick 2, →(3,0) tick 3 —
  3 ticks flat, snapping to each cell center with no intermediate sub-cell position. A fast and
  a slow unit cross in identical time under Rust; gamemd separates them by Speed (~80× drift for
  a slow unit).
- Player sees: units crossing a low/wood bridge teleport-stutter across at fixed 1 cell/tick
  instead of sliding at their real speed; fast and slow units cross in the same time. Triggers
  every time any ground unit crosses any low/wood bridge — common on water maps with pontoon
  bridges.
- Severity: HIGH (visible movement-speed/animation drift, fires on every low-bridge crossing)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x007359f0` (distance/speed branch `if (iVar6 < uStack_68)`
  returning before `*(char*)((int)param_1+0x685)+'\x01'`); Rust `tube_movement.rs:219-269`.

---

### D2: Tube-exit arrival does not set facing from the exit cell's tube direction

- Rust now: `finish_tube_movement` (`src/sim/movement/tube_movement.rs:271-286`) moves the
  entity to `state.exit` and clears tube state. Facing is set only by `move_entity_to_cell`
  from the **delta of the last hop** (`facing_from_delta(next-old)`, lines 297-298). When the
  final hop is zero-length (auto same-cell tube, exit==entry) facing is left unchanged. There
  is no read of the tube's stored direction at the exit cell.
- gamemd: At tube end, `UnitClass__TubeMovement @ 0x007359f0` (no-occupant branch) re-reads the
  tube **at the exit cell** after moving there (`CellClass__Get_Cell_At(&ppuStack_40)` then
  `CellClass__GetTubeAtCell()`), and if a tube exists sets facing via
  `FacingClass__UpdateFacing(&uStack_6c)` where the value is
  `(short)(*(int*)(tube+0x2c) << 0xd) + -0x8000` — i.e. the exit unit facing is derived from the
  **tube's stored direction `+0x2C`**, not from the last movement delta.
- Fixture: auto low-bridge tube at cell (5,5) with direction `+0x2C = 6` (W) (3rd tile of a
  tunnel set → `[2,4,6,0][2] = 6`). gamemd: on arriving, facing = `(6 << 13) - 0x8000 =
  49152 - 32768 = 0x4000`. Rust: facing is whatever the previous ground hop / last delta left
  it as, unrelated to the tube direction. Divergent final facing.
- Player sees: a unit exiting a tube/low-bridge faces a direction set by terrain geometry rather
  than the tube's authored exit direction; on same-cell auto tubes the unit keeps its pre-jump
  facing. Subtle but visible on every tube exit; most noticeable on explicit map `[Tubes]`.
- Severity: MED (facing drift on every tube exit; usually small angle, wrong on authored tubes)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x007359f0` — exit branch
  `uStack_6c = ... (short)(*(int*)(iVar14+0x2c) << 0xd) + -0x8000; FacingClass__UpdateFacing(&uStack_6c)`;
  Rust `tube_movement.rs:271-298`.

---

### D3: No occupant displacement at the tube exit cell

- Rust now: `finish_tube_movement` (`src/sim/movement/tube_movement.rs:271-286`) unconditionally
  moves the entity onto `state.exit`. It does not inspect the exit cell's occupants and never
  pushes other units off. The only exit-block awareness is in the **dead**
  `finish_unit_tube_movement` path (lines 139-165, gated by `exit_ground_blocked`), which is
  never called in production.
- gamemd: `UnitClass__TubeMovement @ 0x007359f0` checks the exit cell's `FirstObject`
  (`iVar14 = *(int*)(iVar6 + 0xe4)`). When `iVar14 == 0` (empty) it does the normal place +
  clear-state path. When `iVar14 != 0` (occupied) it takes the **else** branch: walks the exit
  cell's object list (`FUN_004d0440`, `PTR_FUN_007e4f64` scanner), collects up to 10 occupants,
  and for each occupant of RTTI type 1 or 0xf with no parasite/link (`+0x19d`), calls vtable
  `+0x174` with `(0,0,0),1,1` to **scatter/displace** it off the cell, then re-runs movement.
  The arriving unit does not finalize onto an occupied exit until occupants are pushed.
- Fixture: unit A finishing an explicit tube whose exit is (10,4); unit B already parked at
  (10,4). gamemd: A's tube finish detects B, issues a scatter order to B (B drives off (10,4)),
  A waits/retries; the two do not occupy (10,4) simultaneously. Rust: A snaps onto (10,4)
  regardless of B (occupancy move at `tube_movement.rs:318` relocates only A's own entry; B is
  untouched), producing two units stacked in one cell.
- Player sees: a unit can stack on a unit sitting at a tube/low-bridge exit; the parked unit is
  not pushed aside. Triggers whenever a tube exit cell is already occupied — reproducible by
  queueing several units across one low bridge.
- Severity: MED (cell-stacking / missing scatter; low-to-moderate trigger frequency)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x007359f0` — exit branch `if (iVar14 == 0) {...} else {...
  FUN_004d0440(0,0); ... (**(code**)(*piVar1 + 0x174))(&local_14,1,1); ...}`; Rust
  `tube_movement.rs:271-286`.

---

### D4: Per-tick tube Z does not recompute from live ground heights of entry/exit cells

- Rust now: Both tube paths bake Z linearly. `begin_drive_tube_traversal`
  (`tube_movement.rs:69-98`) computes `z_step = (exit_ground - entry_ground) / path_len` once
  from caller-supplied `entry_ground`/`exit_ground` and accumulates per step. The live
  `tick_low_bridge_tube_movement` path does NOT interpolate Z per tube cell at all — it relies
  on `resolve_tube_landing_bridge_state` to set `position.z` to the destination cell's
  deck/ground level per landed cell (`tube_movement.rs:382-394`).
- gamemd: `UnitClass__TubeMovement @ 0x007359f0` computes the per-step Z increment **inside** the
  function each pass as `(GetGroundHeight(exit_cell) - GetGroundHeight(entry_cell)) /
  tube.path_len` (`iVar7 = CellClass__GetGroundHeight(&ppuStack_34)` on entry `+0x24`,
  `iVar8 = CellClass__GetGroundHeight(&ppuStack_40)` on exit `+0x28`, then
  `uStack_6c = (iVar8 - iVar7) / uStack_6c` where `uStack_6c` was loaded from tube `+0x1C0` path
  length). The per-tick partial move and per-step advance both add this increment to the unit's
  Z (`param_1[0x15c] += ...`). It is a smooth linear ramp keyed on the **two tube endpoint
  ground heights**, divided by path length, applied every interpolation tick.
- Fixture: explicit tube entry-cell ground height 0, exit-cell ground height 8, path_len 4.
  gamemd: z_step = (8-0)/4 = 2; the unit's Z climbs 2 per advanced step and proportionally on
  partial sub-steps, smoothly ramping 0→2→4→6→8. Rust (production
  `tick_low_bridge_tube_movement`): Z is snapped to each landed cell's deck/ground level via
  `resolve_tube_landing_bridge_state`, giving a stair-step keyed on per-cell
  `bridge_deck_level`/`ground_level` rather than (endpoint-diff ÷ path_len); no sub-step Z
  interpolation exists since there are no sub-steps. The dead `begin_drive_tube_traversal` path
  would compute the right `z_step` only if the caller passed the true endpoint ground heights,
  which nothing does.
- Player sees: vertical motion across a sloped tube/low bridge is a per-cell stair-step at the
  cell's deck height instead of a smooth endpoint-to-endpoint ramp; magnitude small on low
  bridges (near ground level), larger on authored sloped `[Tubes]`. Triggers on every tube
  crossing with a height delta.
- Severity: LOW (Z is small on low bridges; visible only on height-delta tubes)
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x007359f0` —
  `iVar7=GetGroundHeight(entry+0x24); iVar8=GetGroundHeight(exit+0x28); uStack_6c=(iVar8-iVar7)/path_len`;
  Rust `tube_movement.rs:69-98` and `tube_movement.rs:357-397`.

---

### D5: A* `explicit_tube_edge` ignores auto low-bridge tubes; binary direction-8 replay honors ANY tube cell

- Rust now: `explicit_tube_edge` (`src/sim/pathfinding/core.rs:677-687`) only emits a tube A*
  jump when `tube.source == TubeSource::ExplicitMap && path_len != 0 && exit != (0,0)`. Auto
  low-bridge tubes (`AutoLowBridge`, `path_len == 0`, `exit == entry`) are excluded, so A* never
  inserts a direction-8 edge for them.
- gamemd: There is no `source`/`path_len` filter on the direction-8 jump. `MapCoord_Step_By_Direction
  @ 0x0042D490` and the path walker treat direction 8 as: read current cell `+0x116`; if valid,
  output = `g_TubeArray[idx]->exit (+0x28)`. The bridge-passability replay
  (`PathfinderClass__UpdateBridgePassability @ 0x0042acf0`, inner replay at `0x0042af15`) likewise
  reads `*(short*)(cell+0x116)` and follows `g_TubeArray[idx]->exit` for **any** tube index, not
  only explicit-source ones. For an auto tube `exit == entry`, so a direction-8 step is a
  same-cell no-op rather than excluded. The Rust `step_coord_by_direction`
  (`resolved_terrain.rs:389-403`) correctly handles any tube; the divergence is only in the A*
  edge generator / A*-side replay honoring exclusively explicit tubes.
- Fixture: cell (4,4) is an auto low-bridge tube (exit (4,4)); a path buffer contains a literal
  step `8` at (4,4). gamemd path walker outputs (4,4) (exit==entry) and proceeds. Rust A*
  `explicit_tube_edge` returns None; A* only emits 0-7 compass edges + the explicit-tube edge, so
  the literal-8 case is unreachable from Rust-A*-generated paths. No divergent output in normal
  A* routes, but an authored `[Tubes]` path replayed through A* could differ.
- Player sees: no observable difference for A*-generated routes (Rust never synthesizes a bare
  8); latent gap only for hypothetical authored-path replay over an auto tube. Not reproducible
  in stock skirmish movement today.
- Severity: LOW (no current trigger in normal play; latent gap)
- Confidence: LIKELY-DRIFT
- Verify-call: `decompile_function 0x0042acf0` (inner `if (iVar4 == 8) {... iVar8 =
  *(int*)(*(int*)(g_TubeArray + cell+0x116 *4)+0x28);}`, no source filter); Rust `core.rs:677-687`.

---

### D6: Bridge-passability peer replay (`UpdateBridgePassability`/`FindNearbyBridgePeer`) and the 0x40000 marker toggle are absent on the tube-aware A* path

- Rust now: A* models the temporary `CellClass+0x140 & 0x40000` marker as a search-scoped
  `SearchMarkerOverlay` cost overlay (`core.rs:215-238, 1294`) applied per-search by the caller.
  It does NOT implement the gamemd `UpdateBridgePassability` peer-marker **replay**: finding an
  in-flight peer unit at a bridge cell, walking that peer's stored path (`peer+0x178` step
  buffer) honoring direction-8 tube jumps, and XOR-toggling the 0x40000 bit along the replayed
  cells, plus the 3×3 neighborhood toggle around the bridge cell. Docs
  `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH` already flag this (`core.rs:382 explicit_tube_edge`
  is "Not the same as 0x0042ACF0 peer-marker replay").
- gamemd: `PathfinderClass__UpdateBridgePassability @ 0x0042acf0` runs per repath on bridge cells
  (`flags & 0x100`). It selects FirstObject vs AltObject by the `|level-diff| < 4` height test and
  `+0x23` urgency byte, falls back to `FindNearbyBridgePeer @ 0x0042b080` (3×3 scan with
  `(-bridge_flag & 4)` level bias) when no peer on the cell, then for each peer of RTTI type 1/0xf
  with a lower object id (`+0x678`) replays the peer's path buffer `+0x178`, following direction-8
  tube jumps via `g_TubeArray[cell+0x116]->exit`, XOR-toggling `cell+0x140 ^= 0x40000 & ...` along
  the replay; finally a -2..2 × -2..2 neighborhood toggle of 0x40000 around the bridge cell with a
  skip of the bridge cell itself.
- Fixture: two units repathing across the same bridge ramp cell where peer ordering matters.
  gamemd: the lower-id peer's reserved path cells (incl. tube exit cell) get the 0x40000 marker,
  steering the later unit around the peer's reserved bridge lane. Rust: only the caller-supplied
  marker overlay (if any) applies; no automatic peer-path-derived marking and no tube-jump-aware
  replay, so two units crossing a bridge do not reserve each other's lanes the same way.
- Player sees: contention routing on bridges/ramps differs — units do not steer around a peer's
  reserved bridge lane the way gamemd does; can cause a different lane choice or a collision-and-
  repath where gamemd pre-avoided. Triggers when ≥2 units cross the same bridge segment
  simultaneously.
- Severity: MED (multi-unit bridge crossings; visible lane/contention drift)
- Confidence: PROVEN-DRIFT (mechanism absent on Rust side; output differs in the 2-unit fixture)
- Verify-call: `decompile_function 0x0042acf0` + `decompile_function 0x0042b080`; Rust
  `core.rs:215-238, 1294` (overlay only) and absence of any `UpdateBridgePassability` analog
  (grep of `src/sim/pathfinding/` found none).

---

### D7: `ResolvePathCoord_BridgeAware` (bridge-record endpoint snap) has no Rust analog

- Rust now: No equivalent of `MapClass__ResolvePathCoord_BridgeAware @ 0x00583180` exists in
  `src/sim/pathfinding/` (grep for `ResolvePathCoord` / bridge-record endpoint snapping: none).
  Path coords are taken as-is; there is no "remap a click/path coord on a bridge to the nearer
  bridge-record endpoint, or to the bridgehead based on IsBridge/IsWoodBridge + LandType!=3" step.
- gamemd: `ResolvePathCoord_BridgeAware @ 0x00583180`: when `param_3 != 0` and the cell has
  `flags & 0x100` (bridge), it finds the bridge record (`FindBridgeRecord(coord,2,0)`); if no
  record, tries `FUN_005835d0`; otherwise computes which of the two record endpoints
  (`record+0`/`record+4` vs `record+8`/`record+0xC`) is nearer via two `Sqrt_Approx` distance
  comparisons and snaps the output coord to the nearer endpoint (NS vs EW chosen by `flags & 0x800`).
  It also has a `psVar12[4] != 0` fast path and an IsBridge/IsWoodBridge + `LandType != 3` branch
  that picks the far endpoint. This routine turns a raw bridge cell click into the correct on-bridge
  target.
- Fixture: click at a mid-span bridge cell on an EW bridge (`flags & 0x800`). gamemd: resolves to
  the nearer of the two bridge-record endpoints, so movement orders target the bridgehead/endpoint
  rather than the literal clicked cell. Rust: targets the clicked cell literally; no endpoint snap.
  Different commanded destination cell.
- Player sees: clicking on a bridge span can command a unit to a different (literal) cell than
  gamemd, which snaps to a bridge endpoint; affects where units stop / how they board a bridge.
  This is partly outside the strict "tube" facet (it is the high/wood bridge-record path) but is
  bridge-aware path-coord resolution sharing the FindBridgeRecord/zone machinery with the
  low-bridge tube system. Triggers on direct clicks onto bridge spans.
- Severity: MED (bridge-click destination drift; high/wood-bridge path, moderate frequency)
- Confidence: LIKELY-DRIFT (no Rust analog found; the exact caller set for the `param_3` flag was
  not fully traced this session — if all live callers pass 0 it is a pass-through and this is moot)
- Verify-call: `decompile_function 0x00583180`; grep of `src/sim/pathfinding/` for any
  ResolvePathCoord/endpoint-snap analog (none).

---

## PARITY-CONFIRMED

Checked against the live binary and found matching (with the noted basis):

1. **Low-bridge cell predicate.** `CellClass__IsLowBridgeCell @ 0x00484ab0`:
   `-1 < cell+0x116 && cell+0x116 < tube_count && cell+0xEC == 10`. Rust
   `ResolvedTerrainCell::is_low_bridge_tube_cell` (`resolved_terrain.rs:248-250`):
   `tube_index.is_some() && yr_cell_land_type == YR_CELL_LAND_TUNNEL`. PARITY by algebraic
   equivalence: `tube_index: Option<TubeId>` is `Some` iff a registered tube index in `[0,count)`
   was assigned at load (`build_auto_low_bridge_tubes`/`seed_explicit_map_tubes` only ever push
   valid indices), so `is_some()` ⟺ `0 <= idx < count`; `YR_CELL_LAND_TUNNEL` is LandType==10.

2. **GetTubeAtCell bounds-only (no land re-check).** `0x00484f20` returns `g_TubeArray[idx]` when
   `0 <= idx < count`, no LandType test. Rust `tube_at_cell` (`resolved_terrain.rs:384-387`)
   returns the tube purely from `tube_index` with no land check. PARITY.

3. **Auto low-bridge tube = same-cell zero-step shell, direction from `[2,4,6,0]`.**
   `TubeClass__Constructor @ 0x00727fd0` sets entry=exit=coord (`param_1[9]=param_1[10]=*coord`),
   fills 100 path dwords with -1, path_len(+0x1C0)=0. Direction table `read_memory 0x0081CC20`
   = `[2,4,6,0]`. Rust `TubeFact::auto_low_bridge` (`tube_facts.rs:41-49`) entry==exit, empty
   path_steps (len 0); `AUTO_TUBE_DIRECTIONS=[2,4,6,0]` (`resolved_terrain.rs:1185`) selected by
   4-tile-band offset (`auto_tube_direction_for_tile` uses `offset < 4`, `resolved_terrain.rs:1273`).
   PARITY on identity, direction table, and per-cell (not per-bridge) granularity.

4. **Constructor `(0,0)` cell-index-write guard.** Binary writes the tube index back to the entry
   cell only `if ((*coord != 0) || (coord[1] != 0))`. The guard exists because gamemd's explicit
   `[Tubes]` constructor is called with coord (0,0) then patched. Rust seeds explicit tubes
   directly with their real entry (`seed_explicit_map_tubes`) and only constructs auto tubes for
   real tunnel cells, so the final cell→tube mapping is identical without needing the guard.
   PARITY (internal artifact of gamemd's two-phase explicit-tube construction).

5. **Direction-8 sentinel in coord stepping.** `MapCoord_Step_By_Direction @ 0x0042D490`: dir 8 →
   if `cell+0x116 == -1` output coord 0 else `g_TubeArray[idx]->exit`. Rust
   `step_coord_by_direction` (`resolved_terrain.rs:389-403`): dir 8 →
   `tube_at_cell(...).map_or((0,0), |t| t.exit)`. PARITY (no-tube → (0,0); tube → exit).

6. **Compass direction table / deltas.** `0=N,1=NE,2=E,3=SE,4=S,5=SW,6=W,7=NW`, `direction & 7`
   for the tube step direction in TubeMovement. Rust `DIRECTION_DELTAS` (`direction.rs:12-21`)
   matches this ordering. PARITY.

7. **A* dual-layer bridge traversal height gate.** `check_bridge_traversal` / four-case
   height-diff tree (`core.rs:506-592`) and ground→bridge diff-4+transition entry are
   corpus-verified against `AStar_create_node`/traversal; spot-consistent with the bridge-aware
   A* docs, not re-derived here. PARITY (carried; not a tube-facet regression).

---

## UNCHECKED

1. **Exact caller distribution of `ResolvePathCoord_BridgeAware`'s `param_3` flag (D7).** Did not
   enumerate every call site to confirm callers pass `param_3 != 0` in normal move orders. If all
   live callers pass 0, the function is a pass-through and D7 is moot. Resolve via
   `get_function_callers 0x00583180` + decompiling each to read the third arg.

2. **Whether any Rust planner ever emits a literal direction-8 step into a path buffer (D5).**
   Confirmed A* generates only 0-7 + explicit-tube edges, but did not exhaustively prove no other
   producer (authored `[Tubes]` replay, navcom) injects a bare 8 over an auto tube. If none does,
   D5 stays latent.

3. **Infantry tube movement (`InfantryClass::AI @ 0x0051BF00` / `FUN_0051B350`).** D1-D4 were
   walked against `UnitClass__TubeMovement` (vehicles). The infantry tube routine is structurally
   parallel per the doc but was not re-decompiled this session; D1-D4 are expected to apply equally
   to infantry but that is unverified for the infantry path.

4. **Whether low-bridge collapse rewrites/clears tube indices, affecting post-collapse path
   tie-break.** The `LOW_BRIDGE_TUBECLASS` report states (Medium confidence) no `cell+0x116` clear
   was found in the primary low damage/destroy/repair functions, implying tube identity persists
   and connectivity is gated by zone validate/invalidate + `UpdateBridgeZonesHelper`. Did not
   re-decompile `DestroyBridge_Low @ 0x0057baa0` / `ProcessBridgeDestruction_Low @ 0x00570050` this
   session to confirm the tube index is untouched, nor compare post-collapse A* tie order. Resolve
   via decompiling those plus checking whether Rust collapse clears `tube_index`.
