# UnitClass PerCellProcess GetDockCoord vs 0x16 Reconciliation - Ghidra Report

**Date:** 2026-05-24  
**Target:** Audit `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md` against newer `0x16` and DriveLocomotor findings.  
**Focus:** Section 9 implementation handoff and OQ-7.  
**Active in YR:** Yes. Standard YR stock `[CMIN]`/`[HARV] -> [GAREFN]`/`[NAREFN]` reaches the documented Mission Enter, Building `0x0E`, Unit `0x16`, Drive arrival, and Unit `PerCellProcess` paths through `DockUnload=yes`, `Refinery=yes`, and harvester `Dock=`.

## 0. Working Notes

- Target question: Which claims in the older PerCellProcess `GetDockCoord` report remain valid after the newer `0x16` and DriveLocomotor findings, and exactly how should Section 9 and OQ-7 be replaced?
- Non-goals: Do not re-audit the whole refinery FSM, non-stock docks, full locomotor physics, Rust implementation, INI parser flag identity, or exact first-source runtime trace beyond the settled documents.
- Evidence needed to mark COMPLETE: Ghidra spot-checks for `UnitClass::PerCellProcess @ 0x00739EC0`, `UnitClass::Receive_Radio @ 0x00737430` case `0x16`, `BuildingClass::Receive_Radio @ 0x0043C2D0`, `BuildingClass::GetDockCoord @ 0x00447B20`, and Drive arrival / `Is_Moving_Now` evidence from `0x004B0F20` and `0x004AFC20`, plus reconciliation against the canonical synthesis.
- Stop conditions: Stop once Section 9 stale wording and OQ-7 are resolved without expanding into full first-frame source-winner proof or code changes.

## 1. Bottom Line

The older report's core PerCellProcess claim remains valid but its Section 9 handoff is too strong.

Valid: `UnitClass::PerCellProcess @ 0x00739EC0` has a `GetDockCoord` equality branch. It sends radio `0x15` only after the unit's current cell equals the destination building's `GetDockCoord` cell. For stock 4x3 GAREFN/NAREFN, that coordinate is `NW+(2,1)`.

Stale: the old handoff implies Rust should withhold the `0x15`/Linked handoff until the miner physically/currently reaches `NW+(2,1)`, "or until the verified `0x16` bridge says otherwise." Newer evidence proves the `0x16` path is exactly that exception, but it is not a bridge/move. Later/already-synced `0x16` can send `0x15` from stopped accepted `NW+(3,1)` when the unit is idle, `+0x418` is set, destination is a building, and the receiving unit mission is `7`.

Therefore OQ-7 is resolved as a negative fact: there is no required upstream move/synchronization from accepted `NW+(3,1)` to `GetDockCoord` `NW+(2,1)` for the `0x16` path. The relationship is two independent `0x15` sources with different gates.

## 2. Claims That Remain Valid

| Older claim | Reconciled verdict | Evidence | Active in YR |
|---|---|---|---|
| PerCellProcess calls destination building vtable `+0xA8` / `GetDockCoord`. | Valid. | Decompile `0x00739EC0`; disassembly `0x0073A391..0x0073A3B1`. | Yes |
| PerCellProcess compares current unit cell to returned dock coordinate before this branch sends `0x15`. | Valid. | Disassembly `0x0073A369`, `0x0073A3B1`, `0x0073A417..0x0073A437`, send at `0x0073A503..0x0073A507`. | Yes |
| Stock accepted `0x0E` / `0x12` target is `NW+(3,1)` and separate from `GetDockCoord`. | Valid. | `BuildingClass::Receive_Radio @ 0x0043C2D0`; accepted payload built from building cell `+3,+1`; canonical synthesis lines 27, 43. | Yes |
| Stock `GetDockCoord` for 4x3 refinery is `NW+(2,1)`. | Valid. | `BuildingClass::GetDockCoord @ 0x00447B20`; `+0x16BB Refinery=yes` branch adds `+0x80` X to building coords. | Yes |
| `0x00739EC0` is a per-cell hook, not the Mission Enter dispatch handler. | Valid and important. | `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`; `MissionClass::Mission_Dispatch` case 7 uses `FootClass::Mission_Enter @ 0x004D9290`. | Yes |

## 3. Stale Or Overbroad Claims

| Older wording / implication | Replacement verdict | Evidence |
|---|---|---|
| "Do not fire the `0x15`/Linked handoff until the miner's physical/current cell is the dock coord `(12,11)` or until the verified `0x16` bridge says otherwise." | Stale because `0x16` is verified and is not a move/bridge. Say: do not fire the PerCellProcess `GetDockCoord` branch until current cell equals `NW+(2,1)`, but a later/already-synced `0x16` may send `0x15` from stopped accepted `NW+(3,1)` without `GetDockCoord` equality. | `0x00737430` disassembly `0x007376AD..0x00737783`; Drive report; canonical synthesis lines 50-51, 106, 130. |
| "Gate the unload FSM on the verified pad-arrival event, not on accepted-cell arrival alone." | Too broad. Say: gate unload on a verified `0x15` source. The source may be PerCellProcess `GetDockCoord` equality, later/already-synced `0x16`, or the later contact-flag adjacent-building PerCellProcess branch. | `0x00739EC0` disassembly `0x0073A503..0x0073A507` and `0x0073A558..0x0073A5C8`; `0x00737430` `0x16` branch. |
| OQ-7 asks which upstream call moves/synchronizes accepted `(rx+3,ry+1)` to dock `(rx+2,ry+1)`. | Resolved as "no required move for `0x16`; the old premise is false for the 0x16 path." Physical `GetDockCoord` crossing remains only relevant to the PerCellProcess equality source. | Drive arrival decompile `0x004B0F20`; `Is_Moving_Now @ 0x004AFC20`; Unit `0x16 @ 0x00737430`. |
| "PerCellProcess sends `0x15` only after current cell equals `GetDockCoord`." | Valid only for the named equality branch. Overbroad for all of PerCellProcess because a later contact-flag adjacent-building branch can also send `0x15`. | `0x00739EC0` disassembly `0x0073A558..0x0073A5C8`; DriveLocomotor report section 3.6. |

## 4. Verified Binary Findings

1. `UnitClass::PerCellProcess @ 0x00739EC0` `GetDockCoord` equality branch:
   - Mission `7` or `0x19`, non-null destination, destination `WhatAmI()==6`, current coord via unit vtable `+0x48`, destination dock coord via building vtable `+0xA8`, sign-correct cell conversion, X/Y compare, then `FootClass::PerCellProcess(2)`, radio `0x15`, locomotor `+0x5C`.
   - Evidence: decompile plus disassembly `0x0073A324..0x0073A359`, `0x0073A369`, `0x0073A391..0x0073A3B1`, `0x0073A417..0x0073A437`, `0x0073A4F7..0x0073A52B`.
   - Active in YR: Yes.

2. `UnitClass::PerCellProcess @ 0x00739EC0` later contact-flag adjacent-building branch:
   - Requires `Unit+0x418 != 0`, non-null building destination, unit mission `7`, then checks cell `(current_x,current_y-1)` for that destination building and can transmit `0x15`.
   - Evidence: decompile plus disassembly `0x0073A558..0x0073A5C8`.
   - Active in YR: Conditional; live code, gated by `+0x418` set by radio `0x18`.

3. `UnitClass::Receive_Radio @ 0x00737430` case `0x16`:
   - First ordinary unsynced call can call locomotor vtable `+0x4C(0x4000)` and return `1` before any `0x15`.
   - Later/already-synced path checks locomotor `+0x10` / not moving, `FootClass::GetDestination`, `Unit+0x418`, destination `WhatAmI()==6`, and unit mission `7`, then sends `0x15`.
   - Evidence: decompile plus disassembly `0x007376AD..0x00737783`.
   - Active in YR: Yes for stock refinery `0x18/0x16` handoff.

4. `0x16` contains no `GetDockCoord`, no `Set_Destination`, no `MOVE_TO_CELL`, and no location write.
   - Evidence: complete case `0x16` body at `0x007376AD..0x00737783`; no destination vtable `+0xA8` call and no coordinate compare in the case.
   - Active in YR: Yes.

5. Drive arrival can expose stopped accepted-cell state:
   - `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` clears Drive head-to/track fields and calls owner `+0x504` after arrival; `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20` can return false without requiring `Foot+0x5A4` to be null.
   - Evidence: decompile `0x004B0F20`; decompile `0x004AFC20`; DriveLocomotor report sections 3.2-3.5.
   - Active in YR: Yes for Drive-locomotor miners; CMIN ground approach uses Drive piggyback/ground movement in the covered path.

## 5. Relationship Model

For stock refinery NW `(rx,ry)`:

| Mechanism | Cell relation | What it proves |
|---|---|---|
| Building `0x0E -> 0x12` accepted move | `NW+(3,1)` | Admission movement target, not `GetDockCoord`. |
| Unit `0x16 -> 0x15` | no cell equality check | Can hand off from stopped accepted state when timer/idle/destination/contact/mission gates pass. |
| PerCellProcess `GetDockCoord -> 0x15` | current cell == `NW+(2,1)` | Separate cell-entry source if the unit physically reaches the stock dock coordinate. |
| PerCellProcess contact-flag adjacent-building -> `0x15` | building in `current_y-1` cell with `+0x418` | Separate later per-cell/contact source; not the same as the `GetDockCoord` branch. |

Inference from sources: the implementation should be source-aware. It must not model every `0x15` as "accepted cell reached", and it must not require `NW+(2,1)` before every `0x15`.

## 6. OQ-7 Final State

Old OQ-7:

> What exact upstream call moves/synchronizes the unit from accepted `(rx+3,ry+1)` to dock `(rx+2,ry+1)`?

Replacement:

> `[RESOLVED] OQ-7 - No upstream move/synchronization from accepted NW+(3,1) to stock GetDockCoord NW+(2,1) is required for the 0x16 path. Drive arrival can leave the unit stopped at accepted NW+(3,1) with destination still live; later/already-synced UnitClass::Receive_Radio(0x16) can send 0x15 from that state without GetDockCoord equality. Physical GetDockCoord equality remains a separate PerCellProcess 0x15 source if the unit actually reaches NW+(2,1).` Evidence: `0x004B0F20`, `0x004AFC20`, `0x00737430` `0x007376AD..0x00737783`, and `0x00739EC0` `0x0073A391..0x0073A52B`.

## 7. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Later/already-synced `0x16` may send `0x15` from stopped accepted `NW+(3,1)` without `GetDockCoord` equality. Evidence: `0x007376AD..0x00737783`, Drive `0x004B0F20` / `0x004AFC20`. | Split `Linked`/unload start from "physical pad cell reached"; model the `0x16` source separately from PerCellProcess equality. | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`, `phase_linked`; dock tests. | Miner at accepted `(13,11)`, idle, destination refinery, contact flag set, facing/timer already synced can enter unload handoff without current cell `(12,11)`. | `miner_dock_synced_0x16_handoff_from_stopped_accepted_cell` | High player-visible timing/position drift if Rust forces an extra pad move. |
| First ordinary unsynced `0x16` can set facing/timer and return without `0x15`. Evidence: `0x007376BF..0x0073770F`. | Do not treat receipt/return `1` from `0x16` as unload start. | `phase_mission_enter`, pivot/facing state. | First `0x16` at accepted cell starts/schedules east-facing sync but does not queue unload in same modeled step when timer is not `0x4000`. | `miner_dock_first_0x16_sets_facing_without_0x15` | Medium; affects first-frame unload/pivot ordering. |
| PerCellProcess `GetDockCoord` equality remains a real separate `0x15` source. Evidence: `0x0073A391..0x0073A52B`. | Keep a source-aware branch/test for actual current-cell `NW+(2,1)` equality; do not delete or fold it into `0x16`. | future per-cell/miner handoff integration; `refinery_pad_cell` naming/tests. | Miner physically/currently on stock `GetDockCoord` sends `0x15` through PerCellProcess even though accepted target is distinct. | `miner_dock_getdockcoord_percellprocess_sends_0x15` | Medium; losing this branch breaks non-0x16 source parity. |
| PerCellProcess has a later `+0x418` adjacent-building `0x15` branch. Evidence: `0x0073A558..0x0073A5C8`. | Do not document or implement PerCellProcess as single-source `GetDockCoord` only. | future per-cell/contact branch modeling. | With contact flag set and destination building found one row north, PerCellProcess can attempt `0x15` without the `GetDockCoord` equality branch. | `miner_dock_contact_flag_adjacent_branch_can_send_0x15` | Medium; exact first-winner still runtime-sensitive. |

## 8. Negative Facts / Do Not Do

- Do not require a physical move from accepted `NW+(3,1)` to `GetDockCoord` `NW+(2,1)` before every `0x15`; `0x16` can send `0x15` without `GetDockCoord`. Evidence: `0x007376AD..0x00737783`.
- Do not call `0x16` a bridge or mover; it has no `GetDockCoord`, no `Set_Destination`, no `MOVE_TO_CELL`, and no location write. Evidence: `0x00737430` case `0x16`.
- Do not say `Is_Moving == false` means the refinery destination is gone; Drive `Is_Moving_Now` can be false while `Foot+0x5A4` remains live. Evidence: `0x004AFC20`, `0x004B0F20`.
- Do not collapse accepted cell, `GetDockCoord`, and QueueingCell. Accepted is `NW+(3,1)`, stock `GetDockCoord` is `NW+(2,1)`, QueueingCell is `NW+(4,1)`. Evidence: canonical synthesis coordinate table.
- Do not say all PerCellProcess `0x15` sends require `GetDockCoord` equality; that is true only for the equality branch. Evidence: contact-flag branch `0x0073A558..0x0073A5C8`.

## 9. Remaining Uncertainty

- Exact first `0x15` source in every concrete replay frame remains runtime-sensitive: later/aligned `0x16`, PerCellProcess `GetDockCoord`, or PerCellProcess contact-flag adjacent-building.
- Exact facing timer duration from first unsynced `0x16` to `RateTimer::Current(+0x388)==0x4000` is not re-audited here.
- Rust-facing phase naming and pad occupancy semantics still need implementation work; this report only replaces the stale handoff wording.

## 10. Stale-Doc Replacement Wording

For `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md` Section 9, replace the first handoff row with:

> Stock `0x0E` accepted movement remains `NW+(3,1)`, while stock `GetDockCoord` for 4x3 GAREFN/NAREFN remains `NW+(2,1)`. `UnitClass::PerCellProcess @ 0x00739EC0` has a `GetDockCoord` equality branch that sends `0x15` only when current cell equals `NW+(2,1)`. This branch is not the only possible `0x15` source: later/already-synced `UnitClass::Receive_Radio(0x16)` can send `0x15` from stopped accepted `NW+(3,1)` without `GetDockCoord` equality, and PerCellProcess also has a later `+0x418` adjacent-building branch. Rust must preserve source-aware handoffs instead of forcing all unload starts through a physical `NW+(2,1)` move.

Replace old OQ-7 with the text in section 6 above.

Replace "Gate the unload FSM on the verified pad-arrival event, not on accepted-cell arrival alone" with:

> Gate unload on a verified `0x15` source. Accepted-cell arrival alone is insufficient, but `0x15` can come from later/already-synced `0x16` while still at accepted `NW+(3,1)`, from PerCellProcess `GetDockCoord` equality at `NW+(2,1)`, or from the later contact-flag adjacent-building PerCellProcess branch.

## Sources

- Ghidra decompile/disassembly `UnitClass::PerCellProcess @ 0x00739EC0`.
- Ghidra decompile/disassembly `UnitClass::Receive_Radio @ 0x00737430`.
- Ghidra decompile `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra decompile `BuildingClass::GetDockCoord @ 0x00447B20`.
- Ghidra decompile `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`.
- Ghidra decompile `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`.
