# TeleportLocomotionClass::Constructor — 0x00718000

**Proposed Ghidra label:** TeleportLocomotionClass__Constructor (existing name is authoritative — labeler skip rename, add plate comment only)

## Summary

Initializes a freshly-allocated `TeleportLocomotionClass` object. Sets three COM vtable pointers (IUnknown, ILocomotion, IPiggyback), then zeroes all state fields and fills both destination-cache slots with the `g_NullCoord_Teleport` sentinel (0, 0, 0). Calls `LocomotionClass__Constructor` as the base-class init.

Object total size: **0x4c bytes** (76), verified from `push 0x4c` before `operator new` at the call site `0x006c4c3e`.

CLSID: `{4A582747-9839-11d1-B709-00A024DDAFD1}` (from plate comment, verified via `get_plate_comment 0x00718000`).

## Active in YR

**Yes.** Constructor is called via a COM factory at `0x006c4c4c` (verified via `get_xrefs_to 0x00718000`). The factory performs `operator new(0x4c)` then calls the constructor. The ILocomotion interface it sets up is consumed by `TeleportLocomotionClass__Process` along with HeadToCoord, Stop_Moving, StateMachineTick, and Update_Position — all of which READ `g_NullCoord_Teleport_X` at the correct address 0x00b0ebf8 (13 READ xrefs total, verified via `get_xrefs_to 0x00b0ebf8`). The 3 reads at 0x00b0ebd8 originally cited here are an UNRELATED global (only 3 reads in Process body, no behavioral match) — they are NOT g_NullCoord_Teleport.

## Decompilation Excerpt

Source: `decompile_function 0x00718000`

```c
undefined4 * __fastcall TeleportLocomotionClass__Constructor(undefined4 *param_1)
{
  undefined4 uVar1;
  
  LocomotionClass__Constructor();            // base class init (0x0055a6c0)
  param_1[7]  = g_NullCoord_Teleport_X;     // +0x1c: dest-cache-0 X = 0
  param_1[8]  = g_NullCoord_Teleport_Y;     // +0x20: dest-cache-0 Y = 0
  param_1[9]  = g_NullCoord_Teleport_Z;     // +0x24: dest-cache-0 Z = 0
  param_1[10] = g_NullCoord_Teleport_X;     // +0x28: dest-cache-1 X = 0
  param_1[11] = g_NullCoord_Teleport_Y;     // +0x2c: dest-cache-1 Y = 0
  param_1[12] = g_NullCoord_Teleport_Z;     // +0x30: dest-cache-1 Z = 0
  *(undefined1 *)(param_1 + 0xd) = 0;       // +0x34: state machine state = 0 (phase 0)
  *(undefined1 *)((int)param_1 + 0x35) = 0; // +0x35: flag byte = 0
  *(undefined1 *)((int)param_1 + 0x36) = 0; // +0x36: flag byte = 0
  param_1[0xe] = 0;                         // +0x38: cleared (frame stamp or timer low)
  uVar1 = g_CurrentFrameCounter;
  param_1[0x11] = 0;                        // +0x44: cleared (timer field)
  param_1[0xf]  = uVar1;                    // +0x3c: frame counter snapshot at init
  param_1[0x12] = 0;                        // +0x48: cleared
  *param_1      = &TeleportLocomotionClass__IUnknown_vtable;    // +0x00
  param_1[1]    = &TeleportLocomotionClass__ILocomotion_vtable; // +0x04
  param_1[6]    = &TeleportLocomotionClass__IPiggyback_vtable;  // +0x18
  return param_1;
}
```

**POINTER-ARITHMETIC NOTE:** `param_1` is `undefined4 *` (int pointer), so `param_1[N]` = byte offset `N × 4`. All offsets in the table below are the resulting byte offsets.

## Behavioral Analysis

### Control flow

Linear — no branches. Single call to base class constructor, then field initialization, then vtable writes. No allocation (caller does `operator new`).

### Call chain to YR-live entry point

```
[COM factory at 0x006c4c4c]
  operator new(0x4c)            // allocates 76-byte object
  TeleportLocomotionClass__Constructor(result)  // 0x00718000
    LocomotionClass__Constructor()               // 0x0055a6c0
```

The factory at `0x006c4c4c` is not a named function in Ghidra but contains the `push 0x4c / call operator_new / call Constructor` pattern (verified via `read_memory 0x006c4c00`, length 96). The factory is the COM `CreateObject`/`FindOrAllocate` path — this is the YR-live allocator for TeleportLocomotionClass instances.

### Init sequencing

State fields are zeroed before vtable pointers are written. This is the standard C++ ordering: base-class construction → member initialization → vtable stamping. The base class `LocomotionClass__Constructor` (0x0055a6c0, verified via `get_function_callees 0x00718000` + `get_function_by_address 0x0055a6c0`) is called first with ECX pointing at the object (fastcall).

### Destination-cache design

Two separate 3-field (X, Y, Z) coordinate caches exist at `+0x1c..+0x24` and `+0x28..+0x30`. Both are zeroed to the `g_NullCoord_Teleport` sentinel. The Process function reads both to decide whether a valid warp destination is cached.

### Frame counter capture

`g_CurrentFrameCounter` is read and stored at `+0x3c` during construction. This seeds the frame-based timer that `TimerCheck` uses.

## Struct Field Accesses

`param_1` is `undefined4 *` — all offsets are byte offsets = index × 4.

| Byte Offset | Index (param_1[N]) | Size | Init Value | Purpose |
|---|---|---|---|---|
| +0x00 | [0] | 4 | `0x007f50cc` | IUnknown vtable pointer |
| +0x04 | [1] | 4 | `0x007f5010` | ILocomotion vtable pointer |
| +0x08..+0x14 | [2..5] | 20 | (base class inits) | LocomotionClass fields (set by base ctor) |
| +0x18 | [6] | 4 | `0x007f4fe0` | IPiggyback vtable pointer |
| +0x1c | [7] | 4 | 0 (g_NullCoord sentinel) | dest-cache-0 X coordinate |
| +0x20 | [8] | 4 | 0 | dest-cache-0 Y coordinate |
| +0x24 | [9] | 4 | 0 | dest-cache-0 Z coordinate |
| +0x28 | [10] | 4 | 0 | dest-cache-1 X coordinate |
| +0x2c | [11] | 4 | 0 | dest-cache-1 Y coordinate |
| +0x30 | [12] | 4 | 0 | dest-cache-1 Z coordinate |
| +0x34 | byte at [0xd] | 1 | 0 | State machine state (phase 0 = idle) |
| +0x35 | byte +0x35 | 1 | 0 | Flag byte |
| +0x36 | byte +0x36 | 1 | 0 | Flag byte |
| +0x38 | [0xe] | 4 | 0 | Frame stamp / timer low word |
| +0x3c | [0xf] | 4 | g_CurrentFrameCounter | Frame counter snapshot at construction |
| +0x44 | [0x11] | 4 | 0 | Timer field |
| +0x48 | [0x12] | 4 | 0 | Timer field |

**Coordinate frame:** offsets +0x1c..+0x30 hold warp destination coordinates. Frame is `GetCoords`-space (leptons, geometric center), consistent with how `InitiateWarp` passes destination coords. These are TeleportLocomotionClass-internal caches, not TechnoClass fields.

**Vtable addresses verified** via:
- `get_xrefs_to 0x00718080` (Is_Moving) → DATA xref from `0x007f5010` → ILocomotion vtable base
- `read_memory 0x007f5010` (80 bytes) → confirmed first slot = `0x00718080` (Is_Moving), second = `0x007180a0` (Destination)
- `get_xrefs_to 0x00719e30` (QueryInterface) → DATA xref from `0x007f50cc` → IUnknown vtable base; `read_memory 0x007f50cc` → slot 0 = `0x00719e30` ✓
- `get_xrefs_to 0x00719e90` (Begin_Piggyback) → DATA xref from `0x007f4fe8`; `read_memory 0x007f4fe0` → slot 2 = `0x00719e90` ✓; vtable base = `0x007f4fe0`

## Globals / Enums / INI Keys Referenced

| Symbol | Address | Type | Value | Role |
|---|---|---|---|---|
| `g_NullCoord_Teleport_X` | `0x00b0ebf8` | int32 | 0x00000000 | Sentinel X for "no dest cached" (verified `read_memory 0x00b0ebf8`, 12 bytes: all zeros). Address corrected from earlier 0x00b0ebd8 which is an unrelated global with only 3 Process-body reads. |
| `g_NullCoord_Teleport_Y` | `0x00b0ebfc` | int32 | 0x00000000 | Sentinel Y |
| `g_NullCoord_Teleport_Z` | `0x00b0ec00` | int32 | 0x00000000 | Sentinel Z |
| `g_CurrentFrameCounter` | (global, read via `uVar1 = g_CurrentFrameCounter`) | int32 | — | Game frame tick counter; stored at +0x3c to seed frame-based warp timer |

**Sentinel value:** All three sentinel coords are `0` (zero), not `-1`. At map coordinates, (0, 0, 0) is never a valid unit location (the map has a border of impassable cells), so zero serves as a safe "no destination" sentinel. Verified via `read_memory 0x00b0ebf8` (12 bytes, all zeros).

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `LocomotionClass__Constructor` | `0x0055a6c0` | Base-class infrastructure; not TeleportLocomotionClass-specific |
| COM factory at | `0x006c4c4c` | Unlabeled factory function; allocation infrastructure; out of scope |

## Unverified (YELLOW)

- **`g_CurrentFrameCounter` exact address**: The decompile names the global symbolically; the exact memory address was not directly read via `read_memory`. The address is not needed for behavioral understanding of the constructor (it seeds the timer field at +0x3c). Confirm via struct-decode task or `get_xrefs_to` on the frame counter global if precise address is needed.

- **Fields +0x08..+0x14** (base-class region): these 20 bytes between IPiggyback-vtable slot and are set by `LocomotionClass__Constructor`. Their layout is not decoded here (base class scope). Noted as "set by base ctor" — decode if struct-decode task requires them.

- **Factory function identity** at `0x006c4c4c`: confirmed as `operator new` + constructor call site, but the containing function is unlabeled in Ghidra. It is the COM-style `CreateObject` factory for `TeleportLocomotionClass` (plate comment mentions CLSID `{4A582747-9839-11d1-B709-00A024DDAFD1}`). Full identity left to struct/COM-stubs decode tasks.
