# RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY

**Date:** 2026-05-21  
**Mode:** exhaustive-slice for direct stock `gamemd.exe` radio message `0x07` senders and `UnitClass::Receive_Radio` case `0x07` reachability.  
**Scope guard:** This report does not expand into the full radio protocol, carryall landing, UnitRepair, or refinery unload state machines except where needed to prove `0x07` sender/reachability claims.

## Executive Summary

Verified binary evidence supports one direct stock sender of radio message `0x07`: `AircraftClass::Mission_Move_Carryall @ 0x00416D50`, in its VALIDATE_LZ pickup handshake. It sends `Transmit_Radio_ToFirst(7)` only after HELLO (`0x02`) and WANT_RIDE (`0x24`) both return ROGER.

`UnitClass::Receive_Radio` case `0x07` at `0x0073750A` is real and reachable when a UnitClass object receives that carryall message. It is not reached by standard refinery DockUnload for `HARV`/`CMIN`: the verified refinery unload path does not transmit `0x07`.

The previous claim that `BuildingClass::MissionRepairAndProduce` sends `0x07` for UnitRepair completion is contradicted by this pass. In `BuildingClass::MissionRepairAndProduce @ 0x0044B780`, the observed `PUSH 0x7` instructions are animation-slot arguments (`CALL 0x00451E40` / `CALL 0x00451890`), not radio sends. The actual radio-like virtual sends in the UnitRepair/repair-produce body use `0x13`, `0x1C`, `0x1D`, `0x1F`, and `0x03`, not `0x07`.

## Prior Research State

Relevant prior docs agreed that `UnitClass::Receive_Radio` contains a direct case `0x07`, and that `UnitClass::Mission_Deploy_Building` does not transmit `0x07`. They disagreed or overreached on the real sender:

- `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` listed two senders: carryall and `BuildingClass::MissionRepairAndProduce`.
- `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` left the actual `0x07` sender open and suggested UnitRepair as possible.
- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` states no standard refinery `0x07`, and lists carryall/UnitRepair as non-refinery paths.
- `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` proved `FootClass::Mission_Enter`, `ReleaseDockedHarvester`, and `UndockUnit` do not send `0x07`, but left the true sender open.

## Verified Findings

### 1. Carryall is a real direct `0x07` sender

**Verified binary finding:** In `AircraftClass::Mission_Move_Carryall @ 0x00416D50`, the VALIDATE_LZ state calls:

- `vtable+0x278(0x02, cargo)` for HELLO.
- `vtable+0x274(0x24)` for WANT_RIDE.
- `vtable+0x274(7)` after the `0x24` reply is ROGER.

**Evidence:** `decompile_function AircraftClass__Mission_Move_Carryall` shows the sequence in state `param_1[0x2f] == 0`; assembly at `0x00416EBF` is `PUSH 0x7`, followed by `CALL dword ptr [EAX + 0x274]` at `0x00416EC3`.

**Active in YR:** Conditional. The path is live engine code and stock `rulesmd.ini` defines `[HIND] Carryall=yes` at `ini/rulesmd.ini:10822`, but `[HIND]` has `TechLevel=-1`; in ordinary skirmish this is not a normal player-built unit. It can be active if a stock scenario/map or rules setup creates/uses a carryall.

### 2. `UnitClass::Receive_Radio` case `0x07` is real

**Verified binary finding:** `UnitClass::Receive_Radio @ 0x00737430` dispatches `param_3 == 0x07` to entry `0x0073750A`.

The case first delegates to `FootClass::Receive_Radio(sender, 0x07, payload)`, then clears destination/path/mission and stops locomotion:

- `0x00737517`: calls `FootClass::Receive_Radio @ 0x004D8FB0`.
- `0x00737524`: calls vtable `+0x480` with `(0, 1)`.
- `0x00737530`: calls vtable `+0x3C8` with `0`.
- `0x0073753E`: calls vtable `+0x1E8` with `(0, 0)`.
- `0x00737546`: calls `0x004DA1C0`, the locomotor stop wrapper.

**Active in YR:** Conditional. It is active when a UnitClass object receives `0x07`; this is proven for carryall pickup. It is not active in standard refinery DockUnload.

### 3. Case `0x07` can reply with `0x02` and `0x18`, but only after its cleanup

**Verified binary finding:** After cleanup, case `0x07` reads `this+0x418`. If `+0x418` is clear, or if `FootClass__GetDestination(0)` returns null, it sends two follow-up messages:

- `0x00737569`: `vtable+0x278(0x02, sender)`.
- `0x00737575`: `vtable+0x274(0x18)`.

If `this+0x418` is set and `GetDestination(0)` returns non-null, it skips those two sends and returns ROGER.

**Active in YR:** Conditional. Active only on actual `0x07` receipt, therefore carryall path for UnitClass cargo. No standard refinery DockUnload trigger was found.

### 4. Standard refinery DockUnload does not send `0x07`

**Verified binary finding:** In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, the relevant refinery unload radio send observed in state-4/exit context is `0x03` BREAK, not `0x07`. The `PUSH 0x7` in that function is an animation-slot call, not a radio send:

- `0x0073E08A`: `PUSH 0x7`, then `CALL 0x00451750` on `BuildingClass__SetAnimSlotImage` style helper.
- `0x0073E279`: `CALL dword ptr [EDX + 0x274]` after `PUSH 0x3`.

Prior refinery-doc findings that `Mission_Deploy_Building` contains no `PUSH 0x7` radio send are corroborated; this pass narrows the one `0x7` immediate in the function to animation-slot use.

**Active in YR:** Yes for refinery DockUnload; negative finding. Standard `HARV`/`CMIN` refinery unload exits without `0x07`.

### 5. `BuildingClass::MissionRepairAndProduce` does not send radio `0x07`

**Verified binary finding:** `BuildingClass::MissionRepairAndProduce @ 0x0044B780` contains `PUSH 0x7` sites, but they are not radio sends:

- `0x0044B7EC`: `PUSH 0x7`, then `CALL 0x00451E40`.
- `0x0044B84F`: `PUSH 0x7`, then `CALL 0x00451890`.

Both are in animation/slot setup/clear context. The radio-like virtual sends observed in the repair/produce body are other message codes:

- `0x0044B924`: `vtable+0x274(0x1C)` repair tick.
- `0x0044C2EA`: `vtable+0x274(0x1C)` repair tick.
- `0x0044C8AC`: `vtable+0x278(0x13, target)` need/move style query.
- `0x0044C8D?` branch sends `0x1F` / `0x1C` checks.
- `0x0044C?` branches send `0x03` BREAK on abort/teardown.

No `vtable+0x274(7)`, `vtable+0x278(7, target)`, or direct radio call with immediate `7` was found in this function.

**Active in YR:** Yes for UnitRepair/repair-produce mission code, but negative for `0x07`. Service depot style UnitRepair paths are active in stock rules (`UnitRepair=yes` in `ini/rules.ini` and `ini/rulesmd.ini`), yet this function does not use radio `0x07` as its completion signal.

## Reachability Verdict For Harvesters / Chrono Miners

**Standard refinery DockUnload:** `UnitClass::Receive_Radio` case `0x07` is not reached for stock `HARV`/`CMIN` refinery unload. The normal unload/departure path is driven by `UnitClass::Mission_Deploy_Building`, adjacent refinery rediscovery, mission transitions, dock-active flags, and BREAK (`0x03`) where applicable; no verified `0x07` sender exists in this path.

**Other YR path:** case `0x07` is live for UnitClass cargo in carryall pickup. If the cargo unit is a harvester/chrono miner in a scenario where a carryall targets it, the case can run, but that is not refinery DockUnload.

**UnitRepair path:** prior docs should not use UnitRepair as a confirmed `0x07` sender. This pass found the opposite: `MissionRepairAndProduce` uses repair messages and mission/destination transitions, not radio `0x07`.

## Cross-Doc Contradictions

1. `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` says `BuildingClass::MissionRepairAndProduce` sends `0x07` for UnitRepair completion. This is contradicted by live decompile and assembly context of `0x0044B780`.
2. `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` currently lists `0x07` as "UnitRepair/carryall path only." The carryall half is verified; the UnitRepair half should be downgraded or corrected.
3. `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` says case `0x07` fires when a refinery sends DOCKING_COMPLETE to abort a queued harvester. This pass found no standard refinery sender; the receiver behavior is real, but the refinery reachability statement should be corrected unless another non-standard/refinery abort sender is later proven.

## Open Questions Deferred

- Is there a computed non-immediate radio-message sender that can pass value `7` into `Transmit_Radio` outside the direct `PUSH 0x7` inventory? None was found in the bounded sender scan or relevant functions, but this report does not claim a whole-program dataflow proof over computed message variables.
- Does any campaign script or map action create a stock `[HIND]`/carryall in a way that makes harvester/CMIN pickup common? This affects frequency, not the binary reachability verdict.

## Sources

- Ghidra `decompile_function AircraftClass__Mission_Move_Carryall`.
- Ghidra assembly context: `0x00416EBF` -> `vtable+0x274(7)`.
- Ghidra assembly context: `0x0073750A` through `0x00737575` for `UnitClass::Receive_Radio` case `0x07`.
- Ghidra `decompile_function BuildingClass__MissionRepairAndProduce`.
- Ghidra assembly context: `0x0044B7EC`, `0x0044B84F`, `0x0044B924`, `0x0044C2EA`, `0x0044C8AC`.
- Ghidra assembly context: `0x0073E08A` and `0x0073E279` for `UnitClass::Mission_Deploy_Building`.
- INI evidence: `ini/rulesmd.ini:10822` (`[HIND] Carryall=yes`), `ini/rules.ini` / `ini/rulesmd.ini` UnitRepair entries.
