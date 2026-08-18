# Refinery Dock Radio 0x16 Design

## 2026-05-26 Update: Mission `0x10` Facing Gate

This design remains correct for radio `0x16`, but its placeholder treatment of
`Pivoting` is superseded by
`docs/research/miner/MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`.

Do not delete the deploy-facing gate from the overall unload path. Move/keep it
under mission `0x10` / `Mission_Deploy_Building`, where stock checks the
`RateTimer` window before setting the unload display latch. Not-ready mission
deploy returns delay `5` and may call active locomotor vtable `+0x4C(0x4000)`.
Accepted unload-start still must not hard snap `entity.facing = 0x40`, and stock
unload latch should not emit `DockDeploy`.

## Goal

Correct the stock refinery dock handoff so radio `0x16` is modeled as a sync/eligibility step, not as an East-facing body pivot or unload-start gate.

## Architecture Context

The existing miner dock flow lives in `src/sim/miner/` and is driven by `tick_miners`, which snapshots each miner and dispatches dock behavior through `handle_dock_sequence`.

The current dock FSM already has most of the needed shape:

- `MissionEnter`: sends/handles CAN_DOCK and accepted-cell movement.
- `AwaitingAcceptedCell`: waits for the accepted-cell move to finish.
- `FaceSync`: intended to represent the contact-entered / radio `0x16` interval.
- `MissionQueued`: represents radio `0x15` queuing mission `0x10`.
- `Pivoting`: currently treats mission `0x10` entry as an East-facing gate before unload.
- `Unloading`: drains cargo.

The bad fit is that `FaceSync` and `Pivoting` use `sync_dock_facing`, `DOCK_FACING_EAST`, and `dock_pivot_facing` as if `UnitClass__Receive_Radio(0x16)` directly owns East-facing body rotation and unlocks unload when facing reaches `0x40`. Verified gamemd behavior contradicts that: first ordinary `0x16` only calls the active locomotor turn/sync method and returns; later eligible `0x16` can send radio `0x15`.

## Impact Analysis

Touched modules:

- `src/sim/miner/miner_dock_sequence.rs`: primary FSM and helper changes.
- `src/sim/miner/mod.rs`: phase comments and likely removal or repurposing of `dock_pivot_facing`.
- `src/sim/miner/miner_tests.rs`: replace East-facing pivot expectations with radio predicate and side-effect tests.

Risk areas:

- Tick timing around accepted-cell arrival, retry due, `0x15`, and unload-start can shift if phases collapse.
- Existing tests that assume `MissionQueued -> Pivoting -> Unloading` with East-facing prealignment need to be rewritten.
- Removing `entity.facing = 0x40` may expose current render-facing assumptions. That is acceptable for this patch unless a separate Mission_Deploy_Building facing investigation proves another owner should set it.

## Chosen Approach

Use a minimal phase rewire.

Keep the existing FSM structure, but change the semantics:

- `MissionEnter` remains the owner of CAN_DOCK retry and accepted-cell movement.
- `FaceSync` becomes the waiting state after first ordinary `0x16` sync, not a body-facing pivot.
- `FaceSync -> MissionQueued` is gated by the later `0x16` predicates: not moving, live building destination/refinery, contact-entered flag set, mission-enter equivalent active, retry due.
- `MissionQueued` represents radio `0x15` only. It must not start unload side effects.
- `Pivoting` should be repurposed or renamed as a mission `0x10` deploy-facing gate. It should not be radio `0x16` behavior and must not hard snap body facing.

This follows the existing miner FSM style and avoids introducing a new radio subsystem for one bounded parity fix.

## Tiny-Detail Ledger

- First ordinary `0x16` calls the base radio handler, checks `+0x6AF == 0`, checks `RateTimer::Current(+0x388) != 0x4000`, calls active locomotor `+0x4C(0x4000)`, then returns `1`. Source: Ghidra `UnitClass__Receive_Radio @ 0x00737430`.
- First ordinary `0x16` does not send `0x15`, does not start unload, and does not set pad/on-dock state. Source: Ghidra `0x00737430`; `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`.
- `0x16` does not call `GetDockCoord`, `Set_Destination`, or write location. Source: `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`.
- `0x16` does not directly write body facing East. Source: Ghidra `0x00737430`.
- Later eligible `0x16` can send `0x15` only when the unit is not moving, has a destination, contact-entered is set, destination `WhatAmI()==6`, and current mission is `7`. Source: Ghidra `0x00737430`.
- `0x15` queues sender mission `0x10`; it is not itself the unload-start side-effect owner. Source: Ghidra `0x00737430`, `UnitClass__Mission_Deploy_Building @ 0x0073D630`.
- MissionEnter retry timing must continue using the current deterministic `14..16` frame path. Source: dock conflict audit and current `schedule_enter_retry`.
- Normal stock zero-link completion must remain separate from `ReleaseDockedHarvester` / `Force_Track(0x47)`. Source: Ghidra `0x0073D630`.

## Design

### Components

`phase_mission_enter`

- Keep CAN_DOCK / accepted-cell logic.
- When the miner is already at the accepted cell and the contact handoff can begin, set the contact-entered marker and enter `FaceSync`.
- Schedule the normal Enter retry as current code does.
- Do not call a body-facing helper.

`phase_face_sync`

- Represents the interval after first ordinary `0x16` sync and before later eligible `0x16 -> 0x15`.
- Updates miner snapshot position from entity position.
- Ensures contact-entered remains set.
- If retry is not due, remain in `FaceSync`.
- If retry is due, evaluate the later-`0x16` predicate:
  - entity is stopped / no movement target;
  - reserved refinery still resolves to a live building destination;
  - contact-entered is set;
  - miner is still in the MissionEnter-equivalent dock state;
  - destination/refinery is still the active dock target.
- If predicate passes, clear retry and set `MissionQueued`.
- If predicate fails but the refinery is still valid, schedule retry or remain waiting as appropriate.

`phase_mission_queued`

- Represents radio `0x15` queuing mission `0x10`.
- It should not set display override, play `DockDeploy`, link on pad, seed unload timer, or drain cargo.
- It should transition to the existing deploy/unload-start owner on the next tick.

`phase_pivoting` / mission `0x10` deploy-facing gate

- Remove radio-`0x16` East-facing claims from this phase.
- Gate unload-start on the verified mission `0x10` `RateTimer` accept expression, not direct equality of `entity.facing == 0x40`.
- When not ready, issue the active locomotor `+0x4C(0x4000)` equivalent only under the stock sync-flag condition and schedule the next mission pass with delay `5`.
- Do not set display override, unload timer, pad/on-dock link, or cargo drain until this gate accepts.
- Do not hard snap `entity.facing = 0x40` when accepted.

`start_unload_deploy`

- Keep display override, pad link, and unload timer seeding only after the mission deploy-facing gate accepts.
- Suppress stock `DockDeploy` sound at unload latch.
- Remove `entity.facing = DOCK_FACING_EAST` unless a separate verified owner remains.
- Keep this as the first unload-active side-effect point.

### Interfaces / Contracts

Add a local predicate helper in `miner_dock_sequence.rs`, for example:

```text
later_radio_0x16_can_send_0x15(sim, snap, ref_sid) -> bool
```

It should stay private to the miner dock sequence and use existing state rather than exposing a generic radio API.

Expected inputs:

- `Simulation`
- `MinerSnapshot`
- `ref_sid`

Expected checks:

- entity exists;
- no active movement target;
- reserved refinery equals `ref_sid`;
- contact-entered reservation state is set;
- refinery entity exists and is a building;
- current phase/state still represents MissionEnter/FaceSync rather than unload.

### Data Flow

```text
MissionEnter
  -> accepted already-there reply
  -> mark contact-entered
  -> schedule retry
  -> FaceSync

FaceSync
  -> retry not due: wait
  -> retry due + later-0x16 predicate false: wait/retry
  -> retry due + later-0x16 predicate true: MissionQueued

MissionQueued
  -> deploy mission phase

Deploy mission phase
  -> start_unload_deploy
  -> Unloading
```

### Error Handling

- If the reserved refinery disappears, keep current invalid-refinery abort behavior.
- If contact state is lost before later `0x16`, remain in the dock/retry path rather than starting unload.
- If entity movement resumes, later `0x16` must not send `0x15`.

### Testing Strategy

Replace East-facing pivot tests with predicate tests:

- First ordinary `0x16` syncs without facing or unload:
  - miner at accepted cell, non-East facing;
  - after first handoff, assert contact-entered set, no on-pad link, no display override, no deploy sound, no unload timer, no facing snap.

- Later `0x16` queues `0x15` only when all predicates hold:
  - table-driven false cases for moving, no contact-entered, missing refinery, wrong reservation, retry not due;
  - all true case advances to `MissionQueued`.

- `0x15` does not start unload side effects:
  - after `MissionQueued`, assert display override/sound/timer unchanged until deploy phase.

- Deploy start is the first unload-active side-effect point:
  - next deploy phase sets display override, sound, timer, and pad link.

- Static comment review:
  - no comments/tests claim radio `0x16` directly sets East body facing or starts unload.

## Architectural Decisions

- Keep the fix local to `sim/miner`. No render, audio, UI, or net dependency is introduced.
- Do not create a generic radio subsystem for this patch. The rest of the project already models this dock path inside the miner FSM.
- Preserve deterministic retry timing with existing `binary_frame`/RNG machinery.
- Avoid broad phase renames in the first patch. The code can keep `FaceSync` as a compatibility alias, but its comments must become source-accurate.

Tech debt:

- `Pivoting` may remain a misleading phase name if retained as a deploy-start placeholder. If retained, document it as legacy/deploy-start, not radio `0x16`.
- Exact visible dump-facing remains unproven. Do not encode an East-facing body snap as a convenience replacement.

## Alternatives Considered

### Full Radio Micro-FSM

Rename phases around `Radio16Synced`, `AwaitingRadio15`, `Mission10Queued`, and `DeployStarting`.

Rejected for this patch because it increases save/test churn without adding parity coverage beyond the narrow verified fix.

### Predicate Patch Only

Keep `sync_dock_facing` name and replace its internals with the later-`0x16` predicate.

Rejected because it leaves the bad mental model in place and makes future parity work harder to audit.
