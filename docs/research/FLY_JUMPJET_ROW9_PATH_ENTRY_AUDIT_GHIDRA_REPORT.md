# Fly / Jumpjet Row 9 Path Entry Audit -- Ghidra Research Report

**Address(es):** `0x0056C510`, `0x0042C900`, `0x0042C290`, `0x004D3920`, `0x004CBBA0`, `0x004D94B0`, `0x004CCC80`, `0x004CD600`, `0x0054B1C0..0x0054B68C`, `0x0056DC20`, `0x004834A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** whether the built Fly `MovementZone` row 9 zone arrays affect standard YR Fly/JumpJet routing, and which path-entry functions use or bypass `Zone_precheck`/zone rebuild for Fly row 9.  
**Non-Scope:** rocket locomotor internals (`{B7B49766-...}`), full aircraft mission state machines, full `Find_Nearby_Passable_Cell` caller census, and full cell-level A* behavior after `Zone_precheck`.  
**Confidence:** High for FlyLocomotion and JumpjetLocomotion move-entry behavior; High for A*/Zone_precheck argument contract; Medium for "no player-visible row-map delta" because rocket locomotor was intentionally not expanded.  
**Active in YR:** Yes for all material entry points below. Stock `rulesmd.ini` has active `MovementZone=Fly` units using FlyLocomotion (`{4A582746-...}`) and JumpjetLocomotion (`{92612C46-...}`), and `TechnoClass__Set_Destination` reaches `FootClass__Set_Destination_Internal` for those objects.

## Target Question

Do the Fly row 9 zone arrays built by gamemd matter for standard YR Fly/JumpJet pathing, and which path-entry functions use or bypass `Zone_precheck` / zone rebuild for Fly?

## Non-Goals

- Do not re-prove the `MovementZone=` parser table or the 13-row rebuild loop; this report consumes the prior row-mapping report and spot-checks the path-entry side.
- Do not audit rocket/missile locomotor movement.
- Do not modify Rust.
- Do not claim all possible modded Fly-row callers are absent; this is standard YR FlyLocomotion/JumpjetLocomotion routing.

## Evidence Needed to Mark COMPLETE

- Decompile or assembly evidence that row 9 is built by the binary zone rebuild.
- Decompile evidence that `AStar_pathfind_search` uses `MovementZone` row and calls `Zone_precheck`.
- Caller/xref evidence for the normal A* path entry.
- Decompile evidence for FlyLocomotion move entry and process loop showing no `FootClass__Find_Path`, `AStar_pathfind_search`, or `Zone_precheck` call.
- Decompile/assembly evidence for JumpjetLocomotion move entry showing the nearby-passable validator call and its arguments.
- INI evidence that standard YR Fly/JumpJet stock content uses `MovementZone=Fly`.
- Current Rust source scan for whether Fly row maps are already built and whether air orders currently bypass ground A*.

## Stop Conditions

- Stop after FlyLocomotion and JumpjetLocomotion path-entry liveness are resolved.
- Stop before rocket locomotor and weapon projectile routing.
- Stop before patching Rust; handoff only.
- Stop if a claim about stock activity lacks INI evidence or decompile/caller evidence.

## 1. Overview

The binary really does build Fly row 9 zone arrays, but standard FlyLocomotion movement does not route through the `FootClass::Find_Path -> FootClass::Run_AStar -> AStar_pathfind_search -> Zone_precheck` stack. It stores a destination coordinate and advances by its flight controller.

JumpjetLocomotion also bypasses `Zone_precheck` for its normal `Head_To_Coord` entry. It does call `FootClass__Find_Nearby_Passable_Cell`, but the verified call passes `zone_id = -1`, `MovementZone = TechnoType+0x5B4`, and `SpeedType = TechnoType+0x67C`; `zone_id = -1` makes `CellClass__CheckCellPassability` skip the `MapClass__GetZoneID` same-zone comparison. Therefore the built Fly row 9 zone arrays are data-parity real but do not affect standard YR Fly/JumpJet route selection in this slice.

## 2. Key Offsets / Fields

| Field / data | Offset / address | Meaning in this slice | Active in YR |
|---|---:|---|---|
| `MovementZone` | `TechnoTypeClass+0x5B4` | Row selector; Fly is row 9. | Yes |
| `SpeedType` | `TechnoTypeClass+0x67C` | Terrain speed/passability class passed to cell validators. | Yes |
| Zone rebuild row arrays | `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` | Builds all 13 row arrays including Fly row 9. | Yes |
| A* path entry | `AStar_pathfind_search @ 0x0042C900` | Derives or accepts `MovementZone` row and calls `Zone_precheck`. | Yes |
| Fly destination | FlyLocomotion `+0x18/+0x1C/+0x20` in decompile view | Stores target X/Y/Z for direct flight controller. | Yes |
| Jumpjet target cell | JumpjetLocomotion `+0x3C/+0x40/+0x44` in assembly view | Stores resolved target coord, possibly after nearby-passable adjustment. | Yes |

## 3. Core Findings

### 3.1 Row 9 arrays are built, but that alone does not prove runtime use

Prior verified report `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md` shows `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` frees/builds all 13 rows and advances the passability row pointer until `0x82A734`. That includes parser/matrix row 9 (`Fly`).

**Verified binary finding. Active in YR: Yes.** The rebuild path is live at map initialization and bridge/zone mutation callsites. This report does not dispute it.

### 3.2 Normal ground pathing uses `Zone_precheck`, and Fly row would matter if this path were entered

`FootClass__Run_AStar @ 0x004CBBA0` is called by `FootClass__Find_Path @ 0x004D3920`, Drive, Ship, Walk, and a few helper paths. It calls:

- active locomotion vtable `+0x4C` to get a start/path coord,
- `Path_walk_directions_to_cell`,
- `AStar_pathfind_search(&global_pathfinder, ..., owner, ..., zone_arg, ...)`.

`AStar_pathfind_search @ 0x0042C900` reads `TechnoTypeClass+0x5B4` when caller passes `param_7 == -1`, calls `MapClass__GetZoneID` for source and destination, and calls `Zone_precheck(&resolved_from, &resolved_to, movement_zone, owner)` when hierarchy is enabled and the zone IDs match.

**Verified binary finding. Active in YR: Yes.** Evidence: decompiles `0x004CBBA0`, `0x0042C900`, caller xrefs to `0x004CBBA0`, and `Zone_precheck @ 0x0042C290` matrix row check `g_PassabilityMatrix[param_4 * 8 + zone_type] == 1`.

### 3.3 FlyLocomotion normal movement bypasses `FootClass__Find_Path`, A*, and `Zone_precheck`

`TechnoClass__Set_Destination @ 0x00741970` reaches `FootClass__Set_Destination_Internal @ 0x004D94B0`; the internal setter calls the active locomotor vtable `+0x44` with the destination object's coordinate.

For FlyLocomotion, the destination method is `FlyLocomotionClass__Move_To_Coord @ 0x004CCC80`. The function:

- early-outs for repeated same-cell destination while already moving,
- rejects several owner states via vtable checks,
- writes target X/Y/Z into FlyLocomotion fields,
- optionally adjusts target Z to ground height plus owner type altitude,
- starts takeoff when needed,
- sets a high-flight flag based on `ground_height + 0x78 < target_z`.

Its decompile contains no call to `FootClass__Find_Path`, `FootClass__Run_AStar`, `AStar_pathfind_search`, `Zone_precheck`, or `MapClass__GetZoneID`. `FlyLocomotionClass__Process @ 0x004CD600` advances the aircraft using facing, speed, altitude, bridge-height compensation, and landing/takeoff logic; it likewise does not enter `Zone_precheck` or A*.

**Verified binary finding. Active in YR: Yes.** Evidence: decompiles `0x004D94B0`, `0x004CCC80`, `0x004CD600`; stock `rulesmd.ini` active FlyLocomotion aircraft include `Locomotor={4A582746-...}` with `MovementZone=Fly` at lines around `10631..10790`, `11302..11349`, `11560..11595`.

### 3.4 Jumpjet normal movement bypasses `Zone_precheck`; it uses nearby-passable validation with `zone_id = -1`

JumpjetLocomotion's ILocomotion vtable points at the method body starting `0x0054B1C0` for the move-to-coordinate entry. Ghidra lacks a clean function boundary there, so this report uses assembly context.

The relevant path:

- `0x0054B1C0..0x0054B237`: stores the requested destination coordinate in JumpjetLocomotion target fields.
- `0x0054B5B0`: calls owner vtable `+0x84` and reads `TechnoTypeClass+0x5B4`.
- `0x0054B5BE`: pushes that movement-zone row.
- `0x0054B5BF`: pushes `-1`.
- `0x0054B5C3..0x0054B5C9`: calls owner vtable `+0x84` again and reads `TechnoTypeClass+0x67C`.
- `0x0054B5DF`: calls `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
- If a non-null coordinate is found, `0x0054B605..0x0054B683` converts the cell to center coords, checks bridge height, and calls owner vtable `+0x44` with the resolved coordinate.

Under the known `Find_Nearby_Passable_Cell` ABI, the push order at `0x0054B5BE..0x0054B5D9` corresponds to `zone_id = -1`, `movement_zone = Type+0x5B4`, `speed_type = Type+0x67C`, plus boolean filter arguments. `Find_Nearby_Passable_Cell` calls `CellRect__CheckPassability @ 0x0056E7C0`, which calls `CellClass__CheckCellPassability @ 0x004834A0`.

`CellClass__CheckCellPassability` only calls `MapClass__GetZoneID` when its zone-id argument is not `-1`. The first branch is:

```text
if (zone_id != -1) {
    cell_zone = MapClass__GetZoneID(cell, movement_zone, bridge_flag);
    if (zone_id != cell_zone) return 0;
}
```

Because the Jumpjet move-entry call passes `zone_id = -1`, this normal Jumpjet validation path skips Fly row 9 zone-array lookup and skips `Zone_precheck`. It still passes `MovementZone=Fly` into the cell validator for movement-zone-specific overlay rules, and passes `SpeedType=Hover` for stock jumpjets, but that is not the row-9 zone-array graph.

**Verified binary finding. Active in YR: Yes.** Evidence: assembly `0x0054B1C0..0x0054B68C`, decompiles `0x0056DC20`, `0x0056E7C0`, `0x004834A0`; stock `rulesmd.ini` active Jumpjet entries include `JumpJet=yes`, `Locomotor={92612C46-...}`, `MovementZone=Fly`, and `SpeedType=Hover` at lines around `3921..3965`, `10519..10569`, `10817..10868`, `10881..10928`, `11151..11196`, `11244..11260`.

### 3.5 `Find_Nearby_Passable_Cell` is not `Zone_precheck`

`FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` scans rings around a center cell, accepts up to `0x18` candidate cells, and validates each by:

- `TechnoClass__IsOnScreen`,
- `CellRect__CheckPassability`,
- optional height/bridge delta checks,
- optional `TechnoClass__Is_Current_Cell_Obstacle_Free`,
- optional bridge rejection,
- optional `CellRect__CheckOccupancy`.

Its direct callee list does not include `Zone_precheck` or `AStar_pathfind_search`; it includes `CellRect__CheckPassability`, `CellRect__CheckOccupancy`, and other local validators.

**Verified binary finding. Active in YR: Yes.** Evidence: decompile/callees of `0x0056DC20`.

## 4. INI Keys / Stock Activity

| Key / value | Evidence | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=Fly` | `rulesmd.ini` lines around `3950`, `4742`, `8727`, `10554`, `10632`, `10684`, `10731`, `10790`, `10853`, `10914`, `11182`, `11260`, `11303`, `11349`, `11561`, `11595` | Stores row 9 in `TechnoTypeClass+0x5B4`. | Yes |
| `Locomotor={4A582746-...}` | `rulesmd.ini` lines around `10631`, `10683`, `10730`, `10789`, `11302`, `11348`, `11560`, `11594` | FlyLocomotion, direct flight controller. | Yes |
| `Locomotor={92612C46-...}` | `rulesmd.ini` lines around `3948`, `4740`, `8725`, `10553`, `10852`, `10913`, `11181`, `11259`; binary constructor `0x0054AC40` vtable `0x007ECD68` | JumpjetLocomotion, nearby-passable move entry. | Yes |
| `SpeedType=Hover` | `rulesmd.ini` lines around `3965`, `4757`, `8726`, `10569`, `10868`, `10928`, `11196`, `11244` | Stock Jumpjet cell validator speed type. | Yes |

## 5. Integration Points

| Entry / function | Uses row 9 zone arrays? | Uses `Zone_precheck`? | Evidence | Active in YR |
|---|---|---:|---|---|
| `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` | Builds them | No, writer side | prior row-mapping report + xrefs | Yes |
| `FootClass__Find_Path @ 0x004D3920` -> `Run_AStar @ 0x004CBBA0` | Yes if mover row is Fly | Yes via `0x0042C900` | decompile/callers | Yes, but not normal Fly/Jumpjet move entry |
| `AStar_pathfind_search @ 0x0042C900` | Yes when `param_7 == -1` derives row 9 or caller passes 9 | Yes | decompile | Yes |
| `FlyLocomotionClass__Move_To_Coord @ 0x004CCC80` | No | No | decompile; no A*/Zone callees | Yes |
| `FlyLocomotionClass__Process @ 0x004CD600` | No | No | decompile; no A*/Zone callees | Yes |
| Jumpjet move entry `0x0054B1C0..0x0054B68C` | Bypasses row arrays by passing `zone_id=-1` to FNPC | No | assembly + `0x0056DC20`/`0x004834A0` decompile | Yes |
| `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` | Only through `CellClass__CheckCellPassability` if caller supplies zone id != -1 | No | decompile/callees | Yes |

## 6. Current Rust Implementation Status

| Surface | Current observed status | Delta for this target |
|---|---|---|
| `src/rules/locomotor_type.rs::MovementZone::all_ground` | Current dirty worktree includes Fly in the build list and comments that gamemd rebuilds every row. | No row-map delta needed for this audit. |
| `src/sim/pathfinding/zone_map.rs::can_reach` | No current special Fly always-true branch; missing map returns true for any movement zone. | No Fly-specific `can_reach` delta needed if Fly maps remain built. |
| `src/sim/pathfinding/zone_search.rs::can_use_reduced_zone_precheck` | Allows `MovementZone::Fly` in reduced precheck. | Guardrail: standard Fly/Jumpjet move commands should not be routed through this path; only ground fallback paths should. |
| `src/sim/world/world_commands.rs` | Air-layer commands call `air_movement::issue_air_move_command`; Jumpjet infantry can use short ground walk fallback only when the explicit fallback predicate fires. | Matches the no-A* standard air command shape; add regression coverage, not a row-map patch. |
| `src/sim/aircraft` | Fly-locomotor missions move by air mission/movement logic; path grid is only noted for paradrop passability. | Consistent with FlyLocomotion no-zone-precheck route selection. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Fly row 9 rebuild existence | verified-by-prior + consumed | `0x0056C510`, prior report | none |
| `AStar_pathfind_search` row/Zone_precheck contract | verified | decompile `0x0042C900`, `0x0042C290` | none for entry contract |
| `FootClass__Run_AStar` caller relation | verified | decompile `0x004CBBA0`; callers list | none |
| `FootClass__Find_Path` invokes A* | verified | decompile `0x004D3920` | full blocked-destination logic out of scope |
| `FootClass__Set_Destination_Internal` active-locomotor dispatch | verified | decompile `0x004D94B0` | none |
| FlyLocomotion `Move_To_Coord` | verified | decompile `0x004CCC80` | exact all flags/fields not named |
| FlyLocomotion `Process` | touched-sufficient | decompile `0x004CD600` | full flight physics out of scope |
| JumpjetLocomotion move entry | verified from assembly | `0x0054B1C0..0x0054B68C` | Ghidra function boundary missing; assembly is sufficient for call/arg contract |
| `Find_Nearby_Passable_Cell` no-Zone_precheck | verified | decompile/callees `0x0056DC20` | full candidate ordering out of scope |
| `CellClass__CheckCellPassability` zone-id `-1` bypass | verified | decompile `0x004834A0` | none for bypass |
| Rocket locomotor `{B7B49766-...}` | not-touched | intentionally non-scope | separate rocket locomotor audit |
| Current Rust Fly row maps | verified source scan | `locomotor_type.rs`, `zone_map.rs`, `zone_search.rs`, `world_commands.rs` | tests not run; no code edits |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Mode and scope? -> exhaustive-slice for Fly/Jumpjet row 9 path entry, excluding rocket locomotor.` (evidence: user target)
- `[RESOLVED] OQ-2 -- Are Fly row 9 zone arrays built? -> Yes, all 13 rows including Fly are built by `0x0056C510`.` (evidence: `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-3 -- What path uses `Zone_precheck`? -> `FootClass__Find_Path -> FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck`.` (evidence: `0x004D3920`, `0x004CBBA0`, `0x0042C900`, `0x0042C290`)
- `[RESOLVED] OQ-4 -- Would A* consume Fly row 9 if entered with row 9? -> Yes, `AStar_pathfind_search` derives `Type+0x5B4` when row arg is `-1`, and `Zone_precheck` indexes `g_PassabilityMatrix[row*8+type]`.` (evidence: `0x0042C900`, `0x0042C290`)
- `[RESOLVED] OQ-5 -- Does standard FlyLocomotion `Move_To_Coord` call A*/Zone_precheck? -> No; it stores destination/altitude/takeoff state only.` (evidence: `0x004CCC80`)
- `[RESOLVED] OQ-6 -- Does FlyLocomotion `Process` call A*/Zone_precheck? -> No; it advances facing/speed/altitude/landing controller.` (evidence: `0x004CD600`)
- `[RESOLVED] OQ-7 -- Does standard Jumpjet move entry call A*/Zone_precheck? -> No; it calls `Find_Nearby_Passable_Cell`, not A*/Zone_precheck.` (evidence: `0x0054B5DF`; callees `0x0056DC20`)
- `[RESOLVED] OQ-8 -- Does Jumpjet normal move use row arrays through FNPC? -> No for this call; it passes `zone_id=-1`, so `CellClass__CheckCellPassability` skips `MapClass__GetZoneID`.` (evidence: `0x0054B5BE..0x0054B5DF`, `0x004834A0`)
- `[RESOLVED] OQ-9 -- Does Jumpjet still pass `MovementZone=Fly` anywhere? -> Yes, it pushes `TechnoType+0x5B4` into FNPC/cell passability; this is movement-zone-specific validation, not zone graph precheck.` (evidence: `0x0054B5B0..0x0054B5BE`)
- `[RESOLVED] OQ-10 -- Are stock Fly/JumpJet entries active in YR? -> Yes, stock `rulesmd.ini` has active FlyLocomotion and JumpjetLocomotion units with `MovementZone=Fly`.` (evidence: INI lines listed in Section 4)
- `[RESOLVED] OQ-11 -- Does current Rust still special-case Fly can_reach as always true? -> Not in current `ZoneGrid::can_reach`; it uses maps if present and returns true only when map missing.` (evidence: source scan `zone_map.rs`)
- `[RESOLVED] OQ-12 -- Does current Rust build Fly row maps? -> Yes, current dirty worktree includes `MovementZone::Fly` in `all_ground`.` (evidence: source scan `locomotor_type.rs`)
- `[DEFERRED] OQ-13 -- Does RocketLocomotion `{B7B49766-...}` consume Fly row arrays?` (category: out-of-scope; reason: user target was Fly/JumpJet row-map audit and rocket/projectile locomotor is a separate movement class; next-step-if-pursued: trace the rocket locomotor vtable and move command/mission callers)
- `[DEFERRED] OQ-14 -- Runtime frequency of rare Fly-row calls into `FootClass__Find_Path` from nonstandard/modded states.` (category: needs-runtime-debugger; reason: static standard entries are resolved, modded/exception paths need instrumentation; next-step-if-pursued: break on `0x004CBBA0` and log `Type+0x5B4==9` callers)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Binary builds Fly row 9 zone arrays, but standard FlyLocomotion move/process does not enter A*/`Zone_precheck`. Active in YR: Yes. | `0x0056C510` prior; `0x004CCC80`, `0x004CD600` decompile | none for row-map construction; keep Fly in build list if already present | `src/rules/locomotor_type.rs::all_ground`, `src/sim/world/world_commands.rs`, `src/sim/aircraft` | Keep row 9 map data available for parity, but route standard Fly-locomotor move orders through air movement, not zone/A*. | Move a Harrier/Black Eagle over water/walls/blocked terrain; command should create straight-line air movement without zone reachability failure. Proposed test name: `fly_locomotor_move_order_bypasses_zone_precheck_even_with_blocked_ground`. | Do not remove Fly from binary-style row rebuild; do not make Fly aircraft depend on same-zone `ZoneGrid::can_reach`. |
| Jumpjet normal move entry calls `Find_Nearby_Passable_Cell` with `zone_id=-1`, `movement_zone=Type+0x5B4`, `speed_type=Type+0x67C`; this bypasses `MapClass__GetZoneID` and `Zone_precheck`. Active in YR: Yes. | assembly `0x0054B5B0..0x0054B5DF`; `0x004834A0` decompile | partial/guardrail: Rust air-layer Jumpjet move uses straight-line air command, while short infantry walk fallback uses ground A* by explicit predicate | `src/sim/world/world_commands.rs`, `src/sim/movement/jumpjet_movement.rs`, `src/sim/pathfinding/cell_entry.rs` | Standard airborne Jumpjet move should not fail due to row-9 zone connectivity; any landing/nearby-cell validation should be local cell legality, not hierarchical reachability. | Move a Rocketeer/Kirov across disconnected islands; airborne command succeeds without `ZoneGrid::can_reach(Fly)` gating. Proposed test name: `jumpjet_air_move_does_not_require_fly_zone_connectivity`. | Do not reuse `zone_search::find_path_zoned` for standard airborne Jumpjet movement. |
| The only verified normal `Zone_precheck` entry is FootClass A*/route-estimate stack; if a future caller intentionally enters it with row 9, row arrays are valid and should work. Active in YR: Yes. | `0x004CBBA0`, `0x0042C900`, `0x0042C290`; prior row rebuild report | none observed for current dirty worktree; Fly maps exist and `can_reach` no longer hard-returns true for Fly | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs` | Preserve generic row-9 data behavior for tests/tools/future edge paths, but avoid using it as the normal air order gate. | A direct `ZoneGrid` unit test can verify Fly row classifies in-bounds reduced-zone classes as connected while OoB/sentinel remains invalid. Proposed test name: `fly_zone_grid_exists_but_air_orders_do_not_consume_it`. | Do not interpret "standard air bypasses it" as "Fly row arrays are dead"; they are built and valid binary data. |

## Negative Facts / Do Not Do

- Do not say Fly row 9 zone arrays are absent; the binary builds all 13 rows including Fly. Active in YR: Yes. Evidence: `0x0056C510` prior verified report.
- Do not route standard FlyLocomotion movement through `FootClass__Find_Path`/A*/`Zone_precheck`; the verified Fly entry stores destination and uses flight physics. Active in YR: Yes. Evidence: `0x004CCC80`, `0x004CD600`.
- Do not route standard airborne Jumpjet movement through `Zone_precheck`; its normal move entry uses `Find_Nearby_Passable_Cell` with `zone_id=-1`. Active in YR: Yes. Evidence: `0x0054B5BE..0x0054B5DF`, `0x004834A0`.
- Do not treat `Find_Nearby_Passable_Cell` as an A* or hierarchical-zone path search. Active in YR: Yes. Evidence: callees of `0x0056DC20` do not include `Zone_precheck` or `AStar_pathfind_search`.
- Do not use `SpeedType` as the zone-row selector in the A*/Zone_precheck path; `AStar_pathfind_search` consumes `TechnoType+0x5B4` for the row. Active in YR: Yes. Evidence: `0x0042C900`.
- Do not collapse Jumpjet's local `MovementZone=Fly` passability validation with row-array reachability. It passes row 9 to the cell validator, but with `zone_id=-1` the row-array lookup is bypassed. Active in YR: Yes. Evidence: `0x0054B5BE..0x0054B5DF`, `0x004834A0`.

## Remaining Uncertainty

- RocketLocomotion (`{B7B49766-E576-11d3-9BD9-00104B972FE8}`) was not traced. Stock aircraft types using that locomotor also have `MovementZone=Fly`, but this report does not claim their route-entry behavior.
- Rare nonstandard states or mods could call `FootClass__Find_Path` with a `MovementZone=Fly` object. The generic A*/Zone_precheck contract would then use row 9 arrays; runtime frequency requires debugger instrumentation.
- Ghidra did not expose a clean function boundary for Jumpjet move entry at `0x0054B1C0`; the assembly context is sufficient for the call/argument proof, but a future project-local label could make the report easier to read.

## Stale Docs / Follow-up Docs

- `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md` is not stale, but its deferred wording can now be narrowed: replace "exact Fly locomotor runtime path remains conditional" with "standard FlyLocomotion `Move_To_Coord @ 0x004CCC80` and `Process @ 0x004CD600` bypass A*/`Zone_precheck`; standard Jumpjet move entry `0x0054B1C0..0x0054B68C` calls `Find_Nearby_Passable_Cell` with `zone_id=-1`, so it also bypasses row-array reachability. Rocket locomotor remains untraced."
- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` has older wording calling `param_7` "SpeedType/zone_type"; replacement wording: "`AStar_pathfind_search` consumes a `MovementZone` row argument; `0xFFFFFFFF` derives the row from `TechnoTypeClass+0x5B4`. SpeedType is separate and not the row selector."

## Sources

- Ghidra decompiled/read this session: `FootClass__Run_AStar @ 0x004CBBA0`; `FootClass__Find_Path @ 0x004D3920`; `AStar_pathfind_search @ 0x0042C900`; `Zone_precheck @ 0x0042C290`; `FootClass__Set_Destination_Internal @ 0x004D94B0`; `TechnoClass__Set_Destination @ 0x00741970`; `FlyLocomotionClass__Move_To_Coord @ 0x004CCC80`; `FlyLocomotionClass__Process @ 0x004CD600`; `JumpjetLocomotionClass__Constructor @ 0x0054AC40`; Jumpjet move-entry assembly `0x0054B1C0..0x0054B68C`; `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`; `CellRect__CheckPassability @ 0x0056E7C0`; `CellClass__CheckCellPassability @ 0x004834A0`.
- Ghidra xrefs/callees: callers of `0x0042C900`, `0x0042C290`, `0x004CBBA0`, `0x0056DC20`, `0x0056D230`, `0x0056C510`; Jumpjet vtable memory at `0x007ECD68`.
- Prior reports: `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`, `FUN_0042D170_BLOCKED_DESTINATION_ZONE_COST_HELPER_GHIDRA_REPORT.md`, `ZONE_GRAPH_ADJACENCY_EMISSION_ORDER_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`.
- Rust scanned read-only: `src/rules/locomotor_type.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/world/world_commands.rs`, `src/sim/aircraft/mod.rs`.
