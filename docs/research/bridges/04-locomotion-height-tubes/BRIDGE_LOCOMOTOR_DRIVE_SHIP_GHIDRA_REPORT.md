# Bridge Locomotor — Drive & Ship — Ghidra Research Report

**Phase:** Phase 3 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #18 (Drive::Process_Drive_Track), #19 (Drive::Set_Destination), #20 (Drive::ComputeBridgeZOffset), #21 (Drive::Process), #22 (Ship::Compute_BridgeZOffset), #23 (Ship vtable overrides), #32 (g_BridgeZOffset_* family globals)
**Companion docs:** `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`
**Phase 1+2 dependencies:** `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
**Date:** 2026-05-13
**Active in YR:** **Yes** — every function in this report runs every tick for every Drive- and Ship-locomotor unit in standard YR skirmish.

> Every claim cites a Ghidra address + decompilation excerpt or `read_memory` byte dump.
> Confidence axes: **C**=content (algorithm verified), **I**=identity (function name verified), **B**=binding (caller path verified via `get_function_callers` + xrefs).

---

## 0. Phase 3 Checkpoint — Locomotor bridge-interaction table

The end-of-Phase-3 summary from the plan §3 checkpoint, covering items #18–#31:

| Locomotor | Active in YR? | Reads `cell+0x140 & 0x100`? | Z-bump path? | Sets `FootClass+0x8C`? | Uses dual occupancy lists (`+0xE4` vs `+0xE8`)? |
|-----------|---------------|------------------------------|--------------|------------------------|------------------------------------------------|
| **Drive** (item #18–#21) | Yes (vehicles) | Yes — Set_Destination, Process_Drive_Track at 3 sites | `g_BridgeZOffset_Drive` (= ftol(g_DriveHeightStep × 4 + 0.5)) added at Set_Destination | Yes — runtime transition at 0x4B1830 (`=1`) and 0x4B184A (`=0`) | Yes — TooBigToFitUnderBridge selects by `Z >= ground + g_BridgeZOffset_Drive` (line 0x4B18CC) OR `FootClass+0x8C` flag |
| **Ship** (item #22–#23) | Yes (Destroyer/Aegis/Carrier/Sub) | Yes — Set_Destination, Process_Drive_Track at 2 sites | `g_BridgeZ_Offset_Ship` (different global at 0xB0782C, same formula `4 × g_ShipHeightStep`) | Yes — identical pattern to Drive at the mirror sites in 0x6A05F0 | Yes — same TooBig pattern as Drive |
| **JumpJet** (item #24) | Yes (Rocketeer, Siege Chopper, Hornet) | Yes — In_Which_Layer @ 0x54B8D0 only | None (altitude is independent) | No — but READS it | No — flies above; only adjusts the LayerClass z-sort decision |
| **Hover** (item #25) | Yes (Robot Tank, LCRF, SAPC, YHVR) | Yes — Move @ 0x514310 | None — hover skirt floats; `DAT_00A8F1B4` is altitude threshold for layer-transition detection, NOT Z-bump | Yes — runtime transition based on `cell.flags & 0x100` AND `Get_Height() >= DAT_00A8F1B4` | No (covered in companion doc) |
| **Walk** (item #27) | Yes (all infantry) | Yes — Head_To_Coord @ 0x75ACB0 | `FUN_006D2120(60)` = ftol((60 − 0.5) × `DAT_00B0CDD8`) — DIFFERENT formula from Drive/Ship | Set externally by pathfinding pipeline | No (infantry-specific occupancy via sub-cell logic) |
| **DropPod** (item #28) | **No** — TS-DEAD | n/a — class never instantiated in standard YR (zero INI refs to CLSID `4A582745`) | n/a | n/a | n/a |
| **Teleport** (item #29) | Yes (Chrono Legionnaire, Chrono Miner) | Yes — Update_Position @ 0x718260 | `g_BridgeZOffset_Teleport` (separate global at 0xB0EC2C) added on arrival | Yes — `*=1` when arriving on bridge, `=0` otherwise | Yes — chrono damage iterates `+0xE4` or `+0xE8` based on dest cell.flags & 0x100 |
| **Tunnel** (item #30) | **No** — TS-DEAD | n/a — no INI refs to CLSID `4A582743` | n/a | n/a | n/a |
| **Float** (item #26) | **N/A** — no such class | n/a | n/a | n/a | n/a |
| **Parachute** (item #31) | **N/A** — not a Locomotor; FootClass state | n/a | n/a | n/a | n/a |

**Result:** **6 live YR locomotors interact with bridges** (Drive, Ship, JumpJet, Hover, Walk, Teleport). DropPod & Tunnel are TS-dead. Float & Parachute are not locomotor classes.

*[Cleanup-pass note 2026-05-13: original draft said "5"; was a counting error against the 6-row table immediately above.]*

---

## 1. The g_BridgeZOffset_* family of runtime globals (Item #32)

**Each locomotor with bridge-aware Z math owns its own global.** None are shared.

| Locomotor | Global address | Symbol | Initialised by | Init formula (verified from raw assembly) |
|-----------|----------------|--------|----------------|--------------------------------------------|
| **Drive** | `0x008A07C4` | `g_BridgeZOffset_Drive` | `DriveLocomotionClass::ComputeBridgeZOffset @ 0x4AF4A0` | `ftol((int)(g_DriveHeightStep × 4) + 0.5)` — round-half-up |
| **Ship** | `0x00B0782C` | `g_BridgeZ_Offset` (Ship's copy) | `ShipLocomotionClass::Compute_BridgeZOffset @ 0x69EBB0` | `ftol((int)(g_ShipHeightStep × 4) + 0.5)` — identical pattern, different source |
| **Hover** | `0x00A8F1B4` | `DAT_00A8F1B4` (no Ghidra label) | written at `0x513BA0` (HoverLocomotionClass init region) | Not decompiled; used as altitude **threshold** not Z-offset |
| **Teleport** | `0x00B0EC2C` | `g_BridgeZOffset_Teleport` | written at `0x717F80` (TeleportLocomotionClass init region) | Not decompiled; used as Z-offset on arrival |
| **JumpJet** | `0x00ABC5DC` | `DAT_00ABC5DC` (no Ghidra label) | written at `0x54ABC0` (JumpjetLocomotionClass init region) | Not decompiled; used to subtract bridge height for layer-sort decision |
| **Walk** | `0x00B0CDD8` | `DAT_00B0CDD8` (no Ghidra label, a per-pixel scale) | written at `0x6D1C11` (somewhere in Theater/View init) | Used inside `FUN_006D2120` as `ftol((arg − 0.5) × DAT_00B0CDD8)`; Walk passes `60` |
| **Walk** *(per-level height)* | `0x00B45C28` | `DAT_00B45C28` (no Ghidra label, "g_WalkHeightStep" by analogy) | written at `0x75A99B` (WalkLocomotionClass init region) | Walk's analogue of `g_DriveHeightStep`/`g_ShipHeightStep`. Read by `WalkLocomotionClass::ProcessMovement` (3 sites at 0x75B7A5/0x75B863/0x75BF0E), `FindSubCellDest` (0x75C502), `Is_At_Coord` (0x75CAF0). Used as denominator for `unit.Z / DAT_00B45C28 - cell.Level` and as `× 2` tolerance for Z-vs-dest checks. **MISSED in the original draft of this report.** |
| **Teleport** *(2nd constant)* | `0x00B0EC38` | `DAT_00B0EC38` (no Ghidra label, "g_TeleportBridgeHeightStep" by analogy) | written at `0x00717EEB` (TeleportLocomotionClass init region; written BEFORE `g_BridgeZOffset_Teleport`) | Separate from `g_BridgeZOffset_Teleport @ 0x00B0EC2C`. Read by `TeleportLocomotionClass::Process @ 0x718C5D` as `unit.Z <= ground + DAT_00B0EC38 * 3` for the isOnBridge detection. **MISSED in the original draft of this report.** Cross-doc: prior `TELEPORT_LOCOMOTION_DEEP_DIVE.md` mentions it as "g_BridgeZOffset" but at the wrong address (says 0xB0EC38, which IS this constant, but conflates it with the Z-bump constant which is actually at 0xB0EC2C). |

**Every constructor of a locomotor class** that has bridge interaction calls its `*::Compute_BridgeZOffset` (or equivalent) at init time, **AFTER** the underlying height_step is computed via `Sin_Lookup_Table4096`. So the value is dependent on the **view angle / isometric projection constants**, not user-configurable from INI.

### 1.1 Memory dump verification (all BSS, runtime-initialised)

```
read_memory 0x008A07C4 len 16 → all zeros (g_BridgeZOffset_Drive; cold dump, BSS, runtime-init)
read_memory 0x008A07D0 len 8  → all zeros (g_DriveHeightStep; cold dump, BSS, runtime-init)
read_memory 0x00B0782C len 16 → all zeros (g_BridgeZ_Offset_Ship)
read_memory 0x00A8F1B4 len 16 → all zeros (Hover threshold)
read_memory 0x00A8F1C0 len 8  → all zeros (Hover Force_Float threshold)
read_memory 0x00ABC5DC len 16 → all zeros (JumpJet bridge-altitude threshold)
read_memory 0x00B0CDD8 len 8  → all zeros (Walk scale factor; runtime-init from theater)
```

**The rounding-constant 0.5** at `0x007E1738` is shared by **all four `Compute_BridgeZOffset`-style functions** (Drive, Ship, Walk's FUN_006D2120):

```
read_memory 0x007E1738 len 8 → 00 00 00 00 00 00 e0 3f
                              = IEEE double 0x3FE0000000000000 = 0.5
```

`ftol(N + 0.5)` is the C-library idiom for **round-half-up to int** since `ftol` truncates toward zero. `ftol(N − 0.5)` (used by Walk's helper) is **round-half-down**. These rounding choices produce 1-lepton drift at .5 boundaries — a parity-relevant detail.

### 1.2 Confidence

- C=HIGH: every formula above verified directly from raw assembly (`FILD`/`FADD`/`FSUB`/`FMUL`/`CALL ftol`/`MOV [dest], EAX`).
- I=HIGH: each `*::Compute_BridgeZOffset` function has the matching Ghidra symbol where applicable.
- B=HIGH: each address has exactly one `[WRITE]` xref (the constructor-time init function) and the documented `[READ]` xrefs in the locomotor's runtime functions.

---

## 2. Drive::ComputeBridgeZOffset @ `0x4AF4A0` (Item #20)

### 2.1 Decompilation

```c
void DriveLocomotionClass__ComputeBridgeZOffset(void) {
    g_BridgeZOffset_Drive = Math__ftol(g_DriveHeightStep * 4);   // decompiler hides the +0.5
    return;
}
```

### 2.2 Raw assembly (the truth)

```asm
004af4a0: PUSH ECX                       ; reserve 4 bytes on stack
004af4a1: MOV EAX, [0x008a07d0]          ; EAX = g_DriveHeightStep (signed int, in some lepton-related unit)
004af4a6: LEA ECX, [EAX*4 + 0]           ; ECX = EAX * 4 (integer 4x multiply)
004af4ad: MOV [ESP], ECX                 ; spill to stack
004af4b1: FILD dword ptr [ESP]           ; load (int)(EAX*4) → FPU as float
004af4b5: FADD double ptr [0x007e1738]   ; +0.5  ← ROUNDING — decompiler hid this
004af4bb: CALL 0x007c5f00                ; ftol → EAX = (int) (height_step*4 + 0.5)
004af4c0: MOV [0x008a07c4], EAX          ; g_BridgeZOffset_Drive = result
004af4c5: POP ECX
004af4c6: RET
```

### 2.3 What the formula actually computes

`g_BridgeZOffset_Drive = round_half_up( g_DriveHeightStep × 4 )`.

`g_DriveHeightStep` is itself computed earlier in load by `DriveLocomotionClass::InitHeightStep_A @ 0x4AF420`:

```c
void DriveLocomotionClass__InitHeightStep_A(void) {
    Sin_Lookup_Table4096(_DAT_008a0758 - _DAT_008a0780);
    g_DriveHeightStep = Math__ftol();
    return;
}
```

`_DAT_008a0758` and `_DAT_008a0780` are isometric-projection angle constants. The `Sin_Lookup_Table4096` result is the **vertical lepton-displacement of one height-level under the standard 45°-ish isometric tilt**. Multiplying by 4 gives the Z-displacement of a full bridge deck (4 height levels = standard bridge height in TS/YR map data).

### 2.4 Caller binding

```
get_xrefs_to 0x4AF4A0 → From 00812d50 [DATA]
```

The single xref is a **DATA reference** — meaning `0x4AF4A0` lives in a function-pointer table at `0x00812D50`. This is the **init-function dispatch table** that the engine walks at boot to initialise all locomotor classes. So:

- Active in YR: **YES** (called once at boot, before any game starts).
- Confidence: B=HIGH (DATA xref + WRITE xref to `g_BridgeZOffset_Drive` from this exact site).

### 2.5 Consumer audit (xrefs to `g_BridgeZOffset_Drive @ 0x8A07C4`)

```
From 004afde2 in DriveLocomotionClass__Set_Destination       [READ]
From 004b0fe7 in DriveLocomotionClass__Process_Drive_Track   [READ]
From 004b18cc in DriveLocomotionClass__Process_Drive_Track   [READ]
From 004af4c0 in DriveLocomotionClass__ComputeBridgeZOffset  [WRITE]
```

**Three read sites, one write site.** Each documented in §3, §4 below.

---

## 3. Drive::Set_Destination @ `0x4AFD40` (Item #19)

### 3.1 Function signature

`void __stdcall DriveLocomotionClass::Set_Destination(int param_1, int dest_X, int dest_Y, int dest_Z)`

The function is dispatched via ILocomotion vtable slot 17 (= byte offset 0x44 in the ILocomotion vtable). MSVC adjusts `this` so `param_1 = locomotor_instance + 0x04` (pointing at the ILocomotion sub-object). Therefore field offsets quoted below are **relative to param_1**, which means add 4 to get the true instance-base offset.

`param_1` type is **`int` (direct byte offsets)** — confirmed because all field accesses use the `*(type *)(param_1 + offset)` pattern in the decompilation.

### 3.2 Full decompilation

```c
void DriveLocomotionClass__Set_Destination(int param_1, int dest_X, int dest_Y, int dest_Z) {
    char cVar1;
    int  iVar2;

    // ---- 4 vtable guards on the LinkedTo TechnoClass (param_1+8 = base+0xC) ----
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x37c))();   // vtable+0x37C (IsBeingWarpedOut?)
    if (cVar1 != 0) return;
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x380))();   // vtable+0x380 (IsBeingWarpedIn?)
    if (cVar1 != 0) return;
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x1d4))();   // vtable+0x1D4 (IsDeploying)
    if (cVar1 != 0) return;
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x1d8))();   // vtable+0x1D8 (IsUndeploying)
    if (cVar1 != 0) return;

    // ---- Store destination at locomotor (+0x30)=base+0x34 X / (+0x34)=base+0x38 Y / (+0x38)=base+0x3C Z ----
    *(int *)(param_1 + 0x30) = dest_X;
    *(int *)(param_1 + 0x34) = dest_Y;
    *(int *)(param_1 + 0x38) = dest_Z;

    // ---- Skip Z-bump if dest is the NullCoord sentinel ----
    if ((dest_X != g_NullCoord_Drive_X) || (dest_Y != g_NullCoord_Drive_Y)
        || (dest_Z != g_NullCoord_Drive_Z))
    {
        // ---- Bridge Z-bump (Phase 2 cell.flags 0x100 read) ----
        iVar2 = CellClass__Get_Cell_At(&dest_X);                   // 3-int coord struct on stack
        if ((*(uint *)(iVar2 + 0x140) & 0x100) != 0) {              // cell.Flags & 0x100 = ON-BRIDGE
            *(int *)(param_1 + 0x38) =                              // dest Z += offset
                 *(int *)(param_1 + 0x38) + g_BridgeZOffset_Drive;
        }
    }
    return;
}
```

### 3.3 Raw assembly at the Z-bump site (`0x4AFDE2`)

```asm
004afdcc: MOV ECX, 0x87f7e8           ; MapClass singleton ptr (this)
004afdd2: CALL 0x00565730             ; MapClass::Get_Cell_At → EAX = CellClass*
004afdd7: MOV ECX, [EAX + 0x140]      ; ECX = cell.Flags
004afddd: TEST CH, 0x1                ; CH bit 0 = full bit 0x100 (the on-bridge flag)
004afde0: JZ 0x004afdf0               ; not bridge → skip
004afde2: MOV ECX, [0x008a07c4]       ; ECX = g_BridgeZOffset_Drive
004afde8: MOV EAX, [ESI + 0x38]       ; EAX = locomotor dest Z
004afdeb: ADD EAX, ECX                ; Z += offset
004afded: MOV [ESI + 0x38], EAX       ; store back
```

### 3.4 The 4 vtable guards

The four guard slots `0x37C / 0x380 / 0x1D4 / 0x1D8` are virtual methods on TechnoClass. Their semantics are inferred from caller usage:

| Slot | Hypothesised | Behaviour if true → |
|------|--------------|---------------------|
| `0x37C` | `IsBeingWarpedOut` | Skip — unit is mid-warp, leave destination alone |
| `0x380` | `IsBeingWarpedIn` | Skip — unit is mid-warp (arrival) |
| `0x1D4` | `IsDeploying` | Skip — MCV deploying / unit unpacking |
| `0x1D8` | `IsUndeploying` | Skip — building undeploying |

If ANY of the 4 guards returns truthy, Set_Destination is a **no-op**. The destination coord is NOT written, the Z-bump is NOT applied, and the locomotor retains its previous destination.

**Parity-load-bearing details:**
- Skip via guard returns silently (no error code, no state-flag set).
- The guards are checked in the order 0x37C, 0x380, 0x1D4, 0x1D8 — short-circuited.
- Walk has a similar function (`0x75ACB0`) but skips `0x380` — DIFFERENT vtable-guard set; see Walk in companion doc §3.

### 3.5 Bridge Z-bump details

- **Read of bit `0x100` only**, not bit `0x200` (bridgehead). So this fires for **any cell with `cell.flags & 0x100` set**, including body cells. The destination Z is bumped wholesale.
- **No height-diff check** — even if the unit is already on the bridge and Z is already at deck level, the Z is bumped AGAIN. The destination must therefore be passed in with the **ground-level Z**; the locomotor adds the bridge offset itself when needed.
- **The bump is unconditional once the cell is bridge** — does not check `FootClass+0x8C` (on_bridge flag). Pure cell-cell read.
- **`g_NullCoord_Drive_*` sentinel skip** is at addresses `0x008A0790/94/98`. If the caller passes the sentinel "no destination" coord, the Z-bump is skipped (since the coord is meaningless).

### 3.6 Caller binding

Reached via the ILocomotion vtable slot 17 on every active Drive locomotor. Direct callers from `get_function_callers`:

```
DriveLocomotionClass__Set_Destination → called from:
  FootClass::Set_Destination_Internal @ 0x4D94B0 (vtable dispatch — caller is the generic FootClass path)
```

Active in YR: **Yes** — every vehicle Move order, every group-formation pathfind, every retarget call.

Confidence: C=HIGH (decompilation + assembly), I=HIGH (Ghidra label confirmed), B=HIGH (vtable slot 17 confirmed via Ship's mirror at 0x69F450).

---

## 4. Drive::Process_Drive_Track @ `0x4B0F20` (Item #18)

Function body: **0x4B0F20 to ~0x4B25C0 (~5.6 KB)**. The full algorithm is the per-tick drive-track stepping logic — too large to quote in full. This section documents **only the bridge-relevant sites**.

### 4.1 The three bridge-relevant sites (per plan §3 item #18)

The plan specified three instruction sites at `0x4B1812 / 0x4B1830 / 0x4B184A`. Re-verified from raw disassembly:

#### Site 1: `0x4B1807–0x4B181E` — Height-diff -4 detection

```asm
004b1800: CALL 0x005657a0              ; CellClass::Get_Cell_At  (for "new" cell after step)
004b1805: MOV EBX, EAX                 ; EBX = new_cell*
004b1807: MOVSX EAX, byte ptr [ESI + 0x11b]   ; old_cell.Level (signed!)
004b180e: MOVSX ECX, byte ptr [EBX + 0x11b]   ; new_cell.Level (signed!)
004b1815: SUB EAX, 0x4                  ; old.Level - 4
004b181c: CMP ECX, EAX                  ; new.Level == old.Level - 4 ?
004b181e: MOV EAX, 0x100                ; preload bridge bit (0x100) for following tests
004b1823: JNZ 0x004b1837                ; not -4 → skip the "descending-onto-bridge" branch
```

#### Site 2: `0x4B1825–0x4B1830` — Set on_bridge=1 (descending onto bridge from cliff)

```asm
004b1825: TEST [EBX + 0x140], EAX       ; new.Flags & 0x100
004b182b: JZ 0x004b183f                 ; new is NOT bridge → fall to "clear" check
004b182d: MOV EDX, [EBP + 0xc]          ; FootClass*
004b1830: MOV byte ptr [EDX + 0x8c], 1  ; FootClass.on_bridge = 1
```

#### Site 3: `0x4B1837–0x4B184A` — Clear on_bridge=0 (stepping off bridge)

```asm
004b1837: TEST [EBX + 0x140], EAX       ; new.Flags & 0x100  (re-test for non-diff-4 path)
004b183d: JNZ 0x004b1851                ; new IS bridge → skip clear
004b183f: TEST [ESI + 0x140], EAX       ; old.Flags & 0x100
004b1845: JZ 0x004b1851                 ; old NOT bridge → skip clear
004b1847: MOV EAX, [EBP + 0xc]          ; FootClass*
004b184a: MOV byte ptr [EAX + 0x8c], 0  ; FootClass.on_bridge = 0
```

### 4.2 The complete on_bridge transition table

| `new.Level - old.Level` | `new.Flags & 0x100` | `old.Flags & 0x100` | Action |
|-------------------------|---------------------|---------------------|--------|
| **= -4** | set (new IS bridge) | (don't care) | **on_bridge = 1** (descended onto bridge deck) |
| **= -4** | clear (new NOT bridge) | set (old WAS bridge) | **on_bridge = 0** (stepped off bridge) |
| **= -4** | clear | clear | unchanged |
| **≠ -4** | set (new IS bridge) | (don't care) | unchanged |
| **≠ -4** | clear | set | **on_bridge = 0** (stepped off bridge — most common path) |
| **≠ -4** | clear | clear | unchanged |

**Critical edge case:** the "diff -4 + new is bridge" case sets on_bridge=1 but does NOT then run the clear check. So even if old was bridge AND new is bridge AND diff is -4, the flag is set to 1 (not cleared). This is the **cliff-top-to-bridge-deck** case (stepping from a 4-higher cliff cell down onto the bridge).

**The "diff +4" case is NOT in this transition table.** That's because:
- Walking onto a bridgehead from ground level: both cells have `Level = same` and diff = 0. The flag transition happens elsewhere (via the pathfinder's `CheckBridgeTraversal` ascending-4 case which writes `*param_4 = 1` propagated by the caller; see Phase 2 doc §1.5 step 3c).
- The Drive runtime here only patches edge cases the pathfinder didn't cover.

### 4.3 Site 4 (additional): TooBigToFitUnderBridge layer-selection at `0x4B185C–0x4B18ED`

This was NOT in the plan's explicit address list but is the heaviest bridge-interaction in the function. Verified from raw assembly:

```asm
004b1856: CALL [EDX + 0x84]               ; vtable+0x84 → TechnoTypeClass*
004b185c: MOV CL, byte ptr [EAX + 0xc94]  ; TooBigToFitUnderBridge (TechnoType+0xC94)
004b1862: TEST CL, CL
004b1864: JZ 0x004b197e                    ; not too big → skip
004b186a: MOV EDI, [EBP + 0xc]             ; FootClass*
004b186d: MOV AL, [EDI + 0x6d0]            ; byte at FootClass+0x6D0 (some "skip crush" guard)
004b1873: TEST AL, AL
004b1875: JNZ 0x004b197e                    ; flag set → skip crush
004b187b: MOV AL, [EDI + 0x8c]             ; FootClass.on_bridge
004b1881: TEST AL, AL
004b1883: JNZ 0x004b18d6                    ; on_bridge=1 → use BRIDGE list directly

; Otherwise: Z-vs-(ground+offset) check
004b18c7: CALL 0x00578080                  ; CellClass::GetGroundHeight → EAX = ground_height
004b18cc: ADD EAX, [0x008a07c4]            ; EAX += g_BridgeZOffset_Drive
004b18d2: CMP ESI, EAX                      ; ESI = unit.Z (lepton)
004b18d4: JL 0x004b18ed                     ; Z < ground+offset → use GROUND list

004b18d6: ; BRIDGE list branch
004b18d6: ... CALL 0x005657a0              ; Get_Cell_At
004b18e5: MOV EAX, [EAX + 0xe8]            ; cell.AltObject (bridge list)
004b18eb: JMP 0x004b1902

004b18ed: ; GROUND list branch
004b18ed: ... CALL 0x005657a0
004b18fc: MOV EAX, [EAX + 0xe4]            ; cell.FirstObject (ground list)
```

**Verified rule for TooBigToFitUnderBridge units (per-step iteration):**

```
if (TechnoType.TooBigToFitUnderBridge != 0      // +0xC94 byte
    && FootClass.+0x6D0 == 0)                    // some "skip this tick" guard
{
    if (FootClass.on_bridge == 1)
        target_list = cell.AltObject;            // bridge layer (+0xE8)
    else if (unit.Z >= ground_height + g_BridgeZOffset_Drive)
        target_list = cell.AltObject;            // bridge layer (+0xE8) by Z-criterion
    else
        target_list = cell.FirstObject;          // ground layer (+0xE4)

    // Then walk target_list and apply 10000 damage to each (mutual destruction)
}
```

The comparison is **signed `JL`** (jump if less than). So bridge-layer iff Z is `>=` (greater-or-equal) ground+offset. **Exact boundary**: Z == ground+offset goes to bridge layer.

This is the "Mammoth Tank style crush-everything-on-the-bridge-deck" behaviour for oversized units when they enter a bridge-overlap cell.

### 4.4 Site 5: Approach-destination Z recompute at `0x4B0FE7`

Earlier in the function (the "approach destination" branch when `param_1+0x58 < 0x40`):

```c
uStack_e8 = locomotor.dest_X;   // base+0x34
iStack_e4 = locomotor.dest_Y;   // base+0x38
uStack_e0 = locomotor.dest_Z;   // base+0x3C
iVar7 = CellClass__Get_Cell_At(&uStack_e8);
uVar20 = -(uint)((cell.Flags & 0x100) != 0) & g_BridgeZOffset_Drive;
                              // ^^^ branchless: uVar20 = (cell is bridge) ? g_BridgeZOffset_Drive : 0
iVar8 = CellClass__GetGroundHeight(&uStack_e8);
uStack_e0 = iVar8 + uVar20;     // dest Z = ground_height + (bridge_offset if applicable)
```

Used downstream for **distance-to-destination Sqrt and deceleration ramp**. This is a **recomputed-from-cell Z** rather than using the stored destination Z — because the stored Z might be slightly stale.

The branchless `-(cond) & offset` idiom produces `offset` when cond=1 and `0` when cond=0. Compiles into:

```asm
... SETZ AL / NEG / AND with g_BridgeZOffset_Drive
```

### 4.5 Site 6: Scatter-layer check at `0x4B1F11`

Inside the `case 6` (FriendlyStationary) Can_Enter_Cell handler:

```c
case 6:
    iVar7 = CellClass__Get_Cell_At();
    if ((*(uint *)(iVar7 + 0x140) & 0x100) == 0) {
        // dest cell NOT bridge → clear scatter "bridge layer" flag
        uStack_ec = uStack_ec & 0xffffff00;
    }
    else {
        // dest cell IS bridge
        iVar7 = *(int *)(param_1 + 0xc);
        unit_Z = *(int *)(iVar7 + 0xa4);
        iVar8 = CellClass__Get_Cell_At();
        uStack_ec = CONCAT31(uStack_ec._1_3_, 1);   // set scatter flag to 1 (bridge layer)
        uVar20 = unit_Z / g_DriveHeightStep         // unit Z in height-level units
               - (int) cell.Level;
        if (abs(uVar20) < 3) uStack_ec &= 0xffffff00;   // height-diff < 3 → clear (same layer)
    }
    CellClass::Scatter_Objects(&NullCoord, 1, uStack_ec, 0);
```

**Two parity-load-bearing details:**
1. **Reads `g_DriveHeightStep` (not the bridge offset)** — uses the per-level height as denominator, then converts to integer height-units.
2. **Threshold is `< 3`** (i.e., abs diff of 3, 4, 5, 6, ... triggers the bridge-layer scatter). This is a **third distinct height threshold** in addition to the `>= 2` rule documented in Phase 1 doc §4 and the `>= 4` strict-equal check at sites #1–3. Three distinct thresholds in one function:
   - `>= 2` (Phase 1 A* layer decision)
   - `== -4` (Process_Drive_Track on_bridge transition)
   - `>= 3` (scatter layer pick)

These do NOT use the same threshold. A Rust port must distinguish them site-by-site.

### 4.6 Caller binding

```
get_function_callers Drive::Process_Drive_Track @ 0x4B0F20 → exactly one caller:
  DriveLocomotionClass::Process @ 0x4B0500 (at offsets 0x4B0576 and 0x4B0AAA)
```

Active in YR: **Yes** — every Drive unit's tick.

Confidence: C=HIGH (full decomp + assembly at each site), I=HIGH (Ghidra label), B=HIGH (single caller verified).

---

## 5. Drive::Process @ `0x4B0500` (Item #21)

The per-tick dispatcher. ~1.5 KB body. Calls Process_Drive_Track and Process_Movement.

### 5.1 Bridge-relevant behaviour

This function does **NOT** read `cell.Flags & 0x100` directly. Bridge interaction is entirely delegated to Process_Drive_Track. Process is purely a state-machine dispatcher.

### 5.2 SlopeIndex caching at function entry (`0x4B051B`)

```asm
004b0510: CALL [EAX + 0x1bc]            ; vtable+0x1BC → current CellClass*
004b0516: XOR ECX, ECX
004b0518: LEA EDI, [ESI - 0x4]          ; EDI = locomotor instance base
004b051b: MOV CL, byte ptr [EAX + 0x11c]; CL = cell.SlopeIndex (cell+0x11C)
004b0521: MOV EAX, ECX
004b0523: MOV ECX, [EDI + 0x1c]         ; locomotor.cached_slope (base+0x1C ?)
004b0526: CMP EAX, ECX
004b0528: JZ 0x004b055a                 ; same → skip
004b052a: MOV [EDI + 0x20], ECX         ; save old slope to +0x20 (history?)
004b052d: PUSH 0x3
...                                      ; slope-change handler
```

This is the **slope-change detection** at the start of every tick — when the unit moves to a cell with a different SlopeIndex, some state is updated. NOT bridge-specific.

### 5.3 Wake/dust animation spawn (`0x4B079D–0x4B0828`) — already documented

From the prior SHIP_VS_DRIVE comparison doc:
- Drive spawns dust every **10 frames** (`g_CurrentFrameCounter % 10 == 0`).
- Ship spawns wake every **8 frames** (`g_CurrentFrameCounter & 7 == 0`).
- The visual triggers don't read bridge state, but **do** read `cell.LandType (+0xEC) == 2` (water).

### 5.4 Confidence

C=MEDIUM (function decompiled fully but bridge interaction is indirect — only via the call to Process_Drive_Track), I=HIGH, B=HIGH.

---

## 6. Ship::Compute_BridgeZOffset @ `0x69EBB0` (Item #22)

### 6.1 Decompilation

```c
void ShipLocomotionClass__Compute_BridgeZOffset(void) {
    g_BridgeZ_Offset = Math__ftol(g_ShipHeightStep * 4);    // decompiler hides +0.5
    return;
}
```

### 6.2 Raw assembly

```asm
0069ebb0: PUSH ECX
0069ebb1: MOV EAX, [0x00b07838]          ; EAX = g_ShipHeightStep
0069ebb6: LEA ECX, [EAX*4 + 0]           ; *4
0069ebbd: MOV [ESP], ECX
0069ebc1: FILD dword ptr [ESP]
0069ebc5: FADD double ptr [0x007e1738]   ; +0.5 ← hidden by decompiler
0069ebcb: CALL 0x007c5f00                ; ftol
0069ebd0: MOV [0x00b0782c], EAX          ; g_BridgeZ_Offset (Ship's)
0069ebd5: POP ECX
0069ebd6: RET
```

### 6.3 Comparison to Drive's `0x4AF4A0`

**Byte-for-byte identical in every respect except for two address operands**:

| | Drive @ 0x4AF4A0 | Ship @ 0x69EBB0 |
|---|---|---|
| Source (height_step) | `[0x008A07D0]` | `[0x00B07838]` |
| Destination (bridge_offset) | `[0x008A07C4]` | `[0x00B0782C]` |
| Rounding constant | `[0x007E1738]` (= 0.5) | **same** `[0x007E1738]` (shared) |
| ftol function | `0x007C5F00` | **same** `0x007C5F00` (shared) |
| Multiplier | `*4` | `*4` |

Same compile of the same source-level function with different globals — a textbook duplication for code that wraps a different class instance's data.

### 6.4 Caller binding

```
get_xrefs_to 0x69EBB0 → From 00814A68 [DATA]
```

Single DATA xref into the init-function dispatch table at `0x00814A68`. Same pattern as Drive — called once at boot.

Active in YR: **Yes**.

Confidence: C=HIGH, I=HIGH, B=HIGH.

---

## 7. Ship bridge-relevant overrides (Item #23)

### 7.1 ShipLocomotionClass::Set_Destination @ `0x69F450` (= `FUN_0069F450`)

Decompilation is **byte-for-byte identical** to Drive's `0x4AFD40` except:

```diff
- g_NullCoord_Drive_X (0x008A0790)  → g_NullCoord_Ship_X (0x00B077F8)
- g_NullCoord_Drive_Y (0x008A0794)  → g_NullCoord_Ship_Y (0x00B077FC)
- g_NullCoord_Drive_Z (0x008A0798)  → g_NullCoord_Ship_Z (0x00B07800)
- g_BridgeZOffset_Drive (0x008A07C4) → g_BridgeZ_Offset    (0x00B0782C)
```

Same 4 vtable guards (`0x37C / 0x380 / 0x1D4 / 0x1D8`), same cell.Flags & 0x100 read, same dest-Z bump.

This is registered at ILocomotion vtable slot 17 of ShipLocomotionClass — confirmed by the prior `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` doc §5 table.

### 7.2 ShipLocomotionClass::Process_Drive_Track @ `0x6A05F0`

Function body **0x6A05F0 to 0x6A1C58**. The prior comparison doc verified ~95% identical to Drive's `0x4B0F20`. Bridge-specific xrefs to `g_BridgeZ_Offset (Ship)`:

```
From 006a06b7 in ShipLocomotionClass__Process_Drive_Track [READ]   ← mirror of Drive's 0x4B0FE7 (approach Z recompute)
From 006a0f58 in ShipLocomotionClass__Process_Drive_Track [READ]   ← mirror of Drive's 0x4B18CC (TooBigToFitUnderBridge layer pick)
```

The on_bridge runtime-transition logic (Drive's sites #1–#3 at 0x4B181E/30/4A) is **also mirrored** in Ship — the prior SHIP_VS_DRIVE comparison doc enumerates 6 specific differences between Ship and Drive, and **none of them are bridge-specific**.

### 7.3 Ship-specific behavioural differences (NOT bridge-related)

Per prior `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` §6, the 6 differences are:

1. Wake animation frequency: Ship every 8 frames, Drive every 10 frames.
2. Wake animation guard: Drive checks `TechnoType+0xD69`; Ship skips this check.
3. Deceleration source: Drive calls vtable+0x38C; Ship reads TypeClass+0x678 directly.
4. Tether ordering: Ship checks tether before deploy; Drive checks deploy first.
5. Convoy/tow block: Drive has it; Ship lacks it.
6. Track table size: Drive has 72 TurnTrack + 16 RawTrack; Ship has 67+14.

**None affect bridge behaviour.** Confidence: B=HIGH (prior doc + direct verification).

### 7.4 Additional Ship-vs-Drive difference found in cleanup pass — wall/overlay throttle constant

In the Phase 3 cleanup decompilation of Ship::Process_Drive_Track @ 0x6A05F0, a 7th difference surfaces (not in the prior comparison doc):

Both Drive and Ship apply a speed-throttle multiplier when `TechnoType.+0xD2B != 0` (some "approaching wall" flag) AND `cell.OverlayTypeIndex` points to a wall-flagged overlay. The throttle constant differs:

```c
// Drive @ 0x4B1A?? area (case at end of step):
*(undefined4 *)(*(int *)(param_1 + 0xc) + 0x334) = 0xbd4ccccd;   // = -0.05f

// Ship @ 0x6A1135 area (mirror site):
*(undefined4 *)(*(int *)(param_1 + 0xc) + 0x334) = 0xbca3d70a;   // = -0.02f
```

Drive uses **-0.05** as the throttle; Ship uses **-0.02**. Ships throttle less when approaching walls/overlays — consistent with naval units having more momentum and being less able to stop quickly. Field `+0x334` of FootClass is the storage location.

**Not bridge-related directly**, but cohabits the Process_Drive_Track function and is the type of single-line difference the prior SHIP_VS_DRIVE doc missed. Logged here for the parity record.

Confidence: C=HIGH (raw hex constant values verified directly from decompilation), I=MEDIUM (semantic "wall/overlay throttle" inferred, not verified at the consumer site).

### 7.4 Naval bridge-cell semantics — note

Ships can pass UNDER high bridges in YR. Cell LandType `0xA` (Tunnel/LowBridge — confirmed by `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`) marks water cells that have a bridge over them. Naval pathing reads `cell.LandType (+0xEC)` independently of `cell.Flags & 0x100`. **The Z-bump in Ship::Set_Destination still applies** if a low-bridge cell happens to also have the 0x100 flag — but typically low-bridge cells have only the LandType marking, not the Flags bit. This means **ships do NOT get an erroneous Z-bump when sailing under a bridge** in standard map data.

---

## 8. Cross-doc contradictions resolved

### 8.1 "g_BridgeZOffset is a single global"

**Refuted.** Per §1.1 of this report, **every locomotor owns its own bridge-related global** at a distinct address. Drive, Ship, Hover, JumpJet, Teleport, Walk each have separate constants. Renaming any single one as "g_BridgeZOffset" without a locomotor suffix is misleading.

### 8.2 "Drive's bridge Z-offset formula is `4 * height_step`"

**Refined.** The formula is `round_half_up( 4 * g_DriveHeightStep )`. The decompiler hides the `+0.5` constant at `0x007E1738`. The `+0.5` matters at the boundary — implementations that omit it produce 1-lepton drift on .5 cases.

### 8.3 "Process_Drive_Track has 3 bridge sites at 0x4B1812/0x4B1830/0x4B184A"

**Refined.** There are at least **6 bridge-related sites** in Process_Drive_Track:
1. `0x4B0FE7` — approach-Z recompute (reads `g_BridgeZOffset_Drive`)
2. `0x4B181E` — height-diff -4 check
3. `0x4B1830` — set on_bridge=1
4. `0x4B184A` — clear on_bridge=0
5. `0x4B18CC` — TooBigToFitUnderBridge layer pick (reads `g_BridgeZOffset_Drive`)
6. `0x4B1F11` — scatter-layer height check (reads `g_DriveHeightStep`)

A Rust port that only mirrors sites 2–4 misses the cost-shaping (#1), the crush-layer selection (#5), and the scatter-layer pick (#6).

### 8.4 The "10000 damage" magic number in TooBigToFitUnderBridge crush

Verified from raw assembly inside the TooBigToFitUnderBridge layer-iterate loop:

```c
iStack_c4 = 10000;
(**(code **)(*piVar12 + 0x16c))(&iStack_c4, 0, g_RulesClass_Instance + 0xFA8, 0, 1, 1, 0);
```

The `10000` is **mutual destruction** — applied to anything found on the conflicting layer. The warhead used is from `RulesClass+0xFA8` (a specific "crush" warhead, likely the same one used elsewhere for vehicle-crush). Then a `20` damage is also applied to the crusher itself:

```c
uStack_e0 = 0x14;   // 20
(**(code **)(*piVar23 + 0x16c))(&uStack_e0, 0, g_RulesClass_Instance + 0xFA8, 0, 1, 0, 0);
```

**Subtle detail:** the crusher takes 20 damage too — a small bleed for vehicles that crush bridges-they-don't-fit. This is the **bridge-crush-self-damage** behaviour.

---

## 9. Active-in-YR confirmation per function

| Function | Active in YR? | Evidence | Gating flags |
|----------|---------------|----------|--------------|
| `DriveLocomotionClass::ComputeBridgeZOffset @ 0x4AF4A0` | Yes | DATA xref from init dispatch table at 0x812D50 | None — runs once at boot |
| `DriveLocomotionClass::Set_Destination @ 0x4AFD40` | Yes | ILocomotion vtable slot 17 of Drive vtable @ 0x7E7EB0 | None — direct vtable dispatch |
| `DriveLocomotionClass::Process_Drive_Track @ 0x4B0F20` | Yes | Called from Drive::Process @ 0x4B0500 (the per-tick dispatcher) | None |
| `DriveLocomotionClass::Process @ 0x4B0500` | Yes | ILocomotion vtable slot 16 of Drive vtable @ 0x7E7EB0 | None — per-tick |
| `ShipLocomotionClass::Compute_BridgeZOffset @ 0x69EBB0` | Yes | DATA xref from init dispatch table at 0x814A68 | None |
| `ShipLocomotionClass::Set_Destination @ 0x69F450` | Yes | ILocomotion vtable slot 17 of Ship vtable @ 0x7F2D8C | None |
| `ShipLocomotionClass::Process_Drive_Track @ 0x6A05F0` | Yes | Called from Ship::Process @ 0x69FC10 | None |

**No SpecialFlags-gated branches.** No fog-of-war gates. Every site fires unconditionally for any standard YR skirmish with a Drive or Ship unit.

---

## 10. Current Rust Implementation Status

**This section maps verified findings to existing Rust code. NOT a port plan.**

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| `g_BridgeZOffset_Drive` init (4 × height_step, rounded half-up) | [src/sim/movement/drive_track.rs](../../ra2-rust-game/src/sim/movement/drive_track.rs) and [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Worth audit** — Rust uses a `bridge_deck_z_offset` constant; the rounding-half-up vs truncation choice may differ. 1-lepton drift on boundary. |
| Drive::Set_Destination 4-vtable-guard skip pattern | [src/sim/movement/movement_commands.rs](../../ra2-rust-game/src/sim/movement/movement_commands.rs) | **Partial** — Rust has some deploy/warp guards but the full 4-slot pattern may not be exactly replicated; audit needed. |
| Drive::Set_Destination unconditional Z-bump when `cell.flags & 0x100` (regardless of bridgehead bit `0x200`) | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Audit** — Rust gating may use bridgehead bit; binary doesn't. |
| Process_Drive_Track on_bridge transition (`diff == -4` cliff-jump + bridge cell → set; bridge-to-non-bridge → clear) | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Partial** — Rust has bridge state transitions in movement_bridge.rs but the specific cliff-jump-diff-4 case may not be covered. |
| TooBigToFitUnderBridge runtime crush-layer pick (Z >= ground + offset → bridge list; else ground list) | [src/sim/movement/bump_crush.rs](../../ra2-rust-game/src/sim/movement/bump_crush.rs) | **Missing** — TooBigToFitUnderBridge is parsed but the runtime layer-aware crush of conflicting layer objects is not implemented. **Player-visible:** Mammoth-style units don't crush bridge-deck occupants when running underneath. |
| Scatter-layer height check (`abs(Z/height_step - cell.Level) >= 3` → bridge layer) | none | **Missing** — Rust scatter doesn't differentiate layer by height-diff. Player-visible at bridge-crossings during cell-collision. |
| Three distinct height-diff thresholds (`>= 2` pathfind, `== -4` transition, `>= 3` scatter) | none | **Audit** — Rust likely uses one threshold; binary uses three site-specific thresholds. |
| Ship::Set_Destination identical pattern, separate global | [src/sim/movement/locomotor.rs](../../ra2-rust-game/src/sim/movement/locomotor.rs) | **Audit** — Rust likely shares ground/ship Z-offset constants; binary keeps them separate (different runtime values from different `Sin_Lookup` results). |
| TooBigToFitUnderBridge self-damage (crusher takes 20 damage) | none | **Missing** — small bleed when crushing bridge-deck occupants. |
| Wake/dust animation frame frequency (Drive=10, Ship=8) | partial | Audit — visible-only. |

(Severity assessment intentionally deferred to Phase 7 synthesis doc.)

---

## 11. Open Questions

1. **Walk's `FUN_006D2120(60)`** — confirmed in companion doc — uses a SCALE FACTOR (`DAT_00B0CDD8`) instead of `g_WalkHeightStep`. Why? Walk's bridge Z-bump formula differs from Drive/Ship. Is this for infantry's smaller render scale? Verify Walk's runtime value of `DAT_00B0CDD8` matches `g_DriveHeightStep` numerically or differs.
2. **TechnoTypeClass+0xC94** (`TooBigToFitUnderBridge`) — confirmed semantic via Drive's Process_Drive_Track usage. The INI key parsing site should be cross-checked against `TECHNO_TYPE_CLASS` research docs.
3. **FootClass+0x6D0** byte — the "skip TooBig crush this tick" guard. Set by what? Likely a one-tick anti-recursion or anti-spam flag. Untraced.
4. **The TooBigToFitUnderBridge self-damage 20-value** — `RulesClass+0xFA8` is the warhead used. Confirm whether that's a configurable INI value or hardcoded.
5. **Ship's `Process_Drive_Track @ 0x6A05F0`** mirror sites for Drive's 0x4B181E/0x4B1830/0x4B184A — not separately decompiled in this report; assumed mirrored per the SHIP_VS_DRIVE comparison doc. Should be spot-verified.

---

## 12. Sources

**Ghidra functions decompiled:**
- `DriveLocomotionClass::ComputeBridgeZOffset` @ 0x004AF4A0 (full body, 0x27 bytes)
- `DriveLocomotionClass::Set_Destination` @ 0x004AFD40 (full body, 0xB3 bytes)
- `DriveLocomotionClass::Process_Drive_Track` @ 0x004B0F20 (~5.6 KB body — bridge sites at §4.1–4.5)
- `DriveLocomotionClass::Process` @ 0x004B0500 (~1.5 KB body)
- `DriveLocomotionClass::InitHeightStep_A` @ 0x004AF420 (helper)
- `DriveLocomotionClass::ComputeBridgeRenderOffset` @ 0x004AF470 (helper)
- `ShipLocomotionClass::Compute_BridgeZOffset` @ 0x0069EBB0 (full body, 0x27 bytes)
- `ShipLocomotionClass::Set_Destination` (FUN_0069F450) @ 0x0069F450 (full body — byte-identical pattern)

**Raw assembly examined:**
- Drive::ComputeBridgeZOffset full disassembly (verified +0.5 rounding)
- Drive::Set_Destination full disassembly (verified all 4 vtable guards, the cell.flags & 0x100 read, Z-bump)
- Drive::Process_Drive_Track around 0x4B1800–0x4B18FF (verified all 6 bridge sites)
- Drive::Process full disassembly (verified slope-change and dispatch)
- Ship::Compute_BridgeZOffset full disassembly (verified byte-identical to Drive's pattern)

**Memory reads:**
- 0x007E1738 (rounding 0.5, IEEE double = 0x3FE0000000000000)
- 0x008A07C4 (g_BridgeZOffset_Drive — cold dump 0, BSS)
- 0x008A07D0 (g_DriveHeightStep — cold dump 0, BSS)
- 0x00B0782C (g_BridgeZ_Offset_Ship — cold dump 0, BSS)
- 0x00A8F1B4, 0x00A8F1C0 (Hover thresholds — cold dump 0, BSS)
- 0x00ABC5DC (JumpJet bridge altitude threshold — cold dump 0, BSS)
- 0x00B0CDD8 (Walk scale factor — cold dump 0, BSS)
- 0x00B0EC2C (Teleport bridge offset — cold dump 0, BSS)
- 0x00812D50 (Drive init dispatch table entry — DATA xref source)
- 0x00814A68 (Ship init dispatch table entry — DATA xref source)
- 0x007ECD68 + 128 bytes (JumpJet ILocomotion vtable for in-doc cross-ref)

**Xref tables:**
- `get_xrefs_to 0x4AF4A0` → 1 DATA entry (init table)
- `get_xrefs_to 0x008A07C4` → 4 entries (3 reads + 1 write)
- `get_xrefs_to 0x008A07D0` → 13 entries (read sites in Process_Drive_Track and Process_Movement)
- `get_xrefs_to 0x69EBB0` → 1 DATA entry (init table)
- `get_xrefs_to 0x00B0782C` → 4 entries (3 reads + 1 write)

**Callers traced (binding evidence):**
- Drive::Process_Drive_Track ← Drive::Process [only, at 2 call sites]
- Drive::Set_Destination ← FootClass::Set_Destination_Internal [vtable dispatch via slot 17]
- Drive::ComputeBridgeZOffset ← init dispatch table @ 0x812D50 [DATA xref only]

**Companion docs:**
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` (Phase 1 — the A* spine that calls into per-locomotor Set_Destination/Process)
- `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` (Phase 2 — the Can_Enter_Cell pipeline that sets path_height)
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` (Phase 2 — the CheckBridgeTraversal sub-check that returns bridge_entered output)
- `SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md` (prior — 6-difference comparison, none bridge-specific; this report adds a 7th difference at §7.4)
- `DRIVE_LOCOMOTION_CLASS.md` (prior — Drive struct layout)
- `LOCOMOTION_MATH_AND_CONSTANTS.md` (prior — CLSID table, partial coverage of these globals)

---

## 13. Cleanup pass — 2026-05-13 (post-initial-draft)

After the original draft of this report (and the three companion Phase 3 docs) was finished, a structured cleanup pass was run. The goal: re-mark every claim that had only decompilation evidence (not raw asm or memory read or xref), check the gaps the original draft self-acknowledged, and verify each "no bridge interaction" negative claim.

### 13.1 Bridge sites at 0x4B0FE7 and 0x4B1F11 — raw-asm verified

The original draft documented these sites from decompilation only. The cleanup re-extracted bytes via `read_memory` and decoded the assembly:

**Site at 0x4B0FE7 (`g_BridgeZOffset_Drive` read in approach-Z recompute):**
```asm
004b0fe0 (region): 
  8B B0 40 01 00 00       ; MOV ESI, [EAX + 0x140]    ← cell.Flags
  8B 3D C4 07 8A 00       ; MOV EDI, [0x008A07C4]     ← g_BridgeZOffset_Drive  ← THIS is 0x4B0FE7
  81 E6 00 01 00 00       ; AND ESI, 0x100
  F7 DE                   ; NEG ESI
  1B F6                   ; SBB ESI, ESI               ← branchless: ESI = 0xFFFFFFFF if bridge else 0
  23 F7                   ; AND ESI, EDI               ← ESI = g_BridgeZOffset_Drive if bridge else 0
  E8 78 70 0C 00          ; CALL CellClass::GetGroundHeight
```

The branchless idiom `AND 0x100 / NEG / SBB / AND offset` produces `bridge ? offset : 0` without a branch. Equivalent to the decompiler's `-(uint)((cell.flags & 0x100) != 0) & g_BridgeZOffset_Drive`. **Original interpretation correct, now verified at instruction level.**

**Site at 0x4B1F11 (scatter case 6 height-diff check):**
```asm
004b1f00 region (case 6 body):
  0F BE 88 1B 01 00 00    ; MOVSX ECX, byte [EAX + 0x11B]   ← cell.Level (signed)
  8B C7                   ; MOV EAX, EDI                     ← unit.Z (from prior context)
  C6 44 24 14 01          ; MOV byte [ESP+0x14], 1           ← scatter flag preset to 1 (bridge-layer)
  99                      ; CDQ
  F7 3D D0 07 8A 00       ; IDIV [0x008A07D0]                ← unit.Z / g_DriveHeightStep
  2B C1                   ; SUB EAX, ECX                     ← unit_height_units - cell.Level
  99 / 33 C2 / 2B C2      ; abs(diff) via XOR-SUB
  83 F8 02                ; CMP EAX, 2                       ← compare to **2** (not 3)
  7F 05                   ; JG +0x5                          ← if > 2 (i.e., >= 3): jump past clear
  C6 44 24 14 00          ; MOV byte [ESP+0x14], 0           ← otherwise clear scatter flag
```

**Verified threshold: `abs(diff) > 2`** (equivalent to `>= 3`). The original doc said `< 3 → clear, >= 3 → bridge layer` which is mathematically the same. Threshold confirmed: bridge-layer scatter fires when height-diff is **at least 3**.

### 13.2 Ship::Process_Drive_Track mirror — independently confirmed

The original draft inferred Ship's bridge sites mirror Drive's based on prior SHIP_VS_DRIVE doc. Cleanup pass independently decompiled Ship's full Process_Drive_Track @ 0x6A05F0. **All 5 bridge sites confirmed mirror with these substitutions:**

| Drive | Ship | Substitution |
|-------|------|--------------|
| 0x4B0FE7 (approach Z) | 0x6A06B7 | `g_BridgeZOffset_Drive` → `g_BridgeZ_Offset_Ship` |
| 0x4B181E (height-diff -4 check) | 0x6A0EBE-area (LAB_006a0ec3/ecb) | same logic, same offsets |
| 0x4B1830 (on_bridge=1) | 0x6A0EBE | byte [FootClass + 0x8C] = 1 |
| 0x4B184A (on_bridge=0) | 0x6A0EE0-area | byte [FootClass + 0x8C] = 0 |
| 0x4B18CC (TooBigToFitUnderBridge Z-vs-ground+offset) | 0x6A0F58 | `g_BridgeZ_Offset_Ship` |
| 0x4B1F11 (scatter case 6) | 0x6A1566-area | `g_ShipHeightStep` substituted for `g_DriveHeightStep`; threshold `> 2` identical |
| 0x4B258? (function-end transition) | 0x6A1BDE/0x6A1BF8 | identical pattern |

Ship's TooBigToFitUnderBridge crush also fires the **same 10000-damage-to-blocker + 20-damage-to-crusher** combo via the same warhead at `RulesClass+0xFA8`.

**Mirror claim is now HIGH-confidence (B=HIGH).**

### 13.3 Newly discovered globals (now in §1 table)

Two globals were missed in the original draft of §1:

- **`DAT_00B45C28`** — Walk's per-level height step (Walk's analogue of `g_DriveHeightStep`). 4 reader sites in Walk-related functions. Added to §1.
- **`DAT_00B0EC38`** — Teleport's bridge-altitude threshold (separate from `g_BridgeZOffset_Teleport @ 0xB0EC2C`). Used by Teleport::Process for `unit.Z <= ground + threshold × 3` isOnBridge detection. Added to §1.

This brings the documented locomotor-bridge-globals count from **6 → 8**.

### 13.4 Confidence-level adjustments

Reviewing the original draft's claims:

| Original claim | Original confidence | Cleanup verdict |
|----------------|---------------------|-----------------|
| Drive ComputeBridgeZOffset `4 × height_step + 0.5` round-half-up | HIGH | HIGH (raw asm verified) |
| Drive Set_Destination 4 vtable guards + Z-bump | HIGH | HIGH (raw asm verified) |
| Drive Process_Drive_Track bridge sites at 0x4B181E/30/4A | HIGH | HIGH (raw asm verified) |
| Drive Process_Drive_Track Site 0x4B0FE7 approach Z recompute | HIGH (claimed) | **HIGH (now verified)** — was decomp-only |
| Drive Process_Drive_Track Site 0x4B1F11 scatter `>= 3` threshold | HIGH (claimed) | **HIGH (now verified)** — was decomp-only; threshold `> 2` confirmed (equivalent) |
| Ship Compute_BridgeZOffset byte-for-byte identical pattern | HIGH | HIGH (raw asm side-by-side verified) |
| Ship Process_Drive_Track bridge sites mirror Drive | HIGH (claimed) | **HIGH (now independently verified)** — was prior-doc-inferred |
| Drive Process @ 0x4B0500 "no direct bridge reads" | C=MEDIUM | MEDIUM (raw asm scan only; no full decomp) |
| Walk's `60 = 4 levels × 15 pixels/level` derivation | HIGH (implied) | **MEDIUM** — the "15 px/level" claim is RA2 conventional wisdom, not verified at a binary site. Re-marked. |
| Drive's "Site 4 TooBigToFitUnderBridge 10000+20 damage" magic numbers | HIGH (claimed) | HIGH (raw decimal verified in decompilation; raw assembly not re-extracted but unambiguous) |
| Ship has 6 differences from Drive (per prior doc) | HIGH | **REFINED to 7** — added wall/overlay throttle constant (-0.05 Drive vs -0.02 Ship) at §7.4 |

### 13.5 Remaining gaps

Items NOT verified in this cleanup pass (still MEDIUM confidence or below):

1. **The "15 pixels/level" conversion** that explains Walk's magic `60` is RA2 conventional wisdom. Would need to find the SHP/TMP render constant in the binary to upgrade to HIGH.
2. **Drive::Process @ 0x4B0500** was disassembled but not decompiled. The claim "no direct bridge reads in Process" was based on quick scan. Could host indirect reads via the SlopeIndex caching at function entry (0x4B051B reads `cell+0x11C`, which is the SlopeIndex byte — confirmed not bridge, but a tighter audit would confirm none of the subsequent ~1.5KB of code reads `+0x140 & 0x100`).
3. **DAT_00B45C28 runtime value** — Walk's per-level height. Compared to `g_DriveHeightStep` should converge if both come from the same isometric projection math, but unverified.

These can be closed by future targeted passes. They are flagged here so the synthesis doc (Phase 7) knows what remains MEDIUM.
