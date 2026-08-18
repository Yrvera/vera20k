# Live Object Vector Order: Two Miners / One Refinery - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, logic add/remove helpers `0x0055BAA0` / `0x0055BAE0`, `DynamicVector::Insert @ 0x005519B0`, `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectClass::Conceal @ 0x005F4D30`, `UnitClass::Mission_Harvest @ 0x0073E5E0`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `FootClass::Mission_Enter @ 0x004D9290`, `ScenarioClass::Read_Units_Section @ 0x00743270`, `ScenarioClass::Generate_Random_Units @ 0x006886B0`, `BuildingClass::ExitObject_Main @ 0x00443C60`, `TeleportLocomotionClass::StateMachineTick @ 0x007192F0`  
**Investigation Mode:** exhaustive-slice for static live-vector ordering; partial for a concrete replay's actual indices/timers because that requires runtime logging.  
**Claimed Scope:** standard YR live object-vector order as it affects a two `CMIN`/`HARV`, one `GAREFN`/`NAREFN` takeover frame: insertion order, forward iteration, removal/compaction, whether teleport/dock/unload reorder miners, and what static evidence can say about normal creation order.  
**Non-Scope:** first rendered pixel displacement, exact replay capture, non-stock multi-dock buildings, slave miner, service depots, transports/garrisons, save/load reconstruction of the logic vector.  
**Confidence:** High for static vector rules and stability across normal CMIN dock/unload/teleport; Medium for "typical" natural order; Low for a specific replay instance without runtime logging.  
**Active in YR:** Yes. Stock `[CMIN]`/`[HARV]` have `Dock=NAREFN,GAREFN`; stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

## 1. Overview

The main live-object vector is append-ordered by successful `Reveal`/`Unlimbo` and ticked forward by increasing index. Objects are not re-sorted by position, mission, docking, or teleporting. Therefore, in the normal "first miner created before second miner" case, miner A remains earlier than miner B; if A releases a refinery contact during A's AI, B can observe the freed contact later in the same frame only if B's mission timer is due.

Static Ghidra cannot prove the exact object indices or mission timer state for an arbitrary retail replay. It can prove the rule to log: capture each miner's live-vector index and B's `MissionClass +0xC8/+0xD0` at the frame where A runs state 4.

## 2. Key Offsets / Values

| Offset / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass+0x81` | `InLimbo`; `Reveal` clears it, `Conceal` sets it | `0x005F4EC0`, `0x005F4D30` | Yes |
| `ObjectClass+0x98` | logic-vector membership flag; duplicate guard and remover gate | `0x0055BAA0`, `0x0055BAE0` | Yes |
| live vector data/count | passed as `param_1 + 4` / `param_1 + 0x10` in `LogicClass::PerTickUpdate` | `0x0055AFB0` | Yes |
| `MissionClass+0xC8/+0xD0` | B retry dispatch timer start/duration | prior slot; `MissionClass::Mission_Dispatch @ 0x005B3060` | Yes |
| accepted dock cell | refinery NW plus `(3,1)`; separate from `QueueingCell=4,1` | `BuildingClass::Receive_Radio @ 0x0043C2D0`; prior slot | Yes |
| `rulesmd.ini:[GAREFN] FreeUnit=CMIN` | Allied refinery creates a free CMIN | `rulesmd.ini:11736` | Yes |
| `rulesmd.ini:[NAREFN] FreeUnit=HARV` | Soviet refinery creates a free HARV | `rulesmd.ini:12530` | Yes |

## 3. Core Logic

### 3.1 Forward live-vector tick

`LogicClass::PerTickUpdate @ 0x0055AFB0` eventually reaches the main object vector pass:

```text
i = 0
while i < live_count:
    object = live_vector[i]
    object->vtable[0x5C]()   // AI / mission dispatch owner
    i += 1
    live_count is reread for the loop comparison
```

The count is not snapshotted at loop entry. Insertions after the current index can run later in the same frame; removals compact the vector and can shift later objects left. For the refinery takeover case no normal A-release path removes A from the live vector, so no A/B compaction skip is expected on healthy unload completion.

### 3.2 Logic-vector insertion is append ordered

`ObjectClass::Reveal @ 0x005F4EC0` calls `FUN_0055BAA0` for logic-enabled object types when the object is active for the scenario. `FUN_0055BAA0 @ 0x0055BAA0` first checks `Object+0x98`; if already set, it returns success without inserting again.

If not already registered, the helper calls `DynamicVector::Insert @ 0x005519B0` with sorted flag false. That function writes the object pointer at `vector[count]` and increments `count`. It does not sort by stable id, cell, house, object type, or mission.

Consequence: creation / successful reveal order is the object AI order unless later conceal/removal compacts the vector.

### 3.3 Conceal is the relevant reorder/removal boundary

`ObjectClass::Conceal @ 0x005F4D30` calls `FUN_0055BAE0` only for logic-enabled object types, then sets `InLimbo=1`. The remover checks `Object+0x98`, finds the object's current vector index, decrements vector count, shifts later entries left, and clears `Object+0x98`.

This means normal no-limbo movement cannot reorder the vector. Actual limbo/destruction/transport/garrison paths can remove and later reappend an object, but those are outside the healthy CMIN refinery takeover.

### 3.4 Normal CMIN return, dock, unload, and teleport do not remove/reappend the unit

The relevant active paths update mission state, destination, contact state, cell/coord, and animation/audio state without calling `ObjectClass::Conceal` or `ObjectClass::Reveal` on the miner:

- `UnitClass::Mission_Harvest @ 0x0073E5E0` sends close-path `HELLO(0x02)` or sets a far/refused fallback destination. It does not limbo the miner.
- `FootClass::Mission_Enter @ 0x004D9290` retries `CAN_DOCK(0x0E)`, sets/clears destination/radio state, and returns a mission delay. It does not limbo the miner.
- `UnitClass::Mission_Deploy_Building @ 0x0073D630` state 3/4 unload and release paths clear unload state, set Harvest, and send `BREAK(3)`. They do not remove the unit from the live vector.
- `TeleportLocomotionClass::StateMachineTick @ 0x007192F0` changes the object's coordinates/cell state through vtable `+0x1B4`, height/marking calls, ghost-cell updates, sounds, and warp anims. It does not call `vtable+0xD4`/`Conceal` or `vtable+0xD8`/`Reveal` for the owning miner on the normal warp path.

Consequence: a CMIN created earlier remains earlier across ore return, chrono warp, dock entry, unloading, and state-4 release.

### 3.5 What static evidence says about "typical" creation order

For ordinary map-spawned units, `ScenarioClass::Read_Units_Section @ 0x00743270` iterates `[Units]` entries by section index, constructs each unit, then calls the unit's vtable `+0xD8` / unlimbo. Since unlimbo/reveal appends to the logic vector, map-unit object order follows map section order for successful unit entries.

For generated multiplayer/skirmish units, `ScenarioClass::Generate_Random_Units @ 0x006886B0` loops houses and creates/unlimbos objects as it goes. The decompiled flow proves successful unlimbo occurs during generation, therefore insertion follows generation sequence, but the exact chosen unit set and per-house order are scenario/random-seed dependent.

For refinery-created free miners and later factory/exit products, `BuildingClass::ExitObject_Main @ 0x00443C60` calls the produced unit's vtable `+0xD8` / unlimbo at the exit coordinate. Stock `[GAREFN] FreeUnit=CMIN` and `[NAREFN] FreeUnit=HARV` mean the first refinery's free miner is inserted when that refinery exits/places its free unit; a later second miner produced from the economy is inserted later and therefore normally runs after the first miner.

The static answer is therefore:

- If miner A is the first/free/older miner and miner B is the later produced miner, A runs before B and same-frame claim is possible when B's mission timer is due.
- If a scenario or test creates B earlier than A, or if A was removed/reappended through an out-of-scope limbo path, B can run before A and no retroactive same-frame promotion happens.
- A concrete natural replay's exact A/B order still needs runtime logging because static Ghidra cannot identify the replay's actual vector indices after all prior object creations/removals.

## 4. Current Rust Implementation Status

Current Rust processes miners by deterministic stable id rather than by a separate gamemd-style live-vector insertion list:

- `src/sim/miner/miner_system.rs:87` says snapshots are sorted by `stable_id`; line 95 obtains `keys_sorted()`, and line 149 processes those snapshots in order.
- `src/sim/miner/miner_dock_sequence.rs:611` owns MissionEnter retry/admission; lines 669-675 keep accepted-cell movement separate from entered/pad handoff.
- `src/sim/miner/miner_dock_sequence.rs:898` owns Departing/state-4 cleanup; lines 912-917 release pad/contact and do not directly promote a waiter.
- `src/sim/miner/miner_tests.rs:3076` covers waiter-after-releaser same-tick claim.
- `src/sim/miner/miner_tests.rs:3223` covers waiter-before-releaser not being retroactively promoted.

Rust's stable-id order can represent the binary rule if tests deliberately set IDs to model both live-vector orderings. It is not the same internal structure as gamemd's append-ordered live vector, so parity claims should be phrased as order-dependent behavior, not a universal same-tick handoff guarantee.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| live-vector forward iteration | verified | `0x0055AFB0` | none |
| insertion appends at `vector[count]` | verified | `0x0055BAA0`, `0x005519B0` | none |
| duplicate insertion guard | verified | `Object+0x98` in `0x0055BAA0` | none |
| conceal/removal compacts later entries | verified | `0x005F4D30`, `0x0055BAE0` | none |
| healthy CMIN return/dock/unload does not conceal/reveal | verified | `0x0073E5E0`, `0x004D9290`, `0x0073D630` | none for scoped path |
| normal teleport does not conceal/reveal owner | verified | `0x007192F0` | none for normal warp path |
| map `[Units]` order source | verified | `0x00743270` loop | exact map order is data-dependent |
| multiplayer random-unit generation order | touched-not-exhausted | `0x006886B0` | exact unit choices/order need runtime seed/session |
| free/progress-produced unit exit insertion | verified for unlimbo call | `0x00443C60`; `FreeUnit` INI | exact production queue frame out of scope |
| concrete natural two-CMIN replay A/B index | deferred | static rule only | runtime trace required |
| concrete B mission timer at release frame | deferred | prior slot `0x005B3060` | runtime trace required |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does the live object pass run forward or reverse? -> The main object vector pass runs forward by increasing index and rereads count after each call.` (evidence: `0x0055AFB0`)
- `[RESOLVED] OQ-02 - Does logic insertion append or sort? -> Ordinary reveal passes sorted=false; `DynamicVector::Insert` writes at `vector[count]` and increments count.` (evidence: `0x0055BAA0`, `0x005519B0`)
- `[RESOLVED] OQ-03 - What prevents duplicate AI registration? -> `Object+0x98` is checked before insertion and set after successful insert.` (evidence: `0x0055BAA0`)
- `[RESOLVED] OQ-04 - What removes/reorders a live object? -> `ObjectClass::Conceal` calls `FUN_0055BAE0`, which compacts later entries and clears `Object+0x98`.` (evidence: `0x005F4D30`, `0x0055BAE0`)
- `[RESOLVED] OQ-05 - Does normal CMIN chrono teleport remove/reappend the miner? -> No in the inspected normal state-machine path; it changes coords/cell/marking/warp state without owner conceal/reveal.` (evidence: `0x007192F0`)
- `[RESOLVED] OQ-06 - Does healthy refinery unload state 4 remove/reappend A? -> No; it clears unload/contact state and sets Harvest, but does not conceal/reveal the unit.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-07 - Does B get admitted by a refinery-side callback? -> No; B admits only through B's later `Mission_Enter` retry.` (evidence: `0x004D9290`; prior slot report)
- `[RESOLVED] OQ-08 - For older A and later B, which runs first? -> A runs first as long as neither was removed/reappended, because append order is creation/reveal order.` (evidence: `0x005519B0`, `0x0055AFB0`, no conceal/reveal in scoped lifecycle)
- `[RESOLVED] OQ-09 - Can a map/test place B before A? -> Yes; map `[Units]` parsing iterates section entries and appends successful units in that order.` (evidence: `0x00743270`)
- `[RESOLVED] OQ-10 - Is the common refinery free miner before later produced miner? -> For stock refinery free miners followed by later production, yes: successful free/exit unit unlimbo appends first; later products append later.` (evidence: `0x00443C60`; `rulesmd.ini:[GAREFN] FreeUnit=CMIN`, `[NAREFN] FreeUnit=HARV`)
- `[DEFERRED] OQ-11 - What are A and B's exact live-vector indices in a particular retail replay?` (category: `needs-runtime-debugger`; reason: static code gives the insertion/removal rule, not the replay's full prior object history; next-step-if-pursued: log the main vector index/object pointer/type for both miners at `0x0055AFB0` on the release frame)
- `[DEFERRED] OQ-12 - Is B's mission timer due in that same concrete replay frame?` (category: `needs-runtime-debugger`; reason: static code gives timer fields/formula, but not the concrete jitter/frame value; next-step-if-pursued: log B `+0xC8/+0xD0` at `MissionClass::Mission_Dispatch @ 0x005B3060`)

## 7. Runtime Trace Needed For The Last Concrete Replay Question

Break/log at `LogicClass::PerTickUpdate @ 0x0055AFB0` for the main live-vector loop on the frame where A reaches `UnitClass::Mission_Deploy_Building` state 4:

1. For every object pointer in the live vector, log index, RTTI/`WhatAmI`, owner, type ID/name, and object pointer.
2. Mark A and B miner pointers and whether A's state is Deploy_Building substate 4.
3. On B's `MissionClass::Mission_Dispatch @ 0x005B3060`, log current frame, mission id, `+0xC8`, `+0xD0`, and whether mission id 7 dispatches.
4. On B's `FootClass::Mission_Enter @ 0x004D9290`, log `CAN_DOCK(0x0E)` reply.
5. On B's `FootClass::Receive_Radio(0x12) @ 0x004D8FB0`, log current cell vs accepted cell and whether it returns already-there `0x14` or movement-command `1`.

That trace resolves the remaining concrete replay outcome without re-opening the static rules.

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Main object order is append/reveal order and forward-ticked; older A before later B can allow same-frame B retry after A release. | `0x0055AFB0`, `0x0055BAA0`, `0x005519B0` | Rust uses stable-id order, not a logic-vector list; acceptable only if tests model both orders explicitly | `src/sim/miner/miner_system.rs::tick_miners`; miner tests | keep handoff order-dependent; stable-id lower/higher should model B-before-A and A-before-B cases | `two_miners_waiter_after_releaser_same_tick_claims_on_own_mission_enter`; `two_miners_waiter_before_releaser_not_retroactively_promoted` | Do not assert universal same-frame or universal next-frame takeover. |
| Healthy CMIN teleport/dock/unload does not reappend or reorder the miner. | `0x007192F0`, `0x0073E5E0`, `0x0073D630`, `0x004D9290` | none observed for current miner lifecycle; Rust entity remains same stable id | `miner_dock_sequence.rs`, movement/teleport integration | keep miner identity/order stable across chrono return, accepted-cell movement, unloading, and state-4 cleanup | dock/unload tests should not remove/reinsert the miner entity during normal completion | Do not model dock/unload by despawning and respawning the miner. |
| B admission after A release belongs to B's own retry, not A's release callback. | `0x004D9290`; prior `MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER` report | mostly matched | `miner_dock_sequence.rs::phase_departing`, `phase_mission_enter`; `RefineryDockContacts` | A release frees contact only; B claims only when B is processed and eligible | waiter-before-releaser remains queued; waiter-after-releaser can claim if eligible | Do not add refinery-side FIFO promotion on release. |

## 9. Negative Facts / Do Not Do

- Do not use cell position, target refinery distance, or dock phase as a sort key for gamemd-style AI order.
- Do not claim same-frame takeover is guaranteed for two normal miners; B's mission timer can still block it.
- Do not claim same-frame takeover is impossible; older A before later B gives the right static order if B's timer is due.
- Do not despawn/respawn a CMIN for normal chrono return, dock, unload, or state-4 release.
- Do not infer a concrete retail replay's A/B order without logging the actual live vector; prior object creations/removals can change indices.

## 10. Remaining Uncertainty

- Exact A/B indices and B's timer value in a natural retail replay remain runtime-only.
- First rendered displacement/overlap remains runtime-only; static evidence proves command/retry timing, not pixel presentation.
- Save/load reconstruction of `Object+0x98` and vector order is out of scope and may need separate persistence research.

## 11. Stale Docs / Follow-up Wording

- Upgrade wording in the older two-miner frame-order report from "runtime object-vector order unknown" to: "Static binary evidence proves append/reveal order and stability across normal CMIN teleport/dock/unload; for the common free/older miner A and later-produced miner B case, A remains earlier than B. A concrete replay still requires live-vector index and mission-timer logging."
- Replace any wording implying docking or chrono teleport reorders a miner with: "Normal CMIN return, warp, dock, unload, and state-4 release update coordinates/state/contact fields without conceal/reveal, so they preserve live-vector order."

## Sources

- Ghidra read-only decompile: `LogicClass::PerTickUpdate @ 0x0055AFB0`.
- Ghidra read-only decompile: `FUN_0055BAA0 @ 0x0055BAA0`.
- Ghidra read-only decompile: `DynamicVector::Insert @ 0x005519B0`.
- Ghidra read-only decompile: `ObjectClass::Reveal @ 0x005F4EC0`.
- Ghidra read-only decompile: `ObjectClass::Conceal @ 0x005F4D30`.
- Ghidra read-only decompile: `FUN_0055BAE0 @ 0x0055BAE0`.
- Ghidra read-only decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only decompile: `TeleportLocomotionClass::StateMachineTick @ 0x007192F0`.
- Ghidra read-only decompile: `ScenarioClass::Read_Units_Section @ 0x00743270`.
- Ghidra read-only decompile: `ScenarioClass::Generate_Random_Units @ 0x006886B0`.
- Ghidra read-only decompile: `BuildingClass::ExitObject_Main @ 0x00443C60`.
- Prior docs: `MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`, `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.

## Status

PARTIAL for a concrete replay's exact A/B indices and B timer; COMPLETE for the static live-vector ordering and normal CMIN lifecycle stability rule.
