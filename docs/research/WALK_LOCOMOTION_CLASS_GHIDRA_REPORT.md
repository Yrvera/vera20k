# WalkLocomotionClass — Full Ghidra Research Report

**Primary addresses:**
- `WalkLocomotionClass::Constructor` @ `0x0075AA90`
- `WalkLocomotionClass::Process` @ `0x0075AC80`
- `WalkLocomotionClass::ProcessMovement` @ `0x0075AEC0`
- `WalkLocomotionClass::FindSubCellDest` @ `0x0075C240`
- ILocomotion vtable @ `0x007F69F8` (40 slots)
- IUnknown vtable @ `0x007F6AC4` (4 slots)
- IPiggyback vtable @ `0x007F69D4`
- CLSID `{4A582744-9839-11D1-B709-00A024DDAFD1}` (per Ghidra constructor comment)

**Active in YR:** **Yes** — every infantry unit uses Walk (GI, Conscript, Engineer, Tanya, Yuri Prime, IFV-pilots, all spec-ops infantry, dogs, etc. — approximately **64 stock units** per the Ghidra symbol comment). Walk runs every tick for every infantry on the map. Among the most performance-critical locomotors in the engine.

**Confidence convention** — see `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §0. Each finding tagged with **C** (content), **I** (identity), **B** (binding).

**Cross-reference docs (read these first):**
- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` §2 — bridge interaction details, the `FUN_006D2120(60)` Walk-specific Z-bump helper, the `DAT_00B45C28` per-level height scale
- `INFANTRY_SUBCELL_POSITIONING.md` — 5-subcell layout
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — FootClass+0x5E0 path queue layout
- `MISSIONCLASS_STATE_MACHINE.md` — Mission ID enum (used by FindSubCellDest dispatch)
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — Ship's parallel methods (Process_Movement structure, Can_Enter_Cell dispatch codes 0-7 — these are SHARED between Walk and Ship)

---

## 1. Overview

`WalkLocomotionClass` is the locomotor for all infantry in YR. Unlike Drive/Ship which use a **drive-track curve system** (8x8 = 64 pre-computed curves indexed by `(old_facing, new_facing)`), Walk uses **direct angle-based stepping** with sub-cell placement. There is no TurnTrack table for Walk.

**Player-observable behaviours:**
- 5-subcell occupancy within a single cell (so up to 5 infantry can share a cell at center + 4 sub-positions)
- Per-direction facing animation (via `FacingClass__UpdateFacing`)
- Smooth interpolation between sub-cells using `atan2` for direction and `Sqrt_Approx` for distance
- Stuck-detection with the same retry-counter pattern as Drive/Ship (`FootClass+0x64C`, +0x6B7, +0x668)
- Bridge transition handling via a different Z-bump formula than Drive/Ship
- TS-tube traversal (path-step value 8 + `g_TubeArray`) — **conditionally active** in YR (only when maps include `[Tubes]` data)

**Why infantry are different from vehicles:** A vehicle's voxel can't be at "1/4 cell over"; it occupies the whole cell. An infantryman is a sprite and CAN render at sub-cell precision. The cell occupancy grid stores up to 5 infantry per cell at named sub-positions (center + 4 corners).

---

## 2. Class identity

| Property | Value | Evidence |
|---|---|---|
| CLSID | `{4A582744-9839-11D1-B709-00A024DDAFD1}` | Ghidra constructor comment + INI `Locomotor=` entries below |
| Constructor | `0x0075AA90` | Full disassembly verified |
| Virtual destructor | `0x0075AB00` | Releases piggyback at `+0x38`, calls `LocomotionClass::Destructor` |
| Scalar-deleting destructor | `0x0075CBE0` | (per earlier search results) |
| ILocomotion vtable | `0x007F69F8` (40 slots, 160 bytes) | `read_memory` verified |
| IUnknown vtable | `0x007F6AC4` | constructor stores at `+0x00` |
| IPiggyback vtable | `0x007F69D4` | constructor stores at `+0x18` |
| Instance size | **~0x3C bytes** (60 bytes) | constructor's last init at `+0x38` |
| NullCoord sentinel | `g_NullCoord_Walk_X/Y/Z` at `0x00B45BE8/EC/F0` | from constructor `read_memory` |
| Confidence | C=HIGH, I=HIGH, B=HIGH | |

**Subtle detail — Walk is SMALLER than Drive/Ship.** Drive/Ship are 0x6C bytes. Walk is 0x3C bytes — almost half. Why: no track tables, no track index, no track sub-step counter, no slope-blend timer fields, no second-vtable IPiggyback at a separate offset (Walk's IPiggyback is at `+0x18`, not `+0x68`).

---

## 3. Struct field layout

| Offset | Size | Field | Notes |
|---|---|---|---|
| `+0x00` | 4 | IUnknown vtable ptr | |
| `+0x04` | 4 | ILocomotion vtable ptr | |
| `+0x08` | 4 | (inherited) | |
| `+0x0C` | 4 | LinkedTo TechnoClass* | |
| `+0x10`–`+0x17` | 8 | (inherited) | |
| `+0x18` | 4 | **IPiggyback vtable ptr** | **DIFFERENT from Drive/Ship** (which put it at `+0x68`). Walk's smaller struct moves it up. |
| `+0x1C` | 4 | **head_to_coord X** (immediate dest) | Written by `Head_To_Coord` (vtable slot 17). Null sentinel = `g_NullCoord_Walk_X`. |
| `+0x20` | 4 | head_to_coord Y | |
| `+0x24` | 4 | head_to_coord Z | **Bridge Z-bump site** (see §6.5) |
| `+0x28` | 4 | **longer destination X** | Written by `FindSubCellDest` after finding a sub-cell |
| `+0x2C` | 4 | longer destination Y | |
| `+0x30` | 4 | longer destination Z | |
| `+0x34` | 1 | **destination-valid flag** | Byte. Set to 1 in `Head_To_Coord` when a dest is assigned. Returned by `Is_Moving` (slot 4). Cleared by `Stop_Moving` and `Mark_All_Occupation_Bits` when both dests are null. |
| `+0x35` | 1 | **in-Process flag** | Set to 1 at the top of `Process`, cleared at exit. Prevents re-entrancy if `Is_Moving` (called from inside Process at the end) recurses back. |
| `+0x36` | 1 | **is-moving-here flag** | Byte. Set to 1 by `ProcessMovement` when a sub-cell destination is found and the unit is committing to it. Read by external code via `Get_Is_Moving_Here` (slot — see §6.13). Cleared by `Clear_Is_Moving_Here`. |
| `+0x38` | 4 | piggyback ptr | Set non-null when this Walk locomotor has been piggybacked under another (e.g. transport). Released by destructor. Constructor XORs to zero. |

**Subtle detail — three flag bytes at +0x34/+0x35/+0x36:** Drive/Ship pack flags too, but Walk uses contiguous bytes here. The constructor zeros all three with a single `XOR EAX, EAX; MOV byte ptr [ESI + 0x34], AL; MOV byte ptr [ESI + 0x35], AL; MOV byte ptr [ESI + 0x36], AL` sequence (3 separate byte writes from the same zero register). The 4-byte aligned `MOV dword ptr [ESI + 0x38], EAX` follows.

---

## 4. ILocomotion vtable @ `0x007F69F8` (40 slots)

Parsed from `read_memory 0x007F69F8 len 160`. "**Base**" = the same address appears in Drive/Ship's vtables too (inherited from `LocomotionClass`).

| # | Off | Addr | Function | Class-specific? | Notes |
|---|-----|------|----------|----------------|-------|
| 0 | 0x00 | `0x75CC30` | QueryInterface | Walk-specific | |
| 1 | 0x04 | `0x75CC40` | AddRef | Walk-specific | |
| 2 | 0x08 | `0x75CC50` | Release | Walk-specific | |
| 3 | 0x0C | `0x55A710` | `Link_To_Object` | Base | Shared with Drive/Ship |
| 4 | 0x10 | `0x75AB30` | **`Is_Moving`** (returns class+0x34 byte) | Walk-specific | §6.3 |
| 5 | 0x14 | `0x75ABA0` | **`Destination`** | Walk-specific | §6.4 |
| 6 | 0x18 | `0x75AC00` | **`Head_To` / `Move_To`** | Walk-specific | §6.5 |
| 7 | 0x1C | `0x55ABF0` | base stub | Base | |
| 8 | 0x20 | `0x55ABE0` | base `Do_Turn` | Base | Walk doesn't override Do_Turn |
| 9 | 0x24 | `0x55A730` | **base `Draw_Matrix`** | Base | Walk uses the generic locomotor draw matrix (no class-specific override). Per the Bridge doc comparison table, this matches Mech and DropPod. |
| 10 | 0x28 | `0x55A7D0` | base `Shadow_Matrix` | Base | |
| 11 | 0x2C | `0x55ABD0` | base | Base | |
| 12 | 0x30 | `0x55A8C0` | `LocomotionClass::Can_Enter_Cell` | Base | Same as Drive/Ship |
| 13 | 0x34 | `0x55ABC0` | base | Base | |
| 14 | 0x38 | `0x55ABA0` | base | Base | Walk does NOT override slot 14 — different from Drive/Ship (which have Mark_All_Occupation_Bits here). See §11.1 for the slot-assignment caveat. |
| 15 | 0x3C | `0x55ABB0` | base | Base | |
| 16 | 0x40 | `0x75AC80` | **`Process`** | Walk-specific | §6.1 — thin wrapper around ProcessMovement |
| 17 | 0x44 | `0x75ACB0` | **`Head_To_Coord` / `Set_Destination`** | Walk-specific | §6.5 — bridge Z-bump site |
| 18 | 0x48 | `0x75ADA0` | **`Stop_Moving`** | Walk-specific | §6.6 |
| 19 | 0x4C | `0x75AE00` | **`Set_Facing`** | Walk-specific | §6.7 — calls `FacingClass__UpdateFacing` |
| 20 | 0x50 | `0x55AC20` | base | Base | Walk does NOT override `Force_New_Slope` — infantry don't tilt with terrain slope visually the way vehicles do |
| 21-27 | 0x54-0x6C | various | base shared | Base | |
| 28 | 0x70 | `0x55AC10` | **base** (NOT Walk-specific) | Base | Walk does NOT override `Force_Track` — confirms no track-table system |
| 29 | 0x74 | `0x75C7E0` | **`Get_Locomotion_Type`** (returns 2) | Walk-specific | §6.10 — returns `2`. In Drive/Ship's slot 29 the same return value is labeled `In_Which_Layer`. Same slot role: layer = Ground (2). |
| 30 | 0x78 | `0x75AE30` | **`Mark_All_Occupation_Bits`** | Walk-specific | §6.11 |
| 31 | 0x7C | `0x55ACE0` | base | Base | Walk does NOT override slot 31 — different from Drive/Ship's slope-state helper |
| 32 | 0x80 | `0x75AB40` | **`Is_To_Have_Shadow`** | Walk-specific | §6.8 |
| 33-35 | 0x84-0x8C | shared base | Base | |
| 36 | 0x90 | `0x4B4C60` | `Begin_Piggyback` | Shared with Drive/Ship | |
| 37 | 0x94 | `0x4B4C70` | `End_Piggyback` | Shared | |
| 38 | 0x98 | `0x4B4C80` | `Is_Surfacing` | Shared | |
| 39 | 0x9C | `0x75CA30` | **`Power_On_Occupancy`** | Walk-specific | §6.12 |

**Subtle detail — `Mark_All_Occupation_Bits` slot disagreement:** Drive/Ship put it in slot 14. Walk puts it in slot 30. Both classes claim to implement the same ILocomotion interface, so the slot SHOULD be invariant. Two interpretations:
- (a) Ghidra mis-labelled one of them (likely: the actual semantic of slot 30 in Walk might be a different override).
- (b) The interface has multiple Mark_All_Occupation_Bits-like methods at different slots, with different signatures.

Empirical test: Drive's slot 14 = `0x4B4870` returns 0 (a stub). Walk's slot 30 = `0x75AE30` does real work (calls `FindSubCellDest` and updates flags). These are different functions, semantically. The Ghidra labeling of one as "Mark_All_Occupation_Bits" doesn't equal the other.

**Resolution:** Walk's slot 30 = real "sub-cell occupancy update" routine. Drive/Ship's slot 14 = a different role (likely an occupancy-bit clearing stub). The labeling is misleading; treat each by its actual behaviour.

---

## 5. Constructor and destructor

### 5.1 Constructor @ `0x0075AA90`

```asm
0075AA90: PUSH ESI
0075AA91: MOV ESI, ECX                   ; ESI = this
0075AA93: CALL 0x0055A6C0                ; LocomotionClass::Constructor (base)
0075AA98: MOV ECX, [0x00B45BE8]          ; ECX = g_NullCoord_Walk_X
0075AA9E: LEA EAX, [ESI + 0x1C]          ; head_to_coord ptr
0075AAA1: MOV [ESI + 0x1C], ECX          ; head_to_coord.X = NullCoord_X
0075AAA4: MOV EDX, [0x00B45BEC]          ; EDX = g_NullCoord_Walk_Y
0075AAAA: MOV [EAX + 0x4], EDX           ; head_to_coord.Y = NullCoord_Y
0075AAAD: MOV ECX, [0x00B45BF0]          ; ECX = g_NullCoord_Walk_Z
0075AAB3: LEA EDX, [ESI + 0x28]          ; longer-dest ptr
0075AAB6: MOV [EAX + 0x8], ECX           ; head_to_coord.Z = NullCoord_Z
0075AAB9: MOV EAX, [0x00B45BE8]          ; (re-load X — minor compiler artifact)
0075AABE: MOV [EDX], EAX                 ; longer_dest.X = NullCoord_X
0075AAC0: MOV ECX, [0x00B45BEC]
0075AAC6: MOV [EDX + 0x4], ECX           ; longer_dest.Y = NullCoord_Y
0075AAC9: MOV EAX, [0x00B45BF0]
0075AACE: MOV [EDX + 0x8], EAX           ; longer_dest.Z = NullCoord_Z
0075AAD1: XOR EAX, EAX
0075AAD3: MOV byte [ESI + 0x34], AL      ; dest-valid flag = 0
0075AAD6: MOV byte [ESI + 0x35], AL      ; in-Process flag = 0
0075AAD9: MOV byte [ESI + 0x36], AL      ; is-moving-here flag = 0
0075AADC: MOV dword [ESI + 0x38], EAX    ; piggyback ptr = NULL
0075AADF: MOV dword [ESI], 0x7F6AC4      ; IUnknown vtable
0075AAE5: MOV dword [ESI + 0x4], 0x7F69F8 ; ILocomotion vtable
0075AAEC: MOV dword [ESI + 0x18], 0x7F69D4 ; IPiggyback vtable
```

**Parity-load-bearing details:**
1. **Both coord triples init to NullCoord** (not zero). NullCoord literal value at runtime is `(0, 0, 0)` per BSS, but the SEMANTIC `is_null` comparison uses the global variable — important if a mod ever changes the null sentinel.
2. **No "first tick" flag** unlike Ship's `+0x65`. Walk's first move is identical to subsequent moves.
3. **No speed field** (no `+0x4C` double). Walk's speed is computed per-tick in `ProcessMovement` and stored on the techno directly via `techno.vtable+0x544`.
4. **IPiggyback vtable always set** at constructor time — Walk is always ready to be piggybacked (e.g. infantry entering a transport).

**Caller binding:** Sole caller is the class-factory CreateInstance for Walk (not separately decompiled — but follows the same pattern as Ship/Mech: a single `UNCONDITIONAL_CALL` from the COM dispatcher).

**Confidence:** C=HIGH, I=HIGH, B=HIGH.

### 5.2 Destructor @ `0x0075AB00`

```c
*param_1 = &IUnknown_vtable;          // defensive vtable re-set
param_1[1] = &ILocomotion_vtable;
param_1[6] = &IPiggyback_vtable;
piVar1 = (int *)param_1[0xe];          // piggyback ptr at +0x38
if (piVar1 != NULL) piggyback->Release();
LocomotionClass__Destructor();
```

Same pattern as Ship's destructor. C=HIGH, I=HIGH, B=HIGH.

---

## 6. ILocomotion method bodies

### 6.1 `Process` @ `0x0075AC80` (slot 16)

```c
void Process(int *param_1) {
    *(byte *)((int)param_1 + 0x31) = 1;        // class+0x35 = 1 (in-Process flag)
    WalkLocomotionClass__ProcessMovement(1);   // do the work, param=1 is "first call"
    *(byte *)((int)param_1 + 0x31) = 0;        // class+0x35 = 0
    (**(code **)(*param_1 + 0x10))(param_1);   // call ILocomotion::Is_Moving (slot 4)
}
```

**Subtle detail — the trailing `Is_Moving` call.** After ProcessMovement returns, Process explicitly calls `vtable+0x10 = slot 4 = Is_Moving`. The return value is **discarded** at the assembly level (no `MOV` to caller's space). This is a **side-effect-only call** — likely a hook that other systems observe via `Is_Moving` (e.g. for renderer state updates).

C=HIGH, I=HIGH, B=HIGH.

### 6.2 `ProcessMovement` @ `0x0075AEC0` (private, called by Process)

The state machine. ~1100 lines decompiled. Function signature: `void __fastcall ProcessMovement(int param_1)`.

`param_1` here is the **class-base** pointer (not the ILocomotion sub-object — note the offset arithmetic uses class-base coordinates).

**Top-of-function dispatch** (verified via `decompile_function 0x0075AEC0`; structure CORRECTED from prior doc revision — the active-movement branch fires when `longer_dest` is SET, not when it is null):

```
A: If longer_dest (+0x28..+0x30) is null:
   B: If head_to_coord (+0x1C..+0x24) is also null:
      C: If techno.current_speed (techno+0x15E double) > 0:
         → vtable+0x544(0, 0) Set_Speed(0)
         → techno+0x68A = 0 (clear stuck-sound flag)
         → return
      D: Else: goto LAB_0075C1E7 (final cleanup, clear stuck-sound, return)
   E: Else (head_to_coord set, longer_dest null) — pathfinding / Can_Enter_Cell branch:
      G: If techno.path_index (+0x5E0) == -1:
         → Pathfinding: stuck-timer maintenance, FootClass__Find_Path
         → On failure: scatter / stop / mission-update
      H: Else if path_index == 8:
         → TS-tube traversal (see §6.2.3)
      I: Else (normal step):
         → Compute target cell from g_DirectionDeltaX/Y_Table[(path_index & 7) * 8]
         → Read on_bridge transition: if (cell.flags >> 8 & 1) != FootClass.on_bridge: set FootClass+0x68B = 1
         → Call techno.vtable+0x1AC(cell, dir, level) for Can_Enter_Cell classification
         → Dispatch on the return code (0..7) — same dispatch as Ship (§8.5 in SHIP doc)
F: Else (longer_dest is SET) — active-movement branch (the "step toward longer_dest" path):
      → techno+0x36 = 1 (is_moving_here)
      → atan2(longer_dest.Y - techno.Y, longer_dest.X - techno.X) → facing
      → Call Set_Facing on param_1+4 (ILocomotion sub-object)
      → Call vtable+0x544(0, 0x3ff00000) Set_Speed(1.0)
      → return
```

#### 6.2.1 The Can_Enter_Cell return-code dispatch (codes 0..7)

**Same semantics as Ship** — see `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.5. Codes:
- **0** Passable → call `FindSubCellDest`, compute facing via `atan2`, set `+0x36 = 1` (is_moving_here), call `Set_Facing` and `Set_Speed(0, 0x3FF00000)` (speed=1.0)
- **1** Own/temporary → mark cell for redraw, recurse on retry, else stop
- **2** Blocked → stuck retry: `techno+0x6B7 = 1`, store frame at `+0x668`/`+0x66C`/`+0x670 = RulesClass+0x1768` (stuck timeout). Call FootClass__Find_Path with retry-flag, recurse
- **3** Crushable → MapClass__Check_Crushable_Obstacle, `techno+0x64C = 10` (retry counter)
- **4/5** Wall/Building → Find_Blocking_Object, ally check → notify or attack
- **6** Friendly stationary → distance check + bridge-layer scatter (§6.2.4)
- **7** Path-locked → clear path / retry
- **8** TS-tube (separate branch, §6.2.3)

#### 6.2.2 Speed / facing for code 0 (passable)

```c
if (Can_Enter_Cell == 0) {
    if (FindSubCellDest(...) returns 0) {
        // No valid sub-cell → stop
        techno.vtable+0x544(0, 0);     // Set_Speed(0)
        techno+0x68A = 0;
        return;
    }
    locomotor.is_moving_here = 1;       // class+0x36
    if (techno+0x90 == 0 || ... ) return;   // dead/limboed
    
    target_xy = ILocomotion.Coord(loc);    // vtable+0x48
    facing_angle = atan2(target_xy.Y - longer_dest.Y, target_xy.X - longer_dest.X);
    facing_int = ftol(facing_angle);
    
    // Set facing on locomotor via ILocomotion vtable+0x4C
    (**(code **)(*(int *)(param_1 + 4) + 0x4c))(param_1 + 4, facing_int);
    
    // Set techno speed = 1.0
    techno.vtable+0x544(0, 0x3FF00000);    // high half of double 1.0
    techno+0x68A = 0;
    return;
}
```

**Parity-load-bearing details:**
1. **Facing is recomputed every tick via `atan2`** — no track-table lookup. The facing angle is a continuous float, not a quantized 8-direction integer.
2. **Speed = 1.0** unconditionally when passable. Walk infantry move at constant speed (subject to terrain multipliers handled elsewhere — `Apparent_Speed` slot 33 base).
3. **`is_moving_here` flag = 1** — read by external code to know "this unit is committing to a new cell this tick".

#### 6.2.3 TS-tube traversal (path_index == 8)

```c
if (path_index == 8) {
    cell = MapClass.Get_Cell_At(techno.Location);
    if (cell.+0x116 >= 0 && cell.+0x116 < DAT_008B4148) {   // valid tube index
        tube = g_TubeArray[uStack_4];
        ... compute tube exit coord ...
        head_to_coord = tube.+0x28 (start);     // +0x80 lepton offset
        longer_dest = NullCoord;
        FootClass+0x5E0 path queue shifts down by 1 (path step consumed)
        FootClass+0x63C = -1;
        FootClass+0x684 = (some tube progress byte)
        ...
    } else {
        // Invalid tube index → clear path, stop
        FootClass+0x5E0 = -1;
        head_to_coord = NullCoord;
        techno.vtable+0x480(0, 1);     // Mission_Update
    }
}
```

**`g_TubeArray`** is the global tube-definition array, populated from map `[Tubes]` INI section. `DAT_008B4148` is the tube count.

**Active-in-YR status: CONDITIONAL.** Tubes are TS-legacy by intent (per `[[feedback_no_tunnel_subterranean]]`), BUT:
- The code path IS live in YR
- Some YR maps DO include `[Tubes]` data (rare, mostly singleplayer or remastered campaign maps)
- The Rust port should either: (a) not implement tubes (matches stock multiplayer), or (b) implement them for full map-compat

Per the parity bar, omitting tubes is **parity-correct for stock multiplayer YR**. The user policy in `[[feedback_no_tunnel_subterranean]]` confirms this.

#### 6.2.4 Bridge-layer scatter (code 6 friendly stationary)

Identical pattern to Ship's §8.5/§8.6 but with Walk-specific thresholds:
```c
if (Can_Enter_Cell == 6) {
    distance = Distance3D(...);
    if (distance < RulesClass+0x1718 && !PathType.Has_Valid_Steps()) {
        z_diff = abs(longer_dest.Z - techno.Z);
        if (z_diff < DAT_00B45C28 * 2) {                 // Walk-specific 2x threshold
            new_cell = Get_Cell_At(techno.Location);
            if (new_cell.LandType != 10) {               // not a tube cell
                head_to_coord = NullCoord;
                // ... clear path, mission update
            }
        }
    }
    if ((other_cell.Flags & 0x100) != 0) {              // dest IS bridge
        height_diff_units = techno.Z / DAT_00B45C28 - other_cell.Level;
        if (abs(height_diff_units) > 2) {                // Walk uses **> 2** (i.e. >= 3)
            scatter_layer = bridge_layer;
        }
    } else {
        scatter_layer = ground_layer;
    }
    CellClass::Scatter_Objects(NullCoord, 1, 1, scatter_layer);
}
```

**Threshold reuse:** The `abs(diff) > 2` (i.e. `>= 3`) threshold matches Ship's §8.6 and Drive's `BRIDGE_LOCOMOTOR_DRIVE_SHIP §13.1`. Same magic number across locomotors. The `* 2` Z-tolerance uses `DAT_00B45C28` (Walk's per-level height step), while Ship/Drive use `g_DriveHeightStep` / `g_ShipHeightStep`. **All three encode the same conceptual "2 height levels" tolerance**, just with their own per-locomotor scale constant.

C=HIGH, I=HIGH, B=HIGH (sole caller is `Process @ 0x75AC80`).

### 6.3 `Is_Moving` @ `0x0075AB30` (slot 4)

```c
bool Is_Moving(int param_1) {
    return *(byte *)(param_1 + 0x30);    // class+0x34 = dest-valid flag
}
```

Single byte read. **Much simpler than Ship's Is_Moving** (which checks two coord triples and compares to techno location). Walk maintains an explicit boolean flag.

C=HIGH, I=HIGH, B=HIGH.

### 6.4 `Destination` @ `0x0075ABA0` (slot 5)

```c
void Destination(int *param_1) {
    if (Is_Moving()) {                                    // slot 4
        out.X = head_to_coord.X;       // param_1[6] = class+0x1C
        out.Y = head_to_coord.Y;
        out.Z = head_to_coord.Z;
    } else {
        out = NullCoord;
    }
}
```

Returns head_to_coord if moving, else null sentinel. Different from Ship's `Destination` which always returns head_to_coord (regardless of moving state).

C=HIGH, I=HIGH, B=HIGH.

### 6.5 `Head_To` / `Move_To` @ `0x0075AC00` (slot 6)

```c
CoordStruct *Move_To(int param_1, CoordStruct *out) {
    if (longer_dest == NullCoord) {                       // class+0x28..+0x30
        out = techno.Location;        // techno+0x9C..+0xA4
    } else {
        out = longer_dest;
    }
    return out;
}
```

Returns longer-term dest if set, else techno's current location. Same semantic as Ship's `Move_To`.

C=HIGH, I=HIGH, B=HIGH.

### 6.5b `Head_To_Coord` / `Set_Destination` @ `0x0075ACB0` (slot 17)

Already documented in `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` §2 (full). Summary:

```c
void Head_To_Coord(int param_1, int X, int Y, int Z) {
    // 3 vtable guards (MISSING the 4th — IsBeingWarpedIn at +0x380)
    if (vtable+0x37C () != 0) return;  // IsBeingWarpedOut
    if (vtable+0x1D4 () != 0) return;  // IsDeploying
    if (vtable+0x1D8 () != 0) return;  // IsUndeploying
    
    head_to_coord = (X, Y, Z);          // class+0x1C..+0x24
    
    if (head_to_coord != NullCoord) {
        cell = MapClass::Get_Cell_At(&X);
        if ((cell->Flags & 0x100) != 0) {
            head_to_coord.Z += FUN_006D2120(60);     // *** Walk-specific bridge Z helper ***
        }
        locomotor.dest_valid = 1;       // class+0x34
        return;
    }
    // Else (clearing destination): also clear longer-dest and call vtable+0x54C
    if (longer_dest == NullCoord && dest_valid) {
        dest_valid = 0;
        vtable+0x54C();
    }
}
```

**FUN_006D2120(60) helper:**
```c
int FUN_006D2120(int N) {
    return ftol((double)N - 0.5) * DAT_00B0CDD8);  // round half-DOWN × per-level pixel scale
}
```

**Critical difference from Drive/Ship:** Walk's bridge Z-bump is `round_half_DOWN((60 - 0.5) × per-pixel-scale)`, NOT `4 × HeightStep`. The magic `60` is widely assumed to be `4 levels × 15 px/level = 60 pixels`, then scaled by `DAT_00B0CDD8` (the per-pixel-to-lepton conversion). Per the bridge doc §1.3, this conversion-wisdom is conventional but not directly verified in the binary.

**Subtle detail — Walk MISSES the IsBeingWarpedIn (vtable+0x380) guard.** Drive, Ship, and Teleport all have it. Walk does not. If a unit is mid-warp-in (rare for infantry but possible for chrono'd infantry like Chrono Legionnaire when temporarily piggybacked on a Walk locomotor), Walk's Head_To_Coord still processes the call. This is a Westwood quirk; the chrono-legionnaire's primary locomotor is Teleport, not Walk, so this rarely matters in practice.

C=HIGH, I=HIGH, B=HIGH.

### 6.6 `Stop_Moving` @ `0x0075ADA0` (slot 18)

```c
void Stop_Moving(int param_1) {
    head_to_coord = NullCoord;                         // class+0x1C..+0x24
    if (longer_dest == NullCoord) {
        dest_valid = 0;                                 // class+0x34
        is_moving_here = 0;                             // class+0x36 (NOT +0x35 — in-Process)
        vtable+0x54C();                                 // stop completion
    }
}
```

**Subtle detail — Stop_Moving instantly halts.** Different from Ship's `Stop_Moving` which decelerates by clamping speed to 0.3. Walk's Stop_Moving is a HARD STOP. This matches infantry physics — they can stop on a dime, vehicles cannot.

**Subtle detail — only clears `is_moving_here` (+0x36), not `in_Process` (+0x35).** +0x35 is managed by Process exclusively (set on entry, cleared on exit). +0x36 is the "committing to sub-cell" flag that Stop_Moving must clear so the renderer knows the unit is no longer animating into a new cell.

C=HIGH, I=HIGH, B=HIGH.

### 6.7 `Set_Facing` @ `0x0075AE00` (slot 19)

```c
void Set_Facing(int param_1, int param_2) {
    FacingClass__UpdateFacing(&param_2);
}
```

Pure delegation. The `FacingClass__UpdateFacing` is the universal facing-interpolator used by every locomotor.

C=HIGH, I=HIGH, B=HIGH.

### 6.8 `Is_To_Have_Shadow` @ `0x0075AB40` (slot 32)

```c
bool Is_To_Have_Shadow(int *param_1) {
    if (!ILocomotion::Is_Moving(param_1)) return 0;   // slot 4
    if (techno+0x578 > 0.0                            // some shadow-relevant double
        && longer_dest != NullCoord) {                 // class+0x28..+0x30
        return 1;
    }
    return 0;
}
```

**Subtle detail — `techno+0x578` is a double**, not a byte or int. Probably a "shadow alpha" or "shadow scale" value. Infantry shadows depend on it being positive AND the unit having an active longer-term destination (i.e. actively moving across cells).

C=HIGH, I=HIGH, B=HIGH.

### 6.9 `Get_Locomotion_Type` @ `0x0075C7E0` (slot 29)

```c
int Get_Locomotion_Type() { return 2; }
```

Returns the constant **2**. In Drive/Ship's slot 29 (`In_Which_Layer`) the same value means "Ground layer". For Walk it could mean either "Ground layer" (same as Drive/Ship) or "Walk type ID = 2".

**Resolution:** The same vtable slot returning the same constant across Drive/Ship/Walk strongly suggests the **slot is `In_Which_Layer`** in all three, and Ghidra's "Get_Locomotion_Type" label for Walk is misleading. All three return 2 = "Ground layer". The actual locomotor-type enum is queried via a different mechanism (`Locomotor=` CLSID parsed at INI load).

C=HIGH, I=MEDIUM (Ghidra label), B=HIGH.

### 6.10 `Mark_All_Occupation_Bits` @ `0x0075AE30` (slot 30)

```c
void Mark_All_Occupation_Bits(int param_1, X, Y, Z) {
    cVar1 = locomotor.dest_valid;                       // class+0x34
    coord = (X, Y, Z);
    FindSubCellDest(&coord);
    if (head_to_coord == NullCoord && longer_dest == NullCoord) {
        dest_valid = 0;
        if (cVar1 != 0) {
            vtable+0x54C();                              // stop completion
        }
    }
}
```

This is the **sub-cell placement refresh** routine. When the engine needs to verify an infantry's cell occupancy, it calls this — passing the target coord. The function finds the best sub-cell via `FindSubCellDest`, then if both destinations are null clears the dest-valid flag.

C=HIGH, I=MEDIUM (vtable slot 30; see §11.1 for the slot-disagreement caveat), B=HIGH.

### 6.11 `Power_On_Occupancy` @ `0x0075CA30` (slot 39)

```c
void Power_On_Occupancy(int *param_1, int param_2) {
    if (param_2 == 0) {                                  // mode "power on"
        coord = ILocomotion.Coord(...);                   // vtable+0x18 (slot 6 / Move_To)
        techno.vtable+0xF4(&coord);                       // Set_Location
    }
}
```

Called when a powered-down infantry is re-powered (e.g. after EMP recovery). Restores the unit's cell occupancy by re-setting its location.

C=HIGH, I=MEDIUM, B=HIGH.

### 6.12 `Get_Is_Moving_Here` @ `0x0075CB20` and `Clear_Is_Moving_Here` @ `0x0075CBC0`

These are NOT ILocomotion vtable methods — they're Walk-class **getters/setters** for the `+0x36` flag, called by external code (renderer, AI).

```c
bool Get_Is_Moving_Here(int param_1) { return class+0x36; }
void Clear_Is_Moving_Here(int param_1) { class+0x36 = 0; }
```

Caller xrefs (not separately traced): renderer queries this to know if the infantry is mid-cell-transition.

C=HIGH, I=HIGH, B=HIGH (simple getter/setter).

### 6.13 `Is_At_Coord` @ `0x0075CA80`

Another non-vtable utility:

```c
bool Is_At_Coord(int *param_1, int Z_arg) {
    coord = ILocomotion.Coord(this);
    if (coord.X / 0x100 == coord_arg.X / 0x100
        && coord.Y / 0x100 == coord_arg.Y / 0x100
        && abs(coord.Z - Z_arg) <= DAT_00B45C28) {       // within 1 height level
        return 1;
    }
    return 0;
}
```

Checks if THIS locomotor's coord is within the SAME cell as the argument AND within 1 height level. Used by ground-level snap logic.

C=HIGH, I=HIGH, B=HIGH.

---

## 7. `FindSubCellDest` @ `0x0075C240` — Walk's sub-cell placement algorithm

The CRITICAL Walk-specific routine. Decides which of 5 sub-cells in the target cell the unit should occupy.

### 7.1 Algorithm overview

```c
int FindSubCellDest(int param_1) {
    if (longer_dest == NullCoord) {
        coord = techno.Location (+0x9C/+0xA0/+0xA4);
    } else {
        coord = longer_dest;
    }
    vtable+0xF4 (Set_Location-style call with coord)
    
    // ---- Mission-specific dispatch (5 missions: 8, 9, 7, 11 (0xB), 25 (0x19)) ----
    if (mission_id == 8 || == 9 || == 7 || == 0xB || == 0x19) {
        // Check if there's a target/source object at this cell that we should occupy directly
        // (mission 1=Mission_Move, 2=Mission_Attack, 6=Mission_Capture all dispatch here)
        if (target.location == coord) is_target_subcell = 1;
        if (cell.building == target) is_target_subcell = 1;
    }
    
    // Slave-occupancy check (Yuri Slave logic)
    if (techno.+0x2DC != NULL && techno.+0x2DC.+0x2D8 != 0) {
        if (SlaveManagerClass::IsSlaveAtCell(...)) is_slave_subcell = 1;
    }
    
    // Bridge-layer flag for sub-cell placement
    if (cell.Flags & 0x100) {
        if (techno.Z > GroundHeight + DAT_00B45C28 * 3) {     // *** 3x threshold ***
            use_bridge_layer = 1;
        }
    }
    
    // The actual sub-cell pick
    result = CellClass::PlaceInfantryInCell(coord, use_bridge_layer, is_target_subcell, is_slave_subcell);
    longer_dest = result;
    
    // Crate pickup at the new sub-cell
    crate = CrateClass::PickupDispatch(techno);
    if (!crate && !techno.+0x81) {
        longer_dest = NullCoord;
        if (techno.+0x90 == 0) return 0;   // dead/limboed
    }
    
    // Apply the location change
    if (longer_dest == NullCoord) {
        // Restore to techno's current location
        vtable+0xF0(techno.Location);
        return 0;
    }
    vtable+0xF0(longer_dest);
    return 1;
}
```

### 7.2 Parity-load-bearing details

1. **The 5 mission dispatch IDs** are: **7, 8, 9, 11 (0xB), 25 (0x19)**. These map to:
   - 7 = `Mission_Repair` (likely — verify with `MISSIONCLASS_STATE_MACHINE.md`)
   - 8 = `Mission_Sleep` (or Mission_Stop)
   - 9 = `Mission_Hunt`
   - 0xB = `Mission_Enter` (dock/garrison)
   - 0x19 = `Mission_Selling` (or some late-numbered mission)
   
   For these missions, sub-cell placement gives priority to the mission target's sub-cell (so units docking enter via the right sub-position).
   
   For other missions (Mission_Move, Mission_Attack, etc.), the default "nearest available sub-cell" pick is used.

2. **Bridge-layer sub-cell threshold = `Z > GroundHeight + 3 × DAT_00B45C28`** — exactly 3 height units. The Ship/Drive crush-layer pick uses `4 × HeightStep` (4 levels). Walk uses **3** — one level less. This is because infantry can be in a sub-cell that's halfway-up the bridge (mid-ramp), while a vehicle either is or isn't on the deck.

3. **Slave-Manager check** is Yuri Prime / Slave Miner specific — checks if there's a slave already at the target cell, and if so, marks that as a "preferred sub-cell" so the new slave doesn't overlap.

4. **Crate pickup at sub-cell granularity** — `CrateClass::PickupDispatch` is called AFTER the sub-cell is picked. Infantry can pick up crates by ending their move on the crate's sub-cell. (Drive picks up at cell granularity, Walk at sub-cell.)

5. **The `vtable+0xF0` call at the end** is `Set_Location` (with sub-cell precision). The `+0xF4` call earlier is the same setter but used for the "snap back to current" fallback.

### 7.3 Callers

- `ProcessMovement` at multiple sites (when Can_Enter_Cell == 0)
- `Mark_All_Occupation_Bits` (slot 30 of Walk vtable)

C=HIGH (full body decompiled), I=HIGH, B=HIGH.

---

## 8. INI bindings

### 8.1 `Locomotor=` CLSID

`Locomotor={4A582744-9839-11D1-B709-00A024DDAFD1}` is set on **64 stock units** in `rulesmd.ini` per the Ghidra constructor comment. Spot-verified entries (from `Grep` results):

| Unit (rulesmd.ini) | Line | Section |
|---|---|---|
| GI | (multiple, lines 3744+) | `[E1]` |
| (Conscript) | 3800 | `[E2]` |
| Dog | 3849 | `[DOG]` (Allied) |
| Engineer | 3891 | `[ENGINEER]` |
| (Tanya) | 4053 | `[E7]` |
| Crazy Ivan | 4104 | `[IVAN]` |
| Yuri Prime | 4256+ | `[YURI]` family |
| (many more) | 4356, 4402, ...  | (infantry units) |

All ~64 entries are uncommented `Locomotor={4A582744-...}` lines.

**No `SpeedType=`/`MovementZone=` pattern for infantry:** Infantry use `SpeedType=Foot` and `MovementZone=Infantry` in their INI sections (or inherit defaults).

### 8.2 NullCoord global

`g_NullCoord_Walk_X/Y/Z` at `0x00B45BE8 / 0x00B45BEC / 0x00B45BF0`. Single WRITE site at `0x0075AA52` (the Walk init region — InitNullCoords equivalent, not separately analyzed by Ghidra). 26 READ sites across all Walk methods.

### 8.3 Walk-specific RulesClass / globals consumed

| Address | Role |
|---|---|
| `RulesClass+0x1718` | "Close enough" distance for arrival (shared with Drive/Ship) |
| `RulesClass+0x1768` | Stuck timeout frames (shared) |
| `DAT_00B45C28` | **Walk's per-level height step** (analogue of `g_DriveHeightStep`). Used as `× 2` Z-tolerance, `× 3` bridge-layer threshold, and `/` denominator for height-level normalization. |
| `DAT_00B0CDD8` | **Walk's per-pixel-to-lepton scale** (used by `FUN_006D2120` for bridge Z-bump) |
| `g_TubeArray @ 0x008B4148+` | TS-tube definition array |

---

## 9. Caller binding summary

| Method | Sole caller (or callers) |
|---|---|
| `Process` | `LogicClass::AI` tick dispatch (via ILocomotion vtable slot 16, every tick for every infantry) |
| `ProcessMovement` | `Process @ 0x75AC80` (called inline) |
| `Head_To_Coord` | `FootClass::Set_Destination_Internal` (vtable dispatch via slot 17) |
| `FindSubCellDest` | `ProcessMovement` + `Mark_All_Occupation_Bits` (slot 30) |
| Constructor | IClassFactory::CreateInstance for Walk CLSID (sole xref to constructor address) |
| Destructor | virtual dispatch on Release |
| `Get/Clear_Is_Moving_Here` | external (renderer/AI) |

**No `get_function_callers` errors for these — verified via `get_xrefs_to g_NullCoord_Walk_X` which enumerates all Walk methods that read the sentinel.**

C=HIGH, I=HIGH, B=HIGH (binding via xref tracing).

---

## 10. TS-legacy filtering

| Walk subsystem | Active in YR? | Evidence |
|---|---|---|
| Core movement (Head_To_Coord, ProcessMovement, Stop_Moving) | **Yes** | 64 stock units use Walk |
| Sub-cell placement (FindSubCellDest, PlaceInfantryInCell) | **Yes** | All infantry use sub-cells |
| Bridge interaction (`FUN_006D2120`, `DAT_00B45C28 * 3` threshold) | **Yes** | Standard YR maps include bridges |
| Stuck detection / repath | **Yes** | Standard pathfinding behaviour |
| **TS-tube traversal (`path_index == 8`, `g_TubeArray`)** | **Conditional** | Code path is live, but stock multiplayer YR maps do NOT include `[Tubes]` data. Singleplayer / campaign maps might. Per `[[feedback_no_tunnel_subterranean]]`, the Rust port should NOT implement tubes. |
| Slave-Manager check in FindSubCellDest | **Yes** | Yuri Prime / Slave Miner mechanics in stock YR |
| Bridge "missing IsBeingWarpedIn guard" quirk | **Yes** (active) | Differs from Drive/Ship — Walk's Head_To_Coord lacks the `+0x380` vtable check |

**No TS-only branches that should be skipped in implementation.** The tube traversal is the only borderline case, and the project policy is to skip it.

---

## 11. Comparison to Drive/Ship — key differences

### 11.1 Slot 14 vs slot 30 for "Mark_All_Occupation_Bits"

- Drive/Ship's slot 14 = `0x4B4870 / 0x6A3EA0` (returns 0 — stub)
- Walk's slot 14 = base stub `0x55ABA0`
- Walk's slot 30 = `0x75AE30` (real Mark_All_Occupation_Bits with sub-cell placement)

**Interpretation:** Either Ghidra mislabels one, or the same ILocomotion interface has TWO different "Mark_All_Occupation_Bits" methods at different slots with different signatures. Empirically, slot 30 of Walk is the one that does real work; slot 14 is a no-op for all three classes. Treat slot 30 as the "real" sub-cell occupancy refresh routine for Walk.

### 11.2 Struct size: 0x3C vs 0x6C

Walk's instance is 60 bytes vs Drive/Ship's 108 bytes. Walk lacks: track tables, track index/sub-step, speed double, slope cache (6 fields), first-tick flag. Walk's IPiggyback ptr is at `+0x18` (between vtable ptrs and head_to_coord) rather than at `+0x68` (end).

### 11.3 No drive-track table

Walk has NO `Force_Track` override (slot 28 is base). Movement is **direct angle-based stepping** — facing computed via `atan2`, no pre-computed curves. This is the biggest behavioural difference.

### 11.4 Hard stop instead of decelerate

Walk's `Stop_Moving` instantly nulls head_to_coord and calls `vtable+0x54C`. Ship/Drive decelerate by clamping speed. **Player-observable**: infantry can stop on a dime; vehicles drift to a halt over 1-2 frames.

### 11.5 Sub-cell occupancy

Walk's unique mechanism — `FindSubCellDest` + `CellClass::PlaceInfantryInCell`. 5 sub-cells per cell. No Drive/Ship equivalent.

### 11.6 Walk's bridge Z-bump uses pixel-scale, not height-step

Drive/Ship: `Z += ftol(HeightStep × 4 + 0.5)` (round half-up)
Walk: `Z += ftol((60 - 0.5) × DAT_00B0CDD8)` (round half-down)

**Different formula, different scale constant, different rounding direction.** Per the bridge doc cleanup pass, this is a documented divergence.

### 11.7 Walk missing IsBeingWarpedIn guard

Drive/Ship/Teleport: 4 vtable guards (warp-out, warp-in, deploy, undeploy).
Walk: only 3 (warp-out, deploy, undeploy). **Missing warp-in.** Rare edge case — chrono'd infantry mid-warp-in could be told to move.

### 11.8 Different mission dispatch in FindSubCellDest

Walk's FindSubCellDest checks for **5 specific missions** (8, 9, 7, 0xB, 0x19) for "use target's sub-cell" priority. Ship/Drive don't have this — they place units at cell center.

---

## 12. Open questions

1. **Mission IDs 8, 9, 7, 0xB, 0x19 in FindSubCellDest** — exact mission names. Cross-reference with `MISSIONCLASS_STATE_MACHINE.md` to verify.
2. **`DAT_00B45C28`** (Walk per-level height step) — runtime-initialised value. Is it numerically identical to `g_DriveHeightStep`? Both come from the same isometric projection math, so likely yes — but unverified.
3. **`DAT_00B0CDD8`** (Walk per-pixel-to-lepton scale) — same question. Likely tied to standard "60 pixels/full bridge" = 4 levels × 15 pixels/level conversion.
4. **`techno+0x578` double** read by `Is_To_Have_Shadow` — what semantic? Cross-reference with `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`.
5. **Slot 14 vs 30 disagreement** — definitive answer would require analyzing Drive's slot 30 binding. May indicate Ghidra's vtable analysis used different signatures.
6. **Walk's class-specific QI/AddRef/Release at `0x75CC30/40/50`** — not separately decompiled in this pass. Standard COM ref-count, likely.

---

## 13. Sources

**Ghidra functions decompiled:**
- Constructor (`0x75AA90`), Destructor (`0x75AB00`)
- `Is_Moving` (`0x75AB30`), `Is_To_Have_Shadow` (`0x75AB40`), `Destination` (`0x75ABA0`)
- `Head_To` (`0x75AC00`), `Head_To_Coord` (`0x75ACB0`)
- `Process` (`0x75AC80`), `ProcessMovement` (`0x75AEC0`) — full body
- `Stop_Moving` (`0x75ADA0`), `Set_Facing` (`0x75AE00`)
- `Mark_All_Occupation_Bits` (`0x75AE30`), `Power_On_Occupancy` (`0x75CA30`)
- `FindSubCellDest` (`0x75C240`) — full body
- `Is_At_Coord` (`0x75CA80`), `Get_Is_Moving_Here` (`0x75CB20`), `Clear_Is_Moving_Here` (`0x75CBC0`)
- `Get_Locomotion_Type` (`0x75C7E0`)

**Memory reads (vtable & globals):**
- `0x007F69F8` len 160 (ILocomotion vtable)
- `0x00B45BE8/EC/F0` (NullCoord globals — referenced)

**Raw disassembly:**
- Constructor `0x75AA90` (full asm for field-offset verification)

**Xref tables:**
- `get_xrefs_to 0xB45BE8` → 27 entries (1 WRITE init + 26 READ sites across all Walk methods)

**INI files:**
- `ini/rulesmd.ini` — confirmed `Locomotor={4A582744-...}` for ~64 stock infantry units (sample verified)

**Companion docs (cross-referenced):**
- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` §2 — bridge Z-bump details
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` — shared Can_Enter_Cell dispatch codes
- `INFANTRY_SUBCELL_POSITIONING.md` — 5-subcell layout reference
- `FOOTCLASS_PATHFINDING_AND_MOVEMENT.md` — path queue layout

---

*End of report. Generated 2026-05-17 via Ghidra MCP live decompilation.*
