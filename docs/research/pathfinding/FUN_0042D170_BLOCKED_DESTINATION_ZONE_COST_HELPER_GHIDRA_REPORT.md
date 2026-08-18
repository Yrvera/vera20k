# FUN_0042D170 Blocked-Destination Zone-Cost Helper -- Ghidra Research Report

**Address(es):** `0x0042D170` primary helper; direct live caller of interest `FootClass__Find_Path @ 0x004D3920`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `FUN_0042D170` argument contract, return semantics, bridge-aware endpoint adjustments, direct caller thresholds, and relation to `Zone_precheck` / `Find_Nearby_Passable_Cell`.  
**Non-Scope:** full `FootClass::Find_Path`, full `AStar_pathfind_search`, full FNPC ring/validator internals, and global zone-map construction.  
**Confidence:** High for the helper and direct callsites; Medium for Rust delta because no implementation was changed or tested.  
**Active in YR:** Yes. Static xrefs include `FootClass__Find_Path @ 0x004D3920`, `FootClass__Mission_Patrol @ 0x004D4280`, `FootClass__Greatest_Threat_Scan @ 0x004D5690`, plus AI/command callers `0x004DC8C0`, `0x005221D0`, `0x0064CDA0`, `0x00746000`.

## 1. Overview

`FUN_0042D170` is a `PathfinderClass` helper that answers: "is there a reasonable zone-level route between two cell coordinates, and how far/costly is that route in a small integer estimate?" It is used by blocked-destination and target-selection fallbacks before committing to a redirected cell/target. It does not run cell A* and does not return a path.

The helper first resolves bridge-aware endpoint coordinates, runs `Zone_precheck`, and returns `0x7fffffff` on zone failure. On success it returns the maximum of direct Chebyshev distance and a zone-chain/bridge-endpoint estimate. Active in YR: Yes, via `0x0042D222` and the direct xrefs above.

## 2. Class Layout / Key Offsets

| Owner | Offset / data | Type | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `PathfinderClass` | `+0x38` | byte | Set to `1` before reset/zone precheck in this helper. | `0x0042D179` | Yes |
| `PathfinderClass` | `+0x74`, `+0x8C`, `+0xA4` | 3 heap/vector-like objects | Each receives vtable `+0x0C` after `PathfinderClass__Reset`. | `0x0042D182..0x0042D195` | Yes |
| `PathfinderClass` | `+0xBC + level*1000` | `u16[500]` zone chain | Level-0 chain entries consumed at `+0xB8/+0xBA/+0xBC/+0xBE` by this helper after `Zone_precheck`. | `0x0042D25D..0x0042D3CE`; writer in `Zone_precheck` | Yes |
| `PathfinderClass` | `+0xC74` | `int` | Level-0 zone-chain count used for base estimate `count * 2 - 2`. | `0x0042D25D..0x0042D267` | Yes |
| `CellClass` | `+0x140 bit 0x100` | flag | Bridge/deck-aware endpoint resolution gate. | `0x00583180`, caller-passed reduced flag from `cell.flags >> 8 & 1` | Yes |
| `TechnoTypeClass` | `+0x5B4` | int | MovementZone row used when caller passes `param_7 == -1`. | `0x0042D1FC..0x0042D214` | Yes |
| `DAT_0089C278` | packed coord | sentinel | Invalid/fallback coordinate for bridge ground-exit helper `0x00583820`. | `0x0042D2D8`, `0x0042D313`, `0x0042D3A5`, `0x0042D3DF` | Yes |

## 3. Core Logic

### Argument contract

Binary signature from Ghidra: `int __thiscall FUN_0042D170(PathfinderClass* this, Cell* from, Cell* to, Techno* owner, bool from_bridge_flag, bool to_bridge_flag, int movement_zone_or_minus1)`.

More precisely:

- `ECX` is `PathfinderClass`; direct call in `FootClass__Find_Path` uses global `0x0087E8B8` (`0x004D3C97`).
- Stack arg 1 is the first `CellStruct*` / packed short pair. It is passed to `MapClass__Get_CellClass` and bridge-aware resolver with `param_5` (`0x0042D197..0x0042D1C9`).
- Stack arg 2 is the second `CellStruct*` / packed short pair. It is passed to `MapClass__Get_CellClass` and bridge-aware resolver with `param_6` (`0x0042D1A6..0x0042D1E4`).
- Stack arg 3 is the owner object; if movement-zone arg is `-1`, the helper reads owner vtable `+0x84`, then `TechnoTypeClass+0x5B4` (`0x0042D1F3..0x0042D214`).
- Stack args 4 and 5 are byte-tested bridge-layer/path-coordinate flags. Callers usually pass `(cell.flags >> 8) & 1`, i.e. `CellClass+0x140 bit 0x100` reduced to bool.
- Stack arg 6 is either a concrete MovementZone row or `-1` for "derive from owner type."

Active in YR: Yes; all direct callers use this ABI.

### Main estimate

The result is an integer threshold/quality estimate:

1. Mark `this+0x38 = 1`, reset pathfinder, clear three local queues/vectors.
2. Resolve both endpoint coordinates through `MapClass__ResolvePathCoord_BridgeAware`.
3. If `movement_zone_or_minus1 == -1`, derive `MovementZone` from owner type; if owner is null, use `0`.
4. Call `Zone_precheck(resolved_from, resolved_to, movement_zone, owner)`.
5. If precheck fails, return `0x7fffffff`.
6. Compute direct Chebyshev distance between the original two input coordinates.
7. Start an alternate estimate at `PathfinderClass+0xC74 * 2 - 2`.
8. If the second endpoint is bridge-marked, add a bridge-exit adjustment from endpoint 2 to a zone-chain-adjacent ground coordinate, unless both endpoints are on the same high-bridge record.
9. If the first endpoint is bridge-marked, add a symmetric bridge-exit adjustment from endpoint 1.
10. Return `max(direct_chebyshev, adjusted_zone_chain_estimate)`.

Important tiny details:

- `Zone_precheck` failure is a hard sentinel return, not `0`, not `-1`: `0x7fffffff` at `0x0042D452`.
- Direct distance is Chebyshev: `max(abs(dx), abs(dy))` using signed integer abs via `CDQ/XOR/SUB`, not Euclidean and not Manhattan (`0x0042D22F..0x0042D25D`).
- Base zone estimate is `count * 2 - 2`, where `count` is level-0 stored zone-chain count at `+0xC74` (`0x0042D25D..0x0042D267`). Same-zone success count is `1`, so base estimate is `0`.
- The final comparison is strict `if direct < adjusted return adjusted else direct`; equality returns direct but same value (`0x0042D438..0x0042D444`).
- The function resets/clears pathfinder state before the precheck, so this helper does not reuse A* retry exclusions from an earlier path search (`0x0042D17D..0x0042D195`). It consumes whatever `Zone_precheck` derives after that reset.
- It does not inspect entity blockers, terrain edge costs, A* open/closed lists, or cell-level path length.

Active in YR: Yes.

### Bridge endpoint adjustments

When `to_bridge_flag != 0`, the helper tries to map the second endpoint from bridge/deck space to a nearby ground/bridge-record coordinate selected from the stored level-0 zone path:

- If both endpoints are bridge-marked, `MapClass__FindBridgeRecord(cell, threshold=3, start=0)` is called for both. If the record indexes match and are not `-1`, the helper returns direct Chebyshev immediately (`0x0042D27F..0x0042D2AA`). This bypasses zone-chain base adjustment.
- Otherwise it chooses a zone-chain entry near the tail of the stored path. If `count < 4`, it uses `*(u16*)(this + 0xBA + count*2)`; if `count >= 4`, it first tries `*(u16*)(this + 0xB8 + count*2)` and falls back to the `<4` expression if invalid (`0x0042D2B4..0x0042D323`).
- It calls `FUN_00583820(MapClass, &out, endpoint, level=0, zone_id)` to find one of six bridge-adjacent candidate cells whose level-0 zone id matches the requested chain entry.
- If `FUN_00583820` returns sentinel `DAT_0089C278`, it substitutes the bridge-aware resolved endpoint from step 2. If that is also sentinel, no adjustment is added (`0x0042D323..0x0042D341`).
- The adjustment added is Chebyshev distance from original endpoint to the selected/substituted coordinate (`0x0042D343..0x0042D372`).

When `from_bridge_flag != 0`, the same pattern applies to the first endpoint:

- `count < 4` uses `*(u16*)(this + 0xBC)`.
- `count >= 4` first tries `*(u16*)(this + 0xBE)`, then falls back to `+0xBC`.
- Sentinel handling substitutes the first resolved endpoint.
- Adjustment is Chebyshev from original first endpoint to selected/substituted coordinate.

Active in YR: Yes. These branches are live for bridge/deck cells where callers pass `cell.flags >> 8 & 1`.

## 4. INI Keys

No INI key is read directly by `FUN_0042D170`.

| Key / data | Binary field / source | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=` | `TechnoTypeClass+0x5B4` when arg 6 is `-1` | Selects `ZonePassabilityMatrix` row inside `Zone_precheck`. | Yes |
| Reduced `ZoneType` | `CellClass+0x140 >> 8 & 1` at callers for bridge-aware bools; broader zone type is consumed by `Zone_precheck` graph records | Endpoint bridge resolution and zone precheck passability. | Yes |
| `ZonePassabilityMatrix` | global `int[13][8]` | `Zone_precheck` accepts graph edges only when matrix row/column value is exactly `1`. | Yes |

## 5. Integration Points

### Direct callees

| Callee | Role | Evidence | Active in YR |
|---|---|---|---|
| `PathfinderClass__Reset @ 0x0042A5B0` | Clears/reinitializes pathfinder state before this helper's precheck. | `0x0042D17D` | Yes |
| `MapClass__Get_CellClass @ 0x005657A0` | Fetches both endpoint cells and caller-side flag inputs. | `0x0042D19C`, `0x0042D1AF` | Yes |
| `MapClass__ResolvePathCoord_BridgeAware @ 0x00583180` | Converts bridge/deck endpoint to path coordinate when bridge flag is set. | `0x0042D1C4`, `0x0042D1DF` | Yes |
| `Zone_precheck @ 0x0042C290` | Proves zone-level reachability and writes level path/count arrays. | `0x0042D222` | Yes |
| `MapClass__FindBridgeRecord @ 0x0056DA10` | Same high-bridge record short-circuit for two bridge endpoints. | `0x0042D289`, `0x0042D29E` | Yes |
| `FUN_00583820 @ 0x00583820` | Finds a bridge-adjacent coordinate whose zone id matches a selected stored chain entry. | `0x0042D2D1`, `0x0042D30C`, `0x0042D39E`, `0x0042D3D8` | Yes |

### Direct callers and threshold semantics

| Caller | Use of helper result | Evidence | Active in YR |
|---|---|---|---|
| `FootClass__Find_Path @ 0x004D3920` | In movement-result `6` branch, after `Find_Nearby_Passable_Cell` finds an alternate for a blocked/far destination, accept the alternate only if helper result `<= cheb(original_dest, alternate) + 6`. | `0x004D3A92`, FNPC call `0x004D3B76`, helper call `0x004D3C9C`, compare `0x004D3CA1..0x004D3CAA` | Yes |
| `FootClass__Find_Path @ 0x004D3920` | Movement-result `7` branch uses FNPC and immediately retargets; this branch did not call the helper in the scoped disassembly window. | `0x004D3CDD..0x004D3DFF` | Yes |
| `FUN_005221D0` | For reachable non-adjacent cells at Chebyshev distance `< 4`, returns `helper_result > 7`. This treats high estimate as blocked/true and small estimate as not blocked/false. | `0x005221D0` decompile | Yes |
| `FUN_00746000` | Similar predicate with `MovementZone=0`, only for distance `< 0x0C`, returns `helper_result > 0x0F`. | `0x00746000` decompile | Yes |
| `FUN_004DC8C0` | Command/target selection accepts a branch only if same/compatible zone checks pass and `helper_result <= min(cheb_delta + 3, 5)` when a local flag is set. | `0x004DCC86` callsite in decompile | Yes |
| `FootClass__Mission_Patrol @ 0x004D4280` | Patrol/attack state compares helper result against `((patrol radius in cells) + 6)` and against paired target alternatives; smaller helper result is better/reachable. | multiple calls in `0x004D4280` decompile | Yes |
| `FootClass__Greatest_Threat_Scan @ 0x004D5690` | Threat scan rejects candidate fire/move cells when helper result exceeds current Chebyshev distance plus `8`; also compares two candidate target cells. | calls around `0x004D6723` and following block | Yes |
| `FUN_0064CDA0` | AI scan/line process calls helper after zone divergence is observed; result is used as an accept/reject quality gate, but the decompiler did not expose the compare cleanly. | `0x0064D5xx` call region | Yes |

### Relation to FNPC and AStar

- `Find_Nearby_Passable_Cell @ 0x0056DC20` produces candidate alternate cells. `FUN_0042D170` scores whether the candidate is zone-reasonable relative to the original target/path context. Evidence: `FootClass__Find_Path` calls FNPC at `0x004D3B76`, then the helper at `0x004D3C9C`.
- `FUN_0042D170` does not call `AStar_pathfind_search @ 0x0042C900`; it only calls `Zone_precheck @ 0x0042C290`.
- `AStar_pathfind_search` independently calls `Zone_precheck` in the normal A* path, but this report does not claim its retry/appending semantics beyond the prior `ZONE_PRECHECK_0042C290...` report.

Active in YR: Yes.

## 6. Current Rust Implementation Status

| Surface | Current observed shape | Delta against this helper |
|---|---|---|
| `src/sim/movement/movement_path.rs::resolve_requested_move_goal` | If requested goal is not walkable, redirects to nearest walkable within a fixed radius. | Missing `FUN_0042D170`-style zone-cost acceptance threshold before accepting the alternate. |
| `src/sim/movement/movement_path.rs::try_repath_after_block` | Repath uses `resolve_requested_move_goal(..., max_radius=10)` and changes `final_goal` when blocked. | Missing binary's small integer acceptance comparison such as `helper_result <= cheb(original, alternate) + 6`. |
| `src/sim/pathfinding/zone_search.rs::find_path_zoned` | Uses one-level corridor approximation, whole-zone exclusions, and unrestricted fallback. | Does not expose a reusable "zone route estimate" equivalent to `FUN_0042D170`; existing zone search is path production, not pre-accept scoring. |
| `src/sim/pathfinding/core.rs` / A* tests | Has blocked-start and blocked-goal behavior tests. | No test appears to cover blocked-destination alternate rejection by zone-cost threshold. |
| Miner/spawn/production nearby-passable helpers | Several callsites use nearby-passable approximations. | This report only proves movement/target direct callers; reuse for miners/spawn remains unchecked unless their binary callsites pass through the same helper. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0042D170 @ 0x0042D170` full helper | verified | decompile and disassembly `0x0042D170..0x0042D45E` | none for scoped behavior |
| Argument stack/`ECX` contract | verified | disassembly `0x0042D197..0x0042D222`, caller `0x004D3C43..0x004D3C9C` | exact semantic names depend on caller context, but binary ABI is settled |
| `Zone_precheck` failure return | verified | `0x0042D227..0x0042D229`, `0x0042D452` | none |
| Direct Chebyshev distance | verified | `0x0042D22F..0x0042D25D` | none |
| Base zone estimate `count*2-2` | verified | `0x0042D25D..0x0042D267` | none |
| Bridge same-record shortcut | verified | `0x0042D27F..0x0042D2AA` | low-bridge record behavior outside high-bridge `FindBridgeRecord` shortcut not expanded |
| Second endpoint bridge adjustment | verified | `0x0042D2B4..0x0042D376` | none for helper semantics |
| First endpoint bridge adjustment | verified | `0x0042D376..0x0042D438` | none for helper semantics |
| `MapClass__ResolvePathCoord_BridgeAware @ 0x00583180` | touched-not-exhausted | decompile `0x00583180` | full bridge coordinate resolver internals beyond helper contract |
| `FUN_00583820 @ 0x00583820` | touched-not-exhausted | decompile `0x00583820` | exact candidate ordering names; enough verified for helper contract |
| `MapClass__FindBridgeRecord @ 0x0056DA10` | touched-not-exhausted | decompile `0x0056DA10` | full bridge-record lifecycle outside scope |
| `Zone_precheck @ 0x0042C290` relation | verified-by-prior + spot-check | decompile `0x0042C290`; `ZONE_PRECHECK_0042C290...` | full A* retry semantics left to slot 1 |
| `FootClass__Find_Path` blocked-destination callsite | verified | disassembly `0x004D3A92..0x004D3CAA` | full Find_Path outside scope |
| Other direct callers | touched-not-exhausted | decompiled xrefs listed in §5 | runtime frequency and player-visible trigger matrix |
| Current Rust movement path fallback | verified for scan | file read `src/sim/movement/movement_path.rs` | implementation not changed |
| Current Rust zone search | verified for scan | file read `src/sim/pathfinding/zone_search.rs` | implementation not changed |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is this exhaustive-slice or coverage-map? -> exhaustive-slice for one helper plus immediate callers/callees.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- Is `FUN_0042D170` live in YR? -> Yes, direct xrefs include `FootClass__Find_Path`, patrol, threat scan, and command/AI predicates.` (evidence: `get_function_callers 0x0042D170`)
- `[RESOLVED] OQ-3 -- Does it run A*? -> No; direct callees include `Zone_precheck` but not `AStar_pathfind_search`.` (evidence: `get_function_callees 0x0042D170`)
- `[RESOLVED] OQ-4 -- What does zone failure return? -> `0x7fffffff`.` (evidence: `0x0042D452`)
- `[RESOLVED] OQ-5 -- What distance metric is used? -> Chebyshev, not Euclidean/Manhattan.` (evidence: `0x0042D22F..0x0042D25D`)
- `[RESOLVED] OQ-6 -- What is the base zone-chain estimate? -> `Pathfinder+0xC74 * 2 - 2`.` (evidence: `0x0042D25D..0x0042D267`)
- `[RESOLVED] OQ-7 -- How is movement zone chosen? -> explicit arg unless `-1`, then owner type `+0x5B4`, null owner => `0`.` (evidence: `0x0042D1F3..0x0042D214`)
- `[RESOLVED] OQ-8 -- What does it do with bridge endpoints? -> resolves endpoints and adds Chebyshev bridge-exit adjustments from selected level-0 zone-chain entries.` (evidence: `0x0042D2B4..0x0042D438`)
- `[RESOLVED] OQ-9 -- Does same high-bridge record get special treatment? -> Yes, if both endpoints bridge-marked and `FindBridgeRecord(...,3,0)` matches non-`-1`, return direct distance.` (evidence: `0x0042D27F..0x0042D2AA`)
- `[RESOLVED] OQ-10 -- How does it relate to FNPC? -> FNPC generates an alternate cell, then this helper gates acceptance in `FootClass__Find_Path`.` (evidence: `0x004D3B76`, `0x004D3C9C`)
- `[RESOLVED] OQ-11 -- What is the blocked-destination threshold in `Find_Path`? -> accept alternate if helper result `<= cheb(original, alternate) + 6`.` (evidence: `0x004D3CA1..0x004D3CAA`)
- `[RESOLVED] OQ-12 -- What are the other obvious threshold predicates? -> `FUN_005221D0` uses `>7`; `FUN_00746000` uses `>0x0F`; threat scan uses `candidate cheb + 8`.` (evidence: decompiles `0x005221D0`, `0x00746000`, `0x004D5690`)
- `[RESOLVED] OQ-13 -- Does helper mutate global zone graphs? -> No evidence; it resets pathfinder-local state and calls consumers/resolvers only.` (evidence: direct callee set and `0x0042D17D..0x0042D222`)
- `[RESOLVED] OQ-14 -- Does current Rust have an equivalent scoring helper? -> No obvious equivalent; movement redirects directly to nearest walkable before pathing.` (evidence: `movement_path.rs`, `zone_search.rs` scan)
- `[DEFERRED] OQ-15 -- Exact runtime frequency of each direct caller.` (category: needs-runtime-debugger; reason: static xrefs prove live reachability but not frequency; next-step-if-pursued: instrument call counts in standard skirmish orders)
- `[DEFERRED] OQ-16 -- Full `FootClass::Find_Path` code-6/code-7 semantic names.` (category: out-of-scope; reason: parent explicitly scoped out full `Find_Path`; next-step-if-pursued: slot 6 blocked-destination fallback report)
- `[DEFERRED] OQ-17 -- Full `FUN_00583820` candidate ordering labels.` (category: out-of-scope; reason: helper contract only needs sentinel/fallback and distance adjustment; next-step-if-pursued: bridge endpoint coordinate helper investigation)
- `[DEFERRED] OQ-18 -- Whether miner/spawn nearby-passable approximations should use this exact helper.` (category: requires-different-system-context; reason: this report only verified direct xrefs to `0x0042D170`; next-step-if-pursued: trace each production/miner caller path to see if it reaches this helper)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Blocked-destination alternate from FNPC is accepted only when `zone_cost_estimate <= cheb(original_goal, alternate) + 6`. | `FootClass__Find_Path` FNPC `0x004D3B76`, helper `0x004D3C9C`, compare `0x004D3CA1..0x004D3CAA` | mismatch: `resolve_requested_move_goal` redirects to nearest walkable without a zone-cost threshold. | `src/sim/movement/movement_path.rs::resolve_requested_move_goal`, `try_repath_after_block`; likely new sim pathfinding helper. | Before mutating `final_goal`, compute binary-style route estimate and reject alternates whose estimate exceeds local alternate distance plus six cells. | Deterministic map with blocked requested goal and nearest passable cell across a disconnected/long zone corridor: movement must not redirect to the near-but-zone-expensive cell. Proposed test name: `blocked_goal_alternate_rejected_when_zone_estimate_exceeds_plus_six`. | Do not accept nearest walkable solely by radius; that lets units retarget across bad zone corridors that gamemd rejects. |
| Helper return is not A* path length; it is `max(direct Chebyshev, level0_zone_count*2-2 + bridge endpoint adjustments)`, or `INT_MAX` if `Zone_precheck` fails. | `0x0042D227..0x0042D267`, `0x0042D438..0x0042D452` | missing: no reusable Rust equivalent. | `src/sim/pathfinding/zone_search.rs` or adjacent helper module; consumers in movement/target selection. | Expose a route-estimate function separate from path production; it must return a large sentinel on zone failure and small integer estimates on success. | Same-zone start/goal with no bridge markers returns direct Chebyshev; disconnected zones return sentinel. Proposed test name: `zone_cost_helper_returns_chebyshev_or_sentinel_without_running_astar`. | Do not substitute actual A* path length or terrain weighted cost; callers compare against small constants and would reject/accept different targets. |
| Bridge-marked endpoints add extra Chebyshev offsets to ground/bridge-adjacent coordinates selected from stored level-0 zone chain; same high-bridge record returns direct distance early. | `0x0042D27F..0x0042D438`; callees `0x00583180`, `0x00583820`, `0x0056DA10` | missing/unchecked: current bridge pathing has layered A*, but no estimate-side bridge endpoint adjustment. | Future bridge-aware zone estimate helper; `PathGrid`/zone metadata must expose bridge endpoint/record relation if parity is pursued. | Two points on the same bridge record produce direct Chebyshev estimate; crossing off a bridge adds endpoint adjustment instead. Proposed test name: `zone_cost_helper_same_bridge_record_bypasses_endpoint_penalty`. | Do not treat bridge flag as just another walkable layer in the estimate; the binary uses bridge-record and stored-zone-chain endpoint correction. |
| MovementZone row defaults to owner type `+0x5B4` only when caller passes `-1`; concrete callers may force row `0` or `7`. | `0x0042D1F3..0x0042D214`; callers `0x005221D0`, `0x00746000` | partial: Rust often passes `movement_zone.unwrap_or(Normal)` into zone search; no per-caller estimate override exists. | Any future estimate API should accept an explicit movement-zone override and owner-derived default. | Ship/infantry predicate wrappers can pass forced rows and get different reachability over the same zones. Proposed test name: `zone_cost_helper_respects_explicit_movement_zone_override`. | Do not always derive from the moving object's locomotor; several binary callers pass concrete override rows. |

### Negative Facts / Do Not Do

- Do not implement this as `find_path_zoned` and count returned path cells. Evidence: no call to `AStar_pathfind_search`; Active in YR: Yes.
- Do not return `None`/`0` on zone failure if matching binary thresholds; the sentinel is `0x7fffffff`. Evidence: `0x0042D452`; Active in YR: Yes.
- Do not use Manhattan or Euclidean distance for the helper estimate. Evidence: signed abs + max at `0x0042D22F..0x0042D25D`; Active in YR: Yes.
- Do not ignore bridge endpoint flags in the estimate. Evidence: two separate branches gated by byte-tested `param_5` and `param_6`; Active in YR: Yes.
- Do not assume all callers use `MovementZone` from the owner. Evidence: callers pass `0`, `7`, and `-1`; Active in YR: Yes.
- Do not conflate this helper with `Zone_precheck`: it calls `Zone_precheck`, then converts the stored level-0 zone chain into a threshold estimate. Active in YR: Yes.

### Remaining Uncertainty

- Exact runtime frequency of each non-`Find_Path` caller needs debugger instrumentation.
- Full `FootClass__Find_Path` code-6/code-7 semantics remain for the sibling blocked-destination fallback slot.
- `FUN_00583820` candidate ordering is only described to the level needed for this helper; a future bridge endpoint report should name each candidate position relative to bridge direction.
- Whether production/miner/spawn nearby-passable Rust approximations should call an equivalent helper depends on their own binary caller traces, not this xref set.

### Stale Docs / Follow-up Docs

- `PATHFINDING_ASTAR_GHIDRA_REPORT.md`: any wording that implies `FUN_0042D170` is part of cell A* retry/appending should be replaced with: "`FUN_0042D170` is a separate route-quality estimate helper used by blocked-destination/target fallbacks; it calls `Zone_precheck` but not `AStar_pathfind_search`, returning `INT_MAX` on precheck failure or a small Chebyshev/zone-chain estimate on success."
- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`: if it describes FNPC fallback acceptance as "nearest passable wins," add: "At least in `FootClass__Find_Path` code-6 blocked-destination fallback, the FNPC candidate is accepted only after `FUN_0042D170(candidate/original context) <= cheb(original,candidate)+6`."

## Sources

- Ghidra HTTP read endpoints used against loaded `gamemd.exe`: `decompile_function`, `disassemble_function`, `get_function_callers`, `get_function_callees`.
- Decompiled/disassembled addresses: `0x0042D170`, `0x004D3920`, `0x004DC8C0`, `0x005221D0`, `0x0064CDA0`, `0x00746000`, `0x004D5690`, `0x004D4280`, `0x00583180`, `0x00583820`, `0x0056DA10`, `0x0042C290`.
- Prior docs referenced: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`.
- Rust files scanned read-only: `src/sim/pathfinding/zone_search.rs`, `src/sim/movement/movement_path.rs`.
