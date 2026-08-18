# TeleportLocomotionClass — Accessor Bundle

**Proposed Ghidra label (Is_Moving 0x00718080):** TeleportLocomotionClass__Is_Moving (existing name authoritative — labeler skip rename, add plate comment only)
**Proposed Ghidra label (Destination 0x007180a0):** TeleportLocomotionClass__Destination (existing name authoritative)
**Proposed Ghidra label (HeadToCoord 0x00718100):** TeleportLocomotionClass__HeadToCoord (existing name authoritative)
**Proposed Ghidra label (Stop_Moving 0x00718230):** TeleportLocomotionClass__Stop_Moving (existing name authoritative)
**Proposed Ghidra label (Mark_All_Occupation_Bits 0x007192c0):** TeleportLocomotionClass__Mark_All_Occupation_Bits (existing name authoritative)

## Summary

Five short accessor/mutator methods on `TeleportLocomotionClass`, all dispatched through the ILocomotion vtable at `0x007f5010` (verified via `read_memory 0x007f5010`). Because they are called via the ILocomotion interface, `param_1` in each function is the **ILocomotion sub-object pointer** = `full_object + 0x04`. All field offsets in these functions must be interpreted relative to that base.

## Active in YR

**Yes** for all five. They are entries in the ILocomotion vtable which is installed by the constructor and used by the locomotion dispatch system. `HeadToCoord` is called by `FootClass::Set_Destination_Internal` (noted in plate comment, verified via `get_plate_comment 0x00718100`). The others are called via vtable dispatch from game code driving the locomotor state.

## ILocomotion Vtable Layout (relevant slots)

Source: `read_memory 0x007f5010` (104 bytes, 26 slots).

| VTable Slot | Offset in vtable | Function address | Name |
|---|---|---|---|
| 0 | +0x00 | `0x00718080` | Is_Moving |
| 1 | +0x04 | `0x007180a0` | Destination |
| 12 | +0x30 | `0x007192f0` | StateMachineTick (out of scope here) |
| 13 | +0x34 | `0x00718100` | HeadToCoord |
| 17 | +0x44 | `0x00718230` | Stop_Moving (confirmed plate comment) |
| 25 | +0x64 | `0x00719e30` | QueryInterface thunk (last slot) |

**Interface dispatch base:** `param_1` in all accessor functions = `full_object + 0x04` (the ILocomotion interface pointer). Add `+0x04` to all offsets below to get full-object byte offsets.

## Is_Moving — 0x00718080

Source: `decompile_function 0x00718080`

```c
bool TeleportLocomotionClass__Is_Moving(int param_1)
{
  return *(char *)(param_1 + 0x30) == '\x01';
}
```

- `param_1` = ILocomotion sub-object = `full_object + 0x04`
- `param_1 + 0x30` = `full_object + 0x34` = **state counter / IsMoving flag**
- Returns `true` when byte at `full_object+0x34` is exactly `1`.
- This is the locomotor's "armed for warp" flag set by `HeadToCoord` and cleared by `Stop_Moving`.

## Destination — 0x007180a0

Source: `decompile_function 0x007180a0`

```c
void TeleportLocomotionClass__Destination(int *param_1)
{
  char cVar4;
  // param_1 is int* (ILocomotion sub-object base)
  // vtable+0x10 = Is_Moving check
  cVar4 = (**(code **)(*param_1 + 0x10))(param_1);  // vtable slot: Is_Moving
  if (cVar4 != '\0') {
    // IsMoving: return HeadToCoord cache
    *param_1     = param_1[6];   // +0x18 = full_object+0x1c (HeadToCoord X)
    param_1[1]   = param_1[7];   // +0x1c = full_object+0x20 (HeadToCoord Y)
    param_1[2]   = param_1[8];   // +0x20 = full_object+0x24 (HeadToCoord Z)
    return;
  }
  // Not moving: return current TechnoClass location
  iVar1 = param_1[2];           // +0x08 = full_object+0x0c = TechnoClass owner ptr
  *param_1   = *(iVar1 + 0x9c); // TechnoClass Location X (leptons)
  param_1[1] = *(iVar1 + 0xa0); // TechnoClass Location Y
  param_1[2] = *(iVar1 + 0xa4); // TechnoClass Location Z
}
```

**NOTE:** `param_1` here is `int *` — the return value is written back through `param_1` (output parameter pattern). The caller allocates a coord struct and passes a pointer. Returns the unit's active destination: HeadToCoord coords if moving, current location if idle.

## HeadToCoord — 0x00718100

Source: `decompile_function 0x00718100`. Plate comment verified via `get_plate_comment 0x00718100`.

```c
void TeleportLocomotionClass__HeadToCoord
     (int param_1, undefined4 param_2, undefined4 param_3, undefined4 param_4)
{
  // param_1 = int (direct byte offsets; ILocomotion sub-object base)
  // param_2/3/4 = requested destination coords (X, Y, Z)
  // param_1 + 8 = full_object+0x0c = TechnoClass owner pointer

  // Gate checks: 4 TechnoClass flags must all be clear
  // vtable+0x37c: IsBusy / in-deploy flag
  // vtable+0x380: second deploy/abort flag
  // vtable+0x1d4: third gate flag
  // vtable+0x1d8: fourth gate flag
  if (any_gate_set) {
    *(TechnoClass + 0x5a4) = 0;  // clear queued mission
    return;
  }

  // Infantry scatter: if owner is infantry (What_Am_I == 0xf) and some flag set
  if ((TechnoClass[0x7e] != 0) && (What_Am_I() == 0xf)) {
    CellClass__Get_Cell_At(&dest);
    CellClass__Scatter_Objects(&g_NullCoord_Teleport_X, 1, 1, 0);
  }

  // Validate destination via Process
  TeleportLocomotionClass__Process(&dest);

  // Check if Process rejected the dest (returned NullCoord in cache-0)
  if (cache0 == NullCoord) {
    (**(TechnoClass vtable+0x480))(0, 1);  // broadcast dest-rejected event
    return;
  }

  // Arm the locomotor
  *(param_1 + 0x30) = 1;                    // IsMoving flag = 1
  *(param_1 + 0x18) = *(param_1 + 0x24);   // HeadToCoord X = validated dest X
  *(param_1 + 0x1c) = *(param_1 + 0x28);   // HeadToCoord Y
  *(param_1 + 0x20) = *(param_1 + 0x2c);   // HeadToCoord Z
}
```

**Field mapping** (all relative to ILocomotion sub-object = `full_object + 0x04`):

| `param_1` offset | `full_object` offset | Purpose |
|---|---|---|
| +0x08 | +0x0c | TechnoClass owner pointer |
| +0x18 | +0x1c | HeadToCoord X (armed warp destination X) |
| +0x1c | +0x20 | HeadToCoord Y |
| +0x20 | +0x24 | HeadToCoord Z |
| +0x24 | +0x28 | dest-cache-0 X (from Process, validated dest) |
| +0x28 | +0x2c | dest-cache-0 Y |
| +0x2c | +0x30 | dest-cache-0 Z |
| +0x30 | +0x34 | IsMoving flag (1 = armed, 0 = idle) |

## Stop_Moving — 0x00718230

Source: `decompile_function 0x00718230`

```c
void TeleportLocomotionClass__Stop_Moving(int param_1)
{
  *(param_1 + 0x18) = g_NullCoord_Teleport_X;  // HeadToCoord X = null sentinel
  *(param_1 + 0x1c) = g_NullCoord_Teleport_Y;  // HeadToCoord Y = null sentinel
  *(param_1 + 0x20) = g_NullCoord_Teleport_Z;  // HeadToCoord Z = null sentinel
  *(char *)(param_1 + 0x30) = 0;                // IsMoving = 0
  *(char *)(param_1 + 0x32) = 0;                // extra flag at +0x32 = 0
}
```

Clears the HeadToCoord cache and marks the locomotor as not moving. Also clears a second byte flag at `param_1+0x32` (`full_object+0x36`) whose purpose is documented in struct-decode.

## Mark_All_Occupation_Bits — 0x007192c0

Source: `decompile_function 0x007192c0`

```c
void TeleportLocomotionClass__Mark_All_Occupation_Bits(undefined4 param_1, undefined4 param_2)
{
  RateTimer__Set(&param_2);
  return;
}
```

This is a thin wrapper that calls `RateTimer__Set` on `param_2`. The name "Mark_All_Occupation_Bits" (inherited from base class ILocomotion interface slot) is misleading for the teleport locomotor — the implementation does not actually mark map occupation bits but instead sets a rate timer. This is likely a no-op or minimal stub implementation for the teleport locomotor, which does not use the standard occupation bit management.

## Struct Field Accesses

All offsets relative to **ILocomotion sub-object** (`full_object + 0x04`). Add `+0x04` for full-object byte offset.

| Sub-obj offset | Full-obj offset | Access | Purpose |
|---|---|---|---|
| +0x08 | +0x0c | `param_1[2]` (int*) | TechnoClass owner pointer |
| +0x18 | +0x1c | `*(param_1 + 0x18)` | HeadToCoord X (armed warp dest X) |
| +0x1c | +0x20 | `*(param_1 + 0x1c)` | HeadToCoord Y |
| +0x20 | +0x24 | `*(param_1 + 0x20)` | HeadToCoord Z |
| +0x24 | +0x28 | `*(param_1 + 0x24)` | dest-cache-0 X (Process-validated dest) |
| +0x28 | +0x2c | `*(param_1 + 0x28)` | dest-cache-0 Y |
| +0x2c | +0x30 | `*(param_1 + 0x2c)` | dest-cache-0 Z |
| +0x30 | +0x34 | `*(char *)(param_1 + 0x30)` | IsMoving flag / state byte |
| +0x32 | +0x36 | `*(char *)(param_1 + 0x32)` | Flag byte cleared by Stop_Moving |

TechnoClass fields accessed (via `*(int **)(param_1 + 8)` = TechnoClass ptr):
- `+0x9c`: Location X (leptons)
- `+0xa0`: Location Y
- `+0xa4`: Location Z
- `+0x5a4`: Queued mission / pending command (cleared on gate-fail)

## Globals / Enums / INI Keys Referenced

| Symbol | Address | Role |
|---|---|---|
| `g_NullCoord_Teleport_X/Y/Z` | `0x00b0ebf8..+0x08` | Null sentinel for HeadToCoord cache (verified `read_memory 0x00b0ebf8`; corrected from 0x00b0ebd8 — that address has only 3 unrelated Process reads) |

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `CellClass__Get_Cell_At` | `0x00565730` | General cell lookup; not teleport-specific |
| `CellClass__Scatter_Objects` | (called in HeadToCoord) | General scatter utility; not teleport-specific |
| `TeleportLocomotionClass__Process` | `0x00718b70` | Separate decode task |
| `RateTimer__Set` | (callee in Mark_All_Occupation_Bits) | General timer utility; not teleport-specific |

## Unverified (YELLOW)

- **`Destination` function's param_1 base**: the `Destination` function takes `int *param_1` and writes back to `*param_1`, `param_1[1]`, `param_1[2]`. This is unusual — it writes the returned coord into the pointer passed in. If `param_1` points to the vtable sub-object, overwriting `param_1[0..2]` would trash vtable and owner pointer. More likely, the caller allocates a local coord struct and passes it as the first argument (output param), and `param_1[2]` reads the TechnoClass pointer from offset +8 of the sub-object. This needs cross-check with calling convention in FootClass dispatcher — `param_1` may be a scratch struct, not the actual object pointer. Marked YELLOW.

- **`Mark_All_Occupation_Bits` semantics**: `RateTimer__Set(&param_2)` — `param_2` is passed by reference. The RateTimer being set is unclear (it could be a local or the locomotor's own timer). The function name is a base-class slot name; the teleport implementation may simply be a stub. Not independently verified against caller behavior.

- **TechnoClass vtable slots `+0x37c`, `+0x380`, `+0x1d4`, `+0x1d8`** (gate flags in HeadToCoord): identity not verified. Likely deploy/boarding/freeze flags from the TechnoClass interface. Noted for struct-decode task.
