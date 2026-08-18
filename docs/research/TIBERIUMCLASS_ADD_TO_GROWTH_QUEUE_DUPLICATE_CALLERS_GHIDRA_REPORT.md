# TiberiumClass AddToGrowthQueue Duplicate Callers - Ghidra Research Report

**Address(es):** `0x007235A0` primary; callers `0x00480BA1`, `0x00487297`, `0x0074A486`, `0x0074A6D9`; related growth pop/reinsert `0x00722F00`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `TiberiumClass::AddToGrowthQueue` duplicate-entry semantics and all Ghidra xref callers in active standard YR.
**Non-Scope:** full growth processor timing/batch behavior, native save/load stream layout, full `CellClass::CanPlaceTiberium` rejection matrix, and spread-queue duplicate semantics except where needed for contrast.
**Confidence:** High for function/caller branch semantics from read-only Ghidra; Medium for the conditional duplicate scenario because static evidence proves the path shape but not a runtime trace occurrence.
**Active in YR:** Conditional. The function and all callers are live in standard YR systems; actual duplicate active heap entries require a queued cell to be removed and then re-placed before the stale heap entry is popped or queues are rebuilt.

## Working Notes

Target question: Can active standard YR callers create duplicate growth-queue entries for one cell, and where is duplicate prevention owned?

Non-goals: Do not investigate full growth processing, save/load, spread duplicates, or already-settled TIBTRE source/midpoint/immune behavior.

Evidence needed to mark COMPLETE: xref every caller of `0x007235A0`, decompile caller guards, inspect whether `AddToGrowthQueue` itself checks growth bitmap membership, inspect growth pop/reinsert enough to understand stale entries, and scan Rust queue surfaces.

Stop conditions: Stop after all xrefs are classified, duplicate/no-duplicate claims are bounded, stale-doc wording is prepared, and Rust-facing acceptance tests are named.

## 1. Overview

`TiberiumClass::AddToGrowthQueue` appends a growth entry when the target cell's current `OverlayData` byte is `< 11`. It does **not** test this tiberium type's growth bitmap before appending. Ordinary new-placement callers require the cell to be empty at the moment they place ore, but that does not prove there is no stale growth heap entry from an earlier ore instance on the same cell.

The strongest conclusion is therefore conditional, not absolute: direct duplicate appends are not produced by the density-11 `Reduce_Tiberium` call, and normal placement of a never-queued empty cell is single-entry; however, active YR can leave stale growth entries when ore is removed before its queued growth pop, and a later new placement on that cell can append a second active heap entry because `AddToGrowthQueue` has no bitmap guard.

## 2. Key Offsets And State

| Owner | Offset | Meaning | Active in YR | Evidence |
|---|---:|---|---|---|
| `CellClass` | `+0x24` | packed map coord passed to queue helpers | Yes | callsite assembly passes `cell + 0x24` |
| `CellClass` | `+0x44` | overlay type index; `-1` means no overlay | Yes | `CanPlaceTiberium @ 0x004838E0`; `PlaceTiberium @ 0x00487190` |
| `CellClass` | `+0x11E` | overlay data / density byte | Yes | `AddToGrowthQueue @ 0x007235A0` tests `< 0x0B` |
| `TiberiumClass` | `+0x10C` | growth entry append count / next array slot | Yes | `0x007235A0`, `0x00722F00`, `0x007233A0` |
| `TiberiumClass` | `+0x110` | growth heap object pointer | Yes | `0x007235A0`, `0x00722F00` |
| `TiberiumClass` | `+0x114` | growth membership bitmap pointer | Yes | set in `0x007235A0`; reset/rebuilt in `0x007233A0` |
| `TiberiumClass` | `+0x118` | growth entry array `{coord, priority_f32}` | Yes | appended in `0x007235A0` |

Important detail: `+0x10C` is an append index and capacity-pressure count, not the heap's live count. The heap object's first dword is the active heap count. Growth pops reduce heap count, but do not decrement `+0x10C`; rebuild resets it.

## 3. AddToGrowthQueue Core Logic

Verified from decompile `0x007235A0` and assembly xrefs:

1. Compute a linear cell index with `FUN_0042B1C0(coord)`.
2. Resolve `CellClass*` via `MapClass::Get_CellClass(coord)`.
3. If `CellClass+0x11E >= 0x0B`, return immediately.
4. If map capacity minus `10` is below current append count `+0x10C`, call `TiberiumClass::RebuildGrowthQueue`.
5. Write packed coord to `entries[+0x10C].coord`.
6. Consume one `Random::Next()`.
7. Store priority as float `currentFrame + (signed_abs(raw) % 50)`.
8. Insert a pointer to that array slot into the min-heap by priority.
9. Increment `+0x10C`.
10. Set `growth_bitmap[cell_index] = 1`.

Active in YR: Yes. Evidence: xrefs from `CellClass::PlaceTiberium`, `CellClass::Reduce_Tiberium`, and `VoxelAnimClass::AI`; these are live ore, harvester/combat, TIBTRE, meteor/gem shard systems.

Duplicate check result: there is no read of `TiberiumClass+0x114[cell_index]` before the append. The bitmap is written after heap insertion, so it is not a guard in this helper.

Invalid-density behavior: if `OverlayData >= 11`, the function returns before capacity rebuild, before RNG consumption, before heap append, and before bitmap write.

## 4. Caller Classification

| Callsite | Containing function | Caller guard before call | Duplicate guarantee | Active in YR |
|---|---|---|---|---|
| `0x00480BA1` | `CellClass::Reduce_Tiberium @ 0x00480A80` | caller calls only when pre-reduction `OverlayData == 11`; callee then sees `11` and rejects `< 11` | Cannot enqueue from this callsite; no duplicate append | Yes; harvesters and ore-damaging effects |
| `0x00487297` | `CellClass::PlaceTiberium @ 0x00487190` new-cell branch | reached after `CanPlaceTiberium` true; overlay constructor attempt occurs before call; `OverlayData` is written after call | Guarantees empty overlay now, but not absence of stale growth heap entry | Yes; TIBTRE/spread/map placement paths |
| `0x0074A486` | `VoxelAnimClass::AI @ 0x00749F30` meteor neighbor loop | per neighbor `CanPlaceTiberium` true; constructs overlay then calls AddToGrowthQueue; then writes `OverlayData = 0` | Guarantees empty overlay now, but not absence of stale growth heap entry | Yes; meteor/gem tiberium voxel anims |
| `0x0074A6D9` | `VoxelAnimClass::AI @ 0x00749F30` non-meteor single-cell branch | single landing cell `CanPlaceTiberium` true; constructs overlay then calls AddToGrowthQueue; then writes `OverlayData = 0` | Guarantees empty overlay now, but not absence of stale growth heap entry | Yes; gem shard / tiberium voxel anims |

Ghidra `get_function_xrefs(0x007235A0)` returned exactly these four unconditional call xrefs in the current database.

## 5. Stale Entries And Conditional Duplicates

Growth processing at `0x00722F00` pops heap entries. If the popped cell still maps to this `TiberiumClass+0x98`, it calls `CellClass::GrowTiberium`. If resulting density is still `< 11`, it appends a new entry inline, sets the growth bitmap to `1`, and calls `AddToSpreadQueue`. If resulting density is `>= 11`, it clears `growth_bitmap[cell_index] = 0` and does not reinsert.

Two important details bound duplicate behavior:

- The pop/reinsert path is not an `AddToGrowthQueue` caller and does not itself create two live heap entries for the same pop. It removes one heap pointer before appending the replacement pointer.
- If a queued cell is removed before it is popped, `Reduce_Tiberium` full removal does not clear the growth bitmap or remove the old growth heap entry. Its documented bitmap reset is spread-only through `TiberiumClass::ClearSpreadBitmaps_AllTypes @ 0x00722AB0`.

Therefore duplicates can be produced conditionally:

1. Cell has a live growth heap entry for type T and density `< 11`.
2. Ore on that cell is fully removed before the heap entry is popped.
3. The same cell later passes `CanPlaceTiberium` and receives new ore before the stale entry is popped or a growth rebuild occurs.
4. `PlaceTiberium` or `VoxelAnimClass::AI` calls `AddToGrowthQueue`.
5. Because `AddToGrowthQueue` does not test `growth_bitmap[cell]`, it appends another heap entry. The stale entry may later grow the new ore if it still maps to the same type.

Active in YR: Conditional. Every component path is active in standard YR: growth queues, harvester/combat full ore removal, TIBTRE/spread/new ore placement, and voxel tiberium placement. A concrete runtime trace of the whole sequence was not run in this slot.

## 6. Current Rust Implementation Status

Current Rust has a partial native-shaped side channel rather than a full YR queue model:

- `src/sim/ore_growth.rs:196` `enqueue_growth_queue_cell` unconditionally pushes a growth queue entry and consumes one raw RNG word.
- `src/sim/ore_growth.rs:165` stores `growth_queue: Vec<OreGrowthQueueEntry>` with no growth membership bitmap.
- `src/sim/ore_growth.rs:220` and `src/sim/ore_growth.rs:234` model spread membership separately, but only for spread.
- `src/sim/terrain_spawn.rs:555` calls the growth enqueue hook during TIBTRE-style placement.
- `src/sim/world/world_hash.rs:179` hashes current `ore_growth_state`, including the queue vector.

Rust currently matches the helper's lack of a dedupe guard at the narrow push level, but it still lacks the binary's per-type growth heap, bitmap, rebuild, stale-entry, and processor semantics. It also lacks a test proving conditional duplicates/stale entries are intentionally preserved.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AddToGrowthQueue @ 0x007235A0` density gate | verified | decompile `0x007235A0` | none |
| `AddToGrowthQueue` bitmap behavior | verified | decompile `0x007235A0`; no pre-append `+0x114` read | none |
| `AddToGrowthQueue` RNG behavior | verified | decompile `0x007235A0`; `Random__Next` after append coord | none |
| `AddToGrowthQueue` capacity rebuild | verified | decompile `0x007235A0`; threshold map cells minus 10 | none |
| All direct xrefs | verified | `get_function_xrefs(0x007235A0)` returned four callsites | none within current Ghidra database |
| `Reduce_Tiberium` callsite | verified | decompile and assembly context `0x00480BA1` | none for duplicate semantics |
| `PlaceTiberium` callsite | verified | decompile and assembly context `0x00487297` | exact map-load caller provenance out of scope |
| `VoxelAnimClass::AI` two callsites | verified | decompile and assembly context `0x0074A486`, `0x0074A6D9`; prior INI report | none for duplicate semantics |
| Growth pop/reinsert stale-entry behavior | touched-not-exhausted | decompile `0x00722F00` | full processor timing/batch out of scope |
| Current Rust growth enqueue | verified-source-scan | `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs` | future full queue implementation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What are all direct callers? -> Four xrefs: Reduce_Tiberium, PlaceTiberium, and two VoxelAnimClass::AI callsites.` (evidence: `get_function_xrefs(0x007235A0)`)
- `[RESOLVED] OQ-02 - Does AddToGrowthQueue check growth bitmap membership? -> No pre-append bitmap read was found; it only writes `+0x114[cell] = 1` after heap insertion.` (evidence: `0x007235A0`)
- `[RESOLVED] OQ-03 - What invalid cell/density state blocks insertion? -> `OverlayData >= 11` blocks insertion before RNG, append, and bitmap write.` (evidence: `0x007235A0`)
- `[RESOLVED] OQ-04 - Can the Reduce_Tiberium callsite append? -> No in the observed density-11 branch; the caller invokes the helper before reduction, so the callee sees `11` and returns.` (evidence: `0x00480BA1`, `0x007235A0`)
- `[RESOLVED] OQ-05 - Does PlaceTiberium prove no duplicate? -> It proves the cell is placeable/empty now, but it does not inspect growth bitmap or heap membership.` (evidence: `0x00487190`, `0x00487297`)
- `[RESOLVED] OQ-06 - Do voxel tiberium placements prove no duplicate? -> Same as PlaceTiberium: `CanPlaceTiberium` gates current cell occupancy, not stale growth heap membership.` (evidence: `0x0074A486`, `0x0074A6D9`)
- `[RESOLVED] OQ-07 - Does full ore removal clear growth membership? -> No evidence in the Reduce_Tiberium full-removal branch; it clears spread bitmaps via `0x00722AB0`, not growth bitmaps.` (evidence: `0x00480A80`, `0x00722AB0`)
- `[RESOLVED] OQ-08 - Does growth pop/reinsert itself duplicate a popped cell? -> No; it removes a heap pointer before inline replacement append when still growable.` (evidence: `0x00722F00`)
- `[RESOLVED] OQ-09 - Can duplicate growth heap entries occur in active standard YR? -> Conditional yes: stale queued entry after full removal plus later new placement can append another entry because the helper lacks a bitmap guard.` (evidence: combined `0x00722F00`, `0x00480A80`, `0x00487190`, `0x007235A0`)
- `[RESOLVED] OQ-10 - Is this TS-only? -> No; callers are live standard YR ore, TIBTRE, meteor/gem, harvester/combat paths.` (evidence: xrefs plus stock INI `TIBTRE01..03`, voxel `IsTiberium`, and existing queue report)
- `[RESOLVED] OQ-11 - What is the current Rust delta? -> Rust has an unconditional growth enqueue vector and no full per-type heap/bitmap/stale-entry processor model.` (evidence: `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs`)
- `[DEFERRED] OQ-12 - Concrete runtime frequency of the stale-remove-replace duplicate sequence.` (category: needs-runtime-debugger; reason: static evidence proves the sequence is reachable but not how often it naturally occurs; next-step-if-pursued: native runtime trace with queued low-density ore, immediate full removal, and TIBTRE/voxel replacement before growth pop)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `AddToGrowthQueue` does not dedupe against growth bitmap; it appends if `OverlayData < 11`. | `0x007235A0` | Rust push is currently unconditional, but no full heap/bitmap model exists. | `src/sim/ore_growth.rs` | Future queue model must allow duplicate heap entries when native would append; bitmap alone cannot be a hard insertion guard. | Place ore, enqueue growth, remove ore before pop, place new ore on same cell, assert two heap entries can coexist. | Do not use `BTreeSet<(type,cell)>` as the growth queue owner. Proposed test: `add_to_growth_queue_allows_stale_duplicate_after_remove_replace`. |
| `Reduce_Tiberium` density-11 callsite is a no-op for growth enqueue because the callee sees `OverlayData == 11`. | `0x00480BA1`, `0x007235A0` | Rust should not enqueue growth when reducing max-density ore merely because binary has that callsite. | `src/sim/tiberium/mod.rs`, future queue bridge | Preserve the pre-reduction density gate. | Reduce a density-11 cell and assert no new growth entry/RNG draw from this callsite. | Do not "fix" the apparent native call by enqueueing after subtract. Proposed test: `reduce_tiberium_density_11_growth_callsite_is_noop`. |
| Full removal does not clear growth heap/bitmap; stale entries are handled lazily by growth pop/rebuild. | `0x00480A80`, `0x00722F00`, `0x007233A0` | Rust currently has no growth membership bitmap or stale pop semantics. | `src/sim/ore_growth.rs`, `src/sim/tiberium/mod.rs`, snapshots/hash | Keep stale growth entries until processor pop or rebuild; hash/serialize them as deterministic future state. | Remove a queued cell, leave queue state intact, then verify later pop either grows replacement same-type ore or no-ops if still empty/different. | Do not eagerly remove growth entries on ore depletion unless a later binary report proves a separate path. Proposed test: `growth_queue_stale_entry_survives_full_removal_until_pop`. |

## 10. Negative Facts / Do Not Do

- Do not claim `AddToGrowthQueue` is deduped by its bitmap. Evidence: `0x007235A0` writes `+0x114[cell] = 1` after append and has no pre-append bitmap read.
- Do not add a growth entry from `Reduce_Tiberium` when pre-reduction density is `11`. Evidence: caller calls before reduction; callee rejects `OverlayData >= 11`.
- Do not make `CanPlaceTiberium` stand in for growth-queue membership. Evidence: placement callers check current cell occupancy/terrain only; stale heap state is in `TiberiumClass`.
- Do not remove growth queue entries on full ore removal as a convenience. Evidence: verified removal branch clears spread bitmaps, not growth heap/bitmap.
- Do not treat `TiberiumClass+0x10C` as active heap count. Evidence: growth pop decrements heap count but not `+0x10C`; rebuild resets append state.

## 11. Stale Docs / Replacement Wording

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:188` says "`PlaceTiberium`, `GrowTiberium`, and `Reduce_Tiberium` call these helpers." Replacement: "`AddToGrowthQueue` direct xrefs are `CellClass::Reduce_Tiberium`, `CellClass::PlaceTiberium`, and two `VoxelAnimClass::AI` callsites. Growth processor reinsert is inline in `0x00722F00`, not a direct `AddToGrowthQueue` call."
- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md:285` deferred `OQ-15`. Replacement: "`OQ-15 resolved: `AddToGrowthQueue` has no growth-bitmap guard. Direct density-11 `Reduce_Tiberium` cannot enqueue, but conditional duplicate active heap entries can occur if a queued cell is fully removed and later re-placed before the stale entry pops or a rebuild occurs."
- `docs/implementation-queue/2026-05-24-implementation-queue-tiberium.md:63` says exact duplicate enqueue semantics are not fully proven. Replacement: "Duplicate semantics are bounded by `TIBERIUMCLASS_ADD_TO_GROWTH_QUEUE_DUPLICATE_CALLERS_GHIDRA_REPORT.md`: implement growth queue as a heap/entry list that can preserve stale and duplicate cell entries; do not dedupe growth inserts by cell membership."

## Sources

- Ghidra read-only decompile: `0x007235A0`, `0x00480A80`, `0x00487190`, `0x00749F30`, `0x00722F00`, `0x007233A0`, `0x00722AB0`, `0x004838E0`.
- Ghidra xrefs: `get_function_xrefs(0x007235A0)` returned `0x00480BA1`, `0x00487297`, `0x0074A486`, `0x0074A6D9`.
- Ghidra assembly context: the four callsites above.
- Prior docs: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`, `ANIMTYPECLASS_TIBERIUM_FLAG_CONSUMERS_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini` for stock TIBTRE and voxel/anim tiberium use.
- Rust source scan: `src/sim/ore_growth.rs`, `src/sim/terrain_spawn.rs`, `src/sim/tiberium/mod.rs`, `src/sim/world/world_hash.rs`.
