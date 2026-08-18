# NumberImpassableRows Radio Contact Vector - Ghidra Research Report

**Address(es):** `0x0073F0A0` (`UnitClass::Can_Enter_Cell`), focused callsite `0x0073F58A` -> `0x0073F5A2`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the radio/contact vector checked before the `NumberImpassableRows` helper call at `UnitClass::Can_Enter_Cell` `0x0073F5A2`, who fills/clears it, and what that means for stock dock/factory/repair scenarios.  
**Non-Scope:** full `NumberImpassableRows` helper semantics beyond this callsite; the separate UnitRepair/Bunker helper xref at `0x0073F76D`; full refinery unload FSM.  
**Confidence:** High  
**Active in YR:** Yes - `UnitClass::Can_Enter_Cell`, `RadioClass`, building dock admission, and the relevant INI keys are live in stock YR.

## 1. Overview

The callsite does not consult a bespoke refinery, repair, or factory field. It checks the moving unit's inherited `RadioClass` contact array (`Contacts.data` at object `+0xE4`, bounded by `Contacts.Capacity` at `+0xE8`) to decide whether an occupied building cell may receive the special radio-contact treatment before the `NumberImpassableRows` row gate is applied.

If the checked building is in the mover's contact vector, is a building RTTI (`vtable+0x2C == 6`), and the `NumberImpassableRows` helper returns false for the candidate cell, `Can_Enter_Cell` skips that building occupant instead of treating it as a blocker. If the helper returns true, normal building blocking continues.

## 2. Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `RadioClass` | `+0xE4` | `Contacts.data`, pointer array of contacted `TechnoClass*`/object pointers | `RadioClass::Constructor @ 0x0065A750`, `DynamicVectorClass::Contains @ 0x0065AD50` | Yes |
| `RadioClass` | `+0xE8` | `Contacts.Capacity`, signed loop bound; default 1 | `0x0065A750`, `0x0065AD50`, `0x0065AE60` | Yes |
| `BuildingTypeClass` | `+0x1620` | `NumberImpassableRows`, default `-1` | constructor defaults doc plus INI reader `0x0046013A` | Yes |
| `BuildingTypeClass` | `+0x1780` | `NumberOfDocks`, default 1; used to grow building contacts | INI reader `0x00464938`; building ctor calls `0x0065AE60` | Yes |
| `TechnoClass` | `+0x418` | dock/contact-entered flag toggled by radio `0x18`/`0x19`; not this vector | `TechnoClass::Receive_Radio @ 0x006F4AB0` | Yes, but contrast only |

## 3. Core Logic At The Callsite

Verified binary flow:

1. `UnitClass::Can_Enter_Cell` iterates the cell occupant list.
2. Before the focused branch, it rejects self and some carrying/same-cell cases.
3. At `0x0073F57C..0x0073F58A`, it passes the occupant pointer to `DynamicVectorClass::Contains @ 0x0065AD50`, using the moving unit object as `this`.
4. The pointer argument is masked to zero unless the occupant's object flags byte at `+0x14` has bit `0x01` set. A zero argument makes `Contains` return false.
5. If the occupant is not in the mover's contacts, or the occupant RTTI is not 6, normal blocking logic runs.
6. If the occupant is in the mover's contacts and RTTI is 6, `FUN_00458A00 @ 0x00458A00` is called with `ECX = occupant building` and stack arg = candidate `CellClass*`.
7. If `FUN_00458A00` returns false, the loop advances to the next occupant and this building is ignored as a blocker for that cell.
8. If `FUN_00458A00` returns true, the normal building-blocking branch continues.

The helper uses `BuildingTypeClass+0x1620`: `-1` returns true immediately; otherwise it compares the candidate cell's X coordinate against `building_anchor_x + NumberImpassableRows`. This report only needs that polarity: true means "do not apply the radio-contact skip here"; false means "skip this contacted building as a blocker."

**Active in YR:** Yes. The callsite is inside the live `UnitClass::Can_Enter_Cell` vtable implementation used by drive/ship/pathfinding checks, and the helper xrefs include this active branch.

## 4. Contact Vector Lifecycle

The vector is the ordinary sparse `RadioClass` contact array:

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `RadioClass::Constructor @ 0x0065A750` | Allocates one pointer slot, sets `Contacts.data`, `Capacity=1`, zeroes slot 0 | decompiled ctor | Yes |
| `BuildingClass::Constructor @ 0x0043B740` | Calls `RadioClass::Set_Contact_Count(max(Type+0x1780,1))` for buildings | calls at `0x0043BCD0`/`0x0043BCE2` | Yes |
| `RadioClass::Set_Contact_Count @ 0x0065AE60` | Grows the vector and zero-fills new slots; does not shrink | decompiled helper | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, msg `0x02` | Sender-side HELLO: if target accepts with return 1, writes target into sender contact slot; if full, evicts `Contacts[0]` by sending BREAK first | decompiled send path | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820`, msg `0x02` | Receiver-side HELLO: ally-gates and writes sender into first null slot | decompiled receive path | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, msg `0x03` | Sender-side BREAK: clears all matching target slots, then forwards BREAK to target | decompiled send path | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820`, msg `0x03` | Receiver-side BREAK: clears matching sender slot, no compaction | decompiled receive path | Yes |

`RadioClass::Tether_Count @ 0x006B7D80` is not the vector used here. It walks a different structure at `+0x3C/+0x48` and counts entries with status `1` or a filtered status `2`; it has no xref from `0x0073F5A2`. **Active in YR:** Yes as a separate helper, but not this callsite.

## 5. Stock Scenario Implications

| Scenario | Stock INI / binary facts | Effect at this callsite | Active in YR |
|---|---|---|---|
| Allied/Soviet refineries `GAREFN`/`NAREFN` | `Refinery=yes`, `DockUnload=yes`, `NumberOfDocks=1`, `NumberImpassableRows=3` in `rulesmd.ini`; buildings get contact capacity 1 | A contacted harvester may ignore only the non-impassable east-side foundation cells; west 3 columns remain blockers | Yes |
| Yuri refinery `YAREFN` | `rulesmd.ini` has commented `;NumberImpassableRows=3`; no active key in that section | With default `-1`, the helper returns true everywhere, so the radio-contact skip is not opened by this key | Yes for `YAREFN`; condition is the active INI omission |
| War factories `GAWEAP`/`NAWEAP`/`YAWEAP` | `WeaponsFactory=yes`, `Factory=UnitType`, active `NumberImpassableRows=1`; `NumberOfDocks` defaults to 1 unless overridden | Factory-created/contacted units can use the contacted-building exception only outside the westmost blocked column; matches comments about hover/exit lanes | Yes |
| Repair depots `GADEPT`/`NADEPT`/YR clones | `UnitRepair=yes`, `NumberOfDocks=1`, `NumberImpassableRows=1` | A repaired/contacted unit has only the westmost column protected by this row gate at the focused branch; the separate UnitRepair/Bunker callsite is distinct and not re-covered here | Yes |
| Civilian outpost `CAOUTP` | `UnitRepair=yes`, `NumberOfDocks=1`, `NumberImpassableRows=3` | Same contact-vector mechanism; larger west-side protected area than stock depots | Yes |
| Airfields | `NumberOfDocks=4`, `Factory=AircraftType`; aircraft-specific `Can_Enter_Cell`/landing logic is outside this ground-unit callsite | Contact vector capacity is 4 on the building, but this UnitClass ground-building branch is not the primary aircraft landing rule | Conditional / out-of-scope for aircraft |

## 6. Current Rust Implementation Status

Rust parses `NumberOfDocks` and `NumberImpassableRows` in `src/rules/object_type.rs` and uses `number_impassable_rows` for static building movement blocking in `src/sim/production/production_tech.rs`. The code there uses X-column filtering consistent with the binary comparison, although the field comment currently says "rows (from the top, Y-axis)".

Rust has refinery contact/reservation state in `src/sim/miner/miner_dock.rs` (`contacts`, `contact_entered`, `on_pad`) and airfield/depot dock reservations, but this investigation did not verify a generic `RadioClass`-style per-object contact vector used by all `Can_Enter_Cell` building-occupant checks.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Can_Enter_Cell` focused branch | verified | `0x0073F57C..0x0073F5A9` assembly/decompile | none |
| Contact membership helper | verified | `DynamicVectorClass::Contains @ 0x0065AD50` | none |
| `NumberImpassableRows` helper polarity at focused branch | verified | `FUN_00458A00 @ 0x00458A00`, branch at `0x0073F5A7/0x0073F5A9` | full helper semantics covered by prior report, not repeated |
| Contact vector allocation/default | verified | `RadioClass::Constructor @ 0x0065A750` | none |
| Building contact capacity sizing | verified | `BuildingClass::Constructor @ 0x0043BCD0`, `BuildingTypeClass+0x1780`, `RadioClass::Set_Contact_Count @ 0x0065AE60` | none |
| HELLO/BREAK population/removal | verified | `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `RadioClass::Receive_Radio @ 0x0065A820` | none |
| Stock refinery/war factory/repair/outpost INI application | verified | `ini/rulesmd.ini` lines listed in Sources | none |
| Airfield use of same contact storage | touched-not-exhausted | `NumberOfDocks=4`, `0x0043BCD0` capacity sizing | aircraft landing/exit path outside this slot |
| Separate UnitRepair/Bunker callsite | deferred | user non-scope; contrast only | separate report already exists |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Which vector does `0x0073F5A2` depend on? It depends on the mover's inherited `RadioClass` `Contacts` vector at `+0xE4/+0xE8`, checked via `DynamicVectorClass::Contains @ 0x0065AD50` from `0x0073F58A`.  
[RESOLVED] OQ-2 - Does it use `RadioClass::Tether_Count @ 0x006B7D80`? No. That helper has no xref from `0x0073F5A2` and walks different fields `+0x3C/+0x48`.  
[RESOLVED] OQ-3 - Who populates/removes the vector? HELLO/BREAK paths in `RadioClass::Transmit_Radio_Impl @ 0x0065A970` and `RadioClass::Receive_Radio @ 0x0065A820`.  
[RESOLVED] OQ-4 - How is capacity set? `RadioClass::Constructor @ 0x0065A750` defaults to 1; `BuildingClass::Constructor @ 0x0043BCD0` grows to `max(BuildingTypeClass+0x1780,1)`.  
[RESOLVED] OQ-5 - Is this active in stock YR? Yes for stock refineries, factories, repair depots, and outpost because the INI keys and binary callsites are live; YAREFN is conditional on its omitted active `NumberImpassableRows` key.

## Sources

- Ghidra: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`; focused assembly `0x0073F57C..0x0073F5A9`.
- Ghidra: `DynamicVectorClass::Contains @ 0x0065AD50`; `FUN_00458A00 @ 0x00458A00`.
- Ghidra: `RadioClass::Constructor @ 0x0065A750`, `RadioClass::Receive_Radio @ 0x0065A820`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `RadioClass::Set_Contact_Count @ 0x0065AE60`, `RadioClass::Tether_Count @ 0x006B7D80`.
- Ghidra: `BuildingClass::Constructor @ 0x0043B740`, `BuildingTypeClass_ReadINI_Water` xrefs `0x0046013A` (`NumberImpassableRows`) and `0x00464938` (`NumberOfDocks`).
- INI: `ini/rulesmd.ini` `GAREFN` lines 11726-11764, `GAWEAP` lines 11775-11804, `NAREFN` lines 12519-12524, `NAWEAP` lines 12565-12598, depots/outpost/Yuri entries lines 11877-11913, 12665-12701, 13234-13339, 13415-13456, 13886-13901.
- Existing docs referenced for contrast: `NUMBER_IMPASSABLE_ROWS_CALLSITE_MATRIX_GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`.
