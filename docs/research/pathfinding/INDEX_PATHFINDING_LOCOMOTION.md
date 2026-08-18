# Pathfinding & Locomotion — Research Index

Exhaustive checklist of every pathfinding/locomotion subsystem in `gamemd.exe`.
Each entry tracks coverage status and links to the deepest existing doc.

**Status legend:**
- `DONE` — a dedicated doc exists, verified against the binary with the project's
  RE standards (3-axis confidence per finding, caller traces, TS-legacy filtering).
- `PARTIAL` — a doc exists but is shallow, missing confidence axes, missing
  caller traces, or only covers a slice (e.g. bridges-only). Needs deepening.
- `TODO` — no dedicated doc, or only tangential coverage scattered across other docs.
- `DEFERRED-TS` — confirmed dormant TS legacy, not active in standard YR play.
  Stub documented so future investigators don't re-research it.
- `IN-PROGRESS` — currently being researched.

**Priority order for picking next TODO:**
1. Follow-ups from existing docs (anything marked "needs verification" / "see also")
2. Locomotor classes (Drive, Hover, Fly, Jumpjet, Ship, Teleport, Tunnel, Mech, DropPod, Rocket, Walk)
3. Pathfinder core
4. Cell/zone/passability
5. Movement missions
6. INI movement keys

---

## 1. Locomotor Classes (ILocomotion vtable implementations)

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| `LocomotionClass` (abstract base) | PARTIAL | `ILOCOMOTION_COM_PROTOCOL_SPEC.md`, `LOCOMOTION_MATH_AND_CONSTANTS.md` | Verify base vtable layout, default impls, COM ref-count protocol; verify `LocomotionClass__Push` / `Shove` semantics |
| `DriveLocomotionClass` | DONE | `DRIVE_LOCOMOTION_CLASS.md`, `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`, `DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md`, `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`, `DRIVE_SHARP_TURN_FALLBACK_RE.md`, `DRIVE_TRACK_SYSTEM.md`, `DRIVE_TRACK_TABLES_DEEP_DECODE.md`, `PROCESS_DRIVE_TRACK_DECOMPILATION.md` | Deeply covered |
| `ShipLocomotionClass` | DONE (2026-05-17) | **`SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`** + `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` + `NAVAL_IMPLEMENTATION_PLAN.md` + `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` + `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` | Full struct + 40-slot vtable + all method bodies + INI bindings + 9-item diff to Drive (incl. 2 corrections to prior docs) |
| `HoverLocomotionClass` | DONE | `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`, `BRIDGE_HOVER_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md` | Verify confidence axes per finding |
| `FlyLocomotionClass` | DONE | `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md` | Verify confidence axes |
| `JumpjetLocomotionClass` | DONE | `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`, `BRIDGE_JUMPJET_ABORT_FLAG_WRITERS_GHIDRA_REPORT.md`, `BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md` | Verify confidence axes |
| `WalkLocomotionClass` | DONE (2026-05-17) | **`WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`** + `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` | Full struct (60 bytes) + 40-slot vtable + all 17 Walk-specific method bodies + sub-cell placement algorithm + INI bindings for 64 infantry units + 8-item diff to Drive/Ship |
| `TeleportLocomotionClass` | DONE | `TELEPORT_LOCOMOTION_DEEP_DIVE.md`, `TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md`, `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` | Verify post-warp validation completeness |
| `DropPodLocomotionClass` | DEFERRED-TS (2026-05-17) | `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §3, `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` (bridges only) | Confirmed: zero INI refs to CLSID `4A582745`. Class factory registered but never instantiated in YR. |
| `RocketLocomotionClass` | DONE | `ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` | V3 rocket bullet locomotion — verify confidence axes |
| `TunnelLocomotionClass` | DEFERRED-TS (2026-05-17) | `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §4, `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` (peripheral) | Confirmed: zero INI refs to CLSID `4A582743`. Per `[[feedback_no_tunnel_subterranean]]` — TS legacy, dormant in YR. |
| `MechLocomotionClass` | DEFERRED-TS (2026-05-17) | `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md` §2 | Confirmed: 8 historical-comment INI refs (all commented/annotated), zero live uses. Class factory registered but never instantiated. TS-only mech locomotion for Cyborg/Wolverine-style units. |

## 2. Pathfinder Core

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| `PathfindClass` overall | DONE | `PATHFINDERCLASS_GHIDRA_REPORT.md` | Verify confidence axes per finding |
| A* implementation (open/closed set, heuristic, cost function) | DONE | `PATHFINDING_ASTAR_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` | |
| Standalone pathfinding helpers | DONE | `PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` | |
| Cell-entry verification (can-enter checks during path) | DONE | `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` | |
| Path smoothing & speed ramping | DONE | `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` | |
| Find nearby passable cell | DONE | `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` | |
| Repath triggers (collision, new obstacle, target moved) | DONE (2026-05-17 — covered by stuck synthesis) | `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` §8 enumerates all 5 trigger sites with urgency levels; `STUCK_DETECTION_SYNTHESIS.md` adds 3-axis confidence | 5 trigger sites confirmed: (1) path_queue empty + head_to set, (2) Code 2 + movement_delay expired (urgency 1/2), (3) path drained + dist > 0x200, (4) is_retry recursive re-entry, (5) tether/linked Pathfinding_update_continued loop |
| Stuck detection & unstick logic | DONE (2026-05-17) | **`STUCK_DETECTION_SYNTHESIS.md`** + `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` (primary source) | 4-state machine (Code 2 / Code 6 / Code 7 / hard give-up). Dual timers (movement_delay + blocked_delay). Path_stuck_counter circuit breaker. **Asymmetric `path_blocked_flag` clear** — Walk clears on each sub-cell arrival; Drive/Ship/Hover/Jumpjet NEVER clear (verified parity-critical detail). |
| Formation / group-move pathing | DONE (2026-05-17) | `MOVEMENT_CLASSIFIERS_REFERENCE.md` §7 + `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` + `ZONE_PASSABILITY_VERIFIED.md` §7 | Team-pathfinding uses `ZoneMap::FindBestCompatibleMovementZone @ 0x5889F0` — picks the most permissive MovementZone row compatible with all team members. Called from `TeamTypeClass::ComputeZoneCategory @ 0x6F1FA0` at scenario init. Convoy doc covers spawned convoys (TS map triggers). For player-issued group moves, each unit pathfinds independently after the group target is set — no "team-level" path is computed at click time. |
| Dynamic obstacle handling (moving units, new buildings) | DONE (2026-05-17) | `STUCK_DETECTION_SYNTHESIS.md` + `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` | No separate "obstacle appeared" handler. Mid-path obstacles trigger the next `Can_Enter_Cell` call → codes 2/4/5/6/7 → same stuck/repath state machine as if the unit was blocked from the start. The 5 repath-trigger sites documented in source doc §8 fire identically for dynamic obstacles as for static. |
| Path cache / reuse | DONE (2026-05-17) | `PATHFINDING_ASTAR_GHIDRA_REPORT.md` + verification this pass via `FootClass::Find_Path @ 0x4D3920` decompile | **No path caching.** Every `Find_Path` call runs fresh `Run_AStar`. The `+0x640` movement_delay timer is a CALL-RATE-LIMITER (prevents pathfinder thrashing), not a result cache. After Run_AStar returns, results copy directly into the 24-entry path queue at `FootClass+0x178`. A unit's path persists in `+0x178..+0x178+24*4` until exhausted or invalidated, but no cross-unit or cross-frame cache. |
| Aircraft pathing (no-cell-traversal) | DONE (2026-05-17) | `FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md` (already comprehensive) | Aircraft fly along **current facing direction**, NOT directly toward goal. Destination only sets the desired facing on FacingClass which gradually turns at ROT rate. Asm verified at `0x4CDA62`. Per-tick: `dx = cos(facing) × ScaledSpeed × CurrentSpeed; dy = sin(facing) × ScaledSpeed × CurrentSpeed`. No A*, no cell queue, no Can_Enter_Cell. Bridge Z handled by Begin_Takeoff/Begin_Landing flight-level state. |

## 3. Cell / Zone / Passability

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| `CellClass` struct layout | DONE | `CELLCLASS_STRUCT_GHIDRA_REPORT.md`, `CELLCLASS_ZONES_SPEED_BRIDGES.md` | Verify field-by-field confidence |
| Cell occupancy bits & ordering | DONE | `CELL_OCCUPANCY_ORDERING_GHIDRA_REPORT.md`, `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`, `CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md` | |
| Limbo & cell occupation lifecycle | DONE | `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` | |
| Cell validation (tiberium / placement) | DONE | `CELL_VALIDATION_TIBERIUM_PLACEMENT_REPORT.md` | |
| Subcell occupancy (infantry) | DONE | `INFANTRY_SUBCELL_POSITIONING.md` | Verify ordering of 5 subcells |
| Zone map build & level | DONE | `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` | |
| Zone passability verification | DONE | `ZONE_PASSABILITY_VERIFIED.md` | |
| Zone incremental divergence | DONE | `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md` | |
| `MovementZone` enumeration & derivation | DONE (2026-05-17) | **`MOVEMENT_CLASSIFIERS_REFERENCE.md`** + `ZONE_PASSABILITY_VERIFIED.md` (source) + `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` | 13 values enumerated with passability profiles, full 13×8 matrix verified via read_memory, parsing site `0x474E40` confirmed, INI binding `MovementZone=` → `TT+0x5B4`. **Note typo: binary expects `Subterannean` (mis-spelled), not `Subterranean`.** |
| `SpeedType` enumeration & terrain speed multipliers | DONE (2026-05-17) | **`MOVEMENT_CLASSIFIERS_REFERENCE.md`** + `ZONE_PASSABILITY_VERIFIED.md` (source) | 8 values enumerated, parsing site `0x476FC0` confirmed, INI binding `SpeedType=` → `TT+0x67C`. Speed table `g_SpeedType_LandType_Table` indexed `[SpeedType + LandType*9]` (stride 9 — 9th slot unused padding). Cliff multipliers `RulesClass+0x768/0x770/0x778/0x780` applied AFTER table lookup (already documented in Ship/Walk locomotor docs). |
| Bridge low-and-zone records | DONE | `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md` | |
| Bridge can-enter-cell hierarchy | DONE | `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, `BRIDGE_RUNTIME_CAN_ENTER_CELL_ARGUMENTS_GHIDRA_REPORT.md`, `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md` | |
| Bridge zone helpers & lifecycle | DONE | `BRIDGE_ZONE_HELPERS_GHIDRA_REPORT.md`, `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`, `BRIDGE_ZONE_EDGE_FLAGS_GHIDRA_REPORT.md` | |
| Bridge check traversal & cell offsets | DONE | `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` | |
| Bridge deferred mechanics | DONE | `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` | |
| Bridge locomotor noncoverage justification | DONE | `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md` | |
| Unit can-enter-cell (UnitClass) | DONE | `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` | |
| Crush system | DONE | `CRUSH_SYSTEM_GHIDRA_REPORT.md` | |
| Naval zone legality | DONE | `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` | |
| Cliff / ramp traversal rules | DONE (2026-05-17) | **`CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`** | Three slope cell-bytes (RampType +0x11A / Level +0x11B / SlopeIndex +0x11C). SlopeIndex 0=flat, 1-4=cardinal smooth ramps, 5-19=steep slopes. Cliff = LandType=3 (Rock, impassable). Speed factor cached at FootClass+0x530 (consulted by Zone_precheck, path smoothing, straight-line reroute — NOT by Process_Movement). Cliff-multipliers (RulesClass+0x768/0x770/0x778/0x780) applied per-tick in Process_Movement when going up/down. Height-diff ≥ 2 triggers "vaulting" LandType=Clear override. |
| Water-shore edge transitions | DONE (2026-05-17) | **`WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`** | Mechanism is entirely matrix-based: Beach (LandType=6) → ZoneType=3, only 4 amphibious-family MovementZones (3/4/5/11) pass col 3. Naval can't reach shore (Water row blocks Beach). `g_ShorePieces` base tile ID covers 42-tile shore set per theater (verified via `IsShorePieceTile @ 0x4865B0`). Stock: SEAL/Tanya/Yuri Prime use AmphibiousDestroyer; SAPC/Robot Tank use Amphibious; no stock unit uses WaterBeach. Westwood comments preserved: "seal stuck on tree bug" workaround. |
| Layer system (ground / air / underground) | DONE (2026-05-17) | **`LAYER_SYSTEM_GHIDRA_REPORT.md`** | **TWO distinct enums conflated:** (1) Display-sort layer (5 LayerClass instances at `g_DisplayLayers @ 0x8A0360`, indices 0-4) returned by `ILocomotion::In_Which_Layer` (slot 29) — Drive/Ship/Walk=2 always, Fly=2 or 4 by altitude, JumpJet=2/3/4 with bridge altitude adjustment. (2) Cell-feature enum: 3 strings "Ground"/"Surface"/"Underground" at `0x81DB84` — consumer NOT identified (open question). Layers 0 and 1 of display-sort are effectively empty in standard YR (TS-legacy reserved). |
| `TODO_ZONE_FIDELITY_FIXES` follow-ups | TODO | `TODO_ZONE_FIDELITY_FIXES.md` | Open follow-ups from prior zone work — audit and resolve each |

## 4. Movement Missions (FootClass / MissionClass state machines)

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| `MissionClass` state machine overall | DONE | `MISSIONCLASS_STATE_MACHINE.md` | |
| `FootClass` complete | DONE | `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md`, `FOOTCLASS_STRUCT_LAYOUT.md`, `FOOTCLASS_VTABLE_COMPLETE.md`, `FOOTCLASS_REINVESTIGATION_2026-04-24.md` | |
| FootClass AI | DONE | `FOOTCLASS_AI_GHIDRA_REPORT.md` | |
| FootClass pathfinding & movement | DONE | `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` | |
| FootClass mission handlers (all) | DONE | `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` | |
| `Mission_Move` (FootClass) | DONE | `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` | |
| `Mission_Attack` (FootClass) | DONE | `FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` | |
| `Mission_Enter` (refinery dock, crosswalk) | DONE | `MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md`, `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`, `MISSION_ENTER_REFINERY_DOCK_VERIFICATION_NOTES.md` | |
| `Mission_Guard` / `Mission_Area_Guard` | DONE | `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md` | |
| `Mission_Harvest` | DONE | `MISSION_HARVEST_GHIDRA_REPORT.md`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` | |
| `Mission_Repair` / `Mission_Produce` | DONE | `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md` | |
| Scatter system (Mission_Move side effects, dispatch) | DONE | `SCATTER_ALL_CALLERS_GHIDRA_REPORT.md`, `SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md`, `SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md` | |
| FootClass enter queue & navcom | DONE | `FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md` | |
| FootClass flags block | DONE | `FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md` | |
| FootClass non-movement fields | DONE | `FOOTCLASS_NON_MOVEMENT_FIELDS.md` | |
| `Mission_Retreat` / FleeToCell logic | TODO | (none) | |
| `Mission_Stop` / `Mission_Sleep` (movement halt) | TODO | (none) | |
| `Mission_Hunt` (movement portion) | TODO | (none) | |
| `Mission_Patrol` | TODO | (none) | Verify whether reachable in YR or TS-legacy |
| `Mission_Capture` (engineer movement to target) | PARTIAL | `ENGINEER_CAPTURE_GHIDRA_REPORT.md` | Verify whether movement portion is fully covered |
| `Mission_Deploy` (UnitClass) | DONE | `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` | |
| `Mission_Unload` (transports) | DONE (2026-05-17) | **`MISSION_UNLOAD_GHIDRA_REPORT.md`** | Mission 16 has 3 forms: FootClass stub (`0x4DA2B0` returns 450), UnitClass override (`0x740EF0` for ground transports), AircraftClass uses different missions 30/31 (ParaDropApproach + ParaDropOverfly). Two-stage unload flow: find spot via `vtable+0x528`, assign Mission_Move (2) to it, queue Mission_Enter (7) for docking. Paradrop drops every 3 frames in Overfly mission; switches at 769-lepton distance threshold. INI: `UnloadingClass=`, `Passengers=`, plus inferred `TransportUnloadRange=` at RulesClass+0x850. |

## 5. ILocomotion COM Protocol & Math

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| ILocomotion COM protocol spec | DONE | `ILOCOMOTION_COM_PROTOCOL_SPEC.md` | |
| Locomotion math & constants | DONE | `LOCOMOTION_MATH_AND_CONSTANTS.md` | |
| Push/Shove caller provenance | DONE | `LOCOMOTION_PUSH_SHOVE_CALLER_PROVENANCE_GHIDRA_REPORT.md` | |
| IPiggyback (locomotor swap, e.g. infantry on transports) | TODO | (none) | Drive/Teleport implement IPiggyback — when is one locomotor piggybacked on another? |

## 6. INI Movement Keys

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| `[General]` movement constants | TODO | (partial in `READINI_FIELD_MAPS.md`) | Need exhaustive list: TurnRate, Acceleration, Crawl%, Crush%, ScatterDistance, etc. |
| Per-unit `Speed=`, `Locomotor=`, `MovementZone=`, `SpeedType=` parsing | TODO | (scattered) | Where parsed, what Locomotor CLSIDs map to which class, default values |
| Locomotor CLSID → class binding table | TODO | (none) | Definitive table mapping GUID → ILocomotion impl |
| `ZoneConnectionClass` / `SubzoneConnectionStruct` | TODO | (none) | Zone-connection runtime structures (TS legacy candidate — verify) |
| Init-function dispatch table (`0x812D50` Drive / `0x814A58` Ship) | TODO | (none) | Mechanism that calls `Compute_*HeightStep` / `Compute_*BridgeZOffset` / `InitNullCoords` at boot — likely shared across all locomotor classes |
| `g_SpeedType_LandType_Table` layout & values | TODO | (none) | 2D float table indexed `[LandType * 9 + SpeedType]` — used for base speed lookup. Stride of 9 suggests 9 SpeedTypes; row count = number of LandTypes. Verify exact layout, values, and INI source (rules.ini `[LandType]` sections) |
| `RulesClass+0x768/+0x770/+0x778/+0x780` cliff multipliers | TODO | (none) | Track-up/non-track-up/track-down/non-track-down speed multipliers in `[General]`. Likely keys `TrackedDownhillSpeed`, `TrackedUphillSpeed`, etc. Verify ReadINI binding |
| `FootClass+0x68A/+0x68B/+0x6B7` movement flags | TODO | partial | Stuck-sound flag, bridge-transition flag, blocked-state flag — set/cleared at multiple Process_Movement sites |

## 7. Air / Special Movement

| Topic | Status | Doc(s) | Notes |
|---|---|---|---|
| Aircraft takeoff / landing sequence | TODO | (scattered in FlyLocomotion doc) | Verify dedicated doc exists or extract into one |
| Parachute descent (locomotor / mission) | PARTIAL | `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` (rendering only) | Movement portion: how the parachute coordinates its descent — TODO |
| WarMiner locomotion integration | DONE | `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` | |
| Chrono miner teleport | DONE | `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`, `CHRONO_MINER_SYSTEM_OVERVIEW.md` | |
| AlphaShape / building light follow logic | DONE | `ALPHA_SHAPE_CLASS_LIFECYCLE.md` | Not strictly movement but follows units — already covered |

## 8. Audit / Verification Backlog

| Topic | Status | Notes |
|---|---|---|
| Confidence-axis audit on all DONE locomotor docs | TODO | Per `[[feedback_research_confidence_axes]]` — each function citation must have content/identity/binding axes. Many older docs lack this. |
| Caller-trace audit on all DONE docs | TODO | Per `[[feedback_caller_trace_before_finding]]` — verify every "HIGH binding" claim has `get_function_callers` evidence. |
| TS-legacy filter pass on all DONE docs | TODO | Per CLAUDE.md — flag any code path not confirmed reachable in standard YR skirmish. |

---

## Iteration log

(Append one line per `/re-investigate` run with date, topic, outcome.)

- 2026-05-17 — **ShipLocomotionClass** — DONE. Produced `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. All 11 distinct Ship methods decompiled (Constructor, 2 destructors, Process, Process_Movement, plus 8 ILocomotion overrides). Vtable verified by `read_memory`. INI bindings verified for 8 stock ship units. Corrections logged to `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` (slot 31 mis-label, +0xD69 wake check) and `NAVAL_SYSTEM_RESEARCH.md` (shared-function-address stale claim). New subtopics surfaced for index: `[Init dispatch table mechanism]`, `[g_SpeedType_LandType_Table layout]`, `[RulesClass+0x768..0x780 cliff multipliers]`, `[FootClass+0x68A/+0x68B/+0x6B7 movement flags]`.
- 2026-05-17 — **Mech + DropPod + Tunnel** — DEFERRED-TS. Produced consolidated `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md`. Each verified TS-dormant: Mech has 8 historical-comment INI refs (all `;commented` or `<-drive mech->` annotations); DropPod has zero refs; Tunnel has zero refs. All three: class factory registered in WinMain, constructor analyzed by Ghidra, but no `CoCreateInstance` ever invoked in stock YR — vtable methods exist as code but Ghidra never auto-analyzed them because no path reaches them. Decision: do NOT implement in Rust. Three index entries flipped from TODO/DEFERRED-TS-stub to DEFERRED-TS with the new doc reference.
- 2026-05-17 — **WalkLocomotionClass** — DONE. Produced `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. Smaller class than Ship (60 bytes vs 108 bytes). All 17 Walk-specific methods decompiled (constructor, destructor, Process, ProcessMovement ~1100 lines, FindSubCellDest, Mark_All_Occupation_Bits, Head_To_Coord, Stop_Moving, etc.). Sub-cell placement algorithm fully documented. Key findings: no drive-track tables, angle-based stepping via atan2, hard-stop instead of decelerate, missing IsBeingWarpedIn vtable guard (only 3 of 4), TS-tube traversal path conditionally active (path_index==8 + g_TubeArray). Slot disagreement with Drive/Ship: Walk's Mark_All_Occupation_Bits at slot 30 vs Drive's slot 14 — flagged as open question. New subtopics surfaced: `[FacingClass__UpdateFacing internals]`, `[CellClass::PlaceInfantryInCell algorithm]`, `[techno+0x578 shadow-double semantic]`, `[Mission IDs 7/8/9/0xB/0x19 enum names]`.
- 2026-05-17 — **Stuck detection & unstick logic** — DONE. Produced `STUCK_DETECTION_SYNTHESIS.md` (consolidates the existing `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` 2026-04-05 doc into a state-machine view with 3-axis confidence labelling). 4-state machine documented: State A (Code 2 blocked, dual-timer urgency 0→1→2), State B (Find_Path failure, path_stuck_counter circuit-breaker), State C (Code 7 deadlock), State D (Code 6 close-enough graceful arrival). Most parity-critical finding: **`path_blocked_flag` (+0x6B7) is cleared ONLY by Walk locomotor, never by Drive/Ship/Hover/Jumpjet** — meaning vehicles get permanently impatient after their first block event. Repath-triggers entry also marked DONE (same coverage). New open question: `RulesClass+0x1724` bridge-stuck override in AreaGuard.
- 2026-05-17 — **Dynamic obstacles + Path cache + Aircraft pathing** — DONE (3 TODOs in one iteration). Verification rather than new docs: dynamic obstacles use the same stuck-state machine; no path caching exists (every Find_Path runs fresh A*, the +0x640 timer is a rate-limiter); aircraft pathing is facing-based not cell-based (already covered by FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md §1). Decompiled `FootClass::Find_Path @ 0x4D3920` to verify no cache logic. Pathfinder-core priority (c) now substantially complete except Formation/group-move pathing (PARTIAL).
- 2026-05-17 — **MovementZone + SpeedType + Formation/group-move** — DONE (3 TODOs in one iteration). Produced `MOVEMENT_CLASSIFIERS_REFERENCE.md` consolidating MovementZone (13 values), SpeedType (8 values), ZoneType (8 values), LandType (12 values), and how they interrelate through the 13×8 passability matrix + the speed multiplier table. Passability matrix bytes verified via `read_memory 0x82A594 len 416`. Enum string tables read at `0x81BAF4` (MovementZone) and `0x81DBA8` (SpeedType). Confirmed binary typo: `MovementZone=Subterannean` (mis-spelled) is the canonical INI token. Formation/group-move uses `FindBestCompatibleMovementZone @ 0x5889F0` for team-level zone selection. Priority (d) cell/zone/passability — 3 of 6 TODOs closed.
- 2026-05-17 — **Cliff / ramp traversal rules** — DONE. Produced `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`. 12 functions decompiled this pass (Get_Slope_Speed_Factor, Get_Slope_Cost_At_Cell, GetGroundHeight, GetEffectiveHeight, IsOnBridgeRamp, IsBridgeRampTile, ApplyLAT_and_SlopeFixup, TMP_ReadSlopeType, ForEach_SetSlopeIndex, Zone_Estimate_Slope_Cost, BridgeSlopeTable_StaticInit). Three slope-related cell bytes identified: RampType @+0x11A (bridge orientation), Level @+0x11B (signed height), SlopeIndex @+0x11C (0=flat, 1-4 smooth cardinal, 5-19 steep). Effective height formula: `Level + (Flags & 0x80 ? 4 : 0)` — bit 7 (0x80), distinct from on-bridge bit 8 (0x100). Slope speed factor cached at FootClass+0x530, consulted by zone pathfinder (Zone_precheck/path smoothing/reroute), NOT by Process_Movement. Cliff-multipliers in RulesClass+0x768/0x770/0x778/0x780. **Open Q's:** exact SlopeIndex→speed-factor mapping, INI key names for cliff mults, BridgeSlopeTable contents. Priority (d) — 4 of 6 TODOs closed; remaining: water-shore edge transitions, Layer system, TODO_ZONE_FIDELITY_FIXES follow-ups.
- 2026-05-17 — **Water-shore edge transitions** — DONE. Produced `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`. Mechanism is entirely matrix-based: only 4 of 13 MovementZones pass Beach (col 3) — AmphibiousDestroyer/AmphibiousCrusher/Amphibious/WaterBeach. Beach cells form the choke point between Water and Ground zones. `IsShorePieceTile @ 0x4865B0` decompiled (range check against `[g_ShorePieces, g_ShorePieces+42)`). Stock-unit roster confirmed by INI grep: only SEAL/Tanya/Yuri Prime use AmphibiousDestroyer (per Westwood "seal stuck on tree bug" annotation); only Hovercraft/Robot Tank use Amphibious; zero stock units use WaterBeach (commented-out alternative only). Naval can never reach shore — MovementZone=Water blocks Beach. Priority (d) — 5 of 6 TODOs closed; remaining: Layer system, TODO_ZONE_FIDELITY_FIXES follow-ups (implementation TODOs).
- 2026-05-17 — **Layer system** — DONE. Produced `LAYER_SYSTEM_GHIDRA_REPORT.md`. Disambiguated TWO conflated concepts: (1) **Display-sort layer** — 5 LayerClass instances at g_DisplayLayers (constructor verified count=5), returned by In_Which_Layer slot 29. Drive=2, Ship=2, Walk=2 unconditionally; Fly=2 or 4 (branchless altitude conditional); JumpJet=2/3/4 with bridge-altitude adjustment. (2) **Cell-feature enum** — 3 strings Ground/Surface/Underground at 0x81DB84 — consumer NOT identified (open question). Decompiled DriveLocomotionClass__In_Which_Layer, FlyLocomotionClass__In_Which_Layer, JumpjetLocomotionClass__In_Which_Layer, LayerClass__Constructor this pass. **Priority (d) — 6 of 6 TODOs closed (TODO_ZONE_FIDELITY_FIXES are Rust impl TODOs, not research).** Next iterations should pick up priority (e) movement missions or (f) INI keys.
- 2026-05-17 — **Mission_Unload** — DONE. Produced `MISSION_UNLOAD_GHIDRA_REPORT.md`. Mission 16 has 3 distinct forms verified: FootClass base stub (`0x4DA2B0` — 1-line return 450), UnitClass override (`0x740EF0` — 2-stage assign Mission_Move + queue Mission_Enter), AircraftClass uses missions 30/31 (ParaDropApproach + ParaDropOverfly) instead. Paradrop algorithm: approach at low rate until distance < 769 leptons (~3 cells), switch to 3-frame-per-call Overfly mission, eject passengers via vtable+0x488 each tick, escape via HouseClass::GetOppositeEdge after passenger queue empties. Decompiled 4 functions this pass. New open questions: vtable+0x528 (Find_Best_Unload_Cell) Mode 0/1 semantics, vtable+0x488 (Eject_Passenger) full body, IFV-specific gunner ejection path. Priority (e) — 1 of 6 TODOs closed.
