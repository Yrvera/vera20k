# Sent Radio `0x17` Caller Sweep - Re-Swarm Research Report

**Address(es):** `0x0043C2D0`, `0x00442230`, `0x004485AD`, `0x00449C30`; receiver context `0x004D8FB0`, `0x00737430`, `0x004190B0`; radio core `0x0065AAA0`, `0x0065ACE0`, `0x0065A970`  
**Investigation Mode:** exhaustive-slice for immediate sent-message `0x17` producer sites found by a bounded literal-radio sweep  
**Claimed Scope:** Callers that transmit radio message `0x17` through radio transmit slots `+0x278` / `+0x280`, plus direct-receive-slot sanity check, distinct from BuildingClass radio `0x08` returning reply code `0x17`.  
**Non-Scope:** Re-decoding Foot/Unit/Aircraft `0x17` receiver bodies, all non-immediate computed message sends, and full owner-change/damage/sell state machines beyond the `0x17` send conditions.  
**Confidence:** High for the four immediate producer sites and stock/legacy activity classification; Medium for the negative claim against non-immediate computed `0x17` sends because this pass was bounded to literal/immediate call shapes.  
**Active in YR:** Yes for sell, building death, and building owner-change paths; conditional/inactive in stock YR for the Hospital/Armory admission cleanup path.

## 1. Overview

The binary has real sent-message `0x17` producers, but they are not the same thing as the `0x17` value returned from `BuildingClass::Receive_Radio(0x08)`. The bounded producer sweep found four real radio send sites: `BuildingClass::Receive_Radio(0x0E)` legacy Hospital/Armory cleanup, `BuildingClass::ReceiveDamage` death cleanup, `BuildingClass::ChangeOwner` contact eviction, and `BuildingClass::Sell` state-0 broadcast.

No stock YR producer was found that turns a BuildingClass `0x08` queued reply into a sent `0x17` message. Rust therefore needs a sent-`0x17` mechanism only for contacted object eviction/cleanup paths, not as a reply-code alias.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence |
|---:|---|---|---|
| `+0x194` | Radio vtable | `Receive_Radio` dispatch slot | `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`; `0x0065A970` dispatch |
| `+0x278` | Radio vtable | `Transmit_Radio(msg, target)`, wrapper around `+0x27C` with global scratch payload | `0x0065AAA0`; call sites below |
| `+0x280` | Radio vtable | `Broadcast_Radio_ToAll(msg)` over all non-null contact slots | `0x0065ACE0`; sell site `0x0044AB5D..0x0044AB68` |
| `+0xE4` | RadioClass | `Contacts.data` | `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md` |
| `+0xE8` | RadioClass | `Contacts.Capacity`, loop bound, not active count | `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md` |
| `+0x16C1` | BuildingTypeClass | `Hospital=` legacy walk-in flag | `SERVICE_REPAIR_RADIO_0X1C_0X22_PATH_GHIDRA_REPORT.md`; stock `rulesmd.ini` comments key out |
| `+0x16C2` | BuildingTypeClass | `Armory=` legacy walk-in flag | same |
| `+0x16CB` | BuildingTypeClass | `Helipad=`; building death path uses it to avoid sent `0x17` and apply close damage path | `0x00442593..0x004425A4` |
| `+0x6D1` | UnitClass | unload/deploy visual latch cleared by Unit radio `0x17` when garrison/weeder/deploy gate passes | `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`; `0x00737A98..0x00737AF6` |

## 3. Core Logic

### Bounded literal-radio sweep

The sweep used Ghidra byte search for immediate `PUSH 0x17` (`6A 17`) and then filtered instruction contexts for radio transmit-slot calls. Relevant radio producer contexts:

| Site | Function | Radio shape | Receiver target | Activity |
|---|---|---|---|---|
| `0x0043CB43..0x0043CB47` | `BuildingClass::Receive_Radio(0x0E)` Hospital/Armory cleanup | `PUSH target; PUSH 0x17; CALL [vtable+0x278]` | one existing contact that answered `0x22` with `10` | Conditional; legacy Hospital/Armory flags inactive in stock YR |
| `0x0044259D..0x004425AA` | `BuildingClass::ReceiveDamage`, death result | `PUSH target; PUSH 0x17; CALL [vtable+0x278]` | collected destination/contact units outside the close-damage case | Yes, when a contacted building is destroyed |
| `0x004485AA..0x004485BE` | `BuildingClass::ChangeOwner` | `PUSH target; PUSH 0x17; CALL [vtable+0x278]`, then `PUSH target; PUSH 3; CALL [vtable+0x278]` | contact that fails transfer/retention tests | Yes, when ownership changes while contacts exist |
| `0x0044AB5A..0x0044AB68` | `BuildingClass::Sell`, state 0 | `PUSH 0x17; CALL [vtable+0x280]` | every non-null contact slot | Yes, on normal sell start for non-upgrade sell |

Other `PUSH 0x17` hits in this sweep were not sent radio `0x17`: non-radio UI/math/data/setup calls, virtual calls at unrelated offsets, or data bytes. Direct receive-slot `+0x194` contexts in this pass were generic `Transmit_Radio_Impl` dispatches carrying variable `msg`, not separate literal direct sends of `0x17`.

### Producer 1: Hospital/Armory cleanup sender

Inside `BuildingClass::Receive_Radio(0x0E)`, the Hospital/Armory branch enters only after the non-Hospital/non-Armory branch is skipped. It first calls `FUN_0065ADF0`; if no free/matching contact slot exists, it loops existing contact slots. For each contact, it sends `0x22` through `+0x278`; if the response is `10`, it sends `0x17` to that same target.

Evidence: decompile `BuildingClass__Receive_Radio`; assembly `0x0043CB18..0x0043CB47`.

Active in YR: Conditional. The binary path is live if a building type sets `Hospital=yes` or `Armory=yes`. Stock YR `rulesmd.ini` comments out those keys for the old tech-hospital/armory path (`rulesmd.ini:13992`, `14016`, `14040`), so standard YR skirmish does not normally use this producer.

### Producer 2: Building death cleanup sender

`BuildingClass::ReceiveDamage` death result case builds a temporary list of contacted/destination objects before teardown. The linked `+0x2E4` unit is removed from the local list before `UndockUnit`. Later, for each remaining listed unit, it compares distance to the building; if the unit is close (`distance < 0x100`) or the destroyed building is a helipad (`Type+0x16CB != 0`), it applies damage through the victim's receive-damage virtual. Otherwise it sends directed radio `0x17` to that target and clears target field `target+0x500 = 0`.

Evidence: decompile `BuildingClass__ReceiveDamage`; assembly `0x00442586..0x004425AA`, especially `0x0044259D MOV vtable`, `0x004425A0 PUSH 0x17`, `0x004425A4 CALL [vtable+0x278]`, and `0x004425AA MOV [target+0x500],0`.

Active in YR: Yes, conditional on a building dying while it has contacted/destination units. This is stock-relevant for refineries, service depots, bunkers, and other contacted building interactions.

### Producer 3: Building owner-change sender

`BuildingClass::ChangeOwner` iterates existing contact slots during owner transfer. For a contact, it first sends `0x13` and uses ownership/distance/type gates to decide whether it can preserve/rebuild the contact under the new owner. If the target does not pass the retention path, the building sends radio `0x17` to that target, then immediately sends radio `0x03` (`BREAK`) to the same target.

Evidence: decompile `BuildingClass__ChangeOwner`; assembly `0x004485AA..0x004485BE`. The exact radio pair is `PUSH target; PUSH 0x17; CALL [vtable+0x278]` followed by `PUSH target; PUSH 3; CALL [vtable+0x278]`.

Active in YR: Yes when buildings are captured, mind-controlled/special-transferred, or otherwise change owner with active contacts. Standard YR has engineer capture and trigger/map ownership changes, so the path is not TS-only.

### Producer 4: Building sell broadcast

`BuildingClass::Sell` state 0 clears the sell animation-complete byte `+0x6DD`, then broadcasts radio `0x17` to every non-null contact slot through `Broadcast_Radio_ToAll`. In the next state it broadcasts `0x03` to break remaining radio links.

Evidence: decompile `BuildingClass__Sell`; assembly `0x0044AB5A..0x0044AB68` for broadcast `0x17`; sibling `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` corroborates the state-0 order. `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0` walks `Contacts[0..Capacity)` and calls `+0x27C` for each non-null entry.

Active in YR: Yes on normal building sell start. Upgrade-only sell early-return skips this broadcast; that path removes the upgrade and queues Guard instead.

## 4. INI Keys

Radio message `0x17` itself has no INI key. INI only gates which building systems can have contact lists or reach legacy branches.

| INI key / section | Stock YR value | Binary field / use | Active in standard YR? |
|---|---|---|---|
| `[GAWEAP]/[NAWEAP]/[YAWEAP] WeaponsFactory=yes` | set | creates contact-driven war-factory contexts; not itself a sent-`0x17` producer | Yes |
| `[GADEPT]/[NADEPT]/[YADEPT]/[CAOUTP] UnitRepair=yes` | set | service-depot contacts; destroyed/change-owner/sell can send `0x17` to contacts | Yes |
| `[NATBNK] Bunker=yes` | set | bunker contact/occupant contexts | Yes |
| `Hospital=` | commented out in `rulesmd.ini` | `BuildingType+0x16C1`; gates legacy Hospital cleanup producer | No for stock YR; conditional for mods/maps |
| `Armory=` | commented out in `rulesmd.ini` | `BuildingType+0x16C2`; gates legacy Armory cleanup producer | No for stock YR; conditional for mods/maps |
| `Helipad=yes` | set on helipads/airfields | `BuildingType+0x16CB`; building death path uses close-damage branch instead of directed `0x17` | Yes |

## 5. Integration Points

- `RadioClass::Transmit_Radio @ 0x0065AAA0`: wrapper used by the three directed producer sites. It passes the global scratch buffer to `Transmit_Radio_Impl`.
- `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0`: used by sell state 0. It visits all non-null contact slots; it does not use a live-count field.
- `FootClass::Receive_Radio @ 0x004D8FB0`: main sent-`0x17` receiver for Foot-derived units, including infantry by inheritance.
- `UnitClass::Receive_Radio @ 0x00737430`: vehicle override has a conditional pre-body for message `0x17`, then falls through to Foot.
- `AircraftClass::Receive_Radio @ 0x004190B0`: aircraft-specific `0x17` reroutes to nearest friendly airfield and sends `0x03` to first contact.
- `BuildingClass::Receive_Radio(0x08) @ 0x0043C2D0`: can return value `0x17`; this report found no path that treats that return as a sent `0x17` producer.

## 6. Current Rust Implementation Status

Rust has contact storage but no generic radio message dispatcher:

- `src/sim/game_entity.rs`: `radio_contacts: Vec<u64>` and helper methods add/contains/remove contacts.
- `src/sim/entity_store.rs`: `clear_radio_contacts_for` removes reciprocal contacts, preserving remaining order.
- `src/sim/production/production_sell.rs`: `sell_building` clears contacts and calls refinery interruption helpers; it does not model sent `0x17` receiver semantics for every contacted Foot/Aircraft.
- `src/sim/miner/miner_dock_sequence.rs` and `src/sim/miner/miner_system.rs`: direct refinery-loss and dock-state cleanup approximate the most visible stock miner effects.
- `src/sim/production/production_spawn.rs`: war factory spawn contact helpers create contact state but not a general radio queue/eviction protocol.

Current Rust can clear links, but it does not have a reusable "send radio `0x17` to each/contacted target and run Foot/Unit/Aircraft receiver side effects" surface.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `PUSH 0x17` literal sweep | verified | Ghidra `search_byte_patterns "6a 17"` plus assembly-context filter | non-immediate computed message values |
| Directed sent `0x17` via `+0x278` | verified | `0x0043CB43`, `0x004425A0`, `0x004485AD` | none for immediate sites |
| Broadcast sent `0x17` via `+0x280` | verified | `0x0044AB5D..0x0044AB68` | none |
| Direct receive-slot literal `0x17` | verified negative for this sweep | direct `+0x194` call contexts found at `0x0065A9DB`, `0x0065AA49` are generic transmit dispatch, not literal producer sites | computed/direct unusual callers outside immediate pattern |
| Building `0x08` reply-code distinction | verified | `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE_GHIDRA_REPORT.md`; `BuildingClass__Receive_Radio` case `0x08` | none |
| Hospital/Armory sent `0x17` cleanup | verified | `0x0043CB18..0x0043CB47`; `SERVICE_REPAIR_RADIO_0X1C_0X22_PATH_GHIDRA_REPORT.md` | stock-inactive unless maps/mods set flags |
| Building death sent `0x17` cleanup | verified | `0x00442586..0x004425AA`; refinery-loss report corroborates | exact close-damage effects are a different slice |
| Building owner-change sent `0x17` cleanup | verified | `0x004485AA..0x004485BE` | exact contact retention field meanings outside radio send are not fully labeled |
| Building sell sent `0x17` broadcast | verified | `0x0044AB5A..0x0044AB68`; `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` | none for radio order |
| Rust generic sent-`0x17` producer surface | touched-not-exhausted | source scan paths in section 6 | future implementation design |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What distinguishes this target from BuildingClass 0x08 returning 0x17? -> Sent 0x17 requires an actual radio transmit call; BuildingClass 0x08 returning 0x17 is only a synchronous return value.` (evidence: `0x0043C2D0`; `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 - Which immediate literal sites are real sent-radio producers? -> Four: `0x0043CB43`, `0x004425A0`, `0x004485AD`, `0x0044AB5D`.` (evidence: Ghidra byte sweep and assembly contexts)
- `[RESOLVED] OQ-03 - Does any immediate direct receive-slot caller send literal 0x17 outside Transmit_Radio_Impl? -> No such producer surfaced in this bounded sweep; direct +0x194 calls are generic dispatch inside Transmit_Radio_Impl.` (evidence: `0x0065A9DB`, `0x0065AA49`)
- `[RESOLVED] OQ-04 - Is Hospital/Armory 0x22 -> 0x17 active in stock YR? -> No for stock rulesmd; keys are parsed but commented out.` (evidence: `rulesmd.ini:13992`, `14016`, `14040`; `0x0043CB18..0x0043CB47`)
- `[RESOLVED] OQ-05 - Is building sell broadcast 0x17 active in stock YR? -> Yes for normal sell state 0, after linked-undock and before state 1 BREAK broadcast.` (evidence: `0x0044AB5A..0x0044AB68`)
- `[RESOLVED] OQ-06 - Is building death sent 0x17 active in stock YR? -> Yes when a contacted building dies and the target is outside the close-damage / helipad path.` (evidence: `0x00442586..0x004425AA`)
- `[RESOLVED] OQ-07 - Is owner-change sent 0x17 active in stock YR? -> Yes for ownership transfer/capture paths with contacts that cannot be retained; it is followed by BREAK.` (evidence: `0x004485AA..0x004485BE`)
- `[RESOLVED] OQ-08 - Does sell use directed `+0x278` or broadcast `+0x280`? -> Broadcast `+0x280`, so it visits all non-null contacts through `Broadcast_Radio_ToAll`.` (evidence: `0x0044AB68`; `0x0065ACE0`)
- `[RESOLVED] OQ-09 - Do sent-0x17 producers require Rust support? -> Yes for contact cleanup/eviction effects, but not for BuildingClass 0x08 queued reply semantics.` (evidence: producer table; Rust scan section 6)
- `[RESOLVED] OQ-10 - What receiver side should a future Rust sender trigger? -> Foot/Unit/Aircraft `Receive_Radio(0x17)` effects, not BuildingClass 0x08 reply handling.` (evidence: `0x004D8FB0`, `0x00737430`, `0x004190B0`)
- `[DEFERRED] OQ-11 - Are there computed non-immediate msg variables that can become 0x17 and call transmit slots?` (category: `bounded-cost-too-high`; reason: this slot was scoped to literal/message sender sweep and immediate radio shapes; next-step-if-pursued: binary-wide dataflow over all `+0x278/+0x27C/+0x280` call sites)
- `[DEFERRED] OQ-12 - Exact player-visible owner-change contact retention scenarios for every building type.` (category: `out-of-scope`; reason: owner-change state machine is broader than sent-message producer classification; next-step-if-pursued: targeted `BuildingClass::ChangeOwner` contact-retention investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Normal building sell state 0 broadcasts sent radio `0x17` to every non-null contact before the later `0x03` BREAK broadcast. | `0x0044AB5A..0x0044AB68`; `0x0065ACE0` | Missing generic receiver effects; Rust mostly clears links directly | `src/sim/production/production_sell.rs`, `src/sim/game_entity.rs`, future radio helper | Run sent-`0x17` receiver cleanup for contacted Foot/Unit/Aircraft before clearing/breaking contacts. | Sell a refinery/repair depot/bunker with contacted mover; mover path/mission/display state matches receiver `0x17` behavior before link break. | `building_sell_broadcasts_radio_0x17_before_break` | Do not replace this with only `clear_radio_contacts_for`; link removal is not the receiver side effect. |
| Building death sends directed `0x17` to listed contacts outside the close-damage/helipad path and clears target `+0x500`. | `0x00442586..0x004425AA` | Partial direct cleanup for refinery loss; no generic contacted-building death producer | `src/sim/combat/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, future radio helper | On contacted building death, deliver sent `0x17` to eligible contacts rather than only deleting contacts. | Destroy a contacted service depot/refinery with a mover outside close range; mover receives path/mission cleanup and is not promoted into removed building. | `building_death_sends_radio_0x17_to_far_contacts` | Do not use the helipad/close-damage path for all contacts; binary sends `0x17` only in the far/non-helipad branch. |
| Building owner-change sends `0x17` then `0x03` to contacts that cannot be retained under the new owner. | `0x004485AA..0x004485BE` | Unchecked/missing generic owner-change contact protocol | building ownership/capture surfaces; likely `src/sim/production` / `src/sim/world` ownership code | Owner transfer must evict stale contacts through receiver `0x17` before BREAK when retention fails. | Capture a service-contacted building and verify the old contacted unit drops its path/mission relationship before contact removal. | `building_change_owner_evicts_unretained_contacts_with_radio_0x17_then_break` | Do not only rewrite owner fields and leave old contacts or old destinations alive. |
| Hospital/Armory cleanup can send `0x17` after a `0x22` response of `10`, but stock YR does not enable these flags. | `0x0043CB18..0x0043CB47`; `rulesmd.ini:13992`, `14016`, `14040` | Acceptable to deprioritize for stock; modded parity still missing | rules object flags, future radio service building logic | Treat as legacy conditional behavior, not stock tech-hospital behavior. | Modded `Hospital=yes` with full contacts evicts a healthy contact through `0x22 -> 0x17`. | `legacy_hospital_armory_healthy_contact_gets_radio_0x17` | Do not implement stock CATHOSP as this legacy walk-in radio path. |

## 10. Negative Facts / Do Not Do

- Do not convert BuildingClass radio `0x08` reply code `0x17` into a sent radio `0x17` event. Evidence: BuildingClass case `0x08` returns `0x17`; producer sweep found separate transmit sites only.
- Do not add a sent-`0x17` producer to stock refinery queue admission. Existing stock refinery sender sweeps found `0x08` cleanup and `0x0E/0x15/0x16` dock handoff, not queue admission through sent `0x17`.
- Do not model sent `0x17` as only contact deletion. Receiver bodies mutate path, destination, mission, unload/deploy latch, aircraft destination, and possibly locomotor state.
- Do not make Hospital/Armory legacy cleanup a stock YR tech-hospital rule; stock `rulesmd.ini` comments out `Hospital=` and `Armory=`.
- Do not treat sell and owner-change as the same radio order: sell broadcasts `0x17` to all contacts, while owner-change sends directed `0x17` then directed `0x03` only for contacts that fail retention.

## 11. Remaining Uncertainty

- Computed non-immediate message values were not exhaustively dataflowed. This report claims the immediate/literal radio producer set, not every possible computed transmit argument in the binary.
- Building owner-change retention field labels are sufficient to classify the `0x17` send but not sufficient to implement the whole retention algorithm without a dedicated owner-change report.
- Close-damage side effects in `BuildingClass::ReceiveDamage` for contacts within `0x100` leptons or helipad contacts are outside this report.

## Sources

- Ghidra read-only byte sweep: `search_byte_patterns "6a 17"`.
- Ghidra assembly contexts: `0x0043CB18..0x0043CB47`, `0x00442586..0x004425AA`, `0x004485AA..0x004485BE`, `0x0044AB5A..0x0044AB68`, `0x0065A9DB`, `0x0065AA49`.
- Ghidra decompiles: `BuildingClass__Receive_Radio`, `BuildingClass__ReceiveDamage`, `BuildingClass__ChangeOwner`, `BuildingClass__Sell`, `FootClass__Receive_Radio`, `UnitClass__Receive_Radio`, `AircraftClass__Receive_Radio`, `RadioClass__Transmit_Radio`, `RadioClass__Broadcast_Radio_ToAll`, `RadioClass__Transmit_Radio_Impl`.
- Docs referenced: `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md`, `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `RADIO_0X08_TO_0X17_FACTORY_REPAIR_BUNKER_CLEARANCE_GHIDRA_REPORT.md`, `SERVICE_REPAIR_RADIO_0X1C_0X22_PATH_GHIDRA_REPORT.md`, `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md`, `miner/REFINERY_DESTROYED_OR_SOLD_MID_UNLOAD_CONTACTS_DISPLAY_CREDITS_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/production/production_sell.rs`, `src/sim/production/production_spawn.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/combat/mod.rs`.

## Status

COMPLETE for the bounded immediate sent-radio `0x17` caller sweep. PARTIAL only for broader computed-message dataflow, which is explicitly out of scope here.
