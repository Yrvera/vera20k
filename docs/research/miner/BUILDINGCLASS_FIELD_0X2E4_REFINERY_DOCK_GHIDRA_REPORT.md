# BuildingClass+0x2E4 Refinery Dock Field Provenance - Ghidra Report

**Date:** 2026-05-21  
**Investigation mode:** exhaustive-slice  
**Target:** `BuildingClass+0x2E4` provenance in the refinery dock/radio cycle  
**Scope:** verified reads/writes of `BuildingClass+0x2E4` that matter to refinery docking; whether `BuildingClass::Receive_Radio` cases `0x0E`/`0x15` and `BuildingClass::ReleaseDockedHarvester` touch it.  
**Non-scope:** full global `+0x2E4` inventory outside Techno/Unit/Building dock cleanup, save/load pointer remapping, and non-refinery dock systems beyond the one Bunker writer needed to identify the writer source.  
**Confidence:** High for this slice.

## Summary

`BuildingClass+0x2E4` is not the RadioClass contact array, not a radio opcode state, and not a mirror of TechnoClass radio `0x18/0x19`. It is a conditional reciprocal link field: when populated, `building+0x2E4` points at a linked unit and `unit+0x2E4` points back at the building. The live writer found in this slice is the Bunker/occupant helper, not the stock refinery DockUnload path.

For standard YR `CMIN/HARV -> GAREFN/NAREFN`, the normal refinery unload path runs with `unit+0x2E4 == 0`. `UnitClass::Mission_Deploy_Building @ 0x0073D630` takes the zero-link branch and rediscovers the refinery by adjacent-cell lookup using the known unload offset globals. `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` accepts dock and sends `0x12 -> 0x18 -> 0x16`; case `0x15` sets the sender mission to `0x10` for `DockUnload=yes`; neither case reads or writes `BuildingClass+0x2E4`.

`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` does touch the field, but only on the nonzero-link branch. If `building+0x2E4` is null, it clears `building+0x718`, sets building mission `5`, and returns. If non-null, it clears `unit+0x2E4`, sets the unit exit destination/mission, then clears `building+0x2E4` and `building+0x718` and transmits radio `3` (`BREAK`) to the first contact. This makes it a conditional link clearer, not evidence that stock refinery unloading sets `+0x2E4`.

## Verified Findings

### 1. `BuildingClass::Receive_Radio` case `0x0E` does not touch `+0x2E4`

**Evidence:** `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x0E`. The DockUnload/refinery acceptance branch checks power/type/contact conditions, sends `0x12` with hardcoded `GetCellLocation() + (3,1)`, then sends `0x18` and `0x16`. No read or write of `param_1->field_0x2e4` occurs in the decompiled case.

**Active in YR:** Yes. Stock `[CMIN]` and `[HARV]` have `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1` in `ini/rulesmd.ini`.

### 2. `BuildingClass::Receive_Radio` case `0x15` does not touch `+0x2E4`

**Evidence:** `BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x15`. The DockUnload branch checks `Type+0x16B3` and calls sender vtable `+0x1E8` with mission `0x10,0`; there is no `+0x2E4` access in this case.

**Active in YR:** Yes for stock refineries because `[GAREFN]`/`[NAREFN] DockUnload=yes` maps to `BuildingType+0x16B3`.

### 3. `ReleaseDockedHarvester` is a conditional `+0x2E4` clearer

**Evidence:** `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` reads `building+0x2E4`. If null, it clears `building+0x718`, sets building mission `5`, and returns. If non-null and the linked object is a unit, it clears `unit+0x2E4`, performs locomotor stop/head/destination/mission work, clears `building+0x2E4` and `building+0x718`, sets building mission `5`, and transmits radio `3`.

**Active in YR:** Conditional. The function is called from `UnitClass::Mission_Deploy_Building @ 0x0073D66D`, but the `+0x2E4` branch only matters when a reciprocal link already exists. The standard stock refinery unload FSM is verified at `0x0073D630` to enter with `unit+0x2E4 == 0`.

### 4. `UndockUnit` and `FUN_00459470` are also conditional clearers, not stock refinery writers

**Evidence:** `BuildingClass::UndockUnit @ 0x004593A0` reads `building+0x2E4`, and if linked to a unit, clears both `unit+0x2E4` and `building+0x2E4`, then transmits radio `3`. `FUN_00459470 @ 0x00459470` checks `building+0x2E4 != 0`, sends the teardown notification, clears `linked+0x2E4`, clears `building+0x2E4`, clears `building+0x718`, and sets building mission `5`.

**Active in YR:** Conditional. `UndockUnit` has callers from `BuildingClass::Sell @ 0x0044AAB0`, `BuildingClass::ReceiveDamage @ 0x004424EA`, and `TemporalClass::Update @ 0x0071AA15`; `FUN_00459470` has callers from `SuperClass::Launch @ 0x006CC955`, `TemporalClass::Update @ 0x0071AA90`, and `UnitClass::ReceiveDamage @ 0x00737D97`. These are interrupt/cleanup surfaces, not the uninterrupted stock ore-dump path.

### 5. The verified writer is the Bunker helper, gated away from stock refineries

**Evidence:** `buildingclass_bunker_occupant_dock_link_writer @ 0x00458E50` case `5` writes `building+0x2E4 = unit`, then `unit+0x2E4 = building`, then sets `building+0x718 = 6` and the linked unit mission. Its only Ghidra xref is `BuildingClass::MissionRepairAndProduce @ 0x0044B7A3`; that caller first checks `BuildingType+0x16AB`.

**Active in YR:** Conditional. `[NABNKR] Bunker=yes` exists in `ini/rulesmd.ini`, so the writer is live for Battle Bunker behavior. Stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes` and `Refinery=yes`, not `Bunker=yes`, so this writer is not the stock refinery DockUnload writer.

### 6. TechnoClass radio `0x18/0x19` uses `+0x418`, not `+0x2E4`

**Evidence:** `TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x18` sets the byte represented in the decompile as `param_1[6].UniqueID`, which corresponds to object offset `+0x418`, and propagates radio `0x18`. Case `0x19` clears the same byte and propagates `0x19`. No `+0x2E4` access occurs in these cases.

**Active in YR:** Yes for `0x18` in standard refinery docking because `BuildingClass::Receive_Radio` case `0x0E` sends `0x18` to the miner. `0x19` receiver logic is live but conditional; this slot did not find it on the normal uninterrupted refinery exit path.

## Field Classification

| Candidate meaning | Verdict | Evidence |
|---|---|---|
| Docked-unit pointer | Conditional / contextual | When nonzero, clearers treat `building+0x2E4` as a linked unit pointer and clear the reciprocal `unit+0x2E4`. Writer exists in Bunker helper `0x00458E50`. |
| Radio state or opcode | No | Receive_Radio `0x0E`/`0x15` do not read/write it; Techno radio `0x18/0x19` writes `+0x418`. |
| Contact slot mirror | No | HELLO/contact admission uses RadioClass `Contacts[]`; `+0x2E4` is absent from HELLO and stock DockUnload accept/handoff. |
| Unrelated coincidental offset | No for Techno/Unit/Building link clearers | Multiple clearers read/write both sides at `+0x2E4`; the offset is semantically linked in those paths. It is still not used by normal stock refinery DockUnload. |

## Stock Refinery Result

For standard `CMIN/HARV -> GAREFN/NAREFN`:

1. The radio accept/handoff path does not set `building+0x2E4`.
2. The miner-side unload FSM normally starts with `unit+0x2E4 == 0`.
3. The refinery is rediscovered from the miner's cell and the refinery unload adjacent lookup globals, not by a reciprocal pointer.
4. `ReleaseDockedHarvester` can clear a nonzero `+0x2E4` link, but the stock uninterrupted DockUnload path is not proven to create such a link.

## Reconciliation Notes

- This report corroborates `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`: stock refinery DockUnload has no reciprocal `+0x2E4` writer.
- It corrects the older `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` wording that described `BuildingClass+0x2E4` as populated by the refinery dock queue state machine for stock refineries. The binary evidence now supports a narrower statement: `+0x2E4` is a conditional reciprocal link field, but stock refinery DockUnload uses the zero-link path.
- It also corroborates `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`: the live radio flag toggled by `0x18/0x19` is `+0x418`, not `+0x2E4`.

## Open Questions

- Exact global semantic name for `BuildingClass+0x718` remains out of scope. It is paired with `+0x2E4` in the Bunker writer and clearers, but this slot did not inventory all consumers.
- Exact `0x19` senders for nonstandard cancellation/leave-dock cases belong to the sibling radio-exit slot. This report only verifies that `ReleaseDockedHarvester` clears `+0x2E4` and sends `0x03`, not `0x19`.

## Sources

- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Ghidra read-only decompile: `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`
- Ghidra read-only decompile: `BuildingClass::UndockUnit @ 0x004593A0`
- Ghidra read-only decompile: `FUN_00459470 @ 0x00459470`
- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra read-only decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`
- Ghidra read-only decompile: `buildingclass_bunker_occupant_dock_link_writer @ 0x00458E50`
- Ghidra read-only xrefs: `0x00458E50`, `0x004595C0`, `0x004593A0`, `0x00459470`
- INI evidence: `ini/rulesmd.ini` sections `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`, `[NABNKR]`
