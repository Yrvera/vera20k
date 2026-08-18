# Tube-Pathfinding Parity Scan — Adversarial Verdicts

Facet: tube-pathfinding (low-bridge TubeClass + bridge-aware pathfinding).
Method: re-decompiled every cited gamemd function live this session; re-read every cited
Rust line. Burden of proof = DRIFT unless equivalence proven. Addresses re-confirmed below.

Anchors re-resolved live:
- `get_function_by_address 0x007359f0` → `UnitClass__TubeMovement` (body 007359f0-007360b1). OK.
- `decompile_function 0x0042acf0` → `PathfinderClass__UpdateBridgePassability`. OK.
- `decompile_function 0x0042b080` → `PathfinderClass__FindNearbyBridgePeer`. OK.
- `decompile_function 0x00583180` → `MapClass__ResolvePathCoord_BridgeAware`. OK.
- `decompile_function 0x0042c900` → `AStar_pathfind_search` (D7 caller). OK.
- `decompile_function 0x0042d170` → `PathfinderClass__EstimateZoneCost` (D7 caller). OK.

Rust live path re-confirmed: `tick_low_bridge_tube_movement` is wired into the tick at
`movement_tick.rs:851`; `begin_drive_tube_traversal`/`tick_unit_tube_payload`/`finish_unit_tube_movement`
and every `active_tube` assignment are ONLY in `#[cfg(test)]` (grep: components.rs:1169-1171,
tube_movement.rs:765/807) — the finder's "dead interpolation path" claim is correct.

---

D1: VERDICT=REAL — `UnitClass__TubeMovement @ 0x007359f0` computes remaining dist
(`CoordStruct__Distance3D` → `uStack_68._4_4_`) and per-tick move `iVar6 = Math__ftol()`; the
`if (iVar6 < (int)uStack_68)` branch writes a fractional Cos/Sin world step and `return`s WITHOUT
touching the path-step byte `+0x685`, while the else branch does `*(char*)(param_1+0x685)+'\x01'`
to advance one tube cell. So gamemd interpolates fractionally per tick and advances a cell only
when per-tick move >= remaining distance. Rust `tick_low_bridge_tube_movement`
(tube_movement.rs:219-269) advances exactly one full cell per tick via `move_entity_to_cell`
(snaps rx/ry + sub to CELL_CENTER, lines 329-332), no per-tick Speed budget. Delta: Rust = 1
cell/tick flat regardless of Speed -> gamemd = speed-gated fractional sub-cell slide, cell
advance only when tick-move >= remaining lepton distance.

D2: VERDICT=REAL — In the finalize branch (`iVar14 == 0`, i.e. exit-cell FirstObject empty),
after placing on exit and clearing the tube index (`+0x1a1 = 0xff`), gamemd re-fetches the tube
at the exit cell (`CellClass__Get_Cell_At(&ppuStack_40)` then `CellClass__GetTubeAtCell()`); if
nonzero it sets facing `uStack_6c = (short)(*(int*)(iVar14+0x2c) << 0xd) + -0x8000` and calls
`FacingClass__UpdateFacing`. Confirmed at 0x007359f0. Rust `finish_tube_movement`
(tube_movement.rs:271-286) never reads tube `+0x2c`; facing comes only from the last-hop delta in
`move_entity_to_cell` (facing_from_delta, lines 296-299) and is left unchanged on a zero-length
final hop. Delta: Rust facing = last-movement-delta (or unchanged) -> gamemd facing =
`(tube.dir<<13) - 0x8000` read at the exit cell.

D3: VERDICT=REAL — At exit, `iVar14 = *(int*)(iVar6 + 0xe4)` (exit-cell FirstObject). `iVar14==0`
→ normal place+finalize. `iVar14!=0` → else branch: `FUN_004d0440(0,0)` + `PTR_FUN_007e4f64`
scanner walks the object list collecting up to 10 (`iStack_18=10`); for each occupant of RTTI
type 1 or 0xf with no parasite link (`piVar1[0x19d]`), calls vtable `+0x174` with `(0,0,0),1,1`
to scatter it, then `return`s WITHOUT setting `+0x1a1=0xff` (does not finalize this tick → retries
next tick). Confirmed at 0x007359f0. Rust `finish_tube_movement` (tube_movement.rs:271-286)
unconditionally `move_entity_to_cell` onto exit; only the dead `finish_unit_tube_movement`
(lines 139-165) is exit-block-aware, and it merely blocks, never scatters occupants. Delta: Rust
= snap onto exit unconditionally, stacks on any parked occupant -> gamemd = scatter RTTI-1/0xf
occupants off exit (vtable +0x174) and retry until clear before finalizing.

D4: VERDICT=REAL — gamemd computes the Z ramp inside TubeMovement each pass:
`iVar7=CellClass__GetGroundHeight(entry +0x24); iVar8=CellClass__GetGroundHeight(exit +0x28);
uStack_6c = (iVar8 - iVar7) / path_len` (path_len from tube `+0x1c0`), and accumulates onto
`param_1[0x15c]` (unit Z) each interpolation/advance pass — a smooth endpoint-diff-÷-path_len
linear ramp. (Note: decompiler stack-aliases `uStack_6c`/`uStack_68._4_4_` in the add; the
endpoint-derived ramp computation itself is unambiguous.) Rust live path
(`tick_low_bridge_tube_movement` → `resolve_tube_landing_bridge_state`, tube_movement.rs:357-397)
snaps Z to each landed cell's `bridge_deck_level`/`ground_level` per-cell with no sub-step
interpolation; the only endpoint-÷-path_len `z_step` is in the dead `begin_drive_tube_traversal`
(lines 69-98), uncalled in production. Delta: Rust = per-cell deck/ground-level stair-step ->
gamemd = (GetGroundHeight(exit) - GetGroundHeight(entry)) / path_len applied every tick.

D5: VERDICT=REAL — In `UpdateBridgePassability @ 0x0042acf0` inner peer-path replay, the
direction-8 step (`if (iVar4 == 8)`) reads `*(short*)(cell+0x116)`; if != -1 it follows
`g_TubeArray[idx]->exit (+0x28)`, else outputs coord 0 — NO source/path_len filter; any registered
tube index is honored. Confirmed at 0x0042acf0. Rust `explicit_tube_edge` (core.rs:677-687) emits
a tube A* edge only when `source == TubeSource::ExplicitMap && path_len != 0 && exit != (0,0)`,
excluding auto low-bridge tubes (path_len 0, exit==entry). Finder correctly notes Rust
`step_coord_by_direction` itself handles any tube; the gap is on the A*-edge generator / replay.
(Latent: no current Rust producer emits a bare-8 over an auto tube, so no live A* route diverges —
finder's LOW/LIKELY framing is right; difference is real in the replay-equivalence sense.) Delta:
Rust A*-tube-edge restricted to explicit-source tubes -> gamemd dir-8 replay honors any tube cell.

D6: VERDICT=REAL — `UpdateBridgePassability @ 0x0042acf0` selects FirstObject vs AltObject by the
`|level-diff| < 4` + `+0x23` urgency test, falls back to `FindNearbyBridgePeer @ 0x0042b080`
(3x3 scan, `(-bVar1 & 4)` level bias) when no peer on the cell, then for each peer of RTTI 1/0xf
with lower object id (`+0x678` compare) replays the peer's path buffer `+0x178`, following dir-8
tube jumps (`g_TubeArray[idx]->exit`), XOR-toggling `cell+0x140 ^= ... & 0x40000` along the
replay, then a -2..2 x -2..2 neighborhood toggle of 0x40000 around the bridge cell (skipping the
bridge cell itself). Both 0x0042acf0 and 0x0042b080 confirmed. Rust models only the
caller-supplied `SearchMarkerOverlay` (core.rs:215-238) — XOR-parity toggle of a cell set — with
NO peer detection, NO peer-path replay, NO tube-jump-aware marking, NO 3x3 fallback. Grep of
src/sim/pathfinding found no UpdateBridgePassability/FindNearbyBridgePeer analog. Delta: Rust =
passive caller-supplied marker overlay only -> gamemd = automatic lower-id-peer path replay with
dir-8 tube jumps + 3x3 neighborhood 0x40000 toggle.

D7: VERDICT=REAL — `ResolvePathCoord_BridgeAware @ 0x00583180` pass-throughs only when
`(char)param_3 == 0 || (Flags & 0x100) == 0`; otherwise it finds the bridge record
(`FindBridgeRecord(coord,2,0)`), and snaps the output to the nearer of the two record endpoints
via two `Sqrt_Approx` distance compares (NS vs EW by `Flags & 0x800`), with a `psVar12[4]!=0`
fast path and an IsBridge/IsWoodBridge + `LandType != 3` far-endpoint branch. CALLER CHECK
(closes the finder's UNCHECKED item): both callers pass a NONZERO param_3 on bridge cells —
`AStar_pathfind_search @ 0x0042c900` passes `(char)param_4[0x23]` for the source and
`*(uint*)(destcell+0x140) >> 8 & 0xffffff01` for the dest (nonzero when the dest cell carries the
bridge flag); `EstimateZoneCost @ 0x0042d170` forwards param_5/param_6 likewise. So param_3 is NOT
universally 0 — the snap is live for bridge-cell source/dest. Rust `astar_search` (core.rs:821)
takes start/goal literally; no bridge-record endpoint resolver exists in src/sim/pathfinding
(grep: none). Delta (upgraded LIKELY-DRIFT → REAL): Rust seeds A* source/dest at the literal
clicked/queried cell -> gamemd snaps a bridge-flagged source/dest coord to the nearer (or
IsBridge/LandType!=3 far) bridge-record endpoint before seeding the search and the zone-cost
estimate.

---

PARITY-CONFIRMED items (1-7) spot-checked: not re-refuted; the IsLowBridgeCell predicate, GetTubeAtCell
bounds-only, [2,4,6,0] table, dir-8 sentinel, and compass deltas are consistent with the cited live
reads. Carried as PARITY.

MISS (new, not raised by finder):
- MISS [LOW]: gamemd TubeMovement sets per-step facing during traversal via the unit's coord-vtable
  `+0x48` + `AnimClass__CalcFacingDir` toward the next tube path cell BEFORE the Cos/Sin move (used
  for both partial and advance branches). Rust derives facing only via `facing_from_delta(next-old)`
  in `move_entity_to_cell` (cell-to-cell delta), which is the SAME 8-way result for axis-aligned
  single-cell tube hops, so likely output-identical for stock tubes — flagged as a potential
  divergence only if an explicit `[Tubes]` path produces a hop whose CalcFacingDir octant differs
  from the cell-delta octant (multi-cell skip). Unproven equivalent; surface, do not triage out.
- MISS [LOW]: D7's `FindBridgeRecord` distance tie-break uses `Sqrt_Approx` (approximate sqrt) on
  the two endpoint distances and `sVar6 < sVar7` (strict-less) — any Rust analog must reproduce the
  approx-sqrt rounding and the strict-< tie direction (equal distance → far/second endpoint), a
  fixed-point parity hazard if/when D7 is implemented.
