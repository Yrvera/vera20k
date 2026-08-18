# Drive RawTrack Metadata and Initializer Reconciliation

**Date:** 2026-07-21  
**Investigation mode:** exhaustive slice  
**Checkpoint:** B, RawTrack/TurnTrack half only  
**Verdict:** **PASS for RawTrack research readiness; production activation remains BLOCKED.**

This report closes the disputed RawTrack field names and the cursor values used by
fresh normal tracks, accepted chains, forced tracks, the short-track selector,
and direction-8 tube handling. It does **not** close the separate
`FootClass::GetCurrentSpeed` target, the full TubeClass lifecycle, or the atomic
Phase-1 production flip.

No Rust, INI, asset, plan, or contract file was edited for this investigation.
No Cargo command, staging, commit, or Ghidra mutation was performed.

## 1. Target identity and evidence rules

The only open Ghidra program was `gamemd.exe`, loaded from
`C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe` as PE x86
32-bit, image base `0x00400000`. The retail file SHA-256 was
`1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`.

Evidence calls:

- `list_open_programs()` and
  `get_current_program_info(program="gamemd.exe")` established the target.
- Every Ghidra read below explicitly passed `program="gamemd.exe"`.
- Local names and comments were navigation hints only. Field roles below come
  from bytes, instruction operands, receiver reconstruction, and callers.
- The complete table-reader scan used `search_instructions` for
  `0x7e7a28`, `0x7e7a2c`, `0x7e7a30`, `0x7e7a34`, `0x7e7b28`, and `0x7e7b30`,
  then cold xref exhaustion. No additional RawTrack field reader was found.

## 2. Executive result

The native record is:

```text
RawTrack[16], base 0x007E7A28, stride 0x10 (256 bytes total)

+0x00  points pointer
+0x04  current-track chain-attempt cursor
+0x08  accepted-candidate restart cursor/anchor
+0x0C  cell-handoff / occupation-mark cursor
```

The runtime Drive object fields are:

```text
Drive complete-object base

+0x4C  integer residual movement budget
+0x58  TurnTrack selector
+0x5C  current RawTrack point cursor
+0x60  nonzero selects TurnTrack.short_raw instead of normal_raw
```

The decisive correction is that object `+0x5C` is one point cursor, not a
second `raw_track_lookup` or chain-group field. A normal fresh track starts at
cursor `0`. An accepted chain writes `RawTrack[next].+0x08 - 1`. If the owner
survives the three immediate lifecycle gates, the common loop tail increments
that seed to `RawTrack[next].+0x08` before the budget continue/return decision;
that is the survivor path's first consumable point. If owner `+0x90 == 0`,
`+0x81 != 0`, or `+0x8D != 0`, native returns directly through the epilogue
before the increment, leaves `+0x08-1` stored, and consumes no candidate point.
`Force_Track` starts at cursor `0` and does not use `+0x08` as a general entry
point.

## 3. Receiver-base normalization

This was the source of the stale two-field model.

| Function | Actual incoming receiver | Normalization | Load-bearing proof |
|---|---|---|---|
| `Process @ 0x004B0500` | explicit ILocomotion pointer on stack | `ESI = ILocomotion`; `EDI = ESI - 4` recovers complete Drive object | `0x004B0506`, `0x004B0518`; object base passed in ECX at `0x004B0574..0x004B0576` and `0x004B0AA8..0x004B0AAA` |
| `Process_Drive_Track @ 0x004B0F20` | complete Drive object in ECX | no adjustment | both Process callsites above |
| `Process_Movement @ 0x004B2630` | complete Drive object in ECX | no adjustment | `0x004B0A73..0x004B0A79` |
| `Apply_Track_Delta @ 0x004B0AD0` | complete Drive object in ECX | no adjustment | `0x004B0ADC MOV ESI,ECX`; fresh call `0x004B4703..0x004B4705` |
| `Transform_Track_Coords @ 0x004B4780` | complete Drive object in ECX | no adjustment | direct object `+0x58/+0x40/+0x44` reads |
| `Force_Track @ 0x004B0C40` | explicit ILocomotion pointer as first stack argument | `ESI = EBP - 4` recovers complete object | `0x004B0C4D`, `0x004B0C53..0x004B0C56`, `0x004B0C88`; object base passed to Apply at `0x004B0D38..0x004B0D3A` |
| `Can_Use_Track @ 0x004B4B00` | explicit ILocomotion pointer as first stack argument | reads are ILocomotion-relative | `0x004B4B02`; owner through interface `+0x08` at `0x004B4B06` |

Consequently:

| Meaning | Complete-object offset | ILocomotion-relative offset |
|---|---:|---:|
| TurnTrack selector | `+0x58` | `+0x54` |
| point cursor | `+0x5C` | `+0x58` |
| short-track selector byte | `+0x60` | `+0x5C` |
| destination coordinate | `+0x34..+0x3C` | `+0x30..+0x38` |
| head-to coordinate | `+0x40..+0x48` | `+0x3C..+0x44` |

`Force_Track` demonstrates the trap directly: `0x004B0C53` writes its selector
to ILocomotion `+0x54` (object `+0x58`), and `0x004B0C56` writes zero to
ILocomotion `+0x58` (object point cursor `+0x5C`).

Evidence: `disassemble_bytes(0x004B04F0..0x004B059F)`,
`disassemble_bytes(0x004B0A70..0x004B0ACF)`,
`disassemble_bytes(0x004B0C40..0x004B0D9F)`, and
`disassemble_bytes(0x004B4B00..0x004B4BDF)`.

### 3.1 COL, type-descriptor, and vtable-slot identity

The receiver interpretation is also tied to Drive's RTTI and both relevant
vtables, rather than to local function labels alone:

- the ILocomotion vtable is `0x007E7EB0`; its preceding pointer at
  `0x007E7EAC` selects COL `0x007FFDE8`, whose `+0x0C` points to type descriptor
  `0x00820248` containing `.?AVDriveLocomotionClass@@`;
- ILocomotion slot `+0x70` is `0x004B0C40` (ForceTrack), and slot `+0xA4` is
  `0x004B4B00` (CanUseTrack);
- the complete-object vtable is `0x007E7F7C`; its preceding pointer selects COL
  `0x007FFEB0`, whose `+0x0C` points to the same Drive type descriptor; complete
  slot `+0x24` is `0x004B4CF0`, the `MOV EAX,0x70; RET` object-size stub.

Evidence: `read_memory(0x007E7EAC,176)`,
`read_memory(0x007FFDE8,32)`, `read_memory(0x007E7F78,48)`,
`read_memory(0x007FFEB0,32)`, `read_memory(0x00820248,64)`, and
`disassemble_bytes(0x004B4CF0..0x004B4CF9)`.

## 4. RawTrack bytes and exact roles

`read_memory(address="0x007E7A28", length=256)` returned all 16 records:

| Raw | points pointer | `+0x04` chain-attempt cursor | `+0x08` accepted restart | `+0x0C` cell-handoff cursor |
|---:|---:|---:|---:|---:|
| 0 | null | 0 | 192 | 0 |
| 1 | `0x007E6258` | -1 | 0 | -1 |
| 2 | `0x007E6378` | -1 | 0 | -1 |
| 3 | `0x007E64F8` | 37 | 12 | 22 |
| 4 | `0x007E6790` | 26 | 11 | 19 |
| 5 | `0x007E6968` | 45 | 15 | 31 |
| 6 | `0x007E6C50` | 44 | 16 | 27 |
| 7 | `0x007E6F00` | -1 | 0 | -1 |
| 8 | `0x007E7050` | -1 | 0 | -1 |
| 9 | `0x007E7158` | -1 | 0 | -1 |
| 10 | `0x007E72D0` | -1 | 0 | -1 |
| 11 | `0x007E7420` | -1 | 0 | -1 |
| 12 | `0x007E74C8` | -1 | 0 | -1 |
| 13 | `0x007E7568` | -1 | 0 | -1 |
| 14 | `0x007E78A8` | -1 | 0 | -1 |
| 15 | `0x007E7968` | -1 | 0 | -1 |

### 4.1 `+0x00`: point-array pointer

`Process_Drive_Track` reads the point at the **current** object cursor, not at a
metadata-defined general entry index:

```text
0x004B1596  cursor = [object+0x5C]
0x004B159D  budget -= 7
0x004B15A7  x = points[cursor].x
0x004B15AF  y = points[cursor].y
```

The xref exhaustion found pointer reads only at Apply `0x004B0B35` and Process
`0x004B1546`, `0x004B1CA7`, and `0x004B22E2`.

### 4.2 `+0x04`: current-track chain-attempt cursor

Both readers compare this field from the **currently selected raw track** to the
same object point cursor:

- Process: `0x004B1B39` loads object `+0x5C`, and `0x004B1B3C` compares
  `RawTrack[current].+0x04`; cursor zero is rejected at `0x004B1B48..0x004B1B4A`.
- CanUse: after choosing current normal/short raw, `0x004B4B7A` loads
  ILocomotion `+0x58` (object cursor `+0x5C`), and `0x004B4B80` compares
  `RawTrack[current].+0x04`; zero is rejected at `0x004B4B88..0x004B4B8A`.

This is not a point count, not a candidate key, and not a separate chain group.
It is the current-curve cursor at which a direction-changing follow-on chain may
be considered.

### 4.3 `+0x08`: accepted-candidate restart cursor

This field is read only for a **candidate** raw track:

- Process `0x004B1B93..0x004B1B9B` requires candidate `+0x08 != 0`.
- CanUse `0x004B4BC1..0x004B4BC9` performs the same eligibility check.
- On accepted Process chain, `0x004B1C9D` reloads candidate `+0x08`,
  `0x004B1CA3` decrements it, and `0x004B1CA4` stores it to object `+0x5C`.

Only raw tracks 3-6 have nonzero usable values. Raw 0's value 192 is inert
because candidate normal raw zero is rejected before this field is read.

`+0x08` is **not** the fresh-normal or ForceTrack start cursor. Its proven role
is accepted-chain eligibility and restart anchoring.

### 4.4 `+0x0C`: cell-handoff / occupation-mark cursor

The two readers establish a threshold/equality pair:

- Apply `0x004B0B3B..0x004B0B5C` requires `+0x0C > -1` and
  `current_cursor < +0x0C`, loads the point at `+0x0C`, transforms it, and
  includes that coordinate in its mark/remove work.
- Process `0x004B1AC6..0x004B1B0D` requires current cursor nonzero and equal to
  `+0x0C`, copies the owner's current coordinate, and calls owner vtable
  `+0xF4` on that coordinate.

The exact binary-backed name should therefore retain both concepts, for example
`cell_handoff_cursor` with documentation that it is also Apply's extra
occupation-point threshold. Calling it merely a coordinate-derived cell crossing
loses its explicit table-driven mark/unmark role.

### 4.5 Reader exhaustion

Cold xref/search exhaustion found:

| Field | Complete reader set |
|---|---|
| `+0x00` | Apply `0x004B0B35`; Process `0x004B1546`, `0x004B1CA7`, `0x004B22E2` |
| `+0x04` | Process `0x004B1B3C`; CanUse `0x004B4B80` |
| `+0x08` | Process `0x004B1B93`, `0x004B1C9D`; CanUse `0x004B4BC1` |
| `+0x0C` | Apply `0x004B0B3B`; Process `0x004B1AD1` |

The table is static; none of these sites writes metadata.

## 5. TurnTrack selector and record

`read_memory(address="0x007E7B28", length=96)` sampled the table head and
`read_memory(address="0x007E7E28", length=96)` sampled selectors 64-71. The
full table is 72 records x 12 bytes = 864 bytes.

| Offset | Width | Proven role | Active readers |
|---:|---:|---|---|
| `+0x00` | byte | normal RawTrack selector | Apply, ProcessTrack, ProcessMovement, CanUse |
| `+0x01` | byte | short RawTrack selector | ProcessTrack and CanUse when object `+0x60 != 0` |
| `+0x02..+0x03` | 2 bytes | zero padding in retail table | no semantic reader found |
| `+0x04` | dword, low byte used | post-turn facing | ProcessTrack chain direction and CanUse; facing quantized to 0-7 |
| `+0x08` | dword | control flags | TransformTrackCoords consumes low bits 0-2; ProcessMovement separately tests bit 3 |

The standard selector formula is:

```text
selector = next_path_direction + current_quantized_facing * 8   // 0..63
```

If its normal raw byte is zero, fresh ProcessMovement substitutes
`current_direction * 9`. Accepted chaining uses the next queued direction plus
the **current TurnTrack's target-facing direction** times 8. `Force_Track`
writes a caller-supplied selector directly; selectors 64-71 are reachable only
through such direct/special use, not the standard formula.

Special retail records are:

| selector | normal | short | target facing | flags |
|---:|---:|---:|---:|---:|
| 64 | 11 | 11 | `0xA0` | `0x00` |
| 65 | 12 | 12 | `0xA0` | `0x00` |
| 66 | 13 | 13 | `0xA0` | `0x00` |
| 67 | 14 | 14 | `0x20` | `0x00` |
| 68 | 14 | 14 | `0x60` | `0x04` |
| 69 | 14 | 14 | `0xA0` | `0x01` |
| 70 | 14 | 14 | `0xE0` | `0x02` |
| 71 (`0x47`) | 15 | 15 | `0xC0` | `0x00` |

## 6. Initializer matrix

All offsets in this table use the complete Drive object base.

| Path | selector / short byte | point cursor | residual `+0x4C` | destination / head-to | marking and first consumable point |
|---|---|---:|---|---|---|
| Constructor `0x004AF540` | selector `-1`; short `0` | `-1` | set `0`; target-speed fraction qword at `+0x50..+0x57` is also zero | destination and head-to both NullCoord | no track point |
| Fresh normal ProcessMovement | formula selector, or `cur_dir*9` fallback; short explicitly `0` | explicitly `0` | **not written by ProcessMovement**; existing object residual is carried into the immediate ProcessTrack call | destination remains the final destination; head-to is the accepted next-cell coordinate | Apply mode 1 is called at `0x004B4705` when its gates pass; first consumed point is raw point 0 |
| Fresh sharp fallback | selector `cur_dir*9`; short `0` | `0` | same preserved residual rule | fallback-specific head-to/path behavior; no use of Raw `+0x08` | first point 0; raw 1/2 happen to have `+0x08=0` |
| Accepted chain inside ProcessTrack | candidate selector; short cleared to `0` | writes candidate `+0x08-1`; surviving lifecycle path reaches common-tail `+1` and stores `+0x08`; any of the three immediate owner-state exits returns with `+0x08-1` | survivor keeps the local in-flight budget; early exits bypass both remaining-budget store `0x004B1F64` and clear `0x004B25F2`, so the pre-call object residual remains stored | final destination is not written; the accepted non-null path replaces head-to with its lookahead cell | survivor can perform eligible Apply/path effects and consumes candidate point `+0x08` if budget remains; lifecycle exits consume no candidate point |
| ForceTrack `0x004B0C40` | direct caller selector is written at entry; short byte is **preserved** | `0` at entry | object residual is **preserved** | successful non-null/map-eligible arm sets head-to before Apply and destination after Apply; guard exits can omit destination/speed writes and can clear head-to | successful arm calls Apply mode 1, then writes destination, then target-speed fraction 1.0; first point 0 |
| Nonzero short selector | no independent fresh initializer found; fresh and accepted-chain paths clear it, Force preserves it, Load can restore it | follows the active path's cursor rule | follows active path | follows active path | ProcessTrack/CanUse choose TurnTrack `+0x01`; standard fresh-map writer-to-1 remains unproved |
| Direction-8 tube branch | Drive selector remains `-1`; no RawTrack selected | no RawTrack cursor is initialized | Drive residual is not initialized here | destination is not written; head-to is first cleared and then becomes the valid Tube endpoint (or remains NullCoord on invalid Tube) | owner tube state uses distinct bytes `+0x684` and tube cursor `+0x685=0`; no RawTrack point is consumed |
| Track sentinel/completion | selector cleared to `-1` | cleared to `0` | common completion ultimately clears residual | path/head-to/lifecycle side effects run | not an initializer, but required reset boundary |
| Save/load restoration | restores serialized selector/cursor/short/residual bytes | serialized value | serialized value | serialized value | resumes saved state; it is not a fresh semantic initializer |

### 6.1 Fresh normal proof

ProcessMovement writes:

```text
0x004B4016  selector = path_dir + current_dir*8
0x004B4019  [object+0x60] = 0
0x004B401D  [object+0x58] = selector
0x004B4023  test TurnTrack[selector].normal_raw
0x004B402E..31  if zero: [object+0x58] = current_dir*9
...
0x004B4659  [object+0x5C] = 0
0x004B4703..05  Apply_Track_Delta(head_to, 1)
```

There is no object `+0x4C` write in ProcessMovement's complete instruction scan.
This matters for a same-Process continuation: a generic Rust constructor that
always sets residual to zero is not equivalent to the native initializer.

### 6.2 Accepted-chain proof and the `-1` correction

The exact control flow is:

```text
0x004B1B3C  require RawTrack[current].+0x04 == current cursor
0x004B1B48  require current cursor != 0
0x004B1B93  require RawTrack[candidate].+0x08 != 0
...
0x004B1C80  short byte = 0
0x004B1C84  selector = candidate TurnTrack
0x004B1C9D  cursor = RawTrack[candidate].+0x08
0x004B1CA3  cursor--
0x004B1CA4  store cursor
0x004B1CFD  call owner vtable +0x18C
0x004B1D12  if owner+0x90 == 0: jump 0x004B25F9
0x004B1D20  if owner+0x81 != 0: jump 0x004B25F9
0x004B1D2E  if owner+0x8D != 0: jump 0x004B25F9
...
0x004B1F48  load cursor
0x004B1F4F  cursor++
0x004B1F53  store cursor
0x004B1F56  only now decide whether budget loops to 0x004B158F
0x004B25F9  direct epilogue reached by the three lifecycle exits
0x004B2605  return
```

Thus `+0x08-1` is the survivor path's pre-common-tail seed, but it is also a
real stored exit value. When all three gates pass, the tail stores `+0x08`; if
budget remains, the loop consumes point `+0x08` in the same call, and if budget
ends, that cursor survives for the next call. When any gate fails, control jumps
straight to `0x004B25F9`, bypassing both the tail increment and the residual
write/clear sites; `+0x08-1` remains stored and no candidate point is consumed.
An acceptance test that expects one cursor for both classes is stale: survivor
expects anchor, while each lifecycle-exit class expects anchor minus one.

### 6.3 ForceTrack proof and residual trap

At entry, EBP is the explicit ILocomotion subobject pointer. Selector and cursor
are written before any coordinate guard:

```text
0x004B0C53  [ILocomotion+0x54] = selector  // object+0x58
0x004B0C56  [ILocomotion+0x58] = 0         // object+0x5C cursor
0x004B0C88  ESI = EBP-4                    // complete object
0x004B0D38  ECX = ESI; call Apply
0x004B0D52  [ILocomotion+0x4C] = 0
0x004B0D59  [ILocomotion+0x50] = 0x3FF00000
```

On the successful non-null/map-eligible arm, Force writes head-to at
`0x004B0CFE..0x004B0D07`, conditionally calls Apply mode 1 at
`0x004B0D31..0x004B0D3A`, writes destination at
`0x004B0D3F..0x004B0D4F`, and only then performs the two displayed speed
writes. Those final writes are one little-endian qword double `1.0` at object
`+0x50..+0x57`, not the integer residual at object `+0x4C`; `Force_Track` does
not reset the residual. Its guard exits can return after the entry
selector/cursor writes without writing destination or target speed, and the
owner-state arm at `0x004B0D6A..0x004B0D90` can clear head-to.

An active standard-YR caller exists in the building dock-release path:
`0x00459744` loads the unit locomotor, pushes selector `0x47` and the explicit
ILocomotion pointer, then calls vtable `+0x70` at `0x00459760`. The exact
gameplay conditions of every ForceTrack caller remain outside this bounded
slice; direct selector 71 itself is proven reachable.

### 6.4 Tube direction 8 is not a RawTrack initializer

ProcessTrack handles path direction 8 only while Drive selector is `-1`. It
sets a Tube endpoint as head-to, shifts the path, stores a Tube index at owner
`+0x684`, initializes the distinct owner Tube cursor at `+0x685` to zero, and
keeps the Drive selector `-1`. The invalid-tube arm clears path/head-to and also
keeps selector `-1`.

Evidence:
`decompile_function(address="0x004B0F20")`, especially the direction-8 block,
plus `disassemble_bytes(0x004B1400..0x004B1495)`. Full tube producer and leaf
precedence belongs to Checkpoint C.

### 6.5 Completion and sentinel boundary

Native RawTrack has no point-count field. Point arrays terminate with `(x=0,
y=0)` at nonzero cursor. ProcessTrack checks budget `>7`, subtracts 7, reads the
current point, and only then recognizes the sentinel (`0x004B1510..0x004B15C8`).
The sentinel therefore participates in the exact budget/completion boundary.
The later common completion path clears selector to `-1` and cursor to `0` at
`0x004B210E..0x004B2115` and performs additional reclaim/lifecycle work.

## 7. Short selector, reverse language, and persistence

The byte at object `+0x60` is proven to choose TurnTrack byte `+0x01` instead of
`+0x00`. It is not proven to be a general `reverse` flag.

Direct Drive-class instruction exhaustion found:

- constructor writes `0` at `0x004AF5AC`;
- ProcessMovement writes `0` at `0x004B4019`;
- accepted chain writes `0` at `0x004B1C80`;
- Apply, ProcessTrack, and CanUse read it;
- no direct runtime write of `1` was found in the Drive code region.

Persistence prevents a stronger global deadness claim. Drive's virtual size
stub at `0x004B4CF0` returns `0x70`; the base Save/Load routines called by
`DriveLocomotionClass::Save @ 0x004AF800` and `Load @ 0x004AF780` write/read that
raw object span, which includes `+0x60`, after which Load restores vtable
pointers. Therefore a saved nonzero byte can be restored. Begin/End Piggyback
only exchange the piggybacked locomotor pointer and do not initialize the track
selector/cursor/short trio.

The stock-fresh producer of a nonzero short byte remains **UNCHECKED**. This does
not block the normal/accepted/forced cursor contract because those paths' writes
or preservation rules are explicit. It does block claiming the short path is
globally dead. A distinct Drive copy/clone initializer that can introduce or
overwrite `+0x60` was not established in this bounded slice; that provenance is
explicitly deferred with the fresh-stock writer question rather than inferred
from Save/Load.

## 8. Live standard-YR activity

| Surface | Activity verdict | Evidence |
|---|---|---|
| ordinary ProcessMovement and ProcessTrack | **ACTIVE** | Drive locomotor Process passes complete object to both; retail Drive units use this locomotor |
| chain eligibility in CanUse | **ACTIVE conditional** | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` loads unit locomotor `+0x674`, pushes it, calls locomotor vtable `+0xA4` at `0x0073FA63`, and promotes the result to at least code 2 |
| accepted chain | **ACTIVE conditional** | same metadata checks are in live ProcessTrack; acceptance additionally depends on path direction and cell-entry result |
| Force selector `0x47` | **ACTIVE conditional** | building dock-release caller `0x00459720..0x00459760` |
| selectors 64-70 | **UNCHECKED reachability** | direct-table records exist, but this slice did not enumerate every vtable `+0x70` caller |
| nonzero short byte from fresh stock play | **UNCHECKED** | readers and persistence proven; no direct writer-to-1 found |
| direction-8 Tube branch | **ACTIVE YR mechanism, separate state** | ProcessTrack direction-8 branch; full low-bridge activation contract deferred to Checkpoint C |

## 9. Reconciliation and explicit supersession ledger

| Older source/claim | Current verdict | Correction |
|---|---|---|
| `DRIVE_TRACK_TABLES_DEEP_DECODE.md` header: RawTrack table is 192 bytes | **STALE** | table is 256 bytes, 16 x 16; that document's later line 238 already gives the correct size |
| same doc: `use_short_track` is confirmed dead in YR | **UNPROVEN / too strong** | constructor/fresh/accepted writes zero and no direct Drive write-to-1 was found, but readers and raw-object Save/Load persistence are active; fresh-stock and copy/clone provenance remain deferred |
| same doc `+0x08 = starting walk position` | **STALE as a general rule** | `+0x08` is accepted-candidate eligibility/restart; fresh and forced cursor are zero |
| same doc sections 13.4-13.8: `+0x04` is a chain-group key and object `+0x5C` is separate `raw_track_lookup` | **STALE receiver-base error** | `+0x04` is compared to the current point cursor at object `+0x5C`; there is no second chain-group field |
| same doc line 417: ProcessMovement `[EBP+0x60]` is a stack local | **STALE** | EBP is the complete object receiver; it clears the short selector byte |
| `PROCESS_DRIVE_TRACK_DECOMPILATION.md`: gate on `+0x0C`, write `+0x04-1` | **STALE** | chain gate uses current `+0x04`; accepted seed uses candidate `+0x08-1` |
| `DRIVE_APPLY_TRACK_DELTA_POINT_RESIDUAL_GHIDRA_REPORT.md`: accepted chain begins persistently at `entry_index-1` | **CONDITIONALLY CORRECT, INCOMPLETE** | three post-install lifecycle exits preserve `+0x08-1` and consume no candidate point; the lifecycle-surviving path reaches common `+1`, stores `+0x08`, and can consume that point |
| `DRIVE_SHARP_TURN_FALLBACK_RE.md`: candidate `+0x08` called `chain_index` | **STALE name, useful bytes** | candidate `+0x08` is restart eligibility/anchor; that report's fresh cursor-zero finding remains correct |
| `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`: Force writes/resets current speed/residual and has separate raw lookup | **STALE offset frame/layout** | normalize ILocomotion by +4; Force writes target speed fraction double 1.0 at object `+0x50`, preserves residual `+0x4C`, and writes the point cursor to zero |
| `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`: Raw `+0x04` count language | **STALE** | no native count exists; `+0x04` is the chain-attempt cursor |

The current `DRIVE_APPLY_TRACK_DELTA_POINT_RESIDUAL_GHIDRA_REPORT.md` remains
authoritative for Apply's mark/remove modes and object residual ownership after
the accepted-chain survivor-versus-lifecycle-exit qualification in this report.

## 10. Current Rust disparity scan

### 10.1 `src/sim/movement/drive_track.rs`

Current Rust stores the native metadata values but gives two of them overbroad
names:

- `RawTrack.entry_index` contains native `+0x08`, but the implementation uses it
  for **every** new track.
- `RawTrack.chain_index` contains native `+0x04`; its trigger comparison is
  mechanically close, but it is deferred out of the native same-loop chain.
- `RawTrack.cell_cross_index` contains native `+0x0C`, but active crossing is
  coordinate-derived and this field is not consumed for the exact mark/unmark
  threshold/equality behavior.
- `points_start` and `points_count` are Rust-synthetic extraction metadata; no
  such count exists in native RawTrack.

Load-bearing drift:

1. `begin_drive_track_with_head_offset` initializes
   `point_index: meta.entry_index` (`drive_track.rs:3649-3658`). Native fresh and
   ForceTrack initialize zero.
2. `advance_drive_track_with_budget` increments `point_index` before reading a
   point (`drive_track.rs:3746-3754`). Native subtracts cost and reads the current
   cursor, then increments at the common loop tail.
3. Combining the first two errors means raw 3 fresh starts at 12 and consumes
   13 first, while native starts at 0 and consumes 0 first.
4. Rust excludes native sentinels and declares finished from synthetic
   `points_count-1`; it does not reproduce the sentinel's seven-budget and
   reclaim boundary.
5. `begin_forced_turn_track` reuses the same `+0x08` initializer. Selector
   `0x47` happens to map raw 15 whose `+0x08` is zero, masking the generic bug in
   that one fixture.
6. `transform_flags: flags & 0x07` preserves transforms but discards bit 3 from
   runtime state. Selection retains bit 3, but current movement code has no
   corresponding native ProcessMovement bit-3 consumer.

Tests at `src/sim/movement/drive_track_tests.rs:158-162` explicitly ratchet the
stale raw-3 fresh start at 12 and must not be treated as native evidence.

### 10.2 `src/sim/movement/movement_tick.rs`

`handle_deferred_drive_track_chain` selects a candidate and calls ordinary
`begin_drive_track` (`movement_tick.rs:809-827`). Because ordinary begin uses
`+0x08` and stepping pre-increments, the effective first candidate point is
`+0x08+1`, not native `+0x08`.

The current chain also:

- breaks the point loop at `+0x04` and defers classification/install until the
  entity borrow is released;
- ends the current tick's point processing regardless of whether the chain
  succeeds (`movement_tick.rs:1488-1550`);
- performs scatter/crush and install through a later helper, instead of the
  native accepted-chain branch and common budget loop;
- lacks ApplyTrackDelta's table-driven `+0x0C` mark/remove ownership.

Native may install and consume the candidate restart point in the same
ProcessTrack call when residual budget remains, so this is observable ordering
and RNG/lifecycle drift, not an internal-only representation difference. Native
can instead return at any of the three post-install lifecycle gates with
candidate anchor minus one stored and no candidate point consumed; the deferred
Rust chain does not model that boundary either.

### 10.3 `src/sim/components.rs` and forced miner path

`DriveLocomotionRuntime` defaults `track_index=-1` but `point_index=0`; native
constructor initializes cursor `-1` and later completion initializes it to 0.
Rust also has `is_reversed`, but this slice found no evidence that object `+0x60`
is a general reverse flag.

The miner helper calls `begin_forced_turn_track(0x47, ...)` from
`src/sim/miner/miner_dock_sequence.rs:594-610`. Its cursor happens to agree only
because raw 15 has `+0x08=0`; ForceTrack's residual preservation, destination,
head-to, Apply ordering, and native caller precedence remain broader than that
helper.

## 11. Implementation handoff

This is a behavior contract, not authorization for the production flip.

| Verified requirement | Current Rust delta | Required implementation effect | Minimum acceptance fixture | Risk / do not do |
|---|---|---|---|---|
| Fresh normal and sharp fallback set cursor 0 and consume point 0 first | common begin uses `+0x08`; stepper pre-increments | split initializer mode from accepted restart; read current cursor before increment | raw 3 fresh: stored start 0, first consumed point 0; fallback raw 1/2 also point 0 | do not rename `+0x08` and keep using it generically |
| Accepted chain gates at current raw `+0x04`, candidate requires `+0x08!=0`; store `+0x08-1`, then test owner `+0x90/+0x81/+0x8D`; survivor reaches common `+1`, while each failed gate returns first | deferred chain uses common begin, pre-increments from `+0x08`, ends the tick, and lacks the three exact post-install exits | install inside a budget-preserving state machine with the three lifecycle checks before common increment | raw3 cursor37 -> raw4 anchor11: survivor stores/consumes 11 if budget remains; separate fixtures for `+0x90=0`, `+0x81!=0`, `+0x8D!=0` store 10 and consume no candidate point | do not force all paths to anchor or all paths to anchor-1; never consume anchor+1 |
| ForceTrack sets cursor 0, preserves residual and short byte, sets target speed fraction 1.0 | common begin resets residual and uses metadata anchor | create a Force-specific initializer and side-effect order | force a raw 3 selector with nonzero `+0x08`: start/consume point 0 while residual survives | selector 0x47 alone is insufficient because raw 15 hides the cursor bug |
| `+0x0C` drives Apply's extra point threshold and Process's exact-cursor occupation removal | stored but unused; crossing coordinate-derived | model the table-driven mark/remove event separately from geometric coordinate crossing | raw 3: before cursor22 Apply includes transformed point22; at cursor22 Process runs the handoff remove; after 22 Apply omits it | do not replace explicit native event with coordinate coincidence without exhaustive proof |
| Native point arrays end on a paid `(0,0)` sentinel and then reset selector/cursor | Rust slices omit sentinel and finish at synthetic last active point | encode sentinel/completion budget and reclaim boundary, even if storage remains sentinel-free | budget exactly enough to reach last active point but not sentinel must not complete early; next 7-budget step reaches native completion/reset | do not treat Rust-vs-Rust tests as native oracle |
| TurnTrack bit 3 is separate from transform bits 0-2 | runtime masks to `0x07`; bit3 has no active native-equivalent consumer | retain full flags where ProcessMovement admission needs bit3; pass only low 3 to coordinate transform | one selector with flags 8 and one with flags 0 exercise different admission while transforming identically | do not feed bit3 into transform or silently discard it before admission |
| Tube direction 8 uses owner tube cursor, not RawTrack | tube and Drive paths are structurally separate but production precedence is incomplete | keep tube state separate and establish Checkpoint C owner/precedence before activation | direction8 + selector-1 initializes owner tube cursor0 and consumes no Raw point | do not map Tube cursor to Drive point cursor |
| Save/load restores the 0x70-byte Drive state including selector/cursor/short | Rust serialization uses higher-level structs/defaults | preserve active selector/cursor/short/residual state exactly across save/load if storage migrates | save immediately before a chain threshold and resume with identical next point/mark/order | executable save oracle remains a later blocker |

### Negative facts / do not implement

- Do not add a second per-locomotor `raw_track_lookup` or `chain_group` field.
- Do not use Raw `+0x04` as a point count, candidate key, or restart cursor.
- Do not use Raw `+0x08` as the fresh/forced start cursor.
- Do not normalize accepted `+0x08-1` before the three post-install lifecycle
  gates. Each failed gate returns with anchor minus one stored and no candidate
  point consumed; only the lifecycle-surviving path reaches common `+1`.
- Do not reset object residual from Force's ILocomotion `+0x4C` write; that write
  is the low half of target speed fraction double 1.0 at object `+0x50`.
- Do not call object `+0x60` a proven reverse flag.
- Do not fold Tube owner `+0x685` into Drive object `+0x5C`.
- Do not certify parity with current Rust tests or replay hashes; no executable
  gamemd oracle was run in this slice.

## 12. Coverage ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| RawTrack bytes, all 16 records | verified | `read_memory(0x007E7A28,256)` | none |
| TurnTrack layout and special rows 64-71 | verified | `read_memory(0x007E7B28,96)`; `read_memory(0x007E7E28,96)`; active readers | none |
| all RawTrack field readers | verified | six-address `search_instructions` inventory and cold zero-add rerun | none |
| complete-object versus ILocomotion receiver frames | verified | `0x004B0500`, `0x004B0C40`, `0x004B4B00`; RTTI/COL/vtable bytes in section 3.1 | none |
| constructor initializer | verified | `0x004AF540..0x004AF5CF` | none |
| fresh normal and sharp-fallback initializer | verified | `0x004B3FA0..0x004B405F`; `0x004B4652..0x004B4755` | none |
| accepted-chain install, lifecycle exits, and survivor tail | verified | `0x004B1B39..0x004B1CA7`; `0x004B1D03..0x004B1D2E`; `0x004B1F48..0x004B1F67`; `0x004B25F9..0x004B2605` | none |
| ForceTrack initializer and selector-71 caller | verified | `0x004B0C40..0x004B0D9F`; `0x00459720..0x00459760` | none for the initializer and proven caller |
| Force selectors 64-70 caller population | deferred | outside the bounded initializer slice | enumerate all active vtable `+0x70` callers if Checkpoint C needs those selectors |
| ApplyTrackDelta Raw `+0x0C` use | verified | `0x004B0B3B..0x004B0B5C`; existing Apply report | none |
| CanUseTrack active caller | verified | `0x004B4B00..0x004B4BD6`; `0x0073FA30..0x0073FA7C` | none |
| short-selector direct readers/writers and Save/Load restore | verified | `0x004AF5AC`, `0x004B4019`, `0x004B1C80`; Drive reader inventory; Save/Load and `0x70` size stub | none for the bounded direct-code and persistence claims |
| fresh-stock or copy/clone producer of short value 1 | deferred | no producer established by the complete Drive-region scan | trace object creation/copy plus save/runtime state in the owning system context |
| Tube direction-8 non-alias with RawTrack | verified | `0x004B0F20`; `0x004B1400..0x004B1495` | none for non-alias; full Tube lifecycle belongs to Checkpoint C |
| current Rust disparity | verified | direct reads of `drive_track.rs`, `movement_tick.rs`, `components.rs`, miner helper, and tests | none for this source snapshot |
| INI/assets | not-touched | none | revisit only if an INI-backed override for these binary tables is discovered |
| executable retail trace | not-touched | none | Checkpoint E must provide the parity oracle |

## 13. Adversarial questions

1. **What if candidate Raw `+0x08` is zero?** Both CanUse and ProcessTrack
   reject the candidate. Fresh normal and ForceTrack can still use that raw
   because they do not use `+0x08` as admission.
2. **Why write `+0x08-1` if the survivor's first point is `+0x08`?** It
   compensates the common loop-tail increment on the lifecycle-surviving path.
   The three immediate lifecycle exits bypass that tail, retain `+0x08-1`, and
   consume no candidate point.
3. **Does Raw 0's `+0x08=192` make it chainable?** No. Candidate normal raw zero
   is rejected before the `+0x08` test; its pointer is null.
4. **Does ForceTrack reset residual because it writes zero near the speed
   fields?** No. After receiver normalization, those two dwords form target
   speed fraction double 1.0 at object `+0x50`; residual is object `+0x4C`.
5. **Can direction-8 tube movement reuse RawTrack cursor zero?** No. It keeps
   Drive selector `-1` and initializes a distinct owner Tube cursor at `+0x685`.
6. **Can the short path be declared dead because no direct write-to-1 was
   found?** No. Save/load restores the raw object span, so global deadness is
   unproved even though normal fresh and accepted-chain explicitly clear it.

## 14. Open Questions — Final State of the Investigation Log

- [RESOLVED] OQ-01 — Is the active binary the retail target? → Yes; it is the single open PE x86 `gamemd.exe` target with the recorded SHA-256. (evidence: `list_open_programs; get_current_program_info; retail file SHA-256`)
- [RESOLVED] OQ-02 — Which functions use complete-object versus ILocomotion receiver frames? → Process, ForceTrack, and CanUseTrack enter through the explicit ILocomotion frame where documented; ApplyTrackDelta, ProcessTrack, ProcessMovement, and Transform use the complete object. (evidence: `0x004B0500; 0x004B0C40; 0x004B4B00; section 3.1 COL/vtable bytes`)
- [RESOLVED] OQ-03 — What is the RawTrack table size and stride? → Sixteen records of 16 bytes, 256 bytes total. (evidence: `read_memory(0x007E7A28,256)`)
- [RESOLVED] OQ-04 — What is Raw `+0x00`? → The point-array pointer. (evidence: `0x004B0B35; 0x004B1546; 0x004B1CA7; 0x004B22E2`)
- [RESOLVED] OQ-05 — What is Raw `+0x04`? → The current-track chain-attempt cursor. (evidence: `0x004B1B3C; 0x004B4B80`)
- [RESOLVED] OQ-06 — What is Raw `+0x08`? → Candidate-chain eligibility and the accepted restart cursor/anchor. (evidence: `0x004B1B93; 0x004B1C9D; 0x004B4BC1`)
- [RESOLVED] OQ-07 — What is Raw `+0x0C`? → The Apply occupation threshold and Process exact-cursor cell-handoff event. (evidence: `0x004B0B3B..0x004B0B5C; 0x004B1AC6..0x004B1B0D`)
- [RESOLVED] OQ-08 — What are all TurnTrack fields? → Normal raw byte, short raw byte, target-facing dword whose low byte is used, and flags dword. (evidence: `read_memory(0x007E7B28,96); read_memory(0x007E7E28,96); 0x004B0B22; 0x004B4791`)
- [RESOLVED] OQ-09 — What state does the constructor establish? → Selector `-1`, cursor `-1`, short byte zero, residual zero, speed qword zero, and null destination/head-to. (evidence: `0x004AF540..0x004AF5CF`)
- [RESOLVED] OQ-10 — What cursor starts a normal fresh track? → Zero. (evidence: `0x004B4659`)
- [RESOLVED] OQ-11 — What cursor starts the sharp fallback? → Zero; raw 1/2 also happen to have anchor zero. (evidence: `0x004B4023..0x004B4031; 0x004B4659; RawTrack bytes`)
- [RESOLVED] OQ-12 — What values install an accepted chain? → Current Raw `+0x04` gates the attempt, candidate `+0x08` gates eligibility, and Process stores candidate `+0x08-1`; the survivor increments to `+0x08`, while each immediate lifecycle exit retains the seed. (evidence: `0x004B1B39..0x004B1B9B; 0x004B1C9D..0x004B1CA4; 0x004B1D03..0x004B1D2E; 0x004B1F48..0x004B1F56`)
- [RESOLVED] OQ-13 — What is the accepted chain's first consumable point? → Candidate `+0x08` on the lifecycle-surviving path; each of the three immediate lifecycle exits returns with `+0x08-1` and consumes no candidate point. (evidence: `0x004B1D12; 0x004B1D20; 0x004B1D2E; 0x004B1F48..0x004B1F56; 0x004B25F9..0x004B2605`)
- [RESOLVED] OQ-14 — What cursor starts ForceTrack? → Zero, independently of Raw `+0x08`. (evidence: `0x004B0C53..0x004B0C56`)
- [RESOLVED] OQ-15 — Does ForceTrack clear residual? → No; after receiver normalization, its two apparent dword writes set the target-speed double to 1.0 at object `+0x50..+0x57`. (evidence: `0x004B0C4D; 0x004B0C88; 0x004B0D52..0x004B0D59`)
- [DEFERRED] OQ-16 — What fresh-stock or copy/clone path writes short selector 1? (category: `requires-different-system-context`; reason: direct Drive writers are exhausted and Save/Load restoration is resolved, but no independent producer was established; next-step-if-pursued: trace Drive object creation/copy plus saved/runtime `+0x60` state in the owning system context.)
- [RESOLVED] OQ-17 — Is object `+0x60` a general reverse flag? → That semantic is not established; the proven role is normal-versus-short RawTrack selection. (evidence: `0x004AF5AC; 0x004B0B04; 0x004B151F; 0x004B4019; 0x004B4B24`)
- [RESOLVED] OQ-18 — Is tube direction 8 a RawTrack initializer? → No; it keeps Drive selector `-1` and initializes separate owner Tube bytes `+0x684/+0x685`. (evidence: `0x004B0F20 direction-8 decompile; 0x004B1400..0x004B1495`)
- [RESOLVED] OQ-19 — Does ProcessMovement initialize residual? → No; it has no object `+0x4C` write, so the existing residual reaches the immediate ProcessTrack call. (evidence: `complete 0x004B2630..0x004B4755 instruction scan; Process caller 0x004B0A73..0x004B0AAA`)
- [RESOLVED] OQ-20 — How does native detect track end? → A paid nonzero-cursor `(0,0)` sentinel precedes completion/reclaim and selector/cursor reset. (evidence: `0x004B1510..0x004B15C8; 0x004B210E..0x004B2115; 0x004B25F2`)
- [RESOLVED] OQ-21 — Is CanUseTrack live in standard YR? → Yes, conditionally through UnitClass CanEnterCell's locomotor vtable `+0xA4` call. (evidence: `0x0073FA30..0x0073FA7C; ILocomotion slot +0xA4 = 0x004B4B00`)
- [RESOLVED] OQ-22 — Are selector, cursor, and short state persisted? → Yes, inside the 0x70-byte Drive Save/Load span. (evidence: `0x004AF780; 0x004AF800; 0x0055AA60; 0x0055AAC0; 0x004B4CF0`)
- [DEFERRED] OQ-23 — Are direct Force selectors 64-70 stock-reachable? (category: `out-of-scope`; reason: selector 71 has a proven caller and the remaining caller census is not needed for initializer semantics; next-step-if-pursued: enumerate active vtable `+0x70` callers during the Checkpoint C forced-track precedence audit.)
- [DEFERRED] OQ-24 — What is the full low-bridge Tube producer/consumer lifecycle? (category: `out-of-scope`; reason: Checkpoint C owns active Tube population and precedence; next-step-if-pursued: trace Tube production, traversal, completion, and competing locomotor branches across the Phase-1 population.)

Deferred count: 3 of 24 (12.5%). No load-bearing RawTrack initializer value is
left guessed.

## 15. Zero-add pass and cold spot checks

After the question log reached closure, one zero-add pass re-ran the Raw field
instruction/xref inventory and added no reader or initializer class.

Two cold checks were then repeated from assembly rather than prior prose:

1. **Accepted chain:** `0x004B1C9D..0x004B1CA4`, owner gates
   `0x004B1D03..0x004B1D2E`, common tail `0x004B1F48..0x004B1F56`, and direct
   epilogue `0x004B25F9..0x004B2605` were re-read together. Corrected verdict:
   survivor reaches common `+1`; each lifecycle exit preserves `+0x08-1` and
   consumes no candidate point.
2. **Receiver trap:** Force entry `0x004B0C40..0x004B0C88` and CanUse entry
   `0x004B4B00..0x004B4B2C` were re-read; verdict unchanged: both enter through
   the ILocomotion subobject, while Apply/ProcessTrack/ProcessMovement use the
   complete object.

## 16. Source ledger

### Live binary evidence

- Raw table bytes: `read_memory(0x007E7A28, 256)`.
- TurnTrack head/special bytes: `read_memory(0x007E7B28, 96)` and
  `read_memory(0x007E7E28, 96)`.
- Constructor: `0x004AF540..0x004AF5CF`.
- Process receiver/callers: `0x004B0500..0x004B0ACB`.
- Apply: `0x004B0AD0..0x004B0C3B`.
- Force: `0x004B0C40..0x004B0D9F`.
- ProcessTrack point read, metadata, chain, common increment, sentinel, reset:
  `0x004B1510..0x004B15C8`, `0x004B1AC6..0x004B1B9B`,
  `0x004B1C78..0x004B1D34`, `0x004B1F48..0x004B1F67`,
  `0x004B210B..0x004B211C`, `0x004B25F2..0x004B2605`.
- Drive COL/vtable/type identity: raw reads at `0x007E7EAC`, `0x007FFDE8`,
  `0x007E7F78`, `0x007FFEB0`, and `0x00820248`; size stub
  `0x004B4CF0..0x004B4CF5`.
- ProcessMovement selector/finalizer: `0x004B3FA0..0x004B405F` and
  `0x004B4652..0x004B4755`.
- Transform: `0x004B4780` decompile/disassembly.
- CanUse: `0x004B4B00..0x004B4BD6`.
- Unit CanEnterCell caller: `0x0073FA30..0x0073FA7C`.
- Building Force selector-71 caller: `0x00459720..0x00459760`.
- Save/Load and object size: `0x004AF780`, `0x004AF800`, base helpers
  `0x0055AAC0`/`0x0055AA60`, and `0x004B4CF0 MOV EAX,0x70; RET`.

### Existing research reconciled

- `docs/research/DRIVE_TRACK_TABLES_DEEP_DECODE.md`
- `docs/research/DRIVE_APPLY_TRACK_DELTA_POINT_RESIDUAL_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/DRIVE_SHARP_TURN_FALLBACK_RE.md`
- `docs/research/PROCESS_DRIVE_TRACK_DECOMPILATION.md`
- `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- `docs/research/CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md`
- `docs/research/traces/DRIVE_TRACK_CHAIN_LOOKAHEAD_BLOCKER_TRACE_20260527.md`
- `docs/research/traces/DRIVE_TRACK_LOOKAHEAD_RUNTIME_TUPLE_TRACE.md`

### Current Rust read-only scan

- `src/sim/movement/drive_track.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/movement_commands.rs`
- `src/sim/components.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/movement/drive_track_tests.rs`

## 17. Final checkpoint statement

Checkpoint B's RawTrack subtarget is research-ready: every native Raw metadata
field, relevant TurnTrack byte/dword, receiver frame, active normal/chain/forced
initializer, tube non-alias, persistence path, and first-consumable cursor is
sourced. The short-selector fresh-stock producer, selectors 64-70 caller
population, and full tube lifecycle are explicit follow-ups rather than silent
assumptions.

This does **not** authorize a vehicle-only live flip. The full production move
still requires the separate exact speed result, complete Phase-1 ground
population and precedence, lifecycle/effect ownership, and an executable retail
oracle.
