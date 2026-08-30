# Phase 3 HouseClass ordinary base-placement research

**Date:** 2026-08-25

**Binary:** active retail Yuri's Revenge `gamemd.exe` in the live `testProsjekt` Ghidra project

**Primary routine:** `FUN_005060B0 @ 0x005060B0`

**Phase/GSI ownership hypothesis:** Phase 3, GSI-04.05 (`CellClass+0xDC` base reservations), reader/consumer Delta D

**Mode:** research only; no Rust implementation and no Ghidra metadata changes
**Verdict:** **ACTIVE, VERIFIED MISMATCH.** Retail ordinary AI building placement is not a base-center spiral. It is a production-planning-time consumer of the House's ordered base-reservation perimeter, a temporary defense-influence grid, exact orientation/probe rules, placement/height/connectivity predicates, and two literal repeated traversals. The current Rust ready-time radius-12 square spiral is not a parity-preserving approximation.

## 1. Scope and boundary

This report establishes the exact active mechanisms owned or consumed by `0x005060B0` sufficiently to decide implementation ownership. It covers:

- both direct caller families;
- the ordinary non-naval base-perimeter selector;
- its ordinary AI influence-grid callback and Building-exit callback;
- the smallest caller-side influence-grid prerequisite needed to supply authentic inputs;
- the independent eight-frame production-chooser scheduler, its `House+0x1E4` mode lifecycle, and its separation from the slower strategy/Manage timer;
- the strategy-owned AI-hate timer, ordered peer-score lifecycle, designated-enemy acquisition, defeated-enemy cleanup, and the 100-frame score decay that can change later acquisition results;
- the strategy timer's earlier `AI_TryFireSW` dispatch, target selection, synchronous launch/spy-reveal effects, post-dispatch `House+0x250` emergency state/actions, no-factory priority, and shared-Scenario RNG ordering required to preserve later placement draws;
- every active-retail BasePlan population path that can supply `AI_Choose_Building`, including scenario nodes, ordinary AI ConstructionYard deployment, recentering, build-queue insertion, wall-plan expansion, and projected-power insertion;
- the active naval branch and its retail INI bindings;
- the `CellClass+0xDC` connectivity reader at `0x0050B760`;
- active retail-data gates and evidence-backed exclusions;
- the current Rust divergence and the minimum correct implementation boundary.

It does **not** close `HouseClass__AI_ScanBasePerimeter @ 0x005082C0`, execution of wall placement, upgrade placement, Railgun, LaserDraw, Sonic Wave, destroyable cliffs, or any Tiberian Sun legacy branch. Section 7.6.5 closes only the active wall-node **population** writer inside `AI_Choose_Building`; the later `-3` perimeter scan remains separate. The other named mechanisms are separate or explicitly excluded user scope.

## 2. Function contract and direct callers

Assembly establishes the effective contract as:

```text
CellStruct* __thiscall HouseClass::FindBaseBuildingSite(
    HouseClass* this,
    CellStruct* out,
    TechnoTypeClass* type,
    SiteSortCallback callback,
    int callback_parameter);                    // RET 0x10
```

`type == null` writes the native packed-zero empty/invalid `CellStruct` sentinel and returns immediately. The sentinel is not a distinct nonzero bit pattern: static initializer `0x004F50A0..0x004F50AE` zeros both halves of `0x00A8EF98`, and every `(0,0)`/“invalid cell” distinction below is semantic context over the same packed dword.

| Direct caller | Callsite(s) | Active path | Type/callback/parameter | Result use |
|---|---:|---|---|---|
| `HouseClass__AI_ChooseNextProduction @ 0x00506EF0` | `0x00507A20` | Ordinary AI building-production planning when no alternate candidate vector is supplied | selected/planned `BuildingTypeClass*`, `FUN_00505FD0`, chosen quadrant `* 2` (`0,2,4,6`) | writes the planned cell into the House's 16-byte BasePlan node at the caller-supplied plan index; it does not attach the cell to a production queue item |
| `BuildingClass__ExitObject_Main @ 0x00443C60` | `0x00444FE1`, `0x004450BD` | AI-controlled Building exit path, case 6; player-controlled owners do not take it | exiting object's type, inline callback `LAB_00505F80`, parameter `-1` | AI exit/place attempt |

`HouseClass__AI_ChooseNextProduction` itself has one caller: `HouseClass__AI_Choose_Building @ 0x004FE3E0` at `0x004FE633`.

Important caller exclusions and alternatives:

- If `House+0x5748 > 0`, `AI_Choose_Building` supplies the alternate vector rooted at `House+0x5738`; `AI_ChooseNextProduction` chooses and removes a cell from that vector and does not call `0x005060B0`.
- A building-production entry equal to `-3` is removed and dispatched to `HouseClass__AI_ScanBasePerimeter @ 0x005082C0`. That consumer is not `0x005060B0` and remains a separate research/implementation slice.
- Building-exit types with non-null `PowersUpBuilding` (`type+0xE88`) never call `0x005060B0`: a nonzero cached BasePlan site is reused directly, while a missing/zero site calls the separate `0x00506B90` upgrade selector.

## 3. Native House and type inputs

| Native input | Proven meaning in this path | Rust status before this report |
|---|---|---|
| `House+0x30` | House array index used to form `1 << (index & 31)` | available through the base-reservation house-index helper |
| `House+0x5490` | primary/starting base cell | represented by `HouseState.base_center` work from the preceding GSI-04.05 slice |
| `House+0x5494` | trigger-set alternate base cell, used unless equal to the packed-zero empty/invalid sentinel | absent; no alternate field and no trigger action 137 consumer |
| `House+0x5724/+0x5730` | ordered packed-cell base-reservation perimeter vector and count | represented by `BaseReservationState.perimeter_cells`; writer lifecycle now preserves native insertion/removal order |
| `House+0x5704..+0x5714` | polymorphic BasePlan vector; `+0x5708` points to ordered 16-byte nodes and `+0x5714` is the count | absent; Rust has a completed-building type-ID queue but no native-equivalent plan nodes or cached-site lifecycle |
| `House+0x5750/+0x5752` | Base-plan center selected from a live owned BuildConst object and used by sorting, adjacency direction, and height reference | absent; `HouseState.base_center` instead stores launch/primary `+0x5490` authority |
| `House+0x5754/+0x5758/+0x575C/+0x5760` | influence-grid bounds: minimum X, minimum Y, width, height | the four base-reservation bounds exist, but there is no influence-grid consumer |
| `House+0x16060` | transient pointer to the selected defense-influence grid | absent |
| `House+0x1609C/+0x160A0/+0x160A4` | dynamically refreshed enemy armor, air, and infantry value ratios | absent |
| `House+0x160A8/+0x160AC/+0x160B0` | tracked infantry, armor, and air values maintained by House Added/Removed-To-Game accounting | absent as native-equivalent House counters |
| `House+0x5600` | designated enemy House index, or `-1` | `HouseState.enemy_house` exists and is snapshot/hash covered, but building production does not consume it |
| `House+0x249` | post-abandonment target-bias latch; when set, candidate scoring forces non-designated-enemy targets to score `1` | absent |
| `House+0x250` | signed Strategy emergency state, constructor zero; active states `0`, `1`, `3`, and externally reachable `4` govern low-wallet and abandon-base behavior | absent |
| `House+0x2FC/+0x30C` through secondary-interface vtable `+0x18` | Storage total and credits used by the exact available-wallet query `0x004F6990`, including `HouseType+0x148` multiplication and native `ftol` | authoritative credits/storage pieces exist in Rust, but no proven combined query or Strategy consumer |
| `House+0x5608/+0x5614` | constructor-ordered peer-House anger nodes, each `{HouseClass*, signed score}` | `HouseState.grudge_scores` is a snapshot/hash-covered sparse interned-ID representation; the damage helper correctly uses global `house_order` for zero-default peers, but Strategy acquisition, cleanup, and periodic decay do not consume it |
| `House+0x5634/+0x563C` | signed strategy timer start/duration | absent |
| `House+0x5640/+0x5648` | signed AI-hate timer start/duration | absent |
| `House+0x54D8` | signed last-Building attack frame; also drives the Strategy state's exact `+900` suppression boundary | absent |
| `House+0x184` | native AI difficulty index, hardest-first `0..2` | `HouseState.difficulty` already preserves the exact index convention and is snapshot/hash covered |
| `BuildingType+0x1524/+0x1528/+0x152C` | `AntiAirValue`, `AntiArmorValue`, `AntiInfantryValue` | not parsed into `ObjectType` |
| `BuildingType+0x1765` | `ProtectWithWall` | parsed |
| `BuildingType+0x1578` | `WantsExtraSpace` | parsed |
| `BuildingType+0x1703` | `PlaceAnywhere` | placement support must be checked/retained in the shared placement predicate |
| `Rules+0x1460` | signed `[General] AIBaseSpacing=` | parsed as signed `RuleSet.ai_base_spacing` |
| `Rules+0xE48` | signed `[General] MaximumBuildingPlacementFailures=`; constructor default `5`, retail override `3` | absent; Rust has no BasePlan retry counter or eviction rule |
| `Rules+0xDD4` / data `+0xDD8` | signed `[General] AIPickWallDefensePercent=` difficulty vector | absent; retail supplies Hard/Normal/Easy `50,25,10` |
| `Rules+0x1174` / data `+0x1178` | signed `[General] AIHateDelays=` difficulty vector | absent; native constructor leaves the vector empty and active retail supplies Hard/Normal/Easy `30,50,70` |
| `Rules+0xEC4/+0xEC8`, `+0xEE0`, `+0xEE4` | `[General] AISuperDefenseProbability=`, `AISuperDefenseFrames=`, `AISuperDefenseDistance=` | absent; probability's native vector is empty, scalar defaults are signed `25` frames and `10` cells, and active retail supplies `90,50,10`, `50`, and `12` |
| `Rules+0xA50` / data `+0xA54` | source-ordered `[AI] ConcreteWalls=` type vector | source strings are parsed as `RuleSet.concrete_walls`, but no BasePlan wall-expansion consumer resolves or uses them |
| `Rules+0x87C`, `+0x89C`, `+0x8A0`, `+0x8A4`, `+0x8A8` | `WallTower`, `GDIPowerPlant`, `NodRegularPower`, `NodAdvancedPower`, `ThirdPowerPlant` type pointers | absent as exact planner inputs |
| `House+0x53A4/+0x53A8`, `+0x160B4`, `+0x2A4/+0x2AC`, `+0x577B` | cached power output/drain, signed AI cost tolerance, power-blackout timer state, and the positive-output-deploying-building latch used by the power splice | Rust has authoritative power totals and blackout duration state, but no exact BasePlan splice, cost-tolerance field, or native deploying-building latch |

The `House+0x50/+0x54/+0x60` DynamicVector is the owned construction-yard list, not an anonymous object vector. `BuildingClass::Unlimbo` inserts a building when its type matches `Rules+0x8B0/+0x8BC`, the `[AI] BuildConst=` list. `RulesClass__ReadAI @ 0x00672AE0` binds it at `0x00672B14..0x00672C01` (key push `0x00672B23`, BuildingType resolver call `0x00672B6A`); there is no `[General]` fallback. The naval branch below reads the first pointer from this list.

### 3.1 Primary and trigger-set alternate base cells

`HouseClass__Constructor @ 0x004F56D0` initializes both `House+0x5490` and `House+0x5494` from `0x00A8EF98`, whose active-retail packed dword is `(0,0)`. A complete direct-store inventory for displacement `0x5494` found one later setter, `FUN_0050DFE0`, whose entire body is `House+0x5494 = packed_cell`. Its only direct caller is `FUN_006E44E0`, reached from `TriggerAction__Execute @ 0x006DD8B0` case `0x89` (decimal action 137).

Action 137 returns false when its House argument is null. Otherwise it reads `TActionClass+0x44`, calls `ScenarioClass__GetWaypointCoords @ 0x0068BCC0`, returns false if that coordinate is invalid, and writes the valid packed cell through `0x0050DFE0`. It does not modify the primary cell or base-plan center.

`TActionClass::Read @ 0x006DD5B0` stores the eighth action token at `+0x44` through `FUN_00763690`. That converter is literal alphabetic waypoint notation: the first letter maps `A..Z -> 0..25`; when a second letter is alphabetic the result is `26 * first_index + second_index + 26`; non-alphabetic first input returns `-1`. Therefore `P -> 15`, `NZ -> 389`, and `AA -> 26`.

A read-only physical-entry census of the six shipped map archives `MAPS01.MIX`, `MAPS02.MIX`, `mapsmd03.mix`, `MULTI.MIX`, `multimd.mix`, and `expandmd01.mix`, plus both entries of every top-level `.mmx`/`.yro` package and both loose `.map` files, examined 461 candidate payloads, recognized 386 `[Map]` payloads, rejected no malformed `[Actions]` record, and decoded 26,422 action entries. Exactly three action-137 entries exist, all in shipped YR campaign archive `mapsmd03.mix`:

| Map payload | Action record / ordinal | Waypoint code / index | `[Waypoints]` value | Cell written |
|---|---:|---:|---:|---:|
| `all01umd.map` | `08A77A5C` / 3 | `P` / 15 | `106093` | `(93,106)` |
| `all03umd.map` | `0918703C` / 4 | `NZ` / 389 | `135122` | `(122,135)` |
| `all07smd.map` | `0964B13C` / 1 | `AA` / 26 | `194105` | `(105,194)` |

No action-137 entry occurred in `MULTI.MIX`, `multimd.mix`, or the loose map candidates. The alternate cell is therefore a stock-active campaign input and a stock-skirmish packed-zero/unset input. Rust already retains each raw eight-token action chunk, but `TriggerRuntime` has no action-137 dispatch and its current numeric waypoint parsing does not decode alphabetic waypoint tokens. The minimum prerequisite is an exact `0x00763690` decoder, a snapshot/hash-covered alternate House cell initialized to packed `(0,0)`, and action 137 wired to set it only for a valid House and valid resolved waypoint.

`HouseClass__RecenterBase @ 0x0050C210` is a verified writer of `House+0x5750`, but its only direct caller is `TriggerAction__Execute` case 30; it is not an ordinary structure-lifecycle callback. It scans `House+0x6C/+0x78` in reverse owned-object order and selects the first object whose type is in BuildConst and whose object byte `+0x81` is clear. It converts that object's lepton X/Y to signed-truncating cells, writes the cell to primary base cell `+0x5490`, refreshes AI build options only when the base-plan node count `+0x5714` is zero, writes the cell into the first node's cell slot `(*(House+0x5708)+4)`, and finally writes `+0x5750`. If no eligible BuildConst object exists, it leaves those cells unchanged. Retail activation of action 30 remains UNCHECKED. The successful non-player-control ConstructionYard deploy branch at `0x007398DE..0x007398F3` is the verified ordinary-skirmish writer: it writes its new yard cell to the first plan node and `+0x5750`.

Current Rust keeps `HouseState.base_center` as the launch/primary `+0x5490` cell, but both the successful AI ConstructionYard-deploy branch and Recenter write that native primary cell before writing the distinct `+0x5750` BasePlan center. Implementation therefore needs separate BasePlan-center state and must not suppress the primary-cell write. Recenter/action 30 remains a separately activated path rather than a generic lifecycle hook.

## 4. Ordinary non-naval selector (`type+0xCCE == 0`)

### 4.1 Early return and ordered sort input

If `House+0x5750/+0x5752 == (0,0)`, the function returns `House+0x5494` unless that cell equals the same packed-zero empty/invalid sentinel, in which case it returns `House+0x5490`. It does not traverse the perimeter.

Otherwise it visits the `House+0x5724` perimeter vector in stored order. For every entry it constructs an eight-byte `{ signed_i32_key, packed_cell }` record. It does not deduplicate here. The records are sorted ascending by signed key through native CRT qsort `FUN_007C8B48` with comparator `LAB_005108F0`:

```text
left.key == right.key =>  0
left.key >  right.key => +1
otherwise             => -1
```

The sort is not stable, and its exact tie permutation is now established from the linked retail body rather than left to the host standard library. `FUN_007C8B48` behaves as follows for these eight-byte records:

1. Ranges of eight records or fewer use `FUN_007C8C9C`: repeatedly scan from the low record through the current high record, retain the first record whose key is maximal (`comparator > 0` is the only replacement), swap that record with the current high record through byte-swap helper `FUN_007C8CEA`, then decrement high.
2. Longer ranges swap the record at `low + floor(count/2)` with the low record and use it as the pivot.
3. The forward scan advances while `candidate <= pivot`; the reverse scan retreats while `candidate >= pivot`. When the scans have not crossed, native swaps them and repeats. After they cross, native swaps the pivot with the reverse-scan record.
4. Native continues with the smaller partition and pushes the larger partition on its two fixed 30-entry local stacks; a later pop resumes the deferred range. The exact strict range tests are `low + 1 < reverse`, `forward < high`, and the size comparison `(reverse - low - 1) < (high - forward)`.

Therefore Rust must use a local port of this exact signed-key sorter for this mechanism; `sort`, `sort_unstable`, or a stable tie fallback is not equivalent. Assembly-derived permutation fixtures are:

```text
input record ids, all keys equal     native output ids
[0,1]                                [1,0]
[0,1,2,3,4,5,6,7]                    [1,2,3,4,5,6,7,0]
[0,1,2,3,4,5,6,7,8]                  [4,1,2,3,0,5,6,7,8]
```

An algebraic placement-key collision fixture uses 1,001 records: record `id=0` has `i=0, score=1, key=1000`; records `id=1..1000` have `score=0, key=i`. Native places the two equal-key records at sorted positions `999:id=1000` and `1000:id=0`. This directly guards the index-difference cancellation identified by review. Signed 32-bit key construction still occurs before this sorter, including native wrap behavior.

### 4.2 Standard AI sort callback (`FUN_00505FD0`)

The ordinary AI caller supplies `FUN_00505FD0` and a quadrant-derived direction parameter (`0,2,4,6`). For candidate index `i` and cell `(x,y)`:

```text
grid_index = (y - House.min_y) * House.width + (x - House.min_x)
grid_value = House.selected_influence_grid[grid_index]

if direction_parameter == -1:
    angular_penalty = 0
else:
    raw16 = ftol((atan2(-(y - center_y), x - center_x) - pi/2) * (-32768/pi))
    candidate_dir8 = ((((uint16)raw16 >> 7) + 1) >> 1) & 0xFF
    angular_penalty = abs(direction_parameter * 32 - candidate_dir8)
    if angular_penalty >= 128:
        angular_penalty = 255 - angular_penalty

key = wrapping_i32(i + (grid_value * 128 + angular_penalty) * 1000)
```

Constants are read from `0x007E2820` (`pi/2`) and `0x007E2818` (`-32768/pi`). The arithmetic and x87 `ftol` conversion are load-bearing. The common stale description “distance * 1000 + index” is false for this standard active path.

The alternate-vector path in `AI_ChooseNextProduction` also calls this callback with parameter `-1`, sorts those candidates by grid value plus index, selects the first, and removes it from that vector.

### 4.3 Building-exit sort callback (`LAB_00505F80`)

The AI Building-exit caller uses an inline callback whose parameter is ignored:

```text
key = wrapping_i32(
    candidate_index
    + 1000 * max(abs(cell.x - center_x), abs(cell.y - center_y)))
```

This is Chebyshev distance to `House+0x5750/+0x5752`, not the influence-grid formula.

### 4.4 Stock-inactive `CloakGenerator` sort branch

When `type+0x16C7` is nonzero, `0x005060B0` ignores the supplied callback and computes:

```text
key = candidate_index
    + 1000 * ftol(Sqrt_Approx(distance3D(candidate_center, base_or_starting_cell_coords)))
```

No active `CloakGenerator=` assignment exists in retail `rulesmd.ini` or `rules.ini`. This is an evidence-backed stock-YR exclusion and an explicitly prohibited TS-legacy implementation target.

## 5. Orientation and exact candidate probes

For each sorted perimeter candidate, native performs the following steps in literal order.

### 5.1 Reservation-neighbor vector sum

It scans `g_DirectionOffsets[0..7]` in order `N, NE, E, SE, S, SW, W, NW`. For every adjacent cell whose `CellClass+0xDC` contains the current House bit, it adds that direction offset to a signed-short accumulator using `MapCoord_Add`.

This is a vector sum, not “last matching neighbor wins.” If the final sum is `(0,0)`, the candidate is skipped. Therefore both “no qualifying neighbor” and symmetric cancellation skip the candidate.

For a nonzero sum, native derives the outward seed direction:

```text
raw16 = ftol((atan2(sum_y, -sum_x) - pi/2) * (-32768/pi))
seed = ((((uint16)raw16 >> 12) + 1) >> 1) & 7
```

### 5.2 Clearance and foundation offset

Let `fw/fh` be the type's foundation width/height (height queried with bib flag `0`). Let:

```text
b = Rules.AIBaseSpacing
if ProtectWithWall || WantsExtraSpace:
    b += 1
```

Native then forms a foundation-placement offset:

```text
ox = b          for seed 1..3
ox = 1-fw-b     for seed 5..7
ox = 0          for seed 0 or 4

oy = b+1        for seed 3..5
oy = 1-fh-b     for seed 7,0,1
oy = 0          for seed 2 or 6
```

The two lazily initialized hardcoded offset tables are:

| seed | Table 1 (`0x00A8EF78`) | Table 2 (`0x00A8EFA8`) |
|---:|---:|---:|
| 0 | `(-1,0)` | `(1,0)` |
| 1 | `(-1,-1)` | `(1,1)` |
| 2 | `(0,-1)` | `(0,1)` |
| 3 | `(1,-1)` | `(-1,1)` |
| 4 | `(1,0)` | `(-1,0)` |
| 5 | `(1,1)` | `(-1,-1)` |
| 6 | `(0,1)` | `(0,-1)` |
| 7 | `(-1,1)` | `(1,-1)` |

Table 1 is the standard direction `(seed-2)&7`; Table 2 is `(seed+2)&7` and is Table 1's opposite.

### 5.3 Two clearance phases, three tangential probes each

Per candidate, native runs two phases:

```text
phase 0:
    phase_start = candidate + (ox,oy) + Table1[seed]
    border = b

phase 1:
    phase_start = phase0_start + g_DirectionOffsets[(seed-4)&7]
    border = b - Rules.AIBaseSpacing
```

Each phase tests exactly three cells: the phase start and two successive additions of Table 2. This is a three-cell **tangential sweep across the candidate**, not a three-step radial push. Phase 1 is an inward one-cell shift plus clearance relaxation; it is not a bridge-aware/bridge-blind retry.

### 5.4 Exact predicate order

For each test cell, native stops at the first failed predicate and evaluates:

1. `CellRect__CheckOccupancy` on `(test.x-border, test.y-border, fw+2*border, fh+2*border)`, reservation argument `House+0x30`.
2. type virtual `+0xA8`, `CanPlaceAt(type,test,house)`.
   - `BuildingTypeClass__CanPlaceAt @ 0x00464AC0` returns true immediately for `PlaceAnywhere`; otherwise it delegates to `TechnoTypeClass__CanPlaceAt @ 0x00716150`.
   - The delegated Building path walks the foundation offsets in INI order and applies building placement passability.
3. Signed cell-byte height gate: `abs(signed(height(base_center_cell)) - signed(height(test_cell))) < 3`, where the byte is `CellClass+0x11B`.
4. `HouseClass__HasBaseReservationNearBuilding @ 0x0050B760`.

The first cell passing all four predicates is returned immediately.

### 5.5 Literal duplicate traversal and failure value

After all candidates fail, the complete sorted candidate traversal is repeated exactly once: outer counts `0` and `1`. No flag, bridge parameter, clearance value, or other explicit input differs between the two passes. It must be retained as a literal duplicate because shared-dummy reads, virtual calls, or other native side effects can still distinguish it. Calling it a “bridge retry” is unsupported.

If both traversals fail, ordinary non-naval placement returns packed cell `(0,0)`. This is bit-identical to the native empty/invalid `CellStruct` sentinel at `0x00A8EF98`; the ordinary path merely materializes literal zero instead of loading that global.

## 6. Base-reservation connectivity reader (`0x0050B760`)

`HouseClass__HasBaseReservationNearBuilding` returns true immediately when `g_GameMode @ 0x00A8B238 == 0`. Mode zero is campaign/single-player, not an unknown discriminator: `Main_Game @ 0x0052D9A0` writes zero on the scenario/campaign cases before `ScenarioClass__Start_Scenario`, while its case `0x0B` writes `5` before the ordinary offline-skirmish setup. Current Rust already carries the exact predicate as `ScenarioSession.game_mode_nonzero`, populated false for campaign and true for skirmish by `src/app/loading/init.rs`; it is persisted and included in `ScenarioSession::fold_identity`. The native shortcut therefore maps exactly to `!sim.session.game_mode_nonzero` and must run before any reservation-cell scan.

For all other mode values, with `s = Rules.AIBaseSpacing`, it scans:

```text
x from test.x - s - 1 inclusive to test.x + 1 + fw + 2*s exclusive
y from test.y - s - 1 inclusive to test.y + 1 + fh + 2*s exclusive
```

and returns true on the first cell with the current House bit in `CellClass+0xDC`. `MapClass__Get_CellClass` supplies the shared dummy for out-of-range coordinates, so dummy reservation state is part of the exact behavior.

The current Rust `has_reservation_inclusive` call uses the equivalent inclusive maxima (`test + foundation + 2*s`) and correct same-House bit. That range math can be preserved, but it currently occurs before native's type-placement and height gates because the surrounding selector is wrong.

## 7. Required caller-side defense-influence prerequisite

The standard callback cannot run authentically with a neutral or guessed grid. `HouseClass__AI_ChooseNextProduction @ 0x00506EF0` constructs and selects its grid during production choice.

### 7.1 Grid construction

Native allocates three zeroed signed-i32 grids, each `House.width * House.height` cells. It walks `House+0x6C/+0x78`, the owned-object vector, in stored order. For each object not exactly at the base center:

1. Bucket the object into quadrant `0..3` from `atan2(-(dy),dx)` and native `ftol` direction conversion.
2. Call virtual slots `+0x2D4/+0x2D8/+0x2DC`.
   - `BuildingClass @ 0x00459870` returns `BuildingType+0x1524 AntiAirValue`.
   - `BuildingClass @ 0x00459880` returns `BuildingType+0x1528 AntiArmorValue`.
   - `BuildingClass @ 0x00459890` returns `BuildingType+0x152C AntiInfantryValue`.
   - the base Techno implementation contributes zero, so ordinary non-building Technos do not add to these grids.
3. If the three values sum positive, add each value to its quadrant total and call `FUN_00506D50` once per category grid.

### 7.2 Exact `FUN_00506D50` falloff

The decompiler omits the arithmetic after `Math__ftol`; assembly `0x00506E53..0x00506EAD` establishes it. For category weight `w`:

```text
if w <= 0: w = 1
object_cell = signed-short(trunc_toward_zero(object_leptons / 256))

for y in intersect([bounds.min_y, bounds.min_y+bounds.height), [object_y-6, object_y+6)):
  for x in intersect([bounds.min_x, bounds.min_x+bounds.width), [object_x-6, object_x+6)):
    d = Sqrt_Approx((x-object_x)^2 + (y-object_y)^2)
    if d < 6:
      effective_d = max(d, 1.0)
      contribution = w / (1.0 + (effective_d - 1.0) * 0.1)
      grid[index] = ftol(grid[index] + contribution)
```

The loops and six-cell radius are half-open exactly as shown. Constants are `1.0 @ 0x007E1718`, `0.1 @ 0x007E3860`, and `6.0 @ 0x007EAA98`. The final conversion uses the native x87 `Math__ftol` chopping helper.

### 7.3 Enemy-ratio refresh and its RNG

Immediately after selecting a quadrant, `AI_ChooseNextProduction` calls `HouseClass__AI_UpdateEnemyThreatRatios @ 0x00508150`. These are not fixed configuration ratios.

If `House+0x5600 == -1`, the updater writes exact float bits `0x3EA8F5C3` (`0.33f`) to all three ratio fields and consumes no RNG. Otherwise it reads the designated enemy's lifecycle-maintained tracked values:

```text
infantry = enemy+0x160A8
armor    = enemy+0x160AC
air      = enemy+0x160B0
```

`HouseClass::Added_To_Game @ 0x00502A80` and `HouseClass::Removed_From_Game @ 0x005025F0` are the active writers. All three counters initialize to zero. The added writer evaluates the object's type-value virtual `+0x84(House)` and performs wrapping signed addition; removal mirrors it with subtraction:

| Object `WhatAmI()` | Type gates | Tracked bucket |
|---:|---|---|
| `1` (UnitClass) | `ConsideredAircraft` (`type+0xD96`) is false and `Spawns` (`type+0xD58`) is null | armor `+0x160AC` |
| `1` (UnitClass) | either gate above is active | air `+0x160B0` |
| `2` (AircraftClass) | unconditional | air `+0x160B0` |
| `0xF` (InfantryClass) | `ConsideredAircraft` is false | infantry `+0x160A8` |
| `0xF` (InfantryClass) | `ConsideredAircraft` is true, including the stock Rocketeer path | air `+0x160B0` |
| `6` (BuildingClass) | all | none of these three buckets |

This live writer mapping corrects older House field maps that called `+0x160A8` aircraft and `+0x160AC` infantry. Recomputing simple counts is not equivalent to these gates, and raw `ObjectType.cost` is not equivalent to the type-value virtual.

#### 7.3.1 Exact type-value virtual and category dispatch

The UnitType, AircraftType, and InfantryType vtables resolve their type-value slot `+0x84` directly to `FUN_00711F00`. BuildingType uses its distinct wrapper at `0x0045EDD0`, whose first cost step calls `0x00711F00` at `0x0045EDDB`; buildings do not enter the three tracked enemy buckets. `0x00711F00` obtains raw signed `Cost` through type vtable `+0xAC`; the shared target `FUN_00711EB0` is exactly `return *(i32 *)(Type+0x610)`. A null House argument returns that raw signed value immediately. With a House, `0x00711F00` obtains the country factor from `HouseClass__GetCostBonus @ 0x0050BDF0` and the accumulated FactoryPlant factor from `HouseClass__GetAccumulatedBonus @ 0x0050BEB0`, then executes:

```text
x87_value = FILD(raw_signed_cost)
x87_value = x87_value * accumulated_factory_plant_f32
x87_value = x87_value * country_cost_f32
return Math__ftol(x87_value)
```

The call order is country accessor first and FactoryPlant accessor second, but the multiplication order is FactoryPlant first and country second at `0x00711F35..0x00711F41`. Both operands are stored `f32`; the two products remain in the x87 stack until the native chopping `Math__ftol @ 0x007C5F00`. There is no intermediate `f32` store between these two multiplies.

Both accessors use the type's `WhatAmI` virtual `+0x2C` and the same exact five-way category map:

| Type `WhatAmI()` | Additional gate | FactoryPlant factor | HouseType country factor |
|---:|---|---:|---:|
| `3` AircraftType | none | `House+0x5398` | `HouseType+0x11C` `CostAircraftMult` |
| `0x10` InfantryType | none | `House+0x5390` | `HouseType+0x114` `CostInfantryMult` |
| `0x28` UnitType | none | `House+0x5394` | `HouseType+0x118` `CostUnitsMult` |
| `7` BuildingType | `Type+0xE08 BuildCat == 5` | `House+0x53A0` | `HouseType+0x124` `CostDefensesMult` |
| `7` BuildingType | every other `BuildCat` | `House+0x539C` | `HouseType+0x120` `CostBuildingsMult` |
| any other type | none | exact `1.0f` | exact `1.0f` |

The building split inside the two factor accessors is `BuildCat == 5`, not naval status, SpeedType, or a generic sidebar category. For the three tracked enemy buckets, the active calls use the first three rows; the building rows establish the exact shared factor-accessor mechanism but do not claim that `0x00711F00` is the complete BuildingType wrapper.

#### 7.3.2 INI fields, ordered FactoryPlant recomputation, and counter lifecycle

`HouseTypeClass` constructor `0x005113F0` initializes all five country cost floats to exact `1.0f`. `HouseTypeClass__ReadINI @ 0x00511850` reads, in this order, `CostInfantryMult`, `CostUnitsMult`, `CostAircraftMult`, `CostBuildingsMult`, and `CostDefensesMult` into `+0x114/+0x118/+0x11C/+0x120/+0x124`. Each `CCINIClass__ReadDouble` call receives the current stored float promoted to double as its default, and the returned value is cast back to stored `f32`. Neither retail rules file assigns any of these five keys, so every stock country retains exact `1.0f`; the parser and category dispatch remain active mechanisms for layered/modded data.

`BuildingTypeClass` constructor initializes `FactoryPlant @ +0x16CD` false and its five floats `+0x16D0..+0x16E0` to exact `1.0f`. `BuildingTypeClass__ReadINI @ 0x0045FE50` reads `FactoryPlant` as a bool and then reads `InfantryCostBonus`, `UnitsCostBonus`, `AircraftCostBonus`, `BuildingsCostBonus`, and `DefensesCostBonus` through `CCINIClass__ReadDouble`, again using current value as default and storing `f32`.

Each House owns an ordered FactoryPlant DynamicVector at `House+0x140` (`data +0x144`, signed count `+0x150`) and five accumulated `f32` values at `+0x5390..+0x53A0`. The House constructor `0x004F54A0` initializes all five to exact `1.0f`. `HouseClass__CalculateCostMultipliers @ 0x0050BF60` then:

1. rewrites all five accumulators to exact `1.0f`;
2. scans vector index `0` upward to `count-1` without sorting;
3. for each live vector entry, reads its BuildingType `+0x16D0/+0x16D4/+0x16D8/+0x16DC/+0x16E0` in that order; and
4. performs `FLD entry_factor; FMUL current_accumulator; FSTP accumulator` for each category.

The `FSTP float` after every plant means every ordered multiplication is rounded to stored `f32` before the next vector entry. Replacing this with an unordered set, an `f64` product, or one final cast is not exact.

The active vector lifecycle is likewise ordered:

- successful `BuildingClass__Unlimbo @ 0x00440580`, after `TechnoClass__Unlimbo` has run, appends a `FactoryPlant=yes` building at the vector tail when the DynamicVector append succeeds and calls `0x0050BF60` at `0x0044154E` even if growth/append failed;
- `BuildingClass__ChangeOwner @ 0x00448260` removes the exact building pointer from the old owner's vector with left-compaction and recomputation, then tail-appends it to the new owner's vector and recomputes for the new owner; and
- the House pointer-expiry callback at `0x004FB9B0` searches/removes the expiring pointer from the same ordered vector, left-compacts later entries, and calls `0x0050BF60` at `0x004FBC58`. `TechnoClass__Limbo @ 0x006F6AC0` itself calls `HouseClass__Removed_From_Game` but does not rebuild these multipliers; final pointer expiry owns the vector removal.

Stock `rulesmd.ini` makes this path ordinary-active through `NAINDP`: `FactoryPlant=yes`, with `UnitsCostBonus=0.75` and the other four bonuses `1`. No `FactoryPlant=yes` assignment exists in base `rules.ini`.

There is no retroactive correction of the tracked infantry/armor/air counters when `0x0050BF60` changes a House multiplier. `Added_To_Game` adds the value computed with the multipliers live at addition time; `Removed_From_Game` independently subtracts the value recomputed with the multipliers live at removal time. The per-object historical value is not stored. Consequently acquisition/loss of an Industrial Plant between those events can make the later subtraction differ from the earlier addition; reproducing native behavior requires event-time wrapping add/subtract, not a periodic raw-cost recount or a rebuild of all three counters.

The updater reads `[AI] AIForcePredictionFudge=` from the three-int vector at `Rules+0x9A8` (data pointer `+0x9AC`), indexed directly by `House+0x184` (`0=Hard, 1=Medium, 2=Easy`). For each value independently, in order air, armor, infantry:

```text
base = tracked_value + 3000
radius = ftol(base * AIForcePredictionFudge[difficulty] * 0.01f)
noisy = base + RandomRanged(-radius, radius)
```

Thus a designated-enemy refresh consumes exactly three shared scenario RNG draws even when the later sum is non-positive. If `air_noisy + armor_noisy + infantry_noisy > 0`, native writes:

```text
House+0x1609C = float32(armor_noisy / total)
House+0x160A0 = float32(air_noisy / total)
House+0x160A4 = float32(infantry_noisy / total)
```

otherwise it restores all three exact `0.33f` defaults. Retail `rulesmd.ini` actively sets `AIForcePredictionFudge=5,25,80`, so this RNG-bearing path is active after ordinary AI enemy designation.

### 7.4 Exact defense-candidate vectors

The three category helpers are not generic sidebar/build-option queries:

- `FUN_00507B80` builds the AntiAir candidate vector and requires `BuildingType+0x1524 > 0`.
- `FUN_00507D70` builds the AntiArmor candidate vector and requires `BuildingType+0x1528 > 0`.
- `FUN_00507F60` builds the AntiInfantry candidate vector and requires `BuildingType+0x152C > 0`.

All three first read `HouseTypeClass+0xBC` through `House+0x34` and choose one Rules vector: side index `0` uses `[AI] AlliedBaseDefenses @ Rules+0x954`, index `1` uses `SovietBaseDefenses @ +0x970`, and every other index uses `ThirdBaseDefenses @ +0x98C`. `RulesClass__ReadAI @ 0x00672AE0` parses each comma-separated BuildingType list in INI order; an absent key clears/leaves that vector empty. Current Rust already stores the exact resolved `HouseState.side_index`, but `RuleSet` does not parse these three lists.

Each helper iterates the chosen Rules vector in **reverse INI order**. A type is appended, preserving that reverse order, only when all of these literal tests pass:

1. Its `TechnoType+0x6CC` owner bit includes `1 << HouseTypeClass::FindIndexOfName(HouseType+0x98)`. This is the House's country/HouseType identity, not merely the side index used to choose the list.
2. `BuildingType+0x634 <= House+0x1D4`. There is no lower-bound test here; a negative TechLevel is not independently rejected by these helpers.
3. The category-specific Anti value named above is strictly positive.
4. `FUN_00505360` reports every prerequisite satisfied by the current owned-type inventory.

The prerequisite inventory is constructed immediately beforehand by walking the House owned-object vector in stored order and appending each object's type except `IsBaseDefense @ BuildingType+0x1706` types and the `[General] WallTower @ Rules+0x87C` type. `FUN_00505360` requires every `BuildingType+0x63C/+0x648` prerequisite entry: a non-negative entry matches that exact BuildingType pointer; negative symbolic entries match any owned type in the corresponding Rules list (`-1 BuildPower`, `-2 BuildWeapons`, `-3 BuildBarracks`, `-4 BuildRadar`, `-5 BuildTech`, `-6 BuildRefinery`). An empty prerequisite vector succeeds. This helper does not apply the current Rust generic strict-option extras such as a TechLevel lower bound, Required/ForbiddenHouses, stolen-tech, factory presence, BuildLimit, cost, or current credits; reusing `BuildOption.enabled` as this filter would therefore be wrong.

Retail YR `rulesmd.ini` activates and orders all three lists:

```text
AlliedBaseDefenses=GAPILL,ATESLA,NASAM
SovietBaseDefenses=NALASR,NABNKR,TESLA,NAFLAK
ThirdBaseDefenses=YAGGUN,YAPSYT,NATBNK
```

Before filtering, the native iteration orders are consequently `NASAM,ATESLA,GAPILL`; `NAFLAK,TESLA,NABNKR,NALASR`; and `NATBNK,YAPSYT,YAGGUN`.

### 7.5 Quadrant/category/type selection and grid lifetime

Native selects the quadrant with the lowest combined AntiAir+AntiArmor+AntiInfantry total. The scan is `0..3` with strict `<`, so lower quadrant index wins equal totals. When an alternate candidate vector is supplied, only quadrants containing at least one alternate cell are eligible.

For the selected quadrant, it computes:

```text
anti_air_deficit      = House.air_ratio_at_160A0      - anti_air_total / combined_total
anti_armor_deficit    = House.armor_ratio_at_1609C    - anti_armor_total / combined_total
anti_infantry_deficit = House.infantry_ratio_at_160A4 - anti_infantry_total / combined_total
```

If the combined total is zero, all three current fractions are zero. Category selection chooses the greatest deficit with tie priority **AntiInfantry, then AntiArmor, then AntiAir**. It sets `House+0x16060` to the corresponding influence grid.

If the selected category vector is empty, fallback priority is **AntiArmor, then AntiInfantry, then AntiAir**; if all three vectors are empty, production choice fails. With exactly one type, native chooses it and consumes no type-selection RNG. With more than one, it preserves the candidate-vector order above, constructs `{type, positive category Anti value}` records, sums the weights with signed 32-bit `ADD`, calls the shared scenario RNG exactly once as `RandomRanged(1,total)`, accumulates weights in record order, and chooses the first record for which the unsigned comparison `draw <= cumulative` succeeds. An already queued/forced building type can override the randomly selected type, but the selected category grid remains the sort input for its site.

Only after selecting category, type, and quadrant does the no-alternate-vector path call `0x005060B0`. Cleanup frees all three grids and resets `House+0x16060 = null`.

This lifetime proves that a ready-time placement function cannot obtain the native sort input by examining only the finished building and current base center. The planned cell belongs to the House BasePlan node selected by `AI_Choose_Building`, not to the factory/ready queue entry.

### 7.6 BasePlan-node site ownership and Building-exit reuse

The caller-side cache is part of this mechanism. Treating the result of `0x005060B0` as a field on Rust's ready queue would reproduce neither native lookup nor retry behavior.

#### 7.6.1 Exact vector and node fields used here

`House+0x5700` is the BasePlan container. Its verified vector fields are:

| House offset | Meaning |
|---:|---|
| `+0x5704` | vector vtable |
| `+0x5708` | pointer to the ordered node array |
| `+0x570C` | capacity |
| `+0x5710` | allocation byte, with count-aligned padding |
| `+0x5714` | active node count |

Every node is 16 bytes:

| Node offset | Verified use |
|---:|---|
| `+0x00` | signed BuildingTypeClass array index (`BuildingType+0xDF8`), with negative planner control values |
| `+0x04` | packed `CellStruct`, signed 16-bit X in the low word and signed 16-bit Y in the high word; packed `(0,0)` is the single native unplanned/empty/invalid-site bit pattern inserted by the planner |
| `+0x08` low byte | filled/placed latch; successful AI `BuildingClass::Unlimbo` sets it to one through `FUN_0042F260` |
| `+0x0C` | placement retry counter; node insertion and successful fill set it to zero, and `FUN_0042F380` increments it |

The upper three bytes at `+0x09..+0x0B` are not a second semantic dword in the verified consumers: native reads and writes the low byte at `+0x08`. Older reports calling both trailing dwords dead are disproved by `0x0042F260`, `0x0042EB50`, and `0x0042F380`.

`FUN_0042EB20 @ 0x0042EB20` returns a node pointer from a vector index supplied by `FUN_0042EB50 @ 0x0042EB50`. The explicit-type form of `0x0042EB50` scans from node zero upward and returns the first node whose `+0x00` equals that BuildingType array index and for which `FUN_0042E780 @ 0x0042E780` is false. It does not perform a separate `+0x08`-byte test in this explicit-type branch.

For this lookup, `0x0042E780` is the occupied/satisfied-node predicate. It first calls `FUN_0042E820`, which resolves the node's nonzero cell and nonnegative type, looks up the exact-cell building, requires the BasePlan owner and building owner to match, and accepts an exact type match; its `PowersUpBuilding` tail also accounts for installed upgrade slots at that cell. If that did not satisfy the node and the planned type has `Type+0x1571`, `0x0042E780` also treats the matching overlay or any building found in the node cell as satisfied. Consequently a later Building exit for the same type skips an already occupied plan node and can find a later node of that type.

The wildcard `FUN_0042EB50(-1)` used by `HouseClass__AI_Choose_Building @ 0x004FE3E0` has additional planner eligibility behavior. Its exact source-order scan is:

1. Skip every node for which `FUN_0042E780` reports satisfied.
2. In campaign (`g_GameMode == 0`), return the first unsatisfied node immediately. The caller does not inspect its filled latch and does not call the recycling helper.
3. In every nonzero mode, return the first unsatisfied node whose `+0x08` filled latch is zero immediately.
4. For an unsatisfied filled node, call `FUN_0050CAD0`. On false, continue scanning. On true, write packed `(0,0)` from `DAT_0089C310` to `node+0x04` and return that node.
5. Return `-1` only when the ordered scan exhausts all nodes.

The cleared value is packed `(0,0)`. `read_memory(0x0089C310, 8)` returned eight zero bytes, and assembly `0x0042EBC3..0x0042EBD1` copies the first dword to the selected node. Static initializer `0x0042E5D0..0x0042E5DE` and the `0x00A8EF98` initializer prove this BaseClass global and the named empty/invalid `CellStruct` sentinel are bit-identical despite their different addresses. The explicit-type lookup used by Building exit remains behaviorally distinct because it never calls the recycling helper or performs this write.

#### 7.6.2 Scenario-authored BasePlan population (`0x0042EBE0`)

The default `HouseClass__Constructor @ 0x004F54A0` constructs the embedded `BaseClass` at `House+0x5700` through `BaseClass__Constructor @ 0x0042E6F0`; the node vector begins empty with count/capacity zero and growth step ten. `HouseClass__Read_Scenario_INI @ 0x00500B40` later calls `FUN_0042EBE0` on that embedded base. This is the active authority for stock campaign BasePlan sections and is not replaceable by generated skirmish priorities.

`FUN_0042EBE0` performs this literal parse:

1. Clear the INI section cache, read signed `PercentBuilt=` using the current `Base+0x1C` value as the default, and read signed `NodeCount=` with default zero.
2. Iterate `i = 0..NodeCount-1` and format the key as `%03d`; source order is node order.
3. Read the value into a 128-byte buffer. If its first byte is `'-'`, tokenize and parse the first token with `atoi` as a signed control value. Otherwise resolve the first token with `BuildingTypeClass__FindIndexByName @ 0x0045E7B0`.
4. Parse the second and third comma-separated tokens with signed `atoi`, narrow each to 16 bits, and pack X in the low word and Y in the high word.
5. Append one 16-byte node. If vector growth fails, skip that node and continue with the next numbered key; there is no all-or-nothing rollback.

The matching scenario writer `FUN_0042ED60 @ 0x0042ED60` writes `PercentBuilt`, `NodeCount`, then the same ordered `%03d` keys. Negative nodes serialize as `%d,%d,%d`; nonnegative nodes serialize as `%s,%d,%d` using the BuildingType INI name. It serializes only type/control and X/Y. `BaseClass__CalculateChecksum @ 0x0042F180` likewise folds only node count, signed type/control, X, and Y.

The retail parser contains a real native undefined-stack defect: assembly `0x0042ED23..0x0042ED2E` copies two never-initialized local dwords into node `+0x08` and `+0x0C`. Those bytes are not scenario fields. The deterministic Rust translation must initialize the semantic filled latch to false and retry count to zero rather than manufacturing process-stack garbage. That normalization is evidence-backed for the active retail inputs:

- the scenario writer and BaseClass checksum exclude both values;
- campaign wildcard lookup returns the first unsatisfied node before reading the filled latch;
- campaign placement failure increments retries but never compares them or evicts a node;
- successful exact-match `BuildingClass::Unlimbo` overwrites filled to one and retry to zero;
- ordinary stock skirmish begins with no scenario-authored player plan and uses the generated path below, which explicitly initializes both values to zero.

The extracted retail campaign data activates this parser. Representative ordered sections include `all07smd.map` `YuriCountry` (`PercentBuilt=100`, `NodeCount=13`), `Yuri2` (`100`, `5`), and `Yuri3` (`0`, `13`); `all02umd.map` `YuriCountry` has 21 nodes; `c1a02md.map` `YuriCountry` has 54; and `all03umd.map` `Americans` has one `CAMISC01,106,46` node. No negative control token occurred in the extracted shipped campaign/co-op BasePlan sections, although the native parser supports it. `PercentBuilt` is preserved as a scenario field but does not substitute for the per-node filled latch in any verified placement consumer.

#### 7.6.3 Fresh-skirmish plan generation (`0x005054B0`)

The primary ordinary-skirmish writer is `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, not `AI_Manage_Build_Queue` and not a generic next-building chooser. `UnitClass__Deploy @ 0x007393C0` reaches it through `FUN_00505180 @ 0x00505180` from the successful-deploy block `0x00739855..0x00739926` when all of these gates hold: deployment successfully created the Building, the owner is not human-controlled, the deployed BuildingType is a ConstructionYard, and `g_GameMode != 0`. `FUN_00505180` calls `Computer_Paranoid` for the ordinary non-player nonzero-mode case, saves `g_MapEditorMode`, and calls Recalc with editor mode forced to zero only when `House+0x5714 == 0`, then restores the mode. The deploy path writes the new yard cell into node zero, `House+0x5750`, three House flags, and then calls `FUN_0050C920`; those extra active side effects are distinct open mechanisms.

`HouseClass__RecenterBase @ 0x0050C210` is another Recalc caller, reached only through `TriggerAction__Execute` case 30 in the current xref audit. After selecting the last live owned BuildConst object in reverse owned-object order, it uses the same empty-count gate and mode save/restore, then writes that yard cell into node zero and `House+0x5750`. Retail action-30 activation remains UNCHECKED, so it is not folded into the ordinary ConstructionYard-deploy mechanism. The remaining Recalc xref is `HouseClass__ComputerTakeover @ 0x0050A5C0`, addressed separately below.

Recalc first clears the complete existing BasePlan. It then builds an eligible BuildingType vector in global registration order. A type is eligible only when all of these literal gates pass:

- its `Owner` bit contains the HouseType index returned by `HouseTypeClass__FindIndexOfName`;
- `AIBasePlanningSide` equals the HouseType side or is `-1`;
- `AIBuildThis` is true;
- signed `TechLevel <= House+0x1D4`;
- `RequiredHouses` is `-1` or contains the HouseType country/type bit;
- `ForbiddenHouses` is `-1` or does not contain that bit;
- when shell superweapons are disabled and the type has a primary superweapon, the type is in source-ordered `BuildTech` or the resolved superweapon type's `DisableableFromShell` byte is false.

It owns a parallel selected-byte vector initialized false, then constructs priority order exactly:

1. Resolve the first buildable `BuildConst` (`Rules+0x8AC`), mark its matching eligible slot selected, and append it only when it occurs in the eligible vector.
2. Resolve the first buildable `BuildPower` (`+0x8C8`) and append it unconditionally. This step does **not** mark its eligible slot selected, so the same power type can occur again in the later topological order.
3. Resolve the first buildable `BuildBarracks` (`+0x900`) and, when it occurs in the eligible vector, swap that eligible entry and its selected byte into eligible index zero.
4. Resolve the first buildable `BuildWeapons` (`+0x938`) and similarly swap it into eligible index one.
5. Initialize the remaining counter to `eligible_count - 1`. Repeatedly scan every unselected eligible entry in current order. `GAPLUG` is withheld from the normal prerequisite-success branch. For every other type, append and mark it when `FUN_00505360` says every prerequisite is represented in the priority vector. If a full pass appends nothing, append and mark the last unselected index encountered as the deterministic cycle break. Continue until the remaining counter reaches zero.

`FUN_00505360 @ 0x00505360` requires every explicit nonnegative prerequisite BuildingType index to already occur in the priority vector. Negative prerequisite tokens require at least one already-present type from the mapped Rules vector: `-1 BuildPower`, `-2 BuildWeapons`, `-3 BuildBarracks`, `-4 BuildRadar` (`Rules+0xA34`, confirmed by the `BuildRadar` reader at `0x0067369B..0x00673731`), `-5 BuildTech`, and `-6 BuildRefinery`. A type with no prerequisites passes. There is no credit, factory, BuildLimit, stolen-tech, or generic `BuildOption.enabled` fallback in this ordering helper.

Recalc next inserts extra refinery occurrences. It resolves the first buildable `BuildRefinery` (`Rules+0x8E4`) and scans source-ordered `HarvesterUnit` (`+0xB40`) for the first candidate whose Owner bit contains the HouseType. If that entry is non-null, the duplicate count is `AIExtraRefineries[House.difficulty]` (`Rules+0x1378`); otherwise it is `AISlaveMinerNumber[difficulty]` (`+0x1340`) minus one. `HarvestersPerRefinery` is the separate vector at `Rules+0x135C` and is not read by this function. It finds the first refinery occurrence before the priority vector's final element. For each positive duplicate, it consumes one shared Scenario RNG draw `RandomRanged(refinery_index, current_count - 1)` inclusive through `ScenarioClass+0x218`, inserts the same refinery immediately after the drawn index, and lets that insertion alter the next draw's range and order.

It then copies priority entries `0`, `1`, and `2` unconditionally into a final vector, followed by entries `3..end`; active retail guarantees those assumed entries. It selects a signed defense-sentinel count by House side and difficulty: side zero uses `AlliedBaseDefenseCounts` (`Rules+0xD84` data), side one `SovietBaseDefenseCounts` (`+0xDA0`), side two `ThirdBaseDefenseCounts` (`+0xDBC`), and every other side uses zero. Each sentinel consumes one shared Scenario RNG draw `RandomRanged(3, final_count - 1)` inclusive and inserts signed `-1` immediately after the drawn index, with later ranges observing earlier insertions.

Finally Recalc appends the final vector to BasePlan in exact order. Signed controls `-4..-1` are stored verbatim; the generated retail path produces `-1`. Nonnegative entries are converted from BuildingType pointers to `BuildingType+0xDF8` array indices. Every generated node receives packed cell `(0,0)`, filled low byte zero, and retry count zero. The uninitialized upper three bytes of the native filled dword have no semantic reader and are not part of the Rust representation.

The active YR retail inputs are source-ordered `BuildConst`, `BuildPower`, `BuildRefinery`, `BuildBarracks`, `BuildTech`, `BuildWeapons`, `BuildRadar`, and `HarvesterUnit` lists; `AIExtraRefineries=2,1,0`, `AISlaveMinerNumber=4,3,2`, and defense counts `25,20,6` Allied, `25,22,6` Soviet, and `25,22,6` Third. `HarvestersPerRefinery=2,2,1` is active but belongs to a different consumer. Their difficulty order is native Hard/Normal/Easy. None of these values may be replaced by Rust's generic infrastructure priority list.

#### 7.6.4 Runtime refinery/weapons insertion (`0x004FDD10`)

`HouseClass__AI_Building_Strategy @ 0x004FD500` calls `HouseClass__AI_Manage_Build_Queue @ 0x004FDD10` only in a nonzero game mode and only when `HouseClass__AI_Check_Build_Need @ 0x004FD9A0` returns `1`. That helper has only boolean `0/1` returns; the strategy's generic priority loop processes four down to one and therefore dispatches Manage at priority one.

Invocation cadence is part of the mechanism, not a host scheduling choice. `HouseClass__Constructor @ 0x004F54A0`, assembly `0x004F5B9D..0x004F5BA8`, initializes signed `House+0x5634` to the current native frame and `+0x563C` duration to zero, making the first eligible Update evaluation immediately expired without an initial RNG draw. `HouseClass::Update @ 0x004F8440`, assembly `0x004F8FBE..0x004F9032`, then evaluates the timer. A later start of `-1` expires only when stored duration is zero; otherwise native computes wrapping `i32(current_frame-start)` and expires when the signed elapsed value is at least the duration. On expiry it calls Strategy only when the House is not current/player-controlled under the native mode predicate (nonzero mode tests `CurrentPlayer`; campaign tests `CurrentPlayer || PlayerControl`) and `HouseType+0x1A6` passive is false. It then stores current frame as the new start and Strategy's return as the duration.

Strategy first owns a second signed timer and the designated-enemy bootstrap. The constructor initializes `House+0x5640` to the current frame and `+0x5648` duration to zero. In `ScenarioClass__Post_Map_Init @ 0x00686890`, assembly `0x00686A04..0x00686A2B`, every non-current, non-passive House is armed with `{start: current_frame, duration: Rules.AIHateDelays[House.difficulty]}`. `RulesClass__ReadGeneral @ 0x0066D530`, assembly `0x0066FDB1..0x0066FDD9`, reads `[General] AIHateDelays=` into the signed vector at `Rules+0x1174/+0x1178`; its constructor state is empty, both active retail rules files supply Hard/Normal/Easy `30,50,70`, and the post-map writer directly indexes the backing vector. No numeric missing-key fallback is evidenced. Strategy tests this timer at `0x004FD50F..0x004FD532` with the same signed start-`-1`, wrapping elapsed, and `elapsed >= duration` expiry semantics. It does not re-arm the hate timer, so after its one post-map delay the test remains expired on later Strategy calls.

On an expired hate timer, Strategy attempts acquisition only when `House+0x5600 == -1`, `g_GameMode != 0`, the owner HouseType is non-passive, and the owner's alternate base cell (when nonzero) or primary base cell is valid. It scans the global House array forward, skipping self, passive HouseTypes, and defeated Houses. The native candidate-distance body at `0x004FD538..0x004FD71E` mistakenly reloads the **owner's** alternate/primary base cell for every candidate rather than the candidate House's cell. Every admitted candidate therefore receives the same zero owner-to-owner distance; the strict-less comparison selects the first eligible global-House-order peer. Strategy does not assign that peer directly. It calls `HouseClass__UpdateAngerNodes @ 0x00504790` with signed delta `+1` and the selected peer.

The peer-score lifecycle is load-bearing. In the registered constructor path, `HouseClass__Constructor @ 0x004F54A0` cross-appends every newly constructed House to each existing House's `House+0x5608/+0x5614` vector and appends the existing Houses to the new vector in global House order, initializing every signed score to zero before the new House itself is appended to the global array. `UpdateAngerNodes` only wrapping-adds its delta to an **existing** matching peer node; it does not create a missing entry. It then rescans the ordered vector with initial best score zero and a strict-greater comparison, rejecting defeated, self/same-index, invalid-index, and allied peers, and writes the greatest positive eligible peer's House index to `+0x5600` or `-1` when none qualifies. Equal positive scores retain the first vector entry. Independently, `HouseClass::Update @ 0x004F8440` decrements every peer score greater than one by exactly one whenever signed `current_frame % 100 == 0`; that decay does not rerun enemy selection, so it changes future `UpdateAngerNodes` results but does not itself replace the current enemy.

Strategy next cleans up a currently designated defeated enemy at `0x004FD723..0x004FD772`. It finds the matching peer node, calls `UpdateAngerNodes(-node.score, defeated_peer)` to zero that node and perform the normal recomputation, and then unconditionally overwrites `House+0x5600 = -1`; the forced clear also happens when no matching node was found. Consequently `AI_TryFireSW` sees no designated enemy in that same Strategy call even if `UpdateAngerNodes` momentarily selected a different positive peer. Acquisition is not retried until a later Strategy call.

Only after acquisition/cleanup does Strategy call `HouseClass__AI_TryFireSW @ 0x005098F0` at `0x004FD77C..0x004FD79B` whenever `g_GameMode != 0` or signed `House+0x24C IQ >= Rules.IQ.SuperWeapons`; `RulesClass__Constructor @ 0x00665650` defaults `Rules+0x1438` to signed `4`, `RulesClass__ReadIQ @ 0x00674240` reads `[IQ] SuperWeapons=`, and active retail explicitly retains `4`. Nonzero ordinary skirmish therefore always reaches the call, while campaign uses the IQ threshold. The synchronous dispatch is followed by the emergency-state/action block below, not directly by `AI_Check_Build_Need`.

##### Strategy emergency state and abandonment actions (`0x004FD7A0..0x004FD911`)

`HouseClass__Constructor @ 0x004F56D0` initializes signed `House+0x250` to zero at `0x004F57C4`. After `AI_TryFireSW`, Strategy consumes it through the House secondary-interface vtable `0x007EA834`: slot `+0x18` is `0x004F6990`, `+0x34` is `0x005013A0`, and `+0x38` is the raw function beginning at `0x00501400`. The state machine is literal:

- State `4` calls `0x005013A0` and then `0x00501400`, leaves state `4` unchanged, and jumps directly to priority construction. It therefore bypasses the wallet and `+900` state logic in that invocation.
- State `0` calls the exact available-wallet query. A result below signed `25` writes state `1`. The control flow then immediately observes the new state; this can make a second wallet call in the same Strategy invocation, and a now-`>=25` result clears it back to zero.
- State `1` calls the same query and clears to zero when the result is signed `>=25`; a lower result retains one.
- State `3` computes wrapping signed `House+0x54D8 + 900`. It clears to zero only when that deadline is strictly less than the current frame; otherwise it remains three and jumps to priority construction. A state other than three is forced to three only when the current frame is strictly less than the deadline. Equality is intentionally asymmetric: an existing three survives equality, while another state is not armed at equality. The constructor's attack frame zero therefore forces ordinary early AI into state three before frame 900, but the state at frame 900 depends on the prior value.

The wallet target `0x004F6990` receives the embedded secondary-interface pointer at `House+0x24`. Its exact result is `ftol(StorageClass::GetTotalValue(House+0x2FC) * HouseType.IncomeMult(+0x148) + House.Balance(+0x30C))`. `StorageClass::GetTotalValue @ 0x006C9600` scans four slots, ignores nonpositive amounts, resolves `TiberiumClass+0xB8` unit value, and applies `ftol(value * amount + running_total)` per admitted slot. The Strategy comparison is therefore not a raw credits-only check and must retain both layers of native conversion.

`0x005013A0` is the first abandonment action. If the signed owned-Building count `House+0x2F0` is not positive it returns one. Otherwise it scans the ordered owned-object vector `House+0x6C/+0x78`; for each non-null object that is not in Limbo and has positive Health it invokes concrete object vtable `+0x1A0(1)`, then returns zero after the full scan. The base Object/Techno slot is an empty stub. Building overrides it with `BuildingClass__TogglePowerOrGate @ 0x00447110`; for a normally manager-initialized Building (`Building+0x6E9 != 0`) this does nothing when the current mission is already `19` (`Selling`) or `Building+0x6DF != 0`, otherwise it queues Mission `19` and commences it. The alternate `+0x6E9 == 0` FirestormWall path has no active retail assignment and is TS-era/inactive in stock YR. Callback returns are ignored by Strategy.

`0x00501400..0x0050153C` is the second abandonment action. It reverse-scans the global Techno vector (`count @ 0x00A8EC88`, data `@ 0x00A8EC7C`), admitting only Technos owned by this House whose active byte `+0x74` is nonzero and Limbo byte `+0x81` is zero, then applies this precedence:

1. A permanently mind-controlled Techno (`Techno+0x2C4`) whose type is `Insignificant` (`ObjectType+0x232`) and whose raw owner-House byte `House+0x1EC` is zero (non-human House) receives an exact lethal packet: damage starts at its type Strength, distance zero, `Rules.C4Warhead @ +0xFA8` (retail `Super`), null attacker, boolean flags `1,1`, and null source House. This branch skips the later Foot/Building action for that Techno.
2. Any remaining object whose Abstract flag bit `0x04` says IsFoot is removed from its Team through `TeamClass__Remove_Member @ 0x006EA870` when `Foot+0x5D4` is non-null, then queues Mission `15` (`Hunt`) with argument zero. It does not explicitly Commence; Aircraft inherit the virtual call but their override can reject Hunt during protected Retreat/paradrop/spyplane states without an AirstrikeManager.
3. Any remaining Building (`WhatAmI == 6`) whose occupant-count virtual `+0x408` is positive receives `BuildingClass__SellBuilding @ 0x00457DE0(1,0)`. In this call that helper evacuates garrison occupants; it is not the final Building sell/destruction operation. Successful occupants are unlimboed at the deterministic chosen exit, removed from any Team, and queued Hunt; failure paths remove occupants, and the disabled second flag does not enable the player-sell inside-foundation fallback.

`HouseClass__Constructor` initializes `House+0x249` to zero at `0x004F579B`. After the reverse scan—even when the vector was empty—the function writes it to one and never clears it. The executable-wide instruction search found one runtime consumer, `TechnoClass__Evaluate_Candidate @ 0x006F8765`: when the attacking owner's latch is set and `House+0x5600 != -1`, it resolves that exact designated-enemy House from the global House array and forces every candidate owned by some other House to score `1`. Non-designated candidates are de-prioritized, not rejected, and the field read is `House+0x5600`; stale prose identifying `+0x1580` is wrong.

State four is genuinely reachable but not present in stock skirmish data. `TriggerAction__Execute` action `9` resolves the requested House, returns zero for null, otherwise writes four at `0x006DEAFF` and returns one. Team script opcode `0x1E` writes four to its owning `Team+0x2C` House at `0x006E99E5` and marks `Team+0x80` complete. The complete retail map census found 31 physical action-9 records: eight in `MAPS01.MIX`, two in `MAPS02.MIX`, nineteen in `mapsmd03.mix`, and two in the duplicate/override campaign payloads in `expandmd01.mix`; it found zero in `MULTI.MIX`, `multimd.mix`, all 53 loose map packages, and both loose `.map` files. Across all 2,917 map-defined ScriptTypes and 10,634 decoded steps, opcode `0x1E` occurs zero times. It also occurs zero times across the 88/458 `aimd.ini` and 52/257 `ai.ini` ScriptTypes/steps.

Trigger action `6` is a separate direct entry: it resolves a House and invokes secondary slot `+0x38` (`All To Hunt`) without running Fire Sale or writing state four. The same census found 69 physical action-6 records—34 in `MAPS01.MIX`, 10 in `MAPS02.MIX`, 22 in `mapsmd03.mix`, and three duplicate/override records in `expandmd01.mix`—and zero in both multiplayer archives or any loose map. Thus both scripted entries are stock-active campaign behavior and evidence-backed stock-skirmish exclusions, while constructor state zero, the wallet transitions, state-three timing gate, and no-factory priority below remain ordinary-skirmish active.

Priority construction begins only after that block. In nonzero mode native gives priority slot zero value `4` exactly when `House+0x250 != 3` and a forward scan of the owned-object vector finds no non-null object with active byte `+0x90 != 0`, Limbo byte `+0x81 == 0`, and resolved `BuildingType+0xEB8 Factory != 0`. This gate does not check Health. Priority four executes `0x005013A0` and `0x00501400` in that order before the lower `AI_Check_Build_Need`/Manage priority. State four can therefore run both actions once in its direct block and a second time in the same Strategy call when the no-factory priority also qualifies. Manage still runs only for `AI_Check_Build_Need == 1` in nonzero mode.

Finally Strategy makes one **unconditional** shared-Scenario `RandomRanged(1,7)` call at `0x004FD913..0x004FD928` and returns `draw + 0x69`, producing the next delay 106..112 frames inclusive. One Strategy invocation therefore owns exactly one unconditional reschedule draw, but not necessarily one total draw: `AI_TryFireSW`, its targeting helpers, the launched superweapon, and post-launch spy-reveal checks can consume an earlier variable number from the same Scenario stream. The required order is `AI-hate acquisition/defeated cleanup -> AI_TryFireSW -> House+0x250 state/action block -> no-factory priority -> AI_Check/Manage -> unconditional reschedule`.

`AI_TryFireSW` repeats the mode-sensitive player-control rejection and then iterates the House Supers vector `House+0x258/+0x264` in stored order, accepting only non-null entries whose ready byte `SuperClass+0x6F` is nonzero. Its `SuperWeaponTypeClass+0xB4` dispatch is literal:

| Type enum / retail `Type=` | AI action in `0x005098F0` |
|---|---|
| `0` / `MultiMissile` | Requires designated enemy `House+0x5600 != -1`; use explicit `+0x54F0` when nonzero, otherwise `AI_FindBestRallyTarget` when target mode `+0x54EC == 1`, else `AI_FindTeamTarget`; fire a valid result synchronously. |
| `1` / `IronCurtain` | No action in this AI dispatcher. |
| `2` / `LightningStorm` | `AI_Fire_LightningStorm @ 0x00509E00`: reject an already active storm or missing designated enemy, then resolve the same explicit/best/team target and fire synchronously. |
| `3` / `ChronoSphere`; `4` / `ChronoWarp` | No action in this AI dispatcher. |
| `5` / `ParaDrop`; `6` / `AmerParaDrop`; `8` / `SpyPlane`; `11` / `PsychicReveal` | `AI_GroundRallyPoint @ 0x00509CD0`: explicit `+0x54F0` wins; mode one chooses the designated enemy (or self without one) alternate base cell when nonzero, else primary base cell, runs the exact `Find_Nearby_Passable_Cell` 5-by-5 query, then adds `(2,2)`; other modes use `AI_FindTeamTarget`; fire a valid result synchronously. |
| `7` / `PsychicDominator` | `AI_Fire_PsyDom @ 0x0050A150`: reject an active Dominator, missing enemy, or explicit target; reverse-scan the global foot/mobile vector, score each live seed by the count of non-own/nonallied and psionic-eligible cell occupants across the first 38 native offset-table entries, retain the first strict greatest seed, and fire its in-playfield cell when the count is nonzero. |
| `9` / `GeneticConverter` | `AI_Fire_GenMutator @ 0x00509F60`: only with no explicit target, reverse-scan the global Infantry vector, score each non-limbo seed by the count of non-own/nonallied Infantry across the first 10 native offset-table entries, retain the first strict greatest seed, and fire its in-playfield cell when the count is nonzero. |
| `10` / `ForceShield` | Use explicit `+0x54F8` when nonzero; otherwise use spy-reveal cell `+0x54F4` only while `+0x54FC + Rules.AISuperDefenseFrames > current_frame`; fire a valid result synchronously. |

The stock constructor initializes target mode `+0x54EC=1`, explicit cells `+0x54F0/+0x54F4/+0x54F8` to packed zero, and spy-reveal frame `+0x54FC=-100`. Default MultiMissile/LightningStorm targeting therefore enters `AI_FindBestRallyTarget @ 0x0050CBF0` whenever the House has a designated enemy. That helper scans the global Techno vector in stored order. A Techno is a final candidate only when its owner is the designated enemy and either `(vtable+0x78 == 2 && Techno+0x90 != 0 && Techno+0x81 == 0)` or, on native Hard difficulty index zero, some global Factory has that exact object current with nonzero rate and is not suspended. Its initial score is one and the RTTI-specific overrides are:

| RTTI | Exact score |
|---|---|
| Unit `1` | `AIIonCannonHarvesterValue[difficulty]` for `Harvester`; otherwise `AIIonCannonMCVValue` when `DeploysInto` matches a source-ordered `BuildConst`; otherwise literal `2` with `Passengers < 1`, else `AIIonCannonAPCValue`. |
| Building `6` | In priority order: `AIIonCannonConYardValue` for Factory enum `7`; `AIIonCannonWarFactoryValue` for Factory enum `0x28` and non-naval; `AIIonCannonPowerValue` when `PowerDrain < PowerOutput`; `AIIonCannonBaseDefenseValue` for `IsBaseDefense`; otherwise `AIIonCannonPlugValue` for `IsPlug`, `AIIonCannonTempleValue` for `IsTemple`, `AIIonCannonHelipadValue` for `HoverPad`, `AIIonCannonTechCenterValue` for source-ordered `BuildTech` membership, else literal `4`. |
| Infantry `0xF` | `AIIonCannonEngineerValue` for `Engineer`; else `AIIonCannonThiefValue` for `VehicleThief`; else literal `2`. |
| Other | Literal `1`. |

All named score vectors are signed Hard/Normal/Easy `[General]` inputs; retail sets ConYard/WarFactory/TechCenter to `100,100,100`, Power to `60,100,100`, Engineer/Thief/Harvester/MCV/APC to `1,1,1`, and BaseDefense to `35,35,35`. The native constructor leaves all these DynamicVectors empty and `ReadGeneral` copies the current vector through `DifficultyClass__ReadINI_IntVector`; active retail supplies every reached vector. Plug/Helipad/Temple lines are commented and remain empty, but neither retail rules file has an active `IsPlug`, `IsTemple`, or `HoverPad` type assignment, so those unsafe lookup branches are evidence-backed inactive for stock YR. Do not invent numeric fallbacks. A non-playfield cell first forces score zero. Native then performs a separate cloak override **before candidate admission**: for every global Techno whose `Techno+0x220 CloakState == 2`, or whose RTTI is Building and `Building+0x6ED == 0x0F`, it replaces that object's score with shared-Scenario `RandomRanged(0, current_best_score + 10)`. Thus even an ineligible cloaked object can advance Scenario RNG. Eligible strict-greater scores clear the tie vector; eligible equal scores append. When the final vector is nonempty, native calls `RandomRanged(0, count-1)` and selects that stored-order entry. The equal-bound count-one call consumes no raw RNG word; larger tie sets consume the native ranged sampler's variable rejection count.

`HouseClass__Fire_SW @ 0x004FAE50` resolves the Supers-vector index, performs the actual `SuperClass` launch synchronously, then scans the global House array in reverse order through `HouseClass__Check_Spy_Reveal @ 0x004FAF00`. Each non-passive, non-current eligible House whose fired SuperWeaponType has the spy-reveal flag and whose target lies within signed `Rules.AISuperDefenseDistance` of its active base cell consumes shared-Scenario `RandomRanged(0,99)` against `Rules.AISuperDefenseProbability[House.difficulty]`; a passing draw updates `+0x54F4/+0x54FC`. Launch/effect handlers also retain their own existing Scenario-RNG ownership. Therefore implementation must execute authoritative targeting, launch, and spy-reveal effects before the unconditional Strategy reschedule and validate the final Scenario RNG state; emitting a deferred `LaunchSuperWeapon` command that is applied only after rescheduling is not equivalent.

`RulesClass__Constructor @ 0x00665650` constructs `AISuperDefenseProbability` at `Rules+0xEC4/+0xEC8` as an empty vector, writes signed `25` to `AISuperDefenseFrames @ +0xEE0`, and signed `10` to `AISuperDefenseDistance @ +0xEE4`. `RulesClass__ReadGeneral` binds the three exact names, and active `rulesmd.ini` overrides them with Hard/Normal/Easy `90,50,10`, `50`, and `12`. The direct difficulty-vector consumer does not justify a fabricated probability fallback when the key is absent.

That strategy timer does **not** replace the independent production-chooser scheduler immediately following it in `HouseClass::Update`, assembly `0x004F9038..0x004F9265`. The second block repeats the same mode-sensitive not-current/not-player-control predicate and passive-House rejection, then runs only when native's signed remainder normalization yields `current_frame % 8 == 0`. Merely reaching this block owns no Scenario RNG draw; RNG consumed inside an invoked chooser remains that chooser's responsibility. It reads signed chooser mode `House+0x1E4`, initialized to zero by the House constructor at `0x004F56DF`, and dispatches as follows:

- In campaign (`g_GameMode == 0`) or mode `0`, call `AI_Choose_Building @ 0x004FE3E0`, `AI_Choose_Unit @ 0x004FEA60`, `AI_Choose_Aircraft @ 0x004FEEE0`, and `AI_Choose_Infantry @ 0x004FF210` in that exact order, unconditionally.
- In a nonzero game mode with chooser mode `1`, call Building first. If cached building choice `House+0x564C` remains `-1`, fall through to Unit, Aircraft, and Infantry in that order. Otherwise call the selected BuildingType virtual `+0x94` with `(1,1,1,House)`; a nonzero result suppresses the other three choosers, while zero invokes all three.
- In a nonzero game mode with chooser mode `2`, call Unit first. Resolve the first Owner-compatible source-ordered `HarvesterUnit`; call Aircraft and Infantry next unless that type exists and cached unit choice `House+0x5650` equals its type index. Native then requests a Building fallback when all cached unit/infantry/aircraft choices are `-1`, or when any present selected type's respective virtual `+0x94(1,1,1,House)` returns zero. The availability checks run Unit, Infantry, Aircraft order. Absent individual choices do not themselves set fallback unless all three are absent.
- Other mode values invoke none of the four choosers in this block.

`House+0x1E4` is authoritative, persistent chooser state rather than a derivation from queue emptiness. `HouseClass__AI_Manage_Build_Queue` writes mode `2` on its funded harvester branch (`0x004FE109`), and writes mode `1` on the refinery/weapons branch (`0x004FE3AB`) immediately before its direct `AI_Choose_Building` call at `0x004FE3B5`. `HouseClass__AI_EconomyStateMachine @ 0x00509700` supplies the remaining runtime transitions. Its only three callers are Building exit at `0x00443CBC` with kind `2`, `0x00444102` with the exiting object's runtime kind, and `0x00444F34` with Building kind `6`. It runs only in a nonzero game mode for a non-current House and applies the following exact state machine around wallet balance and signed `Rules+0x1300`:

- mode `0`: when credits are below the threshold, change to `2` for exiting kind `6`, otherwise `1`;
- mode `1`: credits at or above the threshold change to `0`; otherwise kind `6` changes to `2` and every other kind retains `1`;
- mode `2`: credits at or above the threshold change to `0`. Below it, scan source-ordered `BuildBarracks` and `BuildWeapons` for at least one owned instance from each list. If both exist and signed `PowerDrain <= PowerOutput`, consume `RandomRanged(0,1)` and retain mode `2` immediately on draw one. Otherwise, or on draw zero, a non-Building exit changes to `1` and a Building exit retains `2`;
- mode `3`: only credits strictly above the threshold change to `0`; otherwise retain `3`. No ordinary static writer to mode `3` was found in the complete `House+0x1E4` operand inventory, but the loaded-state branch is literal and must remain deterministic.

`Rules+0x1300` is not the stale `AIPoorCreditThreshold` name found in older prose. `RulesClass__Constructor @ 0x0066703B` defaults it to signed `1000`, and `RulesClass__ReadGeneral @ 0x0066FDE7..0x0066FE00` reads `[General] AIAlternateProductionCreditCutoff=`. Both retail rules files explicitly set `1000`.

The `AI_Check_Build_Need` trigger is exact rather than a generic “queue empty” test. It rejects a current/player-controlled House (`CurrentPlayer || PlayerControl` in campaign, `CurrentPlayer` in nonzero modes), then returns zero immediately when `FUN_004F6540` reports that the current resource-base combination is already supportable. That support helper snapshots:

- signed ResourceDestination count `House+0x15C > 0`, ResourceGatherer count `+0x158 > 0`, and non-naval Refinery count `+0x160 > 0` (`BuildingType.Refinery=yes && Naval=no`);
- whether the House owns a BuildConst object (`House+0x60 > 0`) or any Owner-compatible type from source-ordered `BaseUnit` (`Rules+0xB20`);
- available credits through the `House+0x24` wallet vtable `+0x18`;
- first-buildable BuildRefinery and BuildWeapons costs through their type vtable `+0x84`;
- the first Owner-compatible source-ordered HarvesterUnit and its same cost virtual.

It returns true immediately when both resource counts are positive. Otherwise its literal branch table is:

| BuildRefinery exists | HarvesterUnit exists | Exact true conditions after the both-count fast path |
|---|---|---|
| no | yes | ResourceGatherer count positive; or Refinery count positive and credits `>= harvester_cost`; or BaseUnit/BuildConst present and credits `>= BuildWeapons_cost + harvester_cost` |
| yes | no | ResourceGatherer count positive; or BaseUnit/BuildConst present and credits `>= refinery_cost` |
| yes | yes | with ResourceDestination present: `(Refinery present && credits >= harvester_cost)`, or `(BaseUnit/BuildConst present && credits >= BuildWeapons_cost + harvester_cost)`, or `credits >= refinery_cost`; without ResourceDestination: BaseUnit/BuildConst present and credits **strictly greater than** `refinery_cost` |

Active retail supplies all referenced lists; the nominal neither-candidate/null paths contain unsafe native dereferences and are not fallback behavior.

Only when that support helper returns false does `AI_Check_Build_Need` continue. It resolves the first Owner-compatible HarvesterUnit again and branches on signed ResourceDestination count:

- below one destination, choose first-buildable BuildRefinery, falling back to first-buildable BuildWeapons. If its type differs from cached building choice `House+0x564C`, return zero when its owner-adjusted cost is `<=` available credits and one otherwise. If it equals the cached choice, scan the global FactoryClass array in source order for an owned factory whose current object is RTTI Building and the same type; a match returns zero when signed `Factory+0x60 <= credits`, one otherwise, and exhaustion returns one.
- at one or more destinations, native first scans the prefix of the global BuildingClass array bounded by `House+0x78` and returns zero if it finds either of the first two raw BuildRefinery entries at Building mission/status `+0xAC == 0x12`. If the Owner-compatible HarvesterUnit is null, return one. If its type differs from cached unit choice `House+0x5650`, return zero when cost is `<=` credits and one otherwise. If it equals the cached choice, scan global factories in source order for an owned factory whose current object is RTTI Unit with `ResourceGatherer=yes`; the same `Factory+0x60 <= credits` rule returns zero, a greater value returns one, and exhaustion returns one.

Thus only result one reaches Manage, and current Rust's “no active building queue” predicate cannot substitute for this resource counts, exact candidate identity, current-choice, mission, factory-object, and affordability gate.

Manage selects a funding target before mutating the vector. It computes `both = signed(House+0x15C) > 0 && signed(House+0x160) > 0`, resolves first-buildable `BuildRefinery`, and scans `HarvesterUnit` in source order for the first Owner-compatible unit. Its exact candidate branch is:

- with an Owner-compatible harvester: choose the harvester and set the unit-needed branch when `both` is true; otherwise choose the refinery when available; otherwise choose the harvester when signed `House+0x160 > 0`; otherwise choose first-buildable `BuildWeapons` and set the weapons branch;
- without such a harvester: choose the refinery when available; if no refinery and `House+0x160 != 0`, exit; otherwise choose first-buildable `BuildWeapons` and set the weapons branch.

It walks BasePlan backwards, selling/removing satisfied structures or eligible upgrades and accumulating refunds until the strict funding predicate `candidate_cost < House.Balance + accumulated_refunds` succeeds. If it never succeeds, it exits. On success it cancels the House's factories in reverse global order and clears all four cached production-choice fields. The harvester branch then writes chooser mode `2` and the harvester type and does not call `AI_Choose_Building`.

The refinery and weapons branches call wildcard `FUN_0042EB50(-1)` and preserve a native zero-index quirk:

- return `0` exits this path without inserting and without calling `AI_Choose_Building`, even though zero is a valid first-node index;
- return `> 0` is the insertion index;
- return `-1` substitutes index `1` only when BasePlan capacity `House+0x570C >= 2`; smaller capacity exits. This proves Manage cannot bootstrap the constructor's empty zero-capacity plan.

At the chosen index native shifts every node at and after that index one complete 16-byte slot to the right. The refinery branch inserts first-buildable `BuildRefinery`; the weapons branch inserts first-buildable `BuildWeapons`. Both store the type's `+0xDF8` index, packed `(0,0)`, filled low byte zero, and retry zero. The weapons branch then scans later nodes backwards and removes a later entry only when its type equals both the newly inserted weapons type and the first raw `BuildRefinery` vector entry; the odd double-equality gate is literal. Both branches set chooser mode `1` and call `HouseClass__AI_Choose_Building` after the insertion attempt, including an allocation-growth failure after a nonzero lookup.

`HouseClass__ComputerTakeover` is a separate active transition writer, not an ordinary AI-house bootstrap. In nonzero modes its body requires `CurrentPlayer`; in campaign it accepts `CurrentPlayer || PlayerControl`. It clears both flags, renames the House `Computer`, cancels its factories, and only then recalculates an empty plan and maps already-owned buildings/upgrades into nodes. That trigger and its additional occupied-node insertions belong to player-to-computer takeover parity; ordinary fresh AI houses never enter this gate. The shared BasePlan representation must support its ordered mutations, but the ordinary placement path must not invoke or approximate takeover.

#### 7.6.5 `AI_Choose_Building` wall-plan and projected-power writers (`0x004FE3E0`)

`HouseClass__AI_Choose_Building @ 0x004FE3E0` is itself an active BasePlan population authority. Its mutation code addresses the vector through `House+0x5704` and then relative `+0x10` count/data fields, which is why a literal `House+0x5714` displacement search alone does not prove writer completeness. These branches run after wildcard lookup and before the ordinary cached production-choice write.

The entry gates return without mutation when a cached building choice already exists (`House+0x564C != -1`), the owned BuildConst-object vector count `House+0x60` is zero, or wildcard lookup returns no node. Before the wall branch, a nonnegative naval node (`BuildingType+0xCCE != 0`) is removed by ordered shift and wildcard lookup is repeated when `House+0x1F0 == 0`. The House constructor initializes that byte to one, and its only later House-relative zero stores are the already classified `0x00504860` convoy-script removal family; this is not an ordinary fresh-AI bootstrap rule. A selected `-3` node is likewise removed, but then dispatches the separately open `HouseClass__AI_ScanBasePerimeter` and returns `1`. A later selected `-2` returns without population. These classifications preserve the native ordered mutations without importing convoy or wall-execution ownership into this slice.

The wall-plan branch is entered when the selected node has signed type/control `-1`, or when its nonnegative type resolves to `[General] WallTower @ Rules+0x87C` and its packed cell is exactly `(0,0)`. Native saves all four node dwords and resolves the node's current vector index. It then **always consumes one shared Scenario RNG draw** `RandomRanged(0,99)`, even when wall expansion will fall through. The draw is compared strictly `< AIPickWallDefensePercent[House.difficulty]`, using the signed vector data at `Rules+0xDD8`; active retail supplies Hard/Normal/Easy `50,25,10`. Only a passing draw calls `FUN_0050C340`.

`FUN_0050C340` performs the exact wall expansion:

1. Scan source-ordered `ConcreteWalls` (`Rules+0xA50`, data `+0xA54`, count `+0xA60`) and select the first type whose signed `AIBasePlanningSide` equals the HouseType side or is `-1`. Native leaves the selected type index at `-1` when no entry matches; it has no separate missing-wall rejection. Retail order `GAWALL,NAWALL,GAFWLL` supplies side-specific matches for sides zero, one, and two.
2. Scan BasePlan backwards from `selected_index - 1`. At each index `i`, `FUN_0042E820(i)` must resolve an already placed Building and that BuildingType must have `ProtectWithWall=yes`. Assembly also compares **the following node** `node[i+1].type`, not `node[i].type`, against the selected wall type and accepts only inequality. The first reverse match wins. If none exists, return false without mutation.
3. Convert the matched live Building's lepton X/Y to signed-truncating cells and read its actual foundation width/height. The expanded top-left is `(building_x - 1, building_y - 1)`; the scenario node's cached cell is not used as the geometry source.
4. Insert every wall node at the same vector position `i+1`. For `x=1..width`, attempt top `(left+x, top)` and then bottom `(left+x, top+height+1)`. For `y=1..height`, attempt left `(left, top+y)` and then right `(left+width+1, top+y)`. Because every successful insertion uses the same index, final stored order is `right(height), left(height) ... right(1), left(1), bottom(width), top(width) ... bottom(1), top(1)`. Every inserted node is `{selected_wall_type_index, explicit_cell, filled=false, retry=0}`. Each allocation failure skips only that one insertion; there is no rollback, and a located source Building still makes the helper return true.
5. Feed a synthetic zero-filled wall node at the expanded top-left and the other three width/height corner offsets through the existing `FUN_0050E450`/`FUN_0050EB70` BaseReservation repair calls. This reuses the exact reservation writer services already closed by GSI-04.05 Deltas A-C; it is not permission to replace them or to claim `AI_ScanBasePerimeter` closed.

On helper success, the caller locates the originally saved four-dword sentinel/WallTower node by full value, removes that exact entry with an ordered complete-node shift, and returns `1` before `AI_ChooseNextProduction`. When the percent comparison fails or the helper returns false, the consumed RNG draw remains consumed and the caller continues through `AI_ChooseNextProduction`. If that call fails for a sentinel, native removes the selected entry once. If it fails for a zero-cell WallTower, native first removes that selected WallTower and then removes the new entry now occupying the same index when one remains; the literal double-removal can therefore consume its successor before returning `1`.

After a concrete node type exists—and, for a sentinel/zero-cell WallTower fallthrough, after `AI_ChooseNextProduction` has selected its type/site—the projected-power branch may splice a power plant immediately before the current node. Insertion occurs only when **all** of these signed native predicates hold:

- `House.PowerDrain + candidate.PowerDrain > House.PowerOutput - House.AICostTolerance`, using `House+0x53A8`, `type+0xEE4`, `House+0x53A4`, and `House+0x160B4`; x86 performs the `ADD` and `SUB` as wrapping 32-bit arithmetic and the branch is signed. The constructor initializes the tolerance to zero and the complete operand inventory found no later ordinary writer;
- the candidate type pointer is absent from source-ordered `BuildConst` (`Rules+0x8AC`, data `+0x8B0`, count `+0x8BC`);
- signed candidate `PowerDrain >= 1`;
- the `House+0x2A4/+0x2AC` power-blackout timer gate is clear: with start `-1`, **any nonzero** stored duration blocks insertion; otherwise native computes wrapping `i32(current_frame - start)` and blocks when the signed `elapsed < duration` comparison leaves a nonzero wrapping `duration - elapsed` remainder;
- `House+0x577B == 0`. `HouseClass__AI_AssessPower @ 0x00508C30` is the sole House writer after the constructor: it sets this byte when at least one counted Building has `TechnoClass__IsDeploying @ 0x0070FEC0` true (literal `Techno+0x1D0 != 0`) and `BuildingClass__GetPowerOutput > 0`. It is not an “offensive unit” flag, and no broader semantic substitute is evidence-backed.

Side selection is literal. House side zero chooses `GDIPowerPlant @ Rules+0x89C`; side two chooses `ThirdPowerPlant @ +0x8A8`; every other side builds a reverse-owned-object prerequisite inventory and tests `NodAdvancedPower @ +0x8A4` through `FUN_00505360`, choosing it on success and `NodRegularPower @ +0x8A0` otherwise. Native then resolves the current node's vector index, grows storage if necessary, shifts the current node and complete tail one 16-byte slot right, and writes `{power_type_index, (0,0), false, 0}` at the original index. The shifted original node retains its selected type, planned site, filled latch, and retry count byte-for-byte. If growth fails, no node is inserted, but this branch still returns `1`; it does not fall through to set the ordinary cached building choice. If the power splice gates fail, only types with non-null `PowersUpBuilding` can enter the later cached-upgrade-cell validation tail; ordinary non-upgrade nodes bypass that tail and simply cache the selected node type. That upgrade-only tail remains outside this report's implementation closure.

Retail activates every input: `AIPickWallDefensePercent=50,25,10`, nonempty side-matched `ConcreteWalls`, `WallTower`, many `ProtectWithWall=yes` buildings, and `GDIPowerPlant=GAPOWR`, `NodRegularPower=NAPOWR`, `NodAdvancedPower=NANRCT`, `ThirdPowerPlant=YAPOWR`. `NANRCT` requires `NATECH,NACNST`, so the Soviet advanced-prerequisite fallback is player-visible rather than theoretical. This section closes BasePlan wall/power **population and order** only; execution of the separate `-3` wall-perimeter placement consumer remains open.

#### 7.6.6 Exact filled-node recycling policy (`0x0050CAD0`)

`FUN_0050CAD0` has one xref, the wildcard filled-node call at `0x0042EB8F`. The caller-effective nonzero-mode policy and the helper's literal internal order are both required; replacing either with generic build eligibility changes which plan node is selected and whether production/site selection runs.

The helper evaluates this exact order:

| Order | Native predicate | Result |
|---:|---|---|
| 1 | signed `node.type < 0` | true |
| 2 | signed `node.type >= g_BuildingTypeCount` | false |
| 3 | resolved type has both `ResourceDestination` (`Type+0x5ED`) and `ResourceGatherer` (`Type+0x5EC`) | signed compare `House+0x158 < Rules.AISlaveMinerNumber[House.difficulty]`; return that result immediately |
| 4 | `g_GameMode == 0` | true |
| 5 | resolved type pointer is null | false; valid registry entries never take this defensive branch |
| 6 | `Type.GetWeapon(0)` returns a slot whose first dword is a non-null resolved weapon pointer | true |
| 7 | `Wall=yes` (`BuildingType+0x1571`) | scan all eight adjacent directions `0..7`; true on the first cell whose `Look_up_building_in_cell` result has the same owner House, otherwise false after direction seven |
| 8 | non-wall signed `PowerOutput` (`BuildingType+0xEE0`) is greater than zero | true |
| 9 | every remaining non-wall type | true exactly when signed `wrapping_i32(House+0x54D8 + Rules+0xDF0) <= g_CurrentFrameCounter` |

The wall branch returns after its adjacency scan: a `Wall=yes` type with positive `Power=` but no adjacent same-owner BuildingClass still returns false. Primary weapon presence precedes and bypasses the wall gate. The timer addition is ordinary x86 32-bit wrapping `ADD`, followed by signed `SETLE`; it is not saturating arithmetic and the rule is not clamped.

Because `House+0x54D8` starts at zero, the same timer gate also suppresses remaining ordinary filled-node recycling during the initial `AIRestrictReplaceTime` frames even if no building has yet received an attacker. With stock `400`, the signed predicate first becomes true at frame 400; a qualifying later BuildingClass attack resets the deadline to `attack_frame + 400` with the same wrapping comparison.

The table's type-pointer rows assume the valid active-retail BuildingType registry. Assembly loads `Type+0x5ED/+0x5EC` before the later null test, so a malformed in-range null registry entry would fault before reaching that nominal defensive branch; it is not a supported “campaign accepts null” case and must not motivate Rust fallback behavior.

The helper's internal campaign branch is unreachable from its sole active caller because `FUN_0042EB50` returns the unsatisfied node at `0x0042EB7B..0x0042EBD5` before the call. Likewise, an unfilled nonzero-mode node bypasses the helper. Consequently the resource-gatherer cap, weapon/power/wall tests, and replacement timer affect only **unsatisfied filled-node recycling in nonzero modes**.

Input ownership is exact:

| Input | Native constructor/parser/writer | Active retail value and Rust ownership |
|---|---|---|
| `ResourceGatherer` / `ResourceDestination` | TechnoType bytes `+0x5EC/+0x5ED`, constructor `0x00710FF0/0x00710FF6` defaults false and reader `0x007143D0..0x007143FE` parses the booleans. House constructor `0x004F55FE/0x004F5604` initializes signed `House+0x158/+0x15C` to zero; `HouseClass__Added_To_Game @ 0x00502A80` increments them before its RTTI switch and `Removed_From_Game @ 0x005025F0` decrements them; `TechnoClass::ChangeOwner` removes from the old House then adds to the new | stock `YAREFN` sets both and activates the cap; `ObjectType` already parses both booleans, but `HouseState` and its spawn/despawn/owner-transfer chokepoints maintain only broad building/unit counts, not these signed resource counts |
| `AISlaveMinerNumber` | `RulesClass` constructor initializes the `+0x133C` DynamicVector empty; `ReadGeneral @ 0x00670585..0x006705B7` copies it and `DifficultyClass__ReadINI_IntVector @ 0x00475D70` replaces it from `[General]`. A missing key produces an empty vector; there is no hardcoded three-value fallback. The recycling helper indexes `Rules+0x1340` directly by native House difficulty with no count check | active YR `rulesmd.ini` supplies `4,3,2` in Hard/Normal/Easy order; Rust already has exact `HouseDifficulty` discriminants `0,1,2` but does not parse this vector |
| primary weapon | TechnoType weapon slot zero defaults null and `Primary=` resolves the pointer at reader `0x007129AB..0x007129DE`, consumed by `GetWeapon(0) @ 0x007177C0`; the helper tests the resolved pointer, not merely a nonempty source string | many stock defenses activate it; Rust retains `ObjectType.primary`, so the port must require successful weapon resolution rather than only `Option::is_some()` |
| `Wall` and adjacency | `BuildingTypeClass` constructor `0x0045E03F` defaults `+0x1571` false; reader `0x0046048E..0x004604AC` parses `Wall=`. The helper steps the node's packed cell through all eight directions and uses the map's single-building cell lookup plus exact House pointer equality | stock wall types activate it; Rust parses `ObjectType.wall` and has entity occupancy/ownership, but no BasePlan recycle consumer |
| positive `Power` | BuildingType constructor `0x0045DEF6..0x0045DF08` defaults output/drain and extra output/drain to zero. Reader `0x00461060..0x0046109A` sign-splits nonnegative `Power=` to `PowerOutput+0xEE0` with drain zero and negative input to positive `PowerDrain+0xEE4` with output zero | stock power plants activate it; Rust retains raw signed `ObjectType.power`, for which `power > 0` is the exact gate |
| `AIRestrictReplaceTime` | `RulesClass__Constructor @ 0x006668D4` stores signed `500` at `Rules+0xDF0`; `ReadGeneral @ 0x006700FC..0x0067011A` calls `CCINIClass__ReadInt` with the current value, so absence retains `500` and mods remain signed | both retail rules files override it to `400`; Rust does not parse it |
| last building attack frame | `HouseClass__Constructor @ 0x004F5A59` initializes signed `House+0x54D8` to zero. A non-truncated full-program instruction search found exactly one non-constructor writer: `BuildingClass__ReceiveDamage @ 0x0044229C`. Entry first returns result zero for `attacker == victim` when the shared type has `DamageSelf=no`. Otherwise a non-null attacker writes `g_CurrentFrameCounter` unless Building vtable `+0x80` returns true. That slot resolves through `0x00457620` to `BuildingTypeClass__Is1x1WithUndeploy @ 0x00465D40`, so the skip is exactly `UndeploysInto != null && foundation == 1x1`; it is not a cloak test. The write precedes Building immunity, the already-dead gate, and the generic receiver, with no alliance or damage-sign test. Retail's only `UndeploysInto` buildings are `GACNST/NACNST/YACNST/YAREFN`, whose art foundations are `4x4/4x4/4x4/2x2`, so none takes the skip. The adjacent `House+0x54DC` write is the attacker owner's House array index, not the attacker type index. See `PHASE3_HOUSE_LAST_BUILDING_ATTACK_FRAME_0044229C_GHIDRA_REPORT.md` | Rust now has snapshot/hash-covered `HouseStrategyEmergencyState.last_building_attack_frame`, but the combat receiver has no equivalent pre-receiver writer; the adjacent owner-index and responder-selection mechanism is also absent |

#### 7.6.7 Production planning writes the node, not the queue

`HouseClass__AI_Choose_Building` obtains the node pointer with wildcard lookup, obtains that pointer's vector index through the vector vtable, and calls `HouseClass__AI_ChooseNextProduction(plan_index, alternate_vector)` at `0x004FE633`. `AI_ChooseNextProduction` reads `node[plan_index].type` before selecting the site:

- If the current node type is negative/empty, the selected defense type is the type passed to the site selector. A nonzero site result writes that selected type to node `+0x00` and the site to node `+0x04` at `0x00507B56..0x00507B77`.
- If the current node type is nonnegative, that existing planned type is passed to the site selector. A nonzero result writes the site to the current node `+0x04` at `0x00507A58..0x00507A66`. The newly selected defense type is then eligible to seed the immediately following empty node: native writes its type at `0x00507A86..0x00507A90` and copies the same packed site to that following node's `+0x04` at `0x00507A90..0x00507A9A`.
- A selector result `(0,0)` takes the failure cleanup path before any of those site writes.

The alternate-candidate-vector path changes how the site is selected and removes the chosen vector element, but it reaches the same BasePlan-node write block. There is no native write of the planned coordinate into FactoryClass state or a completed-building ready record.

#### 7.6.8 Building exit reuses, reselects, records, and invalidates the node

The AI Building exit branch at `0x00444F49..0x004451A6` performs this exact decision tree after looking up the first unsatisfied node for `exiting_type+0xDF8`:

1. **Node exists and `node.cell != (0,0)`.** A `PowersUpBuilding` type reuses it directly. An ordinary type first calls `HouseClass__HasBaseReservationNearBuilding @ 0x0050B760` on that cached cell. Connectivity success reuses the cached cell at `0x00445068`; connectivity failure calls `0x005060B0` at `0x00444FE1` and overwrites `node+0x04` with the newly selected packed cell at `0x00445055` before continuing.
2. **No node, or the matched node has `(0,0)`.** Ordinary placement calls `0x005060B0` at `0x004450BD`; `PowersUpBuilding` calls `0x00506B90`. When a node exists and the converted result is not the native null `CoordStruct`, native records the packed result in `node+0x04` at `0x0044519F`. With no node, it cannot record a site.
3. **Final placement is separate.** Cached-cell reuse does not rerun the selector's internal occupancy, type-vtable `CanPlaceAt`, signed-height, or orientation/probe stages. The downstream ExitObject path still performs its own final cell-entry/placement result dispatch at `0x004451CC..0x004454E6` and attempts the building's Unlimbo; this is why “no selector recheck” must not be misread as “unconditional placement.”
4. **Successful AI Unlimbo fills the node, with an active undeploy fallback.** `BuildingClass::Unlimbo @ 0x00440580` calls `FUN_0042F260` for a non-human owner at `0x0044159D..0x004415B3`. Native first requires the Building owner to equal the BasePlan owner and scans nodes from index zero for the first exact BuildingType-array-index and packed-cell match; that exact match has priority and does not require the node's filled byte to be zero. If no exact match was found, `0x0042F2DE..0x0042F31F` checks whether the placed type's `BuildingType+0x408 UndeploysInto` reference is non-null. When it is, native scans again from node zero and selects the first node whose type index matches and whose `+0x08` low byte is zero; this fallback does not require the node cell to equal the placed building cell. Either selected node reaches `0x0042F321..0x0042F325`, which sets `+0x08` low byte to one and resets `+0x0C` to zero. If neither search finds a node, no BasePlan state changes. Stock YR activates the fallback for `GACNST -> AMCV`, `NACNST -> SMCV`, `YACNST -> PCV`, and `YAREFN -> SMIN`; base RA2 activates it for `GACNST` and `NACNST`.
5. **Normalized final result `1` increments and may evict the node.** At `0x00445237..0x00445249`, native calls `FUN_0042F380(node)` first, so a non-null node's signed `+0x0C` retry counter is incremented with 32-bit wrapping arithmetic before any gate or comparison. Campaign mode (`g_GameMode == 0`) returns result `1` without eviction after that increment. In every nonzero mode, `0x00445253..0x0044525F` compares the new counter to signed `Rules+0xE48 MaximumBuildingPlacementFailures` and keeps the node while `retry_count <= maximum`. Only `retry_count > maximum`, with a non-null node, resolves that exact node's current vector index and removes its complete 16-byte entry by ordered shift-left at `0x00445265..0x004452C3`; the function still returns result `1`. The comparison is strict and post-increment: retail's value `3` retains failures one through three and removes on failure four. A null node yields counter result zero and cannot be removed.
6. **Other failed final placement clears the cached site.** For an ordinary non-wall/non-gate plan node, `0x0044552D..0x004455A2` scans all plan nodes and writes the shared packed-zero empty/invalid value to every `+0x04` cell equal to the failed coordinate, then returns failure. Wall/gate-style nodes take the separate vector-removal branch. A later exit attempt therefore reaches the zero-site selection path instead of retaining the failed ordinary coordinate.

This lifecycle is snapshot- and hash-relevant: node order, type, site, filled latch, and retry counter affect which type/site a deterministic AI chooses or retries. The existing `VecDeque<InternedId>` ready state cannot substitute for it.

#### 7.6.9 Building Limbo invalidates occupied plan cells (`0x0050A490`)

Successful placement is not the last BasePlan mutation. `BuildingClass__Limbo @ 0x00445880` calls `FUN_0050A490 @ 0x0050A490` with the Building's owner House and the Building pointer before `TechnoClass__Limbo`. Outside the map editor, the helper scans nodes from zero for the first exact BuildingType index and packed-cell match.

On a match, it first scans the complete plan and writes the native packed-zero empty/invalid sentinel from `0x00A8EF98` to every **other** node whose packed cell equals the matched cell. It leaves those nodes' type/control, filled latch, and retry count unchanged. It then applies an exact type/mode tail:

- if the removed BuildingType has `IsBaseDefense=no` (`type+0x1706`), return with the matched node unchanged;
- if `g_GameMode == 0`, return with the matched node unchanged even for `IsBaseDefense=yes`;
- only for `IsBaseDefense=yes` in a nonzero mode, overwrite the matched node's type/control with `-1` and its cell with the same packed-zero sentinel, again leaving filled/retry unchanged.

The BuildingType constructor defaults `+0x1706` false and `BuildingTypeClass__ReadINI @ 0x00460FFC..0x00461010` binds it to `IsBaseDefense=`. Stock defense types activate the nonzero-mode branch. On the next wildcard scan the matched `-1` node is unsatisfied but still filled; `FUN_0050CAD0` accepts negative controls immediately, after which wildcard lookup writes packed `(0,0)` again and returns it for a replacement defense. Limbo's global-loaded sentinel and wildcard's BaseClass-global zero are bit-identical, so this second site write does not create another state. Campaign instead retains the original matched type/cell and returns that now-unsatisfied node directly. This removal lifecycle is distinct from final-placement failure clearing by its type/fill behavior, not by a distinct cell bit pattern, and must be connected at the authoritative entity-limbo/removal chokepoint.

#### 7.6.10 Program-wide count-store exclusions

A full-program `House+0x5714` instruction inventory closes the remaining direct count-store candidates:

- the three stores at `0x00504970`, `0x00504A8F`, and `0x00504E31` each decrement count once and shift complete later nodes left. Their containing entry `0x00504860` has exactly two xrefs, both from `TeamClass__Convoy_Script_Attack_Production @ 0x006EEAB2/0x006EEB88`; there is no House AI building-strategy or `AI_Choose_Building` caller. They are scripted team target-consumption removals, not BasePlan population or ordinary building placement.
- `FUN_0050D250 @ 0x0050D250` has one xref from `MapClass__Resize @ 0x00566DDF`; it iterates the already existing node count and delegates coordinate adjustment without appending or removing a node. Runtime map resizing is not an ordinary retail skirmish/campaign placement event.
- `TriggerAction__Execute @ 0x006DE276` only compares the count; it does not write it.

Together with the constructor/BaseClass-relative scenario writer, Recalc, Manage, the vector-relative wall/power writers in `AI_Choose_Building`, `AI_ChooseNextProduction`, Building-exit eviction, ComputerTakeover, and the Limbo mutation above, this accounts for every direct or vector-relative active-retail node-count store found in the binary. The two excluded call families must not be copied into the ordinary placement implementation, but their separation is evidence-backed rather than assumed.

## 8. Active naval branch (`type+0xCCE != 0`)

The naval branch bypasses the reservation-perimeter algorithm.

1. Read `[General] Shipyard=` through `Rules+0x880`. Native calls `HouseClass__FirstBuildableFromArray @ 0x005051E0` twice in source order: the result at `0x00506103` supplies foundation width, and the independently repeated result at `0x00506128` supplies foundation height.
2. Use that type's foundation width+2 and height+2.
3. Choose `House+0x5494` unless invalid, otherwise `House+0x5490`, as the search origin.
4. Call `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` with the literal naval query in section 8.2. In particular, `-1` is the disabled required-zone ID; MovementZone is `0` (Normal), not `-1`.
5. If the result is valid and the owned BuildConst vector has a non-null first pointer at `House+0x54`, compare 3D distance from the result cell coordinates to that first construction-yard object's coordinates. Reject to the invalid-cell sentinel when distance is greater than `Rules+0xE0C * 256`.
6. If there is no first construction-yard pointer, do not apply the distance cap.

### 8.1 Exact ordered shipyard selector (`0x005051E0`)

This helper is not the generic sidebar/build-option predicate. It scans the supplied DynamicVector from index zero upward, returns the first passing `BuildingTypeClass*`, returns null if the vector is empty or every entry fails, consumes no RNG, and has no retry or fallback list. For each candidate, the tests are:

1. Resolve the House country index from `House+0x34 -> HouseType+0x98` through `HouseTypeClass__FindIndexOfName @ 0x005117D0`. Require `(type+0x6CC OwnerMask) & (1 << (country_index & 31)) != 0`.
2. Read the HouseType's stored country/type index at `House+0x34 -> +0xB8`. If `type+0xDA0 RequiredHousesMask != -1`, require that bit to be set. If `type+0xDA4 ForbiddenHousesMask != -1`, require that bit to be clear.
3. Read the HouseType side at `House+0x34 -> +0xBC`. Accept `type+0x6D0 AIBasePlanningSide == -1`; otherwise require exact equality with the House side.
4. After those four identity gates, `DAT_00A8B263 != 0` (`SuperWeaponsAllowed`) accepts the candidate immediately.
5. With superweapons disabled, `type+0x16F0 == -1` (no primary `SuperWeapon=`) accepts immediately. This helper does not inspect `SuperWeapon2=`.
6. Otherwise scan `Rules+0x920/+0x92C` (`[AI] BuildTech=`) in source order. Membership accepts the candidate even while superweapons are disabled.
7. Otherwise use the `type+0x16F0` index into the House's `House+0x258` SuperClass pointer vector, follow `SuperClass+0x28` to its type, and accept only when `SuperWeaponType+0xE7 DisableableFromShell == 0`. A non-BuildTech candidate granting a shell-disableable primary superweapon is the only rejection introduced by the superweapon-option tail.

There is no TechLevel, prerequisite, factory-presence, BuildLimit, cost, credit, stolen-tech, or produced-category test in this selector. Substituting Rust's strict generic `BuildOption.enabled` would add all of those gates and can change both the selected type and search footprint.

Stock YR makes the source-order and side gate active while leaving the later restrictions neutral:

| Candidate | `rulesmd.ini` source index | Owner | Required/Forbidden | `AIBasePlanningSide` | `SuperWeapon/SuperWeapon2` | effective stock result |
|---|---:|---|---|---:|---|---|
| `GAYARD` | 0 | all ten playable countries | both absent (`-1` masks) | 0 | both absent | first pass for Allied-side Houses |
| `NAYARD` | 1 | all ten playable countries | both absent (`-1` masks) | 1 | both absent | first pass for Soviet-side Houses |
| `YAYARD` | 2 | all ten playable countries | both absent (`-1` masks) | 2 | both absent | first pass for Yuri-side Houses |

Base RA2 has the same `GAYARD,NAYARD` order and corresponding side values, with its nine playable-country Owner lists. All three YR shipyards have `Foundation=4x4`, so retail YR always asks the nearby-cell service for a `6x6` footprint. The selector's identity is nevertheless active—earlier entries are rejected by side before the matching yard is found—even though the three stock footprints happen to be equal. RequiredHouses, ForbiddenHouses, BuildTech exemption, and the superweapon-option tail are exact supported gates but are not activated by the three retail `Shipyard=` entries.

### 8.2 Literal nearby-cell query and reachable bridge projection

Assembly `0x0050616D..0x00506193` loads the global map at `ECX=0x0087F7E8` and pushes the complete 15-argument query below. The decompiled call is:

```text
Find_Nearby_Passable_Cell(
    out, origin,
    5, -1, 0, 0,
    shipyard_width + 2, shipyard_height + 2,
    0, 0, 0, 1,
    &reference_zero, 0, 0)
```

| Parameter | Naval value | Exact effect |
|---|---:|---|
| output | local packed cell | receives a candidate or the invalid-cell sentinel |
| origin | alternate base cell when valid, otherwise primary base cell | ring-search center |
| speed type | `5` | Float passability row |
| required zone ID | `-1` | disables required-zone equality |
| MovementZone | `0` | Normal zone/passability family |
| bridge-aware height/zone input | `0` | no seed-height `+4`; run projection for collection early-stop and final pool partition |
| width / height | selected Shipyard foundation `+2/+2` | top-left passability rectangle; stock is `6x6` |
| reject any overlay | `0` | disabled |
| height-difference gate | `0` | disabled |
| current-cell obstacle-free gate | `0` | disabled |
| allow structural bridge cells | `1` | the separate `Cell+0x140 & 0x100` rejection is bypassed |
| reference cell | local packed `(0,0)` | equals the active invalid-cell sentinel, so selection uses current-frame modulo rather than nearest-reference distance |
| ring-side skip | `0` | visit the complete ring stream, including the duplicated radius-zero seed |
| final rectangle occupancy | `0` | disabled |

Because the bridge-aware input is zero, native uses `FUN_006D6410` twice: once while collecting candidates to decide whether a direct candidate arms the end-of-ring stop, and again to partition the stored candidates into preferred direct and fallback indirect pools. The function reads the candidate's signed height byte and `CellClass+0x140` flags. Its bridge correction is exact and active:

- when candidate `flags & 0x1000` is clear, projected probe-cell bridge flags do not alter their height;
- when candidate `flags & 0x1000` is set, each projected probe cell with `flags & 0x100` contributes **four additional height levels** before the isometric projection comparison;
- therefore the correction can change first-direct-ring termination, direct/indirect pool membership, pool length, and the cell selected by `g_CurrentFrameCounter % pool_length`.

These are not unidentified or unavailable bits. Rust already names and retains them as `BRIDGE_FLAG_FORWARD_SIDE = 0x1000` and `BRIDGE_FLAG_STRUCTURAL = 0x100` in `src/map/bridge_facts.rs`; active `SetBridgeDirection` stamping writes both into `ResolvedTerrainCell.bridge_facts.raw_flags`. The current `src/sim/find_nearby_cell.rs::is_direct_candidate` explicitly labels this gamemd behavior UNCHECKED and omits it. Its statement that the candidate flag's writer was not found is stale. The shared helper is therefore **not exact yet**, and the naval branch must not be declared closed until this narrow projection correction is implemented and revalidated for existing callers. This is ordinary intact-bridge projection behavior, not bridge destruction or destroyable-cliff work.

`Rules+0xE0C` is `[General] AINavalYardAdjacency=`, proven by the live `RulesClass::ReadGeneral` string load at `0x006701D9..0x006701FE`. Older references calling this field `MaxBaseDistance` are wrong.

Retail data makes the branch active:

- `rulesmd.ini`: `Shipyard=GAYARD,NAYARD,YAYARD`
- `rules.ini`: `Shipyard=GAYARD,NAYARD`
- `rulesmd.ini`: `BuildConst=GACNST,NACNST,YACNST`
- `rules.ini`: `BuildConst=GACNST,NACNST`

The rest of the shared `Find_Nearby_Passable_Cell` ring/validation/selection service is already separately researched and present in Rust. This report re-opens only the reachable `0x1000`/`0x100` projection correction above; that exact prerequisite remains part of this naval implementation slice.

## 9. Retail-data activation and exclusions

Authoritative retail files inspected read-only:

- `C:\Users\enok\Documents\vera20k-docs-backup\ini\rulesmd.ini`
- `C:\Users\enok\Documents\vera20k-docs-backup\ini\rules.ini`

| Mechanism/key | Retail status | Consequence |
|---|---|---|
| `AIBaseSpacing=1` | active in both files | ordinary clearance and connectivity are active; do not hardcode because mods may supply signed values |
| `ProtectWithWall=yes` | many active building assignments | `b += 1` branch is active |
| `WantsExtraSpace=` | no active assignment; commented examples only | mechanism is supported, but stock data does not activate it |
| `AntiAirValue/AntiArmorValue/AntiInfantryValue` | active on stock defenses including `GAPILL`, `NASAM`, `ATESLA`, `NALASR`, `NAFLAK`, `TESLA`, `NABNKR`, `YAGGUN`, `YAPSYT`, `NATBNK` | influence grids and category/type weighting are active |
| `AlliedBaseDefenses/SovietBaseDefenses/ThirdBaseDefenses` | all three lists active in `rulesmd.ini` with source order shown above | exact side selection, reverse candidate order, filters, and weighted RNG are active |
| `AIForcePredictionFudge=5,25,80` | active in `rulesmd.ini` | designated-enemy ratio refresh consumes three difficulty-scaled RNG draws |
| country `CostInfantryMult/CostUnitsMult/CostAircraftMult/CostBuildingsMult/CostDefensesMult` | no active assignment in either retail rules file; constructor defaults all five to exact `1.0f` | parser/category mechanism remains required, but stock countries do not alter tracked value |
| `FactoryPlant` and five `*CostBonus` fields | active only on stock YR `NAINDP`; `UnitsCostBonus=0.75`, other four `1` | stock Soviet Industrial Plant changes vehicle type-value calculations and therefore future tracked armor additions/removals |
| trigger action 137 | active in three `mapsmd03.mix` campaign payloads; absent from scanned stock multiplayer/loose payloads | alternate base cell must be connected for campaign parity and remains packed-zero/unset by default in stock skirmish |
| trigger action 9 / Team script opcode 30 | action 9 has 31 physical campaign records and zero multiplayer/loose records; opcode 30 is absent from all 2,917 map ScriptTypes and from retail `ai.ini`/`aimd.ini` | state-four Fire Sale plus All-To-Hunt is stock-active campaign behavior but an evidence-backed stock-skirmish exclusion; exact writers remain required for campaign parity |
| trigger action 6 | 69 physical campaign records; zero multiplayer/loose records | directly invokes All-To-Hunt, including the persistent target-bias latch, without Fire Sale or state-four mutation |
| `CloakGenerator=` | no active assignment in retail rules files | exclude the TS-legacy distance-sort branch from stock-YR implementation |
| `Shipyard=` | active | naval selector is active |
| `AINavalYardAdjacency=` | active General rule | naval distance cap is active when an owned BuildConst entry exists |
| `MaximumBuildingPlacementFailures=3` | active in both retail rules files; native constructor fallback is signed `5` | nonzero modes evict a BasePlan node on its fourth normalized result-1 placement failure; campaign never evicts it through this gate |
| `AISlaveMinerNumber=4,3,2` | active in `rulesmd.ini`; native constructor/default is an empty vector, not these values | an unsatisfied filled `YAREFN` node recycles in nonzero modes only while the House's signed ResourceGatherer count is below the Hard/Normal/Easy entry |
| `AIRestrictReplaceTime=400` | active in both retail rules files; native constructor fallback is signed `500` | after any non-cloaked BuildingClass receives a non-null attacker, ordinary non-weapon/non-wall/non-power filled nodes cannot recycle until the signed wrapping deadline passes |
| `Primary=`, positive `Power=`, and `Wall=yes` | all active on stock defense, power, and wall types | filled-node recycling accepts resolved-primary and positive-output types immediately; wall types instead require an adjacent same-owner BuildingClass |
| `UndeploysInto=` | active on stock YR `GACNST`, `NACNST`, `YACNST`, and `YAREFN`; active on base-RA2 `GACNST`/`NACNST` | successful AI Unlimbo can fill the first unfilled same-type BasePlan node even when exact cell matching failed |
| scenario `PercentBuilt/NodeCount/%03d` | active throughout shipped campaign data; representative counts include 54 in `c1a02md.map`, 21 in `all02umd.map`, and three independent 13/5/13 plans in `all07smd.map` | parse and preserve ordered type/control and signed 16-bit cell inputs; normalize the native parser's undefined filled/retry stack bytes to deterministic false/zero for the evidence-backed active semantics in section 7.6.2 |
| `BuildConst/BuildPower/BuildRefinery/BuildBarracks/BuildTech/BuildWeapons/BuildRadar` | all source-ordered lists active in `rulesmd.ini` | AI MCV deployment generates the initial BasePlan from these lists; current generic infrastructure priority is not equivalent |
| `AIExtraRefineries=2,1,0` and `AISlaveMinerNumber=4,3,2` | active Hard/Normal/Easy vectors in `rulesmd.ini`; `HarvestersPerRefinery=2,2,1` is separate | generated refinery duplicates and their shared-RNG insertion count are active |
| `AlliedBaseDefenseCounts=25,20,6`, `SovietBaseDefenseCounts=25,22,6`, `ThirdBaseDefenseCounts=25,22,6` | active Hard/Normal/Easy vectors in `rulesmd.ini` | generated `-1` defense sentinels and their shared-RNG insertion order are active for all three retail sides |
| `AIHateDelays=30,50,70`, constructor-registered anger peers, and defeated-enemy cleanup | active in both retail rules files and every registered ordinary House; post-map initialization arms each non-current/non-passive House by Hard/Normal/Easy difficulty | after expiry, a House with no enemy touches the first eligible global-order peer because the native distance loop reuses the owner's own base cell, then `UpdateAngerNodes` selects the strict greatest positive nonallied peer; a defeated current enemy is zeroed and forcibly cleared before AI superweapon targeting |
| 100-frame anger-score decay | active unconditionally in `HouseClass::Update` | every score greater than one decrements by one on exact signed frame multiples of 100 without recomputing the selected enemy, changing later acquisition/damage recomputations |
| strategy timer + `AI_TryFireSW` + `House+0x250` emergency state/no-factory priority + `AI_Check_Build_Need` | active native runtime for AI/non-passive Houses; nonzero ordinary skirmish always passes the AI-superweapon IQ call gate and activates no-factory Fire Sale/All-To-Hunt when state is not three, while Manage remains nonzero-mode/result-one only | every Strategy invocation runs AI-hate acquisition/cleanup, ready-Super targeting/fire, exact wallet/attack-frame state transitions, priority-four actions before AI_Check/Manage, then one unconditional reschedule draw for 106..112; state four can execute the actions twice when no Factory qualifies |
| active retail AI superweapon types | `NukeSpecial`, `IronCurtainSpecial`, `LightningStormSpecial`, `ChronoSphereSpecial`, `ChronoWarpSpecial`, `ParaDropSpecial`, `AmericanParaDropSpecial`, `PsychicDominatorSpecial`, `SpyPlaneSpecial`, `GeneticConverterSpecial`, `ForceShieldSpecial`, and `PsychicRevealSpecial` are all registered in `rulesmd.ini` | native AI actively handles MultiMissile, LightningStorm, ParaDrop/AmerParaDrop, PsychicDominator, SpyPlane, GeneticConverter, ForceShield, and PsychicReveal; IronCurtain/ChronoSphere/ChronoWarp are evidence-backed no-ops in this dispatcher |
| AIIonCannon target-score vectors | every vector reached by active retail type flags is explicitly populated in `rulesmd.ini`; Plug/Helipad/Temple lines are commented and no active retail type sets `IsPlug`, `IsTemple`, or `HoverPad` | default Nuke/Lightning best-target scoring is stock-active; the three empty-vector branches are evidence-backed excluded rather than assigned guessed defaults |
| `AISuperDefenseProbability=90,50,10`, `AISuperDefenseFrames=50`, `AISuperDefenseDistance=12` | active in retail `rulesmd.ini`; native constructor leaves the probability vector empty and defaults the scalars to `25` and `10` | reverse-House spy-reveal checks after a synchronous AI launch can consume shared Scenario draws and retain a ForceShield target window before Strategy's unconditional reschedule draw; no probability-vector fallback may be invented |
| eight-frame chooser scheduler + `AIAlternateProductionCreditCutoff=1000` | scheduler is active for eligible AI/non-passive Houses; the key is explicitly `1000` in both retail rules files and matches the native default | `House+0x1E4` mode dispatch and exit-driven economy transitions independently govern which production choosers run; an eight-frame tick itself does not consume the strategy timer's RNG draw |
| `AIPickWallDefensePercent=50,25,10`, `ConcreteWalls=GAWALL,NAWALL,GAFWLL`, `WallTower=GADUMY`, and `ProtectWithWall=yes` | all active in `rulesmd.ini`; ConcreteWalls supply side-zero/one/two matches | each eligible sentinel/zero-cell WallTower consumes one shared RNG draw; a passing reverse source match replaces it with a fixed-order explicit wall perimeter |
| `GDIPowerPlant=GAPOWR`, `NodRegularPower=NAPOWR`, `NodAdvancedPower=NANRCT`, `ThirdPowerPlant=YAPOWR` | all active in `rulesmd.ini`; `NANRCT` requires `NATECH,NACNST` | an ordinary projected-power deficit can splice the side plant before the already planned node; Soviet AI falls back to `NAPOWR` until the advanced prerequisites pass |

## 10. Current Rust comparison

Current owners inspected: `src/sim/ai.rs`, `src/sim/house_state.rs`, `src/sim/trigger_runtime.rs`, `src/sim/team_script_vm.rs`, and the target-scoring/combat paths under `src/sim`.

`place_ready_buildings` currently:

- waits until a building is ready;
- recomputes an average center of all owned structures;
- calls `find_placement_cell`;
- searches a hardcoded radius-12 square-ring spiral;
- runs `ai_base_reservation_candidate_ok` and then the general production placement preview.

`ai_base_reservation_candidate_ok` correctly contains parts of native substrate behavior—signed `AIBaseSpacing`, the ProtectWithWall/WantsExtraSpace increment, expanded occupancy, same-House reservation bits, shared dummy, and the equivalent connectivity endpoints—but it is not the native selector. Missing or wrong behavior includes:

- wrong timing and ownership: ready-time selection rather than production-planning-time capture in the selected House BasePlan node;
- no scenario `PercentBuilt/NodeCount/%03d` BasePlan parser or ordered initialization path;
- no AI ConstructionYard-deploy/recenter bootstrap through the exact `0x005054B0` eligibility, prerequisite order, refinery duplication, defense-sentinel insertion, and shared Scenario RNG draws;
- no `AI_Manage_Build_Queue` refinery/weapons splice behavior, including its nonzero lookup, index-zero exit, capacity-two fallback, complete-node shifts, and exact zeroed node fields;
- partially matching cadence but wrong eight-frame body: `AI_THINK_INTERVAL_FRAMES=8` matches native's independent chooser scheduler, but Rust turns that tick into a generic `!has_active_building_queue -> decide_next_building` decision. Native repeats the AI/non-passive eligibility gate, reads persistent `House+0x1E4`, and performs exact mode-zero/all-four, mode-one/building-first, or mode-two/unit-first dispatch and availability fallback; the tick itself owns no strategy-reschedule RNG draw;
- no separate signed snapshot/hash-relevant strategy timer: native calls only an eligible AI/non-passive House on expiry, applies the complete `FUN_004F6540`/`AI_Check_Build_Need` resource/factory/current-choice/mission/affordability gate, invokes Manage only for result one in a nonzero mode, and consumes one shared RNG draw to reschedule 106..112 even when Manage does not run;
- no snapshot/hash-covered `House+0x250` emergency state or persistent `House+0x249` All-To-Hunt latch, no exact Storage/IncomeMult/Balance wallet query, and no `<25`/`>=25` hysteresis or signed wrapping `last_building_attack+900` state-three boundary;
- no native no-factory priority-four scan or ordered Fire Sale/All-To-Hunt executor: Rust does not queue Selling across the ordered live owned Building set, reverse-scan global Technos for permanent-MC/Insignificant force-kill, Team removal/Hunt, or garrison evacuation, nor make the possible direct-state-plus-priority double invocation;
- `TriggerRuntime` has neither action 6's direct All-To-Hunt dispatch nor action 9's state-four writer, and `team_script_vm` treats retail opcode 30 as unsupported; the stock map census makes these campaign gaps rather than ordinary-skirmish inputs, but they are the only external entries into the same active mechanism;
- `TechnoClass__Evaluate_Candidate` has no equivalent persistent latch that follows `enemy_house` changes and forces non-designated-enemy candidate scores to one;
- no signed AI-hate timer or `[General] AIHateDelays` parser/post-map arming path, and therefore no expired-timer first-eligible peer touch before superweapon targeting;
- `combat::update_receiver_anger_nodes` already preserves the useful sparse-map equivalent of zero-initialized peer scores, validates registered peers, scans `ScenarioSession::house_order`, uses wrapping score addition, strict-greater winner selection, defeat/alliance filters, and updates `enemy_house`; preserve that behavior, but it is private to damage feedback and is never invoked by Strategy. Rust also lacks native's exact-frame-100 `score > 1` decay and defeated-current-enemy score-zero/forced-clear path;
- no strategy-owned AI superweapon hook before `AI_Check/Manage`: current `tick_ai(&Simulation, ...)` is read-only and returns commands which `World::run_late_region` applies afterward, so a newly emitted `LaunchSuperWeapon` command would run too late to preserve native target/launch/spy-reveal RNG before the reschedule draw;
- existing `HouseState.current_iq` can supply `House+0x24C`, but `RuleSet` does not parse `[IQ] SuperWeapons`; the current nested `BTreeMap<House, BTreeMap<SuperId, Instance>>` orders a House's Supers by interned ID rather than native stored-vector order;
- no `AI_FindBestRallyTarget` owner/factory/cell/type scoring, cloak-override draws, or tie-vector selection; no stored-order ready-Supers scan, exact native AI no-op exclusions, GroundRallyPoint/GenMutator/PsyDom target paths, or reverse-House spy-reveal checks;
- no parsed signed AIIonCannon Hard/Normal/Easy score vectors or native empty missing-key semantics for their stock-inactive Plug/Helipad/Temple branches;
- no parsed `AISuperDefenseProbability`, `AISuperDefenseFrames`, or `AISuperDefenseDistance` inputs for the reverse-House spy-reveal/ForceShield paths;
- current launch dispatch supports LightningStorm, IronCurtain, ForceShield, GeneticConverter, PsychicReveal, ParaDrop, and AmerParaDrop, but stock-active AI-handled MultiMissile, PsychicDominator, and SpyPlane remain unimplemented; failed unsupported launches leave the ready instance ready and cannot be treated as native AI fire;
- no snapshot/hash-covered `House+0x1E4` chooser mode, exact Manage writes/direct Building call, exit-driven `AI_EconomyStateMachine`, or parsed signed `AIAlternateProductionCreditCutoff`;
- no `AI_Choose_Building` wall override: `ConcreteWalls` strings are parsed, but `AIPickWallDefensePercent`, `WallTower`, reverse `ProtectWithWall` source selection, its always-consumed RNG draw, fixed-index perimeter insertions, sentinel removal, and reservation-corner repair are not connected;
- no projected-power BasePlan splice using the cached power equation, BuildConst/drain/blackout/deploying-building gates, side-specific power pointers, Soviet advanced prerequisite fallback, complete-node shift, or planned-site retention;
- no ordered, snapshot/hash-covered 16-byte BasePlan-node equivalent, explicit-type unsatisfied-node lookup, cached-site connectivity/reuse/reselection path, filled latch, retry counter, or failed-site clearing lifecycle;
- no Building Limbo hook that clears other same-cell nodes to packed zero and converts a nonzero-mode `IsBaseDefense` match to the filled `-1`/packed-zero replacement state;
- no wildcard source-order filled-node recycling policy, `(0,0)` cache clearing, or exact primary/positive-power/wall-adjacency/replacement-timer branch order;
- although `ObjectType.undeploys_into` is already parsed, no exact-match-first then ordered first-unfilled same-type BasePlan fill fallback consumes it;
- no parsed signed `MaximumBuildingPlacementFailures`, post-increment strict-threshold comparison, campaign no-eviction gate, or ordered BasePlan-node eviction on normalized final result `1`;
- `ObjectType` already parses `resource_gatherer`, `resource_destination`, `primary`, `wall`, and raw signed `power`, and `HouseState.difficulty` already uses native Hard/Normal/Easy indices; however `RuleSet` lacks `AISlaveMinerNumber` and signed `AIRestrictReplaceTime`, while `HouseState` lacks lifecycle-maintained signed resource counts and the last BuildingClass attack frame;
- wrong center: average live structures rather than native BaseClass primary/alternate/center state;
- no snapshot/hash-covered alternate base cell, alphabetic waypoint decoder, or trigger action 137 writer used by three stock campaigns;
- no distinct `+0x5750` base-plan center or reverse-owned-order BuildConst recenter writer;
- no ordered perimeter-vector sort;
- no transient defense-influence grids or exact category/quadrant selection;
- no `AntiAir/AntiArmor/AntiInfantry` or `IsBaseDefense` type fields, no side-specific BaseDefenses/WallTower Rules inputs, and no exact reverse-order defense-candidate vectors;
- current AI infrastructure priorities and generic `BuildOption.enabled`/first-enabled matching do not reproduce the native owner-bit, tech-ceiling-only, positive-Anti, prerequisite-only filter or the inclusive weighted RNG selection;
- no dynamic House enemy-value ratios;
- no native designated-enemy tracked-value refresh, event-time wrapping infantry/armor/air value counters, `AIForcePredictionFudge` input, or its three shared-RNG draws;
- `ObjectType` retains only raw `cost`; it does not parse `FactoryPlant` or the five per-building `*CostBonus` floats, while `CountryRules` already owns armor-category floats but not the five country `Cost*Mult` floats;
- no ordered live FactoryPlant identity vector, five stored-`f32` accumulated multipliers, exact per-entry rounding/recomputation lifecycle, or `0x00711F00` x87/ftol type-value helper;
- no standard angular key or Building-exit Chebyshev key;
- no adjacent-reservation vector sum and symmetric-cancellation skip;
- no native seed-direction conversion;
- no two clearance phases or three-cell tangential sweep;
- no signed height-byte `< 3` predicate;
- wrong predicate placement/order around the general placement preview;
- always enforces reservation connectivity because of a stale “no verified raw game-mode-0 analogue” comment, although `ScenarioSession.game_mode_nonzero` already provides the exact campaign/nonzero gate;
- no literal duplicate full candidate traversal;
- wrong failure representation/queue behavior;
- no active naval branch with Shipyard/BuildConst/AINavalYardAdjacency inputs;
- no parsed source-ordered `Shipyard` or `BuildTech` list and no `AIBasePlanningSide` type field, so it cannot reproduce `0x005051E0`; ObjectType already retains Owner/RequiredHouses/ForbiddenHouses/SuperWeapon, RuleSet already retains `DisableableFromShell`, HouseState already retains country/side, and ScenarioSession already retains `SuperWeaponsAllowed`, but those available pieces are not connected to a naval selector;
- its shared nearby-cell query surface can express the naval call's active settings, but `find_nearby_cell.rs::is_direct_candidate` omits native's candidate-`0x1000`/probe-`0x100` four-level projection correction even though `bridge_facts.raw_flags` already owns both flags; calling that helper exact would permit different direct rings, pools, and frame-selected naval sites near intact bridges;
- wall bypass is only a comment-backed shortcut and belongs to the separate `0x005082C0` consumer.

Therefore the existing spiral must not be preserved as the ordinary AI parity path. The already-correct reservation writer/grid/order work from commit `190dd620` should be preserved.

## 11. Implementation handoff

### 11.1 Required ownership boundary

The smallest coherent ordinary-player-visible implementation slice is not “replace the spiral loop.” It must connect production planning to placement. Existing `HouseState.enemy_house`, `HouseState.difficulty`, `HouseState.multiplay_passive`, and `ScenarioSession.game_mode_nonzero` should be preserved and consumed rather than duplicated:

1. Parse and retain `AntiAirValue`, `AntiArmorValue`, `AntiInfantryValue`, and `IsBaseDefense` with native defaults and signed integer behavior. Parse `[AI] AlliedBaseDefenses/SovietBaseDefenses/ThirdBaseDefenses` and resolve the already parsed source-ordered `ConcreteWalls` identities. Parse `[General] WallTower`, `GDIPowerPlant`, `NodRegularPower`, `NodAdvancedPower`, and `ThirdPowerPlant`, plus signed Hard/Normal/Easy `AIPickWallDefensePercent`, preserving native source order and resolved type identity. Parse signed `[General] MaximumBuildingPlacementFailures` with native constructor default `5` and signed `[General] AIRestrictReplaceTime` with native constructor default `500`; do not clamp modded negative values. Parse `AISlaveMinerNumber` in source order with no invented native fallback: active retail supplies exactly `[4,3,2]`, while the native missing-key state is an empty vector and its live consumer assumes an indexable difficulty entry.
2. Add a snapshot/hash-covered alternate base cell initialized to packed `(0,0)`, port the exact alphabetic waypoint-code conversion, and connect trigger action 137's valid-House/valid-waypoint writer. Preserve the existing launch/primary cell separately. Do not encode native “invalid,” “empty,” and literal zero `CellStruct` contexts as distinct states: all are the same active-retail packed-zero dword.
3. Add a distinct base-plan center and implement the active AI ConstructionYard-deploy primary/node-zero/BasePlan-center writes. Keep the `0x0050C210` reverse-owned-order BuildConst Recenter path separate until trigger action 30 and its retail activation are closed; Recenter also writes the primary `+0x5490` cell, so preserving separate state must not suppress that mutation.
4. Preserve the existing designated-enemy/anger identities while connecting lifecycle-maintained infantry/armor/air tracked values. Extend `CountryRules` with the five `Cost*Mult` `f32` fields/defaults and `ObjectType` with `FactoryPlant` plus the five `*CostBonus` `f32` fields/defaults. Own the ordered live FactoryPlant identities and the five stored-`f32` accumulators in authoritative lifecycle state (or prove a derived ordering byte-for-byte identical), preserving append, left-compaction, ownership-transfer, and per-entry `f32` rounding order. Port `0x00711F00` with FactoryPlant-first/country-second x87 multiplication and native ftol, then use it in the exact `0x00502A80/0x005025F0` event-time wrapping accounting branches. Do not substitute category counts, raw cost, `f64` accumulation, a periodic recount, or retroactive adjustment when FactoryPlant state changes.
5. Parse `[AI] AIForcePredictionFudge=`, retain the AI difficulty index, and implement `0x00508150` with exact float/x87 conversions and three shared-RNG draws in native order.
6. Build the three influence grids from the ordered owned-object vector with exact quadrant conversion, six-cell half-open falloff, integer accumulation, and native `ftol` behavior.
7. Build the three defense-candidate vectors from the side-selected Rules list in reverse order, applying only the exact owner-bit, tech-ceiling, positive-Anti, and `0x00505360` prerequisite tests over the owned-type inventory that excludes IsBaseDefense and WallTower. Select quadrant/category/type with the exact matching ratio/category pairing, tie/fallback rules, zero-draw singleton behavior, and inclusive one-to-total weighted RNG in preserved order.
8. Add the separate native strategy ownership. Add signed, snapshot/hash-covered `House+0x5634/+0x563C`-equivalent cadence state initialized to `{start: current_frame, duration: 0}`, then evaluate it with the exact start-`-1`, wrapping elapsed, and signed expiry rules. On expiry, require the native mode-sensitive not-current/not-player-control predicate and consume existing `HouseState.multiplay_passive == false`; otherwise do not invoke Strategy or consume any Strategy-owned draws. Add distinct signed, snapshot/hash-covered `House+0x5640/+0x5648` AI-hate timer state initialized to `{start: current_frame, duration: 0}`. Parse `[General] AIHateDelays` as a signed Hard/Normal/Easy vector with an empty native missing-key state and arm non-current/non-passive Houses during the post-map phase from the exact difficulty entry; do not invent a fallback for the direct-index consumer. On expiry do not re-arm it. When no enemy exists in a nonzero mode, reproduce the valid owner alternate/primary-cell gate, forward global-House scan, self/passive/defeated exclusions, owner-cell-reuse distance bug, strict first-peer tie, and `UpdateAngerNodes(+1)` call. Reuse/generalize Rust's existing sparse peer-score implementation only if it retains registered-peer validation, wrapping addition, `ScenarioSession::house_order`, strict positive greatest selection, and self/defeat/alliance filters. Add the exact signed-frame-multiple-of-100 `score > 1` decrement without recomputing `enemy_house`. When the current enemy is defeated, zero its whole matching score through the same update routine and then force `enemy_house = None` before superweapon targeting, even if recomputation briefly found another peer.

   After that AI-hate path, parse signed `[IQ] SuperWeapons` with native default `4` and compare it to existing `HouseState.current_iq` only in campaign; nonzero modes bypass the IQ comparison. Run the exact `AI_TryFireSW @ 0x005098F0` gate and a native-registration-ordered ready-Supers scan, adding explicit order state rather than iterating the current interned-ID `BTreeMap`. Parse every named signed AIIonCannon Hard/Normal/Easy score vector with the native empty missing-key state; do not invent Plug/Helipad/Temple defaults for stock-inactive type-flag branches. Parse and retain signed `AISuperDefenseProbability` by Hard/Normal/Easy difficulty with its native empty missing-key state, plus signed `AISuperDefenseFrames` and `AISuperDefenseDistance` with constructor defaults `25` and `10`, for ForceShield and reverse-House spy-reveal behavior. Port the enum dispatch and evidence-backed IronCurtain/ChronoSphere/ChronoWarp no-ops; explicit/best/team/GroundRallyPoint/GenMutator/PsyDom/ForceShield target paths; the complete `AI_FindBestRallyTarget` designated-owner/factory/cell/RTTI scoring table, per-object cloak draws, ordered greatest-score ties, and final tie draw; and reverse-House post-launch spy-reveal checks. Stock-active MultiMissile, PsychicDominator, and SpyPlane launch behavior must be exact before this path can close; do not swallow an unsupported launch or leave a native-fired Super ready. Targeting and the actual authoritative launch must complete synchronously so every nested Scenario draw precedes later planning.

   Then consume a signed, snapshot/hash-covered emergency state equivalent to `House+0x250`, initialized to zero, plus a persistent boolean latch equivalent to `House+0x249`, also initialized false. Implement the exact per-slot Storage-value rounding and final `ftol(storage * IncomeMult + Balance)` query, state-zero/state-one threshold hysteresis including the possible second query, and wrapping signed last-Building-attack-plus-900 state-three logic with its asymmetric equality. Connect trigger action 9 and Team opcode 30 as exact state-four writers, and trigger action 6 as a direct All-To-Hunt call; do not infer stock-skirmish activation for those data-absent script entries. State four must call Fire Sale then All-To-Hunt without clearing itself. Fire Sale must forward-scan the ordered owned-object vector, call `+0x1A0(1)` only for non-limbo positive-Health objects, and preserve Building Selling-mission guards. All-To-Hunt must reverse-scan native global Techno order; preserve the owner/marked/non-limbo gates, permanent-MC/Insignificant/non-human lethal C4 packet and skip, Foot Team removal plus queued Hunt without an invented Commence, Building occupant evacuation `(1,0)`, and unconditional final latch set. Apply that latch in target evaluation by following the current designated enemy and forcing other-owner candidates to score exactly one. In nonzero mode, construct priority four only when state is not three and the forward owned-object scan finds no active non-limbo Factory type; do not add a Health gate. Execute Fire Sale then All-To-Hunt at priority four before `AI_Check_Build_Need`, even when the state-four direct block already ran them in the same invocation.

   Only after that priority-four action may the complete `FUN_004F6540` support truth table and `AI_Check_Build_Need` resource counts, BaseUnit/BuildConst ownership, current-choice, mission-`0x12`, factory current-object RTTI/type, and strict/non-strict affordability paths run. Only result one in a nonzero mode may invoke Manage; `!has_active_building_queue` is never a substitute. Finally every actual Strategy invocation must make exactly one unconditional shared-Scenario `RandomRanged(1,7)` reschedule call, store current frame as the new start, and store `draw+105` as the 106..112 duration even when no Super fired and Manage did not run. The total Strategy draw count remains variable because the earlier targeting/launch path can consume zero or more. Preserve exact order `AI-hate acquisition/defeated cleanup -> AI_TryFireSW -> emergency state/direct actions -> no-factory priority actions -> AI_Check/Manage -> reschedule`; the current immutable `tick_ai` plus deferred command application cannot be reused without an equivalent staged transaction proving the same state/RNG order. This timer must coexist with, not replace or drive, item 9's eight-frame chooser scheduler.
9. Preserve `AI_THINK_INTERVAL_FRAMES=8` as the cadence of an independent native production-chooser scheduler, but replace its generic empty-building-queue body. Add signed, snapshot/hash-covered `House+0x1E4`-equivalent chooser mode initialized to zero. On exact signed frame remainder zero, repeat the native mode-sensitive AI and passive-House gates, then dispatch mode zero/campaign to Building, Unit, Aircraft, Infantry; mode one to Building with exact absent/unavailable fallback to the other three; and mode two to Unit, the HarvesterUnit-sensitive Aircraft/Infantry calls, and exact selected-type availability fallback to Building. Other modes call no chooser. The scheduler itself must not consume item 8's strategy-reschedule draw. Parse signed `[General] AIAlternateProductionCreditCutoff` with native default `1000`, and port the nonzero-mode/non-current exit-driven `AI_EconomyStateMachine` transitions, including the conditional `RandomRanged(0,1)` in mode two. Connect Manage's mode-two harvester write and its mode-one refinery/weapons write plus immediate Building chooser call as separate direct effects; do not defer that direct call to the next eight-frame tick.
10. Add an ordered, snapshot/hash-covered BasePlan-node representation with signed type/control value, packed site, filled latch, and signed retry counter. Populate it before any wildcard lookup: parse source-ordered scenario `PercentBuilt/NodeCount/%03d` nodes as section 7.6.2 specifies, deterministically initializing the native undefined semantic fields to false/zero; on ordinary AI ConstructionYard deploy or empty-plan recenter, run the complete `0x005054B0` algorithm in section 7.6.3, including exact Rules lists, eligibility, BuildConst/BuildPower seed behavior, BuildBarracks/BuildWeapons swaps, prerequisite-token topology/cycle break, refinery duplication, side/difficulty defense sentinels, and every shared Scenario RNG draw in order. Connect the active `0x004FDD10` refinery/weapons insertions with their literal candidate/funding gates, zero-index exit, capacity-two fallback, ordered 16-byte shifts, and node values; do not seed a missing plan from generic `decide_next_building`. In `AI_Choose_Building` order, consume the wall-percent RNG draw for every `-1` or zero-cell WallTower candidate even on fallthrough; on a passing reverse `ProtectWithWall` match, select the first side-matched ConcreteWalls type, reproduce the repeated-at-one-index perimeter order and partial-allocation semantics, reuse the existing exact BaseReservation corner-repair services, remove the saved original node by full value, and return before normal choice. After ordinary planning, apply the exact signed projected-power equation and BuildConst, positive-drain, blackout, and positive-output-deploying-building gates; choose the side plant with the Soviet advanced-prerequisite fallback, shift the complete tail, insert a zeroed power node immediately before the current node, and retain the current planned site byte-for-byte. Preserve native return `1` even if either vector growth path fails where section 7.6.5 says it does. Connect production planning to the caller-selected plan index and reproduce the `0x00507A58..0x00507B77` current/next-node site writes; do not add the coordinate to `ready_by_owner` or FactoryClass state. Implement explicit-type first-unsatisfied lookup, cached-site connectivity reuse/reselection, successful-Unlimbo fill/reset, ordinary failed-coordinate clearing, Building Limbo same-cell clearing/`IsBaseDefense` replacement conversion, and normalized-result-1 retry handling exactly as section 7.6 establishes. The wildcard lookup must scan from node zero, skip satisfied nodes, return the first unsatisfied node immediately in campaign, return an unfilled node immediately in nonzero modes, and call the exact `0x0050CAD0` policy only for an unsatisfied filled node. Ordinary selector exhaustion, wildcard recycling, failed-site clearing, Limbo invalidation, generated empty sites, and default primary/alternate cells must all fold to the same packed-zero `CellStruct`; an `Option` representation is acceptable only if every native boundary serializes/hashes that one bit pattern and cannot distinguish the semantic labels. Maintain signed ResourceGatherer/Destination counts at the existing spawn/despawn/owner-transfer chokepoints and a snapshot/hash-covered signed last-building-attack frame written at the native pre-receiver boundary; consume resolved primary identity, raw `power > 0`, eight-direction same-owner BuildingClass adjacency, signed wrapping deadline arithmetic, and existing House difficulty exactly. Successful Unlimbo fill must prefer the first exact owner/type/cell node; only if exact matching fails and the placed type has `undeploys_into` may it fill the first same-type node with `filled == 0`, regardless of cell, and both paths reset retries. Retry handling must increment first, never evict in campaign, and in nonzero modes remove the exact ordered node only when the new signed count is strictly greater than the signed rule.
11. Implement the non-naval `0x005060B0` selector over `BaseReservationState.perimeter_cells` with the exact callbacks, the linked `0x007C8B48` signed-key sort and tie permutation, orientation, tables, phases, predicate order, duplicate traversal, and `(0,0)` failure. Its connectivity predicate must return true before scanning when `!sim.session.game_mode_nonzero`; every nonzero mode scans the same-House reservation bits.
12. Keep the shared final Building placement predicate authoritative; expose the precise selector-stage `CanPlaceAt` call without deleting or conflating ExitObject's downstream final placement dispatch.
13. First close the narrow shared `Find_Nearby_Passable_Cell` projection prerequisite: when the candidate has `BRIDGE_FLAG_FORWARD_SIDE`, add four levels for every projected probe carrying `BRIDGE_FLAG_STRUCTURAL`, in both collection early-stop classification and final pool partition. Then implement the active naval branch with the exact query matrix in section 8.2 and parsed `Shipyard`, `BuildConst`, and `AINavalYardAdjacency` inputs. Its shipyard lookup must scan source order with only the exact `0x005051E0` Owner, RequiredHouses, ForbiddenHouses, AIBasePlanningSide, and superweapon-option tail above; do not route it through generic build-option eligibility. If any service/key is absent, keep naval open rather than approximating it.
14. Remove or quarantine the current radius-12 spiral from the ordinary AI parity path after native BasePlan-owned placement is connected.

Do not merge `0x005082C0` wall/base-perimeter scan into this helper, and do not implement the stock-inactive CloakGenerator branch.

### 11.2 Minimum acceptance tests

- `gsi_04_05_ordinary_site_sorts_perimeter_by_influence_then_angle`
- `gsi_04_05_building_exit_site_uses_chebyshev_sort_key`
- `gsi_04_05_native_site_sort_reproduces_short_equal_key_permutations`
- `gsi_04_05_native_site_sort_reproduces_1001_record_key_collision`
- `gsi_04_05_trigger_137_decodes_p_nz_aa_and_sets_only_alternate_base_cell`
- `gsi_04_05_alternate_base_cell_defaults_packed_zero_and_is_snapshot_hash_covered`
- `gsi_04_05_symmetric_reservation_neighbors_cancel_and_skip_candidate`
- `gsi_04_05_recenter_base_uses_last_live_owned_buildconst_and_preserves_launch_center`
- `gsi_04_05_site_probe_is_two_phase_three_cell_tangential_sweep`
- `gsi_04_05_site_predicates_run_occupancy_place_height_connectivity_order`
- `gsi_04_05_height_delta_two_passes_but_three_fails`
- `gsi_04_05_failed_ordinary_site_equals_native_packed_zero_empty_sentinel`
- `gsi_04_05_duplicate_full_traversal_is_preserved`
- `gsi_04_05_influence_falloff_uses_half_open_radius_six_and_x87_chop`
- `gsi_04_05_enemy_ratio_refresh_uses_tracked_values_fudge_and_three_rng_draws`
- `gsi_04_05_no_designated_enemy_restores_exact_point33_ratio_bits_without_rng`
- `gsi_04_05_country_cost_mults_default_exact_one_and_parse_all_five_categories`
- `gsi_04_05_factory_plant_fields_default_exact_one_and_stock_naindp_units_point75`
- `gsi_04_05_factory_plant_recompute_uses_vector_order_and_f32_round_after_each_entry`
- `gsi_04_05_type_value_multiplies_factory_then_country_in_x87_before_native_ftol`
- `gsi_04_05_tracked_value_removal_recomputes_current_multiplier_without_retroactive_recount`
- `gsi_04_05_deficit_mapping_pairs_air_armor_infantry_ratios_with_matching_anti_values`
- `gsi_04_05_category_ties_prefer_infantry_then_armor_then_air`
- `gsi_04_05_category_empty_fallback_prefers_armor_then_infantry_then_air`
- `gsi_04_05_side_defense_lists_iterate_in_reverse_and_side_two_plus_uses_third`
- `gsi_04_05_defense_candidates_filter_owner_tech_ceiling_positive_anti_and_prerequisites_only`
- `gsi_04_05_prerequisite_inventory_excludes_is_base_defense_and_wall_tower`
- `gsi_04_05_single_defense_candidate_consumes_no_type_selection_rng`
- `gsi_04_05_weighted_defense_draw_is_one_to_total_inclusive_in_preserved_order`
- `gsi_04_05_scenario_nodecount_parses_percent_and_numbered_nodes_in_source_order`
- `gsi_04_05_scenario_negative_control_and_signed_i16_cell_parse_are_literal`
- `gsi_04_05_scenario_nodes_normalize_undefined_filled_retry_to_false_zero`
- `gsi_04_05_base_plan_checksum_folds_count_type_x_y_but_not_filled_retry`
- `gsi_04_05_ai_mcv_deploy_populates_plan_before_first_wildcard_lookup`
- `gsi_04_05_recalc_filters_global_types_in_registration_order`
- `gsi_04_05_recalc_buildpower_seed_does_not_mark_eligible_type_selected`
- `gsi_04_05_recalc_prerequisite_tokens_map_power_weapons_barracks_radar_tech_refinery`
- `gsi_04_05_recalc_cycle_break_uses_last_unselected_candidate`
- `gsi_04_05_recalc_refinery_duplicates_consume_scenario_rng_and_shift_each_later_range`
- `gsi_04_05_recalc_side_defense_sentinels_consume_scenario_rng_and_preserve_final_order`
- `gsi_04_05_generated_nodes_start_zero_cell_unfilled_zero_retry`
- `gsi_04_05_strategy_timer_is_signed_snapshot_hash_state_and_blocks_before_expiry`
- `gsi_04_05_strategy_timer_initializes_current_frame_zero_duration_and_expires_immediately`
- `gsi_04_05_strategy_expiry_requires_mode_exact_ai_and_nonpassive_house`
- `gsi_04_05_ai_hate_delays_parse_hard_normal_easy_and_missing_key_stays_empty`
- `gsi_04_05_ai_hate_timer_initializes_immediate_then_post_map_arms_noncurrent_nonpassive_house`
- `gsi_04_05_ai_hate_timer_blocks_acquisition_before_expiry_and_is_not_rearmed_after_expiry`
- `gsi_04_05_ai_hate_acquisition_reuses_owner_cell_and_touches_first_eligible_global_order_peer`
- `gsi_04_05_ai_hate_acquisition_skips_self_passive_defeated_and_requires_nonzero_mode_valid_owner_base_cell`
- `gsi_04_05_update_anger_nodes_wraps_existing_score_and_selects_strict_first_positive_nonallied_peer`
- `gsi_04_05_anger_scores_above_one_decay_only_on_exact_signed_frame_multiple_of_one_hundred_without_reselect`
- `gsi_04_05_defeated_enemy_score_is_zeroed_then_forced_none_before_try_fire_sw`
- `gsi_04_05_emergency_state_and_all_to_hunt_latch_initialize_zero_and_are_snapshot_hash_covered`
- `gsi_04_05_available_wallet_rounds_each_storage_slot_then_applies_income_mult_and_balance_ftol`
- `gsi_04_05_state_zero_below_twenty_five_enters_one_and_can_query_again_same_strategy_call`
- `gsi_04_05_state_one_at_least_twenty_five_clears_but_lower_value_retains_one`
- `gsi_04_05_state_three_attack_plus_nine_hundred_before_equal_after_boundaries_are_asymmetric`
- `gsi_04_05_trigger_action_nine_and_team_opcode_thirty_write_state_four_but_action_six_calls_hunt_directly`
- `gsi_04_05_fire_sale_forward_scans_live_positive_health_owned_objects_and_preserves_sell_guards`
- `gsi_04_05_all_to_hunt_reverse_scans_marked_nonlimbo_owned_technos`
- `gsi_04_05_all_to_hunt_permanent_mc_insignificant_nonhuman_branch_uses_exact_c4_damage_packet`
- `gsi_04_05_all_to_hunt_removes_foot_team_then_queues_hunt_without_commence`
- `gsi_04_05_all_to_hunt_evacuates_garrison_with_one_zero_and_sets_latch_even_when_array_empty`
- `gsi_04_05_all_to_hunt_latch_follows_current_enemy_and_forces_other_owner_candidate_score_one`
- `gsi_04_05_no_factory_priority_requires_nonzero_mode_state_not_three_and_exact_active_nonlimbo_factory_scan`
- `gsi_04_05_state_four_without_factory_runs_fire_sale_and_all_to_hunt_twice_in_one_strategy_call`
- `gsi_04_05_strategy_without_ready_super_consumes_only_unconditional_reschedule_draw`
- `gsi_04_05_strategy_orders_ai_hate_try_fire_sw_emergency_priority_ai_check_manage_then_reschedule`
- `gsi_04_05_iq_superweapons_defaults_and_parses_signed_four_campaign_gate_only`
- `gsi_04_05_try_fire_sw_scans_ready_supers_in_stored_order`
- `gsi_04_05_try_fire_sw_iron_curtain_chronosphere_and_chronowarp_are_native_noops`
- `gsi_04_05_default_nuke_and_lightning_use_best_rally_target_when_enemy_designated`
- `gsi_04_05_ai_ion_target_vectors_parse_hard_normal_easy_and_missing_keys_stay_empty`
- `gsi_04_05_ai_super_defense_probability_stays_empty_without_key_and_scalars_default_twenty_five_ten`
- `gsi_04_05_ai_super_defense_probability_frames_and_distance_parse_retail_ninety_fifty_ten_fifty_twelve`
- `gsi_04_05_stock_has_no_active_plug_temple_or_hoverpad_target_score_branch`
- `gsi_04_05_best_rally_target_scores_designated_enemy_by_exact_rtti_table`
- `gsi_04_05_best_rally_target_cloak_draws_follow_global_techno_order_before_admission`
- `gsi_04_05_best_rally_singleton_tie_range_call_consumes_no_raw_rng_word`
- `gsi_04_05_best_rally_multiple_ties_draw_zero_to_count_minus_one_before_reschedule`
- `gsi_04_05_ai_superweapon_launch_and_reverse_house_spy_reveal_draw_before_reschedule`
- `gsi_04_05_ai_superweapon_strategy_matches_final_scenario_rng_state_end_to_end`
- `gsi_04_05_stock_ai_multimissile_psydom_and_spyplane_launch_and_clear_ready_exactly`
- `gsi_04_05_eight_frame_chooser_scheduler_is_independent_of_strategy_timer`
- `gsi_04_05_chooser_tick_repeats_mode_exact_ai_and_nonpassive_gates`
- `gsi_04_05_chooser_tick_does_not_consume_strategy_reschedule_rng`
- `gsi_04_05_campaign_or_chooser_mode_zero_calls_building_unit_aircraft_infantry_in_order`
- `gsi_04_05_chooser_mode_one_building_absent_or_unavailable_falls_back_to_other_three`
- `gsi_04_05_chooser_mode_two_unit_first_harvester_match_and_building_fallback_are_exact`
- `gsi_04_05_chooser_mode_defaults_zero_and_is_snapshot_hash_covered`
- `gsi_04_05_ai_alternate_production_credit_cutoff_defaults_and_parses_signed_one_thousand`
- `gsi_04_05_exit_economy_state_machine_transitions_modes_zero_one_two_and_loaded_three`
- `gsi_04_05_economy_mode_two_rng_occurs_only_with_barracks_weapons_and_sufficient_power`
- `gsi_04_05_ai_check_support_truth_table_covers_resource_counts_base_refinery_harvester_and_credits`
- `gsi_04_05_ai_check_cached_building_factory_object_identity_and_affordability_are_exact`
- `gsi_04_05_ai_check_destination_mission_and_cached_harvester_factory_gates_are_exact`
- `gsi_04_05_empty_building_queue_alone_never_triggers_manage`
- `gsi_04_05_manage_inserts_refinery_or_weapons_before_returned_nonzero_index`
- `gsi_04_05_manage_wildcard_index_zero_exits_without_choose_or_insert`
- `gsi_04_05_manage_minus_one_requires_capacity_two_and_substitutes_index_one`
- `gsi_04_05_manage_harvester_writes_mode_two_without_immediate_building_choose`
- `gsi_04_05_manage_refinery_or_weapons_writes_mode_one_and_chooses_building_immediately`
- `gsi_04_05_wall_override_fallthrough_still_consumes_one_scenario_rng_draw`
- `gsi_04_05_wall_override_uses_first_side_matched_concrete_wall_and_reverse_protected_source`
- `gsi_04_05_wall_expansion_fixed_index_stores_reverse_perimeter_order_and_removes_saved_sentinel`
- `gsi_04_05_wall_expansion_allocation_failure_keeps_prior_inserts_without_rollback`
- `gsi_04_05_zero_cell_walltower_choose_failure_removes_tower_then_successor`
- `gsi_04_05_power_deficit_selects_allied_soviet_or_third_side_plant_before_current_node`
- `gsi_04_05_power_splice_retains_shifted_current_node_type_site_filled_and_retry`
- `gsi_04_05_soviet_advanced_power_requires_prerequisites_else_falls_back_regular`
- `gsi_04_05_power_splice_skips_buildconst_nonpositive_drain_blackout_and_deploying_power_latch`
- `gsi_04_05_power_splice_growth_failure_returns_one_without_mutating_plan`
- `gsi_04_05_inserted_plan_type_and_index_drive_production_time_site_capture`
- `gsi_04_05_empty_plan_has_no_generic_decide_next_building_seed_fallback`
- `gsi_04_05_planned_cell_is_captured_before_building_becomes_ready`
- `gsi_04_05_planned_cell_lives_in_base_plan_node_not_ready_queue`
- `gsi_04_05_base_plan_nodes_are_ordered_snapshot_and_hash_state`
- `gsi_04_05_building_exit_skips_satisfied_same_type_node`
- `gsi_04_05_cached_site_reuses_only_after_native_connectivity_gate`
- `gsi_04_05_cached_site_connectivity_failure_reselects_and_overwrites_node`
- `gsi_04_05_zero_site_building_exit_selects_and_records_node_site`
- `gsi_04_05_successful_ai_unlimbo_sets_filled_and_resets_retry_count`
- `gsi_04_05_unlimbo_exact_type_cell_match_precedes_undeploy_fallback`
- `gsi_04_05_undeploy_fallback_skips_filled_nodes_and_uses_first_unfilled_same_type`
- `gsi_04_05_undeploy_fallback_ignores_cell_and_resets_retry_count`
- `gsi_04_05_wildcard_campaign_returns_first_unsatisfied_node_without_recycle_helper`
- `gsi_04_05_wildcard_nonzero_returns_unfilled_before_testing_filled_nodes`
- `gsi_04_05_wildcard_recycle_false_continues_in_source_order`
- `gsi_04_05_wildcard_recycle_limbo_and_default_base_cells_share_packed_zero_bits`
- `gsi_04_05_slave_miner_limits_parse_hard_normal_easy_retail_four_three_two`
- `gsi_04_05_resource_counts_follow_spawn_despawn_and_owner_transfer_signed_lifecycle`
- `gsi_04_05_filled_yarefn_recycles_only_below_difficulty_slave_miner_limit`
- `gsi_04_05_resolved_primary_weapon_accepts_before_wall_and_power_gates`
- `gsi_04_05_wall_recycling_requires_eight_direction_same_owner_building_adjacency`
- `gsi_04_05_wall_with_positive_power_does_not_bypass_failed_adjacency`
- `gsi_04_05_nonwall_positive_power_recycles_immediately_but_negative_power_does_not`
- `gsi_04_05_ai_restrict_replace_defaults_signed_five_hundred_and_parses_retail_four_hundred`
- `gsi_04_05_noncloaked_building_attack_stamps_house_frame_before_receiver_outcome`
- `gsi_04_05_null_attacker_or_cloaked_building_does_not_stamp_house_frame`
- `gsi_04_05_initial_zero_attack_frame_restricts_ordinary_recycling_until_retail_frame_four_hundred`
- `gsi_04_05_ordinary_filled_node_recycles_on_signed_wrapping_deadline_setle`
- `gsi_04_05_maximum_building_placement_failures_defaults_five_and_parses_signed_retail_three`
- `gsi_04_05_result_one_increments_before_threshold_comparison`
- `gsi_04_05_skirmish_retry_three_retains_node_but_four_removes_ordered_entry`
- `gsi_04_05_campaign_result_one_increments_but_never_evicts_node`
- `gsi_04_05_negative_maximum_evicts_on_first_nonzero_mode_result_one`
- `gsi_04_05_failed_final_ordinary_placement_clears_all_matching_node_sites`
- `gsi_04_05_limbo_invalidates_other_nodes_sharing_the_removed_building_cell`
- `gsi_04_05_skirmish_base_defense_limbo_keeps_filled_retry_but_writes_minus_one_packed_zero_cell`
- `gsi_04_05_campaign_or_nondefense_limbo_keeps_matched_node_type_and_cell`
- `gsi_04_05_campaign_mode_bypasses_reservation_connectivity_but_skirmish_does_not`
- `gsi_04_05_connectivity_reads_same_house_bit_and_shared_dummy`
- `gsi_04_05_shipyard_selector_uses_source_order_country_masks_and_exact_side`
- `gsi_04_05_shipyard_selector_superweapon_tail_matches_buildtech_and_disableable_from_shell`
- `gsi_04_05_shipyard_selector_ignores_generic_tech_prerequisite_factory_limit_cost_and_credit_gates`
- `gsi_04_05_stock_allied_soviet_yuri_shipyards_all_supply_six_by_six_search_footprint`
- `gsi_04_05_naval_fnpc_uses_float_no_required_zone_normal_movement_and_literal_zero_gates`
- `gsi_04_05_fnpc_forward_side_candidate_adds_four_levels_for_structural_bridge_probes`
- `gsi_04_05_fnpc_structural_probe_bit_is_ignored_without_candidate_forward_side`
- `gsi_04_05_bridge_projection_changes_collection_stop_and_frame_modulo_pool_exactly`
- `gsi_04_05_naval_zero_reference_uses_current_frame_modulo_not_nearest_origin`
- `gsi_04_05_naval_site_uses_shipyard_footprint_and_buildconst_adjacency_cap`
- `gsi_04_05_stock_rules_do_not_activate_cloak_generator_legacy_sort`

## 12. Negative facts and unresolved exactness gates

- **Not a fixed spiral.** No radius-12 square-ring traversal exists in the verified native path.
- **Not universally distance-sorted.** Standard AI uses influence+angle; Building exit uses Chebyshev distance; only the stock-inactive CloakGenerator branch uses 3D distance.
- **Not a radial three-step push.** The three probes are tangential.
- **Not a bridge retry.** The two per-candidate phases change inward offset/clearance; the later whole traversal is an explicit duplicate with no visible parameter change.
- **Not two cell-sentinel states.** Ordinary exhaustion's literal `(0,0)`, the named invalid/empty global `0x00A8EF98`, BaseClass zero global `0x0089C310`, Limbo clearing, and default primary/alternate cells are bit-identical packed zero. Rust must not snapshot or hash separate “zero” and “invalid” BasePlan cell variants.
- **Not safe to implement with a zero/neutral grid.** The active caller computes a category-specific grid during production choice.
- **Not a ready-queue coordinate.** Native stores and later resolves the site through the ordered House BasePlan vector; node satisfaction, connectivity-driven reselection, Unlimbo fill state, retry count, and failure clearing all affect reuse.
- **Not seeded by generic next-building priority.** Fresh non-human skirmish ConstructionYard deployment populates an empty BasePlan through the complete `0x005054B0` Rules/topology/RNG algorithm before wildcard lookup. `AI_Manage_Build_Queue` cannot bootstrap zero capacity, and Rust `decide_next_building` is not a native fallback.
- **Not safe to copy scenario stack garbage.** `0x0042EBE0` leaves node filled/retry locals undefined, while the scenario writer and checksum omit them and active campaign decisions either ignore or overwrite them. Deterministic false/zero is the evidence-backed semantic translation; random host memory would introduce a non-native architecture and nondeterminism.
- **Not a stable topological sort.** BuildPower is seeded without marking its eligible slot, Barracks and Weapons reorder eligible slots, `GAPLUG` skips the normal success branch, and a stalled pass appends the last unselected candidate. These details change duplicates and all later RNG insertion indices.
- **Not an index-zero insertion.** The active Manage path treats wildcard return zero as an exit, even though zero is a valid BasePlan index; only a positive index or the capacity-gated `-1 -> 1` fallback reaches its refinery/weapons splice.
- **Not one production scheduler.** Native retains a signed frame-modulo-eight chooser block, but its body is `House+0x1E4` mode dispatch rather than Rust's empty-building-queue decision. Manage is independently owned by the signed 106..112-frame strategy timer and complete `AI_Check_Build_Need` support/factory/choice/mission/affordability gate. An eight-frame tick does not consume the strategy-reschedule draw; every actual Strategy call does, including calls that do not mutate BasePlan.
- **Not nearest-enemy acquisition.** The expired AI-hate path computes every candidate distance from the owner's selected base cell back to that same owner cell. Strict-less therefore touches the first eligible global-order peer, not the geometrically nearest House.
- **Not supplied by damage feedback alone.** Native constructor registration gives every House an ordered zero-score peer node, the post-map AI-hate timer can add `+1` without any attack, and Strategy cleanup can zero a defeated peer. Rust's existing damage-only caller does not provide that lifecycle by itself.
- **Not immediate fallback to the next anger winner after defeat.** Cleanup asks `UpdateAngerNodes` to recompute after zeroing the defeated peer, then forcibly writes enemy `-1`; `AI_TryFireSW` sees no enemy in that Strategy call even if another peer has a positive score.
- **Not reselection on periodic decay.** Exact-frame-100 decay subtracts one only from scores greater than one and leaves the current enemy identity unchanged until another update/acquisition path recomputes it.
- **Not exactly one total Strategy RNG draw.** Every Strategy call makes one unconditional `RandomRanged(1,7)` reschedule call, but after the no-RNG AI-hate acquisition/cleanup the `AI_TryFireSW` target/launch/spy-reveal path may consume a variable number from the same Scenario stream. `AI_FindBestRallyTarget` can draw once per qualifying cloaked global Techno plus a final tie-range call, and successful fire can add launch and reverse-House spy-reveal draws.
- **Not safe to defer AI superweapon commands until after rescheduling.** Native completes AI-hate acquisition/cleanup, `AI_TryFireSW`, and synchronous `Fire_SW` side effects before `AI_Check/Manage` and the unconditional reschedule. Returning `LaunchSuperWeapon` from immutable `tick_ai` and applying it later reverses shared-RNG order even when the chosen target happens to match.
- **Not a raw-credit emergency threshold.** The `<25`/`>=25` state transitions use four-slot Storage value with per-slot `ftol`, HouseType `IncomeMult`, Balance, and a final `ftol`; a zero-to-one transition can immediately cause a second query in the same Strategy call.
- **Not a symmetric 900-frame boundary.** Existing state three survives `last_attack+900 == current_frame`, while a different state is armed only when the deadline is strictly greater. Use wrapping signed arithmetic.
- **Not campaign-only abandonment behavior.** Action 9/state four, action 6, and opcode 30 are scripted campaign entries or stock-data absent, but the same Fire Sale/All-To-Hunt callbacks are ordinary-skirmish active through nonzero-mode priority four whenever no qualifying Factory exists and state is not three.
- **Not one callback pair per Strategy invocation.** State four leaves itself set and runs Fire Sale then All-To-Hunt before priority construction; if the no-factory priority also qualifies, the same pair runs again before `AI_Check/Manage`.
- **Not a Health-qualified Factory gate.** The suppressing Factory scan requires non-null, active, non-limbo, and `Factory != 0`; it does not test Health.
- **Not one generic sell command.** Fire Sale calls the polymorphic slot across every non-limbo positive-Health owned object; ordinary Buildings queue/Commence Selling only through their own guards, while All-To-Hunt's `SellBuilding(1,0)` is garrison evacuation rather than the Building's final sale.
- **Not immediate Hunt Commence.** The Foot path removes Team membership and queues Mission 15 with zero; it does not explicitly Commence, and Aircraft can reject it under their native protected-state override.
- **Not a one-shot or hard target filter.** `House+0x249` is never cleared by the callback, follows later `enemy_house` changes, does nothing while no enemy is designated, and forces other-owner candidate scores to one rather than rejecting them.
- **Not a queue-derived chooser mode.** `House+0x1E4` starts at zero, is written to two or one by exact Manage branches, and changes through the nonzero-mode exit-driven economy state machine using `AIAlternateProductionCreditCutoff`. The refinery/weapons Manage path calls Building immediately after writing mode one; waiting for the next eight-frame tick changes behavior.
- **Not RNG only on a successful wall override.** Every selected `-1` or zero-cell WallTower node consumes the inclusive `0..99` Scenario draw before the percent/helper decision. A failed percent or missing protected source still advances the shared stream.
- **Not append-order wall geometry.** Every perimeter node is inserted at the same position after the reverse-selected protected source, so the final vector order is the reverse of the top/bottom then left/right attempt order. Partial growth failure does not roll back earlier nodes.
- **Not single removal for every wall fallthrough failure.** Failed `AI_ChooseNextProduction` removes one `-1` sentinel, but a zero-cell WallTower path removes the tower and then the successor now at that index when present.
- **Not a generic low-power enqueue.** The projected-power splice uses signed cached output/drain and zero-default AICostTolerance, excludes BuildConst and nonpositive drain, respects exact blackout state and the literal positive-output `TechnoClass__IsDeploying` latch, then inserts before and preserves the already planned node. `House+0x577B` is not an offensive-unit predicate.
- **Not closure of wall execution.** The wall-node writer and reservation-corner repair in `AI_Choose_Building` are active BasePlan population, but the separate `-3 -> HouseClass__AI_ScanBasePerimeter @ 0x005082C0` consumer remains open.
- **Not finished after Unlimbo.** Building Limbo clears every other same-cell node to packed zero and, for an `IsBaseDefense` type in a nonzero mode, converts the exact matched node to `type=-1` plus the same packed-zero cell without clearing its filled/retry fields. This is the active bridge back to wildcard defense replacement.
- **Not exact-cell-only fill.** Exact owner/type/cell matching has priority, but an active stock `UndeploysInto` type falls back to the first same-type node whose filled byte is zero and resets that node's retry counter even when its cached cell differs.
- **Not generic build eligibility for filled nodes.** Wildcard recycling has a dedicated ordered policy: resource cap, resolved primary, wall adjacency, positive power, then the damage-restriction timer. It does not consult credits, TechLevel, prerequisites, factories, BuildLimit, owner masks, or ordinary `BuildOption.enabled`.
- **Not a distinct invalid-cell encoding.** An eligible filled wildcard node is reset to packed `(0,0)` before reuse, exactly the same bits Limbo and native empty/invalid globals use.
- **Not a universal campaign call to `0x0050CAD0`.** The sole caller returns the first unsatisfied campaign node before the helper; only unsatisfied filled nodes in nonzero modes reach it.
- **Not `Power != 0`.** Only positive native `PowerOutput` passes. Negative `Power=` becomes drain with zero output, and a wall type never reaches the power test after failed adjacency.
- **Not a hostile-or-damaging-hit timestamp.** `BuildingClass__ReceiveDamage` stamps the House frame for any non-null attacker when the victim's virtual `+0x80` is false, before immunity and generic damage resolution, with no alliance or positive-damage predicate.
- **Not inactive before the first attack.** The constructor's zero timestamp plus stock `AIRestrictReplaceTime=400` restricts remaining ordinary filled nodes through frame 399 even when no attack occurred.
- **No native `AISlaveMinerNumber` fallback exists.** The constructor vector is empty and a missing key produces an empty vector; active retail supplies the required Hard/Normal/Easy entries. Do not silently describe `[4,3,2]` as a constructor default.
- **Not `retry >= maximum`.** Native increments first and retains the node through equality; retail `MaximumBuildingPlacementFailures=3` evicts on the fourth normalized result-1 failure, and campaign does not run the eviction comparison at all.
- **Not safe to freeze all ratios at one third.** A designated enemy activates tracked-value ratios, difficulty-scaled noise, and three shared RNG draws.
- **Tracked value is not raw cost.** `0x00711F00` applies the owner's ordered FactoryPlant factor first, then the owner's country category factor, before native ftol. Stock `NAINDP` makes the vehicle factor active.
- **Tracked counters are not retroactively normalized.** A FactoryPlant multiplier change affects later add/remove events only; removal recomputes current value rather than subtracting a stored historical contribution.
- **Not the generic enabled build-option set.** Native defense vectors come only from the side-specific BaseDefenses list in reverse order and apply the four literal filters above; generic credit, factory, BuildLimit, stolen-tech, Required/ForbiddenHouses, and negative-TechLevel rejection would change candidates and RNG.
- **The naval selector is also not the generic enabled build-option set.** It scans `Shipyard=` forward and applies country Owner/Required/Forbidden masks, exact AIBasePlanningSide, and its primary-superweapon shell-option tail, but no TechLevel, prerequisites, factory, BuildLimit, cost, credits, or stolen-tech gate. Stock's three yards all have equal foundations; that coincidence does not authorize selecting an arbitrary type.
- **The naval query does not use MovementZone `-1`.** It uses required-zone ID `-1` and MovementZone `0` (Normal), with Float speed type `5`.
- **The current shared nearby-cell helper is not yet exact for this caller.** Naval allows bridge cells and clears the bridge-aware shortcut, making the omitted forward-side/structural-probe four-level projection correction reachable. Do not bypass it, classify solely from raw terrain levels, or describe it as a bridge-destruction dependency.
- **Not all modes require connectivity.** Native mode zero is campaign/single-player and returns true before the scan; all nonzero modes, including ordinary skirmish mode five, scan. Rust already owns this exact distinction in snapshot/hash-covered `ScenarioSession.game_mode_nonzero`, so adding another mode field or always scanning is wrong.
- **Not permanently unset.** `House+0x5494` starts packed zero in stock skirmish but action 137 sets it in three shipped YR campaign maps.
- **Not the Rust library sort.** Equal placement keys are possible algebraically and retail's linked qsort is non-stable. The exact `0x007C8B48` algorithm and permutation fixtures above are required, so the former qsort exactness gate is closed for implementation.
- **The complete `0x005082C0` consumer remains open.** Nothing here closes wall/base-perimeter scan behavior.

## 13. Superseded documentation claims

`docs/research/FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md` section 3.6 is stale for `0x005060B0` and must not be used as an implementation contract. Replace these claims:

- universal “distance * 1000 + index” ordering;
- “8-direction probe + 3-step push along the same direction”;
- second traversal described as a bridge retry;
- general failure described as a bit-distinct invalid cell.

with the callback-specific sort formulas, neighbor vector-sum seed, two-phase tangential sweep, literal duplicate traversal, and the bit-identical packed-zero empty/invalid ordinary failure established above.

Older documents naming `Rules+0xE0C` `MaxBaseDistance` are also superseded for this path; the active parser binding is `AINavalYardAdjacency`.

`docs/research/HOUSECLASS_AI_PRODUCTION_TAIL_FORMULAS_GHIDRA_REPORT.md` and the derived core-services prose use the stale name `AI_DispatchProduction` for `0x005098F0` and classify its target/dispatch body as out of scope. Live caller/body/type-table evidence identifies it as `HouseClass__AI_TryFireSW`; because it and synchronous `Fire_SW` run before the Strategy reschedule and share Scenario RNG with later BasePlan choices, that old exclusion is superseded for this placement-parity path.

## 14. Evidence log and confidence

| Evidence | Result | Confidence |
|---|---|---|
| live decompile and assembly of `0x005060B0` | complete branch/candidate/predicate/failure structure | HIGH |
| static packed-cell initializers `0x004F50A0..0x004F50AE`, `0x0042E5D0..0x0042E5DE`, `0x005618B0..0x005618BE`; live zero bytes at `0x00A8EF98`, `0x0089C310`, `0x00ABD480`; failure/clearing stores in `0x005060B0`, wildcard lookup, Limbo, and House constructor | named invalid/empty globals and literal `(0,0)` are one bit-identical packed-zero `CellStruct` state across selector, BasePlan, FNPC reference, and base-cell lifecycle | HIGH |
| direct xrefs/caller decompiles `0x00506EF0`, `0x00443C60`, `0x004FE3E0` | caller ownership, callback parameters, alternate-vector and `-3` exclusions | HIGH |
| live decompile/assembly `0x0050C210`; ConstructionYard deploy writes `0x007398DE..0x007398F3` | exact base-plan-center writer and separation from primary/launch center | HIGH |
| constructor store inventory; live `0x0050DFE0`, `0x006E44E0`, `0x006DD5B0`, `0x006DD8B0`, `0x00763690`; retail map census | alternate-cell lifecycle, action-137 writer, waypoint conversion, and exact stock activation | HIGH |
| inline callback bytes at `0x00505F80`; decompile/assembly `0x00505FD0` | exact Chebyshev and influence/angular keys | HIGH |
| live decompile/assembly `0x007C8B48`, `0x007C8C9C`, `0x007C8CEA`; branch-exact permutation fixtures | linked qsort algorithm, equality behavior, and exact tie ordering | HIGH |
| live assembly `0x00506D50..0x00506EE3` | exact influence falloff omitted by decompiler | HIGH |
| live decompile `0x0050B760`; `Main_Game @ 0x0052D9A0` mode writers; Rust `scenario_session.rs`, `loading/init.rs`, snapshot identity tests | exact campaign shortcut, nonzero-mode scan, and existing snapshot/hash-covered Rust authority | HIGH |
| BuildingType vtable `+0xA8 -> 0x00464AC0`; delegate `0x00716150` | placement predicate identity and PlaceAnywhere shortcut | HIGH |
| Building value virtuals `0x00459870/80/90` | exact AntiAir/AntiArmor/AntiInfantry offsets | HIGH |
| live decompile/assembly `0x00508150`; Rules `+0x9A8/+0x9AC`; House writers `0x00502A80/0x005025F0` | exact dynamic ratio inputs, difficulty-scaled RNG, defaults, and tracked-value ownership | HIGH |
| live decompile/assembly `0x00711EB0/0x00711F00`, `0x0050BDF0/0x0050BEB0/0x0050BF60`; House constructor `0x004F54A0`; Building Unlimbo/ChangeOwner and House pointer-expiry call sites; live HouseType/BuildingType INI readers; both retail rules files | raw-cost source, five-way category map, float/ftol order, ordered FactoryPlant lifecycle, exact defaults/parsers, and stock NAINDP activation | HIGH |
| live decompile/assembly `0x00507B80/0x00507D70/0x00507F60`, `0x00505360`, and `0x00506EF0`; live `RulesClass__ReadAI @ 0x00672AE0` | side-list selection, reverse/filter order, prerequisite inventory, and exact weighted type RNG | HIGH |
| live decompile/assembly `0x004FE3E0`, `0x0042EB20`, `0x0042EB50`, `0x0042E780`, `0x0042E820`, `0x0042F260`, `0x0042F380`, `0x00506EF0`, `0x00443C60`, and `BuildingClass::Unlimbo @ 0x00440580`; retail `UndeploysInto` assignments | exact BasePlan node layout/lookup, current-and-next-node site writes, cached-site reuse/reselection, exact/fallback filled-state priority, retry state, and final-failure site clearing | HIGH |
| live constructor/caller/parser/writer/checksum evidence `0x0042E6F0`, `0x004F54A0`, `HouseClass__Read_Scenario_INI @ 0x00500B40`, `0x0042EBE0`, `0x0042ED60`, `0x0042F180`; extracted shipped map sections | empty default plan, ordered `PercentBuilt/NodeCount/%03d` parse/write, native undefined trailing fields, deterministic semantic normalization boundary, checksum exclusions, and stock campaign activation | HIGH |
| live decompile/assembly `FUN_00505180 @ 0x00505180`, `UnitClass__Deploy @ 0x007393C0`, `HouseClass__RecenterBase @ 0x0050C210`, `HouseClass__AI_RecalcBuildOptions @ 0x005054B0`, `FUN_00505360`; Rules reader `BuildRadar` binding; retail Rules lists/count vectors | fresh-skirmish population authority, trigger-owned Recenter separation, exact eligibility and priority topology, prerequisite tokens, refinery/defense insertion counts, shared Scenario RNG ownership, node order and zero initialization | HIGH |
| House constructor strategy-timer stores `0x004F5B9D..0x004F5BA8`; live decompile/assembly `HouseClass::Update @ 0x004F8440` strategy-timer block `0x004F8FBE..0x004F9032`, `HouseClass__AI_Building_Strategy @ 0x004FD500`, `HouseClass__AI_Check_Build_Need @ 0x004FD9A0`, `FUN_004F6540`, `HouseClass__AI_Manage_Build_Queue @ 0x004FDD10`, insertion blocks `0x004FE163..0x004FE223/0x004FE2E7..0x004FE3A6`; `HouseClass__ComputerTakeover @ 0x0050A5C0` | immediate-expiry initialization, signed AI/non-passive cadence, exact AI-hate/SW/planning/reschedule order, one unconditional 106..112 reschedule draw after earlier variable SW draws, complete support/factory trigger, nonzero-mode runtime insertion, funding/candidate branches, index-zero quirk, capacity fallback, exact inserted node fields/order, and evidence-backed takeover boundary | HIGH |
| live assembly `HouseClass__AI_Building_Strategy @ 0x004FD7A0..0x004FD911`, House secondary vtable `0x007EA834`, `0x004F6990`, `StorageClass::GetTotalValue @ 0x006C9600`, `0x005013A0`, raw body `0x00501400..0x0050153C`, `BuildingClass__TogglePowerOrGate @ 0x00447110`, `BuildingClass__SellBuilding @ 0x00457DE0`, `TeamClass__Remove_Member @ 0x006EA870`, and `TechnoClass__Evaluate_Candidate @ 0x006F875F..0x006F878B`; executable-wide writer/consumer search; exact retail map/ScriptTypes/AI INI census; direct Rust state/trigger/team-script inspection | constructor state, exact wallet hysteresis and attack-frame boundary, Fire Sale and All-To-Hunt object-visible effects, persistent target-bias latch, no-factory priority and double-call possibility, action 6/action 9/opcode 30 reachability, stock campaign presence/skirmish exclusions, and missing Rust ownership | HIGH |
| live decompile/assembly `HouseClass__Constructor @ 0x004F54A0` timer initialization and registered peer cross-appends, `HouseClass::Update @ 0x004F8440` frame-100 score decay, `HouseClass__AI_Building_Strategy @ 0x004FD500` hate expiry/acquisition `0x004FD50F..0x004FD71E` and defeated cleanup `0x004FD723..0x004FD772`, `HouseClass__UpdateAngerNodes @ 0x00504790`, `ScenarioClass__Post_Map_Init @ 0x00686890/0x00686A04..0x00686A2B`, `RulesClass__ReadGeneral @ 0x0066D530/0x0066FDB1..0x0066FDD9`, `RULESCLASS_FIELDS.csv`, both retail rules files, and direct Rust `house_state.rs`/`combat::update_receiver_anger_nodes` inspection | ordered zero-score peer lifecycle, signed AIHateDelays timer activation, owner-cell-reuse first-peer acquisition quirk, wrapping score update and strict positive eligible winner, defeated-peer zero plus forced enemy clear, periodic decay without reselection, and exact reusable/currently missing Rust ownership | HIGH |
| live decompile/assembly `HouseClass__AI_TryFireSW @ 0x005098F0`, `HouseClass__AI_FindBestRallyTarget @ 0x0050CBF0`, `AI_GroundRallyPoint @ 0x00509CD0`, `AI_Fire_LightningStorm @ 0x00509E00`, `AI_Fire_GenMutator @ 0x00509F60`, `AI_Fire_PsyDom @ 0x0050A150`, `AI_FindTeamTarget @ 0x0050D170`, `HouseClass__Fire_SW @ 0x004FAE50`, and `HouseClass__Check_Spy_Reveal @ 0x004FAF00`; Rules constructor `0x00665650` (`+0x1438=4`, empty `+0xEC4` probability vector, `+0xEE0=25`, `+0xEE4=10`) plus `ReadGeneral @ 0x0066D530/0x006707FE..0x00670AE6` and `ReadIQ @ 0x00674240`; retail `[SuperWeaponTypes]`, `[IQ]`, AIIonCannon/AISuperDefense values, and type-flag census; direct Rust `tick_ai`/command-dispatch inspection | pre-Manage ready-Super scan and type exclusions, exact IQ and AI-super-defense defaults/gates, target mechanisms and score/RNG ownership, empty missing-key semantics plus stock-inactive unsafe score branches, synchronous launch plus reverse-House spy-reveal ordering, active retail reachability, and current deferred-command/unsupported-type mismatch | HIGH |
| live assembly `HouseClass::Update @ 0x004F9038..0x004F9265`; House constructor mode store `0x004F56DF`; complete House-relative `+0x1E4` operand inventory; live decompile/assembly `HouseClass__AI_EconomyStateMachine @ 0x00509700` and its three Building-exit callers `0x00443CBC/0x00444102/0x00444F34`; Manage writes/call `0x004FE109/0x004FE3AB/0x004FE3B5`; Rules constructor/parser `0x0066703B/0x0066FDE7..0x0066FE00`; retail rules files | separate signed-modulo-eight chooser cadence, exact mode dispatch/fallback, zero initialization, complete mode-writer lifecycle, conditional economy RNG, immediate Manage chooser ownership, `AIAlternateProductionCreditCutoff` identity/default/parser, and stock activation | HIGH |
| live decompile/assembly `HouseClass__AI_Choose_Building @ 0x004FE3E0` and `FUN_0050C340 @ 0x0050C340`; `RulesClass__ReadGeneral @ 0x0066F54D..0x0066F74B/0x006700C3..0x006700F7`; `FUN_00672AE0 @ 0x0067375B..0x00673813`; retail rules blocks | unconditional wall-percent RNG ownership, side wall selection, reverse protected-source scan and following-node quirk, fixed-index wall order, exact original-node removal, projected-power equation/gates, side plant selection, and stock activation | HIGH |
| live decompile/assembly `HouseClass__AI_AssessPower @ 0x00508C30`, `TechnoClass__IsDeploying @ 0x0070FEC0`; full-program `+0x577B` and `+0x160B4` operand inventories | sole House deploying-power latch writer, literal `Techno+0x1D0 != 0` predicate, positive-output gate, and zero-only ordinary AI cost tolerance writer set | HIGH |
| live decompile/assembly `BuildingClass__Limbo @ 0x00445880`, `FUN_0050A490 @ 0x0050A490`, BuildingType constructor/`IsBaseDefense=` reader `0x0045E225/0x00460FFC..0x00461010`; stock defense assignments | exact same-cell packed-zero clearing, campaign/non-defense retention, nonzero-mode defense `-1` conversion, and unchanged filled/retry tail | HIGH |
| full-program `operand_pattern=0x5714` store inventory plus vector-relative `House+0x5704` writer audit; ordered-removal assembly `0x0050495A..0x005049A9`, `0x00504A79..0x00504AC8`, `0x00504E1B..0x00504E6A`; xrefs from `TeamClass__Convoy_Script_Attack_Production`; `FUN_0050D250` sole MapClass resize xref | remaining direct stores classified as scripted removals, resize-only adjustment, or read-only trigger comparison; `AI_Choose_Building` wall/power stores included rather than missed by displacement-only search | HIGH |
| live decompile/assembly `FUN_0042EB50 @ 0x0042EB50` and its sole callee xref to `FUN_0050CAD0 @ 0x0050CAD0`; zero bytes at `0x0089C310` | caller-effective wildcard mode/filled ordering, exact recycling branch order, `(0,0)` clearing, eight-direction wall adjacency, and signed wrapping timer comparison | HIGH |
| full-program `operand_pattern=0x54d8` instruction search; House constructor `0x004F5A59`; `BuildingClass__ReceiveDamage @ 0x00442230`; House AI reads `0x004FD80A/0x004FD82D` | complete `House+0x54D8` constructor/writer/reader ownership and exact pre-receiver write predicate | HIGH |
| TechnoType constructor/reader `0x00710FF0..0x00710FF6/0x007143D0..0x007143FE`; House constructor/lifecycle `0x004F55FE..0x004F5604/0x00502A80/0x005025F0`; Rules constructor `0x006668D4` and DynamicVector construction `0x0066706E..0x006670A4`; `RulesClass__ReadGeneral @ 0x006700FC..0x0067011A/0x00670585..0x006705B7`; `DifficultyClass__ReadINI_IntVector @ 0x00475D70`; retail keys | signed ResourceGatherer/Destination counters, empty native slave-miner vector default, signed replace-time default/parser, Hard/Normal/Easy retail values, and stock activation | HIGH |
| TechnoType `Primary=` reader `0x007129AB..0x007129DE`; BuildingType constructor/reader `0x0045DEF6..0x0045DF08/0x0046048E..0x004604AC/0x00461060..0x0046109A`; retail type assignments | resolved-primary, Wall, and sign-split positive-Power inputs with exact defaults and stock activation | HIGH |
| `RulesClass` constructor `0x00666972`; `RulesClass::ReadGeneral @ 0x0067026C..0x00670286`; Building exit `0x00445237..0x004452C5`; both retail rules files | signed maximum-placement-failure default/key, active retail value three, post-increment strict threshold, campaign gate, and ordered node eviction | HIGH |
| live decompile/assembly `HouseClass__FirstBuildableFromArray @ 0x005051E0`; naval callsites `0x00506103/0x00506128`; retail GAYARD/NAYARD/YAYARD rules and art blocks | exact forward selector gates, no generic production filters, stock side outcomes, and `6x6` footprints | HIGH |
| naval pushes `0x0050616D..0x00506193`; live decompile/assembly `Find_Nearby_Passable_Cell @ 0x0056DC20` and projection `0x006D6410`; Rust `find_nearby_cell.rs` and `bridge_facts.rs` | literal query matrix, frame-modulo selection, reachable four-level bridge projection, and precise shared-helper delta | HIGH |
| `RulesClass::ReadGeneral` live string xrefs for Shipyard/AINavalYardAdjacency; `RulesClass__ReadAI @ 0x00672AE0`, BuildConst block `0x00672B14..0x00672C01` | exact naval INI bindings | HIGH |
| retail `rulesmd.ini` and `rules.ini` searches | active/inactive stock-data gates | HIGH |
| direct Rust inspection of `src/sim/ai.rs`, `house_state.rs`, `object_type.rs`, `ruleset.rs` | current ownership and missing state/fields | HIGH |

No factual claim in this report depends on a newly written Ghidra label or comment. No Ghidra metadata was changed.
