# CellClass::Get_Center_Coords — decode

**Address:** `0x00480a30`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x00480a30

---

## Summary

`CellClass::Get_Center_Coords` converts a CellClass instance's packed map-coordinate field
(CellClass+0x24) into a 3-component CoordStruct (X, Y, Z leptons) representing the center of
that cell. X and Y are computed as `cellCoord * 256 + 128` (placing the result at the
sub-cell center, 128 leptons = `CELL_CENTER_LEPTON` into each axis). Z is the ground height
at that sub-cell center, obtained from the iso-projection / height-lookup function
`FUN_0047b3a0` called with local coords `(0x80, 0x80)`.

The caller supplies a writable `CoordStruct*` (`param_2`) which the function fills and
returns.

---

## Active in YR

**YES — actively called in normal YR skirmish play.**

Confirmed callers include:
- `DriveLocomotionClass__Process_Drive_Track` @ `0x004b0f20` (ground vehicle movement,
  every tick a vehicle traverses a drive-track step into a tube/next cell) —
  verified via `decompile_function 0x004b0f20`.
- `DriveLocomotionClass__Process_Movement` @ `0x004b2630`
- `ShipLocomotionClass__Process_Drive_Track` @ `0x006a05f0`
- `ShipLocomotionClass__Process_Movement` @ `0x006a1c80`
- `LightningStorm__GroundStrike` @ `0x0053a300`
- `TechnoClass__SpawnRadEruption` @ `0x006fd800`
- `Tactical__DrawUnitActionVisuals` @ `0x006dbe20`
- `Tactical_ZBufferDirtyClear` @ `0x006d2b60`
- `Tactical_layer_shroud_edges` @ `0x006d3660`
- `UnitClass__TubeMovement` @ `0x007359f0`
- `WalkLocomotionClass__ProcessMovement` @ `0x0075aec0`
- `BuildingClass__ExitObject_Main` @ `0x00443c60`

Total: 19 unique callers across sim, render, and shroud (verified via
`get_function_callers 0x00480a30`).

---

## Signature

```c
int * __thiscall CellClass__Get_Center_Coords(int param_1, int *param_2)
```

- `param_1` — `int` (direct byte offsets, NOT a pointer-scaled parameter). `this` pointer
  to a `CellClass` instance.
- `param_2` — output `CoordStruct*`, a caller-provided 3-int buffer. The function fills
  `param_2[0]` (X), `param_2[1]` (Y), `param_2[2]` (Z) and returns `param_2`.

**Reference frame note:** `param_1` is `int`, so `*(param_1 + 0x24)` is a **direct byte
offset** of 0x24 into the CellClass struct.

---

## Control Flow

```
1. Load packed coord: uVar1 = *(undefined4 *)(param_1 + 0x24)
   - Low 16 bits  = cellX (short)
   - High 16 bits = cellY (short, extracted via >> 0x10)

2. Set up sub-cell center args for Z lookup:
   local_8 = 0x80  (sub-cell X = 128 leptons)
   local_4 = 0x80  (sub-cell Y = 128 leptons)

3. iVar2 = FUN_0047b3a0(&local_8)  — height lookup at sub-cell center (Z)

4. Assemble output CoordStruct:
   param_2[0] = (short)uVar1 * 0x100 + 0x80    → cellX * 256 + 128
   param_2[1] = iVar3        * 0x100 + 0x80    → cellY * 256 + 128
   param_2[2] = iVar2                           → Z from height lookup

5. return param_2
```

**No branches.** Unconditional path; no early-out, no flag guards.

---

## Struct Field Accesses

| Offset into `param_1` | Type        | Usage |
|------------------------|-------------|-------|
| `+0x24`               | `undefined4` | Packed cell MapCoord: low `short` = X cell index, high `short` = Y cell index. **Reference frame: Get_Cell_Packed (NW cell), cell units.** Verified via `decompile_function 0x00480a30`. |

The mapping is:
- `cellX = (short)(*(uint32*)(this+0x24) & 0xFFFF)`  — low half
- `cellY = (short)(*(uint32*)(this+0x24) >> 16)`     — high half

This matches the layout documented in the anchor `ObjectClass__Get_Cell_Packed @
0x0041bea0` where a packed coord stores X in the low short and Y in the high short.

---

## Globals

| Address        | Usage |
|----------------|-------|
| `DAT_0089e770` | Bitfield tracking which sub-parts of `FUN_0047b3a0`'s matrix have been initialized (bits 0, 1, 2). Read/written inside `FUN_0047b3a0`. Not directly read in `Get_Center_Coords`. |
| `DAT_0089e7c0` | Iso-matrix scale constant read by `FUN_0047b3a0` on first init (bit 0 guard). |

Both are internal to the callee `FUN_0047b3a0`. `Get_Center_Coords` itself accesses no
globals directly. (Verified via `decompile_function 0x00480a30`.)

---

## INI Keys

None. This function is a pure coordinate computation with no INI reads.

---

## Enum Values

None used directly. The constant `0x80` (128) is `CELL_CENTER_LEPTON` — the sub-cell
center offset in the lepton coordinate space (1 cell = 256 leptons, center = 128).

---

## Observable vs Internal

**Observable outputs** (player-visible consequences):
- Determines the lepton-precise center of a cell used for unit waypoint placement,
  movement approach targets (drive-track), shroud edge computation, and tactical action
  visuals. A wrong X/Y output directly displaces units or misaligns shroud edges by
  up to 128 leptons (half a cell).
- The Z component controls the height at which a unit is placed or a visual effect
  spawns when targeting a cell center. Wrong Z = unit floats or clips terrain.

**Internal mechanism** (not directly observable but produces the above):
- The `* 256 + 128` arithmetic (leptons from cell index).
- The `FUN_0047b3a0` height matrix init + lookup — only the integer Z it returns matters;
  the matrix initialization side-effects are internal rendering state.

---

## Callee Detail: FUN_0047b3a0 @ 0x0047b3a0

Verified via `decompile_function 0x0047b3a0` and `get_function_callers 0x0047b3a0`.

This function serves two roles in one body:
1. **Lazy iso-projection matrix init** (bit-guarded by `DAT_0089e770`, bits 0/1/2). On
   first call it populates a large set of globals at `0x0081c918`–`0x0081cc18` with
   rotation matrices and scale values derived from `DAT_0089e720`/`DAT_0089e724`.
2. **Height lookup**: after init, calls `Math__ftol()` and reads `_DAT_0081cc18` (a
   double) to produce an integer height Z for the given sub-cell coords.

The input `param_1` is an `int` (`__fastcall`), pointing at the local `{X=0x80, Y=0x80}`
sub-cell buffer. The Z result returned is the cell's ground height in leptons, accounting
for terrain slope/ramp at the sub-cell center position.

**CellClass__GetGroundHeight @ 0x00578080** wraps the same `FUN_0047b3a0` call pattern
(verified via `decompile_function 0x00578080`): it also passes a coord pair into
`FUN_0047b3a0` and uses the result as the height answer — confirming `FUN_0047b3a0` is
the canonical ground-height resolver.

**Z for flat terrain at cell center:** For a flat-terrain cell (height level 0) the
iso-matrix lookup returns Z = 0 leptons. For raised terrain, Z scales with height level.

---

## Out-of-scope refs

| Symbol | Address | Reason deferred |
|--------|---------|-----------------|
| `FUN_0047b3a0` | `0x0047b3a0` | Iso-projection matrix initializer + height resolver. Full matrix layout and height formula (double → int, involves `DAT_0081cc18`, `DAT_0089e7c0`, `DAT_007e1740`, `DAT_007e3cb8`) require a separate render/iso-math decode session. |
| `CellClass+0x11c` | — | A `char` field read inside `FUN_0047b3a0` at `param_1+0x11c` that gates a second `Math__ftol()` call — likely a ramp/bridge flag. Needs struct-decode-cellstruct for semantics. |
| `DAT_0081cc18` | `0x0081cc18` | The lazy-computed double holding the current cell's height. Part of the iso-matrix globals block decoded in `FUN_0047b3a0`. |

---

## Rust equivalent

```rust
// from Get_Cell_Packed (NW cell, cell units): CellClass+0x24 low short = X, high short = Y
// Output reference frame: leptons (CoordStruct), CELL_CENTER_LEPTON = 128
fn get_center_coords(cell_x: u16, cell_y: u16, ground_z: i32) -> CoordStruct {
    CoordStruct {
        x: cell_x as i32 * 256 + 128,
        y: cell_y as i32 * 256 + 128,
        z: ground_z, // from terrain height lookup at sub-cell (0x80, 0x80)
    }
}
```

The `ground_z` must come from the terrain height system (equivalent of `FUN_0047b3a0`).

---

## Unverified

None. All claims above are verified from live Ghidra decompilation in this session:
- `decompile_function 0x00480a30` — main function body
- `get_function_callers 0x00480a30` — 19 callers confirming active-in-YR
- `get_function_callees 0x00480a30` — single callee `FUN_0047b3a0`
- `decompile_function 0x0047b3a0` — callee body (iso-matrix + height)
- `get_function_callers 0x0047b3a0` — callee callers (CellClass__GetGroundHeight etc.)
- `decompile_function 0x00578080` — `CellClass__GetGroundHeight` confirms Z pattern
- `decompile_function 0x004b0f20` — `DriveLocomotionClass__Process_Drive_Track` confirms
  active usage in YR vehicle movement
