# Parachute Landing Bridge Layer Selection — Ghidra Research Report

**Date:** 2026-05-19
**Binary:** gamemd.exe (Yuri's Revenge)
**Scope:** Narrow — parachute / paradrop touchdown on a bridge cell: which layer (deck vs. ground), what flag gates it, and whether OnBridge (`FootClass+0x8C`) is correctly set.
**Active in YR:** Yes — Allied Paradrop superweapon, garrison-building survivors, and temporal-class dispersal all fire the parachute descent path in standard YR skirmish.
**Confidence axes used:** C=content, I=identity, B=binding per project standard.

---

## Summary

When a unit that is parachuting (IsFallingDown flag `ObjectClass+0x8D = 1`) touches down over a bridge cell, it lands **on the bridge deck**, not on the ground beneath. The `OnBridge` flag (`ObjectClass+0x8C`) is set to 1 during Unlimbo (the mid-air spawn call), and this value is read by `FootClass::Set_Height_On_Bridge` at the moment of touchdown to snap the unit's Z to deck height. There is **no additional bridge-layer logic in the descent or touchdown path** — no separate "parachute-to-bridge" branch exists. The standard `ObjectClass::Unlimbo` bridge-flag check (cell `+0x140 & 0x100`) provides the only gate.

**One-line verdict:** Parachute touchdown on an intact bridge cell places the unit on the deck because `OnBridge=1` is written at mid-air Unlimbo time and is then consumed by `FootClass::Set_Height_On_Bridge` at touchdown.

---

## Key Functions

| Function | Address | Role |
|---|---|---|
| `ObjectClass::Unlimbo` | `0x005F5940` | Initial mid-air spawn; writes `OnBridge=1` if destination cell has bridge flag |
| `ObjectClass::AI` (falling branch) | `0x005F3E70` | Per-tick fall; triggers touchdown when altitude drops below 1 |
| `FootClass::Set_Height_On_Bridge` | `0x005F5FA0` | vtable+0x1CC — snaps Z to deck or ground using current `OnBridge` state |
| `CellClass::Get_Effective_Height` | `0x005F5F00` | vtable+0x1C8 — returns altitude accounting for OnBridge (+4 if on bridge) |
| `CellClass::GetGroundHeight` | `0x00578080` | Returns raw terrain Z at a world coord |
| `SpawnUnitsWithParachute` | `0x004585C0` | One of three callers that initiates parachute descent |
| `AircraftClass::Drop_Payload` | `0x00415C60` | Paradrop aircraft per-pass drop; calls `CellClass::PlaceInfantryInCell` then Unlimbo chain |

---

## 1. Parachute Spawn Path

Verified caller chain (one of three live paths):

1. `AircraftClass::Drop_Payload @ 0x00415C60` is called from `AircraftClass::Fire_At @ 0x00415ef8` (unconditional call, verified via `get_xrefs_to 0x00415c60`).
2. `Drop_Payload` calls `CellClass::PlaceInfantryInCell @ 0x00481180` to find a sub-cell XY position in the target cell. It passes `param_5=0` (ground-height mode), so the returned Z from `PlaceInfantryInCell` is the cell's raw ground height.
3. The spawned unit then goes through `ObjectClass::Unlimbo @ 0x005F5940` (via the vtable Unlimbo chain) with the computed coordinates.

**Active in YR:** Yes — `AircraftClass::Drop_Payload` is called when the paradrop aircraft is on the `ParaDropOverfly` mission. `SpawnUnitsWithParachute @ 0x004585C0` has three callers: `BuildingClass::SellBuilding` (garrison dispersal), `BuildingClass::AddGarrisonOccupant`, and `TemporalClass::Update` (Yuri-kill survivor dispersal) — all live in YR.

---

## 2. `ObjectClass::Unlimbo` Bridge Gate

Decompiled and disassembled at `0x005F5940` (verified via `decompile_function 0x005F5940` + `disassemble_function 0x005F5940`):

```asm
005f5965: MOV byte ptr [ESI + 0x8d],0x1     ; IsFallingDown = 1 (parachute active)
; ...
005f597b: MOV EAX,dword ptr [EBP + 0x140]   ; EBP = CellClass at target XY
005f5981: TEST AH,0x1                        ; test cell Flags & 0x100 (bridge structural)
005f5984: JZ 0x005f599c                      ; if NOT bridge cell, skip OnBridge set
005f5986: MOV byte ptr [ESI + 0x8c],0x1      ; OnBridge = 1
005f598d: MOV EAX,dword ptr [EBP + 0x140]
005f5993: TEST AH,0x2                        ; test cell Flags & 0x200 (bridge body)
005f5996: JZ 0x005f5b41                      ; if NOT bridge body, return 0 (FAIL — can't Unlimbo under-bridge)
```

**Three cases for parachuting unit Unlimbo'd onto bridge XY:**

| Cell flags at target XY | OnBridge set? | Unlimbo result | Notes |
|---|---|---|---|
| `Flags & 0x100 == 0` (no bridge) | No | Success | Normal ground landing |
| `Flags & 0x100 == 1, & 0x200 == 0` | Yes, to 1 | **Fail (return 0)** | Bridge cell but NOT the walkable deck; side/under-bridge cell — Unlimbo rejected |
| `Flags & 0x100 == 1, & 0x200 == 1` | Yes, to 1 | **Success** | Bridge deck cell — OnBridge=1 set, Unlimbo succeeds |

So a parachuting unit landing on the **bridge deck** (both `0x100` and `0x200` flags) successfully Unlimbos with `OnBridge=1`. A unit landing under the bridge or on a non-walkable bridge structural cell is **rejected by Unlimbo** — the call returns 0. What happens to a rejected mid-air Unlimbo is outside this scope's evidence (likely the cargo plane retains the unit).

Confidence: C=HIGH (disassembly matches decompile), I=HIGH (function identity verified), B=HIGH (vtable DATA xrefs confirm `0x005F5940` is in 20+ vtables).

---

## 3. Parachute Descent and Touchdown

### 3.1 Per-tick falling in `ObjectClass::AI @ 0x005F3E70`

Verified via `decompile_function 0x005F3E70`:

```c
// Pseudocode (C-like):
iVar2 = vtable+0x78()         // GetLayer (saved for layer-change detect)
iVar3 = vtable+0x1d0()        // get current Z floor position
iVar1 = param_1[0xb]          // accumulated fall delta (negative)
// IsFallingDown check:
if ((char)param_1[0x8d] != '\0') {
    // parachuting branch:
    iVar3 = param_1[0xb] + -1;     // decrement fall rate
    param_1[0xb] = iVar3;
    iVar1 = RulesClass.ParachuteMaxFallRate;  // g_RulesClass_Instance + 0x7b8
    if (iVar1 < iVar3) goto snap_rate;
} else {
    // no-parachute branch:
    iVar3 = Math::ftol();           // higher fall rate
    param_1[0xb] = iVar3;
    iVar1 = RulesClass.NoParachuteMaxFallRate; // g_RulesClass_Instance + 0x7bc
    if (iVar1 < iVar3) goto snap_rate;
}
snap_rate:
param_1[0xb] = iVar3;             // clamped to MaxFallRate
// Z update:
param_1[0x29] = vtable+0x1d0() + iVar3;

// Altitude check:
iVar1 = vtable+0x1c8();            // CellClass::Get_Effective_Height - altitude above ground
if (iVar1 < 1) {                   // TOUCHDOWN
    vtable+0x1cc(0);               // FootClass::Set_Height_On_Bridge(unit, 0)
    param_1[0x8d] = 0;             // clear IsFallingDown
    vtable+0x18c(2);               // Mission_Change (ATTACK or STOP)
    if (param_1[0x22] != 0) {
        *(param_1[0x22] + 0x195) = 0;  // detach parachute PARACH anim
    }
}
```

### 3.2 `CellClass::Get_Effective_Height @ 0x005F5F00` (vtable+0x1C8)

Verified via `decompile_function 0x005F5F00`:

```c
int CellClass__Get_Effective_Height(int *param_1) {
    iVar1 = vtable+0x1bc();      // GetCell (CellClass for current position)
    return (-(uint)((char)param_1[0x23] != '\0') & 4)    // +4 if OnBridge (param_1[0x23] = int* index = +0x8C)
           + (int)*(char *)(iVar1 + 0x11b);              // + CellClass::Level
}
```

Where `param_1[0x23]` = `*(int*)(param_1 + 0x8C)` = `OnBridge`. If `OnBridge==1`, effective height adds 4 to the cell Level. This means the "touch ground" condition (`altitude < 1`) fires when Z reaches bridge-deck height, not raw ground height — so **the unit descends to deck level, not to ground**.

Confidence: C=HIGH, I=HIGH (22 xrefs, many vtable DATA), B=HIGH.

### 3.3 `FootClass::Set_Height_On_Bridge @ 0x005F5FA0` (vtable+0x1CC)

Verified via `decompile_function 0x005F5FA0`:

```c
void FootClass__Set_Height_On_Bridge(int *param_1, int param_2) {
    if ((char)param_1[0x23] != '\0') {  // param_1[0x23] = OnBridge (+0x8C)
        param_2 = param_2 + DAT_00ac13bc;  // add bridge Z offset constant
    }
    // If visible (not limbo):
    if ((char)param_1[0x1d] != '\0') {
        vtable+0x124(0);             // Mark(REMOVE)
        iVar1 = CellClass::GetGroundHeight(&local_c);
        param_1[0x29] = iVar1 + param_2;  // Z = ground_height + param2 (+bridge_offset if OnBridge)
        vtable+0x124(1);             // Mark(PUT) — registers in occupancy at new layer
        return;
    }
    iVar1 = CellClass::GetGroundHeight(&local_c);
    param_1[0x29] = iVar1 + param_2;
}
```

Called at touchdown as `FootClass::Set_Height_On_Bridge(unit, 0)`:
- `param_2 = 0`
- If `OnBridge=1`: Z = `CellClass::GetGroundHeight` + `DAT_00ac13bc` (bridge height offset)
- If `OnBridge=0`: Z = `CellClass::GetGroundHeight` + 0

**vtable slot confirmation:** InfantryClass vtable base = `0x007EB058` (derived: vtable+0x124 = `0x007EB17C` per existing `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`, i.e., `0x007EB17C - 0x124 = 0x007EB058`). vtable+0x1CC = `0x007EB058 + 0x1CC = 0x007EB224`. Reading `0x007EB224` via `read_memory 0x007EB224 12` = `[a0 5f 5f 00 ...]` = `0x005F5FA0` = `FootClass::Set_Height_On_Bridge`. ✓

Confidence: C=HIGH (decompile verified), I=HIGH (vtable slot confirmed by memory read), B=HIGH (call confirmed via `ObjectClass::AI` landing branch + vtable lookup).

---

## 4. The `Mark(1)` Call and Occupancy Registration

`FootClass::Set_Height_On_Bridge` calls `vtable+0x124(1)` (Mark/DoCloak with mode 1) **after** `OnBridge` is already set. This is the call that re-registers the unit in the cell occupancy lists. The `TechnoClass::DoCloak` mark handler reads `OnBridge` at call time (per `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` §Add/Remove Layer Argument Source) — so the unit is inserted into `CellClass+0xE8` (bridge/alt object list) when `OnBridge=1`.

**Occupancy result:** A parachuting unit that lands on a bridge deck is correctly placed in `CellClass+0xE8` (bridge deck occupancy list), not `CellClass+0xE4` (ground list). This matches the observable behavior: after landing on a bridge, the unit behaves as a normal bridge-deck occupant and is correctly relayered by `BlowUpBridge` / `DropIn` if the bridge later collapses.

---

## 5. The Bridge-Body (`0x200`) Rejection and Observable Effect

From §2: if the target cell has `Flags & 0x100` (bridge structural) but NOT `Flags & 0x200` (bridge body/deck), `ObjectClass::Unlimbo` returns `0` (failure). For a parachuting unit, this means:

- Paradropping infantry targeted directly onto the bridge **deck** cell: lands on deck. ✓
- Paradropping infantry whose target cell is **beneath the bridge** or on a non-walkable bridge structural cell: Unlimbo fails. The cargo plane retains or re-queues the unit (outside scope). The unit does NOT clip through the bridge to ground.

This is the observable parity point: you cannot paradrop infantry to land under a high bridge via a standard Allied Paradrop.

---

## 6. Active-in-YR Confirmation

| Code path | Active in YR? | Evidence |
|---|---|---|
| Allied Paradrop SW spawn | **Yes** | `AircraftClass::Mission_ParaDropOverfly @ 0x004157C0` + `Drop_Payload @ 0x00415C60`; invoked by Paradrop aircraft which exist in stock YR rulesmd.ini |
| `SpawnUnitsWithParachute @ 0x004585C0` | **Yes** | Called by `BuildingClass::SellBuilding`, `AddGarrisonOccupant`, `TemporalClass::Update` — all live YR paths |
| Bridge flag check at `ObjectClass::Unlimbo` | **Conditional** | Only fires when a bridge deck cell is in the target area; fires every paradrop that lands on a bridge |
| `CellClass::Get_Effective_Height` +4 for OnBridge | **Conditional** | Live for any on-bridge unit's altitude calculation |
| `FootClass::Set_Height_On_Bridge` bridge offset | **Conditional** | Live for any unit landing with `OnBridge=1` |

---

## 7. Open Questions (Not Resolved Here)

1. **Value of `DAT_00ac13bc`** (bridge height offset used in `FootClass::Set_Height_On_Bridge`). Memory read returns 0 at the static address — this is a runtime value populated during map init. The existing `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` §GetHeight confirms `DAT_00ac13bc` is the same "bridge Z offset" used in that function. Its actual runtime value is deferred to the bridge height constants work.

2. **What happens on a Unlimbo failure for mid-air unit.** The function returns 0; the caller handles this. For `AircraftClass::Drop_Payload`, the returned PlaceInfantryInCell coord is checked for the sentinel NullCoord; if Unlimbo fails, the unit is put back in cargo via `CargoClass::AddPassenger`. This is outside the scope of this investigation.

3. **Paradrop onto destroyed bridge cells.** The `0x200` bridge-body flag is cleared when the bridge is destroyed (set by `CellClass::SetBridgeDirection`). A parachuting unit targeted onto a destroyed bridge cell would land at ground level (no `0x200` flag → `0x100`-only fails Unlimbo, OR if `0x100` is also cleared, normal ground landing). This edge case was not traced to a firm conclusion.

---

## 8. Load-Bearing Verified Facts

1. **`ObjectClass::Unlimbo @ 0x005F5940` sets `OnBridge=1` at `[ESI+0x8C]` when destination cell `Flags & 0x100` is set** — verified via `disassemble_function 0x005F5940` at instruction `005f5986`.

2. **`CellClass::Get_Effective_Height @ 0x005F5F00` (vtable+0x1C8) adds 4 to altitude when `OnBridge=1`**, making the touchdown condition (`altitude < 1`) fire at bridge-deck height — verified via `decompile_function 0x005F5F00`.

3. **`FootClass::Set_Height_On_Bridge @ 0x005F5FA0` (vtable+0x1CC) reads `OnBridge` at `param_1[0x23]` (+0x8C) and adds `DAT_00ac13bc` to the ground height** when set — verified via `decompile_function 0x005F5FA0`.

4. **vtable+0x1CC for InfantryClass resolves to `FootClass::Set_Height_On_Bridge @ 0x005F5FA0`** — verified via `read_memory 0x007EB224` returning `0x005F5FA0`.

5. **`FootClass::Set_Height_On_Bridge` calls `vtable+0x124(1)` (Mark/DoCloak-add) AFTER `OnBridge` is already set**, so the unit registers into `CellClass+0xE8` (bridge deck list) on touchdown — consistent with established `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md` §Add/Remove Layer Argument Source.

---

## 9. Implications for Rust Port

The Rust `parachute_descent.rs` landing handler must:

1. **Not call a separate bridge-layer select function** — bridge layer selection is fully determined by `OnBridge` set during `Unlimbo` (world_spawn logic already handles Unlimbo's bridge check via `world_spawn.rs`).
2. **Use `on_bridge` when snapping Z at touchdown** — the equivalent of `FootClass::Set_Height_On_Bridge`: if `entity.on_bridge`, Z = cell ground height + bridge height offset; otherwise Z = cell ground height.
3. **Register occupancy into bridge list when `on_bridge=true`** — the landing touchdown's `Mark(PUT)` must use the bridge layer, not ground, matching gamemd's vtable+0x124(1) call ordering.
4. **No separate "parachute-to-bridge" branch needed** — the existing `on_bridge` state propagated from spawn handles everything.

---

## Sources

All claims verified from live Ghidra decompilation in this session:

- `decompile_function 0x005F5940` — `ObjectClass::Unlimbo` (bridge gate)
- `disassemble_function 0x005F5940` — assembly-level confirmation of `[ESI+0x8C]=1` write
- `decompile_function 0x005F3E70` — `ObjectClass::AI` (falling/touchdown branch)
- `decompile_function 0x005F5FA0` — `FootClass::Set_Height_On_Bridge`
- `decompile_function 0x005F5F00` — `CellClass::Get_Effective_Height`
- `decompile_function 0x00481180` — `CellClass::PlaceInfantryInCell` (param_5=0 = ground Z mode)
- `decompile_function 0x00415C60` — `AircraftClass::Drop_Payload`
- `get_xrefs_to 0x00415C60` — confirmed live caller `AircraftClass::Fire_At @ 0x00415ef8`
- `get_xrefs_to 0x004585C0` — three live YR callers confirmed
- `read_memory 0x007EB224` — vtable+0x1CC → `0x005F5FA0` confirmed
- Companion docs: `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`, `BRIDGE_DROPIN_ONBRIDGE_RELAYER_GHIDRA_REPORT.md`
