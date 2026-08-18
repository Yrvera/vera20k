# Chrono Miner Close Return Parity Design

**Date:** 2026-05-22
**Status:** approved by user; implemented in `src/sim/miner/`
**Predecessor research:**
- `docs/research/CHRONO_MINER_NAVCOM_RADIO_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`
- `docs/research/miner/traces/CHRONO_MINER_CLOSE_RETURN_SCHEDULER_FRAME_TRACE.md`
- `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`
- `docs/research/CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`

## Goal

Fix the chrono miner full-cargo close-return path so a player sees the stock Yuri's Revenge behavior around refineries:

- Chrono miners close enough to the target refinery use the radio handshake before walking to the accepted pad cell.
- The close-return distance threshold uses the stock 3D lepton-space check and treats the exact threshold as close enough.
- Accepted radio return and refinery-pad entry respect the verified mission queue/commence boundary instead of collapsing all work into one Rust tick.
- Busy/full refineries reject a new chrono miner without evicting the current receiver-side contact; the miner stages at the art `QueueingCell` fallback.

This design intentionally does not rewrite the full `MissionClass`, `RadioClass`, or refinery unload FSM. It is a targeted parity patch for the already researched chrono miner close-return mismatch.

## Architecture Context

Chrono miner behavior currently lives under `src/sim/miner/`:

```text
tick_resource_economy
  -> tick_miners
    -> process_miner
      -> handle_return
        -> try_issue_chrono_far_return_teleport
        -> maybe set MinerState::Dock
      -> handle_dock_sequence
        -> RefineryDockPhase state machine
```

The current Rust flow waits until the chrono miner reaches the accepted dock cell before entering the dock/radio path. Stock `gamemd.exe` does not do that for the close-return case. `UnitClass::Mission_Harvest` state 2 sends `HELLO(0x02)` to the refinery while the miner is still at its current location. If the refinery returns `ROGER(0x01)`, harvest substate 3 queues mission 7 (`Mission_Enter`) later in that same mission dispatch. `UnitClass::AI` promotes the queued mission later in the same game frame, but the first live `Mission_Enter` dispatch is normally the next frame.

The current code also measures close/far return using cell-distance-squared against a refinery anchor. Stock behavior compares full 3D object-coordinate distance in leptons against `ChronoHarvTooFarDistance * 0x100`, using a strict greater-than check for too-far. Therefore exactly `50` cells at default rules is still close enough.

Layering stays within `sim/` plus tests. No `sim/` dependency on render, UI, sidebar, audio, or net is introduced.

## Impact Analysis

Expected files:

| File | Change | Risk |
|---|---|---|
| `src/sim/miner/miner_system.rs` | Replace cell-squared too-far test with 3D lepton threshold; send early chrono close-return `HELLO`; route accepted/refused results into the dock sequence or QueueingCell fallback | Medium |
| `src/sim/miner/miner_dock_sequence.rs` | Add or adjust a phase boundary so successful close-return HELLO does not immediately execute the `0x0E` accepted-cell handshake in the same tick | Medium |
| `src/sim/miner/mod.rs` | Possible new dock phase/state field for "MissionEnter queued/ready next tick" | Low-medium |
| `src/sim/miner/miner_dock.rs` | Only if existing contact helpers cannot express sender-side HELLO without receiver eviction | Medium |
| `src/sim/miner/miner_tests.rs` | Add focused parity tests and update stale expectations around close-enough chrono returns | Medium |

Risk areas:

1. **Serialization/state compatibility.** Adding a `RefineryDockPhase` variant is simple but affects serialized/debug state. Existing variants should stay stable; old saves using old variants should still deserialize if serde defaults permit.
2. **Double HELLO.** The early close-return HELLO must not be repeated immediately when the miner enters `Dock`.
3. **Receiver eviction regression.** Busy refinery HELLO rejection must not clear the refinery's current linked/awaiting miner.
4. **QueueingCell vs accepted cell.** `QueueingCell=4,1` is a staging fallback. The accepted `0x0E` movement target is the refinery NW origin plus `(3,1)` for stock GAREFN/NAREFN.
5. **Frame boundary.** The patch should model the verified one-dispatch boundary without pretending to know still-unverified mission timer jitter.

## Chosen Approach

Use a scoped miner FSM patch:

1. Add a lepton-space threshold helper for chrono far-return selection.
2. In `handle_return`, if the chrono miner is full and within the threshold, send the stock early `HELLO(0x02)` to the selected refinery before moving to the pad.
3. If the refinery accepts, transition into a dock phase that represents "Mission_Enter queued/commenced, first dispatch next tick" and only then performs the accepted-cell `0x0E` request.
4. If the refinery rejects or cannot accept, move/stage toward the art `QueueingCell` fallback, preserving existing deterministic retry behavior.
5. Keep existing deterministic reservation/contact structures as the scale-friendly implementation of stock radio contact state.

This keeps the implementation localized and avoids a broad `MissionClass`/`RadioClass` emulator.

## Alternatives Considered

### A. Full MissionClass and RadioClass emulation

This would model queued/current mission slots, mission timers, radio contacts, and all message dispatches more literally.

Pros: closest long-term architecture for all mission parity.

Cons: too broad for this verified bug; high blast radius across movement, combat, deploy, and production; would delay a small player-visible fix behind a general engine rewrite.

### B. Threshold-only patch

This would only fix `ChronoHarvTooFarDistance` math and leave the current "walk to accepted cell, then dock" behavior.

Pros: very small and low risk.

Cons: leaves the main stock mismatch: close-return chrono miners should radio the refinery before walking to the accepted pad. It also leaves busy-refinery rejection and QueueingCell fallback behavior under-tested.

### C. Scoped miner FSM patch

Pros: fixes the verified player-visible mismatch, limits changes to miner/refinery contact code, and gives focused tests for the exact cases now researched.

Cons: still an abstraction over stock mission/radio internals, so future mission work may revisit the representation.

Chosen: **C. Scoped miner FSM patch.**

## Tiny-Detail Ledger

| # | Detail | Required behavior | Source |
|---|---|---|---|
| 1 | Default CMIN threshold | `ChronoHarvTooFarDistance=50`; compare against `50 * 0x100 = 12800` leptons | `rulesmd.ini`, full-cargo close-return trace |
| 2 | Too-far comparison | Strict greater-than is too far; exact threshold remains close-return | `UnitClass::Mission_Harvest @ 0x0073E5E0` |
| 3 | Distance space | Use 3D object-coordinate distance in leptons, not 2D cell-distance-squared | `UnitClass::Mission_Harvest @ 0x0073E5E0` |
| 4 | State 2 close return | Close-return sends only `HELLO(0x02)` to the refinery before mission 7 is queued | `MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md` |
| 5 | Accepted HELLO timing | `Queue_Mission(7,0)` is promoted later the same `UnitClass::AI` frame; first live `Mission_Enter` dispatch is normally next frame | `CHRONO_MINER_CLOSE_RETURN_SCHEDULER_FRAME_TRACE.md` |
| 6 | Accepted cell target | `0x0E` movement target is refinery origin plus `(3,1)` for stock refineries | contact saturation report, full-cargo close-return trace |
| 7 | QueueingCell role | `QueueingCell=4,1` is fallback/staging after the close HELLO cannot proceed or after the far-return path needs staging | contact saturation report, `artmd.ini` |
| 8 | Full receiver HELLO | A full refinery returns `NEGATORY(0x0A)` without evicting its receiver-side contact | contact saturation report |
| 9 | Sender-side eviction | HELLO may evict the sender's old contact, not the receiver's current miner | contact saturation report |
| 10 | `0x0E` full contact | Building `0x0E` against a full contact can return `ROGER` after `0x13`/`0x12`; final unload activation waits for pad-clear/already-there conditions | contact saturation report |
| 11 | `0x18`/`0x16` timing | Already-there path sends these synchronously during `Mission_Enter`; `PerCellProcess` can then send `0x15` later that same AI frame | scheduler frame trace |
| 12 | Deploy-building dispatch | Mission `0x10` can be queued/promoted on that frame, but first `Mission_Deploy_Building` dispatch is normally next frame | scheduler frame trace |
| 13 | Zero-link unload exit | Stock state-4 zero-link exit does not call `ReleaseDockedHarvester` or `Force_Track(0x47)` | reachability audit report |
| 14 | Unknown jitter | Exact mission timer table and `RandomRanged(0,2)` harvest epilogue jitter remain unresolved; do not assert exact sub-frame/timer behavior beyond the verified dispatch boundary | scheduler frame trace |

## Design

### Components

**Chrono return threshold helper**

Add a helper near `try_issue_chrono_far_return_teleport` that computes object-coordinate distance in leptons from the miner to the target refinery and compares it to the rules threshold:

```text
too_far = distance_3d_leptons(miner_position, refinery_position) > threshold_cells * 256
```

The helper should use integer/fixed-point math only. If current sim position data lacks subcell precision, use the highest precision already present in `Position`; do not introduce floating point into sim logic.

**Early close-return HELLO**

When a full chrono miner selects a refinery and is not too far:

- Send `HELLO(0x02)` through the existing refinery contact model.
- If accepted, store enough state to enter `Mission_Enter` on the next miner tick without sending HELLO again.
- If refused, send the miner to the refinery art `QueueingCell` staging target and keep retry semantics deterministic.

**MissionEnter boundary phase**

Represent the verified queue/commence boundary explicitly. The state name can be an enum variant such as `RefineryDockPhase::MissionEnterQueued` or a small flag on existing dock state, but it must preserve this contract:

- Tick N: close-return HELLO accepts; miner records pending MissionEnter.
- Tick N+1: dock sequence performs the accepted-cell `0x0E`/movement path.

This is an approximation of stock frame scheduling, but it preserves the player-visible ordering and avoids same-tick HELLO-plus-accepted-cell collapse.

**QueueingCell fallback**

Keep `QueueingCell` as staging/fallback, not as the accepted dock cell. The fallback should be used when the early close HELLO cannot proceed. The accepted-cell helper remains the hardcoded stock refinery pad for current refineries until broader art docking data support is implemented.

### Interfaces / Contracts

- `handle_return` remains the owner of refinery selection and return-path branching.
- `miner_dock_sequence` remains the owner of pad-entry and unload progression.
- `miner_dock` contact helpers must expose accept/refuse semantics without receiver-side eviction.
- No render/UI/audio dependency is added to `sim`.
- All new state must be deterministic and hash/serde-friendly if it participates in game state.

### Data Flow

```text
full chrono miner
  -> choose refinery
  -> compare 3D lepton distance to ChronoHarvTooFarDistance
    -> too far: existing teleport-return path
    -> close:
       -> HELLO(0x02)
          -> ROGER: enter pending MissionEnter boundary
             -> next tick: send accepted-cell 0x0E path
          -> NEGATORY/no contact: stage at QueueingCell and retry later
```

### Error Handling

This path should degrade to existing no-refinery/no-valid-target behavior if:

- The chosen refinery entity no longer exists.
- Its object type cannot be resolved.
- Its contact state is stale after cleanup.
- The QueueingCell is unavailable.

No panic should be introduced in normal sim execution. Tests may use `expect` in setup-only code.

### Testing Strategy

Add focused tests in `src/sim/miner/miner_tests.rs`:

1. `chrono_return_at_exact_threshold_uses_close_radio_path`
2. `chrono_return_over_threshold_uses_far_teleport_path`
3. `chrono_close_return_sends_hello_before_moving_to_accepted_cell`
4. `chrono_close_hello_accept_waits_one_tick_before_mission_enter`
5. `chrono_close_hello_refused_stages_at_queueingcell`
6. `chrono_close_hello_refused_does_not_evict_current_refinery_contact`
7. Update any stale test that currently expects close-enough chrono miners to do nothing until they reach the dock cell.

Run at least:

```text
cargo test miner
```

If compile time is high, first run the narrow miner test module/filter, then the broader miner suite.

## Architectural Decisions

- **Use a local FSM phase instead of a generic MissionClass abstraction.** This keeps the patch proportional to the researched bug.
- **Keep deterministic contact/reservation structures.** Stock has looser radio contact behavior, but the project scale goal allows clean deterministic internals when player-visible behavior matches.
- **Model one tick of queue/commence boundary.** The verified stock boundary is one live mission-dispatch later, normally next frame. Rust's miner tick can represent that cleanly as the next miner tick.
- **Do not solve unrelated stock state-4 exit details in this patch.** The zero-link unload exit correction is documented, but this patch is about chrono close-return entry and contact timing.

## Approval Checklist

- [ ] Approve scoped miner FSM patch.
- [ ] Implement threshold helper, early HELLO, boundary phase, QueueingCell fallback, and tests.
- [ ] Run focused miner tests.
- [ ] Revisit exact mission timer jitter only if tests or gameplay inspection expose a visible mismatch after this patch.
