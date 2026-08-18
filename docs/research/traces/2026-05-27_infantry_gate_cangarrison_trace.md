# Infantry Gate CanGarrison Passability Trace

Scenario: allied infantry attempts to enter a friendly `Gate=yes` building across three runtime states: closed stable, opening, and mission `0x18` stable-open.

Scope: this trace covers only the infantry `Can_Enter_Cell` gate passability/result-code contract. It does not trace enemy gates, vehicle gate contact, refinery/bib pads, bunker row helpers, civilian `CanDock`, or the full visual gate animation.

## Verdict

PASS: 3 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

Top findings:

1. FAIL - Closed friendly gate: gamemd returns allied gate soft-block code `3`; Rust returns generic friendly stationary code `6`.
2. FAIL - Opening friendly gate: gamemd still returns allied gate soft-block code `3`; Rust still returns generic friendly stationary code `6`.

## Pipeline

`Infantry runtime/A* cell entry` -> `target cell object-list building occupant` -> `BuildingType.Gate` -> `BuildingClass::CanGarrison` read-side predicate -> `InfantryClass::Can_Enter_Cell` result-code upgrade -> movement/pathing response.

Rust equivalent path:

`movement_occupancy::detect_deferred_cell_check` -> `request_gate_open_for_cell` -> `classify_occupied_cell_with_layers_and_ignored` -> `classify_blocker` -> `CellEntryResult::yr_code`.

## Evidence Summary

Native active-YR evidence:

- `BuildingClass::CanGarrison @ 0x004525F0` reads `BuildingType+0x16B7` (`Gate=`). Non-gates return `1`; gates require current mission `0x18` and helper `0x004A51B0(Building+0x350)` true.
- `0x004A51B0` returns true only when helper byte `+0x18 == 0` and byte `+0x19 == 1`.
- `InfantryClass::Can_Enter_Cell @ 0x0051BF90`, branch `0x0051C4EB..0x0051C549`, consumes that result. Failed friendly/allied gate passability upgrades `EBX` to at least `3`; successful gate passability jumps to continuation without raising the result.
- This path is active in standard YR: `AStar_main_loop @ 0x00429A90` dispatches through mover vtable `+0x1AC`, and `WalkLocomotionClass::ProcessMovement @ 0x0075B650` calls the same `Can_Enter_Cell` slot at runtime.
- Stock data has `[GAGATE_A] Gate=yes`, `DeployTime=.044`, and `GateCloseDelay=.2` in `ini/rulesmd.ini:17186..17206`.

Rust evidence:

- `ObjectType` parses `Gate`, `DeployTime`, and `GateCloseDelay` in `src/rules/object_type.rs:346`, `src/rules/object_type.rs:969`, and `src/rules/object_type.rs:972`.
- Gate passability is represented as `mission_18_active && phase == OpenStable` in `src/sim/game_entity.rs:118`.
- Stable-open gates are skipped in the live building-entry skip map for infantry/unit movers in `src/sim/movement/movement_occupancy.rs:291`, `src/sim/movement/movement_occupancy.rs:320`, and `src/sim/movement/movement_occupancy.rs:324`.
- Closed/opening friendly gates are not converted to infantry allied-gate code `3`; after the open request, `classify_blocker` returns `FriendlyStationary`, whose `yr_code()` is `6`, in `src/sim/pathfinding/cell_entry.rs:76` and `src/sim/pathfinding/cell_entry.rs:615`.

## Stage Results

| Stage | Concrete state | gamemd output | Rust output | Verdict |
|---|---:|---:|---:|---|
| Data flag | `[GAGATE_A] Gate=yes` | `Gate flag = 1` | `ObjectType.gate = true` | PASS |
| Closed stable friendly gate | `Gate=1`, mission not `0x18` / helper false | `CanGarrison=0`, allied result `max(0,3)=3` | `can_garrison_passable=false`, no skip, friendly stationary result `6` | FAIL |
| Opening friendly gate | `Gate=1`, mission `0x18`, helper not stable-open | `CanGarrison=0`, allied result `max(0,3)=3` | `can_garrison_passable=false`, no skip, friendly stationary result `6` | FAIL |
| Mission `0x18` stable-open friendly gate | `Gate=1`, mission `0x18`, helper bytes `+0x18=0`, `+0x19=1` | `CanGarrison=1`, no gate upgrade; with no other blocker result remains `0` | stable-open skip removes the building blocker; clear result `0` | PASS |
| Helper-byte internal parity | Native stores/read-checks two helper bytes | `+0x18=0`, `+0x19=1` required | Rust stores enum/bool state, not native helper bytes | UNCHECKED |
| Full runtime path timing | A*/walk call vtable `+0x1AC` in active YR | active standard-YR path verified | Rust path identified, but no live frame-by-frame retail-vs-Rust run was captured | UNCHECKED |
| Open request side effect | Friendly contact with closed gate | writer side outside this read-side contract | `request_gate_open_for_cell` assigns mission-open state before classification | PASS for Rust side effect only; not counted as passability parity |

## Failure Details

### Closed friendly gate returns code 6 instead of code 3

Player-visible problem: infantry treating a closed allied gate as generic friendly stationary occupancy can produce the wrong movement response class. Native uses code `3`, the allied gate soft-block/scatter-required class, while Rust returns code `6`.

Mechanism difference:

- gamemd: `Gate=yes` branch calls `CanGarrison`; false + allied owner writes `EBX = 3` if current result is below `3`.
- Rust: closed gate is not skipped, then generic blocker classification sees friendly stationary structure and returns `CellEntryResult::FriendlyStationary`, whose YR code is `6`.

Rust touchpoint: `src/sim/pathfinding/cell_entry.rs:615`.

Native evidence: `InfantryClass::Can_Enter_Cell @ 0x0051BF90`, branch `0x0051C4EB..0x0051C528`.

### Opening friendly gate returns code 6 instead of code 3

Player-visible problem: the same wrong response class persists while the gate is opening. Native still blocks as allied gate code `3` until the helper reports stable-open.

Mechanism difference:

- gamemd: mission `0x18` alone is insufficient; helper `0x004A51B0` must also return true.
- Rust: `phase == Opening` makes `can_garrison_passable=false`, but the fallback result is still generic friendly stationary code `6`, not the gate-specific code `3`.

Rust touchpoint: `src/sim/game_entity.rs:118` and `src/sim/pathfinding/cell_entry.rs:615`.

Native evidence: `BuildingClass::CanGarrison @ 0x004525F0`, helper `0x004A51B0`, and `InfantryClass::Can_Enter_Cell @ 0x0051C4F5..0x0051C528`.

## Adjacent Findings

- Enemy gate result codes (`5` vs `7`) are outside this slot and were not traced here.
- Stable-open skip currently applies to Unit and Infantry movers; this trace only validates the infantry friendly-gate result for the concrete scenario.
- Rust does not store native helper bytes `Building+0x350+0x18/+0x19`; it stores a higher-level enum/boolean phase. The numeric passability result is checked for this scenario, but byte-identical state representation is unchecked.
- No retail-vs-Rust runtime capture was run in this slot; timing and pathfinder side effects beyond the computed result code remain unchecked.

## Sources

- `docs/research/INFANTRY_GATE_CANGARRISON_RESULT_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`
- Read-only Ghidra spot checks: `0x004525F0`, `0x004A51B0`, `0x0051BF90`.
- Rust files read: `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/game_entity.rs`, `src/sim/gate_runtime.rs`, `src/rules/object_type.rs`.
