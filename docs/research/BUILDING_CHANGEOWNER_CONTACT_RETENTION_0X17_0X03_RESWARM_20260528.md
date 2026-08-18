# BuildingClass ChangeOwner Contact Retention 0x17/0x03 - Re-Swarm Research Report

**Address(es):** `BuildingClass::ChangeOwner @ 0x00448260`; contact pre-pass `0x00448566..0x004486D9`; retained-contact replay `0x00449289..0x004492D5`; radio helpers `0x0065AD30`, `0x0065AAA0`, `0x0065A970`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The `BuildingClass::ChangeOwner` contact-retention/eviction algorithm that sends directed radio `0x17` then `0x03` to contacts that fail retention.  
**Non-Scope:** Full BuildingClass owner-transfer feature/list/UI hooks, full Foot/Unit/Aircraft `0x17` receiver bodies, computed sent-radio `0x17` sweeps outside this function, and all possible non-building `ChangeOwner` paths.  
**Confidence:** High for contact-loop gates, ordering, radio arguments, and Rust-facing mismatch; Medium for the semantic label of `Object+0x14 bit 0x04` because this slice verifies the bit predicate but does not relabel the whole Object flag byte.  
**Active in YR:** Yes. The path is reached by standard YR building owner changes such as engineer capture and occupied civilian building owner reconciliation when the building has radio contacts.

## 0. Working Notes Gate

- Target question: Which BuildingClass contacts survive owner transfer, and exactly when does the `0x17` then `0x03` eviction pair fire?
- Non-goals: Re-prove generic `TechnoClass::ChangeOwner`, decode every building type list hook, or implement Rust.
- Evidence needed to mark COMPLETE: Decompile plus assembly for the contact loop, send arguments, post-base HELLO replay, YR-active caller evidence, Rust scan, and acceptance scenarios.
- Stop conditions: Missing Ghidra access, missed function boundary that blocks the contact loop, or more than one material retention predicate left unresolved.

## 1. Overview

`BuildingClass::ChangeOwner` does not simply clear all radio contacts when a building changes owner. It first walks the building `Contacts[]` array, classifies each non-null contact as retained or evicted, immediately changes retained contacts to the new owner, records them in a temporary list, evicts failed contacts with directed `0x17` then directed `0x03`, then later re-establishes retained contacts with `HELLO(0x02)` after the building's own owner/list work.

The important player-visible consequence is capture behavior for contacted structures: a harvester, produced unit, repair client, bunker occupant, or aircraft contact may either remain linked under the new owner or receive full receiver-side eviction behavior before BREAK. Rust's current direct `owner = engineer_owner` path does neither.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Verified meaning in this slice | Evidence |
|---:|---|---|---|
| `+0xE4` | RadioClass/Techno | `Contacts[]` backing pointer | `0x0065AD30..0x0065AD3D`; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` |
| `+0xE8` | RadioClass/Techno | Contacts capacity and loop bound; sparse slots allowed | `0x00448566..0x00448586`, `0x004486C8..0x004486D9` |
| `+0x14 bit 0x04` | contact object | Required before owner-change retention query; if clear, target is nulled before radio send | `0x004485A2..0x004485A8` |
| vtable `+0x278` | RadioClass | Directed `Transmit_Radio(msg, target)` | `0x004485B1`, `0x004485BE`, `0x004485D0`, `0x004492A3`; helper `0x0065AAA0` |
| vtable `+0x3D4` | contact Techno | Contact object's concrete `ChangeOwner(newOwner, announce=1)` | `0x00448652..0x00448663` |
| `BuildingType+0x16BD` | BuildingTypeClass | `WeaponsFactory=yes`; bypasses the distance gate for retaining contacts | `0x004485DB..0x004485E9`; parser/docs `BUILDINGTYPECLASS_FIELDS.csv`, stock INI |
| `contact+0x418` | Techno | Saved endpoint byte for retained contact; nonzero old value causes building and contact `+0x418` to be set to `1` after HELLO replay | save `0x00448658`; replay `0x004492B0..0x004492C3` |

## 3. Core Logic

### 3.1 Contact enumeration

The contact pre-pass initializes a temporary dynamic-vector-like list, then loops `slot_index = 0..Contacts.Capacity-1`.

Verified mechanics:

1. The loop bound is `this+0xE8`; it is not a live contact count. Evidence: `0x00448566 MOV EAX,[ESI+0xE8]`, `0x00448574 CMP EAX,EDI`, loop increment/compare at `0x004486C8..0x004486D9`.
2. Each slot is read by helper bytes at `0x0065AD30`: `MOV EAX,[ECX+0xE4]`, load stack index, then `MOV EAX,[EAX+ECX*4]`, `RET 4`. Evidence: assembly `0x0065AD30..0x0065AD3D`.
3. Null slots are skipped with no radio traffic. Evidence: after slot read, `TEST EDI,EDI; JZ 0x004486C8` at `0x00448598..0x0044859C`.

### 3.2 Retention predicates

A non-null contact is retained only if all relevant gates pass:

1. `contact+0x14 & 0x04` must be set. If the bit is clear, the code executes `XOR EDI,EDI` and enters the eviction block with a null explicit target. Evidence: `0x004485A2 TEST byte ptr [EDI+0x14],0x4`, `0x004485A6 JNZ retain-query`, `0x004485A8 XOR EDI,EDI`, then send block at `0x004485AA`.
2. The building sends directed `0x13` to that contact and requires return value `1`. Evidence: `PUSH EDI; PUSH 0x13; CALL [vtable+0x278]` at `0x004485C9..0x004485D0`; `CMP EAX,0x1; JNZ 0x004485AA` at `0x004485D6..0x004485D9`.
3. If the building is not a weapons factory (`Type+0x16BD == 0`), the code computes a 3D distance from a building helper coordinate to the contact coordinate and evicts when the distance is `>= 0x40`. Evidence: `Type+0x16BD` test at `0x004485DB..0x004485E9`; building slot `+0xA8` and contact slot `+0x48` coordinate calls at `0x004485EB..0x00448606`; `CoordStruct` set/distance calls at `0x0044861E..0x00448644`; `CMP EAX,0x40; JGE 0x004485AA` at `0x00448649..0x0044864C`.
4. If `Type+0x16BD != 0`, the distance gate is skipped and `0x13 == 1` is sufficient for retention. Evidence: `0x004485E9 JNZ 0x00448652`.

Active in YR: Yes. `WeaponsFactory=yes` is set on stock war factories (`rulesmd.ini` has `WeaponsFactory=yes` for Allied, Soviet, Yuri factories). Non-factory contacted buildings such as repair depots and bunkers use the distance gate.

### 3.3 Eviction path

When a contact fails any gate, the building sends two directed messages to the selected target:

1. `0x17`
2. `0x03` (`BREAK`)

Evidence: `0x004485AA..0x004485BE` is exactly `PUSH target; PUSH 0x17; CALL [vtable+0x278]`, then `PUSH target; PUSH 0x3; CALL [vtable+0x278]`.

Ordering matters: receiver-side `0x17` runs before BREAK contact deletion. This matches the earlier producer sweep, but this report adds the retention predicates that decide whether the pair fires.

Edge detail: if `contact+0x14 & 0x04` is clear, the code deliberately sets the explicit target register to zero before the send. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` falls back to `Contacts[0]` when the explicit target is null. This edge is verified from assembly plus helper decompile, but its standard-YR frequency is unknown; normal live contacts are expected to have the bit set.

### 3.4 Retained contact pre-base owner transfer

For a retained contact, the building changes the contact's owner before changing its own base owner:

1. Save the contact's old `+0x418` byte in `BL`. Evidence: `0x00448658 MOV BL,byte ptr [EDI+0x418]`.
2. Call the contact's concrete `ChangeOwner(newOwner, 1)`. Evidence: `0x00448652 MOV EAX,[ESP+0x64]` loads the new owner; `PUSH 0x1; PUSH EAX; MOV ECX,EDI; CALL [EDX+0x3D4]` at `0x00448652..0x00448663`.
3. Allocate an 8-byte temp record and store `(contact_pointer, old_0x418_byte)`. Evidence: `operator_new(8)` at `0x00448669..0x00448670`, record writes at `0x00448675..0x0044867C`.
4. Append that temp record to the local retained list; then continue the contact slot loop. Evidence: append/reserve branch `0x00448682..0x004486C5`, loop increment at `0x004486C8..0x004486D9`.

This happens before the building's own `TechnoClass::ChangeOwner` call at `0x00448BE8` (covered by the subclass-wrapper report and visible in the same decompile). Therefore retained contacts are owner-transferred early, then radio-linked back later.

### 3.5 Retained contact replay after building owner transfer

After old-owner removals, base owner transfer, new-owner additions, radar update, and other BuildingClass work, the function replays the retained list:

1. If retained count is `<= 0`, skip replay. Evidence: `0x00449289 MOV EAX,[ESP+0x58]`; `TEST EAX,EAX; JLE 0x004492D7`.
2. For each retained record, load the contact pointer and send directed `HELLO(0x02)` from the building to the contact. Evidence: `0x00449293..0x004492A3`, especially `PUSH contact; PUSH 0x2; CALL [vtable+0x278]`.
3. The code does not branch on the HELLO return before restoring the endpoint byte. It reads the saved byte from the temp record. Evidence: after `0x004492A3` call, it reloads the retained record and tests byte `record+4` at `0x004492A9..0x004492B5`.
4. If the saved byte was nonzero, it writes `building+0x418 = 1` and `contact+0x418 = 1`. Evidence: `0x004492B7 MOV byte ptr [ESI+0x418],0x1`; `0x004492C1 MOV ECX,[EAX]`; `0x004492C3 MOV byte ptr [ECX+0x418],0x1`.
5. It loops over retained records in insertion/contact-slot order. Evidence: `INC EDI; CMP EDI,ECX; JL 0x00449297` at `0x004492D2..0x004492D5`.

## 4. INI Keys

| INI key / section | Stock YR value | Binary field / use | Active in standard YR? |
|---|---|---|---|
| `WeaponsFactory=yes` on `[GAWEAP]`, `[NAWEAP]`, `[YAWEAP]` and variants | set | `BuildingType+0x16BD`; skips owner-change contact distance gate once `0x13` returns `1` | Yes |
| `UnitRepair=yes` on service depots/outposts | set | Creates contacted building contexts but is not the retention bypass flag | Yes |
| `Bunker=yes` on `[NATBNK]` | set | Creates contacted building contexts but is not the retention bypass flag | Yes |
| Radio messages `0x13`, `0x17`, `0x03`, `0x02` | no INI key | hardcoded protocol values | Yes |

## 5. Integration Points

| Integration | Verified behavior | Evidence |
|---|---|---|
| Prior sent-radio sweep | Correctly identified the `0x17` then `0x03` producer but deferred the retention algorithm | `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md` |
| `RadioClass::Transmit_Radio` | Wrapper passes `(msg, scratch, target)` to slot `+0x27C`; explicit null target later falls back to first contact in impl | `0x0065AAA0..0x0065AAB7`; `0x0065A970` decompile |
| `RadioClass::Transmit_Radio_Impl` BREAK | `0x03` clears all matching contact slots on the sender before forwarding BREAK to receiver | `0x0065A970` decompile; radio protocol docs |
| Engineer capture | Calls building virtual `+0x3D4`, so this owner-change wrapper is active | `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md`, `0x0052044C..0x00520451` |
| Civilian garrison reconciliation | Calls building virtual `+0x3D4(...,0)`, so this wrapper is also active for occupied/empty civilian building owner flips | `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md`, `0x004582E6..0x00458323` |

## 6. Current Rust Implementation Status

Focused Rust scan:

| Rust surface | Current shape | Delta against this slice |
|---|---|---|
| `src/sim/world/world_orders.rs:177` / `tick_capture_orders` | Engineer capture snapshots orders, then directly writes `b.owner = engineer_owner`, adjusts counts, and despawns the engineer | Missing `BuildingClass::ChangeOwner` contact pre-pass, retained-contact owner transfer, evict `0x17` then BREAK, HELLO replay, and `+0x418` restoration |
| `src/sim/game_entity.rs:187`, `526..539` | `radio_contacts: Vec<u64>` with unique append and retain-remove helpers | Not native sparse `Contacts[]` capacity semantics; no endpoint byte corresponding to `+0x418` |
| `src/sim/entity_store.rs:64..68` | `clear_radio_contacts_for` clears reciprocal links directly | Bypasses receiver-side `0x17` before BREAK and cannot express retain-versus-evict gates |
| `src/sim/production/production_spawn.rs:150..180` | Produced units mark live contact with producer | No general owner-change contact replay or sparse slot order |
| `src/sim/miner/miner_dock_sequence.rs:137..144` | Miner/refinery contact helpers add/remove specific contacts | Specialized, not a generic radio protocol for capture/change-owner |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::ChangeOwner` contact slot loop | verified | `0x00448566..0x004486D9`; decompile `0x00448260` | none for this slice |
| Contact slot read helper | verified | `0x0065AD30..0x0065AD3D` assembly | function boundary is not named; behavior is clear |
| Null contact slot | verified | `0x00448598..0x0044859C` | none |
| `contact+0x14 & 0x04` gate | verified as predicate | `0x004485A2..0x004485A8` | semantic name of whole byte/bit |
| `0x13` reply gate | verified | `0x004485C9..0x004485D9` | detailed receiver effects outside this slice |
| Non-WF distance gate `< 0x40` retained / `>= 0x40` evicted | verified | `0x004485DB..0x0044864C` | exact coordinate helper names outside this slice |
| WeaponsFactory bypass of distance gate | verified | `0x004485E1..0x004485E9`; `BuildingType+0x16BD` docs/INI | none |
| Evict `0x17` then `0x03` order | verified | `0x004485AA..0x004485BE` | receiver body details outside this slice |
| Retained contact `ChangeOwner(newOwner,1)` before building base transfer | verified | `0x00448652..0x00448663`; building base call `0x00448BE8` from wrapper report/decompile | none |
| Retained HELLO replay and `+0x418` mirror restoration | verified | `0x00449289..0x004492D5` | none |
| Rust capture/contact surfaces | touched-not-exhausted | source scan paths in section 6 | future implementation design |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the path active in standard YR? -> Yes; engineer capture and garrison reconciliation call BuildingClass virtual +0x3D4.` (evidence: `0x0052044C..0x00520451`, `0x004582E6..0x00458323`; wrapper report)
- `[RESOLVED] OQ-02 - What is the contact iteration bound? -> `this+0xE8` Contacts capacity, not live count.` (evidence: `0x00448566..0x004486D9`)
- `[RESOLVED] OQ-03 - Are null contact slots evicted? -> No, null slots skip to next slot.` (evidence: `0x00448598..0x0044859C`)
- `[RESOLVED] OQ-04 - What non-null object bit gates retention? -> `contact+0x14 & 0x04`; bit clear nulls the explicit send target before eviction path.` (evidence: `0x004485A2..0x004485A8`)
- `[RESOLVED] OQ-05 - Which radio query must pass? -> Directed `0x13` must return `1`.` (evidence: `0x004485C9..0x004485D9`)
- `[RESOLVED] OQ-06 - Is distance checked for all building types? -> No; `Type+0x16BD WeaponsFactory` skips distance.` (evidence: `0x004485DB..0x004485E9`)
- `[RESOLVED] OQ-07 - What is the non-factory distance threshold? -> Retain only when computed 3D distance is `< 0x40`; evict on `>= 0x40`.` (evidence: `0x00448627..0x0044864C`)
- `[RESOLVED] OQ-08 - What exact eviction radio order fires? -> `0x17` first, then `0x03`, both through directed vtable `+0x278`.` (evidence: `0x004485AA..0x004485BE`)
- `[RESOLVED] OQ-09 - Does retained contact owner transfer happen before or after building owner transfer? -> Before; retained contact virtual `+0x3D4(newOwner,1)` runs in the pre-pass, before building base transfer at `0x00448BE8`.` (evidence: `0x00448652..0x00448663`; decompile/wrapper report)
- `[RESOLVED] OQ-10 - How are retained contacts linked after transfer? -> After building owner/list/radar work, the building sends `HELLO(0x02)` to each retained contact in temp-list order.` (evidence: `0x00449289..0x004492D5`)
- `[RESOLVED] OQ-11 - What happens to `+0x418` for retained contacts? -> Old contact byte is saved; if nonzero, replay sets both building and contact `+0x418` to `1`.` (evidence: save `0x00448658`; replay `0x004492B0..0x004492C3`)
- `[RESOLVED] OQ-12 - Does current Rust direct capture match? -> No; it directly writes owner and count deltas with no radio/contact protocol.` (evidence: `src/sim/world/world_orders.rs:177`, `233..244`)
- `[DEFERRED] OQ-13 - What is the canonical semantic name for `Object+0x14 bit 0x04` across all ObjectClass states?` (category: requires-different-system-context; reason: this slice proves the predicate but not the full flag byte taxonomy; next-step-if-pursued: targeted ObjectClass flag-map audit)
- `[DEFERRED] OQ-14 - What player-visible edge occurs if a non-null contact has `+0x14 & 4 == 0` and explicit target becomes null?` (category: needs-runtime-debugger; reason: binary fallback to Contacts[0] is verified, but standard live frequency needs runtime state capture; next-step-if-pursued: breakpoint owner-change with stale/limbo contact)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Building owner change classifies each sparse contact slot before the building base owner transfer; retained contacts get their own `ChangeOwner(newOwner,1)` first. | `0x00448566..0x004486D9`; retained call `0x00448652..0x00448663`; building base call `0x00448BE8` | Missing; Rust capture directly writes `b.owner` | `src/sim/world/world_orders.rs`, future building-owner-transfer/radio helper | Add an owner-transfer path that runs contact retention before direct owner mutation and preserves slot order. | Engineer captures a contacted service depot/war factory; retained eligible contacts change owner before the building's own owner/list side effects. | `building_changeowner_retains_contact_before_base_owner_transfer` | Do not model capture as only `owner = new_owner` plus count rebuild. |
| Failed retention sends directed `0x17` then directed `0x03` to the failed contact; `0x17` receiver side effects must happen before BREAK deletion. | `0x004485AA..0x004485BE`; radio helper `0x0065AAA0`; BREAK impl `0x0065A970` | Missing; Rust clears links directly via `clear_radio_contacts_for` | `src/sim/entity_store.rs`, `src/sim/game_entity.rs`, owner/capture surfaces | Implement explicit sent-radio delivery for owner-change eviction, then BREAK/contact removal. | Capture a building with an out-of-range repair client; the client runs `0x17` path/mission cleanup before the contact is broken. | `building_changeowner_evicts_failed_contact_with_0x17_before_break` | Do not replace the pair with contact deletion only. |
| Retention requires `0x13 == ROGER` and, for non-WeaponsFactory buildings, distance `< 0x40`; `WeaponsFactory=yes` bypasses the distance gate. | `0x004485C9..0x0044864C`; `BuildingType+0x16BD` docs/INI | Missing; Rust has no owner-change retention gate | contact/radio helper plus rules type flag access | Gate retained contacts by the same reply and distance predicates, not by broad same-owner/alliance checks. | Capture contacted repair depot with one close client and one far client; close eligible client is retained, far client receives `0x17` then BREAK. | `building_changeowner_non_factory_retention_uses_0x40_distance_gate` | Do not apply war-factory retention behavior to repair depots or bunkers. |
| Retained contacts are re-linked after building owner/list work by sending `HELLO(0x02)` and then restoring mirrored `+0x418` to `1` only when the saved old contact byte was nonzero. | `0x00449289..0x004492D5`; save `0x00448658` | Missing; Rust contact storage has no endpoint byte | `src/sim/game_entity.rs`, future radio endpoint state | Preserve enough endpoint state to replay HELLO and mirror the native `+0x418` write condition. | Capture a contacted war factory while the produced unit has endpoint byte set; after transfer both endpoints remain marked after HELLO. | `building_changeowner_retained_contact_rehello_restores_endpoint_byte` | Do not blindly set endpoint state for every retained contact; native tests saved byte first. |

## 10. Negative Facts / Do Not Do

- Do not clear all contacts on building capture. Native retains eligible contacts, changes their owner, and re-HELLOs them. Evidence: `0x00448652..0x00448663`, `0x0044929F..0x004492A3`.
- Do not evict with BREAK only. Native sends `0x17` before `0x03`, and receiver `0x17` can mutate path, mission, destination, aircraft routing, and visual latch state. Evidence: `0x004485AA..0x004485BE`; receiver reports.
- Do not use a live-count or compact Vec iteration as the native contact loop. Native loops capacity `+0xE8` and reads sparse slots from `+0xE4`. Evidence: `0x00448566..0x004486D9`, `0x0065AD30..0x0065AD3D`.
- Do not apply the `<0x40` distance gate to weapons factories. `Type+0x16BD` jumps directly to retained-contact owner transfer after `0x13 == 1`. Evidence: `0x004485E1..0x004485E9`.
- Do not assume `HELLO(0x02)` replay return controls `+0x418` restoration in this function. The code tests the saved byte from the temp record after the call. Evidence: `0x004492A3..0x004492C3`.
- Do not silently ignore the `contact+0x14 & 0x04` edge. The binary nulls the explicit target and then uses normal radio fallback behavior. Evidence: `0x004485A2..0x004485B1`; `0x0065A970` fallback.

## 11. Remaining Uncertainty

- The full semantic name and all producers/consumers of `Object+0x14 bit 0x04` remain out of scope. This report proves its role in this contact loop only.
- The exact runtime frequency of the null-explicit-target fallback edge for stale/limbo contacts needs a debugger trace. The code path is real; normal live contacts likely take the bit-set path.
- The exact coordinate helper names for building vtable `+0xA8` in this context are not renamed here; the distance threshold and branch are verified.

## 12. Stale Docs / Follow-up Docs

- `docs/research/SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md` section "Remaining Uncertainty" can replace "Building owner-change retention field labels are sufficient to classify the `0x17` send but not sufficient to implement the whole retention algorithm without a dedicated owner-change report" with: "Building owner-change retention is now decoded in `BUILDING_CHANGEOWNER_CONTACT_RETENTION_0X17_0X03_RESWARM_20260528.md`: contacts are retained only after non-null slot, `Object+0x14 & 4`, `0x13 == 1`, and either `WeaponsFactory=yes` or distance `<0x40`; retained contacts change owner before building base owner transfer and are re-HELLOed after building owner/list work."
- `docs/research/CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md` can add a cross-reference that BuildingClass contact retention is no longer deferred for the radio send/retention slice.

## Sources

- Ghidra decompile: `BuildingClass::ChangeOwner @ 0x00448260`.
- Ghidra assembly contexts: `0x00448566..0x004486D9`, `0x004485AA..0x004485BE`, `0x00448652..0x00448663`, `0x00449289..0x004492D5`, `0x0065AD30..0x0065AD3D`, `0x0065AAA0..0x0065AAB7`, `0x0065A970`.
- Prior docs: `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md`, `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `BUILDINGTYPECLASS_FIELDS.csv`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini` for `WeaponsFactory=yes`, `UnitRepair=yes`, `Bunker=yes`.
- Rust scan: `src/sim/world/world_orders.rs`, `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_spawn.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/production/production_sell.rs`.

## Status

COMPLETE for `BUILDING_CHANGEOWNER_CONTACT_RETENTION_0X17_0X03`: the retention gates, eviction radio pair, retained-contact owner-transfer order, post-base HELLO replay, and Rust-facing handoffs are verified. Remaining uncertainty is limited to the global semantic label/frequency of the `Object+0x14 bit 0x04` stale-contact edge.
