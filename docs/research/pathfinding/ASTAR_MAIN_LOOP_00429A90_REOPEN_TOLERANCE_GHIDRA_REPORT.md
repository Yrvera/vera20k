# AStar Main Loop 0x00429A90 Reopen/Tolerance -- Ghidra Research Report

**Address(es):** `0x00429A90` primary, `0x0042A460` node creation, `0x0042C900` caller
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** closed-list rejection/reopen behavior inside `AStar_main_loop`, including dual ground/bridge closed arrays, additive `1.009`, direction epsilon, direction order, and bridge-vs-ground layer interaction.
**Non-Scope:** edge-cost helper internals except caller placement, Zone_precheck route choice, UpdateBridgePassability internals, path smoothing, and full locomotor command integration.
**Confidence:** High
**Active in YR:** Yes

## 0. Investigation Contract

Target question: exactly how does `AStar_main_loop @ 0x00429A90` reject or allow neighbor work when a ground/bridge closed-list entry already exists, and how do `1.009`, direction epsilon, direction order, and separate bridge/ground lists affect cell-level tie tests?

Non-goals: do not redo `AStar_compute_edge_cost`, `Zone_precheck`, bridge passability toggle internals, low-bridge tube lifecycle, path smoothing, or Rust implementation.

Evidence needed to mark COMPLETE: decompile plus assembly for the closed-list compare, decompile plus assembly for the later insertion guard, binary bytes or prior verified memory for constants, caller evidence that the function is on the standard YR path, and a Rust-facing handoff for cell-level tie tests.

Stop conditions: stop after the scoped branch behavior is resolved; mark partial if the closed-list branch cannot be distinguished from true reopening or if constants cannot be verified.

## 1. Overview

`AStar_main_loop` uses separate per-cell arrays for ground and bridge layers. For each neighbor, it first chooses the target layer from the current path height and the neighbor cell's bridge flag/level. It then checks the matching closed marker array. If that layer/cell is already closed, the binary compares the stored accumulated cost against `current_node.g + 1.009`.

The important correction is that this is **not a normal A* reopen path**. If a closed layer/cell is "good enough", the branch skips the neighbor immediately. If it is not "good enough", the function may still run legality/cost work and blocked-goal fallback logic, but the later insertion guard still refuses to create a new node while the marker equals the current search epoch.

## 2. Key Offsets / Data

| Offset / address | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `Pathfinder+0x18` | ground closed marker array, epoch per cell | `0x00429ECF`, `0x0042A12D` | Yes |
| `Pathfinder+0x1C` | bridge closed marker array, epoch per cell | `0x00429F04`, `0x0042A13F` | Yes |
| `Pathfinder+0x24` | ground stored accumulated `g` cost array | `0x00429EE6`, `0x0042A13A`, `0x0042A156` | Yes |
| `Pathfinder+0x20` | bridge stored accumulated `g` cost array | `0x00429F1B`, `0x0042A14C`, `0x0042A156` | Yes |
| `Pathfinder+0x28` | current search epoch | marker comparisons/writes at `0x00429EDD`, `0x0042A137`, `0x0042A149` | Yes |
| `Pathfinder+0x30` | current path height; updated when next heap node becomes current | `0x0042A3C1..0x0042A3C6` | Yes |
| `node+0x04` | accumulated `g` cost from start, not heap `f` | `AStar_create_node 0x0042A529..0x0042A530`; stored to arrays at `0x0042A153..0x0042A156` | Yes |
| `node+0x08` | heap priority `f = g + heuristic` | `AStar_create_node 0x0042A589..0x0042A59E`; heap compares `[node+8]` | Yes |
| `0x007E37C0` | double `1.009` | bytes `be 9f 1a 2f dd 24 f0 3f`; read at `0x00429EEC`, `0x00429F21` | Yes |
| `0x0081872C` | 9 float direction epsilon table | bytes decode to `[0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008,0.0]`; read at `0x00429F96` for directions 0..7 | Yes |
| `0x007E3774` | 8 neighbor cell-pointer offsets | bytes decode to N, NE, E, SE, S, SW, W, NW on a 512-wide map | Yes |

## 3. Core Logic

### 3.1 Layer choice happens before closed-list lookup

For every normal neighbor, `AStar_main_loop` chooses a one-byte layer flag at `ESP+0x60`.

Verified behavior:

```
ground layer if:
  neighbor is not a bridge cell
  OR abs(Pathfinder+0x30 - neighbor.Level) < 2

bridge layer if:
  neighbor has CellClass+0x140 & 0x100
  AND abs(Pathfinder+0x30 - neighbor.Level) >= 2
```

Evidence: decompile `0x00429A90`; assembly `0x00429E54..0x00429E7A`.

Active in YR: Yes. This is in the standard `AStar_pathfind_search -> AStar_main_loop` path and is not TS-gated.

### 3.2 The closed-list compare uses stored `g`, not heap `f`

Prior docs often call `Pathfinder+0x20/+0x24` "f-cost arrays." The scoped evidence shows they store `node+0x04`, the accumulated path cost from start.

Evidence:

- `AStar_create_node @ 0x0042A529..0x0042A530`: for non-root nodes, `node+0x04 = parent.node+0x04 + step_cost`.
- `AStar_create_node` then computes `node+0x08 = node+0x04 + heuristic`, used by heap comparisons.
- `AStar_main_loop @ 0x0042A153..0x0042A156`: after creating a node, it stores `created_node+0x04` into either `Pathfinder+0x24` or `Pathfinder+0x20`.
- Closed-list tolerance reads load `FLD float ptr [EDX + 0x4]`, where `EDX` is current node, and compare against the stored array value.

Active in YR: Yes. The same node fields are used in the main live heap and closed arrays.

### 3.3 Ground closed-list tolerance branch

Ground branch evidence:

```asm
00429ecf: MOV ECX,dword ptr [ESI + 0x18]    ; ground marker array
00429edd: CMP dword ptr [ECX + EBP],EAX     ; marker == epoch?
00429ee0: JNZ 0x00429f37                    ; not closed -> skip tolerance
00429ee2: MOV EDX,dword ptr [ESP + 0x14]    ; current node
00429ee6: MOV EAX,dword ptr [ESI + 0x24]    ; ground g array
00429ee9: FLD float ptr [EDX + 0x4]         ; current.g
00429eec: FADD double ptr [0x007e37c0]      ; + 1.009
00429ef2: FLD float ptr [EAX + EBP]         ; existing stored g
00429ef5: FCOMPP
00429ef7: FNSTSW AX
00429ef9: TEST AH,0x1
00429efc: JNZ 0x0042a1a1                    ; if existing_g < current_g + 1.009, reject neighbor now
00429f02: JMP 0x00429f37                    ; otherwise continue to legality/cost work
```

Verified behavior: for an already-closed ground layer cell, `existing_g < current_g + 1.009` causes an immediate neighbor skip.

Active in YR: Yes. This is inside normal neighbor expansion.

### 3.4 Bridge closed-list tolerance branch is identical but uses bridge arrays

Bridge branch evidence:

```asm
00429f04: MOV ECX,dword ptr [ESI + 0x1c]    ; bridge marker array
00429f12: CMP dword ptr [ECX + EBP],EAX
00429f15: JNZ 0x00429f37                    ; not closed -> skip tolerance
00429f17: MOV EDX,dword ptr [ESP + 0x14]    ; current node
00429f1b: MOV EAX,dword ptr [ESI + 0x20]    ; bridge g array
00429f1e: FLD float ptr [EDX + 0x4]
00429f21: FADD double ptr [0x007e37c0]
00429f27: FLD float ptr [EAX + EBP]
00429f2a: FCOMPP
00429f2c: FNSTSW AX
00429f2e: TEST AH,0x1
00429f31: JNZ 0x0042a1a1
```

Verified behavior: ground and bridge layer entries do not block each other. The same map cell can be closed once as ground and once as bridge in one search because marker/cost arrays are separate.

Active in YR: Yes. The bridge branch is reached for high bridge cells when the current path height is at least two levels away from the cell ground level.

### 3.5 This is not true reopening

The second guard after legality/cost calculation still requires the chosen marker array entry to be **not** the current epoch before node creation.

Ground insertion guard:

```asm
00429ffb: MOV EAX,dword ptr [ESI + 0x18]
00429ffe: MOV ECX,dword ptr [EAX + EBP]
0042a001: MOV EAX,dword ptr [ESI + 0x28]
0042a004: CMP ECX,EAX
0042a006: JNZ 0x0042a01e   ; only not-current-epoch can create a node
0042a008: JMP 0x0042a1a1   ; already closed -> no node
```

Bridge insertion guard:

```asm
0042a00d: MOV EDX,dword ptr [ESI + 0x1c]
0042a010: MOV ECX,dword ptr [ESI + 0x28]
0042a013: MOV EAX,dword ptr [EDX + EBP]
0042a016: CMP EAX,ECX
0042a018: JZ 0x0042a1a1    ; already closed -> no node
0042a01e: ... CALL 0x0042a460
```

Verified behavior: an already-closed layer/cell is never inserted into the heap again on this path. The `1.009` compare controls whether the function skips immediately or proceeds to the later legality/cost and blocked-goal logic before refusing insertion.

Active in YR: Yes. Both checks are in the normal expansion loop.

### 3.6 Why the "not good enough" closed branch still does work

If `existing_g >= current_g + 1.009`, the branch continues to `Can_Enter_Cell`, edge-cost computation, and special blocked-goal fallback. For passable neighbors with result `< 7`, the later marker guard prevents node creation. For impassable result `>= 7`, the code can still take the near-goal success path if the neighbor is the destination and the source/destination heights are within one level.

Evidence:

- `0x00429F37..0x00429FEA`: calls `Can_Enter_Cell` and computes edge/tube cost.
- `0x00429FEA`: branches to blocked-goal logic when `Can_Enter_Cell` result is `>= 7`.
- `0x0042A17D..0x0042A19B`: if the blocked neighbor is the goal and `abs(Pathfinder+0x30 - Pathfinder+0x34) < 2`, the function accepts the current node as the path end.

Active in YR: Yes. This is standard pathfinding behavior and matters when the requested target cell is blocked.

### 3.7 Direction order and epsilon

The expansion counter `iStack_44` runs `0..8`.

Normal directions:

| Direction index | Direction | Epsilon |
|---:|---|---:|
| 0 | N | 0.001 |
| 1 | NE | 0.005 |
| 2 | E | 0.002 |
| 3 | SE | 0.006 |
| 4 | S | 0.003 |
| 5 | SW | 0.007 |
| 6 | W | 0.004 |
| 7 | NW | 0.008 |

Direction 8 is the tube edge. It does not call `AStar_compute_edge_cost`, does not add the epsilon table value, and uses Chebyshev cell distance to the tube exit.

Evidence:

- Direction loop bound: decompile `iStack_44 < 9`; assembly `0x0042A1A1..0x0042A1AD`.
- Normal edge cost placement: `0x00429F8A` calls `AStar_compute_edge_cost`; `0x00429F8F` multiplies by `Pathfinder+0x04`; `0x00429F96` adds `0x0081872C[dir]`.
- Epsilon bytes: `0x0081872C` read as `6f12833a 0ad7a33b 6f12033b a69bc43b a69b443b 4260e53b 6f12833b 6f12033c 00000000`.
- Tube branch: `0x00429FA3..0x00429FE6`.

Active in YR: Yes for all normal A* calls. Direction 8 is active only on cells with a valid tube index.

### 3.8 Heap tie behavior around a newly-created candidate

Within one neighbor pass, the binary keeps a local candidate pointer. If a second candidate is found before the first is committed, it compares `node+0x08` heap priorities and only inserts/replaces based on strict floating comparisons. Equal `f` values are not explicitly reordered by zone id or coordinate.

Evidence:

- `0x0042A030` creates node.
- `0x0042A04C..0x0042A05F` compares new candidate `f` against the local candidate.
- Heap sift-up/sift-down comparisons use strict `FCOMP`/`TEST AH,0x41` patterns around `0x0042A081`, `0x0042A29D`, and `0x0042A39D`.

Active in YR: Yes. This shapes deterministic first-found behavior after epsilon and insertion order.

## 4. INI Keys

No INI key is read in this scoped branch. Movement-zone and locomotor fields are prepared by callers and other helpers, but the `1.009` tolerance, layer arrays, and direction epsilon table are binary constants/data.

| Key / data | Effect in this slice | Active in YR |
|---|---|---|
| None | No direct INI reader in the closed-list/tolerance branch | Yes |

## 5. Integration Points

`AStar_pathfind_search @ 0x0042C900` calls `AStar_main_loop` after resolving bridge-aware start/destination cells and optional `Zone_precheck`. The caller sets `Pathfinder+0x3C` from its urgency parameter, but the scoped closed-list tolerance branch has no TS/fog/special-mode gate.

Evidence: decompile `0x0042C900`; main call appears as `AStar_main_loop(param_2,param_3,piVar1,param_5,param_6,param_8)`.

Active in YR: Yes. This is the normal FootClass A* path used by standard skirmish movement.

## 6. Current Rust Implementation Status

| Rust surface | Observed status |
|---|---|
| `src/sim/pathfinding/core.rs::astar_search` | Implements separate ground/bridge arrays and push-time layer selection. |
| `src/sim/pathfinding/core.rs::DIR_TIEBREAK` | Matches the binary epsilon order scaled to integers: N, NE, E, SE, S, SW, W, NW = 1,5,2,6,3,7,4,8. |
| `src/sim/pathfinding/core.rs` closed handling | Current code skips any already-closed layer/cell immediately. It does not model the binary's `existing_g < current_g + 1.009` early-skip distinction or the "closed but not within tolerance may still reach blocked-goal fallback work" path. |
| `src/sim/pathfinding/core.rs` update rule | Uses `tentative_g < g_array[n_idx]` before closure. Binary marks a node closed when pushed/accepted into the search arrays, and later insertion requires marker != current epoch. |
| `src/sim/pathfinding/core_tests.rs` | Has many bridge/layer and marker tests. No focused test appears to distinguish "closed good-enough skip" from "closed but still checked for blocked-goal fallback" or to prove no true closed-node reopen. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_main_loop` closed-list layer selection | verified | `0x00429E54..0x00429E7A` | none for this slice |
| Ground closed tolerance compare | verified | `0x00429ECF..0x00429F02`; `0x007E37C0` bytes | none |
| Bridge closed tolerance compare | verified | `0x00429F04..0x00429F31`; `0x007E37C0` bytes | none |
| Later insertion guard | verified | `0x00429FFB..0x0042A01E` | none |
| Node `g` vs `f` fields | verified | `0x0042A529..0x0042A530`; heap uses `node+0x08`; array store `0x0042A153..0x0042A156` | exact final instruction after sqrt not separately disassembled because decompile plus heap evidence is enough for this scope |
| Direction order and epsilon | verified | `0x00429F96`; `0x0081872C` bytes; `0x0042A1A1..0x0042A1AD` | none |
| Direction 8 tube bypass | verified | `0x00429FA3..0x00429FE6` | tube lifecycle out of scope |
| Caller/YR activity | verified | decompile `0x0042C900` | no broader caller tree needed |
| Rust implementation comparison | touched-not-exhausted | `rg` and focused reads of `core.rs`, `core_tests.rs` | no tests run and no Rust changes made |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Is `AStar_main_loop @ 0x00429A90` active in YR? -> Yes, it is called by `AStar_pathfind_search @ 0x0042C900` on the standard pathfinding spine.` (evidence: `0x0042C900` decompile)
- `[RESOLVED] OQ-2 -- Which arrays are used for ground vs bridge closed markers? -> Ground uses `+0x18`; bridge uses `+0x1C`.` (evidence: `0x00429ECF`, `0x00429F04`, `0x0042A12D`, `0x0042A13F`)
- `[RESOLVED] OQ-3 -- Which arrays store the compared cost? -> Ground uses `+0x24`; bridge uses `+0x20`.` (evidence: `0x00429EE6`, `0x00429F1B`, `0x0042A13A`, `0x0042A14C`)
- `[RESOLVED] OQ-4 -- Is the stored cost `g` or `f`? -> It is `g` (`node+0x04`), not heap `f` (`node+0x08`).` (evidence: `0x0042A529..0x0042A530`, `0x0042A153..0x0042A156`)
- `[RESOLVED] OQ-5 -- What is the tolerance constant? -> Double `1.009`, bytes `be 9f 1a 2f dd 24 f0 3f`.` (evidence: memory `0x007E37C0`, reads `0x00429EEC`, `0x00429F21`)
- `[RESOLVED] OQ-6 -- What exact comparison skips a closed neighbor? -> If `existing_g < current_g + 1.009`, skip the neighbor immediately.` (evidence: `0x00429EE9..0x00429EFC`, `0x00429F1E..0x00429F31`)
- `[RESOLVED] OQ-7 -- Does the binary reopen closed cells? -> No true heap reinsertion occurs while the layer marker equals the current epoch; later insertion guard requires marker != epoch.` (evidence: `0x00429FFB..0x0042A01E`)
- `[RESOLVED] OQ-8 -- Do ground and bridge closed entries interact? -> No; they use separate marker and cost arrays selected by the layer byte.` (evidence: `0x00429EC7..0x00429F31`, `0x0042A12D..0x0042A156`)
- `[RESOLVED] OQ-9 -- What is the direction order? -> N, NE, E, SE, S, SW, W, NW, then direction 8 tube.` (evidence: `0x007E3774` bytes, loop bound `0x0042A1A1..0x0042A1AD`)
- `[RESOLVED] OQ-10 -- What are the direction epsilon values? -> `[0.001,0.005,0.002,0.006,0.003,0.007,0.004,0.008,0.0]`.` (evidence: memory `0x0081872C`; add site `0x00429F96`)
- `[RESOLVED] OQ-11 -- Does direction 8 get normal epsilon/marker/edge-cost helper behavior? -> No; it bypasses the helper and uses Chebyshev distance.` (evidence: `0x00429FA3..0x00429FE6`)
- `[RESOLVED] OQ-12 -- Are there INI keys for this exact branch? -> No direct INI readers in the scoped branch.` (evidence: decompile `0x00429A90`; Workstream B not applicable)
- `[RESOLVED] OQ-13 -- Is this gated by TS-only flags? -> No TS/fog/special flag gate observed; branch is in live normal pathfinding.` (evidence: `0x0042C900 -> 0x00429A90`)
- `[RESOLVED] OQ-14 -- What happens for a closed cell that fails the "good enough" skip? -> The function may continue through legality/cost and blocked-goal fallback, but passable-node insertion still refuses current-epoch markers.` (evidence: `0x00429F37..0x0042A01E`, `0x0042A17D..0x0042A19B`)
- `[RESOLVED] OQ-15 -- Current Rust delta? -> Rust has dual arrays and epsilon but skips closed nodes immediately and lacks a focused no-reopen/tolerance-fallback test.` (evidence: `src/sim/pathfinding/core.rs` focused read)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Closed-list tolerance compares stored `g` against `current.g + 1.009`, not `f`, and not candidate tentative cost. | `0x00429EE9..0x00429EF5`, `0x00429F1E..0x00429F2A`, `0x0042A529..0x0042A530`, `0x0042A153..0x0042A156` | mismatch/unchecked: Rust closed handling is boolean immediate skip; g arrays are updated by `tentative_g < existing`. | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs` | Future parity work should distinguish binary closed-marker behavior from standard reopen-by-better-g behavior. | A crafted graph reaches the same ground cell by two routes after it has already been marked closed; even a later lower-g route must not reinsert it into the heap. | Do not implement this as ordinary A* "reopen if better". |
| `1.009` is an early skip gate only. If `existing_g < current.g + 1.009`, the neighbor is skipped before `Can_Enter_Cell`; otherwise work continues but insertion still requires marker != epoch. | `0x00429EEC`, `0x00429EFC`, `0x00429FFB..0x0042A01E` | missing: Rust skips closed immediately, so it cannot reproduce the closed-but-not-good-enough blocked-goal fallback path if observable. | `src/sim/pathfinding/core.rs` near closed checks and blocked-goal fallback | If implemented, preserve the no-reopen rule while allowing the binary's later blocked-goal handling path. | Destination cell is already closed on the selected layer, impassable, and height-compatible; binary can accept current node as near-goal after the tolerance branch does not early-skip. | Do not move the tolerance compare after edge-cost calculation; it is before legality/cost work. |
| Ground and bridge closed entries are independent. A cell closed on ground does not reject a bridge-layer visit, and vice versa. | ground `+0x18/+0x24`; bridge `+0x1C/+0x20`; writes `0x0042A12D..0x0042A156` | mostly matched by Rust dual arrays and push-time `on_bridge`. | `src/sim/pathfinding/core.rs` | Keep layer-keyed closure and came-from storage. | Same `(rx,ry)` reachable under a high bridge and on deck should be visitable once per layer in one search. | Do not collapse closed state to coordinate-only. |
| Direction epsilon is added after edge cost and `Pathfinder+0x04`, with order N, NE, E, SE, S, SW, W, NW. | `0x00429F8A..0x00429F9D`; memory `0x0081872C` | mostly matched by `DIR_TIEBREAK`; tests should pin order in cell tie cases. | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs` | Preserve final additive tie term outside marker/entity/bridge multipliers. | Equal-cost alternatives differing only by first direction choose N before E before S before W and cardinals before diagonals according to epsilon values. | Do not use coordinate or zone id as the primary equal-cost tie. |
| Direction 8 tube edges bypass normal direction epsilon and helper cost. | `0x00429FA3..0x00429FE6` | Rust currently uses `TUBE_DIR_TIEBREAK = 9`, which intentionally differs from binary's no-epsilon Chebyshev path; existing comments should be audited if exact tube parity is required. | `src/sim/pathfinding/core.rs`, tube tests in `core_tests.rs` | Decide whether to keep Rust's artificial tube ordering or match binary no-epsilon behavior in a dedicated tube parity patch. | A tube jump competing with a normal edge of equal Chebyshev cost should follow binary heap/order behavior, not an invented `+9` term, if exact parity is required. | Do not assume direction epsilon table entry 8 participates in the tube branch. |

### Negative Facts / Do Not Do

- Do not call `Pathfinder+0x20/+0x24` f-cost arrays for this branch. The compared/stored value is `node+0x04` accumulated `g`; heap `f` is `node+0x08`.
- Do not implement a standard "if new_g + epsilon < closed_g then reopen closed node" rule. The binary does not insert a new node while the selected layer marker equals the current epoch.
- Do not collapse ground and bridge closed state into one coordinate key.
- Do not add direction epsilon before marker/entity/bridge multipliers. The add is caller-side after `AStar_compute_edge_cost * Pathfinder+0x04`.
- Do not apply the normal direction epsilon to direction 8 tube edges.

### Stale Docs / Follow-up Docs

- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` and related docs describe `+0x20/+0x24` as f-cost arrays in places. Replacement wording for this branch: "`Pathfinder+0x20` and `+0x24` store the accepted node's accumulated `g` cost (`node+0x04`) for bridge and ground layers respectively; heap priority `f` lives at `node+0x08`."
- Any phrase saying "`1.009` tolerance blocks reopening" should be tightened to: "`1.009` is an early closed-neighbor skip test; the later insertion guard still prevents true closed-node reopening on the selected layer."

## Sources

- Ghidra decompiled: `AStar_main_loop @ 0x00429A90`, `AStar_create_node @ 0x0042A460`, `AStar_pathfind_search @ 0x0042C900`.
- Ghidra assembly contexts: `0x00429E54`, `0x00429ECF`, `0x00429EE9`, `0x00429F04`, `0x00429F1E`, `0x00429F96`, `0x00429FFB`, `0x0042A004`, `0x0042A013`, `0x0042A030`, `0x0042A04C`, `0x0042A12D`, `0x0042A13F`, `0x0042A153`, `0x0042A1A1`, `0x0042A3C1`, `0x0042A529`.
- Ghidra memory reads: `0x007E37C0` length 8, `0x0081872C` length 36, `0x007E3774` length 32, `0x0081870C` length 32.
- Prior docs referenced: `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`, `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`, `ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md`.
- Rust files scanned: `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs`.
