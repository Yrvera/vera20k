# AStar_compute_edge_cost — `param_1+0x3c` Identity & Urgency Gate (Ghidra Research Report)

**Scope:** Resolve the previously UNVERIFIABLE identity of `param_1` at the
`AStar_compute_edge_cost` callsite inside `AStar_main_loop`, and confirm whether
urgency levels 1/2 skip the 10-step blocker-prediction loop and apply a fixed edge
cost. Read-only Ghidra MCP verification against `gamemd.exe` (project `testProsjekt`,
image base `0x400000`).

**Confidence:** HIGH — every claim below is a direct disassembly/`read_memory` byte
read this session, not a decompile paraphrase.

**Active in YR:** Yes — Conditional on the unit hitting a friendly moving blocker
(`Can_Enter_Cell` code 2). This is routine in normal skirmish play (units queued
through chokepoints, crossing paths, bridge approaches). See §5.

---

## 1. `param_1` identity — verified this-pointer chain

`param_1` of `AStar_compute_edge_cost` (thiscall, so `param_1` = `ECX` at entry) is
**the same object, by pointer identity, at every level of the call chain** — it is
the shared per-search `PathfinderClass` singleton, not a per-cell or per-neighbor
struct.

Chain, each link independently re-verified this session:

| Step | Evidence |
|---|---|
| `FootClass::Run_AStar @ 0x004cbba0` calls `AStar_pathfind_search @ 0x0042c900` with `ECX = 0x0087e8b8` (hardcoded singleton immediate) | `read_memory 0x004cbc25` len 16 → bytes `b9 b8 e8 87 00` at `0x004cbc2c` = `MOV ECX,0x0087e8b8`, followed by `e8 ..` (`CALL`) at `0x004cbc31` |
| `AStar_pathfind_search`: `EBP = ECX` (this) at entry; the urgency setter writes `[EBP+0x3c] = ECX_of_caller_arg` (the write itself, at `0x0042c92f`); later calls `AStar_main_loop @ 0x00429a90` with `ECX = EBP` (same this) | `disassemble_function 0x0042c900`: `0042c905 MOV EBP,ECX`; `0042c92f MOV dword ptr [EBP + 0x3c],ECX`; `0042cc00 MOV ECX,EBP` / `0042cc02 CALL 0x00429a90` |
| `AStar_main_loop`: `ESI = ECX` (this) at entry; calls `AStar_compute_edge_cost @ 0x00429830` with `ECX = ESI` (same this) | `disassemble_function 0x00429a90`: `00429a9f MOV ESI,ECX`; `00429f88 MOV ECX,ESI` / `00429f8a CALL 0x00429830` |
| `AStar_main_loop` itself also reads `[ESI+0x3c]` directly (bridge-passability gate before `PathfinderClass__UpdateBridgePassability @ 0x0042acf0`) at three sites, confirming `ESI+0x3c` and `param_1+0x3c` in `AStar_compute_edge_cost` are the same field on the same object | `disassemble_function 0x00429a90`: `00429c10`, `0042a423`, `0042a442` — all `MOV EAX,[ESI+0x3c]; TEST EAX,EAX; JZ ...` |
| `get_function_callers(0x00429a90)` → single caller `AStar_pathfind_search`; `get_function_callers(0x0042c900)` → single caller `FootClass::Run_AStar` | `get_function_callers` calls this session |

**Conclusion:** `param_1+0x3c` in `AStar_compute_edge_cost` is `PathfinderClass+0x3c`
on the singleton at `0x0087e8b8` — the field is written **once per `Find_Path`
call** (in `AStar_pathfind_search`, before `AStar_main_loop` runs) and then held
constant for the **entire A* search**, including every per-neighbor edge-cost
evaluation `AStar_main_loop` performs against however many candidate cells it
expands that tick. It is emphatically **not** a per-cell field — there is no
per-cell storage of urgency anywhere in this chain.

This matches (and closes the "callsite identity UNVERIFIABLE" gap in) the existing
`docs/research/pathfinding/PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md`, which had
already identified the singleton and the single write site at `0x0042c900` but did
not carry an explicit assembly ECX-trace all the way through `AStar_main_loop` into
`AStar_compute_edge_cost`'s `param_1`. That trace is now complete (table above).

## 2. The `+0x3c` gate — verified from raw disassembly

Inside `AStar_compute_edge_cost @ 0x00429830`, gated by `bVar9` (see §4), the gate
and both fixed-cost writes are:

```
00429878  MOV EAX,dword ptr [ECX + 0x3c]      ; ECX = param_1 (this)
0042987e  TEST EAX,EAX
00429880  JNZ 0x0042998d                      ; nonzero → SKIP the entire prediction loop
00429886  ...                                  ; (loop body, only reached when +0x3c == 0)
...
0042998d  MOV dword ptr [ESP + 0x38],0x40800000   ; ESP+0x38 (== param_5 spill) = 4.0f
00429995  MOV EAX,dword ptr [ECX + 0x3c]
00429998  POP EBP
00429999  CMP EAX,0x2
0042999c  JNZ 0x004299a6                       ; not urgency 2 → keep 4.0
0042999e  MOV dword ptr [ESP + 0x34],0x447a0000 ; urgency==2 → overwrite to 1000.0f
```

Verified via `disassemble_function 0x00429830`. Immediate decode:
- `0x40800000` = IEEE-754 single `4.0` (sign 0, exponent 129 → ×2², mantissa 1.0).
- `0x447a0000` = IEEE-754 single `1000.0` (sign 0, exponent 136 → ×2⁹, mantissa
  1.953125 = 1000/512).

**Branch predicate, in plain terms:**
- `param_1+0x3c == 0` (urgency 0, first-attempt pathfind): run the 10-step
  blocker-prediction loop (`0x00429886`–`0x0042997e`); it may resolve to the base
  table cost (predicted-clear case, `goto AStar_cost_predict_blocker_clears` at
  `0x00429986`/loop-internal label) or fall through to `4.0` if prediction never
  clears within 10 hops.
- `param_1+0x3c != 0` (urgency 1 or 2): **the loop is not entered at all** — `JNZ`
  at `0x00429880` jumps straight past it to `0x0042998d`, unconditionally setting
  cost `= 4.0`.
- After that, `param_1+0x3c == 2` specifically (re-read at `0x00429995`, independent
  of the first test) overrides the cost to `1000.0`. Urgency `1` stops at `4.0`;
  urgency `2` always ends at `1000.0`.

This confirms the audit's exact concern: **urgency 1 and urgency 2 both skip the
blocker-prediction loop unconditionally** — urgency 1 gets a fixed `4.0` edge cost,
urgency 2 gets a fixed `1000.0` edge cost, with **zero prediction attempt**, for
every `Can_Enter_Cell`-code-2 edge evaluated for the rest of that search.

## 3. Cross-reference: prior docs already had these facts, but scattered

- `docs/research/pathfinding/PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` (2026-05-18,
  older than the audit that flagged UNVERIFIABLE) already tabulates the 0/1/2 →
  1.0-or-4.0 / 4.0 / 1000.0 mapping (§5 "Complete Value Enumeration") and the single
  write site, but did not carry the this-pointer identity proof through
  `AStar_main_loop` explicitly.
- `docs/research/bridges/03-traversal-pathfinding-entry/ASTAR_COMPUTE_EDGE_COST_00429830_BRIDGE_COSTS_GHIDRA_REPORT.md` §3.3 states the same branch semantics
  ("value 0 runs the loop; value 1 skips prediction and leaves the jam cost; value 2
  overrides to 1000.0") — consistent with the asm re-derived here.
- `docs/research/pathfinding/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` §4–5
  documents the same fixed-cost table and the tick-level urgency-escalation state
  machine (`BlockedDelay` timer, `Rules+0x1768` = `BlockagePathDelay`, default 60
  frames) that produces urgency 1 then 2 over time.
- I found no literal "UNVERIFIABLE" string in either `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` or `docs/research/pathfinding/FOOTCLASS_PATHFINDING_AND_MOVEMENT.md`
  (checked via `Grep` this session) — the identity gap this task was dispatched to
  close was tracked outside those two files (swarm-internal), not as doc prose. No
  contradiction was found between docs; this report adds the missing link, it does
  not correct a wrong claim.

## 4. Supporting context (in-scope: when does the gate matter at all)

The entire `+0x3c` gate block, including the loop, sits inside `if (bVar9)` where
`bVar9` tests the function's **original** `param_5` argument (before it is
overwritten with the base-table float) against integer `2`:

```
00429845  CMP EAX,0x2        ; EAX = raw param_5 int (Can_Enter_Cell return code)
0042984f..00429858            ; base-table load into spill regardless of branch
0042985c  JNZ 0x004299aa      ; code != 2 → skip prediction/urgency entirely
```

So the `+0x3c` urgency gate only ever fires when `Can_Enter_Cell` returned code 2
(`TemporaryBlock` — a friendly unit currently occupies/is-moving-through the
destination cell). For all other codes (0, 1, 3–7) the function returns the base
table value directly at `0x004299aa` onward and never reads `+0x3c`. Verified via
the same `disassemble_function 0x00429830` call as §2 (`0x00429845`–`0x0042985c`).

## 5. Active in YR — reached in normal play

Per the already-verified caller chain in `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md`
§4 (not re-verified this session — out of scope, cited for the "Active in YR"
judgment only): `DriveLocomotionClass::Process_Movement`, `ShipLocomotionClass`, and
`WalkLocomotionClass` all compute urgency via the same `SETNZ+INC` pattern from a
`blocked_delay` timer seeded from `Rules+0x1768` (`BlockagePathDelay`, default 60
frames). Any unit that queues behind a friendly unit for more than 0/60 frames
reaches urgency 1/2 respectively — a routine occurrence in group movement, bridge
chokepoints, and base-defense repositioning in ordinary YR skirmish play. This is
**not** an edge case.

## 6. Implementation Handoff

- Model urgency as a **single field on the per-search pathfinder-state struct**
  (whatever the Rust equivalent of `PathfinderClass`/the A* search context is),
  set once when a `Find_Path` call begins and held constant for the entire search
  — never per-cell, per-neighbor, or per-edge-evaluation state.
- When urgency `!= 0`: for any candidate edge where the destination's occupancy
  check resolves to "friendly unit currently occupying/moving" (the Rust analogue
  of `Can_Enter_Cell` code 2), skip blocker-path prediction entirely and assign
  edge cost `4.0`. Do not attempt to walk the blocker's predicted path — the
  binary provably does not for urgency 1 or 2.
- When urgency `== 2` specifically: after the above, override the edge cost to
  `1000.0` (not `4.0`). This is a second, independent check on the same field, not
  a fallthrough from a different branch — implement as two sequential comparisons,
  matching `0x00429880` (`!= 0` gate) then `0x00429999` (`== 2` override).
  Urgency `1` must stop at `4.0` and never reach `1000.0`.
- Both fixed costs (`4.0`, `1000.0`) are only relevant when the destination cell's
  occupancy resolves to "friendly moving blocker" — gate the whole mechanism behind
  that occupancy classification first (§4), or the urgency check is a silent no-op.
- Further implementation guidance (urgency-computation call sites, `BlockedDelay`
  wiring) is already written in `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` §8 —
  do not re-derive it.

## 7. Negative Facts / Do Not Do

- Do **not** model `+0x3c` as per-cell or per-neighbor state — it is proven (§1) to
  be a single shared field on the search-level singleton, read unchanged across
  every edge the search evaluates.
- Do **not** apply the urgency gate to any `Can_Enter_Cell` code other than 2 — the
  `bVar9`/`CMP EAX,0x2`/`JNZ` guard (§4) means codes 0,1,3–7 never touch `+0x3c`.
- Do **not** give urgency `1` the `1000.0` cost — only urgency `2` reaches the
  second override (`0x00429999`/`0x0042999e`); urgency `1` stops at `4.0`.
- Do **not** assume the loop is "sometimes skipped for cell-specific reasons" — the
  skip is unconditional and entirely determined by the single `+0x3c` value at
  `JNZ 0x00429880`; there is no per-cell exception.
- Do **not** treat this as a newly-discovered mechanism requiring re-plumbing from
  scratch — the fixed-cost values and 0/1/2 enumeration were already correctly
  documented in three prior docs (§3); this report's contribution is closing the
  `param_1` identity gap with a verified this-pointer trace, not new cost values.

## 8. Remaining Uncertainty

None for the assigned scope (gate predicate, `param_1` identity, fixed-cost values,
and their branch conditions are now fully asm-verified end-to-end, §1–§2). Open
items explicitly out of scope for this report (already flagged as open in
`PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` §7 and not re-checked here):
HoverLocomotion's urgency-computation call chain past `FUN_005164d0`, and whether
Aircraft/Teleport locomotion ever reach this branch with stale residual `+0x3c`
state from a prior ground-unit pathfind (documented there as believed-harmless but
unverified).

## 9. Verification Calls Made This Session

- `get_current_program_info` — confirmed connected to `gamemd.exe`, image base `0x400000`.
- `get_function_callees(0x00429A90)` → `AStar_compute_edge_cost @ 00429830` (among others).
- `decompile_function(0x00429830)`, `disassemble_function(0x00429830)`.
- `disassemble_function(0x00429A90)` — full listing, callsite `0x00429f88`/`0x00429f8a`, entry `0x00429a9f`, internal `+0x3c` reads at `0x00429c10`/`0x0042a423`/`0x0042a442`.
- `get_function_callers(0x00429A90)` → `AStar_pathfind_search @ 0042c900`.
- `disassemble_function(0x0042c900)` — `this` capture `0x0042c905`, urgency-write `0x0042c92f`, `AStar_main_loop` callsite `0x0042cc00`/`0x0042cc02`.
- `get_function_callers(0x0042c900)` → `FootClass__Run_AStar @ 004cbba0`.
- `read_memory(0x004cbc25, 16)` → raw bytes decoded to `MOV ECX,0x0087e8b8` at `0x004cbc2c`.
- `research_search` (research-index MCP) for prior "UNVERIFIABLE"/urgency/`0x3c` doc coverage; `Grep` for literal "UNVERIFIABLE" across `docs/research/pathfinding/` (no hits in the two named source docs).
