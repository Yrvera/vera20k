# Captured Civilian Garrison Player Sell Parity Design

## Goal

Make player sell of captured civilian `CanBeOccupied=yes` garrisons match YR: occupants eject through the sell ejection helper and the building is then sold/removed normally, with no captured-civilian preserve/revert branch.

## Architecture Context

The sell command path is already layered correctly. `src/app_commands.rs::sell_selected_buildings` collects selected structures owned by the local player and schedules `Command::SellBuilding`. `src/sim/world/world_commands.rs` validates the command owner still owns the entity, then calls `production::sell_building`.

`src/sim/production/production_sell.rs` owns the whole sell transaction: refund calculation, crew survivor ejection, garrison occupant ejection, docked miner interruption, entity removal, SpySat reshroud, superweapon refresh, and credit deposit. This is the correct module for the player-sell outcome.

`src/sim/passenger.rs` currently owns boarding/unloading and the broader approximate garrison ownership model using `garrison_original_owner`. That model is stale versus the newest ownership timing report, but it is intentionally out of scope for this player-sell-only design. The design must not change boarding timing, last-occupant unload reversion, or `StructureAbandoned` emission from the unload path.

Relevant convention: keep this fully inside `sim/` and `rules/` data access. No render, UI, sidebar, audio sink, or app-level behavior should be added for the sim sell outcome.

## Impact Analysis

Future implementation should touch only:

- `src/sim/production/production_sell.rs`
- focused tests in the same module, or an adjacent existing production/passenger test module if needed

Expected effects:

- Captured civilian garrison player sell changes from "eject occupants, emit StructureAbandoned, revert owner, keep building, no refund" to "eject occupants, remove building, pay normal sell refund."
- Normal non-garrison building sell remains unchanged.
- Normal garrisoned building sell remains unchanged except that the garrison ejection helper must stop doing an owner revert during player sell.
- Last-occupant unload/abandon behavior in `src/sim/passenger.rs` remains unchanged.
- Destruction ejection remains unchanged because it already uses `eject_destruction_garrison`, not the player-sell helper.

Risk areas:

- `eject_garrison_occupants` currently clears cargo and also takes `garrison_original_owner` to revert owner. Native `SellBuilding` does not change owner, so the helper should not perform revert when called from player sell.
- The current captured branch emits `StructureAbandoned`; native player sell does not. Keep that event tied to empty-garrison reconciliation/unload, not player sell.
- Refund owner must be captured before any ejection mutation and credited to the selling player, as the current normal sell path already does.
- Existing ejection scatter/pathing approximation is known stale, but this design must not expand into fixing Scatter mission ordering.

Determinism: no new RNG, data structure iteration, floating point, or tick-order surface is required. The future change should remove a branch and keep the existing deterministic sell sequence.

## Chosen Approach

Use a narrow player-sell fix:

1. Remove `garrison_original_owner.is_some()` as a player-sell outcome discriminator.
2. Let captured civilian garrisons use the same normal `sell_building` path as other player-owned structures.
3. Adjust the player-sell garrison ejection helper so it ejects and clears occupants without changing building owner or emitting `StructureAbandoned`.
4. Leave `passenger.rs` ownership transfer/revert behavior unchanged for this pass.

This matches the verified native split: `BuildingClass::Sell` controls the player sell transaction and calls `SellBuilding` only as an occupant-ejection stage; ownership reversion belongs to `CheckAutoSellOrCivilian`, not player sell.

## Tiny-Detail Ledger

- Standard YR sell mode is active for this target once the building is player-owned; sell-mode checks current ownership/human owner and has no captured-civilian-origin exception. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` sections 3.1-3.2.
- Captured civilian garrisons become player-owned before this target through `CheckAutoSellOrCivilian`; this design starts at the player sell command after ownership reconciliation has made the building owned by the command owner. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` section 3.2; `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` sections 3.2-3.3.
- Sell event execution requires a valid live object and then dispatches the building sell mission; Rust's world command already mirrors the observable requirement by validating ownership and entity presence before calling production sell. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` section 3.3.
- `BuildingClass::Sell` state 1 calls occupant count and, if positive, calls `SellBuilding`; it then continues to final sell/removal/refund logic. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` section 3.4.
- `SellBuilding @ 0x00457DE0` is occupant ejection only for this target: it does not call `ChangeOwner` and is not the complete sell transaction. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` sections 3.4 and 8; `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` section 3.5.
- There is no native player-sell branch that preserves the captured civilian building, reverts it to Civilian/Neutral/Special, or suppresses refund because `garrison_original_owner` is set. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` sections 1, 3.4, 8, and 10.
- `StructureAbandoned`-style side effects belong to empty-building reconciliation before `ChangeOwner`, not the player sell transaction. Source: `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` sections 3.4 and 8.
- Empty garrison edge case: if occupant count is zero, native skips the `SellBuilding` ejection stage and still proceeds to final sell logic. Source: `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` open question OQ-14.
- Occupant exit order/Scatter details are not changed by this design. Current Rust still uses its existing sell-edge ejection approximation; exact Scatter parity is a separate queued design target. Source: `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`; `GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`.

## Design

### Components

`sell_building`

- Stop capturing `is_captured` and `abandoning_owner` for player sell.
- Remove the early captured-civilian branch.
- Preserve the current normal sell sequence:
  - snapshot current owner/type/position/health;
  - compute refund;
  - eject normal crew survivors;
  - eject garrison occupants;
  - interrupt docked miners;
  - clear contacts;
  - remove the building;
  - apply SpySat and superweapon refresh side effects;
  - credit the selling owner.

Garrison sell ejection helper

- Keep the occupant snapshot, LIFO edge ejection call, cargo clear, and fire-index reset.
- Remove owner revert from the player-sell helper path.
- Update comments so the helper reflects the native contract: ejection/clear only, no `ChangeOwner`.

Tests

- Add a player-sell regression test that constructs a player-owned captured civilian garrison state:
  - building category `Structure`;
  - owner `Americans`;
  - `garrison_original_owner = Some(Neutral)` to model the current Rust captured state;
  - cargo contains at least one hidden occupant.
- Call `sell_building`.
- Assert:
  - function returns `true`;
  - building entity is removed;
  - occupant entity remains outside/ungarrisoned if an exit cell exists;
  - seller receives normal refund when cost/health make refund nonzero;
  - no `SimSoundEvent::StructureAbandoned` is emitted by player sell.

- Add or adjust a helper-level test if needed to prove player-sell garrison ejection does not revert owner before removal. Since the building is removed in the public transaction, this can remain an internal helper test only if it is cheap and stable.

### Interfaces / Contracts

No public API change is required. `production::sell_building(sim, rules, stable_id) -> bool` stays the transaction boundary.

The private ejection helper contract should become:

```text
player-sell garrison ejection = eject occupants and clear building cargo; do not decide ownership outcome
```

Ownership reversion remains the contract of the passenger/unload/reconciliation path, not the production sell path.

### Data Flow

1. App schedules `Command::SellBuilding`.
2. World command validates current command-owner ownership.
3. `production_sell::sell_building` snapshots current owner/type/position/health.
4. It computes refund from current owner/type/health.
5. It ejects crew survivors and garrison occupants.
6. It removes the building and pays/refreshed side effects for the selling owner.

The future implementation should not inspect `garrison_original_owner` inside `sell_building`. A set `garrison_original_owner` may still exist on the entity before removal because the broader ownership model is unchanged, but it must not alter player-sell outcome.

### Error Handling

Preserve existing behavior:

- return `false` if the entity is missing;
- return `false` if the entity is not a structure;
- return `false` if the rules object is missing;
- return `true` after a successful sell transaction, even if no garrison occupants exist or no garrison occupants could be ejected.

No new error type is needed.

### Testing Strategy

Focused commands after implementation:

```powershell
cargo test --lib production_sell
cargo test --lib garrison
cargo check --lib
```

Expected new/updated assertions:

- Captured civilian garrison player sell removes the building.
- Captured civilian garrison player sell does not emit `StructureAbandoned`.
- Captured civilian garrison player sell pays normal refund to current owner.
- Existing destruction ejection test still passes.
- Existing last-occupant unload abandonment test still passes.

## Architectural Decisions

- Keep the fix in `production_sell.rs` because the mismatch is the player-sell transaction outcome.
- Do not change `passenger.rs` ownership reconciliation in this pass. That stale model is verified separately, but changing it now would expand the blast radius into boarding tick order, abandon events, and red-HP ejection.
- Do not introduce a new enum or command variant unless the helper needs an internal mode for readability. Since this target only needs player sell to stop reverting owner, the simplest acceptable shape is either a renamed helper or a private boolean-free helper split.
- Do not add app/UI sell gating. Rust already queues sell only for selected local-owner structures and world command validates ownership, which is enough for this scoped outcome.

Tech debt left intentionally:

- `garrison_original_owner` remains a broader approximation of native civilian reconciliation.
- Sell ejection Scatter behavior remains approximate and is queued separately.

## Alternatives Considered

### Alternative A: Delete Only The Early Captured Branch

This would make captured civilian garrisons fall through to normal removal/refund. It is very small, but the current ejection helper would still revert owner before removal because it takes `garrison_original_owner`. That transient owner write is probably not visible in the same transaction, but it contradicts the verified native helper contract and keeps misleading comments alive. Rejected.

### Alternative B: Chosen Narrow Player-Sell Fix

Remove the captured branch and make the player-sell garrison ejection helper eject/clear without ownership reversion. This directly matches the verified `BuildingClass::Sell`/`SellBuilding` split, keeps the implementation local, and avoids pulling in the broader reconciliation rewrite.

### Alternative C: Full Civilian Garrison Reconciliation Rewrite

Move ownership transfer/revert out of immediate boarding/unload and into a building reconciliation phase, resolve Civilian house instead of stored original owner, and include red-HP ejection behavior. This is probably the correct later design, but it is not needed to fix player-sell outcome and it touches tick ordering and several player-visible side effects. Deferred by user scope.
