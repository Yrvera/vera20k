# UpdateBridgePassability Probe/RNG 0x0042AD35 - Ghidra Research Report

**Address(es):** `0x0042AD35..0x0042ADD4` primary slice inside `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`; caller cleanup context `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`; 5x5 binding context `0x0042AFCB..0x0042B063`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** exact probe-cell selection, timer-derived direction input, selected object-list layer, own-current-cell skip, 5x5 center rules needed to bind the probe, and normal A* cleanup pairing for static success/failure paths.
**Non-Scope:** full peer path propagation loop, full A* expansion/cost semantics, full TubeClass handling, all callers above `AStar_main_loop`, and `CellClass+0x140` bits other than `0x40000`.
**Confidence:** High for the scoped probe/list/5x5/cleanup facts. Fresh Ghidra MCP was not exposed in this slot, so the fresh spot-check used local `gamemd.exe` PE disassembly plus existing same-day Ghidra decompile reports.
**Active in YR:** Yes / Conditional. The function is enabled by `PathfinderClass+0x03 = 1` and called by live A* when `PathfinderClass+0x3C != 0`; each material branch below states its YR activity.

## Working Notes Gate

Target question: Verify exact probe-cell selection for `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, focused on `0x0042AD35..0x0042ADD4`, and bind only the 5x5/cleanup context needed for implementation.

Non-goals: Do not re-investigate the full peer path propagation loop, full A* cost model, full movement-zone system, or Rust implementation; do not mutate Ghidra; do not write any file except this report and the shared claims file.

Evidence needed to mark COMPLETE: instruction-level evidence for timer-derived direction, `g_DirectionOffsets` coordinate add, selected `Cell+0xE4/+0xE8` layer branch including signed height comparison and `Foot+0x8C`, 5x5 own-cell/center rules, and static normal A* pre/post cleanup call pairing.

Stop conditions: Stop after the probe/list/5x5 binding and cleanup pairing are resolved; defer runtime-abnormal cleanup, full peer path propagation, and higher-level caller value provenance.

## 1. Overview

`UpdateBridgePassability` does not call the global game RNG for its initial probe. It samples the searching foot object's `RateTimer/FacingClass` at `Foot+0x388`, derives a 3-bit bucket with `((current >> 12) + 1) >> 1 & 7`, and uses that as an index into the eight-entry `g_DirectionOffsets @ 0x0089F688` table.

Active in YR: Yes. Evidence: existing Ghidra report `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` section 3.2 plus fresh local `gamemd.exe` disassembly `0x0042AD35..0x0042AD67`.

The selected probe coordinate is `current_cell_coord + g_DirectionOffsets[dir]`. The selected object-list layer is then chosen from the probe cell, not from the current cell: `Cell+0xE4` for ground list or `Cell+0xE8` for bridge/alternate list. The branch uses structural bridge bit `Cell+0x140 & 0x100`, signed `Cell+0x11B` levels, and searching-unit `Foot+0x8C`.

Active in YR: Yes. Evidence: fresh local disassembly `0x0042AD93..0x0042ADD4`.

## 2. Class Layout / Key Offsets

| Offset / item | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `Foot+0x388` | RateTimer/FacingClass object sampled for probe bucket | Yes | `0x0042AD35 lea ecx,[esi+0x388]`, call `0x004C93D0` |
| `0x0089F688` | `g_DirectionOffsets`, 8 signed packed `(dx,dy)` table | Yes | consumer `0x0042AD50`, `0x0042AD5F`; values/source in `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md` |
| Current foot cell coord | Source coordinate for probe add; produced from vtable `+0x1B8` and `MapClass::Get_CellClass` | Yes | `0x0042AD0F..0x0042AD32`; existing Ghidra report section 3.2 |
| `Cell+0x140 & 0x100` | Structural bridge bit for probe layer choice | Yes | `0x0042AD93..0x0042AD9C` tests `AH & 1`, equivalent to bit `0x100` |
| `Cell+0x11B` | Signed byte cell level used in absolute level-gap compare | Yes | `0x0042AD9E`, `0x0042ADA5` |
| `Foot+0x8C` | Searching unit on-bridge byte; nonzero forces bridge list when probe is bridge and gap is not `>3` | Yes | `0x0042ADB8..0x0042ADC0` |
| `Cell+0xE4` | Ground object-list head selected by probe branch | Yes | `0x0042ADCC` |
| `Cell+0xE8` | Bridge/alternate object-list head selected by probe branch | Yes | `0x0042ADC2` |
| `Cell+0x124` | Occupation/bridge-record byte gating 5x5 candidate toggles | Yes | `0x0042B005..0x0042B00D` |
| `Cell+0x24` | Packed cell coordinate used for own-current-cell skip and probe-center base | Yes | `0x0042AFD5`, `0x0042B00F` |
| `Cell+0x140 & 0x40000` | Temporary A* cost marker toggled by this function | Yes | candidate write `0x0042B029..0x0042B03D`, center write `0x0042B04F..0x0042B063`; cost consumer `0x00429830` in prior reports |

## 3. Core Logic

### 3.1 Probe Direction Source

Active in YR: Yes.

The probe direction is timer-derived, not a `RandomClass` or `RandomRanged` draw:

1. `0x0042AD35`: `ECX = Foot + 0x388`.
2. `0x0042AD3B`: calls `RateTimer__Current @ 0x004C93D0`.
3. `0x0042AD40`: loads the returned dword.
4. `0x0042AD47..0x0042AD4D`: computes `dir = ((value >> 12) + 1) >> 1 & 7`.
5. `0x0042AD50` and `0x0042AD5F`: use `dir * 4 + 0x0089F688` / `+0x0089F68A` to read signed X/Y offsets.

Handoff-critical detail: the `+1` occurs after the `>> 12`, before the second right shift. This is not equivalent to simply using high facing bits without the rounding step.

Evidence: existing Ghidra decompile report plus fresh local disassembly `0x0042AD35..0x0042AD67`; RateTimer helper contract in `RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`. Active in YR: Yes.

### 3.2 Probe Coordinate

Active in YR: Yes.

The probe coordinate is exactly `current_coord + g_DirectionOffsets[dir]`. The current coordinate is fetched from the searching foot object via vtable `+0x1B8` before the RateTimer sample, and the probe cell is resolved through `MapClass::Get_CellClass @ 0x005657A0`.

| Address range | Finding | Active in YR |
|---|---|---|
| `0x0042AD0F..0x0042AD29` | calls foot vtable `+0x1B8`, stores packed current coord, resolves current cell | Yes |
| `0x0042AD50..0x0042AD6C` | adds signed table X/Y to current coord words | Yes |
| `0x0042AD71..0x0042AD8D` | resolves the computed probe coordinate to `CellClass*` and stores it in `EBX` | Yes |

Negative boundary: the selected probe is not based on the caller's destination, current A* expansion direction, ordered path direction, `RandomRanged`, or a scan over all adjacent cells. Active in YR: Yes.

### 3.3 Probe Layer / Object-List Selection

Active in YR: Yes.

The list-choice branch is:

```text
if !(probe.flags & 0x100):
    selected = probe.Cell+0xE4
    bridge_selected = false
else:
    level_gap = abs(current_cell.level - probe.level)
    if level_gap > 3:
        selected = probe.Cell+0xE8
        bridge_selected = true
    else if searching_foot.on_bridge_byte != 0:
        selected = probe.Cell+0xE8
        bridge_selected = true
    else:
        selected = probe.Cell+0xE4
        bridge_selected = false
```

| Address | Detail | Active in YR |
|---|---|---|
| `0x0042AD93..0x0042AD9C` | tests `probe.Cell+0x140 & 0x100`; non-bridge jumps to ground list | Yes |
| `0x0042AD9E..0x0042ADB3` | reads signed levels and computes absolute difference | Yes |
| `0x0042ADB3..0x0042ADB6` | `CMP EAX,3; JG bridge-list`, so only `abs > 3` selects bridge from gap alone | Yes |
| `0x0042ADB8..0x0042ADC0` | if `Foot+0x8C == 0`, ground list; otherwise fall into bridge list | Yes |
| `0x0042ADC2..0x0042ADD4` | bridge list = `Cell+0xE8`, ground list = `Cell+0xE4`, with AL tracking bridge-selected for fallback height | Yes |

Handoff-critical boundary: `abs == 3` does not select bridge by height gap alone. It selects ground unless `Foot+0x8C` is nonzero. Evidence: `CMP EAX,3; JG 0x0042ADC2`. Active in YR: Yes.

If the selected list is null, the fallback call uses the probe coordinate and selected height: `probe.level + 4` when bridge-selected, otherwise `probe.level`.

Evidence: `0x0042ADD4..0x0042ADF1`; existing Ghidra decompile of helper `0x0042B080`. Active in YR: Conditional on selected list null.

### 3.4 5x5 Binding Rules Needed for the Probe

Active in YR: Conditional. The 5x5 phase runs after peer path processing, or when no peer path was processed and `PathfinderClass+0x3C != 1`. The no-peer urgency-1 path zeroes `+0x3C` and returns before this phase.

Evidence: `0x0042AEAD..0x0042AED5` for no-peer/no-write early tail; `0x0042AFCB` begins the 5x5 phase.

When the 5x5 phase runs, it is centered on the selected probe cell (`EBX`), not on the searching unit's current cell:

- outer offset register starts at `-2` and increments while `< 3`;
- inner offset register starts at `-2` and increments while `< 3`;
- candidate coordinate is `probe.Cell+0x24 + (dx, dy)`;
- candidate toggles only if `candidate.Cell+0x124 != 0`;
- if candidate coordinate equals the searching unit's original current coordinate, the candidate is skipped;
- after all 25 candidates, the probe-center cell itself is toggled unconditionally.

Evidence: fresh local disassembly `0x0042AFCB..0x0042B063`; existing Ghidra report section 3.6. Active in YR: Conditional as above.

Tiny parity details:

| Detail | Evidence | Active in YR |
|---|---|---|
| Loop bounds are inclusive `-2..=2` by `start -2`, `inc`, `cmp < 3` | `0x0042AFCB`, `0x0042AFD0`, `0x0042B043..0x0042B04D` | Conditional |
| Only occupied/recorded candidates toggle during the loop | `0x0042B005..0x0042B00D` | Conditional |
| Searching unit's own current cell is skipped even if occupied | `0x0042B00F..0x0042B027` | Conditional |
| Probe center is toggled after the loop regardless of `Cell+0x124` and regardless of own-cell skip | `0x0042B04F..0x0042B063` | Conditional |
| Occupied probe center toggles once in the loop and once at the final center write, so its net state is unchanged; unoccupied probe center toggles once | derived from `0x0042B005..0x0042B063` | Conditional |

### 3.5 Cleanup Pairing on Normal Static A* Paths

Active in YR: Yes when `PathfinderClass+0x3C != 0`.

Normal A* call sites are paired:

- Pre-search call: `0x00429C10..0x00429C1A` tests `Pathfinder+0x3C` and calls `0x0042ACF0` before the main search body.
- Success cleanup: `0x0042A423..0x0042A42D` tests `Pathfinder+0x3C` and calls `0x0042ACF0` before returning the path result.
- Failure cleanup: `0x0042A442..0x0042A44C` tests `Pathfinder+0x3C` and calls `0x0042ACF0` before returning zero.

Evidence: fresh local disassembly of the three ranges plus `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`. Active in YR: Conditional on `+0x3C != 0`, but this is the live A* marker path.

The internal path that can set `Pathfinder+0x3C = 0` occurs before the 5x5 phase and only when no peer marker was processed, so the later cleanup gate is not skipping already-written 5x5 markers.

Evidence: `0x0042AEAD..0x0042AED5`, with 5x5 phase starting later at `0x0042AFCB`; `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`. Active in YR: Conditional.

Runtime abnormal interruption, process abort, or exception cleanup was not proven. Active in YR: not a normal gameplay path in this static slice.

## 4. INI Keys

No INI key directly controls the probe RNG/bucket, `Cell+0xE4/+0xE8` selection, or `Cell+0x140 & 0x40000` marker in this slice.

| Input | Relationship | Active in YR | Evidence |
|---|---|---|---|
| `Speed=` / `TechnoType+0x678` | Already settled by parent as parsed/scaled and used in peer eligibility outside this probe slice | Yes | parent context; not re-investigated |
| Movement/path urgency | Supplies `PathfinderClass+0x3C`, which gates call and 5x5 no-peer behavior | Conditional | `AStar_pathfind_search @ 0x0042C900` in prior reports; cleanup report |
| `RateTimer/FacingClass` state | Runtime object state, not INI; used for probe bucket | Yes | `0x0042AD35..0x0042AD4D` |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | Owns probe selection and `0x40000` toggles | Yes / Conditional on `+0x03` enabled | existing Ghidra report; local disassembly |
| `AStar_main_loop @ 0x00429A90` | Sole verified live caller in prior reports; wraps search with pre/post toggles | Yes / Conditional on `+0x3C` | `0x00429C10`, `0x0042A423`, `0x0042A442` |
| `RateTimer__Current @ 0x004C93D0` | Provides dword from `Foot+0x388` for probe bucket | Yes | call at `0x0042AD3B`; timer helper report |
| `g_DirectionOffsets @ 0x0089F688` | Supplies signed 8-neighbor probe delta | Yes | `0x0042AD50`, `0x0042AD5F`; direction table report |
| `FUN_0042B080` | Fallback object lookup if selected probe list is null | Conditional | `0x0042ADD4..0x0042ADF1`; existing Ghidra decompile |

## 6. Current Rust Implementation Status

Read-only scan only. No Rust files were modified.

| Rust surface | Current status vs scoped finding | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs` | Has layered bridge A*, dual closed arrays, urgency, tube edges, and entity soft-block costs; targeted scan found no per-search `0x40000` bridge-marker overlay or probe-bucket generator | Codegraph `find_path_with_costs`; `rg` on `0x40000`, `urgency`, `entity_block_map`, `tube` |
| `src/sim/pathfinding/zone_search.rs` | Passes urgency/entity block inputs into A* and handles zone/tube precheck; no probe marker generator found | Codegraph `find_path_zoned`; `rg` |
| `src/sim/movement/movement_path.rs` | Builds movement path requests and passes entity blocks / layered block map / urgency; no 5x5 probe overlay found | `rg` |
| `src/sim/movement/movement_blocked.rs` | Escalates blocked repath urgency (`1` traffic penalty, `2` route-around); this is not the same as `UpdateBridgePassability`'s search-scoped marker generation | `rg` lines around urgency comments |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working-notes gate | verified | report section above | none |
| Probe direction source | verified | local disassembly `0x0042AD35..0x0042AD4D`; existing Ghidra report | none |
| Direction table coordinate add | verified | local disassembly `0x0042AD50..0x0042AD8D`; direction table report | none |
| Selected probe list layer | verified | local disassembly `0x0042AD93..0x0042ADD4`; existing Ghidra report | none |
| Fallback helper argument binding | verified | local disassembly `0x0042ADD4..0x0042ADF1`; existing Ghidra helper report | helper internals outside this target |
| 5x5 own-current skip / center rules | verified | local disassembly `0x0042AFCB..0x0042B063`; existing Ghidra report | none for binding rules |
| Normal A* cleanup pairing | verified | local disassembly `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`; cleanup report | abnormal interruption not covered |
| Full peer path propagation loop | deferred | user non-scope | separate swarm slot |
| Runtime distribution sampling of probe buckets | deferred | out-of-scope | live replay/debugger sampling if needed |
| Rust current implementation status | touched-not-exhausted | codegraph + `rg` | implementation pass |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is the investigation mode exhaustive-slice? -> Yes, for the bounded `0x0042AD35..0x0042ADD4` probe/list slice plus required 5x5/cleanup context.` (evidence: scope; Active in YR: Yes / Conditional)
- `[RESOLVED] OQ-2 - Does the probe use pseudo-random/game RNG? -> It uses `RateTimer__Current(Foot+0x388)` bucket bits, not `RandomClass`/`RandomRanged`.` (evidence: `0x0042AD35..0x0042AD4D`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What exact bucket formula selects direction? -> `((value >> 12) + 1) >> 1 & 7`.` (evidence: `0x0042AD47..0x0042AD4D`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - Which coordinate is probed? -> Searching foot current cell coordinate plus `g_DirectionOffsets[dir]`.` (evidence: `0x0042AD0F..0x0042AD8D`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - Which list is selected on non-bridge probe cells? -> Ground list `Cell+0xE4`.` (evidence: `0x0042AD93..0x0042ADCC`; Active in YR: Yes)
- `[RESOLVED] OQ-6 - Which list is selected on bridge probe cells? -> Bridge list `Cell+0xE8` if `abs(current.level - probe.level) > 3` or `Foot+0x8C != 0`; otherwise ground list.` (evidence: `0x0042AD9E..0x0042ADD4`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - Is the height boundary inclusive or exclusive? -> Bridge by height uses `> 3`, so `3` stays ground unless `Foot+0x8C != 0`.` (evidence: `0x0042ADB3..0x0042ADB6`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - If selected list is null, what fallback inputs are passed? -> `0x0042B080(probe+0x24, probe.level + (bridge_selected ? 4 : 0))`.` (evidence: `0x0042ADD4..0x0042ADF1`; Active in YR: Conditional)
- `[RESOLVED] OQ-9 - What is the 5x5 center? -> The pseudo-random probe cell, not the current cell.` (evidence: `0x0042AFD5`; Active in YR: Conditional)
- `[RESOLVED] OQ-10 - Is the searching unit's own current cell marked by the 5x5 candidate loop? -> No; candidate coordinate equal to original current coord is skipped.` (evidence: `0x0042B00F..0x0042B027`; Active in YR: Conditional)
- `[RESOLVED] OQ-11 - Is the probe center subject only to occupation byte? -> No; after the loop, probe center toggles unconditionally.` (evidence: `0x0042B04F..0x0042B063`; Active in YR: Conditional)
- `[RESOLVED] OQ-12 - Are normal A* success/failure paths paired with cleanup? -> Yes; pre-search, success tail, and failure tail all call `0x0042ACF0` when `+0x3C != 0`.` (evidence: `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`; Active in YR: Conditional)
- `[RESOLVED] OQ-13 - Can `+0x3C` zeroing suppress cleanup after marker writes? -> In the scoped static path, zeroing occurs before 5x5 and only when no peer marker was processed.` (evidence: `0x0042AEAD..0x0042AED5`, `0x0042AFCB`; Active in YR: Conditional)
- `[DEFERRED] OQ-14 - Runtime-abnormal interruption cleanup.` (category: `needs-runtime-debugger`; reason: static normal paths are paired, but process abort/exception/thread interruption cannot be proven from this slice; next-step-if-pursued: runtime breakpoint/watchpoint around paired calls)
- `[DEFERRED] OQ-15 - Full peer path propagation loop.` (category: `out-of-scope`; reason: explicitly assigned to a different swarm target; next-step-if-pursued: inspect `0x0042AEF6` peer path slot)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Probe direction is `((RateTimerCurrent(Foot+0x388) >> 12) + 1) >> 1 & 7`, not a `RandomRanged` draw | existing Ghidra report + local disassembly `0x0042AD35..0x0042AD4D`; Active in YR: Yes | missing | `src/sim/pathfinding/core.rs`; any future per-search bridge-marker generator | Drive the marker probe from a deterministic facing/timer bucket or an explicitly compatible shim, not from the sim RNG stream | With a fixed `Foot+0x388` current value yielding bucket `dir`, the marker generator probes exactly `current + g_DirectionOffsets[dir]` and does not advance `SimRng`; proposed test name: `astar_bridge_marker_probe_uses_ratetimer_bucket_not_rng` | Same probability is not enough; using RNG changes replay stream and chosen adjacent cell |
| Probe layer uses the probe cell's `0x100` bridge bit, signed level gap `>3`, and `Foot+0x8C` to select `E4` vs `E8` | local disassembly `0x0042AD93..0x0042ADD4`; Active in YR: Yes | missing | `src/sim/pathfinding/core.rs`; layered entity/object-list lookup surface | Select peer object list by the verified bridge/height/on-bridge branch before fallback lookup | A bridge probe with level gap `3` and `Foot+0x8C=0` scans ground list, while gap `4` or `Foot+0x8C=1` scans bridge list; proposed test name: `astar_bridge_marker_probe_layer_selects_e4_e8_with_gap_gt3` | Do not use `>=3`, current-cell layer, or movement layer alone |
| 5x5 fallback is centered on the probe, toggles occupied non-own candidates, then toggles probe center unconditionally | local disassembly `0x0042AFCB..0x0042B063`; Active in YR: Conditional | missing | `src/sim/pathfinding/core.rs`; temporary marker overlay / entity occupancy snapshot | Add exact 5x5 candidate/own-skip/center-net behavior only on the verified no-peer/urgency conditions | Fixture: own current cell occupied inside the square is unmarked; occupied non-center cell marked; occupied probe center nets unchanged; unoccupied probe center marked; proposed test name: `astar_bridge_marker_5x5_probe_center_and_own_cell_rules` | Do not mark all 25 cells or skip the final center toggle |
| Normal A* static success/failure tails pair the pre-search toggle | local disassembly `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`; Active in YR: Conditional | missing if overlay added | `src/sim/pathfinding/core.rs`; `src/sim/pathfinding/zone_search.rs` callers | Scope temporary marker state to one search and ensure cleanup/restoration runs for both found-path and no-path exits | Two consecutive path searches over the same map have no residual marker state after success or failure; proposed test name: `astar_bridge_marker_overlay_cleans_on_success_and_failure` | Do not persist markers in `PathGrid`, terrain costs, save state, or global bridge state |

## Negative Facts / Do Not Do

- Do not use `RandomClass`, `RandomRanged`, `SimRng`, or modulo RNG for the initial probe. Evidence: `0x0042AD35..0x0042AD4D` calls `RateTimer__Current` and shifts/masks the result. Active in YR: Yes.
- Do not use the searching unit's movement direction, destination direction, or path queue entry to choose this probe. Evidence: `0x0042AD50..0x0042AD8D` computes one adjacent coordinate from `g_DirectionOffsets[RateTimerBucket]`. Active in YR: Yes.
- Do not choose the object-list layer from the current cell or from Rust's eventual A* movement layer alone. Evidence: `0x0042AD93..0x0042ADD4` branches on the probe cell's `0x100`, probe/current levels, and `Foot+0x8C`. Active in YR: Yes.
- Do not implement the height boundary as `>= 3` or `>= 4` without preserving the assembly result. Evidence: `CMP EAX,3; JG bridge-list`, so bridge-by-gap is `abs > 3`. Active in YR: Yes.
- Do not mark all cells in the 5x5 square. Evidence: candidate loop requires `Cell+0x124 != 0`, skips own current coord, and then separately toggles probe center unconditionally. Active in YR: Conditional.

## 10. Remaining Uncertainty

- Runtime-abnormal cleanup under exception/process abort/thread interruption remains unproven; normal static success/failure paths are paired.
- Runtime distribution sampling of `Foot+0x388` bucket values was not performed because the implementation needs exact selection, not probability.
- Original Westwood symbolic names for `Foot+0x388`, `Foot+0x8C`, and the `0x40000` marker are not proven in this slice; behavior and offsets are verified.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` replacement wording for section 5.1 probe source:
  "`UpdateBridgePassability` does not draw from the game RNG for its initial probe. It calls `RateTimer__Current` on `Foot+0x388`, computes `dir = ((value >> 12) + 1) >> 1 & 7`, and probes `current_cell + g_DirectionOffsets[dir]`. The layer list is selected from the probe cell: non-bridge uses `Cell+0xE4`; bridge uses `Cell+0xE8` only when `abs(current.Level - probe.Level) > 3` or `Foot+0x8C != 0`, otherwise `Cell+0xE4`."

## Sources

- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/RATETIMER_CURRENT_FRAME_COUNTER_HELPERS_GHIDRA_REPORT.md`
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`
- Fresh local `gamemd.exe` PE disassembly with Capstone from `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`: `0x0042ACF0..0x0042ADD4`, `0x0042ADD4..0x0042AE10`, `0x0042AEAD..0x0042AED5`, `0x0042AFCB..0x0042B063`, `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/zone_search.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_path.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/movement_blocked.rs`
