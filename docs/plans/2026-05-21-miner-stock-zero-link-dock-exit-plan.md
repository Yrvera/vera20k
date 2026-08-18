# Miner Stock Zero-Link Dock Exit Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the normal stock refinery unload exit model with the verified zero-link `Mission_Deploy_Building` state-4 behavior, while preserving `ReleaseDockedHarvester` / `Force_Track(0x47)` behavior only for conditional reciprocal-link interrupts.

**Architecture:** This is a deterministic `sim/` parity correction in the miner/refinery dock state machine. The fix stays inside `src/sim/miner`, `src/sim/world`, and tests; no `sim/` code may depend on render, ui, sidebar, audio, or net.

**Design Doc:** None. This plan is grounded directly in the 2026-05-21 re-swarm reports and the corrected research docs. Review before implementation.

---

## Grounding Summary

The latest reports show stock `CMIN/HARV -> GAREFN/NAREFN` DockUnload does not establish reciprocal `unit/building +0x2E4`. Normal completion remains in the zero-link `UnitClass::Mission_Deploy_Building @ 0x0073D630` state-4 path.

Verified stock state-4 behavior: it locates the refinery using `DAT_0089F6A0 == (-1,0)`, checks `Refinery=yes` and `building+0x57C`, clears `unit+0x6D1`, may send BREAK(0x03), and returns to Harvest scheduling. It does not call `ReleaseDockedHarvester`, `UndockUnit`, or `Force_Track(0x47)`.

`ReleaseDockedHarvester @ 0x004595C0` and `UndockUnit @ 0x004593A0` remain valid conditional reciprocal-link helpers for nonzero-link or interrupt contexts such as sell/destroy/temporal cleanup.

`TechnoClass::Receive_Radio(0x18/0x19) @ 0x006F4AB0` toggles byte `+0x418`, not `+0x2E4`. Rust already has `RefineryDockContacts::contact_entered` for this concept.

Pre-fix Rust normal `Departing` followed the old model: it started `Force_Track(0x47)`, emitted `RefineryExitSfx`, cached a queue-cell exit, drove off, and released contact/pad state around that exit drive.

INI inputs remain unchanged: `[CMIN]/[HARV] Dock=NAREFN,GAREFN`, `Harvester=yes`; `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`; art `QueueingCell=4,1` remains queue/fallback data, not the state-4 exit source. `BunkerWallsDownSound=TankBunkerDown` must no longer fire for normal stock refinery unload.

Still unknown: exact live-frame timing for any post-state-4 `+0x418 -> 0x08 -> 0x19/0x03` cleanup cascade. This should remain a deferred runtime trace, not block removing the wrong forced-exit behavior.

## Key Technical Decisions

- Keep `RefineryDockPhase::Departing` as the Rust stock state-4 exit phase, but strip the `ReleaseDockedHarvester` effects from it. **Confidence:** high.
  - **Source:** `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`; current code pattern in `miner_dock_sequence.rs`.
- Preserve `start_refinery_exit_force_track` only for `interrupt_refinery_docked_miners` and other future nonzero-link paths. **Confidence:** high.
  - **Source:** `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`; sell interrupt test already exists.
- Do not emit `RefineryExitSfx` for normal stock `Departing`. **Confidence:** high.
  - **Source:** `BUILDING_MISSIONREPAIRANDPRODUCE_DOCKUNLOAD_REACHABILITY_GHIDRA_REPORT.md`; `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` corrections.
- Release normal stock dock contact/pad state when the Rust state-4 handoff runs, and cover two-miner admission timing with a focused regression. **Confidence:** medium.
  - **Source:** state-4 zero-link branch clears `+0x6D1`, may send BREAK(0x03), and current Rust contact state is a model spanning several binary concepts. Do not claim exact `+0x418` timing without a runtime trace.
- Do not keep a dedicated cached queue-cell exit leg for stock completion. Let the next Harvest/SearchOre scheduling choose the next movement from the miner's current cell. **Confidence:** high for removing the cached leg, medium for exact first post-state-4 movement frames.
  - **Source:** stock state 4 queues/continues Harvest and does not install a new passable-cell/NavCom destination.

## Open Questions

### Resolved During Planning

- Is normal stock exit `ReleaseDockedHarvester`? No. It is zero-link `Mission_Deploy_Building` state 4.
- Is `Force_Track(0x47)` normal for stock CMIN/HARV delivery? No. It is conditional reciprocal-link / interrupt release.
- Is `BunkerWallsDownSound` normal stock refinery departure audio? No. That came from the wrong `ReleaseDockedHarvester` model.
- Does `DAT_0089F6A0` come from `DockingOffset%d`? No. It is hardcoded `(-1,0)`.

### Deferred to Implementation

- Exact timing of lingering `+0x418` cleanup after state-4: static evidence proves the branch and gates, but runtime frame timing requires a replay/runtime trace.
- Whether `Departing` should be renamed after behavior is corrected: renaming would churn many tests and is not necessary for parity.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/miner/miner_dock_sequence.rs` | Remove normal forced-exit effects from stock `Departing`; keep interrupt forced-track helper. |
| Modify | `src/sim/miner/mod.rs` | Update `RefineryDockPhase` comments to describe stock zero-link state-4 behavior. |
| Modify | `src/sim/miner/miner_tests.rs` | Replace normal forced-track/SFX/deferred-release tests with zero-link state-4 expectations; preserve sell interrupt coverage. |
| Modify | `src/sim/world/mod.rs` | Update `SimSoundEvent::RefineryExitSfx` comments or scope to conditional release only. |
| Modify | `src/rules/ruleset.rs` | Update `BunkerWallsDownSound` comments to remove normal refinery departure claim. |
| Review | `src/sim/world/world_hash.rs` | Confirm no state hash changes are needed after removing normal forced-track state. |

## Interface Changes

No new public APIs are required. Existing `RefineryDockPhase::Departing` and `RefineryDockContacts` can model the corrected stock path.

Behavioral contract changes:
- `phase_departing` must no longer seed `forced_drive_track` for normal stock unload.
- `phase_departing` must no longer emit `SimSoundEvent::RefineryExitSfx` for normal stock unload.
- `RefineryDockContacts::contact_entered` continues to represent the `+0x418` radio/contact byte, not a reciprocal dock link.
- `interrupt_refinery_docked_miners` remains the owner of sell/destroy-style forced `0x47` release behavior.

## Sim Checklist

- [x] No new floating-point game logic is planned.
- [x] No new state is planned; deterministic hash likely unchanged.
- [x] No dependencies on render/ui/sidebar/audio/net.
- [x] Tick ordering impact noted: state-4 cleanup moves earlier than current exit-cell arrival release.
- [x] BTreeMap iteration order preserved through existing `EntityStore` and `RefineryDockContacts`.

## Risk Areas

- Normal miner throughput may change because dock/contact release moves earlier than the current exit-cell arrival.
- Existing tests around `Departing`, exit-cell cache, `RefineryExitSfx`, and `Force_Track(0x47)` encode the wrong model and must be rewritten, not blindly preserved.
- Two-miner queue behavior depends on contact release timing and should get a focused regression.
- Removing normal forced track may change visual departure direction; verify with a follow-up trace/fidelity check.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 2 | No normal `Force_Track(0x47)` on stock delivery | Player-visible departure curve/facing should not use the interrupt release curve every ore cycle | Unit test asserts no `forced_drive_track` after normal state-4; sell interrupt still asserts `0x47`. |
| 2 | No normal `TankBunkerDown` / `RefineryExitSfx` on stock delivery | Audio cue would play every ore delivery if left wrong | Unit test asserts zero `RefineryExitSfx` during normal delivery; parser test for key can remain. |
| 3 | Stock contact/pad cleanup at handoff | Queue throughput and second-miner admission are visible in normal economy | Two-miner test asserts cleanup happens at the Rust state-4 handoff and does not depend on artificial exit-cell arrival. |
| 3 | Return to Harvest/SearchOre after state-4 | Miner should continue economy loop without a fake dedicated release path | Integration test runs full unload cycle and asserts transition to `SearchOre` or `WaitNoOre` with contact cleared. |
| 4 | Interrupt release still uses `Force_Track(0x47)` | Sell/destroy/temporal interruption remains visibly distinct and still matches verified helper behavior | Existing sell interrupt test remains or is tightened. |

---

## Tasks

### Task 1: Rename Comments And Test Intent, Not Runtime Behavior

**Why:** Prevent the next edit from preserving stale assumptions through comments and test names.

**Files:**
- Modify: `src/sim/miner/mod.rs`
- Modify: `src/sim/miner/miner_dock_sequence.rs`
- Modify: `src/sim/world/mod.rs`
- Modify: `src/rules/ruleset.rs`

**Pattern:** Existing module-level doc comments and enum variant comments.

**Step 1:** In `RefineryDockPhase::DepositCooldown`, replace the claim that cooldown transitions into `ReleaseDockedHarvester` with: cargo-empty waits one dump-gate interval, then enters stock state-4 cleanup.

**Step 2:** In `RefineryDockPhase::Departing`, describe it as Rust's stock state-4 cleanup/hand-off phase. Mention explicitly that normal stock exit does not seed `Force_Track(0x47)`.

**Step 3:** In `miner_dock_sequence.rs`, update comments above `refinery_exit_cell` and `phase_departing` so they no longer cite `ReleaseDockedHarvester` as the normal stock source.

**Step 4:** In `SimSoundEvent::RefineryExitSfx` and `RuleSet::bunker_walls_down_sound` comments, scope the sound to conditional reciprocal-link release, not normal refinery unload.

**Step 5:** Verify with:
```powershell
rg -n "ReleaseDockedHarvester|Force_Track|BunkerWallsDownSound|RefineryExitSfx" src/sim/miner src/sim/world src/rules
```
Expected: remaining normal-path mentions are gone or explicitly say conditional/interrupt.

### Task 2: Remove Normal Forced Track And Normal Exit SFX

**Why:** This removes the highest-confidence wrong behavior from every stock ore delivery.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs`

**Pattern:** Keep `start_refinery_exit_force_track` as a helper used by `interrupt_refinery_docked_miners`; remove only the normal call from `phase_departing`.

**Step 1:** In `phase_departing`, delete the first-entry `SimSoundEvent::RefineryExitSfx` push.

**Step 2:** In `phase_departing`, delete the normal call to `start_refinery_exit_force_track(entity, snap.speed)`.

**Step 3:** Keep `start_refinery_exit_force_track` unchanged for `interrupt_refinery_docked_miners`.

**Step 4:** Keep `entity.forced_drive_track = None` cleanup in abort paths so stale forced tracks cannot leak between states.

**Step 5:** Verify with:
```powershell
rg -n "start_refinery_exit_force_track|RefineryExitSfx" src/sim/miner/miner_dock_sequence.rs
```
Expected: `start_refinery_exit_force_track` appears in helper definition and interrupt path only; `RefineryExitSfx` no longer appears in normal `phase_departing`.

### Task 3: Move Stock Dock/Contact Release To Stock Handoff

**Why:** Current Rust holds the dock/contact until artificial exit-cell arrival. Stock zero-link state 4 clears unload-active state and may break radio/contact without waiting for `ReleaseDockedHarvester`.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs`
- Modify: `src/sim/miner/miner_dock.rs` only if an idempotent helper is clearer.

**Pattern:** Existing `release_on_pad`, `release_contact`, and `clear_contact_entered` methods in `RefineryDockContacts`.

**Step 1:** In stock `phase_departing`, release the Rust pad/contact bookkeeping during the handoff to SearchOre/Harvest scheduling.

**Step 2:** Treat this as Rust's current modeled handoff, not a direct claim that binary state 4 clears `+0x418`. The exact `+0x418 -> 0x08 -> 0x19/0x03` timing remains a runtime-trace follow-up.

**Step 3:** Remove any arrival-only release branch tied to a cached exit-cell drive.

**Step 4:** Preserve miner state cleanup on final transition out of `Departing`: `reserved_refinery = None`, `dock_queued = false`, `forced_return = false`, `target_ore_cell = None`, `exit_cell = None`, `dock_phase = Approach`, `state = SearchOre`.

**Step 5:** Verify with a narrow unit test update in Task 5, then run:
```powershell
cargo test deposit_cooldown_releases_dock_on_stock_state4_handoff -- --nocapture
```
Expected after test rewrite: dock/contact are cleared when stock `Departing` performs the handoff, without waiting for an exit-cell arrival.

### Task 4: Remove Dedicated Stock Exit-Cell Movement

**Why:** Verified stock state 4 does not install a new passable-cell/NavCom destination. Movement out of the pad should be a consequence of the next Harvest/SearchOre movement, not a hardcoded queue-cell release leg.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs`

**Pattern:** Existing `issue_move_command` and `issue_direct_move` path movement.

**Step 1:** In stock `phase_departing`, do not compute, cache, or drive to `exit_cell`.

**Step 2:** Return to `SearchOre`/Harvest-equivalent scheduling directly after the state-4 cleanup.

**Step 3:** Remove normal-path `bypass_grid` exit-drive comments and tests. If blocked-start SearchOre movement exposes a real Rust pathing issue, fix that as SearchOre/pathing behavior, not as a fake refinery release path.

**Step 4:** Keep facing derived from actual path movement; do not pin facing to `0x47`.

**Step 5:** Verify with:
```powershell
cargo test stock_departing_hands_directly_to_search_without_exit_move -- --nocapture
```
Expected: stock `Departing` clears dock state, leaves `exit_cell` unset, emits no forced track, emits no refinery exit SFX, and transitions to SearchOre without a movement target.

### Task 5: Rewrite Normal-Exit Tests

**Why:** Existing tests encode the old wrong model and will otherwise fight the fix.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs`

**Pattern:** Existing focused miner tests with direct phase setup.

**Step 1:** Replace `dock_exit_emits_refinery_exit_sfx_once_per_cycle` with `stock_dock_exit_does_not_emit_refinery_exit_sfx`.

**Step 2:** Replace `chrono_departing_starts_force_track_0x47_before_exit_move` with `stock_departing_does_not_start_force_track_0x47`.

**Step 3:** Replace `chrono_departing_force_track_runs_before_normal_exit_move` with a test asserting stock `Departing` does not start an explicit exit movement.

**Step 4:** Update `deposit_cooldown_holds_pad_and_defers_dock_release`: after cooldown completes, tick stock `Departing` once and assert `!is_occupied(refinery)` after the handoff.

**Step 5:** Keep `sell_refinery_interrupts_docked_miner_with_force_track_0x47` and tighten it to assert forced track remains interrupt-only.

**Step 6:** Verify with:
```powershell
cargo test stock_dock_exit_does_not_emit_refinery_exit_sfx -- --nocapture
cargo test stock_departing_does_not_start_force_track_0x47 -- --nocapture
cargo test sell_refinery_interrupts_docked_miner_with_force_track_0x47 -- --nocapture
```
Expected: new stock tests pass and interrupt forced-track test still passes.

### Task 6: Review Deterministic Hash And Save-State Impact

**Why:** Removing normal forced-drive state may change deterministic state transitions but should not require a new serialized field.

**Files:**
- Review: `src/sim/world/world_hash.rs`
- Review: `src/sim/production/production_types.rs`
- Review: `src/sim/miner/mod.rs`

**Pattern:** Existing world hash already includes dock contacts, waiting queues, `contact_entered`, and `on_pad`.

**Step 1:** Confirm no new state was added in Tasks 2-4.

**Step 2:** Confirm existing `forced_drive_track` remains hashed where movement state is hashed.

**Step 3:** Confirm removing normal forced-drive creation does not require schema changes.

**Step 4:** If no fields changed, do not edit `world_hash.rs`.

**Step 5:** Verify with:
```powershell
cargo test world_hash -- --nocapture
```
Expected: existing world hash tests pass or only expected hash snapshots change if the suite stores exact values.

### Task 7: Run Focused Miner Regression

**Why:** The dock system is hot-path economy behavior; focused tests should catch timing and queue regressions before a full suite.

**Files:**
- No planned edits unless tests expose a defect.

**Pattern:** Existing `miner_tests.rs` integration-style tests.

**Step 1:** Run:
```powershell
cargo test miner::miner_tests -- --nocapture
```

**Step 2:** If failures are from old test expectations around normal `ReleaseDockedHarvester`, update those tests to the verified zero-link model.

**Step 3:** If failures show a real behavioral regression outside this plan, stop and reassess before layering fixes.

**Expected:** Miner tests pass with normal stock exit using no forced track and no exit SFX.

### Task 8: Run Full Verification

**Why:** Miner dock state touches movement, production contacts, sound events, and deterministic world state.

**Files:**
- No planned edits unless tests expose a defect.

**Step 1:** Run:
```powershell
cargo test -- --nocapture
```

**Step 2:** Record any unrelated pre-existing failures separately; do not fix unrelated dirty-worktree failures unless asked.

**Step 3:** Run a quick source scan:
```powershell
rg -n "normal.*ReleaseDockedHarvester|ReleaseDockedHarvester.*normal|RefineryExitSfx.*dock cycle|Force_Track\\(0x47\\).*Departing" src docs/plans
```
Expected: no remaining implementation comments claim stock normal exit uses `ReleaseDockedHarvester`.

### Task 9: Mandatory Fidelity Follow-Up

**Why:** Static evidence is strong, but player-visible first movement after state-4 still deserves a runtime parity trace.

**Files:**
- No Rust edits in this task.

**Step 1:** Run `/trace-action` or `/fidelity-check` for one standard CMIN full-cargo unload at GAREFN.

**Step 2:** Check these outputs:
- No forced track `0x47` on stock completion.
- No `TankBunkerDown` departure cue on stock completion.
- Miner returns to Harvest/SearchOre after state-4 cleanup.
- Second miner admission timing does not wait for an artificial `ReleaseDockedHarvester` exit.

**Step 3:** If the trace finds a player-visible mismatch not covered by this plan, write a follow-up plan rather than expanding this patch.

## Sources & References

- `docs/research/MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`
- `docs/research/CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
- `docs/research/DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/BUILDING_MISSIONREPAIRANDPRODUCE_DOCKUNLOAD_REACHABILITY_GHIDRA_REPORT.md`
- `docs/research/RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md`
- `docs/research/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
- Ghidra addresses: `UnitClass::Mission_Deploy_Building @ 0x0073D630`; `BuildingClass::Receive_Radio @ 0x0043C2D0`; `TechnoClass::Receive_Radio @ 0x006F4AB0`; `Foundation_direction_table_init @ 0x0049F2F0`; conditional `ReleaseDockedHarvester @ 0x004595C0`; conditional `UndockUnit @ 0x004593A0`.
- INI keys: `ini/rulesmd.ini` `[CMIN]/[HARV] Dock=NAREFN,GAREFN`, `Harvester=yes`; `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`; `[AudioVisual] BunkerWallsDownSound=TankBunkerDown`; `ini/artmd.ini` `[GAREFN]/[NAREFN] QueueingCell=4,1`, commented `[NAREFN] ;DockingOffset0=256,0,0`.
- Current code: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_tests.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/rules/ruleset.rs`.
- Recent relevant commits: `973149b sim/miner: refinery exit Force_Track 0x47 + chrono warp to QueueingCell + sell undocks`; `39be632 sim/miner: refinery contact protocol, dock pivot/exit cache, exit SFX`; `3fc928a sim/miner: refinery exit + chrono inbound warp parity fixes`.
