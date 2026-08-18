# FootClass::Find_Path -> Find_Nearby_Passable_Cell Argument Row Reswarm 2 - 2026-05-27

**Slot:** 2  
**Target:** `FootClass::Find_Path @ 0x004D3920` calls to `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.  
**Status:** COMPLETE for the two direct fallback calls in this function.  
**Active in YR:** Yes. `FootClass::Find_Path` is the standard A* entry for ground/pathfinding foot objects, and both decoded fallback arms are inside that live function.

## Scope

This report closes the exact 15-stack-argument row for the `Find_Path -> FNPC` blocked-destination fallback calls. It does not re-open FNPC internals, A* neighbor expansion, path smoothing, or bridge/TMP classification.

No Rust, INI, or non-research files were edited. Ghidra was used read-only: `decompile_function`, `read_memory`, and byte/disassembly inspection only. No comments, labels, function creation, renames, or save operations were used.

## Executive Summary

`FootClass::Find_Path` has two direct calls to `FootClass__Find_Nearby_Passable_Cell`:

- `0x004D3B76`: destination probe returned code `6`, distance is beyond close-enough, and the object is not the train/naval-exempt branch.
- `0x004D3DD8`: destination probe returned code `7`, destination cell contains a building, and the object is not the train/naval-exempt branch.

Both calls use the same FNPC parameter row. The helper searches from the requested destination cell, but it validates candidates against the unit's current-zone context:

- `speed_type = this->Type->SpeedType (+0x67C)`
- `required_zone_id = MapClass::GetZoneID(current_cell, mapped_movement_zone, this->OnBridge)`
- `movement_zone = remap_table_0x007E8BF0[this->Type->MovementZone (+0x5B4)]`
- `bridge_aware = this->OnBridge (+0x8C)`
- rectangle is `1x1`
- `reject_overlay = 0`
- `height_check = 1`
- `object_safety` is dynamic from `Type+0xD2C` and the current-cell obstacle-free probe
- `allow_bridge_cells = 1`
- `target = current_cell`
- `skip/direct flag = 0`
- `final_occupancy = 0`

Player-visible implication: native blocked-destination correction is not "nearest pathgrid-walkable around the clicked cell." It is a mover-specific FNPC passability query using the current reachable zone, SpeedType, MovementZone remap, bridge context, and current-cell target ranking. This is directly relevant to water/pier drift: a ground unit near a pier must not be redirected onto bare water merely because a coarse grid says that cell is walkable.

## Verified Fallback Branches

### Code 6 fallback: `0x004D3A92..0x004D3CC7`

Verified facts:

- `0x004D3A92` compares the destination `Can_Enter_Cell` result with `6`.
- `0x004D3A9B..0x004D3AA7` requires distance beyond the close-enough threshold and rejects the train/naval-exempt branch.
- `0x004D3AB2..0x004D3AC3` reads `this->Type->MovementZone (+0x5B4)` and remaps it through DWORD table `0x007E8BF0`.
- `0x004D3AD0..0x004D3B24` computes the dynamic object-safety flag from `Type+0xD2C` and `TechnoClass__Is_Current_Cell_Obstacle_Free`.
- `0x004D3B40..0x004D3B4C` computes `MapClass::GetZoneID(current_cell, mapped_movement_zone, this->OnBridge)`.
- `0x004D3B76` calls `FootClass__Find_Nearby_Passable_Cell`.
- `0x004D3B7B..0x004D3CC7` accepts the returned cell only after invalid-cell rejection, distance/close-enough checks, `PathfinderClass__EstimateZoneCost`, and `estimate <= chebyshev_delta + 6`.

### Code 7 building fallback: `0x004D3CDD..0x004D3DFF`

Verified facts:

- `0x004D3CDD` compares the destination `Can_Enter_Cell` result with `7`.
- `0x004D3CE5` rejects the train/naval-exempt branch.
- `0x004D3CF5..0x004D3D09` requires `Look_up_building_in_cell(destination)` to return non-null.
- `0x004D3D14..0x004D3D25` reads `this->Type->MovementZone (+0x5B4)` and remaps it through DWORD table `0x007E8BF0`.
- `0x004D3D32..0x004D3D86` computes the same dynamic object-safety flag.
- `0x004D3DA2..0x004D3DAE` computes the same current-cell zone id.
- `0x004D3DD8` calls `FootClass__Find_Nearby_Passable_Cell`.
- `0x004D3DDD..0x004D3DFF` immediately uses the returned cell to call vtable `+0x480` (`FootClass::Set_Destination_Internal`) without the code-6 `EstimateZoneCost` filter.

## Exact FNPC Argument Row

`FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` returns with `RET 0x3C`, so it has 15 stack arguments after `ECX=this`. `ObjectClass__Get_Cell_Packed @ 0x0041BEA0` returns with `RET 0x04`, and `MapClass__GetZoneID @ 0x0056D230` returns with `RET 0x0C`; those return sizes are required to decode the stacked row correctly.

| FNPC parameter | Code 6 call value | Code 7 call value | Verified source / meaning |
|---|---|---|---|
| `this` / `ECX` | `0x0087F7E8` | `0x0087F7E8` | Global map object passed as FNPC receiver. |
| `param_2 out_cell` | local pointer from `lea edx,[esp+0x60]`, pushed at `0x004D3B70` | local pointer from `lea edx,[esp+0x64]`, pushed at `0x004D3DD2` | FNPC writes the selected packed cell here and returns this pointer. |
| `param_3 origin_seed` | requested destination local from `lea ecx,[esp+0x1FE0]`, pushed at `0x004D3B6F` | same local, pushed at `0x004D3DD1` | The ring search is seeded from the requested destination cell. |
| `param_4 speed_type` | `this->Type->SpeedType (+0x67C)`, pushed at `0x004D3B6A` | same, pushed at `0x004D3DCC` | Feeds `CellRect::CheckPassability` speed/land checks. |
| `param_5 required_zone_id` | result of `MapClass::GetZoneID(current_cell, mapped_movement_zone, OnBridge)`, pushed at `0x004D3B54` | same, pushed at `0x004D3DB6` | Candidate must match the current reachable zone unless GetZoneID returns sentinel behavior downstream. |
| `param_6 movement_zone` | remap table value from `0x007E8BF0[Type+0x5B4]`, pushed earlier at `0x004D3B36` and left after nested cleanup | same, pushed at `0x004D3D98` | MovementZone row family, not SpeedType. |
| `param_7 bridge_aware` | `this->OnBridge (+0x8C)`, pushed at `0x004D3B35` | same, pushed at `0x004D3D97` | Bridge-aware zone/layer flag passed to FNPC and `CheckPassability`. |
| `param_8 rect_width` | `1`, pushed at `0x004D3B34` | `1`, pushed at `0x004D3D96` | 1x1 candidate rectangle. |
| `param_9 rect_height` | `1`, pushed at `0x004D3B33` | `1`, pushed at `0x004D3D95` | 1x1 candidate rectangle. |
| `param_10 reject_overlay` | `0`, pushed at `0x004D3B31` | `0`, pushed at `0x004D3D93` | This fallback does not reject every overlay before normal passability. |
| `param_11 height_check` | `1`, pushed at `0x004D3B2B` | `1`, pushed at `0x004D3D8D` | Enables FNPC origin/candidate height consistency gate. |
| `param_12 object_safety` | dynamic local from `0x004D3AD0..0x004D3B24`, pushed at `0x004D3B2A` | dynamic local from `0x004D3D32..0x004D3D86`, pushed at `0x004D3D8C` | Set to `1` only when the `Type+0xD2C` branch probes obstacle-free successfully; otherwise `0`. |
| `param_13 allow_bridge_cells` | `1`, pushed at `0x004D3B29` | `1`, pushed at `0x004D3D8B` | Structural bridge cells are allowed past FNPC's separate bridge filter. |
| `param_14 target` | current cell pointer from vtable `+0x1B8`, pushed at `0x004D3B28` | current cell pointer from vtable `+0x1B8`, pushed at `0x004D3D8A` | Non-null target means FNPC picks nearest accepted candidate to the unit's current cell, not frame-random/null-target selection. |
| `param_15 skip/direct flag` | `0`, pushed at `0x004D3B15` and left after `Get_Cell_Packed` cleanup | `0`, pushed at `0x004D3D77` and left after cleanup | Uses the normal FNPC ring candidate collection mode. |
| `param_16 final_occupancy` | `0`, pushed at `0x004D3B0A` | `0`, pushed at `0x004D3D6C` | No final `CellRect::CheckOccupancy(rect, -1)` call for this `Find_Path` fallback row. |

### MovementZone Remap Table

`read_memory 0x007E8BF0 64` shows the DWORD table used by both fallback arms:

```text
0: 0
1: 0
2: 0
3: 5
4: 5
5: 5
6: 6
7: 7
8: 7
9: 9
10: 10
11: 11
12: 0
```

This row is therefore not simply raw `TechnoTypeClass+0x5B4`; `Find_Path` first maps the movement-zone value through `0x007E8BF0` before both `MapClass::GetZoneID` and FNPC.

## Water / Pier Relevance

The row confirms the key parity point for the pier/water bug class:

- The fallback seed is the clicked/requested destination, but candidate legality is not `PathGrid::is_walkable`.
- Candidate passability goes through FNPC and then `CellRect::CheckPassability` with SpeedType, required current-zone id, mapped MovementZone, bridge context, 1x1 rectangle, and height gate.
- Bare water adjacent to a pier should not be accepted for an ordinary ground unit unless the native SpeedType/MovementZone/zone/bridge path accepts it. A coarse `PathGrid` that marks water ground-walkable is therefore not a valid replacement for this fallback.
- Structural bridge cells are not globally rejected in this row (`allow_bridge_cells=1`), so bridge/deck candidates must be decided by native bridge-aware passability rather than by a blanket pier/water blacklist.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `Find_Path` only invokes FNPC after destination `Can_Enter_Cell` returns code `6` or building-backed code `7`; it does not pre-redirect every move command. | Replace/gate generic command-time nearest-walkable redirection with native-shaped destination-probe fallback. | `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_path.rs`, future `FindPath`-style layer | A blocked destination redirects only when the cell-entry code matches the native fallback branch. | `find_path_goal_redirect_requires_can_enter_code_6_or_7` | High |
| Code-6 fallback uses FNPC row above, then `EstimateZoneCost <= chebyshev_delta + 6` before accepting. | Add post-FNPC zone-cost acceptance for code-6 blocked destination fallback. | pathfinding goal correction / A* entry wrapper | A nearby passable candidate in the wrong/too-expensive zone is rejected even if geometrically close. | `find_path_code6_fnpc_candidate_must_pass_zone_cost_bound` | High |
| Code-7 building fallback uses the same FNPC row but skips the code-6 zone-cost filter before setting destination. | Keep code-7 building-destination fallback distinct from code-6 soft-block fallback. | same | A building-occupied target can redirect via the FNPC result without applying the code-6 `EstimateZoneCost` bound. | `find_path_code7_building_fallback_uses_fnpc_without_code6_zone_cost_filter` | Medium-high |
| FNPC `param_5` is the current-cell zone id for mapped MovementZone and OnBridge, while `param_3` seed is the requested destination. | Nearby fallback API must support separate seed cell, required zone id, and target cell. | future shared FNPC helper | Seed near water/pier still rejects candidates outside the current ground zone for a normal ground unit. | `find_path_fnpc_seed_destination_but_requires_current_zone` | High |
| FNPC target is the current cell, not null/random, for both fallback arms. | Candidate selection must use nearest-to-current-cell mode, not frame modulo, for this row. | future shared FNPC helper | Two accepted candidates around the destination choose the one closest to the unit's current cell. | `find_path_fnpc_uses_current_cell_as_target` | Medium |
| Final occupancy flag is `0` in both rows. | Do not add FNPC final `CheckOccupancy(rect,-1)` to this specific fallback row unless another caller row requires it. | future shared FNPC helper config | A candidate blocked only by final-occupancy-only state is not rejected by this row, while passability still applies. | `find_path_fnpc_row_does_not_enable_final_occupancy` | Medium |
| `allow_bridge_cells=1` in both rows. | Do not solve pier drift by globally rejecting structural bridge cells in this fallback. | passability / bridge classifier | A valid native bridge-deck candidate remains eligible; bare water remains rejected through SpeedType/zone/passability. | `find_path_fnpc_allows_bridge_cells_but_rejects_bare_water_for_ground` | High |

## Negative Facts / Do Not Do

- Do not model `Find_Path` blocked-destination fallback as "nearest any-layer walkable within radius N".
- Do not seed and rank FNPC with the same cell. Native uses the requested destination as the ring origin and the current unit cell as the target ranking point.
- Do not use raw `TechnoTypeClass+0x5B4` directly for this row; native remaps through `0x007E8BF0`.
- Do not use SpeedType as the zone/matrix row. SpeedType is `param_4`; mapped MovementZone is `param_6`.
- Do not enable final `CheckOccupancy` for this `Find_Path` FNPC row.
- Do not globally reject bridge structural cells to fix pier/water drift. This row passes `allow_bridge_cells=1`.
- Do not treat the code-6 and code-7 fallbacks as identical after FNPC; code 6 has the extra zone-cost acceptance bound, code 7 does not.

## Remaining Uncertainty

- The semantic name of `Type+0xD2C` is not resolved in this slice. Its effect in this row is verified only as the gate that decides whether `TechnoClass__Is_Current_Cell_Obstacle_Free` can set FNPC `param_12`.
- This report does not classify WaterBridge TMP terrain bytes or concrete pier cells. It only proves the `Find_Path` fallback argument row that such cells must pass through.
- This report does not audit InfantryClass/AircraftClass-specific cell-entry overrides.

## Evidence

- Ghidra read-only decompile: `FootClass::Find_Path @ 0x004D3920`.
- Ghidra read-only decompile: `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
- Ghidra read-only decompile and memory: `MapClass::GetZoneID @ 0x0056D230`; `RET 0x0C`.
- Ghidra read-only decompile and memory: `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`; `RET 0x04`.
- Ghidra read-only memory/disassembly ranges: `0x004D3AAD..0x004D3B76` and `0x004D3D14..0x004D3DD8`.
- Ghidra read-only memory: `0x007E8BF0` MovementZone remap table.
- Existing supporting docs: `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`, `CELLRECT_CHECKPASSABILITY_0056E7C0_FULL_ARG_DECODE_GHIDRA_REPORT.md`, `HELPER_PASSABLE_CELL_CONTRACTS_RESWARM_20260527.md`.
