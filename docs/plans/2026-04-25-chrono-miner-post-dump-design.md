# Chrono Miner Post-Dump Return Path — Design

## Goal

Fix the chrono miner oscillation bug where, after dumping ore at the refinery, the miner drives into the back corner of the refinery and head-butts the wall back-and-forth, by clearing stale post-dock target state and gating ExitPad on full locomotion completion.

## Architecture Context

The miner state machine lives in three files:

- [src/sim/miner/mod.rs](../../src/sim/miner/mod.rs) — `Miner` struct, `MinerState` enum, `RefineryDockPhase` sub-state
- [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) — top-level state handlers (`handle_search_ore`, `handle_move_to_ore`, etc.)
- [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) — refinery dock choreography (`phase_approach`, `phase_exit_pad`, etc.)

**Post-dump flow today:**

```
RefineryDockPhase::Unloading (drains cargo, awards credits)
  → RefineryDockPhase::ExitPad
      ↓ phase_exit_pad: issues direct move to exit cell
      ↓ on (!moving && at_exit):
          - clears reserved_refinery, dock_queued, forced_return
          - sets state = MinerState::SearchOre
  → MinerState::SearchOre
      ↓ handle_search_ore:
          search_center = last_harvest_cell.unwrap_or(current_pos)
          short scan → archive (last_harvest_cell) → long scan → global
          sets target_ore_cell, transitions to MoveToOre
  → MinerState::MoveToOre
      ↓ handle_move_to_ore:
          if teleporting: wait
          else if arrived: → Harvest
          else if dx<=1 && dy<=1: issue_direct_move (workaround for blocked ore cells)
          else: A* pathfinding
```

**The existing `issue_direct_move` for near targets is load-bearing.** A comment at [miner_system.rs:340-343](../../src/sim/miner/miner_system.rs#L340) explains: the passability matrix blocks Tiberium cells for Track-type units, so A* refuses to path onto an ore cell. The final step into the ore cell must bypass A*. This is not a bug — it's a deliberate workaround for a constraint of the path grid.

**The verified gamemd.exe equivalent** (from `MINER_DOCK_GAPS_RESEARCH.md`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`, and our own Ghidra verification this session):
1. `BuildingClass::UndockUnit` issues `Head_To` (drive away)
2. `DriveLocomotionClass` (piggybacked on `TeleportLocomotion` for chrono) drives the unit out, with full pathfinding
3. `FootClass::AI` per-tick polls `IPiggyback::Is_Ok_To_End` — gates on `Is_Moving_Now == false`
4. When stopped: `End_Piggyback` swaps active locomotor back to `TeleportLocomotion`
5. `FootClass::Locomotion_AI` reassigns mission based on cargo level + Teleporter flag
6. `Mission_Guard_Harvester` (for harvester units) immediately transitions to `SetMission(HARVEST)` — **no `HarvestInterval` pause exists for normal harvesters**. The `RulesClass+0x1790` field documented as "HarvestInterval" in one doc is actually `SlaveMinerKickFrameDelay` (verified across three other docs and the Mission_Guard_Harvester decompile, which gates the timer on `param_1[0xb6] != 0` = SlaveManager pointer).

So the gamemd "pause" the player perceives is purely the time taken for the drive-out move + locomotion swap. There is no explicit timer.

## Impact Analysis

**What changes:**

- [phase_exit_pad](../../src/sim/miner/miner_dock_sequence.rs) at miner_dock_sequence.rs:422-461 — adds two field clears and one new gate condition.

**What this depends on:**

- `entity.movement_target.is_some()` semantics (already used as the existing settled-gate)
- `entity.teleport_state.is_some()` semantics (already used in `handle_move_to_ore`)
- `Miner::target_ore_cell` and `Miner::last_harvest_cell` fields (already exist, currently never cleared on undock)

**Blast radius:**

- Sim only. No render/UI/audio touch.
- All three miner kinds (War, Chrono, Slave) go through `phase_exit_pad`. The change applies to all by design — gamemd's behavior is uniform across harvester kinds.
- Determinism: no new state, no new timers, no new RNG calls. Field clears are deterministic.
- Snapshot serialization: no change (touching existing fields).

**Risks:**

- **Test breakage:** any existing test that asserts `target_ore_cell == Some(...)` or `last_harvest_cell == Some(...)` after a dock cycle will need updating. The previous behavior (sticky archive) was the bug.
- **Regression in scenarios where the previous patch is the right answer:** if the miner just dumped a load from a still-rich patch right next to the refinery, clearing the archive forces a re-scan from the exit cell. The re-scan should still find the same patch (it's the closest ore), so no behavioral regression — just one extra scan.

**Not in scope:**

- Adding a `HarvestInterval` Guard pause (gamemd doesn't have one for normal harvesters; verified)
- Replacing `issue_direct_move` with pathfinding-aware move (would break the blocked-ore-cell workaround)
- Implementing the `IPiggyback::Is_Ok_To_End` swap-back chain (Rust's `teleport_state` model already implies this; the gate condition added below covers the observable behavior)
- Implementing `Locomotion_AI`-equivalent ore-level mission gate (G4 in the gap-scan; redundant with the cleared archive)

## Chosen Approach

Two surgical changes in `phase_exit_pad`:

1. **Clear `target_ore_cell` and `last_harvest_cell`** when transitioning ExitPad → SearchOre. Forces SearchOre to re-evaluate from the miner's current position (the exit cell, away from the refinery) instead of biasing toward a previous patch that may sit on the back side of the refinery.

2. **Add `teleport_state.is_none()` to the settled gate.** ExitPad currently gates on `!moving && at_exit` where `moving` is `movement_target.is_some()`. Add a third condition: no teleport in progress. This prevents the unlikely edge case of transitioning out of ExitPad while a chrono effect is still resolving.

That's the entire fix.

## Design

### Components

No new components. No new state variants. No new fields. No new INI keys.

### Interfaces / Contracts

`phase_exit_pad` contract is unchanged from the caller's perspective:
- Input: `&mut Simulation`, `&mut MinerSnapshot`, pad cell, exit cell, refinery sid
- Output: mutates `snap.miner` (state, dock_phase, dock-related fields)
- Side effects: may issue movement command via `movement::issue_direct_move`

The semantic change is internal to the function: the post-arrival cleanup now also clears the ore-target archive, and the gate condition is stricter.

### Data Flow

**Before (current):**
```
Unloading completes → ExitPad
ExitPad tick 1:    issue_direct_move(exit), set facing
ExitPad tick K:    !moving && at_exit
                   → reserved_refinery = None
                   → dock_queued = false
                   → forced_return = false
                   → state = SearchOre
                     [target_ore_cell preserved]
                     [last_harvest_cell preserved]
SearchOre tick K+1: search_center = last_harvest_cell (back of refinery)
                    target_ore_cell = (cell behind refinery)
                    → state = MoveToOre
MoveToOre tick K+2: A* path OR direct_move toward target
                    [headbutt cycle]
```

**After (fixed):**
```
Unloading completes → ExitPad
ExitPad tick 1:    issue_direct_move(exit), set facing
ExitPad tick K:    !moving && at_exit && !teleporting
                   → reserved_refinery = None
                   → dock_queued = false
                   → forced_return = false
                   → target_ore_cell = None       [NEW]
                   → last_harvest_cell = None     [NEW]
                   → state = SearchOre
SearchOre tick K+1: search_center = current_pos (exit cell)
                    target_ore_cell = nearest ore from exit cell
                    [ore on the EXIT side of refinery — accessible]
                    → state = MoveToOre
MoveToOre tick K+2..N: normal harvest cycle
```

### Concrete diff

[src/sim/miner/miner_dock_sequence.rs:446-453](../../src/sim/miner/miner_dock_sequence.rs#L446):

```rust
// BEFORE
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

```rust
// AFTER
let teleporting = sim
    .entities
    .get(snap.entity_id)
    .is_some_and(|e| e.teleport_state.is_some());

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

### Error Handling

No new error paths. Existing assertions and edge cases are preserved:

- If the entity is missing from `sim.entities`, the `is_some_and` returns false, and the gate behaves the same as before (the existing `moving` check has the same handling).
- If `reserved_refinery` was already cleared by another code path (e.g., refinery destroyed mid-cycle), the field assignments are idempotent.

### Testing Strategy

Three new unit tests in [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs):

1. **`exit_pad_clears_ore_targets_on_arrival`** — set up a chrono miner in ExitPad with `target_ore_cell = Some((10, 10))` and `last_harvest_cell = Some((10, 10))`. Run a tick after the unit reaches the exit cell. Assert: state is now SearchOre, both target fields are None.

2. **`exit_pad_blocks_transition_during_teleport`** — set up a chrono miner at the exit cell with `teleport_state = Some(...)` (mid-warp). Run a tick. Assert: still in ExitPad phase (gate held), state still Dock, ore targets unchanged.

3. **`chrono_miner_no_headbutt_with_ore_behind_refinery`** — integration-style test: spawn a chrono miner, refinery, and ore patch arranged so the ore is ~3 cells behind the refinery (the bug scenario). Run the miner through a full cycle: search → harvest → return → dock → unload → exit. Assert: after exit, the miner's next target_ore_cell is on the exit side of the refinery (not the back side), and movement_target produces a path that does not collide with the refinery footprint.

The existing chrono-miner tests in [miner_tests.rs:321-465](../../src/sim/miner/miner_tests.rs) will need a quick review for any that relied on the sticky-archive behavior. Likely candidates: any test that asserts `target_ore_cell == Some(...)` immediately after `Unload`. Update them to match the new contract (cleared on ExitPad).

## Architectural Decisions

**Patterns followed:**
- Existing two-phase snapshot pattern in `tick_miners` (no new mechanism).
- Existing settled-gate pattern (`!moving && at_exit`) is extended, not replaced.
- Field clear at state transition matches the established convention for `reserved_refinery`, `dock_queued`, `forced_return`.

**Patterns deviated from:** none.

**Tech debt:**
- We are NOT implementing a Guard state with HarvestInterval — but only because gamemd doesn't have one for harvesters. This decision is documented in this design doc and in the gap-scan report. If a future audit re-discovers the alleged "HarvestInterval pause", this design doc is the authoritative record that it doesn't actually exist in gamemd.
- The `last_harvest_cell` archive lifecycle now diverges slightly from gamemd's `UnitClass+0x218` archive lifecycle. gamemd's archive persists across docks and is cleared elsewhere; ours is cleared at undock. The observable behavior is equivalent in normal play (both produce a fresh-scan-then-pick-nearest pattern) but the internal mechanism differs. Acceptable — the archive lifecycle is invisible to the player.

## Alternatives Considered

**Approach B (originally pitched):** Clear `target_ore_cell` only, not `last_harvest_cell`.
Rejected because `handle_search_ore` uses `last_harvest_cell` as the search center if non-null. Clearing only `target_ore_cell` would still bias the next scan toward the back-of-refinery cell.

**Approach C (originally pitched):** Replace `issue_direct_move` in MoveToOre's near branch with pathfinding-aware move.
Rejected because the existing direct_move usage is load-bearing — A* refuses to path onto blocked ore cells. Replacing it would break harvesting itself. The bug isn't that direct_move can't pathfind; the bug is that it's targeting a cell that requires pathfinding around an obstacle. Fix the destination, not the locomotion.

**Approach A (originally pitched):** Add `MinerState::Guard` variant + `HarvestInterval` Guard pause + `Is_Ok_To_End`-equivalent gate + clear archive.
Rejected after verification: gamemd has no HarvestInterval pause for normal harvesters (verified — `RulesClass+0x1790` is `SlaveMinerKickFrameDelay`, only triggers for slave miners). Adding a 4-second pause would be a divergence from gamemd, not a parity fix. The locomotion gate reduces to `teleport_state.is_none()` in the simpler form below.

**Approach D (originally pitched):** Hybrid B + C.
Rejected for the same reason as C — the direct_move replacement would break harvesting.

## Out-of-scope (deferred or unrelated)

These came up during the brainstorm but are not part of this fix:

- **G5 (DockAnim spawn on EnterTransport)** — separate visual polish, no interaction with this bug.
- **G6 (atomic vs drip unload + Purifier formula)** — semantic correctness gap in the unload sequence, separate concern.
- **G7 (ore-overlay destruction on dock pad)** — visual polish.
- **G9 (out-of-ore Hunt fallback)** — gap-scan downgraded to LOW; Rust's WaitNoOre is arguably saner. Don't port.
- **Doc fixes outstanding** — `HARVESTER_DOCK_UNLOAD_SEQUENCE.md` has at least three errors uncovered this session (inverted DockedUnit/DockedBuilding offsets, wrong radio-7 narrative, mislabeled `RulesClass+0x1790`). These should be patched separately.

## Definition of Done

- [ ] `phase_exit_pad` updated as described
- [ ] Three new unit tests pass
- [ ] All existing miner tests pass (with updates if any encoded the buggy behavior)
- [ ] `cargo clippy` clean for the changed file
- [ ] Manual verification in-game: spawn refinery + chrono miner + ore patch with ore on the BACK side of refinery; verify miner exits, scans, picks ore on the front/exit side, no head-butt cycle observed.
