# High Bridge OnBridge Occupancy Transition Sequence - Ghidra Research Report

**Address(es):** `0x0075AEC0`, `0x004B0F20`, `0x005684B1`, `0x005688E1`, `0x0047E8A0`, `0x0047EA90`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** normal walking/driving FootClass/ObjectClass cell-boundary steps across high-bridge ramp/body/ground transitions, focused on `ObjectClass+0x8C` (`OnBridge`) and `CellClass` object-list removal/insertion sequencing.  
**Non-Scope:** low-bridge TubeClass behavior, rendering body/railing selection except direct `OnBridge` consequences, teleport/drop-in/carryall/aircraft writers, bridge collapse.  
**Confidence:** High for walk/drive sequencing and high-bridge ramp cases; Medium for exact visual tick-cycle placement outside the locomotor process body.  
**Active in YR:** Yes. `WalkLocomotionClass::Process @ 0x0075AC80` directly calls `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0`; `DriveLocomotionClass::Process @ 0x004B0500` calls `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` on the drive-track path. No TS-only gate was found around the verified sequence.

## 1. Overview

The high-bridge ramp transition is a two-step logical transition, not a lookahead claim. On a standard high bridge, a unit moving ground -> ramp -> body remains `OnBridge=0` on the ground -> ramp step, then sets `OnBridge=1` on the ramp -> body step. In reverse, body -> ramp keeps `OnBridge=1`, then ramp -> ground clears `OnBridge=0`.

The binary removes the object from the old cell while the old `OnBridge` byte is still in force, updates coordinates, applies the bridge predicate, then re-adds the object so insertion observes the post-transition `OnBridge` byte.

## 2. Class Layout / Key Offsets

| Field | Meaning | Evidence | Active in YR |
|---:|---|---|---|
| `ObjectClass+0x8C` | persistent `OnBridge` byte; normal list selector passed to cell add/remove | `TechnoClass__EnterCell_AddToMultiCells @ 0x005684B1`, `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005688E1` read `[object+0x8C]` before pushing the layer argument | Yes |
| `CellClass+0xE4` | ground/FirstObject list head | `CellClass::AddContent @ 0x0047E8A0`, `RemoveContent @ 0x0047EA90` select this when list-layer argument is zero | Yes |
| `CellClass+0xE8` | bridge/AltObject list head | same functions select this when list-layer argument is nonzero | Yes |
| `CellClass+0x11B` | signed terrain level byte used by the bridge predicate | walk block at `0x0075C154..0x0075C16A`, drive blocks at `0x004B1807..0x004B181C` and `0x004B2568..0x004B2572` | Yes |
| `CellClass+0x140 & 0x100` | structural bridge flag used by set/clear predicate | walk writes at `0x0075C179`/`0x0075C193`; drive writes at `0x004B1830`/`0x004B184A`, `0x004B2586`/`0x004B25A0` | Yes |

## 3. Core Logic

Normal walk and drive boundary crossings use this ordering:

1. Call object mark/remove (`vtable+0x124` with `0`) while `ObjectClass+0x8C` still holds the old list state.
2. Call coordinate update (`vtable+0x1B4`) to move to the new coordinate.
3. Fetch previous and destination cells, compare signed `CellClass+0x11B` levels.
4. If `dst.Level == src.Level - 4` and destination has `0x100`, write `OnBridge=1`.
5. Otherwise, if destination lacks `0x100` and source has `0x100`, write `OnBridge=0`.
6. Call per-cell processing (`vtable+0x1CC`, where present) and call object mark/add (`vtable+0x124` with `1`) after the `OnBridge` write.

Load-bearing high-bridge ramp cases:

| Step | Predicate result | Removal observes | Insertion observes | Active in YR |
|---|---|---|---|---|
| ground h=4, non-bridge -> ramp h=4, bridgehead/bridge flag | no change: `dst.Level != src.Level - 4`, destination has `0x100` so clear branch is skipped | old `OnBridge=0` / ground list | post-state still `OnBridge=0` / ground list | Yes; normal approach to high-bridge ramp |
| ramp h=4, bridge flag -> body h=0, bridge flag | set: `0 == 4 - 4` and destination has `0x100` | old `OnBridge=0` / ground list | post-state `OnBridge=1` / bridge list | Yes; normal entry onto deck |
| body h=0, bridge flag -> ramp h=4, bridge flag | no change: level relation is false and destination has `0x100` | old `OnBridge=1` / bridge list | post-state still `OnBridge=1` / bridge list | Yes; normal exit toward ramp |
| ramp h=4, bridge flag -> ground h=4, non-bridge | clear: level relation false, destination lacks `0x100`, source has `0x100` | old `OnBridge=1` / bridge list | post-state `OnBridge=0` / ground list | Yes; normal step off high bridge |

Tiny details:

- The level bytes are sign-extended before comparison. This matters for malformed or high-bit map data; retail heights remain in the normal small signed range.
- The set branch has priority: after `OnBridge=1`, the code jumps through the destination-bridge test and skips the clear write.
- Destination bridge flag alone does not set `OnBridge`. Ground -> ramp and body -> ramp both land on bridge-flagged cells, but neither changes the byte because the exact `-4` level relation is not met.
- The clear branch requires destination not bridge-flagged and source bridge-flagged. A body -> ramp step keeps `OnBridge=1`.

## 4. INI Keys

No INI key controls this sequencing. `rulesmd.ini`, `rules.ini`, `artmd.ini`, and `art.ini` bridge-related searches found bridge strength/destruction/repair, `ZFudgeBridge`, and `TooBigToFitUnderBridge`, but not an INI override for `ObjectClass+0x8C`, the dual `CellClass` object lists, or the high-bridge `-4` level predicate.

Active in YR: Yes; the behavior is map/cell-flag and runtime-locomotor driven, not an optional INI switch.

## 5. Integration Points

`CellClass::AddContent @ 0x0047E8A0` selects `+0xE4` when the list-layer argument is `0` and `+0xE8` when nonzero. `CellClass::RemoveContent @ 0x0047EA90` uses the same selector.

`TechnoClass__EnterCell_AddToMultiCells @ 0x005684B1` reads `byte ptr [object+0x8C]`, pushes it, and calls `CellClass::AddContent`. `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005688E1` reads the same byte, pushes it, and calls `CellClass::RemoveContent`. Therefore the list layer is sampled at the exact call site; there is no deferred recomputation inside `CellClass`.

For walk, the bridge writer occurs at `0x0075C179` (`OnBridge=1`) or `0x0075C193` (`OnBridge=0`) after the coordinate update and before `vtable+0x124(1)` at `0x0075C1AE`. For drive, the first track block writes at `0x004B1830`/`0x004B184A` after `vtable+0x124(0)` at `0x004B17CC`, and the second block writes at `0x004B2586`/`0x004B25A0` before `vtable+0x124(1)` at `0x004B25B1`.

## 6. Current Rust Implementation Status

Current Rust no longer matches the stale risk wording in the older bridge reports:

- `src/sim/game_entity.rs:458` exposes `occupancy_list_layer()` from `on_bridge`, not `locomotor.layer`.
- `src/sim/occupancy.rs:110` rebuilds occupancy via `entity.occupancy_list_layer()`.
- `src/sim/movement/movement_step.rs:675` resolves the bridge update before choosing `occupancy_layer` at `movement_step.rs:690` and inserting via `move_entity` at `movement_step.rs:699`.
- `src/sim/movement/movement_tick.rs:712` snapshots old `entity.on_bridge`, resolves the bridge update around `movement_tick.rs:723`, computes projected `new_on_bridge`, and inserts with that projected layer at `movement_tick.rs:747`.
- Tests already pin the main high-bridge cases: `on_bridge_fires_at_ramp_to_body_only` (`movement_tests.rs:1558`), `on_bridge_clears_at_ramp_to_ground_only` (`movement_tests.rs:1633`), and `no_bridge_lookahead_pre_claim` (`movement_tests.rs:1732`).

Rust caveat: `OccupancyGrid::remove` removes by entity id without a layer argument. That differs structurally from gamemd's selected-list remove but is behaviorally equivalent for unique entity IDs unless future code permits duplicate same-entity entries on multiple layers.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Walk boundary transition sequencing | verified | `0x0075C12E` coordinate update, `0x0075C179`/`0x0075C193` writes, `0x0075C1AE` mark/add | none for normal walking steps |
| Drive first track boundary block | verified | `0x004B17CC` mark/remove, `0x004B1830`/`0x004B184A` writes | none for this drive block |
| Drive second track boundary block | verified | `0x004B2586`/`0x004B25A0` writes, `0x004B25B1` mark/add | exact earlier remove site not separately re-contexted beyond decompile in this pass |
| Cell list add/remove selector | verified | `0x0047E8A0`, `0x0047EA90`, `0x005684B1`, `0x005688E1` | none |
| High-bridge four-step ramp/body sequence | verified | predicate at `0x0075C154..0x0075C193`, `0x004B1807..0x004B184A`, `0x004B2568..0x004B25A0` | none for standard high-bridge height pattern |
| Low bridge | deferred | parent scope says already settled; low-bridge trace checked for contradiction only | out-of-scope |
| Rendering body/railing art | deferred | no direct rendering selection branch needed for this sequencing | out-of-scope |
| Teleport/drop-in/air/carryall writers | deferred | prior writer reports cover families outside normal walking/driving | out-of-scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is this an exhaustive-slice or coverage-map? -> exhaustive-slice for normal walk/drive high-bridge boundary steps only` (evidence: user target and scoped functions `0x0075AEC0`, `0x004B0F20`)
- `[RESOLVED] OQ2 - Does removal observe the pre-transition layer? -> yes; mark/remove is before the `OnBridge` writer in walk and drive` (evidence: `0x004B17CC` before `0x004B1830`, walk decompile at `0x0075AEC0`)
- `[RESOLVED] OQ3 - Does insertion observe the post-transition layer? -> yes; mark/add is after the `OnBridge` writer in walk and drive` (evidence: `0x0075C1AE`, `0x004B25B1`)
- `[RESOLVED] OQ4 - What sets `OnBridge` for normal high-bridge entry? -> ramp/body step where `dst.Level == src.Level - 4` and destination has `0x100`` (evidence: `0x0075C154..0x0075C179`, `0x004B1807..0x004B1830`)
- `[RESOLVED] OQ5 - What clears `OnBridge` for normal high-bridge exit? -> ramp/ground step where destination lacks `0x100` and source has `0x100`` (evidence: `0x0075C180..0x0075C193`, `0x004B258D..0x004B25A0`)
- `[RESOLVED] OQ6 - Does ground -> ramp pre-claim bridge occupancy? -> no; destination bridge flag alone is not enough to set `OnBridge`` (evidence: same predicate blocks)
- `[RESOLVED] OQ7 - Does body -> ramp clear early? -> no; destination still has `0x100`, so clear branch is skipped` (evidence: `0x0075C180..0x0075C188`)
- `[RESOLVED] OQ8 - Is `CellClass` list selection sampled from `OnBridge` at the add/remove call site? -> yes` (evidence: `0x005684B1`, `0x005688E1`)
- `[RESOLVED] OQ9 - Is the sequence active in standard YR? -> yes for Walk and Drive locomotors; their Process functions call the verified bodies without TS-only gates around this branch` (evidence: `0x0075AC80`, `0x004B0500`)
- `[RESOLVED] OQ10 - Are INI keys involved? -> no key found that controls this sequencing` (evidence: `rg` over `rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`)
- `[RESOLVED] OQ11 - Does current Rust still insert with `active_layer` before bridge update? -> no for scanned current files; that wording is stale` (evidence: `src/sim/movement/movement_step.rs:675`, `:690`, `:699`; `movement_tick.rs:712`, `:747`)
- `[RESOLVED] OQ12 - Does current Rust rebuild from locomotor layer? -> no; it calls `entity.occupancy_list_layer()` derived from `on_bridge`` (evidence: `src/sim/occupancy.rs:110`, `src/sim/game_entity.rs:458`)
- `[DEFERRED] OQ13 - Does the exact same selected-list sequencing hold for ship locomotor?` (category: `out-of-scope`; reason: target is normal walking/driving units; prior docs say ship mirrors it; next-step-if-pursued: verify `0x006A05F0` and `0x006A1C80`)
- `[DEFERRED] OQ14 - Can duplicate same-entity entries make Rust's layerless remove diverge?` (category: `requires-different-system-context`; reason: normal occupancy invariant is one entry per occupying entity; next-step-if-pursued: audit all `OccupancyGrid::add` callers for duplicate insertion)
- `[DEFERRED] OQ15 - Does rendering read `OnBridge` in this same tick after insertion?` (category: `out-of-scope`; reason: no direct render selection was needed to settle occupancy sequencing; next-step-if-pursued: trace display submit/mark readers after `vtable+0x124(1)`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ground -> ramp must not pre-claim bridge list; ramp -> body must insert on bridge list after `OnBridge=1` | `0x0075C179`, `0x004B1830`, `0x005684B1` | none observed in current scan | `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_tests.rs` | Preserve projected `on_bridge` insertion layer, not path layer | Deterministic high-bridge path ground h=4 -> ramp h=4 -> body h=0: first tick ground-list on ramp, second tick bridge-list on body | `test_high_bridge_ground_ramp_body_relinks_after_onbridge_update`; risk: reintroducing a lookahead bridge claim |
| Body -> ramp must remain bridge-list; ramp -> ground must clear and insert ground-list | `0x0075C193`, `0x004B25A0`, `0x005688E1` | none observed in current scan | same movement surfaces plus occupancy validation | Keep `OnBridge` true on body->ramp and clear only when destination lacks structural bridge | Unit starts on bridge body, steps to ramp, then ground: ramp cell has bridge occupant; final ground cell has ground occupant | `test_high_bridge_body_ramp_ground_relinks_after_onbridge_clear`; risk: deriving list layer from A* `next_layer` will clear too early |
| Rebuild/object-list membership must be sourced from `on_bridge`, not `locomotor.layer` | `0x005684B1`, `0x005688E1`; current Rust `game_entity.rs:458`, `occupancy.rs:110` | fixed versus stale docs; keep guarded | `src/sim/game_entity.rs`, `src/sim/occupancy.rs` | Preserve `occupancy_list_layer()` as the rebuild source | Artificial entity with `locomotor.layer=Bridge` but `on_bridge=false` rebuilds into ground layer; inverse rebuilds into bridge layer | `test_high_bridge_rebuild_uses_onbridge_not_locomotor_layer`; risk: future cleanup treating `loco.layer` as authoritative |

### Negative Facts / Do Not Do

- Do not set `OnBridge` on the ground -> ramp step just because the ramp has `Flags&0x100`. Evidence: set requires `dst.Level == src.Level - 4` plus destination bridge flag at `0x0075C16A..0x0075C179`.
- Do not clear `OnBridge` on body -> ramp because A* path layer may be ground there. Evidence: clear requires destination lacks `0x100`; destination ramp still has `0x100` at `0x0075C180..0x0075C188`.
- Do not use a single path/A* layer as the object-list selector. Evidence: add/remove read `ObjectClass+0x8C` at `0x005684B1`/`0x005688E1`; path layer is not read there.
- Do not update `OnBridge` before old-cell removal. Evidence: drive removal/mark call at `0x004B17CC` precedes writes at `0x004B1830`/`0x004B184A`; insertion/mark follows after the writes.
- Do not treat low-bridge TubeClass behavior as evidence for high-bridge deck list behavior. Evidence: this report's active path is `Flags&0x100` plus signed `Level - 4`; low-bridge parent trace says no height offset and no `OnBridge` set.

### Remaining Uncertainty

- Ship locomotor likely mirrors this sequence per prior reports, but this slot did not re-verify ship because the target was normal walking/driving units.
- Rust's layerless `OccupancyGrid::remove` is parity-safe under the one-entry-per-entity invariant; duplicate-entry impossible-ness was not exhaustively audited here.
- Direct same-tick render consumers after `Mark(1)` were not traced because no `OnBridge` write-to-render contradiction was required to settle occupancy sequencing.

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md:338` replacement wording: "Current Rust `OccupancyGrid::rebuild` now derives the occupancy layer from `GameEntity::occupancy_list_layer()` / `on_bridge`, not from `locomotor.layer`; preserve this because gamemd cell-list membership is selected by `ObjectClass+0x8C`."
- `docs/research/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md:339` replacement wording: "Current Rust `movement_step.rs::process_cell_crossings` resolves/projects the bridge transition before choosing the insertion layer; the remaining invariant is to keep removal conceptually pre-transition and insertion post-transition."
- `docs/research/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md:340` replacement wording: "Current Rust drive-track cell-jump code resolves/projects `new_on_bridge` before `OccupancyGrid::move_entity`; preserve this and do not fall back to A* `active_layer` as the object-list selector."
- `docs/research/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md:422-423` replacement wording: "As of the current scan, both straight movement and drive-track cell jumps project the post-transition `on_bridge` state before choosing the insertion layer. The older risk was valid historically but is stale for the scanned files."
- `docs/research/BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md:433-434` replacement wording: "The binary requires old-list removal and post-`OnBridge` insertion. Current Rust's layerless `remove` plus projected insertion is parity-safe under the one-entry-per-entity occupancy invariant; keep tests pinning both ramp directions."

## Sources

- Ghidra decompiled/read: `0x0075AC80`, `0x0075AEC0`, `0x004B0500`, `0x004B0F20`, `0x0047E8A0`, `0x0047EA90`, `0x005683C0`, `0x005687F0`
- Ghidra assembly context: `0x005684B1`, `0x005688E1`, `0x0075C179`, `0x0075C193`, `0x0075C1AE`, `0x004B17CC`, `0x004B1830`, `0x004B184A`, `0x004B2586`, `0x004B25A0`, `0x004B25B1`
- Existing docs checked: `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`, `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`, `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`, `HIGH_BRIDGE_EDGE_LANE_TRAVERSAL_REINVESTIGATION_GHIDRA_REPORT.md`, `traces/PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md`
- Current Rust scanned: `src/sim/game_entity.rs`, `src/sim/occupancy.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_tests.rs`
- INI searched: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
