# Garrison System Model Synthesis

Date: 2026-05-27

## Scope

Civilian `CanBeOccupied` garrisons: entry gates, occupant storage, ownership reconciliation, firing/kill-credit state, sell/destruction/red-HP ejection, no-exit fallback, and render lifecycle. Tank bunkers are treated as a separate `Bunker=yes` / `Bunkerable` lifecycle and only referenced to prevent conflation.

This is a model-synthesis, not new reverse engineering. Ghidra spot-check was attempted for `0x00522910`, `0x00458200`, `0x00449C30`, and `0x004585C0`, but the current MCP session returned "Function not found" for those addresses, so binary spot-check status remains sourced from the existing verified reports.

## Current Model

1. Entry validation is `BuildingClass::CanDock @ 0x00457CE0`, not `CanGarrison`. Normal occupant entry requires `CanBeOccupied`, an `Occupier` infantry, same owner or target-owner `MultiplayPassive`, count exactly not equal to `MaxNumberOccupants`, not red HP, and not mind-controlled. `CanGarrison @ 0x004525F0` is a gate passability helper for `Gate=` buildings.
2. Entry commit is `AddGarrisonOccupant @ 0x00522910`: limbo infantry, append to the building occupant vector, increment count, and emit first-occupant mission/sound/EVA side effects. It does not change building owner.
3. Ownership is lazy building-update reconciliation in `CheckAutoSellOrCivilian @ 0x00458200`, called by `BuildingClass::Update` for `CanBeOccupied` buildings. Occupied Civilian-owned buildings transfer to occupant slot 0 owner. Empty non-Civilian buildings revert to the resolved Civilian-side house. There is no native per-building "original owner" field for this path.
4. Red HP reconciliation ejects occupants through `SellBuilding @ 0x00457DE0` without destroying the building, then can immediately take the empty-revert branch in the same reconciliation call.
5. Firing uses the occupant vector plus current fire index `BuildingClass+0x69C`. `GetWeapon` selects `Items[index]`; successful `Fire_At` advances `(index + 1) % count` after launch. Elite missing `EliteOccupyWeapon` falls directly to occupant primary, not normal `OccupyWeapon`.
6. Occupied-building kill credit reads the live current index in `RegisterDestruction`; no captured shooter id was found in the scoped static path.
7. `PenetratesBunker` is not a civilian garrison occupant-removal mechanism. It belongs to the `TechnoClass+0x2E4` bunker/shelter branch; normal area damage should not compact the `CanBeOccupied` occupant vector.
8. Player sell, destruction, and red-HP ejection all use the `SellBuilding` occupant-eject helper, but player sell then continues into normal building removal/refund. Captured civilian garrisons do not have a preservation/revert exception during player sell.
9. Successful ejection processes occupants high-to-low, attempts `Unlimbo(exit, 0)`, calls the occupant scatter virtual immediately, then queues mission `0xF`. For infantry, Scatter can consume scenario `RandomRanged(0,4)` before mission `0xF`.
10. If no exit coordinate exists, `SellBuilding` calls `SpawnUnitsWithParachute(0)`. The null branch destroys/removes occupants high-to-low, with no parachute visual, no unlimbo, no scatter mission, and no RNG.
11. Body SHP frame swap for `CanBeOccupied` buildings is gated by nonzero BState. A healthy occupied civilian building does not get body frame 2 from `GetCurrentFrame`; visible healthy occupied effects may require the separate anim-overlay swap path or may be absent for stock static civilian art.

## Claim Table

| Claim | Best evidence | Status | Active in YR | Safe? |
|---|---|---|---|---|
| `CanDock` is the entry validator | `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| `CanGarrison` is gate passability, not boarding | same | confirmed | yes | implementation-safe |
| `AddGarrisonOccupant` does not change owner | `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| Ownership transfer/revert is update reconciliation | same | confirmed, exact global frame order unresolved | yes | implementation-safe for phase separation |
| Revert target is Civilian side, not stored original owner | same | confirmed | yes | implementation-safe |
| Captured civilian player sell removes building | `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| Current fire index is `+0x69C` | `GARRISON_FIRE_INDEX_KILL_CREDIT_VETERANCY_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| Successful ejection calls direct Scatter before `0xF` | `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| No-exit `SpawnUnitsWithParachute(0)` kills/removes, no chute | `GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md` | confirmed | yes | implementation-safe |
| Healthy occupied body frame is 2 | `GARRISON_FRAME_SWAP_GHIDRA_REPORT.md` | contradicted for `GetCurrentFrame` path | conditional | unsafe |
| Exact foundation-edge scan pseudocode | sell/ejection reports | partially checked | yes | needs targeted re-investigate before pixel-perfect implementation |

## Current Rust Fit

Already aligned or mostly aligned in the current working tree:

- `src/sim/passenger.rs` stores `PassengerCargo.garrison_fire_index` with the verified `+0x69C` meaning.
- `src/sim/combat/combat_weapon.rs` already uses the binary elite fallback: `EliteOccupyWeapon` else primary.
- `src/sim/production/production_sell.rs` now treats captured civilian player sell as normal removal/refund, and destruction ejection uses the sell-style edge helper rather than shuffled foundation interiors.
- `src/sim/production/production_sell.rs` no-exit behavior kills occupants, which matches final outcome, though the helper comment and exact branch semantics need cleanup.

Remaining important mismatches or stale surfaces:

- `src/sim/passenger.rs` still transfers ownership immediately on boarding and restores `garrison_original_owner` on empty unload. Native does neither in the boarding/unload call stack; reconciliation belongs to building update and reverts to Civilian side.
- Cursor/order/sim entry predicates still use owner-name `Neutral`/`Special` shortcuts instead of a `MultiplayPassive` house/country predicate, and the predicate is not centralized.
- `src/sim/production/production_sell.rs` still approximates ejected infantry scatter with `next_u32() % 8` and direct move. Native calls class Scatter and uses scenario `RandomRanged(0,4)` after pre-RNG gates.
- `src/sim/production/production_placement_tests.rs` still contains stale captured-civilian sell tests expecting preserve/revert/no refund and `StructureAbandoned`.
- Body frame/BState and anim-overlay garrison visuals are not ready for exact implementation from current Rust state.

## Decision

Do not run a broad `/re-swarm` now. The lifecycle is well covered by recent targeted Ghidra reports. A broad swarm would mostly rediscover facts already resolved and risk more stale parallel prose.

Use targeted follow-ups only where implementation would otherwise guess:

1. `/re-investigate garrison SellBuilding foundation edge scan exact coordinate order`
2. `/re-investigate garrison global object update order for AddGarrisonOccupant to CheckAutoSellOrCivilian same-frame timing`
3. `/re-investigate garrison BState and anim overlay occupied civilian visuals`

## Recommended Fix Order

1. Clean stale tests and comments for captured civilian player sell so the suite no longer encodes the disproven preserve/revert behavior.
2. Introduce a central CanDock-equivalent predicate and reuse it from cursor/order/command validation; replace owner-name neutral checks with `MultiplayPassive` once house/country data can answer it.
3. Move garrison ownership transfer/revert into a deterministic building reconciliation phase; remove gameplay dependence on `garrison_original_owner` as native state.
4. Implement red-HP occupied-garrison reconciliation: eject via SellBuilding helper, keep building alive, then revert if empty.
5. Replace ejection `% 8` direct scatter with an InfantryClass::Scatter-equivalent path using scenario `RandomRanged(0,4)` and correct mission ordering.
6. Defer body-frame/BState visual work until the targeted visual investigation resolves healthy occupied civilian behavior and anim-overlay variant mapping.

## Do-Not-Implement Notes

- Do not use `CanGarrison` for civilian boarding.
- Do not make captured civilian player sell preserve the building.
- Do not treat `SpawnUnitsWithParachute(0)` as a chute/falling fallback.
- Do not make `PenetratesBunker` kill or remove normal `CanBeOccupied` occupants.
- Do not claim healthy occupied civilian body frame 2 without resolving the BState/overlay question.

## Source Ledger

- `docs/research/GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`
- `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`
- `docs/research/CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`
- `docs/research/GARRISON_FIRE_INDEX_KILL_CREDIT_VETERANCY_GHIDRA_REPORT.md`
- `docs/research/GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`
- `docs/research/GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`
- `docs/research/GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`
- `docs/research/GARRISON_NO_EXIT_PARACHUTE_FALLBACK_GHIDRA_REPORT.md`
- `docs/research/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md`
- `docs/research/BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md`
