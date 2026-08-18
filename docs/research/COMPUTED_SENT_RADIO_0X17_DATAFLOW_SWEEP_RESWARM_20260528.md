# Computed Sent Radio `0x17` Dataflow Sweep - Re-Swarm Research Report

**Address(es):** transmit slots `+0x278`, `+0x27C`, `+0x280`; receive slot `+0x194`; core helpers `0x0065AAA0`, `0x0065A970`, `0x0065ACE0`; comparison producers `0x0043CB47`, `0x004425A4`, `0x004485B1`, `0x0044AB68`  
**Investigation Mode:** exhaustive-slice for a bounded vtable-slot caller census  
**Claimed Scope:** Binary-wide byte-pattern sweep for indirect calls through radio slots `+0x278`, `+0x27C`, `+0x280`, and direct receiver slot `+0x194`, classifying whether each call site's message argument is a literal constant, helper pass-through, or computed value that could become sent radio `0x17`.  
**Non-Scope:** Direct calls to non-vtable helper addresses outside these slot patterns, full receiver semantics for message `0x17`, and full dataflow for non-radio functions that may compute integers equal to `0x17` but do not call these slots.  
**Confidence:** High for "no additional computed sent-0x17 producers within this slot census"; Medium for binary-global negative outside slot-call patterns.  
**Active in YR:** Yes. The radio slots are universal active YR infrastructure.

## 1. Overview

This follow-up closes the deferred computed-message question from `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md` for the bounded radio-slot surface. The caller census found no producer call site that computes a non-immediate message value which can become sent radio `0x17`.

The only variable-message paths are the generic RadioClass helper bodies: `RadioClass::Transmit_Radio`, `RadioClass::Transmit_Radio_Impl`, and `RadioClass::Broadcast_Radio_ToAll`. Those helpers forward the caller's already supplied message. They do not compute or transform some other value into `0x17`. Therefore the only sent-radio `0x17` producers in this bounded sweep remain the known literal producers: legacy Hospital/Armory cleanup, building death far-contact cleanup, building owner-change eviction, and building sell broadcast.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence |
|---:|---|---|---|
| `+0x194` | Radio vtable family | `Receive_Radio(sender,msg,payload)` dispatch slot | `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`; direct calls at `0x0065A9DB`, `0x0065AA49` |
| `+0x278` | Radio vtable family | `Transmit_Radio(msg,target)`, wrapper around `+0x27C` with global scratch payload | decompile `0x0065AAA0`; caller sweep pattern `FF ?? 78 02 00 00` |
| `+0x27C` | Radio vtable family | `Transmit_Radio_Impl(msg,payload,target)` | decompile `0x0065A970`; caller sweep pattern `FF ?? 7C 02 00 00` |
| `+0x280` | Radio vtable family | `Broadcast_Radio_ToAll(msg)` | decompile `0x0065ACE0`; caller sweep pattern `FF ?? 80 02 00 00` |
| `+0xE4` | RadioClass | Contact pointer array | `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md`; decompile `0x0065A970` |
| `+0xE8` | RadioClass | Contact capacity, not live count | same |

## 3. Core Logic

### Bounded method

The sweep searched executable bytes for vtable slot-call encodings:

| Slot | Search pattern | Hits | Radio-call hits after context filter | Message provenance |
|---|---|---:|---:|---|
| `+0x278` | `FF ?? 78 02 00 00` | 86 | 86 | all producer call sites push immediate message constants |
| `+0x27C` | `FF ?? 7C 02 00 00` | 9 | 8 plus one non-call false positive | constants at non-helper callers; variable stack argument only inside helper wrappers |
| `+0x280` | `FF ?? 80 02 00 00` | 13 | 11 plus two no-instruction/false-positive entries | all call sites push immediate message constants |
| `+0x194` | `FF ?? 94 01 00 00` | 9 | 2 direct receiver calls plus data false positives | variable message only inside `Transmit_Radio_Impl` forwarding |

For each real call, the assembly context was inspected around the call. The key test was the pushed message argument immediately before the vtable call:

- `+0x278`: signature is `target`, then `msg`; all real external producer contexts have `PUSH <target>` plus `PUSH imm`.
- `+0x27C`: signature is `target`, then `payload`, then `msg`; non-helper callers push immediate `0x12`, and helper bodies pass their own caller's argument.
- `+0x280`: signature is `msg`; all real call sites push an immediate constant.
- `+0x194`: direct calls are only inside `RadioClass::Transmit_Radio_Impl`, which forwards `param_2` from the already classified transmit caller.

### Radio helper pass-through is not a producer

`RadioClass::Transmit_Radio @ 0x0065AAA0` decompiles to a wrapper:

```text
call vtable+0x27C(param_2, &g_RadioScratchBuffer, param_3)
```

Assembly at `0x0065AAB1` confirms the wrapper pushes its stack arguments and calls `+0x27C`. It does not branch, compare, or remap the message value.

`RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0` loops `Contacts[0..Capacity)` and calls `+0x27C(param_2, &g_RadioScratchBuffer, contact)` for each non-null contact. Decompile and assembly at `0x0065AD0D` confirm the broadcast helper also forwards its input message unchanged.

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` is where variable `param_2` reaches direct receiver slot `+0x194`. Its direct receive calls at `0x0065A9DB` and `0x0065AA49` forward the same `param_2` after contact bookkeeping or the HELLO path. It treats `2` and `3` specially, but for other values, including `0x17`, it does not synthesize them; it just forwards the already supplied message.

### External `+0x278` caller census

All real `+0x278` call sites in the bounded sweep use immediate messages. The observed constants are:

| Message | Representative call sites | Meaning / role in this slice |
|---:|---|---|
| `0x02` | `0x00415C2E`, `0x00416E64`, `0x004179C5`, `0x0041A0A9`, `0x0044401B`, `0x00737569`, `0x00742EF7` | HELLO/contact insertion |
| `0x03` | `0x0041943A`, `0x004485BE`, `0x0044EF5A`, `0x006F4C41`, `0x0070D889` | BREAK/contact removal |
| `0x0C` | `0x004FB2AD` | non-`0x17` radio command |
| `0x0E` | `0x0041ABD6`, `0x004D92B9`, `0x0073A981`, `0x00741DDA` | enter/dock request family |
| `0x0F` | many: `0x00417E68`, `0x00419224`, `0x0041AB62`, `0x0041BBF4`, `0x0051E8EF`, `0x00740182` | move/stop/contact query family |
| `0x13` | `0x00419413`, `0x0043C9FC`, `0x004485D0`, `0x0044990F`, `0x0044C8AC`, `0x00737A5C` | need-to-move/status query family |
| `0x15` | `0x0073777A`, `0x0073A5C8` | unload/start service handoff |
| `0x16` | `0x0043CADB` | facing/sync handoff |
| `0x17` | `0x0043CB47`, `0x004425A4`, `0x004485B1` | known sent-`0x17` directed producers |
| `0x18` | `0x0043CACE`, `0x00444028`, `0x004445E3`, `0x00446FAA`, `0x006F4B79`, `0x006F4C7A` | mirrored endpoint/contact state set |
| `0x19` | `0x006F4BAD`, `0x006F4C34` | mirrored endpoint/contact state clear |
| `0x1A` / `0x1B` | `0x006F4BE1`, `0x006F4C15` | Techno radio bridge state messages |
| `0x1C` / `0x1D` | `0x0044C8F2`, `0x0044C873` | repair/service query family |
| `0x1F` | `0x0044C8DF` and branch-selected follow-ups around `0x0051E8EF..0x0051EE21` | non-`0x17` fallback/status path |
| `0x22` / `0x23` | `0x0043C837`, `0x0043CB35`, `0x0043C526`, `0x0043C571` | legacy service/repair query family |

One context looked superficially computed because it pushes a pointer loaded from a list before the call:

- `0x004492A3`: assembly context is `MOV ECX,[EAX]`, `PUSH ECX`, then `PUSH 0x2`, then `CALL [EDX+0x278]`. The computed value is the target pointer. The message is immediate `0x02`, not computed. Decompile of the enclosing building sold/update helper also shows `(**vtable+0x278)(2, **list_entry)`.

### External `+0x27C` caller census

The real non-helper `+0x27C` calls at `0x00419428`, `0x0043CA47`, `0x0043CAB8`, `0x0043CBAE`, and `0x00737A71` all push immediate `0x12` as the message and a stack/local payload pointer. These are movement-cell payload sends, not sent `0x17`.

The helper entries are:

- `0x0065AAB1`: `RadioClass::Transmit_Radio` forwards its caller's message to `+0x27C`.
- `0x0065ACC9`: `RadioClass::Transmit_Radio_ToFirst` forwards its caller's message to first contact if present.
- `0x0065AD0D`: `RadioClass::Broadcast_Radio_ToAll` forwards its caller's message to each non-null contact.

`0x00684EED` is not a call; it is `INC [EAX+0x27C]` and is a pattern false positive.

### External `+0x280` caller census

All real `+0x280` call sites push immediate constants. Only one uses `0x17`:

- `0x0044AB68`: `BuildingClass::Sell` state 0 pushes `0x17` and calls broadcast.

The other real broadcast calls push `0x03`: `0x0043F5B9`, `0x0043F639`, `0x0044A2F5`, `0x0044EED4`, `0x004C75E0`, `0x0065AA91`, `0x006CCC54`, `0x006FCD61`, `0x00702206`, `0x00710374`. These are BREAK/contact cleanup broadcasts, not computed `0x17`.

`0x00403F3C` and `0x00548E98` were no-instruction false positives for this pattern.

### Direct `+0x194` receive-slot census

The only real direct receive-slot calls found by the bounded slot pattern are inside `RadioClass::Transmit_Radio_Impl`:

- `0x0065A9DB`: default/BREAK direct dispatch to target `Receive_Radio(param_2, payload)`.
- `0x0065AA49`: HELLO dispatch after contact slot scan/eviction logic.

All other pattern hits were data or non-call operations such as `INC/DEC [x+0x194]`, not radio receive calls. Since both direct receive-slot calls live inside `Transmit_Radio_Impl`, they do not create new `0x17`; they forward `param_2` from the already-classified transmit caller.

## 4. INI Keys

No INI key directly controls message `0x17` or the computed/literal nature of radio message arguments.

| INI key / section | Stock YR value | Effect in this slice | Active in standard YR? |
|---|---|---|---|
| `Hospital=` | commented out in stock `rulesmd.ini` service-building entries | Gates one known literal sent-`0x17` legacy producer from the prior report, not a computed producer | No for stock, conditional for mods |
| `Armory=` | commented out in stock `rulesmd.ini` service-building entries | Same as above | No for stock, conditional for mods |
| `Helipad=yes` | set on helipad/airfield types | Selects the close/helipad branch in building death, avoiding the far-contact sent-`0x17` path | Yes |

## 5. Integration Points

- `RadioClass::Transmit_Radio @ 0x0065AAA0`: pass-through wrapper from slot `+0x278` to slot `+0x27C`.
- `RadioClass::Transmit_Radio_Impl @ 0x0065A970`: owns contact bookkeeping for `0x02` and `0x03`, then direct-calls target `+0x194`. It forwards arbitrary non-`0x02`/`0x03` messages unchanged.
- `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0`: loops contacts and forwards caller-supplied message through `+0x27C`.
- `Receive_Radio` implementations: receive whatever message the transmit helper was given. They are consumers, not producers, for this sweep.
- Known literal sent-`0x17` producers remain: `0x0043CB47`, `0x004425A4`, `0x004485B1`, `0x0044AB68`.

## 6. Current Rust Implementation Status

Rust has contact storage and direct contact clearing but no generic RadioClass message dispatcher:

- `src/sim/game_entity.rs`: `radio_contacts: Vec<u64>` and add/remove helpers.
- `src/sim/entity_store.rs`: `clear_radio_contacts_for` removes reciprocal contacts.
- `src/sim/production/production_sell.rs`: sell cleanup clears contacts directly; it does not broadcast message `0x17` to receivers first.
- `src/sim/combat/mod.rs`: death cleanup clears contacts for removed entities; generic building-death sent-`0x17` receiver effects are not modeled.
- `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, and `src/sim/production/production_spawn.rs`: model selected refinery/factory radio state directly rather than a general synchronous radio dispatcher.

No Rust surface needs to handle a new computed sent-`0x17` producer from this report. The implementation debt remains the literal sent-`0x17` producer set and the receiver side effects they must trigger.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `+0x278` slot-call byte census | verified | pattern `FF ?? 78 02 00 00`; 86 contexts inspected with `get_assembly_context`; all message pushes immediate | none within slot-call pattern |
| `+0x278` suspicious list loop at `0x004492A3` | verified | assembly `0x00449297..0x004492A3`; decompile of enclosing function shows `+0x278(2, target)` | none |
| Known directed literal `0x17` producers | verified comparison | `0x0043CB47`, `0x004425A4`, `0x004485B1` | no reclassification |
| `+0x27C` slot-call census | verified | pattern `FF ?? 7C 02 00 00`; non-helper callers push `0x12`; helper bodies pass caller arg | none within slot-call pattern |
| `RadioClass::Transmit_Radio @ 0x0065AAA0` | verified | decompile plus assembly `0x0065AAB1` | none |
| `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0` | verified | decompile plus assembly `0x0065AD0D` | none |
| `+0x280` slot-call census | verified | pattern `FF ?? 80 02 00 00`; real calls push immediate `0x03` or `0x17` | none within slot-call pattern |
| Known broadcast literal `0x17` producer | verified comparison | `0x0044AB5D..0x0044AB68` | no reclassification |
| `+0x194` direct receive-slot census | verified | pattern `FF ?? 94 01 00 00`; real calls only at `0x0065A9DB`, `0x0065AA49` inside `Transmit_Radio_Impl` | none within slot-call pattern |
| Direct calls to helper addresses not through vtable slots | deferred | out of exact user scope; no direct-helper xref tool pass in this slot | run a separate helper-address xref sweep if desired |
| Full receiver behavior for sent `0x17` | deferred | prior receiver reports cover major bodies; not needed to classify producers | separate receiver parity report if implementation starts |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Are there non-immediate message variables at external +0x278 call sites that can become 0x17? -> No. All real external +0x278 caller contexts push immediate message constants; computed values in those contexts are target pointers or payloads.` (evidence: `FF ?? 78 02 00 00` sweep; `0x004492A3` assembly/decompile spot check)
- `[RESOLVED] OQ-02 - Does the suspicious +0x278 call at 0x004492A3 push a computed message? -> No. It pushes computed target pointer first, then immediate message 0x02.` (evidence: `0x00449297..0x004492A3`)
- `[RESOLVED] OQ-03 - Can +0x27C external callers compute message 0x17? -> Not in this sweep. Non-helper +0x27C callers push immediate 0x12; helper bodies only forward caller arguments.` (evidence: `0x00419428`, `0x0043CA47`, `0x0043CAB8`, `0x0043CBAE`, `0x00737A71`, `0x0065AAB1`, `0x0065ACC9`, `0x0065AD0D`)
- `[RESOLVED] OQ-04 - Can +0x280 broadcast callers compute message 0x17? -> No. Broadcast call sites push immediate constants; only 0x0044AB68 pushes 0x17.` (evidence: `FF ?? 80 02 00 00` sweep; `0x0044AB5D..0x0044AB68`)
- `[RESOLVED] OQ-05 - Are direct +0x194 receive calls independent producers? -> No. The real direct +0x194 calls are inside Transmit_Radio_Impl and forward param_2 from the transmit side.` (evidence: `0x0065A9DB`, `0x0065AA49`; decompile `0x0065A970`)
- `[RESOLVED] OQ-06 - Does RadioClass::Transmit_Radio compute or remap messages? -> No. It passes param_2 through to +0x27C with the global scratch payload.` (evidence: decompile `0x0065AAA0`; assembly `0x0065AAB1`)
- `[RESOLVED] OQ-07 - Does Broadcast_Radio_ToAll compute or remap messages? -> No. It loops contacts and forwards param_2 to +0x27C unchanged.` (evidence: decompile `0x0065ACE0`; assembly `0x0065AD0D`)
- `[RESOLVED] OQ-08 - Do known literal sent-0x17 producers change after this sweep? -> No. They remain the same four producers from the prior report.` (evidence: `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md`; spot contexts `0x0043CB47`, `0x004425A4`, `0x004485B1`, `0x0044AB68`)
- `[RESOLVED] OQ-09 - Is BuildingClass Receive_Radio(0x08) returning 0x17 newly implicated as sent 0x17? -> No. This sweep finds no call shape that aliases that return into a sent message.` (evidence: producer census; `RADIO_0X08_0X17_FACTORY_REPAIR_BUNKER_SENDER_PATHS_RESWARM_20260528.md`)
- `[RESOLVED] OQ-10 - Is there any INI key that turns computed messages into 0x17? -> No such key exists in this radio-slot slice; INI only gates path reachability for some known literal producers.` (evidence: prior radio reports; `rulesmd.ini` comments for `Hospital=` / `Armory=`)
- `[RESOLVED] OQ-11 - What should Rust implement from this report? -> No additional computed producer. Implement literal producer dispatch and receiver side effects only.` (evidence: Rust scan of contact-clearing surfaces; producer census)
- `[DEFERRED] OQ-12 - Do direct calls to helper addresses outside vtable slot patterns ever pass computed 0x17?` (category: `out-of-scope`; reason: user scoped this slot to calls through radio transmit/direct receive slots; next-step-if-pursued: run an address-xref sweep for `0x0065AAA0`, `0x0065A970`, and `0x0065ACE0` in addition to vtable slots)
- `[DEFERRED] OQ-13 - Are every receiver-side 0x17 byte writes modeled in Rust?` (category: `requires-different-system-context`; reason: this report classifies producers only; next-step-if-pursued: receiver-side implementation contract from Foot/Unit/Aircraft reports)

The report is complete for the bounded slot-call census. It is not a claim about every direct helper-address call form outside the requested slot patterns.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| No additional computed sent-radio `0x17` producers were found among `+0x278`, `+0x27C`, `+0x280`, or `+0x194` slot calls. | slot byte sweeps plus assembly context; helper decompiles `0x0065AAA0`, `0x0065A970`, `0x0065ACE0` | no new producer needed | future radio dispatcher / contact cleanup code | Keep producer set limited to verified literal producers unless another report proves a new source. | Static parity test or trace fixture enumerates producer causes and confirms no generic "message variable == 0x17" path is invented. | Do not create a broad computed-message hook that sends `0x17` from arbitrary radio returns or local variables. |
| Helper pass-through can forward `0x17` when the caller is the known literal producer. | `0x0065AAA0`, `0x0065A970`, `0x0065ACE0`; known producer sites | missing generic radio dispatcher remains | `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs` | When implementing literal producers, route through receiver semantics rather than direct contact deletion. | Selling a contacted building broadcasts literal `0x17` to each contact before later break, with receiver state changes visible before contact removal. | Do not treat "no computed producers" as "0x17 receiver behavior can be skipped." |
| `+0x194` direct receives are consumers reached through `Transmit_Radio_Impl`, not independent producers. | `0x0065A9DB`, `0x0065AA49`; decompile `0x0065A970` | receiver side effects still incomplete | future radio receive dispatcher; Foot/Unit/Aircraft receive surfaces | Receiver implementation should depend on actual sent message from transmit producer. | A sent literal `0x17` reaches Unit/Foot/Aircraft receiver once, synchronously, with the same ordering as RadioClass helper dispatch. | Do not call receivers with `0x17` because some unrelated function returned `0x17`. |
| BuildingClass `0x08` returned `0x17` remains a reply code only, not a sent-message producer. | producer census; prior `RADIO_0X08_0X17_FACTORY_REPAIR_BUNKER_SENDER_PATHS_RESWARM_20260528.md` | Rust should keep reply-code handling separate from sent-message dispatch | refinery/factory/repair/bunker radio abstractions | Model `0x08 -> return 0x17` as synchronous caller response where verified, not as a broadcast/direct sent `0x17`. | War-factory final cleanup can branch on returned `0x17` without triggering Unit/Foot sent-`0x17` receiver effects. | Do not alias reply code `0x17` to sent radio `0x17`. |

### Stale Docs / Follow-up Docs

- Replacement wording for the prior deferred uncertainty: "A bounded vtable-slot sweep found no computed non-immediate sent-radio `0x17` producers. The only variable message paths are RadioClass helper pass-throughs from already-classified callers. The known literal producers remain the full verified producer set for this bounded slot surface."
- Keep the prior caveat only for direct helper-address xrefs outside vtable slot patterns, if a later report chooses to inspect them.

## 10. Negative Facts / Do Not Do

- Do not implement any new computed sent-radio `0x17` producer from this sweep.
- Do not turn BuildingClass `Receive_Radio(0x08)` return `0x17` into a sent `0x17`.
- Do not treat direct `+0x194` calls as producers. They are receiver dispatches inside `Transmit_Radio_Impl`.
- Do not assume `Broadcast_Radio_ToAll` computes its own message. It forwards the immediate message supplied by the caller.
- Do not generalize false-positive pattern hits such as `INC/DEC [x+0x194]`, `INC [x+0x27C]`, or no-instruction addresses into radio calls.

## 11. Remaining Uncertainty

- Direct calls to helper addresses outside the vtable-slot pattern were not exhaustively xrefed in this slot. This does not weaken the slot-surface conclusion, but a future binary-wide helper-address xref sweep could close the broader global negative.
- This report does not restate or re-verify every receiver-side byte write for sent `0x17`; use the Foot/Unit/Aircraft receiver reports for implementation.

## Sources

- Ghidra read-only byte searches: `FF ?? 78 02 00 00`, `FF ?? 7C 02 00 00`, `FF ?? 80 02 00 00`, `FF ?? 94 01 00 00`.
- Ghidra assembly contexts for all real slot-call hits, including `0x004492A3`, `0x0065AAB1`, `0x0065ACC9`, `0x0065AD0D`, `0x0065A9DB`, `0x0065AA49`, `0x0043CB47`, `0x004425A4`, `0x004485B1`, `0x0044AB68`.
- Ghidra decompiles: `RadioClass::Transmit_Radio @ 0x0065AAA0`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0`, enclosing helper around `0x004492A3`.
- Prior docs: `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md`, `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`, `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md`, `RADIO_0X08_0X17_FACTORY_REPAIR_BUNKER_SENDER_PATHS_RESWARM_20260528.md`.
- Rust scan: `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/production/production_spawn.rs`.

## Status

COMPLETE for the bounded vtable-slot computed sent-radio `0x17` dataflow sweep. No additional computed sent-`0x17` producers were found.
