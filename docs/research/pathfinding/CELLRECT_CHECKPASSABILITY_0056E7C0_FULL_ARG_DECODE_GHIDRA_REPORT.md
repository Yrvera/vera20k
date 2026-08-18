# CellRect::CheckPassability 0x0056E7C0 Full Arg Decode - Ghidra Research Report

**Address(es):** `0x0056E7C0` primary, callee `0x004834A0`, immediate caller `0x0056DC20`
**Investigation Mode:** exhaustive-slice, downgraded to evidence-synthesis because this subagent session had no callable Ghidra MCP endpoint.
**Claimed Scope:** `CellRect::CheckPassability` stack arguments, its per-cell call into `CellClass::CheckCellPassability`, and the movement-row / height / layer / bridge / zone-matrix behavior needed to design a Rust validator split.
**Non-Scope:** `CellRect::CheckOccupancy` beyond distinguishing it, full `Find_Nearby_Passable_Cell`, full A*, full `RecalcZoneType`, and any Rust implementation.
**Confidence:** Medium-High. The report cites prior Ghidra-backed reports; no fresh live decompile was possible in this slot.
**Active in YR:** Yes for `CheckPassability` via `FootClass::Find_Nearby_Passable_Cell`. Height values other than `-1` are verified in callee behavior but are not used by FNPC's direct calls to this wrapper.

## 1. Overview

`CellRect::CheckPassability @ 0x0056E7C0` is a rectangle-wide passability validator. It walks every cell in a top-left `CellStruct` plus explicit width/height rectangle and calls `CellClass::CheckCellPassability @ 0x004834A0` for each sub-cell.

The important split is this: `CheckPassability` is not the full rectangle occupancy validator and it does not directly read `ZonePassabilityMatrix`. It checks overlays, zone identity, required height/layer, selected occupation bits, wall overlay exceptions, and the speed/land table through `CheckCellPassability`. Final object-list/blocker/playfield occupancy belongs to `CellRect::CheckOccupancy @ 0x00586780`, and FNPC calls that separately only when its final occupancy flag is enabled.

## 2. Class Layout / Key Offsets

| Offset / item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `RET 0x24` | nine 32-bit stack arguments | `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` | Yes |
| top-left arg | packed `CellStruct`: signed low 16-bit `x`, signed high/next 16-bit `y` | `0x0056E7E8..0x0056E7FA` in validator report | Yes |
| cell lookup | flat `y * 0x200 + x`, valid index `[0,0x3FFFF]`, else dummy cell | `0x0056E7FF..0x0056E832` | Yes |
| `CellClass+0x44` | overlay type index; `-1` means none | wrapper reject-any-overlay branch `0x0056E832..0x0056E83E` | Conditional |
| `CellClass+0x4C` | reduced `ZoneType` / movement-class column for zone-system records; not raw land type | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; `CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md` | Yes |
| `CellClass+0x48` | current base `LandType` used by `RecalcZoneType`; companion to land-speed table | `CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md` | Yes |
| `CellClass+0x11B` | base cell level/height byte | `0x004834EF..0x00483527` in validator report | Yes |
| `CellClass+0x124` | ground occupation bitfield | `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` | Yes |
| `CellClass+0x128` | bridge/deck occupation bitfield | same | Conditional on structural bridge/layer |
| `CellClass+0x140 & 0x100` | structural bridge flag | `0x004834FA..0x00483527`; FNPC bridge reject `0x0056DE77..0x0056DE80` | Yes |
| `TechnoTypeClass+0x5B4` | `MovementZone`, direct row source for zone maps/matrix | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | Yes |
| `TechnoTypeClass+0x67C` | `SpeedType`, separate speed/land table selector | `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | Yes |

## 3. Core Logic

### 3.1 Full stack signature

Reconstructed signature, using stack order from `RET 0x24`, FNPC calls, and the callee argument contract:

```text
bool CellRect_CheckPassability(
    CellStruct* top_left,          // arg1
    int rect_width,                // arg2
    int rect_height,               // arg3
    int speed_type,                // arg4; TechnoType+0x67C source in callers
    int required_zone_id,          // arg5; -1 skips zone-id comparison
    int movement_zone,             // arg6; TechnoType+0x5B4 / zone row selector
    int required_height_or_level,  // arg7; -1 means unrestricted
    int bridge_layer_arg,          // arg8; passed to GetZoneID and layer/height logic
    int reject_any_overlay         // arg9; nonzero rejects Cell+0x44 != -1
)
```

Immediate caller evidence: `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` calls this wrapper four times (`0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`) and always passes `required_height_or_level = -1`. Its caller-facing argument matrix supplies `speed_type`, `required_zone_id`, `movement_zone`, `bridge_layer_arg`, rectangle width/height, and overlay rejection. Active in YR: Yes.

### 3.2 Wrapper behavior

| Step | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Loop shape | Outer `x`, inner `y`, signed `< width/height`; `width <= 0` or `height <= 0` returns true without checking cells | `0x0056E7CA..0x0056E87C` in validator report | Conditional |
| Cell lookup | Missing/out-of-range cells substitute dummy `DAT_00ABDC50`; wrapper has no final `IsRectInPlayfield` check | `0x0056E7FF..0x0056E832`; no call to `0x00578390` | Yes |
| Overlay precheck | If `reject_any_overlay != 0` and `CellClass+0x44 != -1`, fail before callee | `0x0056E832..0x0056E83E` | Conditional |
| Callee args | Calls `CellClass::CheckCellPassability` with `speed_type`, two zero mask flags, `required_zone_id`, `movement_zone`, `required_height_or_level`, `bridge_layer_arg` | `0x0056E840..0x0056E859`; callee `0x004834A0` | Yes |
| Occupancy split | Does not call `CellRect::CheckOccupancy`; object-list/building/playfield blocker checks are separate | full body and validator report | Yes |

The two zero mask flags mean this wrapper does not ignore infantry or vehicle occupation bits. Any remaining selected occupation bits in `+0x124` or `+0x128` block.

### 3.3 `CellClass::CheckCellPassability @ 0x004834A0`

| Topic | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Fly/Winged speed | `speed_type == 4` immediately succeeds, skipping zone, height, occupation, overlay-wall, and land-speed checks | `0x004834A7`, `0x004835FF` | Yes when caller passes Winged |
| Zone id | If `required_zone_id != -1`, `MapClass::GetZoneID(cell, movement_zone, bridge_layer_arg)` must equal it | `0x004834BF..0x004834D8`; `0x0056D230` | Conditional |
| Movement row | `movement_zone` is the `MovementZone` row family (`TechnoType+0x5B4`), not `SpeedType` | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; FNPC caller matrix correction | Yes |
| Matrix relationship | This callee does not directly read `ZonePassabilityMatrix`; it consumes the zone id returned by `MapClass::GetZoneID`. The zone maps behind that ID are built from matrix rows where only value `1` passes | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; validator report | Yes |
| Required height base case | If `required_height_or_level == cell.Level`, structural bridge cells reject when the explicit bridge/layer arg is false; otherwise base layer may proceed | `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md` section on `0x004834A0` | Conditional |
| Required height bridge case | If required height differs from base level, it requires `Cell+0x140 & 0x100` and `required_height_or_level == cell.Level + 4` | same | Conditional |
| Occupation field select | Uses `+0x128` only when structural bridge is present and `(required_height == -1 or required_height == cell.Level + 4)`; otherwise uses `+0x124` | same | Yes/Conditional |
| Occupation mask | Ignore-infantry would keep `0xE0`; ignore-vehicle would clear `0x20` with `0x5F`; this wrapper passes both false/zero | same | Yes |
| Bridge layer speed-table skip | The speed/land table check is skipped when the selected occupation path is bridge/deck | same | Conditional |
| Wall overlay exception | Wall overlays are passable only for certain movement zones or wall-buster-style flag; accepted wall overlay forces land type to Clear before speed lookup | validator report `0x00483583..0x004835D5` | Conditional |
| Speed table | Non-bridge selected path checks `g_SpeedType_LandType_Table[speed_type + LandType*9]`; exact `0.0` rejects | `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`; `0x004835DE` | Yes |

### 3.4 Bridge structural and allow flags

`CheckPassability` has no caller-facing `allow_bridge_cells` boolean. The structural bridge allow/reject gate belongs to FNPC after `CheckPassability`: if FNPC `allow_bridge_cells == 0`, a candidate whose `CellClass+0x140 & 0x100` is set is rejected. Evidence: `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md` and parent context. Active in YR: Yes.

Inside `CheckCellPassability`, `CellClass+0x140 & 0x100` is not a simple reject flag. It selects bridge-height handling, bridge occupation field `+0x128`, and the deck path that can bypass speed-table terrain rejection. Treating this flag as "always blocked" would break bridge-deck placement/search parity.

### 3.5 Zone-matrix interpretation

The corrected matrix facts relevant to this validator are:

- Matrix is `int[13][8]` at `0x0082A594`, rows are `MovementZone` values `0..12`, columns are reduced `ZoneType` values `0..7`.
- Only value `1` passes. Values `2` and `3` block zone connectivity; `3` is not a special pass value.
- `CellClass+0x4C` is written by `CellClass::RecalcZoneType @ 0x00483C80`; it is not a raw `LandType`.
- `CheckPassability` does not read the matrix. Its `required_zone_id` comparison is against the zone ids previously built from that matrix. If `required_zone_id == -1`, this entire zone-family comparison is skipped.

Evidence: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`. Active in YR: Yes.

## 4. INI Keys

No INI key is read directly by `0x0056E7C0` or `0x004834A0`; all inputs arrive through caller arguments or precomputed cell/type fields.

| Key/data | Field / effect reaching validator | Evidence | Active in YR |
|---|---|---|---|
| `SpeedType=` | `TechnoTypeClass+0x67C`, `speed_type` arg, speed/land table column | `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | Yes |
| `MovementZone=` | `TechnoTypeClass+0x5B4`, `movement_zone` arg, zone-map row family | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; INI grep | Yes |
| LandType speed sections | populate `g_SpeedType_LandType_Table`, checked when not on selected bridge path | speed-table report | Yes |
| `Buildable=` | not read by this validator; belongs to building-placement predicates | validator report; speed-table report | No for this slice |
| bridge INI keys | do not change the argument contract; runtime bridge flags/state feed `CellClass+0x140` and zone rebuilds | bridge docs | Conditional |

## 5. Integration Points

| Caller / callee | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20` | only direct caller found in prior Ghidra xrefs; four internal calls | validator report xrefs | Yes |
| FNPC internal height gate | separate `+/-2` origin/candidate height check; not the same as `required_height_or_level` because FNPC passes `-1` to the validator | FNPC report; caller matrix report | Conditional |
| FNPC bridge allow gate | separate `allow_bridge_cells` polarity: nonzero allows structural bridge cells, zero rejects after passability | caller matrix report | Conditional |
| `CellClass::CheckCellPassability @ 0x004834A0` | per-cell callee that owns zone/height/layer/occupation/speed checks | validator report; bridge occupancy report | Yes |
| `CellRect::CheckOccupancy @ 0x00586780` | distinct optional final rectangle blocker validator; FNPC calls it with `-1`, skipping `Cell+0xDC` | validator report; parent settled facts | Conditional |

## 6. Current Rust Implementation Status

| Surface | Status vs this slice |
|---|---|
| `src/sim/pathfinding/passability.rs` | Has a `13x8` matrix and direct `MovementZone` row mapping, but the public comments and `PASSABILITY_MATRIX` are still a local land-type remap, not the literal reduced `ZoneType` matrix. It also still exposes `zone_layer_for_speed_type`, which is not the matrix row source for this validator's zone id behavior. |
| `src/sim/pathfinding/zone_build.rs` | Contains a literal `MOVEMENT_CLASS_PASSABILITY[13][8]` matching the binary rows and uses `ResolvedTerrainCell.zone_type` in the terrain-aware rebuild path. This is the closest current Rust match for the zone-matrix side. |
| `src/sim/pathfinding/zone_map.rs` / `zone_search.rs` | Zone maps are per `MovementZone`, but `MovementZone::Fly` is excluded from built maps and reduced precheck is gated to selected zones. Binary rebuild has all 13 rows; whether Fly should stay special is a deliberate parity decision needing tests. |
| `src/sim/occupancy.rs` and movement layer code | Models per-layer occupants, but there is no exact `CheckPassability` rectangle API that selects `+0x124`/`+0x128` using required height plus bridge structural state. |
| `src/sim/production/production_placement.rs`, `src/sim/world/world_spawn.rs`, production spawn | Placement/spawn checks exist, but they use current PathGrid/occupancy/build-blocked predicates rather than the binary `CheckPassability` + optional `CheckOccupancy` split. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellRect::CheckPassability @ 0x0056E7C0` stack arity/order | verified-from-prior-report | validator report, FNPC caller matrix | fresh Ghidra re-read unavailable in this slot |
| overlay precheck and rectangle loop | verified-from-prior-report | `0x0056E7CA..0x0056E87C`, `0x0056E832..0x0056E83E` | none for synthesis |
| direct caller set | verified-from-prior-report | xrefs in validator report | runtime coverage not rerun |
| `CellClass::CheckCellPassability @ 0x004834A0` height/layer/occupation behavior | verified-from-prior-report | bridge occupancy report and validator report | exact non-CellRect caller taxonomy out-of-scope |
| FNPC required height `-1` | verified-from-prior-report | FNPC calls at four addresses | none |
| FNPC allow-bridge separate flag | verified-from-prior-report | caller matrix report | none |
| matrix dimensions/value semantics | verified-from-prior-report | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md` | none |
| Rust surface scan | touched-not-exhausted | `rg`, Codegraph, selected file reads | no code edits/tests per swarm constraints |
| Live Ghidra MCP | deferred | no Ghidra MCP tool exposed in this session | rerun in parent/subagent with Ghidra endpoint for fresh spot-checks |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What are all `0x0056E7C0` stack args? -> Nine args: top-left, width, height, speed type, required zone id, MovementZone, required height, bridge/layer arg, reject-any-overlay.` (evidence: validator report; FNPC caller matrix)
- `[RESOLVED] OQ-2 - Does arg6 mean SpeedType or MovementZone? -> MovementZone / matrix row family; SpeedType is arg4.` (evidence: caller matrix report; zone matrix reader report)
- `[RESOLVED] OQ-3 - Does `CheckPassability` directly read `ZonePassabilityMatrix`? -> No; it compares zone IDs through `MapClass::GetZoneID` when required zone id is not `-1`.` (evidence: validator report; matrix reader report)
- `[RESOLVED] OQ-4 - Which matrix values pass? -> Only `1`; `2` and `3` block zone connectivity.` (evidence: matrix reader report)
- `[RESOLVED] OQ-5 - Does FNPC expose required height/layer to callers? -> No; FNPC always passes `-1` to this validator.` (evidence: FNPC caller matrix)
- `[RESOLVED] OQ-6 - Does required height still matter to the helper? -> Yes, in `0x004834A0`; non-`-1` selects base or bridge layer using exact level / `level+4` rules.` (evidence: bridge occupancy report)
- `[RESOLVED] OQ-7 - Are bridge structural cells always rejected by this validator? -> No; bridge rejection is an FNPC flag outside this wrapper, while helper logic uses structural bridges for layer/height/occupation selection.` (evidence: caller matrix report; bridge occupancy report)
- `[RESOLVED] OQ-8 - Does `CheckPassability` include `CheckOccupancy` behavior? -> No; separate validator and separate FNPC flag.` (evidence: validator report)
- `[RESOLVED] OQ-9 - What does `reject_any_overlay` do? -> Wrapper rejects any `Cell+0x44 != -1` before wall exceptions or speed checks.` (evidence: `0x0056E832..0x0056E83E` in validator report)
- `[RESOLVED] OQ-10 - How are zero-size rectangles handled? -> wrapper loop skips and returns true.` (evidence: validator report)
- `[DEFERRED] OQ-11 - Fresh live decompiler verification of every cited address.` (category: needs-runtime-debugger; reason: no Ghidra MCP tool was exposed to this subagent; next-step-if-pursued: rerun `0x0056E7C0` and `0x004834A0` decompile in a session with Ghidra MCP)
- `[DEFERRED] OQ-12 - Exact Rust API ownership/design for a future validator pair.` (category: out-of-scope; reason: swarm slot is research-only; next-step-if-pursued: write implementation plan after parent reconciliation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CheckPassability` needs a typed 9-field config: `SpeedType`, required zone id, `MovementZone`, required height, bridge/layer arg, overlay reject, and rectangle size. | `0x0056E7C0`, FNPC caller matrix | missing exact API | future `src/sim/pathfinding/passability.rs` or nearby-passable helper; production/world spawn call sites | Add a binary-shaped rectangle passability validator separate from occupancy/blocker validation. | Same 1x1 candidate passes in generic scatter but fails when `reject_any_overlay=true`; proposed test `cellrect_check_passability_reject_overlay_flag_is_caller_specific` | Do not collapse it into `PathGrid::is_walkable`. |
| Zone matching uses `MovementZone`/zone ids, while terrain speed uses `SpeedType`; matrix rows are not SpeedType rows. | `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md` | partial/mismatch risk: local remap and `zone_layer_for_speed_type` still exist | `src/sim/pathfinding/passability.rs`, `zone_build.rs`, `zone_map.rs` | Keep MovementZone as the zone-row/source for reachability and SpeedType for speed/land table checks. | Unit with `MovementZone=Water` and non-water `SpeedType` still uses Water row for required-zone comparison; proposed test `cellrect_passability_zone_check_uses_movement_zone_not_speed_type` | Do not implement `SpeedType x LandType` as the zone matrix. |
| Required height `-1` through FNPC still selects bridge occupation bits on structural bridge cells; explicit non-`-1` height uses base vs `level+4` rules. | `0x004834A0` bridge occupancy report; FNPC caller matrix | missing exact selected-bitfield behavior | occupancy/pathfinding validator layer helpers | Preserve bridge and ground occupation bit selection independently from final `CheckOccupancy`. | Structural bridge cell with bridge bits set blocks FNPC passability even when ground bits are clear; proposed test `cellrect_passability_minus_one_checks_bridge_occupation_bits_on_bridge_cells` | Do not make required height `-1` mean "ground only". |
| `allow_bridge_cells` is not a `CheckPassability` argument; it is a later FNPC candidate filter. | caller matrix report | likely missing/unchecked | nearby-passable helper config | Keep bridge structural allow/reject outside the rectangle passability wrapper. | Same bridge candidate passes `CheckPassability` but is rejected only when FNPC allow-bridge flag is false; proposed test `find_nearby_allow_bridge_flag_filters_after_passability` | Do not name the wrapper's bridge/layer arg as `allow_bridge_cells`. |
| `CheckOccupancy(rect, -1)` is separate and skips `Cell+0xDC`; it must not be fused into this validator. | validator report; parent settled facts | no exact validator split | production spawn, world spawn, placement/deploy helpers | Implement passability and occupancy as distinct predicates with distinct caller flags. | Reservation-only cell passes FNPC final occupancy but a real blocker field fails; proposed test `find_nearby_final_occupancy_minus_one_skips_reservation_bits` | Do not use `Cell+0xDC` as GapGen or dynamic occupancy in FNPC. |

### Negative Facts / Do Not Do

- Do not call arg6 a locomotor type or SpeedType. It is `MovementZone` / zone-row family. Evidence: FNPC caller matrix and `TechnoType+0x5B4` matrix reader reports. Active in YR: Yes.
- Do not make `required_height_or_level` a public FNPC caller parameter. FNPC always supplies `-1` to this wrapper. Evidence: FNPC calls at `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`. Active in YR: Yes.
- Do not treat bridge structural flag as a blanket `CheckPassability` failure. Evidence: `0x004834A0` uses it for `level+4`, `+0x128`, and bridge selected path behavior. Active in YR: Yes.
- Do not treat matrix value `3` as passable/special bridge allowance. Evidence: direct readers require `== 1`. Active in YR: Yes.
- Do not merge `CheckPassability` and `CheckOccupancy`; their fields, call flags, and `Cell+0xDC` behavior differ. Evidence: validator report. Active in YR: Yes.

## 10. Remaining Uncertainty

- Fresh live Ghidra re-check was not possible because this subagent session had no callable Ghidra MCP endpoint. All binary claims here are from prior Ghidra-backed reports.
- Exact dummy-cell field values at `DAT_00ABDC50` were not re-dumped; behavior is only characterized by substitution and subsequent checks.
- Full non-FNPC callers of `CellClass::CheckCellPassability @ 0x004834A0` remain outside this report; only the path through `CellRect::CheckPassability` is claimed.
- Rust ownership of a future exact `CellRect` validator API is an implementation-design question, not resolved here.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace "`param_6` Locomotor type" with "`param_6` MovementZone / zone-row selector used by `CellRect::CheckPassability` and zone lookups; SpeedType is `param_4`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: replace "`param_13` Reject bridge cells" with "`param_13` allow-bridge-cells polarity: nonzero allows structural bridge cells; zero rejects cells with `CellClass+0x140 & 0x100` after passability checks."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace any `g_PassabilityMatrix[speed_type * 8 + ...]` wording with `g_PassabilityMatrix[movementZone * 8 + reducedZoneType]`; `SpeedType` belongs to the speed/land table, not the zone matrix row.

## Sources

- Prior Ghidra-backed reports read: `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`, `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`, `CELLCLASS_RECALCZONETYPE_00483C80_GHIDRA_REPORT.md`, `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`, `SPEEDTYPE_LANDTYPE_TABLE_GHIDRA_REPORT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md`.
- Addresses cited from those reports: `0x0056E7C0`, `0x004834A0`, `0x0056DC20`, `0x0056DE0E`, `0x0056E024`, `0x0056E265`, `0x0056E467`, `0x0056D230`, `0x00586780`, `0x00483C80`, `0x0082A594`, `0x0089EA40`.
- INI files searched: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust surfaces scanned: `src/sim/pathfinding/passability.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/occupancy.rs`, `src/sim/production/production_placement.rs`, `src/sim/world/world_spawn.rs`, `src/rules/locomotor_type.rs`.
