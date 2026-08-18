# GSI-07.15 Harvest Filling-Return Gate Design

Date: 2026-07-24  
Design status: AUTONOMOUSLY_APPROVED_FOR_PLAN  
Contract:
`docs/contracts/2026-07-24-gsi-07-15-harvest-filling-return-gate-implementation-contract.md`  
Committed base inspected:
`dev` `68302b5d2d0b558400e2e0cf9b51c6994fa180c7`

## Goal

Make a standard harvester that becomes full from a positive extraction remain
visibly in Harvest until the next native-equivalent full gate at `F+19`, then
save its archive and enter Return without executing return-state refinery work
until the following miner tick.

## Autonomous Scope Decision

The operator specification explicitly authorizes autonomous design choices and
forbids routine approval pauses. This design therefore treats the approved
implementation contract as the scope answer that the generic brainstorm
workflow would otherwise request from the user.

The loop begins at a due, positive extraction that makes cargo exactly full.
It ends after the later full failure has selected archive/Return and the next
tick has crossed into existing return-state authority. A separate newly proven
GSI-07.15 drift—timer initialization on search-and-move success rather than
physical arrival—is preserved as a sibling feature. It is not a prerequisite to
the success-to-next-gate loop because this design's starting event has already
reset the native timer.

## System Card

| Field | Value |
|---|---|
| GSI / name | GSI-07.15 / `Mission_Harvest` successful-fill to later full-return gate |
| Core slug | `harvest-filling-return-gate` |
| Activation | Stock standard YR `HARV` and `CMIN`, normal ore/gem path, `Harvester=yes`, `Weeder=no`, positive extraction reaches exact storage capacity |
| Player-visible loop | filling pickup -> 19 frame-number Harvest continuation -> full gate -> harvest visual off/archive selected -> Return state -> next-tick refinery action |
| Stop condition | Production miner path proves all contract rows; no reducer, timer-schema, snapshot-version, or sibling arrival-timing change |
| Native owner | `Mission_Harvest @ 0x0073E5E0` state 1 plus `Harvest_Ore_Tick @ 0x0073D450`; StepTimer maintenance later in `TechnoClass::AI_Update @ 0x006F9E50` |
| Rust owner | `src/sim/miner/miner_system.rs::handle_harvest`; existing `Miner` state/timer/archive; existing later `handle_return` |
| Call chain | `Simulation::advance_tick` -> production economy -> `tick_miners_with_overlay_registry` -> live-order snapshots -> harvest mission seam -> `handle_harvest` -> writeback -> harvest visual synchronization |
| State readers/writers | Cargo, `MinerState`, `harvest_timer`, `last_harvest_cell`, refinery reservation, movement/teleport/dock state; Phase 4 voxel/overlay readers |
| Dependencies | Existing `MissionTimer` inclusive due semantics; existing short scan; shared reducer; existing Return handler |
| Consumers | Rendering through Harvest state, world hash through Miner fields, later refinery/dock/teleport logic |
| INI/assets | `HarvesterLoadRate` already parsed; stock default `2`; no asset changes |
| RNG | No draw in fill/full/archive branch. Existing later return may emit sound/teleport effects but is not moved earlier. |
| Integer/timer contract | Existing `harvest_tick_interval=9*load_rate`; keep `+1`; full reset uses current binary frame and duration zero |
| Lifecycle/persistence | No entity create/delete/limbo; no schema or snapshot-version change; existing serialized/hashed fields change only at corrected frames |
| Render/audio/input/net | No direct imports or writes. Harvest visual remains active through `F+18`; return audio/movement cannot start before the following state-2 tick. No input/network format change. |
| Intended feature write set | `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`, and stale cadence comments only in `src/sim/miner/mod.rs` |
| Protected overlap | None with dirty `feature/gsi-08-10-damage-authority`; do not touch its world/combat/rules/entity/snapshot/hash paths |
| Focused validation | Production miner tests for fill tick, `F+18/F+19`, archive timing, next-tick return boundary; existing miner focused suite; final `cargo check -q` |
| Native oracle | Static live disassembly and exact frame sequence; no debugger capture required for the proven ordering |

## Architecture Context

The production resource-economy rung calls
`tick_miners_with_overlay_registry`. It snapshots miners in native live-object
order, processes each snapshot in that order, mutates shared ore immediately,
writes each `Miner` snapshot back, and only then synchronizes voxel animation
and `HarvestOverlay` visibility from `MinerState::Harvest`.

`handle_harvest` currently owns too much of one branch:

1. tests the frame-anchored gate;
2. computes remaining capacity;
3. calls the shared cell reducer;
4. appends cargo;
5. if cargo is now full, immediately scans the archive and calls
   `begin_return`.

`begin_return` is not a pure state setter. It can find and reserve a refinery,
send contact, issue normal movement, begin chrono teleport with sound events,
or enter `WaitNoOre`. Calling it from the state-1 branch collapses a native
mission-dispatch boundary.

The existing architecture already has the correct Rust-native owners:

- `Miner` stores cargo, timer, state, archive, and dock/return bookkeeping.
- `handle_harvest` owns state-1 decisions.
- the shared tiberium reducer owns cell state and dirty/queue effects.
- `handle_return` owns state-2 refinery behavior on a later tick.
- post-processing derives player-visible harvest animation from the final state.

No new component, event, enum variant, or scheduler seam is needed.

## Dependency and Impact Analysis

### Direct changes

- `src/sim/miner/miner_system.rs`
  - reorder the due full check before reduction;
  - remove the newly-full transition from the positive reduction path;
  - set Return without calling `begin_return` at the later full gate.
- `src/sim/miner/miner_tests.rs`
  - strengthen the existing capacity-capping fixture;
  - add frame-boundary, archive-time, visual-state, and next-state-tick tests.

### Downstream effects to preserve

- Cargo and ore still mutate synchronously on the filling tick.
- Other miners later in live order see that reduced cell immediately.
- `MinerState::Harvest` remains hashed and rendered for 19 additional frame
  numbers.
- `last_harvest_cell` is selected from the later world state.
- Refinery reservation/movement/teleport/sound begins no earlier than the next
  `handle_return` call.
- No additional RNG draw is introduced or reordered.

### Blast radius

The code branch applies to War and Chrono Miners and to ore and gems. It does
not alter slave miner ownership, direct reducer helpers, search ranking,
movement, docking, or unload logic. Tests must cover a real valid refinery
because an assertion on state alone would miss premature reservation or chrono
movement.

### Migration

None. `MinerState`, `MissionTimer`, cargo, archive, reservation, and movement
fields already serialize and hash. Correct timing changes save/hash values at
the affected frames but requires no schema or snapshot version change.

## Tiny-Detail Ledger

- A positive standard reduction is success even if storage becomes exactly
  full. `[Ghidra 0x73D5A1..0x73D5F7]`
- Success writes counter `0`, start=current frame, and duration/repeat equal to
  `HarvesterLoadRate`; it returns low-byte `1`.
  `[Ghidra 0x73D5BE..0x73D5F7]`
- Success does not write mission substate `+0xBC`.
  `[Ghidra 0x73D450 instruction scan]`
- The state-1 caller's success jump returns `1` without archive/full logic.
  `[Ghidra 0x73E987..0x73E98E]`
- Mission dispatch precedes StepTimer maintenance in the same object AI pass.
  `[Ghidra 0x6FA655 before 0x6FABC4..0x6FAC22]`
- The timer's same-pass elapsed value is zero; the ninth stock increment occurs
  post-mission at `F+18`. `[Ghidra 0x6FABD5..0x6FAC22]`
- Mission dispatch at `F+18` sees counter `8`; `F+19` first sees `9`.
  `[derived exhaustively from the cited instruction order and stock rate 2]`
- Stock `HarvesterLoadRate` remains constructor default `2` because neither
  stock INI overrides it. `[Ghidra 0x6671CD,0x6673C7,0x670CE7..0x670D01;
  ini rules.ini/rulesmd.ini]`
- Rust's inclusive deadline needs `9*rate + 1`; removing `+1` is drift.
  `[Rust mission/timer.rs due; live sequence above]`
- At the later gate, full percentage is checked before the reducer.
  `[Ghidra 0x73D4B6..0x73D4BC]`
- Full failure resets counter/rate fields and returns false without touching
  cargo or cell. `[Ghidra 0x73D5FE..0x73D626]`
- The false caller clears active harvesting and writes return substate 2 before
  the archive scan. `[Ghidra 0x73E98E..0x73EA09]`
- Archive selection is made at the later full gate, so intervening resource
  changes are visible. `[Ghidra 0x73EA09..0x73EA7B]`
- State-2 refinery behavior is not recursively executed from state 1; the
  mission returns `1`. `[Ghidra 0x73E9D0..0x73EA7B]`
- Rust Phase 4 derives voxel and OREGATH visibility from `Harvest`; keeping the
  state through `F+18` preserves the visible native continuation.
  `[Rust miner_system.rs post-processing]`
- The short archive scan is deterministic and consumes no RNG; no draw is
  added by the design. `[Rust save_archive_via_short_scan/search_local_ore]`
- Shared ore mutation remains immediate during the filling call and before
  later live-order miners. `[Rust tick_miners_in_order]`
- Full-gate timer reset maps to existing `MissionTimer::reset(now)`, yielding
  start=current and duration zero. It does not certify every unrepresented
  native StepTimer byte outside this bounded branch.
  `[contract scope; Rust mission/timer.rs]`
- No `begin_return` side effect—reservation, radio, movement, teleport, sound,
  dock state, or `WaitNoOre`—may occur on either the filling tick or the later
  state-1 full tick. `[native mission boundary; Rust begin_return]`

## Approaches Considered

### Approach A — Reorder existing state-1 authority (chosen)

How it works:

- After the due check, branch on already-full cargo before computing/requesting
  reduction.
- In that branch, reset the harvest timer, set `ReturnToRefinery`, run the
  archive scan, and return.
- For every positive reduction, append cargo, re-arm the existing
  `interval + 1` gate, and return without a fullness transition.
- Let the next tick's existing state dispatch call `handle_return`.

Architectural fit:

- Uses existing state, timer, archive, reducer, and return owner.
- Preserves the current live-order snapshot/commit pattern.
- Adds no new state representation or cross-module dependency.

Tiny-detail coverage:

- The positive branch owns success/no-substate-change/re-arm.
- The existing deadline owns exact `F+19`.
- The new pre-reducer full branch owns helper ordering, reset, archive, and
  state-2 selection.
- The outer per-tick state match owns the next-dispatch boundary.
- Existing post-processing owns visual timing.

Trade-offs:

- Smallest diff and smallest ownership surface.
- The code continues to compress native StepTimer internals into an equivalent
  deadline for this proven branch. It does not claim generic timer parity.

Risk:

- An implementation could accidentally call `begin_return` or omit timer reset.
- A weak test could pass without a refinery and miss premature side effects.
- A helper refactor could accidentally affect the no-bale, not-full branch.

Parity verdict: exact for every scoped ledger item; no known scoped drift.

### Approach B — Add a `pending_full_return` latch

How it works:

- Set a new boolean when a positive extraction fills cargo.
- Keep a timer, then consume the latch after 19 frames to archive and return.

Architectural fit:

- Adds duplicate state because `cargo.is_full()` plus the existing timer already
  encode the native condition.
- Requires serialization/hash/default/migration consideration.

Tiny-detail coverage:

- Can delay transition but invents a byte native does not have.
- Risks using the latch rather than performing the native later full check.
- World/cargo changes between fill and gate become ambiguous.

Trade-offs:

- Makes intent visually explicit in Rust, but creates unnecessary coupled
  authority and broader validation.

Parity verdict: DRIFT. It replaces the verified later helper/full predicate with
an invented pending state.

### Approach C — Enter a delayed-return state immediately on fill

How it works:

- Change state on the filling tick but defer `begin_return` through a countdown
  or delayed state.

Architectural fit:

- Introduces a new mission/state shape and moves harvest visuals away from the
  existing state owner.

Tiny-detail coverage:

- Fails success's no-substate-write requirement.
- Turns off harvest visuals early.
- Chooses or carries return intent without the native later full helper call.
- Can choose archive at the wrong time.

Trade-offs:

- Superficially isolates the delay, but encodes the observed outcome rather than
  the native mechanism.

Parity verdict: DRIFT and rejected.

## Chosen Approach

Choose Approach A. The current architecture already contains the native
behavioral owners; the bug is branch ordering and a collapsed mission boundary.
Reordering those owners closes the gap without a new state, duplicated timer,
schema change, or broad refactor.

## Design

### Components

`handle_harvest` remains the only production code component changed.

Conceptual order:

```text
if harvest gate pending:
    return

if cargo already full:
    reset harvest timer at now
    set state ReturnToRefinery
    save archive from current world state
    return

reduce current cell by remaining capacity

if removed > 0:
    append typed cargo
    arm next gate for harvest_tick_interval + 1
    return

existing empty/not-full continuation behavior
```

The full branch deliberately does not call `begin_return`.

### Interfaces and Contracts

No public interface changes.

- `MissionTimer::reset(now)` supplies the bounded failure reset.
- The Return state write precedes `save_archive_via_short_scan`, matching the
  verified native caller order; the scan does not recursively dispatch state 2.
- `save_archive_via_short_scan` remains the archive owner.
- `reduce_tiberium_at_with_native_context` is called only when capacity is
  nonzero.
- `handle_return` remains reachable only through the next top-level miner state
  dispatch.

### Data Flow

Filling frame `F`:

```text
due -> reducer -> cargo exactly full -> timer arm(F, 19) -> Harvest writeback
    -> voxel/OREGATH remains active
```

Frames `F+1..F+18`:

```text
gate pending -> no cargo/cell/archive/return mutation -> Harvest remains active
```

Frame `F+19`:

```text
gate due -> full precheck -> timer reset(F+19)
    -> Return state write -> archive scan using F+19 world state
    -> voxel/OREGATH becomes inactive
```

Frame `F+20`:

```text
top-level state dispatch -> existing handle_return
    -> deterministic far-refinery fixture reserves and issues existing return
       movement/teleport as already implemented
```

### Error Handling

No new fallible API. If archive scan finds no resource, it stores `None` as it
does today. If no refinery exists, the following `handle_return` tick retains
its existing behavior. The state-1 full gate must not preemptively translate
that condition to `WaitNoOre`.

### Determinism

- No iteration order changes.
- No new collection.
- No RNG draw.
- Shared ore still commits at the same filling point in live order.
- Archive selection moves to its verified later point and uses existing
  deterministic scan/tie order.
- Existing hashed fields change at the native-equivalent frames; no hash format
  changes.

### Testing Strategy

1. Strengthen the existing remaining-capacity test for both War and Chrono
   miners to assert: `Harvest`, duration 19, no
   archive/reservation/movement/teleport/dock/sound change.
2. Advance the production miner path and assert pending at `F+18`, transition at
   `F+19`, with cargo and cell unchanged after the fill.
3. Mutate resource candidates during the interval and prove archive selection
   uses `F+19` state.
4. Spawn a deterministic far, reachable refinery and prove the `F+19` state-1
   call produces no reservation/movement/teleport/sound; assert the `F+20` HARV
   tick reserves it and issues return movement. A far-CMIN variant pins that its
   teleport/sound cannot start at `F+19` and starts through existing return
   authority at `F+20`.
5. Explicitly attach both `VoxelAnimation` and `HarvestOverlay`; assert both
   remain active until the `F+19` transition and reset there. Do not rely on
   `spawn_miner`, which leaves these optional components absent.
6. Run the focused miner suite serially, then final `cargo check -q`.

The tests use the production `tick_miners` path, not the test-only duplicate
reducer helper.

## Architectural Decisions

- Preserve the existing Rust-native `Miner` owner instead of recreating native
  class layout.
- Preserve native mission boundaries through the existing per-tick state match.
- Use the current frame-anchored timer because its exact scoped branch timing is
  now positively proven; do not generalize this design into a parity
  certification for all StepTimer state.
- Avoid a new pending state because cargo fullness and the timer already form
  the native predicate inputs.
- Keep the sibling acquisition/arrival timer drift separate because its
  starting boundary precedes this contract's successful reset and requires a
  larger movement/state design. It remains an explicit DRIFT, not a waived gap.

## Alternatives Considered

- A serialized pending-full latch was rejected as duplicate authority and a
  different mechanism.
- An immediate delayed-return state was rejected because it changes state,
  visuals, and archive timing on the successful filling tick.

## Approval Gate

This draft requires:

1. independent adversarial review of binary timing, Rust branch ordering, and
   the asserted non-dependency on the acquisition/arrival sibling;
2. repair of every load-bearing finding;
3. an autonomous approval record stating why the design is justified and what
   evidence could still invalidate it.

No Rust implementation may begin before that gate and the subsequent reviewed
implementation plan are complete.
