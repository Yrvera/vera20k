# Airfield Radio CachedDock Contact Lifetime - Ghidra Report

Target: `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME`
Report path: `docs/research/AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`
Date: 2026-05-22
Mode: `/re-investigate` exhaustive-slice
Status: COMPLETE

## Investigation Contract

### Target Question

How does stock Yuri's Revenge reconcile aircraft airfield/helipad docking through radio messages, `AircraftClass+0x6CC` `CachedDock`, `NumberOfDocks` pad capacity, reload/dock contact lifetime, and cleanup when aircraft or airfield dies; and what does that imply for Rust `AirfieldDocks`?

### Non-goals

- Do not re-investigate full aircraft combat, target choice, strafing, flight physics, or locomotor movement.
- Do not implement Rust changes.
- Do not treat Carryall/passenger aircraft as normal stock YR airfield behavior except as contrast where encountered.
- Do not replace verified radio evidence with YRpp labels or external symbol names.

### Evidence Needed To Mark COMPLETE

- `CachedDock` write, reuse, invalidation, and pointer-expiry cleanup evidence from decompile plus disassembly.
- Building pad capacity source from `NumberOfDocks`, including INI reader evidence and runtime contact-capacity evidence.
- Aircraft/building radio message evidence showing docking/reload contact lifetime.
- Evidence that pad coordinate/slot identity is driven by RadioClass contact slot, not a separate queue.
- Current Rust `AirfieldDocks` implications scoped to contact lifetime, capacity, and cleanup.

### Stop Conditions

- Stop after proving the airfield radio/contact model and Rust handoff implications.
- Stop if a path enters full flight physics, aircraft attack resolution, or unrelated Carryall mechanics.
- Stop before editing Rust, INI, or in-repo docs.

## Executive Summary

Stock YR does not model airfield docking as an independent FIFO reservation queue. For airfield/helipad aircraft, `CachedDock` is only a nullable building pointer cache. Before reuse, `AircraftClass::FindBuildingToDock` sends radio `0x0F` to the cached building and keeps it only if the building answers `1`. If the cached building rejects, is no longer valid, or expires through pointer cleanup, `CachedDock` is cleared and a new dock search is performed.

Pad capacity and pad identity are RadioClass state. `BuildingTypeClass::ReadINI` reads `NumberOfDocks` into `BuildingTypeClass+0x1780`. `BuildingClass` construction calls `RadioClass::Set_Contact_Count(max(NumberOfDocks, 1))`. For multi-pad helipads/airfields, `BuildingClass::GetDockCoord` calls `RadioClass::FindDockSlot(contact)` and uses that contact slot index to choose `DockingOffsetN`. Therefore the pad index is the radio contact slot index.

Reload is also contact-driven. UnitReload buildings iterate their RadioClass contacts and send aircraft radio messages including `0x1D`, `0x13`, `0x1F`, and `0x1C`. The contact remains meaningful through landing/reload and is cleared by BREAK/pointer-expiry/death cleanup, not by a hidden airfield FIFO. Rust `AirfieldDocks` currently approximates this with `slots`, `queues`, and `aircraft_to_pad`; the slot part is directionally useful, but the FIFO queue and automatic promotion have no verified binary counterpart in the airfield radio primitive.

## Active In YR

Yes for stock `ORCA` and `BEAG` airfield reload/docking:

- `ini/rulesmd.ini`: `[ORCA]` and `[BEAG]` have `AirportBound=yes`, `Ammo=1`, and `Dock=GAAIRC,AMRADR`.
- `ini/rulesmd.ini`: `[GAAIRC]` and `[AMRADR]` have `UnitReload=yes`, `Helipad=yes`, and `NumberOfDocks=4`.
- `ini/artmd.ini`: `[GAAIRC]` provides `DockingOffset0..3`.
- `[General] ReloadRate=.3` is read by `RulesClass::ReadGeneral` at `0x00670B00`; exact reload cadence was not the target of this slice.

Carryall/passenger radio branches are Active in YR only conditionally. They are present in `AircraftClass::Receive_Radio`, but are not the normal stock ORCA/BEAG airfield reload path.

## Verified Binary Findings

### 1. CachedDock is a revalidated pointer cache, not a reservation

Active in YR: Yes, for AirportBound stock aircraft.

Evidence:

- Decompile: `AircraftClass::FindBuildingToDock @ 0x0041BBD0`.
- Disassembly range:
  - `0x0041BBD3`: reads `AircraftClass+0x6C4` type pointer.
  - `0x0041BBD9`: reads `AircraftTypeClass+0xE0D` `AirportBound`.
  - `0x0041BBE3`: reads `AircraftClass+0x6CC` `CachedDock`.
  - `0x0041BBEF..0x0041BBF4`: sends radio `0x0F` to cached building through the aircraft radio vtable call.
  - `0x0041BBFA`: compares reply with `1`.
  - `0x0041BBFF`: clears `AircraftClass+0x6CC` when reply is not `1`.
  - `0x0041BC17`: calls `FootClass::Find_Docking_Bay`.
  - `0x0041BC1C`: stores search result back to `AircraftClass+0x6CC`.

Verified behavior:

```c
if (Type->AirportBound && this->CachedDock != NULL) {
    if (this->Transmit_Radio(0x0F, this->CachedDock) == 1) {
        return this->CachedDock;
    }
    this->CachedDock = NULL;
}
this->CachedDock = FootClass::Find_Docking_Bay(...);
return this->CachedDock;
```

Implication: A Rust cached dock must be revalidated before use. It is not enough to remember a prior airfield id.

### 2. NumberOfDocks becomes RadioClass contact capacity

Active in YR: Yes for `GAAIRC` and `AMRADR`, both `NumberOfDocks=4`.

Evidence:

- INI reader: `BuildingTypeClass::ReadINI` function containing `0x00464800`.
- Disassembly range for `NumberOfDocks`:
  - `0x0046492E`: reads previous `BuildingTypeClass+0x1780`.
  - `0x00464938`: pushes `"NumberOfDocks"`.
  - `0x00464940`: calls the INI integer reader.
  - `0x00464945`: writes result to `BuildingTypeClass+0x1780`.
- Disassembly range for `DockingOffset%d`:
  - `0x0046499D`: reads `BuildingTypeClass+0x1780`.
  - `0x004649B7`: formats `"DockingOffset%d"`.
- Runtime constructor: `BuildingClass::Constructor @ 0x0043B740`.
- Disassembly range:
  - `0x0043BCB3`: reads building type pointer at `BuildingClass+0x520`.
  - `0x0043BCBD`: reads `BuildingTypeClass+0x1780`.
  - `0x0043BCC3..0x0043BCC8`: clamps to at least `1`.
  - `0x0043BCCD`: pushes contact count.
  - `0x0043BCD0`: calls `RadioClass::Set_Contact_Count @ 0x0065AE60`.
- `RadioClass::Set_Contact_Count @ 0x0065AE60` grows RadioClass contacts when requested capacity exceeds current `RadioClass+0xE8`.

Verified behavior:

```c
contact_count = max(building_type->NumberOfDocks, 1);
RadioClass::Set_Contact_Count(contact_count);
```

Implication: Airfield capacity is not only a building-type rule value. It directly sizes the building's RadioClass contact slots.

### 3. Pad index is the RadioClass contact slot index

Active in YR: Yes for multi-pad airfields/helipads.

Evidence:

- Decompile: `BuildingClass::GetDockCoord @ 0x00447B20`.
- Supporting decompile/disassembly: `RadioClass::FindDockSlot @ 0x0065AD90`.
- `FindDockSlot` disassembly range:
  - `0x0065AD99`: reads `RadioClass+0xE8` capacity.
  - `0x0065ADA5`: reads `RadioClass+0xE4` contact array.
  - `0x0065ADAB`: compares each slot with the requested contact pointer.

Verified behavior:

- For `Helipad` or `UnitRepair` buildings:
  - `NumberOfDocks == 0`: use building coordinate.
  - `NumberOfDocks == 1`: use `DockingOffset0`.
  - `NumberOfDocks > 1`: call `RadioClass::FindDockSlot(contact)`.
  - If slot is valid, use `DockingOffset[slot]`; otherwise fall back to building coordinate.

Implication: Rust must not let a pad reservation index diverge from the contact slot that the airfield would use for that aircraft. In gamemd.exe, the contact slot is the pad slot.

### 4. Radio HELLO/BREAK controls contact lifetime; no airfield FIFO was found

Active in YR: Yes, generic RadioClass behavior used by buildings and aircraft.

Evidence:

- Decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.
- Decompile/disassembly: `RadioClass::Receive_Radio @ 0x0065A820`.
- `Receive_Radio` disassembly range:
  - `0x0065A854`: reads contact capacity `RadioClass+0xE8`.
  - `0x0065A860`: reads contact array `RadioClass+0xE4`.
  - `0x0065A8A0`: clears a matching contact slot for BREAK.
  - `0x0065A8B2..0x0065A8C9`: HELLO path checks live/allied state before accepting.

Verified behavior:

- HELLO `0x02` is idempotent if already linked.
- If the sender's contact array has no free slot, sender-side transmit evicts slot `0` with BREAK before trying the new target.
- Receiver-side HELLO accepts into a free contact slot and rejects when no free slot is available.
- BREAK `0x03` clears matching contact slots.
- There is no verified RadioClass FIFO queue or automatic "next waiting aircraft" promotion.

Implication: Rust `AirfieldDocks::queues` and automatic promotion on release are gameplay policy not found in the core airfield radio evidence. If kept, they need separate proof from a higher-level aircraft mission path; this slice did not find it.

### 5. UnitReload buildings drive reload through contacted aircraft

Active in YR: Yes for stock `GAAIRC` and `AMRADR`.

Evidence:

- Decompile: `BuildingClass::MissionRepairAndProduce @ 0x0044B780`.
- The `UnitReload` branch checks `BuildingTypeClass+0x16AA`.
- It iterates the building's contact capacity and sends radio messages to contacted objects:
  - `0x1D`: aircraft ammo/full/no-target check.
  - `0x13`: aircraft movement/docking readiness check.
  - `0x1F`: reload/give-ammo path.
  - `0x1C`: follow-up message when reload path does not answer `1`.
- Aircraft receiver: `AircraftClass::Receive_Radio @ 0x004190B0`.
- Aircraft receiver disassembly highlights:
  - `0x004190B6`: reads current aircraft mission.
  - `0x00419124`: branch for message `0x1F`, including target/ammo gating.
  - `0x0041913A..0x00419147`: falls back to `FootClass::Receive_Radio @ 0x004D8FB0`.

Verified behavior:

- Aircraft-side `0x1D` answers `1` only when ammo is full and there is no target; it answers `10` while ammo is not full or while target-gated.
- Aircraft-side `0x1F` may early-answer `1` when ammo is at least half and the aircraft has a target; otherwise it delegates to the FootClass reload path.
- The airfield reload loop works over radio contacts, not over a separate airfield dock queue.

Implication: A Rust airfield pad/contact should remain occupied through the reload contact lifetime, then clear via the equivalent of mission transition/BREAK/death cleanup.

### 6. CachedDock cleanup occurs through aircraft pointer expiry

Active in YR: Yes.

Evidence:

- Decompile: `AircraftClass::Detach @ 0x0041B660`.
- Disassembly range:
  - `0x0041B66E`: calls `FootClass` pointer-expiry cleanup.
  - `0x0041B673`: reads `AircraftClass+0x6CC`.
  - `0x0041B67B`: compares cached dock pointer with expired object pointer.
  - `0x0041B67F`: clears `AircraftClass+0x6CC` if it matched.

Verified behavior:

```c
FootClass::PointerExpired(expired);
if (this->CachedDock == expired) {
    this->CachedDock = NULL;
}
```

Implication: Airfield destruction must clear aircraft cached dock state even if the aircraft did not voluntarily release a pad.

## INI / Default Evidence

### Stock YR aircraft

`ini/rulesmd.ini`:

- `[ORCA]`: `Dock=GAAIRC,AMRADR`, `PipScale=Ammo`, `Ammo=1`, `AirportBound=yes`.
- `[BEAG]`: `Dock=GAAIRC,AMRADR`, `PipScale=Ammo`, `Ammo=1`, `AirportBound=yes`.

### Stock YR airfields

`ini/rulesmd.ini`:

- `[GAAIRC]`: `UnitReload=yes`, `Helipad=yes`, `NumberOfDocks=4`.
- `[AMRADR]`: `UnitReload=yes`, `Helipad=yes`, `NumberOfDocks=4`.

`ini/artmd.ini`:

- `[GAAIRC]`: `DockingOffset0=0,-128,0`, `DockingOffset1=0,128,0`, `DockingOffset2=256,-128,0`, `DockingOffset3=256,128,0`.

### INI readers

- `Helipad` reader at the `BuildingTypeClass::ReadINI` function:
  - `0x004604CC`: reads old `BuildingTypeClass+0x16CB`.
  - `0x004604D3`: pushes `"Helipad"`.
  - `0x004604DB`: calls INI bool reader.
  - `0x004604E0`: stores result to `BuildingTypeClass+0x16CB`.
- `UnitReload` reader:
  - `0x0046091A`: reads old `BuildingTypeClass+0x16AA`.
  - `0x00460923`: pushes `"UnitReload"`.
  - `0x0046092F`: calls INI bool reader.
  - `0x00460934`: stores result to `BuildingTypeClass+0x16AA`.
- `NumberOfDocks` and `DockingOffset%d` reader evidence is listed above.
- `ReloadRate` reader in `RulesClass::ReadGeneral @ 0x00670B00`:
  - `0x00670C86`: pushes `"ReloadRate"`.
  - `0x00670C8E`: calls INI double reader.
  - `0x00670C93`: stores to `RulesClass+0x1508`.

## Rust Surface And Delta

Current Rust surfaces inspected:

- `src/sim/docking/aircraft_dock.rs`
- `src/sim/aircraft/mod.rs`
- `src/rules/object_type.rs`
- `src/rules/ruleset.rs`

Observed Rust model:

- `AirfieldDocks` stores `slots: BTreeMap<u64, Vec<Option<u64>>>`.
- `try_reserve(airfield_sid, aircraft_sid, num_pads)` picks the first empty slot, otherwise enqueues the aircraft in a FIFO queue.
- `release(aircraft_sid)` frees the occupied slot and immediately promotes the next queued aircraft.
- `cleanup_dead(&alive)` drops dead airfields and releases dead aircraft.
- Aircraft mission code also searches nearest same-owner alive `UnitReload` or `Helipad` building whose type appears in the aircraft `Dock=` list.

Binary-backed alignment:

- The Rust `slots` concept is useful only if the slot index is treated as the RadioClass contact slot.
- `number_of_docks` parsing and art `DockingOffsetN` merge are directionally aligned with the binary.
- The Rust release-on-full-reload behavior is plausible as a mission-level simplification, but the binary evidence shows the building reload loop runs through contacts until mission/contact cleanup.

Binary-backed mismatch/risk:

- The FIFO queue and automatic promotion were not found in RadioClass, BuildingClass airfield reload, or `CachedDock` evidence.
- `CachedDock` revalidation by radio `0x0F` is missing from the simple nearest-airfield/reservation model.
- When an airfield target dies in `tick_aircraft_docks`, the Rust branch using `airfield_docks.cancel(old_sid)` appears to pass an airfield id to a method that cancels by aircraft id. This is a source-scan finding, not a binary fact. It may leave the aircraft's old queue/reservation state uncleared in that branch.

## Implementation Handoff

1. Model cached dock separately from pad reservation.
   - `CachedDock` should be reused only after an equivalent of radio `0x0F` returns accept/`1`.
   - If the cached building rejects, is dead, or no longer accepts the aircraft, clear it and search again.
   - Test proposal: `airfield_cached_dock_revalidates_contact_slot_before_reuse`.

2. Tie pad identity to contact slot.
   - `NumberOfDocks` should set airfield contact capacity.
   - A multi-pad airfield's `DockingOffsetN` should be selected by the aircraft's radio contact slot index.
   - Avoid a separate pad index that can diverge from the contact slot.

3. Treat reload lifetime as contact lifetime.
   - A landed/reloading aircraft remains linked to the airfield contact slot while `UnitReload` sends reload/status radio messages.
   - Destroyed aircraft/airfield must clear both the contact and the aircraft cached dock.
   - Do not promote a hidden FIFO waiter unless a higher-level aircraft mission path is later verified to do that.

## Negative Facts / Do Not Do

- Do not implement stock airfield capacity as an independent FIFO queue. The verified primitive is sparse RadioClass contact slots with HELLO/BREAK acceptance, not a queue.
- Do not let `AirfieldDocks` pad index diverge from the RadioClass contact slot. `BuildingClass::GetDockCoord` uses `FindDockSlot(contact)` for `DockingOffsetN`.
- Do not treat `AircraftClass+0x6CC` `CachedDock` as authoritative. It is revalidated with radio `0x0F` and cleared on rejection.
- Do not release the airfield contact merely because the aircraft has touched down. The UnitReload loop continues to address contacted aircraft until reload/mission/contact cleanup.
- Do not reuse refinery/harvester reciprocal dock-link behavior for aircraft. Airfield docking is radio-contact driven.

## Remaining Uncertainty

- Exact reload cadence and timer return values inside `BuildingClass::MissionRepairAndProduce` were not fully decoded because they are not required to reconcile contact lifetime.
- The internals of `FootClass::Receive_Radio` handling for `0x1F` ammo increment were not re-decompiled in this slice; the aircraft/building caller edges are verified.
- `PadAircraft` INI reading exists elsewhere, but this slice did not trace production-selection use. It is not needed for `CachedDock`/contact lifetime.
- Carryall/passenger radio branches are present but not normal stock ORCA/BEAG airfield reload behavior.

## Stale-Doc Wording

Suggested replacement for broad aircraft/Rust gap docs:

> `CachedDock` is not a reservation. It is a nullable airfield/helipad pointer reused only after `AircraftClass` sends radio `0x0F` and receives `1`; if the reply is not `1`, it is cleared and the aircraft searches again. Airfield pad identity is the `RadioClass` contact slot. `BuildingType.NumberOfDocks` sizes the building contact array, and `BuildingClass::GetDockCoord` selects `DockingOffsetN` by `RadioClass::FindDockSlot(contact)`.

For `RUST_RADIO_ABSTRACTION_GAP_SCAN_GHIDRA_REPORT.md`, the prior open wording that Rust `AirfieldDocks` was not reconciled with aircraft `Receive_Radio`/`CachedDock` can now point to this report. The resolved delta is: Rust's `slots` can represent contact slots, but its FIFO queue/promotion and lack of radio `0x0F` cached-dock revalidation are not binary-backed.

## Coverage Ledger

- `AircraftClass::Receive_Radio @ 0x004190B0`: examined for airfield-relevant messages `0x0F`, `0x12`, `0x13`, `0x1D`, `0x1F`, `0x21`; passenger/carryall paths treated as conditional contrast.
- `AircraftClass::FindBuildingToDock @ 0x0041BBD0`: complete for `CachedDock` read/revalidate/write.
- `AircraftClass::Detach @ 0x0041B660`: complete for `CachedDock` pointer-expiry cleanup.
- `AircraftClass` mission/destination paths around `0x00419C80` and vtable-bound destination setter `0x0041AA80`: sampled to confirm cached building and radio HELLO/MoveHere paths; full flight-state behavior not pursued.
- `BuildingClass::Receive_Radio @ 0x0043C2D0`: sampled for helipad `0x0F` acceptance.
- `BuildingClass::MissionRepairAndProduce @ 0x0044B780`: sampled for `UnitReload` contact loop and aircraft radio messages.
- `BuildingClass::Constructor @ 0x0043B740`: complete for `NumberOfDocks` -> RadioClass contact capacity.
- `BuildingClass::GetDockCoord @ 0x00447B20`: complete for contact-slot -> `DockingOffsetN`.
- `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `Receive_Radio @ 0x0065A820`, `FindDockSlot @ 0x0065AD90`, `Set_Contact_Count @ 0x0065AE60`: complete for contact lifetime/capacity semantics used by this slice.

## Open Questions Final State

- OQ-01: Is `CachedDock` a durable reservation? Resolved: No. It is a revalidated pointer cache.
- OQ-02: Where is `CachedDock` written? Resolved: `FindBuildingToDock` stores the dock search result; mission/destination code can also refresh it to the current NavCom building.
- OQ-03: How is `CachedDock` invalidated? Resolved: radio `0x0F` rejection clears it, and pointer-expiry cleanup clears it if the building expires.
- OQ-04: What sizes airfield pad capacity? Resolved: `NumberOfDocks` sizes RadioClass contact capacity at building construction.
- OQ-05: What selects `DockingOffsetN`? Resolved: `RadioClass::FindDockSlot(contact)`.
- OQ-06: Is there a verified FIFO wait queue? Resolved: Not in the radio/contact evidence for this target.
- OQ-07: Who drives reload while docked? Resolved: UnitReload building mission loop over contacts.
- OQ-08: How does full/reloaded aircraft answer? Resolved at radio surface: aircraft `0x1D` returns full/no-target status; `0x1F` enters reload/give-ammo path or early-answers under target/ammo gating.
- OQ-09: What clears contacts on death? Resolved at primitive level: BREAK/contact cleanup plus aircraft pointer expiry for `CachedDock`; full global death broadcast path not re-opened because prior radio cleanup docs cover it and local cleanup evidence was sufficient for this slice.
- OQ-10: Is Carryall normal airfield behavior? Resolved: Conditional only; present branches are not the stock ORCA/BEAG UnitReload path.
