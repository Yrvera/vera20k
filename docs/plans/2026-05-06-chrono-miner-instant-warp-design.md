# Chrono Miner Instant-Warp Wiring — Design

## Goal

Match the binary's `InitiateWarp` harvester special case: when a unit with `Harvester=yes` self-teleports, skip the chrono delay entirely and clean up the teleport state in one tick.

## Architecture Context

The teleport pipeline lives in [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs):

- **`issue_teleport_command(entities, entity_id, target, rules)`** — entry point. Computes `chrono_ticks` via `compute_chrono_delay(rules, distance_leptons)`, attaches `TeleportState { phase: Relocate, target_rx, target_ry, being_warped_ticks: chrono_ticks }`, optionally applies a piggyback override (Drive → Teleport).
- **`tick_teleport_movement(entities, occupancy, tick_ms, sim_tick)`** — two-phase state machine. `Relocate` snaps position + occupancy and transitions to `ChronoDelay`. `ChronoDelay` decrements `being_warped_ticks` per tick; on zero, pushes the entity to a `finished` list. After the loop, `finished` entries get `teleport_state = None` and the override is `end_override()`'d.
- **`compute_chrono_delay(rules, distance_leptons) -> u32`** — pure function. Distance-based formula clamped to `chrono_minimum_delay`. No knowledge of the calling unit.

Call sites for `issue_teleport_command`:

- [src/sim/world/world_commands.rs:158](../../src/sim/world/world_commands.rs#L158) — player-issued move on a `Teleporter=yes` unit. Local `MoveInfo` already carries `is_harvester: bool`.
- [src/sim/miner/miner_system.rs:476](../../src/sim/miner/miner_system.rs#L476), [610](../../src/sim/miner/miner_system.rs#L610), [710](../../src/sim/miner/miner_system.rs#L710) — chrono miner returns. Caller is in a code path gated on `MinerKind::Chrono`, so `is_harvester=true` always.

Render gating: [src/app_instances/units.rs:145](../../src/app_instances/units.rs#L145) reads `entity.teleport_state.is_some_and(|t| t.being_warped_ticks > 0)` to apply 50% alpha. With `being_warped_ticks=0` from the start, this branch is never taken for harvesters — but render is fixed independently as part of the broader chrono-warp parity bundle (not in this design's scope).

## Impact Analysis

**Touched files:**

- `src/sim/movement/teleport_movement.rs` — add `is_harvester: bool` parameter to `issue_teleport_command`; add a one-arm branch in `tick_teleport_movement::Relocate` for `being_warped_ticks == 0`.
- `src/sim/miner/miner_system.rs` — three call sites pass `true`.
- `src/sim/world/world_commands.rs` — one call site passes `info.is_harvester`.

**Depends on:** nothing new. `is_harvester` flag exists on `ObjectType` already and flows through `MoveInfo` (player path) and is implicit at the miner-system call sites.

**Risk areas:**

- Tests in [src/sim/movement/teleport_movement.rs:236-507](../../src/sim/movement/teleport_movement.rs#L236) call `issue_teleport_command` with various unit types. The signature change requires updating every call. Existing tests use `make_drive_obj()` (CMIN type but `harvester: false` in the test fixture) and CLEG; behavior should remain unchanged when `is_harvester=false` is passed.
- The chrono-miner dock-flow tests in [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) may assert specific tick counts. With `being_warped_ticks=0` and the Relocate-skips-ChronoDelay branch, the cleanup tick shifts from "tick 2" to "tick 1" of teleport. One-line updates likely needed in any test that counts ticks past Relocate.
- Snapshot/replay determinism: `TeleportState` struct unchanged. Deterministic value (`0`) substituted in for harvesters at issue time. State hash unaffected.
- Sim/ → render decoupling preserved (no new dependency from teleport_movement.rs).

**Migration:** none. No INI changes, no save format changes, no public API consumers outside the call sites listed.

## Chosen Approach

**Approach B from brainstorm:** `is_harvester` parameter on `issue_teleport_command` short-circuits `compute_chrono_delay` to 0; in `tick_teleport_movement::Relocate`, transition directly to cleanup (skip `ChronoDelay`) when `being_warped_ticks == 0`.

The extra branch in `Relocate` matches the binary's 1-tick effective behavior. Without it, harvesters would still take 2 ticks for cleanup (Relocate → ChronoDelay-with-zero-countdown → cleanup) — which is invisible in normal play but is exactly the kind of small timing detail the parity bar exists to preserve.

Approach A (parameter only, no Relocate branch) was rejected because it locks in a 1-tick parity drift for no real complexity savings: the Relocate branch is three lines and uses the existing `finished` list mechanism.

Approach C (lookup `is_harvester` inside `issue_teleport_command` via `RuleSet + Interner`) was rejected as an anti-pattern: it would force `teleport_movement.rs` to take a `RuleSet` and `Interner`, breaking the module's current independence from rules-data and setting a precedent the other special-locomotor modules (`tunnel_movement`, `air_movement`) don't follow.

## Tiny-Detail Ledger

Each item must be preserved by the implementation. Source citations in brackets.

1. **Trigger condition** — `WhatAmI() == 1 && type+0xE0E != 0`. [GHIDRA 0x00719400, verified 2026-05-06] In our model: the calling site asserts the unit's `ObjectType.harvester == true`. Since YR's `Harvester=yes` only meaningfully appears on UnitClass (vehicle) instances per `UnitTypeClass::ReadINI` at 0x007476A6, the `WhatAmI()==1` part is implicit in the harvester flag's domain.

2. **Chrono lock effect** — `timer.duration = 0`. [GHIDRA 0x00719400] In our model: `being_warped_ticks = 0` set at issue time.

3. **BeingWarped flag effect** — `+0x271 = 0` set in same call frame as `+0x271 = 1`. [GHIDRA 0x00719400] Doesn't affect rendering (verified: `TechnoClass::Draw` at 0x706640 doesn't read +0x271, nor does any subclass via the `vtable+0x43c` virtual). Equivalent in our model: `teleport_state` should be cleared as fast as possible since it's the gate that keeps `handle_return` and other miner-FSM consumers idle. Approach B clears it on the same tick as Relocate.

4. **Other side-effects unchanged** — WarpOut anim still spawns at depart and arrive, ChronoIn/OutSound still plays, position still updates, `IsOnBridge` flag still set from destination cell. [GHIDRA 0x00719400] These run regardless of the harvester branch in the binary. In our model, all of this happens via the existing `Relocate` phase + `spawn_warp_effects` call in `miner_system::begin_return` — not affected by this design.

5. **Tick-count from issue → fully cleaned up** — binary: 1 tick (StateMachineTick + InitiateWarp run in one tick). [GHIDRA 0x00719400 + 0x007192F0, structural inference] In our model with Approach B: 1 tick (Relocate snaps position + finishes immediately when `being_warped_ticks == 0`). For non-harvesters: unchanged (Relocate → ChronoDelay countdown → cleanup, distance-dependent).

6. **Override teardown** — locomotor `end_override()` runs at cleanup, restoring base locomotor (Drive for chrono miners). [our existing `tick_teleport_movement` cleanup loop, lines 222-233] Unchanged by this design — the cleanup loop is unchanged; only the path that pushes onto `finished` differs.

7. **Phase semantics for non-harvesters** — `Relocate` always transitions to `ChronoDelay` for delay > 0. [our existing state machine] Approach B preserves this — the new branch only fires when `being_warped_ticks == 0`.

## Design

### Components

No new components. No new structs. No new fields.

### Interfaces / Contracts

**Modified:**

```rust
// teleport_movement.rs
pub fn issue_teleport_command(
    entities: &mut EntityStore,
    entity_id: u64,
    target: (u16, u16),
    rules: &GeneralRules,
    is_harvester: bool,    // NEW
) -> bool;
```

Semantics: when `is_harvester == true`, the chrono delay is forced to 0 and the resulting `TeleportState` cleans up in a single tick (Relocate phase). When `false`, behavior is unchanged from today.

The function does not look up the harvester flag itself — that's the caller's responsibility, matching how `tunnel_movement::issue_tunnel_move_command` already takes a `tunnel_speed` from caller context rather than re-deriving it. Pattern is consistent with the rest of the special-locomotor modules.

### Data Flow

**Issue:**

```
issue_teleport_command(..., is_harvester=true)
  ├─ chrono_ticks = 0  (skip compute_chrono_delay)
  ├─ entity.locomotor.begin_override(Teleport)  if base != Teleport
  └─ entity.teleport_state = Some(TeleportState {
       phase: Relocate,
       being_warped_ticks: 0,
       ...
     })
```

**Tick (Approach B branch):**

```
tick_teleport_movement():
  for each entity with teleport_state:
    match phase:
      Relocate:
        update position + occupancy
        if being_warped_ticks == 0:
          finished.push(id)               # NEW: skip ChronoDelay
        else:
          phase = ChronoDelay
      ChronoDelay:
        decrement being_warped_ticks
        if 0: finished.push(id)

  for id in finished:
    entity.teleport_state = None
    entity.locomotor.end_override()
```

For chrono miners: 1 tick from issue to fully cleaned up.
For chrono legionnaires: unchanged — Relocate → ChronoDelay → countdown → cleanup.

### Error Handling

No new error paths. The `is_harvester=true` path bypasses `compute_chrono_delay` entirely, so any defensive logic in that function (clamps, divide-by-zero guards) is moot for the harvester case but unchanged for non-harvester.

### Testing Strategy

**Existing tests to update:**

- [teleport_movement.rs:test_teleport_issues_and_completes](../../src/sim/movement/teleport_movement.rs#L412) — uses CLEG (infantry, `harvester: false`). Pass `false` for the new parameter. Behavior assertions unchanged.
- [teleport_movement.rs:test_teleport_with_piggyback_restores_drive](../../src/sim/movement/teleport_movement.rs#L460) — uses `make_drive_obj()` (CMIN type but `harvester: false` in fixture). Pass `false`. Loops 200 ticks until restore — behavior unchanged.
- [teleport_movement.rs:test_chrono_delay_formula](../../src/sim/movement/teleport_movement.rs#L489) — tests `compute_chrono_delay` directly. Unchanged (function signature not modified).
- Any chrono-miner-specific tests in [src/sim/miner/miner_tests.rs](../../src/sim/miner/miner_tests.rs) that count ticks across the teleport phase will need a one-line update.

**New tests in [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs):**

1. **`test_harvester_skips_chrono_delay`** — issue teleport with `is_harvester=true` and a long distance (would normally yield ~100 ticks delay). Assert `being_warped_ticks == 0` immediately after issue.
2. **`test_harvester_relocate_cleans_up_in_one_tick`** — issue teleport with `is_harvester=true`. Run one tick. Assert position is at target AND `teleport_state` is `None` AND if a piggyback override was set, it's been ended.
3. **`test_non_harvester_uses_full_delay`** — regression: issue teleport with `is_harvester=false`. Assert `being_warped_ticks > 0` initially and the entity goes through `ChronoDelay` phase normally.

### Determinism

`is_harvester` is a deterministic property of the unit's `ObjectType` (parsed once from rulesmd.ini at game start). No RNG, no per-tick variance. State hash includes `teleport_state` whose value is deterministic for both branches.

The new `Relocate` branch is a pure conditional on `being_warped_ticks` — both arms are deterministic, no ordering dependencies introduced.

## Architectural Decisions

**Patterns followed:**

- Caller-provides-context pattern (matching `tunnel_movement::issue_tunnel_move_command(..., tunnel_speed: SimFixed)` and `parachute_descent::tick_parachute_descent(..., max_fall_rate)` — special-locomotor entry points take their context as parameters rather than re-deriving from rules).
- Existing two-phase state machine in `tick_teleport_movement` extended, not replaced.
- `finished` list cleanup pattern unchanged.

**Patterns deviated from:** none.

**Tech debt:** none introduced. The `is_harvester` parameter is a slight API growth, but it's documented and only takes effect when `true`.

## Alternatives Considered

**Approach A (parameter only, no Relocate branch):** rejected. The 1-tick parity drift is small but committed-to-forever, with no complexity savings since the Relocate branch is three lines and reuses existing infrastructure.

**Approach C (lookup `is_harvester` inside `issue_teleport_command`):** rejected. Forces `teleport_movement.rs` to depend on `RuleSet` and `Interner`, which it currently does not. Anti-pattern: hidden coupling. Would also set a precedent the other special-locomotor modules (`tunnel_movement`, `air_movement`, `droppod_movement`, `rocket_movement`) don't follow.

**Inverse condition (`is_chrono_legionnaire: bool` to opt-IN to delay):** rejected. The binary special-cases harvesters specifically (`type+0xE0E`), not legionnaires. Inverting the gate would make our code structurally diverge from the binary's logic for no reason.

## Definition of Done

- [ ] `issue_teleport_command` signature includes `is_harvester: bool`
- [ ] `tick_teleport_movement::Relocate` arm handles `being_warped_ticks == 0` by pushing to `finished`
- [ ] All four call sites updated (1 in `world_commands.rs`, 3 in `miner_system.rs`)
- [ ] Three new tests pass; existing teleport tests pass with the new parameter
- [ ] Existing miner-flow tests pass (with one-line updates if any encoded the 2-tick teardown)
- [ ] `cargo clippy` clean for changed files
- [ ] Manual verification: load a YR skirmish, build a chrono miner, watch its return-to-refinery cycle. Confirm: no translucency phase visible, miner appears at dock cell and immediately drives in (no ~1-second pause)
