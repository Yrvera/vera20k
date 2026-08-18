# Engineer CABHUT Action Gate High Bridge Trace

Trace date: 2026-05-20

Scenario: selected Allied Engineer targets a CABHUT serving a damaged high bridge.

Scope: action/cursor/order/mission gate only. Bridge overlay mutation, sound/EVA ordering, low-bridge behavior, and multi-engineer same-tick behavior are adjacent findings and not traced here.

## Verdict Summary

The Rust player-input gate does not reach bridge repair for the concrete scenario. `gamemd.exe` classifies an Engineer targeting a `BridgeRepairHut=yes` CABHUT as the special bridge-repair action before capturable fallback. Rust classifies the same visible target through the generic enemy-structure path because the engineer-specific cursor and click branches only recognize capturable enemy buildings or damaged friendly structures. Since CABHUT omits `Capturable=`, no real player click sets `capture_target`, so `tick_bridge_repair_orders` is only reachable from tests or manually seeded state.

Verdict tally: PASS: 2 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Scenario Data

- Allied Engineer: `ini/rulesmd.ini:3817-3833` has `[ENGINEER]`, `Primary=DefuseKit`, `Secondary=VirtualScanner`, `Speed=4`, `Engineer=yes`.
- CABHUT: `ini/rulesmd.ini:16336-16348` has `Strength=2000`, `Immune=yes`, `Repairable=true`, `Selectable=yes`, `BridgeRepairHut=yes`.
- CABHUT explicitly lacks `Capturable=`. `TECH_CABHUT_GHIDRA_REPORT.md:89-92` records absent flags defaulting false, and `:122` verifies `Capturable=` defaults to 0 at `BuildingTypeClass+0x1572`.
- Active-YR check: `TECH_CABHUT_GHIDRA_REPORT.md:45-48` states CABHUT and the repair/destruction mechanics are active in stock YR maps; `:528` marks `BridgeRepairHut=` live in YR via parse and runtime gates.

## Pipeline

1. Data gate: Engineer and CABHUT flags load.
2. Cursor/action classifier: hover/right-click target classification.
3. App click dispatch: player command envelope selection.
4. Sim command validation: `CaptureBuilding` writes or rejects `capture_target`.
5. Mission/tick gate: bridge repair consumes an engineer whose target is a bridge hut.
6. Capture fallback: CABHUT must not become owned by the engineer's house.

## Stage Results

### Stage 1 - INI and Type Flags

Rust:
- `ObjectType::from_ini_section` parses `engineer`, `capturable`, `repairable`, and `bridge_repair_hut` at `src/rules/object_type.rs:978-986`.
- CABHUT values from merged YR INI compute to `repairable=true`, `bridge_repair_hut=true`, `capturable=false`.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:116-123` verifies `Repairable=`, `BridgeRepairHut=`, and `Capturable=` parse offsets and defaults.
- `TECH_CABHUT_GHIDRA_REPORT.md:129` verifies `Engineer=` parse/write for the infantry type used by Engineer.

Verdict: PASS. The data bits needed by the gate are present in Rust and match the verified gamemd values.

### Stage 2 - Cursor / Action Classifier

Rust:
- `capability_cursor_for_hover` checks `sel_obj.engineer` at `src/app_cursor.rs:265`.
- Enemy structure branch only returns `Enter` when `hovered_obj.capturable` is true at `src/app_cursor.rs:267-270`.
- Friendly repair branch only returns `EngineerRepair` for damaged friendly structures at `src/app_cursor.rs:272-277`.
- There is no `bridge_repair_hut` check before generic enemy fallback.
- For a visible enemy/neutral CABHUT at adjacent range, the generic fallback reaches `CursorFeedbackKind::EnemyStructure`, rendered as `CursorId::Attack` at `src/app_cursor.rs:516-518`; if out of weapon range it becomes `EnemyOutOfRange`.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:146-164` shows the engineer-on-building block is gated by `Engineer`, building RTTI, human player, target not in limbo/building, and `Repairable=yes`.
- `TECH_CABHUT_GHIDRA_REPORT.md:166-212` shows the first branch inside that block is `BridgeRepairHut`, before Hospital/Capturable fallback.
- Corrected cursor action code is `0x1D` when the cell has visible radar color, `0x20` when it does not (`TECH_CABHUT_GHIDRA_REPORT.md:178-184`, audit note `:636-638`).

Verdict: FAIL. Rust output is generic attack/out-of-range cursor, gamemd output is bridge-repair action `0x1D`/`0x20`.

### Stage 3 - Player Click Command Dispatch

Rust:
- `app_context_order.rs` has a C4 branch first, then engineer capture.
- Engineer capture only accepts `HoverTargetKind::EnemyStructure` plus `obj.capturable` at `src/app_context_order.rs:378-400`.
- The only engineer command emitted here is `Command::CaptureBuilding` at `src/app_context_order.rs:417-425`, but CABHUT has `capturable=false`, so this branch returns no command for CABHUT.
- The later generic path can select the CABHUT as an enemy attack target and emit `Command::Attack` at `src/app_context_order.rs:586-597`, not a bridge repair mission.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:540-544` records the player-facing quick reference: hover Engineer over hut -> bridge-repair cursor; click Engineer on hut -> mission `0x08` Capture; stepping onto hut with mission `8/0xB/0x19` runs bridge repair.

Verdict: FAIL. Rust emits no bridge-repair/capture-enter command for CABHUT from the player click; gamemd emits an enter/capture mission that is later specialized by the hut type.

### Stage 4 - Sim Command Validation and `capture_target`

Rust:
- `Command::CaptureBuilding` validation rejects targets whose object type is not capturable at `src/sim/world/world_commands.rs:1052-1067`.
- Because CABHUT is not capturable, even a manually enqueued `CaptureBuilding` command would return before `e.capture_target = Some(target_building_id)` at `src/sim/world/world_commands.rs:1077-1083`.
- `Command` has no explicit `RepairBridgeHut` or `EnterBridgeRepairHut` variant in `src/sim/command.rs:125-131`.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:199-205` places `BridgeRepairHut` before `Capturable` in action resolution.
- `TECH_CABHUT_GHIDRA_REPORT.md:281-327` shows the later PerCellProcess branch detects the target is CABHUT and performs bridge repair, then limbos the engineer.

Verdict: FAIL. Rust's sim-side command validation still treats CABHUT through the normal capturable-building filter; gamemd does not require `Capturable=yes` for CABHUT bridge repair.

### Stage 5 - Bridge-Repair Tick Reachability

Rust:
- `tick_bridge_repair_orders` can repair only engineers already carrying `capture_target` at `src/sim/world/world_orders.rs:264-293`.
- Existing tests seed `capture_target` directly, for example `world_orders_bridge_repair_tests.rs:414`, bypassing the real player action gate.
- `advance_tick` runs `tick_bridge_repair_orders` before `tick_capture_orders` at `src/sim/world/mod.rs:1243-1248`, so seeded bridge-hut targets do avoid normal capture fallback.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:281-327` ties the live repair branch to Mission `8/0xB/0x19` and the target building type.

Verdict: NOT-IMPLEMENTED. The repair tick exists, but the real player action gate that should create the bridge-repair/enter mission for CABHUT is absent.

### Stage 6 - Arrival / Mission Trigger Position

Rust:
- `tick_bridge_repair_orders` fires when the engineer is Chebyshev-adjacent to the hut (`dx <= 1 && dy <= 1`) at `src/sim/world/world_orders.rs:307-328`.
- The Rust scan is centered on the engineer cell at `src/sim/world/world_orders.rs:340-345`.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:288-300` checks that the building in the current cell is the target, then scans for bridge cells.
- `TECH_CABHUT_GHIDRA_REPORT.md:351-353` states the 5x5 scan is centered on the hut cell and visits exactly 25 cells.

Verdict: FAIL. If the gate is manually seeded, Rust can fire from an adjacent cell and scan around the engineer; gamemd fires when the engineer enters the hut cell and scans around the hut.

### Stage 7 - Normal Capture Fallback

Rust:
- `tick_capture_orders` explicitly skips `BridgeRepairHut=yes` targets at `src/sim/world/world_orders.rs:186-199`.
- No owner transfer occurs for bridge huts through this tick path.

gamemd:
- `TECH_CABHUT_GHIDRA_REPORT.md:330-349` verifies the bridge-repair branch has no `ChangeOwner` call; CABHUT stays neutral/Special and the engineer is destroyed.

Verdict: PASS for seeded state. Once a bridge-hut target reaches the tick layer, Rust does not capture the hut, matching gamemd's no-owner-change result.

## Failures

1. Stage 2: wrong cursor/action classifier. Player sees attack/out-of-range instead of bridge-repair.
2. Stage 3: right-click does not emit an enter/capture mission for CABHUT. Player order cannot start bridge repair.
3. Stage 4: sim command validation rejects CABHUT because it is not capturable, blocking even explicit `CaptureBuilding`.
4. Stage 6: manually seeded bridge repair fires adjacent to the hut and scans around the engineer, while gamemd fires on hut-cell entry and scans around the hut.

## Not Implemented

1. Explicit bridge-hut engineer action gate from player command to sim mission. Rust currently relies on overloading `capture_target`, but the actual command path never sets it for CABHUT.

## Unchecked

1. Exact Rust cursor enum for a non-adjacent CABHUT target was not computed for every possible distance; the adjacent visible case resolves to generic enemy attack, and all non-adjacent cases remain non-bridge-repair. The failure does not depend on the range-specific generic fallback.

## Adjacent Findings

- Rust has bridge mutation and sound tests for direct `capture_target` seeding. Those are out of scope for this slot and should be handled by the bridge mutation and sound slots.
- The high-vs-low bridge walker details are out of scope here. This trace only confirms the action gate does not correctly reach the repair branch.

## Sources

- `docs/research/TECH_CABHUT_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `docs/research/ENGINEER_CAPTURE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `src/app_cursor.rs`
- `src/app_context_order.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/world/world_orders.rs`
- `src/sim/world/mod.rs`
