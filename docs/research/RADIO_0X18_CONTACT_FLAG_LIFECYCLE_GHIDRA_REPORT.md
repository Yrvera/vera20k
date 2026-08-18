# Radio 0x18 Contact Flag Lifecycle - Ghidra Research Report

**Address(es):** `0x006F4AB0`, `0x0043C2D0`, `0x004D8FB0`, `0x00737430`, `0x00739EC0`, `0x00741970`, `0x0073D630`, `0x0065A820`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** radio `0x18` contact flag lifecycle in stock refinery docking: fields written, clearers, predicate consumers, persistence, and relation to the `UnitClass::PerCellProcess` adjacent-building `0x15` branch.
**Non-Scope:** accepted-cell vs GetDockCoord vs QueueingCell coordinate proof; full non-dock meaning of the same byte; Rust edits.
**Confidence:** High for the scoped docking lifecycle.
**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`; clear paths are live but conditional.

## 0. Working Notes

**Target question:** What exact unit/building field(s) are affected by radio `0x18` in the refinery docking path, including writers, clearers, consumers, and persistence?

**Non-goals:** Do not re-investigate the settled coordinate split: accepted refinery target is `NW+(3,1)`, stock `GetDockCoord` is `NW+(2,1)`, and no physical `NW+3 -> NW+2` move is implied by `0x16`.

**Evidence needed to mark COMPLETE:** Decompile evidence for `TechnoClass::Receive_Radio(0x18/0x19)`, `BuildingClass::Receive_Radio(0x0E)` sender order, Unit/Foot fallthrough to TechnoClass, Unit `0x16` consumer, Unit `PerCellProcess` adjacent-building consumer, one clear cascade, and stock INI evidence for YR activity.

**Stop conditions:** Stop once the field lifecycle is proven for stock refinery docking and remaining questions are limited to runtime frame timing or global non-dock consumers.

## 1. Bottom Line

Radio `0x18` is not an immediate unload/link event. It establishes mirrored Techno-derived contact state:

- receiver `Techno+0x418 = 1`;
- receiver transmits `0x18` back through radio vtable `+0x278`;
- the partner also reaches `TechnoClass::Receive_Radio(0x18)` and sets its own `+0x418 = 1`;
- already-set endpoints do not rewrite or re-propagate.

In stock refinery docking, `BuildingClass::Receive_Radio(0x0E)` sends directed `0x18` to the miner only after `0x12` returns `0x14`. The miner sets `Unit+0x418`, propagation sets `Building+0x418`, then the building sends `0x16`. Later `0x15` unload handoff is gated elsewhere (`UnitClass::Receive_Radio(0x16)` or `UnitClass::PerCellProcess`); `0x18` alone does not start unload.

The flag is persistent radio-contact state: initialized to `0`, set by `0x18`, cleared by `0x19`, and not cleared by DockUnload state-4 exit. It is not `+0x2E4`, not `+0x6D1`, not a pad occupancy marker, and not a coordinate/movement update.

## 2. Field And Message Map

| Field/message | Owner | Verified role | Evidence | Active in YR |
|---|---|---|---|---|
| `Techno+0x418` | Unit and Building endpoints | Contact-entered/radio-contact byte set by `0x18`, cleared by `0x19` | constructor `0x006F2B40`; radio writes `0x006F4B72`, `0x006F4BA6` | Yes |
| `Techno+0x419` | Techno endpoint | Adjacent byte toggled by `0x1A/0x1B`, not dock `0x18` | `0x006F4AB0` cases `0x1A/0x1B` | Conditional, not this path |
| `Unit+0x5A4` | Foot/Unit | logical destination/NavCom pointer, used with `+0x418` consumers | `0x004D8FB0`, `0x00737430`, `0x00739EC0` | Yes |
| `Unit+0x6D1` | Unit | DockUnload active latch, cleared at unload exit | `0x0073D630` state-4 clear | Yes |
| `+0x2E4` | Techno/Building/Unit layouts | Not written by `0x18/0x19` in this path | absence in `0x006F4AB0`; prior writer inventory | Conditional, not stock zero-link DockUnload |
| radio `0x18` | Techno receiver | set `+0x418` and propagate once | `0x006F4AB0` case `0x18` | Yes |
| radio `0x19` | Techno receiver | clear `+0x418` and propagate once | `0x006F4AB0` case `0x19` | Conditional |
| radio `0x08` | Techno receiver | sends `0x19` then `0x03`; one clear-cascade source | `0x006F4AB0` case `0x08`; `0x0065A820` break handling | Conditional |

## 3. Verified Lifecycle

### 3.1 Initialization

`TechnoClass::Constructor @ 0x006F2B40` initializes `byte [this+0x418] = 0`. The decompile shows `*(undefined1 *)(param_1 + 0x106) = 0`, where `0x106 * 4 == 0x418`, followed by explicit initialization of bytes `+0x419..+0x41F`. This proves `+0x418` is byte state, not a pointer/int dock link.

**Active in YR:** Yes. Stock harvesters and refineries are Techno-derived.

### 3.2 Stock refinery sender order

`BuildingClass::Receive_Radio @ 0x0043C2D0`, case `0x0E`, handles stock DockUnload admission. For `DockUnload=yes` / `Weeder=yes` buildings it sends `0x12` with the already-settled accepted cell payload. Only if `0x12` returns `0x14` does it execute:

1. directed transmit `0x18` to the requesting unit;
2. directed transmit `0x16` to the requesting unit;
3. fallback action if `0x16` returns non-ROGER.

This order matters: contact state is established before the same burst's `0x16` receive logic.

**Evidence:** fresh decompile `0x0043C2D0`; disassembly spot-check range `0x0043C2D0..0x0043CE5F`.
**Active in YR:** Yes. `ini/rulesmd.ini` has `[GAREFN] DockUnload=yes` / `Refinery=yes` at `11726..11727`, `[NAREFN]` at `12519..12520`, and `[CMIN]` / `[HARV] Dock=NAREFN,GAREFN` at `7361` / `8225`.

### 3.3 Unit/Foot fallthrough reaches TechnoClass

`UnitClass::Receive_Radio @ 0x00737430` has no direct `0x18` case. `FootClass::Receive_Radio @ 0x004D8FB0` also has no direct `0x18` case. The directed building send therefore reaches `TechnoClass::Receive_Radio @ 0x006F4AB0`.

**Evidence:** fresh decompile `0x00737430`; fresh decompile `0x004D8FB0`; Unit receiver vtable xref `0x007F5E04 -> 0x00737430`.

### 3.4 `0x18` write and propagation

`TechnoClass::Receive_Radio(0x18) @ 0x006F4AB0`:

1. checks an aircraft-only `WhatAmI()==2` / type `+0xE0D` skip gate;
2. reads `byte [this+0x418]`;
3. if zero, writes `1`;
4. calls radio transmit vtable `+0x278` with `0x18` and the sender;
5. returns `1`;
6. if already nonzero, does not rewrite or propagate and falls through to `RadioClass::Receive_Radio`.

For stock refinery docking, the miner sets `Unit+0x418 = 1` and propagates `0x18` back to the refinery. The refinery has no `BuildingClass` case `0x18`, so it falls through to `TechnoClass::Receive_Radio(0x18)`, sets `Building+0x418 = 1`, then propagates back. The miner is already set and stops the ping-pong.

**Evidence:** fresh decompile `0x006F4AB0`; disassembly spot-check range `0x006F4AB0..0x006F4CDF`; prior exact assembly contexts identify set at `0x006F4B72` and transmit at `0x006F4B79`.
**Active in YR:** Yes for CMIN/HARV because they are UnitClass, not AircraftClass with the `+0xE0D` skip.

### 3.5 `0x19` clear and propagation

`TechnoClass::Receive_Radio(0x19)`:

1. reads `byte [this+0x418]`;
2. if nonzero, writes `0`;
3. calls radio transmit vtable `+0x278` with `0x19` and the sender;
4. returns `1`;
5. if already zero, does not rewrite or propagate and falls through to base radio handling.

The clear is mirrored across the contact pair. The first endpoint to receive `0x19` clears and propagates; the other endpoint clears and propagates back; already-clear endpoints stop propagation.

**Evidence:** fresh decompile `0x006F4AB0`; prior exact assembly contexts identify clear at `0x006F4BA6` and transmit at `0x006F4BAD`.
**Active in YR:** Conditional. The code is live, but uninterrupted DockUnload state-4 exit does not directly send `0x19`.

### 3.6 Clear cascade sources

Verified routes that can cause `0x19` after `+0x418` has been set:

- `TechnoClass::Receive_Radio(0x08)` sends `0x19` then `0x03` through vtable `+0x278`. This matters because Unit `PerCellProcess` cleanup can transmit radio `0x08` while `+0x418` remains set.
- `TechnoClass::Receive_Radio(0x03)` checks both sides' `+0x418` state before falling into base break handling; if applicable it emits `0x19` before `RadioClass::Receive_Radio` removes the contact slot.
- `TechnoClass::Set_Destination @ 0x00741970` has a cancel path that reads this object's `+0x418` and emits `0x19` then `0x03`; the inspected branch is aircraft/carryall-gated and is not the standard CMIN refinery path.

`RadioClass::Receive_Radio @ 0x0065A820` handles `0x03` by removing the sender from the contact array. It does not itself clear `+0x418`; the clear is the paired `0x19` behavior.

**Evidence:** fresh decompile `0x006F4AB0`, `0x0065A820`, `0x00741970`.

### 3.7 DockUnload exit does not clear `+0x418`

`UnitClass::Mission_Deploy_Building @ 0x0073D630` is the DockUnload mission body. State-4 exit clears `Unit+0x6D1` and queues Harvest/mission `0x0A`; it does not read or write `+0x418` in that exit block. `+0x418` is therefore not a transient "inside unload state" latch.

**Evidence:** fresh decompile `0x0073D630`; prior assembly context identifies `+0x6D1` clear at `0x0073E1F6`; no `+0x418` read/clear in the state-4 block.

## 4. Predicate Consumers Relevant To `0x15`

### 4.1 Unit `0x16` consumes `+0x418`

`UnitClass::Receive_Radio @ 0x00737430`, case `0x16`, first calls `FootClass::Receive_Radio`, which lets TechnoClass process `0x16` as a contact-refresh `0x18` transmit. Unit-specific logic then may:

- return early after facing/rate sync if the timer is not `0x4000`;
- otherwise query locomotor `Is_Moving`;
- if not moving, require destination non-null, `Unit+0x418 != 0`, destination `WhatAmI()==6`, and current mission `7`;
- transmit directed `0x15` to the destination building.

This branch does not compare current cell to `GetDockCoord`.

**Evidence:** fresh decompile `0x00737430`; prior exact assembly contexts identify `+0x418` read at `0x0073774A` and directed `0x15` at `0x00737776..0x0073777A`.
**Active in YR:** Yes. Building `0x0E` sends `0x16` immediately after `0x18` once `0x12` returns `0x14`.

### 4.2 Unit PerCellProcess adjacent-building branch consumes `+0x418`

`UnitClass::PerCellProcess @ 0x00739EC0` has a contact-flag `0x15` branch after the `GetDockCoord` equality branch. It requires:

- `Unit+0x418 != 0`;
- `FootClass::GetDestination(0)` non-null;
- destination `WhatAmI()==6`;
- current mission `7`;
- lookup of the cell one row north of the current unit cell;
- north-cell building equals the destination;
- directed `0x15` to that destination;
- if return is neither `1` nor `5`, fallback action `vtable+0x174`.

This is the branch directly tied to the parent question: a live `0x15` source whose first gate is the `0x18`-set contact flag. It is not the earlier `GetDockCoord` equality branch.

**Evidence:** fresh decompile `0x00739EC0`; disassembly spot-check `0x0073A430..0x0073A5D3`; prior exact contexts identify `+0x418` gate at `0x0073A558` and `0x15` send at `0x0073A5C3..0x0073A5C8`.
**Active in YR:** Conditional. The code is live, but it only fires after `0x18` has set `+0x418` and the north-cell destination lookup matches.

### 4.3 Unit PerCellProcess cleanup consumes `+0x418`

Later in `UnitClass::PerCellProcess`, a cleanup block checks `+0x418` and mission/destination conditions. If contact remains set outside the accepted mission-7/destination case and not mission `0x10`, it can send radio `0x08` to the first radio contact. Receiver-side `0x08` cascades to `0x19` then `0x03`, clearing the mirrored contact flag and breaking the radio contact.

**Evidence:** fresh decompile `0x00739EC0`; disassembly spot-check `0x0073A780..0x0073AA63`; prior assembly context identifies cleanup around `0x0073A936..0x0073A93D`.

## 5. Persistence Classification

`+0x418` is persistent radio-contact-scoped state:

- not transient: it is not set and cleared inside one `0x18` call;
- not mission-scoped: DockUnload state-4 does not clear it, and consumers inspect it across mission/per-cell transitions;
- not a reciprocal dock pointer: it is one byte;
- not a pad occupancy marker: neither `0x18` nor `0x19` changes coordinates or movement destinations;
- not an unload-start event: unload starts only after later `0x15` receiver behavior.

Best Rust-facing semantic name: `radio_contact_entered` or `dock_contact_flag`, with separate endpoint ownership if modeling generic radio state.

## 6. Implementation Handoff

Affected Rust surfaces likely include:

- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_system.rs`
- `src/sim/miner/miner_tests.rs`
- any `RefineryDockContacts` / `dock_phase` state that currently conflates contact, link, pad occupancy, and unload start.

Required behavior:

1. Model `0x18` as contact-flag establishment before `0x16`, not unload/link completion.
2. Preserve separate endpoint contact state or an equivalent mirrored contact record: unit and refinery both become contact-flagged by `0x18` propagation.
3. Gate `0x16 -> 0x15` on stopped unit, live destination building, mission `7`, and contact flag, not on `GetDockCoord` equality.
4. Gate the PerCellProcess adjacent-building `0x15` branch separately on contact flag and north-cell building lookup.
5. Do not clear contact state merely because DockUnload reaches state 4; clear through a modeled `0x19`/break/cleanup equivalent.

Concrete Rust test names:

- `miner_radio_0x18_sets_contact_before_0x16_without_unload`
- `miner_radio_0x18_propagates_contact_to_refinery_endpoint`
- `miner_first_0x16_with_contact_can_sync_without_starting_unload`
- `miner_later_0x16_contact_flag_gates_0x15_from_accepted_cell`
- `miner_percell_adjacent_building_0x15_requires_contact_flag`
- `miner_dock_unload_state4_does_not_clear_contact_flag_directly`
- `miner_radio_0x19_clears_mirrored_contact_flag`

## 7. Negative Facts / Do Not Do

- Do not treat radio `0x18` as immediate `0x15`, immediate unload, or `Linked`.
- Do not write `+0x2E4` for radio `0x18`; this handler writes byte `+0x418`.
- Do not equate `+0x418` with `+0x6D1`; DockUnload state clears `+0x6D1`, while `+0x418` is radio-contact state.
- Do not clear the contact flag at DockUnload state-4 exit unless a `0x19`/break/cleanup equivalent has happened.
- Do not require physical movement to stock `GetDockCoord` for the `0x16 -> 0x15` path.
- Do not collapse the `0x16` `0x15` source and the PerCellProcess adjacent-building `0x15` source into one generic `Linked` transition.
- Do not use `0x18` to mutate current cell, snapshot cell, pad occupancy, or movement target.

## 8. Remaining Uncertainty

- Exact runtime frame where later `0x08 -> 0x19 -> 0x03` cleanup fires after ordinary unload remains a runtime-trace question. Static code proves the branch and gates, not the replay frame.
- Full non-dock semantic name for `+0x418` remains outside this slice.
- The exact Rust field split should be chosen after auditing current `RefineryDockContacts` and `phase_linked`, but the binary-side contract above is stable.

## 9. Stale Docs / Replacement Wording

Replace:

> Radio `0x18` links the miner to the refinery / starts unloading.

with:

> Radio `0x18` sets a mirrored Techno `+0x418` radio-contact flag on the receiver and, through one propagation round, on the partner. It does not start unloading. Later `0x16` or `UnitClass::PerCellProcess` branches can use that contact flag to send `0x15`.

Replace:

> Radio `0x18` writes the dock link pointer.

with:

> Radio `0x18` writes byte `+0x418`, not `+0x2E4`. Reciprocal dock-link pointer wording is stale for the stock refinery path.

Replace:

> Contact is cleared when the dock unload mission exits.

with:

> DockUnload exit clears `Unit+0x6D1`; the `+0x418` contact flag is cleared by radio `0x19` cascades such as break/cancel/cleanup.

## 10. Coverage Ledger

| Area | Status | Evidence | Remaining |
|---|---|---|---|
| `+0x418` initialization | verified | `0x006F2B40` decompile | none |
| Building admission sends `0x18` then `0x16` after `0x12 == 0x14` | verified | `0x0043C2D0` decompile | none |
| Unit/Foot fallthrough for `0x18` | verified | `0x00737430`, `0x004D8FB0` decompile | none |
| Techno `0x18` set/propagate | verified | `0x006F4AB0`; set `0x006F4B72`, transmit `0x006F4B79` | none |
| Techno `0x19` clear/propagate | verified | `0x006F4AB0`; clear `0x006F4BA6`, transmit `0x006F4BAD` | none |
| mirrored unit/building flag effect | verified by call chain | Building sends to unit; unit propagates; building default falls to Techno `0x18` | none |
| Unit `0x16` consumer | verified | `0x00737430`; `0x0073774A`, `0x00737776` | exact runtime second-call cadence owned by sibling slot |
| PerCell adjacent-building `0x15` consumer | verified | `0x00739EC0`; `0x0073A558`, `0x0073A5C3` | runtime frequency deferred |
| PerCell cleanup `0x08` consumer | verified | `0x00739EC0`; `0x0073A936..0x0073A93D` | exact post-unload frame deferred |
| DockUnload exit non-clear | verified | `0x0073D630`; `+0x6D1` clear, no `+0x418` clear | none |
| INI activity | verified | `rulesmd.ini` lines listed above | none |

## 11. Open Questions - Final State

- `[RESOLVED] OQ-18-001 - Which field does radio 0x18 write? -> Byte Techno+0x418, initialized to 0 in the Techno constructor.`
- `[RESOLVED] OQ-18-002 - Does radio 0x18 affect the building as well as the unit? -> Yes, through propagation: miner sets +0x418 then transmits 0x18 back; refinery falls through to TechnoClass and sets Building+0x418.`
- `[RESOLVED] OQ-18-003 - What clears it? -> Radio 0x19 clears byte +0x418 and propagates once; 0x08/0x03/cancel paths can cause that clear cascade.`
- `[RESOLVED] OQ-18-004 - Is it transient? -> No. It persists until 0x19; DockUnload state-4 does not clear it.`
- `[RESOLVED] OQ-18-005 - Is it mission-scoped? -> No. Mission/per-cell consumers read it across mission transitions; the clear mechanism is radio-contact cleanup.`
- `[RESOLVED] OQ-18-006 - Does PerCellProcess adjacent-building 0x15 require this flag? -> Yes, Unit+0x418 is the first gate for that branch.`
- `[RESOLVED] OQ-18-007 - Does 0x18 itself start unload? -> No. Later 0x16 or PerCellProcess can send 0x15, and BuildingClass 0x15 starts DockUnload for DockUnload buildings.`
- `[DEFERRED] OQ-18-008 - Exact normal replay frame for cleanup 0x08 -> 0x19 -> 0x03 after unload?` Category: needs runtime trace.
- `[DEFERRED] OQ-18-009 - Global non-dock semantic name for +0x418?` Category: out of scope.

## Sources

- Fresh Ghidra decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`.
- Fresh Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Fresh Ghidra decompile: `UnitClass::Receive_Radio @ 0x00737430`.
- Fresh Ghidra decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Fresh Ghidra decompile: `UnitClass::PerCellProcess @ 0x00739EC0`.
- Fresh Ghidra decompile: `TechnoClass::Constructor @ 0x006F2B40`.
- Fresh Ghidra decompile: `TechnoClass::Set_Destination @ 0x00741970`.
- Fresh Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Fresh Ghidra decompile: `RadioClass::Receive_Radio @ 0x0065A820`.
- Fresh Ghidra xrefs: `0x007F5E04 -> UnitClass::Receive_Radio`, `0x007F5DFC -> UnitClass::PerCellProcess`, TechnoClass receiver xrefs from BuildingClass and FootClass.
- Fresh Ghidra disassembly spot-check ranges: `0x006F4AB0..0x006F4CDF`, `0x0043C2D0..0x0043CE5F`, `0x0073A430..0x0073A5D3`, `0x0073A780..0x0073AA63`.
- Prior address-confirming reports: `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`, `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`, `DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`.
- Stock INI evidence: `ini/rulesmd.ini`.
