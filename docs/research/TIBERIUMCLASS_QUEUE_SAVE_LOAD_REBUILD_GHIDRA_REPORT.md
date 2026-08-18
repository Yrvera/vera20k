# TiberiumClass Queue Save/Load/Rebuild - Ghidra Research Report

**Address(es):** `0x00721F70`, `0x007220D0`, `0x0067E440`, `0x00722D00`, `0x00722240`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** native save/load or post-load rebuild behavior for `TiberiumClass` growth/spread queue entries, membership bitmaps, heap objects, and timer rehydration.
**Non-Scope:** growth/spread processor internals, duplicate `AddToGrowthQueue` caller proof, exact savegame stream section inventory beyond the `TiberiumClass` save/load slots and the load-game post-content rebuild path.
**Confidence:** High for dynamic queue pointer/count sanitization and post-load queue rebuild; Medium for exact timer subobject offset naming because the decompiler elides `ECX` on the two `CDTimerClass::Start(0)` calls.
**Active in YR:** Yes for the standard load-game path; conditional for individual `TiberiumClass` save/load slots when the save stream dispatches a `TiberiumClass` object through COM/OLE persistence.

## Working Notes

Target question: Determine whether runtime growth/spread queue entries, bitmaps, and timers are serialized directly, rebuilt after scenario/load, or derived by init/rebuild calls.
Non-goals: Do not re-investigate GrowthProcessor internals, AddToGrowthQueue duplicate callers, or TIBTRE placement facts already settled.
Evidence needed to mark COMPLETE: Fresh Ghidra evidence for `TiberiumClass` save/load slots, queue field treatment during load, post-load queue init xrefs, and Rust save/hash implications.
Stop conditions: Stop once queue entry/bitmap/heap rebuild and timer treatment are bounded enough for Rust snapshot/world-hash handoff.

## 1. Overview

Native `TiberiumClass` queue arrays and bitmaps are not authoritative save payload that should be replayed after load. The `TiberiumClass` load slot sanitizes the dynamic spread/growth queue heap, entry-array, bitmap, and count fields, then the load-game entry `FUN_0067E440` unconditionally calls `TiberiumClass::InitGrowthQueues_All` followed by `TiberiumClass::InitSpreadQueues_All` after the saved content stream has been loaded.

For Rust, this means the native save/load model is rebuild-from-loaded-cell-state for queue membership and entry arrays, not "restore exact old heap entries." Queue timers are rehydrated through the class load path and normal driver writes, not through the queue init functions themselves.

## 2. Class Layout / Key Offsets

| Offset | Field | Load behavior | Active in YR |
|---:|---|---|---|
| `+0xF0` | Spread entry count | Set to `0` before/after load sanitation. Evidence: `0x00721F70` writes `puVar2[0x3C] = 0`. | Conditional through `TiberiumClass` load slot. |
| `+0xF4` | Spread heap object pointer | Freed/sanitized before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x3D]`. | Conditional. |
| `+0xF8` | Spread bitmap pointer | Freed before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x3E]`. | Conditional. |
| `+0xFC` | Spread entry array pointer | Freed before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x3F]`. | Conditional. |
| `+0x100/+0x104/+0x108` | Spread timer object | Re-started by a `CDTimerClass::Start(0)` call in the load reconstruction block; driver later writes current frame and interval from `+0x9C`. Evidence: `0x00721F70`, `0x0046B640`, `0x007221B0`. | Conditional. |
| `+0x10C` | Growth entry count | Set to `0` before/after load sanitation. Evidence: `0x00721F70` writes `puVar2[0x43] = 0`. | Conditional. |
| `+0x110` | Growth heap object pointer | Freed/sanitized before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x44]`. | Conditional. |
| `+0x114` | Growth bitmap pointer | Freed before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x45]`. | Conditional. |
| `+0x118` | Growth entry array pointer | Freed before load if present; set to `0` after load. Evidence: `0x00721F70` handles `puVar2[0x46]`. | Conditional. |
| `+0x11C/+0x120/+0x124` | Growth timer object | Re-started by the second `CDTimerClass::Start(0)` call in the load reconstruction block; driver later writes current frame and computed interval. Evidence: `0x00721F70`, `0x0046B640`, `0x00722C40`. | Conditional. |

## 3. Core Logic

### 3.1 TiberiumClass save slot

`FUN_007220D0` is the `TiberiumClass` save slot. It first calls `AbstractClass::Save @ 0x00410320`, whose mechanism writes the object pointer and then writes a raw object body of size returned by vtable slot `+0x30`. After that, `FUN_007220D0` writes the debris vector state at `+0xD4/+0xC8`.

Active in YR: Conditional. Evidence: `get_function_xrefs 0x007220D0` returns data xref `0x007F5740`, consistent with a vtable slot; `get_function_callees 0x007220D0` shows `AbstractClass::Save`.

Load-bearing detail: even if raw object bytes include stale native pointer values, the matching `TiberiumClass` load slot does not trust them for dynamic queues.

### 3.2 TiberiumClass load slot

`TiberiumClass` load at `0x00721F70` performs three relevant phases:

1. Before the raw load, it frees any currently live spread/growth heap, entry-array, and bitmap allocations and clears their fields.
2. It calls `AbstractClass::Load @ 0x00410380`, then reconstructs type-class scaffolding: `AbstractTypeClass::Constructor`, `DynamicVectorClass::Constructor`, two `CDTimerClass::Start(0)` calls, and reinstalls the `TiberiumClass` vtables.
3. After reading extra vector data and registering swizzles, it explicitly writes the dynamic queue fields back to zero: growth `+0x110/+0x114/+0x118`, spread `+0xF4/+0xF8/+0xFC`, and their counts.

Active in YR: Conditional. Evidence: `get_function_xrefs 0x00410380` includes `0x00721F83 in TiberiumClass__Constructor`, and decompiling `0x00721F70` shows the full load body. The function is installed via `TiberiumClass` vtable data, so it is live when savegame persistence dispatches a `TiberiumClass`.

### 3.3 Post-load global rebuild

The standard load-game entry `FUN_0067E440` opens the save storage, opens the `CONTENTS` stream, creates the saved content object, calls its stream load, then runs post-load scene reinitialization. The ordering after content load is:

1. `FUN_0067E730` content load and object-list load work.
2. Cleanup/refresh helpers including `FUN_00685120`, `FUN_006D03A0`, `FUN_006D04F0`, `SidebarClass::InitSurface`.
3. `TiberiumClass::InitGrowthQueues_All @ 0x00722D00`.
4. `TiberiumClass::InitSpreadQueues_All @ 0x00722240`.
5. `RadarClass::RefreshRadar`.

Active in YR: Yes. Evidence: `FUN_0067E440` is reached from a load-game caller at `0x00559DC5`, uses `StgOpenStorage`, opens `CONTENTS`, calls stream load, then directly calls the two queue init functions at `0x0067E6AE` and `0x0067E6B3`.

### 3.4 Queue init functions are destructive rebuilds

`TiberiumClass::InitGrowthQueues_All @ 0x00722D00` and `TiberiumClass::InitSpreadQueues_All @ 0x00722240` iterate every `TiberiumClass`, free existing heap entry storage, free the entry arrays and bitmaps, allocate new arrays sized to map cell count, construct a 0x14-byte heap object, zero heap backing storage, and call the matching rebuild function.

Active in YR: Yes. Evidence: `get_function_xrefs` for both init functions returns the standard scenario init call, random-map generation call, and savegame load-game post-content call.

Tiny detail: the init functions rebuild queue entries and bitmaps but do not themselves write the timer fields. Timer state is handled by the `TiberiumClass` load slot and later by the drivers.

## 4. INI Keys

No new INI keys are introduced by this save/load slice. The queue rebuild still derives membership from loaded map-cell tiberium state and the normal `TiberiumClass` rules fields already documented in `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`.

## 5. Integration Points

| Integration | Evidence | Active in YR |
|---|---|---|
| Standard scenario init builds queues after map overlay/terrain load. | `ScenarioClass::Full_Init @ 0x00686B20` calls `0x00722D00` then `0x00722240`. | Yes. |
| Random-map generation builds queues near its tail. | `FUN_00598960` calls `0x00722D00` then `0x00722240`. | Conditional: random-map path. |
| Savegame load rebuilds queues after `CONTENTS` stream load. | `FUN_0067E440` calls `0x0067E730`, then `0x00722D00`, then `0x00722240`. | Yes for load-game. |
| `TiberiumClass` save slot exists and delegates to raw `AbstractClass::Save`. | `0x007220D0`, data xref `0x007F5740`. | Conditional via persistence dispatch. |
| `TiberiumClass` load slot exists and sanitizes dynamic queues. | `0x00721F70`, xref to `AbstractClass::Load` at `0x00721F83`. | Conditional via persistence dispatch. |

## 6. Current Rust Implementation Status

Rust currently has a hybrid state:

- `src/sim/ore_growth.rs:139` still declares the main state as an incremental map scanner/reservoir model.
- `src/sim/ore_growth.rs:163` and `:166` now carry partial native-style `growth_queue` and `spread_queue` vectors.
- `src/sim/ore_growth.rs:277` hashes those partial queues through `OreGrowthState::hash_state`.
- `src/sim/production/production_types.rs:196` derives serde for `ProductionState`, so the current Rust ore-growth scheduler state is serialized directly.
- `src/app_init.rs:856` constructs a new `OreGrowthState` at map initialization, but there is no native-style post-load queue rebuild from loaded visible ore cells.

Current Rust delta: Rust snapshots should not blindly preserve old scan/reservoir state once the native queue model is implemented. For native parity, queue entries/bitmaps should be rebuildable from restored map-cell tiberium state on load, or Rust must deliberately choose stronger deterministic serialization for lockstep with a clear divergence note.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TiberiumClass` save slot | verified | `0x007220D0`; data xref `0x007F5740`; callee `0x00410320` | exact stream inventory outside this class slot not covered |
| `TiberiumClass` load slot queue sanitation | verified | `0x00721F70` | none for queue pointer/count/bitmap fields |
| `CDTimerClass::Start(0)` during `TiberiumClass` load | verified | `0x00721F70`, `0x0046B640` | exact decompiler-hidden `ECX` offsets are inferred from class layout and call position |
| Load-game post-content rebuild | verified | `0x0067E440`, calls at `0x0067E6AE` and `0x0067E6B3` | none |
| Queue init destructive allocation/rebuild | verified | `0x00722D00`, `0x00722240` | rebuild internals already covered by parent queue report |
| Exact OLE save stream object ordering | touched-not-exhausted | `0x0067E730` nested `OleLoadFromStream` chain | not needed to prove post-load queue rebuild; defer to save-system work |
| Rust snapshot/load behavior | verified-source-scan | `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, `src/sim/world/world_hash.rs` | future implementation tests after queue model lands |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is there a `TiberiumClass` save slot? -> Yes, `0x007220D0` is vtable-referenced and calls `AbstractClass::Save`.` (evidence: `0x007220D0`, xref `0x007F5740`)
- `[RESOLVED] OQ-02 - Is there a `TiberiumClass` load slot? -> Yes, `0x00721F70` calls `AbstractClass::Load` and reinstalls `TiberiumClass` vtables.` (evidence: `0x00721F70`)
- `[RESOLVED] OQ-03 - Are growth/spread queue entries restored as authoritative dynamic allocations? -> No; load zeroes growth/spread heap, bitmap, entry-array, and count fields after raw load.` (evidence: `0x00721F70`)
- `[RESOLVED] OQ-04 - Are queue bitmaps rebuilt after load-game? -> Yes; `FUN_0067E440` calls both queue init functions after content load, and each allocates/zeroes bitmaps before rebuild.` (evidence: `0x0067E440`, `0x00722D00`, `0x00722240`)
- `[RESOLVED] OQ-05 - Are queue entries rebuilt after load-game? -> Yes; queue init allocates fresh entry arrays/heaps and calls the rebuild functions.` (evidence: `0x00722D00`, `0x00722240`)
- `[RESOLVED] OQ-06 - Do queue init functions reset timers? -> No direct timer writes in init; timers are touched by the `TiberiumClass` load reconstruction block and later drivers.` (evidence: `0x00722D00`, `0x00722240`, `0x00721F70`, `0x007221B0`, `0x00722C40`)
- `[RESOLVED] OQ-07 - Is the post-load rebuild active in standard YR? -> Yes for loading a saved game through `FUN_0067E440`.` (evidence: `StgOpenStorage`/`CONTENTS` stream load and calls at `0x0067E6AE/0x0067E6B3`)
- `[RESOLVED] OQ-08 - Does normal scenario init also rebuild queues? -> Yes, `ScenarioClass::Full_Init` calls growth then spread init after overlay and terrain load.` (evidence: `0x00686B20`)
- `[RESOLVED] OQ-09 - Does random-map generation also rebuild queues? -> Yes, `FUN_00598960` calls growth then spread init near its tail.` (evidence: xrefs and decompile `0x00598960`)
- `[RESOLVED] OQ-10 - Should Rust preserve current scan/reservoir queues through save/load for parity? -> No; that model is not the native queue model, and native dynamic queue entries are sanitized/rebuilt after load.` (evidence: Rust scan plus `0x00721F70`, `0x0067E440`)
- `[DEFERRED] OQ-11 - Exact complete save stream object ordering around all `OleLoadFromStream` groups.` (category: out-of-scope; reason: post-load queue rebuild proves the queue handoff without decoding every saved object list; next-step-if-pursued: dedicated savegame object graph report)
- `[DEFERRED] OQ-12 - Exact raw byte count returned by the `TiberiumClass` get-size slot.` (category: bounded-cost-too-high; reason: not needed because the load slot sanitizes dynamic queue fields after raw load; next-step-if-pursued: inspect vtable `0x007F5740` neighborhood and slot `+0x30`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native load-game rebuilds growth/spread queue entries and bitmaps after content load, rather than preserving dynamic heap entries as authoritative. | `0x0067E440`, `0x00722D00`, `0x00722240` | mismatch/partial: current Rust serde preserves scan/reservoir scheduler and partial queue vectors. | `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, snapshot load path | Add a native-style `rebuild_tiberium_queues_from_cells` after snapshot/map restore once the real queue model exists. | Save after several queued growth/spread mutations, load, rebuild queues from visible tiberium cells, then first post-load queue tick matches GameMD. Proposed test: `ore_growth_queues_rebuild_from_cells_after_snapshot_load`. | Do not deserialize stale heap order or old scan candidates as parity state. |
| `TiberiumClass` load sanitizes dynamic growth/spread heap, bitmap, entry-array, and count fields to zero. | `0x00721F70` | missing explicit Rust equivalent for future native queues. | future queue state in `OreGrowthState` / per-type tiberium state | Treat queue entries/bitmaps as rebuildable runtime state on load; if serialized for determinism, immediately rebuild or validate against cells. | Snapshot containing deliberately divergent queue membership for same visible ore map loads into canonical rebuilt membership. Proposed test: `ore_growth_snapshot_load_discards_stale_queue_membership`. | Do not let saved pointer-like/dynamic queue fields outrank loaded map-cell tiberium state. |
| Timer subobjects are re-started during `TiberiumClass` load, while queue init does not touch timers. | `0x00721F70`, `0x0046B640`, `0x007221B0`, `0x00722C40` | unchecked: Rust native queue timers do not exist yet; current scanner cursor is serialized. | future per-type growth/spread timers and world hash | Model timers separately from queue membership rebuild; hash timer state if it affects future tick output. | Save/load just before a mature growth interval; after load, the next growth/spread firing frame follows native timer rehydration, not old scan cursor. Proposed test: `tiberium_queue_timers_rehydrate_separately_from_membership`. | Do not assume queue rebuild also resets intervals or last-fire frames unless the timer slot proof is extended. |

## Negative Facts / Do Not Do

- Do not preserve native queue heap entries, entry arrays, or bitmap bytes as authoritative after save/load. Evidence: `0x00721F70` zeroes dynamic queue fields and `0x0067E440` rebuilds queues.
- Do not implement Rust snapshot load by restoring the old RA1-style scan cursor/candidate reservoir and call that GameMD parity. Evidence: native load-game runs queue init/rebuild calls, while Rust `ore_growth.rs` is still scan/reservoir-shaped.
- Do not rebuild queues before saved content/object state is loaded. Evidence: `FUN_0067E440` calls queue init after stream load and UI/surface post-load setup.
- Do not conflate `TiberiumClass::ComputeCRC @ 0x00721DC0` with save/load coverage. Evidence: CRC only hashes rules/type fields and has a data xref, while save/load slots are `0x007220D0` and `0x00721F70`.
- Do not treat queue init as timer reset. Evidence: `0x00722D00` and `0x00722240` rebuild allocations/membership; timer writes appear in `0x00721F70` and the drivers.

## Remaining Uncertainty

- Exact vtable get-size byte count for `TiberiumClass` raw save remains deferred; it is not needed for the queue handoff because dynamic queue fields are sanitized after load.
- Exact complete OLE save stream object ordering remains deferred; the post-load queue rebuild point is verified.
- Exact `ECX` offsets for the two `CDTimerClass::Start(0)` calls are inferred from the load reconstruction block and established timer layout; a follow-up can disassemble the call setup if timer frame parity becomes the next implementation blocker.

## Stale Docs / Follow-up Docs

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`: replace the OQ-13 deferred wording with:
  "Resolved by `TIBERIUMCLASS_QUEUE_SAVE_LOAD_REBUILD_GHIDRA_REPORT.md`: `TiberiumClass` save/load slots exist (`0x007220D0`, `0x00721F70`), but load sanitizes dynamic growth/spread heap, entry-array, bitmap, and count fields, and the standard load-game path `FUN_0067E440` rebuilds growth then spread queues after the saved `CONTENTS` stream is loaded. Queue entries/bitmaps should be treated as rebuilt runtime state after load, not serialized authoritative heap state."

## Sources

- Ghidra decompile: `0x00721F70`, `0x007220D0`, `0x0067E440`, `0x0067E730`, `0x00722D00`, `0x00722240`, `0x00410320`, `0x00410380`, `0x0046B640`, `0x007221B0`, `0x00722C40`, `0x00686B20`, `0x00598960`.
- Ghidra xrefs: `0x00722D00`, `0x00722240`, `0x007220D0`, `0x00410380`, `0x0067E440`.
- Prior docs: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/ore_growth.rs`, `src/sim/production/production_types.rs`, `src/sim/world/world_hash.rs`, `src/app_init.rs`.
