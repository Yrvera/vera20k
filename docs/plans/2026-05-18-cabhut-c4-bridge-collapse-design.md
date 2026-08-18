# CABHUT C4 Bridge Collapse Design

## Goal

Make C4 on `BridgeRepairHut=yes` buildings collapse the connected bridge like gamemd.exe, including overlay-first discovery, ramp/flag fallback, and C4 marker cleanup.

## Architecture Context

The current command path is:

1. `app_context_order::try_queue_context_order_at_screen_point` queues `Command::PlantC4` for selected C4 infantry against an enemy `CanC4` structure.
2. `Simulation::apply_command(Command::PlantC4)` validates the attacker and target, sets `entity.c4_plant`, and moves the attacker toward the target building cell.
3. `Simulation::tick_c4_plants` claims a pending C4 detonation when the attacker reaches the target, waits `rules.c4_delay_ticks`, then calls `apply_c4_damage_to_building`.
4. `apply_c4_damage_to_building` routes `BridgeRepairHut=yes` targets to `bridge_orchestrator::dispatch_bridge_collapse_from_hut`.
5. `dispatch_bridge_collapse_from_hut` currently scans a hut-centered 5x5, keeps only cells with `anchor_span_id`, drives `body_cell_advance_state(..., is_high_bridge=false)` to convergence, then runs the same collapse cascade used by normal bridge damage.

The bridge runtime already has the right lower-level machinery for this fix:

- `BridgeRuntimeState::destroy_bridge_low` and `destroy_bridge_high` are overlay-direct entries.
- `destroy_bridge_walker_*_{low,high}` mutate 3-cell strips and return `StateOutcome`s.
- `bridge_orchestrator` already aggregates `StateOutcome::Collapsed` into ground kills, deck drop-in, debris, adjacent refresh, trigger hook, and zone refresh.
- Engineer repair already has an overlay-first scan shape in `repair_bridge_from_engineer_scan`, so this design follows an existing pattern.

The fix belongs in `sim/` only. It must not introduce render, UI, audio, or net dependencies.

## Impact Analysis

Touched modules:

- `src/sim/world/world_orders.rs`
  - Clear `pending_c4_detonation` after the BridgeRepairHut dispatch returns.
  - Keep non-hut behavior unchanged.

- `src/sim/world/bridge_orchestrator.rs`
  - Replace the current span-only hut dispatcher with a gamemd-shaped dispatcher.
  - Factor shared cascade aggregation if useful, but keep behavior local and deterministic.

- `src/sim/bridge_state/walker.rs`
  - Reuse existing `destroy_bridge_low/high`.
  - Add small public or crate-visible helpers only if the hut dispatcher needs overlay predicates or start-cell evidence helpers.

- Tests:
  - Extend `world_orders_bridge_repair_tests.rs` or add focused bridge-hut C4 tests.
  - Add coverage for low overlay, high overlay, bridgehead/ramp fallback, and stale pending cleanup.

Risk areas:

- Deterministic iteration: 5x5 scan order and 8-direction fallback order must be fixed and tested.
- False positives: fallback must not collapse an unrelated bridge outside gamemd's local search envelope.
- Existing bridge damage: normal combat bridge collapse must keep using the current event dispatcher.
- RNG stream: this design reuses existing destroy walkers and cascade, so RNG draw order changes should be limited to cases that currently no-op.

## Chosen Approach

Use a dedicated gamemd-shaped CABHUT dispatcher inside `bridge_orchestrator`.

The dispatcher will:

1. Build the hut-centered 5x5 scan using the existing `cells_in_5x5_scan`.
2. Decide low vs high using gamemd's outer evidence rule:
   - low if any scan cell has low bridge tile evidence or overlay `0x4A..=0x65`;
   - high otherwise.
3. Try overlay-first dispatch:
   - for low, find the first scan cell with overlay `0x4A..=0x65`;
   - for high, find the first scan cell with overlay `0xCD..=0xE8`;
   - call `BridgeRuntimeState::destroy_bridge_low/high` on that cell.
4. If no overlay was found, run a fallback bridge evidence search:
   - inspect the hut cell and the 8 directions out to 3 cells for bridgehead/ramp/deck evidence derived from `ResolvedTerrainCell` and `BridgeRuntimeCell`;
   - from that evidence, find the nearest real overlay/body cell in the selected low/high family;
   - call `destroy_bridge_low/high` on that cell.
5. Collect every non-`NoChange` outcome from the destroy attempts.
6. Run the existing cascade aggregation used by `dispatch_bridge_collapse_from_hut`.
7. Return `true` iff at least one cell reached `StateOutcome::Collapsed`.

`world_orders.rs` will clear the CABHUT pending C4 marker after the hut dispatch returns, matching gamemd's `BuildingClass+0x6DF = 0` and attacker pointer cleanup.

## Tiny-Detail Ledger

- C4 action on CABHUT is valid when infantry has `C4=yes`, target is a building, target `CanC4=true`, `InvisibleInGame=false`, not destroyed, and not Iron Curtained. `Immune=yes` is not a blocker. Source: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` 18A.1.
- Gamemd claims the plant from the target building cell lookup. Rust currently claims at Chebyshev adjacency; this is existing documented drift and not part of this design. Source: `c4-on-bridge-repair-hut.md` Trace 1.
- `[CombatDamage] C4Delay=.03` and `C4Warhead=Super`. Source: `ini/rulesmd.ini`.
- CABHUT branch leaves hut HP unchanged and does not call normal building damage. Source: `BuildingClass::Update @ 0x0043FB20`, report 18A.3.
- CABHUT clears the C4 marker and attacker pointer after dispatch. Source: report 18A.3.
- Hut scan is 5x5 centered on the hut/input cell. Source: report 18A.4.
- Low/high selection: low if low tile evidence or overlay `0x4A..=0x65` exists in the scan; otherwise high. Source: report 18A.3 and 18A.4.
- Overlay-first entry: low accepts `0x4A..=0x65`; high accepts `0xCD..=0xE8`; first match calls `DestroyBridgeFromCell_*` and returns. Source: report 18A.4.
- `DestroyBridgeFromCell_*` canonicalizes by checking one and two cells backward along the bridge axis before calling collapse walkers. Source: report 18A.5.
- If no overlay is found, fallback searches bridge/ramp flags from the input cell and 8 directions up to 3 cells. Source: report 18A.4.
- Collapse dispatch performs at most four sweep steps and at most three destroy attempts per step. Source: report 18A.6.
- Collapse walkers write 3-cell strips and terminal caps are `0x64/0x65` low, `0xE7/0xE8` high. Source: report 18A.7.
- Existing cascade must keep C4Warhead ground-kill semantics, deck drop-in, debris, adjacent refresh, trigger hook, and zone refresh. Source: `docs/traces/2026-05-08-trace-bridge-damage-collapse.md`.

## Design

### Components

`dispatch_bridge_collapse_from_hut`

- Becomes the high-level CABHUT destroy entry.
- Owns the 5x5 scan, low/high decision, overlay-first attempt, fallback attempt, outcome aggregation, and cascade handoff.

`HutBridgeEvidence`

- A small local struct or enum inside `bridge_orchestrator.rs`.
- Represents evidence discovered from `ResolvedTerrainCell` and `BridgeRuntimeCell`.
- Expected variants:
  - `LowOverlay(rx, ry)`
  - `HighOverlay(rx, ry)`
  - `BridgeheadOrRamp(rx, ry)`
  - `DeckOrSpan(rx, ry)`
- This is not a persisted sim component. It exists only during the hut dispatch.

`find_hut_bridge_overlay_entry`

- Scans in fixed 5x5 order.
- Returns the first overlay cell in the selected family.
- Uses bridge runtime `overlay_byte`, not `anchor_span_id`.

`find_hut_bridge_fallback_entry`

- Used only when overlay-first found no entry.
- Searches the hut cell, then 8 directions out to distance 1, 2, and 3 in fixed direction order.
- For bridgehead/ramp evidence, walks locally to the nearest low/high overlay cell that can enter `destroy_bridge_low/high`.
- Returns `None` if no real entry can be found.

`run_hut_destroy_entry`

- Calls `destroy_bridge_low` or `destroy_bridge_high` on the chosen cell.
- If the result is `Absorbed`, repeats up to the gamemd cap where applicable so a multi-stage overlay can reach `Collapsed`.
- Stops after `Collapsed` or `NoChange` per attempt rules.

### Interfaces / Contracts

No public game API changes.

Potential crate-visible bridge-state helpers:

- `BridgeRuntimeState::is_low_destroy_overlay(overlay: u8) -> bool`
- `BridgeRuntimeState::is_high_destroy_overlay(overlay: u8) -> bool`

These should wrap existing predicates rather than duplicate numeric ranges in multiple modules.

`dispatch_bridge_collapse_from_hut` keeps its current signature:

```rust
pub(crate) fn dispatch_bridge_collapse_from_hut(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool
```

`world_orders.rs` must clear pending C4 on a BridgeRepairHut after this function returns, even when it returns `false`. The hut survives, but the C4 marker does not.

### Data Flow

1. C4 timer expires.
2. `apply_c4_damage_to_building` detects `bridge_repair_hut`.
3. It calls `dispatch_bridge_collapse_from_hut`.
4. The dispatcher takes immutable terrain and mutable bridge state in a scoped borrow.
5. The dispatcher records `StateOutcome`s and releases the bridge-state borrow.
6. The existing cascade consumes the collected outcomes:
   - kill ground occupants at BlowUpBridge cells;
   - drop deck occupants;
   - spawn debris;
   - refresh adjacent bridges;
   - notify trigger hook;
   - refresh zones.
7. Control returns to `tick_c4_plants`.
8. The CABHUT pending marker is cleared.
9. `bridge_state_changed` propagates to `TickResult` so the app rebuilds pathing/render state.

### Error Handling

This is deterministic simulation code, so it should not throw or log on normal map edge cases.

- Missing `resolved_terrain` or `bridge_state`: return `false`.
- No overlay or fallback evidence: return `false`, but still let world-order cleanup clear the C4 marker.
- Fallback evidence found but no usable overlay entry: return `false`.
- Out-of-bounds search cells: skip.
- Non-destroyable bridge state: match existing bridge-damage behavior and return `false`.

### Testing Strategy

Unit tests should construct small `Simulation`/`BridgeRuntimeState` fixtures without needing full map archives.

Required tests:

- C4 on CABHUT low overlay in 5x5 collapses bridge and hut survives.
- C4 on CABHUT high overlay in 5x5 uses high destroy path.
- C4 on CABHUT with only spanless bridgehead/ramp evidence in the 5x5 still finds a nearby overlay/body entry.
- C4 on CABHUT clears `pending_c4_detonation` after dispatch, even when no bridge state changed.
- Second C4 plant after a previous CABHUT dispatch is not blocked by stale pending state.
- Existing seeded span test remains passing or is rewritten to assert overlay-first behavior instead of span-only behavior.

Regression tests:

- Normal non-hut building C4 still kills the building.
- Existing combat bridge damage tests still pass.
- Engineer bridge repair tests still pass.

Determinism tests:

- Lock fixed scan order for 5x5 overlay match.
- Lock fixed fallback direction/distance order.
- If the dispatcher loops multiple destroy attempts, test the cap and outcome order.

## Architectural Decisions

- Reuse overlay-direct destroy walkers instead of adding a separate CABHUT collapse state machine.
- Keep gamemd cell-flag behavior as local "bridge evidence" derived from current terrain/runtime data, not as a persisted `CellClass+0x140` clone.
- Keep normal combat bridge damage and CABHUT destruction as separate entries that converge at `StateOutcome` cascade aggregation.
- Clear the C4 marker in `world_orders.rs`, where the building pending state lives, not inside bridge orchestration.

Tech debt:

- The fallback is an evidence approximation unless a future investigation requires storing exact gamemd `CellClass+0x140` bits. The design keeps this isolated so it can be replaced later.
- The existing adjacency-based C4 plant claim remains a documented parity drift.

## Alternatives Considered

### Minimal Overlay Rewire

Only use the 5x5 overlay scan and clear pending C4. Rejected because it leaves a known parity hole when gamemd would continue through bridgehead/ramp flag fallback.

### Full Cell-Flag Model

Persist a gamemd-like bridge flag model in resolved terrain and implement `DestroyBridge_*_MapInit` nearly structurally. Rejected for this fix because it broadens the terrain contract and risks unrelated pathing/bridge behavior. The chosen design can evolve into this later if exact flags become necessary.

### Current Span-Only Dispatch

Keep scanning for `anchor_span_id` and improve tests. Rejected because it is the observed failure mode: valid gamemd bridge/hut topologies can no-op when the hut scan sees bridgehead/ramp evidence but no span-tagged body cell.
