# Bridge Deep Slot 3 - Engineer Repair Action Path Into CABHUT

Trace date: 2026-05-22

Scenario: A player selects an Engineer and targets a CABHUT serving a damaged high bridge. Scope is only the action path into the CABHUT repair branch: cursor/action code, command emission, sim command validation, movement target/mission setup, trigger position/cell, scan origin, engineer consumption, and no CABHUT owner transfer.

Adjacent mechanics intentionally not traced here: high-bridge overlay repair mutations, repaired-variant RNG, radar dirty propagation, sound/EVA ordering, multiple engineers, C4, hut death, and low bridges.

## Verdict Summary

Current Rust is closer than the older action-gate trace: the player click path now accepts `BridgeRepairHut=yes` even though CABHUT is not capturable, and sim validation now sets `capture_target` for CABHUT. However, this path is still not fully on `gamemd.exe` parity.

The two important remaining differences are player-visible or timing-visible:

1. Rust shows the generic Enter cursor for CABHUT bridge repair; `gamemd.exe` returns the bridge-repair action code `0x1D` for a visible hut cell, or `0x20` when the hut cell has no radar color.
2. Rust fires bridge repair when the engineer is Chebyshev-adjacent to the hut (`dx <= 1 && dy <= 1`); `gamemd.exe` runs the bridge-repair branch from `InfantryClass::PerCellProcess` only after the infantry's current cell lookup resolves to the target building. For the concrete adjacent cell `(10,10)` next to a hut at `(9,10)`, Rust repairs and consumes the engineer; `gamemd.exe` would not fire that branch until the hut-cell condition is true.

Verdict tally: PASS: 4 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Evidence Baseline

Retail INI facts:

- `ini/rulesmd.ini:3833`: `[ENGINEER] Engineer=yes`.
- `ini/rulesmd.ini:16336-16348`: `[CABHUT] Repairable=true`, `BridgeRepairHut=yes`, no `Capturable=`.
- `ini/rulesmd.ini:721`: `RepairBridgeSound= BridgeRepaired`.

Active-YR binary facts, read-only:

- `InfantryClass::What_Action_OnObject @ 0x0051E3B0` is active in standard YR and returns bridge-repair action from the engineer-on-building block before Capturable fallback when `target.Type+0x16B6 != 0`.
- `InfantryClass::PerCellProcess @ 0x00519630` is active in standard YR and handles missions `8`, `0xB`, and `0x19`; inside that branch, it requires the building in the infantry's current cell to match the target before running the `BridgeRepairHut` repair branch.
- `TECH_CABHUT_GHIDRA_REPORT.md` confirms `BridgeRepairHut=` is live in YR and that the bridge-repair branch has no owner-transfer call.
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` confirms the repair-side bridge dispatchers are runtime reachable from engineer entering CABHUT.

Current Rust facts:

- `src/app_cursor.rs:265-270` recognizes selected Engineer over `BridgeRepairHut=yes`, but returns `CursorFeedbackKind::Enter`.
- `src/app_cursor.rs:530` maps `CursorFeedbackKind::Enter` to `CursorId::Enter`.
- `src/render/cursor_atlas.rs:99-104` renders `CursorId::Enter` as `mouse.shp` frames `89..98`.
- `src/app_context_order.rs:388` allows engineer click targets when `obj.capturable || obj.bridge_repair_hut`.
- `src/sim/world/world_commands.rs:1082` allows command validation when `obj.capturable || obj.bridge_repair_hut`.
- `src/sim/world/world_orders.rs:307-328` triggers bridge repair from Chebyshev adjacency.
- `src/sim/world/world_orders.rs:340-345` scans 5x5 from the hut/building cell.
- `src/sim/world/world_orders.rs:360-361` consumes the engineer.
- `src/sim/world/world_orders.rs:186-199` skips `BridgeRepairHut=yes` targets in normal capture fallback.

## Stage Results

| Stage | Boundary | gamemd.exe output | Rust output | Verdict |
|---|---|---|---|---|
| 1 | INI/type gate | Engineer flag true; CABHUT `Repairable=true`, `BridgeRepairHut=yes`, `Capturable=false`; path is active in YR. | Same flags are parsed and consulted for this path. | PASS |
| 2 | Cursor/action code | Visible hut cell: action `0x1D` (29). No radar color: action `0x20` (32). This is the bridge-repair cursor branch. | `CursorFeedbackKind::Enter` -> `CursorId::Enter` -> frames `89..98`. No distinct bridge-repair action/cursor mapping is produced for CABHUT. | FAIL |
| 3 | Player click command emission | Click uses the Enter/Capture-style mission path; doc table records mission `0x08` for click-to-hut, then PerCellProcess specializes by hut type. | `Command::CaptureBuilding { engineer_id, target_building_id }` is emitted for `BridgeRepairHut=yes`. No numeric mission id is exposed by Rust. | UNCHECKED |
| 4 | Sim command validation | CABHUT bridge repair does not require `Capturable=yes`; bridge hut branch wins before capture fallback. | Validation accepts CABHUT because `!capturable && !bridge_repair_hut` is false, then sets `capture_target`. | PASS |
| 5 | Movement target / mission setup | Target building pointer is stored; repair path is under missions `8`, `0xB`, or `0x19`; the relevant building-cell comparison must later succeed. Exact mission setter was not re-derived in this slot. | `capture_target = Some(cabhut)` and `issue_move_command_with_layered(..., (trx, try_))` where `(trx, try_)` is the hut position. No numeric mission state exists. | UNCHECKED |
| 6 | Trigger position/cell | Branch requires `Look_up_building_in_cell() == target`; for hut `(9,10)`, an engineer at adjacent `(10,10)` does not satisfy equality. Effective cell distance at trigger is `0` cells from hut cell. | Branch triggers at `dx <= 1 && dy <= 1`; for hut `(9,10)`, engineer `(10,10)` has `dx=1`, `dy=0` and repairs immediately. | FAIL |
| 7 | 5x5 scan origin | Eligible branch scans around the building/current hut cell; because the branch requires current building cell match, the scan origin is the hut cell. | Scans `cells_in_5x5_scan((brx,bry))`, where `(brx,bry)` is the hut cell. For hut `(9,10)`, origin is `(9,10)`. | PASS |
| 8 | Engineer consumption + owner transfer | Bridge-hut branch limbos one engineer via vtable `+0xF8`; no `ChangeOwner`, no credits, hut keeps previous owner. | `despawn_entity(engineer_id)` consumes one engineer; bridge-hut targets are skipped by normal capture, and repair path does not write hut owner. | PASS |

## Player-Visible Failures

1. Wrong cursor identity. The player sees a generic Enter cursor over CABHUT rather than the dedicated bridge-repair action that `gamemd.exe` returns (`0x1D`/`0x20`). Current Rust: `src/app_cursor.rs:268-269`, `src/render/cursor_atlas.rs:99-104`. Binary evidence: `InfantryClass::What_Action_OnObject @ 0x0051E3B0`, bridge-hut branch returns `(-(radarColor != 0) & 0xfffffffd) + 0x20`.

2. Repair triggers one cell too early. The player can repair/consume an engineer while merely adjacent to CABHUT in Rust. `gamemd.exe` waits for the PerCellProcess building-cell equality path. Current Rust: `src/sim/world/world_orders.rs:307-328`. Binary evidence: `InfantryClass::PerCellProcess @ 0x00519630`, repair branch is downstream of `Look_up_building_in_cell()` matching `param_1[0x169]`.

## Unchecked Items

1. Exact Rust command id vs `gamemd.exe` mission id. Rust uses a typed enum (`Command::CaptureBuilding`) and does not expose numeric mission `0x08`, so literal numeric equality was not computed.

2. Exact mission-setter call for the click path. The binary PerCellProcess mission consumers are verified (`8`, `0xB`, `0x19`), but this slot did not decompile the click command handler that writes mission `8`.

## Adjacent Findings

- The high-bridge repair mutation path and repaired overlay variant RNG belong to the bridge-repair mutation slot, not this action-path slot.
- Sound/EVA ordering belongs to the audio/render presentation slot.
- C4 and CABHUT destruction are separate mechanics and were not traced here.
- Current Rust comments still describe the bridge-repair order as adjacent-by-design in places; that is an implementation note, not gamemd parity.

## Sources

- `docs/research/TECH_CABHUT_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `docs/research/traces/ENGINEER_CABHUT_ACTION_GATE_HIGH_BRIDGE_TRACE.md`
- `ini/rulesmd.ini`
- `src/app_cursor.rs`
- `src/app_context_order.rs`
- `src/render/cursor_atlas.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/world/world_orders.rs`

