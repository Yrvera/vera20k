# TiberiumClass SpreadProcessor Audit - Ghidra Research Report

**Address(es):** `0x00722440` primary, `0x007221B0`, `0x00483780`, `0x00483690`, `0x004838E0`, `0x007228B0`, `0x00722AF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Exact `TiberiumClass::SpreadProcessor @ 0x00722440` queue-processing semantics: gates, batch math, pop loop, stale/source-invalid behavior, valid-target count, `SpreadTiberium(0)` call, reinsertion, bitmap writes, RNG consumption, and priority treatment.  
**Non-Scope:** Full `CanPlaceTiberium` matrix beyond target-count use, full spread driver timer multiplier proof, save/load behavior, heap helper internals beyond visible pop/insert effects, and Rust implementation patches.  
**Confidence:** High for the claimed slice.  
**Active in YR:** Yes, conditional on scenario tiberium-spread gate and per-type `SpreadPercentage > 0.0`; stock `[Riparius] SpreadPercentage=.06` is active, stock `[Cruentus] SpreadPercentage=0` exits.

## 0. Working Notes

Target question: Re-audit `TiberiumClass::SpreadProcessor @ 0x00722440` with growth-processor-level rigor before replacing Rust's scan/reservoir ore spread model.

Non-goals: Do not redo `CanPlaceTiberium` full matrix, do not implement Rust, do not update shared swarm claims, and do not broaden into save/load or map-load seeding.

Evidence needed to mark COMPLETE: decompile plus assembly-context evidence for entry gates, batch clamp, RNG budget, heap pop, valid-neighbor count, `SpreadTiberium(0)`, reinsertion condition, bitmap clear/set, priority handling, and YR liveness through the tick driver.

Stop conditions: Stop after one zero-add pass over `0x00722440`, its direct tick driver, and direct callees needed for source/target validation; defer heap helper internals and timer multiplier fields to their own reports.

## 1. Overview

`TiberiumClass::SpreadProcessor` is the live per-tiberium-type spread-queue processor. It is not a map scan. When the per-type spread driver matures, it computes a random loop budget from heap count and `SpreadPercentage`, pops heap entries ordered by stored float priority, pre-counts target cells that can receive new tiberium, then calls `CellClass::SpreadTiberium(force=false)` only for popped sources with at least one valid target.

The important correction is that the random budget is not a raw heap-pop budget. Heap entries with zero valid targets are popped, have this type's spread bitmap cleared, and do not advance the processed counter. Entries with at least one valid target call `SpreadTiberium(0)` and advance the counter regardless of the callee return value.

## 2. Key Offsets

| Owner | Offset | Meaning in this slice | Evidence |
|---|---:|---|---|
| `TiberiumClass` | `+0x98` | Type index used by rebuild filters | `0x007228B0` decompile |
| `TiberiumClass` | `+0x9C` | Spread driver interval reloaded after processor | `0x007221B0` decompile |
| `TiberiumClass` | `+0xA0` | `SpreadPercentage` double | `0x00722469..0x0072248A` assembly context |
| `TiberiumClass` | `+0xF0` | Spread entry append/count cursor | `0x00722586..0x00722632` assembly context |
| `TiberiumClass` | `+0xF4` | Spread heap pointer; heap count at `[heap+0]` | `0x0072244F..0x00722463` assembly context |
| `TiberiumClass` | `+0xF8` | Spread bitmap base | `0x00722634..0x00722653` assembly context |
| `TiberiumClass` | `+0xFC` | Spread entry array, entries are `{cell, float_priority}` | `0x00722586..0x00722632` assembly context |
| `TiberiumClass` | `+0x100/+0x108` | Spread timer start/interval fields used by driver | `0x007221B0` decompile |

## 3. Core Logic

Entry gates, in order:

1. Read spread heap pointer from `this+0xF4`; return if null.
2. Read heap count from `*heap`; return if zero.
3. Compare `SpreadPercentage` at `this+0xA0` against `0.0`; return when `<= 0.0` or unordered by the x87 status test.
4. Compute `ftol(heap_count * SpreadPercentage)`.
5. Clamp that result to `[5, 25]`.
6. Consume one raw `Random::Next`, signed-absolute it with `CDQ; XOR; SUB`, then compute `actual_budget = abs(raw) % clamped_batch + 1`.
7. If `map_cell_count - 20 < heap_count`, rebuild the spread queue before popping.

The first heap entry is popped before the main loop. Subsequent entries are popped at the loop tail only if the processed counter is still below `actual_budget`. There is no comparison between entry priority and `g_CurrentFrameCounter`; priority is only the heap ordering key.

For each popped entry:

1. If popped entry pointer/cell is zero, return.
2. Resolve the source cell from the popped stored map coordinate.
3. Count all eight adjacent target cells in deterministic direction order `0..7`.
4. For each target, call the neighbor-coordinate helper and then `CellClass::CanPlaceTiberium` on that target cell.
5. If the count is zero, clear this tiberium type's spread bitmap byte for the source cell.
6. If the count is positive, call `CellClass::SpreadTiberium(0)` on the source cell, increment the processed counter, and ignore the return value.
7. If the pre-count is greater than one, append a new spread entry for the same source coordinate with priority `0.0`, insert it into this heap, and set this tiberium type's spread bitmap byte for the source cell to `1`.
8. If the pre-count is exactly one, the source is not reinserted and no bitmap clear occurs in this function.

This means stale/source-invalid entries are not prefiltered by current source tiberium type before the neighbor-count phase. A popped source whose current overlay is gone or no longer spread-capable can still consume the processor's successful-source budget if at least one adjacent target passes `CanPlaceTiberium`; `SpreadTiberium(0)` then performs its own source preflight and may return false before consuming its inner direction RNG, but the processor does not inspect that return.

## 4. RNG Consumption

| Point | RNG consumed? | Details | Evidence |
|---|---:|---|---|
| Entry gates fail | No | Null heap, zero heap count, or `SpreadPercentage <= 0.0` exits before random. | `0x0072244F..0x0072247A` |
| Processor budget | Yes, one raw word | `Random::Next`, signed absolute, `% batch + 1`. | `0x007224AE..0x007224D1` |
| Queue rebuild | No | Rebuild seeds priority `0.0`. | `0x007228B0` |
| No-valid-target popped entry | No extra RNG | Clears bitmap and continues. | `0x00722563..0x00722657` |
| Valid-target source | Conditional inner RNG | Calls `SpreadTiberium(0)`; the callee consumes `RandomRanged(0,7)` only after its non-force source preflight passes. | `0x0072256B..0x0072256F`, `0x00483780` |
| Processor reinsertion | No | Reinsert priority is literal `0.0`, not random jitter. | `0x00722598..0x0072263F` |
| Runtime `AddToSpreadQueue` | Yes | Separate helper stores `currentFrame + abs(Random::Next()) % 50`. | `0x00722AF0` |

## 5. Integration Points

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40`, then `TiberiumClass::SpreadDriver_AllTypes @ 0x007221B0`, before later bomb/laser/lightning/tactical/factory/house updates. This proves the spread processor is on the standard live tick path.

`SpreadDriver_AllTypes @ 0x007221B0` is gated by `ScenarioClass+0x34A6`, iterates `g_TiberiumClass_Array`, checks the per-type spread timer at `+0x100/+0x108`, calls `SpreadProcessor`, then reloads start/current interval from `g_CurrentFrameCounter` and `TiberiumClass+0x9C`.

`CellClass::CanSpreadTiberium @ 0x00483690` is not called by the processor on popped entries. It is used by rebuild and `AddToSpreadQueue`; the processor relies on the popped queue membership and `SpreadTiberium(0)` source preflight instead.

`CellClass::SpreadTiberium @ 0x00483780` is called with stack argument `0`, meaning normal/non-forced spread. Its non-force preflight checks `TiberiumSpreads`, source overlay-to-tiberium mapping, source density threshold, flat slope, positive `SpreadPercentage`, and no object list before it consumes direction RNG.

## 6. Current Rust Implementation Status

Current Rust still uses a scan/reservoir model in `src/sim/ore_growth.rs`: `tick_ore_growth` scans map cells, collects growth/spread candidates, and calls `try_spread_ore` at full scan completion. This is not the native YR queue processor.

Rust has partial queue-shaped fields (`OreSpreadQueueEntry`, `spread_queue`, `spread_membership`) and explicit enqueue/reseed helpers, but the live processor is not yet native. The Rust `spread_membership` is a `BTreeSet` guard, while native spread membership is a bitmap plus heap/entry-array state with the exact stale/one-neighbor behaviors above.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SpreadProcessor` entry gates | verified | decompile `0x00722440`; assembly `0x0072244F..0x0072247A` | none |
| Batch math and clamp | verified | assembly `0x00722480..0x007224A5` | none |
| Budget RNG and signed abs/mod | verified | assembly `0x007224AE..0x007224D1` | none |
| Capacity rebuild trigger | verified | assembly `0x007224D5..0x007224E3`; decompile `0x007228B0` | none |
| Heap pop and no priority wake-up | verified | decompile `0x00722440`; heap pop contexts `0x0072250E..0x00722528`, `0x00722667..0x00722756` | heap helper internals not separately named |
| Valid-target count | verified | assembly `0x00722547..0x00722565`; decompile `0x004838E0` | full target gate matrix out-of-scope |
| `SpreadTiberium(0)` call | verified | assembly `0x0072256B..0x0072256F`; decompile `0x00483780` | none |
| Reinsertion condition and priority | verified | assembly `0x00722579..0x0072263F` | none |
| Bitmap clear/set | verified | assembly `0x00722634..0x00722653` | none |
| Driver liveness | verified | decompile `0x0055AFB0`, `0x007221B0`; assembly `0x0055B4D7..0x0055B4DC`, `0x00722200` | timer multiplier out-of-scope |
| Current Rust delta | verified | `src/sim/ore_growth.rs` scan/reservoir and partial queue fields | no patch in this report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x00722440` active in standard YR? -> Yes via `LogicClassPerTickUpdateLiveVector -> GrowthDriver_AllTypes -> SpreadDriver_AllTypes -> SpreadProcessor`, conditional on scenario gate and per-type timer.` (evidence: `0x0055AFB0`, `0x007221B0`)
- `[RESOLVED] OQ-02 - What exits before RNG? -> Null heap pointer, zero heap count, and `SpreadPercentage <= 0.0`/unordered return before the budget RNG call.` (evidence: `0x0072244F..0x0072247A`)
- `[RESOLVED] OQ-03 - What is the batch formula? -> `ftol(heap_count * SpreadPercentage)`, clamped to `[5,25]`, then `abs(Random::Next()) % batch + 1`.` (evidence: `0x00722480..0x007224D1`)
- `[RESOLVED] OQ-04 - Is the random budget a heap-pop budget? -> No; zero-target popped entries do not increment the processed counter.` (evidence: `0x00722563..0x00722657`)
- `[RESOLVED] OQ-05 - Does priority delay by frame? -> No current-frame comparison exists after pop; priority orders heap entries only.` (evidence: `0x00722440` decompile, heap pop contexts)
- `[RESOLVED] OQ-06 - Does processor check source current tiberium type before neighbor count? -> No; it resolves the source cell, counts target cells, then delegates source preflight to `SpreadTiberium(0)`.` (evidence: `0x00722536..0x0072256F`, `0x00483780`)
- `[RESOLVED] OQ-07 - Is `SpreadTiberium` return used? -> No; processed counter increments after the call and before any return-value test, and no return test appears.` (evidence: `0x0072256F..0x00722580`)
- `[RESOLVED] OQ-08 - When does bitmap clear? -> Only the zero-valid-target branch clears this type's bitmap for the source cell.` (evidence: `0x00722645..0x00722653`)
- `[RESOLVED] OQ-09 - What happens with exactly one valid target? -> Calls `SpreadTiberium(0)`, increments processed counter, does not reinsert, and does not clear the bitmap in this function.` (evidence: `0x00722579..0x00722580`)
- `[RESOLVED] OQ-10 - What priority does processor reinsertion use? -> Literal float `0.0`.` (evidence: `0x00722598..0x007225A8`; decompile `0x00722440`)
- `[RESOLVED] OQ-11 - Does processor reinsertion consume RNG? -> No; only `AddToSpreadQueue` runtime insertion uses random jitter.` (evidence: `0x00722440`, `0x00722AF0`)
- `[RESOLVED] OQ-12 - What does stock data do? -> Stock Riparius has `SpreadPercentage=.06`, Cruentus has `0`; Riparius can run, Cruentus exits at percentage gate.` (evidence: `ini/rulesmd.ini [Riparius]`, `[Cruentus]`; `0x00722469..0x0072247A`)
- `[DEFERRED] OQ-13 - What are exact driver multiplier semantics?` (category: out-of-scope; reason: this slot targets processor audit, not timer report; next-step-if-pursued: use timer certainty-pass slot)
- `[DEFERRED] OQ-14 - What are named heap helper fields beyond visible count/capacity/pointer effects?` (category: bounded-cost-too-high; reason: visible ordering/insert/pop effects are enough for Rust handoff; next-step-if-pursued: isolate heap class helpers)

Adversarial corner cases answered: empty heap exits before RNG; zero-percent tiberium exits before RNG; zero-target stale entry clears bitmap without spending budget; one-target source leaves bitmap set and does not reinsert; source-invalid but target-valid entry can spend processor budget while `SpreadTiberium(0)` return is ignored.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Spread processor uses per-type heap/bitmap state and a budget of processed valid-target sources, not a map scan. | `0x00722440`; assembly `0x0072244F..0x00722580` | Rust still scans/reservoir-samples in `tick_ore_growth`. | `src/sim/ore_growth.rs`, `ProductionState`, `world_hash.rs` | Replace live spread execution with native queue pop/count/call/reinsert semantics. | `spread_processor_zero_target_entries_do_not_consume_budget` with heap containing zero-target then valid-target entries. | Do not keep RA1 scan/reservoir timing as YR parity. |
| Processor reinserts only when pre-counted valid targets are `> 1`, uses priority `0.0`, and leaves the bitmap unchanged for exactly one valid target. | `0x00722579..0x0072263F`; bitmap clear `0x00722645..0x00722653` | Rust `BTreeSet` membership cannot represent this exact one-target stale-bit state. | Future per-type spread bitmap and heap model in `src/sim/ore_growth.rs` | Model bitmap separately from heap entries and preserve the one-target no-reinsert/no-clear behavior. | `spread_processor_one_valid_neighbor_leaves_bitmap_set_without_heap_entry`. | Do not make spread membership a pure `BTreeSet` of live heap entries. |
| Stale/source-invalid entries are not source-type-prefiltered by the processor; target-valid entries call `SpreadTiberium(0)`, ignore its return, and may consume budget/reinsert based on target count. | `0x00722536..0x00722580`; `0x00483780` | Rust currently validates source and target in a high-level helper before spread. | `src/sim/ore_growth.rs`, `src/sim/tiberium/mod.rs` | Preserve processor-vs-callee split: target pre-count first, then source preflight inside `SpreadTiberium(0)`, ignored return. | `spread_processor_source_removed_but_targets_valid_consumes_budget_like_gamemd`. | Do not eagerly drop stale queue entries by checking current source resource before budget accounting. |

## 10. Stale Docs / Follow-Up Docs

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md` section 3.5 should replace "processes until actual successful spreads reach the chosen count" with: "processes until the count of popped entries that had at least one valid target and therefore called `SpreadTiberium(0)` reaches the chosen count, or the heap empties. Zero-target entries are popped and clear this type's bitmap without spending the count. The return value from `SpreadTiberium(0)` is ignored."
- Same section should add: "For exactly one valid target, the processor calls `SpreadTiberium(0)` but neither reinserts nor clears this type's spread bitmap. For more than one valid target, it reinserts the source with priority `0.0` and sets the bitmap to `1`."
- Same section should add: "The processor does not pre-check that the popped source still maps to this `TiberiumClass`; stale/source-invalid entries can still call `SpreadTiberium(0)` if adjacent targets pass `CanPlaceTiberium`, and the processor bases reinsertion on the pre-counted target count rather than the callee return."

## Sources

- Ghidra read-only decompile: `TiberiumClass::SpreadProcessor @ 0x00722440`
- Ghidra read-only decompile: `TiberiumClass::SpreadDriver_AllTypes @ 0x007221B0`
- Ghidra read-only decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`
- Ghidra read-only decompile: `CellClass::SpreadTiberium @ 0x00483780`
- Ghidra read-only decompile: `CellClass::CanSpreadTiberium @ 0x00483690`
- Ghidra read-only decompile: `CellClass::CanPlaceTiberium @ 0x004838E0`
- Ghidra read-only decompile: `TiberiumClass::RebuildSpreadQueue @ 0x007228B0`
- Ghidra read-only decompile: `TiberiumClass::AddToSpreadQueue @ 0x00722AF0`
- Ghidra assembly contexts: `0x0055B4D7..0x0055B4DC`, `0x00722200`, `0x0072244F..0x00722756`
- INI defaults: `ini/rulesmd.ini [Riparius]`, `[Cruentus]`
- Current Rust scan: `src/sim/ore_growth.rs`
