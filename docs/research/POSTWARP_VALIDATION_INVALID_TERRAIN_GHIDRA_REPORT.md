# PostWarpValidation Invalid-Terrain / ShouldSelfDestruct Chain

**Function:** `TeleportLocomotionClass::PostWarpValidation`  
**Address:** `0x7187A0`  
**Caller:** `TeleportLocomotionClass::StateMachineTick` @ `0x7192F0` — **sole caller**  
**Session date:** 2026-05-19  
**Status:** COMPLETE

---

## 1. Call-Site in StateMachineTick (Phase 5)

In `StateMachineTick` (0x7192F0), state `uVar3 == 5` runs the following (verified from
decompilation):

```
if (*(int *)(param_1[2] + 0x280) == 0) {
    iVar6 = TeleportLocomotionClass__PostWarpValidation(param_1[9], param_1[10], param_1[0xb]);
}
```

`param_1[2]` is the owning `TechnoClass*`.  
`+0x280` is `PendingWarpPhase` — set to `0` by `InitiateWarp` (self-teleport path) and to
`3` by the Chronosphere superweapon path (external caller sets it before triggering Phase 0).  
`param_1[9/10/0xb]` are the destination X/Y/Z coordinates passed to the validator.

**Critical finding: PostWarpValidation fires for both paths — self-teleport AND Chronosphere
— whenever Phase 5 executes, subject to the `+0x280 == 0` guard.**

For the **self-teleport path (chrono miner harvest return):**  
The self-teleport completes entirely within Phase 0 of StateMachineTick (state 0, single
tick). It does NOT advance `WarpPhase` (the state counter at `param_1[0xd]`). After Phase 0
the pre-phase check `BeingWarped==1 AND WarpPhase==0 AND PendingWarpPhase==0` routes every
subsequent tick to `TimerCheck` (0x719BF0), which just counts down the chrono-lock timer.
**The state machine never reaches state 5 on the self-teleport path; therefore
`PostWarpValidation` at 0x7187A0 is never called for a chrono miner self-teleport.**

For the **Chronosphere superweapon path:** the state machine runs states 0–7 sequentially
across multiple ticks. State 5 fires PostWarpValidation — unless `+0x280 != 0` at that
point (which would mean the warp was superseded or cancelled; in normal YR play
`+0x280` is 0 in state 5).

**Active in YR: Yes (Chronosphere path). No (self-teleport path).**

---

## 2. Field Identities Confirmed at the Call-Site

From `StateMachineTick` decompilation, state 7 (0x7192F0):

| Field offset (TechnoClass) | Name | Evidence |
|---|---|---|
| `+0x271` | `BeingWarped` | `*(undefined1 *)(piVar1 + 0x271) = 0` in state 7 clear-all block |
| `+0x270` | `IsWarpingOut` | `*(undefined1 *)(param_1[2] + 0x270) = 0` in state 2 |
| `+0x27C` | `ChronoInTransit` | `*(undefined1 *)(param_1[2] + 0x27c) = 0` in state 2 |
| `+0x280` | `PendingWarpPhase` | Guard: `if (*(int *)(param_1[2] + 0x280) == 0)` before PostWarpValidation call |
| `+0x3D5` | `Discovered` | `*(undefined1 *)(iVar6 + 0x3d5) = 0` in state 5 when not in playfield |
| `+0x428` | `ChronoSourceBuilding` | `*(undefined4 *)(param_1[2] + 0x428) = 0` cleared after PostWarpValidation |
| `+0x42C` | `ChronoSourceHouse` | `*(undefined4 *)(param_1[2] + 0x42c) = 0` cleared after PostWarpValidation |
| `+0x8C` | `IsOnBridge` | Set in states 0 and 2 from `CellClass+0x140 & 0x100` |

**`+0x3CD` (`ShouldSelfDestruct`):** This field is written inside `PostWarpValidation`
at `*(undefined1 *)(*(int *)(param_1 + 0xc) + 0x3cd) = 1` in two separate branches
(water death and bridge-fail path — see §3 and §4). The offset `0x3CD` on TechnoClass
is confirmed from the decompilation.

No `+0x3CD` read was observed; the function only **sets** it to 1 as a self-destruct flag.

---

## 3. Water / Impassable Terrain Death Formula

PostWarpValidation has two distinct death-trigger branches. The outer branch is:

```c
iVar4 = MapClass__Get_CellClass(destCoord);
// Check cell terrain type
if (*(int *)(iVar4 + 0xec) == 2)     // land type 2 = water
    && !bVar2                         // bVar2 = "has power surplus" exemption (see §5)
{
    if (unit is NOT a naval unit)     // vtable+0x2c != 0xf (SpeedType 15 = float)
    {
        if (cell has no bridge overlay)  // CellClass+0x140 & 0x100 == 0
        {
            if (cell land type != 1)     // not tiberium/ore
            {
                // SELF-DESTRUCT path
                *(TechnoClass+0x3CD) = 1;            // ShouldSelfDestruct = 1
                vtable[0x3A0]();                     // Die() call
                if (TechnoClass+0x2D8 != 0)          // active chrono anim exists
                {
                    FUN_006b0ae0(+0x428, +0x42C);    // handle kill credit
                    anim->vtable[0x20](1);            // Detach/destroy anim
                    TechnoClass+0x2D8 = 0;           // clear anim ptr
                }
                // check for docked passengers
                if (+0x10a == 0 && +0x10b == 0) return;
                if (+0x10a != 0) vtable[0xE0]();     // kill passengers
                else             vtable[0xE4]();
                return;
            }
        }
    }
}
```

The terrain type check is `CellClass+0xEC == 2`; the bridge-overlay check is
`CellClass+0x140 & 0x100`. No damage formula is used — the unit is simply killed
via the `Die()` vtable call (vtable offset `0x3A0`). There is no damage value
or warhead: it is an instant-kill.

The `FUN_006b0ae0` call at `0x6B0AE0` is the kill-credit handler: it iterates all
garrisoned/carried passengers (`+0x48` count, `+0x3C` pointer array) and calls their
`Die` or `TakeDamage` vtable depending on source params. When `+0x428` (ChronoSourceBuilding)
is NULL (as it is for self-teleport), `FUN_006b0ae0` calls
`vtable+0x16C(Rules+0xFA8, ...)` — delivering `g_RulesClass_Instance+0xFA8` as the
warhead, which is the generic "C4/demo" warhead stored in Rules. This applies to
**cargo units** carried by the destroyed unit, not to the teleporting unit itself which
is directly killed via `Die()`.

**Player-visible outcome for the chrono miner on Chronosphere path:**  
If the miner is warped onto water/impassable terrain via the Chronosphere superweapon and
no bridge overlay exists, the miner dies instantly. The death is attributed to
`ChronoSourceHouse` (`+0x42C`). The active chrono anim is destroyed. Any cargo in the
miner is killed via the Rules generic warhead.

**This path is unreachable on the self-teleport path** (see §1) — the miner's own
self-teleport bypasses PostWarpValidation entirely.

---

## 4. Occupied-Cell / Temporal-Weapon Occupant Damage

The function's first loop iterates units already occupying the destination cell:

```c
iVar4 = CellClass__Get_Cell_At(destCoord);
for (piVar1 = *(int **)(iVar4 + 0xe4); piVar1 != NULL; piVar1 = piVar1[0xc]) {
    cVar3 = piVar1->vtable[0x160]();    // IsInfantry or similar occupancy check
    if (cVar3 != 0) {
        iVar4 = TechnoClass->vtable[0x84]();        // GetTechnoType
        uStack_4 = *(iVar4 + 0xa0);                 // type's primary warhead
        vtable[0x16C](&uStack_4, 0, Rules+0xFA8, 0, 1, 0);  // TakeDamage call
    }
}
```

`CellClass+0xE4` is the occupant linked-list head. For each occupant, if the occupant
returns true for vtable slot `0x160` (confirmed: this slot is `IsCellOccupier` / infantry
occupant check), the occupier is hit with `TakeDamage` using the **warping unit's own
warhead** (`TechnoTypeClass+0xA0`, the type's primary weapon warhead reference) and
`Rules+0xFA8` as the damage multiplier source.

The warping unit itself is NOT destroyed in this branch — only the occupier takes damage.
This is **not** a mutual-kill; it is one-way: the teleporting unit survives, the occupant
takes a weapon hit.

**Active in YR: Yes (Chronosphere path only; see §1).**

---

## 5. Bridge Check (+0x8C IsOnBridge)

The outer section of PostWarpValidation runs this after the occupant loop and before the
water-death check:

```c
iVar4 = TechnoClass->vtable[0x84]();        // GetTechnoType
if (*(char *)(iVar4 + 0xcce) != 0) {        // type flag at TechnoType+0xCCE = Naval
    iVar4 = CellClass__Get_Cell_At(destCoord);
    if ((*(uint *)(iVar4 + 0x140) & 0x100) == 0) {
        CellClass__Get_Cell_At(destCoord);  // second call — appears to re-query, possibly side-effecting
    }
}
```

`TechnoTypeClass+0xCCE` = `Naval=` (verified: `TechnoTypeClass::ReadINI` at 0x00714A6A pushes
string `"Naval"` @ 0x0084395C and `MOV [EBP+0xCCE], AL` after ReadBool returns). The chrono
docs (CHRONO_MINER_SYSTEM_OVERVIEW §7, CHRONO_MINER_TELEPORT §12) mislabel this byte as
`Chronoshiftable` — the string `Chronoshiftable` is not present in the binary at all.

The check before querying bridge overlay reads:
`CellClass+0x140 & 0x100` is the "has bridge overlay" mask. If the flag is set but the
cell lacks a bridge overlay, the second `Get_Cell_At` is called; no explicit write to
`+0x8C` is performed inside this function itself.

`IsOnBridge (+0x8C)` is **written** in StateMachineTick states 0 and 2 (verified):
```c
if ((CellClass+0x140 & 0x100) == 0)
    TechnoClass+0x8C = 0;   // not on bridge
else
    TechnoClass+0x8C = 1;   // on bridge
```

So `+0x8C` is set to reflect the destination cell's bridge status at warp-in time, not
by PostWarpValidation. The bridge check inside PostWarpValidation uses
`CellClass__HasBridgeOverlay` (0x4865D0) in the second outer block:

```c
cVar3 = CellClass__HasBridgeOverlay();
if (cVar3 == 0 || SpeedType == 0xf) {
    // standard terrain-incompatible → kill via TakeDamage(Rules+0xFA8)
} else {
    // bridge present AND unit is not float → ShouldSelfDestruct path
    TechnoClass+0x3CD = 1;
    Die();
    ...
}
```

When the destination cell has a bridge overlay and the unit is not a naval unit,
the unit self-destructs rather than taking the "incompatible terrain" damage path.
This branch sets `+0x3CD = 1` (same ShouldSelfDestruct flag as the water-death path).

---

## 6. Parity Concern: Self-Teleport Skips All Validation

**The chrono miner's self-teleport path NEVER calls PostWarpValidation.**

The self-teleport completes in `StateMachineTick` Phase 0 (state 0), increments no
state counter, and routes all subsequent ticks through `TimerCheck`. States 1–7 are
never executed. This means:

- If a chrono miner self-teleports onto water or impassable terrain, the game will
  NOT kill it. It will arrive at an impassable cell, survive, and begin counting down
  its chrono lock timer.
- This is **original gamemd.exe behavior**, not a Rust-port bug to fix. The
  self-teleport path (Process + Phase 0) contains its own destination correction logic
  in `TeleportLocomotionClass::Process` (0x718B70): it calls
  `FootClass::Find_Nearby_Passable_Cell` and remaps the destination before warp-in.
  So the miner should never reach an impassable cell under normal conditions; if it
  does somehow reach one anyway, it does NOT self-destruct.

Active in YR for self-teleport: **No** (PostWarpValidation unreachable on this path).

---

## 7. Verified Facts (Load-Bearing)

1. **Sole caller:** `PostWarpValidation` (0x7187A0) is called only from
   `StateMachineTick` (0x7192F0), state 5, guarded by `TechnoClass+0x280 == 0`.
   Confirmed: `get_function_callers` returns exactly one entry.

2. **Self-teleport never reaches state 5.** StateMachineTick Phase 0 (self-teleport)
   terminates without advancing `WarpPhase`. Confirmed from decompilation: Phase 0 code
   path ends with `return uVar3 & 0xffffff00` without touching `param_1[0xd]`; the
   pre-phase check routes subsequent ticks to TimerCheck (0x719BF0), bypassing states
   1–7 entirely.

3. **`+0x3CD` is ShouldSelfDestruct, write-only in this function.**
   Written to 1 in two branches: (a) water/impassable terrain (no bridge), (b) bridge
   present + non-naval unit. Confirmed at decompiled addresses 0x71897D and 0x719…
   (bridge-fail inner block).

4. **Occupied-cell hit is one-way (occupier only).** The first loop in PostWarpValidation
   hits occupants with the warping unit's primary warhead; the warping unit itself is
   not damaged or killed in this loop. Confirmed: loop body calls `vtable[0x16C]` on
   `piVar1` (occupant), not on `param_1` (self).

5. **Water death is instant-kill via `Die()`, not a damage formula.**
   `vtable[0x3A0]` is the kill call. No `TakeDamage` invocation precedes it.
   `FUN_006b0ae0` (0x6B0AE0) handles cargo/passenger kill-credit only; the warping
   unit itself is killed directly. Confirmed from decompilation structure.

---

*Report file: `docs/research/POSTWARP_VALIDATION_INVALID_TERRAIN_GHIDRA_REPORT.md`*
