# RADIO_0X08_0X17 Factory/Repair/Bunker Sender Paths - Reswarm Report

**Date:** 2026-05-28  
**Slot:** 2  
**Target:** `RADIO_0X08_0X17_FACTORY_REPAIR_BUNKER_SENDER_PATHS`  
**Investigation Mode:** exhaustive-slice for literal sender-side `0x08` paths and the one returned-`0x17` compare branch.  
**Status:** COMPLETE for literal default-contact `0x08` senders and the live `UnitClass::PerCellProcess` compare path; PARTIAL only for runtime replay frequency of repair/bunker far-distance cleanup frames.

## Scope

This report only covers sender-side contexts for BuildingClass radio message `0x08` whose receiver may return queued/deferred reply `0x17` for `WeaponsFactory=yes`, `UnitRepair=yes`, or `Bunker=yes` buildings.

Stock refinery queue admission is excluded. Returned reply code `0x17` is kept separate from transmitted radio message `0x17`.

## Prior Settled Context

- `BuildingClass::Receive_Radio(0x08)` returns `0x17` only after base Techno cleanup when the receiver type has `WeaponsFactory`, `UnitRepair`, or `Bunker`.
- `UnitRepair`/`Bunker` receivers first run a near-distance shortcut: if sender/building center distance is `< 0x180` leptons, they return `1` before `TechnoClass::Receive_Radio(0x08)`.
- `TechnoClass::Receive_Radio(0x08)` sends directed `0x19`, then directed `0x03`, to clear the mirrored `+0x418` byte and break RadioClass contact.
- Stock `GAREFN/NAREFN` refineries do not satisfy the `WeaponsFactory/UnitRepair/Bunker` gates and must not return `0x17` from this path.

## Open Questions Log

- `[RESOLVED] OQ-1 - Which literal sender compares returned 0x17? -> Only `UnitClass::PerCellProcess @ 0x00739EC0`, send site `0x0073A93D`, compares `EAX == 0x17`.` Evidence: decompile plus binary disassembly `0x0073A936..0x0073AB66`.
- `[RESOLVED] OQ-2 - Does the war-factory final drive-out/contact-clear sender exist? -> Yes. The same `UnitClass::PerCellProcess` cleanup path is live for produced units carrying reciprocal factory contact and `+0x418` from the factory `HELLO(2)`/`0x18` setup.` Evidence: `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`; decompile/disassembly below.
- `[RESOLVED] OQ-3 - Do repair depot and bunker senders use a special sender function? -> No special `UnitRepair`/`Bunker` sender was found. They use the same `UnitClass::PerCellProcess` contact cleanup sender when the unit has `+0x418` and a building contact/destination.`
- `[RESOLVED] OQ-4 - Do the other literal `0x08` sender sites compare reply `0x17`? -> No. Infantry and two boundary-missing helper sites send `0x08` and ignore the returned value.`
- `[RESOLVED] OQ-5 - Is transmitted message `0x17` part of this sender branch? -> No. This branch compares returned `0x17`; it does not transmit `0x17`.`
- `[DEFERRED] OQ-6 - Exact live replay frame for every repair depot/bunker far-distance cleanup after install/repair handoff.` Category: needs runtime trace. Static code proves the sender branch and receiver result, not every player replay's winning frame.

## Verified Binary Findings

### 1. Receiver `0x08` returns `0x17` after cleanup only for the three gates

Active in YR: Conditional on receiver type.

`BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x08`, has this verified order:

1. Read `BuildingType+0x16A9 UnitRepair` and `+0x16AB Bunker`.
2. If either is set, compute 3D center distance to sender and return `1` when `Math__ftol(distance) < 0x180`.
3. Call `TechnoClass::Receive_Radio(sender, 0x08, payload)`.
4. Read receiver type flags `+0x16BD WeaponsFactory`, `+0x16A9 UnitRepair`, `+0x16AB Bunker`.
5. If none are set, return `1`.
6. Otherwise return `0x17`.

Disassembly evidence:

```text
0043CD2B  MOV EAX,[ESI+0x520]
0043CD35  MOV CL,[EAX+0x16A9]
0043CD3D  JNE 0x43CD4D
0043CD3F  MOV CL,[EAX+0x16AB]
0043CD47  JE  0x43CDD4
0043CDB9  CALL Math__ftol
0043CDBE  CMP EAX,0x180
0043CDC3  JGE 0x43CDD4
0043CDC8  MOV EAX,1
0043CDDD  CALL 0x006F4AB0
0043CDE8  MOV AL,[ESI+0x16BD]
0043CDF2  MOV AL,[ESI+0x16A9]
0043CDFC  MOV AL,[ESI+0x16AB]
0043CE09  MOV EAX,1
0043CE18  MOV EAX,0x17
```

Interpretation: `0x17` is a receiver reply code after cleanup, not the cleanup itself. For `UnitRepair`/`Bunker`, close senders get immediate `1` and skip both Techno cleanup and queued reply.

### 2. Techno `0x08` cleanup sends `0x19`, then `0x03`

Active in YR: Conditional on any receiver that reaches TechnoClass case `0x08`.

`TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x08` is not a pure query:

```text
006F4C2F  PUSH partner
006F4C30  PUSH 0x19
006F4C34  CALL [this.vtable+0x278]
006F4C3C  PUSH partner
006F4C3D  PUSH 0x03
006F4C41  CALL [this.vtable+0x278]
```

Therefore the sender-side compare receives `0x17` only after the receiver has already attempted mirrored `+0x418` clear and RadioClass `BREAK`.

### 3. The only literal `0x08` sender that compares `0x17` is `UnitClass::PerCellProcess`

Active in YR: Yes, conditionally. `UnitClass::PerCellProcess @ 0x00739EC0` is a live per-cell unit path.

The key send/compare block:

```text
0073A936  MOV EDX,[EBP]
0073A939  PUSH 8
0073A93B  MOV ECX,EBP
0073A93D  CALL [EDX+0x274]
0073A943  CMP EAX,0x17
0073A946  JNE 0x73AAF7
```

Preconditions from decompile:

- sender `Techno+0x418` is nonzero;
- current mission is not unload/deploy-building `0x10`;
- the mission/destination guard does not suppress cleanup for the still-arrived mission-7 building case;
- the current cell has no blocking building, or the branch has marked cleanup eligible;
- if the destination is a `WeaponsFactory` and the unit is still on that destination building's cell, the branch skips the `0x08` send and goes to `0x0073AB6C`.

This is the active final drive-out/contact-clear sender for stock land war-factory produced units after they are no longer in the factory cell window. It is also the generic cleanup sender for repair/bunker contacts when their unit endpoint carries `+0x418`.

### 4. Returned `0x17` behavior has a factory-specific AI sub-branch

Active in YR: Conditional; the factory-special sub-branch requires non-player control and a `WeaponsFactory=yes` destination.

After `EAX == 0x17`, the sender first checks `Unit+0x5A4` destination/current-building consistency:

```text
0073A94C  MOV EAX,[EBP+0x5A4]
0073A952  TEST EAX,EAX
0073A954  JE 0x73A98C
0073A95D  CALL [this+0x1BC]        ; current building/cell lookup style helper
0073A963  CMP ESI,EAX
0073A965  JE 0x73A98C
0073A96F  TEST byte ptr [ESI+0x14],1
0073A981  CALL [this+0x278]        ; sends 0x0E to live/flagged destination, then exits
```

If the destination check accepts, it builds a masked building pointer from the current Foot destination and then excludes harvesters/weeders:

```text
0073A99B..0073A9A6  mask destination to nonzero only when WhatAmI()==6
0073A9A8  MOV EAX,[EBP+0x6C4]
0073A9AE  TEST UnitType+0xE0E Harvester
0073A9BC  TEST UnitType+0xE0F Weeder
```

For an AI-controlled unit with a `WeaponsFactory=yes` destination, the branch computes a cell through `FUN_00500200`, then if the result is not the sentinel/global cell, it:

```text
0073AA42  PUSH 0
0073AA44  PUSH 2
0073AA48  CALL [this+0x1E8]        ; MissionSet/Queue mission 2
0073AA65  CALL [this+0x480]        ; Set_Destination(cell,1)
0073AA70  CALL [this+0x1EC]
0073AA88  CALL TechnoClass__SetGhostCell(cell)
0073AA92  PUSH 0
0073AA94  PUSH 0x0B
0073AA96  CALL [this+0x1E8]        ; Mission 0x0B
```

This is the war-factory-specific continuation. It must not be applied to repair depots or bunkers because the sub-branch explicitly checks destination type `+0x16BD`.

### 5. Returned `0x17` behavior for repair/bunker falls into generic stop/scatter/NavCom handling

Active in YR: Conditional on the sender having a `UnitRepair` or `Bunker` contact and being far enough to avoid the receiver `< 0x180` early return.

When the returned `0x17` receiver is not a `WeaponsFactory` destination, or the sender is player-controlled, the branch chooses between a stored NavCom destination and stop/scatter:

```text
0073AAA1  MOV EAX,[EBP+0x218]
0073AAA7  TEST EAX,EAX
0073AAAB  CMP EAX,[EBP+0x5A4]
0073AABB  CALL [this+0x480]        ; if +0x218 exists and differs, Set_Destination(+0x218,1)
0073AAC8  CALL FootClass__Stop_Moving
0073AAD0  PUSH 0
0073AAD2  PUSH 1
0073AAD4  PUSH DAT_00B1CFE8
0073AADB  CALL [this+0x174]        ; otherwise Scatter(null,1,0)
```

For `UnitRepair` and `Bunker`, this is the implementation-relevant sender outcome after a far-distance `0x08 -> 0x17` reply. Their near-distance `return 1` path does not enter this `0x17` handling.

### 6. Non-`0x17` replies have separate handling for harvester/weeder cleanup

Active in YR: Conditional.

If the `0x08` result is neither `0x17` nor `10`, and the sender type is `Harvester` or `Weeder`, the code can queue mission `10`, set a stored destination, and clear ghost-cell state:

```text
0073AAF7  CMP EAX,0x0A
0073AAFC  TEST UnitType+0xE0E/+0xE0F
0073AB23  PUSH 0
0073AB25  PUSH 0x0A
0073AB29  CALL [this+0x1E8]
0073AB3D  CALL [this+0x480]
0073AB43  CALL TechnoClass__SetGhostCell(0)
```

This is not the factory/repair/bunker returned-`0x17` path and should not be borrowed for them.

### 7. Other literal `0x08` senders ignore return `0x17`

Active in YR: Conditional, but not handoff-critical for `0x17` behavior.

The prior global-sender inventory remains correct after spot-checking the actual byte ranges:

| Site | Context | Return used? | Evidence |
|---|---|---:|---|
| `0x0051A80C` | `InfantryClass::PerCellProcess`, `+0x418` cleanup | No. Next instruction reads type data; no `CMP EAX,0x17`. | `0x0051A7F8..0x0051A88B` |
| `0x00522AA2` | boundary-missing helper gated by type bytes `+0xD6A/+0xD94`, control id `0x117B`, and `HasAnyContact` | No. It immediately checks `+0x210/+0x59C`, then Set_Destination or Stop/Scatter. | `0x00522A72..0x00522AF7` |
| `0x0073A93D` | `UnitClass::PerCellProcess` | Yes. Compares `EAX == 0x17`. | `0x0073A936..0x0073AB66` |
| `0x00746142` | boundary-missing UnitClass-range helper gated by control id `0x117B` and `HasAnyContact` | No. It immediately checks `+0x210/+0x59C`, then Set_Destination or Stop/Scatter. | `0x00746126..0x00746197` |

Negative fact: the boundary helpers can trigger receiver-side `0x08` cleanup, but their sender continuation is not selected by returned `0x17`.

## Active Stock YR Data

Active receivers:

- `GAWEAP`, `NAWEAP`, `YAWEAP`: `WeaponsFactory=yes` in `ini/rulesmd.ini`.
- `GADEPT`, `NADEPT`, `YADEPT`, `CAOUTP`: `UnitRepair=yes`.
- `NATBNK`: `Bunker=yes` and `NumberOfDocks=1`.

War factory sender liveness:

- `BuildingClass::ExitObject_Main` for stock land war factories sends `HELLO(2)` and `0x18` to the produced unit after unlimbo, creating reciprocal RadioClass contact and setting mirrored `+0x418`.
- `UnitClass::PerCellProcess` later sees that `+0x418` state and sends default-contact `0x08` once the drive-out cleanup gates admit the branch.
- Because stock land war factories have `WeaponsFactory=yes`, the receiver returns `0x17` after Techno cleanup.

Repair/bunker sender liveness:

- Standard repair depots and `NATBNK` activate receiver gates from stock INI.
- Static evidence proves the same unit per-cell cleanup sender will compare `0x17` when the contacted receiver returns it.
- Exact player replay frame for the far-distance repair/bunker cleanup case remains runtime-sensitive.

## Rust Surface Scan

Current Rust has pieces, but no general radio sender model matching this branch:

- `src/sim/game_entity.rs`: `radio_contacts`, `bunker_occupant`, contact helpers.
- `src/sim/entity_store.rs` / `src/sim/world/mod.rs`: generic contact clearing.
- `src/sim/production/production_spawn.rs`: war-factory produced-unit spawn/contact-like behavior area.
- `src/sim/docking/building_dock.rs`: repair depot dock/reservation system.
- `src/sim/pathfinding/cell_entry.rs`: `UnitRepairOrBunker` and contact-row movement gates.
- `src/sim/miner/miner_dock.rs`: refinery-specific `contact_entered`; useful pattern but stock refinery behavior must stay separate from factory/repair/bunker `0x17`.

No Rust files were modified.

## Implementation Handoff

| Verified behavior | Evidence | Rust-facing effect | Acceptance scenario |
|---|---|---|---|
| `UnitClass::PerCellProcess` sends default-contact `0x08`, compares returned `0x17`, and only then enters queued/deferred sender handling. | Decompile `0x00739EC0`; disassembly `0x0073A936..0x0073AB66` | Implement `0x08` cleanup as a per-cell contact cleanup branch, not as receiver-only state. The sender must observe the returned code. | `unit_percell_radio_0x08_return_0x17_drives_sender_followup` |
| War-factory produced vehicles have the live prerequisites for the sender path: reciprocal contact and `+0x418` from `HELLO(2)`/`0x18`. | `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`; `0x00443C60`; `BUILDING_RADIO_0X18...` | Preserve produced-unit/factory radio contact through drive-out until the unit per-cell cleanup sends `0x08`; do not clear it immediately at spawn. | `war_factory_driveout_clears_contact_via_unit_percell_radio_0x08` |
| AI war-factory returned-`0x17` has a special destination/ghost/mission path gated by destination `WeaponsFactory=yes`. | `0x0073A9F9..0x0073AA9C` | If AI drive-out recovery is modeled, gate it on the destination building type, not on any `0x17` reply. | `ai_war_factory_queued_reply_sets_driveout_destination_and_mission_0x0b` |
| Repair/bunker returned-`0x17` uses generic stop/scatter or stored NavCom continuation, not the war-factory branch. | `0x0073AAA1..0x0073AAE1`; receiver gates `0x0043CD2B..0x0043CE21` | For `UnitRepair`/`Bunker`, handle far-distance `0x17` by the generic sender continuation; near-distance reply `1` must not run it. | `repair_bunker_far_radio_0x08_queued_reply_uses_generic_sender_followup` |
| Receiver cleanup occurs before returned `0x17`: `0x08` invokes `0x19`, then `0x03`. | `0x006F4C2F..0x006F4C41`; Building return block `0x0043CDD4..0x0043CE18` | Update contact-entered and RadioClass contact state before applying sender-side `0x17` behavior. | `radio_0x08_clears_entered_and_breaks_contact_before_sender_queued_followup` |
| Other literal `0x08` sender sites ignore returned `0x17`. | `0x0051A7F8..0x0051A88B`, `0x00522A72..0x00522AF7`, `0x00746126..0x00746197` | Do not share the `UnitClass::PerCellProcess` returned-`0x17` continuation with infantry or boundary helper senders. | `literal_radio_0x08_senders_do_not_all_branch_on_0x17` |

## Negative Facts / Do Not Do

- Do not route stock refinery queue admission through `0x08 -> 0x17`.
- Do not transmit radio message `0x17` from the `UnitClass::PerCellProcess` returned-`0x17` branch; it compares a reply code.
- Do not treat all literal `0x08` senders as equivalent; only `0x0073A93D` compares `0x17`.
- Do not run the AI war-factory destination/ghost/mission branch for `UnitRepair` or `Bunker`; it explicitly checks destination `WeaponsFactory=yes`.
- Do not skip the UnitRepair/Bunker `< 0x180` early return. Close repair/bunker senders return `1` before Techno cleanup and before queued reply.
- Do not model `0x08` as read-only. Receiver-side Techno cleanup sends `0x19` and `0x03` before any returned `0x17` behavior.
- Do not clear war-factory produced-unit radio contact at spawn/unlimbo. The contact is needed for live drive-out passability and later per-cell cleanup.

## Remaining Uncertainty

- Exact runtime frame and frequency for far-distance repair depot and bunker `0x08 -> 0x17` cleanup in player replays requires a runtime trace. Static evidence proves the branch and semantics.
- Exact semantic names for the boundary-missing helpers around `0x00522AA2` and `0x00746142` remain unresolved. Their return-ignoring behavior is proven and sufficient for this target.

## Sources

- Fresh Ghidra read-only decompile: `BuildingClass__Receive_Radio @ 0x0043C2D0`.
- Fresh Ghidra read-only decompile: `TechnoClass__Receive_Radio @ 0x006F4AB0`.
- Fresh Ghidra read-only decompile: `UnitClass__PerCellProcess @ 0x00739EC0`.
- Binary disassembly via retail `gamemd.exe`: `0x0043CD2B..0x0043CE21`, `0x006F4C2F..0x006F4C41`, `0x0073A936..0x0073AB66`, `0x0051A7F8..0x0051A88B`, `0x00522A72..0x00522AF7`, `0x00746126..0x00746197`.
- Prior reports: `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE_GHIDRA_REPORT.md`, `WAR_FACTORY_EXIT_CONTACT_ROW_SKIP_GHIDRA_REPORT.md`, `BUILDING_RADIO_0X18_CONTACT_LIFECYCLE_RESWARM_20260528.md`, `BUILDING_RADIO_0X15_UNLOAD_SIDE_EFFECTS_RESWARM_20260528.md`, `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md`, `UNITREPAIR_BUNKER_NUMBER_IMPASSABLE_ROWS_SECOND_CALLSITE_GHIDRA_REPORT.md`.
- INI evidence: `ini/rulesmd.ini` stock `GAWEAP/NAWEAP/YAWEAP`, `GADEPT/NADEPT/YADEPT/CAOUTP`, and `NATBNK` sections.
- Rust surface scan only: `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/production/production_spawn.rs`, `src/sim/docking/building_dock.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/miner/miner_dock.rs`.
