# Crate Authority Evidence and Passive Foundation Implementation Plan

> **For Codex:** Execute this plan task by task. Do not start live crate runtime
> work from this document.

**Goal:** Close every active-YR contract needed by the approved dedicated
`CrateAuthority` design and land only the already verified, non-activating rule
vocabulary and independent crate metadata flags.

**Architecture:** This is the first executable wave of the approved Approach A
design. It produces primary research, an evidence-ranked system model, and an
exact implementation contract. Its code portion adds passive rules data only.
It deliberately does not specify or implement the committed-cell driver,
`CrateAuthority`, placement, regeneration, pickup, effects, ingress,
presentation, session authority, or persistence. Those runtime interfaces must
be planned in a new `/write-plan` after the evidence gates close.

**Approved design:**
`docs/plans/2026-07-23-crate-authority-design.md`

**Stop status:** `PARTIAL/UNVERIFIED`

---

## Scope and Non-Activation Boundary

At the end of this plan:

- `GameOptions::crates` still has no gameplay consumer;
- `Simulation` has no `CrateAuthority`;
- no fixed slot table or runtime crate state exists;
- no initial, regenerated, dropped, or trigger-created crate is placed;
- no cell-entry callback or movement control flow changes;
- no crate is selected, removed, replaced, or picked up;
- none of the 19 outcomes executes;
- no crate sound, animation, EVA, trigger, damage, spawn, shroud, economy, or
  modifier consequence executes;
- raw `g_GameMode`, `Session::NumPlayers`, and native-frame plumbing remain
  unchanged;
- snapshot, replay, hash, and deterministic golden schemas remain unchanged;
- no crate code consumes Scenario, Main, or MapGen RNG; and
- visible overlay bytes remain solely in the existing `OverlayGrid`.

The plan is complete when the evidence and implementation-contract gates pass,
the passive types and flags compile and test, and the status remains explicitly
`PARTIAL/UNVERIFIED`.

---

## Grounding Summary

- The canonical active dispatch has 19 indices:
  `Money`, `Unit`, `HealBase`, `Cloak`, `Explosion`, `Napalm`, `Squad`,
  `Darkness`, `Reveal`, `Armor`, `Speed`, `Firepower`, `ICBM`,
  `Invulnerability`, `Veteran`, `IonStorm`, `Gas`, `Tiberium`, and `Pod`.
- Stock weights are
  `[20, 20, 10, 0, 0, 0, 0, 0, 10, 10, 10, 10, 0, 0, 20, 0, 0, 0, 0]`,
  totaling 110.
- `[Powerups]` token three is the over-water permission and token four is a
  native double. The native loader uses its own tokenization and
  `atoi`/`atof` path, not Rust's generic `read_double`.
- The native map authority owns exactly 256 slot records of 16 bytes, but the
  active meanings and constructor values of all four words are not closed.
- Live checks on 2026-07-23 verified that clearing a picked slot writes sentinel
  coordinates and `start_frame = -1` while preserving the remaining duration.
  Multiplayer performs one immediate first-free-slot replacement; there is no
  second delayed replacement.
- Live `0x00668BF0` evidence shows that native `[OverlayTypes]` identity follows
  section-entry ordinal and entry string value, not sparse numeric-key
  indexing. Stock current Rust compaction maps `CRATE` and `WCRATE`
  compatibly. Duplicate, descending, nonnumeric, empty, and base/patch edge
  behavior is not yet fully verified.
- Current Rust already stores visible overlay ID/data bytes in serialized and
  hashed `OverlayGrid`; a crate must not become a `GameEntity`.
- Current movement is an opaque ground batch plus separate special-movement
  batches. A correct runtime plan needs synchronous control transfer after
  every verified committed arrival.
- Current `for_each_live_object` already models native live-length reread and
  same-pass tail append. A crate subsystem must reuse that authority rather
  than create a private scheduler.
- Current session state lacks raw `g_GameMode` and authoritative
  `Session::NumPlayers`; app session mode is currently hardcoded to skirmish.
- Snapshot schema is 29 at planning time, but the worktree contains a broad
  uncommitted mission-authority migration across snapshot, hash, entity,
  world, and movement files.
- Stock `[CrateRules]` includes `CrateMaximum=255`, `CrateMinimum=1`,
  `CrateRadius=3.0`, `CrateRegen=3`, `SilverCrate=HealBase`,
  `SoloCrateMoney=5000`, `UnitCrateType=none`, `WoodCrate=Money`,
  `WaterCrate=Money`, `WoodCrateImg=CRATE`, `CrateImg=CRATE`,
  `WaterCrateImg=WCRATE`, `HealCrateSound=HealCrate`, and `FreeMCV=yes`.
- Stock `CRATE` and `WCRATE` both set `Crate=yes` and `CrateTrigger=yes`.
- The research index has no crate checksum mismatch after the 2026-07-23
  refresh. Focused validation still reports unrelated corpus link/status
  issues, so primary reports must be read directly.

---

## Decisions, Confidence, and Sources

| Decision | Confidence | Source |
|---|---|---|
| Keep the approved dedicated 256-slot `CrateAuthority` design for the later runtime wave | High | Approved design; active MapClass slot storage |
| Stop this plan before runtime movement/state interfaces | High | Exact caller, continuation, slot, effect, and session contracts remain open |
| Land canonical 19-way type identity now | High | Active pickup jump table and corrected crate report |
| Store powerup numeric data as `NativeF64Bits` | High | Native Powerups double field and `src/util/native_x87.rs` |
| Do not add `CrateRules` or a parser in this wave | High | Defaults, malformed input, patch behavior, resolved handles, and ownership require Task 3 |
| Parse `CrateGoodie`, `CarriesCrate`, `CrateBeneath`, and `CrateBeneathIsMoney` independently | High | Native field reads and stock INI |
| Parse `CrateTrigger` independently from `Crate` | High | Native overlay fields and stock `CRATE`/`WCRATE` |
| Preserve current Rust overlay registry ordering in this wave | High | Stock compatibility is established; mod-edge replacement contract is incomplete |
| Make no serialized/hashed/session/runtime-state change | High | Approved migration boundary and dirty snapshot ownership |
| Require a new reviewed runtime implementation plan | High | Current movement and effect interfaces cannot be specified exactly before Tasks 1-6 |

---

## Open Questions and Evidence Gates

The following are blocking questions, not implementation choices:

1. What do slot words `+0x00`, `+0x04`, `+0x08`, and `+0x0C` mean, and what
   exact bytes do all constructors, clears, failures, and saves produce?
2. How are map-authored crate overlays reconciled with slots during
   `Post_Map_Init`?
3. What exact call-stack stage invokes pickup for every active movement family?
4. Which movement values persist across a pickup callback, which are reread,
   and what happens if the collector is deleted, limboed, teleported, or
   retasked?
5. How do ground-to-bridge continuation and post-commit reservation failure
   behave across the callback?
6. What exact raw game-mode values and participant count feed initial placement?
7. What is the authoritative frame value in campaign, skirmish, LAN, and WOL?
8. What are the exact malformed/missing/duplicate `[Powerups]` semantics?
9. What are the duplicate, nonnumeric, empty, descending, and base/patch
   `[OverlayTypes]` semantics?
10. What is the complete ordered pickup transaction, including
    `CrateTrigger`, removal, immediate replacement, gameplay, animation, sound,
    EVA, and collector continuation?
11. What are the exact unit-death, building-unplace, and trigger-action ingress
    transactions?
12. What exact state, RNG, scan order, lifecycle, and presentation behavior
    applies to each of the 19 canonical outcomes?

Tasks 1-6 close these questions. Any unresolved load-bearing item prevents a
runtime `/write-plan`.

---

## Design Requirement Traceability

| Approved requirement | This plan | Runtime-plan input |
|---|---:|---|
| Canonical 19-type identity | Tasks 3, 5, 7 | Fixed enum and table |
| `[CrateRules]` and `[Powerups]` | Task 3; passive table in Task 7 | Exact parser and owner |
| Six independent rule flags | Tasks 3, 4, 8 | Parsed metadata |
| Fixed 256-slot authority | Tasks 1, 6 | Exact slot struct and persistence |
| Authored bootstrap and initial placement | Task 1 | Bootstrap transaction |
| Scenario RNG and x87 jitter | Tasks 1, 3, 5 | Ordered native fixtures |
| Synchronous per-cell pickup | Tasks 2, 6 | Per-family resumable driver contract |
| Regeneration before first factory step | Tasks 1, 3, 6 | Exact scheduler rung |
| All 19 outcomes | Tasks 5, 6 | Handler and shared-authority matrix |
| Unit/building/trigger ingress | Tasks 4, 6 | Exact transaction adapters |
| Presentation | Tasks 2, 5, 6 | Common and per-handler tails |
| Snapshot/hash/replay | Tasks 1, 3, 6 | One coordinated migration |
| Native-derived certification | Tasks 1-6 | Executable native result fixtures |

---

## File Map

### Research and Contract Artifacts

| Action | Path | Responsibility |
|---|---|---|
| Create | `docs/research/CRATE_SLOT_BOOTSTRAP_PLACEMENT_GHIDRA_REPORT.md` | Slot words, bootstrap, placement, timer, coordinates, and RNG |
| Create | `docs/research/CRATE_PICKUP_TRANSACTION_GHIDRA_REPORT.md` | Caller census and ordered pickup transaction |
| Create | `docs/research/CRATE_SESSION_FRAME_RULES_GHIDRA_REPORT.md` | Session/frame authority, Powerups parser, totals, overlay ordering |
| Create | `docs/research/CRATE_DROP_TRIGGER_INGRESS_GHIDRA_REPORT.md` | Unit, building, trigger, and specific-cell ingress |
| Create | `docs/research/CRATE_EFFECT_HANDLERS_GHIDRA_REPORT.md` | Reconciled 19-handler and presentation matrix |
| Create | `docs/research/CRATE_AUTHORITY_SYSTEM_MODEL.md` | Evidence-ranked system model |
| Create | `docs/research/CRATE_AUTHORITY_IMPLEMENTATION_CONTRACT.md` | Exact future Rust obligations and blockers |

### Passive Foundation Code

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/rules/crate_rules.rs` | Canonical type and passive Powerups value containers |
| Modify | `src/rules/mod.rs` | Export `crate_rules` |
| Modify | `src/rules/object_type.rs` | Store and parse four independent object flags |
| Modify | `src/map/overlay_types.rs` | Store and parse `CrateTrigger` without changing order |
| Modify | `src/sim/movement/locomotor_tests.rs` | Initialize new passive fields in its complete test literal |
| Modify | `src/sim/movement/teleport_movement.rs` | Initialize new passive fields in its complete test literal only |

### Held Runtime Surfaces

This plan does not modify:

- `src/sim/game_options.rs`;
- `src/sim/scenario_session.rs`;
- `src/sim/replay.rs`;
- `src/sim/snapshot.rs`;
- `src/sim/world/world_hash.rs`;
- `src/sim/world/world_spawn.rs`;
- `src/sim/game_entity.rs`;
- `src/sim/house_state.rs`;
- `src/sim/trigger_runtime.rs`;
- `src/sim/find_nearby_cell.rs`;
- `src/sim/overlay_grid.rs`;
- production code in `src/sim/movement/`;
- `src/sim/world/mod.rs`;
- `src/app_init.rs`;
- `src/app_sim_tick.rs`;
- `src/app_skirmish.rs`;
- `src/skirmish_launch.rs`;
- `src/render/`, `src/sidebar/`, `src/ui/`, `src/audio/`, and `src/net/`.

The two test-only movement literals named above are the only allowed `src/sim`
edits. If any other runtime surface becomes necessary, stop and write a new
plan after Task 6.

---

## Frozen Existing Research Inputs

These are the existing reports that Tasks 2-5 must read before opening Ghidra.
The new task reports supersede any stale or contradictory claim they contain.

### Movement and Pickup Inputs (Task 2)

- `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/FUN_00481A00_CRATE_PICKUP_WARP_ARRIVAL_GHIDRA_REPORT.md`
- `docs/research/GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`
- `docs/research/PROCESS_DRIVE_TRACK_DECOMPILATION.md`
- `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
- `docs/research/GROUND_PHASE1_LOCOMOTOR_POPULATION_AND_PRECEDENCE_GHIDRA_REPORT.md`
- `docs/research/OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_0XDC_RESERVATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/TELEPORT_LOCOMOTION_DEEP_DIVE.md`
- `docs/research/TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- `docs/research/chronominer-locomotion/fn-state-machine-tick.md`
- `docs/research/chronominer-locomotion/fn-initiate-warp.md`
- `docs/research/chronominer-locomotion/fn-update-position.md`

Current Rust files:

- `src/sim/movement/mod.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/teleport_movement.rs`
- `src/sim/movement/tube_movement.rs`
- `src/sim/movement/tunnel_movement.rs`
- `src/sim/movement/air_movement.rs`
- `src/sim/world/mod.rs`

### Session, Frame, Rules, and Replay Inputs (Task 3)

- `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/RULESCLASS_POWERUPS_TABLE.md`
- `docs/research/INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
- `docs/research/SESSIONCLASS_GHIDRA_REPORT.md`
- `docs/research/LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`
- `docs/research/CHOOSE_MAP_SELECTED_RECORD_LOAD_COMMITTED_GLOBALS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md`
- `docs/research/FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`
- `docs/research/FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md`
- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
- `docs/research/NATIVE_FRAME_RATE_WALLCLOCK_RECONCILIATION_GHIDRA_REPORT.md`
- `docs/research/NETWORK_FRAME_SCHEDULING_GHIDRA_REPORT.md`
- `docs/research/MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md`
- `docs/research/LIVE_SKIRMISH_PACING_PATH_GHIDRA_REPORT.md`
- `docs/research/ADVANCE_TICK_PHASE_PARTITION_NATIVE_SPINE_GHIDRA_REPORT.md`
- `docs/research/PER_FRAME_RNG_CONSUMPTION_ORDER_GHIDRA_REPORT.md`
- `docs/research/RNG_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`
- `docs/research/traces/SKIRMISH_CRATES_OFF_INITIAL_CRATES_TRACE.md`
- `docs/research/RULESCLASS_GHIDRA_REPORT.md`
- `docs/research/OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`

Current Rust files:

- `src/rules/ini_value.rs`
- `src/rules/ini_parser.rs`
- `src/sim/scenario_session.rs`
- `src/sim/replay.rs`
- `src/app_init.rs`
- `src/app_skirmish.rs`
- `src/app_sim_tick.rs`
- `src/skirmish_launch.rs`

### Lifecycle and Ingress Inputs (Task 4)

- `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/SPECIAL_FLAGS_SYSTEM.md`
- `docs/research/DAMAGE_SIGNED_VITALITY_WRITER_AND_FATAL_HANDOFF_GHIDRA_REPORT.md`
- `docs/research/DAMAGE_RECEIVER_OBJECT_CALLBACKS_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_RECEIVER_TECHNO_GATES_REINVESTIGATION_2026-07-13.md`
- `docs/research/OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/OBJECT_TECHNO_LIFECYCLE_SHARED_STATE_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
- `docs/research/GENERIC_DESPAWN_LIMBO_CLEANUP_ENTRY_POINTS_GHIDRA_REPORT.md`
- `docs/research/SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md`
- `docs/research/BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`
- `docs/research/BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`
- `docs/research/TARGETDEATH_BUILDINGCLASS_DESTRUCTION_REMOVAL_OWNER_RESWARM_20260528.md`
- `docs/research/MCV_DEPLOY_UNDEPLOY_LIFECYCLE_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`

Current Rust files:

- `src/sim/world/world_commands.rs`
- `src/sim/production/production_sell.rs`
- `src/sim/trigger_runtime.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/world/lifecycle.rs`
- `src/sim/lifecycle_request.rs`

### Effects, Shared Authorities, and Presentation Inputs (Task 5)

- `docs/research/coord-cell-conversions/fn-coordstruct-distance3d.md`
- `docs/research/DAMAGE_MATH_GHIDRA_REPORT.md`
- `docs/research/DAMAGE_RECEIVER_CORE_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_AREA_DISPATCH_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_RECEIVER_RULE_HOUSE_ASSEMBLY_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_RECEIVER_TECHNO_GATES_REINVESTIGATION_2026-07-13.md`
- `docs/research/DAMAGE_SPECIAL_PRODUCER_TIMING_REINVESTIGATION_2026-07-13.md`
- `docs/research/APPLY_AREA_DAMAGE_BRIDGE_RNG_Z_WINDOW_GHIDRA_REPORT.md`
- `docs/research/WARHEAD_DETONATE_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md`
- `docs/research/SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/SHROUD_IMPLEMENTATION_REPORT.md`
- `docs/research/SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md`
- `docs/research/CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/CLOAKING_VISUAL_PIPELINE.md`
- `docs/research/VETERANCY_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/TECHNOCLASS_IC_FS_FIELD_0X190_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md`
- `docs/research/ANIM_CLASS_GHIDRA_REPORT.md`
- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md`
- `docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md`
- `docs/research/EVA_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_PLACETIBERIUM_FUN_00487190_GHIDRA_REPORT.md`
- `docs/research/CREDITS_COUNTER_SYSTEM.md`
- `docs/research/HOUSECLASS_GHIDRA_REPORT.md`
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
- `docs/research/OBJECT_LOGIC_LIFECYCLE_ACTIVE_MEMBERSHIP_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/RALLY_POINTS_AND_UNIT_SPAWNING.md`
- `docs/research/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`
- `docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_ADDCAMEO_INSERTENTRY_ORDER_STATUS_GHIDRA_REPORT.md`
- `docs/research/SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md`
- `docs/research/NUKE_SUPERWEAPON_GHIDRA_REPORT.md`
- `docs/research/SUPERWEAPON_GAPS_INVESTIGATION_REPORT.md`
- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`
- `docs/research/SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/GLOBAL_SOUNDS_GHIDRA_REPORT.md`
- `docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md`
- `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_GROWTIBERIUM_EXISTING_PLACETIBERIUM_GHIDRA_REPORT.md`
- `docs/research/PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`
- `docs/research/ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/CURRENT_RUST_TIBERIUM_INTEGRATION_GAPS_AND_OWNERSHIP_GHIDRA_REPORT.md`

Current Rust files:

- `src/sim/game_entity.rs`
- `src/sim/house_state.rs`
- `src/sim/combat/damage/`
- `src/sim/vision/mod.rs`
- `src/sim/superweapon/`
- `src/sim/animation.rs`
- `src/sim/anim_class.rs`
- `src/sim/tiberium/mod.rs`
- `src/sim/world/lifecycle.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/trigger_runtime.rs`
- `src/sim/world/mod.rs`

---

## Exact Passive Interfaces

### Canonical Type and Powerup Containers

Create `src/rules/crate_rules.rs` with a module doc comment and:

```rust
//! Canonical crate type identity and passive Powerups rule values.
//!
//! Runtime crate state and behavior belong to `sim`, not this rules module.

use crate::util::native_x87::NativeF64Bits;

pub const CRATE_TYPE_COUNT: usize = 19;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrateType {
    Money = 0,
    Unit = 1,
    HealBase = 2,
    Cloak = 3,
    Explosion = 4,
    Napalm = 5,
    Squad = 6,
    Darkness = 7,
    Reveal = 8,
    Armor = 9,
    Speed = 10,
    Firepower = 11,
    Icbm = 12,
    Invulnerability = 13,
    Veteran = 14,
    IonStorm = 15,
    Gas = 16,
    Tiberium = 17,
    Pod = 18,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerupEntry {
    pub weight: i32,
    pub animation: Option<String>,
    pub over_water: bool,
    pub data: NativeF64Bits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerupTable {
    entries: [PowerupEntry; CRATE_TYPE_COUNT],
}
```

These types have no `Default`, serde implementation, INI parser, total-weight
method, or random selector in this wave. `CrateRules`, sound/overlay/unit
handles, defaults, and merged-INI ownership remain runtime-plan work.

### Independent Metadata

`ObjectType` gains:

```rust
pub crate_goodie: bool,
pub carries_crate: bool,
pub crate_beneath: bool,
pub crate_beneath_is_money: bool,
```

`OverlayTypeFlags` gains:

```rust
pub crate_trigger: bool,
```

All five fields default independently to `false`. They are passive rule
metadata, not serialized runtime state.

---

## Sim and Determinism Checklist

- [ ] No simulation behavior consumes the new types or fields.
- [ ] No `f32` or `f64` crate simulation math is added.
- [ ] `NativeF64Bits` remains a passive rules value.
- [ ] No serialized or hashed field is added.
- [ ] `SNAPSHOT_VERSION` remains at the Task 0 execution baseline.
- [ ] No dependency from `sim/` to render, UI, sidebar, audio, or net is added.
- [ ] No crate `GameEntity`, scheduler, queue, callback registry, or RNG stream
      is introduced.
- [ ] Current overlay ID assignment is byte-for-byte unchanged.
- [ ] The only `src/sim` changes initialize passive test literals.
- [ ] Final status is `PARTIAL/UNVERIFIED`.

---

## Risk Areas

| Risk | Failure mode | Control |
|---|---|---|
| Dirty worktree collision | Passive edits overwrite mission-authority work | Task 0 ownership gate and exact hunk review |
| Premature runtime design | Unknown movement/effect details become API guarantees | Runtime interfaces are absent from this plan |
| False slot model | Semantic names freeze unverified words | No slot type until Task 1 and a new plan |
| Parser drift | Generic Rust helpers replace `strtok`/`atoi`/`atof` | No crate value parser in this wave |
| Overlay identity drift | Numeric-order correction mishandles mod edges | Preserve current order; Task 3 records future contract |
| Derived flags | One crate flag silently implies another | Exhaustive independent-combination tests |
| Accidental activation | Default `Crates=yes` begins changing state/RNG | Static production-use census in Task 9 |
| False parity claim | Rust unit tests are presented as native certification | Stop status remains `PARTIAL/UNVERIFIED` |

---

## Parity-Critical Evidence

| Task | Item | Verification |
|---:|---|---|
| 1 | Four slot words and sentinel bytes | All active readers/writers plus constructor |
| 1 | Authored bootstrap order | `Post_Map_Init` call graph and retail fixture |
| 1 | X/Y/FNPC/jitter draw order | Native success/failure traces |
| 2 | Per-cell callback point | Direct/virtual caller census and assembly |
| 2 | Collector mutation/deletion continuation | Native return-path traces |
| 2 | Bridge and reservation continuation | Ground/bridge and failed-subcell traces |
| 2 | Trigger/removal/replacement order | Ordered pickup transaction |
| 3 | Powerups tokenization and malformed data | Native parser differential corpus |
| 3 | Signed totals and selection failure | Negative/zero/overflow probes |
| 3 | Raw mode, player count, and frame | Writer census per supported mode |
| 3 | Overlay declaration order | Duplicate/descending/nonnumeric/empty probes |
| 4 | Death/unplace/trigger ingress | Closed-loop transaction traces |
| 5 | All 19 outcomes | One native branch/result fixture per index |
| 5 | Radius and active-object order | Boundary and ordered-object captures |
| 5 | Common presentation tail | Animation/sound/EVA ordering captures |
| 9 | Honest status | No runtime consumer, schema change, or parity claim |

---

## Tasks

### Task 0: Establish Ownership and the Execution Baseline

**Why:** The planning tree has 57 modified and 8 untracked files, including the
test literals and future runtime surfaces.

**Files:** Read every path in `git status --short`; do not edit in this task.

**Step 1: Record the baseline**

Run:

```powershell
git rev-parse HEAD
git show -s --format='%h %ci %s' HEAD
git status --short
git diff --stat
rg -n "SNAPSHOT_VERSION" src/sim/snapshot.rs
```

Planning HEAD was:

```text
928644d44fee61d0bb8d3214a4f9e0eb7390bc4e
```

Treat that value and the dirty-file counts as planning evidence, not execution
expectations.

**Step 2: Record held-surface fingerprints**

Save the literal output of:

```powershell
git status --short -- src/sim src/app_init.rs src/app_sim_tick.rs src/app_skirmish.rs src/skirmish_launch.rs src/render src/sidebar src/ui src/audio src/net ':(exclude)src/sim/movement/locomotor_tests.rs' ':(exclude)src/sim/movement/teleport_movement.rs'
git diff --binary -- src/sim src/app_init.rs src/app_sim_tick.rs src/app_skirmish.rs src/skirmish_launch.rs src/render src/sidebar src/ui src/audio src/net ':(exclude)src/sim/movement/locomotor_tests.rs' ':(exclude)src/sim/movement/teleport_movement.rs' | git hash-object --stdin
rg -n "CrateAuthority|crate_authority|pickup_crate|place_crate|tick_crate|pending_crate|crate_queue" src
rg -n "game_options\.crates|GameOptions::crates" src
```

These are comparison evidence for Task 9. Existing dirty changes are preserved.
Record an explicit `<no matches>` result when an `rg` command exits 1.

**Step 3: Resolve file ownership**

Code tasks may proceed only when each targeted file satisfies one condition:

1. existing work has landed and the file is reread at the new HEAD;
2. its active owner explicitly hands it off; or
3. the planned edit is a disjoint hunk and `git diff` proves existing work is
   untouched.

If `teleport_movement.rs` is not available for its test-only initializer, Tasks
1-6 may proceed but Task 8 waits. Do not revert or overwrite another session.

**Step 4: Check Cargo ownership**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

Do not start Cargo while another session owns a build.

**Verify:** Record HEAD, snapshot version, dirty manifest, held-surface hash,
owners, and whether Tasks 7-9 are open. No file is edited.

---

### Task 1: Close Slot, Bootstrap, Placement, and Timer Semantics

**Why:** A semantic Rust slot cannot be designed from the currently ambiguous
four-word record.

**Files:**

- Create: `docs/research/CRATE_SLOT_BOOTSTRAP_PLACEMENT_GHIDRA_REPORT.md`
- Update when corrected: `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- Read: `docs/research/MAPCLASS_GHIDRA_REPORT.md`
- Read: `docs/research/MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md`
- Read:
  `docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- Read: relevant map overlay loading and bootstrap Rust

**Binary anchors:**

```text
0x004A1750  slot clear/pause
0x004A17C0  placement/timer write
0x004A18F0  validation/overlay path
0x0056BBE0  regeneration sweep
0x0056BD40  first-free random placement
0x0056BEC0  specific-cell placement
0x0056C020  pickup removal
0x00686890  Post_Map_Init
```

**Step 1: Enumerate raw slot state**

For `+0x00`, `+0x04`, `+0x08`, and `+0x0C`, record:

- width and signedness;
- constructor value;
- every active reader and writer;
- success, failure, clear-before-expiry, and clear-after-expiry bytes;
- empty sentinel;
- same-frame behavior;
- wrap behavior; and
- save/checksum participation.

Do not assign a semantic name to `+0x04` until its complete data flow proves it.

**Step 2: Trace map-authored bootstrap**

Walk a retail authored crate from map bytes through overlay load and
`Post_Map_Init`. Record slot allocation, count contribution, timer state,
`OverlayData` preservation, replacement behavior, and ordering relative to all
earlier Scenario RNG consumers.

**Step 3: Close both placement paths**

For random and specific-cell placement, record:

- first-free definition and scan order;
- duplicate-coordinate behavior;
- attempt bound;
- inclusive X/Y bounds and draw order;
- coordinate reference frame and Rust `(rx, ry)` translation;
- land/water classification;
- every FNPC argument;
- final overlay/cell predicate;
- overlay ID/data and dirty/radar writes;
- timer writes;
- jitter draw position;
- return value; and
- all failure-state and RNG effects.

Capture native traces for first-attempt success, two failures then success, and
all 1000 attempts failing.

**Step 4: Prove timer arithmetic**

Transcribe the exact x87 load/operation/store sequence involving:

```text
RandomRanged(0, 0x7FFFFFFE)
CrateRegen * 1800
CrateRegen * 450
upper + sample/N * (lower - upper)
Math__ftol
```

Compare native lower, upper, midpoint, and truncation-boundary samples with
`X87Chop53`.

**Step 5: Classify every claim**

Each report row ends as `VERIFIED ACTIVE`, `VERIFIED INERT`, or
`VERIFIED UNREACHABLE`. Any other classification blocks Task 1.

**Verify:**

```powershell
python tools/research_index/index.py
python tools/research_index/validate.py --system root crate
```

The new report must have no checksum mismatch. Unrelated existing corpus links
do not pass or fail this gate.

---

### Task 2: Close Pickup, Caller, and Continuation Semantics

**Why:** The future movement handoff needs exact before/after writes and must
survive synchronous mutation of the collector.

**Files:**

- Create: `docs/research/CRATE_PICKUP_TRANSACTION_GHIDRA_REPORT.md`
- Update when corrected: `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- Read every Task 2 path in Frozen Existing Research Inputs

**Step 1: Rebound `0x00481A00`**

Verify boundaries, receiver, picker argument, calling convention, jump table,
tail, and every direct/virtual caller. Treat local labels as navigation hints.

**Step 2: Build the caller matrix**

Include ordinary Drive, forced drive track, drive track, Walk, Ship, Teleport,
Tube, Tunnel, Hover, Jumpjet, Fly, Droppod, and every xref-discovered family.
For each, record:

- active standard-YR reachability;
- exact last write before callback and first write after return;
- position, occupancy, layer, bridge, reservation, path, facing, mission,
  radio, and trigger state at that point;
- whether multiple cells can commit before the caller returns;
- whether the collector can be deleted, limboed, teleported, or retasked;
- retained versus reread fields after callback; and
- exact current Rust caller and required future ownership boundary.

Exclude a family only with verified inactive/unreachable evidence.

**Step 3: Close difficult continuation cases**

Capture native traces for:

1. ground to bridge to bridge in one tick;
2. bridge to ground;
3. a committed infantry cell change followed by subcell reservation failure;
4. one low-bridge tube step followed by same-tick exit placement;
5. teleport arrival;
6. collector deletion during the callback;
7. collector limbo or teleport during the callback; and
8. a fast mover committing multiple crate-capable cells.

Distinguish pre-commit rejection from post-commit reservation failure.

**Step 4: Resolve gates and `CrateTrigger`**

Verify picker class/state/house/civilian/observer/water/session gates. Trace
overlay `CrateTrigger` through cell action `0x31`, including arguments,
synchronous mutations, early exits, and relation to later selection and
presentation.

**Step 5: Write the ordered transaction**

Number every stage from cell/overlay read through return:

- picker gates;
- trigger;
- predetermined/random selection;
- mode/water/Squad/anti-stack/FreeMCV fixups;
- slot match and clear;
- overlay clear;
- multiplayer immediate replacement;
- gameplay handler;
- animation;
- sound;
- EVA;
- dirty/radar state;
- return value; and
- collector revalidation/continuation.

Every neighboring order has an instruction-range citation.

**Verify:** Caller count equals the xref census; every caller has a
classification and exact Rust location; no transaction stage is unordered.

---

### Task 3: Close Session, Frame, Rules, Weights, and Overlay Ordering

**Why:** Native selection and placement depend on raw authorities and parser
edge behavior absent from current Rust.

**Files:**

- Create: `docs/research/CRATE_SESSION_FRAME_RULES_GHIDRA_REPORT.md`
- Update when corrected: `docs/research/OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`
- Read:
  `docs/research/FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md`
- Read:
  `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
- Read: `docs/research/RULESCLASS_GHIDRA_REPORT.md`
- Read every Task 3 path in Frozen Existing Research Inputs

**Binary anchors:**

```text
0x0066B900  rules dispatch neighborhood
0x00673E80  Powerups reader
0x00668BF0  OverlayTypes loader
```

**Step 1: Differential-test Powerups parsing**

Record canonical-name lookup, missing names, delimiters, token count,
`strtok` empty handling, whitespace, `atoi` overflow, animation `<none>`,
token-three boolean behavior, token-four `atof` grammar/precision/percent/
NaN/Inf behavior, extra tokens, duplicates, unknown names, and base/patch
replacement.

Native probes cover absent, empty, one/two/three/four/extra token, negative,
decimal, exponent, percent, overflow, and malformed values.

**Step 2: Verify weighted selection failures**

Trace signed accumulation and `RandomRanged(1, total)`. Capture totals 110, 1,
0, negative, signed overflow, negative individual weights, and a cumulative
walk that selects no row. Do not clamp or normalize without evidence.

**Step 3: Verify overlay declaration behavior**

Probe numeric gaps, descending keys, duplicate keys, nonnumeric keys, empty
values, and base/patch replacement. Record the exact final name array and IDs
for every fixture. Do not alter Rust ordering in this plan.

**Step 4: Verify raw session authority**

For `g_GameMode`, record every writer and active values for campaign, offline
skirmish, LAN, and WOL. For `Session::NumPlayers`, record width, timing, and
whether humans, AI, observers, defeated, neutral/civilian, empty lobby, and
cooperative participants count.

Map both to accepted-launch inputs; do not derive them from current houses or
selected `MPModes` row ID.

**Step 5: Verify frame authority**

Trace `g_CurrentFrameCounter` writers and crate readers in every supported
mode. Record increment location, pre/post visibility, pause/modal/network pump
behavior, logic-step rate relation, wrap, and whether Rust's synthetic 15 Hz
counter is equivalent.

**Verify:** Every parser probe has native output; every raw field has a writer
and pre-tick Rust source; no frame conclusion is inferred from timer units.

---

### Task 4: Close Unit, Building, and Trigger Crate Ingress

**Why:** Crate creation is active outside random spawning and belongs to
specific lifecycle/trigger transactions.

**Files:**

- Create: `docs/research/CRATE_DROP_TRIGGER_INGRESS_GHIDRA_REPORT.md`
- Read: `src/sim/world/world_commands.rs`
- Read: `src/sim/production/production_sell.rs`
- Read: `src/sim/trigger_runtime.rs`
- Read: `src/sim/world/world_spawn.rs`
- Read every Task 4 path in Frozen Existing Research Inputs

**Step 1: Trace unit-death ingress**

Trace `0x00737C90`, both scenario bytes at `+0x34A5` and `+0x34B1`, and the
`0x0056BEC0` specific-cell call. Record parsing/defaults, startup writers,
land/water branch, FNPC tuple, raw content, death-stage order, failure, and RNG.

**Step 2: Trace building-unplace ingress**

Trace `0x00441F60` and all active sell, destruction, undeploy, capture,
replacement, and script-removal callers. Record both `CrateBeneath` fields,
raw data `0` versus `0x14`, slot/full behavior, and overlay/occupancy order.

**Step 3: Trace action `0x6C`**

At the action switch near `0x006DD8B0`, record parameter widths, waypoint
resolution, invalid waypoint/type behavior, raw data, full slots, Scenario RNG,
trigger continuation, and presentation.

**Step 4: Produce the ingress matrix**

Each ingress names its owning Rust transaction, immutable context, mutable
authorities, exact order, RNG calls, state writes, failure result, and
native-derived fixture.

**Verify:** No ingress is attached generically to final deletion; both scenario
byte defaults and the one-argument raw content are proven.

---

### Task 5: Close All 19 Effects and Presentation

**Why:** Eight outcomes occur in stock random play and weight-zero outcomes
remain reachable through authored data and triggers.

**Files:**

- Create: `docs/research/CRATE_EFFECT_HANDLERS_GHIDRA_REPORT.md`
- Read every Task 5 path in Frozen Existing Research Inputs

**Dispatch anchors:**

| Index | Type | Handler/tail |
|---:|---|---|
| 0 | Money | `0x00482463` |
| 1 | Unit | `0x00482041` |
| 2 | HealBase | `0x00482B8F` |
| 3 | Cloak | `0x00482840` |
| 4 | Explosion | `0x00482565` |
| 5 | Napalm | `0x0048271E` |
| 6 | Squad | remap/common tail |
| 7 | Darkness | `0x00481F6D` |
| 8 | Reveal | `0x00481F9D` |
| 9 | Armor | `0x00482D56` |
| 10 | Speed | `0x00482F36` |
| 11 | Firepower | `0x00483125` |
| 12 | ICBM | `0x00482CA1` |
| 13 | Invulnerability | common tail |
| 14 | Veteran | `0x00482972` |
| 15 | IonStorm | common tail |
| 16 | Gas | `0x00481DE7` |
| 17 | Tiberium | `0x00481E99` |
| 18 | Pod | skip/common tail |
| — | Common presentation | `0x004832F5` |

**Step 1: Audit every index**

For each row record preconditions/remaps, scan source/order, every eligibility
gate, coordinate frame and radius arithmetic, RNG calls, state reads/writes,
damage/lifecycle/spawn calls, failure paths, collector survival, animation,
sound, EVA, shroud/radar/dirty effects, and active/remapped/animation-only/
skipped classification.

**Step 2: Reconcile stale claims**

Correct Veteran to canonical index 14; source magnitude from the Powerups row;
do not equate current Rust veterancy, speed multiplier, or integer distance
with native crate fields/math; reject totals 100/144; preserve verified common
tails for inert gameplay outcomes.

**Step 3: Build the Rust readiness matrix**

For every native field/call, classify the current Rust owner as:

- `READY EXACT`;
- `READY AFTER NAMED FIX`;
- `ABSENT`; or
- `CONTRADICTORY`.

Name the file and interface. Do not propose a crate-local duplicate authority.

**Step 4: Define native captures**

Specify at least one executable native result capture for every index,
including zero-gameplay assertions for inert outcomes, strict radius-boundary
fixtures, and multiple eligible/ineligible objects in live order.

**Verify:** Indices 0 through 18 appear once; every row has complete RNG/state/
presentation data; every shared Rust authority has an owner/readiness status.

---

### Task 6: Synthesize and Review the Exact Runtime Contract

**Why:** Independent reports can each be correct yet disagree on global order
or ownership.

**Files:**

- Create: `docs/research/CRATE_AUTHORITY_SYSTEM_MODEL.md`
- Create: `docs/research/CRATE_AUTHORITY_IMPLEMENTATION_CONTRACT.md`
- Read: all Task 1-5 reports
- Read: the approved design and this plan

**Step 1: Synthesize**

Run `/synthesize-system-model` for the crate system. The model reconciles
bootstrap, slots, placement, frame, parsing, pickup, callers, continuation,
effects, ingress, presentation, scheduler, persistence, and failure paths.
Every claim is tagged as verified binary, verified INI/asset, current Rust,
inference, or unresolved.

**Step 2: Produce the implementation contract**

Run `/implementation-contract`. It must name:

- exact Rust types and native widths;
- all serialized and hashed fields;
- parser defaults and malformed behavior;
- every RNG draw and order;
- scheduler rungs;
- every active movement caller;
- exact yield/resume state per caller, including bridge projection,
  post-commit reservation failure, and collector revalidation;
- transaction and handler order;
- lifecycle/trigger/presentation adapters;
- raw session/frame sources;
- snapshot/replay migration; and
- executable native fixtures.

**Step 3: Validate artifacts**

```powershell
python tools/research_index/index.py
python tools/research_index/validate.py --system root crate
```

Every new crate artifact must be indexed without checksum mismatch.

**Step 4: Apply the gate**

Tasks 7-9 may proceed only if canonical type identity, every `PowerupEntry`
field width/meaning, and all passive flag mappings remain verified, and the
current target files pass Task 0 ownership. Runtime work is blocked regardless.

If any load-bearing runtime item remains unresolved, record it as a blocker in
the implementation contract. Do not invent an interface to make the report
look complete.

**Step 5: Require the next plan**

After this plan finishes, run:

```text
/write-plan crate authority runtime activation
```

Then run `/review-plan` on that new plan before implementation. The new plan,
not this one, must contain exact driver structs, field types, visibility,
per-family begin/resume APIs, world caller sites, pre-refactor regression
oracles, tests, snapshot version, and activation tasks.

**Verify:** The system model has no hidden contradiction; every unresolved item
is an explicit blocker; no runtime code was written.

---

### Task 7: Add Canonical Crate Rule Vocabulary

**Why:** Canonical identity and native-bit value containers are verified and
have no runtime effect without a parser/consumer.

**Files:**

- Create: `src/rules/crate_rules.rs`
- Modify: `src/rules/mod.rs`

**Step 1: Add the exact types**

Implement the interface above plus:

```rust
impl CrateType {
    pub const ALL: [Self; CRATE_TYPE_COUNT] = [
        Self::Money,
        Self::Unit,
        Self::HealBase,
        Self::Cloak,
        Self::Explosion,
        Self::Napalm,
        Self::Squad,
        Self::Darkness,
        Self::Reveal,
        Self::Armor,
        Self::Speed,
        Self::Firepower,
        Self::Icbm,
        Self::Invulnerability,
        Self::Veteran,
        Self::IonStorm,
        Self::Gas,
        Self::Tiberium,
        Self::Pod,
    ];

    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Money),
            1 => Some(Self::Unit),
            2 => Some(Self::HealBase),
            3 => Some(Self::Cloak),
            4 => Some(Self::Explosion),
            5 => Some(Self::Napalm),
            6 => Some(Self::Squad),
            7 => Some(Self::Darkness),
            8 => Some(Self::Reveal),
            9 => Some(Self::Armor),
            10 => Some(Self::Speed),
            11 => Some(Self::Firepower),
            12 => Some(Self::Icbm),
            13 => Some(Self::Invulnerability),
            14 => Some(Self::Veteran),
            15 => Some(Self::IonStorm),
            16 => Some(Self::Gas),
            17 => Some(Self::Tiberium),
            18 => Some(Self::Pod),
            _ => None,
        }
    }

    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Money => "Money",
            Self::Unit => "Unit",
            Self::HealBase => "HealBase",
            Self::Cloak => "Cloak",
            Self::Explosion => "Explosion",
            Self::Napalm => "Napalm",
            Self::Squad => "Squad",
            Self::Darkness => "Darkness",
            Self::Reveal => "Reveal",
            Self::Armor => "Armor",
            Self::Speed => "Speed",
            Self::Firepower => "Firepower",
            Self::Icbm => "ICBM",
            Self::Invulnerability => "Invulnerability",
            Self::Veteran => "Veteran",
            Self::IonStorm => "IonStorm",
            Self::Gas => "Gas",
            Self::Tiberium => "Tiberium",
            Self::Pod => "Pod",
        }
    }
}

impl PowerupTable {
    pub fn new(entries: [PowerupEntry; CRATE_TYPE_COUNT]) -> Self {
        Self { entries }
    }

    pub fn entry(&self, crate_type: CrateType) -> &PowerupEntry {
        &self.entries[usize::from(crate_type.raw())]
    }

    pub fn entries(&self) -> &[PowerupEntry; CRATE_TYPE_COUNT] {
        &self.entries
    }
}
```

Export with:

```rust
pub mod crate_rules;
```

Do not add defaults, serde, parsing, weight totals, or selection.

**Step 2: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_indices_and_names_are_fixed() {
        for (raw, crate_type) in CrateType::ALL.into_iter().enumerate() {
            assert_eq!(crate_type.raw(), raw as u8);
            assert_eq!(CrateType::from_raw(raw as u8), Some(crate_type));
            assert!(!crate_type.canonical_name().is_empty());
        }
        assert_eq!(CrateType::from_raw(19), None);
        assert_eq!(CrateType::from_raw(u8::MAX), None);
        assert_eq!(CrateType::Veteran.raw(), 14);
        assert_eq!(CrateType::Icbm.canonical_name(), "ICBM");
    }

    #[test]
    fn table_indexes_by_canonical_type_not_file_order() {
        let entries = std::array::from_fn(|index| PowerupEntry {
            weight: index as i32,
            animation: None,
            over_water: false,
            data: NativeF64Bits::from_bits(index as u64),
        });
        let table = PowerupTable::new(entries);
        assert_eq!(table.entry(CrateType::Money).weight, 0);
        assert_eq!(table.entry(CrateType::Veteran).weight, 14);
        assert_eq!(table.entry(CrateType::Pod).weight, 18);
    }
}
```

**Verify serially:**

```powershell
cargo test -p vera20k canonical_indices_and_names_are_fixed -- --nocapture
cargo test -p vera20k table_indexes_by_canonical_type_not_file_order -- --nocapture
```

Each command must report literal `test result: ok.`

---

### Task 8: Parse Independent Passive Crate Metadata

**Why:** These verified rule facts are required later and do not activate
gameplay.

**Files:**

- Modify: `src/rules/object_type.rs`
- Modify: `src/map/overlay_types.rs`
- Modify: `src/sim/movement/locomotor_tests.rs`
- Modify: the `#[cfg(test)]` `make_drive_obj` literal only in
  `src/sim/movement/teleport_movement.rs`

**Step 1: Add object fields**

Add near other passive capabilities:

```rust
/// May be selected by a multiplayer Unit crate.
pub crate_goodie: bool,
/// May create a crate through the verified unit-death ingress.
pub carries_crate: bool,
/// May create a crate through the verified building-unplace ingress.
pub crate_beneath: bool,
/// Selects the Money raw-content branch for `CrateBeneath`.
pub crate_beneath_is_money: bool,
```

Initialize in `ObjectType::from_ini_section`:

```rust
crate_goodie: section.get_bool("CrateGoodie").unwrap_or(false),
carries_crate: section.get_bool("CarriesCrate").unwrap_or(false),
crate_beneath: section.get_bool("CrateBeneath").unwrap_or(false),
crate_beneath_is_money: section
    .get_bool("CrateBeneathIsMoney")
    .unwrap_or(false),
```

Set all four to `false` in the complete test literals in
`locomotor_tests.rs` and `teleport_movement.rs`. Confirm the literal census:

```powershell
rg -n "^\s*ObjectType\s*\{" src --glob '*.rs'
```

Expected direct literals are `locomotor_tests.rs` and the single
`make_drive_obj` helper in `teleport_movement.rs`. The constructor uses
`Self`; the later teleport helper mutates the first object; and
`src/sim/infantry.rs` returns a parsed clone.

**Step 2: Add overlay flag**

Add:

```rust
/// `CrateTrigger=yes` invokes the verified cell-trigger stage on pickup.
pub crate_trigger: bool,
```

Set `false` in `Default` and parse:

```rust
crate_type: type_section.get_bool("Crate").unwrap_or(false),
crate_trigger: type_section.get_bool("CrateTrigger").unwrap_or(false),
```

Do not modify pair collection, numeric sort, deduplication, or ID assignment.

**Step 3: Add exhaustive flag tests**

In `object_type.rs`, add:

```rust
#[test]
fn object_type_crate_flags_are_independent() {
    for bits in 0_u8..16 {
        let value = |mask: u8| if bits & mask == 0 { "no" } else { "yes" };
        let ini = IniFile::from_str(&format!(
            "[X]\nCrateGoodie={}\nCarriesCrate={}\n\
             CrateBeneath={}\nCrateBeneathIsMoney={}\n",
            value(1),
            value(2),
            value(4),
            value(8),
        ));
        let section = ini.section("X").expect("X section");
        let object =
            ObjectType::from_ini_section("X", section, ObjectCategory::Vehicle);

        assert_eq!(
            (
                object.crate_goodie,
                object.carries_crate,
                object.crate_beneath,
                object.crate_beneath_is_money,
            ),
            (
                bits & 1 != 0,
                bits & 2 != 0,
                bits & 4 != 0,
                bits & 8 != 0,
            ),
            "combination {bits:04b}",
        );
    }
}
```

This parses all 16 combinations and asserts the exact four-bit tuple.

In `overlay_types.rs`, add:

```rust
#[test]
fn crate_and_crate_trigger_are_independent() {
    let ini = IniFile::from_str(
        "[OverlayTypes]\n1=A\n2=B\n3=C\n\
         [A]\nCrate=yes\nCrateTrigger=no\n\
         [B]\nCrate=no\nCrateTrigger=yes\n\
         [C]\nCrate=yes\nCrateTrigger=yes\n",
    );
    let registry = OverlayTypeRegistry::from_ini(&ini, None);

    assert_eq!(
        registry.flags(0).map(|flags| (flags.crate_type, flags.crate_trigger)),
        Some((true, false))
    );
    assert_eq!(
        registry.flags(1).map(|flags| (flags.crate_type, flags.crate_trigger)),
        Some((false, true))
    );
    assert_eq!(
        registry.flags(2).map(|flags| (flags.crate_type, flags.crate_trigger)),
        Some((true, true))
    );
}
```

**Verify serially:**

```powershell
cargo test -p vera20k object_type_crate_flags_are_independent -- --nocapture
cargo test -p vera20k crate_and_crate_trigger_are_independent -- --nocapture
```

Each command must report literal `test result: ok.`

---

### Task 9: Final Serial Verification and Honest Handoff

**Why:** The wave must prove passive-only scope and leave a precise runtime
handoff.

**Step 1: Format only edited Rust files**

Run each command separately:

```powershell
rustfmt --edition 2024 src/rules/crate_rules.rs
rustfmt --edition 2024 --config skip_children=true src/rules/mod.rs
rustfmt --edition 2024 src/rules/object_type.rs
rustfmt --edition 2024 src/map/overlay_types.rs
rustfmt --edition 2024 src/sim/movement/locomotor_tests.rs
rustfmt --edition 2024 src/sim/movement/teleport_movement.rs
```

Inspect every diff and reject unrelated formatting churn.

**Step 2: Check scope**

```powershell
git diff --check
git diff -- src/rules/crate_rules.rs src/rules/mod.rs src/rules/object_type.rs src/map/overlay_types.rs
git diff -- src/sim/movement/locomotor_tests.rs src/sim/movement/teleport_movement.rs
rg -n "CrateAuthority|crate_authority|pickup_crate|place_crate|tick_crate|pending_crate|crate_queue" src
rg -n "game_options\.crates|GameOptions::crates" src
rg -n "SNAPSHOT_VERSION" src/sim/snapshot.rs
```

Expected:

- no runtime crate authority or consumer;
- no crate queue;
- no new `GameOptions::crates` consumer;
- snapshot version equals Task 0;
- `teleport_movement.rs` changes only its test object literal.

**Step 3: Recheck held surfaces**

Re-run the Task 0 held-surface status and hash commands. Differences from the
baseline are allowed only for the two named test files and only at their object
literal initializers. Every other new held-surface change blocks completion.

**Step 4: Check Cargo ownership**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
    Select-Object ProcessName,Id,CPU
```

Wait if another session owns Cargo.

**Step 5: Run focused tests serially**

Run all four Task 7/8 commands one at a time and record each literal
`test result:` line.

**Step 6: Run full library tests and check**

```powershell
cargo test -p vera20k --lib
cargo check -q -p vera20k
```

The test command must report `test result: ok.` and the check must exit 0.
If unrelated dirty work fails, report the exact target and owner without
repairing or reverting it.

**Step 7: Update the implementation contract**

Record execution HEAD, report revisions, passive files changed, test results,
held-surface comparison, unchanged snapshot version, every runtime blocker, and
status `PARTIAL/UNVERIFIED`.

Do not describe crate gameplay as implemented, enabled, complete, or parity
verified.

**Stop condition:** Evidence artifacts are indexed, passive metadata tests
pass, runtime scope is untouched, and the implementation contract is ready as
input to a new `/write-plan`.

---

## Required Inputs to the Future Runtime Plan

The next plan must quote the reviewed implementation contract for:

1. exact `CrateSlot` bytes and constructor;
2. authored-map reconciliation;
3. initial placement and earlier-RNG ordering;
4. raw mode, player count, and native frame;
5. exact `[CrateRules]` and `[Powerups]` parsing;
6. random and specific-cell placement;
7. complete active arrival caller set;
8. exact pickup/trigger/removal/replacement transaction;
9. all 19 outcomes and common presentation;
10. unit/building/trigger ingress;
11. shared entity, house, lifecycle, visibility, superweapon, damage, sound,
    animation, and scheduler authorities;
12. regeneration immediately before the first factory step;
13. one snapshot/hash/replay migration;
14. executable native RNG/order/result fixtures;
15. a pre-refactor Rust regression oracle that does not compare two wrappers
    over the same new implementation;
16. exact crate-private driver structs with private fields and visible types;
17. move-owned live-order vectors without redundant hot-path allocation;
18. resumable Tube handling for both a path step and same-tick exit placement;
19. family-specific resumable Teleport, Tunnel, and every other active
    special-movement API at its actual world caller;
20. persisted bridge projection and every other crossing-local value;
21. explicit pre-commit rejection versus post-commit reservation failure; and
22. an after-host collector revalidation stage.

The runtime plan chooses its snapshot version and code line anchors from its
execution baseline, not from this document.

---

## Sources and References

### Approved Specification

- `docs/plans/2026-07-23-crate-authority-design.md`

### Primary Research

- `docs/research/CRATE_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/MAPCLASS_GHIDRA_REPORT.md`
- `docs/research/MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md`
- `docs/research/OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md`
- `docs/research/RULESCLASS_GHIDRA_REPORT.md`
- `docs/research/FRAME_BASIS_ONE_INCREMENT_ONE_LOGIC_STEP_GHIDRA_REPORT.md`
- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`
- `docs/research/FUN_00481A00_CRATE_PICKUP_WARP_ARRIVAL_GHIDRA_REPORT.md`
- `docs/research/pathfinding/FIND_NEARBY_PASSABLE_CELL_CALLER_PARAMETER_MATRIX_GHIDRA_REPORT.md`
- `docs/research/VETERANCY_SYSTEM_GHIDRA_REPORT.md`

### Live Binary Checks Used During Planning

- `0x004A1750` slot clear/pause
- `0x004A17C0` placement/timer
- `0x004A18F0` validation/overlay
- `0x0056BBE0` regeneration
- `0x0056BD40` random placement
- `0x0056C020` removal
- `0x00668BF0` OverlayTypes entry enumeration

### Stock Data

- `ini/rules.ini`
- `ini/rulesmd.ini`
- `ini/art.ini`
- `ini/artmd.ini`

### Current Rust Anchors

- `src/rules/ini_parser.rs`
- `src/rules/ini_value.rs`
- `src/rules/object_type.rs`
- `src/map/overlay_types.rs`
- `src/sim/overlay_grid.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/teleport_movement.rs`
- `src/sim/movement/tube_movement.rs`
- `src/sim/movement/tunnel_movement.rs`
- `src/sim/world/mod.rs`
- `src/sim/scenario_session.rs`
- `src/sim/snapshot.rs`
- `src/sim/world/world_hash.rs`
- `src/util/native_x87.rs`

---

## Mandatory Post-Plan Self-Review

1. **Spec coverage:** Every approved requirement maps to evidence work and a
   named future-plan input.
2. **No placeholders:** Every task in this document has exact files, outputs,
   commands, and pass criteria.
3. **No speculative runtime interface:** Movement drivers, slot structs,
   effect adapters, session fields, and persistence are intentionally absent.
4. **Architecture:** Visible bytes stay in `OverlayGrid`; no crate entity,
   queue, callback registry, private scheduler, or RNG stream is added.
5. **Interface ordering:** Research and contract synthesis precede passive code.
6. **Risk coverage:** Dirty ownership, parser edges, overlay order, derived
   flags, accidental activation, and false certification all have controls.
7. **Sim compliance:** No float simulation math, upper-layer dependency, or
   authoritative runtime state is added.
8. **Grounding:** Primary docs, live binary checks, current Rust, stock INI,
   research-index state, and git state are cited.
9. **Confidence:** Every code decision is based on high-confidence evidence;
   unresolved runtime claims remain explicit gates.
10. **Verification:** Focused tests, full library tests, `cargo check`, diff
    review, held-surface comparison, and a production-use census are required.
11. **Honest status:** Completion is `PARTIAL/UNVERIFIED` and requires a new
    reviewed runtime implementation plan.

---

## Recommended Next Review

Before execution, run:

```text
/review-plan docs/plans/2026-07-23-crate-authority-evidence-foundation-plan.md
```

Research Tasks 1-6 are read-only and may start while runtime files remain
owned elsewhere. Passive Tasks 7-9 wait for Task 0 ownership and Task 6
confirmation.
