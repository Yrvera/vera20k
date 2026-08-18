# Failed-A* Retry `Can_Enter_Cell` Adapter -- Ghidra Research Report

**Address(es):** `0x005840C0` (`ZoneMap__FloodFillReachableZones`),
`0x0042CCD0` (`PathfinderClass__UpdateHierarchicalEdges`), `0x0073F0A0`
(`UnitClass::Can_Enter_Cell`), `0x0051BF90` (`InfantryClass::Can_Enter_Cell`),
`0x004D9C60` (`CheckBridgeTraversal`)

**Investigation Mode:** coverage-map

**Claimed Scope:** resolve the concrete standard-YR mover variants, exact argument
tuple, layer consequences, and minimum Rust input surface for the virtual
`Can_Enter_Cell` call made by the failed-hierarchical-A* retry flood.

**Non-Scope:** implementing Rust, re-documenting every return-code branch of the
full Unit/Infantry classifiers, retry edge-selection after the flood result,
runtime branch frequency, or modded class/locomotor combinations.

**Confidence:** High for the call tuple, polarity, Unit/Infantry bindings, special
layer derivation, standard-YR reachability, and current Rust blocker. Medium for
the final architecture name because implementation design remains separate.

**Active in YR:** Yes. The call is on the live
`AStar_pathfind_search -> UpdateHierarchicalEdges -> FloodFillReachableZones`
retry path.

> **Correction:** older
> `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md` section
> 3.6 states the inverse local-flood polarity. Live assembly at
> `0x00584271..0x00584286` enters local bookkeeping when
> `Can_Enter_Cell == 0 || matrix_value != 1`; only a **nonzero** entry code paired
> with matrix value **1** skips the bookkeeping path.

## 0. Investigation Contract

Target question: can Rust supply the exact zero/nonzero result needed by
`0x005840C0` with a small adapter, and what state must that adapter consume?

Prior-work decision: the recent flood/retry reports explicitly deferred the
concrete virtual targets. This report extends only that gap; settled retry count,
progress-cell, and exclusion-order findings are not repeated.

Hypotheses tested:

1. A terrain/path-grid predicate is sufficient. **Refuted.** Both live class
   bodies retain overlays, ordered occupants, ownership, missions, weapons,
   shroud, occupancy bits, and bridge/tube policy.
2. Results from the failed ordinary A* can be reused. **Refuted.** Ordinary A*
   passes explicit parent/path height; this helper passes candidate-self height
   and null parent, and it may probe cells ordinary A* never expanded.
3. The movement-zone matrix alone is sufficient. **Refuted.** Matrix value `1`
   makes exact `Can_Enter_Cell == 0` decisive.
4. A fresh Unit/Infantry-dispatched read-only predicate is sufficient.
   **Supported**, provided it is backed by an exact native-shaped world snapshot,
   not the current approximate block maps.

## 1. Verdict

The smallest exact adapter is a fresh, read-only, class-dispatched
`Can_Enter_Cell` query with the fixed retry tuple:

```text
target      = neighbor CellClass
direction   = 0..7 in native direction-table order
height      = sign_extend(neighbor.Level)
parent      = null
arg5        = 1
result use  = result == 0 versus result != 0 only
```

For stock YR, the concrete Techno-side implementations reaching this path are
`UnitClass::Can_Enter_Cell @ 0x0073F0A0` and
`InfantryClass::Can_Enter_Cell @ 0x0051BF90`. This is still not an isolated
terrain helper: it needs the exact Unit/Infantry classifier state. Current Rust
does not expose that state to pathfinding and its classifier is explicitly an
approximation. Therefore production activation remains **BLOCKED**, while a
pure retry kernel accepting injected exact results is **READY**.

## 2. Exact Virtual Call and Polarity

Live `disassemble_function(0x005840C0)` proves the push sequence:

```asm
0058424d  PUSH 0x1
0058424f  PUSH 0x0
00584261  MOVSX EAX,byte ptr [ESI + 0x11b]
00584268  MOV EDX,dword ptr [ECX]
0058426a  PUSH EAX
0058426b  PUSH direction
00584270  PUSH ESI
00584271  CALL dword ptr [EDX + 0x1ac]
00584277  TEST EAX,EAX
00584279  JZ 0x0058428c
0058427b  ... load CellClass+0x4c matrix column ...
00584282  CMP dword ptr [movement_zone_row + column*4],0x1
00584286  JZ 0x00584339
0058428c  ... local-zone bookkeeping ...
```

Load-bearing details:

- `MOVSX` makes `CellClass+0x11B` a signed byte before it becomes the 32-bit
  height argument.
- `parent=0` is meaningful; `CheckBridgeTraversal` reconstructs the predecessor
  from the candidate and reverse direction.
- The helper fetches `MovementZone` through mover vtable `+0x84`, then reads type
  offset `+0x5B4`; the matrix row is 8 dwords (`SHL EAX,5`) at `0x005841B4..0x005841CE`.
- Return codes `1..7` all collapse to nonzero. No soft-code cost semantics are
  used inside this helper.
- Direction order is ascending `0..7` for every popped local-flood cell
  (`0x00584339..0x00584345`).

## 3. Special Tuple Layer Semantics

Both Unit and Infantry functions begin with the same early object-list decision:
a structural bridge cell selects ground when `abs(height - candidate.Level) < 2`.
Because this helper passes `height = candidate.Level`, the initial object-list
layer is always ground.

`CheckBridgeTraversal @ 0x004D9C60` then reconstructs the predecessor because
the parent argument is null. Substituting the helper's exact tuple gives:

| Reconstructed predecessor state | Effective bridge check | Result/layer effect |
|---|---|---|
| predecessor lacks structural bridge bit `0x100` | diff source becomes the supplied height, which equals candidate level | diff `0`; allowed; object list remains ground |
| predecessor has bridge, level diff `0` | supplied height already matches candidate level | allowed; object list remains ground |
| predecessor has bridge, absolute level diff `1` | normal direction-selected slope-byte test | missing required slope returns `7` |
| predecessor has bridge, absolute diff `2`, `3`, or `>=5` | normal hard rejection | returns `7` |
| predecessor bridge level is candidate level minus `4` | supplied height equals candidate level and predecessor has bridge | allowed; object list remains ground |
| candidate level is predecessor bridge level minus `4` | candidate must have structural bridge and bridgehead bit `0x200` | allowed and forces **bridge object list** |

The supplied height is never `-1`, so `CheckBridgeTraversal` does not rewrite it.
The later bridge-occupancy re-snapshot requires
`height == candidate.Level + 4`, which is impossible for this tuple. Therefore:

```text
terrain mode         = candidate ground-height mode
occupancy-bits layer = ground (+0x124 / ground sidecar +0x54)
object-list layer    = ground normally, bridge only on the verified diff-4 branch
```

This split is observable and must be preserved. On that diff-4 branch, both
class bodies scan the bridge object list while retaining ground occupancy bits.

Concrete fixture: candidate level `0`, predecessor level `4` with predecessor
bridge set, candidate bridge plus bridgehead set, and direction pointing from
predecessor to candidate. The call starts with height `0`; bridge traversal forces
the bridge object list, but the occupancy-bit snapshot remains ground because
`0 != 0 + 4`.

## 4. Concrete Class Coverage

### 4.1 UnitClass binding

Live RTTI/vtable walk:

- vtable base `0x007F5C70`; `read_memory(0x007F5C6C,8)` gives COL pointer
  `0x0080CC68` immediately before the table;
- COL `+0x0C` is TypeDescriptor `0x00842D80`;
- `inspect_memory_content(0x00842D88)` yields `.?AVUnitClass@@`;
- `read_memory(0x007F5E1C,8)` gives `+0x1AC = 0x0073F0A0` and
  `+0x1B0 = 0x004D9C60`.

`decompile_function(0x0073F0A0)` and its `RET 0x14` tail confirm the five-stack-
argument entry. With the retry tuple, the following remain active before the
final zero/nonzero result:

- tunnel-type and tube-direction gates; notably the supplied height equals the
  candidate level, so the documented overlay tunnel exception does not open;
- reconstructed-parent bridge traversal and split list/occupancy layers;
- shroud/reveal gate;
- the arg5-gated locomotor COM call;
- overlay/wall ownership and weapon policy;
- ordered ground/bridge object-list scan, including mission/target exceptions,
  garrison/gate/fence/building rules, alliances, crushing, and moving blockers;
- ground speed-type/land-type zero-cost rejection when the selected object list
  remains ground;
- ground occupancy-bit and owner resolution.

The COM subcall does not add a separate stock-YR terrain policy here:
`FootClass::LocomotorPassabilityCheck @ 0x004D9C10` calls locomotor vtable `+0x1C`,
while live `decompile_function(0x0055ABF0)` is an unconditional return `0` and
`get_xrefs_to(0x0055ABF0)` lists 11 locomotor-vtable DATA bindings. The fixed
`arg5=1` must still be represented as the native call mode, but all audited
concrete locomotors share the no-op result.

### 4.2 InfantryClass binding

Live RTTI/vtable walk:

- vtable base `0x007EB058`; `read_memory(0x007EB054,8)` gives COL pointer
  `0x008033B8`;
- COL `+0x0C` is TypeDescriptor `0x00825508`;
- `inspect_memory_content(0x00825510)` yields `.?AVInfantryClass@@`;
- `read_memory(0x007EB204,8)` gives `+0x1AC = 0x0051BF90` and
  `+0x1B0 = 0x004D9C60`.

Fresh `decompile_function(0x0051BF90)` resolves the prior report's live-Ghidra
gap, and the tail bytes contain `RET 0x14`. For this tuple:

- the infantry-only `height - candidate.Level > 4` shortcut is unreachable
  because the difference is exactly zero;
- tube, reconstructed-parent bridge, shroud, overlay/wall, ordered occupant,
  mission/target, building/garrison, alliance, weapon-range, speed-table, and
  ground subcell/occupancy resolution remain active;
- Infantry does **not** directly call `FootClass::LocomotorPassabilityCheck` in
  this body;
- it uses the same special split where bridge object-list selection can coexist
  with ground occupancy bits.

### 4.3 Why Aircraft, Building, and bare Foot are not adapter variants

- `AircraftClass::Can_Enter_Cell @ 0x00415B10` is an incompatible two-argument
  landing-blocker routine. It treats its second argument as an object, scans
  eight cells, and may issue movement commands. It cannot safely be the target
  of this cell predicate call.
- `BuildingClass` does not enter `FootClass::Find_Path` and is not a moving Foot
  object.
- bare `FootClass` is abstract; its `+0x1AC` binding is only the thin locomotor
  stub and is not a stock concrete mover.

Fresh `get_function_callers(0x004D3920)` shows the live `Find_Path` producers are
Drive, Ship, Hover, Walk, and Jumpjet locomotion paths. Stock `rulesmd.ini`
confirms the concrete object classes involved:

- `[JUMPJET]` is in `[InfantryTypes]` and uses the Jumpjet locomotor
  (`rulesmd.ini:998..1064`, `3916..3968`);
- `[SCHP]` and `[DISK]` are in `[VehicleTypes]` and use Jumpjet locomotion
  (`rulesmd.ini:1069..1152`, `8691..8730`, `10872..10928`), so they dispatch as
  UnitClass despite being airborne;
- `[ORCA]` is in `[AircraftTypes]` and uses Fly locomotion
  (`rulesmd.ini:1159..1172`, `10582..10632`); Fly is absent from the direct
  `Find_Path` caller inventory.

Thus standard YR reaches exactly the Unit and Infantry Techno-side predicates in
this retry adapter. Modded class/locomotor combinations are outside this report.

## 5. Why Ordinary A* Results Cannot Be Reused

The normal A* call supplies an explicit current-node CellClass and inherited path
height. The retry flood supplies null parent and the candidate's own level. Those
tuples can choose different bridge lists, slope outcomes, and occupancy layers.

The retry flood also covers a `2`, `4`, or `8` cell block for hierarchy levels
`0..2`, starting at the tracked progress cell and traversing directions in native
order. It may therefore ask about cells never generated by the failed ordinary
A*. Caching the failed A*'s entry results would be incomplete even if their tuple
were identical.

The adapter must evaluate each probed edge fresh in flood order against one
stable, read-only search snapshot. It must not mutate simulation state or consume
RNG; the audited class predicates are query paths for this call mode.

## 6. Current Rust Status

| Rust surface | Evidence | Exact adapter impact |
|---|---|---|
| `zone_search.rs` | file header `:4`; hierarchy branch returns directly from one `find_path_with_costs_hierarchy_marker_progress` call at `:295` | no native failed-A* flood/update/re-precheck loop is wired |
| `PathfindingContext` | `src/sim/movement/mod.rs:125` | contains grids/terrain/blocker counts only; no mover identity, class policy, live ordered cell objects, houses, missions, weapons, or visibility |
| `AStarOptions` | `src/sim/pathfinding/core.rs:692`; `is_infantry` at `:730` | has coarse mover flags and block maps, not a native cell-entry snapshot |
| `LayeredEntityBlockMap` | `core.rs:167`; one `cost_code` field at `:158` | one denormalized entry per cell cannot reproduce ordered multi-occupant Unit/Infantry scans or all zero-producing exceptions |
| occupied-cell classifier | `cell_entry.rs:449..455` | explicitly documents approximate priority and missing native candidate policy |

Existing Rust has useful pieces -- split `CanEnterLayerContext`, bridge traversal,
terrain/tube facts, occupancy, entities, alliances, and mover snapshots -- but
they are not assembled into an exact, immutable Unit/Infantry query surface at
the pathfinding boundary.

## 7. Minimum Rust Adapter Contract

Use a Rust-native callback/trait boundary, but preserve gamemd-native semantics:

```text
RetryCanEnterOracle::classify(neighbor, direction) -> CellEntryCode
```

The implementation behind it must be created from a stable search snapshot and
must internally dispatch Unit versus Infantry using the fixed tuple. Minimum
snapshot categories are:

1. mover class and ID; type/rules flags; owner/alliance view; mission, target,
   cargo/contact state; weapon/action/crush/garrison capabilities; shroud gate;
2. candidate and reconstructed-predecessor terrain level, slope, bridge/tube,
   overlay, ownership, visibility, speed-row, and playfield state;
3. ordered ground and bridge occupant IDs plus ground/bridge occupancy bits and
   occupant owner fields;
4. read-only occupant facts needed by the class-specific branches.

The retry producer itself needs only the returned zero/nonzero code plus the
existing movement-zone matrix value. Keeping the full code is preferable because
the same exact classifier can serve ordinary A* later, but this helper must branch
only on equality with zero.

Do not add all snapshot fields as more parameters to `find_path_zoned_marker`.
Pass one immutable oracle/context object through the search boundary.

## 8. Coverage Ledger

| Area | Status | Evidence | Remaining work |
|---|---|---|---|
| helper call count/order/signed height | verified | decompile + assembly `0x005840C0`, especially `0x0058424D..0x00584271` | none |
| local bookkeeping polarity | verified/corrected | `0x00584277..0x0058428C` | older doc wording remains stale |
| null-parent bridge behavior | verified | decompile `0x004D9C60`; tuple substitution above | runtime frequency of rare diff-4 split |
| Unit RTTI/vtable/body | verified | COL/TypeDescriptor/slot reads; decompile `0x0073F0A0`; `RET 0x14` | full Rust classifier implementation |
| Infantry RTTI/vtable/body | verified | COL/TypeDescriptor/slot reads; fresh decompile `0x0051BF90`; `RET 0x14` | full Rust classifier implementation |
| stock concrete caller classes | verified | `Find_Path` caller inventory plus `rulesmd.ini` type lists/sections | modded class/loco combinations out of scope |
| Aircraft adversarial check | verified excluded | decompile `0x00415B10`; incompatible semantics; Fly absent from caller inventory | none for stock YR |
| arg5 locomotor call | verified no-op for audited concrete vtables | decompile `0x004D9C10`, `0x0055ABF0`; 11 DATA xrefs | retain fixed call mode in reusable context |
| current Rust context sufficiency | verified insufficient | source rows in section 6 | implementation/design phase |

## 9. Open Questions -- Final State

- `[RESOLVED] tuple -- neighbor, direction 0..7, signed neighbor.Level, null parent, arg5=1.`
- `[RESOLVED] polarity -- local bookkeeping runs on code 0 OR matrix != 1.`
- `[RESOLVED] variants -- stock YR dispatches UnitClass or InfantryClass.`
- `[RESOLVED] initial layer -- candidate-self height makes the early list choice ground.`
- `[RESOLVED] split layer -- one diff-4 bridge branch can force bridge object list while occupancy bits stay ground.`
- `[RESOLVED] high infantry shortcut -- unreachable for this tuple because height-level is zero.`
- `[RESOLVED] arg5 -- Unit reaches the locomotor COM slot; all audited concrete vtables bind the return-0 stub; Infantry does not make the direct subcall.`
- `[RESOLVED] cache -- ordinary failed-A* results cannot be reused because tuple and coverage differ.`
- `[RESOLVED] smallest exact surface -- one immutable, class-dispatched retry cell-entry oracle.`
- `[DEFERRED] runtime frequency -- frequency of zero/nonzero flood outcomes and the rare bridge-list/ground-bits split.` (category: `needs-runtime-debugger`; next step: instrument stock-map failed hierarchy retries)
- `[DEFERRED] full Rust classifier -- exact implementation of all Unit/Infantry zero-producing branches.` (category: `requires-implementation-design`; next step: synthesize the existing Unit/Infantry reports into one classifier implementation contract)
- `[DEFERRED] mod support -- behavior for deliberately mismatched class/locomotor INI combinations.` (category: `out-of-scope`; stock retail parity is the active target)

## 10. Implementation Handoff

| Verified behavior | Current Rust delta | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|
| Retry flood invokes a fresh Unit/Infantry predicate with candidate-self signed height and null parent. | no exact oracle at path boundary | snapshot exact cell-entry state once per search and inject one immutable class-dispatched oracle | ordinary A* explicit-parent result differs from retry null-parent result for the same bridge pair; retry uses the latter | do not reuse PathGrid walkability or ordinary A* cache |
| Initial object list and occupancy bits are ground; one diff-4 branch forces bridge object list only. | split layer type exists, but no retry producer/oracle uses it | derive and pass `{terrain=ground, object_list=ground/bridge, occupancy_bits=ground}` exactly | candidate level 0 / predecessor bridge level 4 fixture scans bridge occupants but ground occupancy bits | do not collapse to one `MovementLayer` |
| Code 0 and codes 1..7 matter only as zero/nonzero, combined with matrix condition `code==0 || matrix!=1`. | pure injected-result kernel allowed by existing contract | keep matrix test outside oracle and preserve native branch polarity/order | code 0 + matrix 1 enters bookkeeping; code 5 + matrix 1 skips; code 5 + matrix 0 enters | do not invert the legacy report's stale wording |
| Ordered dynamic Unit/Infantry policy remains active. | current block sets/maps are lossy approximations | oracle snapshot must preserve ordered occupants and class-specific mission/owner/weapon/building facts | same cell occupants in reversed native order can produce the matching native zero/nonzero outcome | do not reduce to one primary blocker per cell |

Suggested next task:

```text
/implementation-contract exact shared UnitClass/InfantryClass Can_Enter_Cell read-only snapshot and classifier for A* plus failed-retry flood
```

The existing failed-A* retry contract can remain `PARTIAL_READY`: implement and
test its pure injected-result kernel, but do not activate production retry until
this shared exact classifier contract is satisfied.

## 11. Negative Facts / Do Not Do

- Do not treat `Can_Enter_Cell != 0` as locally reachable; that is the stale
  inverse polarity.
- Do not call the Aircraft `+0x1AC` routine as a cell predicate.
- Do not use MovementZone or SpeedType to select Unit versus Infantry; class
  dispatch is independent (`SCHP`/`DISK` are UnitClass with `MovementZone=Fly`).
- Do not use candidate level to infer one final bridge layer.
- Do not assume `arg5=1` adds a second stock locomotor terrain policy; the
  concrete locomotor slot is the return-0 stub in the audited binary.
- Do not add a retry-only approximate classifier. The adapter should reuse the
  eventual exact Unit/Infantry classifier with a different argument tuple.
- Do not read mutable world state during the flood in a way that changes with
  Rust borrow/iteration artifacts; use one deterministic search snapshot.

## 12. Sources

Live Ghidra, read-only:

- `decompile_function` and `disassemble_function`: `0x005840C0`.
- `decompile_function`: `0x0042C900`, `0x0042CCD0`, `0x004D9C60`,
  `0x004D9C10`, `0x0073F0A0`, `0x0051BF90`, `0x00415B10`, `0x0055ABF0`.
- `read_memory`: Unit vtable/COL/slots `0x007F5C6C`, `0x0080CC68`,
  `0x007F5E1C`; Infantry `0x007EB054`, `0x008033B8`, `0x007EB204`;
  function tails `0x0073FD30`, `0x0051C870`.
- `inspect_memory_content`: Unit TypeDescriptor name `0x00842D88`;
  Infantry TypeDescriptor name `0x00825510`.
- `get_function_callers(0x004D3920)` and
  `get_xrefs_to(0x0055ABF0)`.

Repository evidence:

- `ini/rulesmd.ini` type lists and `[JUMPJET]`, `[SCHP]`, `[DISK]`, `[ORCA]`.
- `src/sim/pathfinding/zone_search.rs`.
- `src/sim/pathfinding/core.rs`.
- `src/sim/pathfinding/cell_entry.rs`.
- `src/sim/movement/mod.rs`.

Prior reports used for navigation and contradiction checks:

- `ZONEMAP_FLOODFILLREACHABLEZONES_RETRY_PRODUCER_GHIDRA_REPORT.md`.
- `UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`.
- `CELLCLASS_SUBSTRATE_CAN_ENTER_CELL_RUNTIME_SHAPE_GHIDRA_REPORT.md`.
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`.
- `INFANTRYCLASS_CAN_ENTER_CELL_VTABLE_0X1AC_GHIDRA_REPORT.md`.
- `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`.

**Status:** COMPLETE for the failed-A* retry `Can_Enter_Cell` adapter contract.
