# TeleportLocomotionClass__PostWarpValidation — function decode

**Address:** `0x007187a0`
**Kind:** function
**Proposed Ghidra label:** TeleportLocomotionClass__PostWarpValidation (existing label is authoritative — plate comment update only)

---

## Summary

Called from `TeleportLocomotionClass__StateMachineTick` state 5 after the unit has
teleported to its destination. Determines whether the landing cell is valid. If invalid
(water, impassable terrain, certain vtable-return states), sets `TechnoClass+0x3cd`
(falling/dying flag), calls the Die vtable method, invokes kill-credit attribution via
`FUN_006b0ae0`, and clears `TechnoClass+0x2d8`. Aircraft units bypass the water check
if their house has power surplus. Teleporter-type units (vtable+0x2c returns 0xf) bypass
the bridge check. Cell occupants at the destination receive a warp-anim notification
through vtable+0x16c.

Verified via `decompile_function 0x007187a0`.

---

## Active in YR

**Yes — unconditionally live.** Called from `TeleportLocomotionClass__StateMachineTick`
at `0x00719ac4` (UNCONDITIONAL_CALL, verified via `get_xrefs_to 0x007187a0`). No gating
flag. Fires on every completed warp in a standard YR game.

---

## Signature

```c
void __thiscall TeleportLocomotionClass__PostWarpValidation(int param_1, int param_2, int param_3)
```
- `param_1` — `TeleportLocomotionClass*` (this pointer via thiscall; `*(int*)(param_1+0xc)` = TechnoClass ptr)
- `param_2` — destination X coordinate (leptons, NW-cell frame)
- `param_3` — destination Y coordinate (leptons, NW-cell frame)

Caller in StateMachineTick state 5:
```c
if (*(int *)(param_1[2] + 0x280) == 0) {
    TeleportLocomotionClass__PostWarpValidation(param_1[9], param_1[10], param_1[11]);
    //  param_1[9..11] = TeleportLocomotionClass+0x24/0x28/0x2c = dest coords
}
```
Guard: skipped if `TechnoClass+0x280` (WarpState) != 0.

---

## Behavioral analysis

### Phase 1 — Warp occupants at destination cell

```c
iVar4 = CellClass__Get_Cell_At(&param_2);
for (piVar1 = *(int**)(iVar4+0xe4); piVar1 != NULL; piVar1 = piVar1[0xc]) {
    cVar3 = (**(code**)(*piVar1 + 0x160))();  // vtable+0x160: "can be warped" check
    if (cVar3) {
        iVar4 = GetTechnoType();
        // call vtable+0x16c with (TechnoType+0xa0, 0, Rules+0xfa8, 0, 1, 0)
        (**(code**)(**(int**)(param_1+0xc) + 0x16c))(&TechnoType_field, 0, g_Rules+0xfa8, 0, 1, 0);
    }
}
```
Walks the ground-occupant list at the destination cell (`CellClass+0xe4`). For each
occupant whose vtable+0x160 returns true (warpable), calls vtable+0x16c — the warp-
notification/displacement method — passing `g_RulesClass_Instance+0xfa8` as the third arg.
`Rules+0xfa8` is likely `ChronoCellSpread` or a warp-anim type.
Verified: `decompile_function 0x007187a0` entry loop.

### Phase 2 — Aircraft exception: plane check

```c
iVar4 = GetTechnoType();
if (*(char*)(iVar4 + 0xcce) != '\0') {  // TechnoType+0xcce: IsPlane or CanFly flag
    // check bridge overlay for aircraft landing
}
```
If TechnoType+0xcce (likely `IsPlane` / aircraft flag) is set, performs a bridge-overlay
check on the destination cell but does NOT set the dying flag — aircraft are handled
differently. Verified from decompile.

### Phase 3 — Aircraft kind + power check

```c
bVar2 = false;
iVar4 = GetTechnoType();
if (*(int*)(iVar4 + 0x67c) == 3) {  // TechnoType+0x67c: SpeedType==3 (aircraft)
    bVar2 = true;  // aircraft = exempt from water death
    iVar4 = GetTechnoType();
    if (*(char*)(iVar4 + 0x410) != '\0') {  // TechnoType+0x410: Powered flag
        HouseClass__HasPowerSurplus(TechnoClass+0x21c);  // owning house
        if (!surplus) bVar2 = false;  // no power → NOT exempt
    }
}
```
Aircraft (SpeedType == 3) are initially exempt from water death (`bVar2 = true`).
However, if the aircraft type has `Powered=yes` (`TechnoType+0x410`) AND the owning
house (`TechnoClass+0x21c`) has no power surplus: the exemption is revoked.
Verified: `HouseClass__HasPowerSurplus @ 0x0050e1b0` confirmed in callees.
`TechnoClass+0x21c` = owning HouseClass ptr (direct byte offset; param_1 is int not int*).

### Phase 4 — Water / terrain death check

```c
uStack_4 = cell_coords(param_2, param_3);
iVar4 = MapClass__Get_CellClass(&uStack_4);
if (*(int*)(iVar4 + 0xec) == 2 && !bVar2) {  // CellClass+0xec: TerrainType==2 (water)
    iVar4 = GetTechnoType();
    if (*(char*)(iVar4 + 0xcce) == '\0') {   // not a plane
        iVar4 = GetTypeID();
        if (iVar4 != 0xf) {                  // not a teleporter (kind 0xf)
            iVar4 = CellClass__Get_Cell_At(&param_2);
            if ((*(uint*)(iVar4+0x140) & 0x100) == 0) {  // no bridge overlay
                iVar4 = CellClass__Get_Cell_At(&param_2);
                if (*(int*)(iVar4 + 0xec) != 1) {  // TerrainType != 1 (not bridge)
                    // DEATH PATH
                    *(TechnoClass+0x3cd) = 1;       // falling/dying flag
                    vtable+0x3a0();                  // Die()
                    if (TechnoClass+0x2d8 != 0) {
                        FUN_006b0ae0(TechnoClass+0x428, TechnoClass+0x42c);  // kill credit
                        piVar1 = *(int**)(TechnoClass+0x2d8);
                        if (piVar1) (**(code**)(piVar1+0x20))();  // vtable+0x20 on +0x2d8 obj
                        *(TechnoClass+0x2d8) = 0;
                    }
                    // then check piVar1[0x10a] / [0x10b] for final dispatch
                    return;
                }
            }
        }
    }
}
```
**Death conditions (all required):**
1. Destination cell is water (`CellClass+0xec == 2`)
2. Unit is NOT an exempt aircraft (`!bVar2`)
3. TechnoType+0xcce (IsPlane) is not set
4. GetTypeID() != 0xf (not a teleporter-type unit — Chrono Legionnaire immune)
5. Destination cell has NO bridge overlay (`CellClass+0x140 & 0x100 == 0`)
6. Destination cell terrain is NOT 1 (bridge)

If all conditions met: sets dying flag, calls Die, invokes kill-credit chain.

Verified: `CellClass__HasBridgeOverlay @ 0x004865d0` and `FUN_006b0ae0 @ 0x006b0ae0`
confirmed in callees.

### Phase 5 — Impassable terrain check + bridge fallback

```c
uStack_4 = cell_coords;
iVar4 = *vtable+0x1ac(MapClass__Get_CellClass, -1, -1);  // Can_Enter_Cell?
if (iVar4 == 7 || cVar3) {  // 7 = blocked/impassable
    MapClass__Get_CellClass(&uStack_18);
    cVar3 = CellClass__HasBridgeOverlay();
    if (cVar3 == '\0' || GetTypeID() == 0xf) {
        // warp occupants at blocked cell (vtable+0x16c with Rules+0xfa8)
    } else {
        // Has bridge overlay AND NOT teleporter-type → DEATH PATH
        *(TechnoClass+0x3cd) = 1;
        vtable+0x3a0();  // Die()
        if (TechnoClass+0x2d8 != 0) {
            FUN_006b0ae0(TechnoClass+0x428, TechnoClass+0x42c);
            if (piVar1 != NULL) (**(code**)(piVar1+0x20))(1);
            *(TechnoClass+0x2d8) = 0;
        }
        return;
    }
}
```
Second death path: cell returns Can_Enter_Cell = 7 (blocked) AND cell has bridge
overlay AND unit is not teleporter-type → die. Bridge-on-blocked is a specific edge
case (bridge construction zone, partial bridge, etc.).

If the cell is blocked but either has NO bridge overlay OR unit is teleporter-type:
warp occupants via vtable+0x16c (same as Phase 1) and survive.

---

## Struct fields accessed

**TechnoClass** (via `*(int*)(param_1+0xc)`, direct byte offset):

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0x21c` | direct | Owning HouseClass ptr | Power-surplus check for aircraft |
| `+0x2d8` | direct | Anim/object ptr | Cleared after die; object at +0x2d8 notified via vtable+0x20 |
| `+0x3cd` | direct byte | Falling/dying flag | Set to 1 to trigger death sequence |
| `+0x428` | direct | Source building ptr | Kill-credit attribution arg 1 |
| `+0x42c` | direct | Source house ptr | Kill-credit attribution arg 2 |

**TechnoType** (via `GetTechnoType()` = `vtable+0x84`):

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0x410` | direct byte | Powered flag | Aircraft power-check gate |
| `+0x67c` | direct | SpeedType | 3 = aircraft; exempts from water death |
| `+0xa0` | direct | (TechnoType field) | Passed to vtable+0x16c warp call |
| `+0xcce` | direct byte | IsPlane/aircraft flag | Skips certain water checks |

**CellClass**:

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0xe4` | direct | Ground-occupant list head | Walked in Phase 1 |
| `+0xec` | direct | TerrainType | 1=bridge, 2=water |
| `+0x140` | direct | Cell flags bitmask | Bit 0x100 = bridge overlay |

---

## Globals / enums / INI keys

| Symbol | Address | Role |
|---|---|---|
| `g_RulesClass_Instance` | inline | Rules singleton |
| `Rules+0xfa8` | — | Warp occupant anim type / ChronoCellSpread (passed to vtable+0x16c) |

---

## Out-of-scope refs

- `CellClass__Get_Cell_At` (`0x00565730`) — general map utility; not teleport-specific
- `MapClass__Get_CellClass` (`0x005657a0`) — general map utility; not teleport-specific
- `CellClass__HasBridgeOverlay` (`0x004865d0`) — general map utility; not teleport-specific
- `HouseClass__HasPowerSurplus` (`0x0050e1b0`) — general house utility; not teleport-specific
- `FUN_006b0ae0` (`0x006b0ae0`) — kill-credit attribution helper; called from multiple game paths, not teleport-specific

---

## Unverified / YELLOW

- **`Rules+0xfa8` identity**: Passed as the third argument to vtable+0x16c (warp-occupant
  notification). The task description calls it `ChronoCellSpread` or "warp anim type." Not
  verified against the Rules INI key layout. YELLOW.

- **`TechnoType+0xcce` exact flag name**: Used as an aircraft/plane gate. The field name
  (likely `IsPlane`, `CanFly`, or a sub-type flag) is unverified against TechnoTypeClass
  layout. YELLOW.

- **GetTypeID vtable+0x2c returns `0xf` for teleporter**: Assumed from context (teleporter
  locomotor type ID = 0xf). Consistent with the warp-occupant vtable+0x160 check in Phase 1
  but not independently verified. YELLOW.

- **TechnoClass+0x2d8 object**: After die, the object at `TechnoClass+0x2d8` has its
  vtable+0x20 called (with arg 1 in second path). The type of this object is unknown.
  Likely a linked anim or helper object. YELLOW.

- **`piVar1[0x10a]` / `[0x10b]` final dispatch**: At the end of the first death path,
  TechnoClass fields 0x10a×4=0x428 and 0x10b×4=0x42c (building/house ptrs — same as
  +0x428/+0x42c). The vtable calls on these (`+0xe0`, `+0xe4`) are the final kill/award
  dispatch. Exact method names unverified. YELLOW.
