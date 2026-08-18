# Bridge Collapse Ground vs Deck Occupants Trace

**Date:** 2026-05-22  
**Scenario:** one bridge-collapse cell contains one ground-layer unit under the deck and one bridge-deck unit on the same cell.  
**Scope:** occupant fallout only: ground-list force kill, deck-list `DropIn`, Rust comparison.  
**Non-scope:** bridge collapse footprint, debris/audio RNG, trigger event 0x1F, pathing after collapse except occupancy relayer output.

## Summary Verdict

Gamemd's active YR path is clear: `CellClass::BlowUpBridge @ 0x0047DD70` walks the ground object list first and applies Rules `C4Warhead`; only after that does it walk the bridge/deck object list and call `ObjectClass::DropIn @ 0x005F4160`. `DropIn` removes the object from the old bridge list while `OnBridge` is still set, then clears `OnBridge`, then re-adds it to the ground list.

Current Rust matches the high-level player-visible result for a clean one-ground plus one-deck setup: the ground unit is selected by the kill pass and the deck unit is selected by the drop pass. The deck unit survives, clears `on_bridge`, clears `bridge_occupancy`, flips locomotor to `Ground`, and retains full HP.

The main mismatch is in the relayer ordering inside `drop_in_bridge_deck_entities`: Rust clears `on_bridge` and `bridge_occupancy` before moving the occupancy record, while gamemd removes from the bridge list before clearing `OnBridge`. With a clean single occupancy record this still produces the same final visible layer, but it is not the same selected-list ordering and can hide stale or duplicate occupancy bugs.

## Evidence

### Gamemd, Active in Standard YR

- `ini/rulesmd.ini` has `DestroyableBridges=yes`, so bridge collapse paths are active in stock YR.
- `ini/rulesmd.ini` has `[CombatDamage] C4Warhead=Super`; `[Super] InfDeath=2`.
- `CellClass::BlowUpBridge @ 0x0047DD70` has no TS/fog gate in this occupant path, only a map-editor early-out.
- `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` marks `BlowUpBridge`, `DropIn`, `TechnoClass::DoCloak`, `EnterCell_AddToMultiCells`, and `ExitCell_RemoveFromMultiCells` active in YR for ordinary bridge-deck units.

Fresh read-only Ghidra spot-checks:

- `CellClass::BlowUpBridge @ 0x0047DD70`: loops `this->FirstObject`, snapshots next, calls vtable `+0x16C` with `RulesClass + 0xFA8` and force-kill flags; then loops `this->AltObject`, snapshots next, calls vtable `+0xEC`.
- `ObjectClass::DropIn @ 0x005F4160`: sets falling bytes, calls vtable `+0x124(0)`, removes from display layer, clears `OnBridge` at `+0x8C`, submits, then calls vtable `+0x124(1)`.
- `TechnoClass::DoCloak @ 0x004D3780`: mode `0` calls `ExitCell_RemoveFromMultiCells`; mode `1` calls `EnterCell_AddToMultiCells`.

### Rust Surfaces

- `src/sim/world/bridge_orchestrator.rs:94`: `apply_bridge_damage_events` runs ground kill before deck `DropIn`.
- `src/sim/world/bridge_orchestrator.rs:286`: hut-collapse execution uses the same order.
- `src/sim/world/bridge_orchestrator.rs:873`: `kill_ground_occupants_at` filters entities at the cell where `!e.is_on_bridge_layer()` and sets `health.current = 0`, `dying = true`, clears targets/selection, and switches infantry death sequence from C4 `InfDeath`.
- `src/sim/world/bridge_orchestrator.rs:1136`: `drop_in_bridge_deck_entities` filters entities where `e.is_on_bridge_layer()`, clears bridge state, sets ground Z, clears movement, flips locomotor to `Ground/Idle`, then calls `occupancy.move_entity(..., MovementLayer::Ground, ...)`.
- `src/sim/occupancy.rs:182`: `OccupancyGrid::remove` removes by entity id, not by selected old layer.
- `src/sim/world/world_tests.rs:797`: deck-only collapse over water verifies deck unit survives and clears bridge state.
- `src/sim/world/world_tests.rs:1031`: ground-only collapse verifies ground unit reaches `health.current = 0`, `dying = true`.

## Stage Verdicts

| Stage | Gamemd output | Rust output for this scenario | Verdict |
|---|---|---|---|
| 1. Active collapse cell reaches occupant fallout | `BlowUpBridge` is live in standard YR when a collapse cell receives it | `apply_bridge_damage_events` / hut execution consume `CellAction::BlowUpBridge` cells | PASS |
| 2. Ground/deck selection | Ground list `FirstObject` only for kill, bridge/deck list `AltObject` only for `DropIn`; with one of each: kill count=1, drop count=1 | `kill_ground_occupants_at` filters `!is_on_bridge_layer`; `drop_in_bridge_deck_entities` filters `is_on_bridge_layer`; with one of each: kill count=1, drop count=1 | PASS |
| 3. Ground force kill | Calls damage vtable with `RulesClass+0xFA8` C4Warhead and force-kill flags | Sets HP to 0, `dying=true`, clears orders/selection, uses C4 `InfDeath` for infantry | PASS for killed/not-killed decision; UNCHECKED for exact binary death-lifecycle side effects |
| 4. Deck survival | Deck object receives `DropIn`, not damage; survives | Deck object is not in kill pass and keeps HP | PASS |
| 5. DropIn state fields | `DropIn` sets falling/bomb bytes, removes while `OnBridge=1`, clears `OnBridge=0`, re-adds while `OnBridge=0` | Rust clears `on_bridge=false` and `bridge_occupancy=None`, then moves occupancy to ground | FAIL |
| 6. Final deck layer | Deck object ends in ground list, not bridge list | Clean occupancy case ends with deck entity on `MovementLayer::Ground`, bridge count 0 | PASS |
| 7. Exact same-cell regression coverage | Scenario has both lists populated simultaneously | No exact combined test exists; only separate deck-only and ground-only tests | NOT-IMPLEMENTED |

## Player-Visible Findings

### FAIL - DropIn selected-list order differs

Rust clears the deck entity's bridge state before occupancy relayering. Gamemd removes from the bridge/deck list first, then clears `OnBridge`, then re-adds to the ground list. In a clean case the final layer is still correct, but if occupancy state is stale or duplicated, Rust's layer-agnostic remove can silently repair the wrong record. Player-visible risk: a deck unit could leave an incorrect bridge/ground occupancy footprint, affecting selection, blocking, or later damage/pathing around the collapsed cell.

Rust: `src/sim/world/bridge_orchestrator.rs:1157`, `src/sim/world/bridge_orchestrator.rs:1174`, `src/sim/occupancy.rs:182`  
Gamemd evidence: `ObjectClass::DropIn @ 0x005F4160`; `TechnoClass::DoCloak @ 0x004D3780`; fallout report `§3.2`.

### NOT-IMPLEMENTED - No exact combined same-cell regression

Current tests prove the two halves separately, but not the concrete scenario with one ground-layer occupant and one deck-layer occupant in the same collapse cell. Player-visible risk: a future edit could make the ground pass kill both entities or make the deck pass skip after the ground kill, and existing tests would not catch it.

Rust: `src/sim/world/world_tests.rs:797`, `src/sim/world/world_tests.rs:1031`  
Gamemd evidence: `CellClass::BlowUpBridge @ 0x0047DD70` ground loop followed by deck loop.

## Adjacent Findings

- `BlowUpBridge` debris/audio ordering and walker-spawned `BridgeExplosions` are adjacent to this scenario but were not traced here.
- Event `0x1F` trigger dispatch is adjacent but not part of ground-vs-deck occupant fallout.
- Exact death animation/despawn timing for the force-killed ground unit remains a narrower follow-up if we need frame-perfect death lifecycle parity.

## Acceptance Test Recommendation

Add a single combined regression:

1. Place one ground unit and one bridge-deck unit on the same bridge cell.
2. Trigger a collapse outcome whose `set_bridge_direction.actions` includes `CellAction::BlowUpBridge` for that cell.
3. Assert ground unit: `health.current == 0`, `dying == true`, no movement/attack target.
4. Assert deck unit: still exists, `health.current == max`, `dying == false`, `on_bridge == false`, `bridge_occupancy == None`, locomotor layer `Ground`, movement target cleared.
5. Assert occupancy: deck entity no longer appears on `MovementLayer::Bridge`; final ground-list content is explicit and deterministic.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
