# FootClass::Find_Path Blocked-Destination Fallback -- Ghidra Research Report

**Address(es):** `0x004D3920` (`FootClass::Find_Path`), related `0x0056DC20` (`FootClass::Find_Nearby_Passable_Cell`), `0x0042D170` (zone-distance helper), `0x0042C290` (`Zone_precheck`)
**Investigation Mode:** targeted exhaustive-slice attempted; final status is partial because this session had no live Ghidra MCP/decompiler endpoint.
**Claimed Scope:** static reconstruction, from prior Ghidra reports and current Rust scan, of the `Find_Path` destination probe that reacts to blocked destination `Can_Enter_Cell` results, invokes nearby-passable fallback and/or `FUN_0042D170`, and may substitute the path target before A*.
**Non-Scope:** `Find_Nearby_Passable_Cell` internals, full A*, bridge dual-closed-list internals, and full `UnitClass::Can_Enter_Cell` decoding.
**Confidence:** Medium overall. High for facts cited to prior binary reports; low for the exact `Find_Path -> FNPC` stack argument row because the caller-parameter report explicitly left this callsite unresolved and no fresh decompile was available.
**Active in YR:** Yes for the path. Prior reports bind `FootClass::Find_Path @ 0x004D3920` to standard movement and bind `0x0042D170` from `Find_Path` at `0x004D3C9C`; no TS-only gate is documented for this path.

## 1. Overview

`FootClass::Find_Path` is the top-level foot path request used by drive/walk movement when a unit needs a path to its NavCom destination. Before the normal A* run, it probes the destination with the unit's `Can_Enter_Cell` vtable slot and has a special fallback path for blocked destinations. That fallback is real in standard YR, but prior reports only prove the outline: code `6`/`7`-related destination handling can call `Find_Nearby_Passable_Cell` and uses `FUN_0042D170`/`Zone_precheck` as a reachability or distance-quality helper before substituting a target.

The most important implementation-facing result is negative: current Rust redirects blocked move goals up front with `resolve_requested_move_goal(..., max_radius=10)` and a simple nearest-walkable search. The binary evidence available here does not support that as an exact `Find_Path` mirror: gamemd's fallback is conditional on destination `Can_Enter_Cell`, uses the foot object's FNPC contract, and involves `FUN_0042D170` for zone-aware evaluation.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `FootClass` | `+0x5E0..+0x63F` | 24-entry path queue written by successful `Find_Path`. | `FOOTCLASS_STRUCT_LAYOUT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md` | Yes |
| `FootClass` | `+0x640..+0x648` | path/movement delay timer touched around pathfinding. | `FOOTCLASS_STRUCT_LAYOUT.md`; drive reports | Yes |
| `FootClass` | `+0x5D4` | team pointer; gates path length/limits in `Find_Path`. | `FOOTCLASS_STRUCT_LAYOUT.md` | Conditional, live for team members |
| `TechnoTypeClass` | `+0x67C` | SpeedType used by A*/FNPC callers. | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, FNPC caller matrix | Yes |
| `TechnoTypeClass` | `+0x5B4` | MovementZone row used by zone precheck and many FNPC callers. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | Yes |
| Rules | `CloseEnough=2.25` | runtime movement close-enough stop threshold; related but not proven to be the Find_Path fallback radius. | `ini/rulesmd.ini:58`; drive reports | Yes |
| Rules | `PathDelay=.01`, `BlockagePathDelay=60` | repath timer and urgency timing; caller-side to `Find_Path`, not target substitution itself. | `ini/rulesmd.ini:3106-3107` | Yes |

## 3. Core Logic

Verified from prior Ghidra reports:

1. `Find_Path @ 0x004D3920` gets the requested destination cell, computes straight-line distance, probes destination entry via vtable `+0x1AC` (`Can_Enter_Cell`), handles special cases before `Run_AStar @ 0x004CBBA0`, and copies up to 24 path entries on success. Evidence: `PATHFINDING_ASTAR_GHIDRA_REPORT.md` section 2.
2. Destination `Can_Enter_Cell` code `6` is a soft occupied/blocking class in A* cost tables (`8.0`), while code `7` is impassable and not expanded by A*. Evidence: `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`.
3. `Find_Path` reaches `FUN_0042D170`; that helper calls `Zone_precheck @ 0x0042C290` and returns a huge failed estimate (`0x7fffffff`) if the zone precheck fails. Evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` lines for call at `0x0042D222` and `Find_Path` xref at `0x004D3C9C`.
4. The alternate helper is documented as the helper used by `Find_Path`'s code-6/7 nearby-cell fallback. Evidence: `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` section 6.
5. `Find_Nearby_Passable_Cell @ 0x0056DC20` has `RET 0x3c`, proving 15 stack args after `this/out`, derives its search radius internally from the foot object and caps at `32`, uses `CheckPassability(..., -1)`, and optional final occupancy uses `CheckOccupancy(rect, -1)`. Evidence: `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`.

Not verified in this session and therefore not claimed as fact:

- The exact branch condition for code `6` versus code `7` inside `Find_Path`.
- The exact `Find_Path -> FNPC` 15 stack arguments.
- The exact "direct_distance + 6" tolerance claim. I found no source in the searched docs that proves this arithmetic, and no live decompile was available to confirm or refute it.
- Whether `Find_Path` substitutes the first FNPC result, the best `FUN_0042D170` candidate, or only substitutes after a strict score threshold.

## 4. INI Keys

No INI key directly controls this `Find_Path` target substitution branch in the evidence reviewed. The relevant adjacent keys are:

| Key | File / default | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `CloseEnough=2.25` | `rulesmd.ini:58` | Used by movement blockage/stop logic; not proven as Find_Path fallback tolerance. | drive movement docs | Yes |
| `PathDelay=.01` | `rulesmd.ini:3106` | Controls retry rate for callers that invoke `Find_Path`. | `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` | Yes |
| `BlockagePathDelay=60` | `rulesmd.ini:3107` | Controls urgency escalation (`urgency=1/2`) for blocked repaths that call `Find_Path`. | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` | Yes |
| MovementZone/SpeedType on techno types | type data | Supplies movement row/speed type to A*/FNPC paths. | A* and FNPC reports | Yes |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `DriveLocomotionClass::Process_Movement -> Find_Path` | No-path, blocked-code-2, and far-continuation cases call `Find_Path(dest_cell, allow_crush/type flag, urgency)`. | `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` | Yes |
| `Find_Path -> Run_AStar` | Normal path after prehandling; result copies into foot path queue. | `PATHFINDING_ASTAR_GHIDRA_REPORT.md` | Yes |
| `Find_Path -> FNPC` | Prior reports say blocked destination fallback calls FNPC, but exact call stack decode is unresolved. | `PATHFINDING_ASTAR_GHIDRA_REPORT.md`; FNPC caller matrix | Yes, partial |
| `Find_Path -> FUN_0042D170 -> Zone_precheck` | Zone-aware estimate/check helper used by code-6/7 fallback. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`; `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` | Yes |
| A* neighbor `Can_Enter_Cell` | Codes `0..6` costed, code `7` rejected; separate from destination substitution probe. | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

| Surface | Current behavior observed | Delta vs verified/inferred binary slice |
|---|---|---|
| `src/sim/movement/movement_commands.rs::issue_move_command_with_layered` | Calls `resolve_requested_move_goal(..., max_radius=10)` before A*, for initial move commands; redirects if goal is not walkable or in entity block set. | Likely approximate: binary fallback is inside `Find_Path`, after destination `Can_Enter_Cell`, not a generic pre-command nearest-walkable redirect. |
| `src/sim/movement/movement_path.rs::resolve_requested_move_goal` | Uses `is_any_layer_walkable` for ground movers and `nearest_walkable_any_layer` / simple ring search for water movers. | Missing known FNPC contract: internal radius from speed+sight capped at 32, rect/validator config, direct/indirect candidate split, optional target-mode choice, and `FUN_0042D170` scoring/reachability. |
| `src/sim/movement/movement_path.rs::try_repath_after_block` | On blocked repath, uses `resolve_requested_move_goal(..., max_radius=10)` and mutates `target.final_goal` if redirected. | Likely too aggressive/too persistent: binary target substitution conditions and whether NavCom/final destination are overwritten remain unresolved. |
| `src/sim/miner/miner_dock_sequence.rs::find_nearby_passable_cell_with_index` | Has a local nearby-passable approximation for refinery/miner use. | Not reusable as exact `Find_Path` fallback: caller config and target selection differ; FNPC internals were non-scope here. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::Find_Path @ 0x004D3920` entry and normal A* integration | touched-not-exhausted | prior reports decompiled it; no live Ghidra in this session | Fresh branch-level decompile of `0x004D3920..0x004D41F2` |
| Destination `Can_Enter_Cell` probe | touched-not-exhausted | `PATHFINDING_ASTAR_GHIDRA_REPORT.md` | Exact arguments and branch targets for code `6`/`7` |
| Code `6`/`7` semantics | verified for A* code table, partial for Find_Path branch | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` | Exact meaning at the destination probe site |
| `Find_Path -> Find_Nearby_Passable_Cell` | touched-not-exhausted | `PATHFINDING_ASTAR_GHIDRA_REPORT.md`, FNPC caller matrix remaining uncertainty | Exact 15 stack args and substitution condition |
| `Find_Path -> FUN_0042D170` | verified for existence/integration, not full caller use | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` (`0x004D3C9C`, `0x0042D222`) | Exact role in candidate ranking/substitution |
| `direct_distance + 6` tolerance claim | conflict-needs-resolution | searched docs; no proof found | Fresh decompile or assembly walk |
| FNPC internals | deferred | user non-scope; existing FNPC docs | none for this report |
| Full A* | deferred | user non-scope; existing A* docs | none for this report |
| Current Rust goal redirect | touched-not-exhausted | code scan of movement files | Behavioral tests against binary facts once exact branch is known |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-FPBD-001 -- Is this a bounded slice? -> yes, only Find_Path blocked-destination pre-A* target fallback.` (evidence: user scope)
- `[RESOLVED] OQ-FPBD-002 -- Is `Find_Path` live in standard YR? -> yes, called by drive/walk movement path requests.` (evidence: `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-FPBD-003 -- Is `FUN_0042D170` live from `Find_Path`? -> yes, prior report records xref at `0x004D3C9C` and helper call to `Zone_precheck` at `0x0042D222`.` (evidence: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-FPBD-004 -- Is FNPC caller stack arity known? -> yes globally: 15 stack args after this/out, `RET 0x3c`.` (evidence: `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-FPBD-005 -- Is the `Find_Path` FNPC call fully decoded in prior docs? -> no, the caller matrix explicitly lists `FootClass::Find_Path` as unresolved.` (evidence: FNPC caller matrix Remaining Uncertainty)
- `[RESOLVED] OQ-FPBD-006 -- Does FNPC caller control required layer/height? -> no, FNPC internally passes `-1` to `CheckPassability`.` (evidence: FNPC caller matrix)
- `[RESOLVED] OQ-FPBD-007 -- Does FNPC final occupancy check reservations? -> no, it calls `CheckOccupancy(rect, -1)` when enabled.` (evidence: FNPC caller matrix)
- `[RESOLVED] OQ-FPBD-008 -- Is `direct_distance + 6` verified? -> not from available docs; treat as unverified.` (evidence: doc search)
- `[RESOLVED] OQ-FPBD-009 -- What Rust surface approximates this today? -> `movement_path.rs::resolve_requested_move_goal` and callers in `movement_commands.rs`/`try_repath_after_block`.` (evidence: code scan)
- `[DEFERRED] OQ-FPBD-010 -- Exact `Find_Path -> FNPC` stack argument order and constants.` (category: needs-runtime-debugger; reason: no Ghidra MCP/live decompile endpoint in this session; next-step-if-pursued: assembly-walk pushes before the `0x0056DC20` call inside `0x004D3920`)
- `[DEFERRED] OQ-FPBD-011 -- Exact substitution condition for code `6` vs code `7`.` (category: needs-runtime-debugger; reason: prior docs are inconsistent/stale at this branch; next-step-if-pursued: decompile the destination-probe block and record both branch arms)
- `[DEFERRED] OQ-FPBD-012 -- Whether substitution overwrites NavCom/final destination or only A* local target.` (category: needs-runtime-debugger; reason: requires branch-local field write trace; next-step-if-pursued: inspect writes around the fallback call and post-A* copy)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Blocked-destination target correction is in/under `Find_Path`, not only command issue. | `PATHFINDING_ASTAR_GHIDRA_REPORT.md`; `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` | mismatch/approximate: Rust redirects before A* in command and blocked-repath wrappers. | `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_path.rs` | Move parity-sensitive target substitution into the pathfinding request path once exact branch is decoded. | Proposed test: `find_path_redirects_blocked_destination_before_astar`. | Do not redirect every move goal through a generic nearest-walkable helper; the binary first probes `Can_Enter_Cell`. |
| `FUN_0042D170` participates in `Find_Path` fallback and uses `Zone_precheck`; failed precheck yields huge distance. | `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` | missing: Rust nearest-goal search does not zone-score candidates through this helper. | `src/sim/movement/movement_path.rs`; possibly `src/sim/pathfinding/zone_search.rs` | Candidate substitution must reject or deprioritize zone-unreachable candidates according to decoded helper behavior. | Proposed test: `find_path_fallback_rejects_nearby_cell_in_unreachable_zone`. | Do not select the geometrically nearest passable cell if the helper would score it as unreachable. |
| FNPC radius is internal to foot object speed+sight, capped at 32; not caller max radius. | `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`; caller matrix | mismatch: Rust uses hardcoded `max_radius=10` for move/repath goal redirects. | `resolve_requested_move_goal` or future FNPC wrapper | Use decoded FNPC contract for this fallback rather than a fixed radius. | Proposed test: `find_path_blocked_goal_uses_speed_sight_radius_cap_32`. | Do not bake `10` as the Find_Path fallback search radius. |
| FNPC final occupancy, if enabled, uses `CheckOccupancy(rect,-1)` and ignores reservation layer. | FNPC caller matrix | unchecked for current movement redirect; miner helper checks its own occupancy model. | future FNPC config / movement fallback tests | Preserve reservation-layer skip for FNPC-style placement. | Proposed test: `find_path_fallback_final_occupancy_ignores_reservation_layer`. | Do not use movement reservation masks as if they were FNPC final occupancy. |
| `direct_distance + 6` is not verified by the available evidence. | doc search; no live decompile | unknown | test names only until binary decode | Treat this as a research gap, not an implementation constant. | Proposed test after verification: `find_path_fallback_respects_decoded_distance_tolerance`. | Do not implement `+6` from memory or stale notes without fresh binary evidence. |

### Stale Docs / Follow-up Docs

- `PATHFINDING_ASTAR_GHIDRA_REPORT.md` section 2 says code `7` is "building entrance"; newer Can_Enter_Cell/A* reports identify code `7` as impassable/not-expanded. Replacement wording: "Special-case branches exist for destination `Can_Enter_Cell` codes including `6` and `7`; exact code-7 target-redirection semantics require branch-level verification."
- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` parameter table should not be used for caller-specific `Find_Path` arguments; the caller matrix supersedes global arity/layer/occupancy facts and explicitly leaves `FootClass::Find_Path` unresolved.
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` correctly flags "Find_Path nearby-cell fallback (code 6/7)" as partial; this report does not close that gap because live Ghidra was unavailable.

## Negative Facts / Do Not Do

- Do not re-investigate or reimplement FNPC internals from this report; use the existing FNPC reports for internals.
- Do not treat Rust `nearest_walkable_any_layer(max_radius=10)` as a verified mirror of `Find_Path` blocked-destination fallback.
- Do not implement the `direct_distance + 6` tolerance claim until a fresh `0x004D3920` branch decompile proves it.
- Do not classify code `7` as a building entrance based on the older A* report without reconciling it against newer Can_Enter_Cell code-table reports.
- Do not make fallback substitution ignore zones; `FUN_0042D170` reaches `Zone_precheck`.

## Remaining Uncertainty

1. Exact `Find_Path` code block addresses for destination probe branches.
2. Exact branch condition: whether code `6`, code `7`, or both call FNPC.
3. Exact 15 FNPC stack args from the `Find_Path` callsite.
4. Exact role of `FUN_0042D170`: candidate prefilter, score, tolerance gate, or final substitution validator.
5. Whether the substituted target is local to A* or stored back into a movement destination/NavCom field.
6. Whether any `direct_distance + 6` tolerance exists, and if so whether it is cells, leptons, squared distance, or helper return units.

## Sources

- `PATHFINDING_ASTAR_GHIDRA_REPORT.md`
- `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`
- `FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
- `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`
- `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`
- `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- `FOOTCLASS_STRUCT_LAYOUT.md`
- `ini/rulesmd.ini`
- Rust scan: `src/sim/movement/movement_path.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/miner/miner_dock_sequence.rs`
