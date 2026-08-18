# GSI-07.15 Harvest Filling-Return Gate Implementation Plan

Date: 2026-07-24  
Plan status: AUTONOMOUSLY_APPROVED_FOR_IMPLEMENTATION  
Committed base reviewed:
`dev` `68302b5d2d0b558400e2e0cf9b51c6994fa180c7`

> **For Codex:** Execute this plan task-by-task in the dedicated feature
> worktree. The primary coordinator alone runs Cargo and performs integration.

**Goal:** A standard harvester that becomes exactly full from a positive
extraction remains in Harvest until the verified `F+19` full gate, then writes
Return before scanning its archive and begins refinery work only on the next
miner tick.

**Architecture:** Keep the existing `Miner` state/timer/archive owner and the
existing per-tick state dispatcher. Reorder only `handle_harvest`: a due,
already-full miner takes a pre-reducer failure branch; a positive extraction
always rearms and remains Harvest; the later full branch writes Return, scans
the archive, and returns without calling `begin_return`.

**Design Doc:**
`docs/plans/2026-07-24-gsi-07-15-harvest-filling-return-gate-design.md`

**Implementation Contract:**
`docs/contracts/2026-07-24-gsi-07-15-harvest-filling-return-gate-implementation-contract.md`

**Design Approval:**
`docs/approvals/2026-07-24-gsi-07-15-harvest-filling-return-gate-design-approval.md`

---

## Execution Preconditions

- Reconcile immediately before worktree creation:
  - main checkout is on `dev`;
  - `refs/heads/dev` resolves to a commit;
  - main tracked worktree is clean;
  - no Cargo/rustc process belongs to another session;
  - no existing branch/worktree uses the exact names below;
  - none of the three owned paths is dirty in another worktree.
- Planned branch:
  `feature/gsi-07-15-harvest-fill-gate-20260724-1525`
- Planned linked worktree:
  `<local>/Documents/ra2-rust-game-gsi-07-15-harvest-fill-gate-20260724-1525`
- Create the branch/worktree from the exact committed `refs/heads/dev` observed
  at creation time. If `dev` moved since planning and any owned miner path
  changed, stop implementation and re-run plan review against the new base.
- Feature-owned Rust paths only:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/miner/mod.rs` (comments only)
- Protected non-owned work:
  `<local>/Documents/ra2-rust-game-gsi-08-10-damage-authority` and all of
  its dirty rules/combat/entity/world/snapshot/hash paths.
- Do not touch `SNAPSHOT_VERSION`, snapshots, goldens, research docs, Cargo
  manifests, or Ghidra labels from the feature worktree.

## Grounding Summary

- `Harvest_Ore_Tick @ 0x0073D450` returns success after any positive normal
  extraction, resets the StepTimer, and does not write mission substate—even
  when the added amount makes storage exactly full.
- `Mission_Harvest @ 0x0073E5E0` returns directly on that success. Archive and
  return selection occur only after a later helper call returns false.
- `TechnoClass::AI_Update @ 0x006F9E50` calls Mission dispatch at
  `0x006FA655`, then maintains the shared timer at
  `0x006FABC4..0x006FAC22`.
- With stock `HarvesterLoadRate=2`, the ninth increment is written after the
  mission at `F+18`; the mission first observes it at `F+19`.
- The full helper check at `0x0073D4B6..0x0073D4BC` occurs before
  `Reduce_Tiberium` and resets timer fields on failure.
- The false caller writes return substate 2 before its short archive scan at
  `0x0073E9D0..0x0073EA7B`; state-2 refinery work waits for the next mission
  dispatch.
- Rust already has an inclusive `MissionTimer`; `harvest_tick_interval + 1`
  exactly reproduces the bounded stock reset-to-helper observation and must
  remain.
- Rust currently calls the reducer first, transitions immediately on a filling
  extraction, scans archive early, and calls effectful `begin_return` from the
  same state-1 handler.
- Existing `tick_miners_in_order` mutates shared ore immediately, writes miner
  snapshots back, then derives `VoxelAnimation.playing` and
  `HarvestOverlay.visible` from `MinerState::Harvest`.
- `[General] HarvesterLoadRate` is already parsed; stock base and YR patch INIs
  do not override it, leaving native/Rust default `2`. No parser task is needed.
- The initial search/move-to-first-extraction timer drift remains a named sibling
  and is not consumed by this plan because the positive filling success at `F`
  resets the cadence.

## Key Technical Decisions

- **Keep `harvest_tick_interval + 1`.** The live AI-order proof establishes
  `F+19`; changing to the nominal 18-frame threshold would fire one frame early.
  **Confidence: high.**
  - **Source:** live `0x006FA655`,
    `0x006FABC4..0x006FAC22`, `0x0073E96F..0x0073E987`;
    corrected harvest timing report.
- **Use the existing due-time full predicate; add no pending latch.** Native
  rechecks storage later, and current cargo fullness plus timer already owns the
  condition. **Confidence: high.**
  - **Source:** live `0x0073D4B6..0x0073D626`; design alternatives.
- **Order the later branch as reset -> Return state -> archive scan -> return.**
  State-write order is verified and no state-2 logic runs recursively.
  **Confidence: high.**
  - **Source:** live `0x0073E9D0..0x0073EA7B`; independent design review.
- **Use `MissionTimer::reset(now)` for the bounded full failure.** It produces
  current-frame start plus zero duration, the closest existing Rust owner for
  the native failure reset, without new state. **Confidence: high for this
  scoped branch; generic intermediate StepTimer bytes remain unverified.**
  - **Source:** `src/sim/mission/timer.rs`; implementation contract scope.
- **Prove return dispatch with a concrete next-tick fixture.** A far War Miner
  must have no reservation/movement at `F+19`, then reserve and issue movement
  at `F+20`. **Confidence: high.**
  - **Source:** existing `handle_return` and War far-return path; adversarial
    design review.

## Open Questions

### Resolved During Planning

- **Is Rust one frame late today?** No for success-reset-to-next-helper. Its
  `interval + 1` is correct; old `F+18` prose was stale.
- **Does a filling success itself choose Return?** No. The helper returns true
  and the caller exits state 1 unchanged.
- **Does the later full gate call the cell reducer with zero capacity?** No. The
  native full check precedes the reducer.
- **May `begin_return` run in the later state-1 call?** No. State 2 begins on a
  later mission dispatch.
- **Is search/arrival timer drift a prerequisite?** No for this loop: the
  filling success resets the timer at the selected starting boundary.
- **Can visual tests rely on `spawn_miner` defaults?** No. The helper attaches
  neither optional visual component; tests must install both.

### Deferred Outside This Plan

- Initial timer initialization on search-and-move success, destination-present
  helper calls, and variable physical-arrival latency are a separate GSI-07.15
  design.
- Generic native StepTimer intermediate fields and extreme/non-positive modded
  load rates remain unverified.
- Fractional and overfull storage behavior is outside the exact-full stock
  fixture.
- GSI-04.09 exact-match/density-zero reducer semantics and radar/tactical dirty
  split remain on the dependency stack after this prerequisite integrates.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/miner/miner_tests.rs` | Production-path acceptance for fill, `F+18/F+19`, visuals/archive, and concrete `F+20` return action |
| Modify | `src/sim/miner/miner_system.rs` | Correct state-1 success/full branch order and state-dispatch boundary |
| Modify (comments only) | `src/sim/miner/mod.rs` | Distinguish the 18-frame ninth-step threshold from the `F+19` helper observation |

No file is created, no module grows, and no public API changes.

## Interface Changes

None.

- No new type, enum variant, field, trait, export, event, config key, or schema.
- Existing private helpers keep their signatures.
- Existing `begin_return` remains unchanged and continues serving other
  no-resource/forced paths; this plan only stops calling it from the scoped full
  gate.

## Sim Checklist

- [x] No new math; no `f32`/`f64`.
- [x] No new state or hash-format change.
- [x] No dependency on render/ui/sidebar/audio/net.
- [x] Tick-order impact is explicit: state-2 effect work moves to the next
  top-level miner dispatch.
- [x] Live-object snapshot order remains unchanged.
- [x] Shared resource mutation remains immediate for later live-order miners.
- [x] No RNG draw is added, removed, or reordered in the scoped branch.

## Risk Areas

- Moving the full check too early relative to the due gate would make a miner
  return before waiting 19 frame numbers.
- Leaving the old newly-full branch in place would preserve the original bug
  despite a new precheck.
- Calling `begin_return` at `F+19` would still collapse the native state
  boundary by one tick.
- Scanning archive before writing Return would violate verified state-write
  order.
- Removing `+1` would introduce a new timing drift.
- A test without a valid far refinery cannot detect premature reservation,
  movement, teleport, or sound.
- A test without explicit visual components can pass vacuously.
- Editing the no-bale/not-full continuation branch could broaden the feature
  into the sibling retarget loop.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Filling extraction remains `Harvest` | Native success does not write substate or archive; visual stays active | Production test immediately after fill |
| 1 | `F+18` pending, `F+19` due | One-frame error changes every full harvest cycle | Exact `MissionTimer` field/boundary assertions |
| 1 | Visual components present and nonzero | State timing is player-visible through voxel/OREGATH | Explicit active components with nonzero frame/elapsed preconditions, then inactive/zero assertions |
| 1 | Archive uses `F+19` resource state | Early scan remembers a different patch after intervening world change | Remove/add candidate during wait and assert final archive |
| 1 | No state-2 effects at `F+19`, concrete effects at `F+20` | Native mission boundary affects reservation, movement, chrono/audio order | Far reachable refinery fixture |
| 2 | Full check occurs after due but before reducer | Full helper failure must not consume/mutate ore | Code order plus unchanged cell fixture |
| 2 | Failure order is reset -> Return -> archive | Exact state-write/call order is part of parity | Code inspection and state/archive assertions |
| 2 | Positive branch always rearms and returns | Filling success follows the same helper success path | Production test and focused branch review |
| 2 | No `begin_return` in scoped full branch | Prevents same-tick reservation/teleport/audio | Negative assertions at `F+19` |
| 3 | Comments retain threshold/observation distinction | Prevents a future “fix” that removes the required `+1` | Diff review |
| 4 | Only three owned paths committed | Preserves concurrent user work and protected worktree | `git diff --name-only`, staged diff audit |

---

## Tasks

### Task 1: Add production-loop regression coverage

**Why:** Pin every observable and state-order boundary before modifying the
handler, using fixtures that fail on current `dev`.

**Files:**

- Modify:
  `src/sim/miner/miner_tests.rs` near
  `harvester_caps_extraction_at_remaining_capacity`

**Pattern:** Existing miner acceptance tests drive
`miner_system::tick_miners` through `tick_miners_n`, inspect entity/miner
fields directly, and use real refinery/path fixtures.

**Step 1: Import the optional visual components**

Change the current component import to:

```rust
use crate::sim::components::{HarvestOverlay, Health, VoxelAnimation};
```

**Step 2: Strengthen the existing exact-capacity test**

Replace the post-tick assertions in
`harvester_caps_extraction_at_remaining_capacity` so the fixture also proves
the successful filling result:

```rust
let miner = get_miner(&sim, miner_id);
assert_eq!(miner.cargo.len(), 40, "capped at capacity");
assert_eq!(
    miner.state,
    MinerState::Harvest,
    "positive filling extraction remains a successful Harvest tick"
);
assert_eq!(
    miner.harvest_timer.duration,
    u32::from(config.harvest_tick_interval) + 1,
    "success-reset gate remains due at the native F+19 observation"
);
assert_eq!(miner.last_harvest_cell, None, "archive is not selected on fill");
assert_eq!(miner.reserved_refinery, None, "return does not begin on fill");

let entity = sim
    .substrate
    .entities
    .get(miner_id)
    .expect("miner entity");
assert!(entity.movement_target.is_none());
assert!(entity.teleport_state.is_none());

let after = sim
    .production
    .resource_nodes
    .get(&(20, 20))
    .expect("cell still has ore");
assert_eq!(after.remaining, 9 * 120, "cell drops to density 9");
```

**Step 3: Add the complete War Miner loop test**

Add a test named
`filling_extraction_waits_for_full_gate_before_war_return` with this exact
fixture and assertions:

```rust
#[test]
fn filling_extraction_waits_for_full_gate_before_war_return() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    place_ore(&mut sim, 30, 30, 11 * 120);
    place_ore(&mut sim, 31, 30, 5 * 120);
    spawn_refinery(&mut sim, 2, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 30, 30);

    {
        let entity = sim
            .substrate
            .entities
            .get_mut(miner_id)
            .expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..38 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: config.ore_bale_value,
            });
        }
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((30, 30));
        miner.harvest_timer.clear();
        let mut voxel = VoxelAnimation::new(15, 67);
        voxel.frame = 7;
        voxel.elapsed_ms = 31;
        voxel.playing = true;
        entity.voxel_animation = Some(voxel);
        entity.harvest_overlay = Some(HarvestOverlay {
            frame: 6,
            visible: true,
            elapsed_ms: 29,
        });
    }

    tick_miners_n(&mut sim, &rules, 1);
    let fill_frame = sim.session.binary_frame;
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(miner.cargo.len(), 40);
        assert_eq!(miner.state, MinerState::Harvest);
        assert_eq!(miner.harvest_timer.start_frame, fill_frame);
        assert_eq!(
            miner.harvest_timer.duration,
            u32::from(config.harvest_tick_interval) + 1
        );
        assert_eq!(miner.last_harvest_cell, None);
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        assert!(entity.teleport_state.is_none());
        let voxel = entity.voxel_animation.expect("voxel anim");
        assert!(voxel.playing);
        assert_eq!((voxel.frame, voxel.elapsed_ms), (7, 31));
        let overlay = entity.harvest_overlay.expect("harvest overlay");
        assert!(overlay.visible);
        assert_eq!((overlay.frame, overlay.elapsed_ms), (6, 29));
    }

    tick_miners_n(
        &mut sim,
        &rules,
        config.harvest_tick_interval as usize,
    );
    assert_eq!(
        sim.session.binary_frame.wrapping_sub(fill_frame),
        u32::from(config.harvest_tick_interval)
    );
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(miner.state, MinerState::Harvest, "F+18 remains pending");
        assert_eq!(miner.last_harvest_cell, None);
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        let voxel = entity.voxel_animation.expect("voxel anim");
        assert!(voxel.playing);
        assert_eq!(
            (voxel.frame, voxel.elapsed_ms),
            (7, 31),
            "nonzero visual state remains live through F+18"
        );
        let overlay = entity.harvest_overlay.expect("harvest overlay");
        assert!(overlay.visible);
        assert_eq!(
            (overlay.frame, overlay.elapsed_ms),
            (6, 29),
            "nonzero overlay state remains live through F+18"
        );
    }

    sim.production.resource_nodes.remove(&(30, 30));
    tick_miners_n(&mut sim, &rules, 1);
    let full_gate_frame = sim.session.binary_frame;
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(
            full_gate_frame.wrapping_sub(fill_frame),
            u32::from(config.harvest_tick_interval) + 1
        );
        assert_eq!(miner.state, MinerState::ReturnToRefinery);
        assert_eq!(miner.harvest_timer.start_frame, full_gate_frame);
        assert_eq!(miner.harvest_timer.duration, 0);
        assert_eq!(miner.last_harvest_cell, Some((31, 30)));
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        assert!(entity.teleport_state.is_none());
        let voxel = entity.voxel_animation.expect("voxel anim");
        assert!(!voxel.playing);
        assert_eq!((voxel.frame, voxel.elapsed_ms), (0, 0));
        let overlay = entity.harvest_overlay.expect("harvest overlay");
        assert!(!overlay.visible);
        assert_eq!((overlay.frame, overlay.elapsed_ms), (0, 0));
    }

    tick_miners_n(&mut sim, &rules, 1);
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(miner.reserved_refinery, Some(2));
        assert!(
            entity.movement_target.is_some(),
            "F+20 state-2 dispatch issues the existing far HARV return move"
        );
    }
}
```

The existing 67-ms helper advances exactly one binary frame per call throughout
this bounded 20-call window (`floor(67 * n * 15 / 1000) == n` for
`1 <= n <= 20`). Keep the explicit frame-delta assertions above; do not add a
test-only production timing path.

**Step 4: Add a Chrono negative-boundary test**

Add
`chrono_filling_extraction_does_not_warp_before_state2_tick` exactly as below.
The source cell stays productive through the due full gate, so this fixture also
pins that the full precheck leaves the current node untouched. Position
`(63,63)` is inside the existing helper's 64-by-64 path grid and is still
strictly farther than the stock 50-cell Chrono threshold from the refinery at
`(10,10)`.

```rust
#[test]
fn chrono_filling_extraction_does_not_warp_before_state2_tick() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();

    place_ore(&mut sim, 63, 63, 11 * 120);
    spawn_refinery(&mut sim, 2, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 63, 63);

    {
        let entity = sim
            .substrate
            .entities
            .get_mut(miner_id)
            .expect("miner entity");
        let miner = entity.miner.as_mut().expect("miner component");
        for _ in 0..18 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: config.ore_bale_value,
            });
        }
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((63, 63));
        miner.harvest_timer.clear();
    }

    sim.sound_events.clear();
    tick_miners_n(&mut sim, &rules, 1);
    let fill_frame = sim.session.binary_frame;
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(miner.cargo.len(), 20);
        assert_eq!(miner.state, MinerState::Harvest);
        assert_eq!(miner.harvest_timer.start_frame, fill_frame);
        assert_eq!(
            miner.harvest_timer.duration,
            u32::from(config.harvest_tick_interval) + 1
        );
        assert_eq!(miner.last_harvest_cell, None);
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        assert!(entity.teleport_state.is_none());
        assert!(sim.sound_events.iter().all(|event| !matches!(
            event,
            crate::sim::world::SimSoundEvent::ChronoTeleport { .. }
        )));
    }
    assert_eq!(
        sim.production
            .resource_nodes
            .get(&(63, 63))
            .expect("productive source cell after fill")
            .remaining,
        9 * 120
    );

    tick_miners_n(
        &mut sim,
        &rules,
        config.harvest_tick_interval as usize,
    );
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(
            sim.session.binary_frame.wrapping_sub(fill_frame),
            u32::from(config.harvest_tick_interval)
        );
        assert_eq!(miner.state, MinerState::Harvest, "F+18 remains pending");
        assert_eq!(miner.last_harvest_cell, None);
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        assert!(entity.teleport_state.is_none());
        assert!(sim.sound_events.iter().all(|event| !matches!(
            event,
            crate::sim::world::SimSoundEvent::ChronoTeleport { .. }
        )));
    }

    tick_miners_n(&mut sim, &rules, 1);
    let full_gate_frame = sim.session.binary_frame;
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(
            full_gate_frame.wrapping_sub(fill_frame),
            u32::from(config.harvest_tick_interval) + 1
        );
        assert_eq!(miner.cargo.len(), 20);
        assert_eq!(miner.state, MinerState::ReturnToRefinery);
        assert_eq!(miner.harvest_timer.start_frame, full_gate_frame);
        assert_eq!(miner.harvest_timer.duration, 0);
        assert_eq!(
            miner.last_harvest_cell,
            Some((63, 63)),
            "archive is selected from the productive F+19 source cell"
        );
        assert_eq!(miner.reserved_refinery, None);
        assert!(entity.movement_target.is_none());
        assert!(entity.teleport_state.is_none());
        assert!(sim.sound_events.iter().all(|event| !matches!(
            event,
            crate::sim::world::SimSoundEvent::ChronoTeleport { .. }
        )));
    }
    assert_eq!(
        sim.production
            .resource_nodes
            .get(&(63, 63))
            .expect("full gate must not reduce the productive cell")
            .remaining,
        9 * 120
    );

    tick_miners_n(&mut sim, &rules, 1);
    {
        let entity = sim.substrate.entities.get(miner_id).expect("miner entity");
        let miner = entity.miner.as_ref().expect("miner component");
        assert_eq!(miner.reserved_refinery, Some(2));
        assert!(entity.teleport_state.is_some());
    }
    assert_eq!(
        sim.sound_events
            .iter()
            .filter(|event| matches!(
                event,
                crate::sim::world::SimSoundEvent::ChronoTeleport { .. }
            ))
            .count(),
        2
    );
}
```

The War test owns nonzero visual reset and intervening archive-world mutation;
this Chrono test owns productive-cell preservation and the delayed warp/sound
boundary. Together they cover both miner kinds without an unused alternate
grid.

**Step 5: Primary coordinator runs the expected-failing tests**

Before the production patch:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
  Select-Object ProcessName,Id,CPU,StartTime
cargo test filling_extraction_waits_for_full_gate_before_war_return -- --nocapture
cargo test chrono_filling_extraction_does_not_warp_before_state2_tick -- --nocapture
```

Expected before Task 2: both tests fail because current Rust leaves Harvest and
starts return on the filling tick. Record the literal `test result:` lines.

### Task 2: Correct `handle_harvest` branch and dispatch ordering

**Why:** Implement the verified helper/caller sequence without new state or
cross-module changes.

**Files:**

- Modify:
  `src/sim/miner/miner_system.rs:611..706`

**Pattern:** Existing early-return state handlers and existing
`MissionTimer::reset/arm` APIs.

**Step 1: Add the full helper-failure branch after the due gate**

Immediately after the `due` early return and before computing empty capacity or
calling the reducer, add:

```rust
if snap.miner.is_full() {
    // Harvest_Ore_Tick checks full storage before Reduce_Tiberium, resets its
    // timer, and returns failure. Mission_Harvest then writes return state
    // before choosing the ghost/archive cell; state-2 work waits for the next
    // mission dispatch.
    snap.miner.harvest_timer.reset(sim.session.binary_frame);
    snap.miner.state = MinerState::ReturnToRefinery;
    save_archive_via_short_scan(sim, config, path_grid, snap);
    return;
}
```

Do not call `begin_return`. Do not move this branch above the timer-due check.
Do not add a destination check in this feature.

**Step 2: Make every positive extraction remain Harvest**

Delete the current newly-full block:

```rust
if snap.miner.is_full() {
    save_archive_via_short_scan(sim, config, path_grid, snap);
    begin_return(sim, rules, config, path_grid, snap);
    return;
}
```

Keep one unconditional positive-success tail:

```rust
// A positive extraction is success even when it fills storage. Native
// Mission_Harvest remains in state 1 and observes fullness only at the next
// helper gate.
snap.miner.harvest_timer.arm(
    sim.session.binary_frame,
    u32::from(config.harvest_tick_interval) + 1,
);
return;
```

The `rules` argument remains used by the reducer call, so no signature change is
needed.

**Step 3: Remove the now-unreachable full subcase from the no-removal comment
and branch**

After the new precheck, `snap.miner.is_full()` cannot become true without a
positive extraction, which already returned. Remove the later block:

```rust
if snap.miner.is_full() {
    save_archive_via_short_scan(sim, config, path_grid, snap);
    begin_return(sim, rules, config, path_grid, snap);
    return;
}
```

Rewrite the no-bale comment to cover only:

```rust
// No bales extracted while not full. Run the caller-owned short continuation
// scan; a hit moves toward the next patch, while a miss begins the existing
// no-resource return path.
```

Do not change the existing continuation-hit or scan-miss behavior. Its
`begin_return` call is outside the scoped exact-full branch.

**Step 4: Primary coordinator reruns the focused tests**

```powershell
cargo test harvester_caps_extraction_at_remaining_capacity -- --nocapture
cargo test filling_extraction_waits_for_full_gate_before_war_return -- --nocapture
cargo test chrono_filling_extraction_does_not_warp_before_state2_tick -- --nocapture
```

Expected: each command exits zero and prints a literal passing
`test result:` line.

### Task 3: Correct owned cadence comments

**Why:** Prevent the contradicted `F+18` claim from inviting removal of the
required observation fencepost.

**Files:**

- Modify comments only:
  `src/sim/miner/mod.rs:169..207,300..305`
- Modify comments only:
  `src/sim/miner/miner_system.rs` at arrival and positive re-arm

**Pattern:** Existing “why” comments cite behavior, not binary addresses.

**Step 1: Clarify `MinerConfig::harvest_tick_interval`**

Use wording equivalent to:

```rust
/// Frame span for the nine native harvest StepTimer expiries:
/// `9 * HarvesterLoadRate`. Mission dispatch observes the ninth post-mission
/// increment on the following frame, so harvest gates arm for this value + 1.
pub harvest_tick_interval: u8,
```

Update the stock default comment to:

```rust
// HarvesterLoadRate=2 and nine expiries produce the 18-frame threshold.
// Mission dispatch runs before timer maintenance, so it observes step 9 on
// frame 19; call sites preserve that with harvest_tick_interval + 1.
harvest_tick_interval: 18,
```

**Step 2: Replace the obsolete countdown-migration rationale**

Update the `Miner::harvest_timer` comment to explain the verified
mission-before-timer observation:

```rust
/// Frame-anchored gate for the next harvest helper call. Call sites arm for
/// `harvest_tick_interval + 1`: the ninth native StepTimer expiry occurs after
/// the frame-18 mission call, so the mission first observes it on frame 19.
pub harvest_timer: MissionTimer,
```

**Step 3: Correct `miner_system.rs` comments without changing behavior**

- At arrival, do not claim the first physical-arrival delay is exactly 18; say
  this legacy Rust anchor is a separately tracked acquisition-timing drift and
  retain `+1`.
- At positive success, say the next helper observation is
  `9 * HarvesterLoadRate + 1` frame numbers under the verified order.
- Do not implement the sibling acquisition/destination fix in comments or code.

### Task 4: Format, validate, inspect, and commit the feature milestone

**Why:** Prove the real production loop, preserve owned scope, and leave a
crash-safe coherent commit for guarded integration.

**Files:**

- Owned:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/miner/mod.rs`

**Step 1: Format only owned Rust files**

From the feature worktree:

```powershell
rustfmt --edition 2024 src/sim/miner/miner_system.rs
rustfmt --edition 2024 src/sim/miner/miner_tests.rs
rustfmt --edition 2024 src/sim/miner/mod.rs
```

Inspect `git diff --stat` and `git diff --check`. If rustfmt touched unrelated
lines outside the local edited regions, restore only those formatter changes
with a targeted patch; never use a destructive checkout/reset.

**Step 2: Run the focused miner slice serially**

Primary coordinator first checks Cargo ownership, then runs one command at a
time:

```powershell
cargo test filling_extraction -- --nocapture
cargo test harvester_caps_extraction_at_remaining_capacity -- --nocapture
cargo test harvester_continues_to_short_scan_when_partial_then_empty -- --nocapture
cargo test chrono_miner_teleports_to_refinery_on_return -- --nocapture
cargo test war_miner_does_not_teleport -- --nocapture
```

Expected: every command exits zero with literal passing `test result:` lines.
The continuation test guards the intentionally untouched no-bale/not-full path;
the existing return tests guard the unchanged state-2 implementation.

**Step 3: Run the full miner module test target**

The test module is declared as `sim::miner::miner_tests`; run its exact path
filter:

```powershell
cargo test 'sim::miner::miner_tests::' -- --nocapture
```

Expected: exit zero and literal passing `test result:` line. Do not run Cargo
in parallel.

**Step 4: Run final compile validation**

```powershell
cargo check -q
```

Expected: exit zero. Report the literal command result; do not infer success
from silence alone.

**Step 5: Audit owned scope**

```powershell
git status --short
git diff --name-only
git diff --check
git diff -- src/sim/miner/miner_system.rs
git diff -- src/sim/miner/miner_tests.rs
git diff -- src/sim/miner/mod.rs
```

Expected `git diff --name-only`:

```text
src/sim/miner/miner_system.rs
src/sim/miner/miner_tests.rs
src/sim/miner/mod.rs
```

Reject the milestone if any other path appears.

**Step 6: Commit the reviewed milestone**

The user explicitly authorized coherent feature commits. Stage exact literal
paths only:

```powershell
git add -- src/sim/miner/miner_system.rs src/sim/miner/miner_tests.rs src/sim/miner/mod.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "miner: defer full return to the next harvest gate"
```

Expected cached names are exactly the three owned files. Record the commit SHA
and verify the feature worktree is clean.

### Task 5: Guarded no-commit integration into `dev`

**Why:** Integrate only if the current committed `dev` plus the feature commit
passes together, without overwriting concurrent work or hiding conflicts in an
automatic merge commit.

**Owner:** Primary coordinator only, in the main checkout.

**Step 1: Reconcile main checkout and feature metadata**

```powershell
git status --porcelain=v2 --branch
git rev-parse refs/heads/dev
git rev-parse feature/gsi-07-15-harvest-fill-gate-20260724-1525
git worktree list --porcelain
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
  Select-Object ProcessName,Id,CPU,StartTime
```

Stop with `MERGE_DEFERRED_DIRTY_DEV` if tracked `dev` is dirty. If `dev` moved
and any owned path changed since the feature base, review the combined diff
before merging.

**Step 2: Start the guarded no-commit merge**

```powershell
git merge --no-ff --no-commit feature/gsi-07-15-harvest-fill-gate-20260724-1525
```

If conflicts occur, abort the merge with `git merge --abort`, record the exact
paths, and do not resolve by discarding either side.

**Step 3: Validate combined state serially**

Run the same focused production tests from Task 4, then:

```powershell
cargo check -q
git diff --cached --check
```

Inspect staged names; they must match the three owned feature paths plus only
Git's expected merge metadata.

**Step 4: Complete the merge commit only after validation**

```powershell
git commit -m "Merge GSI-07.15 harvest full-gate parity"
```

Record the merge SHA. Do not push.

**Step 5: Post-merge state**

- Confirm `dev` is clean.
- Preserve the feature branch/worktree until the operational journal records
  the merge and parent-stack unwind.
- Update the crash-safe operator journal with base, feature SHA, merge SHA,
  test result lines, and next dependency-stack owner.
- Resume the suspended GSI-04.09 parent and retest the complete
  filling/full-gate/exact-reduction loop after its own isolated feature.

## Sources and References

- **Contract:**
  `docs/contracts/2026-07-24-gsi-07-15-harvest-filling-return-gate-implementation-contract.md`
- **Design:**
  `docs/plans/2026-07-24-gsi-07-15-harvest-filling-return-gate-design.md`
- **Approval:**
  `docs/approvals/2026-07-24-gsi-07-15-harvest-filling-return-gate-design-approval.md`
- **Primary research:**
  - `docs/research/HARVEST_ORE_TICK_TIMING_PARTIAL_FULL_EDGE_CASES_ORE_GEMS_GHIDRA_REPORT.md`
  - `docs/research/miner/DOCK_RATE_TIMER_FRAME_COUNTER_ORDERING_GHIDRA_REPORT.md`
  - `docs/research/miner/MISSION_HARVEST_GHIDRA_REPORT.md`
  - `docs/research/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
- **Live binary anchors:**
  - `Harvest_Ore_Tick @ 0x0073D450`
  - `Mission_Harvest @ 0x0073E5E0`
  - `TechnoClass::AI_Update @ 0x006F9E50`
  - `MissionClass::Mission_Dispatch @ 0x005B3060`
  - `Main_Tick @ 0x0055D360`
- **INI:**
  - base/patch `[General] HarvesterLoadRate` lookup; stock files omit an
    override, native/Rust default `2`
  - stock `[HARV]` and `[CMIN]` activation in `ini/rulesmd.ini`
- **Current Rust:**
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/miner/mod.rs`
  - `src/sim/mission/timer.rs`
  - `src/sim/production/production_economy.rs`
  - `src/sim/world/world_hash.rs`
- **Base reviewed:** `dev`
  `68302b5d2d0b558400e2e0cf9b51c6994fa180c7`

## Post-Plan Self-Review

- [x] Every approved design requirement maps to a task.
- [x] No `TBD`, implementation `TODO`, vague “add tests,” or hidden choice.
- [x] Existing architecture/state owners are preserved.
- [x] No public interface task is required.
- [x] All high-risk branch/timing/visual/return boundaries have production
  regression tests.
- [x] Each task names exact paths, code shape, and expected result.
- [x] Sim layering, deterministic order, hash, RNG, and fixed-math constraints
  are explicit.
- [x] Research docs, live binary, current Rust, and INI activation are cited.
- [x] Every technical decision has confidence and source.
- [x] Deferred sibling gaps are explicit and cannot be mistaken for parity
  certification.
- [x] Parity-critical timing, state order, negative side effects, visuals, and
  next-tick action are enumerated before tasks.
- [x] Feature/worktree/commit/integration steps match the operator's guarded Git
  workflow and never push.
