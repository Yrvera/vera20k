# Strip AI Factory Delivery Globals and Queue Restart - Ghidra Research Report

**Address(es):** `0x006A8B30` (`StripClass::AI`), `0x00734250` (vehicle delivery global setter), `0x00734270` (delivery global clearer), `0x007342A0/0x007342B0` (delivery global getters), `0x004FB0E0` (`HouseClass::Place_Production`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** completed-item delivery dispatch from `StripClass::AI`, vehicle delivery globals `DAT_00B0FE5C/60`, and immediate `Place_Production -> CompletedProduction -> FUN_004FAA10 -> StartNextQueued` queue-restart timing.
**Non-Scope:** full sidebar rendering, full war-factory exit-cell search, full command-button class catalog, refinery docking, and full production-rate math.
**Confidence:** High
**Active in YR:** Yes. These paths are reachable from ordinary stock YR production completion, sidebar action, and command execution. No TS-only gate was found in this slice.

## 1. Overview

`StripClass::AI` is the sidebar poller that notices completed factories. For buildings, infantry, and aircraft it queues a network `Place_Production` command (`0x0B`) immediately during the same strip AI pass. For produced vehicles (`WhatAmI == 6`) it does not queue that command there; it plays the unit-ready EVA and stores the completed unit pointer in one of two global delivery slots for later command/UI handling.

Successful delivery/place is what restarts the next queued item. `HouseClass::Place_Production` calls `FactoryClass::CompletedProduction` and then `FUN_004FAA10` in the same command execution. Normal completion commands carry `heapId = -1`, so `FUN_004FAA10` skips the non-naval "remove one queued type" cancel branch and reaches `FactoryClass::StartNextQueued` immediately if the queue is non-empty.

## 2. Class Layout / Key Offsets

| Field / global | Type | Purpose | Active in YR | Evidence |
|---|---:|---|---|---|
| `CameoEntry +0x0C` | `FactoryClass*` | Factory pointer polled by `StripClass::AI` | Yes | `0x006A8DC6..0x006A8DD3` |
| `FactoryClass +0x24` | int | Production value, complete at `0x36` | Yes | `FactoryClass::IsComplete @ 0x004CA130` |
| `FactoryClass +0x50` | int | queued type count | Yes | `FUN_004FAA10`, `StartNextQueued` |
| `FactoryClass +0x58` | `TechnoClass*` | produced object pointer | Yes | `GetObject @ 0x004CA160` |
| `FactoryClass +0x5D` | bool | `IsDifferent`, read-and-reset by sidebar | Yes | `HasChanged @ 0x004C9C60` |
| `FactoryClass +0x70` | bool | suspended/completed flag | Yes | `CompletedProduction @ 0x004CA1A0` |
| produced unit `+0x520` | type pointer | read by delivery helper | Yes | `0x00734250` |
| type `+0xE08` | int | naval split value; `5` selects naval slot | Yes | `0x00734256` |
| `DAT_00B0FE5C` | `UnitClass*` | pending non-naval vehicle delivery global | Yes | `0x00734266`, getter `0x007342A0` |
| `DAT_00B0FE60` | `UnitClass*` | pending naval vehicle delivery global | Yes | `0x0073425F`, getter `0x007342B0` |

## 3. Core Logic

### Completed-factory polling in `StripClass::AI`

The production-delivery block is gated by strip byte `+0x3D != 0`. For each cameo entry:

1. Reads `entry.FactoryPtr`.
2. Calls `FactoryClass::HasChanged`; this clears `FactoryClass +0x5D`.
3. If changed, calls `IsComplete`.
4. If complete and `GetObject` is non-null, dispatches by produced object's `WhatAmI`.

Dispatch details:

| Produced object `WhatAmI` | `StripClass::AI` action | Command queued here? | Evidence |
|---:|---|---|---|
| `1` building | Build `EventClass` command `0x0B` with invalid cell and `heapId = -1`; insert into 0x80-entry command ring | Yes | `0x006A8E05..0x006A8E1E`, `0x006A8E52` block not taken; command builder call in decompile |
| `2` infantry | same command path as building | Yes | same switch body |
| `0x0F` aircraft | same command path as building | Yes | same switch body |
| `6` unit/vehicle | play EVA, then call `FUN_00734250` with the produced unit pointer | No | `0x006A8E25..0x006A8E48` |

The command builder `FUN_004C6AE0` writes command fields as `[+0x07]=rtti`, `[+0x0B]=heapId`, `[+0x0F]=naval flag`, `[+0x13/+0x15]=cell`. The `StripClass::AI` completion command passes `heapId = -1`, not the produced object's heap id. This one value is the important queue-restart detail.

### Vehicle delivery globals

`FUN_00734250` is only a setter:

| Condition | Write |
|---|---|
| `*(produced +0x520)->+0xE08 == 5` | `DAT_00B0FE60 = produced` |
| otherwise | `DAT_00B0FE5C = produced` |

Adjacent helpers in the same code island:

| Helper | Behavior | Xrefs |
|---|---|---|
| `0x00734270` | if arg is zero, clear both globals; else clear whichever slot equals arg | `0x00685120`, `0x006851F0`, `0x006ABB60`, `0x004ABBD7` |
| `0x007342A0` | return `DAT_00B0FE5C` | command action at `0x00535DAA` |
| `0x007342B0` | return `DAT_00B0FE60` | command action at `0x00535E6A` |

The command action readers first switch the sidebar to tab `0` for the non-naval getter or tab `1` for the naval getter, then retrieve the pending unit. If non-null, they call the produced unit's vtable `+0x190` and pass the result plus the unit into `HouseClass::Begin_Building_Placement @ 0x004FB840`. That path sets placement/UI globals for the pending produced vehicle; it is not itself `Place_Production`.

### Successful `Place_Production` restart timing

`EventClass::Execute @ 0x004C710B` is the sole direct caller of `HouseClass::Place_Production`. For command `0x0B`, it passes:

| Event field | `Place_Production` arg |
|---|---|
| `[event+0x07]` | RTTI |
| `[event+0x0B]` | heap id |
| `[event+0x0F] != 0` | naval bool |
| `[event+0x13/+0x15]` | cell pointer |

Normal completed-item commands from `StripClass::AI` and ready-cameo click paths use `heapId = -1`. After a successful place/exit, `HouseClass::Place_Production` calls:

1. `FactoryClass::CompletedProduction(factory)`.
2. `FUN_004FAA10(house, rtti, heapId, navalBool, removeAll=0)`.
3. `Record_Last_Built` and placement/sound side effects.

Because normal completion uses `heapId = -1`, `FUN_004FAA10` does not enter the non-naval queue-removal branch. It reaches `FactoryClass::AbandonProduction` (a no-op after `CompletedProduction` has already cleared `Object`) and then calls `FactoryClass::StartNextQueued` if `QueuedObjects_Count != 0`. Therefore queue restart after successful delivery is same command execution, not a later sidebar tick.

### Blocked exit timing boundary

In the invalid-cell auto-exit branch of `Place_Production`, the factory building/contact object is asked whether it can accept/exit the produced object. If the result is not accepted:

| Produced object | Failure behavior | Queue restart? |
|---|---|---|
| `WhatAmI == 6` vehicle | returns `0` from `Place_Production` without `CompletedProduction` or `FUN_004FAA10` | No |
| not vehicle | logs failed factory exit and calls `FUN_004FAA10` | Cancel/cleanup path only |

Thus a blocked war-factory vehicle exit must keep the completed object attached to the factory and must not start the next queued item until a later successful delivery.

## 4. INI Keys

No INI key controls the delivery globals or queue-restart branch. Relevant stock data only determines which factory/produced type is considered naval:

| INI key | Stock YR value / examples | Effect in this slice | Evidence |
|---|---|---|---|
| `Factory=UnitType` | `GAWEAP`, `NAWEAP`, `YAWEAP`, `GAYARD`, `NAYARD`, `YAYARD` | determines production building category in Rust-facing scan; binary slice uses factory pointers already selected upstream | `ini/rulesmd.ini` |
| `Naval=yes` | shipyards and naval units | upstream type field contributes to naval factory/global split | `ini/rulesmd.ini`; binary reads type `+0xE08 == 5` |
| `[General] MaximumQueuedObjects=29` | stock YR `29` | queue capacity, not restart timing | `ini/rulesmd.ini` |

## 5. Integration Points

| Function | Role | Verified detail |
|---|---|---|
| `FactoryClass::HasChanged @ 0x004C9C60` | sidebar poll trigger | read-and-clear `IsDifferent`; if another poll consumes it, this strip will not deliver on that pass |
| `FactoryClass::IsComplete @ 0x004CA130` | completion predicate | true if object non-null and progress `0x36`, or special item non-`-1` and progress `0x36` |
| `FactoryClass::GetObject @ 0x004CA160` | produced object accessor | returns `FactoryClass +0x58` |
| `StripClass::AI @ 0x006A8B30` | completion command/global writer | queues `0x0B` for building/infantry/aircraft; writes globals for vehicles |
| `FUN_00734250 @ 0x00734250` | vehicle delivery global setter | non-naval slot vs naval slot split by `+0xE08 == 5` |
| `FUN_00734270 @ 0x00734270` | global clearer | clear both on arg zero; clear matching one on arg nonzero |
| command action readers `0x00535DAA/0x00535E6A` | vehicle global consumers | select tab 0/1 and call `Begin_Building_Placement` if a pending unit exists |
| `EventClass::Execute @ 0x004C710B` | `Place_Production` executor | command `0x0B` is what calls `Place_Production` |
| `HouseClass::Place_Production @ 0x004FB0E0` | place/exit commit | successful commit calls `CompletedProduction` then `FUN_004FAA10` |
| `FactoryClass::StartNextQueued @ 0x004CA5A0` | restart next queued type | pops queue front and calls `Begin_Production(..., resume=1)` |

## 6. Current Rust Implementation Status

Rust currently models production as a `VecDeque<BuildQueueItem>` where the front item is the active item. In `src/sim/production/production_queue.rs`, `tick_production` advances the front item, pops it on completion, and immediately either pushes buildings into `ready_by_owner` or spawns units directly.

Important deltas:

| Rust surface | Current behavior | Binary delta |
|---|---|---|
| `tick_production` | pops completed vehicle before spawn attempt | gamemd keeps `FactoryClass::Object` complete until successful `Place_Production` |
| blocked vehicle spawn | refunds cost and continues | gamemd blocked vehicle exit does not call `CompletedProduction`; no refund and no queue restart |
| queue restart | next item becomes front immediately after pop; effective next tick | gamemd `StartNextQueued` is called inside successful `Place_Production` command |
| aircraft no-pad case | refunds if no helipad | gamemd aircraft completion uses `Place_Production` command path and does not fit this vehicle-global slice |
| `production_spawn.rs` | searches fallback cells around factories | blocked war-factory exit needs exact `Place_Production` accepted/rejected behavior from slot 3/4 reports |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `StripClass::AI` completed production dispatch | verified | `0x006A8DC6..0x006A8E48` | none for scoped dispatch |
| `FUN_00734250` setter | verified | `0x00734250` | none |
| delivery global clear/get helpers | verified | `0x00734270`, `0x007342A0`, `0x007342B0` | full command class naming out-of-scope |
| command action global readers | touched-not-exhausted | `0x00535DAA`, `0x00535E6A`, `0x004FB840` | exact UI command class names/vtable entries |
| `EventClass::Execute` command `0x0B` arg mapping | verified | `0x004C70E1..0x004C710B` | none |
| `HouseClass::Place_Production` success tail | verified | `0x004FB649..0x004FB67A` | exact exit-cell accept codes owned by slot 3/4 |
| `FUN_004FAA10` restart path | verified | `0x004FAA10`, `0x004FAB64..0x004FABB5` | none for normal `heapId=-1` completion |
| non-naval cancel/remove branch | verified | `0x004FAA10`, `0x004FAAEB..0x004FAB37` | full cancel UI semantics out-of-scope |
| naval vs non-naval restart difference | verified | `0x004FAA10`, `0x00734250` | none |
| Rust production queue scan | verified | `src/sim/production/production_queue.rs`, `production_spawn.rs` | no code changes in this research pass |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `StripClass::AI @ 0x006A8B30` active in YR? -> Yes; it is the live sidebar strip AI and directly handles completed factories.` (evidence: `0x006A8B30`, prior sidebar docs)
- `[RESOLVED] OQ-2 - Does `StripClass::AI` queue `Place_Production` for vehicles? -> No; `WhatAmI == 6` plays EVA and calls `FUN_00734250` instead.` (evidence: `0x006A8E25..0x006A8E48`)
- `[RESOLVED] OQ-3 - What do `DAT_00B0FE5C/60` store? -> Produced vehicle pointers; `5C` non-naval, `60` naval.` (evidence: `0x00734250`)
- `[RESOLVED] OQ-4 - What selects naval vs non-naval global? -> produced unit type pointer `+0x520`, field `+0xE08 == 5`.` (evidence: `0x00734256`)
- `[RESOLVED] OQ-5 - Who clears the globals? -> `FUN_00734270`, called on display/session reset, factory-strip removal, bandbox path; zero arg clears both.` (evidence: xrefs to `0x00734270`)
- `[RESOLVED] OQ-6 - Who reads the globals? -> tiny command action readers at `0x00535DAA` and `0x00535E6A`.` (evidence: xrefs to `0x007342A0/B0`)
- `[RESOLVED] OQ-7 - Do global readers directly call `Place_Production`? -> No; they call `HouseClass::Begin_Building_Placement @ 0x004FB840`.` (evidence: `0x00535DC9`, `0x00535E89`)
- `[RESOLVED] OQ-8 - What command executes `Place_Production`? -> `EventClass::Execute` command `0x0B` at `0x004C710B`.` (evidence: `get_function_xrefs 0x004FB0E0`)
- `[RESOLVED] OQ-9 - What heap id does normal completed-item placement use? -> `-1`, so it is not treated like cancel-one.` (evidence: `StripClass::AI` call to `FUN_004C6AE0`; `SelectClass::Action @ 0x006AB3ED`)
- `[RESOLVED] OQ-10 - Does `CompletedProduction` start the next queued item? -> No; it only clears object/suspends/resets progress/timer/dirty flag.` (evidence: `0x004CA1A0`)
- `[RESOLVED] OQ-11 - Where is queue restart after successful delivery? -> `FUN_004FAA10` calls `StartNextQueued` after `CompletedProduction` when queue count is non-zero.` (evidence: `0x004FAB64..0x004FABB5`)
- `[RESOLVED] OQ-12 - Is restart same tick/command or next sidebar tick? -> Same `Place_Production` command execution after successful delivery.` (evidence: `0x004FB649`, `0x004FAB64`)
- `[RESOLVED] OQ-13 - What does non-naval `heapId >= 0` branch mean? -> cancel/remove-one path; not normal completion because normal completion uses `heapId = -1`.` (evidence: `0x004FAAEB..0x004FAB37`, `0x004C70E1..0x004C710B`)
- `[RESOLVED] OQ-14 - What happens on blocked vehicle exit? -> `Place_Production` returns without `CompletedProduction`/restart for `WhatAmI == 6`.` (evidence: `0x004FB58E..0x004FB5B9`)
- `[RESOLVED] OQ-15 - Are there INI keys for delivery globals? -> No direct keys; only type/factory naval data feeds upstream fields.` (evidence: `ini/rulesmd.ini`, no string/xref in scoped functions)
- `[RESOLVED] OQ-16 - Current Rust delta? -> Rust pops/spawns/refunds on completion; it does not retain a completed vehicle pending successful factory exit.` (evidence: `src/sim/production/production_queue.rs`)
- `[DEFERRED] OQ-17 - Exact command class names for `0x00535DAA/0x00535E6A`.` (category: out-of-scope; reason: names are not needed to verify globals/timing; next-step-if-pursued: map command vtable entries around `0x00535D90` and `0x00535E50`.)
- `[DEFERRED] OQ-18 - Exact war-factory accepted cell algorithm.` (category: out-of-scope; reason: owned by swarm slots 3 and 4; next-step-if-pursued: use those reports for exit-cell tests.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Completed non-vehicle production queues command `0x0B` from `StripClass::AI` with `heapId=-1`; actual placement/restart happens when the command executes. | `0x006A8E05..0x006A8E1E`, `0x004C6AE0`, `0x004C710B` | Rust completes directly in `tick_production` | `src/sim/production/production_queue.rs` | Preserve deterministic command-stage semantics where possible; at minimum, do not treat normal completion as cancel-one by type. | `production_completion_uses_ready_commit_not_cancel_one_heap_id` | Do not remove one queued matching type during normal completion. |
| Successful `Place_Production` calls `CompletedProduction` then `FUN_004FAA10`; `StartNextQueued` runs immediately when queue remains. | `0x004FB649`, `0x004CA1A0`, `0x004FAA10`, `0x004CA5A0` | Rust pop-front makes next item active after completion; broadly similar for success but not command-bound | `production_queue.rs`, future delivery-commit surface | Restart the next queued item only after successful delivery/placement, not when progress merely reaches complete. | `vehicle_queue_restarts_after_successful_factory_exit_same_commit` | Do not start next queued vehicle while the completed one is still blocked in the factory. |
| Blocked vehicle exit returns without `CompletedProduction` and without queue restart. | `0x004FB58E..0x004FB5B9` | Rust refunds and continues if no spawn cell exists | `production_queue.rs`, `production_spawn.rs`, tests in `production_placement_tests.rs` or queue tests | Keep completed vehicle pending and retry/await successful exit; no refund, no pop, no next-item start while exit is blocked. | `blocked_war_factory_exit_keeps_completed_vehicle_and_holds_queue` | Do not refund blocked war-factory output; blocked exit is not failed production. |
| Vehicle delivery globals split land/naval by produced type `+0xE08 == 5`, not by the producing building's stock ID string. | `0x00734250` | Rust uses `rules.object(done_type).naval` for water requirement; no pending globals | `production_queue.rs`, `production_spawn.rs`, possibly new pending-delivery state | Model separate pending land and naval delivery slots or equivalent category split if UI parity requires it. | `completed_naval_unit_uses_naval_pending_delivery_slot` | Do not route naval production through the land war-factory pending slot. |
| Delivery globals are cleared when the exact pending unit is cleared, or both if called with zero. | `0x00734270`, xrefs `0x00685120`, `0x006851F0`, `0x006ABB60` | no equivalent pending state | future delivery state | Clear stale pending delivery if the produced object/factory strip is removed. | `clearing_completed_vehicle_removes_only_matching_pending_delivery_slot` | Do not leave a stale pointer/id visible after cancel/reset/load. |

### Stale Docs / Follow-up Docs

Replace the non-naval restart wording in `BUILD_QUEUE_GHIDRA_REPORT.md` with:

> Normal completed-item `Place_Production` commands carry `heapId = -1`. After a successful delivery/place, `HouseClass::Place_Production` calls `FactoryClass::CompletedProduction` and then `FUN_004FAA10`. Because `heapId < 0`, the non-naval remove-from-queue branch is skipped; if the factory queue is non-empty, `FUN_004FAA10` calls `FactoryClass::StartNextQueued` in the same command execution. The non-naval remove-one branch is for cancel/remove commands with a real heap id, not for normal completion.

Replace the conflicting queue-restart lines in `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` with:

> Queue restart after successful normal delivery is server/command-side, not sidebar-driven. `StripClass::AI` queues command `0x0B` (or for vehicles writes delivery globals); when `EventClass::Execute` later runs `HouseClass::Place_Production`, successful delivery calls `CompletedProduction -> FUN_004FAA10 -> StartNextQueued` immediately if queue count remains. The key discriminator is `heapId = -1` on normal completion commands.

## Negative Facts / Do Not Do

- Do not model `FUN_00734250` as the war-factory exit algorithm; it only writes a pending produced-unit pointer to one of two globals.
- Do not say `StripClass::AI` queues `Place_Production` for vehicles; the scoped binary path writes `DAT_00B0FE5C/60` instead.
- Do not use `QueueingCell`, `DockingOffset`, or refinery dock logic for these vehicle delivery globals.
- Do not remove a queued matching type on normal completion; that branch requires `heapId >= 0`, while normal completion commands use `heapId = -1`.
- Do not refund a vehicle just because the factory exit cell is blocked. Stock behavior keeps completion pending and withholds queue restart until a successful delivery.
- Do not collapse naval and non-naval pending delivery into one global-equivalent slot if UI/action parity is being modeled.

## Remaining Uncertainty

- Exact command-class names for the two global readers were not recovered; only their behavior and xrefs were needed for this slice.
- Exact accepted-cell selection and blocked-exit acceptance codes belong to the sibling war-factory reports in this swarm.
- Runtime command-queue frame latency beyond "not reentrant; executed by `EventClass::Execute`" was not measured with a debugger. Static evidence proves insertion during `StripClass::AI` and restart during later command execution.

## Sources

- Ghidra decompiled/read-only: `0x006A8B30`, `0x00734250`, `0x00734270`, `0x007342A0`, `0x007342B0`, `0x004FB0E0`, `0x004CA1A0`, `0x004FAA10`, `0x004CA5A0`, `0x004C9C60`, `0x004CA130`, `0x004CA160`, `0x004C6AE0`, `0x004C710B`, `0x004FB840`, `0x006AB3CF`, `0x00535DAA`, `0x00535E6A`.
- Prior docs referenced: `BUILD_QUEUE_GHIDRA_REPORT.md`, `FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md`, `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`, `SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md`, `timing/unit-build-time.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/production/production_queue.rs`, `src/sim/production/production_spawn.rs`, `src/sim/production/production_placement.rs`, `src/sim/production/production_tech.rs`, `src/sim/production/production_types.rs`.
