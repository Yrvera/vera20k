# TiberiumClass Save/Load Timer Rehydration - Ghidra Research Report

**Address(es):** `0x00721E80`, `0x00721FA9`, `0x00721FBB`, `0x0046B640`, `0x007221B0`, `0x00722C40`, `0x0067E440`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact `CDTimerClass::Start(0)` call setup in the `TiberiumClass` load slot, which timer object offsets are used, whether timer fields are authoritative after load, and how those timers interact with post-load queue rebuild.  
**Non-Scope:** queue entry/bitmap rebuild internals, growth/spread processor internals, complete OLE save stream object ordering, and runtime frequency of save/load edge cases.  
**Confidence:** High for `ECX`/offset call setup, `Start(0)` writes, driver gate reads, and post-load queue rebuild order. Medium for the semantic name of the middle timer dword because the scoped queue drivers do not read it in their due checks.  
**Active in YR:** Yes for the standard load-game path; conditional for the `TiberiumClass` load slot itself when savegame persistence dispatches `TiberiumClass` objects.

## Working Notes

Target question: Resolve the medium-confidence timer part of `TIBERIUMCLASS_QUEUE_SAVE_LOAD_REBUILD_GHIDRA_REPORT.md`: exact `CDTimerClass::Start(0)` setup in the `TiberiumClass` load slot, offsets used, timer authority after load, and interaction with post-load queue rebuild.

Non-goals: Do not redo queue entry/bitmap rebuild, `AddToGrowthQueue`, map-load seeding, processor batch mechanics, or Rust implementation.

Evidence needed to mark COMPLETE: Disassembly evidence for both `CDTimerClass::Start(0)` call sites, `CDTimerClass::Start` writes, spread/growth driver due-check reads, path liveness through vtable/save-load and standard load-game rebuild, and handoff implications for Rust timer/snapshot state.

Stop conditions: Stop once the two timer offsets, overwritten fields, first post-load firing condition, and rebuild ordering are proven or tightly bounded.

## Summary

The prior uncertainty is resolved: the `TiberiumClass` load slot restarts the spread timer at `this+0x100` and the growth timer at `this+0x11C` by calling `CDTimerClass::Start(0)` twice after `AbstractClass::Load`. The call setup is explicit in assembly:

- `0x00721FA9`: `PUSH 0`; `0x00721FAA`: `LEA ECX,[ESI+0x100]`; `0x00721FB6`: `CALL 0x0046B640`.
- `0x00721FBB`: `PUSH 0`; `0x00721FBC`: `LEA ECX,[ESI+0x11C]`; `0x00721FC2`: `CALL 0x0046B640`.

`CDTimerClass::Start @ 0x0046B640` writes only two dwords: `[ECX+0] = g_CurrentFrameCounter` and `[ECX+8] = argument`. Therefore load writes:

- spread timer start `+0x100 = g_CurrentFrameCounter`, spread interval/duration `+0x108 = 0`;
- growth timer start `+0x11C = g_CurrentFrameCounter`, growth interval/duration `+0x124 = 0`.

The middle dwords `+0x104` and `+0x120` are not written by `Start(0)`. In the scoped queue drivers, they are not read by the due-check logic; the drivers later overwrite them when reloading timers after a processor call. For queue firing parity, the authoritative post-load timer state is therefore start=current frame and interval=0, not any saved raw interval/duration.

Post-load queue rebuild still happens after content load through `FUN_0067E440`, which calls `InitGrowthQueues_All @ 0x00722D00` then `InitSpreadQueues_All @ 0x00722240`. Those rebuild queue entries/bitmaps but do not reset timer fields. Since the load slot has already set both queue timer intervals to zero, the first eligible live queue driver pass after load fires immediately, then reloads the real per-type intervals.

## Timer Layout for This Slice

| Offset | Verified role in scoped code | Load-slot write | Driver read/write | Active in YR |
|---:|---|---|---|---|
| `+0x100` | Spread timer start frame | `CDTimerClass::Start(0)` writes `g_CurrentFrameCounter` | Spread driver reads for elapsed test; writes current frame after processing | Yes |
| `+0x104` | Spread timer middle/cached dword | Not written by `Start(0)` | Spread driver writes caller-local value after processing; not used by due-check | Yes, but not authoritative for due-check |
| `+0x108` | Spread timer interval/duration | `CDTimerClass::Start(0)` writes `0` | Spread driver reads due interval; writes `TiberiumClass+0x9C` after processing | Yes |
| `+0x11C` | Growth timer start frame | `CDTimerClass::Start(0)` writes `g_CurrentFrameCounter` | Growth driver reads for elapsed test; writes current frame after processing | Yes |
| `+0x120` | Growth timer middle/cached dword | Not written by `Start(0)` | Growth driver writes caller-local value after processing; not used by due-check | Yes, but not authoritative for due-check |
| `+0x124` | Growth timer interval/duration | `CDTimerClass::Start(0)` writes `0` | Growth driver reads due interval; writes computed `ftol(Growth * multiplier)` after processing | Yes |

## Verified Binary Evidence

1. **The TiberiumClass load slot explicitly restarts the spread timer at `this+0x100`.**  
   Evidence: `0x00721FA9 PUSH 0`, `0x00721FAA LEA ECX,[ESI+0x100]`, `0x00721FB6 CALL 0x0046B640`. This is after `AbstractClass::Load @ 0x00410380` at `0x00721F83`. Active in YR: Conditional through the `TiberiumClass` persistence load vtable entry at data xref `0x007F573C`.

2. **The TiberiumClass load slot explicitly restarts the growth timer at `this+0x11C`.**  
   Evidence: `0x00721FBB PUSH 0`, `0x00721FBC LEA ECX,[ESI+0x11C]`, `0x00721FC2 CALL 0x0046B640`. This is in the same reconstruction block after raw load and before vtable reinstall writes at `0x00721FC7..0x00721FDB`. Active in YR: Conditional through the same load slot.

3. **`CDTimerClass::Start(0)` overwrites start frame and interval, not the middle dword.**  
   Evidence: `0x0046B640 MOV EDX,[ESP+4]`; `0x0046B646 MOV ECX,[0x00A8ED84]`; `0x0046B64C MOV [EAX],ECX`; `0x0046B64E MOV [EAX+8],EDX`; `RET 4`. Decompiler confirms `*timer = g_CurrentFrameCounter; timer[2] = arg`. Active in YR: Yes; directly called by the load slot.

4. **Spread and growth due checks use only start and interval, so `Start(0)` makes each timer due on the next eligible driver pass.**  
   Evidence: Spread driver `0x007221DD..0x00722200` reads `+0x100` into `EDI`, `+0x108` into `EAX`, computes `currentFrame - start`, and falls through to process when interval is zero. Growth driver `0x00722C76..0x00722C99` does the same with `+0x11C/+0x124`. Neither due-check reads `+0x104` or `+0x120`. Active in YR: Yes, called from `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.

5. **Post-load queue rebuild is membership-only for this slice; it follows content load and does not reset timer fields.**  
   Evidence: `FUN_0067E440` reaches post-content calls `0x0067E6AE -> InitGrowthQueues_All @ 0x00722D00` and `0x0067E6B3 -> InitSpreadQueues_All @ 0x00722240`; xrefs also show these init functions are called from scenario init and random map generation. Prior disassembly of the init functions shows allocation/bitmap/heap rebuild work, while the timer writes for load are at `0x00721FA9/0x00721FBB` and driver reloads at `0x00722222..0x00722227` and `0x00722CDE..0x00722CE3`. Active in YR: Yes for standard load-game path.

## Mechanism Details

### Load Slot Ordering

The load function body starts at `0x00721E80`; the previously cited `0x00721F70` lies inside the same function near the pre-load dynamic queue cleanup. The relevant order is:

1. Free/clear existing spread dynamic fields around `+0xF0..+0xFC`.
2. Free/clear existing growth dynamic fields around `+0x10C..+0x118`.
3. Call a virtual cleanup on `this+0xC4`, then `AbstractClass::Load(this, stream)`.
4. Reconstruct type/vector scaffolding.
5. Call `CDTimerClass::Start(0)` for `this+0x100`.
6. Call `CDTimerClass::Start(0)` for `this+0x11C`.
7. Reinstall `TiberiumClass` vtables.
8. Read extra vector data/swizzle references.
9. Zero dynamic growth/spread pointer fields again.

Tiny detail: the start calls occur after raw load, so any saved/raw bytes for `+0x100/+0x108/+0x11C/+0x124` are overwritten by constructor-like timer state.

### First Post-Load Queue Fire

The spread driver reads `+0x100/+0x108`. If start is not `-1`, it computes elapsed frames. If elapsed is below interval, it subtracts elapsed from interval and only skips when the remaining interval is nonzero. With interval `0`, the `TEST EAX,EAX` branch does not skip, so `SpreadProcessor @ 0x00722440` runs.

The growth driver has the same due-check shape for `+0x11C/+0x124`. With interval `0`, `GrowthProcessor @ 0x00722F00` runs on the next eligible driver pass.

Both drivers then reload the timers:

- Spread writes `+0x100 = g_CurrentFrameCounter`, `+0x104 = caller-local`, `+0x108 = TiberiumClass+0x9C`.
- Growth writes `+0x11C = g_CurrentFrameCounter`, `+0x120 = caller-local`, `+0x124 = ftol(TiberiumClass+0xA8 * multiplier)`.

The queue processors can still internally exit due to empty heaps or zero percentages. Timer due-ness only controls whether the processor is called.

### Constructor Consistency

The regular constructor initializes the same first-fire shape: at `0x0072176A..0x00721776`, spread start `+0x100` is set to `g_CurrentFrameCounter` and spread interval `+0x108` to zero; at `0x00721794..0x007217A0`, growth interval `+0x124` is set to zero and growth start `+0x11C` to `g_CurrentFrameCounter`. The load slot’s two `Start(0)` calls recreate that constructor-like timer state for saved objects.

## Current Rust Delta

Current Rust still stores `OreGrowthState` as serde state and hashes it via `world_hash.rs`, while live behavior remains scan/reservoir-shaped plus partial queue vectors. Once native per-type queues land, Rust should treat queue membership as rebuilt from visible cells after load, while timer state should be modeled separately as the native constructor/load state: start at load/current frame and interval zero, followed by driver reload after the immediate eligible processor call.

The important distinction is that native load does not restore old queue timer remaining time for the scoped growth/spread drivers. It overwrites both interval fields with zero after raw load.

## Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Spread timer load offset | verified | `0x00721FAA LEA ECX,[ESI+0x100]` | none |
| Growth timer load offset | verified | `0x00721FBC LEA ECX,[ESI+0x11C]` | none |
| `Start(0)` field writes | verified | `0x0046B640` assembly | none |
| Driver due-check fields | verified | `0x007221DD..0x00722200`, `0x00722C76..0x00722C99` | none |
| Middle timer dword semantics | bounded | drivers write but do not read for due-check | semantic name deferred |
| Load-game queue rebuild order | verified-context | `0x0067E6AE`, `0x0067E6B3`, xrefs | queue internals already covered elsewhere |
| Runtime exact first frame after UI/load handoff | deferred | not needed for field-offset proof | runtime debugger would be needed for user-facing frame number |

## Open Questions - Final State

- `[RESOLVED] OQ-01 - Which object offset is passed to the first Start(0)? -> this+0x100, by LEA ECX,[ESI+0x100] at 0x00721FAA.`
- `[RESOLVED] OQ-02 - Which object offset is passed to the second Start(0)? -> this+0x11C, by LEA ECX,[ESI+0x11C] at 0x00721FBC.`
- `[RESOLVED] OQ-03 - What does Start(0) write? -> start=current frame and interval=0; it does not write the middle dword.`
- `[RESOLVED] OQ-04 - Are saved timer start/interval fields authoritative after load? -> No for +0/+8 fields in each scoped timer; the load slot overwrites them after raw load.`
- `[RESOLVED] OQ-05 - Does queue init reset timers? -> No evidence of timer writes in queue init; load slot and drivers own timer writes for this slice.`
- `[RESOLVED] OQ-06 - Does zero interval make the next driver pass due? -> Yes; spread/growth due checks skip only when remaining interval is nonzero.`
- `[DEFERRED] OQ-07 - What is the semantic name of the middle dword +0x104/+0x120? -> Deferred as not queue-cadence-critical; scoped drivers write it but do not read it in due checks.`
- `[DEFERRED] OQ-08 - What exact user-visible frame does the first post-load queue call land on? -> Deferred to runtime/load-flow trace; static evidence proves first eligible driver pass, not UI transition frame count.`

## Implementation Handoff

| Verified behavior | Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk |
|---|---|---|---|---|---|
| Load restart writes spread `start=current_frame`, `interval=0` at `+0x100/+0x108` and growth `start=current_frame`, `interval=0` at `+0x11C/+0x124`. | Future native queue timers do not exist; current scanner state serializes. | `src/sim/ore_growth.rs`, `src/sim/snapshot.rs`, snapshot load/rebuild path | After snapshot/load, construct per-type queue timers in constructor/load shape rather than preserving old remaining intervals. | `tiberium_queue_timers_restart_zero_interval_after_snapshot_load` | Do not deserialize old queue timer remaining time as native parity state. |
| Queue membership rebuild after load is separate from timer restart; init rebuilds entries/bitmaps but does not reset timer fields. | Rust needs both rebuild-from-cells and a separate timer model. | `OreGrowthState`, `ProductionState`, `world_hash.rs`, app load cache rebuild path | Rebuild queue membership from restored visible ore cells, then keep timer state as load-restarted zero-interval timers. | `ore_growth_snapshot_load_rebuilds_membership_but_restarts_timers` | Do not hide timer reset inside queue membership rebuild in a way that prevents testing each side separately. |
| First eligible post-load growth/spread driver pass is due because interval is zero; processors may still no-op due to empty heaps or zero percentages. | Rust old scan cadence may wait a full interval or scan cursor cycle. | `Simulation::advance_tick`, `ore_growth` queue driver | The first post-load eligible logic tick should call growth then spread drivers and only then reload per-type intervals. | `ore_growth_snapshot_load_first_eligible_tick_processes_before_interval_reload` | Do not delay the first post-load queue processing by `Growth`/`Spread` interval. |

## Negative Facts / Do Not Do

- Do not preserve native saved `+0x100/+0x108/+0x11C/+0x124` timer values as authoritative after load; `0x00721FA9..0x00721FC2` overwrites them after `AbstractClass::Load`.
- Do not claim the load slot calls `Start(0)` on ambiguous inferred offsets; the exact offsets are `this+0x100` and `this+0x11C`.
- Do not treat the middle dwords `+0x104/+0x120` as part of the queue driver due-check unless another consumer is proven; the scoped spread/growth drivers check only start and interval.
- Do not make queue init/rebuild responsible for timer reset; timer reset is in the load slot, while queue init/rebuild owns allocation/membership.
- Do not delay the first post-load queue processing by the type interval; the load-restored interval is zero until the first driver reload.

## Remaining Uncertainty

- Exact semantic name/purpose of timer middle dwords `+0x104` and `+0x120` remains deferred. The scoped queue drivers write them after processing but do not read them for due checks.
- Exact user-visible frame of the first post-load queue processor call remains deferred to a runtime trace because static evidence proves the first eligible driver pass, not frontend/load transition frame count.

## Stale Docs / Replacement Wording

`docs/research/TIBERIUMCLASS_QUEUE_SAVE_LOAD_REBUILD_GHIDRA_REPORT.md`

Replace the medium-confidence timer caveat with:

> Resolved by `TIBERIUMCLASS_SAVE_LOAD_TIMER_REHYDRATION_GHIDRA_REPORT.md`: the `TiberiumClass` load slot calls `CDTimerClass::Start(0)` with `ECX=this+0x100` for spread and `ECX=this+0x11C` for growth. `Start(0)` writes start frame `g_CurrentFrameCounter` and interval `0`, overwriting raw-loaded start/interval fields. Queue init/rebuild after content load does not reset these timers; therefore the first eligible post-load queue driver pass is due immediately and then reloads the real per-type interval.

`docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`

Replace the save/load uncertainty sentence with:

> Save/load timer rehydration is resolved: queue membership is rebuilt after content load, while the load slot restarts spread/growth timers at `this+0x100` and `this+0x11C` with zero interval, making the next eligible growth/spread driver pass due immediately.

## Sources

- Ghidra disassembly/decompile: `0x00721E80`, `0x00721FA9..0x00721FC2`, `0x0046B640`, `0x007221B0`, `0x00722C40`, `0x0072176A..0x007217A0`, `0x0067E440`.
- Ghidra xrefs: `0x00721F70` data xref `0x007F573C`; `CDTimerClass::Start @ 0x0046B640` callers include `0x00721FB6` and `0x00721FC2`; queue init xrefs include `0x0067E6AE` and `0x0067E6B3`.
- Prior reports: `TIBERIUMCLASS_QUEUE_SAVE_LOAD_REBUILD_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBERIUMCLASS_MAP_LOAD_QUEUE_SEEDING_GHIDRA_REPORT.md`.

