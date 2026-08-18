# GSI-07.15 Miner Far-Return Drive Command Implementation Plan

Date: 2026-07-25  
Status: READY_FOR_REVIEW (repaired 2026-07-25 after shared mission-delay RNG verification)  
Parent owner: GSI-07.15 level-zero scan/move production loop

## Frozen Design Inputs

- Contract:
  `docs/contracts/2026-07-25-gsi-07-15-miner-far-return-drive-command-implementation-contract.md`
- Contract SHA-256:
  `463497132023906C71B7E9D4A24FE4B94D58F046A13A154A7382A9FC494357D7`
- Design:
  `docs/plans/2026-07-25-gsi-07-15-miner-far-return-drive-command-design.md`
- Design SHA-256:
  `38D5CCF1D254877944BAB5F3B12AB83B7B907744EC876F5497415C5B8F0A3D1B`
- Approval:
  `docs/approvals/2026-07-25-gsi-07-15-miner-far-return-drive-command-design-approval.md`

Any change to either frozen input invalidates this plan review.

## Current Execution Boundary

Per the user's 2026-07-25 stop instruction, this run executes the owned
far-return/Drive prerequisite through Task 6 and records its handoff under
Task 8. Task 7 (suspended-parent replay/validation) is explicitly deferred to
the next session. This run must not select another owner or begin another
dependency slice after the prerequisite is stable.

## Grounding Summary

Confidence: high for the bounded owner gate, destination handoff, rule profile,
and speed ramp; unknown for immediate Rust A* failure/retry timing and native
immediate `head_to`.

Verified active-YR anchors:

- state-2 owner gate and far branch:
  `UnitClass::Mission_Harvest @ 0x0073E5E0`;
- valid fallback destination call: `0x0073EDB5`;
- normal destination chain:
  `0x00741970 -> 0x004D94B0 -> 0x004AFD40`;
- Drive speed ramp: `0x004B0F20`;
- stock QueueingCell: merged `[GAREFN] QueueingCell=4,1`;
- stock HARV: Drive locomotor, `Teleporter=no`, Track, Speed 4,
  `Accelerates=true`, acceleration `0.03`, deceleration `0.002`, slowdown
  `500`.

Current Rust facts:

- `handle_return` starts near
  `src/sim/miner/miner_system.rs:694` and currently selects/stores a refinery
  without the verified stock-HARV NavCom gate.
- `try_issue_standard_far_return_drive` starts near
  `src/sim/miner/miner_system.rs:1093` and calls metadata-free
  `issue_move_if_idle`.
- `issue_outbound_ore_move` starts near
  `src/sim/miner/miner_system.rs:1458`; it already resolves merged `MoveInfo`,
  terrain, zone, crusher, Drive/Teleport activation, rollback, and movement
  profile, but it does not own an existing-destination gate.
- merged-retail helpers and seven production outbound tests live in
  `src/sim/miner/outbound_drive_tests.rs`.

## File Ownership

Implementation branch owns only:

- `src/sim/miner/miner_system.rs`
- `src/sim/miner/outbound_drive_tests.rs`

No other Rust, INI, research, snapshot, dependency, serialization, render,
audio, or UI file may change. Root `dev` remains integration-only. The parent
worktree remains suspended and untouched until this prerequisite is merged.

## Task 1: Create And Reconcile The Feature Worktree

Create a timestamped branch and worktree from the current validated `dev`
commit:

```powershell
$repo = '.'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$branch = "feature/gsi-07-15-miner-far-return-drive-command-$stamp"
$worktree = "<local>/Documents/ra2-rust-game-gsi-07-15-miner-far-return-drive-command-$stamp"
git -C $repo worktree add -b $branch $worktree dev
```

Before editing:

- verify root `dev` remains tracked-clean at the expected commit;
- verify the new worktree is clean and based on the same commit;
- verify the parent worktree still has only its three staged files;
- verify recovery stash
  `8ef7d05070ca63fa97ba9b79b5643403bc126c1c` remains present;
- verify no Cargo/rustc process is active and take the one global Cargo lease.

Stop if any tracked state differs.

## Task 2: Add The Red-First Merged-Retail Oracles

Edit `src/sim/miner/outbound_drive_tests.rs`.

### Imports

Extend the miner import with:

```rust
CargoBale, MinerConfig
```

Do not add a test-only rules substitute. Derive cargo value and thresholds from:

```rust
let config = MinerConfig::from_rules(&oracle.rules);
```

### Shared Far Geometry

In each new test:

```rust
let refinery_anchor = (10, 10);
let refinery = oracle.rules.object("GAREFN").expect("GAREFN");
let queueing = refinery.queueing_cell.expect("stock QueueingCell");
let staging = (
    refinery_anchor.0 + queueing.0,
    refinery_anchor.1 + queueing.1,
);
let accepted_dock = (refinery_anchor.0 + 3, refinery_anchor.1 + 1);
```

Use a mutable `PathGrid`, call
`block_building_movement_cells` with the merged GAREFN foundation/Bib, prove
the foundation is blocked and `staging` remains walkable, then call
`install_world` so zone authority reflects the blocked building.

Spawn GAREFN with `Simulation::spawn_object`, assert its normal lifecycle
facts, then spawn HARV with `spawn_stock_miner`.

### Positive Test

Add:

```rust
#[test]
fn production_stock_harv_far_return_drive_uses_rule_profile()
```

Fixture:

- full stock HARV cargo:

```rust
miner.cargo = (0..miner.capacity_bales)
    .map(|_| CargoBale {
        resource_type: ResourceType::Ore,
        value: config.ore_bale_value,
    })
    .collect();
```

- `miner.state = MinerState::ReturnToRefinery`;
- `miner.reserved_refinery = None`;
- START `(32,32)` is farther than merged
  `config.too_far_threshold_standard` from refinery anchor;
- capture the full starting position tuple.

After one production `advance`:

- state is still `ReturnToRefinery`;
- selected reservation is the spawned GAREFN;
- `MovementTarget.final_goal == Some(staging)`;
- NavCom is `NavTargetRef::cell(staging)`;
- Drive destination is `DriveCoord::cell(staging.0, staging.1, 0)`;
- `staging != accepted_dock`;
- speed equals `ra2_speed_to_leptons_per_second(HARV.Speed)`;
- acceleration, deceleration, and slowdown equal the merged HARV object;
- current Drive fraction starts at `0`.

Do not call `assert_command_state` and do not assert immediate `head_to` or a
particular A* path shape.

After the next production `advance`:

- `Drive.current_speed_fraction == HARV.accel_factor`;
- `MovementTarget.current_speed == MovementTarget.speed * HARV.accel_factor`;
- current speed is positive.

Continue for at most 96 production ticks and require
`position_tuple != start`. This test must fail on unmodified code at the zero
acceleration/profile assertion or bounded departure assertion.

Do not assert unchanged RNG across these full production dispatches. Live
Ghidra re-verification shows the valid far-destination path jumps to
`0x0073EF77`, calls `Scenario+0x218.RandomRanged(0,2)`, and returns the current
mission base delay plus jitter. That scheduler mechanism is a separate
residual; this prerequisite must neither certify the current no-draw behavior
nor add an isolated draw without its delay consumer.

### Negative Owner-Gate Test

Add:

```rust
#[test]
fn production_stock_harv_far_return_preserves_existing_navcom_owner()
```

Use the same refinery geometry plus an ordinary ore target `(32,29)`. Spawn
and arm a stock HARV, then run the existing two production advances that issue
its outbound Drive command. Prove the production-created NavCom and Drive
destination own that ore cell with direct field assertions. Do not call
`assert_command_state`; its immediate `head_to` and A* path-shape requirements
are outside this parity slice.

Then:

- remove only `entity.movement_target`;
- fill cargo from `MinerConfig::from_rules`;
- set miner state to `ReturnToRefinery`;
- set `reserved_refinery=None`;
- retain `target_ore_cell`, NavCom, and Drive runtime;
- snapshot NavCom, the full `DriveLocomotionRuntime`, target cell, cargo,
  all six `MissionTimer` fields, reservation, sound-event length, GAREFN/miner
  radio/contact facts.

Advance one production tick and require:

- state remains `ReturnToRefinery`;
- reservation remains `None`;
- NavCom and full Drive runtime equal their snapshots;
- `movement_target` remains absent;
- target, cargo, and every timer equal their snapshots;
- no dock reservation/contact/entered/on-pad fact exists;
- neither entity gained a radio contact;
- sound-event length is unchanged.

This must fail on unmodified code because `handle_return` selects a refinery and
replaces the existing owner before a helper-local guard could run.

Do not assert unchanged RNG here either. The verified non-null destination jump
at `0x0073EB62` reaches the same native mission-delay jitter tail.

### Red Validation

Format only the edited test file:

```powershell
rustfmt --edition 2024 src/sim/miner/outbound_drive_tests.rs
```

Inspect the diff, then run serially:

```powershell
cargo test -p vera20k --lib production_stock_harv_far_return_drive_uses_rule_profile -- --nocapture
cargo test -p vera20k --lib production_stock_harv_far_return_preserves_existing_navcom_owner -- --nocapture
```

Record both literal failing `test result:` lines and their intended assertion
sites. If either passes before production changes, stop and repair the oracle.
If either fails for fixture/compile reasons, repair the test and repeat red.

## Task 3: Restore The Native State-2 Owner Gate

Edit `src/sim/miner/miner_system.rs`.

In `handle_return`, immediately after the existing teleport-state early return
and before `reserved_refinery` selection, add:

```rust
let has_destination_or_movement =
    snap.miner.kind == MinerKind::War
        && sim
            .substrate
            .entities
            .get(snap.entity_id)
            .is_some_and(|entity| {
                entity.navigation.nav_com.is_some() || entity.movement_target.is_some()
            });
if has_destination_or_movement {
    return;
}
```

Comment why:

- active-YR HARV state 2 checks `Foot/Unit+0x5A4` before refinery selection;
- `MovementTarget` remains Rust's transitional duplicate movement owner;
- CMIN is excluded because this contract does not generalize its return
  mechanism.

Do not:

- set or clear `reserved_refinery`;
- clear/rewrite NavCom, Drive runtime, movement, mission state, or timers;
- emit radio, sound, or RNG work;
- move the gate into `try_issue_standard_far_return_drive`;
- alter close-HARV or CMIN logic.

## Task 4: Reuse The Full Miner Drive Command Authority

Still in `src/sim/miner/miner_system.rs`:

1. Rename:

```rust
issue_outbound_ore_move
```

to:

```rust
issue_stock_miner_drive_move
```

2. Change its purpose comment to state that it hands a selected stock-miner
   destination to the normal Drive command authority.
3. Update the `handle_move_to_ore` call with no other outbound change.
4. Replace the metadata-free call inside
   `try_issue_standard_far_return_drive` with:

```rust
let _ = issue_stock_miner_drive_move(sim, rules, grid, snap.entity_id, staging);
```

5. Keep the following state write and `true` return exactly where they are.

The helper body must otherwise remain unchanged. In particular, preserve:

- bounds and `resolve_move_info` failures;
- CMIN Drive piggyback activation and exact rollback;
- terrain cost, resolved terrain, zone grid, and crusher inputs;
- generic command parameters;
- post-success acceleration, deceleration, and slowdown stamping.

Do not make the wrapper return the helper boolean.

## Task 5: Focused Green Validation And Diff Audit

Format only the two edited Rust files:

```powershell
rustfmt --edition 2024 src/sim/miner/miner_system.rs src/sim/miner/outbound_drive_tests.rs
```

Inspect both staged and unstaged diffs. Require exactly:

- one War-only early owner gate;
- one private helper rename;
- two caller updates;
- one far-return caller replacement;
- two production tests and required imports;
- no unrelated formatting churn.

Run serially, checking Cargo/rustc ownership before the first command:

```powershell
cargo test -p vera20k --lib production_stock_harv_far_return_drive_uses_rule_profile -- --nocapture
cargo test -p vera20k --lib production_stock_harv_far_return_preserves_existing_navcom_owner -- --nocapture
cargo test -p vera20k --lib outbound_drive_tests -- --nocapture
```

Required literal results:

- positive: `1 passed; 0 failed`;
- negative: `1 passed; 0 failed`;
- outbound module: `9 passed; 0 failed`.

Then run these exact neighbors serially:

```powershell
cargo test -p vera20k --lib drive_target_speed_fraction_uses_terrain_modifier -- --nocapture
cargo test -p vera20k --lib drive_accelerates_true_tick_ramps_fraction_before_movement_speed -- --nocapture
cargo test -p vera20k --lib drive_piggyback_restores_primary_teleport_only_after_not_moving -- --nocapture
cargo test -p vera20k --lib return_close_enough_to_refinery_enters_dock -- --nocapture
cargo test -p vera20k --lib chrono_return_over_too_far_threshold_uses_queueingcell_teleport -- --nocapture
cargo test -p vera20k --lib harvester_uses_dock_list_for_refinery_selection -- --nocapture
cargo test -p vera20k --lib full_dock_cycle_war_miner -- --nocapture
```

Each command must report `1 passed; 0 failed`. The outbound module's nine
tests already include the exact CMIN activation/rollback and NavCom production
neighbors.

Finally:

```powershell
cargo check -q
```

Require zero errors. A passing Rust-vs-Rust test is only a bounded
mechanism/result check; record broader Drive/path/render/audio/byte/pixel parity
as UNVERIFIED.

## Task 6: Adversarial Review, Commit, And Local Dev Merge

Before committing, have independent read-only reviewers answer:

- Does the code implement the exact frozen plan?
- Can any path replace an existing stock-HARV owner before the gate?
- Did the far wrapper preserve the ignored A* result and same-tick state write?
- Did the test accidentally certify immediate `head_to` or path shape?
- Did outbound CMIN rollback or rule/terrain/zone inputs change?
- What evidence could still make this wrong?

Repair and rerun all affected validation for any load-bearing objection.

Stage only the two owned Rust files and commit one coherent milestone:

```text
miner: preserve far-return Drive authority
```

Reconfirm root `dev` has not moved and is tracked-clean. Merge locally into
`dev` with a no-fast-forward merge commit. Never push.

Post-merge, rerun in root:

```powershell
cargo test -p vera20k --lib outbound_drive_tests -- --nocapture
cargo test -p vera20k --lib drive_target_speed_fraction_uses_terrain_modifier -- --nocapture
cargo test -p vera20k --lib drive_accelerates_true_tick_ramps_fraction_before_movement_speed -- --nocapture
cargo test -p vera20k --lib drive_piggyback_restores_primary_teleport_only_after_not_moving -- --nocapture
cargo test -p vera20k --lib return_close_enough_to_refinery_enters_dock -- --nocapture
cargo test -p vera20k --lib chrono_return_over_too_far_threshold_uses_queueingcell_teleport -- --nocapture
cargo test -p vera20k --lib harvester_uses_dock_list_for_refinery_selection -- --nocapture
cargo test -p vera20k --lib full_dock_cycle_war_miner -- --nocapture
cargo check -q
```

Require `9 passed; 0 failed` for the outbound module, `1 passed; 0 failed` for
each exact neighbor, and zero errors before releasing the Cargo lease.

## Task 7: Replay And Validate The Suspended Parent

Only after the prerequisite is merged and root validation is green:

1. Verify the parent still has exactly its three staged files and no unstaged
   changes.
2. Run this loss-safe recovery sequence, retaining the existing
   `8ef7d05070ca63fa97ba9b79b5643403bc126c1c`:

```powershell
$parent = '<local>/Documents/ra2-rust-game-gsi-07-15-level-zero-scan-move-20260725-102933'
$expectedParentPaths = @(
    'src/sim/miner/miner_system.rs'
    'src/sim/miner/miner_tests.rs'
    'src/sim/slave_miner.rs'
) | Sort-Object

$stagedParentPaths = @(
    git -C $parent diff --cached --name-only |
        Where-Object { $_ } |
        Sort-Object
)
if (Compare-Object $expectedParentPaths $stagedParentPaths) {
    throw 'parent staged path set is not exact'
}
if (git -C $parent diff --name-only) {
    throw 'parent has unstaged tracked changes'
}

git -C $parent stash push --staged `
    -m 'gsi-07-15-parent-before-far-return-drive-prereq' -- `
    src/sim/miner/miner_system.rs `
    src/sim/miner/miner_tests.rs `
    src/sim/slave_miner.rs
if ($LASTEXITCODE -ne 0) { throw 'parent staged stash failed' }

$newParentStash = (git -C $parent rev-parse 'stash@{0}').Trim()
git -C $parent cat-file -e "$newParentStash^{commit}"
if ($LASTEXITCODE -ne 0) { throw 'new parent stash object is missing' }

$actualParentPaths = @(
    git -C $parent stash show --name-only --format= $newParentStash |
        Where-Object { $_ } |
        Sort-Object
)
if (Compare-Object $expectedParentPaths $actualParentPaths) {
    throw 'new parent stash path set is not exact'
}
if (git -C $parent status --porcelain) {
    throw 'parent is not clean after staged stash'
}
git -C $parent merge --ff-only dev
if ($LASTEXITCODE -ne 0) { throw 'parent fast-forward failed' }

git -C $parent stash apply --index $newParentStash
```

Never use `pop`. Record the exact new stash hash before the fast-forward. If
the apply reports the expected `miner_system.rs` overlap, resolve that one
overlap and continue; stop for any other conflict. Retain both exact stash
objects through merged-parent validation.

3. After apply/conflict resolution, verify the restored path set is still
   exactly the three parent-owned files and the intended changes are staged.
4. Resolve only the expected `miner_system.rs` overlap, preserving:
   - the merged owner gate and full far-return Drive issuer;
   - the parent's zero-resource scan/archive changes.
5. Format only the three parent-owned Rust files and inspect the complete
   diff.

Rerun serially:

```powershell
cargo test -p vera20k --lib outbound_drive_tests -- --nocapture
cargo test -p vera20k --lib production_stock_miners_accept_present_zero_ring_zero -- --nocapture
cargo test -p vera20k --lib production_stock_miners_filter_and_travel_to_present_zero_ring_one -- --nocapture
cargo test -p vera20k --lib standard_present_zero_scan_preserves_value_tie_and_first_ring_order -- --nocapture
cargo test -p vera20k --lib present_zero_resource_node_changes_state_hash -- --nocapture
cargo test -p vera20k --lib production_full_harv_archives_zero_through_dock_and_drives_back -- --nocapture
cargo test -p vera20k --lib slave_search_preserves_current_unverified_zero_rejection -- --nocapture
cargo test -p vera20k --lib drive_target_speed_fraction_uses_terrain_modifier -- --nocapture
cargo test -p vera20k --lib drive_accelerates_true_tick_ramps_fraction_before_movement_speed -- --nocapture
cargo test -p vera20k --lib drive_piggyback_restores_primary_teleport_only_after_not_moving -- --nocapture
cargo test -p vera20k --lib return_close_enough_to_refinery_enters_dock -- --nocapture
cargo test -p vera20k --lib chrono_return_over_too_far_threshold_uses_queueingcell_teleport -- --nocapture
cargo test -p vera20k --lib harvester_uses_dock_list_for_refinery_selection -- --nocapture
cargo test -p vera20k --lib full_dock_cycle_war_miner -- --nocapture
cargo check -q
```

Require `9 passed; 0 failed` for the outbound module, `1 passed; 0 failed` for
each other test filter, and zero check errors.

Commit the parent only after every branch result is green, merge it locally
into `dev`, and rerun every exact command in the preceding block in root with
the same required literal results. Verify both recovery stashes by exact hash
before dropping them; retain every stash until the merged parent is
independently recoverable.

## Task 8: Journal And Handoff

Update
`docs/goals/2026-07-24-system-by-system-parity-state.md` with:

- exact branch, worktree, feature commit, and merge commit hashes;
- red and green literal `test result:` lines;
- branch and post-merge `cargo check -q` results;
- explicit deferred-parent state: parent branch/worktree, exact staged paths,
  retained recovery stash hash, and next safe replay action. Do not claim
  parent conflict resolution or its six production results in this run;
- verification level: bounded mechanism/result checks, not certification;
- residuals:
  - close-HARV accepted-dock metadata-free movement;
  - CMIN refused-contact metadata-free staging;
  - immediate Drive `head_to` timing;
- native path-failure/retry timing;
- shared state-2 `base mission delay + RandomRanged(0,2)` scheduler ownership
  and RNG call order;
- broader A* terrain weighting and Drive residual/track parity;
- next exact owner/action after the parent completes.

Before any cutoff, leave no Cargo, merge, rebase, or stash-pop operation in
progress, keep root tracked-clean apart from protected untracked paths, and
record a crash-safe handoff. Never push.
