# UpdateBridgePassability Peer Path Propagation 0x0042AEF6 - Ghidra Research Report

**Address(es):** `0x0042ACF0` primary; scoped path propagation range `0x0042AE33..0x0042AFB5`; cost consumer `0x00429830`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Peer queued-path marker propagation inside `PathfinderClass::UpdateBridgePassability`, specifically kind `1` vs kind `0xF` setup, peer object `+0x558` start/reference coordinate, peer object `+0x5E0` direction queue, exact 24-entry and `-1` terminator behavior, direction `8` tube handling, and the `CellClass+0x140 bit 0x40000` cells toggled by the path replay loop.  
**Non-Scope:** Speed gate provenance beyond settled facts, probe-cell RNG, 5x5 fallback phase, cleanup-tail proof, full A* cost model, full TubeClass producer invariants, and Rust implementation patches.  
**Confidence:** High for the scoped branch and constants. Fresh Ghidra MCP tools were unavailable in this session, so this report combines existing Ghidra-backed decompile reports with fresh local read-only PE disassembly of `gamemd.exe` for the requested address range.  
**Active in YR:** Yes, conditional on live A* calling `PathfinderClass::UpdateBridgePassability` with `PathfinderClass+0x3C != 0`; prior caller reports verify the live path, and this report verifies the scoped branch body.

## Working Notes

Target question: How exactly does `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` replay a peer object's queued path at `0x0042AEF6..0x0042AFB5` and toggle temporary `CellClass+0x140 & 0x40000` markers?

Non-goals: Do not re-investigate the settled speed/urgency gate, probe RNG, 5x5 fallback, cleanup tails, or implement Rust.

Evidence needed to mark COMPLETE: Existing decompile evidence plus fresh disassembly for kind setup, `+0x558`, `+0x5E0`, loop bound/terminator, direction `8`, marker write, YR liveness, and Rust-facing delta.

Stop conditions: Stop once every scoped byte-range behavior is resolved with address evidence; defer only wider TubeClass producer/runtime debugger questions; write only this report and the shared claims file.

## 1. Overview

The scoped branch replays a nearby peer object's already-queued path directions and XOR-toggles `CellClass+0x140 bit 0x40000` on each replayed destination cell. It does not walk the searcher's own path, does not block cells, and does not persist a static `PathGrid` flag.

Active in YR: Yes. Prior Ghidra reports verify `FootClass::Run_AStar -> AStar_pathfind_search -> AStar_main_loop -> UpdateBridgePassability`; this report's local disassembly confirms the requested propagation instructions in the retail `gamemd.exe` image.

## 2. Key Offsets / Constants

| Item | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| peer vtable `+0x2C` return `1` | Eligible object kind using two-entry prerequisite | Yes | `0x0042AE1A..0x0042AE20`, `0x0042AE82..0x0042AE93` disassembly |
| peer vtable `+0x2C` return `0xF` | Eligible object kind using three-entry prerequisite | Yes | `0x0042AE22..0x0042AE2D`, `0x0042AED8..0x0042AEEF` disassembly |
| peer object `+0x558` | Initial replay/reference packed cell coordinate | Yes | `0x0042AE33..0x0042AE3B` disassembly |
| peer object `+0x5E0` | Direction queue base, `int[24]` style entries | Yes | `0x0042AE88`, `0x0042AF9E`, `0x0042AFB0` disassembly |
| `0x18` | Maximum processed peer path entries, 24 decimal | Yes | `0x0042AEF6..0x0042AEF9` disassembly |
| `-1` | Queue terminator / prerequisite failure sentinel | Yes | `0x0042AE90`, `0x0042AE95`, `0x0042AEDD`, `0x0042AEE1`, `0x0042AEE9`, `0x0042AFB2` disassembly |
| direction `8` | Tube jump sentinel, not a compass offset | Conditional | `0x0042AF01..0x0042AF3F` disassembly |
| `CellClass+0x116` | Signed tube index; only `-1` is checked in this consumer | Conditional | `0x0042AF15..0x0042AF27` disassembly |
| `g_TubeArray`, `Tube+0x28` | Direction-8 destination coordinate source | Conditional | `0x0042AF21..0x0042AF2D`; `GDIRECTIONOFFSETS_0089F688...` |
| `CellClass+0x140 & 0x40000` | Temporary A* bridge/crowd cost marker toggled per replayed destination | Yes | `0x0042AF8D..0x0042AFAE`; reader `0x00429830` in prior report |

## 3. Core Logic

### 3.1 Peer Entry And Kind Setup

Active in YR: Yes, when a candidate peer object is found by the live `UpdateBridgePassability` object scan.

The function accepts only peers whose vtable `+0x2C` returns `1` or `0xF`. Other kinds branch to the next object via `peer+0x30`.

Before the settled speed/type/playfield gate, the peer's `+0x558` dword is copied to a stack coordinate and becomes the replay cursor. This happens before queue validation, so both kind paths start from the same peer reference coordinate.

For kind `1`, the function sets `EDI = peer + 0x5E0`, requires `path[0] != -1` and `path[1] != -1`, then enters the shared replay loop with the first processed entry still at `path[0]`.

For kind `0xF`, the function also sets `EDI = peer + 0x5E0`, requires `path[0] != -1`, `path[1] != -1`, and `path[2] != -1`, then enters the same loop with the first processed entry still at `path[0]`.

Evidence: existing decompile in `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`; fresh local disassembly `0x0042AE15..0x0042AE33`, `0x0042AE7D..0x0042AEF1`.

### 3.2 Replay Loop Bounds And Terminator

Active in YR: Yes.

The shared replay loop has two stop conditions:

- At loop top, `EBX` is compared to `0x18`; if `EBX >= 24`, replay stops before reading/processing another path entry.
- After each processed entry, the queue pointer advances by 4 bytes and the next dword is compared with `-1`; if it is not `-1`, the loop repeats.

This means a valid path with no earlier terminator processes at most entries `path[0]..path[23]`. `path[24]` is never processed by this loop. The post-step `-1` check controls whether to continue, not whether the already-processed entry was valid; the kind-specific prerequisites guarantee enough initial entries before the first loop iteration.

Evidence: `0x0042AEF6..0x0042AEFF`, `0x0042AF9E..0x0042AFB5` disassembly. Active in YR: Yes.

### 3.3 Direction `0..7`

Active in YR: Yes, when replayed peer queue entries are normal direction ids.

Any entry other than `8` indexes `g_DirectionOffsets @ 0x0089F688` directly as a signed `(dx, dy)` pair. The pair is added to the current replay coordinate, the stack replay cursor is updated, and the resulting destination coordinate is passed to `MapClass::Get_CellClass`.

The table order is N, NE, E, SE, S, SW, W, NW from the sibling direction-table report. This report did not re-open the table initializer beyond using that settled fact.

Evidence: `0x0042AF41..0x0042AF69` disassembly; `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`. Active in YR: Yes.

### 3.4 Direction `8` Tube Handling

Active in YR: Conditional, only when the peer queue entry is exactly `8`.

Direction `8` does not index `g_DirectionOffsets`. The function resolves the current replay coordinate to a `CellClass`, reads signed `CellClass+0x116`, and checks only whether it is `-1`.

If the tube index is not `-1`, the function loads `g_TubeArray[index]`, reads `TubeClass+0x28`, and uses that packed coordinate as the next replay cursor and marker destination. If the tube index is `-1`, it writes packed coordinate `(0,0)` and uses that as the next replay cursor and marker destination.

No upper-bound compare against tube count appears in this consumer before indexing `g_TubeArray`; producer validity is outside this slice.

Evidence: `0x0042AF01..0x0042AF3F` disassembly; direction-8 sibling reports. Active in YR: Conditional.

### 3.5 Marker Write / Output Cell

Active in YR: Yes.

After each replayed step, the function calls `MapClass::Get_CellClass` twice with the same updated replay coordinate. It reads `Cell+0x140` from both returned cells and applies a masked XOR:

`new_flags = dest_flags ^ ((~same_dest_flags ^ dest_flags) & 0x40000)`.

Because both `Get_CellClass` calls use the same updated coordinate in this instruction sequence, the scoped `0x40000` effect is equivalent to toggling the replay destination cell's `0x40000` bit once per processed path entry. If the replay visits the same destination coordinate twice, the bit toggles twice and nets back to its previous value.

Evidence: `0x0042AF69..0x0042AFAE` disassembly; especially `0x0042AF77`, `0x0042AF88`, `0x0042AF8D`, `0x0042AF93`, `0x0042AFA5`, `0x0042AFAB`, `0x0042AFAE`. Active in YR: Yes.

## 4. INI Keys

No INI key directly configures this peer path propagation loop or `CellClass+0x140 & 0x40000`.

Settled inputs from sibling reports:

| Input | Relationship | Active in YR | Evidence |
|---|---|---|---|
| `Speed=` | Parsed/scaled to `TechnoTypeClass+0x678`; used by the normal peer eligibility gate before this loop | Yes | `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md` |
| `[Tubes]` map data | Populates tube metadata consumed indirectly when peer queue entry is `8` | Conditional | `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`; current Rust tube parser scan |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `AStar_main_loop @ 0x00429A90` | Calls `UpdateBridgePassability` around live A* searches when `Pathfinder+0x3C != 0` | Conditional | prior `PATHFINDER_UPDATE...` and cleanup-tail reports |
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | Scans peer objects and executes the scoped replay loop | Yes | existing decompile; fresh disassembly |
| `MapClass::Get_CellClass @ 0x005657A0` | Resolves replay coordinates to cells for tube and marker writes | Yes | calls at `0x0042AF10`, `0x0042AF77`, `0x0042AF88` |
| `AStar_compute_edge_cost @ 0x00429830` | Reads destination `0x40000` and applies 4x cost | Yes | prior reports |

## 6. Current Rust Implementation Status

Read-only scan only.

| Rust surface | Current status vs scoped finding | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs` | Has `MAX_PATH_SEGMENT_STEPS = 24`, `PathGrid`, `entity_block_map`, and `urgency`, but no search-scoped peer queued-path marker overlay equivalent to `0x40000` replay | `rg` scan |
| `src/sim/movement/bump_crush.rs` | Builds `entity_block_map` from current/next occupancy-style blockers, not from replaying a peer's full `+0x5E0` direction queue | `rg` scan |
| `src/sim/movement/movement_path.rs` / `zone_search.rs` | Passes `entity_block_map` and `urgency` into pathfinding; no separate cost-marker overlay parameter was found | `rg` scan |
| `src/map/tube_facts.rs`, `src/sim/pathfinding/core.rs` tube support | Tube exits exist for normal path expansion, but marker replay semantics are not a normal tube edge cost | sibling direction/tube reports and source scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes four-line gate | verified | Section "Working Notes" | none |
| `0x0042ACF0` YR liveness | verified from prior Ghidra reports | `PATHFINDER_UPDATE...`, cleanup-tail report | none for this slice |
| Kind `1` setup | verified | decompile plus disassembly `0x0042AE7D..0x0042AE9C` | none |
| Kind `0xF` setup | verified | decompile plus disassembly `0x0042AED8..0x0042AEEF` | none |
| Peer `+0x558` replay start | verified | disassembly `0x0042AE33..0x0042AE3B` | none |
| Peer `+0x5E0` queue base | verified | disassembly `0x0042AE88`, `0x0042AF9E`, `0x0042AFB0` | none |
| 24-entry bound | verified | disassembly `0x0042AEF6..0x0042AEF9` | none |
| `-1` terminator | verified | disassembly `0x0042AFB0..0x0042AFB5` plus prerequisites | none |
| Direction `8` tube branch | verified | disassembly `0x0042AF01..0x0042AF3F` | producer invariants out of scope |
| Marker destination toggle | verified | disassembly `0x0042AF69..0x0042AFAE` | none |
| Current Rust status | verified enough for handoff | `rg` source scan | exact design remains future work |
| Fresh live Ghidra MCP decompile | deferred | tool discovery returned no Ghidra tools | not needed for scoped completion because existing decompile plus fresh binary disassembly cover claims |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is this branch active in YR? -> Yes, through live A* when `PathfinderClass+0x3C != 0`.` (evidence: prior caller reports `0x00429A90 -> 0x0042ACF0`; Active in YR: Conditional)
- `[RESOLVED] OQ-2 - Which peer kinds enter the path replay setup? -> Only vtable `+0x2C` return `1` or `0xF`; others skip to next peer.` (evidence: `0x0042AE15..0x0042AE2D`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What is the replay start coordinate? -> Peer object dword `+0x558` copied before replay.` (evidence: `0x0042AE33..0x0042AE3B`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - Where is the direction queue? -> Peer object `+0x5E0`, advanced in 4-byte entries.` (evidence: `0x0042AE88`, `0x0042AF9E`, `0x0042AFB0`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - How does kind `1` differ from kind `0xF`? -> Kind `1` requires first two queue entries non-`-1`; kind `0xF` requires first three; both process starting at `path[0]`.` (evidence: `0x0042AE90..0x0042AE9C`, `0x0042AED8..0x0042AEEF`; Active in YR: Yes)
- `[RESOLVED] OQ-6 - What is the maximum processed length? -> 24 entries, because `EBX >= 0x18` exits before processing another entry.` (evidence: `0x0042AEF6..0x0042AEF9`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - What terminates early? -> After each processed entry, next dword `-1` stops replay.` (evidence: `0x0042AFB0..0x0042AFB5`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - What does direction `8` do? -> Reads current cell `+0x116`; valid tube jumps to `Tube+0x28`, `-1` resets to `(0,0)`.` (evidence: `0x0042AF01..0x0042AF3F`; Active in YR: Conditional)
- `[RESOLVED] OQ-9 - Is direction `8` bounds-checked against tube count here? -> No; only `idx == -1` is checked in this consumer.` (evidence: `0x0042AF15..0x0042AF27`; Active in YR: Conditional)
- `[RESOLVED] OQ-10 - Which cell is toggled? -> The updated replay destination coordinate's `Cell+0x140 & 0x40000` is toggled once per processed entry.` (evidence: `0x0042AF69..0x0042AFAE`; Active in YR: Yes)
- `[RESOLVED] OQ-11 - Does the source/reference cell receive a separate inverse marker? -> No for the scoped `0x40000` bit; both cell lookups use the same updated coordinate in the observed write sequence.` (evidence: `0x0042AF69..0x0042AFAE`; Active in YR: Yes)
- `[RESOLVED] OQ-12 - Does Rust already have this peer marker overlay? -> No obvious equivalent found; existing `entity_block_map`/urgency is not queued-path marker propagation.` (evidence: source scan; Active in YR: Rust status only)
- `[DEFERRED] OQ-13 - What producer invariant prevents positive out-of-range `Cell+0x116` values?` (category: `out-of-scope`; reason: this slice verifies the consumer branch only; next-step-if-pursued: TubeClass producer/lifecycle audit)
- `[DEFERRED] OQ-14 - Can a live Ghidra MCP decompile be re-run in this exact session?` (category: `requires-different-system-context`; reason: no Ghidra MCP tools were exposed; next-step-if-pursued: rerun with Ghidra MCP enabled if the parent requires fresh decompiler output)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Eligible peer path replay starts at peer `+0x558`, reads direction entries from peer `+0x5E0`, processes `path[0]` first, and uses kind-specific prerequisites: kind `1` needs two non-`-1` entries, kind `0xF` needs three. | `0x0042AE33..0x0042AE3B`, `0x0042AE88..0x0042AEF1`; Active in YR: Yes | missing | `src/sim/pathfinding/core.rs`; movement/entity path snapshot surface | Future marker overlay must consume a peer queued-path snapshot, not recompute from the peer's current cell or only next step. | Peer kind `1` with `[E,S,-1]` marks the first two destinations; kind `0xF` with `[E,S,-1]` marks nothing but `[E,S,W,-1]` starts at `E`. Proposed test: `astar_bridge_peer_marker_kind_prerequisites_start_at_path0`. | Do not skip `path[0]` or treat the prerequisite entries as consumed setup bytes. |
| The replay loop processes at most 24 entries and stops early only when the next queue entry is `-1` after a processed step. | `0x0042AEF6..0x0042AEF9`, `0x0042AF9E..0x0042AFB5`; Active in YR: Yes | partial constants exist (`MAX_PATH_SEGMENT_STEPS = 24`) but no marker replay | `src/sim/pathfinding/core.rs` future overlay helper | Bound marker replay exactly to 24 processed dword entries and preserve duplicate-coordinate XOR parity. | A 25-entry non-terminating peer queue marks only the first 24 replay destinations; a repeated destination toggled twice is not marked at the end. Proposed test: `astar_bridge_peer_marker_replay_caps_at_24_and_xors_duplicates`. | Do not iterate until vector length, path length, or 25 entries; do not use set insertion if duplicate toggles must cancel. |
| Direction `8` in the marker replay jumps via current cell `Cell+0x116` to `Tube+0x28`; `-1` tube index resets the replay coordinate to `(0,0)`. | `0x0042AF01..0x0042AF3F`; Active in YR: Conditional | missing for marker overlay; normal tube edge support exists separately | `src/sim/pathfinding/core.rs`; tube facts/PathGrid metadata | Marker replay must treat `8` as a tube sentinel separate from normal neighbor offsets and mark only the resulting destination coordinate. | A peer queue `[E,8]` from a cell whose tube exit is `(10,4)` toggles `(10,4)` after the first step; `[8]` from a no-tube cell toggles `(0,0)`. Proposed test: `astar_bridge_peer_marker_direction8_uses_tube_exit_or_origin`. | Do not use `direction & 7`, do not mark the tube entry/intermediate path, and do not silently skip missing-tube direction `8` if modeling this consumer. |

### Negative Facts / Do Not Do

- Do not treat kind `1` and kind `0xF` as having identical queue prerequisites. Evidence: kind `1` checks `+0x5E0/+0x5E4`; kind `0xF` checks `+0x5E0/+0x5E4/+0x5E8`. Active in YR: Yes.
- Do not skip `path[0]` after validating prerequisites. Evidence: both kind paths enter with `EDI = peer+0x5E0`, and first loop read is `[EDI]`. Active in YR: Yes.
- Do not implement marker replay as hard blocking or occupancy. Evidence: the scoped write only toggles `Cell+0x140 & 0x40000`; prior cost consumer verifies this is a 4x cost marker. Active in YR: Yes.
- Do not store replay markers in persistent `PathGrid`, bridge runtime state, zones, or save data. Evidence: prior A* caller reports verify pre/post temporary toggler lifecycle, and the scoped write is XOR. Active in YR: Yes.
- Do not turn direction `8` into a ninth compass offset or wrap it with `& 7`. Evidence: exact branch at `0x0042AF01` enters tube handling. Active in YR: Conditional.

## 10. Remaining Uncertainty

- No uncertainty remains for the scoped propagation loop behavior.
- Wider TubeClass producer guarantees for positive out-of-range `Cell+0x116` values remain outside this report.
- Fresh live Ghidra MCP decompilation could not be run because no Ghidra MCP tools were exposed; the scoped bytes were instead verified by local PE disassembly plus existing Ghidra-backed reports.

## Stale Docs / Follow-up Docs

No stale-doc replacement wording was found for the scoped peer propagation facts. The prior `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` already states the high-level behavior; this report narrows and confirms the kind-specific prerequisites and replay-loop details.

## Sources

- Local read-only PE disassembly of `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, image base `0x00400000`, range `0x0042AE00..0x0042AFC0`.
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- Existing Ghidra report: `C:/Users/enok/Documents/ra2-rust-game-docs/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`.
- Existing trace: `C:/Users/enok/Documents/ra2-rust-game-docs/traces/DIRECTION_8_SENTINEL_INVALID_BYTES_TRACE.md`.
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/zone_search.rs`.
