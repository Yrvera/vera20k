# Pathfinder Alternate Object Lookup `FUN_0042B080` - Ghidra Research Report

**Address(es):** `0x0042B080` primary; caller binding inside `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, call site `0x0042ADF1`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** identify what `FUN_0042B080` does when `UpdateBridgePassability` has selected an empty probe-cell object list, including the scan data source, ground-vs-bridge list choice, object eligibility filter, subobject acceptance test, return value, and Rust-facing model needed for temporary bridge passability probes.
**Non-Scope:** full peer path propagation, probe RNG, 5x5 fallback marker geometry, full A* cost/reopen behavior, full object/subobject class identity for `object+0x674`, and implementation patches.
**Confidence:** High for the helper behavior and caller binding; Medium for the human-readable subobject name because the concrete vtable behind `object+0x674` was not resolved in this slice.
**Active in YR:** Yes / Conditional. The helper is live through standard YR A* when `PathfinderClass+0x3C != 0`, `UpdateBridgePassability` selects a probe cell, and the initially selected `CellClass+0xE4` or `+0xE8` list pointer is null.

## Working Notes Gate

Target question: Identify `FUN_0042B080` used by `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, especially what data structure/list it fetches or builds for a given height/layer, how it selects ground vs bridge occupants, and what Rust must model for temporary bridge passability probes.

Non-goals: Do not re-investigate broad dual-layer A*, peer path propagation, zone precheck, edge cost, or Rust implementation; do not mutate Ghidra; write only this report and the shared claims file.

Evidence needed to mark COMPLETE: decompile plus assembly context for `0x0042B080`, decompile plus caller/call-site evidence from `0x0042ACF0`, proof of list/layer selection and return semantics, YR activity statement, Rust surface scan, and explicit implementation handoff/negative facts.

Stop conditions: Stop once the fallback helper's scan/list/object-return semantics are resolved; defer only concrete `object+0x674` vtable identity or wider pathfinder systems.

## 1. Overview

`FUN_0042B080` does **not** create, allocate, or cache an alternate object list. It is a fallback search helper: given a center cell coordinate and a requested height, it scans a 5x5 square around that center, chooses each candidate cell's ground or bridge object-list head, walks those existing linked lists, and returns the first object whose attached subobject accepts the original center point at the requested height.

The helper is called only after `UpdateBridgePassability` has already selected a probe cell's `CellClass+0xE4` or `CellClass+0xE8` list and found it null. The returned object then feeds the same peer-kind/path-queue logic as an object from the direct list.

## 2. Class Layout / Key Offsets

| Offset / item | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `param_1` | Pointer to packed center cell coordinate, two signed shorts `(x,y)` | Conditional | `0x0042B08A`, `0x0042B08D`; caller passes `probe_cell+0x24` at `0x0042ADE3..0x0042ADF1` |
| `param_2` | Requested height/level in height units | Conditional | `0x0042B0AB` multiplies by `DAT_0089C2D8`; caller passes `probe.level + (bridge_selected ? 4 : 0)` at `0x0042ADD8..0x0042ADEB` |
| `DAT_0089C2D8` | Leptons-per-height-level scale used to form the Z coordinate for subobject query | Yes | `0x0042B098`, `0x0042B0AB` |
| `MapClass::Get_CellClass @ 0x005657A0` | Converts candidate packed coordinate to `CellClass*` | Yes | call at `0x0042B0F8` |
| `CellClass+0x140 & 0x100` | Structural bridge-cell bit used by helper list selection | Yes | `0x0042B0FF..0x0042B108` |
| `CellClass+0x11B` | Signed cell level for `abs(candidate.level - requested_height)` | Yes | `0x0042B10A..0x0042B11A` |
| `CellClass+0xE4` | Ground object-list head | Yes | `0x0042B127` |
| `CellClass+0xE8` | Bridge/deck object-list head | Yes | `0x0042B11F` |
| `object+0x30` | Next pointer in selected cell object list | Yes | `0x0042B17C` |
| `object+0x14 bit 2` | Eligibility bit; objects without this bit are skipped without subobject query | Yes | `0x0042B131..0x0042B13E` |
| `object+0x674` | Required attached subobject queried through vtable `+0xA0`; null triggers assert but execution continues to use it after the assert call | Conditional | `0x0042B140..0x0042B172` |
| subobject vtable `+0xA0` | Acceptance predicate called with center lepton X/Y/Z | Conditional | `0x0042B158..0x0042B178` |

## 3. Core Logic

### 3.1 Caller Binding And Inputs

Active in YR: Conditional on direct selected list being null.

Inside `PathfinderClass::UpdateBridgePassability`, the probe-cell list choice immediately before the helper is:

- `Cell+0xE4` when the probe is not a structural bridge cell, or when the probe is bridge but `abs(current_cell.level - probe.level) <= 3` and the searching unit's `Foot+0x8C` on-bridge byte is zero.
- `Cell+0xE8` when the probe is bridge and either the level gap is `> 3` or the searching unit is already on a bridge.

If that selected list pointer is non-null, the helper is not called. If it is null, the caller passes:

- `param_1 = &probe_cell.CellCoord` (`probe_cell + 0x24`);
- `param_2 = probe_cell.level + 4` when bridge list was selected, otherwise `probe_cell.level`.

Evidence: `0x0042AD93..0x0042ADD6` for direct `E4/E8` choice and null test; `0x0042ADD8..0x0042ADF1` for helper arguments and call. Active in YR: Yes / Conditional.

### 3.2 It Scans A 5x5 Square; It Does Not Build A List

Active in YR: Conditional on helper call.

The helper initializes two loop counters to `-2` and increments each while `< 3`, making the candidate offsets exactly `dx=-2..=2`, `dy=-2..=2`. For every candidate, it computes:

```text
candidate = center + (dx, dy)
cell = MapClass::Get_CellClass(candidate)
```

It returns immediately on the first accepted object. If all 25 candidate cells and all linked-list objects fail, it returns null.

Evidence: loop setup `0x0042B0BC..0x0042B0C9`; candidate coordinate formation `0x0042B0CD..0x0042B0F8`; inner loop advance `0x0042B187..0x0042B18F`; outer loop advance and null return `0x0042B195..0x0042B1B0`; success return `0x0042B1B3..0x0042B1BC`. Active in YR: Conditional.

Handoff-critical negative: no allocation, insertion, cache lookup table, or persistent "alt object list" is present in this helper. It returns a single `ObjectClass*` or `0`.

### 3.3 Per-Candidate Ground Vs Bridge List Selection

Active in YR: Conditional on helper call.

For each candidate cell, the helper chooses the linked-list head as:

```text
if !(candidate.flags & 0x100):
    list = candidate.Cell+0xE4
else if abs(candidate.level - requested_height) <= 2:
    list = candidate.Cell+0xE4
else:
    list = candidate.Cell+0xE8
```

The threshold is stricter than the caller's direct probe list choice. The helper uses `CMP EAX,0x2; JLE ground-list`, so a difference of exactly `3` selects the bridge list. In equivalent positive form:

```text
bridge list iff candidate is structural bridge AND abs(candidate.level - requested_height) > 2
```

Evidence: `0x0042B0FF..0x0042B108` bridge-bit test; `0x0042B10A..0x0042B11D` signed absolute height difference and `<= 2` ground branch; `0x0042B11F` bridge list; `0x0042B127` ground list. Active in YR: Conditional.

Important distinction from the caller: the caller uses the probe/current cell level gap and `Foot+0x8C`, with `> 3` by height alone. The helper uses candidate/requested height and `> 2`, and does **not** read `Foot+0x8C`.

### 3.4 Object Filtering And Acceptance Predicate

Active in YR: Conditional on non-empty candidate list.

The helper walks the selected cell list by repeatedly reading `object+0x30`. For each object:

1. Read `object+0x14`, shift right by 2, and keep bit 0.
2. If that bit is zero, skip the object and continue to `object+0x30`.
3. If the bit is set, require `object+0x674` non-null. If null, call `GameDebugLog::Assert(0x80004003)`.
4. Call `(*(object+0x674)->vtable+0xA0)(subobject, center_x*256+128, center_y*256+128, requested_height * DAT_0089C2D8)`.
5. Return this object if the predicate returns nonzero.

Evidence: bit filter `0x0042B131..0x0042B13E`; null assert `0x0042B140..0x0042B14F`; argument setup and vtable call `0x0042B154..0x0042B178`; success return `0x0042B17A`, `0x0042B1B3`; next pointer `0x0042B17C`. Active in YR: Conditional.

The X/Y/Z passed to the predicate are based on the **original center coordinate**, not the candidate cell currently being scanned. The candidate cell only supplies an object list to inspect.

### 3.5 Return Semantics And Downstream Use

Active in YR: Conditional on helper call.

The helper returns the first accepted object pointer, not a list head. `UpdateBridgePassability` stores that returned pointer in the same variable used for a direct `Cell+0xE4/+0xE8` list head, then enters a `do { ...; obj = obj+0x30 }` loop. This means the fallback starts scanning from the matched object and then follows its `+0x30` next pointers within that object's original cell list.

Evidence: helper success return `0x0042B1B3`; caller stores return in `EBP` at `0x0042ADF6`; caller tests and enters object loop at `0x0042AE09..0x0042AE1A`; caller advances through `object+0x30` at `0x0042AFBF`. Active in YR: Conditional.

Parity consequence: a Rust implementation should model "fallback found first eligible peer object near the probe at requested height" rather than materializing all nearby candidates or merging 25 cell lists.

## 4. INI Keys

No INI key directly configures `FUN_0042B080`, the helper scan radius, the `>2` helper list threshold, `object+0x14 bit 2`, or the `object+0x674` predicate.

| Input | Relationship | Active in YR | Evidence |
|---|---|---|---|
| Movement/path urgency | Gates whether `UpdateBridgePassability` is called by A* and whether the helper can be reached | Conditional | prior `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`; caller `0x00429A90` |
| `Foot+0x8C` on-bridge byte | Used by caller direct probe list choice, not by this helper | Yes | caller `0x0042ADB8..0x0042ADC2`; no read inside `0x0042B080` |
| `Speed=` / `TechnoType+0x678` | Used later by peer eligibility in `UpdateBridgePassability`, after helper return | Yes | caller `0x0042AE58..0x0042AE66`; not part of helper |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `AStar_main_loop @ 0x00429A90` | Calls `UpdateBridgePassability` around A* when `Pathfinder+0x3C != 0` | Conditional | prior spine reports |
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | Sole observed caller in this slice; invokes helper only after selected probe list is null | Conditional | decompile; call site `0x0042ADF1` |
| `FUN_0042B080 @ 0x0042B080` | Fallback first-object finder over existing nearby cell object lists | Conditional | decompile plus assembly context |
| `MapClass::Get_CellClass @ 0x005657A0` | Resolves each candidate coordinate | Yes | call at `0x0042B0F8` |
| `object+0x674 vtable+0xA0` | Height/point acceptance predicate for candidate object | Conditional | call at `0x0042B172` |

## 6. Current Rust Implementation Status

Read-only scan only. No Rust files were modified.

| Rust surface | Current status vs scoped finding | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs` | Has layered A*, `SearchMarkerOverlay`, `LayeredEntityBlockMap`, urgency, and 24-step path constants; no exact `FUN_0042B080` fallback object lookup was found | Codegraph context; `rg` for marker/entity block/list terms |
| `src/sim/occupancy.rs` | Maintains per-cell ordered occupancy lists by `MovementLayer`; this is the closest data source for modeling `Cell+0xE4/+0xE8` selection | source scan |
| `src/sim/pathfinding/cell_entry.rs` | Already contains explicit `object_list_layer` vs `occupancy_bits_layer` concepts for CanEnter-style checks, but not this helper's 5x5 first-object fallback | source scan |
| `src/sim/movement/movement_path.rs` / `zone_search.rs` | Thread entity block maps and search marker overlays into pathfinding; no probe fallback over nearby object lists found | source scan |
| `src/sim/movement/movement_step.rs` / movement tests | Maintain runtime bridge object-list relinking via `on_bridge`; useful source for snapshots, but not an A* temporary marker fallback | source scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes gate | verified | report section above | none |
| Helper function boundary `0x0042B080` | verified | successful Ghidra decompile and assembly context | none |
| Caller binding from `0x0042ACF0` | verified | `0x0042ADD4..0x0042ADF1`; call stores return at `0x0042ADF6` | none |
| 5x5 scan radius/order | verified | `0x0042B0BC..0x0042B1A1` | none |
| Candidate list selection | verified | `0x0042B0FF..0x0042B127` | none |
| Object eligibility bit | verified | `0x0042B131..0x0042B13E` | semantic name of bit remains out-of-scope |
| `object+0x674` predicate call | verified | `0x0042B140..0x0042B178` | concrete subobject class identity deferred |
| Return/null behavior | verified | `0x0042B17A`, `0x0042B1A7`, `0x0042B1B3` | none |
| YR activity | verified through caller chain | prior A* spine docs; `0x0042ADF1` | none for normal pathfinder |
| Rust status | touched-not-exhausted | Codegraph + `rg` scan | implementation design remains future work |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is this helper live in YR? -> Yes, conditionally through live A* `UpdateBridgePassability` when the selected probe list is null.` (evidence: `0x0042ADF1`; prior A* spine docs)
- `[RESOLVED] OQ-2 - Does the helper build or fetch a persistent alternate list? -> No; it scans existing nearby `Cell+0xE4/+0xE8` linked lists and returns one object pointer or null.` (evidence: `0x0042B0BC..0x0042B1BC`)
- `[RESOLVED] OQ-3 - What are the helper inputs? -> Center coordinate pointer and requested height; caller passes `probe_cell+0x24` and `probe.level + (bridge_selected ? 4 : 0)`.` (evidence: `0x0042ADD8..0x0042ADF1`)
- `[RESOLVED] OQ-4 - What scan radius and order are used? -> Nested `-2..=2` loops over a 5x5 square; inner counter is the X offset in the emitted coordinate expression, outer is Y.` (evidence: `0x0042B0BC..0x0042B0EA`, `0x0042B187..0x0042B1A1`)
- `[RESOLVED] OQ-5 - How is ground vs bridge list chosen in helper? -> Bridge list only when candidate has `0x100` and `abs(candidate.level - requested_height) > 2`; otherwise ground list.` (evidence: `0x0042B0FF..0x0042B127`)
- `[RESOLVED] OQ-6 - Is helper threshold the same as the caller threshold? -> No; caller direct choice uses `>3` or `Foot+0x8C`, helper fallback uses `>2` and no `Foot+0x8C`.` (evidence: caller `0x0042ADB3..0x0042ADC2`; helper `0x0042B11A..0x0042B127`)
- `[RESOLVED] OQ-7 - Which objects are eligible for the subobject predicate? -> Only objects with `(object+0x14 >> 2) & 1` set.` (evidence: `0x0042B131..0x0042B13E`)
- `[RESOLVED] OQ-8 - What happens if eligible object lacks `+0x674`? -> It calls `GameDebugLog::Assert(0x80004003)`; this is an assertion path, not a graceful skip in the decompile.` (evidence: `0x0042B140..0x0042B14F`)
- `[RESOLVED] OQ-9 - What point is passed to vtable `+0xA0`? -> Original center cell center in leptons and requested height scaled by `DAT_0089C2D8`.` (evidence: `0x0042B08A..0x0042B0B8`, `0x0042B154..0x0042B172`)
- `[RESOLVED] OQ-10 - Does the predicate use candidate cell center? -> No; candidate only supplies the object list, while the predicate point uses the original center coordinate.` (evidence: center precompute `0x0042B08A..0x0042B0B8`; candidate loop starts later `0x0042B0BC`)
- `[RESOLVED] OQ-11 - What is returned on success? -> The object pointer whose subobject predicate returned nonzero.` (evidence: `0x0042B178..0x0042B1B3`)
- `[RESOLVED] OQ-12 - What is returned on failure? -> Null after all 25 candidate cells and all selected-list objects are exhausted.` (evidence: `0x0042B195..0x0042B1B0`)
- `[RESOLVED] OQ-13 - How does caller use the returned object? -> As the starting object of the same peer scan loop, then advances by `object+0x30`.` (evidence: `0x0042ADF6`, `0x0042AE09..0x0042AE1A`, `0x0042AFBF`)
- `[RESOLVED] OQ-14 - Are there direct INI keys for this helper? -> No direct key found; inputs are runtime pathfinder/cell/object state.` (evidence: INI grep; helper decompile)
- `[RESOLVED] OQ-15 - What Rust surfaces are affected? -> Pathfinding marker generation needs occupancy/list-layer snapshots, not just entity block maps; current Rust has no exact helper equivalent.` (evidence: Codegraph context; `rg` scan)
- `[DEFERRED] OQ-16 - What exact concrete class/interface is `object+0x674` and vtable `+0xA0`?` (category: `out-of-scope`; reason: the slot target is helper list/selection semantics; resolving this vtable identity requires a broader ObjectClass/subobject pass; next-step-if-pursued: trace writers of `object+0x674` and vtable implementations of slot `+0xA0`)

The single deferred item does not block the scoped handoff: Rust can model the helper as "find first nearby object whose path/footprint subobject contains the probe center at requested height" until the concrete interface name is resolved.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `FUN_0042B080` is a first-object fallback scan, not list construction. It scans 25 nearby cells and returns one object pointer whose subobject accepts the original center point/height. | `0x0042B080` decompile; assembly `0x0042B0BC..0x0042B1BC`; Active in YR: Conditional | missing | `src/sim/pathfinding/core.rs`; occupancy/list snapshot helper likely using `src/sim/occupancy.rs` | Future `UpdateBridgePassability` overlay generation should fallback from an empty selected probe list by finding the first eligible nearby peer object, not by merging all neighbors or building a persistent list. | A probe with empty direct list and two nearby eligible peers returns/uses the first scan-order object whose footprint accepts the probe center; proposed test: `bridge_marker_fallback_uses_first_nearby_accepted_object`. | Do not allocate/cache an alternate list or collect all 25-cell occupants into unordered sets. |
| Helper list-layer choice differs from caller choice: per candidate, bridge list is selected only for structural bridge cells with `abs(candidate.level - requested_height) > 2`; no `Foot+0x8C` input is read. | `0x0042B0FF..0x0042B127`; caller contrast `0x0042ADB3..0x0042ADC2`; Active in YR: Conditional | unchecked/missing | `src/sim/occupancy.rs` list-layer query; `src/sim/pathfinding/core.rs` future marker generator | Model helper fallback with a separate list-selection predicate from the direct probe predicate. | Candidate bridge cell with height diff `2` scans ground list; diff `3` scans bridge list, even if the searching unit is not currently on bridge; proposed test: `bridge_marker_fallback_height_gap_two_ground_three_bridge`. | Do not reuse the caller's `>3 or on_bridge` predicate for helper candidates. |
| Candidate objects are filtered by `object+0x14 bit 2`, then by attached subobject `+0x674` vtable `+0xA0` against original center X/Y/Z. | `0x0042B131..0x0042B178`; Active in YR: Conditional | missing | entity footprint/path occupancy metadata; likely movement/path snapshot surface | Rust needs an eligibility/footprint acceptance concept for pathfinder peer marker fallback; simple cell occupancy alone is insufficient for exact parity. | An object in a candidate cell whose object-list layer matches but whose footprint predicate rejects the original probe center is skipped in favor of a later accepted object; proposed test: `bridge_marker_fallback_requires_object_footprint_acceptance`. | Do not treat every occupant in the chosen layer as accepted; the original center point is the query, not the candidate cell center. |
| The helper returns null cleanly when no object passes; caller then follows the no-peer branch in `UpdateBridgePassability`. | `0x0042B1A7..0x0042B1B0`; caller `0x0042AE09..0x0042AEB9`; Active in YR: Conditional | missing with marker overlay absent | `src/sim/pathfinding/core.rs` future marker generator | Preserve no-fallback-object behavior so urgency/no-peer logic remains the same as direct empty list. | Empty direct list plus no accepted nearby object reaches the no-peer `+0x3C` handling rather than marking a fabricated blocker; proposed test: `bridge_marker_fallback_none_preserves_no_peer_urgency_path`. | Do not invent a synthetic blocker/marker when fallback returns none. |

### Negative Facts / Do Not Do

- Do not call this an alternate object-list builder. Evidence: no allocation or storage; it scans linked lists and returns one object pointer. Active in YR: Conditional.
- Do not use sorted/global entity order for fallback. Evidence: scan order is nested cell offsets and then each selected cell's `object+0x30` linked-list order. Active in YR: Conditional.
- Do not reuse `UpdateBridgePassability` direct probe layer predicate inside the helper. Evidence: direct probe uses `>3` plus `Foot+0x8C`; helper uses candidate/requested height `>2` and no `Foot+0x8C`. Active in YR: Conditional.
- Do not query candidate cell center for subobject acceptance. Evidence: center X/Y are precomputed from `param_1` before the candidate loop and passed unchanged to vtable `+0xA0`. Active in YR: Conditional.
- Do not persist helper results into path grids, zones, or bridge runtime state. Evidence: helper is only a temporary object finder feeding per-search marker generation. Active in YR: Conditional.

## 10. Remaining Uncertainty

- The original symbolic identity of `object+0x674` and its vtable `+0xA0` predicate remains unresolved. The behavior needed by this pathfinder helper is verified, but a later object/subobject field pass could name it more accurately.
- The exact map edge behavior of `MapClass::Get_CellClass` for candidate offsets outside the playfield was not re-investigated here; existing pathfinder reports treat `Get_CellClass` as the standard map resolver.

## Sources

- Ghidra decompile: `FUN_0042B080 @ 0x0042B080`.
- Ghidra assembly context: `0x0042B080..0x0042B1BC`.
- Ghidra decompile: `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`.
- Ghidra assembly context: caller/list binding `0x0042AD93..0x0042ADF1`, helper use `0x0042AE09..0x0042AE1A`, next-object advance `0x0042AFBF`.
- Existing docs: `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `UPDATEBRIDGEPASSABILITY_PROBE_RNG_0042AD35_GHIDRA_REPORT.md`, `UPDATEBRIDGEPASSABILITY_PEER_PATH_PROPAGATION_0042AEF6_GHIDRA_REPORT.md`.
- INI scan: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini` for bridge/pathfinding/list-related keys.
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/occupancy.rs`, `src/sim/movement/movement_path.rs`, `src/sim/movement/movement_step.rs`.
