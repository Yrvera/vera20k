# ReleaseDockedHarvester Exit Radio 0x19 / 0x03 - Ghidra Research Report

**Target:** `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`  
**Entry context only:** `UnitClass::Mission_Deploy_Building @ 0x0073D630`  
**Investigation mode:** exhaustive-slice  
**Date:** 2026-05-21  
**Status:** COMPLETE  
**Scope:** decide whether this helper sends refinery-exit radio `0x19` in addition to `0x03`, or only `0x03`; identify branch conditions and the immediate state-machine handoff around the docked-harvester departure branch.

## Summary

`ReleaseDockedHarvester` directly sends only radio `0x03` through the building's `Transmit_Radio_ToFirst` vtable slot `+0x274`. There is no direct `0x19` send in the helper body.

However, `0x03` is not the whole protocol consequence. When the target receives `BREAK`, `TechnoClass::Receive_Radio @ 0x006F4AB0` can send `0x19` first if both receiver and sender still have byte `+0x418` set. That means the narrow answer is:

- **Direct helper behavior:** only `0x03`.
- **Protocol cascade caused by that `0x03`:** conditional `0x19` can fire before the receiver falls through to base `RadioClass::Receive_Radio(0x03)`.

This also confirms the newer branch correction: the helper is not the stock cargo-empty state-4 path for ordinary zero-link GAREFN/NAREFN unloading. It is reached from `Mission_Deploy_Building` only when unit `+0x2E4` is already nonzero and a building is found at the unit's current cell.

## Verified Findings

### 1. `ReleaseDockedHarvester` has one terminal radio send, and it is `0x03`

At the end of the successful DriveLocomotion branch, the helper clears the building-side dock link and then calls building vtable slot `+0x274` with immediate `3`.

Evidence:

- `0x00459814`: writes `0` to building `+0x2E4`.
- `0x0045981A`: writes `0` to building `+0x718`.
- `0x00459828`: `PUSH 0x3`.
- `0x0045982C`: `CALL [building_vtable + 0x274]`.
- `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` sends the supplied message to `Contacts[0]` if present, otherwise returns `0`.

**Active in YR:** Conditional. The helper is live code, but this terminal send runs only after the helper passes the non-null building `+0x2E4` docked-unit pointer and the docked unit's locomotion type query returns `1` (DriveLocomotion).

### 2. No direct `0x19` send exists in the helper body

The decompiled helper body was checked from entry `0x004595C0` through return. The radio send in this helper is the single `vtable+0x274` call at `0x0045982C` with immediate `3`. The other vtable calls in the successful branch are locomotion power/track, speed, destination, mission, and building mission calls.

Evidence:

- Full Ghidra decompile of `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`.
- Assembly terminal context `0x00459805..0x00459839` shows `Set_Destination`, unit `SetMission(2)`, building link clears, building `SetMission(5)`, then `PUSH 0x3` / `CALL [vtable+0x274]` / return.
- No `PUSH 0x19` or `vtable+0x278(0x19, target)` appears in the helper body.

**Active in YR:** Yes as a negative finding for this helper body; conditional only in the sense that the helper body is reached under the `+0x2E4` branch described below.

### 3. `0x19` can still be caused indirectly by the receiver of `0x03`

`TechnoClass::Receive_Radio @ 0x006F4AB0`, case `0x03`, checks byte `+0x418` on both the receiver and sender. If both are nonzero, it sends `0x19` to the sender before falling through to base `RadioClass::Receive_Radio(0x03)`.

Evidence:

- `0x006F4C50`: reads receiver byte `+0x418`.
- `0x006F4C5A` / `0x006F4C5C`: zero receiver flag skips to base handling.
- `0x006F4C5E`: reads sender byte `+0x418`.
- `0x006F4C64` / `0x006F4C66`: zero sender flag skips to base handling.
- `0x006F4C6B`: pushes `0x19`.
- `0x006F4C7A`: calls vtable slot `+0x278` directed at the sender.
- `0x006F4C80` then continues into base radio handling for the original message.

**Active in YR:** Conditional. The case is live for YR Techno-derived objects. The cascade requires both sides' `+0x418` dock/contact flags to be set at the moment `BREAK` is received.

### 4. `0x19` clears `+0x418` and propagates before returning

`TechnoClass::Receive_Radio(0x19)` clears receiver byte `+0x418`, then sends directed `0x19` back to the sender through vtable slot `+0x278`.

Evidence:

- `0x006F4BA6`: writes `0` to receiver `+0x418`.
- `0x006F4BAD`: calls vtable slot `+0x278` after pushing `0x19` and the sender pointer.
- Existing `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md` verifies the same write/propagate order and distinguishes `+0x418` from unit/building `+0x2E4`.

**Active in YR:** Conditional. The handler is live; it performs the clear only when receiver `+0x418` was already nonzero.

### 5. `Transmit_Radio_ToFirst(3)` removes the building's contact before dispatching `BREAK`

The helper calls vtable slot `+0x274`, which resolves to `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0`. That wrapper uses `Contacts[0]` and calls `Transmit_Radio_Impl @ 0x0065A970`. For message `3`, the implementation clears matching target entries from the sender's contact vector before dispatching the target's `Receive_Radio(3)`.

Evidence:

- `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0`: if `Contacts[0] != 0`, calls vtable `+0x27C` with message and `Contacts[0]`.
- `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, message `3`: loops over `Contacts[0..Capacity)` and writes `0` to matching target slots, then dispatches target vtable `+0x194`.
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` independently documents the same sparse-null break semantics.

**Active in YR:** Yes. RadioClass is the live shared protocol for building/unit dock links.

### 6. The entry from `Mission_Deploy_Building` is conditional on unit `+0x2E4 != 0`

`Mission_Deploy_Building` calls `ReleaseDockedHarvester` only in the top-level nonzero `unit+0x2E4` branch, before the ordinary stock harvester unload FSM. The call site is not the cargo-empty state-4 path.

Evidence:

- `0x0073D63B`: compares unit `+0x2E4` with zero.
- `0x0073D641`: zero branch jumps into the ordinary deploy/refinery FSM.
- `0x0073D66D`: calls `0x004595C0` after building lookup.
- `get_function_xrefs 0x004595C0` reports only the `0x0073D66D` callsite.
- `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md` verifies stock zero-link state 3/4 unload can complete without this helper.

**Active in YR:** Conditional. The mission handler is live for HARV/CMIN, but this helper path requires a nonzero unit `+0x2E4` link.

### 7. The helper's successful branch performs local departure state handoff before radio cleanup

Before the terminal radio `0x03`, the helper:

- clears unit-side `+0x2E4`,
- powers on the active locomotor,
- force-tracks with track index `0x47`,
- sets speed multiplier `1.0`,
- computes a passable destination near `Get_Cell_Packed()+(-1,+1)`,
- sets the unit destination,
- sets the unit mission to `2`,
- clears building-side `+0x2E4` and `+0x718`,
- sets building mission `5`,
- then sends `0x03`.

Evidence:

- Ghidra decompile of `0x004595C0`.
- Assembly terminal context `0x004597FA` destination call, `0x00459807` unit mission call, `0x00459814`/`0x0045981A` building-field clears, `0x00459820` building mission call, `0x00459828`/`0x0045982C` radio send.

**Active in YR:** Conditional. Runs only for the non-null docked unit with locomotion type `1`.

## Narrow Answer To Deferred OQ-1

**Does `0x19 LEAVE_DOCK` fire on refinery exit, in addition to `BREAK(0x03)`?**

For `ReleaseDockedHarvester` itself: **No direct `0x19`; only direct `0x03`.**

For the radio protocol consequence of that `0x03`: **Conditional yes.** If the target's `TechnoClass::Receive_Radio(0x03)` sees both receiver and sender `+0x418` set, it sends `0x19` before base `BREAK` handling clears the contact. Therefore an implementation should not model this helper as explicitly sending `0x19`, but a faithful RadioClass/TechnoClass implementation must allow `BREAK` to trigger the `0x19` dock/contact-flag cleanup cascade.

For stock uninterrupted GAREFN/NAREFN cargo-empty unload: **this helper is not the ordinary zero-link state-4 completion path according to current binary evidence.** Stock zero-link state 4 clears `+0x6D1`, sets mission Harvest (`0x0A`), may send `0x03` through the unit's `vtable+0x274`, and queues the next mission. Its later `+0x418` cleanup belongs to the per-cell/radio cleanup path, not to this helper.

## Open Questions

- Exact live-frame timing of the indirect `0x19` cascade after ordinary stock zero-link state-4 exit remains a runtime trace question. Static code proves the conditional cascade and its gates, not the exact frame in a live replay.
- Slot 4 of this swarm owns the full `+0x2E4` writer/read inventory. This report only uses the branch evidence needed to classify this helper's reachability.

## Sources

- Ghidra decompile: `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`.
- Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra decompile: `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0`.
- Ghidra decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`.
- Ghidra decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`.
- Ghidra xrefs: `get_function_xrefs 0x004595C0` -> `0x0073D66D` only.
- Prior docs: `C:/Users/enok/Documents/ra2-rust-game-docs/RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`.
- Prior docs: `C:/Users/enok/Documents/ra2-rust-game-docs/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- Prior docs: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`.

