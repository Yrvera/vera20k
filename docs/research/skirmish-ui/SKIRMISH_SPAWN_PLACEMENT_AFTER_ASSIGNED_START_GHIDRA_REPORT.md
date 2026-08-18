# Skirmish Spawn Placement After Assigned Start - Ghidra Research Report

**Address(es):** `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`, `FUN_005EE6F0`, `FUN_0050E000`, `FUN_005D7030`, `FUN_00688ED0`, `FUN_0050DF30`, `FUN_0050DEF0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard selected-mode offline Skirmish placement after a house already has or receives an assigned start cell: start-cell selection into `House+0x5490`, standard Battle/ManBattle/FreeForAll/Cooperative MCV/base-unit direct placement, and shared blocked-cell fallback needed for the first playable frame.
**Non-Scope:** shell packing, choose-map UI, random terrain generation, full `UnitCount` extra-unit spending, custom Siege/Unholy MCV callbacks, full mission scheduler/deploy facing, and exact RNG replay transcript.
**Confidence:** High for standard selected-mode Battle-style placement and fallback helper; Medium for naming of generic object-type `0xF` compatibility because the vtable slot name/type enum was not re-audited here.
**Active in YR:** Yes for standard selected offline Skirmish with a selected MPModes object; conditional for null-mode `Generate_Random_Units`, which is not the normal selected-mode branch.

## 0. Working Notes

Target question: After Start/house assignment has selected a Skirmish start cell for a house, exactly how does `gamemd.exe` place the starting MCV/base unit and recover from blocked or missing exact cells?

Non-goals: Do not re-investigate shell packing, UI controls, map chooser, random terrain formulas, full extra-unit budget, or mode-specific Siege/Unholy start logic.

Evidence needed to mark COMPLETE: decompile plus assembly range for start assignment into `House+0x5490`; decompile plus caller/vtable evidence for standard MCV placement; decompile plus assembly range for `FUN_00688ED0` direct and fallback placement; INI/default source for `BaseUnit`/`Bases`; Rust source scan for current placement/fallback delta.

Stop conditions: stop at standard selected-mode placement once the exact direct-place and fallback behavior are known; defer custom mode callbacks and deploy mission scheduling; do not mutate Ghidra or Rust.

## 1. Overview

Standard selected offline Skirmish does not place an MCV directly from the shell start selection. `ScenarioClass__AssignStartingPoints` / the Battle-style mode `+0x84` start helper first chooses a start cell and writes it to `House+0x5490`. Later, `ScenarioClass__Post_Map_Init @ 0x00686890` calls selected mode `+0x84`, then `FUN_005D6D80`, whose standard `+0xC8` callback at `0x005D7030` creates the MCV and tries to place it at that house base cell.

The standard MCV placement path tries the exact base cell first using centered leptons. If exact placement fails, it calls `FUN_00688ED0` with radius `1`; that helper searches outward through radius `1..31`, trying compass directions and then a one-cell randomized jitter pass per radius/direction, stopping at the first successful `Place`. If all attempts fail, the MCV object is deleted and that house gets no starting MCV from this callback.

## 2. Key Offsets And Globals

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass+0x1180 + index*4` | start-index table: `-1` unassigned, otherwise house index | `0x005EE9DE..0x005EEA03`, `0x005EE6F0` writes | Yes |
| `House+0x5490` | primary base/start cell used by MCV placement | `FUN_0050E000 @ 0x0050E000`; readers `0x0050DF30`, `0x0050DEF0` | Yes |
| `House+0x5494` | alternate/override base cell; if not invalid, it wins over `+0x5490` | `0x0050DF30`, `0x0050DEF0` invalid-cell checks | Yes |
| `DAT_00A8B258` | `Bases` option; gates standard MCV creation | `0x005D7030` entry; `rulesmd.ini [MultiplayerDialogSettings] Bases=yes` | Yes |
| `RulesClass+0xB20` | `[General] BaseUnit` vector | `0x005D7051..0x005D7059`, `FUN_00505310`; `rulesmd.ini BaseUnit=AMCV,SMCV,PCV` | Yes |
| playfield bounds globals `DAT_0087F90C/910/914/918` | min X/Y and width/height clamps used by fallback | `FUN_00688ED0 @ 0x00689013..0x006890A6` | Yes |
| `g_InvalidCell` / `DAT_00B05458` | invalid cell sentinel | `0x0050DF30`, `0x0050DEF0`, `0x00688380` waypoint/fallback tests | Yes |

## 3. Core Logic

### 3.1 Assigned start becomes `House+0x5490`

`ScenarioClass__AssignStartingPoints @ 0x005EE9D0` first calls `ScenarioClass__Gather_Start_Positions @ 0x00688380`, then builds a 16-byte occupied table from `ScenarioClass+0x1180`: byte `1` means the corresponding start index is already explicitly assigned to a house, byte `0` means free. It loops non-special player-control houses first (`House+0x1EC != 0`), then non-special AI/non-player-control houses. Active in YR: Yes. Evidence: decompile `0x005EE9D0`; assembly `0x005EE9D9` calls gather, `0x005EE9E6..0x005EEA03` builds the occupied table, and the two house loops are visible in the decompile.

If a human-controlled house already appears in `ScenarioClass+0x1180`, the matching start index is used directly. Otherwise `FUN_005EE6F0` chooses a free start and writes the chosen house index back to `ScenarioClass+0x1180 + start_index*4`. Active in YR: Yes. Evidence: decompile `FUN_005EE6F0`; assembly context around `0x005EEA78..0x005EEA8D` passes the occupied table and house index.

After choosing a start cell, `AssignStartingPoints` calls `FUN_0050E000`, which writes exactly one dword cell value to `House+0x5490`. Active in YR: Yes. Evidence: decompile `FUN_0050E000`; assembly `0x0050E000..0x0050E00A` is `MOV [ECX+0x5490], EAX; RET 0x4`.

`FUN_005EE6F0` selection details that affect output:

- If no start is occupied yet and the caller's `param_5` is nonzero, it chooses a random start index in `0..count-1`.
- If exactly two starts are occupied and `param_5 == 0`, it picks a random unoccupied start by ordinal using `Random(0, count-3)`.
- Otherwise, with more than one occupied start, it chooses the unoccupied start with the greatest summed Euclidean distance to all occupied starts.
- With exactly one occupied start, it chooses the unoccupied start with the smallest summed distance to occupied starts.
- Distances are `sqrt(dx*dx + dy*dy)` converted with `Math__ftol` before summing.

Active in YR: Yes for standard start assignment. Evidence: decompile `FUN_005EE6F0`; assembly `0x005EE7D2` branches on occupied count, `0x005EE850..0x005EE85B` performs sqrt/ftol accumulation.

### 3.2 Deficient start list generation

`ScenarioClass__Gather_Start_Positions @ 0x00688380` scans scenario waypoints `0..7` until invalid sentinel or eight entries, counts required non-observer human plus AI starts, and if the real waypoint count is deficient it appends random passable fallback cells until it has enough starts. Active in YR: Yes. Evidence: decompile `0x00688380`; it reads node observer markers from `DAT_00A8DA78`, AI count `DAT_00A8B274`, and calls `FootClass__Find_Nearby_Passable_Cell`.

The deficient fallback seed cell is not anywhere on the map. It draws `x` from `Random(10, width-10)` plus the map min X, and `y` from `Random(0, height-10)` plus `10 + minY`, then asks `FootClass__Find_Nearby_Passable_Cell(..., 8, 8, ..., 1, ...)`. Active in YR: Conditional on maps with too few starts. Evidence: decompile `0x00688380`, fallback branch after log string `Multiplayer start waypoint defic...`.

### 3.3 Standard selected-mode MCV creation and direct placement

`ScenarioClass__Post_Map_Init @ 0x00686890` is the active selected-mode entry: if `DAT_00A8B23C != null`, it calls selected mode vtable `+0x84` and then `FUN_005D6D80`; only the null-selected-mode branch calls `ScenarioClass__Generate_Random_Units @ 0x006886B0`. Active in YR: Yes/Conditional; standard offline Skirmish has a selected MPModes object. Evidence: decompile `0x00686890`; assembly previously verified at `0x00686928..0x00686940`.

For standard Battle/ManBattle/FreeForAll/Cooperative MPModes, the `+0xC8` callback is `FUN_005D7030`. If `Bases` (`DAT_00A8B258`) is false, it returns success without creating an MCV. If true, it resolves the MCV type through `FUN_00505310(Rules+0xB20)`, constructs a `UnitClass`, converts the house base cell to coordinates with `FUN_0050DF30`, and calls the object's `Place` vtable slot `+0xD8`. Active in YR: Yes for standard selected modes. Evidence: decompile `0x005D7030`; assembly `0x005D7030..0x005D7041` gates `DAT_00A8B258`, `0x005D7090..0x005D709E` calls `FUN_0050DF30` then `Place`.

`FUN_0050DF30` chooses `House+0x5494` if it is not invalid; otherwise it uses `House+0x5490`. If the chosen cell is invalid, it returns global fallback coordinates `DAT_00A8EFF8..F000`. Otherwise it asks the cell object for a coordinate through vtable `+0x48`. Active in YR: Yes. Evidence: decompile `0x0050DF30`; assembly `0x0050DF30..0x0050DF5F` performs the invalid-cell branch.

### 3.4 Blocked-start fallback `FUN_00688ED0`

If direct `Place` fails, `FUN_005D7030` calls `FUN_0050DEF0` to get the selected base cell (`+0x5494` if valid else `+0x5490`), then calls `FUN_00688ED0(mcv, base_cell, 1)`. If that returns false, the MCV is deleted through vtable `+0x20`. Active in YR: Yes. Evidence: decompile `0x005D7030`; assembly `0x005D70AD..0x005D70C4` calls `FUN_0050DEF0`, then `FUN_00688ED0`, then tests the result.

`FUN_00688ED0` first retries the exact requested cell only if it is inside the playfield and the nearest object gate allows placement: no nearest object, or both nearest object and candidate object report vtable `+0x2C == 0xF`. It places at `x = cell_x * 0x100 + 0x80`, `y = cell_y * 0x100 + 0x80`, `z = ground height`. Active in YR: Yes. Evidence: decompile `0x00688ED0`; assembly `0x00688EDD..0x00688F29` performs playfield/object gate and `0x00688F52..0x00688F9A` computes centered leptons and calls `Place`.

If exact placement fails, the fallback radius starts at the caller-provided radius and increments through `31` inclusive. Standard MCV callback passes radius `1`; standard extra-unit callback passes radius `4`. Active in YR: Yes for MCV radius 1; extra-unit radius 4 is supporting context. Evidence: `0x005D70B1` pushes `1`; `0x005D70BF` calls `0x00688ED0`; `FUN_00688ED0` returns false when `param_3 > 0x1F`.

For each radius, the helper chooses a random starting compass direction with `Random(0,7)`, then tries eight directions in rotating order. Direction mapping is:

| Direction | Candidate cell |
|---:|---|
| 0 | `(x, y-radius)` |
| 1 | `(x+radius, y-radius)` |
| 2 | `(x+radius, y)` |
| 3 | `(x+radius, y+radius)` |
| 4 | `(x, y+radius)` |
| 5 | `(x-radius, y+radius)` |
| 6 | `(x-radius, y)` |
| 7 | `(x-radius, y-radius)` |

Each direction candidate is clamped to the current map playfield bounds before testing. Active in YR: Yes. Evidence: decompile `0x00688ED0`; assembly `0x00689032` jump table and clamp context `0x00689091..0x006890A6`.

For each radius/direction, there are two passes. Pass 0 tests the compass candidate. Pass 1 jitters the candidate independently on X and Y by `Random(0,1)` cells and chooses plus vs minus with `Random(0,99) < 0x32` for each axis, then clamps again. The original requested cell is explicitly skipped. Active in YR: Yes. Evidence: decompile `0x00688ED0`; assembly `0x0068919B..0x006891A6` marks original-cell skip and `0x00689268..0x00689283` computes centered coordinates before `Place`.

The first fallback cell that passes playfield/object gate and object `Place` returns success immediately. If no candidate succeeds through radius 31, the helper returns `0`; the standard MCV callback deletes the MCV and returns failure. Active in YR: Yes. Evidence: decompile `0x00688ED0` and `0x005D7030`; assembly `0x005D70C4` tests fallback result and falls through to vtable `+0x20` delete on failure.

## 4. INI Keys

| INI key | Stock YR value | Binary consumer | Effect | Active in YR |
|---|---|---|---|---|
| `[MultiplayerDialogSettings] Bases` | `yes` in `rulesmd.ini` | `DAT_00A8B258`, `0x005D7030` | false skips standard MCV creation entirely | Yes |
| `[General] BaseUnit` | `AMCV,SMCV,PCV` in `rulesmd.ini` | `Rules+0xB20`, `FUN_00505310`, `0x005D7030` | first side-mask matching entry supplies opening MCV type | Yes |
| `[SpecialFlags] MCVDeploy` | map/session flag, not default standard shell option | null-mode `0x006886B0` checks `ScenarioClass flags & 0x10`; no check found in standard `0x005D7030` | auto-deploy is not part of the standard selected-mode MCV callback verified here | Conditional |

## 5. Integration Points

`ScenarioClass__Full_Init @ 0x00686B20` sets up houses and the start table before map/post-map initialization. For selected standard Skirmish, `Post_Map_Init` later calls selected mode `+0x84` and `FUN_005D6D80`; `FUN_005D6D80` iterates non-special, non-observer houses and calls selected mode `+0xC8`, where the standard `0x005D7030` MCV placement path lives.

The null-selected-mode `Generate_Random_Units @ 0x006886B0` contains a similar direct-place plus `FUN_00688ED0` fallback body and an `MCVDeploy` flag check. It is useful corroboration for the placement helper but is not the standard selected MPModes branch when `DAT_00A8B23C` is non-null.

## 6. Current Rust Implementation Status

Current Rust has a launch-session path in `src/app_skirmish.rs::apply_skirmish_launch_session` that creates launch houses, assigns explicit starts first, assigns auto starts to the first unused waypoint, and calls `Simulation::spawn_object` at the waypoint cell. It marks deficient starts as unsupported but does not generate native fallback start cells.

`src/app_skirmish.rs::assign_launch_starts` lacks the native human-first/AI-second distance/random selection behavior and does not write a start table equivalent before placement. It also does not use the native deficient-waypoint random passable-cell generation.

`src/sim/world/world_spawn.rs::spawn_object` places directly at `(rx, ry)` using height-map Z. It does not model the native object `Place` failure path, `FUN_00688ED0` fallback radius/direction/jitter search, or deletion-on-total-failure semantics for startup MCVs.

`src/skirmish_launch.rs::LaunchCountry::opening_mcv_candidates` and `src/app_skirmish.rs::launch_mcv_type_for_country` use hardcoded country candidate lists. Native standard selected-mode MCV type comes from parsed `[General] BaseUnit` plus side/house mask matching.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AssignStartingPoints` occupied-table and house loops | verified | decompile `0x005EE9D0`; assembly `0x005EE9D9..0x005EEA03` | none for this slice |
| `FUN_005EE6F0` start picker | verified | decompile `0x005EE6F0`; assembly `0x005EE7D2`, `0x005EE850` | exact RNG seed replay deferred |
| `Gather_Start_Positions` deficient waypoint fallback | verified | decompile `0x00688380` | exact `FootClass__Find_Nearby_Passable_Cell` internals out of scope |
| `House+0x5490` writer | verified | `FUN_0050E000`; assembly `0x0050E000..0x0050E00A` | none |
| standard selected MCV callback | verified | decompile `0x005D7030`; prior vtable docs for standard modes | custom Siege/Unholy callbacks deferred |
| `FUN_0050DF30` / `FUN_0050DEF0` base-cell accessors | verified | decompile and assembly `0x0050DF30`, `0x0050DEF0` | none |
| `FUN_00688ED0` fallback placement | verified | decompile `0x00688ED0`; assembly contexts listed in sources | exact abstract-type enum name for `0xF` deferred |
| null-mode `Generate_Random_Units` contrast | touched-not-exhausted | decompile `0x006886B0` | not standard selected-mode branch |
| current Rust delta | verified | `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/sim/world/world_spawn.rs` | implementation future work |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this the standard selected Skirmish path or null-mode fallback? -> Standard selected path is `Post_Map_Init -> mode +0x84 -> FUN_005D6D80 -> mode +0xC8`; null-mode `Generate_Random_Units` is not used when `DAT_00A8B23C` is non-null.` (evidence: `0x00686890`)
- `[RESOLVED] OQ-02 - Which field stores the assigned start cell used by MCV placement? -> `House+0x5490`, with `House+0x5494` as an override if valid.` (evidence: `0x0050E000`, `0x0050DF30`, `0x0050DEF0`)
- `[RESOLVED] OQ-03 - Does exact placement use cell corner or center? -> Centered leptons, `cell * 0x100 + 0x80`, plus ground height.` (evidence: `0x00688ED0`, `0x00688F52..0x00688F9A`)
- `[RESOLVED] OQ-04 - What happens when the exact MCV start cell is blocked? -> Standard MCV callback calls `FUN_00688ED0` with radius `1`; fallback searches through radius 31 and deletes the MCV on total failure.` (evidence: `0x005D70AD..0x005D70C4`, `0x00688ED0`)
- `[RESOLVED] OQ-05 - Are deficient waypoints no-spawn? -> No; `Gather_Start_Positions` appends random passable fallback cells until enough starts exist.` (evidence: `0x00688380`)
- `[RESOLVED] OQ-06 - Does `Bases=no` still run MCV placement? -> No; standard callback returns success before MCV creation/placement.` (evidence: `0x005D7030..0x005D7041`)
- `[RESOLVED] OQ-07 - Does standard selected `0x005D7030` queue `MCVDeploy`? -> No check/call found in `0x005D7030`; the observed flag check is in null-mode `0x006886B0`.` (evidence: decompile `0x005D7030`, decompile `0x006886B0`)
- `[RESOLVED] OQ-08 - Does current Rust implement native blocked-start fallback? -> No; it calls `spawn_object` directly and logs failure.` (evidence: `src/app_skirmish.rs`, `src/sim/world/world_spawn.rs`)
- `[DEFERRED] OQ-09 - Exact internals of `FootClass__Find_Nearby_Passable_Cell` used for deficient start generation.` (category: out-of-scope; reason: this slice only needed to prove deficient starts produce fallback cells before placement; next-step-if-pursued: targeted passable-cell search investigation)
- `[DEFERRED] OQ-10 - Custom Siege/Unholy MCV callback placement differences.` (category: out-of-scope; reason: standard selected Battle-style callback is enough for this slot; next-step-if-pursued: decode `0x005CAAC0` and `0x005CB440`)
- `[DEFERRED] OQ-11 - Exact runtime RNG seed replay for fallback direction/jitter.` (category: needs-runtime-debugger; reason: static ranges/call order are verified, but full seed transcript needs runtime tracing; next-step-if-pursued: breakpoint `0x00688ED0` and log `Random__RandomRanged` outputs)
- `[DEFERRED] OQ-12 - Exact object abstract-type enum name for `Place` compatibility code `0xF`.` (category: requires-different-system-context; reason: not needed to implement first-pass blocked-start behavior beyond the verified code check; next-step-if-pursued: audit abstract type enum/vtable `+0x2C`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Assigned starts become house base cell `+0x5490`; MCV placement uses `+0x5494` override if valid else `+0x5490` | `0x0050E000`, `0x0050DF30`, `0x0050DEF0` | missing/mismatch; Rust stores `base_center` after direct spawn only | `src/app_skirmish.rs`, `src/sim/house_state.rs` or future scenario-start module | keep an explicit per-house assigned base cell before unit generation and place MCVs from that cell | explicit local and AI starts produce house base cells before MCV creation and are not just zipped waypoint/spawn pairs | `skirmish_assigned_start_sets_house_base_cell_before_mcv_spawn` | Do not treat shell selected-start index as a direct spawn command |
| Standard MCV callback uses `[General] BaseUnit` first side-mask match and `Bases` gates only MCV creation | `0x005D7030`, `FUN_00505310`, `rulesmd.ini BaseUnit`, `rulesmd.ini Bases=yes` | Rust hardcodes MCV candidates by launch country | `src/skirmish_launch.rs`, `src/app_skirmish.rs`, rules object masks | resolve starting MCV type from parsed BaseUnit vector and side/house mask, and skip MCV creation when `Bases=no` | Yuri slot receives `PCV` because BaseUnit mask matches, and `Bases=no` creates no opening MCV while still leaving house/session state | `skirmish_start_mcv_uses_baseunit_mask_and_bases_gate` | Do not hardcode AMCV/SMCV/PCV order outside parsed INI data |
| Exact-cell placement is centered leptons with native `Place`; failure calls `FUN_00688ED0` radius 1 and deletes only after total failure | `0x005D7090..0x005D70C4`, `0x00688ED0` | missing; Rust `spawn_object` directly inserts or fails | `src/app_skirmish.rs`, `src/sim/world/world_spawn.rs`, occupancy/place helper surface | startup MCV placement should try exact cell, then native radius/direction/jitter fallback around base cell before giving up | if the assigned start cell is occupied by a neutral object, the MCV appears at the first valid fallback cell instead of failing the house start | `skirmish_mcv_start_uses_radius1_fallback_when_start_cell_blocked` | Do not mark deficient/blocked starts as unsupported no-spawn if native can find a fallback cell |
| Deficient waypoint maps append random passable starts using 8x8 clearance before assignment | `0x00688380` | Rust sets `unsupported_deficient_starts` and drops unassigned slots | `src/app_skirmish.rs::assign_launch_starts`, future map passability query | generate enough fallback starts for active houses when map waypoints are deficient | a two-player Skirmish map with one valid start still starts both houses, with the second base cell produced by passable fallback search | `skirmish_deficient_waypoints_generate_passable_fallback_start` | Do not convert deficient start pools into launch failure or missing MCVs |
| `MCVDeploy` evidence in this pass belongs to null-mode `Generate_Random_Units`, not standard selected `0x005D7030` | decompile `0x005D7030`, `0x006886B0` | existing docs imply queued deploy generically for startup MCVs | future startup deploy flag work | keep auto-deploy implementation gated by the verified active path; re-check selected-mode callbacks before applying to all modes | standard selected Battle start with `MCVDeploy` flag does not auto-convert unless a verified selected-mode path queues it | `skirmish_standard_selected_mcv_callback_does_not_assume_null_mode_mcvdeploy` | Do not copy null-mode `Generate_Random_Units` `MCVDeploy` behavior into standard selected modes without a selected-mode caller |

## 10. Negative Facts / Do Not Do

- Do not model blocked exact starts as no-spawn by default. Active in YR: Yes. Evidence: `0x005D70AD..0x005D70C4` calls `FUN_00688ED0` before deleting the MCV.
- Do not treat deficient map waypoints as launch failure. Active in YR: Conditional on deficient maps. Evidence: `0x00688380` appends random passable cells with `FootClass__Find_Nearby_Passable_Cell`.
- Do not use null-mode `ScenarioClass__Generate_Random_Units @ 0x006886B0` as the standard selected MPModes implementation. Active in YR: No for standard selected mode. Evidence: `0x00686890` calls it only when `DAT_00A8B23C == null`.
- Do not hardcode country-to-MCV IDs when `[General] BaseUnit` and type masks are available. Active in YR: Yes. Evidence: `0x005D7030` calls `FUN_00505310(Rules+0xB20)`.
- Do not assume standard selected `0x005D7030` queues `MCVDeploy`; no such check appears there. Active in YR: No for this verified callback. Evidence: decompile `0x005D7030`; the observed flag check is in null-mode `0x006886B0`.

## 11. Remaining Uncertainty

- Exact `FootClass__Find_Nearby_Passable_Cell` 8x8 search internals remain out of scope; this report only verifies that deficient starts call it and append valid results.
- Exact RNG seed replay for direction/jitter ordering needs a runtime debugger trace if deterministic replay tests require byte-for-byte sequence matching.
- Custom Siege `+0xC8 = 0x005CAAC0` and Unholy `+0xC8 = 0x005CB440` may use different MCV/base behavior and require separate mode-specific reports.
- The symbolic name for abstract type code `0xF` in the `FUN_00688ED0` nearest-object compatibility gate was not re-audited.

## 12. Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`: replace `Placement fallback / queued deploy | NOT IMPLEMENTED | starting MCV placement uses native Place plus fallback helpers; MCVDeploy queues normal Deploy mission after placement` with: `Placement fallback | NOT IMPLEMENTED | standard selected-mode MCV placement uses native Place from the house base cell plus FUN_00688ED0 radius-1 fallback before deleting the MCV. The MCVDeploy queueing evidence belongs to null-mode Generate_Random_Units and should not be applied to standard selected MPModes until a selected-mode callback is verified.`
- `docs/research/MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`: add a caveat after the Overview: `Live re-check of standard selected-mode callback 0x005D7030 found no MCVDeploy flag check; the checked flag path is in null-mode ScenarioClass__Generate_Random_Units @ 0x006886B0. Treat selected-mode startup auto-deploy as unverified pending a dedicated selected-mode MCVDeploy trace.`

## Sources

- Ghidra decompiled/read-only: `ScenarioClass__Post_Map_Init @ 0x00686890`, `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`, `ScenarioClass__Gather_Start_Positions @ 0x00688380`, `FUN_005EE6F0`, `FUN_0050E000`, `FUN_0050DF30`, `FUN_0050DEF0`, `FUN_005D7030`, `FUN_00688ED0`, `ScenarioClass__Generate_Random_Units @ 0x006886B0`, `Force_MCV_Deploy @ 0x004FC060`.
- Ghidra assembly context: `0x0050E000..0x0050E00A`, `0x0050DF30..0x0050DF5F`, `0x005D7030..0x005D7041`, `0x005D7090..0x005D70C4`, `0x005EE9D9..0x005EEA03`, `0x005EEA78..0x005EEA8D`, `0x00688EDD..0x00688F29`, `0x00688F52..0x00688F9A`, `0x00689032`, `0x0068919B..0x006891A6`, `0x00689268..0x00689283`.
- Existing docs referenced: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`, `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`, `MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/sim/world/world_spawn.rs`.

## Status

COMPLETE for standard selected-mode assigned-start MCV/base-unit placement and blocked-start fallback. PARTIAL only for custom mode callbacks, full passable-cell internals, and runtime RNG replay, all explicitly outside this slot.
