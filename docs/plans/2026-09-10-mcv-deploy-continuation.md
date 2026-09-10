# MCV deployment continuation

Status: implemented and independently reviewed; normal-game visual confirmation pending.
Baseline: `c7432d8c`, branch `feature/mcv-deploy-continuation`.

## Outcome

One accepted deployment order must carry an ordinary MCV through any required
stop and turn into its construction yard, without a second press. Preserve the
native target building's `DeployFacing`, turn progression, mission cadence and
interruption behavior. Cover AMCV, SMCV and PCV through the production command and
tick path. This is a deployment-continuation fix, not certification of the entire
placement, redeployment, transport or slave-miner system.

## Evidence and present failure

Live `gamemd.exe` (SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`):

- `UnitClass__Deploy` at `0x007393C0` reads the target building's facing through
  `0x00465D70`. In `0x007395D2..0x007395EB`, current 16-bit facing is rounded to
  an 8-bit direction, rather than simply taking its upper byte.
- On facing mismatch, `0x007395EF..0x0073965F` checks locomotor movement, requests
  a turn through locomotor slot `+0x4C` when permitted, sets runtime unit field
  `+0x68C` at `0x00739650`, and returns in progress without creating a building.
- `UnitClass__Mission_Unload` at `0x0073D630` calls Deploy at `0x0073DDC6` after
  its movement wait. The check at `0x0073DDCB` is object liveness, not Deploy's
  return value. Runtime `+0x68C` then selects state 2 (`0x0073DDD5..0x0073DDDF`),
  whose `0x0073DD5D` call retries automatically. A failed retry can clear the
  flag when NavCom is non-null (`0x0073DD76`).

These statements were checked against live decompilation/disassembly and the
three continuation sites annotated in the saved Ghidra program. Older research
claiming ordinary MCVs cannot reach state 2 because `DeployToFire` defaults false
is contradicted by the runtime writer above; do not use that claim as a premise.

At the baseline, [`world_spawn.rs`](../../src/sim/world/world_spawn.rs), `deploy_mcv`,
snaps `entity.facing`, sets `facing_target`, clears `movement_target`, then returns.
Its production caller in [`world_commands.rs`](../../src/sim/world/world_commands.rs)
does not retain a deployment mission. The test
`deploy_mcv_waits_for_target_building_deploy_facing` in
[`deploy_tests.rs`](../../src/sim/deploy_tests.rs) checks only this first call,
and therefore accepts the missing continuation.

## Implementation sequence

1. **Close the native handoff contract before changing behavior.** Trace the
   active player deployment event into destination clearing, mission queueing,
   both Unit AI commencement gates, Unload states 0/1/2 and the locomotor turn.
   Record exact delay returns, RNG draws, order of movement versus dispatch,
   field initialization/reset, duplicate orders, and replacement by Move/Stop.
   Check stationary, moving and already-turning MCVs. Existing historical traces
   are pointers, not proof of current Rust gaps. Repair their consequential false
   state-2 claim where encountered, without producing another standalone report.
2. **Make the existing mission authority own continuation.** Route accepted MCV
   deployment commands through the native-equivalent stop/queue handoff. Add the
   ordinary DeploysInto branch to the Unit Unload dispatch in
   [`mission_handlers.rs`](../../src/sim/world/techno_ai/mission_handlers.rs), with
   native branch precedence. Use `MissionCom` handler state and a narrowly owned
   runtime field only if the traced flag cannot be represented by existing state.
   Do not reuse infantry `DeployPhase`, dead Guard deploy latches, or an app timer.
   Preserve the specialized slave-miner command route and transport unload branch.
3. **Separate a deployment attempt from order acceptance.** Keep the existing
   construction-yard creation/transfer transaction coherent, but let the mission
   invoke it. Represent blocked, turning/in-progress and converted outcomes
   explicitly enough to avoid confusing object survival with a boolean success.
   Recheck placement when native does; do not cache permission at command time.
   Preserve native failed-deploy fallback and feedback frequency rather than
   leaving an unconditional retry loop. Repeated AI/player orders must follow the
   traced queue semantics, without accidentally restarting a turn every tick.
4. **Drive the actual stop and turn.** Reuse the navigation/locomotor authorities
   in [`navcom.rs`](../../src/sim/movement/navcom.rs) and `FacingClass`; retain a
   committed track endpoint when native Stop_Moving does. The current rotation
   helper is behind the `movement_target` gate in `movement_tick.rs`, so setting
   `facing_target` alone cannot turn a stationary unit. Start the authoritative
   `body_facing` interpolator through an appropriate locomotor turn operation and
   retain the existing stationary post-unit facing mirror; do not fabricate a
   movement destination merely to enter the movement helper.
   Sample native rounded facing for the deploy comparison, and ensure post-unit
   facing updates do not overwrite/restart the turn. In particular,
   `transport_unload::refresh_idle_hull_turn` currently runs from the Unit host
   for any Unload state 1 with no movement target, without a transport-type gate.
   Make its applicability explicit or move the shared facing read to a common
   owner; MCV and transport state numbers must not accidentally share semantics.
   Honor rules ROT and native
   zero-ROT behavior established in step 1.
5. **Close lifecycle and deterministic state.** Trace cancellation, death,
   limbo/removal, successful replacement, save/load and replay hashing through
   existing owners. Add persistence/hash support for any new authoritative field.
   Update direct-call tests and consumers to respect command acceptance versus
   conversion on mission dispatch; include AI's repeated deployment producer.
6. **Publish conversion effects on the conversion tick.** Command execution
   currently marks `spawned_entities` synchronously for DeployMcv. Carry an
   actual structural-change/spawn result out of the mission/object-AI path into
   `advance_tick` and `TickResult` when the yard is created, rather than treating
   accepted intent as creation. Preserve the consumers in world navigation
   finalization and `app/match_runtime/sim_tick.rs` atlas refresh, including
   removal of the MCV and visibility of the new yard. Reuse existing tick effect
   aggregation where possible; app code must not infer conversion by polling.

The global readiness adapter currently omits signed height because existing Move
orders start movement before commencement (see
[`authority.rs`](../../src/sim/mission/authority.rs)). Do not enable that global
gate casually. Step 1 must establish whether the existing gate plus the native
Unload movement wait can implement this handoff faithfully. If it cannot,
promote the required command/commencement prerequisite into this change and revise
the plan before implementation; do not bypass readiness with a second scheduler.

## Validation and completion gate

- Save a reproducible native oracle under `tools/` using the existing
  [`native_oracle.py`](../../tools/native_oracle.py) harness where feasible.
  Exercise original executable branches for turn request, pending state, retry,
  aligned conversion boundary and failure exit. Capture binary identity, entry
  state, helper substitutions, visited addresses, return/state outputs and timing
  limitations. Rust expectations come from native outputs, not hand-built parity
  goldens. A mocked conversion helper proves control flow only, not placement.
- Production regressions must issue one `Command::DeployMcv` and advance normal
  ticks: all three stock MCV types; already aligned and multiple other headings;
  moving, mid-turn and custom DeployFacing/ROT; no manual second call. Assert
  intermediate motion/state and exactly one yard at the proper final cell.
  Assert the conversion-tick effect reaches navigation and app resource refresh,
  including a yard whose sprite resources were not already loaded; check the
  new structure blocks paths on the correct tick.
- Exercise blocked initial/final placement, Move/Stop during the wait, repeated
  orders, destruction/removal, and save/load during the pending turn. Assert no
  late yard, stale intent, duplicate feedback or changed replay hash sequence
  versus uninterrupted execution where equivalence is expected. Preserve current
  conversion ownership/selection/production behavior and transport, infantry and
  slave-miner regressions. Derive expected interruption outcomes from step 1.
- Compare mission dispatch frames, delay/RNG progression and rounded facing at
  boundary values against the bounded native oracle. Then run focused tests,
  one full `cargo test -p vera20k --lib`, and `cargo clippy -p vera20k --lib`, after
  checking for competing builds and fresh-worktree assets/config.
- Build and launch the normal release game from this branch for a visible
  one-press test from several headings and while moving. Record what actually
  ran; unit tests alone do not certify visual turning. Keep native/script
  references near implementation and tests. Obtain a fresh implementation critic.

Native handoff investigation resolved the critical questions. Drive turn completion
(`0x004B077B..0x004B08A4`) retains a previous-rotation latch and calls
PerCellProcess(0), whose pending-flag branch retries Deploy without waiting for
mission cadence. Track completion calls reason 2 before FootStop clears NavCom.
Move and ordinary-MCV Stop do not blanket-clear pending; Stop retains the current
mission. Tests must reflect these outcomes, rather than require universal cancellation.
The oracle explicitly separates interior mission branch results from complete
Drive/FacingClass execution. Native cadence/RNG is established by disassembly,
not claimed covered by that bounded executable oracle.

Implementation is authorized; publication/merge is not part of this request.

Validation on 2026-09-10: all 13 MCV regression tests pass, including retail
AMCV/SMCV/PCV rules, normal command/tick movement and conversion, replacement
orders, placement recheck, navigation receipt, native result/facing comparisons,
and snapshot restoration. Full library suite: 8,607 passed, 7 unrelated existing
macOS/path failures, 120 ignored. Clippy and release build pass with existing
warnings. Native oracle `--check` passes. The independent implementation critic
rechecked fixes and found no remaining blocking issue in the reviewed path.
The broader Unit Hunt override and general Move command/readiness adapter remain
existing adjacent gaps; this change does not certify those systems.
