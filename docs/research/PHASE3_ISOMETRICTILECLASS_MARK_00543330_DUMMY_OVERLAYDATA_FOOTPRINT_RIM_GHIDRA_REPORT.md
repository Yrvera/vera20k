# Phase 3 IsometricTileClass Mark, dummy, OverlayData, footprint and LAT contract

**Primary address:** `IsometricTileClass::Mark @ 0x00543330`
**Supporting addresses:** `0x00543780`, `0x00543A10`, `0x00543B10`,
`0x00549AA0`, `0x00549AE0`, `0x00581140`, `0x00568300`, `0x00565C10`,
`0x0056BAC0`, `0x005A6C10`, `0x005FD2E0`

**Investigation mode:** exact-mechanism coverage map

**Binary:** active retail Yuri's Revenge `gamemd.exe`

**Confidence:** HIGH for the function body, ABI, vtable owner, active cliff-collapse
calls, map-allocation exclusion, ordinary-load/RMG separation, and retail data
gates.

**Phase-row verdict:** the shared-dummy hypothesis belongs to GSI-04.01 and is
closed as an evidence-backed valid-map exclusion. The active tile replacement
mechanism crosses GSI-04.02, GSI-04.03, GSI-04.06, and GSI-04.07; it is not a
GSI-04.01 dummy mutation.

## Verdict

`0x00543330` is `IsometricTileClass::Mark`, not the stale
`BuildingTypeClass::SetOwnerAndOccupy` name found in older reports. The owner is
proved by the `IsometricTileClass` RTTI descriptor and vtable slot `+0x124`.

The investigation corrects four scope-defining assumptions:

1. `CellClass+0x11E` is one byte of **OverlayData**, not an isometric-tile
   active flag and not a short.
2. `Mark` has no separate footprint rim. It walks the TMP rectangle row-major,
   skips null TMP entries as true holes, and only the mode-1/3 writer tail runs
   LAT on the stamped cell plus cardinal directions `0,2,4,6`.
3. A successfully resized valid map cannot route a primary footprint cell to
   the shared dummy. The admission predicate and allocation predicate are
   identical.
4. `Mark(0)` and `Mark(1)` are reached by destroyable-cliff collapse. Ordinary
   IsoMapPack5 load and stock RMG stamping are separate direct-CellClass writers.

No GSI-04.01 Rust state should be added for tile/subtile/overlay fields solely
because `Mark` calls `Get_CellClass`: those mutations cannot reach the shared
dummy in a valid retail map. The actual missing Rust candidate is the
destroyable-cliff collapse transaction, which must be investigated and closed
under its full cross-row ownership before implementation.

## 1. Identity, ABI and object/type inputs

### 1.1 Owner and virtual binding

- Complete-object-locator type descriptor `0x008288C0` is
  `.?AVIsometricTileClass@@`.
- `IsometricTileClass` vtable base is `0x007EC258`.
- Vtable slot `+0x124`, stored at `0x007EC37C`, is `0x00543330`.
- Recovered ABI is:

```text
bool __thiscall IsometricTileClass::Mark(IsometricTileClass* this, int mode)
```

The function returns with `RET 4` at `0x0054373D..0x00543745`.

### 1.2 Inputs

| Input | Native source | Meaning |
|---|---|---|
| tile type | `this+0xAC` | `IsometricTileTypeClass*` |
| resolved first tile id | `type+0x294` | Cell `+0x38` identity |
| width | signed `type+0x2E4` | TMP footprint width |
| height | signed `type+0x2E8` | TMP footprint height |
| cached TMP | `type+0xA4` | lazy-loaded when required |
| lazy-load flag | `type+0x2F4` | controls TMP load path |
| TMP entries | `tmp+0x10 + index*4` | null means absent subtile/hole |
| height delta | `entry+0x28` byte | wrapping Cell level delta |
| slope byte | `entry+0x2A` | source consumed by slope reader |
| object anchor | virtual `+0x1B8` | signed packed cell from world X/Y |

At `0x0054333D`, virtual `+0x6C` resolves through `0x005F3E30` to the
IsometricTile getter and then type virtual `+0x9C @ 0x00544CB0`. That getter
lazy-loads through `TMP_Loader @ 0x00547020` and returns the TMP pointer or
null. A null TMP makes `Mark` return false before base marking.

## 2. Complete branch ledger

### 2.1 Entry and base Mark

1. Get TMP at `0x0054333D`; null returns false at
   `0x00543340..0x00543346` / `0x00543740`.
2. Call `ObjectClass::Mark @ 0x005F5850(mode)` at `0x00543353`; false returns
   false at `0x00543358..0x0054335A`.
3. The base object gates modes: 1/3 require off-map state and set on-map;
   mode 0 requires on-map and clears it; mode 2 retains its redraw/on-map gate.

### 2.2 Footprint enumeration and holes

- Outer `y` loop starts at zero and compares signed `type+0x2E8`.
- Inner `x` loop starts at zero and compares signed `type+0x2E4`.
- Nonpositive width or height performs no per-cell work.
- Each iteration recomputes the object anchor through virtual `+0x1B8`, then
  adds `x` and `y` as 16-bit words. Addition therefore wraps at 16 bits.
- `Cell_in_bounds_check @ 0x00568300` runs before `Get_CellClass`.
- Out-of-diamond candidates skip every cell effect.
- The row-major index is `width*y+x`.
- A null TMP entry skips every write, LAT call, and Recalc. It is a sparse hole,
  not a rim cell.

### 2.3 Mode 0: remove the exact placed tile

For every in-bounds nonnull TMP entry:

```text
if cell.TileIndex == type.FirstTileId && zero_extend(cell.SubTile) == full_index:
    cell.TileIndex = 0xFFFF
    cell.SubTile = 0
    cell.Level = byte(cell.Level - tmp_entry.HeightDelta)
CellClass::RecalcAttributes(cell, -1)
```

The identity test and writes are at `0x0054340D..0x00543457`. A mismatch writes
nothing but still reaches Recalc. Mode 0 does **not** clear `+0x44`, `+0x11E`,
or `+0x11C`, and it does not run LAT.

### 2.4 Modes 1 and 3: stamp, clear overlay state, LAT, Recalc

Only modes 1 and 3 enter the writer body at `0x00543462..0x00543470`.
Other base-accepted nonzero modes write nothing but still Recalc every admitted
nonnull TMP entry.

Writer order is exact:

1. If the type is the clear-tile global `0x00AA10B0`, write
   `cell+0x38=0xFFFF`, `cell+0x11A=0`. Otherwise write
   `cell+0x38=type+0x294`, `cell+0x11A=low_byte(index)`, and write
   `cell+0x11C=TMP_ReadSlopeType(index)`.
2. Canonicalize four hard-coded ramp/slope families at
   `0x005434A7..0x00543569`:
   - `RampEndBlock+5` and `SlopeSetPieces2+5`, subtiles `0/3/6/9`, map to the
     corresponding ramp base `+1`, subtile 0.
   - Their `+8` cases map subtiles `0..3` to the corresponding ramp base,
     subtile 0.
3. Write `cell+0x48=-1` and `cell+0x11F=0`.
4. Read the old overlay identity at `cell+0x44`. For signed overlay ids
   `0x1B..0x25`, clear flag `0x20000` and remove/uninit the first matching
   overlay animation whose type relation and exact cell coordinate match.
5. Unconditionally write `cell+0x44=-1` at `0x0054366A`.
6. Unconditionally write **one byte** `cell+0x11E=0` at `0x00543671`.
7. Add `entry+0x28` to `cell+0x11B` with byte wrapping.
8. Call `CellClass::ApplyLAT_and_SlopeFixup @ 0x0047CA80` on the stamped cell,
   then on neighbors returned for directions `0,2,4,6`, in that order.
9. Call `CellClass::RecalcAttributes(cell,-1)`.

After all rows, modes 1/3 write object `+0x74=0` and invoke object virtual
`+0x20(1)` at `0x00543715..0x00543733`, resolving to the scalar deleting
destructor. The function returns true.

### 2.5 Tiny but parity-relevant cases

- Mode-1/3 stores only the low byte of the row-major index. An index greater
  than 255 is truncated, while later mode 0 compares the zero-extended byte to
  the full index and therefore cannot remove that entry through the equality
  branch.
- Width and dimensions are reread while looping.
- Overlapping stamps are order-sensitive. Level arithmetic wraps modulo 256,
  and later cells observe earlier writes.
- Null `this` is unsupported. The mode-1/3 tail writes object fields.
- OOM during Resize is not a supported dummy fallback; it proceeds into a null
  construction/dereference failure.

## 3. `+0x11E` is OverlayData

The type and meaning are independently established:

- `Mark` uses `MOV byte ptr [cell+0x11E],0` immediately after clearing the
  overlay id.
- `ReadMapOverlayPacks @ 0x005FD2E0` pass 2 directly writes one decoded byte to
  `CellClass+0x11E` after pass 1 constructs overlay identities.
- The program-wide `+0x11E` census contains byte consumers for ore density,
  crate subcell/damage data, walls, bridges, and overlay rendering.
- `CellClass::Reduce_Tiberium` and `PlaceTiberium` read and write the same byte
  as overlay density.

Therefore the previous phrase "tile active flag" is wrong. In the active
collapse replacement path, clearing `+0x44/+0x11E` removes any overlay identity
and per-overlay state covered by the replacement footprint.

## 4. Shared-dummy exclusion on valid maps

### 4.1 Exact admission predicate

`Cell_in_bounds_check @ 0x00568300` accepts coordinate `(x,y)` iff:

```text
W < x+y
x-y < W
y-x < W
x+y <= W + 2*H
```

The comparisons are visible at `0x00568304..0x00568335`.

### 4.2 Resize uses the same predicate

`MapClass::Resize @ 0x00565C10` uses the identical four inequalities at
`0x0056637C..0x005663AD`. For each admitted coordinate it allocates a
`0x148`-byte `CellClass`, constructs it, publishes the pointer in the fixed
512x512 slot table, and sets its level at `0x005663AF..0x0056641A`.

Consequences:

- After a successful ordinary Resize, every coordinate admitted by `Mark` has
  a real `CellClass` slot.
- A primary `Mark` footprint lookup cannot return the shared dummy on a valid
  map.
- Out-of-diamond footprint candidates skip before `Get_CellClass` and cannot
  stamp the dummy.
- A null TMP entry skips before writes and is not a dummy/rim route.

Only inconsistent/corrupt lifecycle state can force a miss after the guard:
manual slot corruption, partial/malformed save state, or an external deletion
that leaves an admitted slot null. Native would then mutate the shared dummy in
modes 1/3, and those mutations would persist until dummy reconstruction; this
is malformed-state behavior, not an active retail-map contract.

`RecalcAttributes @ 0x0047D2B0` immediately returns when passed the shared
dummy. That guard is later than the four explicit LAT-neighbor calls, however,
so their fallback side effects require a separate contract.

### 4.3 Cardinal LAT calls do stamp the dummy coordinate

After every mode-1/3 nonnull primary stamp, `Mark` executes:

```text
ApplyLAT(primary)
GetCell(step(primary, 0)); ApplyLAT(result)
GetCell(step(primary, 2)); ApplyLAT(result)
GetCell(step(primary, 4)); ApplyLAT(result)
GetCell(step(primary, 6)); ApplyLAT(result)
```

The sites are `0x0054368F..0x005436CF`. `MapCoord_StepByDir_GetCell @
0x00481810` does not call the Size-diamond predicate; it adds the signed
direction pair to receiver `+0x24` and calls `Get_CellClass` directly. A valid
primary cell at the diamond edge can therefore miss on a cardinal neighbor,
return the shared dummy, and persistently overwrite dummy `+0x24`. This is an
active valid-map fallback effect. The final dummy coordinate is the last miss
in the complete call order, including nested neighbor probes made while LAT is
processing an eligible real cell.

`ApplyLAT_and_SlopeFixup @ 0x0047CA80` has no dummy-address guard, but its
receiver write surface is only dword `+0x38`:

- it always reads receiver `+0x38`;
- eligible LAT branches read receiver `+0x24` and neighbor `+0x38`;
- eligible ramp branches additionally read receiver/neighbor `+0x11C`;
- LAT and slope results can write only receiver `+0x38`;
- it never writes `+0x11A..+0x11F`, `+0x44`, `+0x48`, or `+0x140`.

Constructor/reset gives the dummy `+0x38=0x0000FFFF`, `+0x11B=0`, and
`+0x11C=0`. A miss changes only `+0x24`. Tile `0xFFFF` fails every Rough, Sand,
Green, Pave, RampBase, and RampSmooth eligibility range, so an outside dummy
receiver returns false without rewriting `+0x38` and without making nested
lookups. Real edge cells do consume the dummy defaults: tile `0xFFFF` is an
out-of-family LAT neighbor, while slope zero is a flat ramp neighbor.

If another malformed or unguarded mechanism had already contaminated dummy
`+0x38` into an eligible LAT/ramp range, ApplyLAT could persistently rewrite it.
The valid replacement-Mark chain does not create that precondition. The exact
valid-map contract for this mechanism is therefore **coordinate stamp only**;
Rust's current shared dummy already models that field, and no tile/subtile or
overlay expansion is justified here.

## 5. Active caller closure

### 5.1 Constructors and factory slots

- Constructor `0x00543780` stores the type at `this+0xAC`, sets vtables and,
  when given a nonsentinel cell coordinate, converts it to cell-center world
  coordinates, calls `ObjectClass::SetRawCoords`, then virtual `Mark(1)`.
- Scalar deleting destructor `0x00543B10`, while the game is active and the
  object is placed, invokes virtual `Mark(0)` before unregistering.
- Isometric type vtable `0x007ECC48`, slot `+0x80 @ 0x007ECCC8`, points to
  explicit-coordinate wrapper `0x00549AA0`.
- Slot `+0x8C @ 0x007ECCD4` points to sentinel-coordinate wrapper
  `0x00549AE0`.

A full census of 79 `g_IsometricTileTypeClass_Array` xrefs and 182 program-wide
indirect `+0x80` calls found no array-sourced active caller of the explicit
wrapper. The BSurface/class-vtable name collisions were rejected by receiver
dataflow. For the active retail scope, no mode-3 owner was established; the
complete array-sourced `+0x8C` closure below supplies only modes 0 and 1.

### 5.2 Destroyable-cliff collapse makes six objects

`FUN_00581140` is destroyable-cliff destruction, not bridge destruction.
It is the complete active array-sourced `+0x8C` owner found by the census.

| Family | Operation | Type factory call | Object operation |
|---|---|---:|---:|
| A | remove current destroyable cliff | `0x005811AA` | `Mark(0) @ 0x00581203` |
| A | place replacement 1 | `0x00581228` | virtual `+0xD8 @ 0x00581248` -> `Mark(1)` |
| A | place replacement 2 | `0x00581264` | virtual `+0xD8 @ 0x005812A0` -> `Mark(1)` |
| B | remove current destroyable cliff | `0x00581892` | `Mark(0) @ 0x005818EB` |
| B | place replacement 1 | `0x00581911` | virtual `+0xD8 @ 0x00581935` -> `Mark(1)` |
| B | place replacement 2 | `0x0058194D` | virtual `+0xD8 @ 0x00581989` -> `Mark(1)` |

Isometric object virtual `+0xD8` resolves to `0x00543A10`: clear object
`+0x81`, set raw coordinates, then `Mark(1)`. The replacement types are rooted
at the theater's resolved `SlopeSetPieces` tile globals.

Direct callers of `FUN_00581140` are:

- projectile/damage helper `FUN_0070C690` at `0x0070CB2B` and `0x0070CC39`;
- `WaveClass::DamageArea` at `0x0075F4B2`.

Each path first checks `IsDestroyableCliff @ 0x00486900`, then gates the call
on `RandomRanged(0,99) < RulesClass+0x17CC` (`CollapseChance`).

## 6. Retail data and trigger frequency

### 6.1 Theater and rules activation

`Read_Theater_TileSets_INI @ 0x00545150`:

- reads `DestroyableCliffs` with default `-2` at `0x00545965..0x00545978`;
- initializes global `0x00ABC2C8=-2` at `0x00545B1F`;
- when the configured tileset ordinal is reached, writes the resolved cumulative
  first tile id at `0x00545F3A..0x00545F43`.

Current retail MD theater inputs enable the set:

| Theater input | Ordinal | Resolved first tile id | TilesInSet |
|---|---:|---:|---:|
| `ini/temperatmd.ini` | 56 | 572 | 2 |
| `ini/urbanmd.ini` | 56 | 572 | 2 |
| `ini/urbannmd.ini` | 56 | 572 | 2 |
| `ini/desertmd.ini` | 56 | 528 | 2 |
| `ini/snowmd.ini` | 61 | 694 | 2 |
| `ini/lunarmd.ini` | 56 | no tiles | 0 |

The non-lunar sections name `SetName=Destroyable Cliffs`, `FileName=dcliff`.
`ini/rulesmd.ini:908` sets `CollapseChance=100`.

This corrects `CLIFF_OBJECTS_GHIDRA_REPORT.md`'s stale assertion that no
standard YR theater defines the set. The executable and retail assets are
active-capable.

### 6.2 Shipped-map corpus result

The installed retail map directory was scanned directly, including the MIX
entries inside `.mmx` and `.yro` containers:

```text
SUMMARY scanned=55 ok=55 fail=0 hit_files=0
```

For each decoded map, the scan selected the resolved two-tile range for its
theater and counted IsoMapPack5 tile ids. None of the 55 shipped files contains
either destroyable-cliff tile id. The same corpus had already been independently
loaded by the current Rust `inspect-maps` diagnostic, which also reports all 55
files readable; the CNCMaps FileFormats 2.4.0 decoder was used for the independent
tile-id census. A custom-map corpus control, `2_impasse.map`, contains 20 cells
of `TEMPERATE` tile 573, proving the decoder and retail-data gate do observe
authored destroyable-cliff placements.

Trigger frequency is therefore:

- **shipped retail maps:** zero;
- **supported custom/editor-authored maps using retail `dcliff` assets:**
  conditional on projectile/wave damage reaching the tile; with retail
  `CollapseChance=100`, the probability gate then always passes.

This is not TS-dead code, but it is outside the ordinary shipped-map loop. It
remains a Phase 3 residual because the phase goal includes active retail-format
custom maps and does not permit an approximate or missing conditional mechanism.

## 7. Ordinary map load and RMG are separate mechanisms

### 7.1 IsoMapPack5

The ordinary path is:

```text
ScenarioClass::Full_Init
  -> Read_Map_Section_And_IsoMapPacks @ 0x004ACE70
  -> IsoMapPack5 reader @ 0x0056BAC0
```

The reader directly writes Cell `+0x38`, `+0x11A`, `+0x11B`, and `+0x119`.
An invalid/null record consumes the seven payload bytes after its coordinate
header but does not stamp tile payload into the dummy. It never constructs an
`IsometricTileClass`.

`Full_Init` later calls `ReadMapOverlayPacks @ 0x005FD2E0`: pass 1 constructs
overlay identities, pass 2 writes byte `+0x11E`, then the full cell Recalc pass
begins. There is no `0x00543330` integration in this load sequence.

### 7.2 Random map generator

`RandomMapGenerator::StampIsometricTileBlock @ 0x005A6C10` has fourteen RMG
callers. It directly writes `+0x11A`, `+0x38`, optional `+0x11B`, and `+0x11C`.
It does not clear overlays or `+0x11E`, run the `Mark` LAT/cardinal sequence,
construct `IsometricTileClass`, or consume `DestroyableCliffs`.

Therefore neither ordinary map load nor RMG may be routed through the dynamic
`IsometricTileClass::Mark` transaction in Rust.

## 8. Rust evidence and required ownership

### 8.1 What already matches

- `src/map/theater.rs::TheaterCliffRanges` retains the resolved
  `destroyable_cliffs` start and classifies the two-tile range.
- `src/map/resolved_terrain.rs` materializes the native Size diamond and keeps
  allocation lookup separate from shared-dummy fallback.
- Ordinary map load directly materializes cells; it does not manufacture
  isometric tile objects.
- `src/map/rmg` owns direct RMG tile stamping separately.

### 8.2 GSI-04.01 delta

**No code delta.** Do not extend `SharedCellDummy` with tile id, subtile,
OverlayData, overlay identity, or a generic `Mark` object solely for this path.
The valid-map predicate/allocation proof excludes the mutation.

Acceptance check for the row:

1. a valid Resize allocation contains every `Cell_in_bounds_check` coordinate;
2. an out-of-diamond footprint candidate skips before GetCell;
3. a null TMP entry produces no lookup-side effect;
4. ordinary IsoMap OOB payload does not stamp dummy tile state.

### 8.3 Cross-row missing mechanism

No Rust `CollapseChance` consumer or `FUN_00581140`-equivalent transaction was
found. A later builder must not implement only `Mark` as an isolated object
facsimile. It must first complete a dedicated exact investigation of the full
destroyable-cliff transaction, then translate its observable state changes
through the existing Rust owners.

Minimum proven transaction requirements from this report are:

| Required effect | Native evidence | Rust owner candidate | Do not do |
|---|---|---|---|
| classify only the resolved two-tile destroyable range | `0x00486900`; theater reader | `TheaterCliffRanges` | do not use `SetName.contains` |
| use scenario RNG and strict `< CollapseChance` gate | projectile/wave callers | combat/AoE transaction | do not reuse bridge RNG aliases without stream proof |
| remove old TMP footprint before replacements | `Mark(0)` calls | mutable resolved terrain | do not overwrite only the struck cell |
| place two slope replacement footprints in native order | six-call ledger | terrain tile mutation service | do not call RMG raw stamper |
| clear overlay id and OverlayData on replacement cells | `0x0054366A..0x00543671` | `OverlayGrid` + terrain transaction | do not mutate terrain without overlay state |
| apply wrapping height and exact slope canonicalization | `Mark(1)` body | resolved terrain/elevation owner | do not saturate bytes |
| LAT self then dirs 0,2,4,6 and Recalc each nonnull cell | `0x0054368F..0x005436DC` | terrain/zone refresh | do not batch in an order-changing pass |
| let every out-of-diamond cardinal GetCell miss stamp the one live dummy coordinate while eligible real edge cells consume dummy tile `0xFFFF` / slope `0` | `0x00481810`; ApplyLAT `0x0047CA80` | `SharedCellDummy` coordinate + terrain LAT owner | do not silently clip neighbor lookups at the allocation mask |
| preserve sparse TMP holes and diamond skips | footprint ledger | TMP registry + grid | do not fill the rectangle blindly |

Before code, the follow-up investigation must close the rest of
`FUN_00581140`: exact family-origin arithmetic, replacement type/subtile
selection, RNG draw order, animation creation, target invalidation, dirty-cell
and screen rectangles, zone/subzone rebuild order, and all early exits. Until
that report and its implementation/critic loop pass, the cross-row mechanism
and its owning Phase 3 rows remain open.

## 9. Coverage ledger and resolved questions

| Question | Resolution |
|---|---|
| exact function identity and ABI | resolved: IsometricTileClass virtual Mark, `__thiscall`, `RET 4` |
| `+0x11E` type and meaning | resolved: byte OverlayData |
| footprint and rim | resolved: row-major rectangle, null TMP holes, no rim pass |
| bounds formula | resolved: four Size-diamond inequalities |
| valid-map dummy reachability | excluded by identical Resize allocation predicate |
| valid-map LAT-neighbor dummy effect | resolved: ordered `+0x24` stamps only; fresh dummy makes ApplyLAT field writes a no-op |
| mode 0 semantics | resolved, including mismatch-Recalc and wrapping subtract |
| mode 1/3 semantics | resolved, including overlay clear, wrapping add, LAT order |
| constructor/destructor reachability | resolved by vtables and complete active array caller census |
| active retail owner | resolved: destroyable-cliff collapse, not bridge/RMG |
| retail data gate | resolved: five non-lunar MD theaters have two `dcliff` tiles; lunar has zero |
| shipped-map incidence | resolved: 55/55 decoded, zero placements |
| ordinary IsoMap relation | resolved: direct writes, no class construction |
| RMG relation | resolved: direct stamper, no class construction |
| save/load relation | resolved: Isometric object Load restores/swizzles, does not Mark |
| GSI-04.01 Rust delta | resolved: none |
| cross-row Rust delta | resolved as open full-collapse mechanism; exact next research target stated |

No question within the scoped `Mark`/dummy contract remains approximate. The
full cliff-collapse caller is deliberately named as the next separate
mechanism, not rounded into this function report.

## 10. Adversarial checks

1. **Could a footprint edge hit the dummy?** A primary footprint write cannot:
   it either fails the same diamond predicate used by Resize or has a
   materialized slot. Its unguarded cardinal LAT lookups can hit the dummy and
   stamp only its live coordinate under constructor-fresh valid state.
2. **Does a null TMP entry behave like a visual rim?** No: it skips every cell
   write, LAT call, and Recalc.
3. **Can mode 0 silently clear ore/overlay data?** No: mode 0 does not touch
   `+0x44/+0x11E`; replacement mode 1 does.
4. **Can RMG reuse this transaction?** No: the native RMG direct stamper has a
   different write set and ordering.
5. **Is the mechanism TS-only because the default is -2?** No: retail non-lunar
   theater INIs replace the sentinel with real two-tile ranges. The precise
   exclusion is only zero placement in the 55 shipped maps, not dead code.

Cold checks performed:

- reread `0x00543330` assembly around mode-0 mismatch/Recalc and the byte
  `+0x11E` write;
- independently matched `0x00568300` inequalities to the Resize allocation
  loop;
- cold-decompiled all `0x0047CA80` receiver reads/writes and replayed the
  constructor-fresh dummy through every eligibility range;
- reconciled `FUN_00581140` against the current binary theater loader, current
  retail MD INIs, and the direct installed retail map corpus.

## Sources

### Live Ghidra evidence

- `IsometricTileClass::Mark @ 0x00543330`
- `IsometricTileClass` constructor `0x00543780`
- Isometric placement helper `0x00543A10`
- scalar deleting destructor `0x00543B10`
- type creation wrappers `0x00549AA0`, `0x00549AE0`
- `FUN_00581140` destroyable-cliff replacement transaction
- `IsDestroyableCliff @ 0x00486900`
- `Cell_in_bounds_check @ 0x00568300`
- `MapClass::Resize @ 0x00565C10`
- IsoMapPack5 reader `0x0056BAC0`
- RMG tile stamper `0x005A6C10`
- `ReadMapOverlayPacks @ 0x005FD2E0`
- projectile/wave caller sites `0x0070CB2B`, `0x0070CC39`, `0x0075F4B2`

### Retail data

- `ini/temperatmd.ini`
- `ini/snowmd.ini`
- `ini/urbanmd.ini`
- `ini/urbannmd.ini`
- `ini/desertmd.ini`
- `ini/lunarmd.ini`
- `ini/rulesmd.ini`
- installed retail map corpus under
  `C:/Users/enok/Documents/Command and Conquer Red Alert II`

### Corroborating reports, with corrections applied here

- `PHASE3_MAPCLASS_GET_CELLCLASS_005657A0_DUMMY_CONTRACT_GHIDRA_REPORT.md`
- `PHASE3_MAPCLASS_RESIZE_00565C10_GRID_LIFECYCLE_GHIDRA_REPORT.md`
- `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md`
- `THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md`
- `CLIFF_OBJECTS_GHIDRA_REPORT.md` — its "no YR theater defines
  DestroyableCliffs" statement is stale and superseded.
- `PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` — its name for
  `0x00543330` is stale and superseded.
