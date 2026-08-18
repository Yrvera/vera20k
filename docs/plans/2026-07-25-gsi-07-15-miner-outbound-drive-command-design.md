# GSI-07.15 Miner Outbound Drive Command Design

Date: 2026-07-25  
Status: AUTONOMOUSLY APPROVED FOR IMPLEMENTATION PLANNING  
Parent: suspended GSI-07.15 present-level-zero scan/archive/move slice  
Contract: `docs/contracts/2026-07-25-gsi-07-15-miner-outbound-drive-command-implementation-contract.md`

## Goal

Make stock HARV and CMIN hand a selected outbound ore cell to the existing
normal Drive destination/path authority with the merged-rule speed profile,
while preserving CMIN's primary-Teleport/active-Drive piggyback ownership and
leaving every heterogeneous scripted direct-movement caller unchanged.

## Architecture Context

The production miner loop currently owns selection and movement dispatch in
`src/sim/miner/miner_system.rs`. It has two movement paths:

- adjacent ore calls `movement::issue_direct_move`, which builds only a generic
  two-cell `MovementTarget`;
- farther ore calls the basic `movement::issue_move_command`, then marks the
  target terrain-cost-exempt.

Those paths predate the current Drive scaffold. Normal Drive commands now use
`issue_move_command_with_layered`, which performs A*, writes `NavigationState`
NavCom, writes Drive destination/head-to state, seeds Drive directions and turn
state, and may start DriveTrack. The production player-command owner then stamps
`MovementTarget.accel_factor`, `decel_factor`, and `slowdown_distance` from
`Simulation::resolve_move_info`.

The command/locomotor ownership split is therefore already present:

- miner mission code selects the target and decides whether to issue;
- `Simulation::resolve_move_info` is the current merged-rule/entity snapshot;
- `LocomotorState` owns primary/active/piggyback identity;
- `NavigationState` owns NavCom;
- `DriveLocomotionRuntime` and DriveTrack own Drive execution state;
- `MovementTarget` remains the transitional path adapter and carries the current
  rule profile consumed by `movement_tick`.

The native outbound path supports this shape. `Mission_Harvest` state 0 calls
`FootClass::Search_For_Tiberium_And_Move`; for a selected non-current cell that
wrapper reaches the UnitClass destination path. `FootClass::Set_Destination_Internal`
writes NavCom before dispatching the coordinate to the active locomotor. For a
stock CMIN with old NavCom `NULL`, the verified Teleporter gate defaults to
creating/activating Drive and piggybacking the prior Teleport locomotor. Outbound
never arms Teleport.

The observed parent failure is exactly at this handoff. The correct ore target
and generic path survive, but the stock HARV has target speed fraction `1`,
current fraction `0`, and `MovementTarget.accel_factor=0`; normal Drive authority
therefore produces zero speed on every tick.

The first review also exposed two ownership details that the initial draft
missed. `restore_primary_from_piggyback()` is a lifecycle transition, not an
exact undo, and the native scan wrapper gates another scan on owner NavCom, not
only on Rust's transitional `MovementTarget`.

## Impact Analysis

### Files to modify

- `src/sim/miner/miner_system.rs`
  - replace the adjacent-direct/non-adjacent-reduced dispatch split with one
    miner-owned outbound command helper;
  - resolve the existing move snapshot, activate CMIN Drive piggyback, invoke the
    layered command, and stamp its profile;
  - gate target validation, arrival, another scan, and another issue on owner
    NavCom as well as `MovementTarget`;
  - remove stale comments claiming stock Track cannot traverse Tiberium.
- `src/sim/movement/mod.rs`
  - when the normal piggyback lifecycle restores primary Teleport, drop the
    retired Drive runtime just as native `FootClass::AI` releases the old
    active Drive locomotor.
- `src/sim/miner/mod.rs`
  - register one separate test-only module.

### File to create

- `src/sim/miner/outbound_drive_tests.rs`
  - merged-retail production-loop fixtures for stock HARV and CMIN;
  - command-state, speed-profile, piggyback, RNG, physical-departure, and arrival
    assertions.

### Files deliberately not modified

- `src/sim/movement/movement_commands.rs`
  - the existing layered command is sufficient;
  - `issue_direct_move` remains unchanged for refinery foundation traversal,
    sell egress, passenger approach, bump/crush, and other scripted consumers.
- `src/sim/movement/locomotor.rs`
  - the required Drive-over-Teleport primitive and restore gate already exist.
- `src/sim/world/world_commands.rs`
  - `resolve_move_info` and the producer-side profile-stamp pattern already
    exist.
- serialization, snapshots, state hashing, tick order, render, UI, audio, and
  networking surfaces.

### Dependency and ordering impact

- No new state or hash field is introduced.
- No iteration order changes.
- No RNG call is introduced.
- No new per-tick collection allocation is required; the helper snapshots only
  existing copyable rule/entity values.
- `Simulation::advance_tick` order is unchanged. Miner dispatch still occurs at
  the same phase; the next normal movement phase consumes the now-complete
  command state.
- `sim/` gains no dependency on render, UI, sidebar, audio, or net.

### Integration risk

- The suspended parent also edits `miner_system.rs`; it must be replayed onto
  the prerequisite merge rather than merged blindly.
- The parent has a large `miner_tests.rs` diff. A separate prerequisite test
  file avoids an unnecessary same-file conflict.
- CMIN must not restore Teleport between command issue and movement. The
  existing restore gate observes the active movement target, so the production
  test must assert ownership on every moving tick. When restoration succeeds,
  the Rust lifecycle owner must also remove the retired
  `DriveLocomotionRuntime`; retaining it would preserve Drive destination bytes
  after native `FootClass::AI` has released the active Drive object.
- A Rust-only synchronous A* failure can occur after a newly activated CMIN
  piggyback even though native path generation is locomotor-tick-owned.
  `restore_primary_from_piggyback()` cannot serve as rollback because it writes
  lifecycle state. The helper must restore exactly the five copyable fields
  touched by activation, without disturbing an already-active Drive piggyback.
- Once the helper installs NavCom, a `NavCom=Some` / `MovementTarget=None`
  intermediate state is representable. The miner must return before target
  validation, arrival, rescan, or issue through that state. This preserves the
  native wrapper-entry owner gate and prevents an arrived-but-not-yet-cleared
  NavCom from advancing the Rust mission one tick early. Blocked/repath
  completion remains an explicitly excluded Drive-authority residual.

## Tiny-Detail Ledger

- Stock `[CMIN]` is `Harvester=yes`, `Teleporter=yes`, `Speed=4`, `ROT=5`,
  `MovementZone=Crusher`, with Teleport locomotor.
- Stock `[HARV]` is `Harvester=yes`, `Speed=4`, `ROT=5`,
  `MovementZone=Crusher`, with Drive locomotor.
- Neither stock miner sets `Accelerates=false`; merged ObjectType default is
  true.
- Missing type keys resolve through the existing parsed defaults:
  `AccelerationFactor=0.03`, `DeaccelerationFactor=0.002`, and
  `SlowdownDistance=500`. The helper must not repeat these literals.
- `[Tiberium] Track=70%`; stock Tiberium is a positive terrain-cost cell, not a
  reason to bypass the grid/cost authority.
- `Search_For_Tiberium_And_Move @ 0x004DCFE0` does not rescan when a destination
  already exists. The check is the first operation in the wrapper, so the
  revised caller must gate on owner NavCom as well as the transitional movement
  target before target validation, arrival, rescan, or issue.
- A selected non-current cell reaches the UnitClass destination virtual.
- Successful non-null destination setup writes owner NavCom before active
  locomotor head-to.
- With old NavCom `NULL`, CMIN's Teleporter gate defaults to Drive piggyback.
- Outbound CMIN must have active Drive, primary Teleport, and a stored
  piggyback; it must not create `teleport_state`.
- `Accelerates=true` starts/runs from current fraction `0` and adds the parsed
  acceleration factor on a normal Drive frame only when outside
  `SlowdownDistance`.
- An adjacent cell is 256 leptons away and therefore lies inside stock
  `SlowdownDistance=500`; its first Drive frame takes the destination-braking
  branch and clamps a zero current fraction to the native 0.3 floor.
- `Accelerates=false` snaps current fraction to the adjusted target; the design
  must not force all miners through the true branch.
- Terrain target speed still comes from the existing current/next-cell Drive
  computation.
- No scenario, main, or mapgen RNG draw belongs to target selection handoff or
  command issue.
- The adjacent and non-adjacent outbound paths must use the same command owner;
  adjacency is not a different native movement mechanism.
- A failed Rust A* issue must not leave a newly created CMIN Drive piggyback
  orphaned.
- Failure rollback must restore `kind`, `primary_kind`, `piggyback`, `layer`,
  and `phase` exactly; the normal restore method is not an undo.
- An already-active CMIN Drive piggyback must not be restored merely because a
  reissue attempt fails.
- Native Drive track completion clears the arrived Drive destination before
  `FootClass::AI` ends piggyback and releases the old active Drive locomotor.
  A successful Rust primary-Teleport restoration must therefore drop the
  retired `DriveLocomotionRuntime`; keeping it is observable in state hashing
  and can leak a stale destination into later outbound legs.
- `issue_direct_move` and every non-miner caller remain byte-for-byte unchanged.
- This slice does not certify DriveTrack curve points, path-cell identity,
  blocked retries, collision, bridge transitions, arrival tick, or pixels.
- The suspended parent must rerun its full ring-0, ring-1, archive/dock/return,
  Slave boundary, RNG, lifecycle, and hash oracles after this prerequisite is
  merged.

## Approaches Considered

### Approach A: Miner-owned normal outbound Drive dispatch

Add one private helper in `miner_system.rs` that snapshots `MoveInfo`, activates
Drive piggyback for a teleporter-harvester when needed, invokes
`issue_move_command_with_layered` with live terrain/zone inputs, rolls back only
a newly created piggyback by restoring every activation-mutated field if the
synchronous Rust issue fails, and stamps the successful `MovementTarget` with
the resolved acceleration/deceleration/slowdown profile.

Use this helper for every newly issued outbound ore move, adjacent or farther.

Advantages:

- matches the verified native state-0 destination/Drive ownership;
- reuses the existing normal Drive/NavCom/DriveTrack scaffold;
- uses the same profile authority as production player movement;
- removes the false Tiberium-bypass assumption;
- touches only the parent producer and dedicated tests;
- keeps global scripted direct movement stable.

Risk:

- the helper duplicates three profile assignments already present in the player
  command producer. This is bounded duplication until command-profile ownership
  is centralized under a separately reviewed movement-wide slice.

Verdict: chosen.

### Approach B: Upgrade `issue_direct_move` globally

Make `issue_direct_move` create NavCom, Drive runtime/path/track state, CMIN
piggyback, and a rule profile.

Advantages:

- one apparent central fix.

Rejection:

- the helper has heterogeneous scripted callers whose destination, bypass,
  ForceTrack, foundation, passenger, sell, and collision semantics are not
  proven to share the miner contract;
- it lacks `RuleSet`, terrain, and zone authority;
- a global change would broaden the prerequisite into several untraced systems.

### Approach C: Extend the general Teleporter destination bridge

Rework `set_destination_for_teleporter_entity` and all movement producers to
implement the complete verified Teleporter old-NavCom/destination predicate,
then route miners through it.

Advantages:

- moves toward one exact Set_Destination authority for player, mission, dock,
  and miner calls.

Deferral:

- the current bridge's empty-cell behavior is broader than the verified
  predicate;
- completing it requires old-NavCom RTTI/DockUnload/unit-occupancy and deferred
  restore-state semantics across several callers;
- inbound accepted-dock behavior remains separately unresolved;
- it is not the smallest prerequisite for the parent oracle.

### Approach D: Keep direct movement and only stamp acceleration

Leave adjacent ore on `issue_direct_move`, manually create a Drive runtime or
set its current fraction, and keep the farther reduced command.

Rejection:

- it would make the test move while knowingly preserving the wrong NavCom,
  Drive-path, CMIN-piggyback, and terrain-cost mechanism;
- forcing current fraction to `1` would contradict stock `Accelerates=true`.

## Chosen Design

### Component

Add one private function:

`issue_outbound_ore_move(sim, rules, grid, entity_id, target) -> bool`

It is a producer adapter, not a new movement authority. It owns only the
miner-specific facts that the destination came from the outbound ore scan and
that a Teleport-primary stock harvester must run Drive for this leg.

### Command preparation

1. Reject an out-of-grid target before mutating locomotor state.
2. Call `Simulation::resolve_move_info(entity_id, Some(rules))` exactly once.
3. For every teleporter-harvester with a locomotor, snapshot the exact copyable
   activation tuple before calling the primitive:
   `(kind, primary_kind, piggyback, layer, phase)`.
4. If the resolved entity is both `is_teleporter` and `is_harvester`, call the
   existing `begin_drive_piggyback_for_teleporter`.
5. Borrow the live terrain-cost grid by `MoveInfo.speed_type`, resolved terrain,
   and zone grid, then call `issue_move_command_with_layered` with:
   - resolved speed;
   - `queue=false`;
   - live terrain costs;
   - current resolved terrain and zone grid;
   - current mover crusher flag;
   - no new entity-block map, preserving the existing producer's blocker scope.
6. If issue fails, write the tuple from step 3 back exactly whenever one was
   captured. Do not call `restore_primary_from_piggyback()`. This is safe for an
   already-active Drive state because the identical pre-call tuple is restored;
   it also covers malformed/legacy combinations where the primitive would
   repair a missing piggyback before pathfinding fails.
7. If issue succeeds, stamp the created `MovementTarget` with the resolved
   acceleration factor, deceleration factor, and slowdown distance.
8. Return the command result. Do not create debug/RNG/state side effects.

### Caller flow

`handle_move_to_ore` keeps:

- present target validation;
- in-progress teleport wait;
- physical-arrival transition;
- rescan/retarget only when no owner destination or transitional movement exists;
- the existing `has_movement` no-reissue gate.

At entry, before reading or validating the selected resource node, checking
physical arrival, rescanning, or issuing, return when
`navigation.nav_com` is present or a `movement_target` exists. This is the
verified wrapper-entry ordering at `0x004DCFE0`, not merely a no-reissue
optimization. It preserves the native local state in which Drive has finished
its track but owner NavCom is still non-null until a later no-active-track
clear. Only when both owners are absent does the caller validate the retained
target, wait for any teleport state, rescan/retarget, perform the existing
physical-arrival transition, and call the new helper once when a path grid is
available. It no longer branches on adjacency and no longer marks the result
terrain-cost-exempt.

### Piggyback completion cleanup

Keep the existing `LocomotorState` restore predicate and field transition. In
`tick_locomotor_piggyback_restore`, after
`restore_primary_from_piggyback()` succeeds, set
`entity.drive_locomotion = None` and `entity.drive_track = None`. This is the
Rust-native ownership equivalent of `FootClass::AI @ 0x004DA530` releasing the
old active Drive locomotor before installing the stored primary locomotor.
Do not clear Drive runtime on a failed command rollback or while Drive remains
active.

### Failure behavior

- Missing entity, missing move info, out-of-bounds target, or no path returns
  false and does not attach a movement target.
- Only locomotor state created by this exact failed helper call is rolled back,
  by restoring the five fields the activation primitive can mutate.
- The miner remains in its current `MoveToOre` state with the target retained;
  the existing production tick may retry. This preserves current Rust retry
  behavior without inventing a new timer.
- No existing command/path state is cleared by this helper; the caller invokes
  it only when `movement_target` is absent.

### State and determinism

- No new serialized field.
- Existing NavCom, Drive runtime, DriveTrack, locomotor piggyback, and movement
  target fields are already serialized/hashed by their current owners.
- Successful primary restoration removes the retired Drive runtime rather than
  preserving inactive, hashed Drive state after native object release.
- All math stays in `SimFixed`.
- Entity access remains by stable entity ID; no collection iteration is added.
- The failure snapshot is a five-field copy tuple; it allocates nothing.
- No RNG consumption.

## Testing Strategy

Create `outbound_drive_tests.rs` and drive the public production tick, not the
private helper.

The success fixtures:

- reads and merges ignored retail `ini/rules.ini` and `ini/rulesmd.ini`;
- asserts retail HARV/CMIN type facts before use;
- stages a 64x64 clear resolved terrain grid with either one adjacent Tiberium
  cell or one aligned Tiberium cell at least three cells away, plus a positive
  stock ore node;
- builds `TerrainCostGrid` for every speed type and a live zone grid;
- spawns each stock miner through `Simulation::spawn_object`;
- advances with `Simulation::advance_tick`.

Assertions:

- first miner phase selects the adjacent ore cell;
- command issue does not consume scenario/main/mapgen RNG;
- `NavigationState.nav_com` and Drive destination/path state point at the target;
- the target profile equals merged `ObjectType` values;
- adjacent stock HARV starts at Drive current fraction zero and gains positive
  speed through the inside-`SlowdownDistance` brake-floor branch;
- the farther aligned HARV starts at zero and gains exactly the parsed
  `AccelerationFactor` on its first normal Drive frame;
- CMIN has active Drive, primary Teleport, and a stored piggyback on issue;
- CMIN old NavCom is null immediately before issue;
- CMIN has no `teleport_state` on every tick;
- both units physically leave their start and reach the target through the real
  loop;
- the existing miner state reaches Harvest;
- CMIN restoration, if it occurs within the fixture window, occurs only after
  movement has ended, leaves no retired Drive runtime, and does not advance the
  miner to Harvest until owner NavCom has cleared;
- the ore node and overlay remain valid through arrival.

Failure and authority fixtures:

- an unreachable selected CMIN ore target is admitted with no zone grid so the
  real miner producer reaches a synchronous A* failure; the five locomotor
  fields are identical before and after the failed issue;
- a successful HARV issue is followed by the bounded injected state
  `NavCom=Some`, `MovementTarget=None`, still away from target, plus a newly
  preferable ore candidate; one real production tick must retain the original
  target/NavCom and must not rescan or reissue.
- an en-route HARV loses its selected resource node while NavCom still owns the
  destination; one real tick must retain `MoveToOre` and the old target instead
  of validating or rescanning ahead of the owner gate.
- a real CMIN arrival must expose the pending owner-NavCom interval without
  entering Harvest, restore primary Teleport only after movement stops, release
  the retired Drive runtime, and enter Harvest only after NavCom clears.

Focused neighboring regressions:

- existing Drive acceleration true/false tests;
- existing locomotor piggyback tests;
- existing direct movement tests;
- existing miner/refinery tests selected by names affected by the producer;
- one final `cargo check -q`.

Parent resume acceptance:

- integrate prerequisite into clean `dev`;
- rebase/replay the suspended GSI-07.15 three-file diff onto validated `dev`;
- rerun the complete parent production suite, not merely the new prerequisite
  tests.

## Approval Challenge

Why should this be approved?

- It follows the verified native state-0 owner chain instead of repairing the
  symptom inside generic direct movement.
- It uses existing Rust-native NavCom, Drive, path, profile, and piggyback
  authorities with no new state or global interface.
- It is the smallest slice that serves both the immediate adjacent failure and
  the later archive drive-back path.
- It isolates tests from the suspended parent's large test-file diff.

What evidence could still make it wrong?

- Evidence that stock state-0 selected-cell movement bypasses UnitClass
  destination/Drive for adjacent cells would invalidate the unified command.
  The current `0x004DCFE0` and state-0 reports say the opposite.
- Evidence that old NavCom `NULL` preserves Teleport for outbound CMIN would
  invalidate explicit Drive activation. The asm-verified Teleporter gate says
  old NavCom `NULL` takes the Drive-default path.
- Evidence that HARV/CMIN explicitly use `Accelerates=false` would invalidate
  the ramp expectation. Retail INIs omit the key and current verified parser/
  binary defaults are true.
- Evidence that Tiberium is zero-cost for Track would require a bypass. Retail
  `[Tiberium] Track=70%` directly disproves that.
- Evidence that synchronous path failure should retain a newly created Drive
  piggyback would affect only the Rust-only failed-issue branch. The scoped
  production target is prefiltered reachable; this branch remains
  non-certifying and is handled by exact field restoration to avoid orphaned or
  phase-drifted state.
- Evidence that `Search_For_Tiberium_And_Move` scans with non-null NavCom would
  invalidate the new no-rescan gate. The verified wrapper decompile says it
  checks NavCom first and does not scan when present.

The plan-stage adversarial review found that the earlier approved wording put
the NavCom gate after validation/arrival and failed to retire the CMIN Drive
runtime. Both objections are now repaired: the owner gate is first, removed
targets and arrival ordering have separate production oracles, and successful
primary restoration releases the retired Drive runtime. Transactional rollback
and the adjacent/far speed split remain unchanged. Exact global
Drive/path/tick/pixel parity remains explicitly outside this prerequisite and
must not be inferred from its production pass.

## Sources

- `docs/contracts/2026-07-25-gsi-07-15-miner-outbound-drive-command-implementation-contract.md`
- `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
- `docs/research/miner/HARV_HARVEST_STATE_RETARGET_VISUAL_FLAG_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_SET_DESTINATION_INTERNAL_NAVCOM_HEADTO_HANDOFF_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`
- `docs/research/DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`
- `docs/research/FOOTCLASS_AI_GHIDRA_REPORT.md`
- `docs/research/DRIVE_ACCELERATES_TRUE_FALSE_SPEED_RAMP_GHIDRA_REPORT.md`
- `docs/research/DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md`
- `docs/plans/2026-05-27-drivelocomotion-current-state-parity-design.md`
- `ini/rulesmd.ini`
- `src/rules/object_type.rs`
- `src/sim/miner/miner_system.rs`
- `src/sim/movement/locomotor.rs`
- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/world/world_commands.rs`
- `docs/goals/2026-07-24-system-by-system-parity-state.md`
