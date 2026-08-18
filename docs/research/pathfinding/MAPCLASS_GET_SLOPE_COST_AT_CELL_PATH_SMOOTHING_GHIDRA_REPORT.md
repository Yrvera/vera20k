# MapClass Get_Slope_Cost_At_Cell Path Smoothing - Ghidra Research Report

**Address(es):** `0x0056BCD0` (`MapClass__Get_Slope_Cost_At_Cell`), callers `0x0042B420` (`Path_smooth_single_segment`) and `0x0042BE20` (`Path_Reroute_Straight_Line`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact `MapClass__Get_Slope_Cost_At_Cell` read formula and its active use by path smoothing/reroute, including `CellClass+0x24` decode, signed division, table indexing, return use, thresholds, `Foot+0x530` interaction, and YR liveness.
**Non-Scope:** construction/population of the `Foot+0x21C` slope-cost context arrays, full `Zone_Estimate_Slope_Cost`, full `Can_Enter_Cell`, and route outcome capture on a named retail map.
**Confidence:** High for helper formula, caller gates, constants, and liveness; Medium for semantic meaning of the `+0x59F0` table contents because this report verifies reads, not table construction.
**Active in YR:** Yes. `AStar_main_loop @ 0x00429A90` calls `Path_smooth_corners @ 0x0042B210` and `Path_optimize_straight_segments @ 0x0042B7F0` after path reconstruction, with no TS/fog/SpecialFlags gate in the verified call chain.

## Summary

`MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0` is a live path-postprocessing helper. It reads the actual `CellClass+0x24` map coordinate for a cell, converts the signed x/y shorts to a 4-cell coarse grid using truncation toward zero, and returns a signed 32-bit table entry from `slope_context+0x59F0`.

Path smoothing uses the same helper in two different ways: pass 1 rejects a smoothing replacement when `slope_cost * FootClass__Get_Slope_Speed_Factor(foot) >= 1.0`, while pass 2/reroute counts cells as steep when the product is `>= 0.01` and rejects based on a strict/lenient steep-count rule. Current Rust `path_smooth.rs` only asks a boolean `walkable` closure, so both slope rejection mechanisms are missing.

## Verified Binary Findings

### Helper formula at `0x0056BCD0`

`MapClass__Get_Slope_Cost_At_Cell(short *coord, int slope_context)`:

1. Loads `coord->y` and `coord->x` as signed 16-bit values.
2. Computes a flat cell index as `y * 0x200 + x`.
3. If the index is negative, `>= 0x40000`, or `g_CellArray_Base[index]` is null, it writes the requested 32-bit coordinate to `DAT_00ABDC74` and uses dummy cell base `0x00ABDC50`.
4. Reads dword `CellClass+0x24` from the chosen cell.
5. Treats the low 16 bits of that dword as signed x and the high 16 bits as signed y.
6. Divides each signed component by 4 with truncation toward zero, implemented as `(v + ((v >> 31) & 3)) >> 2`.
7. Returns `*(i32 *)(slope_context + 0x59F0 + ((x4 + y4 * 0x82) * 4))`.

Evidence:
- Decompile `0x0056BCD0`.
- Assembly `0x0056BCD0..0x0056BD3A`: `MOVSX` signed x/y loads, `SHL EAX,0x9`, bounds/null fallback, `MOV ECX,[EAX+0x24]`, two `CDQ; AND EDX,0x3; ADD EAX,EDX; SAR EAX,0x2` signed division sequences, `LEA ECX,[EAX + ESI*0x2]`, and final `MOV EAX,[EDX + ECX*4 + 0x59F0]`.
- Raw bytes at `0x0056BCD0` match the above sequence (`read_memory 0x0056bcd0 128`).

Important correction: the division is not mathematical floor for negative values. It is signed truncation toward zero. For example, `-1 -> 0`, `-5 -> -1`. Prior wording in `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` says "signed-floor"; that wording is stale.

### Field and table layout

| Owner | Offset / address | Type / shape | Verified behavior | Evidence |
|---|---:|---|---|---|
| Input coord | stack arg 1 | two signed i16 | Used only to find the cell array entry or seed dummy-cell coord on invalid lookup. | `0x0056BCD0..0x0056BCFD` |
| Global cell array base | `0x0087F924` | pointer to `CellClass*[]` | Indexed as `y*512+x`, max checked against `0x40000`. | `0x0056BCE2..0x0056BCF4` |
| Dummy cell | `0x00ABDC50` | `CellClass` fallback | Used when index invalid or cell pointer null. | `0x0056BCF6..0x0056BD02` |
| Dummy coord field | `0x00ABDC74` | dword at dummy `+0x24` | Receives requested coord before dummy is read. | `0x0056BCF6..0x0056BCFD` |
| CellClass | `+0x24` | packed coord dword | Low signed short is x; high signed short is y. | `0x0056BD02..0x0056BD22` |
| Slope context | `+0x59F0` | `i32` grid | Table is addressed as 130-wide rows at 4-cell resolution. | `0x0056BD2F..0x0056BD33` |
| FootClass | `+0x21C` | pointer | Passed as `slope_context` by both path-smoothing callers. | `0x0042B513..0x0042B51D`, `0x0042BED2..0x0042BEDD` |
| FootClass | `+0x530` | double | Fallback slope sensitivity/scalar returned by `FootClass__Get_Slope_Speed_Factor`. | `0x004DC77E` |
| FootClass linked object gate | `+0x5D4 -> +0x24 -> +0xF2` | pointer/type byte | Forces returned scalar to `1.0` when present and nonzero. | `0x004DC760..0x004DC77D` |
| TechnoTypeClass | `+0x2F0` | double | `ThreatAvoidanceCoefficient` copied into `Foot+0x530` during `FootClass__Unlimbo`. | `0x004D72EA..0x004D72F4`, `0x00712460..0x0071246D` |

### Return type and x87 conversion

The helper returns a 32-bit table value in `EAX`. Both smoothing callers store it into a 32-bit stack slot and load it with `FILD dword`, so the value is consumed as signed `i32` before multiplying by the double slope scalar.

Evidence:
- Pass 1: `0x0042B5D8 MOV [ESP+0x34],EAX`, `0x0042B5DC FILD dword ptr [ESP+0x34]`.
- Pass 2 first leg: `0x0042BF8A MOV [ESP+0x38],EAX`, `0x0042BF8E FILD dword ptr [ESP+0x38]`.
- Pass 2 second leg: `0x0042C08E MOV [ESP+0x38],EAX`, `0x0042C092 FILD dword ptr [ESP+0x38]`.

### `FootClass__Get_Slope_Speed_Factor @ 0x004DC760`

The helper is a simple x87-returning function:

```text
if foot+0x5D4 != 0 and (*(foot+0x5D4)+0x24)->+0xF2 != 0:
    return 1.0
else:
    return *(double *)(foot+0x530)
```

Evidence:
- Decompile `0x004DC760`.
- Assembly `0x004DC760..0x004DC784`: read `ECX+0x5D4`, null check, read linked object type pointer at `+0x24`, test byte `+0xF2`, `FLD double ptr [0x007E1718]` for `1.0`, otherwise `FLD double ptr [ECX+0x530]`.
- `read_memory 0x007e1718 8` -> `000000000000f03f`, IEEE-754 double `1.0`.

`Foot+0x530` is copied from `TechnoTypeClass+0x2F0` during `FootClass__Unlimbo`, and `TechnoTypeClass+0x2F0` is the `ThreatAvoidanceCoefficient` INI key.

Evidence:
- `FootClass__Unlimbo @ 0x004D72E0` decompile and assembly `0x004D72EA FLD [EAX+0x2F0]`, `0x004D72F4 FSTP [ESI+0x530]`.
- `TechnoTypeClass::ReadINI` assembly `0x00712460 PUSH 0x844420`, `0x00712468 CALL 0x005283D0`, `0x0071246D FSTP [EBP+0x2F0]`.
- `read_memory 0x00844420 32` decodes `"ThreatAvoidanceCoefficient"`.
- Stock `rulesmd.ini` contains harvester values `1` and `.65` at lines 7344, 7402, 8210, 8262, 9034, and 9086.

This confirms the prior correction: `Foot+0x530` is not a per-cell slope cache and not a slope-index speed table. It is a mover scalar sourced from `ThreatAvoidanceCoefficient`, except for the linked-object/type `+0xF2` exemption.

### Pass 1: `Path_smooth_single_segment @ 0x0042B420`

`Path_smooth_single_segment` is called only by `Path_smooth_corners @ 0x0042B210`. It reads the slope scalar once near function setup, reads `Foot+0x21C` once, and then may reject a proposed smoothing segment using a slope product threshold of `1.0`.

Verified order in the inner cell validation:

1. Call virtual `foot->vtable+0x1AC` with `(cell, direction, height, 0, 1)`.
2. If `Can_Enter_Cell` is nonzero, reject the smoothing candidate.
3. Else if `CellClass+0x140 & 0x40000` is set, reject the smoothing candidate.
4. Else call `MapClass__Get_Slope_Cost_At_Cell`.
5. Convert returned `i32` with `FILD`, multiply by saved slope scalar, and compare against double `1.0`.
6. Reject if `slope_cost * scalar >= 1.0`; otherwise keep validating.

Evidence:
- Decompile `0x0042B420`.
- Setup assembly `0x0042B50C..0x0042B51D`: `CALL 0x004DC760`, `MOV EAX,[EBX+0x21C]`, save double scalar and context.
- Call-site assembly `0x0042B5A5..0x0042B5F5`: virtual call at `0x0042B5AE`, marker test at `0x0042B5B8`, slope call `0x0042B5D3`, `FILD`, `FMUL`, `FCOMP [0x007E1718]`, and branch that sets reject byte.

Pass 1 has no `1e-5` enable gate. A zero scalar simply makes the product zero, so slope does not reject.

### Pass 2: `Path_Reroute_Straight_Line @ 0x0042BE20`

`Path_Reroute_Straight_Line` is called by `Path_optimize_straight_segments @ 0x0042B7F0`. It reads the slope scalar once, enables slope checks only if the scalar is strictly greater than `1e-5`, and reads `Foot+0x21C` once.

Verified setup:

1. `FootClass__Get_Slope_Speed_Factor(foot)` at `0x0042BEC3`.
2. Save scalar to stack.
3. Compare scalar to `0x007E3810` (`1e-5`).
4. Set the local slope-enabled byte to false unless scalar is strictly greater.
5. Read `Foot+0x21C` into the saved slope context.

Evidence:
- Decompile `0x0042BE20`.
- Assembly `0x0042BEC3..0x0042BEED`.
- `read_memory 0x007e3810 8` -> `f168e388b5f8e43e`, IEEE-754 double `1e-5`.

Verified per-cell validation order in each leg:

1. Step to the next candidate cell.
2. Get `CellClass`.
3. If slope-enabled, call `MapClass__Get_Slope_Cost_At_Cell`.
4. Convert returned `i32` with `FILD`, multiply by scalar, and compare against `0.01`.
5. If `product >= 0.01`, increment the local steep-cell counter.
6. Call virtual `foot->vtable+0x1AC` for `Can_Enter_Cell`.
7. Reject the candidate ordering if any of these are true:
   - `Can_Enter_Cell` returned nonzero.
   - `CellClass+0x140 & 0x40000` is set.
   - `steep_count >= 4`.
   - strict mode (`param_7 == 0`) and `steep_count >= 1`.

Evidence:
- First leg call site `0x0042BF85`, then `FILD/FMUL/FCOMP [0x007E3808]`, `JNZ`, and `INC [ESP+0x18]` at `0x0042BFA3`.
- Second leg call site `0x0042C089`, same compare/increment sequence at `0x0042C092..0x0042C0A7`.
- Decompile branch condition in `0x0042BE20`.
- `read_memory 0x007e3808 16` -> first double `0.01`, second double `1e-5`.

Important threshold correction from previous audits is confirmed: `0x007E3808` is `0.01`, not `1.01`.

### Caller chain and active YR status

`MapClass__Get_Slope_Cost_At_Cell` has other callers, but this report only claims the path-smoothing use:

| Caller | Call site(s) | In scope? | Finding |
|---|---:|---|---|
| `Path_smooth_single_segment @ 0x0042B420` | `0x0042B5D3` | yes | Uses threshold `1.0` after CanEnter/marker short-circuit. |
| `Path_Reroute_Straight_Line @ 0x0042BE20` | `0x0042BF85`, `0x0042C089` | yes | Uses threshold `0.01`, `1e-5` enable gate, and steep-count rejection. |
| `FUN_006EA0D0` | `0x006EA266` | no | Out of scope. |
| `TeamClass__Find_Best_Target_Building @ 0x006EEBD0` | `0x006EECD1`, `0x006EED30` | no | Out of scope. |
| `FUN_006EEEA0` | `0x006EEF63`, `0x006EEFC2` | no | Out of scope. |

Active path chain:

- `AStar_main_loop @ 0x00429A90` calls `AStar_reconstruct_path @ 0x0042AA90`, then `Path_smooth_corners @ 0x0042B210` at `0x0042A415`, then `Path_optimize_straight_segments @ 0x0042B7F0` at `0x0042A41E`.
- `Path_smooth_corners @ 0x0042B210` calls `Path_smooth_single_segment @ 0x0042B420`.
- `Path_optimize_straight_segments @ 0x0042B7F0` calls `Path_Reroute_Straight_Line @ 0x0042BE20`.

Evidence:
- `get_function_callers 0x0042B420` -> `Path_smooth_corners`.
- `get_function_callers 0x0042BE20` -> `Path_optimize_straight_segments`.
- `get_function_callers 0x0042B210` and `0x0042B7F0` -> `AStar_main_loop`.
- Assembly context `0x0042A415`, `0x0042A41E`.

No TS-only flag, fog-of-war flag, or `SpecialFlags` gate was observed on this path. Active in standard YR: Yes, for successful A* paths that reach the post-processing sequence.

## Active YR Status

Active in YR: Yes.

The path-smoothing callers are reached from the live `AStar_main_loop` after successful path reconstruction. The slope scalar source is also active in YR: `ThreatAvoidanceCoefficient` is parsed by `TechnoTypeClass::ReadINI`, copied into `Foot+0x530` during `FootClass__Unlimbo`, and read by both path-smoothing passes through `FootClass__Get_Slope_Speed_Factor`.

Conditional behavior:

- Pass 1 slope check is effectively disabled only when the returned scalar makes every product below `1.0`; there is no explicit `1e-5` gate.
- Pass 2 slope check is disabled unless scalar is strictly greater than `1e-5`.
- Linked-object/type byte `+0xF2` forces scalar `1.0`; this report verifies the branch but does not identify the exact gameplay cases that set that type byte.

## Rust Delta

Current Rust path smoothing does not implement this slope-cost path.

Verified current Rust surfaces:

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/pathfinding/path_smooth.rs::smooth_path` | Takes `walkable(x,y)` and only checks shortcut/corner cells for passability. | Missing pass-1 `MapClass__Get_Slope_Cost_At_Cell` rejection with `1.0` threshold and marker/CanEnter ordering. |
| `src/sim/pathfinding/path_smooth.rs::optimize_path` | Finds drift segments and calls `reroute_segment` with only `walkable`. | Missing binary pass-2 strict/lenient steep-count model and `0.01` threshold. |
| `src/sim/pathfinding/path_smooth.rs::reroute_segment` | Builds a simplified cardinal/diagonal route and rejects only on `walkable` and corner-cutting. | Missing slope-enabled gate, per-cell slope table lookup, marker rejection, CanEnter order, and `param_7` strictness. |
| `src/sim/pathfinding/terrain_speed.rs` | Has runtime `SlopeClimb`/`SlopeDescend` speed factor for movement execution. | Not the same mechanism; do not reuse it as the path smoothing slope-cost table. |
| Zone-level slope docs/Rust | `Zone_Estimate_Slope_Cost` is a separate `Zone_precheck` pipeline. | This report covers path smoothing, not zone precheck; both need distinct implementations. |

Evidence:
- Codegraph context for `smooth_path`, `optimize_path`, `reroute_segment`.
- `path_smooth.rs` lines 86, 258, and 402 show boolean closure signatures.
- `path_smooth.rs` lines 121, 275, 472, and 478 show passability-only decisions.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0` formula | verified | decompile and assembly `0x0056BCD0..0x0056BD3A` | none for read formula |
| `CellClass+0x24` signed component decode | verified | `MOV ECX,[EAX+0x24]`, signed `MOVSX` low/high words at `0x0056BD02..0x0056BD22` | none |
| Signed division by 4 rounding | verified | `CDQ; AND EDX,3; ADD; SAR 2` at `0x0056BD0A..0x0056BD2C` | none |
| `slope_context+0x59F0` table base/stride | verified | `LEA ECX,[EAX+ESI*2]`, `MOV EAX,[EDX+ECX*4+0x59F0]` at `0x0056BD2F..0x0056BD33` | table construction remains separate |
| Invalid/null cell fallback | verified | `0x0056BCE0..0x0056BD02` | runtime proof that path callers never pass invalid coords is not claimed |
| `FootClass__Get_Slope_Speed_Factor @ 0x004DC760` | verified | decompile and assembly `0x004DC760..0x004DC784` | exact gameplay setters for linked type `+0xF2` not traced |
| `ThreatAvoidanceCoefficient -> TechnoType+0x2F0 -> Foot+0x530` | verified | `0x00712460..0x0071246D`, `0x004D72EA..0x004D72F4`, INI grep | no full constructor/default audit |
| Pass 1 caller `Path_smooth_single_segment @ 0x0042B420` | verified | decompile, setup `0x0042B50C..0x0042B51D`, call site `0x0042B5A5..0x0042B5F5` | full segment rewrite mechanics not re-documented beyond slope gate |
| Pass 2 caller `Path_Reroute_Straight_Line @ 0x0042BE20` | verified | decompile, setup `0x0042BEC3..0x0042BEED`, call sites `0x0042BF85`, `0x0042C089` | full route ordering beyond slope/validation gate belongs to slot 2 |
| A* liveness for smoothing passes | verified | xrefs and assembly `0x0042A415`, `0x0042A41E` | no retail replay route capture |
| Other helper callers outside path smoothing | deferred | xrefs to `0x006EA0D0`, `0x006EEBD0`, `0x006EEEA0` | separate investigation if target-building scoring needs slope-cost parity |
| Rust path smoothing delta | verified | Codegraph context; `path_smooth.rs` lines 86, 258, 402 | implementation not attempted by this research report |

## Open Questions - Final State

- `[RESOLVED] OQ-1 - What is the exact helper entry point? -> `MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0`.` (evidence: `search_functions`, decompile `0x0056BCD0`)
- `[RESOLVED] OQ-2 - Does helper use caller coord or stored cell coord for table indexing? -> Valid cells use stored `CellClass+0x24`; invalid/null lookup writes requested coord into dummy `+0x24` then uses that.` (evidence: `0x0056BCE0..0x0056BD33`)
- `[RESOLVED] OQ-3 - Are x/y components signed? -> Yes, both input lookup and `CellClass+0x24` components are loaded with signed `MOVSX` from 16-bit words.` (evidence: `0x0056BCD4`, `0x0056BCD8`, `0x0056BD0A`, `0x0056BD1F`)
- `[RESOLVED] OQ-4 - Is division by 4 floor or truncation? -> Truncation toward zero via signed bias-add then arithmetic shift.` (evidence: `0x0056BD0F..0x0056BD2C`)
- `[RESOLVED] OQ-5 - What table base and stride are used? -> `slope_context+0x59F0`, 130 columns (`0x82`) with signed i32 entries.` (evidence: `0x0056BD18..0x0056BD33`)
- `[RESOLVED] OQ-6 - How is the return consumed? -> As signed dword through `FILD dword` before double multiplication.` (evidence: `0x0042B5D8..0x0042B5E4`, `0x0042BF8A..0x0042BF96`, `0x0042C08E..0x0042C09A`)
- `[RESOLVED] OQ-7 - Which path-smoothing callers use the helper? -> `Path_smooth_single_segment` and `Path_Reroute_Straight_Line`, with call sites `0x0042B5D3`, `0x0042BF85`, and `0x0042C089`.` (evidence: xrefs to `0x0056BCD0`)
- `[RESOLVED] OQ-8 - What threshold does pass 1 use? -> Product threshold is `>= 1.0`, using constant `0x007E1718`.` (evidence: `0x0042B5DC..0x0042B5EF`, `read_memory 0x007E1718 8`)
- `[RESOLVED] OQ-9 - What threshold and enable gate does pass 2 use? -> Enable gate is scalar `> 1e-5`; steep threshold is product `>= 0.01`.` (evidence: `0x0042BEC3..0x0042BEED`, `0x0042BF8E..0x0042BFA3`, `0x0042C092..0x0042C0A7`, constant reads)
- `[RESOLVED] OQ-10 - Is `Foot+0x530` a per-cell slope cache? -> No; it is the returned slope scalar unless linked-object/type `+0xF2` forces `1.0`.` (evidence: `0x004DC760..0x004DC784`)
- `[RESOLVED] OQ-11 - What initializes `Foot+0x530` for standard units? -> `FootClass__Unlimbo` copies `TechnoTypeClass+0x2F0`, parsed from `ThreatAvoidanceCoefficient`.` (evidence: `0x004D72EA..0x004D72F4`, `0x00712460..0x0071246D`, INI grep)
- `[RESOLVED] OQ-12 - Is this path active in standard YR? -> Yes; successful A* paths call smoothing and optimization from `AStar_main_loop` without observed TS/fog/SpecialFlags gate.` (evidence: `0x0042A415`, `0x0042A41E`)
- `[RESOLVED] OQ-13 - Does current Rust implement the slope-cost checks? -> No; current path smoothing exposes only boolean passability callbacks.` (evidence: Codegraph context; `path_smooth.rs` lines 86, 258, 402)
- `[DEFERRED] OQ-14 - How is every `Foot+0x21C/+0x59F0` slope table entry constructed?` (category: out-of-scope; reason: this report verifies path-smoothing read/use contract only; next-step-if-pursued: dedicated slope-context construction investigation)
- `[DEFERRED] OQ-15 - Which exact stock gameplay cases trigger the linked-object/type `+0xF2` scalar exemption?` (category: requires-different-system-context; reason: branch is verified but all setters/users of that type byte are outside this path-smoothing helper slice; next-step-if-pursued: trace `+0xF2` type-field readers/writers)
- `[DEFERRED] OQ-16 - Do named retail cliff-heavy maps visibly diverge due to this Rust gap today?` (category: needs-runtime-debugger; reason: static binary evidence proves mechanism mismatch, but route capture needs runtime scenario logging; next-step-if-pursued: record gamemd vs Rust path post-processing on a sloped stock map)
- `[DEFERRED] OQ-17 - What do the non-path-smoothing callers at `0x006EA0D0`, `0x006EEBD0`, and `0x006EEEA0` do with the helper?` (category: out-of-scope; reason: target is path smoothing only; next-step-if-pursued: separate target-building/team scoring slope-cost report)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Helper indexes `slope_context+0x59F0` by the chosen cell's `CellClass+0x24`, not by raw caller coordinate after a valid lookup. | `0x0056BD02..0x0056BD33` | missing | future slope-cost accessor used by `path_smooth.rs` | Look up the actual cell coord, split signed x/y shorts, divide each by 4 using truncation toward zero, use stride 130, return signed i32. | Unit test with caller coord mapping to a cell whose stored `+0x24` differs from the input must use stored cell coord. | Do not index directly by Rust path coordinate if the cell object stores a different coord. |
| Invalid/null cell lookup writes requested coord into dummy `+0x24` and still performs the same table formula. | `0x0056BCE0..0x0056BD02` | missing | future slope-cost accessor | Preserve fallback semantics or prove callers never send invalid coords before eliding. | Synthetic invalid coord case should not clamp to map edge; it should follow dummy/requested coord behavior or be explicitly impossible at caller. | Do not silently clamp or return zero on invalid coords without binary-equivalence proof. |
| Signed division by 4 is truncation toward zero: `(v + ((v >> 31) & 3)) >> 2`. | `0x0056BD0A..0x0056BD2C` | missing | future slope-cost accessor | Implement exact signed rounding for negative packed coords. | Table index test for `x=-1,y=-5` must produce `x4=0,y4=-1`, not floor values. | Do not use Rust `div_floor`; normal signed `/ 4` is closer for truncation but must be explicit in tests. |
| Pass 1 rejects a smoothing candidate if CanEnter fails, marker `0x40000` is set, or `slope_cost * scalar >= 1.0`. | `0x0042B5A5..0x0042B5F5` | missing | `smooth_path` / layered smoothing call contract | Add a richer validation callback or context so pass 1 can apply exact CanEnter/marker/slope ordering. | Path with an otherwise walkable shortcut over a cell with slope cost high enough for product `1.0` must not smooth. | Do not hide this in the boolean `walkable` closure unless it can preserve call order and marker behavior. |
| Pass 1 has no `1e-5` slope enable gate. | `0x0042B50C..0x0042B5F5` | missing | `smooth_path` | Always compute product if CanEnter/marker pass; zero scalar naturally yields zero product. | Scalar `0.0` with high table value should not reject; scalar `0.5` with cost `2` should reject. | Do not share pass-2's enable gate with pass 1. |
| Pass 2 enables slope counting only when scalar is strictly greater than `1e-5`. | `0x0042BEC3..0x0042BEED`, `0x007E3810` | missing | `optimize_path` / `reroute_segment` | Carry mover scalar into reroute and skip all slope increments unless `scalar > 1e-5`. | Scalar exactly `1e-5` should not count slopes; slightly above should. | Do not use `>= 1e-5`. |
| Pass 2 increments steep count when `slope_cost * scalar >= 0.01`, before the later CanEnter decision. | `0x0042BF85..0x0042BFA3`, `0x0042C089..0x0042C0A7`, `0x007E3808` | missing | `reroute_segment` | Count steep cells using `0.01` threshold on both route legs. | One nonzero slope-cost cell with scalar high enough should increment count even if later validation rejects for another reason. | Do not reuse stale `1.01` wording or pass-1 `1.0` threshold. |
| Pass 2 rejects if `steep_count >= 4`, and in strict mode (`param_7 == 0`) rejects on the first steep cell. | `0x0042BE20` decompile; call sites from `0x0042B7F0` pass `0` mid-scan and `1` at end-scan. | missing | `optimize_path` / `reroute_segment` | Preserve strict vs lenient reroute call semantics. | Mid-scan reroute with one steep cell must fail; end-scan reroute with three steep cells may pass but four must fail. | Do not model this as a single global "max steep cells = 3" rule. |
| `Foot+0x530` is copied from `TechnoTypeClass+0x2F0` (`ThreatAvoidanceCoefficient`), except linked-object/type `+0xF2` returns `1.0`. | `0x004DC760..0x004DC784`, `0x004D72EA..0x004D72F4`, `0x00712460..0x0071246D` | missing/unchecked in path smoothing | unit pathfinding context fed into smoothing | Provide the path smoother with the exact scalar and slope-context pointer/source equivalent. | Harvester variants with `ThreatAvoidanceCoefficient=1` and `.65` should produce different slope rejection products on the same slope table. | Do not use `SlopeClimb`/`SlopeDescend`, locomotor slope index, or terrain speed multipliers here. |

## Acceptance Tests

1. `path_slope_cost_uses_cellclass_coord_stride_130`: build a slope context with unique `+0x59F0` entries and a cell whose stored coordinate maps to a different coarse bucket than the requested coordinate; assert the stored-cell bucket is returned.
2. `path_slope_cost_signed_division_truncates_toward_zero`: cover negative packed x/y components and assert `-1/4 -> 0`, `-5/4 -> -1`.
3. `smooth_single_segment_rejects_shortcut_at_product_one`: an otherwise enterable shortcut with `slope_cost * scalar == 1.0` must not be smoothed.
4. `smooth_single_segment_allows_below_product_one`: same setup with product just below `1.0` must allow smoothing if CanEnter and marker pass.
5. `reroute_strict_rejects_first_steep_cell`: pass-2 strict mode (`param_7 == 0`) must fail when one candidate cell has product `>= 0.01`.
6. `reroute_lenient_allows_three_steep_cells_but_not_four`: pass-2 lenient mode (`param_7 == 1`) must allow three steep cells and reject four, assuming CanEnter and marker pass.
7. `reroute_slope_gate_is_strictly_greater_than_one_e_minus_five`: scalar exactly `1e-5` disables slope counting; scalar just above enables it.
8. `path_smoothing_uses_threat_avoidance_scalar_not_slopeclimb`: two movers with same terrain but different `ThreatAvoidanceCoefficient` should differ only by the product scalar.

## Remaining Uncertainty

- The construction and update lifecycle of the `Foot+0x21C` slope context arrays remains out of scope. This report verifies the read contract at `+0x59F0`, not where every table value comes from.
- The exact gameplay meaning of the linked object/type `+0xF2` exemption in `FootClass__Get_Slope_Speed_Factor` was not traced to all setters. The branch is verified and active if the fields are populated, but standard stock cases using it need a separate focused report.
- Other non-path-smoothing callers of `MapClass__Get_Slope_Cost_At_Cell` were listed but not investigated.
- Runtime route captures on named cliff-heavy maps were not taken; this report is static binary analysis with Rust surface comparison.

## Sources

- Ghidra decompile: `MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0`.
- Ghidra decompile: `Path_smooth_single_segment @ 0x0042B420`.
- Ghidra decompile: `Path_Reroute_Straight_Line @ 0x0042BE20`.
- Ghidra decompile: `Path_smooth_corners @ 0x0042B210`.
- Ghidra decompile: `Path_optimize_straight_segments @ 0x0042B7F0`.
- Ghidra decompile: `FootClass__Get_Slope_Speed_Factor @ 0x004DC760`.
- Ghidra decompile: `FootClass__Unlimbo @ 0x004D72E0`.
- Assembly contexts: `0x0056BCD0..0x0056BD3A`, `0x0042B50C..0x0042B5F5`, `0x0042BEC3..0x0042C0A7`, `0x004DC760..0x004DC784`, `0x004D72EA..0x004D72F4`, `0x00712460..0x0071246D`, `0x0042A415`, `0x0042A41E`.
- Constant reads: `read_memory 0x007E1718 8` (`1.0`), `read_memory 0x007E3808 16` (`0.01`, `1e-5`), `read_memory 0x00844420 32` (`ThreatAvoidanceCoefficient`).
- Current Rust: `src/sim/pathfinding/path_smooth.rs`, `src/sim/pathfinding/terrain_speed.rs`.
- Prior docs referenced: `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`, `docs/research/SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`, `docs/research/ZONE_ESTIMATE_SLOPE_COST_GHIDRA_REPORT.md`, `docs/research/ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`.
