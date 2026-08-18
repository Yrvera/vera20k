# WAYPOINTPATHCLASS_PERMANENT_APPEND_WRITER reswarm report

Date: 2026-05-28
Slot: 3
Target: WAYPOINTPATHCLASS_PERMANENT_APPEND_WRITER
Mode: exhaustive-slice for static writer identity; bounded partial for active runtime liveness
Verdict: PARTIAL

## Working Notes Gate

- Target question: identify the active permanent append writer for `WaypointPathClass+0x2C/+0x38`, if one exists, and relate it to true Planning Mode `Techno+0x514` PlanningToken authoring.
- Non-goals: do not re-decode overlay rendering, ordinary Shift move/NavQueue, complete cursor art, or unrelated path-following consumers.
- Evidence needed to mark COMPLETE: writer body, wrapper/caller chain, callsite liveness in standard YR, field writes to `+0x2C/+0x38`, cap/loop behavior, and Rust-facing acceptance scenarios.
- Stop conditions: Ghidra read-only only; no missing-function creation; if the only append writer has no verified active caller, record the writer and downgrade liveness to partial.

## 1. Overview

The binary contains a real permanent coordinate append helper for `WaypointPathClass`: `FUN_007639A0`. It appends one normalized 0x0C coordinate entry to the vector at `WaypointPathClass+0x28`, increments the count at `+0x38`, writes through the data pointer at `+0x2C`, and refuses to append once the path loop index at `+0x24` is set.

The only direct caller found is a House/player wrapper at raw code range `0x00502160..0x00502283`. That wrapper scans all 12 House/player path slots for duplicate cell coordinates, ensures the current path slot exists, then calls `FUN_007639A0`.

No direct active caller into `0x00502160` was found, and the true Planning Mode `EventClass` path still resolves to the per-unit PlanningToken path documented in `TRUE_PLANNING_MODE_POINT_ADD_WRITERS_RESWARM_20260528.md`: `FUN_00637E00 -> FUN_00638120 -> FUN_00633FA0/FUN_00639A50`. Therefore this slot does not prove that normal standard-YR Planning Mode commits token data into House/player `WaypointPathClass` storage.

## 2. Key Offsets

| Owner | Offset | Type | Verified role | Active in YR |
|---|---:|---|---|---|
| House/player | `+0x20C` | int | current `WaypointPathClass` slot index, `0..11`, `-1` none | Conditional; consumed by cursor/render helpers |
| House/player | `+0x210 + slot*4` | pointer | 12 `WaypointPathClass*` slots | Conditional; render and lookup consumers are active |
| `WaypointPathClass` | `+0x24` | int | loop/closure index; `-1` means append is allowed | Conditional; writer and renderer consume it |
| `WaypointPathClass` | `+0x28` | vtable/header | embedded dynamic vector header | Conditional |
| `WaypointPathClass` | `+0x2C` | `CoordStruct*` | vector data pointer for 0x0C entries | Conditional |
| `WaypointPathClass` | `+0x30` | int | vector capacity/max used by append growth check | Conditional |
| `WaypointPathClass` | `+0x35` | byte | vector growth flag used by append helper | Conditional |
| `WaypointPathClass` | `+0x38` | int | active point count | Conditional |
| `WaypointPathClass` | `+0x3C` | int | growth amount; constructor initializes to `10` | Conditional |
| `TechnoClass` | `+0x514` | pointer | per-unit PlanningToken, true Planning Mode command authoring store | Yes when Planning Mode command events execute |

## 3. Core Logic

### 3.1 Permanent append helper: `FUN_007639A0`

Evidence: Ghidra decompile `FUN_007639A0`; read-only disassembly `0x007639A0..0x00763A4E`; local binary call scan.

Inputs:

- `ECX`: `WaypointPathClass*`.
- stack argument: source coordinate pointer with three dwords.

Verified behavior:

1. Reads the source coordinate x/y/z from the input pointer.
2. Converts x and y to cell coordinates using the signed YR pattern `(value + ((value >> 31) & 0xff)) >> 8`.
3. Re-centers x and y to cell center with `cell * 0x100 + 0x80`.
4. If `WaypointPathClass+0x24 != -1`, returns `1` without appending.
5. If count has reached capacity, uses the dynamic vector vtable at `+0x28` slot `+0x08` to grow by `+0x3C + old_capacity`.
6. If growth is disallowed or fails, returns `1` without appending.
7. Reads old count from `+0x38`, increments `+0x38` by one, computes `+0x2C + old_count * 0x0C`, and writes x/y/z into that 0x0C slot.
8. Always returns `1`; the return value is not a success flag.

Tiny details:

- The helper floors/truncates toward zero-style cell conversion through the `cdq; and edx,0xff; add; sar 8` idiom before re-centering.
- The z dword is copied from input unchanged.
- The loop index gate happens before capacity/growth and before count increment.
- When append is blocked by loop index, capacity, growth flag, or growth failure, the helper still returns `1`.
- The embedded vector fields line up with `+0x28` header: data pointer at object `+0x2C`, capacity at `+0x30`, growth flag byte at `+0x35`, count at `+0x38`, growth amount at `+0x3C`.

### 3.2 House/player append wrapper: raw range `0x00502160..0x00502283`

Evidence: raw disassembly `0x00502160..0x00502283`; no Ghidra function boundary was present and none was created.

Verified behavior:

1. Converts the candidate coordinate to cell x/y once at entry.
2. Iterates House/player path slots `0..11`, backing array `house+0x210`.
3. Lazily allocates missing path objects with `operator_new(0x40)` and `WaypointPathClass__Constructor(slot)`.
4. For each path, scans `index 0..path->count-1` through `FUN_00763980`.
5. Compares existing point cell x/y after the same signed cell conversion.
6. If a matching point is found in a path that is not the current `house+0x20C` slot, returns without append.
7. If a matching point is found in the current path, it does not return there; it continues scanning.
8. After all 12 slots pass, it ensures the current path slot exists and calls `FUN_007639A0(current_path, coord)`.

Important negative result:

- A direct CALL-rel32 scan of retail `gamemd.exe` found no direct caller for `0x00502160`.
- The same scan found exactly one direct caller for `FUN_007639A0`: `0x00502277`, inside the wrapper above.

### 3.3 Existing lookup and loop helpers remain consumers/helpers

Evidence: Ghidra decompile and direct call scan.

- `FUN_00763980(path,index)` returns `path+0x2C + index*0x0C` only when `0 <= index < path+0x38`; direct callers include aircraft/foot path-following, House/player lookup wrappers, and `FUN_006DAD60` renderer.
- `FUN_00763BA0(path,current_point)` computes the next point; at end it wraps to `+0x24` only when `+0x24 != -1`.
- `FUN_00763A50(path,coord)` scans existing points and writes `+0x24 = matching_index`; its only direct caller found is `FUN_00502290`.
- `FUN_00502290` ensures current path and calls `FUN_00763A50`, but a direct CALL-rel32 scan found no direct caller into `0x00502290`.
- `FUN_00763BE0(path)` clears vector and resets `+0x24 = -1`; direct call scan found no direct callers in this bounded pass.

### 3.4 True Planning Mode relation

Evidence: prior report plus Ghidra spot-checks of `FUN_00637E00`, `FUN_00638120`, `FUN_00633FA0`, and `FUN_00639A50`.

The active true Planning Mode EventClass command-authoring path remains per-unit:

- `EventClass::Execute` cases `0x2A`, `0x2B`, `0x2C`, plus planning-flagged ordinary events, route to `FUN_00637E00`.
- `FUN_00637E00` routes ordinary planning command add to `FUN_00638120`.
- `FUN_00638120` allocates/gets `Techno+0x514` PlanningToken and appends command nodes through `FUN_00633FA0` or `FUN_00639A50`.
- Those helpers append 0x10 command-entry objects and 0x6F event copies to PlanningToken/node dynamic vectors; they do not call `FUN_007639A0` or write `WaypointPathClass+0x2C/+0x38`.

Result:

- The active event path does not prove any commit from PlanningToken storage into House/player `WaypointPathClass` permanent storage.
- The `WaypointPathClass` overlay renderer and hit-test helpers are live consumers, but the standard-YR active producer for permanent points remains unverified.

## 4. INI Keys

| Section | Key | Stock YR value | Binary use in this slice | Active in YR |
|---|---|---:|---|---|
| `[General]` | `MaxWaypointPathLength` | `15` | read into `RulesClass+0x90`; `FUN_005090F0` checks `path+0x38 < Rules+0x90` and `path+0x24 == -1` before action `0x2A` addability | Yes |
| `[AudioVisual]` | `StartPlanningModeSound` | `PlanningModeStart` | true Planning Mode entry sound; not the permanent append writer | Conditional |
| `[AudioVisual]` | `EndPlanningModeSound` | `PlanningModeEnd` | true Planning Mode exit sound; not the permanent append writer | Conditional |
| `[AudioVisual]` | `AddPlanningModeCommandSound` | `PlanningModeAdd` | played by PlanningToken command-add path; not proof of `WaypointPathClass` append | Conditional |
| `[AudioVisual]` | `WaypointAnimationSpeed` | `10` | read by rules; no direct append-writer use found | Conditional/consumer deferred |

## 5. Integration Points

| Integration | Status | Evidence | Active in standard YR |
|---|---|---|---|
| `FUN_007639A0` coordinate append helper | verified | decompile plus disassembly `0x007639A0..0x00763A4E` | Helper exists; caller liveness unverified |
| Wrapper `0x00502160..0x00502283` | verified static bytes | raw disassembly plus call to `0x007639A0` at `0x00502277` | No active caller verified |
| Direct callers to append helper | verified static scan | CALL-rel32 scan: only `0x00502277` | No other direct producer |
| Direct callers to wrapper `0x00502160` | verified static scan | CALL-rel32 scan: none | No direct standard path found |
| True Planning Mode EventClass writer | verified by prior report, spot-checked | `FUN_00637E00 -> FUN_00638120 -> FUN_00633FA0/FUN_00639A50` | Yes, PlanningMode conditional |
| Overlay renderer | verified by prior report | `FUN_006DAD60`, calls `FUN_00763980`/`FUN_00763BA0` | Conditional on nonempty paths |

## 6. Current Rust Implementation Status

Rust scan was reconnaissance only; no Rust files were modified.

Relevant current surfaces:

- `src/app_context_order.rs:44` derives `queue_mode` from Shift.
- `src/app_context_order.rs:158`, `:246`, `:642`, `:681` emit `Command::Move { queue }`.
- `src/app_target_lines.rs:90` records command lines for move/attack-move.
- `src/app_target_lines.rs:198` reads Rust `navigation.nav_queue` for selected action lines.
- `src/sim/components.rs:296` defines `NavigationState`; `:306` stores `nav_queue`.
- `src/sim/movement/movement_commands.rs:556` clears `navigation.nav_queue` when Drive destinations are issued.
- `src/sim/movement/movement_tick.rs:426..434` consumes Rust `nav_queue` on arrival.

No Rust surface equivalent to:

- House/player `+0x20C` current waypoint path slot.
- 12 `WaypointPathClass` slots at `+0x210`.
- `WaypointPathClass+0x24/+0x2C/+0x38/+0x3C`.
- Per-unit `Techno+0x514` PlanningToken graph.
- Separate true Planning Mode UI active flag plus PlanningToken command authoring.

Existing Rust `NavigationState.nav_queue` and `MovementTarget` must not be treated as parity coverage for true Planning Mode.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_007639A0` append helper | verified | decompile plus disassembly `0x007639A0..0x00763A4E` | none for helper mechanics |
| `0x00502160..0x00502283` wrapper | verified static | raw disassembly, call to `0x007639A0` at `0x00502277` | no Ghidra function boundary; active caller unverified |
| Direct caller scan for `0x00502160` | verified static | retail binary CALL-rel32 scan found none | indirect/runtime caller not fully excluded |
| Direct caller scan for `FUN_007639A0` | verified static | only caller `0x00502277` | none for direct calls |
| `FUN_00502290` loop-index wrapper | verified static | decompile, direct call to `FUN_00763A50` | active caller unverified |
| True Planning Mode EventClass command add | verified by prior report and spot-check | `FUN_00637E00`, `FUN_00638120`, `FUN_00633FA0`, `FUN_00639A50` | none for separation from `WaypointPathClass` append |
| Overlay render consumers | prior verified, spot-used | `FUN_006DAD60`, `FUN_00763980`, `FUN_00763BA0` | producer liveness |
| Rust scan | touched | `rg` over app/movement/target-line surfaces | future implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-WPAPP-001 - Is there a helper that appends one permanent coordinate point to WaypointPathClass+0x2C/+0x38? -> Yes: FUN_007639A0.` (evidence: decompile and disassembly `0x007639A0..0x00763A4E`)
- `[RESOLVED] OQ-WPAPP-002 - What exactly does the append helper write? -> It increments vector count at object `+0x38`, then writes centered x/y and unchanged z to `+0x2C + old_count*0x0C`.` (evidence: `0x00763A25..0x00763A3D`)
- `[RESOLVED] OQ-WPAPP-003 - Does loop state block append? -> Yes. `+0x24 != -1` returns before capacity/growth/count writes.` (evidence: `0x007639E2..0x007639F4`)
- `[RESOLVED] OQ-WPAPP-004 - Does the append helper enforce Rules.MaxWaypointPathLength itself? -> No direct `Rules+0x90` read in helper; UI addability predicate `FUN_005090F0` owns that gate.` (evidence: `FUN_007639A0`, `FUN_005090F0`, `ini/rulesmd.ini:424`)
- `[RESOLVED] OQ-WPAPP-005 - Is there a House/player wrapper around the append helper? -> Yes, raw range `0x00502160..0x00502283`.` (evidence: disassembly; append call at `0x00502277`)
- `[RESOLVED] OQ-WPAPP-006 - What duplicate rule does the wrapper apply? -> A same-cell point in another path slot blocks append; a same-cell point in the current path does not block and scanning continues.` (evidence: `0x005021C2..0x00502229`)
- `[RESOLVED] OQ-WPAPP-007 - Are there direct calls to the wrapper? -> None found by CALL-rel32 scan of retail `gamemd.exe`.` (evidence: local binary scan, target `0x00502160`)
- `[RESOLVED] OQ-WPAPP-008 - Are there other direct calls to the append helper? -> No; only `0x00502277`.` (evidence: local binary scan, target `0x007639A0`)
- `[RESOLVED] OQ-WPAPP-009 - Does true Planning Mode EventClass command authoring call this append helper? -> No direct evidence; the verified path uses PlanningToken writers.` (evidence: `FUN_00637E00`, `FUN_00638120`, `FUN_00633FA0`, `FUN_00639A50`)
- `[RESOLVED] OQ-WPAPP-010 - Does `AddPlanningModeCommandSound` prove WaypointPath append? -> No. It belongs to the PlanningToken command-add success path.` (evidence: `FUN_00638120`; `ini/rulesmd.ini:632`)
- `[RESOLVED] OQ-WPAPP-011 - Does `FUN_00763A50` append points? -> No. It scans existing points and writes loop index `+0x24`.` (evidence: decompile `FUN_00763A50`)
- `[DEFERRED] OQ-WPAPP-012 - Could an indirect, missed-boundary, or runtime-generated caller reach `0x00502160`?` (category: `needs-runtime-debugger`; reason: direct call scan and current Ghidra call graph did not prove one, but absence of all indirect dispatch cannot be proven statically here; next-step-if-pursued: set a runtime breakpoint on `0x00502160` and enter Planning Mode/add points in stock YR)
- `[DEFERRED] OQ-WPAPP-013 - What active system populates nonempty House/player WaypointPathClass paths in ordinary stock play, if not true Planning Mode token commit?` (category: `requires-different-system-context`; reason: render/follow consumers are live, but standard producer liveness was not proven; next-step-if-pursued: runtime breakpoint on `FUN_007639A0` plus scenario/path-following actions)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `FUN_007639A0` appends to `WaypointPathClass` only when `+0x24 == -1`, grows embedded vector if needed, increments `+0x38`, and writes centered x/y plus unchanged z to `+0x2C + old_count*0x0C`. | `0x007639A0..0x00763A4E`; call at `0x00502277` | missing | future planning/waypoint-path model, not existing `NavigationState.nav_queue` | If Rust later models native House/player waypoint paths, append must reproduce loop gate, capacity growth, centered coordinate normalization, and always-`1` return semantics. | Enter a native-like path-add helper with loop index `-1`, source coord `(N*256+k, M*256+k, z)`, assert stored point is `(N*256+128, M*256+128, z)` and count increments once. Proposed test: `waypoint_path_append_centers_cells_and_increments_count_once` | Do not use raw clicked world coordinates as stored path points. |
| The only direct wrapper found scans all 12 path slots and blocks appending a coordinate if the same cell exists in a different slot, but not if it is in the current slot. | `0x00502160..0x00502283`; duplicate branch `0x00502201..0x0050220F`; append call `0x00502277` | missing | future House/player planning overlay storage | If implementing this wrapper, preserve all-slot duplicate rejection and current-slot exception. | Seed two path slots; adding a point whose cell exists in another slot does not change current path count, while same-cell only in current path still reaches append helper. Proposed test: `waypoint_path_append_blocks_duplicate_in_other_slot_only` | Do not apply a simple global duplicate-set rule without the current-slot exception. |
| Standard true Planning Mode command authoring verified so far writes per-unit `Techno+0x514` PlanningToken data, not House/player `WaypointPathClass+0x2C/+0x38`. | `FUN_00637E00`, `FUN_00638120`, `FUN_00633FA0`, `FUN_00639A50`; no direct call to `FUN_007639A0` from that path | missing | future true Planning Mode command subsystem | Model PlanningToken command graph separately from House/player overlay paths until a verified bridge is found. | Enter Planning Mode, issue a command, and assert command authoring creates per-unit token state rather than Rust `nav_queue` or House-path append unless a verified append call is added. Proposed test: `true_planning_command_add_writes_planning_token_not_waypoint_path` | Do not force token data into `WaypointPathClass` just because the overlay renderer consumes that class. |
| No active direct caller to `0x00502160` was found in this static pass; no standard-YR commit from PlanningToken to House paths was proven. | CALL-rel32 scan: `0x00502160` has no direct callers; `0x007639A0` only called from `0x00502277` | unchecked/missing | implementation planning and test triage | Treat House/player permanent path append as unimplemented/uncertain until runtime liveness is verified. | Runtime parity harness breakpoint equivalent: adding a standard Planning Mode command should not be assumed to hit `WaypointPathClass` append without evidence. Proposed test placeholder: `planning_mode_runtime_append_liveness_requires_verified_breakpoint_trace` | Do not claim full Planning Mode overlay parity from static helper existence alone. |

## Negative Facts / Do Not Do

- Do not treat `FUN_007639A0` return value as append success; blocked paths still return `1`.
- Do not append when `WaypointPathClass+0x24 != -1`; native returns before count/data writes.
- Do not use `[General] MaxWaypointPathLength` inside the append helper; the verified INI cap is enforced by `FUN_005090F0` before addability/action, not by `FUN_007639A0`.
- Do not merge PlanningToken command nodes with `WaypointPathClass` point storage; the verified EventClass path appends token command entries, not path coordinates.
- Do not implement `FUN_00763A50` as an append; it writes loop index `+0x24` after finding an existing cell.
- Do not model true Planning Mode as Rust `NavigationState.nav_queue`; prior and current evidence keep these surfaces separate.

## Remaining Uncertainty

- Active standard-YR liveness of wrapper `0x00502160..0x00502283`: no direct caller was found, but an indirect or missed-boundary caller cannot be fully excluded without runtime breakpoint evidence.
- Active stock producer for nonempty House/player `WaypointPathClass` paths remains unresolved. Consumers are verified; the standard producer was not.
- Whether ordinary true Planning Mode overlay lines in stock YR are derived only from PlanningToken/preview state, from dormant House paths, or from an indirect call into the wrapper requires runtime tracing.
- `FUN_00763BE0` clear helper and `FUN_00502290` loop wrapper have no direct callers in this pass; their runtime liveness remains separate follow-up work.

## Stale Docs / Follow-up Docs

- `docs/research/TRUE_PLANNING_MODE_POINT_ADD_WRITERS_RESWARM_20260528.md`: replace "Exact active permanent append writer for `WaypointPathClass+0x2C/+0x38` remains unresolved" with "The append helper is `FUN_007639A0`, reached by raw wrapper `0x00502160..0x00502283`; static direct-call scanning found no active caller to the wrapper, so standard Planning Mode liveness remains unresolved."
- `docs/research/DRIVE_QUEUED_CLICK_EVENT_PLANNING_MODE_OUTCOME_RESWARM_20260528.md`: wording that says true Planning Mode clicks are "owned by House/WaypointPathClass" should be narrowed to "planning overlay consumers use House/player `WaypointPathClass`, while verified Planning Mode command authoring writes per-unit PlanningToken state; the House-path append producer is not yet active-proven."

## Sources

- Ghidra read-only decompile: `FUN_007639A0`, `FUN_00763980`, `FUN_00763A50`, `FUN_00763BA0`, `FUN_00763BE0`, `FUN_00502290`, `FUN_005023B0`, `FUN_00502460`, `FUN_005090F0`, `FUN_00637E00`, `FUN_00638120`, `FUN_00633FA0`, `FUN_00639A50`, `FUN_006DAD60`.
- Ghidra read-only disassembly requested: `0x00502160..0x00502285`, `0x007639A0..0x00763A4E`.
- Local retail binary static scan: `<ra2-install>/gamemd.exe`, CALL-rel32 targets `0x00502160`, `0x007639A0`, `0x00763A50`, `0x00763980`, `0x00763BA0`, `0x005023B0`, `0x00502460`, `0x00638120`, `0x00633FA0`, `0x00639A50`.
- Existing docs: `docs/research/TRUE_PLANNING_MODE_POINT_ADD_WRITERS_RESWARM_20260528.md`, `docs/research/PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`, `docs/research/DRIVE_QUEUED_CLICK_EVENT_PLANNING_MODE_OUTCOME_RESWARM_20260528.md`, `docs/research/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini:424`, `:630`, `:631`, `:632`, `:670`; base `ini/rules.ini:336`, `:476`, `:477`, `:478`, `:514`.
- Rust scan: `src/app_context_order.rs`, `src/app_target_lines.rs`, `src/sim/components.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/navcom.rs`.
