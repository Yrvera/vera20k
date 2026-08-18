# Paradrop Infantry Subcell Placement - Ghidra Research Report

**Address(es):** `0x00415C60` (`AircraftClass__Drop_Payload`), `0x00481180` (`CellClass__PlaceInfantryInCell`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard YR paradrop payload release from an already-loaded carrier into an infantry passenger's final ground coordinate/subcell, including failure restoration through `PlaceInfantryInCell` and `Unlimbo`.
**Non-Scope:** Carrier spawn edge selection, superweapon launch list construction, mission timing/cadence beyond the immediate `Drop_Payload` caller, PARACH render-depth/anim attachment, and exact global RNG-stream parity.
**Confidence:** High for call ordering, failure branches, invalid sentinel handling, quadrant rules, preference-table bytes, and subcell coordinate offsets. Medium only for exact global RNG-stream parity.
**Active in YR:** Yes. `AircraftClass__Drop_Payload` is reached from standard paradrop overfly/rescue mission flow for `Type=ParaDrop` and `Type=AmerParaDrop` carriers.

## 1. Overview

When a paradrop carrier releases infantry, gamemd does not use the aircraft's V-pattern lepton coordinate as the final infantry position. `AircraftClass__Drop_Payload` computes that V-pattern coordinate, uses it as the candidate input, and then calls `CellClass__PlaceInfantryInCell` to snap the passenger to one of the functional infantry subcells in the target cell.

The important player-visible effect is that dropped infantry land on ordinary infantry subcell centers, not on the raw half-cell V-pattern coordinate. If placement or unlimbo fails, gamemd restores the same passenger to the carrier cargo and restores the payload count so the drop can retry with the same parity.

## 2. Class Layout / Key Offsets

| Field / object | Offset / source | Type | Purpose |
|---|---:|---|---|
| Aircraft cargo list | `aircraft + 0x114` passed to `FUN_00473430` / `CargoClass__AddPassenger` | cargo container | Head passenger is popped for each drop and re-added on failure. |
| Aircraft payload count | `aircraft + 0x2FC` (`param_1[0xBF]`) | integer | Decremented before parity calculation; incremented back on failure. |
| Aircraft current coordinate | vtable slot `+0x48` | coord pointer | Base coordinate for V-pattern release math. |
| Passenger cell-entry check | passenger vtable slot `+0x1AC` | virtual method | Called before `PlaceInfantryInCell`; nonzero result means placement attempt fails and passenger is restored. |
| Passenger unlimbo | passenger vtable slot `+0xE8` | virtual method | Called with the `PlaceInfantryInCell` output coordinate; false result restores passenger/counter. |
| Passenger limbo/drop-in restore | passenger vtable slot `+0x11C` | virtual method | Called after `CargoClass__AddPassenger` on failure. |
| Cell ground occupancy flags | `CellClass + 0x124` (`param_1[0x49]`) | bitfield | Used by `PlaceInfantryInCell` when `param_5 == 0`, which is the paradrop call. |
| Cell alternate occupancy flags | `CellClass + 0x128` (`param_1[0x4A]`) | bitfield | Used only when `param_5 != 0`; not used by paradrop. |
| Invalid coordinate sentinel | `DAT_00889E88/8C/90` in `Drop_Payload`; `DAT_0089E778/7C/80` in `PlaceInfantryInCell` | coord triplet | Compared after placement. If returned, drop fails and passenger is restored. |
| Last dropped cell scratch | passenger `+0x55C` | packed cell | Set after successful unlimbo from the final coordinate's cell, not the raw V input. |
| Aircraft landing state mirror | `aircraft + 0x6D3` | byte | Set to `5` after successful drop. |
| Aircraft last drop frame | `aircraft + 0x2EC` | frame counter | Set to `g_CurrentFrameCounter` after successful drop. |
| Aircraft last drop coordinate scratch | `aircraft + 0x2F0` | coordinate component | Set after successful drop from stack value. |
| Aircraft scratch clear | `aircraft + 0x2F4` | integer | Cleared to `0` after successful drop. |

## 3. Core Logic

### Drop payload ordering

At `AircraftClass__Drop_Payload @ 0x00415C60`:

1. Pop cargo head via `FUN_00473430`.
2. If no passenger was popped, return `0`.
3. Decrement payload count at `aircraft + 0x2FC`.
4. Read aircraft coordinate via vtable slot `+0x48`.
5. Choose V-pattern side from the already-decremented payload count parity:
   - even post-decrement count: adds `+0x3FFF` before the later angle correction,
   - odd post-decrement count: adds `-0x3FFF`.
6. Apply sine/cosine lookup and `ftol` to build the V-pattern candidate coordinate.
7. Get the target `CellClass` for that coordinate.
8. Call passenger vtable slot `+0x1AC` with `(cell, -1, -1, 0, 1)`.
9. Only if that returns `0`, call `CellClass__PlaceInfantryInCell`.
10. If placement returns the invalid coordinate sentinel, restore passenger and payload count.
11. If placement returns a valid coordinate, call passenger vtable slot `+0xE8` (`Unlimbo` path).
12. If unlimbo fails, restore passenger and payload count.
13. If unlimbo succeeds, play chute sound, set passenger last cell, attach parachute when relevant, set aircraft drop bookkeeping, and return `0`.

The restoration path is shared for cell-entry failure, placement sentinel failure, and unlimbo failure:

1. `CargoClass__AddPassenger` is called with the passenger.
2. Passenger vtable slot `+0x11C` is called.
3. Payload count at `aircraft + 0x2FC` is incremented back.
4. Function returns `0`.

### PlaceInfantryInCell paradrop call arguments

The call at `0x00415DAA` pushes five explicit arguments after setting `ECX` to the target `CellClass`:

| Argument | Paradrop value | Meaning in decompiled function |
|---|---:|---|
| `this` | target `CellClass` from V coordinate | Cell receiving the infantry. |
| `param_2` | output coord stack pointer | Receives final placement coordinate or invalid sentinel. |
| `param_3` | input V-pattern coord pointer | Supplies low-byte subcell offset and base cell coordinate. |
| `param_4` | `0` | Enables normal placement checks. |
| `param_5` | `0` | Uses ground occupancy flags at `CellClass + 0x124`; no bridge/alternate layer offset. |
| `param_6` | `0` | Base coordinate is derived by stripping low bytes from the input V coordinate. |

### Subcell quadrant selection

At `CellClass__PlaceInfantryInCell @ 0x00481180`:

1. It reads the low byte of input X and Y from `param_3`.
2. It measures distance from `(128,128)`.
3. If distance is `< 0x3C` (`60` leptons), selected quadrant is `0`.
4. Otherwise it builds quadrant bits:
   - bit 0 if `sub_x > 128`,
   - bit 1 if `sub_y > 128`.
5. If the bits are `0`, selected quadrant remains `0`.
6. If bits are nonzero, selected quadrant is `bits + 1`, producing only `2`, `3`, or `4`.
7. The function never produces quadrant `1` from this logic.

For the traced south-facing V-pattern examples where the input subcell is `(0,128)`, the distance is `128`, but both strict comparisons are false (`0 > 128` false, `128 > 128` false), so the quadrant is still `0`.

### Base coordinate

For paradrop (`param_6 == 0`), the final coordinate starts from the input coordinate's cell origin:

| Component | Formula |
|---|---|
| base X | `input_x - (input_x & 0xFF)` |
| base Y | `input_y - (input_y & 0xFF)` |
| base Z | `input_z` |

This is why the raw V-pattern subcell offset is not preserved as final XY. The low byte only influences quadrant/preference selection.

### Placement availability checks

For paradrop (`param_4 == 0`, `param_5 == 0`):

1. If bit `0x20` is set in the selected occupancy flag word, return the invalid coordinate sentinel immediately.
2. If bit `0x40` is set in the ground cell flags, the function checks for a building occupier and garrison eligibility:
   - if no building is found, return invalid;
   - if the building type flag at `type + 0x16B7` is false, return invalid (corrected 2026-07-18: doc previously said "continue"; binary shows `(iVar8 == 0) || (*(char *)(*(int *)(iVar8 + 0x520) + 0x16b7) == '\0')` sharing one branch to the invalid-sentinel target `LAB_004812ec`, i.e. no-building and flag-false are the SAME early-out, not sequential steps — via `decompile_function 0x00481180` — OPERATOR_OR_ORDER_DRIFT);
   - only when a building is found AND the type flag is true does the function call `BuildingClass__CanGarrison`; if that returns false, return invalid; if true, continue to placement selection.
3. If selected quadrant is `0`, call `Random__RandomRanged(0,3)` and select one of four random rotation tables at `DAT_0081CC98 + result * 4`.
4. If selected quadrant is not `0`, check direct occupancy of bit `1 << quadrant`; if free, go straight to that subcell coordinate. If occupied, use the fixed preference table at `DAT_0081CC84 + quadrant * 4`.
5. The search loop explicitly skips table entries `0` and `1`; only entries `2`, `3`, and `4` can become final infantry positions.
6. If no valid functional subcell is found after four table entries, return the invalid coordinate sentinel.

### Final coordinate construction

When a subcell is accepted:

1. X offset is read from `DAT_0089E9F0[subcell * 3]`.
2. Y offset is read from `DAT_0089E9F4[subcell * 3]`.
3. Z offset is read from `DAT_0089E9F8[subcell * 3]`.
4. These offsets are added to the base cell coordinate.
5. `CellClass__GetGroundHeight` supplies final ground height.
6. Because paradrop passes `param_5 == 0`, the alternate-layer height addition at `DAT_0089E7B4` is not applied.
7. The output coordinate is written to `param_2`.

The runtime initializer at `0x0048E480` writes the subcell coordinate table used by `CellClass__PlaceInfantryInCell`:

| Subcell | Offset |
|---:|---|
| `0` | `(128,128)` |
| `1` | `(64,64)` |
| `2` | `(192,64)` |
| `3` | `(64,192)` |
| `4` | `(192,192)` |

The placement loop still accepts only subcells `2`, `3`, and `4`. A raw program-image read of `DAT_0081CC84` also confirms the five fixed preference rows:

| Quadrant | Raw row | Effective row after skipping `0`/`1` |
|---:|---|---|
| `0` | `[1,2,3,4]` | not used directly; quadrant `0` uses random rotations |
| `1` | `[0,2,3,4]` | `[2,3,4]` |
| `2` | `[0,1,4,3]` | `[4,3]` |
| `3` | `[0,1,4,2]` | `[4,2]` |
| `4` | `[0,2,3,1]` | `[2,3]` |

The raw bytes at `DAT_0081CC98` confirm the four quadrant-`0` random rotations:

| Random result | Raw row | Effective row after skipping `1` |
|---:|---|---|
| `0` | `[1,2,3,4]` | `[2,3,4]` |
| `1` | `[2,3,4,1]` | `[2,3,4]` |
| `2` | `[3,4,1,2]` | `[3,4,2]` |
| `3` | `[4,1,2,3]` | `[4,2,3]` |

## 4. INI Keys

No INI key directly changes the infantry subcell placement algorithm. The relevant INI values only get a carrier and payload into this code path.

| INI key | Default / YR value | Effect on this slice |
|---|---|---|
| `ParadropRadius` | `1024` in `rulesmd.ini` | Controls when the carrier reaches drop mission range; not used by `PlaceInfantryInCell`. |
| `ParaDropPlane` | `PDPLANE` from rules parsing default / general rules | Selects the carrier aircraft type that eventually calls `Drop_Payload`. |
| `AmerParaDropInf` | `E1` in `rulesmd.ini` | Supplies standard American paradrop infantry payload. |
| `AmerParaDropNum` | `8` in `rulesmd.ini` | Supplies payload count. |
| `AllyParaDropInf` | `E1` in `rulesmd.ini` | Supplies standard allied paradrop infantry payload. |
| `AllyParaDropNum` | `6` in `rulesmd.ini` | Supplies payload count. |
| `SovParaDropInf` | `E2` in `rulesmd.ini` | Supplies fallback Soviet paradrop infantry payload. |
| `SovParaDropNum` | `9` in `rulesmd.ini` | Supplies payload count. |
| `YuriParaDropInf` | `INIT` in `rulesmd.ini` | Supplies Yuri paradrop infantry payload. |
| `YuriParaDropNum` | `6` in `rulesmd.ini` | Supplies payload count. |

## 5. Integration Points

| Function | Role |
|---|---|
| `AircraftClass__Mission_ParaDropApproach @ 0x004155F0` | Enters paradrop mission range and changes mission state. Adjacent timing details are outside this report. |
| `AircraftClass__Mission_ParaDropOverfly @ 0x004157C0` | Active overfly mission. It handles fog/target movement and returns `3`; full cadence belongs to timing reports. |
| `AircraftClass__Drop_Payload @ 0x00415C60` | Primary release logic and restoration owner for this slice. |
| `FUN_00473430 @ 0x00473430` | Pops cargo head by advancing cargo head pointer and clearing passenger next pointer. |
| `CargoClass__AddPassenger @ 0x004733A0` | Re-adds failed passenger to cargo head and rebuilds cargo count. |
| `CellClass__PlaceInfantryInCell @ 0x00481180` | Converts raw coordinate into final infantry subcell coordinate or invalid sentinel. |
| `FootClass__Unlimbo @ 0x004D7170` | Foot-level unlimbo wrapper after techno/object unlimbo; updates surrounding cells and object state on success. |
| Passenger vtable `+0xE8` -> `FootClass__Unlimbo @ 0x004D7170` | Live unlimbo path for dropped infantry in this slice. Ghidra currently has duplicate `ObjectClass__Unlimbo` labels: `0x005F4240` is a two-instruction zero-return stub, while a same-named full lower-level implementation also exists. The drop-site evidence anchor is the passenger vtable `+0xE8` call resolving through `FootClass__Unlimbo`; cite the foot-level wrapper for this report. |
| `BuildingClass__CanGarrison @ 0x004525F0` | Only relevant when cell flags take the building/garrison branch. |

Immediate callers of `CellClass__PlaceInfantryInCell` include paradrop, building sell survivors, spawn survivors, chrono warp, infantry movement, teleport locomotion, and walk locomotion. This report only claims the paradrop argument pattern.

## 6. Current Rust Implementation Status

Current Rust has already been changed to model the main verified behavior:

| Surface | Status |
|---|---|
| `src/sim/aircraft/drop_payload.rs:105` `try_drop` | Implements cargo pop, post-decrement parity V-pattern, infantry subcell allocation, occupancy insertion, parachute descent begin, and retry restoration. |
| `src/sim/aircraft/drop_payload.rs:86` `restore_passenger_to_cargo_head` | Mirrors the shared failure restore intent. |
| `src/sim/movement/bump_crush.rs:301` `allocate_sub_cell_with_preference` | Provides quadrant/preference selection from subcell offset and current occupancy. |
| `src/sim/movement/bump_crush.rs:32` `FUNCTIONAL_SUB_CELLS` | Uses only subcells `2`, `3`, `4`, matching the verified loop skip of `0` and `1`. |
| `src/util/lepton.rs:106` `subcell_lepton_offset` | Converts chosen subcell to final lepton offset. |

Remaining Rust risk: the implementation uses the existing Rust `SimRng`. The static preference tables and subcell coordinate offsets now match the binary evidence, but this report does not prove the Rust RNG stream is bit-identical at this call site.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AircraftClass__Drop_Payload` cargo pop/decrement/V-pattern/restore | verified | `0x00415C60`, assembly around `0x00415D92`-`0x00415ECD` | none for this slice |
| Passenger cell-entry gate before placement | verified | `0x00415D7D`-`0x00415D95`, vtable `+0x1AC` call | concrete passenger class method name not resolved; behavior at call site verified |
| `CellClass__PlaceInfantryInCell` paradrop args | verified | `0x00415D9B`-`0x00415DAA` | none |
| Quadrant derivation from input low bytes | verified | `0x00481180` decompilation | none |
| Center/NW quadrant random rotation | verified | `Random__RandomRanged(0,3)` call in `0x00481180`; `read_memory 0x0081CC84 len 64` verifies `DAT_0081CC98` rows | none for static table bytes; RNG stream parity remains separate |
| Direct quadrant fast path | verified | `0x00481180`, branch to `LAB_00481437` | none |
| Loop skips subcells `0` and `1` | verified | `0x00481180`, `if ((uVar11 != 0) && (uVar11 != 1))` | none |
| Full/blocked cell invalid sentinel | verified | `0x00481180`, invalid sentinel writes after four attempts and bit-flag early-outs | exact human names for cell flag bits deferred |
| Unlimbo failure restore | verified | `0x00415DE3`-`0x00415DF0`, fallback at `0x00415EB1` | none |
| Successful drop bookkeeping | verified | `0x00415DF6`-`0x00415EAA` | PARACH visual details are out of scope |
| Rust `try_drop` current behavior | verified by source scan | `src/sim/aircraft/drop_payload.rs:105` | focused Cargo tests were blocked by unrelated jobs in prior implementation pass |
| Raw static preference table bytes | verified | `read_memory 0x0081CC84 len 64` | none |
| Runtime subcell coordinate table writes | verified | initializer writes at `0x0048E480`-`0x0048E4F3` (corrected 2026-05-28: was `0x0048E4ED`; RET is at `0x0048E4F3` per `get_function_by_address 0x0048E480` — GHIDRA_ADDRESS_SHIFT) | none |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is `AircraftClass__Drop_Payload` on the live standard-YR paradrop path? -> Yes, it is called from paradrop overfly/rescue flow and handles standard payload release.` (evidence: `0x00415C60`, caller/callee scan)
- `[RESOLVED] OQ2 - Does gamemd use the raw V-pattern coordinate as final infantry XY? -> No. It passes that coordinate into `CellClass__PlaceInfantryInCell` and uses the returned coordinate.` (evidence: `0x00415DAA`)
- `[RESOLVED] OQ3 - Is payload count decremented before V-pattern parity? -> Yes. `aircraft + 0x2FC` is decremented before the parity test.` (evidence: `0x00415C60`)
- `[RESOLVED] OQ4 - What happens if cargo pop returns no passenger? -> Function returns without placement work.` (evidence: `0x00415C60`)
- `[RESOLVED] OQ5 - Is there a cell-entry gate before subcell placement? -> Yes. Passenger vtable `+0x1AC` is called with the target cell; nonzero skips to restore.` (evidence: `0x00415D7D`-`0x00415D95`)
- `[RESOLVED] OQ6 - What arguments does paradrop pass to `PlaceInfantryInCell`? -> Target cell as `this`, output coord, input V coord, then `0,0,0`.` (evidence: `0x00415D9B`-`0x00415DAA`)
- `[RESOLVED] OQ7 - How is quadrant `0` selected? -> Distance from center below `60`, or both strict comparisons `sub_x > 128` and `sub_y > 128` false.` (evidence: `0x00481180`)
- `[RESOLVED] OQ8 - Does `(0,128)` map to a functional subcell directly? -> No. It maps to quadrant `0`, causing random rotation.` (evidence: `0x00481180`)
- `[RESOLVED] OQ9 - Does the placement loop ever accept subcells `0` or `1`? -> No. The loop explicitly skips entries `0` and `1`.` (evidence: `0x00481180`)
- `[RESOLVED] OQ10 - What happens when no functional subcell is available? -> The invalid coordinate sentinel is returned.` (evidence: `0x00481180`)
- `[RESOLVED] OQ11 - What does `Drop_Payload` do when placement returns the invalid sentinel? -> It re-adds the passenger to cargo, calls passenger vtable `+0x11C`, increments payload count, and returns.` (evidence: `0x00415DB7`-`0x00415ECD`)
- `[RESOLVED] OQ12 - What happens when unlimbo fails? -> Same restore path as placement failure.` (evidence: `0x00415DE3`-`0x00415DF0`, `0x00415EB1`)
- `[RESOLVED] OQ13 - Does successful drop set final cell from the returned coordinate? -> Yes, the code computes packed cell from the final coordinate after unlimbo succeeds.` (evidence: `0x00415E30`-`0x00415E6E`)
- `[RESOLVED] OQ14 - Is bridge/alternate layer height adjustment active for paradrop placement? -> No. Paradrop passes `param_5 == 0`, so the `DAT_0089E7B4` addition path is not taken.` (evidence: `0x00415D9B`-`0x00415DAA`, `0x00481180`)
- `[RESOLVED] OQ15 - Are standard YR INI keys directly changing subcell placement? -> No direct key found; INI keys only select carrier/payload/radius before this slice.` (evidence: `rulesmd.ini` `ParadropRadius`, `AmerParaDropInf/Num`, `AllyParaDropInf/Num`, `SovParaDropInf/Num`, `YuriParaDropInf/Num`)
- `[RESOLVED] OQ16 - Does current Rust now avoid raw V-coordinate final placement? -> Yes. It chooses a subcell through `allocate_sub_cell_with_preference` and converts it via `subcell_lepton_offset`.` (evidence: `src/sim/aircraft/drop_payload.rs:178`, `src/sim/aircraft/drop_payload.rs:196`)
- `[RESOLVED] OQ17 - Does current Rust restore passenger/cargo on full subcell failure? -> Yes, it returns `ImpassableRetry` after restoring cargo head.` (evidence: `src/sim/aircraft/drop_payload.rs:188`)
- `[RESOLVED] OQ18 - Are the Rust static preference tables byte-identical to `DAT_0081CC84` and `DAT_0081CC98`? -> Yes. Raw bytes match `SUBCELL_PREFERENCE` and `SUBCELL_RANDOM_ROTATIONS`.` (evidence: `read_memory 0x0081CC84 len 64`, `src/sim/movement/bump_crush.rs:40`, `src/sim/movement/bump_crush.rs:50`)
- `[DEFERRED] OQ19 - Is the Rust RNG stream bit-identical at this exact `RandomRanged(0,3)` call?` (category: `requires-different-system-context`; reason: requires global RNG implementation/seed/tick-order audit, not just placement decompilation; next-step-if-pursued: trace `Random__RandomRanged` and compare with `SimRng` call ordering)
- `[DEFERRED] OQ20 - What are the exact human names for cell flag bits `0x20` and `0x40`?` (category: `out-of-scope`; reason: the branch behavior is verified, but naming requires broader `CellClass` flag audit; next-step-if-pursued: run a cell flag layout investigation)

The deferred items do not undermine the main placement fix: gamemd does not land infantry at raw V-pattern `(sub_x, sub_y)` coordinates, and it retries on unavailable functional subcells. They only limit claims about exact random-sequence parity and cell-flag naming.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| V-pattern coordinate is only the input to infantry placement, not final XY | `0x00415DAA`, `0x00481180` | fixed in current worktree | `src/sim/aircraft/drop_payload.rs:178`, `src/util/lepton.rs:106` | Final dropped infantry must occupy subcell `2`, `3`, or `4`, never raw `(0,128)` for the traced V case. | Standard PDPLANE over target with empty cells: dropped infantry coordinates equal canonical subcell offsets. | Do not preserve raw V low-byte coordinate as the entity's final subcell offset. |
| Placement failure restores the same passenger and payload parity | `0x00415EB1`-`0x00415ECD` | fixed for full-subcell retry | `src/sim/aircraft/drop_payload.rs:86`, `src/sim/aircraft/drop_payload.rs:188` | If the target cell is walkable but infantry subcells are full, the same passenger remains at cargo head and mission count does not advance. | Fill subcells `2/3/4` in target cell, attempt drop, expect `ImpassableRetry`, cargo head unchanged. | Do not drop the passenger into an invalid coordinate or advance payload count on failed subcell allocation. |
| Quadrant `0` uses `RandomRanged(0,3)` rotation over verified rows | `0x00481180`, `read_memory 0x0081CC98` | table matched; RNG stream unchecked | `src/sim/movement/bump_crush.rs:301` | For center/NW inputs such as `(0,128)`, use random rotation over the verified rows, not deterministic first-free ordering. | Seeded deterministic test where `(0,128)` does not always choose the same subcell when RNG differs. | Do not replace this with `FUNCTIONAL_SUB_CELLS[0]` for empty cells if exact placement scatter matters. |
| Subcells `0` and `1` are not accepted by placement loop | `0x00481180` skip condition | matched | `src/sim/movement/bump_crush.rs:32` | Only subcells `2`, `3`, `4` may represent placed infantry. | Property test: allocation never returns `0` or `1`. | Do not treat center/NW as legal infantry occupancy. |
| Successful unlimbo writes occupancy/last-cell effects from returned coordinate | `0x00415E30`-`0x00415E6E`, `FootClass__Unlimbo @ 0x004D7170` | mostly matched, render/last scratch not fully modeled | `src/sim/aircraft/drop_payload.rs:213`, `src/sim/movement/parachute_descent.rs` | Occupancy should reflect the chosen final subcell before descent/render systems query the unit. | After a successful drop, occupancy grid lists the infantry in the chosen subcell. | Do not leave paradropped infantry absent from occupancy while visually descending if collision/subcell systems depend on it. |

## Stale Docs / Follow-up Docs

- `traces/PARADROP_PAYLOAD_RELEASE_PATTERN_TRACE.md` remains directionally correct for the two FAIL findings. It should be considered superseded by this report for exact ordering details around the cell-entry gate, `PlaceInfantryInCell` arguments, and shared restoration path.
- Static table exactness is now closed: `DAT_0081CC84`, `DAT_0081CC98`, and the runtime-initialized `DAT_0089E9F0/4/8` coordinate table match the current Rust constants.
- A separate RNG audit is still needed before claiming exact drop-subcell sequence parity across a full match.

## Sources

- Ghidra decompilation:
  - `AircraftClass__Drop_Payload @ 0x00415C60`
  - `CellClass__PlaceInfantryInCell @ 0x00481180`
  - `AircraftClass__Mission_ParaDropApproach @ 0x004155F0`
  - `AircraftClass__Mission_ParaDropOverfly @ 0x004157C0`
  - `CargoClass__AddPassenger @ 0x004733A0`
  - `FUN_00473430 @ 0x00473430`
  - `FootClass__Unlimbo @ 0x004D7170`
  - `BuildingClass__CanGarrison @ 0x004525F0`
  - `CellClass__FindOccupierByRTTI @ 0x0047C4D0`
  - `FUN_00487F20 @ 0x00487F20`
- Ghidra caller/callee scans:
  - Callers of `CellClass__PlaceInfantryInCell`
  - Callees of `AircraftClass__Drop_Payload`
  - Callees of `CellClass__PlaceInfantryInCell`
- Ghidra raw memory / assembly verification:
  - `read_memory 0x0081CC84 len 64`
  - initializer writes at `0x0048E480`-`0x0048E4F3` (corrected 2026-07-18: was `0x0048E4ED` here; the Coverage Ledger entry above was already fixed 2026-05-28 but this Sources line was not updated in that pass; RET confirmed at `0x0048E4F3` via `get_function_by_address 0x0048E480` and `read_memory 0x0048E4E0 len 20` (byte 19 = `0xC3`) — GHIDRA_ADDRESS_SHIFT)
- Prior trace:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/traces/PARADROP_PAYLOAD_RELEASE_PATTERN_TRACE.md`
- INI:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/art.ini`
- Rust surfaces scanned:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/aircraft/drop_payload.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/movement/bump_crush.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/util/lepton.rs`
