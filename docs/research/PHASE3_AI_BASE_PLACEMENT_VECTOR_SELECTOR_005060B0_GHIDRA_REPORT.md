# Phase 3 AI Base-Placement Vector Selector (`0x005060B0`) — Ghidra Report

**Date:** 2026-08-26
**Binary:** active Yuri's Revenge `gamemd.exe`, image base `0x00400000`
**Primary function:** `FUN_005060B0 @ 0x005060B0..0x00506B65`
**Mode:** `/re-investigate`, exhaustive single-mechanism slice
**Status:** **COMPLETE** for the selector core and its ownership boundary; **not** a
closure claim for the separate BasePlan, defense-influence, or production-scheduler mechanisms
**Rust revision inspected:** `origin/main` at `eeb2515e361669e08c7570baeb57a90fe56cb68b`
**Confidence:** high for every implementation-handoff claim; load-bearing bounds,
signedness, ordering, activity, return, and RNG claims have decompile plus assembly or
decompile plus caller evidence.

## Scope

This report closes the active selector that consumes the ordered House/Base reservation
perimeter vector maintained by `BuildingClass::MarkBaseReservation` and its clearer. It
establishes:

- every direct caller and ordinary YR reachability;
- naval, uninitialized-base, and null-type branches;
- perimeter-vector iteration, both score callbacks, the cloak-generator override,
  signed sort semantics, and tie behavior;
- adjacent `CellClass+0xDC` probes, their signed-short vector sum, and symmetric
  cancellation;
- coordinate derivation, two local passes, three attempts per pass, two whole retries,
  all rejection gates in order, success, and failure;
- RNG/non-RNG behavior; and
- the exact current-Rust ownership and missing delta.

`HouseClass__AI_ScanBasePerimeter @ 0x005082C0` and AITrigger/Team AI remain outside this
slice. The caller-side BasePlan, defense-influence, and production scheduler/Strategy
mechanisms are bounded below because they own inputs and call timing, but their internal
policies remain separate named implementation mechanisms. The naval branch is included
because it is an active branch of this function and must bypass the vector path exactly.

No Tiberian Sun source, address, or behavior was used as authority. All reachability and
mechanism claims below come from the open active `gamemd.exe`; direct active YR callers
prove this is not an inherited-but-dead TS helper.

## Verdict

`FUN_005060B0` is the active deterministic House AI building-site selector. For ordinary
non-naval buildings it scores the writer-maintained ordered perimeter vector, sorts by a
signed 32-bit key, derives an outward site from the signed-short **sum of every** same-House
reservation direction among eight clockwise probes, skips a final zero vector (including
symmetric cancellation), and tests up to 12 sites per retained vector entry (two local
passes times three sites, repeated as a whole twice). It does not perform a radius spiral
and does not consume scenario RNG. This correction is cold-verified by
`disassemble_function(address="0x005060B0", program="/gamemd.exe")` at
`0x0050655B..0x005065E0` and `disassemble_function(address="0x0042D510",
program="/gamemd.exe")` at `0x0042D510..0x0042D536`.

Current Rust has the exact prerequisite perimeter vector but does not consume it.
`src/sim/ai.rs::find_placement_cell` still uses a clamped radius-12 square-ring scan around
an average of live structure anchors. Its partial reservation predicate also orders the
native gates differently, omits the selector's base-height gate, and sends Naval types
through the non-naval spiral. This is a material, frequent stock-skirmish divergence whenever
an AI places a completed building. However, replacing that ready-time spiral with this helper
alone would still not close native behavior: current Rust lacks the distinct base-plan center,
the BasePlan node/site lifecycle, and the production-planning influence grid that owns the
standard callback.

## 1. Function contract and direct callers

The effective ABI is:

```text
CellStruct* __thiscall HouseClass::SelectBasePlacementCell(
    HouseClass* this,                 // ECX
    CellStruct* out,                  // stack +0x08
    BuildingTypeClass* type,          // stack +0x0C
    int (__fastcall *score)(...),     // stack +0x10; House in ECX, cell* in EDX
    int score_arg                     // stack +0x14
);                                   // RET 0x10
```

`get_function_xrefs(0x005060B0)` returns exactly three call instructions in two functions:

| Callsite | Caller | Arguments and role |
|---|---|---|
| `0x00444FE1` | `BuildingClass__ExitObject_Main @ 0x00443C60` | House=`Building+0x21C`, type, scorer `0x00505F80`, arg `-1`; non-human AI construction/factory exit-placement path |
| `0x004450BD` | same | same selector arguments on the caller's alternate placement branch |
| `0x00507A20` | `HouseClass__AI_ChooseNextProduction @ 0x00506EF0` | chosen type, scorer `FUN_00505FD0`, arg `selected_quadrant*2` (`0,2,4,6`); preselects a building site when its optional candidate-list parameter is null |

At both `ExitObject_Main` sites, raw assembly pushes `-1`, `0x00505F80`, the type,
and the output pointer, then sets ECX from `[building+0x21C]`. The enclosing switch case
has already rejected a human-controlled House. This is the normal computer building
placement path, not an editor-only or legacy-only path.

`HouseClass__AI_ChooseNextProduction` has one direct caller,
`HouseClass__AI_Choose_Building @ 0x004FE3E0` at `0x004FE633`.
`AI_Choose_Building` is called from `HouseClass__AI_Manage_Build_Queue` at `0x004FE3B5`
and from normal `HouseClass__Update` at `0x004F90AE`, `0x004F9242`, and `0x004F924B`.
This independently proves ordinary active YR tick reachability.

The non-null optional-candidate-list branch of `AI_ChooseNextProduction` scores and sorts
that supplied list itself; it does **not** call `0x005060B0`. It is not another selector
caller.

## 2. Entry branches and evidence-backed exclusions

### 2.1 Null type

Assembly `0x005060C6..0x005060DF` stores the packed `_g_InvalidCell` from
`0x00A8EF98` in `out` and returns immediately. No vector, map, or RNG access occurs.

### 2.2 Naval type (`BuildingType+0x0CCE != 0`)

Assembly `0x005060E2..0x0050623A` bypasses the perimeter vector completely.

1. It calls `HouseClass__FirstBuildableFromArray @ 0x005051E0` twice on the Rules
   `Shipyard=` vector rooted at `Rules+0x880`; the first result supplies foundation width
   and the independently repeated result supplies foundation height. This is a source-order
   shipyard selector, not generic `BuildOption.enabled`: it applies Owner,
   RequiredHouses, ForbiddenHouses, and exact `AIBasePlanningSide`, followed only by the
   primary-superweapon shell-disable/`BuildTech` exemption tail. It applies no TechLevel,
   prerequisites, factory, BuildLimit, cost, credit, stolen-tech, or category gate.
2. Its seed is `House+0x5494` unless that packed cell is the invalid sentinel, in which
   case it uses `House+0x5490`.
3. It calls `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` with native arguments:
   `out, origin, 5, -1, 0, 0, width+2, height+2, 0, 0, 0, 1,
   &packed_zero, 0, 0`. Here `-1` disables required-zone equality, MovementZone is
   `0` (Normal), bridge-aware height/zone input is `0`, and structural bridges are
   allowed. Packed-zero reference selects by current-frame modulo rather than RNG.
4. If the owned BuildConst vector at `House+0x54` has a first object, the result must be
   within `Rules+0xE0C * 256` leptons of it; otherwise the function returns invalid.

`Rules+0xE0C` is `[General] AINavalYardAdjacency`, **not** `MaxBaseDistance`.
Retail `rulesmd.ini` sets `AINavalYardAdjacency=20`, `Shipyard=GAYARD,NAYARD,YAYARD`,
and has many active `Naval=yes` types. Therefore this is an active stock branch, but it is
an evidence-backed exclusion from perimeter-vector selection.

The selector itself calls no RNG in this branch. FNPC's packed-zero reference selects by
simulation frame modulo the chosen pool length; it is deterministic and is not
`Random__RandomRanged`.

This naval call makes one previously omitted shared-helper rule reachable. Because its
bridge-aware input is zero while structural bridge cells are allowed, FNPC runs
`FUN_006D6410` at four collection-side sites and again during final pool partition. The
helper reads candidate `CellClass+0x140 & 0x1000`; only when that bit is set, each projected
probe with `CellClass+0x140 & 0x100` receives **four additional signed height levels** before
the isometric direct/indirect comparison. That can change first-direct-ring termination,
pool membership and length, and the frame-modulo-selected cell. Fresh
`decompile_function(address="0x006D6410", program="/gamemd.exe")` and
`disassemble_function(address="0x006D6410", program="/gamemd.exe")` verify the flag gates
and `+4` at `0x006D6473..0x006D6513`; `get_function_xrefs(address="0x006D6410",
program="/gamemd.exe")` verifies FNPC callsites `0x0056DF0B`, `0x0056E122`,
`0x0056E364`, `0x0056E559`, and `0x0056E62E`.

Current Rust already retains and stamps those bits as
`BRIDGE_FLAG_FORWARD_SIDE=0x1000` and `BRIDGE_FLAG_STRUCTURAL=0x100` in
`src/map/bridge_facts.rs`, but `src/sim/find_nearby_cell.rs::is_direct_candidate`
explicitly omits the correction under stale `UNCHECKED` wording. The smallest prerequisite
is local to that shared pure classifier: include candidate-forward-side/probe-structural
height adjustment and test both collection early-stop and final direct/fallback pool
selection. No bridge destruction, cliff, or new flag-writer work is required.

### 2.3 Non-naval type with `(House+0x5750,+0x5752) == (0,0)`

Assembly/decompile `0x00506248..0x005062C1` bypasses vector scoring and validation. It
returns `House+0x5494`, or `House+0x5490` if `+0x5494` is the packed-zero
empty/invalid sentinel. `House+0x5750..+0x5753` is the distinct **base-plan center**, not
the primary/launch cell and not a reservation-writer bound. Fresh
`disassemble_function(address="0x005060B0", program="/gamemd.exe")` verifies the three
field reads at `0x00506248..0x005062A3`.

### 2.4 Three House cell authorities must remain distinct

The complete direct-instruction inventories are separate:

- `House+0x5490` is the primary/starting cell. The House constructor initializes it from
  packed-zero `0x00A8EF98` at `0x004F5A3C..0x004F5A41`; active writers include
  `HouseClass__Set_Starting_Cell @ 0x0050E000`, Recenter, ComputerTakeover, and the
  separate base-center recalculation routine. The selector reads it only as the naval
  origin or zero-base-plan-center fallback.
- `House+0x5494` is the alternate/override primary cell. The constructor initializes it
  to the same packed zero at `0x004F5A47..0x004F5A4D`. Trigger action `137` reaches
  `FUN_006E44E0 -> FUN_0050DFE0` and writes a valid waypoint cell; action `138` reaches
  `FUN_006E4540 -> FUN_0050DFF0` and restores packed zero. The selector prefers it over
  `+0x5490` only in the two fallback/origin branches above. Fresh
  `get_function_xrefs(address="0x0050DFE0"/"0x0050DFF0", program="/gamemd.exe")`,
  `decompile_function(address="0x006E44E0"/"0x006E4540", program="/gamemd.exe")`, and
  `disassemble_function(address="0x0050DFE0"/"0x0050DFF0", program="/gamemd.exe")`
  verify both writers and TriggerAction cases `0x89/0x8A`. The tracked exhaustive report's
  physical retail census finds action 137 in exactly three shipped YR campaign maps; a
  separate resolved-name scan of 310 archive maps plus two loose maps reproduced those
  three and found no action 138 occurrence.
- `House+0x5750` is the base-plan center used by both score callbacks, selector zero/nonzero
  dispatch, orientation reference, and anchor-height reference. The complete direct
  `search_instructions(operand_pattern="0x5750", limit=200, program="/gamemd.exe")`
  inventory is untruncated and contains four stores: ordinary reverse-owned-order
  `HouseClass__RecenterBase @ 0x0050C325`, successful non-player ConstructionYard deploy
  `0x007398F3`, the conditional ComputerTakeover writer `0x0050A7E9`, and TriggerAction
  case `0x1E` at `0x006DE270`. The last has no occurrence in the resolved retail map
  census; it remains a supported binary writer, not an ordinary stock-skirmish input.
  Recenter and deploy also write `+0x5490`, but that synchronization does not merge the
  fields: other writers and selector uses remain distinct.

`HouseState::base_center` at the inspected Rust revision is assigned from the launch/start
waypoint and corresponds to the primary `+0x5490` authority. There is no alternate cell or
distinct base-plan-center state. Reusing or recentering that field as `+0x5750` would break
its existing launch/primary consumers.

### 2.5 Cloak-generator scoring override

When `BuildingType+0x16C7` (`CloakGenerator=`) is nonzero, the function ignores the
supplied callback. For each perimeter cell it gets the House base/starting cell's 3-D
world coordinate, compares it to the candidate cell center `(x*256+128,y*256+128,0)`,
and forms:

```text
key = wrapping_i32(vector_index + ftol(Sqrt_Approx(dx*dx + dy*dy + dz*dz)) * 1000)
```

This branch is implemented and reachable for custom data, but no active
`CloakGenerator=yes` occurs in stock `rulesmd.ini` or `rules.ini`. It is therefore an
evidence-backed stock-retail exclusion, not permission to omit custom-data behavior.

## 3. Perimeter-vector traversal and scoring

### 3.1 Source order

The non-naval main path reads:

- coordinate storage pointer: `House+0x5724`;
- signed count: `House+0x5730`;
- element: packed signed-short `(x,y)` at `data + 4*index`.

It traverses `index=0,1,...,count-1`. This is the exact stable append/remove order
maintained by the reservation writer; it is not an `ExitList`, foundation list, map scan,
or recomputed geometric perimeter. A signed count `<=0` yields no scored entries.

Each scored record is exactly `{ i32 key, packed CellStruct coordinate }`. Native vector
allocation failure can omit an append; that OOM behavior is not a normal parity target.

### 3.2 Exit-placement score callback at `0x00505F80`

This coherent callback body is not currently defined as a Ghidra function, but both
`ExitObject_Main` callsites push its aligned entry and it ends in `RET 0x8` at
`0x00505FBF`. Assembly uses `MOVSX`, signed `SUB`, signed absolute-value idioms, and
signed `JG`:

```text
dx = signed_short(candidate.x) - signed_short(House+0x5750)
dy = signed_short(candidate.y) - signed_short(House+0x5752)
coarse = max(abs_i32(dx), abs_i32(dy))        // Chebyshev cell distance
key = wrapping_i32(vector_index + coarse * 1000)
```

The second callback argument is ignored. This is **not** square root/Euclidean distance.

### 3.3 Threat-and-angle score callback `FUN_00505FD0`

The callback indexes the current House threat grid:

```text
linear = (candidate.y - House+0x5758) * (House+0x575C)
       + (candidate.x - House+0x5754)
threat = *(i32*)(House+0x16060 + 4*linear)
```

If `score_arg == -1`, angular penalty is zero. Otherwise:

```text
raw16 = ftol((atan2(-(candidate.y-base_y), candidate.x-base_x) - pi/2)
             * (-32768/pi))
angle_byte = ((((uint16)raw16 >> 7) + 1) >> 1) & 0xff
delta = abs_i32(score_arg * 32 - angle_byte)
if delta >= 128: delta = 255 - delta       // literal 255, not 256
key = wrapping_i32(vector_index + (threat * 128 + delta) * 1000)
```

`AI_ChooseNextProduction` passes quadrant indices doubled, so its ideal angle bytes are
`0`, `64`, `128`, and `192`. Constants are the live `pi/2 @ 0x007E2820` and
`-32768/pi @ 0x007E2818`; the x87 `ftol`, unsigned 16-bit truncation, shifts, and all
integer math feeding the later comparator are load-bearing.

### 3.4 Sort signedness and ties

The function calls `FUN_007C8B48` (the shipped CRT-style `qsort`) with record size `8`
and comparator `0x005108F0`. Comparator assembly `CMP EAX,ECX`, `SETGE`, and the
`DEC/AND/INC` sequence returns:

- `-1` when left key is signed-less-than right;
- `0` for equality;
- `+1` when left key is signed-greater-than right.

The result is ascending **signed `i32`** order. Coordinates are then copied in that sort
order to a second vector and traversed from index zero upward.

Equal keys are not given an insertion-order tie-break. `FUN_007C8B48` is deterministic
but unstable, and its complete linked-retail behavior is required rather than delegated
to a host-library sort:

1. ranges of at most eight records use `FUN_007C8C9C`, repeatedly retaining the first
   strictly greatest record, swapping it with the current high record through byte-swap
   helper `FUN_007C8CEA`, and decrementing high;
2. longer ranges swap `low + floor(count/2)` with low and use it as the pivot;
3. the forward scan advances while `candidate <= pivot`, the reverse scan retreats while
   `candidate >= pivot`, crossed scans swap the pivot with the reverse record, and native
   continues with the smaller partition while pushing the larger partition on its fixed
   30-entry local stacks. The strict range tests are `low+1 < reverse`, `forward < high`,
   and `(reverse-low-1) < (high-forward)` for the smaller-partition choice.

Assembly-derived fixtures are exact:

```text
input ids, all keys equal              native output ids
[0,1]                                  [1,0]
[0,1,2,3,4,5,6,7]                      [1,2,3,4,5,6,7,0]
[0,1,2,3,4,5,6,7,8]                    [4,1,2,3,0,5,6,7,8]
```

Collisions are possible: in a 1,001-record fixture, `id=0` has index `0`, score `1`, key
`1000`; records `id=1..1000` have score `0`, key equal to index. Native places the equal
keys at sorted positions `999:id=1000`, `1000:id=0`. Rust must locally port this sorter;
neither stable sort nor the standard-library unstable sort is an exact substitute.

## 4. Reservation-neighbor probe and direction choice

Before candidate traversal the selector resolves the CellClass at `House+0x5750`, reads
its **signed byte** height at `CellClass+0x11B`, and caches the same-House reservation bit
`1 << (House+0x30 & 31)`.

For every sorted perimeter coordinate it probes all eight entries of
`g_DirectionOffsets @ 0x0089F688` in this exact order:

| index | delta | direction |
|---:|---:|---|
| 0 | `(0,-1)` | N |
| 1 | `(1,-1)` | NE |
| 2 | `(1,0)` | E |
| 3 | `(1,1)` | SE |
| 4 | `(0,1)` | S |
| 5 | `(-1,1)` | SW |
| 6 | `(-1,0)` | W |
| 7 | `(-1,-1)` | NW |

Each probe uses `MapCoord_Add @ 0x0042D510`, whose two word `ADD`s wrap X and Y
independently at 16 bits, resolves the adjacent CellClass, and tests its `+0xDC` mask.
Every hit is added through the same helper to a signed-short accumulator initialized to
`(0,0)`. Native does **not** retain the last hit. After all eight probes, a final `(0,0)`
sum skips the perimeter coordinate. Thus no hits skip, and symmetric arrangements such as
N+S or all eight directions also cancel and skip. (The eight unique unit offsets bound
the mathematical sum to `[-3,3]` per component, but the mechanism still uses word-wrap
addition and candidate-neighbor coordinates can wrap.)

A nonzero `(sum_x,sum_y)` becomes direction index:

```text
raw16 = ftol((atan2(sum_y, -sum_x) - pi/2) * (-32768/pi))
q = ((((uint16)raw16 >> 12) + 1) >> 1) & 7
```

There is no priority for the last scanned direction and no random wobble.

Map-edge probes use the native shared dummy CellClass behavior. A faithful Rust port must
use the existing reservation grid's off-map/dummy semantics and 16-bit packed-coordinate
arithmetic, not clamp candidates to `0..511`.

## 5. Candidate-coordinate derivation

Let:

```text
W = BuildingType foundation width
H = BuildingType foundation height
b = signed Rules.AIBaseSpacing (Rules+0x1460)
extra = 1 if (BuildingType+0x1765 ProtectWithWall)
             OR (BuildingType+0x1578 WantsExtraSpace), else 0
s = wrapping_i32(b + extra)
```

Assembly `0x00506673` and `0x00506684` proves the second flag's byte offset is direct
`BuildingType+0x1578`. The decompiler spelling `param_3[0x55e]` scales through `int*`
and must not be read as byte offset `+0x55E`.

The selector stores the following initial signed-short offset; conversion/truncation and
later coordinate adds have native 16-bit wrapping semantics:

```text
ox = s           when q in {1,2,3}
   = 1-W-s       when q in {5,6,7}
   = 0           when q in {0,4}

oy = s+1         when q in {3,4,5}
   = 1-H-s       when q in {7,0,1}
   = 0           when q in {2,6}
```

Two selector-local direction tables are initialized once and are **rotations**, not copies
of the static N-first table:

| q | local T1 at `0x00A8EF78` | local T2 at `0x00A8EFA8` |
|---:|---|---|
| 0 | W `(-1,0)` | E `(1,0)` |
| 1 | NW `(-1,-1)` | SE `(1,1)` |
| 2 | N `(0,-1)` | S `(0,1)` |
| 3 | NE `(1,-1)` | SW `(-1,1)` |
| 4 | E `(1,0)` | W `(-1,0)` |
| 5 | SE `(1,1)` | NW `(-1,-1)` |
| 6 | S `(0,1)` | N `(0,-1)` |
| 7 | SW `(-1,1)` | NE `(1,-1)` |

For each candidate, each of two local passes starts from:

```text
p0 = perimeter_candidate + (ox,oy) + T1[q]
```

- local pass 0 uses clearance `s` and tests `p0`, `p0+T2[q]`,
  `p0+2*T2[q]`;
- local pass 1 first adds static `g_DirectionOffsets[(q-4)&7]` to `p0`, uses
  clearance `s-b`, and tests the analogous three T2 steps.

`(ox,oy)` is **not recomputed** for the reduced second-pass clearance. With stock
`AIBaseSpacing=1`, the second clearance is `0` normally and `1` for either extra-space
flag. The third failed attempt computes one further unused step.

Retail `rulesmd.ini` has `AIBaseSpacing=1` and many active `ProtectWithWall=yes` types.
Its `WantsExtraSpace=yes` examples are commented out, so that flag is inactive in stock
data but active for custom INI. Current Rust already parses both booleans and signed
`AIBaseSpacing`; those fields should be preserved.

## 6. Per-attempt rejection gates, exact order

Every attempted packed cell is tested in this order. The first failure advances to the
next T2 site or local pass.

1. **Expanded-rectangle occupancy.** Construct
   `(x-clearance, y-clearance, W+2*clearance, H+2*clearance)` with native wrapping
   signed arithmetic and call `CellRect__CheckOccupancy @ 0x00586780` with
   `House+0x30`. It must return true.
2. **Building-type placeability.** Call type virtual slot `+0xA8` with the exact
   unexpanded anchor and House. The Building override
   `BuildingTypeClass__CanPlaceAt @ 0x00464AC0` accepts immediately when `PlaceAnywhere`
   is set; otherwise it delegates to `TechnoTypeClass__CanPlaceAt @ 0x00716150`, which
   walks the foundation/occupy list and calls
   `Cell_passability_building_placement @ 0x0047C620` per Building cell. It must return
   true.
3. **Anchor height.** Resolve the attempted anchor's CellClass, read signed byte `+0x11B`,
   and require `abs_i32(base_reference_height - candidate_height) < 3`. Assembly compares
   to `3` and rejects on signed `JGE`; a difference of exactly three fails.
4. **Reservation-network proximity.** Call
   `HouseClass__HasBaseReservationNearBuilding @ 0x0050B760`; it must return true.

`HasBaseReservationNearBuilding` returns true immediately only when `g_GameMode==0`.
Otherwise, for signed spacing `b`, it scans X outer and Y inner from
`(x-b-1,y-b-1)` through inclusive `(x+W+2b,y+H+2b)` and returns true on the first
`CellClass+0xDC` hit for the House bit. These intentionally asymmetric literal limits are
also what current Rust's `has_reservation_inclusive` call expresses. The selector therefore
requires connection to the House reservation network in ordinary active games.

The selector does not clamp anchors to map dimensions and does not replace these four
gates with one generic preview call. Shared-dummy lookups remain part of off-map behavior.

## 7. Retry, success, failure, and RNG

- Success stores and returns the exact current attempted packed cell immediately at
  `0x00506B0E`.
- After all sorted candidates are exhausted, the selector repeats the **entire sorted
  traversal once**, unchanged. Therefore the maximum is `vector_count * 2 * 2 * 3`
  validation attempts, excluding vector entries with no reserved neighbor.
- No bridge flag changes between the two local passes or the two full traversals. Calling
  either a "bridge retry" is unsupported.
- After both whole traversals fail, assembly `0x00506AAD..0x00506AC3` stores packed cell
  `0`, i.e. `(0,0)`, not `_g_InvalidCell`.
- `AI_ChooseNextProduction` explicitly treats `(0,0)` as failure at
  `0x00507A34..0x00507A47`.
- An empty/nonpositive vector still performs the base-height lookup and both empty
  traversals, then returns `(0,0)`.
- The non-naval main path contains no `Random__` call, scenario-RNG access, frame-counter
  read, or random direction offset. For fixed House/map/type inputs and native qsort it is
  deterministic. RNG used elsewhere in upstream AI policy does not belong to this selector.

## 8. Current Rust mapping at `eeb2515e...`

### Reusable state that already matches

- `src/sim/house_state.rs:148-215` owns `BaseReservationState`, including ordered
  `perimeter_cells: Vec<u32>`, append-if-absent, stable first removal, bounds, and an
  accessor. This is the exact selector-vector input.
- `src/sim/world/lifecycle.rs:946-1009` updates that vector from the authoritative
  reservation grid after mark/clear; `src/sim/world/substrate.rs` owns per-House masks and
  shared-dummy state. The selector must consume, not rebuild or reorder, this state.
- `src/rules/object_type.rs` parses `naval`, `protect_with_wall`, and
  `wants_extra_space`; `src/rules/ruleset.rs` preserves signed `ai_base_spacing`.
- `HouseState::base_center` is the launch/primary `House+0x5490` equivalent. Preserving it
  is correct, but it is not the missing `House+0x5750` base-plan center.
- `src/map/bridge_facts.rs` already retains/stamps the two FNPC projection flags. The
  missing `+4` rule is local to `src/sim/find_nearby_cell.rs::is_direct_candidate`.

### Wrong or missing ownership

- `src/sim/ai.rs::find_placement_cell` ignores the ordered perimeter vector, clamps to
  `0..511`, walks radius-`0..12` square rings around an average of live structure anchors,
  and uses one generic preview. It lacks the exact sort, vector-sum orientation, site
  derivation, height gate, gate order, duplicate traversal, packed-zero failure, and Naval
  branch.
- `ai_base_reservation_candidate_ok` combines partial occupancy/proximity concepts before
  generic preview. Native order is occupancy, type placeability, signed height, proximity.
- `production_placement.rs` leaves its `_height_map` unused, so generic preview cannot
  satisfy `abs(base-plan-center height - anchor height) < 3`.
- Rust has no alternate `+0x5494` cell, distinct `+0x5750` base-plan center, ordered
  BasePlan nodes/cached sites, authentic defense-influence grids/category selector, parsed
  source-ordered `Shipyard=`, or `AINavalYardAdjacency` owner.
- No production code consumes `BaseReservationState::perimeter_cells()` outside tests,
  snapshot, and hashing.

The earlier proposal to route an isolated selector directly through
`place_ready_buildings` is **not parity preserving**. Native ordinary production calls the
standard scorer during planning, stores the cell in the caller-selected House BasePlan node,
then Building exit looks up/reuses or reselects that node's cell with the Chebyshev scorer.
Current Rust chooses only when a building is ready and has no equivalent plan node. The pure
selector core can be implemented and tested in isolation, but it must not be presented as a
player-path closure until the caller-owned lifecycle is connected.

## 9. Corrected implementation handoff and mechanism boundary

### Handoff A — independently implement the pure selector core

Create deterministic, caller-parameterized selector services over the existing ordered
perimeter vector. This coherent slice includes:

- exact callback-key construction with wrapping `i32` arithmetic;
- a local port of linked `0x007C8B48/0x007C8C9C/0x007C8CEA` signed-key sort;
- all-hit signed-short reservation-direction summation, zero/symmetric-cancellation skip,
  exact x87-derived direction quantization, initial offsets, T1/T2 tables, two clearance
  phases, three tangential attempts, four short-circuit gates, two complete traversals,
  immediate success, and packed `(0,0)` exhaustion;
- the non-naval zero-base-plan-center fallback, given explicit primary, alternate, and
  base-plan-center inputs.

Acceptance must include the three equal-key permutations and 1,001-record collision above;
negative/wrapping keys; vector insertion-order indices; no-hit, one-hit, multiple-hit sum,
N+S cancellation, and all-eight cancellation; all eight seeds; 1x1/non-square foundations;
stock, extra, negative, and short-wrapping spacing; four-gate trace order; height deltas
`-3,-2,+2,+3`; off-map dummy behavior; exact attempt count; first success; zero failure;
and proof of zero selector RNG.

This helper must accept a supplied exact score policy/grid and distinct base-plan center.
It must not read `HouseState::base_center` as `+0x5750`, fabricate a neutral influence
grid, mutate the writer vector, or be wired into ready-time placement as a closure claim.

### Handoff B — close the narrow shared FNPC prerequisite, then the Naval branch

First change only the shared FNPC direct-classification rule: when the candidate has
`BRIDGE_FLAG_FORWARD_SIDE`, add four signed height levels for every projected probe with
`BRIDGE_FLAG_STRUCTURAL`. Prove the correction affects both collection-side early-stop and
final direct/fallback partition; recheck existing FNPC callers. No new bridge writer,
bridge-destruction, cliff, or terrain-state mechanism is required.

Then implement the Naval selector branch with explicit primary/alternate cells, the exact
15-argument query, frame-modulo pool selection, first owned BuildConst distance cap, source-
ordered `Shipyard=` selection, and signed `AINavalYardAdjacency`. Its buildability helper
must use only the exact Owner/Required/Forbidden/AIBasePlanningSide and primary-superweapon
tail, not generic `BuildOption.enabled`. Tests must prove vector bypass; the three retail
yard side outcomes and `6x6` footprint; alternate-to-primary fallback; absent BuildConst cap
bypass; cap/cap+1; both bridge-projection semantic sites; and zero Scenario RNG.

### Handoff C — keep caller-owned prerequisites as separate named mechanisms

The following are not selector-core implementation details and remain independently open:

1. **House base-cell authority lifecycle:** preserve primary `+0x5490`; add snapshot/hash-
   covered alternate `+0x5494` with exact action 137 writer, action 138 clearer, alphabetic
   waypoint decoder, and packed-zero initialization; add distinct base-plan center `+0x5750`
   with its verified Recenter/deploy/ComputerTakeover/action-30 writers.
2. **Ordered BasePlan node/site lifecycle:** scenario/generated population, production-time
   selected-node site capture, cached-site lookup/reuse/reselection, successful-Unlimbo fill
   and retry reset, failure clearing/retry/eviction, and wall/power node mutations.
3. **Defense influence/category chooser:** lifecycle-owned grids, exact ratios/category/type
   choice and RNG, so the standard scorer receives an authentic grid and direction.
4. **Production scheduler/Strategy:** exact call cadence/mode and RNG ordering that decides
   when planning occurs; it must remain separate from ready-building placement.

Building-exit Chebyshev scoring can be unit-tested with an explicit base-plan center, and
the standard scorer can be unit-tested with a supplied exact grid. Neither is end-to-end
active until its separate caller-side owner above is present. Any missing owner keeps the
ordinary player-path row open rather than authorizing a guessed input or ready-time bypass.

## 10. Negative facts / do not do

- Do not retain, extend, or widen the radius-12 spiral for ordinary AI building placement.
- Do not call the writer vector an `ExitList`, sort by Euclidean distance for the normal
  ExitObject callback, or assume the static and two local direction tables are identical.
- Do not stop at the first reserved neighbor or let the last hit win; add every matching
  direction and skip a final zero sum, including symmetric cancellation.
- Do not use stable sort, add a coordinate tie-break, or assume `+index` prevents equal keys.
- Do not substitute the host's unstable sort; port the linked retail algorithm and fixtures.
- Do not read decompiler `param_3[0x55e]` as byte offset `+0x55E`; it is
  `BuildingType+0x1578 WantsExtraSpace`.
- Do not label either repeated pass a bridge retry; no bridge state changes here.
- Do not return invalid sentinel on main-vector exhaustion; the native value is packed zero.
- Do not consume scenario RNG or add random direction wobble.
- Do not fold the wall/base-bounds scanner into this selector; that is a separate active owner.
- Do not use `MaxBaseDistance` for the Naval cap; the field is `AINavalYardAdjacency`.
- Do not treat `HouseState::base_center` as the base-plan center; it owns primary `+0x5490`.
- Do not connect the isolated selector to ready-time placement and call the native path
  closed; BasePlan owns planned/cached sites and the influence grid exists only at planning.
- Do not supply a neutral/guessed influence grid, use generic build eligibility for the
  Shipyard scan, or bypass the FNPC forward-side/structural-probe `+4` projection.

## 11. Exhaustiveness ledger

| Obligation | Status | Evidence |
|---|---|---|
| Function boundary/ABI | resolved | decompile plus prologue/`RET 0x10` assembly |
| Complete direct caller census | resolved | three active xrefs in two functions |
| Ordinary YR reachability | resolved | `HouseClass__Update`/AI queue caller chain plus non-human ExitObject branch |
| Null, naval, zero-base fallbacks | resolved | entry/branch assembly `0x5060C6..0x5062C1` |
| Primary/alternate/base-plan-center distinction | resolved | complete direct instruction inventories plus constructor/setter/clearer/Recenter/deploy/caller evidence |
| Retail alternate-cell activation | resolved | tracked exhaustive 461-candidate/386-map physical census; separate resolved-name scan corroborates three action-137 and zero action-138 entries |
| Vector pointer/count/order | resolved | decompile plus indexed-load assembly |
| Both supplied score callbacks | resolved | full raw assembly `0x505F80..0x5060A3` plus caller arguments |
| CloakGenerator override | resolved | target decompile/assembly plus retail INI negative search |
| Sort key signedness and complete tie behavior | resolved | comparator plus full qsort/selection-sort/byte-swap assembly and four permutation fixtures |
| Adjacent `+0xDC` probe order/vector sum | resolved | target and `MapCoord_Add` assembly; zero initialization, all-hit add, final zero skip |
| Direction quantization and tables | resolved | target decompile plus literal table initialization stores |
| Coordinate derivation/all attempts | resolved | target decompile plus `0x506673..0x506B0E` assembly |
| Rejection gates and order | resolved | target assembly plus decompile of `0x716150` and `0x50B760` |
| Success/failure/retry | resolved | target tail assembly and AIChoose consumer check |
| RNG | resolved | complete target/callee census; no selector RNG reference |
| Naval FNPC bridge projection | resolved/native, open/Rust prerequisite | `0x6D6410` decompile+assembly, five FNPC callsites, Rust bridge flag writers and omitted classifier rule |
| Retail INI activation/exclusion | resolved | `rulesmd.ini` and `rules.ini` literal searches |
| Current Rust owner/delta | resolved | direct inspection of exact `origin/main` revision |
| Save/load/replay | partially reusable, caller state open | perimeter vector and primary cell are covered; alternate cell, base-plan center, BasePlan nodes, and caller grids/lifecycle are absent |
| Pause/frame behavior | resolved | non-naval selector has no frame input; naval FNPC uses the authoritative frame-counter argument |
| Selector/caller mechanism boundary | resolved | selector callers plus BasePlan plan-time write and Building-exit lookup/reselection evidence |

No material native selector-core question remains. The named caller-side mechanisms remain
implementation prerequisites, not uncertainties that may be approximated inside this slice.

## 12. Adversarial zero-add pass

Seven adversarial questions were asked after the first complete model:

1. **Can a tie actually occur despite the index term?** Yes: score bands can differ by one
   while indices differ by 1000; the full linked sort and exact permutations are required.
2. **Does the reservation-neighbor loop choose a first or last matching side?** Neither:
   it sums every matching direction with `MapCoord_Add`; opposite/symmetric hits can cancel
   to zero and skip the candidate.
3. **Does the extra-space retry recompute its footprint offset?** No: it preserves `(ox,oy)`
   from `s` and only changes the clearance to `s-b` plus the opposite static delta.
4. **Are the two retries bridge-dependent or randomized?** No: neither bridge state nor RNG
   changes; the sorted traversal is repeated literally.
5. **Can current primary `base_center` or generic preview supply selector inputs?** No:
   `base_center` is `+0x5490`, while scoring/height use distinct `+0x5750`; preview's
   `height_map` parameter is unused.
6. **Can the active selector be attached only at completed-building ready time?** No:
   the standard scorer runs during production planning and writes a BasePlan-node site;
   Building exit later performs cached-site lookup/reuse/reselection.
7. **Is shared FNPC exact enough for Naval today?** No: its current classifier omits the
   candidate-`0x1000`/probe-`0x100` `+4` projection at both semantic uses.

Zero-add result: no further native selector branch or input remains unbounded. The pass did
correct three earlier false closures: last-hit became vector sum, `base_center` became three
distinct authorities, and Naval gained its narrow shared-FNPC prerequisite. It also moved
BasePlan/influence/scheduler work out of the core instead of hiding those dependencies.

Cold spot-checks:

- A fresh decompile of `0x005060B0` and raw re-read of `0x005069DB..0x00506B30`
  reconfirmed four-gate order, strict height `<3`, three T2 attempts, success, and two whole
  traversals.
- A fresh raw re-read of `0x00505F80..0x005060A3`, comparator `0x005108F0`, and selection
  sort `0x007C8C9C`, plus full `0x007C8B48` and byte-swap `0x007C8CEA`, reconfirmed exact
  scoring conversion, signed comparison, and complete unstable equal-key behavior.
- Fresh direct-instruction inventories for `0x5490`, `0x5494`, and `0x5750`, plus cold
  reads of `RecenterBase`, ConstructionYard deploy, the action-137/138 setter/clearer, and
  `FUN_006D6410`, reconfirmed the three cell authorities and FNPC bridge projection.

## 13. Required stale-doc replacement wording

In `FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md`, replace Variant F wording
that says `ExitList`, Euclidean/square-root normal scoring, identical direction tables,
first probe/bridge retry, impossible ties, or `MaxBaseDistance` with:

> `FUN_005060B0` consumes the House/Base writer-maintained ordered reservation-perimeter
> vector. The ExitObject scorer uses `index + 1000*ChebyshevDistance`; AI preplanning uses
> `index + 1000*(128*threat + anglePenalty)`. Signed i32 keys are sorted by the shipped
> unstable qsort, whose exact linked algorithm and equality permutations must be ported.
> Each vector coordinate probes static N,NE,E,SE,S,SW,W,NW, sums every same-House
> `Cell+0xDC` direction with wrapping word arithmetic, and skips a zero result including
> symmetric cancellation. Two rotated local tables derive three sites in each of two local
> passes, and the entire sorted traversal is repeated twice. Neither repeat is a bridge
> retry. Scoring/orientation/height use the distinct `House+0x5750` BasePlan center; zero
> fallback and Naval origin prefer alternate `+0x5494` then primary `+0x5490`. The Naval
> branch bypasses the vector, requires FNPC's candidate-forward/probe-structural `+4`
> projection, and caps output with `AINavalYardAdjacency`.

In `WALL_PLACEMENT_AND_PROTECTWITHWALL_GHIDRA_REPORT.md`, replace `BuildingType+0x55E`
with:

> The raw address is `BuildingTypeClass+0x1578` (`WantsExtraSpace`). The decompiler rendered
> it as `param_3[0x55e]` because `param_3` was typed `int*`; `0x55e * 4 == 0x1578`.

## 14. Ghidra annotation candidates (not applied)

| Address | Candidate | Confidence/status |
|---|---|---|
| `0x005060B0` | rename `FUN_005060B0` to `HouseClass__SelectBasePlacementCell` and type the ABI above | high; receiver and both active roles proven; worker-report-only |
| `0x00505F80` | create/function-label `HouseClass__ScoreBasePerimeterChebyshev` | high boundary/role; function creation requires separate authorized metadata pass |
| `0x00505FD0` | rename `FUN_00505FD0` to `HouseClass__ScoreBasePerimeterThreatAngle` | high; formula and caller proven |
| `0x005108F0` | create/function-label `BasePlacementScoreComparator` | high boundary/role; callback body ends at `0x00510911` |
| `0x00A8EF78` / `0x00A8EFA8` | label `g_BasePlacementStartOffsets` / `g_BasePlacementStepOffsets` | high literal initialization; no mutation authorized |

No Ghidra metadata was changed.

## Sources inspected

Primary live binary evidence:

- `FUN_005060B0 @ 0x005060B0`
- callback body `0x00505F80..0x00505FBF`
- `FUN_00505FD0 @ 0x00505FD0`
- comparator `0x005108F0..0x00510911`
- qsort `FUN_007C8B48`, small-partition sort `FUN_007C8C9C`, byte swap `FUN_007C8CEA`
- `BuildingClass__ExitObject_Main @ 0x00443C60`
- `HouseClass__AI_ChooseNextProduction @ 0x00506EF0`
- `HouseClass__AI_Choose_Building @ 0x004FE3E0`
- `HouseClass__FirstBuildableFromArray @ 0x005051E0`
- `HouseClass__RecenterBase @ 0x0050C210`
- `HouseClass__Set_Starting_Cell @ 0x0050E000`
- alternate-cell setter/clearer `FUN_0050DFE0` / `FUN_0050DFF0`
- trigger action bodies `FUN_006E44E0` / `FUN_006E4540`
- `HouseClass__HasBaseReservationNearBuilding @ 0x0050B760`
- `BuildingTypeClass__CanPlaceAt @ 0x00464AC0`
- `TechnoTypeClass__CanPlaceAt @ 0x00716150`
- `MapCoord_Add @ 0x0042D510`
- `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`
- FNPC projection helper `FUN_006D6410 @ 0x006D6410`

Repository/data evidence:

- `docs/research/PHASE3_CELLCLASS_0XDC_ACTIVE_WRITER_CLEARER_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_0XDC_RESERVATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- `docs/research/FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md`
- `docs/research/WALL_PLACEMENT_AND_PROTECTWITHWALL_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/rules.ini`
- exact `origin/main` sources at `eeb2515e361669e08c7570baeb57a90fe56cb68b`
