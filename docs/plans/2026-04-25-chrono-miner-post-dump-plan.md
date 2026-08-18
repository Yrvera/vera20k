# Chrono Miner Post-Dump Return Path — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Fix the chrono miner head-butt oscillation by clearing stale ore targets at undock and gating ExitPad on full locomotion completion.

**Architecture:** Two surgical changes in [`phase_exit_pad`](../../src/sim/miner/miner_dock_sequence.rs#L422), no new state, no new INI keys, no new abstractions. Plus three new unit tests.

**Design Doc:** [docs/plans/2026-04-25-chrono-miner-post-dump-design.md](2026-04-25-chrono-miner-post-dump-design.md)

---

## Grounding Summary

- **Docs say:** post-dump path goes through `BuildingClass::UndockUnit` (0x4593A0) → `Mission_Guard_Harvester` (0x740810) → `Mission_Harvest` state 0. Verified via `HARVESTER_DOCK_UNLOAD_SEQUENCE.md`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`, `MINER_DOCK_GAPS_RESEARCH.md`.
- **Ghidra confirmed:** `RulesClass+0x1790` is `SlaveMinerKickFrameDelay` (NOT "HarvestInterval" as one doc claimed). Mission_Guard_Harvester for normal (non-slave) harvesters has no timer pause — it goes straight to `SetMission(HARVEST)`. So no `HarvestInterval` Guard pause to port.
- **Repo pattern this mirrors:** field-clear-on-state-transition pattern already used in [phase_exit_pad lines 446-453](../../src/sim/miner/miner_dock_sequence.rs#L446) for `reserved_refinery`, `dock_queued`, `forced_return`. We add two more fields to that same clear block.
- **INI keys driving behavior:** none added. `HarvestInterval` is not a real INI key in `rules(md).ini`.
- **Still unknown:** none. The design is fully grounded.

## Key Technical Decisions

- **Don't add a Guard state with `HarvestInterval` pause.** Verified `RulesClass+0x1790` is `SlaveMinerKickFrameDelay`, not "HarvestInterval"; no equivalent pause exists in gamemd for normal harvesters. — **Confidence:** high — **Source:** Ghidra `Mission_Guard_Harvester` decompile + 3 cross-doc references in `ra2-rust-game-docs/`
- **Don't replace `issue_direct_move` with pathfinding for near targets.** Existing usage is load-bearing per [comment at miner_system.rs:340-343](../../src/sim/miner/miner_system.rs#L340) — A* refuses to path onto blocked Tiberium cells. — **Confidence:** high — **Source:** repo code + comment
- **Clear both `target_ore_cell` and `last_harvest_cell` on ExitPad → SearchOre.** Both are read by [handle_search_ore](../../src/sim/miner/miner_system.rs#L209); clearing only one would still bias the next scan toward the back of the refinery. — **Confidence:** high — **Source:** repo code reading
- **Add `teleport_state.is_none()` to the settled gate.** Mirrors the existing teleport check in [handle_move_to_ore lines 301-307](../../src/sim/miner/miner_system.rs#L301). — **Confidence:** high — **Source:** repo pattern

## Open Questions

### Resolved During Planning

- "Does gamemd have a `HarvestInterval` post-undock pause?" — **No**, verified `RulesClass+0x1790` is `SlaveMinerKickFrameDelay`, the timer only triggers for slave miners (gated on `param_1[0xb6] != 0` = SlaveManager pointer).
- "Should we replace `issue_direct_move` with pathfinding-aware?" — **No**, the load-bearing comment in miner_system.rs:340-343 confirms direct move is required because A* can't path onto Tiberium cells.
- "Apply fix to all miner kinds or chrono only?" — **All kinds.** Both go through `phase_exit_pad`; gamemd's behavior is uniform. War miner not currently bugged but skipping it would create unprincipled divergence.

### Deferred to Implementation

- None. All decisions are settled.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) | Update `phase_exit_pad` gate + cleanup |
| Modify | [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) | Add 3 unit tests |

## Interface Changes

None. `phase_exit_pad` signature unchanged. No new pub fns, structs, or INI keys.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 in game logic (no math added; only field clears + bool gate)
- [x] New state included in deterministic state hash — no new state added; only writes to existing fields
- [x] No dependencies on render/ui/sidebar/audio/net — only touches `sim/miner` and reads `entity.teleport_state` (already in `sim`)
- [x] Tick ordering impact noted — none; behavior changes within an existing handler called in the existing phase order
- [x] BTreeMap iteration order considered — no iteration change

## Risk Areas

- **Existing test breakage:** any test that asserted `target_ore_cell == Some(...)` immediately after a dock cycle is now wrong. A grep of [miner_tests.rs](../../src/sim/miner/miner_tests.rs) shows the existing post-dock assertions use loose patterns like `state == SearchOre || state == WaitNoOre` (lines 1037, 1162) which will still pass. No tight assertions on `target_ore_cell` post-dock found, but Task 2 explicitly verifies this with a full test run.
- **Regression in close-ore scenarios:** if the previous patch is the right answer (still rich, closest to refinery), clearing the archive forces an extra scan from the exit cell. The re-scan still finds the same patch (it's the closest). One extra scan per dock cycle, no behavioral regression.

### Known limitation (from /review-plan)

**This fix is a hypothesis test, not a guaranteed bug fix.** `search_local_ore` picks targets by geometric (Euclidean) distance — it does not check pathfinding reachability. If the user's actual scenario has the back-side ore patch as the geometrically closest cell, clearing the archive will not change which patch gets re-targeted, and the head-butt symptom may recur after this fix.

If Task 8 (manual in-game verification) shows the symptom persists:
- This is **expected** under the limitation above.
- Do **NOT** patch the symptom by tightening tests or adding more clears.
- The follow-up is a separate brainstorm covering one of:
  - Stuck detection in `MoveToOre` (track no-progress ticks → bail to SearchOre with the failed target blacklisted)
  - Pathfinding-aware filter in `search_local_ore` (use the path grid to skip unreachable candidates)
  - Re-search-on-blocked-direct-move in `MoveToOre`'s near-target branch

If Task 8 shows the symptom is fixed: ship it. The fix was sufficient for the user's specific scenario.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Cleared archive on undock | Player-observable: chrono miner no longer drives into back of refinery after dumping | Integration test in Task 5 + manual in-game observation in Task 8 |
| Task 1 | Teleport-state gate on ExitPad | Ensures chrono miner doesn't transition out of dock mid-warp; prevents subtle visual artifacts | Unit test in Task 4 |

---

## Tasks

### Task 1: Update `phase_exit_pad` with the two changes

**Why:** Single production code change. Clears stale ore-target archive and tightens the settled gate to include teleport state.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs:422-461](../../src/sim/miner/miner_dock_sequence.rs#L422)

**Pattern:** Field-clear-on-state-transition; mirrors existing clears for `reserved_refinery`, `dock_queued`, `forced_return` in the same block.

**Step 1: Locate the exact block**

In [src/sim/miner/miner_dock_sequence.rs:422-461](../../src/sim/miner/miner_dock_sequence.rs#L422), find the function `phase_exit_pad`. The relevant gate currently reads:

```rust
let moving = sim
    .entities
    .get(snap.entity_id)
    .is_some_and(|e| e.movement_target.is_some());
let at_exit = (snap.rx, snap.ry) == exit;

if !moving && !at_exit {
    // Issue the exit move and set facing to match original engine.
    movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.facing_target = Some(EXIT_FACING);
    }
    return;
}

if !moving && at_exit {
    // Arrived at exit — finish docking.
    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.forced_return = false;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.state = MinerState::SearchOre;
    return;
}
```

**Step 2: Add the teleport gate variable**

Right after `let at_exit = ...`, add:

```rust
let teleporting = sim
    .entities
    .get(snap.entity_id)
    .is_some_and(|e| e.teleport_state.is_some());
```

**Step 3: Tighten the arrival condition and clear the ore-target archive**

Replace the `if !moving && at_exit { ... }` block with:

```rust
if !moving && at_exit && !teleporting {
    // Arrived at exit — finish docking.
    snap.miner.reserved_refinery = None;
    snap.miner.dock_queued = false;
    snap.miner.forced_return = false;
    // Clear stale ore targets so SearchOre re-scans from the exit cell.
    // Without this, the miner re-targets the patch it came from, which
    // for refineries placed adjacent to ore puts the destination on the
    // back side of the building footprint, producing a head-butt cycle.
    snap.miner.target_ore_cell = None;
    snap.miner.last_harvest_cell = None;
    snap.miner.dock_phase = RefineryDockPhase::Approach;
    snap.miner.state = MinerState::SearchOre;
    return;
}
```

**Step 4: Verify compile**

Run: `cargo check -p ra2-rust-game --lib`
Expected: clean compile, no errors, no warnings introduced by the change.

### Task 2: Run existing miner tests, identify any breakage

**Why:** Surface any test that encoded the buggy behavior before adding new ones, so we know exactly what (if anything) needs updating.

**Files:** none modified yet — read-only verification step.

**Step 1: Run the miner test suite**

Run: `cargo test -p ra2-rust-game --lib miner -- --nocapture`
Expected: PASS for all existing tests. The grep in Risk Areas suggests no tight assertions on `target_ore_cell` post-dock exist, so all should still pass.

**Step 2: If any tests fail**

For each failing test:
1. Read the test body to determine if it encoded the buggy behavior (asserted that `target_ore_cell` is `Some(...)` after dock, or that `last_harvest_cell` survives a dock cycle).
2. If yes (encoded the bug): update the assertion to match the new contract — `target_ore_cell` and `last_harvest_cell` are `None` after ExitPad → SearchOre.
3. If no (test failure unrelated to the change): stop and reassess. Don't proceed with new tests until this is understood.

**Step 3: Commit if updates were needed**

If any tests were updated:
```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: update assertions for cleared ore-target archive on undock"
```

If no updates needed, skip the commit and proceed to Task 3.

### Task 3: Add unit test — `exit_pad_clears_ore_targets_on_arrival`

**Why:** Direct unit-level coverage of the field-clear behavior. Verifies both `target_ore_cell` and `last_harvest_cell` are cleared at the ExitPad → SearchOre transition.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) — add new test at end of file

**Pattern:** Mirrors existing miner-tests structure (rules setup → spawn entities → tick → assert). Use `MinerKind::Chrono` since chrono is the kind exhibiting the bug.

**Step 1: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) (end of file, after the last `#[test]`):

```rust
/// After ExitPad arrival, both `target_ore_cell` and `last_harvest_cell` must
/// be cleared so SearchOre re-scans from the exit cell instead of biasing
/// toward the previous patch (which may sit on the back side of the refinery).
#[test]
fn exit_pad_clears_ore_targets_on_arrival() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // Refinery at (10, 10). Exit cell for a 4x3 foundation = (11, 11).
    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);

    // Set up the miner mid-ExitPad with stale archive populated.
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::ExitPad;
    miner.reserved_refinery = Some(100);
    miner.dock_queued = false;
    miner.target_ore_cell = Some((20, 20));      // pre-dock target
    miner.last_harvest_cell = Some((20, 20));    // pre-dock archive

    // Tick the miner system — should detect arrival and run the cleanup.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::SearchOre, "must transition to SearchOre");
    assert!(miner.target_ore_cell.is_none(), "target_ore_cell must be cleared");
    assert!(miner.last_harvest_cell.is_none(), "last_harvest_cell must be cleared");
    assert!(miner.reserved_refinery.is_none(), "reserved_refinery must be cleared");
}
```

**Step 2: Verify the helper functions exist**

The test uses `spawn_refinery`. Grep [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) for `fn spawn_refinery`. If it doesn't exist with that exact signature, find the closest existing spawn helper for refineries and use that name instead. Update the test accordingly.

**Step 3: Run the test**

Run: `cargo test -p ra2-rust-game --lib exit_pad_clears_ore_targets_on_arrival -- --nocapture`
Expected: PASS.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add exit_pad_clears_ore_targets_on_arrival"
```

### Task 4: Add unit test — `exit_pad_blocks_transition_during_teleport`

**Why:** Verifies the new `teleport_state.is_none()` gate condition. Without this gate, a chrono miner mid-warp could transition out of ExitPad prematurely.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) — add new test

**Pattern:** Same structure as Task 3.

**Step 1: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs):

```rust
/// ExitPad must NOT transition to SearchOre while a teleport is in progress
/// (`entity.teleport_state.is_some()`). Without this gate a chrono miner
/// mid-warp could leave the dock sub-state machine prematurely.
#[test]
fn exit_pad_blocks_transition_during_teleport() {
    use crate::sim::movement::teleport_movement::{TeleportPhase, TeleportState};

    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    spawn_refinery(&mut sim, 100, 10, 10);
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);

    // Set up miner at the exit cell, in ExitPad, with a teleport in progress.
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::ExitPad;
    miner.reserved_refinery = Some(100);
    miner.target_ore_cell = Some((20, 20));
    // Inject an active teleport state to trip the gate.
    entity.teleport_state = Some(TeleportState {
        phase: TeleportPhase::ChronoDelay,
        target_rx: 20,
        target_ry: 20,
        being_warped_ticks: 16,
    });

    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");
    assert_eq!(miner.state, MinerState::Dock, "must stay in Dock state during teleport");
    assert_eq!(miner.dock_phase, RefineryDockPhase::ExitPad, "must stay in ExitPad");
    assert_eq!(
        miner.target_ore_cell,
        Some((20, 20)),
        "ore target must NOT be cleared while teleport is active"
    );
}
```

**Step 2: Verify `TeleportState` field names**

The test constructs `TeleportState { target, being_warped_ticks }`. If the actual struct uses different field names or has additional required fields, adjust the literal accordingly. Check [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs) for the canonical struct definition.

**Step 3: Run the test**

Run: `cargo test -p ra2-rust-game --lib exit_pad_blocks_transition_during_teleport -- --nocapture`
Expected: PASS.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add exit_pad_blocks_transition_during_teleport"
```

### Task 5: Add smoke test — `chrono_miner_archive_cleared_after_undock_picks_new_target`

**Why:** Smoke test that the post-fix flow runs end-to-end: miner mid-ExitPad with a stale archive successfully transitions to SearchOre with the archive cleared, then picks a fresh target via local scan from its current position. This is **not** a regression test for the headbutt symptom itself — that requires in-game observation (Task 8), because `search_local_ore` picks by geometric distance and can't tell whether a target is reachable around obstacles. See the design doc's "Out of scope" section for why a stronger integration test isn't possible without redesigning the search.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) — add new test

**Pattern:** Higher-level than Tasks 3/4 — runs multiple ticks through the state machine. Mirrors the existing chrono-miner end-to-end tests around lines 321-465.

**Step 1: Add the test function**

Append to [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs):

```rust
/// Smoke test for the post-undock flow with a stale archive.
///
/// Sets up a chrono miner mid-ExitPad with `last_harvest_cell` pointing to a
/// patch outside the local scan radius. After the fix, the archive is cleared
/// at exit and SearchOre runs with the miner's current position as search
/// center, picking the only ore patch in range. Verifies the field-clear
/// behavior end-to-end (state transitions ExitPad → SearchOre → MoveToOre,
/// archive is cleared, fresh target is picked from current position).
///
/// NOTE: this does NOT verify the headbutt symptom is fixed. The fix clears
/// the archive but the search algorithm still picks by geometric distance
/// (no pathfinding-aware reachability). If a back-side ore patch is the
/// closest in the user's scenario, the headbutt may recur. That hypothesis
/// must be tested in-game (see Task 8).
#[test]
fn chrono_miner_archive_cleared_after_undock_picks_new_target() {
    let mut sim = Simulation::new();
    let rules = miner_rules();
    let config = MinerConfig::default();
    let path_grid = PathGrid::new(64, 64);

    // Refinery at (10, 10), 4x3 foundation. Exit cell = (11, 11).
    spawn_refinery(&mut sim, 100, 10, 10);

    // Place ONE ore patch at (15, 11): distance 4 from exit, within
    // local_continuation_radius (default 6). This is what the fresh local
    // scan from current position should pick.
    sim.production.resource_nodes.insert(
        (15, 11),
        ResourceNode { resource_type: ResourceType::Ore, remaining: 1200 },
    );

    // Spawn miner at exit cell (11, 11), mid-ExitPad. Stale archive points
    // far away (50, 50) — outside any scan radius from current position,
    // and no ore at that cell. If the archive were NOT cleared, the search
    // would start from (50, 50), the local scan would find nothing, the
    // archive check would also find nothing, and only the long scan would
    // eventually fall back to current position. With the fix the local scan
    // from current position immediately picks (15, 11).
    let miner_id = spawn_miner(&mut sim, 1, MinerKind::Chrono, 11, 11);
    let entity = sim.entities.get_mut(miner_id).expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::Dock;
    miner.dock_phase = RefineryDockPhase::ExitPad;
    miner.reserved_refinery = Some(100);
    miner.target_ore_cell = Some((50, 50));
    miner.last_harvest_cell = Some((50, 50));
    miner.cargo.clear();

    // Tick twice: (1) ExitPad → SearchOre with cleared archive,
    // (2) SearchOre → MoveToOre with target picked.
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));
    crate::sim::miner::miner_system::tick_miners(&mut sim, &rules, &config, Some(&path_grid));

    let entity = sim.entities.get(miner_id).expect("miner entity");
    let miner = entity.miner.as_ref().expect("miner component");

    // The stale (50, 50) target must be replaced. Any other value (None or
    // Some((15, 11))) is acceptable — the precise target depends on which
    // tick SearchOre ran in. The key property: the stale archive does not
    // survive the dock cycle.
    assert_ne!(
        miner.target_ore_cell,
        Some((50, 50)),
        "stale archive must be replaced after ExitPad → SearchOre. \
         Got state={:?}, target={:?}",
        miner.state,
        miner.target_ore_cell,
    );

    // After the second tick, the only available ore should be the picked target.
    if let Some(target) = miner.target_ore_cell {
        assert_eq!(
            target,
            (15, 11),
            "the only ore at (15, 11) should be picked. Got {:?}",
            target
        );
    }
}
```

**Step 2: Verify imports and helpers**

Confirm `ResourceNode { resource_type, remaining }` matches the actual struct in [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs). Adjust if field names differ.

Confirm `sim.production.resource_nodes` is the correct path to the BTreeMap. If different, update.

**Step 3: Run the test**

Run: `cargo test -p ra2-rust-game --lib chrono_miner_archive_cleared_after_undock_picks_new_target -- --nocapture`
Expected: PASS.

If the test fails because `target_ore_cell` is still `Some((50, 50))` after two ticks, that means the fix isn't actually clearing the archive — something is wrong with Task 1. Re-verify the changes in `phase_exit_pad`.

**Step 4: Commit**

```
git add src/sim/miner/miner_tests.rs
git commit -m "miner_tests: add archive-cleared smoke test for chrono undock"
```

### Task 6: Run full test suite, verify clean

**Why:** Catch any cross-test interaction or downstream breakage.

**Files:** none modified.

**Step 1: Run all sim tests**

Run: `cargo test -p ra2-rust-game --lib`
Expected: PASS, 0 failures, 0 ignored newly.

**Step 2: Run clippy on the changed file**

Run: `cargo clippy -p ra2-rust-game --lib -- -D warnings`
Expected: clean.

**Step 3: If anything fails**

Investigate the failure. If it's a downstream interaction not anticipated by the design doc, stop and reassess — do not patch tests reflexively. The fix may have surfaced a real second-order issue.

### Task 7: Commit the production code change

**Why:** Production code change deserves its own focused commit (separate from test additions, which were committed individually in Tasks 3-5).

**Files:** [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs)

**Step 1: Check status**

Run: `git status`
Expected: only `src/sim/miner/miner_dock_sequence.rs` modified (the production change from Task 1).

**Step 2: Commit**

```
git add src/sim/miner/miner_dock_sequence.rs
git commit -m "miner: clear stale ore targets and add teleport gate on undock

Fixes chrono miner oscillation where, after dumping ore at a refinery
adjacent to the previous harvest patch, the miner would re-target the
back side of the refinery footprint and head-butt the wall.

phase_exit_pad now (a) clears target_ore_cell and last_harvest_cell on
ExitPad arrival so SearchOre re-scans from the exit position, and (b)
gates the arrival transition on teleport_state.is_none() to prevent
leaving the dock sub-state machine mid-warp.

No new state, no new INI keys; matches the existing field-clear-on-
state-transition pattern in the same block.

Design doc: docs/plans/2026-04-25-chrono-miner-post-dump-design.md"
```

### Task 8: Manual in-game verification

**Why:** Per CLAUDE.md "verify the end-to-end result of every change, not just the mechanical task." Unit tests verify code correctness; only running the game verifies feature correctness.

**Files:** none modified.

**Step 1: Build and run the game**

Run: `cargo run -p ra2-rust-game --release`

**Step 2: Set up the bug scenario**

Load a map with an Allied refinery placed adjacent to ore on its back side. The fastest way is a skirmish on a small map — manually build a refinery near a back-of-base ore patch, or use a save file if one exists.

**Step 3: Observe the chrono miner cycle**

Wait for the chrono miner to:
1. Teleport to the back-side ore patch
2. Harvest a load
3. Return (drive or teleport) to the refinery
4. Dock and unload
5. Exit the dock pad

**Expected:** After step 5, the chrono miner picks an ore cell that does not require going through the refinery footprint — no headbutting against the wall. The miner should harvest the next load without visible oscillation.

**If the symptom persists:** the fix didn't address the root cause. Stop and re-investigate; do not patch over the symptom with additional changes.

**Step 4: Document the verification**

Add a one-liner to the design doc's "Definition of Done" section confirming the manual verification was done (date + brief outcome). This is the only doc edit done outside the implementation tasks.

---

## Sources & References

- **Design doc:** [docs/plans/2026-04-25-chrono-miner-post-dump-design.md](2026-04-25-chrono-miner-post-dump-design.md)
- **Ghidra reports:**
  - `HARVESTER_DOCK_UNLOAD_SEQUENCE.md` (audit YELLOW; see design doc for the doc errors uncovered)
  - `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
  - `MINER_DOCK_GAPS_RESEARCH.md` (audit GREEN)
  - `MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md`
- **gamemd.exe addresses verified this session:**
  - `BuildingClass::UndockUnit` @ 0x4593A0
  - `Mission_Guard_Harvester` @ 0x740810 (gates `RulesClass+0x1790` on `param_1[0xb6] != 0` = SlaveManager — confirms field is SlaveMinerKickFrameDelay, not HarvestInterval)
  - `Mission_Harvest` @ 0x73E5E0
  - `DriveLocomotionClass::Is_Ok_To_End` @ 0x4AF970
  - `FootClass::AI` @ 0x4DA530 (piggyback swap-back)
- **INI keys:** none new. `HarvestInterval` confirmed not present in `rules(md).ini` and not in binary string table.
- **Related Rust code:**
  - [src/sim/miner/miner_dock_sequence.rs:422-461](../../src/sim/miner/miner_dock_sequence.rs#L422) — `phase_exit_pad` (the file we change)
  - [src/sim/miner/miner_system.rs:209-380](../../src/sim/miner/miner_system.rs#L209) — `handle_search_ore` and `handle_move_to_ore` (downstream consumers)
  - [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs) — `TeleportState` struct used in Task 4 test
