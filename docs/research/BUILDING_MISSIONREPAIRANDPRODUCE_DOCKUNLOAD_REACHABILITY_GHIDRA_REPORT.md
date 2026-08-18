# Building MissionRepairAndProduce DockUnload Reachability - Ghidra Research Report

**Target:** `BUILDING_MISSIONREPAIRANDPRODUCE_DOCKUNLOAD_REACHABILITY`  
**Investigation mode:** exhaustive-slice  
**Address(es):** `BuildingClass::Receive_Radio @ 0x0043C2D0`; `BuildingClass::MissionRepairAndProduce @ 0x0044B780`; `UnitClass::Mission_Deploy_Building @ 0x0073D630`; anim helpers `0x00451750`, `0x00451890`, `0x00451E40`  
**Claimed scope:** whether building mission `0x14` / `BuildingClass::MissionRepairAndProduce` participates in the stock `CMIN/HARV -> GAREFN/NAREFN` DockUnload flow after building radio `0x15`, plus the exact player-visible departure guard.  
**Non-scope:** full refinery unload credit math, exact `DAT_0089F6A0` source/value, radio `0x07` reachability, and full modded `ProductionAnim` frame lifetime.  
**Confidence:** High for stock GAREFN/NAREFN no-reachability; High for the non-refinery mission `0x14` handoff; Medium for modded edge cases that combine `DockUnload=yes` with service flags.

## Summary

`BuildingClass::MissionRepairAndProduce` is not part of the stock YR ore refinery DockUnload path.

When a stock `GAREFN` or `NAREFN` receives radio `0x15`, `BuildingClass::Receive_Radio` reaches the `DockUnload` branch at `0x0043C788..0x0043C7A0`. That branch reads `BuildingType+0x16B3` and queues mission `0x10` on the sender unit only. It does not queue building mission `0x14`, does not set building `+0x6DD`, and does not call `MissionRepairAndProduce`.

Building mission `0x14` is queued from the same radio case only for `UnitRepair`, `UnitReload`, `Hospital`, `Armory`, and `Bunker` buildings. Those flags are absent from stock `GAREFN`/`NAREFN`; stock refineries only set `DockUnload=yes` and `Refinery=yes` in the relevant gate set.

Player-visible departure timing for stock ore refinery unload is controlled by the unit-side `UnitClass::Mission_Deploy_Building` state 4. The only verified building-side wait in that state is `building+0x57C` (`Anims_0[8]` / `ProductionAnim`). Stock `GAREFN` has no `ProductionAnim`, and stock `NAREFN` has `ProductionAnim=NAREFN_AR` commented, so the guard is reached but does not delay departure.

## Verified Findings

### 1. Radio `0x15` queues building mission `0x14` only before the DockUnload branch

In `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x15` first rejects current mission `0x13`, then tests service/dock building type flags in order.

For `UnitRepair`, `UnitReload`, `Hospital`, or `Armory`, the function writes `building+0x6DD = 1`, queues mission `0x14` on the building, queues mission `0` on the sender, and returns `1`. Evidence: `0x0043C732..0x0043C7D4`; decompile shows the flag group `+0x16A9/+0x16AA/+0x16C1/+0x16C2`, then `self.Queue_Mission(0x14,0)`.

For `Bunker`, the function writes `building+0x6DD = 1`, queues mission `0x14` on the building, and returns `1`. Evidence: `0x0043C75A..0x0043C779`.

**Active in YR:** Conditional. Active for buildings whose type has those service/bunker flags. Not active for stock GAREFN/NAREFN because they do not have those flags in `rulesmd.ini`.

### 2. Stock DockUnload radio `0x15` queues only sender mission `0x10`

After the service/bunker checks fail, the DockUnload branch reads `BuildingType+0x16B3` and, if true, calls the sender's mission queue vtable slot with `(0x10, 0)`. Evidence: `0x0043C788 MOV [Type+0x16B3]`; `0x0043C79A PUSH 0`; `0x0043C79C PUSH 0x10`; `0x0043C7A0 CALL [sender_vtable+0x1E8]`.

There is no `self.Queue_Mission(0x14,0)` in this branch and no write to building `+0x6DD`. The prior `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF` report is confirmed for this target.

**Active in YR:** Yes. `rulesmd.ini:[GAREFN]` and `[NAREFN]` set `DockUnload=yes`; `CMIN/HARV` dock to `NAREFN,GAREFN`.

### 3. Stock GAREFN/NAREFN do not satisfy MissionRepairAndProduce's service gates

`rulesmd.ini` stock refinery sections show the relevant live flags:

- `[GAREFN]`: `DockUnload=yes` at `rulesmd.ini:11726`, `Refinery=yes` at `11727`, `FreeUnit=CMIN` at `11736`.
- `[NAREFN]`: `DockUnload=yes` at `rulesmd.ini:12519`, `Refinery=yes` at `12520`, `FreeUnit=HARV` at `12530`.

No `UnitRepair`, `UnitReload`, `Hospital`, `Armory`, or `Bunker` key is present in either stock section block. Therefore stock radio `0x15` cannot take the branch that queues building mission `0x14`.

**Active in YR:** Yes. These are stock YR rulesmd definitions.

### 4. MissionRepairAndProduce itself has no stock DockUnload/refinery branch

`BuildingClass::MissionRepairAndProduce @ 0x0044B780` checks these top-level flags: `Bunker +0x16AB`, `ConstructionYard +0x16B9`, `Hospital +0x16C1`, `Armory +0x16C2`, `UnitRepair +0x16A9`, and `UnitReload +0x16AA`.

The function does not have a top-level `DockUnload +0x16B3` or `Refinery +0x16BB` branch. If a building with none of the handled flags reaches this function, the final fallthrough at `0x0044C970` returns `0x0F`. This is consistent with stock refineries not using this mission handler for ore unloading.

**Active in YR:** Conditional. The function is live for buildings through the building vtable data xref at `0x007E4108`, but the stock refinery DockUnload path does not queue it.

### 5. Departure timing is unit-side state 4 plus slot-8 animation guard, not MissionRepairAndProduce

The harvester's queued mission `0x10` runs `UnitClass::Mission_Deploy_Building @ 0x0073D630`. During unload completion, state 3 can request building anim slot 8 with `BuildingClass::SetAnimSlotImage(slot=8, ...)` when `Refinery=yes`; state 4 later checks `building+0x57C` and returns early while that pointer is non-null. Evidence: `0x0073E4DC..0x0073E517` / `0x0073E539..0x0073E58F` slot-8 requests; `0x0073E1D5..0x0073E1EA` guard from prior `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM` report.

`building+0x57C` is `Anims_0[8]`, populated by `BuildingClass::CreateAnimForSlot @ 0x00451890` and cleared by `BuildingClass::ClearAnimSlot @ 0x00451E40`. Stock `NAREFN` has `;ProductionAnim=NAREFN_AR` commented at `artmd.ini:1749`; stock `GAREFN` has no `ProductionAnim` key in its `artmd.ini:1763..1789` block.

**Active in YR:** Conditional. The state-4 guard is active for stock harvesters, but the wait branch is normally inactive for stock GAREFN/NAREFN because slot 8 remains null. A modded DockUnload refinery with `ProductionAnim` can create a visible departure wait.

## Handoff Answer

The exact stock handoff after pad-arrival radio `0x15` is:

1. Unit sends radio `0x15` to its building contact.
2. `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x15` reaches `DockUnload +0x16B3`.
3. Building queues sender mission `0x10` with `commence_now=0`.
4. Building returns `1`.
5. Unit-side mission `0x10` later executes `UnitClass::Mission_Deploy_Building`.
6. At unload completion, unit-side state 4 may wait on building `Anims_0[8]` only.

`BuildingClass::MissionRepairAndProduce` / building mission `0x14` is not in that stock handoff.

## Reconciliation Notes

- Confirms `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`: stock DockUnload radio `0x15` queues sender mission `0x10`, not building mission `0x14`.
- Confirms `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM_GHIDRA_REPORT.md`: the stock departure guard is `Anims_0[8]` / `ProductionAnim`, and stock GAREFN/NAREFN normally do not wait there.
- Refines stale language in `miner/BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md`: `MissionRepairAndProduce` is the service/bunker/CY-style building mission handler, not the stock ore refinery unload handler; stock ore refinery unload is unit-side `Mission_Deploy_Building`.

## Open Questions

- Exact modded `ProductionAnim` lifetime and frame-count delay are out of scope; they require `AnimClass` lifecycle and specific mod art data.
- A building type that combines `DockUnload=yes` with `UnitRepair`, `UnitReload`, `Hospital`, `Armory`, or `Bunker` would take the earlier radio `0x15` branch, not the stock refinery branch. That mixed-flag behavior is real but out of scope for stock GAREFN/NAREFN.

## Sources

- Ghidra MCP read-only decompile/disassembly: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra MCP read-only decompile/disassembly: `BuildingClass::MissionRepairAndProduce @ 0x0044B780`.
- Ghidra MCP read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra MCP xref: `BuildingClass::MissionRepairAndProduce` referenced from data `0x007E4108`.
- Ghidra MCP read-only decompile: `MissionClass::Queue_Mission @ 0x005B35E0`; `MissionClass::GetCurrentMission @ 0x005B3040`.
- Prior docs: `docs/research/miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`.
- Prior docs: `docs/research/BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini:11721..11740`, `ini/rulesmd.ini:12514..12531`, `ini/artmd.ini:1706..1750`, `ini/artmd.ini:1763..1789`.
