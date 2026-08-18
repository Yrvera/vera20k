# Broadcast Radio To All Limbo Break Cleanup - Ghidra Research Report

**Address(es):** `0x0065AA80` (`TechnoClass__Limbo_Tail_CallConceal`), `0x0065ACE0` (`RadioClass__Broadcast_Radio_ToAll`), `0x0065A970` (`RadioClass__Transmit_Radio_Impl`), `0x0065A820` (`RadioClass__Receive_Radio`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** callers and semantics of `Broadcast_Radio_ToAll(3)` on Techno limbo/death/despawn cleanup, plus Rust-facing cleanup consequences for radio contacts, dock links, passengers/transports, carryall links, service links, mind-control links, and building contact arrays.
**Non-Scope:** full radio message catalog, full Transmit/Receive internals outside BREAK call order, full CargoClass unload ordering, full CaptureManager implementation, runtime validation in a live match.
**Confidence:** High for the broadcast caller/ordering and Rust cleanup gap; Medium for player-visible consequences that depend on adjacent systems already covered by prior reports.
**Active in YR:** Yes for normal Techno limbo/destruction. Conditional for carryall pickup because stock `[HIND] Carryall=yes` is unbuildable (`TechLevel=-1`). Mind-control cleanup is active in YR but not owned by this radio broadcast hook.

## 0. Investigation Gate

**Target question:** When a Techno enters limbo, dies, or is otherwise despawned, which binary path broadcasts `BREAK(3)` to all radio contacts, what exactly is the call order, and what cleanup semantics must Rust preserve?

**Non-goals:** Do not re-document the full radio protocol; do not re-investigate every transport/cargo/mind-control state machine; do not implement Rust changes; do not edit in-repo docs.

**Evidence needed to mark COMPLETE:**

- Decompile and assembly for `TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80`.
- Caller/xref evidence for `0x0065AA80`, including the suspicious `0x007F05DC` data xref.
- Decompile and assembly for `RadioClass__Broadcast_Radio_ToAll @ 0x0065ACE0`.
- Decompile and assembly for `RadioClass__Transmit_Radio_Impl @ 0x0065A970` and `RadioClass__Receive_Radio @ 0x0065A820` sufficient to prove BREAK ordering.
- Immediate caller evidence for normal Foot/Building limbo paths.
- Rust scan of current radio-contact/despawn/dock/passenger surfaces.

**Stop conditions:**

- Stop at call order and cleanup semantics after proving the broadcast caller set.
- Defer exact CargoClass unload and CaptureManager internals to existing reports unless the broadcast path directly calls them.
- Do not follow unrelated `Detach_From_All_Lists` listener arrays beyond identifying that they are separate from radio cleanup.

## 1. Overview

`TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80` is the live pre-conceal cleanup hook that sends `BREAK(3)` to every non-null `RadioClass::Contacts[]` entry before `ObjectClass__Conceal` flips `InLimbo`. The active call chain is:

`FootClass__Limbo` or `BuildingClass__Limbo` -> `TechnoClass__Limbo_Helper @ 0x006F6AC0` -> `TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80` -> virtual slot `+0x280` with arg `3` -> `RadioClass__Broadcast_Radio_ToAll @ 0x0065ACE0`.

The suspicious `0x007F05DC` reference is not a second caller. It is a data pointer inside `vtable__RadioClass`: `RadioClass__Constructor @ 0x0065A750` writes vtable base `0x007F0508`, and memory at base `+0xD4 = 0x007F05DC` is the pointer `0x0065AA80`. That is a virtual Limbo-slot registration for the base RadioClass vtable.

## 2. Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `ObjectClass` | `+0x81` | `InLimbo` byte checked before broadcast and then set by Conceal | `0x0065AA83..0x0065AA8B`, `0x005F4E9E..0x005F4EA5` | Yes |
| `RadioClass` | `+0xE4` | sparse contacts pointer array | `0x0065ACE6..0x0065ACFB`, `0x0065A9A8..0x0065A9B8` | Yes |
| `RadioClass` | `+0xE8` | contacts capacity loop bound | `0x0065ACE6..0x0065ACEE`, `0x0065A99C..0x0065A9C7` | Yes |
| `TechnoClass` | `+0x418` | dock/contact-entered flag, cleared by radio `0x19`; BREAK may trigger `0x19` first if both sides have it | `0x006F4C50..0x006F4C7A` | Conditional, live when link flag is set |
| `TechnoClass` | `+0x419` | second dock lock toggled by radio `0x1A/0x1B` | `0x006F4BC1..0x006F4C15` | Conditional |
| Cargo/Techno | `+0x114/+0x118` | cargo head/count area used by passenger chains, not RadioClass contacts | `CargoClass__AddPassenger @ 0x004733A0`; `FootClass__EMPPassengers @ 0x00707CB0` | Yes |
| Mind control | `+0x2BC/+0x2C0/+0x2C8` | CaptureManager, victim controller pointer, MC ring anim | existing `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` | Yes, separate subsystem |

## 3. Core Binary Findings

### 3.1 Limbo tail broadcasts before Conceal

`TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80`:

1. Reads `this+0x81`.
2. If not already in limbo, pushes `3`.
3. Calls `this->vtable[0x280]`.
4. Calls `ObjectClass__Conceal @ 0x005F4D30`.

Assembly proof: `0x0065AA83..0x0065AA91` checks `+0x81` and calls `[EAX+0x280]` with `PUSH 0x3`; `0x0065AA97..0x0065AA99` calls `0x005F4D30`.

**Active in YR:** Yes. `get_function_callers(0x0065AA80)` returned `TechnoClass__Limbo_Helper @ 0x006F6AC0`; `get_function_xrefs` returned one call from `0x006F6C95` and one data xref at `0x007F05DC`.

### 3.2 Foot and Building limbo both reach the helper

`TechnoClass__Limbo_Helper @ 0x006F6AC0` has exactly two direct callers in this Ghidra project:

- `FootClass__Limbo @ 0x004DB260`, call at `0x004DB3B1`.
- `BuildingClass__Limbo @ 0x00445880`, call at `0x00445DDA`.

The helper itself tail-calls `0x0065AA80` at `0x006F6C95`. This resolves the older uncertainty that buildings might follow a separate path: `BuildingClass__Limbo` does class-specific cleanup first, then still enters the Techno helper and the broadcast tail.

**Active in YR:** Yes. Foot/Unit/Infantry/Aircraft and buildings use these virtual Limbo paths for normal removal/destruction. No TS-only scenario flag gates the `0x006F6C95 -> 0x0065AA80` tail.

### 3.3 Broadcast loops all non-null contacts and dispatches through Transmit_Radio_Impl

`RadioClass__Broadcast_Radio_ToAll @ 0x0065ACE0`:

1. Initializes index to 0.
2. If `Contacts.Capacity <= 0`, returns.
3. For each index `< Capacity`, reads `Contacts.data[index]`.
4. If non-null, calls `this->vtable[0x27C](msg, &g_RadioScratchBuffer, target)`.

Assembly proof: `0x0065ACE6..0x0065ACEE` checks capacity; `0x0065ACF5..0x0065ACFE` loads the slot and null-checks; `0x0065AD04..0x0065AD0D` pushes target, global scratch `0x00A8EC30`, message, and calls `[EDX+0x27C]`.

**Active in YR:** Yes. The function is present in multiple Techno-derived vtables (`get_function_xrefs(0x0065ACE0)` data refs include `0x007E2524`, `0x007E413C`, `0x007E8F14`, `0x007EB2D8`, `0x007F0788`, `0x007F4BE0`, `0x007F5EF0`), and the limbo tail invokes it virtually.

### 3.4 BREAK clears sender-side slot before target Receive_Radio

For message `3`, `RadioClass__Transmit_Radio_Impl @ 0x0065A970` walks every contact slot and nulls slots matching the target before dispatching to the target's receive slot `+0x194`.

Assembly proof:

- Clear loop: `0x0065A99C..0x0065A9C7`, store zero at `0x0065A9B8`.
- Dispatch after clearing: `0x0065A9C9..0x0065A9DB`, call `[EBX+0x194]`.

**Active in YR:** Yes. This is the shared send implementation called by broadcast and single-target BREAK.

### 3.5 Target-side BREAK clears matching sender slot after ObjectClass receive side effects

`RadioClass__Receive_Radio @ 0x0065A820` updates radio history, then for message `3` searches contacts for the sender. If found, it calls `ObjectClass__Receive_Radio(sender, 3, payload)` and then nulls the matching contact slot, returning `1`.

Assembly proof: `0x0065A854..0x0065A870` scans; `0x0065A886..0x0065A890` calls `0x005F5320`; `0x0065A895..0x0065A8A0` writes zero and returns `1`.

**Active in YR:** Yes. Subclass receivers delegate unhandled or tail behavior to this base handler.

### 3.6 Techno BREAK may cascade `0x19` before base BREAK cleanup

`TechnoClass__Receive_Radio @ 0x006F4AB0` case `3` checks receiver `+0x418` and sender `+0x418`; if both are non-zero, it transmits `0x19` to the sender before calling `RadioClass__Receive_Radio`.

Assembly proof: `0x006F4C50..0x006F4C66` checks both flags; `0x006F4C68..0x006F4C7A` sends `0x19`; `0x006F4C80..0x006F4C89` calls `0x0065A820`.

**Active in YR:** Conditional. The code is live for Techno-derived contacts, but the `0x19` cascade only happens while both sides have the radio/contact flag set.

### 3.7 Building BREAK does GrandOpening before Techno BREAK

`BuildingClass__Receive_Radio @ 0x0043C2D0` case `3` calls `BuildingClass__GrandOpening()`, then `TechnoClass__Receive_Radio(sender, 3, payload)`, then returns `1`.

**Active in YR:** Yes. This is the building-specific receiver path for break messages. Player-visible consequence: building-side contact break can reset/open building visual state before the common contact clear runs.

### 3.8 Object destruction uses Limbo after separate observer cleanup

`ObjectClass__UnInit @ 0x005F65F0` runs non-radio cleanup first, then calls virtual slot `+0xD4` (Limbo), then clears `IsAlive` and queues pending delete.

Assembly proof:

- `0x005F6609..0x005F660D`: Foot objects call `FootClass__EMPPassengers(0)`.
- `0x005F6612..0x005F6616`: calls `Detach_From_All_Lists @ 0x007258D0`.
- `0x005F661B..0x005F661F`: calls virtual `+0xD4`, which reaches the limbo/broadcast path for Techno objects.
- `0x005F6625`: clears `+0x90` alive byte after limbo.

**Active in YR:** Yes. This means death/destruction reaches `Broadcast_Radio_ToAll(3)` through the normal Limbo virtual call, but observer/listener cleanup is a separate step.

### 3.9 The `0x007F05DC` xref is vtable registration, not an additional caller

Memory read at `0x007F0508` shows the RadioClass vtable. `RadioClass__Constructor @ 0x0065A750` writes `0x007F0508` to `this+0`, and the dword at `0x007F05DC` is `0x0065AA80`. Since `0x007F05DC - 0x007F0508 = 0xD4`, this is the virtual Limbo slot in the RadioClass vtable.

**Active in YR:** Yes as class metadata. It is not an execution edge by itself.

## 4. Player-Visible Consequences By Link Type

| Link type | Binary cleanup consequence | Evidence | Active in YR |
|---|---|---|---|
| Dock links and building contact arrays | Limbo/death sends `BREAK(3)` to every contact before conceal. Sender-side contact slots are nulled before target receive; target slots are nulled in `RadioClass__Receive_Radio`. Building receiver additionally runs `GrandOpening` before common cleanup. | `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x0065A820`, `0x0043C2D0` | Yes |
| War factory/NumberImpassableRows contact exception | Since Rust models `radio_contacts` as mover-side passability state, stale contacts can keep a moved/despawned unit eligible for contacted-building cell exceptions. The binary prevents this by BREAKing contacts on limbo. | this report plus `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`; Rust `game_entity.rs:115..120`, `movement_occupancy.rs:190` | Yes |
| Passenger/transports | Boarding uses `CargoClass__AddPassenger`, which calls passenger virtual `+0xD4` Conceal before inserting into cargo; this will run the same limbo radio BREAK for the passenger. Transport death uses `ObjectClass__UnInit`, calls `FootClass__EMPPassengers(0)` before Limbo, then the transport's radio BREAK broadcast. Cargo chain cleanup is separate from RadioClass contacts. | `0x004733AC..0x004733B0`, `0x005F6609..0x005F661F`, `0x00707CB0` | Yes |
| Carryall links | Carryall pickup protocol uses radio HELLO/WANT_RIDE/NEED_TO_MOVE and final `0x19` then `0x03`; any limbo of either side broadcasts BREAK to the current contact before conceal. | this report; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` carryall section; INI `[HIND] Carryall=yes`, `TechLevel=-1` | Conditional: protocol live, stock skirmish carrier dormant |
| Service links (repair depot/hospital/armory style radio service) | UnitRepair uses radio contact and periodic radio checks/repair messages. If depot or serviced unit limbos, BREAK clears contacts; future service ticks must not keep repairing or blocking based on stale partner IDs. | `BuildingClass__Receive_Radio @ 0x0043C2D0`, `TechnoClass__Receive_Radio @ 0x006F4AB0`, `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md` | Yes |
| Mind-control links | Mind control is CaptureManager-based, not RadioClass contact-based in this slice. `ObjectClass__UnInit` calls `Detach_From_All_Lists` before Limbo and prior reports show controller death/transport/chrono cleanup uses `CaptureManagerClass::FreeAll/FreeUnit`. Do not implement MC release as a RadioClass BREAK side effect. | `0x005F6612..0x005F661F`, `0x007258D0`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` | Yes, separate subsystem |

## 5. Current Rust Implementation Status

Rust has a deterministic `radio_contacts: Vec<u64>` on `GameEntity`, with helpers `mark_live_contact_with`, `has_live_contact_with`, and `clear_live_contact_with` in `src/sim/game_entity.rs`. The state hash includes this vector in `src/sim/world/world_hash.rs`.

Important current surfaces:

- `src/sim/game_entity.rs:115..120`: per-entity `radio_contacts`.
- `src/sim/game_entity.rs:425..438`: mark/query/clear helpers.
- `src/sim/movement/movement_occupancy.rs:190`: passability checks use `mover.has_live_contact_with(building.stable_id)`.
- `src/sim/production/production_spawn.rs:180`: produced land vehicle marks live contact with factory.
- `src/sim/world/mod.rs:530..555`: `despawn_entity` removes the entity and occupancy, but does not broadcast or clear other entities' `radio_contacts`.
- `src/sim/miner/miner_dock.rs:138..150` and `278..290`: refinery/depot reservation cleanup exists.
- `src/sim/docking/building_dock.rs:73..85`: repair depot reservations remove dying entities immediately.
- `src/sim/docking/aircraft_dock.rs:239..245`, `349..352`: airfield dock cleanup exists, but the tick-level alive set includes dying entities in this path.
- `src/sim/passenger.rs:399..405`: boarding hides passenger and clears movement/attack/order intent; `src/sim/passenger.rs:531..555` unloads and re-adds occupancy.

**Rust delta:** There is no generic RadioClass-style "broadcast BREAK to all contacts and reciprocally clear peers" operation at limbo/death/despawn. Several per-system cleanup hooks exist, but the generic `GameEntity.radio_contacts` vector can retain stale IDs unless every producer cleans it explicitly. `clear_live_contact_with` exists but has no non-test production caller in the current scan.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80` | verified | decompile + assembly `0x0065AA80..0x0065AA9F` | none |
| callers of `0x0065AA80` | verified | `get_function_callers` -> `0x006F6AC0`; `get_function_xrefs` -> `0x006F6C95`, `0x007F05DC` | none |
| `0x007F05DC` identity | verified | `read_memory(0x007F0508)`, `RadioClass__Constructor @ 0x0065A750` | none |
| `TechnoClass__Limbo_Helper @ 0x006F6AC0` callers | verified | callers `0x004DB260`, `0x00445880`; xrefs `0x004DB3B1`, `0x00445DDA`, `0x007F4A34` | none |
| `RadioClass__Broadcast_Radio_ToAll @ 0x0065ACE0` | verified | decompile + assembly `0x0065ACE0..0x0065AD21` | none |
| `RadioClass__Transmit_Radio_Impl` BREAK order | verified | decompile + assembly `0x0065A99C..0x0065A9DB` | none |
| `RadioClass__Receive_Radio` BREAK order | verified | decompile + assembly `0x0065A854..0x0065A8A0` | none |
| `TechnoClass__Receive_Radio` `0x19` cascade | verified | decompile + assembly `0x006F4C50..0x006F4C89` | none |
| `BuildingClass__Receive_Radio` BREAK case | verified | decompile `0x0043C2D0` | exact `GrandOpening` internals out of scope |
| `ObjectClass__UnInit` ordering | verified | decompile + assembly `0x005F65F0..0x005F6681` | exact pending-delete allocator gates out of scope |
| Passenger cargo interaction | touched-not-exhausted | decompile + assembly `CargoClass__AddPassenger @ 0x004733A0`; `FootClass__EMPPassengers @ 0x00707CB0` | full unload/destructor order deferred |
| Mind-control cleanup | touched-not-exhausted | `ObjectClass__UnInit @ 0x005F65F0`, `Detach_From_All_Lists @ 0x007258D0`, prior MC reports | exact CaptureManager callsite re-decompile deferred |
| Rust generic radio contact cleanup | verified-by-scan | `rg` and source reads listed in section 5 | implementation not performed |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `0x0065AA80` the primary limbo broadcast caller? -> Yes, it is the tail called from `TechnoClass__Limbo_Helper`, and it invokes vtable `+0x280` with arg `3` before Conceal.` (evidence: `0x0065AA80`, `0x006F6C95`)
- `[RESOLVED] OQ-2 - Does `Broadcast_Radio_ToAll(3)` skip null contacts? -> Yes, it checks each slot and only calls `+0x27C` when the slot pointer is non-null.` (evidence: `0x0065ACF5..0x0065AD0D`)
- `[RESOLVED] OQ-3 - Is `0x007F05DC` a second caller? -> No, it is RadioClass vtable base `0x007F0508` plus `0xD4`, containing pointer `0x0065AA80`.` (evidence: `read_memory(0x007F0508)`, `0x0065A798`)
- `[RESOLVED] OQ-4 - Do buildings also reach this broadcast path? -> Yes, `BuildingClass__Limbo @ 0x00445880` calls `TechnoClass__Limbo_Helper @ 0x00445DDA`, which calls the tail.` (evidence: `0x00445DDA`, `0x006F6C95`)
- `[RESOLVED] OQ-5 - Does sender-side BREAK clear before target receive? -> Yes, `Transmit_Radio_Impl` nulls matching target slots before calling target `Receive_Radio`.` (evidence: `0x0065A9B8`, `0x0065A9DB`)
- `[RESOLVED] OQ-6 - Does target-side BREAK clear reciprocally? -> Yes when sender is found in target contacts; it calls `ObjectClass__Receive_Radio` then nulls that slot.` (evidence: `0x0065A886..0x0065A8A0`)
- `[RESOLVED] OQ-7 - Can BREAK trigger a `0x19` cleanup cascade? -> Yes, conditionally when both receiver and sender `+0x418` bytes are set.` (evidence: `0x006F4C50..0x006F4C7A`)
- `[RESOLVED] OQ-8 - Is mind-control release implemented by RadioClass BREAK? -> No evidence in this slice; destruction calls `Detach_From_All_Lists` before Limbo, and prior reports place MC release in CaptureManager `FreeAll/FreeUnit`.` (evidence: `0x005F6612..0x005F661F`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-9 - Does current Rust have generic reciprocal radio contact cleanup on despawn? -> No production caller of `clear_live_contact_with` was found; `despawn_entity` removes entity/occupancy only.` (evidence: `rg clear_live_contact_with`, `src/sim/world/mod.rs:530..555`)
- `[DEFERRED] OQ-10 - Exact transport destructor/cargo disposal order after `FootClass__EMPPassengers`?` (category: `out-of-scope`; reason: cargo disposal is adjacent to but not called by `Broadcast_Radio_ToAll`; next-step-if-pursued: focused CargoClass destructor/unload/death report)
- `[DEFERRED] OQ-11 - Exact CaptureManager listener path for victim removal?` (category: `out-of-scope`; reason: MC cleanup is separate from RadioClass broadcast; next-step-if-pursued: verify `CaptureManagerClass::FreeUnit/FreeAll` callsites against current Ghidra)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Techno limbo/death broadcasts `BREAK(3)` to every non-null contact before Conceal/InLimbo. | `0x0065AA80`, `0x0065ACE0`, `0x005F4D30` | Missing generic equivalent | `Simulation::despawn_entity`, future limbo/conceal API, `GameEntity.radio_contacts` | Before removing or hiding a Techno, collect its contacts, clear its own list, and clear reciprocal contact IDs from peers while preserving deterministic order. | `radio_break_broadcast_on_despawn_clears_reciprocal_contacts` | Do not rely only on per-system dock reservation cleanup; passability contacts are generic state-hashed sim data. |
| BREAK order is sender clear first, target receive second; target clear occurs in target receive. | `0x0065A99C..0x0065A9DB`, `0x0065A886..0x0065A8A0` | Unchecked/missing | radio contact cleanup helper | Future helper should be idempotent and tolerate stale/missing peer IDs; one side may clear before the other observes the break. | Same test plus a one-sided stale peer contact fixture | Do not compact/reorder unrelated contacts as a side effect beyond removing matching IDs. |
| Techno BREAK can send `0x19` first if both `+0x418` flags are set. | `0x006F4C50..0x006F4C7A` | Modeled indirectly in miner/depot phase cleanup, not generic | dock/contact-entered abstractions: `RefineryDockContacts`, `DockReservations`, future service/carryall links | When modeling dock-entered flags, BREAK cleanup must clear entered/contact state before or along with contact removal. | `radio_break_dock_entered_flags_clear_before_contact_drop` | Do not implement `0x19` as an unconditional limbo broadcast; it is conditional on both sides' entered flags. |
| Building receivers run `GrandOpening` before common Techno BREAK cleanup. | `BuildingClass__Receive_Radio @ 0x0043C2D0` case `3` | Partly missing/unchecked | building dock/service visual state, production/dock anim surfaces | If building contact break has visual state, reset/open building state before dropping the contact. | `building_radio_break_resets_dock_visual_before_contact_clear` | Do not model building BREAK as only deleting a vector entry when dock/service visuals are implemented. |
| Mind-control cleanup is not a RadioClass BREAK side effect. | `ObjectClass__UnInit @ 0x005F65F0`; `Detach_From_All_Lists @ 0x007258D0`; prior MC docs | Mind-control implementation status outside this slot | future CaptureManager/mind-control subsystem | MC controller/victim release must be implemented in CaptureManager-style ownership cleanup, not hidden inside radio contact cleanup. | `mind_control_controller_despawn_releases_victims_without_radio_contact` | Do not conflate MC link lines with RadioClass contacts. |

Concrete Rust test-name proposal: `radio_break_broadcast_on_despawn_clears_reciprocal_contacts`.

## 9. Negative Facts / Do Not Do

- Do not treat `0x007F05DC` as a runtime caller. It is a RadioClass vtable entry.
- Do not skip buildings: `BuildingClass__Limbo` reaches the same Techno limbo helper and broadcast tail.
- Do not clear only the despawned entity's `radio_contacts`; the peer-side contact must also be removed.
- Do not implement mind-control release as a radio BREAK effect. CaptureManager owns that behavior.
- Do not make carryall cleanup a standard skirmish requirement for stock YR parity; the protocol is live, but `[HIND] Carryall=yes` is `TechLevel=-1`.
- Do not use `RadioClass::Tether_Count` as the contacts vector cleanup primitive; the passability/contact vector is `RadioClass +0xE4/+0xE8`.

## 10. Stale Docs / Follow-up Wording

`RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` section 8.5 should be updated later with this replacement wording:

> `TechnoClass::Limbo_Tail_CallConceal @ 0x0065AA80` is reached through `TechnoClass::Limbo_Helper @ 0x006F6AC0`, whose direct callers are `FootClass::Limbo @ 0x004DB260` and `BuildingClass::Limbo @ 0x00445880`. The data xref at `0x007F05DC` is not an unknown second caller; it is `vtable__RadioClass` base `0x007F0508` plus `0xD4`, the virtual Limbo slot pointing to `0x0065AA80`.

No in-repo or standalone docs were edited by this slot.

## Sources

- Live Ghidra decompile + assembly: `0x0065AA80`, `0x006F6AC0`, `0x004DB260`, `0x00445880`, `0x0065ACE0`, `0x0065A970`, `0x0065A820`, `0x006F4AB0`, `0x0043C2D0`, `0x005F4D30`, `0x005F65F0`, `0x004733A0`, `0x00707CB0`, `0x007258D0`, `0x007104C0`.
- Ghidra xrefs/callers: `get_function_callers/get_function_xrefs` on `0x0065AA80`, `0x006F6AC0`, `0x0065ACE0`.
- Ghidra memory: `read_memory(0x007F0508, 256)` showing `0x0065AA80` at `0x007F05DC`.
- Existing docs referenced: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`, `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md`, `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`.
- Rust source scan: `src/sim/game_entity.rs`, `src/sim/world/mod.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/production/production_spawn.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/docking/building_dock.rs`, `src/sim/docking/aircraft_dock.rs`, `src/sim/passenger.rs`, `src/sim/world/world_hash.rs`.
