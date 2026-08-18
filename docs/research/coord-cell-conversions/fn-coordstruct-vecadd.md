# CoordStruct::VecAdd — decode doc

**Address:** `0x006ce240`
**Kind:** function-decode-v1
**Source:** decompile_function @ 0x006ce240

---

## Summary

`CoordStruct::VecAdd` (0x006ce240) performs component-wise 3D vector addition of two
CoordStructs and writes the result into a third caller-supplied output buffer:
`out = A + B` for each of {X, Y, Z}. Both operands and the output are `int*` pointers to
CoordStruct (3 × int32 in leptons). No callees, no globals. Leaf function.

---

## Active in YR

**YES — called in sim and render contexts during normal YR gameplay.**

2 direct callers via `get_function_callers 0x006ce240`:
- `SuperClass__Launch` @ `0x006cc390` — superweapon launch position computation
- `TechnoClass__DrawExtras` @ `0x006f5190` — draw extra effects (tethers, lasers)

Both active in normal YR skirmish.

---

## Signature

```c
// verified via decompile_function 0x006ce240
void __thiscall CoordStruct__VecAdd(int *param_1, int *param_2, int *param_3)
```

- `param_1` — `int*`. Operand A (source CoordStruct).
- `param_2` — `int*`. Output CoordStruct. Receives `A + B`.
- `param_3` — `int*`. Operand B (source CoordStruct).

**`param_1`, `param_2`, `param_3` are all `int*`** — indices × 4 bytes.
`param_1[0]` = byte 0 = X, `param_1[1]` = byte 4 = Y, `param_1[2]` = byte 8 = Z.

**Calling convention:** `__thiscall` — `param_1` in ECX.

---

## Control Flow

```c
// verified via decompile_function 0x006ce240
void __thiscall CoordStruct__VecAdd(int *param_1, int *param_2, int *param_3) {
    int iVar1 = param_1[1];  // A.Y
    int iVar2 = param_3[1];  // B.Y
    int iVar3 = param_3[2];  // B.Z
    int iVar4 = param_1[2];  // A.Z
    *param_2    = *param_3 + *param_1;   // X_out = A.X + B.X
    param_2[1]  = iVar2 + iVar1;         // Y_out = A.Y + B.Y
    param_2[2]  = iVar3 + iVar4;         // Z_out = A.Z + B.Z
    return;
}
```

No branches, no guards. Pure component-wise int32 addition.

**Overflow:** `int32 + int32` wraps silently. At maximum map lepton coordinates (~131k leptons
for a 512-cell map), overflow from single additions is not expected for normal game use.

---

## Struct Field Accesses

| Index | Byte offset | Operand | Semantics |
|---|---|---|---|
| `param_1[0]` | 0 | A.X | Lepton X |
| `param_1[1]` | 4 | A.Y | Lepton Y |
| `param_1[2]` | 8 | A.Z | Lepton Z |
| `param_3[0]` | 0 | B.X | Lepton X |
| `param_3[1]` | 4 | B.Y | Lepton Y |
| `param_3[2]` | 8 | B.Z | Lepton Z |
| `param_2[0]` | 0 | Out.X | Written: A.X + B.X |
| `param_2[1]` | 4 | Out.Y | Written: A.Y + B.Y |
| `param_2[2]` | 8 | Out.Z | Written: A.Z + B.Z |

(verified via `decompile_function 0x006ce240`)

---

## Globals / INI Keys

None.

---

## Callees

None. Leaf function.
(verified via `get_function_callees 0x006ce240`)

---

## Reference Frame

Input and output are both in the **Location frame** (leptons). Reference frame depends on
what the caller passes — commonly GetCoords-relative offsets for position computation.

---

## Observable vs Internal

**Observable:** Used in superweapon launch position arithmetic and draw-extras positioning.
A wrong result misplaces the superweapon effect or a rendered extra. Player-visible.

**Internal:** The addition itself is invisible; only the resulting position matters.

---

## Rust Equivalent

```rust
// CoordStruct: [X, Y, Z] as i32 leptons at offsets 0, 4, 8.
fn vec_add(a: CoordStruct, b: CoordStruct) -> CoordStruct {
    CoordStruct {
        x: a.x.wrapping_add(b.x),
        y: a.y.wrapping_add(b.y),
        z: a.z.wrapping_add(b.z),
    }
}
```

---

## Unverified

None. All claims verified from live Ghidra in this session:
- `decompile_function 0x006ce240` — function body
- `get_function_callers 0x006ce240` — 2 callers
- `get_function_callees 0x006ce240` — no callees
