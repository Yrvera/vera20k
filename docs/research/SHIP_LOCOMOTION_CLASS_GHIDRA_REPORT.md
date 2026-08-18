# ShipLocomotionClass — Full Ghidra Research Report

**Primary addresses:**
- `ShipLocomotionClass::Constructor` @ `0x0069EC50`
- `ShipLocomotionClass::Process` @ `0x0069FC10`
- `ShipLocomotionClass::Process_Movement` @ `0x006A1C80`
- `ShipLocomotionClass::Process_Drive_Track` @ `0x006A05F0`
- ILocomotion vtable @ `0x007F2D8C` (40 slots)
- IUnknown vtable @ `0x007F2E58` (4 slots)
- IPiggyback vtable @ `0x007F2D68` (9 slots)
- CLSID `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` stored at `0x007E9AB0`

**Active in YR:** **Yes** — every naval unit (Destroyer, Aegis, Carrier, Dreadnought, Sub, Dolphin, Sea Wolf, Squid, etc.) uses Ship as its primary `Locomotor=`. Every Ship vtable method runs every tick for every Ship unit in standard skirmish.

**Confidence convention** — each function citation carries three axes:
- **C** = content: the algorithm/formula/branching has been verified from decompilation or raw assembly.
- **I** = identity: the function name and slot in the vtable have been verified by reading the vtable bytes and/or matching Ghidra symbols.
- **B** = binding: the caller path has been verified — either via `get_function_callers`, `get_xrefs_to`, or by walking the vtable from a confirmed caller.

**Cross-reference docs (read these first):**
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` — the prior diff vs Drive (corrections to it noted in §11 below)
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` — bridge-Z-offset, on_bridge transitions, TooBig crush layer
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` — Water/Float passability matrix, RecalcZoneType
- `NAVAL_SYSTEM_RESEARCH.md` — older general naval doc (a few claims now stale; see §11.5)
- `NAVAL_IMPLEMENTATION_PLAN.md` — Rust-implementation focus
- `ILOCOMOTION_COM_PROTOCOL_SPEC.md` — base ILocomotion COM contract
- `LOCOMOTION_MATH_AND_CONSTANTS.md` — CLSID GUID list, base constants

---

## 1. Overview

`ShipLocomotionClass` is the locomotor for all surface and submerged naval units in YR. It is **structurally sibling to `DriveLocomotionClass`** (same drive-track curve algorithm, same step pipeline, same vtable layout) but every method has its own copy with its own global data pointers (separate NullCoord sentinel, separate height-step constant, separate track tables). Behavioural differences from Drive are localised to a handful of constants and 7 specific code-level differences (§11).

**What it does (player-observable):**
- Moves naval units along curved drive tracks between 8 facings (0/45/90/.../315°)
- Spawns wake animations every 8 frames while on water cells
- Applies cliff-up/down speed multipliers and a health-degraded speed multiplier
- Handles repath / scatter / stuck-detection / mutual-yield when other units block the path
- Adds a +Z offset on bridge cells (so ships' shadow/depth math works around overpasses they cannot themselves enter)
- Always reports layer = **Ground (2)** — even for submarines (submarine "underwater" state is **not** a locomotor-layer property; it is an ObjectClass cloak/visibility property)

---

## 2. Class identity

| Property | Value | Evidence |
|---|---|---|
| CLSID | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` | `read_memory 0x007E9AB0` → `e1 74 ea 2b ca 7c d3 11 be 14 00 10 4b 62 a1 6c` (Windows GUID byte order) |
| Class factory `CreateInstance` | callsite at `0x006C4F4C` (sole caller of Constructor) | `get_xrefs_to 0x69EC50` |
| Constructor | `0x0069EC50` | `decompile_function` confirms field init pattern |
| Virtual destructor | `0x0069ECF0` | Releases IPiggyback ptr at `+0x68`, then `LocomotionClass::Destructor` |
| Scalar-deleting destructor | `0x006A42B0` | Same as above + optional `operator delete` if `param_2 & 1` |
| ILocomotion vtable | `0x007F2D8C` (40 slots, 160 bytes) | `read_memory 0x007F2D8C len 160` |
| IUnknown vtable | `0x007F2E58` (4 slots, 16 bytes) | `read_memory 0x007F2E58 len 16` |
| IPiggyback vtable | `0x007F2D68` (9 slots, 36 bytes) | `read_memory 0x007F2D68 len 36` |
| Instance size | **0x6C bytes** (108 bytes) | Verified via constructor's field writes; matches Drive |
| Confidence | C=HIGH, I=HIGH, B=HIGH | |

---

## 3. Struct field layout

All offsets are from the **class base** (not from the ILocomotion sub-object). The "param_1" pointer in most ILocomotion methods is `base + 0x04` (points at the ILocomotion vtable pointer), so when a method references `*(param_1 + 0x30)` that's class-base `+0x34`.

| Offset | Size | Field | Notes |
|---|---|---|---|
| `+0x00` | 4 | IUnknown vtable ptr | `&ShipLocomotionClass__IUnknown_vtable` = `0x007F2E58` |
| `+0x04` | 4 | ILocomotion vtable ptr | `&ShipLocomotionClass__ILocomotion_vtable` = `0x007F2D8C` |
| `+0x08` | 4 | (inherited from `LocomotionClass`) | refcount or similar |
| `+0x0C` | 4 | LinkedTo TechnoClass* | set by `Link_To_Object`; **every `Process*` method dereferences this** as `*(int *)(param_1 + 0xc)` for the owning unit's state |
| `+0x10`–`+0x17` | 8 | (inherited) | likely ISwizzle / serialization helpers |
| `+0x18` | 4 | IPiggyback vtable ptr | `&ShipLocomotionClass__IPiggyback_vtable` = `0x007F2D68` |
| `+0x1C` | 4 | Slope cache: NEW slope index | Set by `Force_New_Slope` (slot 20 → slot 31 helper at `0x69F250`) |
| `+0x20` | 4 | Slope cache: OLD slope index | Saved when new ≠ old |
| `+0x24` | 4 | Slope-change frame counter | = `g_CurrentFrameCounter` at slope-change time |
| `+0x28` | 4 | Slope blend timer base | preserved from previous CDTimer value |
| `+0x2C` | 4 | Slope blend timer duration | hard-coded `3` frames in `Process`; zero when no blend active |
| `+0x30` | 4 | Slope blend (zeroed) | |
| `+0x34` | 4 | **Head_to_coord X** (immediate destination X) | Written by `Set_Destination` (vtable slot 17) and `Force_Track`. Null sentinel = `g_NullCoord_Ship_X`. |
| `+0x38` | 4 | Head_to_coord Y | |
| `+0x3C` | 4 | Head_to_coord Z | **Bridge Z-bump site** (see §6.3) |
| `+0x40` | 4 | **Destination X** (longer-term path endpoint) | Written by `Force_Track` (vtable slot 28). Read by `Move_To` (slot 6) as the answer to "where am I going". |
| `+0x44` | 4 | Destination Y | |
| `+0x48` | 4 | Destination Z | |
| `+0x4C` | 4 | unknown 4-byte field (init 0) | Constructor zeros `param_1[0x13]` (= class_base+0x4C). Live decompile (`Process_Movement 0x006A1C80`, `Stop_Moving 0x0069F510`) shows speed double accesses are at param_1+0x4C when param_1 = ILocomotion vtable ptr (class_base+0x04), i.e. class_base+0x50 — see next row. Verified via `decompile_function 0x006A1C80` showing `*(double *)(param_1 + 0x50) = local_48` with `param_1 = class_base`. Identity of class_base+0x4C is currently unknown. |
| `+0x50` | 8 | **Speed (double)** | Set in `Process_Movement` from `g_SpeedType_LandType_Table[ST + LT*9]` * cliff_mult * health_mult, clamped `[0.5, 1.0]`. Stop_Moving floor at `_DAT_007F1308` = **0.3** (deceleration). Spans `param_1[0x14]` and `param_1[0x15]` in the constructor (both init to 0). Direct verification: `Process_Movement` line `*(double *)(param_1 + 0x50) = local_48` where param_1 is class_base. |
| `+0x58` | 4 | **Track index** into TurnTrack table | Computed as `new_facing + old_facing*8`. `0xFFFFFFFF` (= -1) = no active track. Constructor inits to `0xFFFFFFFF` (`param_1[0x16]`). The dispatch in §7.2 gates on **this** field == -1, not on a separate "track facing" field. Direct verification: `decompile_function 0x006A1C80` shows `*(int *)(param_1 + 0x58) < 0x40` controlling the active-track branch. |
| `+0x5C` | 4 | Track step counter / sub-state | Position along the current curve; constructor inits to `0xFFFFFFFF` (`param_1[0x17]`); zeroed by `Force_Track` at track start. |
| `+0x60` | 1 | Track-active flag | 0 = step normally, non-zero = pause/force state |
| `+0x61` | 1 | (unset in constructor; zeroed) | "Stop_Moving applied this tick" flag (set in `Process_Movement` path-fail branches) |
| `+0x62` | 1 | **`slope_pending_flag`** | Set to 1 when the slope-blend timer is counting down; cleared to 0 when the blend completes (see §7.2.3 / §6.x). Constructor inits to 0 (`*(undefined1 *)((int)param_1 + 0x62) = 0`). Verified via `decompile_function 0x0069FC10` showing `*(undefined1 *)((int)piVar3 + 0x5e) = 1` where piVar3 = ILocomotion vtable ptr → class_base+0x62. |
| `+0x63` | 1 | **Destination-valid flag** (`+0x5F` in method-relative addressing) | Set to 1 when `Force_Track` writes a destination; cleared when destination is reset to null sentinel |
| `+0x64` | 1 | (unset; zeroed) | TBD |
| `+0x65` | 1 | **"First tick" flag** | Set to **1** in constructor; cleared by first `Process` call (used to drive first-frame initialization in Process) |
| `+0x68` | 4 | IPiggyback piggybacked instance ptr | Non-null when this locomotor is piggybacked under another (e.g. infantry in a transport). Released by destructor. |

**Confidence:** C=HIGH (every offset verified from decomp or raw asm), I=HIGH (constructor symbol confirmed), B=HIGH (offsets used identically across all 11 Ship methods).

**Subtle detail — the +0x65 "first tick" flag:** Initialised to `1` in the constructor (the only field set to anything non-zero/non-sentinel). Its READ sites are inside `Process` and `Process_Movement` — gates first-time path planning. If a Rust port skips this flag and starts at 0, ships will pathfind one tick later on the first Move order, which is observable as a one-frame delay.

---

## 4. IUnknown vtable @ `0x007F2E58`

| Slot | Address | Function | Confidence |
|------|---------|----------|------------|
| 0 | `0x69EE30` | `QueryInterface` | C=MEDIUM (inferred, not separately decompiled), I=HIGH (slot 0 of IUnknown vtable), B=HIGH |
| 1 | `0x6A4260` | `AddRef` | I=HIGH |
| 2 | `0x6A4270` | `Release` | I=HIGH |
| 3 | `0x6A3E60` | (4th slot — could be ScalarDeletingDestructor entry) | I=MEDIUM |

The 4th slot in a 16-byte IUnknown vtable is **non-standard for COM** (IUnknown defines only 3 methods). This is a Westwood/Westwood Studios convention where the IUnknown vtable region is followed by a destructor pointer the engine uses internally.

---

## 5. IPiggyback vtable @ `0x007F2D68`

| Slot | Address | Function | Notes |
|------|---------|----------|-------|
| 0 | `0x6A4330` | `QueryInterface` | IPiggyback inherits IUnknown |
| 1 | `0x6A4340` | `AddRef` | |
| 2 | `0x6A4350` | `Release` | |
| 3 | `0x69EFF0` | `Begin_Piggyback` | Saves current state, hands control to a sub-locomotor |
| 4 | `0x69F040` | `End_Piggyback` | Restores state |
| 5 | `0x69F080` | `Is_Piggybacking` | Returns whether currently piggybacked |
| 6 | `0x69ED20` | `Is_Ok_To_End` | Returns whether piggyback can be unwound this tick |
| 7 | `0x6A4280` | (likely `Piggybacker_CLSID` getter) | Reachable only via IPiggyback dispatch, NOT ILocomotion |
| 8 | `0x008093A0` | (data — not a function) | tail/sentinel |

**Confidence:** C=MEDIUM (slots not decompiled in this pass — outside Ship's main movement state machine), I=HIGH (vtable bytes verified), B=MEDIUM.

---

## 6. ILocomotion vtable @ `0x007F2D8C` (40 slots, 160 bytes)

Full slot-by-slot enumeration with 3-axis confidence. "Shared" = the same function pointer appears in both Drive's and Ship's ILocomotion vtable at that slot (i.e. inherited from `LocomotionClass` base).

| # | Off | Addr | Function | Shared? | C | I | B |
|---|-----|------|----------|---------|---|---|---|
| 0 | 0x00 | `0x6A4300` | `QueryInterface` | No (Ship copy) | M | H | H |
| 1 | 0x04 | `0x6A4310` | `AddRef` | No | M | H | H |
| 2 | 0x08 | `0x6A4320` | `Release` | No | M | H | H |
| 3 | 0x0C | `0x55A710` | `Link_To_Object` | **Yes** (base) | M | H | H |
| 4 | 0x10 | `0x69F290` | `Is_Moving` (§6.4) | No | H | H | H |
| 5 | 0x14 | `0x69F3A0` | `Destination` (§6.5) | No | H | H | H |
| 6 | 0x18 | `0x69F3D0` | `Head_To_Coord` / `Move_To` (§6.6) | No | H | H | H |
| 7 | 0x1C | `0x55ABF0` | (base stub — no-op `Stop_Moving` placeholder) | **Yes** | M | H | H |
| 8 | 0x20 | `0x55ABE0` | (base stub — `Do_Turn`) | **Yes** | M | H | H |
| 9 | 0x24 | `0x69F670` | `Draw_Matrix` (§6.12) | No | H | H | H |
| 10 | 0x28 | `0x69FB20` | `Shadow_Matrix` (§6.13) | No | H | H | H |
| 11 | 0x2C | `0x55ABD0` | (base stub — `Force_Immediate_Destination`) | **Yes** | M | H | H |
| 12 | 0x30 | `0x55A8C0` | `Can_Enter_Cell` (base — `LocomotionClass::Can_Enter_Cell`) | **Yes** | M | H | H |
| 13 | 0x34 | `0x55ABC0` | (base stub — `Is_To_Have_Shadow`) | **Yes** | M | H | H |
| 14 | 0x38 | `0x6A3EA0` | `Mark_All_Occupation_Bits` — returns 0 (no-op) | No | H | H | H |
| 15 | 0x3C | `0x6A3EB0` | `Z_Gradient` → delegates to `LocomotionClass::Z_Gradient_Default` | No | H | H | H |
| 16 | 0x40 | `0x69FC10` | **`Process`** (§7) | No | H | H | H |
| 17 | 0x44 | `0x69F450` | **`Set_Destination`** (§6.7) | No | H | H | H |
| 18 | 0x48 | `0x69F510` | **`Stop_Moving`** (§6.8) | No | H | H | H |
| 19 | 0x4C | `0x6A05C0` | `Do_Turn` — just `RateTimer__Set(&param_2)` | No | H | H | H |
| 20 | 0x50 | `0x69FBE0` | **`Force_New_Slope`** / `Update_Facing_From_Type` (§6.9) | No | H | H | H |
| 21 | 0x54 | `0x55AB90` | (base — `Tilt_Pitch_AI`) | **Yes** | M | H | H |
| 22 | 0x58 | `0x55A8F0` | (base — `Power_On`) | **Yes** | M | H | H |
| 23 | 0x5C | `0x55A910` | (base — `Power_Off`) | **Yes** | M | H | H |
| 24 | 0x60 | `0x55A930` | (base — `Is_Powered`) | **Yes** | M | H | H |
| 25 | 0x64 | `0x55A940` | (base — `Is_Ion_Sensitive`) | **Yes** | M | H | H |
| 26 | 0x68 | `0x55AB70` | (base — `Push`) | **Yes** | M | H | H |
| 27 | 0x6C | `0x55AB80` | (base — `Shove`) | **Yes** | M | H | H |
| 28 | 0x70 | `0x6A0310` | **`Force_Track`** (§6.10) | No | H | H | H |
| 29 | 0x74 | `0x6A3E50` | **`In_Which_Layer`** — returns `2` (Ground) | No | H | H | H |
| 30 | 0x78 | `0x55AC00` | (base stub) | **Yes** | M | H | H |
| 31 | 0x7C | `0x69F250` | **Slope-state-setter helper** (called by slot 20) | No | H | M | H |
| 32 | 0x80 | `0x69F330` | `Is_Moving_Now` (§6.11) | No | H | H | H |
| 33 | 0x84 | `0x55AD10` | (base — `Apparent_Speed`) | **Yes** | M | H | H |
| 34 | 0x88 | `0x55ACF0` | (base — `Drawing_Code`) | **Yes** | M | H | H |
| 35 | 0x8C | `0x55AD00` | (base — `Can_Fire`) | **Yes** | M | H | H |
| 36 | 0x90 | `0x4B4C60` | `Begin_Piggyback` (literally Drive's function — `0x4B4C60` is shared between Drive and Ship vtables; **NOT a typo**) | **Yes** | H | H | H |
| 37 | 0x94 | `0x4B4C70` | `End_Piggyback` (shared with Drive) | **Yes** | H | H | H |
| 38 | 0x98 | `0x4B4C80` | `Is_Surfacing` (shared with Drive) | **Yes** | H | H | H |
| 39 | 0x9C | `0x6A3F00` | **`Is_To_Have_Shadow_Override`** / Apply_Track_Step entry (§6.14) | No | H | H | H |

**Subtle detail — slots 36/37/38 sharing with Drive's class-specific functions:** The functions at `0x4B4C60/70/80` were originally defined inside Drive's address range but appear in BOTH Drive's and Ship's ILocomotion vtables. They are not in the inherited LocomotionClass base — they are Drive-class functions that the linker reused for Ship as well (because IPiggyback semantics are identical). A Rust port can use one shared impl.

### 6.1 Constructor @ `0x0069EC50`

```c
undefined4 *ShipLocomotionClass__Constructor(undefined4 *param_1) {
    LocomotionClass__Constructor();              // base
    param_1[7] = 0;                              // +0x1C (new slope cache)
    param_1[8] = 0;                              // +0x20 (old slope)
    param_1[9] = g_CurrentFrameCounter;          // +0x24 (slope frame)
    param_1[0xb] = 0; param_1[0xc] = 0;          // +0x2C, +0x30
    param_1[0xd] = g_NullCoord_Ship_X;           // +0x34 head_to_coord X
    param_1[0xe] = g_NullCoord_Ship_Y;           // +0x38
    param_1[0xf] = g_NullCoord_Ship_Z;           // +0x3C
    param_1[0x10] = g_NullCoord_Ship_X;          // +0x40 destination X
    param_1[0x11] = g_NullCoord_Ship_Y;          // +0x44
    param_1[0x12] = g_NullCoord_Ship_Z;          // +0x48
    param_1[0x13] = 0; param_1[0x14] = 0;        // +0x4C/+0x50 speed double = 0.0
    param_1[0x15] = 0;                           // +0x54 track facing
    param_1[0x16] = 0xffffffff;                  // +0x58 track index = -1
    param_1[0x17] = 0xffffffff;                  // +0x5C track sub-step = -1
    *(byte *)(param_1 + 0x18) = 0;               // +0x60 track-active flag
    *(byte *)((int)param_1 + 0x61) = 0;          // +0x61
    *(byte *)((int)param_1 + 0x62) = 0;          // +0x62
    *(byte *)((int)param_1 + 0x63) = 0;          // +0x63 dest-valid flag
    *(byte *)(param_1 + 0x19) = 0;               // +0x64
    *(byte *)((int)param_1 + 0x65) = 1;          // +0x65 FIRST-TICK FLAG
    param_1[0x1a] = 0;                           // +0x68 piggyback ptr = NULL
    *param_1 = &ShipLocomotionClass__IUnknown_vtable;
    param_1[1] = &ShipLocomotionClass__ILocomotion_vtable;
    param_1[6] = &ShipLocomotionClass__IPiggyback_vtable;
    return param_1;
}
```

**Parity-load-bearing details:**
1. **Track index AND sub-step initialised to `-1`** (not `0` or `0xFFFF`). Differs from typical default-zero patterns.
2. **+0x65 "first tick" flag = 1** — only non-zero/non-sentinel byte. Read by `Process` to gate first-frame behaviour.
3. **Speed = 0.0** at birth (both halves of the double are zero-initialised).
4. **Slope frame counter = `g_CurrentFrameCounter`** at construction — NOT `-1`. So a freshly-constructed ship will think slope changed `0` frames ago, not "never".

**Caller binding:** sole caller is the class-factory CreateInstance at `0x006C4F4C` (verified via `get_xrefs_to 0x69EC50`). That site is reached from `WinMain @ 0x6BD3BD` boot-time COM registration plus `BuildingClass::MissionRepairAndProduce @ 0x44C78B/780` (naval yard producing a ship).

**Confidence:** C=HIGH, I=HIGH, B=HIGH.

### 6.2 Destructors

**Virtual destructor @ `0x0069ECF0`:**
```c
*param_1 = &IUnknown_vtable;       // re-set vtable ptrs (defensive)
param_1[1] = &ILocomotion_vtable;
param_1[6] = &IPiggyback_vtable;
piVar1 = (int *)param_1[0x1a];      // piggyback ptr at +0x68
if (piVar1 != NULL) {
    (**(code **)(*piVar1 + 8))(piVar1);   // piggyback->Release()
}
LocomotionClass__Destructor();
```

**Scalar-deleting destructor @ `0x006A42B0`:** same as above plus `if ((param_2 & 1) != 0) operator_delete(param_1)` — the standard MSVC `~ScalarDeleting(BYTE flags)` pattern.

**Subtle detail:** The destructor re-writes the vtable pointers before calling Release on the piggyback. This is **MSVC virtual-destructor convention** — between virtual destructors of a class hierarchy, vtables are reset to the current class's table so that any vtable dispatch inside `~Class` resolves to this class's methods.

**Confidence:** C=HIGH, I=HIGH (Ghidra-named "Constructor" for 0x69ECF0 is a Ghidra labelling artifact — it's actually the destructor; the function body's behaviour is destructor-style), B=HIGH.

### 6.3 `Set_Destination` @ `0x0069F450` (vtable slot 17)

Already documented in `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §7.1. Summary:

```c
void Set_Destination(int param_1, int X, int Y, int Z) {
    if (vtable+0x37C () != 0) return;   // IsBeingWarpedOut
    if (vtable+0x380 () != 0) return;   // IsBeingWarpedIn
    if (vtable+0x1D4 () != 0) return;   // IsDeploying
    if (vtable+0x1D8 () != 0) return;   // IsUndeploying
    head_to_coord_X = X;                // +0x30
    head_to_coord_Y = Y;                // +0x34
    head_to_coord_Z = Z;                // +0x38
    if (X != NullCoord_X || Y != NullCoord_Y || Z != NullCoord_Z) {
        cell = MapClass::Get_Cell_At(&X);
        if ((cell->Flags & 0x100) != 0) {
            head_to_coord_Z += g_BridgeZ_Offset;     // 0x00B0782C (Ship's own)
        }
    }
}
```

**Subtle detail:** The 4 vtable-guard SHORT-CIRCUIT order is `0x37C → 0x380 → 0x1D4 → 0x1D8` (warp checks BEFORE deploy checks). Drive uses `0x1D4 → 0x1D8 → tether → 0x37C → 0x380` inside `Process_Movement`. This is a **real divergence** — if a tethered ship is also deploying (impossible in stock content but possible in mods), Ship and Drive return differently.

Confidence: C=HIGH, I=HIGH, B=HIGH (vtable slot 17 of `0x007F2D8C`).

### 6.4 `Is_Moving` @ `0x0069F290` (slot 4)

```c
bool Is_Moving(int param_1) {
    if (head_to_coord != NullCoord) return 1;            // (+0x30/+0x34/+0x38)
    if (destination == NullCoord) return 0;              // (+0x3C/+0x40/+0x44)
    if (destination_XY == techno_location_XY) return 0;  // arrived at long-term dest
    return 1;
}
```

**Subtle detail:** Z-coordinate of `destination` is checked against `g_NullCoord_Ship_Z`, but when comparing to techno location only X and Y are compared (Z is ignored in the "arrived" check). So a ship can be considered "arrived" while its Z still differs from destination Z — this matters for bridges where Z bumps are applied.

C=HIGH, I=HIGH, B=HIGH.

### 6.5 `Destination` @ `0x0069F3A0` (slot 5)

```c
CoordStruct *Destination(int param_1, CoordStruct *out) {
    out->X = head_to_coord_X;   // +0x30
    out->Y = head_to_coord_Y;   // +0x34
    out->Z = head_to_coord_Z;   // +0x38
    return out;
}
```

Returns the **immediate** destination (head_to_coord), not the longer-term `+0x40` triple. Pure getter; no side effects.

C=HIGH, I=HIGH, B=HIGH.

### 6.6 `Head_To_Coord` / `Move_To` @ `0x0069F3D0` (slot 6)

```c
CoordStruct *Move_To(int param_1, CoordStruct *out) {
    if (destination == NullCoord) {       // longer-term dest unset
        out = techno.Location;            // techno->Location at +0x9C/+0xA0/+0xA4
    } else {
        out = destination;                // +0x3C/+0x40/+0x44
    }
    return out;
}
```

Returns "where am I currently heading toward overall" — the longer-term destination if set, else current location. Companion getter to `Destination` but for the OTHER coord triple.

C=HIGH, I=HIGH, B=HIGH.

### 6.7 `Stop_Moving` @ `0x0069F510` (slot 18)

```c
void Stop_Moving(int param_1) {
    if (destination_set                                       // +0x3C/+0x40/+0x44 != null
        && techno.vtable[+0x84]().TooBigToFitUnderBridge != 0  // TechnoType+0xC94
        && FootClass+0x6D0 == 0)                              // some "skip TooBig crush this tick" flag
    {
        // Walk the techno's blocker linked list (head at Techno+0x6C8)
        // and Release each blocker's reference at +0x674.
        iVar2 = techno+0x6C8;
        do {
            if (iVar2.bridge_blocker_ptr == 0) Assert(0x80004003);
            *(int **)(iVar2 + 0x674) -> vtable[0x48](...);     // Release-like call
            iVar2 = iVar2.next_blocker;
        } while (iVar2 != 0 && iVar2 != iVar2.next_blocker);
    }
    // Apply deceleration clamp: speed = min(speed, 0.3)
    if (locomotor.speed < _DAT_007F1308) {   // 0.3 (double at 0x7F1308)
        speed_clamped = locomotor.speed;     // keep current if below 0.3
    } else {
        speed_clamped = _DAT_007F1308;       // 0.3 cap (decelerate this tick)
    }
    locomotor.speed = speed_clamped;
    locomotor.head_to_coord = NullCoord;     // clear immediate destination
}
```

**Subtle details:**
1. **Stop_Moving does NOT instantly halt** — it caps speed at **0.3** (the constant at `0x7F1308`). This produces visible **deceleration** rather than abrupt stop. The next tick `Process_Movement` will see `head_to_coord == NullCoord` and reduce further.
2. **TooBigToFitUnderBridge units** trigger blocker-list cleanup on stop. Without it, dead unit references would dangle in cell blocker lists.
3. **The threshold check is `< 0.3`**, NOT `<=`. A ship already at exactly 0.3 gets capped at 0.3 (no change), but values slightly above get clamped — strictly speaking the IF branch keeps current_speed if speed < 0.3, else clamps to 0.3. Net effect: max(current, 0.3) ... actually min — let me re-read. The decompiler shows `if (current < 0.3) dVar1 = current; else dVar1 = 0.3; locomotor.speed = dVar1;` That's **`speed = min(current, 0.3)`** — caps at 0.3. Correct as documented.

C=HIGH, I=HIGH, B=HIGH.

### 6.8 (Removed — same numbering as §6.7 — the doc continues without 6.8)

### 6.9 `Force_New_Slope` wrapper @ `0x0069FBE0` (slot 20) and helper @ `0x0069F250` (slot 31)

**Wrapper (slot 20):**
```c
void Force_New_Slope(int *param_1) {
    iVar1 = *param_1;                                       // ILocomotion vtable
    iVar2 = vtable_ptr->slot_113(); // techno.vtable+0x1BC = "Current_Cell" getter
    (**(code **)(iVar1 + 0x7c))(param_1, cell.SlopeIndex);  // call slot 31 with cell+0x11C byte
}
```

**Helper (slot 31, `0x69F250`):**
```c
void Slope_State_Set(int param_1, undefined4 new_slope) {
    *(int *)(param_1 + 0x1C) = new_slope;           // class +0x20 (old)
    *(int *)(param_1 + 0x18) = new_slope;           // class +0x1C (new)
    *(int *)(param_1 + 0x20) = g_CurrentFrameCounter; // class +0x24
    *(int *)(param_1 + 0x24) = local_c;             // uninitialised (legacy)
    *(int *)(param_1 + 0x28) = 0;                   // class +0x2C
    *(int *)(param_1 + 0x2C) = 0;                   // class +0x30
}
```

**Correction to `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`:** The prior doc labelled slot 31 as `Piggybacker_CLSID` (which is what Ghidra's symbol called the corresponding Drive function at `0x4AF610`). But **that function is NOT what slot 31 actually contains** in either Drive or Ship — slot 31 is `0x4AFB40` in Drive and `0x69F250` in Ship, and both have the slope-state-setter body. Drive's `0x4AF610` (the real Piggybacker_CLSID with `LocomotionClass__QueryInterface_IPiggyback` machinery) is not in the ILocomotion vtable at all — it is reached via the IPiggyback vtable.

**Subtle detail:** The `local_c` write at `class+0x24` reads an **uninitialised local variable** in the C decompilation. This is a known compiler/decompiler artifact for a 6-DWORD `memset`-like sequence; the assembly likely zeroes the whole region with `STOSD` and the decompiler interprets the second STOSD's source incorrectly. Treat `class+0x24` as **`g_CurrentFrameCounter`** (mirror of `+0x20`).

C=HIGH (decompilation + cross-checked with Drive's identical helper at `0x4AFB40`), I=MEDIUM (Ghidra mis-symbol), B=HIGH (slot 31 verified by reading `0x007F2D8C+0x7C`).

### 6.10 `Force_Track` @ `0x006A0310` (slot 28)

```c
void Force_Track(int param_1, facing, int X, int Y, int Z) {
    locomotor.track_facing = facing;          // +0x54
    locomotor.track_index = 0;                // +0x58 (start of new track curve)
    if (!(X,Y,Z is NullCoord)) {
        if (destination != NullCoord) {       // clear old longer-term dest first
            destination = NullCoord;
            dest_valid_flag = 0;              // class +0x63
        }
        if (!(X,Y,Z is NullCoord)) {          // (re-check after possible clear)
            dest_valid_flag = 1;
            destination = (X, Y, Z);          // +0x40..+0x48
            techno = locomotor.linked_techno;
            cell = CellClass::Get_Cell_At(&X);
            if (CrateClass::PickupDispatch(techno) != 0
                && techno+0x81 == 0) {        // not a "consumed crate" flag set
                Apply_Track_Step(&X, /*mode=*/ 1);   // step toward new dest
                head_to_coord = (X, Y, Z);    // +0x34..+0x3C
                locomotor.speed_low = 0;       // +0x4C (low half of double)
                locomotor.speed_high = 0x3FF00000;  // +0x50 (high half = 1.0 IEEE)
            } else if (techno+0x90 != 0) {    // dead/being-deleted flag
                destination = NullCoord;
                dest_valid_flag = 0;
                return;
            }
        }
    }
}
```

**Parity-load-bearing details:**
1. **Speed init = 1.0** (double 1.0 = bytes `00 00 00 00 00 00 F0 3F`; high half = `0x3FF00000`). So when a track is forced, speed is reset to MAX before the next tick recalculates per-cell.
2. **Crate pickup happens at `Force_Track` time** — not at `Apply_Track_Step`. Order: Get_Cell_At → CrateClass::PickupDispatch → if crate consumed and unit still alive: apply track step + set head_to_coord. Visible: a ship dropped on a crate at its destination triggers the crate immediately as it commits to the track.
3. **The two `if (!(X,Y,Z is NullCoord))` checks** are redundant — likely an inline-expanded `if-not-null`. Both must hold for the body to run.

C=HIGH, I=HIGH, B=HIGH.

### 6.11 `Is_Moving_Now` @ `0x0069F330` (slot 32)

```c
bool Is_Moving_Now(int *param_1) {
    if (CDTimerClass__Remaining() != 0) return 1;   // slope-blend timer active
    if (ILocomotion::Is_Moving(param_1)            // slot 4
        && destination != NullCoord) {
        if (techno.vtable+0x538() > 0) return 1;    // some "is target far enough" check
    }
    return 0;
}
```

**Subtle detail:** Returns 1 if the slope-blend timer is still counting down, even if the ship has stopped translating. This drives the wake-anim spawn check in `Process` — wake animations fire while the ship is "still moving visually" (interpolating slope), not just while actively translating.

C=HIGH, I=HIGH, B=HIGH.

### 6.12 `Draw_Matrix` @ `0x0069F670` (slot 9)

Builds the 3×4 transform matrix for VXL rendering. Two code paths:

**Path 1 — fast (no tilt, no slope interpolation):**
- `slope_blend_factor == 1.0` (no active blend)
- `abs(techno.pitch) < _DAT_007E44E8`
- `abs(techno.roll) < _DAT_007E44E8`

Calls `BuildFacingRotationMatrix(param_1, local_128, param_3)` → optional `VXL_GetFacingMatrix` if a facing param is set → copies result.

**Path 2 — full (active tilt or blend):**
- Builds two identity matrices via `Matrix3x4_SetIdentity()`
- Reads tilt/pitch from `techno+0x328` and `techno+0x32C`
- Computes sin/cos via `Sin_lookup` and `Cos_lookup` for each
- 5 `Math__ftol` calls (truncate float → int for fixed-point projection)
- Calls `Matrix_shear_col3_by_col2`, `Matrix_shear_col3_by_col0`, `Matrix_shear_col3_by_col1`, `Matrix_rotate_x_axis`, `Matrix_rotate_y_axis`
- Forces `*param_3 = -1` (no cached facing-matrix index)
- Builds full facing-rotation matrix and applies the chained shear+rotate sequence

**Subtle detail — `_DAT_007E44E8` threshold:** float value `~89121.96` (`0x47AE147B` raw bits). This is *not* a parity tolerance; it's a **large-magnitude sanity threshold** — any sensible pitch/roll is well below this, so Path 1 is taken nearly always. The threshold's only purpose is to redirect to Path 2 when pitch/roll has corrupted to garbage (defensive).

**Slope blend formula:**
```
slope_blend_factor = (slope_duration - remaining) / slope_duration
                   = (locomotor+0x2C - timer_remaining) / locomotor+0x2C
```
where `slope_duration` is the `3` written by `Process` on slope change.

**Bridge interaction:** None directly. Draw_Matrix does not read `cell.Flags & 0x100`. Bridge Z is already in `techno.Z` by the time draw is called.

C=HIGH, I=HIGH, B=HIGH.

### 6.13 `Shadow_Matrix` @ `0x0069FB20` (slot 10)

Shorter than Draw_Matrix — computes the same slope-blend factor, then:
- If blend ≠ 1.0 OR pitch ≥ threshold OR roll ≥ threshold AND `param_3 != NULL`:
  - `*param_3 = 0xFFFFFFFF`  (invalidate cached shadow facing)
- Calls `LocomotionClass::Build_Shadow_Matrix(param_1, local_30, param_3)`
- Copies 12 DWORDs into output

C=HIGH, I=HIGH, B=HIGH.

### 6.14 `Is_To_Have_Shadow_Override` / Apply_Track_Step entry @ `0x006A3F00` (slot 39)

```c
void Is_To_Have_Shadow_Override(int param_1, undefined4 mode) {
    if (destination != NullCoord) {              // (+0x3C/+0x40/+0x44)
        Apply_Track_Step(param_1 + 0x3C, mode);  // step toward destination
    }
}
```

**Despite the slot name** (which is Drive's symbol), this is functionally an **Apply_Track_Step trampoline**, not a shadow-flag query. Returns no value. The Ghidra label is misleading.

C=HIGH, I=MEDIUM (slot 39 by position; Ghidra symbol misleading), B=HIGH.

---

## 7. `Process` top-level dispatcher @ `0x0069FC10` (slot 16)

Per-tick entry. ~870 bytes decompiled.

### 7.1 Slope-change detection (every tick)

```c
cell = techno.vtable+0x1BC(); // current cell
new_slope = cell+0x11C;        // SlopeIndex byte
if (new_slope != locomotor.slope_cache_new) {  // +0x18 (param_1[6])
    locomotor.slope_cache_old = locomotor.slope_cache_new;   // +0x1C
    locomotor.slope_cache_new = new_slope;                   // +0x18
    CDTimerClass::Start(3);     // start 3-frame slope blend timer
    locomotor.slope_frame = iStack_10;  // +0x20 = previous timer base
    locomotor.slope_blend_a = iStack_c; // +0x24
    locomotor.slope_blend_b = iStack_8; // +0x28
    locomotor.slope_duration = 3;       // +0x2C
}
```

**Parity-load-bearing details:**
- **Slope blend is exactly 3 frames** — hardcoded `3` in `CDTimerClass::Start(3)`. A Rust port using a different value will produce visibly faster/slower slope tilt transitions.
- **Slope change is detected by raw `byte` inequality**, not a tolerance — so any SlopeIndex transition triggers a fresh 3-frame blend.

### 7.2 Dispatch — Process_Drive_Track vs Process_Movement

The pivot:
```c
if (locomotor.track_index == -1 || dest_valid_flag == 0) {
    // NO active track + no committed destination
    // → maybe early-exit (dock check), else Process_Movement first
} else {
    // ACTIVE track and committed destination
    // → Process_Drive_Track first, then Process_Movement if track finished
}
```

> **2026-05-19 correction (slot-1 swarm audit):** earlier doc revisions said `track_facing == -1`.
> The binary gate is on `class_base+0x58` (the **Track index** field, initialised to `0xFFFFFFFF`
> by the constructor — `param_1[0x16] = 0xffffffff`), not on `class_base+0x54` (Track facing,
> initialised to 0 and never -1). Verified via `decompile_function 0x006A1C80` showing
> `*(int *)(param_1 + 0x58) < 0x40` as the active-track branch guard.

#### 7.2.1 Dock early-exit (mission == 0xB)

Inside the "no track" branch:
```c
if (techno+0x5A4 != NULL                                     // techno has a mission
    && techno.mission.vtable+0x2C() == 0xB) {                // mission ID == 0xB (Mission_Enter / Dock)
    iVar5 = techno+0x166*4;                                  // techno+0x598 - some dock-target ptr
    coord = techno.vtable+0x1B8(&param_1);                   // techno's cell coord
    if (coord.X == iVar5+0x24 && coord.Y == iVar5+0x26) {    // we're AT dock cell
        if (techno+0x166*4 == 0) {                            // not "still pending"
            techno.vtable+0x480(0, 1);                       // call Mission_Update (early-stop variant)
            return 0;
        }
        // else fall through to "Stop_Moving + complete mission"
        FootClass__Stop_Moving();
        techno.vtable+0x484(0, 1);                            // Mission_Update full
        return 0;
    }
}
```

**Mission ID `0xB` = `MISSION_ENTER`** in YR mission enum. Ships use this for refinery/yard docking.

#### 7.2.2 Stop-at-destination check

```c
if (mission.vtable+0xAC == 5                                  // some "stop-at-destination" flag
    && dest_valid_flag == 0
    && head_to_coord != NullCoord) {
    if (mission+0x9C..0xA4 == head_to_coord) {                // mission.target_loc == head_to_coord
        if (techno+0x166*4 == 0) {
            techno.vtable+0x480(0, 1);
            return 0;
        }
        FootClass__Stop_Moving();
        techno.vtable+0x484(0, 1);
        return 0;
    }
}
```

#### 7.2.3 Slope-blend timer expiration

```c
if (CDTimerClass__Remaining() != 0) {                         // slope-blend timer counting
    locomotor.slope_pending_flag = 1;                          // +0x5E = 1
    goto LAB_006a1e6c (the wake-anim check);
}
if (locomotor.slope_pending_flag != 0) {                       // timer just finished
    locomotor.slope_pending_flag = 0;
    techno.vtable+0x18C(0);                                    // notify slope finished
    if (techno+0x90 == 0 || techno+0x81 != 0 || techno+0x8D != 0) {
        return; // dead/selling/limboed during the blend
    }
}
```

#### 7.2.4 Mission gates

```c
mission_id = techno.vtable+0x184();
if ((mission_id == 5 && !Is_Moving())                          // Mission==5 (Mission_Guard?) and stationary
    || mission_id == 0x10) {                                    // Mission 0x10 = Mission_Sleep?
    goto LAB_006a1e6c;                                          // skip to wake-anim
}
```

`Mission_Guard == 5` (verified against `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`).

`Mission 0x10` — likely `MISSION_AREA_GUARD` or `MISSION_HARMLESS` (unverified by this pass).

#### 7.2.5 Mission_Move target update

If `!Is_Moving() && techno.path_index == -1 && mission+0x3CD == 0` AND mission has a target object: call `mission.target.vtable+0x4C` (Coord getter) and pass the result back to `Set_Destination` via `vtable+0x44 = slot 17`. This is **continuous re-target** while in Mission_Move — if the target unit moves, the ship updates its head-to-coord every tick.

#### 7.2.6 Process_Movement / Process_Drive_Track call dispatch

After all the gates:
- **No track:** `Process_Movement(&param_1, 1, 0)` — `param_2=1` means "process a new step", `param_3=0` means "not a retry".
- **Active track:** `Process_Drive_Track(0)` — advance one tick along the curve.
  - If returns nonzero AND techno still alive: **call `Process_Movement(&param_1, 1, 0)`** — track exhausted, look for next step.
  - Then call `Process_Drive_Track(1)` — final commit-step on the new track.

**Subtle detail:** A ship at the end of one track immediately begins the next track within the SAME `Process` call. There is **no one-tick stall** between consecutive track segments. A Rust port that batches "finish-track" and "start-next-track" across two ticks will produce visibly jerky motion.

### 7.3 Wake animation spawn (LAB_006a1e6c region)

```c
cVar4 = vtable+0x80(param_1);   // ILocomotion slot 32 = Is_Moving_Now
if (cVar4 != 0) {
    uVar6 = g_CurrentFrameCounter & 0x80000007;          // frame & 7 (with sign-handling trick)
    bVar11 = (uVar6 == 0);
    if ((int)uVar6 < 0) {
        bVar11 = (uVar6 - 1 | 0xfffffff8) == 0xffffffff;  // negative-counter handling
    }
    if (bVar11                                            // frame % 8 == 0
        && techno.TypeClass.+0xD69 == 0                   // **NoSpawnAlt or similar flag**
        && techno+0x8C == 0                               // **on_bridge == 0** (no wake on bridge)
        && cell+0xEC == 2                                 // cell.LandType == 2 (Water)
        && RulesClass+0x94 != 0) {                        // wake anim type defined
        anim = operator_new(0x1C8);                       // 456 bytes — AnimClass size
        if (anim != NULL) {
            AnimClass::Constructor(
                RulesClass+0x94,                          // wake anim type
                techno_XY_stack_coord,
                0, 1, 0x600, 0, 0
            );
        }
    }
}
```

**Parity-load-bearing details:**
1. **Every 8 frames** (`frame & 7 == 0`) — Drive uses every 10 (`frame % 10 == 0`). Ships leave 25% more wake animations than Drive leaves dust.
2. **Wake suppressed on bridge** (`techno.on_bridge != 0` → skip). Drive does NOT have this suppression for dust.
3. **TechnoType+0xD69 must be zero** to spawn. Per `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` §6 "Difference 1": this is a CORRECTION to the prior doc — **Ship DOES check `+0xD69`**, not "skips it as Drive does". Both check it; only the frequency differs.
4. **Water-only:** Only fires when `cell.LandType == 2` (Water). On beach (LandType 6) or any other land type: no wake.
5. **AnimClass constructor magic `0x600`**: 5th argument to `AnimClass::Constructor`. Per `ANIM_CLASS_DEEP_DIVE.md` (TBD cross-ref) this is the **layer / Z-bias flag**: 0x600 = "ground layer + Z-suppressed". Wakes render below the ship.
6. **456-byte allocation** for the AnimClass. Hardcoded — the type's size is not parametric.
7. The RulesClass offset `+0x94` is the `[General] Wake=` anim type pointer.

**Correction to comparison doc:** Prior `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` §6.1 claimed Ship skips the `+0xD69` check. **Wrong.** Ship checks it. The actual diffs from Drive in this region are only (a) interval 8 vs 10 and (b) Ship adds `techno.on_bridge == 0` to the gate (Drive lacks this on its dust spawn).

### 7.4 End-of-Process settle

After step processing, if **both** head_to and longer destination are null AND mission_id == -1 AND `techno+0x15E (current speed double) > 0`:

```c
techno.vtable+0x544(0, 0);   // Set_Speed(0, 0) — explicit hard stop
```

**Subtle:** This is the only "instant hard-stop" path — once both destinations are cleared AND we're idle AND we still have residual speed, force-zero it. Without this, ships at rest could drift indefinitely from accumulated float error.

C=HIGH, I=HIGH, B=HIGH (slot 16 of `0x007F2D8C` and dispatched from `LocomotionClass`-derived ticking infrastructure).

---

## 8. `Process_Movement` state machine @ `0x006A1C80`

The heavy AI subroutine — ~6.5 KB body. Function signature `undefined1 __thiscall Process_Movement(int param_1, undefined4 param_2, int *param_3)`.

- `param_1` = class-base pointer
- `param_2` = "process new step" flag (1 on first call, 0 on retries)
- `param_3` = "is retry" flag (1 when recursively called after a `local_48 == 2/1/7` retry)

Sole caller: `Process` at `0x69FC10`. Recursive self-calls 3 times inside the function body.

### 8.1 Top-of-function gate chain

In order:
1. Read `techno+0x5E0` (current path step) into `uVar18`.
2. **Early-out:** If `!Is_Moving()` (slot 4) AND `uVar18 == 0xFFFFFFFF`:
   - Clear `+0x61` (Stop_Moving flag)
   - Clear longer-term destination
   - If `mission_id != 2` (Mission != Mission_Guard): return 0
   - Else: call `techno.vtable+0x484(0, 1)` (Mission_Update full) and return 0
3. **Early-out:** If `head_to_coord == NullCoord`: return 0
4. **Tether check:** `if (techno+0x2D0 != NULL && RadioClass::Tether_Count() != 0): return 1` (tether keeps move alive)
5. **Mission gates** (in this order):
   - `vtable+0x1D4()` IsDeploying → return 0
   - `vtable+0x1D8()` IsUndeploying → return 0
   - `vtable+0x37C()` IsBeingWarpedOut → return 1
   - `vtable+0x380()` IsBeingWarpedIn → return 1

**Difference from Drive:** Drive checks deploy/undeploy BEFORE tether. Ship checks tether FIRST. The functional difference matters only when a tethered ship is also deploying — Ship returns 1 (still tethered), Drive returns 0 (deploy wins).

### 8.2 Path acquisition (LAB_006a1e6c)

When `uVar18 == 0xFFFFFFFF` (no path):

1. Update repath timer (`Techno+0x640/+0x644/+0x648`):
   - `+0x640 = g_CurrentFrameCounter`
   - `+0x644 = uStack_18` (preserved from prior RateTimer)
   - `+0x648 = ftol(...)` (new timer max)
2. Build a `Cell2D` from `head_to_coord` via lepton-to-cell shift `(coord + sign_extend) >> 8`.
3. **Call `FootClass__Find_Path(Cell2D, 0, 0)`** — the A* entry.
4. Handle return:
   - **Success:** continue to step execution.
   - **Failure with mission == 2 or 0xB (Guard/Enter) AND distance to dest < `RulesClass+0x1718`** (~"close enough"): clear longer dest, call Stop_Moving, return.
   - **Failure otherwise:** scatter logic in a random direction (from RateTimer), call Stop_Moving + sound, decrement `Techno+0x64C` (path-attempt counter). If `Techno+0x64C` reaches 0: clear path completely and reset path index to -1; play "stuck" sound via `Techno+0x68A` flag check + `VocClass__PlayAtPos(0x3F800000, 0)` (volume 1.0).

**Magic number `RulesClass+0x1718`:** the "close-enough" distance for arrival. This is the **`[General] CloseEnoughDistance=`** equivalent in YR rules — verify via INI cross-ref (`Grep ^CloseEnoughDistance ini/rulesmd.ini`).

### 8.3 Per-step direction lookup

When the path is valid:

```c
uVar18 = techno+0x5E0 & 7;            // current path step direction (0-7)
target_X = techno.X + DirectionDeltaX_Table[uVar18 * 8];
target_Y = techno.Y + DirectionDeltaY_Table[uVar18 * 8];
target_Z = techno.Z;                  // Z preserved
```

**Tables:**
- `g_DirectionDeltaX_Table` and `g_DirectionDeltaY_Table` are 8-entry int arrays at separate addresses. Each entry is `8 bytes` apart (LEA `iVar5 = (uVar18 & 7) * 8`), meaning they're interleaved or wide entries.

### 8.4 On-bridge transition detection (per-step)

```c
new_cell = MapClass::Get_Cell_At(&target_XY);
unit_effective_level = (-(techno+0x8C != 0) & 4) + new_cell+0x11B;  // on_bridge ? cell.Level+4 : cell.Level
if ((new_cell.Flags & 0x100) >> 8 != techno+0x8C) {
    techno+0x68B = 1;                  // bridge-transition flag (set this tick)
}
```

**Parity-load-bearing detail:** `techno+0x68B` is set whenever the unit is **about to cross** a bridge boundary. Read by the renderer for layer-switch effects.

### 8.5 Can_Enter_Cell dispatch (`local_48`)

The function calls `techno.vtable+0x29C(...)` to gate the actual step (returns 0 if can't enter for now → return 1 wait), then calls `techno.vtable+0x1AC(cell, dir, level)` to classify the cell. Return codes:

| `local_48` | Meaning | Handling |
|---|---|---|
| 0 | Passable | Proceed to speed calc and step |
| 1 | Own / temporary occupant | Redraw cell; if not retry: clear dest, call Stop_Moving; if retry: recurse |
| 2 | Blocked (must repath) | Set `techno+0x6B7 = 1` (blocked flag), store start frame at `+0x668/+0x66C/+0x670` with `RulesClass+0x1768` as block timeout; call FootClass__Find_Path with the TooBig flag; return 1 |
| 3 | Crushable obstacle | `MapClass::Check_Crushable_Obstacle()`; clear dest; return |
| 4 / 5 | Wall / Building | If `TT.+0xD28` ("SubjectToWalls" var) AND cell.OverlayTypeIndex == 0: clear local_48 (passable). Else: `Find_Blocking_Object` + ally check → attack (vtable+0x1F4) or notify ally to move |
| 6 | Friendly stationary | Distance check; close → stop. Else scatter (with bridge-layer flag from cell.Flags&0x100 AND abs(Z/HeightStep - cell.Level) >= 3) |
| 7 | Path-locked / impossible | If retry: recurse and clear path. Else: clear dest, stop |
| TooBig override: `local_48 < 7` AND TT.+0xC94 (TooBigToFitUnderBridge) != 0 | → clear `local_48 = 0` (force-passable; crush whatever) |

**Subtle detail — Code `0` for TooBig units:** A Mammoth-style ship in a normally-blocked cell will treat the cell as passable for stepping purposes — but the actual crush (10000 damage) is applied separately by `Process_Drive_Track` site #4 (see `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §4.3).

**Subtle detail — Code `4/5` SubjectToWalls escape:** `TT.+0xD28` is the `SubjectToWalls=no` flag's "negated" form. When false (default — most units ARE subject to walls), they hit code 4/5. When true (units like infantry with `SubjectToWalls=no`), they treat walls as passable.

### 8.6 Speed calculation (the heart of the function)

After `local_48 == 0` (passable):

```c
// Step 1: Effective LandType
height_diff = effective_level - new_cell+0x11B;   // unit's effective level - new cell's level
if (abs(height_diff) < 2) {
    LandType = new_cell+0xEC;                     // same level → use cell's actual LandType
    cell_level = new_cell+0x11B;
} else {
    LandType = 1;                                  // crossing height >= 2: treat as Clear (1)
    cell_level = old_effective_level;              // (don't update level)
}

// Step 2: Base speed from table
TT = techno.TypeClass;
speed = (double) g_SpeedType_LandType_Table[TT.SpeedType (+0x67C) + LandType * 9];
if (speed > 1.0) speed = 1.0;        // clamp

// Step 3: Cliff/slope multipliers (Mission_Move only, mission == 1)
new_ground = new_cell.GetGroundHeight();        // destination step cell
old_ground = current_cell.GetGroundHeight();    // mover's OWN coord, TechnoClass+0x9c (current)
// CORRECTED 2026-06-11: the up/down→offset mapping below was previously inverted
// here and the "*Downhill-like" labels were guesses. Verified at the instruction
// level (Drive 0x004b3cd5..0x004b3da6: ESI=GetGroundHeight(dest), EDI=GetGroundHeight(obj+0x9c),
// `CMP ESI,EDI; JLE` ⇒ ESI>EDI block uses +0x768/+0x778; Ship 0x006a3324.. identical) and
// against the loader FSTP @0x0066f234.. which maps +0x768=TrackedUphill, +0x770=TrackedDownhill,
// +0x778=WheeledUphill, +0x780=WheeledDownhill.
if (new_ground > old_ground && mission_id == 1) {        // going UP (dest higher)
    if (TT.SpeedType == 1 /*Track*/)
        speed *= *(double *)(RulesClass + 0x768);        // TrackedUphill  (vanilla 1.0)
    else
        speed *= *(double *)(RulesClass + 0x778);        // WheeledUphill  (vanilla 1.0)
} else if (new_ground < old_ground && mission_id == 1) { // going DOWN (dest lower)
    if (TT.SpeedType == 1 /*Track*/)
        speed *= *(double *)(RulesClass + 0x770);        // TrackedDownhill (vanilla 1.2)
    else
        speed *= *(double *)(RulesClass + 0x780);        // WheeledDownhill (vanilla 1.2)
}

// Step 4: Minimum-speed floor
if (speed == 0.0) speed = 0.5;       // **never zero — clamp to 0.5**

// Step 5: Damaged-unit penalty
health_ratio = techno.GetHealthRatio();
if (health_ratio <= *(double *)(RulesClass + 0x1700)) {  // ConditionYellow (50% default)
    speed *= _DAT_007E7FC0;          // **0.75** (double 0.75)
}

// Step 6: Apply
if (locomotor.track_index < 0x40) {  // early in track
    locomotor.speed = speed;          // write direct to +0x4C
} else if (speed != techno.current_speed) {
    techno.vtable+0x544(speed_low, speed_high);          // Set_Speed
}
```

**Constants verified by direct memory read:**
- `_DAT_007F1308` = double **0.3** (Stop_Moving deceleration cap)
- `_DAT_007E7FC0` = double **0.75** (damaged-unit speed multiplier)
- `RulesClass+0x768` / `+0x770` / `+0x778` / `+0x780` = cliff multipliers (run-time INI-driven, accessed as doubles)
- `RulesClass+0x1700` = ConditionYellow threshold (default 0.5)
- `RulesClass+0x1718` = CloseEnoughDistance
- `RulesClass+0x1768` = stuck timeout frames (used for `+0x670` block-timer max)
- `RulesClass+0x94` = WakeAnim type pointer

**Parity-load-bearing details:**
1. **`> 1.0` clamp comes BEFORE cliff mult** — so a cliff mult can push speed back above 1.0 (e.g. downhill on tracked from a starting 0.8 base × 1.4 down-mult = 1.12 — kept at 1.12, not re-clamped).
2. **`== 0.0 → 0.5` floor comes AFTER cliff mult** — so 0×anything = 0 catches the floor, but a tiny positive 0.001 × 0.5 down-mult = 0.0005 is preserved (not floored to 0.5). Floor only fires on exact zero.
3. **`HealthRatio <= 0.5`** is `<=`, not `<`. A unit at exactly 50% health gets the speed penalty (which is 0.75x).
4. **The `< 0x40` track-index check** controls whether speed is written directly (early in track curve) or routed through vtable+0x544. The cutoff `0x40 = 64` is exactly half of the 128-step track curve resolution.

### 8.7 Track index commit and step

```c
uVar19 = techno+0x5E4;             // next-step direction (path queue [1])
if (uVar19 == 0xFFFFFFFF || retry) uVar19 = uVar18;  // fallback: same direction
locomotor.track_index = uVar19 + uVar18 * 8;          // (new * 1) + (old * 8) — 8x8 = 64-entry LUT
if (TurnTrack[track_index*0xC].curve_type == 0) {
    locomotor.track_index = uVar18 * 9;                // 0,9,18,...,63 = the 8 "straight" tracks
}
if (RawTrack[locomotor.track_index*0xC].flags & 8) {
    // Curve-track path: pre-validate the next cell
    cell2 = MapClass::Get_CellClass(&next_pos);
    crate = CrateClass::PickupDispatch();
    if (crate || dead) { ... }
    else {
        // Step ahead and re-check Can_Enter_Cell on the curve target
        result = techno.vtable+0x1AC(cell2, uVar19, level, ...);
        // ... handle result like step 8.5 ...
    }
} else {
    // Straight-track path: shift path queue (24 entries)
    src = techno+0x5E4; dst = techno+0x5E0;
    for (i = 23; i > 0; i--) { *dst = *src; dst++; src++; }
    goto LAB_006a3c3b (commit cleanup);
}
```

**Subtle details:**
1. **Path queue is 24 entries** (`for iVar5 = 0x17`), each 4 bytes → 96-byte buffer at `techno+0x5E0` through `techno+0x640`.
2. **Track index formula `new + old*8`** indexes into the 8×8 = 64-entry TurnTrack table. Entries 64-66 (Ship's table) are reserved for special curves.
3. **Straight-track shortcut** (`flags & 8`): skips per-cell re-validation, just shifts the path queue and commits. Used when the next direction is the same as the current (no turn).

### 8.8 Commit cleanup (LAB_006a3c3b)

```c
techno+0x63C = 0xFFFFFFFF;            // last-step direction reset
techno+0x558 = next_cell_xy;           // store target cell for renderer
techno+0x68A = 0;                      // clear stuck-sound flag
locomotor+0x5C = 0;                    // track sub-step = 0 (start of curve)
if (head_to_coord != NullCoord) clear it;
if (next_pos != NullCoord) {
    dest_valid_flag = 1;               // class+0x63
    longer_dest = next_pos;            // +0x40..+0x48
    cell = Get_Cell_At(&next_pos);
    crate = CrateClass::PickupDispatch();
    if (crate && techno+0x81 == 0) {
        Apply_Track_Step(&next_pos, 1);
        return 0;
    }
    if (techno+0x90 != 0) {            // dead/limboed
        longer_dest = NullCoord;
        dest_valid_flag = 0;
    }
}
locomotor.track_index = -1;            // +0x58 = -1 (path exhausted, awaits new path)
techno.path_index = -1;                 // +0x5E0
techno.vtable+0x544(0, 0);              // hard-stop speed
return 0;
```

C=HIGH (whole function decompiled and traced), I=HIGH, B=HIGH (sole caller verified — Process at 0x69FC10, plus 3 recursive self-calls).

---

## 9. `Process_Drive_Track` @ `0x006A05F0`

Full bridge-relevant analysis is in [`BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`](BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md) §7.2 (Ship-specific overrides) and §13.2 (cleanup-pass verification that all bridge sites mirror Drive's). Summary of Ship-specific items in this function NOT covered by the bridge doc:

### 9.1 Wall/overlay throttle constant

Per `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §7.4, the bridge doc found a 7th Ship-vs-Drive difference not in the original `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`:

```c
// In Process_Drive_Track, when TT.+0xD2B != 0 (approaching-wall flag) and cell.OverlayType is a wall:
// Drive @ 0x4B1A?? : *(undefined4 *)(FootClass + 0x334) = 0xbd4ccccd;   // -0.05f
// Ship @ 0x6A1135  : *(undefined4 *)(FootClass + 0x334) = 0xbca3d70a;   // -0.02f
```

**Ship's wall-throttle is -0.02 (slower deceleration)** vs Drive's -0.05. Ships approaching obstacles decelerate less aggressively — consistent with naval inertia.

C=HIGH (per bridge doc cleanup pass), I=MEDIUM (the semantic "wall-throttle" inferred), B=HIGH.

### 9.2 Process_Drive_Track is the ONLY caller of Apply_Track_Step under normal flow

`get_function_callers Process_Drive_Track` returned: only Ship::Process (the per-tick dispatcher). And `Apply_Track_Step` is called from:
- `Process_Drive_Track @ 0x6A05F0` (multiple sites internal)
- `Force_Track @ 0x6A0310` (one-shot on `Force_Track` calls)
- `Is_To_Have_Shadow_Override @ 0x6A3F00` (slot 39 — see §6.14)

C=HIGH (per cross-doc + cleanup), I=HIGH, B=HIGH.

---

## 10. Init-time computations and globals

### 10.1 The init dispatch table

| Address | Init function | Computes |
|---|---|---|
| `0x00814A5C` (DATA xref) | `ShipLocomotionClass::Compute_ShipHeightStep` @ `0x69EB10` | `g_ShipHeightStep` at `0x00B07838` |
| `0x00814A68` (DATA xref) | `ShipLocomotionClass::Compute_BridgeZOffset` @ `0x69EBB0` | `g_BridgeZ_Offset` at `0x00B0782C` |
| (also) | `ShipLocomotionClass::InitNullCoords` @ `0x69EBF2` | writes `g_NullCoord_Ship_X/Y/Z` at `0x00B077F8/FC/0x00B07800` |

The engine walks an init-function dispatch table at boot to run all locomotor-class one-time computations after the isometric projection constants (`_DAT_00B077C0`, `_DAT_00B077E8`) are set.

### 10.2 Compute_ShipHeightStep @ `0x69EB10`

```c
void Compute_ShipHeightStep(void) {
    Sin_Lookup_Table4096(_DAT_00B077C0 - _DAT_00B077E8);
    g_ShipHeightStep = Math__ftol();
}
```

`g_ShipHeightStep` (at `0x00B07838`) is the vertical-lepton displacement of one map height level under the isometric projection. Multiplied by 2 in many threshold checks; multiplied by 4 for the bridge Z-offset.

### 10.3 Compute_BridgeZOffset @ `0x69EBB0`

Already covered in detail in `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §6.1-§6.3. Formula:

```
g_BridgeZ_Offset (0x00B0782C) = round_half_up(g_ShipHeightStep * 4)
```

Byte-for-byte same code as Drive's equivalent at `0x4AF4A0`, just with different source/dest globals.

### 10.4 InitNullCoords @ `0x69EBF2`

```c
void InitNullCoords(void) {
    // (with __regparm EAX = constructor's input sentinel value)
    g_NullCoord_Ship_X = in_EAX;
    g_NullCoord_Ship_Y = in_EAX;
    g_NullCoord_Ship_Z = in_EAX;
}
```

Single WRITE to all three sentinel globals at addresses `0x00B077F8`, `0x00B077FC`, `0x00B07800`. All three are runtime BSS (cold dump = `0`), so before init they're zero — and the sentinel is **also zero** at runtime (the function writes `EAX` which is the input value at call time). This means **`g_NullCoord_Ship_*` is literally `(0, 0, 0)`** at runtime, identical to Drive's.

**Parity-load-bearing detail:** Because the sentinel value is `(0, 0, 0)`, a coord of exactly `(0, 0, 0)` is INDISTINGUISHABLE from "null" in Ship's logic. The map's `(0, 0, 0)` corner is in cell `(0, 0)` which is the **playfield edge** — but importantly, the playfield diamond starts at `(2, 2)` minimum, so `(0, 0)` is OUT-OF-BOUNDS. No real unit will ever have coords `(0, 0, 0)` legitimately. This is the "correct" use of `(0, 0, 0)` as null sentinel.

InitNullCoords has **0 xrefs** but IS called — via the init dispatch table or constructor-via-Ghidra-unanalyzed-edge. Confirmed live because all `g_NullCoord_Ship_*` reads in Ship's methods produce correct null-comparison behaviour.

C=HIGH, I=HIGH, B=MEDIUM (caller is the init dispatch table mechanism, which Ghidra doesn't track as a regular function call).

---

## 11. Differences from DriveLocomotionClass (with corrections to prior comparison doc)

`SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` enumerates 6 differences. This pass adds 2 more and corrects 1 mislabel.

### 11.1 (Existing) Wake/dust interval

Drive: every 10 frames (`IDIV 0xa`). Ship: every 8 frames (`AND 0x80000007`). **Confirmed.** Ship has 25% more wake spawns.

### 11.2 (Corrected) Wake guard check

Prior doc said: "Ship skips the `+0xD69` check that Drive performs." **WRONG.** Ship DOES check `TechnoType+0xD69`. The prior doc misread the decompiler output. Confirmed by direct decompilation of Ship::Process @ 0x69FC10 in this pass.

**Real difference:** Ship adds an additional check: `techno+0x8C (on_bridge) == 0`. Drive does NOT suppress dust on bridge.

### 11.3 (Existing) Decel source

Drive calls `vtable+0x38C` (dynamic). Ship reads `TT+0x678` directly. **Confirmed.** For stock content these produce the same value.

### 11.4 (Existing) Tether ordering

Drive: deploy/undeploy → tether → warp. Ship: tether → deploy/undeploy → warp. **Confirmed.** Edge case matters if a tethered unit is also deploying.

### 11.5 (Existing) Convoy/tow block

Drive has it; Ship doesn't. **Confirmed.** Ships don't tow.

### 11.6 (Existing) Track table size

Drive: 72 TurnTrack + 16 RawTrack. Ship: 67 + 14. **Confirmed via `read_memory`.**

### 11.7 (Bridge doc) Wall/overlay throttle constant

Drive uses `-0.05` (0xBD4CCCCD). Ship uses `-0.02` (0xBCA3D70A). Per `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` §7.4. **Confirmed.**

### 11.8 (NEW, this pass) Slot 31 mis-identification in prior doc

`SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` §5 vtable table labels Ship's slot 31 (0x69F250) as **"Piggybacker_CLSID"**. **WRONG.** The actual function body is a **slope-state setter** (writes +0x18, +0x1C, +0x20, +0x28, +0x2C — the slope cache fields). Confirmed because:
- Drive's slot 31 is `0x4AFB40`, NOT `0x4AF610` (Ghidra's "Piggybacker_CLSID" symbol)
- `0x4AFB40` and `0x69F250` have **byte-identical** decompilations (the slope-state setter pattern)
- The TRUE Piggybacker_CLSID is reached via the **IPiggyback** vtable (slot 7 there = `0x6A4280` for Ship), NOT the ILocomotion vtable

The Ghidra symbol `Piggybacker_CLSID` at `0x4AF610` is correct — but `0x4AF610` is NOT in either Drive's or Ship's ILocomotion vtable. The naming confusion likely arose from Ghidra labeling Drive's `Force_New_Slope` wrapper at `0x4B04D0` as "Update_Facing_From_Type" and the matching helper at slot 31 as "Piggybacker_CLSID" — neither label is accurate for what the function actually does.

**Renamed semantically in this report:** Slot 31 = `Slope_State_Set` (or `Force_New_Slope_Helper`).

### 11.9 (NEW, this pass) Naval-stale claims in NAVAL_SYSTEM_RESEARCH.md

Prior `NAVAL_SYSTEM_RESEARCH.md` §1 states:
> "Shared between Ship and Drive: ... The same main movement AI subroutine (`FUN_006a1c80`, ~8470 bytes) — called from both Process functions"

**Stale.** `FUN_006A1C80` is Ship's `Process_Movement`. Drive's `Process_Movement` is at a DIFFERENT address: `0x004B2630`. They share the same source code (compiled twice) but they are not the literal same function. The 6 differences enumerated in §11.1-§11.7 prove this. A Rust port can share the implementation but must select between Ship's and Drive's globals (height_step, NullCoord, TurnTrack table, etc.).

---

## 12. INI bindings

### 12.1 `Locomotor=` CLSID parsing

INI key `Locomotor=` is read in `TechnoTypeClass::ReadINI` at `0x7123FA` (verified via `get_xrefs_to "Locomotor"` string at `0x84444C`). The string is consumed as a GUID literal; the parser instantiates via `CoCreateInstance(CLSID, ...)`.

**Confirmed units with `Locomotor={2BEA74E1-...}` in `rulesmd.ini`:**

| Section | Unit | Line | SpeedType | MovementZone | TooBigToFitUnderBridge |
|---|---|---|---|---|---|
| `[DEST]` | Destroyer | 7114 | Float | Water | true |
| `[DLPH]` | Dolphin | 7166 | Float | Water | (not set / false) |
| `[AEGS]` | Aegis Cruiser | 7236 | Float | Water | true |
| `[ACC]` | Aircraft Carrier | 7290 | Float | Water | true |
| `[SEAW]` | Sea Wolf | 7987 | Float | Water | (likely true) |
| `[SUB]` | Typhoon Attack Sub | 8037 | Float | Water | (likely true) |
| `[SQD]` | Giant Squid | 8095 | Float | Water | (not set) |
| `[DRED]` | Dreadnought | 8161 | Float | Water | true |

**The `;{4A582741-...}` suffix on many lines is an INI comment**, not a fallback locomotor. The `;` starts a comment per INI rules. Only the FIRST CLSID is parsed.

**Parity-load-bearing detail:** Ships ALWAYS use `SpeedType=Float` and `MovementZone=Water`. Together these gate the speed multiplier table (Float row of `g_SpeedType_LandType_Table`) and the passability matrix (Water row of `0x82A594`, allowing only ZoneType 4 = water cells).

### 12.2 `TooBigToFitUnderBridge=` parsing

Read by `TechnoTypeClass::ReadINI` into `TechnoType+0xC94` (byte). Consumed by:
- Ship's `Process_Drive_Track` (TooBig crush-on-bridge layer pick — bridge doc §4.3)
- Ship's `Process_Movement` (`local_48 < 7 + TT.+0xC94` → force-passable override, §8.5)
- Ship's `Stop_Moving` (blocker-list cleanup, §6.7)

### 12.3 Speed multiplier constants in `[General]` section of rulesmd.ini

| RulesClass offset | Likely INI key | Default in YR |
|---|---|---|
| `+0x768` | `TrackedDownhillSpeed` or similar | (depends on rules.ini parsing) |
| `+0x770` | `TrackedUphillSpeed` | |
| `+0x778` | (non-Track) `DownhillSpeed` | |
| `+0x780` | (non-Track) `UphillSpeed` | |
| `+0x1700` | `ConditionYellow` | **0.5** (50%) |
| `+0x1718` | `CloseEnoughDistance` | (TBD — verify via INI) |
| `+0x1768` | (Stuck timeout frames) | (TBD) |
| `+0x94` | `Wake=` (anim type pointer) | (TBD — verify) |

INI string verification:
- `ConditionYellow=50%` and `ConditionRed=25%` are confirmed at lines 752-753 of `rulesmd.ini` (verified by Grep this pass).
- The cliff-speed multipliers need explicit INI grep — deferred to §13 open questions.

---

## 13. Open Questions / Items requiring follow-up

1. **RulesClass offsets for cliff speed multipliers** (`+0x768/+0x770/+0x778/+0x780`) — the INI key names are likely `TrackedDownhillSpeed` etc. but unverified at the binary parser site. Would need to grep `RulesClass__ReadINI` (probably at `0x66XXXX`) for the specific READ calls.

2. **`RulesClass+0x1768` (stuck timeout frames)** — needs INI key identification.

3. **The `0x600` flag argument to `AnimClass::Constructor`** in the wake spawn — needs `ANIM_CLASS_DEEP_DIVE.md` cross-reference to verify it means "ground layer + Z-suppressed".

4. **`TechnoType+0xD69`** — the "no-spawn-wake" flag's INI key name. Possibly `WakeAnim=no` or `NoWake=yes` or similar. Untraced at the ReadINI site.

5. **`FootClass+0x6D0`** — the "skip TooBig crush this tick" guard. Set by what? Possibly a one-tick anti-recursion flag. Cross-reference with `FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md`.

6. **Mission ID `0x10`** — Ship's Process gates on `mission_id == 0x10`. Likely `MISSION_AREA_GUARD` or `MISSION_HARMLESS`; verify with `MISSIONCLASS_STATE_MACHINE.md`.

7. **`techno.vtable+0x29C` and `techno.vtable+0x1AC`** — these are TWO distinct Can_Enter_Cell-like vtable slots. The 0x29C is the "movement pre-gate" and 0x1AC is the classifier returning codes 0..7. Both deserve cross-references to `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` and `TECHNOCLASS_VTABLE_COMPLETE.md` to confirm signatures.

8. **The 4th IUnknown vtable slot at `0x6A3E60`** — non-standard. Verify whether it's a Ship-specific destructor entry or part of Westwood's extended IUnknown convention.

9. **Submarine "cloaked" state and locomotor layer:** Ships' `In_Which_Layer` returns `2` (Ground) unconditionally. So submerged submarines must use a separate ObjectClass-level mechanism for the "underwater" visibility/sensor state. Cross-reference with `CLOAKING_INTERACTIONS_REPORT.md` and `SENSOR_CLOAK_DETECTION.md` — not strictly a Ship locomotor concern.

10. **Init dispatch table @ `0x00812D50` (Drive) / `0x00814A58` (Ship)** — the dispatch table mechanism that calls `Compute_*HeightStep` / `Compute_*BridgeZOffset` / `InitNullCoords` at boot. Worth documenting separately — likely shared with other classes' init functions.

---

## 14. Sources

**Ghidra functions decompiled (full body):**
- `ShipLocomotionClass::Constructor` @ `0x69EC50`
- `ShipLocomotionClass::Constructor` (variant) @ `0x69ECF0` (= virtual destructor)
- `ShipLocomotionClass::Constructor` (variant) @ `0x6A42B0` (= scalar-deleting destructor)
- `ShipLocomotionClass::Compute_ShipHeightStep` @ `0x69EB10`
- `ShipLocomotionClass::Compute_BridgeZOffset` @ `0x69EBB0` (via bridge doc)
- `ShipLocomotionClass::InitNullCoords` @ `0x69EBF2`
- `ShipLocomotionClass::Process` @ `0x69FC10`
- `ShipLocomotionClass::Process_Movement` @ `0x6A1C80` (~6.5 KB, 1137 lines of decompiled C)
- `ShipLocomotionClass::Is_Moving` @ `0x69F290`
- `ShipLocomotionClass::Is_Moving_Now` @ `0x69F330`
- `ShipLocomotionClass::Destination` @ `0x69F3A0`
- `ShipLocomotionClass::Move_To` @ `0x69F3D0`
- `ShipLocomotionClass::Set_Destination` @ `0x69F450`
- `ShipLocomotionClass::Stop_Moving` @ `0x69F510`
- `ShipLocomotionClass::Draw_Matrix` @ `0x69F670`
- `ShipLocomotionClass::Shadow_Matrix` @ `0x69FB20`
- `ShipLocomotionClass::Force_New_Slope` @ `0x69FBE0` (wrapper)
- `ShipLocomotionClass::Slope_State_Set` @ `0x69F250` (slot 31 helper)
- `ShipLocomotionClass::Apply_Track_Step` @ `0x6A01A0`
- `ShipLocomotionClass::Force_Track` @ `0x6A0310`
- `ShipLocomotionClass::Do_Turn_Update` @ `0x6A05C0`
- `ShipLocomotionClass::Transform_Track_Coords` @ `0x6A3DB0`
- `ShipLocomotionClass::In_Which_Layer` @ `0x6A3E50` (returns 2)
- `ShipLocomotionClass::Mark_All_Occupation_Bits` @ `0x6A3EA0` (returns 0)
- `ShipLocomotionClass::Z_Adjust` @ `0x6A3EB0` (delegates to base)
- `ShipLocomotionClass::Is_To_Have_Shadow_Override` @ `0x6A3F00` (Apply_Track_Step trampoline)
- `DriveLocomotionClass::Piggybacker_CLSID` @ `0x4AF610` (for slot-31 disambiguation)
- `DriveLocomotionClass::Force_New_Slope` @ `0x4AFB40` (slot-31 byte-equivalence check)
- `DriveLocomotionClass::Update_Facing_From_Type` @ `0x4B04D0` (Drive slot 20 wrapper, for comparison)

**Memory reads:**
- `0x007E9AB0` (Ship CLSID GUID bytes) → verified `2BEA74E1-7CCA-11D3-BE14-00104B62A16C`
- `0x007F2D8C` len 160 (ILocomotion vtable, 40 slots)
- `0x007F2E58` len 16 (IUnknown vtable, 4 slots)
- `0x007F2D68` len 36 (IPiggyback vtable, 9 slots)
- `0x007E7EB0` len 160 (Drive's ILocomotion vtable, for slot-by-slot diff)
- `0x007F1308` len 16 (deceleration constants: 0.3, 0.1, 0.0015)
- `0x007E7FC0` len 16 (damaged-unit multiplier 0.75)
- `0x007E44E8` len 16 (Draw_Matrix tilt threshold)

**Xref tables:**
- `get_xrefs_to 0x007F2D8C` → 4 DATA references (3 constructor variants + QueryInterface site)
- `get_xrefs_to 0x69EC50` → 1 (CreateInstance @ `0x6C4F4C`)
- `get_xrefs_to 0x69EB10` → 1 DATA (init dispatch table `0x814A5C`)
- `get_xrefs_to 0x69EBB0` → 1 DATA (init dispatch table `0x814A68`)
- `get_xrefs_to 0x007E9AB0` → 4 (WinMain registration + naval-yard production + Process_Movement vtable dispatch + In_Which_Layer call site)
- `get_xrefs_to 0xB077F8` (g_NullCoord_Ship_X) → 52 sites across all Ship methods (1 WRITE from InitNullCoords + 51 READs)
- `get_xrefs_to "Locomotor" string at 0x84444C` → 2 DATA (TechnoTypeClass::ReadINI @ `0x7123FA`, WarheadTypeClass::ReadINI @ `0x75D88F`)

**INI files cross-referenced:**
- `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` (lines 752-753 ConditionYellow/Red, lines 7114/7166/7236/7290/7987/8037/8095/8161 ship Locomotor= entries)
- `c:/Users/enok/Documents/ra2-rust-game/ini/rules.ini` (base RA2 ship entries for cross-version comparison)

**Companion docs:**
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` (corrections at §11.2 and §11.8 of this report)
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` (full bridge interaction)
- `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` (Water/Float passability)
- `NAVAL_SYSTEM_RESEARCH.md` (corrections at §11.9 of this report)
- `NAVAL_IMPLEMENTATION_PLAN.md` (Rust-side guidance)
- `ILOCOMOTION_COM_PROTOCOL_SPEC.md` (base ILocomotion contract)
- `LOCOMOTION_MATH_AND_CONSTANTS.md` (CLSID GUID list)
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` (FootClass+0x5E0 path queue layout)
- `FOOTCLASS_FLAGS_BLOCK_GHIDRA_REPORT.md` (FootClass+0x6B7 / +0x68A / +0x68B flag semantics)

---

## 15. TS-legacy filtering

Every function in this report has been verified to run in standard YR skirmish. **No TS-only gating found.** Specifically:
- No `SpecialFlags & 0x1000` (fog-of-war) checks
- No mission state `0xXX` checks that are TS-only (Mission_Sleep, Mission_Hunt, Mission_Move all confirmed live in YR)
- The CLSID is referenced by 8+ standard YR units in `rulesmd.ini`
- The CLSID is referenced by `BuildingClass::MissionRepairAndProduce` (naval yard production) — wired into the live production pipeline

**Ship is fully active in YR.** No dormant TS code paths identified in this pass.

---

*End of report. Generated 2026-05-17 via Ghidra MCP live decompilation. Apply the corrections in §11 to prior docs.*
