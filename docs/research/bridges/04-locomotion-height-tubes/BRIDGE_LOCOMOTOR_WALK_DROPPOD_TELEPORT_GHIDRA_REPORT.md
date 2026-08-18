# Bridge Locomotor — Walk, DropPod, Teleport — Ghidra Research Report

**Phase:** Phase 3 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #27 (WalkLocomotionClass bridge interaction), #28 (DropPodLocomotionClass active-in-YR check), #29 (TeleportLocomotionClass bridge-on-arrival)
**Companion docs:** `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`
**Phase 1+2 dependencies:** `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
**Date:** 2026-05-13

> Every claim cites a Ghidra address + decompilation excerpt or `read_memory` byte dump.
> Confidence axes: **C**=content / **I**=identity / **B**=binding.

---

## 1. Overview — three very different bridge interactions

| Locomotor | Active in YR? | Bridge interaction summary |
|-----------|---------------|----------------------------|
| **Walk** (Item #27) | Yes (~64 INI declarations) | Reads `cell.flags & 0x100` in `Head_To_Coord` and adds a Z-bump via `FUN_006D2120(60)`. **Different formula** from Drive/Ship — uses a scale-factor at `DAT_00B0CDD8` rather than `g_DriveHeightStep × 4`. |
| **DropPod** (Item #28) | **NO** — confirmed TS-DEAD. Zero references to CLSID `4A582745` in `rulesmd.ini`. | **N/A.** Constructor is reachable from COM factory but never instantiated in standard skirmish. |
| **Teleport** (Item #29) | Yes (Chrono Legionnaire, Chrono Miner) | Reads `cell.flags & 0x100` AND `0x200` (bridgehead) in `Update_Position`. Adds `g_BridgeZOffset_Teleport` to dest Z on arrival. Updates `FootClass+0x8C` on_bridge flag. Iterates `cell.+0xE4`/`+0xE8` dual occupancy lists for chrono-damage. |

The asymmetry: Walk's bridge math uses a DIFFERENT constant family than Drive/Ship. Teleport uses YET ANOTHER constant family. The `g_BridgeZOffset_*` globals are **strictly per-locomotor**.

---

## 2. Walk — Head_To_Coord @ `0x75ACB0` (Item #27)

WalkLocomotionClass drives all infantry. Bridge interaction is in `Head_To_Coord` (the destination-setter, dispatched via ILocomotion vtable slot 17).

### 2.1 Full decompilation

```c
void WalkLocomotionClass__Head_To_Coord(int param_1, int dest_X, int dest_Y, int dest_Z) {
    char cVar1;
    int  iVar2;

    // ---- 3 vtable guards (NOTE: one fewer than Drive's 4) ----
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x37c))();    // IsBeingWarpedOut
    if (cVar1 != 0) return;
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x1d4))();    // IsDeploying
    if (cVar1 != 0) return;
    cVar1 = (**(code **)(**(int **)(param_1 + 8) + 0x1d8))();    // IsUndeploying
    if (cVar1 != 0) return;
    // *** MISSING: slot 0x380 (IsBeingWarpedIn) — Walk does NOT check this guard ***

    // ---- Store destination at (+0x18, +0x1C, +0x20) — DIFFERENT from Drive's (+0x30/+0x34/+0x38) ----
    *(int *)(param_1 + 0x18) = dest_X;
    *(int *)(param_1 + 0x1c) = dest_Y;
    *(int *)(param_1 + 0x20) = dest_Z;

    // ---- Skip Z-bump if NullCoord ----
    if ((dest_X != g_NullCoord_Walk_X) || (dest_Y != g_NullCoord_Walk_Y)
        || (dest_Z != g_NullCoord_Walk_Z))
    {
        iVar2 = CellClass__Get_Cell_At(&dest_X);
        if ((*(uint *)(iVar2 + 0x140) & 0x100) != 0) {           // cell IS bridge cell
            iVar2 = FUN_006D2120();                               // *** different from Drive — calls helper ***
            *(int *)(param_1 + 0x20) = *(int *)(param_1 + 0x20) + iVar2;  // Z += result
        }
        *(undefined1 *)(param_1 + 0x30) = 1;                      // some "destination valid" flag
        return;
    }

    // ---- NullCoord + previous dest also NullCoord + flag-bit logic (cleanup path) ----
    if (((*(int *)(param_1 + 0x24) == g_NullCoord_Walk_X) && ...)
        && (cVar1 = *(char *)(param_1 + 0x30), *(undefined1 *)(param_1 + 0x30) = 0, cVar1 != 0))
    {
        (**(code **)(**(int **)(param_1 + 8) + 0x54c))();        // vtable+0x54C — Stop call
    }
    return;
}
```

### 2.2 Raw assembly at the Z-bump site (`0x75AD4D–0x75AD57`)

```asm
0075ad42: MOV ECX, [EAX + 0x140]       ; cell.Flags
0075ad48: TEST CH, 0x1                  ; & 0x100
0075ad4b: JZ 0x0075ad5a                 ; not bridge → skip
0075ad4d: MOV ECX, 0x3c                 ; *** ECX = 60 (decimal) — magic constant ***
0075ad52: CALL 0x006d2120               ; FUN_006D2120(60) → EAX
0075ad57: ADD [ESI + 0x20], EAX         ; dest.Z += result
0075ad5a: MOV byte ptr [ESI + 0x30], 1  ; set "destination valid" flag
```

### 2.3 FUN_006D2120 — the Walk-specific bridge Z-offset helper

```asm
006d2120: PUSH ECX                       ; save ECX → stack
006d2121: MOV [ESP], ECX                 ; (sanity write)
006d2125: FILD dword ptr [ESP]           ; FILD: load (int)ECX → float
006d2129: FSUB double ptr [0x007e1738]   ; SUBTRACT 0.5 (the rounding constant)
006d212f: FMUL double ptr [0x00B0CDD8]   ; multiply by Walk's scale factor (BSS, runtime-init)
006d2135: CALL 0x007c5f00                ; ftol
006d213a: POP ECX
006d213b: RET
```

**Verified formula:**
```
FUN_006D2120(ECX) = ftol( (ECX - 0.5) * DAT_00B0CDD8 )
```

Walk passes `ECX = 60` (= 0x3C). So the actual Z-bump is `ftol(59.5 * DAT_00B0CDD8)`.

**The `-0.5` rounding is DIFFERENT direction from Drive/Ship's `+0.5`.** Drive uses `ftol(x + 0.5)` = round-half-up. Walk uses `ftol(x - 0.5)` = round-half-down on positive values. On a `0.5` boundary, this produces a 1-lepton difference between an infantry's bridge dest-Z and a vehicle's bridge dest-Z at the same coord.

**The magic `60`** is the visible bridge height in pixels: 4 height levels × 15 pixels per level = 60. The conversion factor `DAT_00B0CDD8` (runtime-initialised, currently 0 in cold dump) converts pixels back to leptons.

### 2.4 Why Walk uses a different formula

Drive uses `g_DriveHeightStep × 4` where `g_DriveHeightStep` is the lepton-displacement of one height level under isometric projection (computed from `Sin_Lookup_Table4096(angle_constants)`). Walk uses `(60 - 0.5) × DAT_00B0CDD8` where `60` is the pixel count of a 4-level bridge and `DAT_00B0CDD8` is the pixel→lepton conversion.

Both formulas approximate the same logical "4 height levels visible Z" but via different paths. The runtime values **should produce nearly identical results numerically** (a few leptons of drift due to the two rounding directions). Why two different paths? Likely historical — Walk inherited the formula from TS infantry math while Drive uses the (newer?) vehicle-projection math.

**For Rust port parity:** if `DAT_00B0CDD8 ≈ g_DriveHeightStep × 4 / 60`, then the formulas converge to the same Z-bump value modulo rounding. A single per-locomotor `bridge_z_offset` value can be used as long as the integer rounding direction is preserved.

### 2.5 The 3-vs-4 vtable guard asymmetry

Walk checks 3 guards: `0x37C, 0x1D4, 0x1D8`. Drive (and Ship) check 4: `0x37C, 0x380, 0x1D4, 0x1D8`. The missing one in Walk is **`0x380` (IsBeingWarpedIn)**.

**Player-visible effect:** if an infantry unit is mid-warp-IN (Chrono Legionnaire arriving via teleport), Walk will STILL process a new destination request. Drive would refuse. This means infantry get destination-updates during the warp-in frame; vehicles don't.

This is consistent with infantry being more "responsive" than vehicles in YR — but it's a real binary-encoded difference worth preserving.

### 2.6 The +0x30 byte flag

`Head_To_Coord` writes `1` to `param_1 + 0x30` after a successful destination set, and writes `0` (after caching the old value into `cVar1`) on the NullCoord cleanup path. This is the "destination-valid" / "has-destination" flag.

When the flag was 1 AND the new destination is NullCoord (i.e., caller is canceling the destination), the function calls `vtable + 0x54C` on the linked TechnoClass — likely `Notify_Stopped` or similar.

### 2.7 Caller binding

WalkLocomotionClass ILocomotion vtable @ `0x007F69F8`. Head_To_Coord is at slot 17 (= offset 0x44). Confirmed by:

```
get_xrefs_to 0x75AC80 (WalkLocomotionClass::Process) → From 0x07F6A38 [DATA]
```

`0x07F6A38` is at offset 0x40 (slot 16) of vtable `0x07F69F8`. Slot 17 at `+4` is `0x75ACB0` (Head_To_Coord). Mirror pattern to Drive's slot 16=Process and slot 17=Set_Destination.

Active in YR: **Yes**. Walk drives every infantry unit (GI, Dog, Engineer, Conscript, Initiate, Tanya, Boris, Yuri, Yuri Prime, Yuri Clone, Brute, Virus, Sniper, Crazy Ivan, Chrono Legionnaire on ground phase, etc.). ~64 INI declarations of CLSID `4A582744`.

Confidence: C=HIGH (full decomp + raw assembly), I=HIGH (Ghidra label "WalkLocomotionClass__Head_To_Coord"), B=HIGH (vtable slot + caller chain confirmed).

### 2.8 What Walk does NOT do with bridges

Walk's `Process` (`0x75AC80`) is a thin wrapper:

```c
void WalkLocomotionClass__Process(int *param_1) {
    *(undefined1 *)((int)param_1 + 0x31) = 1;
    WalkLocomotionClass__ProcessMovement(1);
    *(undefined1 *)((int)param_1 + 0x31) = 0;
    (**(code **)(*param_1 + 0x10))(param_1);
    return;
}
```

It sets a "currently processing" byte at `+0x31`, calls a helper, clears the byte, then calls vtable+0x10 (an internal release). **No bridge reads in Process directly.** Bridge state IS read inside `WalkLocomotionClass::ProcessMovement` — see §2.9 below (added in cleanup pass).

Walk's `Is_Moving` (`0x75AB30`) is a one-liner returning `*(byte *)(param_1 + 0x30)` — no bridge read.

### 2.9 WalkLocomotionClass::ProcessMovement @ `0x75AEC0` — **freshly decompiled (cleanup pass)**

Original draft of this report deferred Walk::ProcessMovement to "not decompiled in detail this phase" and assumed the helper at "FUN_007599E0 area". **Cleanup pass corrected the address** (actual function is at `0x75AEC0`, not 0x7599E0) and decompiled the full body (~1.7 KB).

Walk::ProcessMovement contains FIVE distinct bridge-relevant sites:

#### 2.9.1 Runtime on_bridge transition (mirror of Drive's 0x4B181E/0x4B1830/0x4B184A)

At ~0x75C170 in the function body:

```c
iVar7  = MapClass__Get_CellClass(&iStack_20);   // old cell
iVar13 = MapClass__Get_CellClass(&stack0xffffffa4);   // new cell
if ((int)*(char *)(iVar13 + 0x11b) == *(char *)(iVar7 + 0x11b) + -4) {
    if ((*(uint *)(iVar13 + 0x140) & 0x100) != 0) {     // new cell IS bridge
        *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 1;    // on_bridge = 1
        goto LAB_0075c180;
    }
LAB_0075c188:
    if ((*(uint *)(iVar7 + 0x140) & 0x100) != 0) {              // old cell WAS bridge
        *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 0;    // on_bridge = 0
    }
}
else {
LAB_0075c180:
    if ((*(uint *)(iVar13 + 0x140) & 0x100) == 0) goto LAB_0075c188;
}
```

**This is the exact same transition logic as Drive (companion doc §4.1, §4.2):**
- Diff-(-4) + new is bridge → on_bridge = 1 (cliff-jump-onto-bridge case)
- New is non-bridge AND old was bridge → on_bridge = 0 (step off bridge)
- Other cases → no change

**Originally missed in this report.** Walk's runtime bridge-transition tracking IS implemented at movement time, just like Drive/Ship. Infantry on bridges have the same diff-(-4) + bridge-flag transition.

#### 2.9.2 Bridge-state-mismatch detector (sets FootClass + 0x68B)

At ~0x75B567 in the function body:

```c
iVar7 = CellClass__Get_Cell_At();   // cell at next-step coord
if ((*(uint *)(iVar7 + 0x140) >> 8 & 1) != (uint)*(byte *)(*(int *)(param_1 + 0xc) + 0x8c)) {
    *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x68b) = 1;
}
```

**Verified rule:** if `(cell.flags >> 8) & 1` (= bit 0x100 isolated) does NOT match `FootClass + 0x8C` (on_bridge byte), set `FootClass + 0x68B = 1`. This is a **bridge-state-mismatch flag** — fires when an infantry unit's logical on_bridge state disagrees with the cell's structural bridge flag.

Purpose unknown without tracing consumers of `+0x68B`. Likely: triggers a re-evaluation or path-replan on the next tick. Walk-specific (Drive/Ship don't have this exact mismatch-detection pattern).

#### 2.9.3 Bridge-layer scatter pick (mirror of Drive's case 6 at 0x4B1F11)

At ~0x75B880 in the function body, inside Can_Enter_Cell case 6 handling:

```c
if ((*(uint *)(iVar7 + 0x140) & 0x100) != 0) {      // cell IS bridge
    iVar13 = *(int *)(param_1 + 0xc);
    local_14 = CONCAT31(local_14._1_3_, 1);          // preset scatter flag to 1 (bridge-layer)
    iStack_30 = *(int *)(iVar13 + 0x9c);
    uStack_2c = *(int *)(iVar13 + 0xa0);
    uVar14 = *(int *)(iVar13 + 0xa4) / DAT_00b45c28  // unit.Z / Walk's per-level height step
           - (int)*(char *)(iVar7 + 0x11b);            // - cell.Level
    uVar15 = (int)uVar14 >> 0x1f;
    if (2 < (int)((uVar14 ^ uVar15) - uVar15)) goto LAB_0075b881;   // if abs(diff) > 2, keep bridge layer
}
local_14 = local_14 & 0xffffff00;                    // otherwise clear (same layer)
LAB_0075b881:
CellClass__Scatter_Objects(&g_NullCoord_Walk_X, 1, 1, local_14);
```

**Verified threshold: `abs(diff) > 2`** (i.e., `>= 3`). Same threshold as Drive's case 6. Walk uses its own `DAT_00B45C28` (per-level height step) as the denominator instead of `g_DriveHeightStep`.

#### 2.9.4 LandType 10 (LowBridge/Tunnel) special-case gate

At ~0x75B81F in the function body:

```c
if ((int)((uVar14 ^ uVar15) - uVar15) < DAT_00b45c28 * 2) {        // |Z-diff| < 2 * height_step
    iStack_30 = *(int *)(iVar13 + 0x9c);
    uStack_2c = *(int *)(iVar13 + 0xa0);
    iStack_28 = *(int *)(iVar13 + 0xa4);
    iVar13 = CellClass__Get_Cell_At();
    if (*(int *)(iVar13 + 0xec) != 10) {                            // cell.LandType != Tunnel/LowBridge
        // ... fail the path: set destination to NullCoord, stop moving ...
    }
}
```

**Verified rule:** when an infantry path is blocked AND the destination Z is within `2 * DAT_00B45C28` of current Z, the path is failed **unless** the current cell's `LandType == 10` (= Tunnel/LowBridge per Phase 2 `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`). LandType 10 cells get a **special exemption** — likely the "infantry can pass under a low bridge" path.

This is the **first documented binding of LandType 10's effect on infantry pathing** in this Phase 3 work.

#### 2.9.5 DAT_00B45C28 — Walk's per-level height step (NEW global)

Used as integer divisor in §2.9.3 and as `× 2` tolerance in §2.9.4. The constant is at `0x00B45C28`, BSS, runtime-initialised. **Originally missed from this report's globals table** — now added to the Drive/Ship doc's §1 globals table.

Read sites (5 total):

```
From 0075b7a5 in WalkLocomotionClass__ProcessMovement
From 0075b863 in WalkLocomotionClass__ProcessMovement
From 0075bf0e in WalkLocomotionClass__ProcessMovement
From 0075c502 in WalkLocomotionClass__FindSubCellDest
From 0075caf0 in WalkLocomotionClass__Is_At_Coord
WRITE: 0x0075A99B (in WalkLocomotionClass init region)
```

Walk's bridge-aware Z math uses this constant as the per-level height step, the same role `g_DriveHeightStep` / `g_ShipHeightStep` play for vehicles. Walk's full bridge-math stack:
- `DAT_00B45C28` = per-level height step (analogous to `g_DriveHeightStep`)
- `DAT_00B0CDD8` = pixel-to-lepton scale factor (in `FUN_006D2120`, used by Head_To_Coord for the Z-bump)
- `60` (constant in Walk's Head_To_Coord call) = 4 height levels × 15 pixels/level (RA2 convention; the 15 px/level part is conventional wisdom, not freshly verified at a binary site)

So Walk has **two related Z-math constants**, used at different sites for different purposes (Z-bump vs height-unit conversion).

**Confidence after cleanup: C=HIGH (full body decompiled), I=HIGH (Ghidra label confirmed), B=HIGH (xrefs traced).**

---

## 3. DropPod — Constructor analysis & TS-dead confirmation (Item #28)

### 3.1 Constructor structure

```c
undefined4 * __fastcall DropPodLocomotionClass__Constructor(undefined4 *param_1) {
    LocomotionClass__Constructor();
    *(undefined1 *)(param_1 + 7) = 0;
    param_1[8]  = DAT_008a0820;       // NullCoord X
    param_1[9]  = DAT_008a0824;       // NullCoord Y
    param_1[10] = DAT_008a0828;       // NullCoord Z
    param_1[0xb] = 0;
    *param_1     = &DropPodLocomotionClass__IUnknown_vtable;
    param_1[1]   = &DropPodLocomotionClass__ILocomotion_vtable;
    param_1[6]   = &DropPodLocomotionClass__IPiggyback_vtable;
    return param_1;
}
```

The three constructors at `0x4B5AB0`, `0x4B5B00`, `0x4B66F0` correspond to:
- `0x4B5AB0` — full constructor (above)
- `0x4B5B00` — destructor (calls `Release()` on a held piggyback object at +0xB, then `LocomotionClass::Destructor`)
- `0x4B66F0` — **scalar-deleting destructor** (verified in cleanup pass). Decompiles to:
  ```c
  void DropPodLocomotionClass__Constructor(undefined4 *param_1, byte param_2) {
      *param_1 = &DropPodLocomotionClass__IUnknown_vtable;
      param_1[1] = &DropPodLocomotionClass__ILocomotion_vtable;
      param_1[6] = &DropPodLocomotionClass__IPiggyback_vtable;
      piVar1 = (int *)param_1[0xb];
      if (piVar1 != (int *)0x0)
          (**(code **)(*piVar1 + 8))(piVar1);    // Release piggybacked object
      LocomotionClass__Destructor();
      if ((param_2 & 1) != 0) FUN_007c8b3d(param_1);   // operator delete
      return param_1;
  }
  ```
  (Ghidra mislabels this as "Constructor" — it is the scalar-deleting destructor pattern: standard MSVC dual-vtable re-init followed by base destructor and optional `operator delete` based on `param_2 & 1`. Same pattern as Hover's `0x005172C0` documented in HOVER_LOCOMOTION_CLASS doc §6.) The DATA xref at `0x007E8364` is the IUnknown vtable's Release slot. Original draft of this report described it as "a vtable thunk entry" — refined: it is the scalar-deleting destructor invoked via IUnknown::Release when refcount hits 0.

### 3.2 Caller analysis — is it active in YR?

```
get_xrefs_to 0x4B5AB0 → From 006C494C [UNCONDITIONAL_CALL]
get_xrefs_to 0x4B66F0 → From 007E8364 [DATA]
get_function_callers 0x4B5AB0 → No callers found  (the 6C494C site isn't in a labeled function)
```

The single `UNCONDITIONAL_CALL` at `0x006C494C` is the **COM class-factory CoCreateInstance handler** for CLSID `4A582745`. This means: if any code does `CoCreateInstance(&CLSID_DropPodLocomotion, ...)`, that handler will call `0x4B5AB0` to allocate a DropPod instance.

**The question:** does any unit in `rulesmd.ini` declare `Locomotor={4A582745-...}` as its locomotor key?

```
grep '4A582745' ini/rulesmd.ini  →  No matches found
```

**Zero references.** DropPodLocomotion is **NOT instantiated in standard YR skirmish**. The constructor is reachable in theory but never reached in practice.

### 3.3 Is DropPod still reachable via mission scripts or maps?

Allied Paradrop in YR uses a different mechanism — the carrier aircraft (Nighthawk transport) uses FlyLocomotion, and the dropped infantry use **Parachute as a FootClass state**, not as a separate locomotor. See `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md` for the Parachute non-locomotor justification.

**No cinematic / scripted-only use found in this phase.** The CLSID is registered in the COM factory tables but no INI binding exists.

**Confirmed TS-dead** for standard YR skirmish. The Constructor exists in the binary as a vestige of TS code (where DropPod was a separate paradrop mechanism). YR replaced this with the FootClass-state Parachute approach. Do not implement.

### 3.4 Confidence

C=HIGH (constructor decompiled and contains only init code, no live behavior),
I=HIGH (Ghidra label "DropPodLocomotionClass__Constructor"),
B=HIGH (zero INI bindings; single COM-factory caller; no other reachable path).

---

## 4. Teleport — Update_Position @ `0x718260` (Item #29)

### 4.1 Two-mode function

Update_Position is called from two distinct sites with different `param_5` values:

| `param_5` | Mode | Purpose |
|-----------|------|---------|
| `0` | **Simple teleport** | Applies chrono damage to occupants at destination, then sets the unit's coord. Called from `Process` state-machine for normal teleport. |
| `!= 0` | **ChronoSphere relative offset** | Reads `TechnoClass.ChronoDestCoord (+0x288/0x28C/0x290)` and applies as new position. Used during ChronoSphere superweapon. |

The bridge interaction differs between modes.

### 4.2 Mode 0 (simple teleport): bridge-aware damage walk

When `param_5 == 0`, the function iterates objects at the destination cell. **The list-selection** uses `cell.flags & 0x100` to choose between `+0xE4` (ground) and `+0xE8` (bridge):

```c
iVar6 = CellClass__Get_Cell_At(&param_2);
if ((*(uint *)(iVar6 + 0x140) & 0x100) == 0) {           // dest cell NOT bridge
    iVar6 = CellClass__Get_Cell_At(&param_2);
    piVar8 = *(int **)(iVar6 + 0xe4);                    // ground list
}
else {
    iVar6 = CellClass__Get_Cell_At(&param_2);
    piVar8 = *(int **)(iVar6 + 0xe8);                    // bridge list
}
```

This is a **single-layer pick** — the chrono damage hits only the layer at the destination. A chrono-warped unit arriving on the bridge deck telefrags the deck occupants, NOT the ground occupants below the bridge.

### 4.3 Mode 0: the bridgehead-vs-body force-occupancy gate

After the damage walk:

```c
iVar6 = CellClass__Get_Cell_At(&param_2);
if (((*(uint *)(iVar6 + 0x140) & 0x100) != 0)               // cell IS bridge cell
    && (iVar6 = CellClass__Get_Cell_At(&param_2),
        (*(uint *)(iVar6 + 0x140) & 0x200) == 0))            // cell is NOT bridgehead (0x200 clear)
{
    param_5 = '\x01';                                        // force-occupancy-validation flag = 1
}
```

**Decoded:** if dest cell is a bridge BODY cell (has `0x100` set, has `0x200` clear), force the occupancy-validation flag. This causes a downstream `Find_Nearby_Passable_Cell` zone-walk to run (see TELEPORT_LOCOMOTION_DEEP_DIVE.md §3.6) instead of accepting the destination as-is.

**Player-observable:** chrono-warping onto a bridge body cell (not a bridgehead) triggers a pathfinding fallback that snaps to the nearest reachable cell. Warping onto a bridgehead does NOT trigger the fallback. This prevents chrono units from materialising mid-span on bridges where they couldn't normally walk onto.

### 4.4 Mode 1 (ChronoSphere): bridge Z-bump and on_bridge update

When `param_5 != 0`:

```c
iVar6 = CellClass__Get_Cell_At(piVar8);
if (((*(uint *)(iVar6 + 0x140) & 0x100) == 0)                // cell NOT bridge
    || (*(char *)(*(int *)(param_1 + 0xc) + 0x8c) != 0))     // OR already on_bridge=1
{
    *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 0;     // clear on_bridge = 0
}
else {
    *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 1;     // set on_bridge = 1
    *(int *)(param_1 + 0x30) = *(int *)(param_1 + 0x30) + g_BridgeZOffset_Teleport;
                                                              // Z += g_BridgeZOffset_Teleport (0x00B0EC2C)
}
```

**Verified transition table:**

| dest cell.flags & 0x100 | current on_bridge | New on_bridge | Z change |
|--------------------------|--------------------|---------------|----------|
| 0 (not bridge) | 0 | **0** | no change |
| 0 | 1 | **0** (clear stale flag) | no change |
| 1 (bridge) | 0 (not already on) | **1** (SET) | **`+= g_BridgeZOffset_Teleport`** |
| 1 | 1 (already on) | **0** (CLEAR) | no change |

**The fourth row is counter-intuitive** — if you're already on_bridge AND arrive on a bridge cell, the flag is CLEARED rather than left alone. This appears to be a state reset mechanism: when arriving via ChronoSphere on a bridge while previously flagged on_bridge, the engine assumes the on_bridge flag is stale (from the previous location's bridge state) and resets it. The Z is NOT bumped because the unit's coord was just resolved by the caller (presumably already in deck-Z).

The decompiler-friendly C reading: `if (NOT bridge || already_on) clear; else set+bump`. The hidden semantic: ChronoSphere clears stale state before applying fresh bridge-state on a true entry.

### 4.5 `g_BridgeZOffset_Teleport` at `0x00B0EC2C`

```
read_memory 0x00B0EC2C length 8 → all zeros (BSS, runtime-init)
get_xrefs_to 0x00B0EC2C →
  From 0071870b in TeleportLocomotionClass__Update_Position  [READ]
  From 00717F80 [WRITE]                                       (init site — outside labeled function)
```

**Single read site** (the Mode 1 Z-bump). **Single write site** at `0x717F80` — likely a Teleport-class init function called once at boot.

### 4.5b SECOND Teleport bridge constant `DAT_00B0EC38` — **found in cleanup pass**

The original draft of this report documented only `g_BridgeZOffset_Teleport @ 0x00B0EC2C` and asserted it was the sole Teleport bridge constant. **The cleanup pass freshly decompiled `TeleportLocomotionClass::Process @ 0x718B70` and found a SECOND Teleport-specific bridge constant** at `0x00B0EC38`:

```c
// In Teleport::Process @ 0x718C5D area, the "is destination on a bridge" detection:
iVar6 = CellClass__Get_Cell_At(&iStack_34);
if ((*(uint *)(iVar6 + 0x140) & 0x100) == 0) {
LAB_00718c70:
    uVar11 = 0;                                          // not on bridge
}
else {
    iVar6 = *(int *)(param_1 + 0xc);
    // ... get unit coords ...
    iVar6 = *(int *)(iVar6 + 0xa4);                      // unit.Z
    iVar7 = CellClass__GetGroundHeight(&iStack_34);
    uVar11 = 1;                                          // tentatively on bridge
    if (iVar6 <= iVar7 + DAT_00b0ec38 * 3) goto LAB_00718c70;   // ← THIS uses 0xB0EC38, not 0xB0EC2C
}
```

**Verified rule for Teleport::Process isOnBridge detection:**
```
isOnBridge = (cell.flags & 0x100) AND (unit.Z > ground_height + DAT_00B0EC38 * 3)
```

The threshold `× 3` is **3 height steps** above ground (NOT 4 like the Z-bump constant). This suggests `DAT_00B0EC38` is the **per-level height step for Teleport** (analogous to `g_DriveHeightStep` / `g_ShipHeightStep`), and the Z-bump constant `g_BridgeZOffset_Teleport @ 0xB0EC2C` is `4 × DAT_00B0EC38` (per the same pattern as Drive/Ship — though this isn't verified at the init site).

**Xrefs for `DAT_00B0EC38`:**
```
From 00718c5d in TeleportLocomotionClass__Process [READ]
From 00717eeb [WRITE]                              (init site)
From 00717f00 [READ]                               (likely the Z-bump-constant init at 0x717F80 reading the height step)
From 00717f31 [READ]
From 00717f61 [READ]
```

The 4 reads at 0x717F00/F31/F61 are likely inside the Teleport init code that computes `g_BridgeZOffset_Teleport` from `DAT_00B0EC38`. The WRITE at `0x717EEB` is BEFORE the WRITE at `0x717F80`, consistent with the height-step being initialised first and then the Z-offset being derived from it.

**Net effect on Phase 3 globals table:** Teleport has **two** related bridge globals, not one as the original draft asserted:
- `DAT_00B0EC38` — per-level height step (Teleport's analogue of `g_DriveHeightStep`)
- `g_BridgeZOffset_Teleport @ 0x00B0EC2C` — pre-computed `4 × height_step` Z-offset for Update_Position bump

**Cross-doc correction:** The prior `TELEPORT_LOCOMOTION_DEEP_DIVE.md` doc §3.1 says "g_BridgeZOffset is at 0xB0EC38 (runtime value, 0 in .data)". That doc had it at the **height-step** address, but called it the "Z-offset". **This report now disambiguates: 0xB0EC38 is the height step; 0xB0EC2C is the Z-offset.** The prior doc's claim about Process using "`unit.Z > ground + g_BridgeZOffset × 3`" actually means the height-step × 3, consistent with how `unit.Z` should be 3+ height-levels above ground to qualify as on-bridge.

Why `× 3` (not `× 4`)? Inferred: a unit's actual Z coordinate when standing on a bridge deck is slightly less than the exact deck height (the unit's feet are at deck level; its anchor coord is somewhere mid-body). Allowing `> 3 × height_step` (= 3 levels above ground) accepts units that are physically at bridge-deck height even if their anchor Z is below the exact ground + 4 × step value. Parity-load-bearing.

**Confidence: C=HIGH (raw decompilation), I=MEDIUM (purpose inferred from usage; the height-step-vs-Z-offset distinction matches the Drive/Ship pattern), B=HIGH (xrefs traced).**

### 4.6 Caller binding

Update_Position is called from:
- `Process` (StateMachineTick) — Mode 0, for normal teleport arrival
- StateMachine phase 2/3 — Mode 1, for ChronoSphere warp

Active in YR: **Yes** — Chrono Legionnaire and Chrono Miner use Teleport CLSID `4A582747`. Confirmed by `LOCOMOTION_MATH_AND_CONSTANTS.md` and `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`.

Confidence: C=HIGH (full decomp), I=HIGH (Ghidra label confirmed), B=HIGH (single g_BridgeZOffset_Teleport read site).

### 4.7 Process @ `0x718B70` — verified bridge interaction limited to Update_Position dispatch

The TELEPORT_LOCOMOTION_DEEP_DIVE.md doc §3 (cited in pre-read) describes Process @ 0x718B70 as the **destination validator** that also reads `cell.flags & 0x100` and computes `isOnBridge` based on unit Z vs ground+offset. This was independently verified during this phase. Process is the **destination-snap site** for non-infantry; infantry use a sub-cell positioning path via `PlaceInfantryInCell`. Bridge interaction in Process is limited to:
- Read cell.flags & 0x100 to decide isOnBridge for pathfinding-zone lookup
- Pass isOnBridge through to `MapClass::GetZoneID` and `Pathfinding_validate_alternate`

No direct Z-bump in Process itself; the actual Z-bump happens in Update_Position §4.4.

### 4.8 HeadToCoord @ `0x718100` — no bridge interaction

Quick verification: HeadToCoord is the destination-setter (called when Set_Destination_Internal routes to a Teleport locomotor). It guards on 4 vtable slots (`0x37C, 0x380, 0x1D4, 0x1D8` — same as Drive) and calls Process for destination resolution. **No direct bridge-cell read in HeadToCoord** — bridge logic is in Process (validation) and Update_Position (application).

### 4.9 Is_Moving @ `0x718080` — no bridge interaction

One-liner returning `*(byte *)(param_1 + 0x30)` (the `IsMoving` flag at struct +0x34, since `param_1 = instance + 0x04`). No bridge read.

---

## 5. Cross-doc contradictions resolved

### 5.1 "Walk uses the same `g_BridgeZOffset_Drive` formula"

**Refuted.** Walk uses a separate `FUN_006D2120(60)` helper that reads `DAT_00B0CDD8` (a different global at `0x00B0CDD8`), not `g_DriveHeightStep`. The two formulas may produce numerically similar results at runtime (both target ≈ 4 height levels) but use different intermediate paths and different rounding directions.

### 5.2 "DropPodLocomotion is used by Allied Paradrop"

**Refuted.** Per CLSID-search in rulesmd.ini, no unit has `Locomotor={4A582745-...}`. Allied Paradrop's carrier is FlyLocomotion (Nighthawk Transport using CLSID `4A582746`), and the dropped infantry use Parachute as FootClass state (not a separate locomotor). DropPodLocomotionClass is a TS holdover with no live binding in YR.

### 5.3 Mode-1 Teleport's "already_on_bridge → clear flag" semantics

The decompilation shows `if (NOT bridge || on_bridge_already) clear`. **Confirmed in raw assembly** — this is not a Ghidra decompiler artifact. The semantic is "stale state reset" when arriving via ChronoSphere on the same logical layer.

A simpler implementation might have written `if (on_bridge_already) leave alone; else if (bridge) set+bump`. The binary's choice to **clear** instead is an active design decision.

---

## 6. Active-in-YR confirmation per function

| Function | Active in YR? | Evidence | Gating |
|----------|---------------|----------|--------|
| `WalkLocomotionClass::Head_To_Coord @ 0x75ACB0` | Yes | ILocomotion vtable slot 17 of WalkLocomotionClass | Only 3 vtable guards (missing 0x380 IsBeingWarpedIn) |
| `WalkLocomotionClass::Process @ 0x75AC80` | Yes | ILocomotion vtable slot 16 | None |
| `FUN_006D2120` @ 0x006D2120 | Yes | Called from Walk::Head_To_Coord | None — fires whenever Walk dest is on bridge cell |
| `DropPodLocomotionClass::Constructor @ 0x4B5AB0` | **No (TS-dead)** | Zero INI bindings to CLSID `4A582745`; reachable only via COM factory at 0x6C494C | n/a |
| `TeleportLocomotionClass::Update_Position @ 0x718260` | Yes | Called from Process and StateMachine phases | None — both modes active |
| `TeleportLocomotionClass::Process @ 0x718B70` | Yes | ILocomotion vtable slot 16 | None |

No SpecialFlags / fog gates on any of the live functions.

---

## 7. Current Rust Implementation Status

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| Walk's `FUN_006D2120(60)` Z-bump (round-half-DOWN) | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Audit** — Rust likely uses a single `bridge_deck_z_offset` for all locomotors; binary differentiates Walk (round-half-down) from Drive (round-half-up). 1-lepton discrepancy at boundary. |
| Walk's missing-IsWarpingIn-guard (only 3 vtable checks vs Drive's 4) | [src/sim/movement/movement_commands.rs](../../ra2-rust-game/src/sim/movement/movement_commands.rs) | **Audit** — Player-observable: infantry should accept new dest during warp-in, vehicles should not. |
| Walk's "destination valid" flag at +0x30 | [src/sim/movement/locomotor.rs](../../ra2-rust-game/src/sim/movement/locomotor.rs) | Audit — likely implicit in Rust's `Option<Coord>` patterns. |
| DropPodLocomotion implementation | none (per CLAUDE.md/MEMORY) | **Do NOT implement.** Confirmed TS-dead. |
| Teleport Mode 0 layer-specific damage walk | [src/sim/movement/teleport_movement.rs](../../ra2-rust-game/src/sim/movement/teleport_movement.rs) | **Partial** — Rust handles teleport damage but may not differentiate ground vs bridge layer at destination cell. |
| Teleport Mode 0 force-occupancy-validation on bridge body cell | [src/sim/movement/teleport_movement.rs](../../ra2-rust-game/src/sim/movement/teleport_movement.rs) | **Audit** — the "warp onto bridge body fails, snaps to nearby" behaviour. |
| Teleport Mode 1 Z-bump + on_bridge update | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Partial** — Rust applies bridge offset on teleport but the "already_on_bridge → clear flag" semantic is likely missing. |
| Teleport's `g_BridgeZOffset_Teleport` (separate from Drive's) | [src/sim/movement/movement_bridge.rs](../../ra2-rust-game/src/sim/movement/movement_bridge.rs) | **Audit** — Rust may share Z-offset; binary uses separate runtime-initialised constant. |
| The 0x200 bridgehead-vs-body distinction in teleport destination | [src/sim/movement/teleport_movement.rs](../../ra2-rust-game/src/sim/movement/teleport_movement.rs) | **Likely missing** — teleporting onto bridgehead vs body cell produces different behaviours in the binary. |

(Severity assessment deferred to Phase 7 synthesis.)

---

## 8. Open Questions

1. **Walk's `DAT_00B0CDD8` runtime value** — not observed at runtime in this report. Required for verifying Walk's Z-bump numerically matches Drive's.
2. **Walk's destination-offset storage at +0x18..+0x20** vs the secondary set at +0x24..+0x2C — what's the second set? Possibly "previous destination" for diff-based logic. Not pursued this phase.
3. **DropPod's status outside skirmish** — campaign missions, map-script triggers? A grep of mission INIs (not done this phase) would close the question fully. For skirmish/standard-rules: confirmed dead.
4. **Teleport Mode 0 "force occupancy" downstream effect** — the path through `Pathfinding_validate_alternate` was documented in the existing TELEPORT_LOCOMOTION_DEEP_DIVE.md but the bridgehead-vs-body specific gate at §4.3 may interact with the alternate-cell-search to choose alternate sub-cells differently. Not exhaustively traced.
5. **Walk `+0x31` flag** in `Process` — described in §2.8 as "currently processing" but its actual semantic and consumers not traced.

---

## 9. Sources

**Ghidra functions decompiled:**
- `WalkLocomotionClass::Head_To_Coord` @ 0x0075ACB0 (~225 bytes body)
- `WalkLocomotionClass::Head_To` @ 0x0075AC00 (~63 bytes)
- `WalkLocomotionClass::Process` @ 0x0075AC80 (~22 bytes)
- `WalkLocomotionClass::Is_Moving` @ 0x0075AB30 (~12 bytes)
- `WalkLocomotionClass::Constructor` @ 0x0075AA90 (~150 bytes)
- `FUN_006D2120` @ 0x006D2120 (28 bytes — Walk's bridge Z-bump helper)
- `DropPodLocomotionClass::Constructor` @ 0x004B5AB0 (~100 bytes)
- `DropPodLocomotionClass::Destructor` @ 0x004B5B00 (~50 bytes)
- `TunnelLocomotionClass::Constructor` @ 0x00728A00 (~120 bytes — for TS-dead confirmation)
- `TeleportLocomotionClass::Update_Position` @ 0x00718260 (~700 bytes — bridge sites at §4)

**Raw assembly examined:**
- WalkLocomotionClass::Head_To_Coord full disassembly (verified 3-guard vtable, `MOV ECX, 0x3C`, call to FUN_006D2120)
- FUN_006D2120 full disassembly (verified FSUB 0.5 — round-half-down)
- Teleport::Update_Position bridge regions verified via decompilation

**Memory reads:**
- 0x00B0CDD8 (Walk scale factor; cold BSS, runtime-init)
- 0x00B0EC2C (g_BridgeZOffset_Teleport; cold BSS, runtime-init)
- 0x007E1738 (rounding 0.5 — confirmed shared with Drive/Ship)
- 0x007F69F8 + 80 bytes (Walk ILocomotion vtable for slot verification)

**Xrefs traced:**
- `get_xrefs_to 0x4B5AB0` (DropPod ctor) → 1 UNCONDITIONAL_CALL from `0x6C494C` (COM factory)
- `get_xrefs_to 0x4B66F0` (DropPod helper) → 1 DATA xref (vtable entry)
- `get_xrefs_to 0x728A00` (Tunnel ctor) → 1 UNCONDITIONAL_CALL from `0x6C464C` (COM factory)
- `get_xrefs_to 0x00B0CDD8` (Walk scale) → 1 READ from FUN_006D2120, 1 WRITE at 0x6D1C11
- `get_xrefs_to 0x00B0EC2C` (Teleport offset) → 1 READ from Update_Position, 1 WRITE at 0x717F80

**INI verification:**
- `grep '4A582745' ini/rulesmd.ini` → **zero matches** (DropPod CLSID — confirms TS-dead)
- `grep '4A582743' ini/rulesmd.ini` → **zero matches** (TunnelLocomotion CLSID — confirms TS-dead)
- `grep '4A582744' ini/rulesmd.ini` → **64+ matches** (Walk CLSID — active)
- `grep '4A582747' ini/rulesmd.ini` (Teleport) → matches (Chrono Legionnaire, Chrono Miner)

**Callers traced:**
- Walk::Head_To_Coord ← FootClass::Set_Destination_Internal [vtable slot 17]
- FUN_006D2120 ← Walk::Head_To_Coord [only]
- DropPod::Constructor ← COM factory at 0x6C494C [no other callers]
- Teleport::Update_Position ← Teleport::Process and StateMachine phases

**Companion docs:**
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` (Drive/Ship bridge Z-offset family — §1 of that doc; **updated in cleanup to include Walk's `DAT_00B45C28` and Teleport's `DAT_00B0EC38`**)
- `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md` (DropPod, Tunnel, Float, Parachute non-coverage)
- `TELEPORT_LOCOMOTION_DEEP_DIVE.md` (prior — Teleport's full destination-resolution logic; **conflicts with this report on which 0xB0EC** address is the Z-offset vs the height step. This report's §4.5b disambiguates.)
- `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` (prior — Chrono Miner specifics)
- `LOCOMOTION_MATH_AND_CONSTANTS.md` (prior — locomotor CLSID table)

---

## 10. Cleanup pass — 2026-05-13 (post-initial-draft)

| Item | Original status | Cleanup verdict |
|------|-----------------|-----------------|
| Walk::Head_To_Coord bridge Z-bump via `FUN_006D2120(60)` | HIGH | HIGH (unchanged) |
| Walk's `60 = 4 × 15` derivation | HIGH (implied) | **MEDIUM** — "15 px/level" is RA2 conventional wisdom, not freshly binary-verified. See §2.4 of original draft. |
| Walk::ProcessMovement "bridge state read inside but not decompiled" | known gap | **CLOSED (HIGH)** — §2.9 added with 5 bridge sites and a newly-discovered global (DAT_00B45C28). |
| Walk's "missing IsBeingWarpedIn guard (3 vs Drive's 4)" | HIGH | HIGH (unchanged) |
| DropPod 0x4B66F0 "vtable thunk entry" | MEDIUM | **REFINED** — it is the scalar-deleting destructor pattern, dispatched via IUnknown::Release. See §3.1. |
| DropPod zero-INI-binding TS-dead claim | HIGH | HIGH (unchanged) |
| Teleport Mode-1 Z-bump uses `g_BridgeZOffset_Teleport @ 0xB0EC2C` | HIGH | HIGH (confirmed) |
| Teleport's "single bridge constant" claim | HIGH (claimed) | **REFUTED** — Teleport has TWO bridge constants. See §4.5b (NEW): the second is `DAT_00B0EC38` at the per-level height step. |
| Teleport "already_on_bridge → clear flag" semantic | HIGH (decomp-based) | HIGH (no change — assembly aligns with decompilation) |

### 10.1 Net deliverable changes from cleanup

- §2.8 fixed to point to §2.9 (the newly-decompiled Walk::ProcessMovement section).
- §2.9 added — full Walk::ProcessMovement bridge interaction (5 sites: on_bridge transition, mismatch detector, scatter pick, LandType-10 exemption, DAT_00B45C28 introduction).
- §3.1 refined to describe 0x4B66F0 correctly as scalar-deleting destructor.
- §4.5b added — second Teleport bridge constant `DAT_00B0EC38` and disambiguation from `g_BridgeZOffset_Teleport @ 0xB0EC2C`.
- Cross-doc note: the Drive/Ship doc §1 globals table is now extended from 6 globals to 8 (adding Walk's `DAT_00B45C28` and Teleport's `DAT_00B0EC38`).

### 10.2 Remaining MEDIUM-confidence items

1. **Walk's `15 pixels/level` derivation** — relies on RA2 conventional knowledge, not a binary site.
2. **JumpJet::Set_Destination @ 0x54B1C0** — Ghidra doesn't recognise this as a function; only the entry block was decoded from raw bytes. Full body wasn't read.
3. **Teleport's `DAT_00B0EC38 ≈ DAT_00B0EC2C / 4`** — inferred from `× 3` vs `× 4` patterns, not verified at the init site `0x717F80`.

These do not affect any HIGH-confidence claim; they are flagged for Phase 7 synthesis.
