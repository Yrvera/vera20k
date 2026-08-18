# Bridge Collapse Slot 4 - BlowUpBridge Fallout Trace

Date: 2026-05-22

Scope: one bridge collapse from weapon damage while one normal ground unit is on the ground layer directly below a collapse-affected bridge cell and one normal ground unit is on the bridge deck at that cell. This report covers only BlowUpBridge fallout: deck DropIn handling, ground kill handling, collapse cell scoping, and collapsed-cell queue/state mutation.

Non-scope: DestroyableBridges session flag source, BridgeStrength RNG gate, debris RNG exactness, collapse sound, render atlas selection, C4/CABHUT entry, and campaign trigger action execution.

## Verdict

Not complete parity.

For a unit pair sitting on an actual `BlowUpBridge` cell, Rust matches the broad player-visible intent: the ground-layer unit is force-killed and the bridge-deck unit survives by being snapped to ground. The parity break is in fallout scoping and queue semantics: gamemd executes kill, DropIn, queue append, and per-cell debris together for each cell that actually receives `CellClass::BlowUpBridge`; Rust kills only `blow_up_cells` but runs DropIn/debris over the wider `destroyed_set`, and it has no gamemd-equivalent persistent collapsed-cell queue.

Verdict tally: PASS: 3 | FAIL: 3 | UNCHECKED: 4 | NOT-IMPLEMENTED: 1

## Concrete Scenario

- Trigger: weapon damage has already passed the `DestroyableBridges`, `Wall=yes`, and BridgeStrength gates and has reached a bridge-collapse path.
- Cell C: a cell that actually receives `CellClass::BlowUpBridge` in gamemd.
- Occupants:
  - Ground unit G at C, `OnBridge == 0`, in the ground/FirstObject list.
  - Deck unit D at C, `OnBridge == 1`, in the bridge/AltObject list.

## Pipeline

1. Weapon-collapse dispatcher produces one or more bridge collapse outcomes.
2. `CellClass::BlowUpBridge(C)` runs for actual BlowUpBridge cells.
3. Ground-list pass force-kills ground occupants.
4. Deck-list pass calls `ObjectClass::DropIn` for deck occupants.
5. Cell coordinate is appended to the collapsed-cell queue.
6. Optional BlowUpBridge debris/anim block runs for that same cell.
7. Bridge state/zone/path updates are finalized by the collapse/state-machine tail.

## Stage Results

### Stage 1 - Active gamemd path

gamemd evidence: Read-only Ghidra decompile of `CellClass__BlowUpBridge @ 0047DD70` in the active `gamemd.exe` program shows the ground object-list loop, then the bridge object-list loop, then queue append, then optional debris. `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` marks this active in standard YR through live bridge-collapse callers and stock `DestroyableBridges=yes`.

Rust evidence: `src/sim/world/bridge_orchestrator.rs:53` enters `apply_bridge_damage_events`; `src/sim/world/bridge_orchestrator.rs:70` collects collapse outcomes.

Verdict: PASS for path presence, but exact trigger gate is outside this slot.

### Stage 2 - Ground unit below bridge

gamemd output for G at actual BlowUpBridge cell C: `CellClass__BlowUpBridge @ 0047DD70` iterates `this->FirstObject` first and calls the object's damage virtual with `RulesClass+C4Warhead`, damage `0`, and force-kill style flags before any deck DropIn. The next pointer is snapshotted before mutation. Active in standard YR: yes, per direct decompile and the fallout report.

Rust output: `kill_ground_occupants_at` filters `position.rx == C.x`, `position.ry == C.y`, `!is_on_bridge_layer()`, and `health.current > 0`; it sets `health.current = 0`, `dying = true`, clears attack/movement targets, clears selection, and switches infantry death sequence from C4Warhead `InfDeath`. See `src/sim/world/bridge_orchestrator.rs:873`.

Verdict: PASS for selected occupant and player-visible death at C. UNCHECKED for literal equality of all death side effects because this trace did not compute gamemd's full post-damage object state field-by-field.

### Stage 3 - Deck unit on bridge

gamemd output for D at actual BlowUpBridge cell C: the deck list `AltObject` is walked after the ground kill loop. Each deck object receives vtable `+0xEC`, which is `ObjectClass::DropIn` for normal objects. `ObjectClass__DropIn @ 005F4160` sets two falling/bomb bytes, calls the cell exit hook, removes from display layer, clears `OnBridge`, submits to display layer, then calls the enter hook. Active in standard YR: yes for normal Techno occupants.

Rust output: `drop_in_bridge_deck_entities` finds entities at C where `is_on_bridge_layer()` is true, clears `bridge_occupancy`, sets `on_bridge=false`, sets `position.z` to terrain ground level, refreshes screen coordinates, clears movement target, changes locomotor layer to `Ground`, and sets phase `Idle`. See `src/sim/world/bridge_orchestrator.rs:1136`.

Verdict: PASS for deck unit survival, ground snap, and layer change at C. UNCHECKED for exact falling/bomb bytes and display-layer timing because Rust has no obvious equivalent fields and this trace did not compute a field-for-field mapping.

### Stage 4 - DropIn relayer order

gamemd output: `ObjectClass__DropIn @ 005F4160` calls the exit hook before clearing `OnBridge`, then clears `OnBridge`, then calls the enter hook. The fallout report verifies Techno enter/exit helpers read `OnBridge` at call time, so removal observes bridge layer and insertion observes ground layer.

Rust output: `drop_in_bridge_deck_entities` clears `on_bridge` and changes locomotor layer before calling `sim.occupancy.move_entity(... MovementLayer::Ground ...)`. `OccupancyGrid::remove` removes by entity id across the whole cell rather than by selected old layer. See `src/sim/world/bridge_orchestrator.rs:1157` and `src/sim/occupancy.rs:182`.

Verdict: FAIL as an internal ordering/selected-list parity point. Player-visible risk is duplicate/mis-layered occupancy if stale layer data exists or if a future occupancy invariant depends on old-layer removal.

### Stage 5 - Exact cell scoping

gamemd output: `CellClass::BlowUpBridge` is per-cell. Ground kill, deck DropIn, collapsed-cell queue append, and BlowUpBridge debris happen only for cells where the binary calls `BlowUpBridge`. State-machine final branches call specific BlowUpBridge cells; other overlay/state writes can mutate additional cells without making them BlowUpBridge fallout cells.

Rust output: `apply_bridge_damage_events` builds both `blow_up_cells` and `destroyed_set`. It kills ground occupants only for `blow_up_cells`, but calls `drop_in_bridge_deck_entities` and `spawn_bridge_debris` for every cell in `destroyed_set`. See `src/sim/world/bridge_orchestrator.rs:75`, `src/sim/world/bridge_orchestrator.rs:108`, `src/sim/world/bridge_orchestrator.rs:116`, and `src/sim/world/bridge_orchestrator.rs:123`.

Verdict: FAIL. A deck unit in a destroyed-only/flag-only cell can be dropped in Rust even though gamemd would not run `DropIn` unless that cell received `BlowUpBridge`.

### Stage 6 - Collapsed-cell queue/state mutation

gamemd output: after ground kill and deck DropIn, `CellClass::BlowUpBridge @ 0047DD70` appends the cell coordinate to a global collapsed-cell queue if capacity/allocation checks pass. This happens before the optional debris block.

Rust output: no persistent collapsed-cell queue was found in `src/sim`; `destroyed_set` is a local `BTreeSet` in the orchestrator and is later passed to no-op trigger notification and debris/rim hooks. Search for collapsed-cell queue equivalents found no durable queue.

Verdict: NOT-IMPLEMENTED for literal collapsed-cell queue. Player-visible effect is UNCHECKED because this trace did not identify a current Rust consumer or prove the gamemd queue's downstream screen effect in standard skirmish.

### Stage 7 - Bridge state mutation

gamemd output: state-machine and collapse paths mutate bridge cell state before/around BlowUpBridge calls and run zone/path invalidation tails after collapse. `ProcessBridgeDamageStateMachine_High @ 00576BA0` decompile shows BlowUpBridge calls in final branches, then `CellClass__SetBridgeDirection_NESW`, state byte/overlay clear, adjacent bridge update, and `InvalidateBridgeZones`.

Rust output: `BridgeRuntimeState::body_cell_advance_state` returns `StateOutcome::Collapsed` with `destroyed_cells`, `set_bridge_direction`, adjacent dirty cells, and `zones_dirty`; the orchestrator applies fallout then calls `refresh_bridge_zones_if_dirty`. See `src/sim/bridge_state/mod.rs:1087` and `src/sim/world/bridge_orchestrator.rs:151`.

Verdict: PASS for existence and broad ordering. UNCHECKED for exact mutation order and all state bytes because this trace did not compute a complete before/after cell table for a retail bridge fixture.

### Stage 8 - Tests and coverage

Existing Rust tests cover separate slices:

- `test_bridge_collapse_kills_ground_unit_under_destroyed_cell` asserts ground unit HP 0 and dying true after collapse.
- `test_destroyed_bridge_snaps_unit_to_ground_over_water_below` asserts deck unit survives, z snaps to 0, `on_bridge` clears, and locomotor is ground.
- `drop_in_snaps_deck_entity_to_ground_over_water_no_despawn` asserts the helper-level DropIn behavior.
- `drop_in_ignores_ground_layer_entities_at_destroyed_cell` asserts DropIn helper does not touch ground-layer occupants.

No focused test was found for the exact combined two-occupant scenario in one collapse cell, no test was found proving DropIn/debris are scoped only to `blow_up_cells`, and no test was found for a collapsed-cell queue.

Verdict: FAIL for missing exact scoping test; UNCHECKED for combined occupant same-cell end-to-end test.

## Player-Visible Findings

1. Stage 5 FAIL: Rust can drop deck units on destroyed-only/flag-only cells; gamemd only calls `DropIn` on actual `BlowUpBridge` cells. Rust: `src/sim/world/bridge_orchestrator.rs:116`. gamemd: `CellClass__BlowUpBridge @ 0047DD70` per-cell AltObject loop.
2. Stage 5 FAIL: Rust can spawn BlowUpBridge debris for destroyed-only cells; gamemd's BlowUpBridge debris belongs to the same per-cell `BlowUpBridge` call. Rust: `src/sim/world/bridge_orchestrator.rs:123`. gamemd: `CellClass__BlowUpBridge @ 0047DD70` optional debris after queue append.
3. Stage 6 NOT-IMPLEMENTED: Rust has no persistent collapsed-cell queue equivalent. Rust: local `destroyed_set` at `src/sim/world/bridge_orchestrator.rs:75`. gamemd: queue append inside `CellClass__BlowUpBridge @ 0047DD70`.
4. Stage 4 FAIL: Rust clears bridge state before occupancy removal and removes by id across layers; gamemd removes while `OnBridge==1`, clears, then re-adds while `OnBridge==0`. Rust: `src/sim/world/bridge_orchestrator.rs:1157`; occupancy remove: `src/sim/occupancy.rs:182`. gamemd: `ObjectClass__DropIn @ 005F4160`.

## Adjacent Findings

- Debris RNG and sound parity are adjacent to this trace but intentionally not traced here.
- Campaign trigger event 31 remains adjacent; this slot only needed to distinguish it from BlowUpBridge queue/fallout.
- The Rust comments around "BlowUpBridge force-kills ground-layer entities at each destroyed cell" are misleading because the code actually kills only `blow_up_cells` but DropIn/debris use `destroyed_set`.

## Evidence Ledger

- Read-only Ghidra decompile: `CellClass__BlowUpBridge @ 0047DD70`.
- Read-only Ghidra decompile: `ObjectClass__DropIn @ 005F4160`.
- Read-only Ghidra decompile: `TechnoClass__DoCloak @ 004D3780`.
- Read-only Ghidra decompile: `TechnoClass__EnterCell_AddToMultiCells @ 005683C0`.
- Read-only Ghidra decompile: `TechnoClass__ExitCell_RemoveFromMultiCells @ 005687F0`.
- Read-only Ghidra decompile: `ProcessBridgeDamageStateMachine_High @ 00576BA0`.
- `docs/research/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`.
- `docs/research/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`.
- `src/sim/world/bridge_orchestrator.rs`.
- `src/sim/bridge_state/mod.rs`.
- `src/sim/bridge_specs.rs`.
- `src/sim/occupancy.rs`.

## Status

COMPLETE for the requested slot. No Rust, INI, or existing docs were modified.
