# RadioHistory Read/Use Scan - Ghidra Research Report

**Address(es):** `0x0065A750` `RadioClass__Constructor`, `0x0065A820` `RadioClass__Receive_Radio`, `0x0065AB10` radio save helper, `0x0065AB80` radio load helper, `0x0065AC40` radio stream-save helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Determine whether RadioClass/TechnoClass instance fields `+0xD4`, `+0xD8`, and `+0xDC` are read outside RadioHistory self-maintenance in `RadioClass__Receive_Radio`, including statically discoverable save/load/debug paths.  
**Non-Scope:** unrelated class fields at the same byte offsets; full radio message semantics; contact-array behavior except where it proves save/load boundaries.  
**Confidence:** High for RadioClass functions and save/load; medium-high for global negative consumer proof because it is static instruction/decompile evidence rather than a runtime read watchpoint.  
**Active in YR:** Yes for writes and self-maintenance reads in `Receive_Radio`; No verified gameplay/debug consumer beyond that.

## 0. Investigation Gate

**Target question:** Are RadioHistory fields at Techno/RadioClass `+0xD4/+0xD8/+0xDC` ever read in `gamemd.exe` beyond being written/maintained by `RadioClass__Receive_Radio`?

**Non-goals:** Do not investigate unrelated Techno fields, unrelated classes with the same byte offsets, or the full radio protocol.

**Evidence needed to mark COMPLETE:** decompile and assembly/bytes for the writer/self-maintenance site; binary-wide displacement scan for memory operands using `0xD4`, `0xD8`, and `0xDC`; save/load/debug path check; Rust handoff for omit/model decision.

**Stop conditions:** Stop if global scan plus decompiled RadioClass/save/load functions show no external reader; or, if any external reader is found, decompile it and identify player-visible effect.

## 1. Overview

RadioHistory is a three-dword linear push-down log at `RadioClass +0xD4/+0xD8/+0xDC`. The live base receive handler updates it only when the new radio message differs from the current head. No verified consumer reads the log for gameplay, save/load, or discoverable debug output.

The only confirmed reads of these fields on a RadioClass object are inside `RadioClass__Receive_Radio` itself: read `+0xD4` to suppress duplicate pushes, read `+0xD8` to shift the previous second entry down, then write `+0xD8`, `+0xDC`, and `+0xD4`.

## 2. Class Layout / Key Offsets

| Offset | Type | Meaning in this slice | Evidence |
|--------|------|-----------------------|----------|
| `+0xD4` | `int` | RadioHistory[0], most recent distinct radio message | `RadioClass__Constructor @ 0x0065A750`; `RadioClass__Receive_Radio @ 0x0065A820` |
| `+0xD8` | `int` | RadioHistory[1], previous distinct message | same |
| `+0xDC` | `int` | RadioHistory[2], older distinct message | same |
| `+0xE4` | pointer | Contacts array buffer, serialized by radio save/load helpers | `FUN_0065AB10`, `FUN_0065AB80`, `FUN_0065AC40` |
| `+0xE8` | `int` | Contacts capacity/count, serialized by radio save/load helpers | same |

## 3. Core Logic

`RadioClass__Constructor @ 0x0065A750` zeroes all three history dwords after allocating the one-slot default contacts buffer.

Verified disassembly range from bytes at `0x0065A7B8..0x0065A7D8`:

```asm
0065A7C4  89 8E D4 00 00 00    mov [esi+0D4h], ecx
0065A7CA  89 8E D8 00 00 00    mov [esi+0D8h], ecx
0065A7D2  89 8E DC 00 00 00    mov [esi+0DCh], ecx
```

Decompile evidence:

```text
param_1[0x35] = 0;    // +0xD4
param_1[0x36] = 0;    // +0xD8
param_1[0x37] = 0;    // +0xDC
```

`RadioClass__Receive_Radio @ 0x0065A820` maintains the log before message-specific handling:

```text
if (msg != *(int *)(this + 0xD4)) {
    old_second = *(int *)(this + 0xD8);
    *(int *)(this + 0xD8) = *(int *)(this + 0xD4);
    *(int *)(this + 0xDC) = old_second;
    *(int *)(this + 0xD4) = msg;
}
```

Verified disassembly range from bytes at `0x0065A820..0x0065A84A`:

```asm
0065A829  8B 86 D4 00 00 00    mov eax, [esi+0D4h]   ; read current head
0065A82F  3B D8                cmp ebx, eax          ; duplicate-message gate
0065A833  8B 8E D8 00 00 00    mov ecx, [esi+0D8h]   ; read second entry
0065A839  89 86 D8 00 00 00    mov [esi+0D8h], eax   ; old head -> second
0065A83F  89 8E DC 00 00 00    mov [esi+0DCh], ecx   ; old second -> third
0065A845  89 9E D4 00 00 00    mov [esi+0D4h], ebx   ; new msg -> head
```

Tiny details:

- Duplicate messages do not push the history. `msg == History[0]` leaves all three fields unchanged.
- `+0xDC` is never read by `Receive_Radio`; it is only overwritten with the previous `+0xD8`.
- The history update happens before HELLO/BREAK/base `ObjectClass__Receive_Radio` dispatch.
- The self-maintenance reads do not branch on `+0xD8/+0xDC` values for gameplay; `+0xD8` is just a temporary source for the shift.

## 4. INI Keys

No INI keys apply. This is runtime-only per-object scratch state; no radio-history key was found in the repo INI files or in the Ghidra string scan.

## 5. Integration Points

### Radio receive path

`RadioClass__Receive_Radio @ 0x0065A820` has one direct Ghidra caller, `TechnoClass__Receive_Radio @ 0x006F4AB0`. Derived class receive handlers fall through through Techno/base dispatch in prior radio reports, but this slice only needed the base reader/writer site.

`RadioClass__Receive_Radio` callees are `HouseClass__Is_Ally_ByObject @ 0x004F9A90` and `ObjectClass__Receive_Radio @ 0x005F5320`. Neither callee receives a history pointer or value; the decompile passes sender/message/payload only.

### Radio save/load paths

`TechnoClass__Save @ 0x0070C270` calls `FUN_0065AB10`, which saves MissionClass state and the radio contacts capacity/entries:

```text
MissionClass__Save(...)
save *(this + 0xE8)
for each contact in *(this + 0xE4): save contact ID/type
```

No `+0xD4/+0xD8/+0xDC` read appears in this save helper.

`FUN_0070BF50` is the Techno load helper called by `BuildingClass__Load @ 0x00453E20` and `FootClass__Load @ 0x004DB3C0`; it calls `FUN_0065AB80`, which reloads only the contacts vector metadata/entries (`+0xE0/+0xE4/+0xE8/+0xEC/+0xED`) after the lower object load.

`FUN_0070C250` is the save wrapper called by `BuildingClass__Save @ 0x00454190` and `FootClass__Save @ 0x004DB690`; it calls `FUN_0065AC40`, which stream-saves `AbstractClass` and contacts `+0xE8/+0xE4` only.

### Debug/discoverability scan

Ghidra string scan for `Radio|RADIO|radio` finds only RTTI and Carryall radio debug strings:

- `.?AVRadioClass@@`
- `Do_MISSION_MOVE_Carryall - LAND - RADIO_NEED_TO_MOVE got RADIO_ROGER`
- `Do_MISSION_MOVE_Carryall - VALIDATE_LZ - RADIO_WANT_RIDE did not get RADIO_ROGER`
- `Do_MISSION_MOVE_Carryall - VALIDATE_LZ - RADIO_HELLO got RADIO_ROGER`

String scan for `History|history` finds World Domination Tour history strings, not RadioHistory. No discoverable debug/log formatting string names or dumps the three radio-history fields.

### Binary-wide offset scan

A static executable-section scan of `gamemd.exe` memory operands with displacement exactly `0xD4`, `0xD8`, or `0xDC` was run and then filtered by Ghidra function identity:

- Total memory-displacement hits in executable section: 923.
- Non-stack, non-vtable-call hits: 347.
- RadioClass hits: exactly 8 direct instructions:
  - Constructor writes at `0x0065A7C4`, `0x0065A7CA`, `0x0065A7D2`.
  - Receive self-maintenance at `0x0065A829`, `0x0065A833`, `0x0065A839`, `0x0065A83F`, `0x0065A845`.
- Nearby/same-offset hits outside RadioClass resolve to unrelated classes or non-object uses, for example:
  - `0x0040214A` in `DSoundBuffer__Create`.
  - `0x00671926` in `RulesClass__ReadGeneral`.
  - `0x0072203B` in `TiberiumClass__Constructor`.
  - `0x0071D175` in `TerrainClass__Get_Render_Rect`.
  - `0x006F079F` in `TeamTypeClass__Constructor`, not TechnoClass despite address proximity.

This rules out a known direct RadioClass/Techno consumer in ordinary decompiled code. It does not claim that no unrelated class has fields at the same byte offsets.

## 6. Current Rust Implementation Status

Rust scan results:

- Codegraph search for `radio` found no modeled RadioHistory symbol.
- `rg` found no `RadioHistory` or radio-history fields in `src/`.
- Existing radio-related Rust work appears in miner/contact behavior, e.g. `src/sim/miner/miner_dock_sequence.rs` and `src/sim/world/world_hash.rs`, but no three-slot receive-message history exists.

Current Rust delta: none required for player-visible behavior if Rust omits RadioHistory. If Rust later adds debugging parity or savegame-byte-layout parity, it should still avoid making gameplay decisions from this log unless a future binary watchpoint proves a consumer.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `RadioClass__Constructor @ 0x0065A750` | verified | decompile plus bytes/disassembly `0x0065A7C4..0x0065A7D8` | none |
| `RadioClass__Receive_Radio @ 0x0065A820` history update | verified | decompile plus bytes/disassembly `0x0065A829..0x0065A845` | none |
| `RadioClass__Transmit_Radio_Impl @ 0x0065A970` | verified | batch decompile; uses `+0xE4/+0xE8`, no `+0xD4/+0xD8/+0xDC` | none |
| `RadioClass__Transmit_Radio`, `Transmit_Radio_ToFirst`, `Broadcast_Radio_ToAll`, `FindDockSlot`, `Set_Contact_Count` | verified | batch decompile of all named RadioClass functions | none |
| Radio save helper `FUN_0065AB10` | verified | decompile; saves MissionClass plus contacts only | none |
| Radio load helper `FUN_0065AB80` | verified | decompile; reloads contacts only | none |
| Radio stream-save helper `FUN_0065AC40` | verified | decompile; saves AbstractClass plus contacts only | none |
| Techno save/load callers | verified | callers: `TechnoClass__Save`, `FUN_0070BF50`, `FUN_0070C250`; subclass save/load callers listed by Ghidra | none |
| Debug/string path | verified | Ghidra string scans for `Radio|RADIO|radio` and `History|history` | no runtime debugger watchpoint was set |
| Binary-wide direct-offset scan | verified | executable-section memory operand scan plus Ghidra spot checks for candidate function identities | runtime watchpoint remains optional follow-up, not required for static omission |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What is the target slice? -> RadioClass/Techno instance fields +0xD4/+0xD8/+0xDC only.` (evidence: task scope)
- `[RESOLVED] OQ-2 - Does constructor initialize the fields? -> Yes, all three are zeroed.` (evidence: `0x0065A750`, disassembly `0x0065A7C4..0x0065A7D8`)
- `[RESOLVED] OQ-3 - Does Receive_Radio read the fields? -> Yes, +0xD4 and +0xD8 are read for duplicate suppression and shift maintenance; +0xDC is not read there.` (evidence: `0x0065A820`, disassembly `0x0065A829..0x0065A845`)
- `[RESOLVED] OQ-4 - Is there any gameplay branch on the history values inside Receive_Radio? -> No; only msg equality against head suppresses pushing a duplicate history entry.` (evidence: `0x0065A820` decompile)
- `[RESOLVED] OQ-5 - Do transmit helpers read history? -> No; all named RadioClass transmit/contact helpers use contacts +0xE4/+0xE8 and scratch payloads.` (evidence: batch decompile of named RadioClass functions)
- `[RESOLVED] OQ-6 - Does TechnoClass__Receive_Radio consume history? -> No; it delegates to RadioClass__Receive_Radio and handles other Techno flags such as +0x418/+0x419.` (evidence: `0x006F4AB0` decompile)
- `[RESOLVED] OQ-7 - Are history fields saved? -> No; radio save helpers serialize contacts, not +0xD4/+0xD8/+0xDC.` (evidence: `0x0065AB10`, `0x0065AC40`)
- `[RESOLVED] OQ-8 - Are history fields loaded? -> No; radio load helper rebuilds contacts and pointer fixups only.` (evidence: `0x0065AB80`, `0x0070BF50`)
- `[RESOLVED] OQ-9 - Does debug/log string evidence expose RadioHistory? -> No discoverable string names or formats RadioHistory.` (evidence: string scans)
- `[RESOLVED] OQ-10 - Do same-offset hits elsewhere imply consumers? -> No; sampled same-offset hits resolve to unrelated classes such as DSoundBuffer, RulesClass, TiberiumClass, TerrainClass, TeamTypeClass.` (evidence: `get_function_by_address` spot checks)
- `[RESOLVED] OQ-11 - Is this active in YR? -> The write/self-maintenance path is active whenever radio messages are received in YR; no active YR consumer was found.` (evidence: `TechnoClass__Receive_Radio` caller of `RadioClass__Receive_Radio`)
- `[RESOLVED] OQ-12 - Can Rust omit the fields? -> Yes for player-visible parity; no verified consumer observes them.` (evidence: negative scan plus save/load/debug decompiles)
- `[DEFERRED] OQ-13 - Would a hardware read watchpoint during a full live match ever fire outside Receive_Radio?` (category: `needs-runtime-debugger`; reason: static evidence is already sufficient for implementation omission, but dynamic watchpoints would be an optional audit; next-step-if-pursued: set read watchpoints on a live object's +0xD4/+0xD8/+0xDC after construction and run refinery/carryall/contact scenarios)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| RadioHistory is initialized and maintained but has no verified consumer outside receive self-maintenance | `0x0065A750`, `0x0065A820`, global displacement scan | Rust currently omits it; this is acceptable | `src/sim` radio/contact/miner surfaces | Deliberately omit gameplay state for RadioHistory | `radio_history_is_not_modeled_and_does_not_affect_contact_protocol` | Do not add gameplay branches based on previous radio messages |
| Save/load does not persist RadioHistory | `FUN_0065AB10`, `FUN_0065AB80`, `FUN_0065AC40` | Rust save/load unchecked, but no history persistence is needed | future savegame serialization | Do not serialize a three-slot radio message history for parity | save/load a dock/contact scenario and confirm contacts restore without history state | Do not confuse contacts `+0xE4/+0xE8` with history `+0xD4/+0xD8/+0xDC` |
| Duplicate receive messages do not push the log, but this has no observed output because no consumer reads it | `0x0065A829..0x0065A845` | omitted | none unless debug tooling wants native-like inspection | No player-visible behavior required | repeated same radio message should not change any Rust gameplay result | If adding debug-only history, keep it non-authoritative |

Concrete Rust test-name proposal:

`radio_history_is_not_modeled_and_does_not_affect_contact_protocol`

### Stale Docs / Follow-up Docs

No stale-doc correction is required from this slice. Suggested wording if `RADIO_CLASS_PROTOCOL` carries the old open question:

> Binary-wide RadioHistory read/use scan resolved: `RadioClass +0xD4/+0xD8/+0xDC` is initialized by `RadioClass__Constructor` and self-maintained by `RadioClass__Receive_Radio`; save/load serializes contacts only (`+0xE4/+0xE8`), and no gameplay/debug consumer was found. Rust may omit RadioHistory for player-visible parity.

## Sources

- Ghidra decompile: `RadioClass__Constructor @ 0x0065A750`
- Ghidra decompile: `RadioClass__Receive_Radio @ 0x0065A820`
- Ghidra decompile: `RadioClass__Transmit_Radio_Impl @ 0x0065A970`
- Ghidra decompile: `RadioClass__Transmit_Radio @ 0x0065AAA0`
- Ghidra decompile: `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0`
- Ghidra decompile: `RadioClass__Broadcast_Radio_ToAll @ 0x0065ACE0`
- Ghidra decompile: `RadioClass__FindDockSlot @ 0x0065AD90`
- Ghidra decompile: `RadioClass__Set_Contact_Count @ 0x0065AE60`
- Ghidra decompile: `FUN_0065AB10`, `FUN_0065AB80`, `FUN_0065AC40`
- Ghidra decompile: `TechnoClass__Receive_Radio @ 0x006F4AB0`
- Ghidra decompile: `TechnoClass__Save @ 0x0070C270`, `FUN_0070BF50`, `FUN_0070C250`
- Ghidra caller evidence for radio save/load helpers and receive helper
- Ghidra string scans for `Radio|RADIO|radio` and `History|history`
- Static executable-section memory operand scan of `gamemd.exe` for displacements `0xD4`, `0xD8`, `0xDC`
- Rust scans: Codegraph `radio`; `rg RadioHistory`, `rg Receive_Radio`
