# Skirmish Short Game Defeat Design

## Goal

Make Rust's deterministic sim defeat condition match standard YR Skirmish Short Game behavior for last-building loss.

## Architecture Context

`sim/` owns deterministic game state and gameplay behavior. Per `AGENTS.md`, it must not depend on render, UI, sidebar, audio, or net layers. Defeat state currently lives in `HouseState` and is advanced by `Simulation::check_defeat` in `src/sim/world/mod.rs`.

`GameOptions.short_game` already exists as per-match deterministic state. It is set from skirmish launch options, stored on `Simulation.game_options`, and hashed in `src/sim/world/world_hash.rs`. The missing piece is that `check_defeat` ignores this option and only defeats houses when `owned_building_count + owned_unit_count == 0`.

Current flow:

1. Entity spawn/despawn updates `HouseState.owned_building_count` and `owned_unit_count`.
2. `Simulation::advance_tick` calls `check_defeat` near the end of a tick, gated behind `self.tick > 0`.
3. `check_defeat` marks houses defeated, then computes surviving houses and sets `has_won` when one house remains or all remaining houses are allied.

## Impact Analysis

Primary touched file:

- `src/sim/world/mod.rs` - `Simulation::check_defeat`.

Likely test file:

- A focused sim test module near existing world tests, or a new narrow test in the world test suite if that matches local organization.

State/hash impact:

- No new state is required. `GameOptions.short_game`, `HouseState.is_defeated`, `has_won`, `owned_building_count`, and `owned_unit_count` are already hash inputs.
- Determinism risk is low because the condition is a pure function of existing deterministic state.

Tick-order impact:

- Keep `check_defeat` in its current tick position. This design changes the predicate only; it does not move defeat evaluation earlier or later.

Behavioral risk:

- Victory resolution can change sooner when Short Game is enabled because a house with remaining units can now be removed from the alive set once its last building is gone. This is intended parity.
- Local-player defeat aftermath such as map reveal, sidebar disable, EVA, messages, and radar/input changes is out of scope for this sim-condition design.

## Chosen Approach

Use a minimal predicate change inside `check_defeat`.

When `self.game_options.short_game` is true, mark a non-defeated house defeated when its `owned_building_count == 0`.

When `self.game_options.short_game` is false, preserve the existing long-game behavior: mark defeated only when `owned_building_count + owned_unit_count == 0`.

This matches the verified player-visible gap for standard YR Skirmish: with Short Game enabled, a player loses after the last building is gone even if ordinary units remain.

## Tiny-Detail Ledger

- Short Game is the skirmish checkbox `0x54E`; Start Game packs it as `DAT_00A8B262 = (BM_GETCHECK == 1)`. Rust equivalent is `GameOptions.short_game`. Source: `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- Stock YR default is `ShortGame=yes`. Source: `ini/rulesmd.ini:3039`.
- The branch is active in standard offline Skirmish, not TS legacy. Source: `SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`.
- Defeat checking applies after play has begun. YR requires current frame `> 0`; Rust already calls `check_defeat` only when `self.tick > 0`. Source: `SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`; `src/sim/world/mod.rs`.
- With Short Game on, ordinary non-building units do not keep a player alive after the last building is gone. Concrete verified scenario: `OwnedBuildings=0`, ordinary unit count `1`, counted ConYard-style instances `0` -> YR defeats; Rust currently does not. Source: `SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`.
- With Short Game off, YR uses the longer defeat condition: no buildings plus no owned object totals. Source: `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`.
- Existing Rust `owned_building_count` and `owned_unit_count` are updated on spawn/despawn through deterministic house counters. Source: `src/sim/world/mod.rs`, `src/sim/house_state.rs`.
- The exact YR "counted ConYard-style instances" term in the Short Game branch is not fully represented in current Rust evidence. This design intentionally handles the verified common output, `last building gone while ordinary units remain`, and leaves exact counted-instance modeling as a documented follow-up rather than guessing. Source: `SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`, `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`.
- YR calls `ScatterAllUnits` and then `MPlayer_Defeated` when the condition is true. This design scopes only the sim defeat condition (`is_defeated` and consequent win resolution); player-facing aftermath is deferred. Source: `SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`, `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`.

## Design

### Components

`Simulation::check_defeat`

- Add a tiny predicate inside the house loop:
  - `short_game == true`: defeat when `owned_building_count == 0`.
  - `short_game == false`: defeat when `owned_building_count + owned_unit_count == 0`.
- Keep the existing `house.is_defeated` skip.
- Keep the existing alive-house collection and win/allied resolution logic after defeat marking.

No new module, component, or persistent state is needed.

### Interfaces / Contracts

No public API changes are required.

The implicit contract for `check_defeat` becomes:

- It reads `self.game_options.short_game`.
- It only mutates `HouseState.is_defeated` and existing victory flags as current code already does.
- It remains deterministic and independent of UI/app/render state.

### Data Flow

Existing flow remains:

1. Skirmish launch writes `SkirmishLaunchOptions.short_game`.
2. `SkirmishLaunchOptions::to_game_options` copies it into `GameOptions.short_game`.
3. `apply_skirmish_launch_session` installs `GameOptions` on `Simulation`.
4. During ticks after tick 0, `check_defeat` reads the option and house counts.
5. Defeated houses are excluded from alive-house victory resolution.

### Error Handling

No fallible operations are introduced.

Counts already saturate downward on despawn, so the predicate should not need special underflow handling.

### Testing Strategy

Add focused deterministic sim tests for the predicate:

- `short_game_defeats_house_with_no_buildings_even_if_units_remain`
  - Set `game_options.short_game = true`.
  - Use a house with `owned_building_count = 0`, `owned_unit_count = 1`.
  - Advance enough to pass tick-0 gating or call through the existing tick path.
  - Assert `is_defeated == true`.

- `long_game_keeps_house_alive_when_units_remain`
  - Set `game_options.short_game = false`.
  - Use the same `0` buildings, `1` unit counts.
  - Assert `is_defeated == false`.

- `long_game_defeats_when_no_owned_objects_remain`
  - Set `short_game = false`, counts `0 + 0`.
  - Assert defeated.

- `short_game_victory_resolution_uses_new_defeat_state`
  - Two houses, one loses last building under Short Game.
  - Assert remaining non-defeated house receives `has_won` if no non-allied enemy remains, matching existing victory-resolution behavior.

Avoid tests that require app UI, audio, map reveal, sidebar state, or rendering. Those are out of scope.

## Architectural Decisions

- Keep the behavior in `sim/world` because defeat is deterministic gameplay state.
- Do not add UI/app dependencies; player-facing aftermath remains a separate design target.
- Do not introduce a new defeat subsystem yet. The change is small and fits the existing `check_defeat` ownership.
- Do not invent exact ConYard-style counted-instance modeling without a dedicated follow-up. The current design closes the verified common player-visible mismatch and records the unresolved tiny detail.

## Alternatives Considered

### Add a separate defeat subsystem

This would make sense if implementing full `MPlayer_Defeated`, local-player loss presentation, AI alliance rearrangement, map reveal, and EVA. It is unnecessary for the selected sim-condition scope and would broaden the task.

### Research exact counted-instance behavior before any change

This is the most precise path, but it delays a binary-backed common fix. The trace already verifies the concrete normal scenario: ordinary units do not prevent defeat when the last building is gone under Short Game. Exact counted-instance handling should be revisited when Rust has the relevant owner/type-count model or when a scenario depends on it.

### Set `has_lost` as part of this change

This may be correct for local-player defeat presentation later, but current `check_defeat` only sets `is_defeated` and `has_won`. Adding `has_lost` would expand scope into defeat aftermath and requires separate design against `MPlayer_Defeated` / `Flag_To_Lose`.
