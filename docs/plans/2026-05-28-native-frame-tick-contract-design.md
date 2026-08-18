# Native Frame / Tick Contract — Design

## Goal

Make the synthetic 15 Hz `binary_frame` counter **old-frame-visible during the
update and committed late** (mirroring `Main_Tick`'s guarded `g_CurrentFrameCounter++`),
and produce a verified classification of every `binary_frame` / `sim.tick` /
`tick_ms` consumer so the global timing shift is proven correct per system.

This is roadmap item #2 in
`docs/plans/2026-05-28-foundational-scheduler-roadmap-todo.md`.

## Scope (decided with user, 2026-05-28)

- **In:** the commit-timing fix (late-commit of `binary_frame`), the consumer
  classification (the per-system proof artifact), and acceptance tests
  (same-tick start/check, synthetic modulo-gate boundary, facing-retarget
  boundary), plus updating the existing timer tests that encode the current
  drifted behavior to the corrected behavior.
- **Out (named DRIFT, not silently cut):**
  - **45 Hz (`sim.tick`) vs 15 Hz (`binary_frame`) rate mismatch.** This is the
    frame-counter report's separate "wall-clock pace" question (the exact retail
    `g_CurrentFrameCounter` increment rate was not measured). `binary_frame`
    advances at ~15 Hz regardless of the fixed-step rate, so all current
    relative-timer consumers are rate-correct via elapsed-frame math; the open
    question is only whether native's per-frame frequency is truly 15 Hz and
    whether any *future* unconditional per-tick effect belongs on `binary_frame`
    vs `sim.tick`. Deferred to its own roadmap concern; named here so a
    consumer's clock choice is never made by accident.
  - **No production absolute-frame consumer is added.** There is no live
    `binary_frame % N` modulo gate in Rust yet (the native bridge-shroud
    `% 0x78` gate is unimplemented). The contract makes such a gate
    correct-by-default when implemented; implementing bridge shroud is separate
    system work.
  - **No reclassification of `sim.tick`-gated systems.** Candidates like the
    production retry `sim.tick % 90` are flagged in the classification but not
    converted without per-system RE proof (guardrail).

## Verification preflight (this session, live Ghidra + GREEN audit)

`FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md` re-audited
GREEN this session (logged in `AUDIT_LOG.md`). All four binary functions
verified literally:

- `Main_Tick 0x0055D360` — order is `LogicClassPerTickUpdateLiveVector()` →
  `Network_ServiceLoop()` → guarded `g_CurrentFrameCounter = g_CurrentFrameCounter + 1`
  behind exactly four flags (`DAT_00a83d49`, `DAT_00a8ecd0`, `DAT_008b41c0`,
  `DAT_00a83d48`) all `== 0`. (`decompile_function 0x0055D360`)
- `LogicClass::PerTickUpdate 0x0055AFB0` — reads pre-increment frame: scenario
  timer `g_CurrentFrameCounter - inst[0x47a] < dur`; bridge-shroud gate
  `(int)g_CurrentFrameCounter % 0x78 == 0`. (`decompile_function 0x0055AFB0`)
- `CDTimerClass::Start 0x0046B640` — `*timer = g_CurrentFrameCounter; timer[2] = duration`.
  (`decompile_function 0x0046B640`)
- `CDTimerClass::GetTimeRemaining 0x00426630` — `if timer[0] != -1: elapsed =
  frame - timer[0]; if elapsed < duration return duration - elapsed; return 0;
  else return duration`. The `<` operator and `!= -1` sentinel confirmed.
  (`decompile_function 0x00426630`)

## Architecture Context

`Simulation::advance_tick` (`src/sim/world/mod.rs`) is a phased pipeline. Three
distinct clocks flow through it:

| Clock | Rate | Commit timing today | Native analog |
|---|---|---|---|
| `binary_frame` | 15 Hz synthetic | **top** of tick (mod.rs:1227) → exposes next-frame value mid-tick (**the DRIFT**) | `g_CurrentFrameCounter` (pre-increment-visible, late increment) |
| `self.tick` | 45 Hz fixed step | **late** (mod.rs:1882, `self.tick = execute_tick`) → already pre-increment-visible | Rust scheduling clock; **not** a native frame counter |
| `tick_ms` | per-tick ms delta | passed by value | wall-clock duration (presentation) |

`binary_frame` is derived `((total_sim_ms * 15) / 1000)` after
`total_sim_ms += tick_ms` (mod.rs:1226-1227). `total_sim_ms` is read mid-tick
**only** by that derivation (verified); post-tick it is read by render
(`build_instances.rs:348`) and is in the state hash (`world_hash.rs:37`).
`binary_frame` is also in the state hash (`world_hash.rs:38`), but the hash is
computed at tick-end (mod.rs:1883), after the commit point.

### Consumer classification (the deliverable — verified by reading the code)

**All current `binary_frame` consumers are relative / stored-start CDTimers.**
None is an absolute modulo gate or fixed-frame comparison.

| Consumer | File | Pattern | Class |
|---|---|---|---|
| FacingClass set/snap/current/is_rotating | `movement/facing_class.rs:80-163` | store `start_frame=binary_frame`; `elapsed = binary_frame.saturating_sub(start)` | relative |
| Turret rotation | `movement/turret.rs:169` | `barrel.set(target, binary_frame)` (delegates to FacingClass) | relative |
| Combat fire-ready check | `combat/mod.rs:1848` | `barrel.current(bf) == desired && !is_rotating(bf)` | relative |
| Building gates | `gate_runtime.rs:63,74,87` | `elapsed = binary_frame.wrapping_sub(last_frame)` | relative |
| Miner dock sequence | `miner/miner_dock_sequence.rs:84-168` | store `start_frame`; `binary_frame.saturating_sub(start) >= duration` | relative |
| Ore growth/spread native queue | `ore_growth.rs:1044,1471` | `current.wrapping_sub(start) >= interval`; priority `= binary_frame + delay` (due-frame) | relative |
| Terrain spawn → growth queue | `terrain_spawn.rs` (`with_growth_queue`) | feeds growth-queue start-frame | relative |
| Smudge reseed → growth queue | `combat/smudge_dispatch.rs` | feeds spread-neighbor start-frame | relative |
| Render facing | `app_instances/units.rs:259`, `shp.rs:324` | `f.current(sim.binary_frame)` post-tick | post-tick read (unaffected) |

**`sim.tick` consumers** (Rust scheduling, already late-committed): command
scheduling `execute_tick = tick+1`; all movement systems; wake spawn
`self.tick & 7 == 0`; production retry `sim.tick % 90 == 0` (Rust heuristic, an
absolute modulo on `sim.tick` — flagged, not converted); combat/air/teleport
pass `self.tick`.

**`tick_ms` consumers** (wall-clock/presentation): `world_effects` animation,
radar event aging, power, particle effects.

**Why the global shift is proven correct (the guardrail's per-system proof):**
every `binary_frame` consumer captures and checks against `binary_frame`
self-consistently, so its *duration* math is invariant to a uniform offset; and
each wants its *capture* to be the pre-increment value (native `CDTimerClass::Start`
stores the pre-increment `g_CurrentFrameCounter`). The late-commit moves every
consumer's capture to pre-increment — matching native — without any per-consumer
`±1`. No absolute gate exists that would need the opposite convention.

## Impact Analysis

**Touched:**

- `src/sim/world/mod.rs` — move the `total_sim_ms += tick_ms; binary_frame = …`
  block from tick-top (1226-1227) to tick-end (beside `self.tick = execute_tick`,
  1882). Net: 2 lines relocated; comment updated.
- Tests encoding the current (drifted) capture relationship:
  - `src/sim/miner/miner_tests.rs:265-266,4019,4107` — the 67 ms-step helper
    captures `start_frame == sim.binary_frame` (post-tick). At 67 ms every tick
    crosses a boundary, so the captured start-frame becomes pre-increment
    (`post-tick − 1`). Update assertions to the corrected value.
  - `gate_runtime.rs` and `facing_class.rs` unit tests pass explicit frame
    literals to `tick_gate` / `FacingClass`, so they are independent of the
    commit point and should be unaffected; verify during implementation.
  - `world_hash.rs` `binary_frame_advances_each_66ms_block` /
    `binary_frame_drift_free_at_22ms_ticks` assert the **post-tick**
    `binary_frame` value, which Approach A preserves — should pass unchanged.
- New acceptance tests (see Testing Strategy).

**Blast radius / risk:**

1. **Replay-hash divergence on boundary ticks.** Any pre-recorded golden replay
   hash will differ once captures shift; this is the intended parity correction,
   not a regression. There is no committed golden-replay baseline in-repo that
   this breaks (state-hash tests assert *internal consistency*, not a frozen
   value) — confirm during implementation.
2. **Timing shift up to 1 frame** on the capture tick for miner dock / gate /
   facing. This is the native-correct direction (pre-increment capture).
   Player-visibility: miner dock retry/unload/deploy waits and gate
   open/close transitions can resolve one 15 Hz frame (~66 ms) earlier or later
   on the specific tick a timer is seeded — only on boundary-crossing seeds.
   Frequency: every dock cycle and gate cycle seeds a timer, so it triggers
   routinely, but the magnitude is ≤1 frame and only when the seed lands on a
   boundary tick (~1 in 3 fixed steps).
3. **Determinism** preserved: single commit point, value derived from the
   deterministic `total_sim_ms` accumulator, no `HashMap`/`HashSet` in the path.

## Chosen Approach

**Approach A — late-commit, single field.** Relocate the `total_sim_ms` advance
and `binary_frame` derivation to the end of `advance_tick`, co-located with the
existing `self.tick = execute_tick` commit. During a tick, `binary_frame` holds
the value committed at the end of the previous tick (= the pre-increment frame
`N`); the increment to `N+1` happens late, after all phase work, exactly as
`Main_Tick` increments after `Network_ServiceLoop`.

Properties:
- During-tick `binary_frame` is **constant** (single write per tick) → same-tick
  start/check yields `elapsed == 0`.
- Post-tick `binary_frame` value is **unchanged** (`= f(K·dt)`) → existing
  derivation tests pass; render and external readers unaffected.
- Only *captured* start-frames shift (to pre-increment) on boundary ticks → the
  parity correction.

Rejected alternatives:
- **B (separate `visible_frame` field):** two frame fields invite confusion,
  force every consumer to switch, don't mirror the late increment, add hash
  state.
- **C (compute-before-advance at top):** also gives pre-increment mid-tick but
  changes the *post-tick* `binary_frame` to `f((K-1)·dt)`, breaking the existing
  derivation tests and going asymmetric with how `self.tick` hashes — more
  disruptive for no parity gain over A.

## Tiny-Detail Ledger

1. `binary_frame` is constant within one `advance_tick` (single commit point);
   same-tick start/check → `elapsed == 0`. `[code: single write site; matches
   native non-advancing counter during a tick]`
2. The increment commits **late**, after all phase work — structural mirror of
   `Main_Tick`. `[GHIDRA 0x0055D360: PerTickUpdate → Network_ServiceLoop →
   guarded increment]`
3. During-tick value = previous tick's committed frame = pre-increment `N`.
   `[GHIDRA 0x0055AFB0 reads g_CurrentFrameCounter pre-increment]`
4. CDTimer semantics: `elapsed = frame − start`; `remaining = duration − elapsed`
   clamped at 0; `elapsed < duration` means still running; `start == -1` sentinel
   returns full duration. `[GHIDRA 0x00426630, `<` and `!= -1` verified]`
5. `CDTimerClass::Start` stores the current (pre-increment) frame + duration.
   `[GHIDRA 0x0046B640]`
6. Native increment is guarded by four flags (pause / load / non-advancing
   states). Rust analog: `advance_tick` is simply not called when the sim is not
   stepping, so the late commit only fires on real ticks. Assumption: Rust never
   calls `advance_tick` in a state where native would skip the increment.
   `[GHIDRA 0x0055D360 guard DAT_00a83d49/00a8ecd0/008b41c0/00a83d48]`
7. A modulo gate fires on the tick whose pre-increment frame equals the multiple
   (i.e., the tick *after* the increment), e.g. bridge-shroud `% 0x78`.
   `[GHIDRA 0x0055AFB0 `(int)g_CurrentFrameCounter % 0x78 == 0`]` — no live Rust
   gate yet; encoded only in the synthetic acceptance test.
8. First tick exposes frame 0. `[derivation: total_sim_ms=0, binary_frame=0 at
   construction]`
9. Post-tick `binary_frame` value is preserved (`= f(K·dt)`); the hashed field at
   tick-end is unchanged; only captured start-frames shift on boundary ticks.
   `[code: world_hash.rs:38; hash at mod.rs:1883 after commit]`
10. 45 Hz vs 15 Hz rate mismatch is OUT OF SCOPE, named DRIFT (separate roadmap
    concern). `[doc: FRAME_COUNTER report Inference + Remaining Uncertainty]`
11. All current `binary_frame` consumers are relative/stored-start and want
    pre-increment capture; the uniform late-commit is correct for all of them
    with no per-consumer `±1`. `[code survey, this session]`

## Design

### Components

No new types. The change is the relocation of two statements and a comment in
`Simulation::advance_tick`. The contract is documented on the `binary_frame`
field doc-comment (`mod.rs:277-279`): "pre-increment-visible during the tick,
committed late at tick end — mirrors `g_CurrentFrameCounter`. Read it as the
*current* frame for stored-start CDTimer-style consumers; never as the next
frame."

### Interfaces / Contracts

- `Simulation.binary_frame: u32` — during `advance_tick`, holds the frame `N`
  the tick is executing under. Consumers store it as a start-frame and compute
  `binary_frame.saturating_sub(start)` for elapsed; this is the canonical
  CDTimer contract.
- New private invariant (debug-only assertion candidate): `binary_frame` is not
  written between the top of `advance_tick` and the late commit.

### Data Flow

```
tick K begins ── binary_frame holds f((K-1)·dt)  (committed end of K-1)
   ├─ commands, movement, vision, power, combat, … all read frame N = f((K-1)·dt)
   ├─ timers capture start = binary_frame (= N, pre-increment)
   └─ tick K ends:
        total_sim_ms += tick_ms          # advance wall clock
        binary_frame  = f(total_sim_ms)  # late increment to N+1 (or N if no boundary)
        self.tick     = execute_tick     # existing late tick commit
        state_hash    = hash(...)        # carries post-commit values (symmetric)
```

### Error Handling

No new error types. The invariant is enforced by having a single commit point;
optionally guarded by a `debug_assert!` that no phase mutated `binary_frame`.

### Testing Strategy

Acceptance tests (the user-named criteria):

1. **Same-tick start/check → elapsed 0.** Within one `advance_tick`, a stored
   start-frame and a later check in the same tick yield `elapsed == 0`
   (guards the within-tick constancy survives the refactor).
2. **Modulo-gate boundary (synthetic).** A test-only `binary_frame % N == 0`
   gate stepped across a boundary fires on the tick whose pre-increment frame
   first equals the multiple — i.e., the tick *after* the increment — proving
   pre-increment exposure. No production gate added.
3. **Facing-retarget boundary.** A `FacingClass` retarget issued on a
   boundary-crossing tick captures the pre-increment frame; subsequent rotation
   progress matches relative elapsed.
4. **Regression — post-tick value preserved.** Existing
   `binary_frame_advances_each_66ms_block` and `binary_frame_drift_free_at_22ms_ticks`
   pass unchanged.
5. **Corrected capture (updates the drifted tests).** On a boundary tick a
   captured start-frame equals the pre-increment value (`post-tick − 1`); the
   miner-dock test expectations are updated to this.

### Determinism

Single commit point; `binary_frame` derived from the deterministic `total_sim_ms`
accumulator; no unordered collections in the path; both `binary_frame` and
`total_sim_ms` remain in the state hash. Lockstep-safe.

## Architectural Decisions

- **Follows the native authority contract** (CLAUDE.md "subsystem owner owns
  native order / late commit"): `binary_frame` is committed late beside
  `self.tick`, mirroring `Main_Tick`'s post-`Network_ServiceLoop` guarded
  increment.
- **Single source of truth** for the frame number (rejects the two-field
  Approach B).
- **Tech debt / named DRIFT:** the 45/15 rate mismatch is left unresolved
  (separate roadmap concern); the `sim.tick % 90` production retry is flagged as
  a possible mis-classified frame gate but not converted without RE proof.

## Alternatives Considered

- **B — separate `visible_frame` field:** rejected (two frame fields, no late
  increment, more hash state, forces consumer churn).
- **C — compute-before-advance at tick top:** rejected (changes the post-tick
  `binary_frame`, breaks derivation tests, asymmetric with `self.tick` hashing).
- **Folding in the 45/15 rate question:** rejected per user (separate roadmap
  concern; named DRIFT here).
- **Implementing the bridge-shroud `% 0x78` gate as the first real consumer:**
  rejected per user (separate bridge-shroud system work; contract makes it
  correct-by-default when built).
