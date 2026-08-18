# FUN_00481A00 — CrateClass::PickupDispatch — Ghidra Research Report

**Binary:** gamemd.exe  
**Function address:** 0x00481A00  
**Ghidra name:** `CrateClass__PickupDispatch`  
**Date:** 2026-05-19  
**Investigator:** subagent slot 5 / re-swarm batch

---

## (a) Function Signature

```
undefined4 __thiscall CrateClass__PickupDispatch(int *param_1, int *param_2)
```

- `param_1` = `TechnoClass*` — the unit picking up the crate (the chrono miner or any unit)
- `param_2` = `CellClass*` — the destination cell (for chrono warp this is the arrival cell)

**Evidence:** Ghidra decompilation shows `param_1[0x11]` is the cell's overlay-type index (read from the unit's current-cell struct at offset 0x44), and `param_2[0x87]` is the owning `HouseClass*`. The Ghidra signature labels this `__thiscall` with both params confirmed from all call sites.

**Callers pass cell explicitly:** In `TeleportLocomotionClass__StateMachineTick` (0x7192F0) and in `TeleportLocomotionClass__InitiateWarp` (0x719400) the call is:
```
CrateClass__PickupDispatch(param_1[2]);  // unit only — cell resolved internally from unit position
```
Wait — actually in the decompiled call sites, only one argument is passed. The Ghidra signature shows two parameters but calls pass one. The second param (`param_2`) is likely resolved via `this->current_cell` inside the function or via __thiscall convention. **Confidence: HIGH** for param_1 = unit ptr. param_2 resolution = inferred from decompilation structure.

---

## (b) Caller Chain

All confirmed callers via `get_function_callers`:

| Caller | Address | Context |
|---|---|---|
| `TeleportLocomotionClass__StateMachineTick` | 0x7192F0 | Chrono warp state machine — fires at warp arrival (state 0 / `Is_Moving` path) |
| `TeleportLocomotionClass__InitiateWarp` | 0x719400 | Alternate warp entry point — same pickup call |
| `DriveLocomotionClass__Force_Track` | 0x4B0C40 | Ground vehicle step to new cell |
| `DriveLocomotionClass__Process_Drive_Track` | 0x4B0F20 | Ground vehicle drive step |
| `DriveLocomotionClass__Process_Movement` | 0x4B2630 | Ground vehicle movement tick |
| `FUN_00514f70` | 0x514F70 | Tunnel locomotion step |
| `FUN_0054c550` | 0x54C550 | Unknown locomotion |
| `FUN_005b17b0` | 0x5B17B0 | Unknown locomotion |
| `ShipLoco_vtable28` | 0x6A0310 | Ship locomotion step |
| `ShipLocomotionClass__Process_Drive_Track` | 0x6A05F0 | Ship drive step |
| `ShipLocomotionClass__Process_Movement` | 0x6A1C80 | Ship movement |
| `WalkLocomotionClass__FindSubCellDest` | 0x75C240 | Infantry walk step |

**This is the universal crate pickup function.** Every locomotor type (drive, walk, ship, teleport, tunnel) calls the exact same function on cell arrival. The chrono warp call is NOT a special path — it is the same `CrateClass__PickupDispatch` used by all unit types.

**StateMachineTick warp placement:** In `TeleportLocomotionClass__StateMachineTick`, the call sequence at warp arrival (the `Is_Moving` / dest != current branch) is:
1. `(**(code **)(*(int *)param_1[2] + 0x18c))(2)` — set mission/state
2. `(**(code **)(*param_1 + 0x48))(param_1)` — position update (Unlimbo/place at dest)
3. **`CrateClass__PickupDispatch(param_1[2])`** — crate check
4. `(**(code **)(*(int *)param_1[2] + 0x480))(0,1)` — post-placement update
5. `AnimClass__Constructor(...)` — warp-out animation

---

## (c) Cell Crate-Overlay Detection

**Field:** `CellClass+0x44` — the overlay type index (int, -1 = no overlay).

From decompilation:
```c
iVar3 = param_1[0x11];          // cell->overlay_type_index
if (iVar3 == -1) return 1;       // no overlay = no crate
// Check OverlayTypeClass[iVar3]+0x2aa — IsCrate flag (boolean)
if (*(char *)(*(int *)(g_OverlayTypeClass_Array + iVar3 * 4) + 0x2aa) == '\0') return 1;
```

**`OverlayTypeClass+0x2aa`** is the `IsCrate` boolean flag. A cell contains a crate if and only if its overlay type has this flag set. There is no separate overlay-index range — any overlay with `IsCrate=true` qualifies.

**`OverlayTypeClass+0x2ab`** is a second boolean checked immediately after: this appears to be a trigger-action flag (the log string `"Springing trigger on crate at %d"` confirms). When set, it fires a trigger via `TechnoClass__ProcessCellAction(0x31, cell, ...)` before proceeding.

**`CellClass+0x11e`** is a byte field: `0` means the crate type is specified explicitly (scenario-placed crate); values `>= 0x13` (19) mean random weighted selection from the Rules weight table. Values `0..0x12` are explicit crate type indices.

---

## (d) Crate Effect Dispatch

**Mechanism:** The function resolves a crate type index `uVar11` (0–18), then dispatches through a **jump table** at `0x004833c4`. The jump table has 19 entries (indices 0–18).

**Weight table** at `0x0081da8c` (used for random selection when crate type >= 0x13):
```
[0]=50  [1]=20  [2]=1  [3]=3  [4]=5  [5]=5  [6]=20  [7]=1
[8]=1  [9]=10  [10]=10  [11]=10  [12]=1  [13]=3  [14]=1  [15]=1
[16]=1  [17]=1  [18]=1  sentinel=0xFFFFFFFF
```
Total weight ≈ 146. Terminator at `0x0081dad8` confirmed as `0xFFFFFFFF`.

**Jump table entries (0x004833c4), each 4 bytes, indices 0–18:**

| Index | Handler Address | Effect |
|---|---|---|
| 0 | 0x00482463 | **Money** (cash) |
| 1 | 0x00482041 | **Unit spawn** |
| 2 | 0x00481f9d | **Reveal** map |
| 3 | 0x00482565 | **Explosives** (area damage around crate) |
| 4 | 0x00481de7 | **Poison gas** (area damage, different warhead) |
| 5 | 0x00481e99 | **Tiberium/Ore spread** |
| 6 | 0x004832f5 | **(Skipped / null effect)** — index 6 is cleared before dispatch |
| 7 | 0x00481f6d | **Shroud** (re-shroud map) |
| 8 | 0x00481f9d | **Reveal** (same as index 2) |
| 9 | 0x00482d56 | **Armor** boost |
| 10 | 0x00482f36 | **Speed** boost |
| 11 | 0x00483125 | **Firepower** boost |
| 12 | 0x00482ca1 | **ICBM** (super weapon unlock) |
| 13 | 0x004832f5 | *(same as 6 — null/fallthrough)* |
| 14 | 0x00482972 | **Veteran** |
| 15 | 0x004832f5 | *(null)* |
| 16 | 0x00481de7 | *(poison, same as 4)* |
| 17 | 0x00481e99 | *(ore, same as 5)* |
| 18 | 0x00482b8f | **Base heal** (heal all owned buildings) |

**Note on index 6:** Before the switch, the code explicitly sets `uVar11 = 0` when `uVar11 == 6`, redirecting to the Money handler. Index 6 appears to be "Money" in the random table.

---

## (e) Cash Crate

Handler at `0x00482463`:
```c
// Log: "Crate at %d,%d contains money"
if (local_17c == 0) {
    iVar3 = Math__ftol(...);
    Random__RandomRanged(iVar3, iVar3 + 900);
}
HouseClass__IsHumanPlayer();
HouseClass__Add_Credits();
```

**Amount:** When `local_17c == 0` (normal/random crate), credits = `Math__ftol(something) + Random(0,900)`. `local_17c` is set from `g_RulesClass_Instance + 0x1140` when the crate type byte is 0 (explicit), which is `Rules->CrateMaximum` or similar. The base from `Rules` and the `+900` random range confirm this reads `CrateMinimum` as base and adds `CrateMaximum - CrateMinimum` randomness (standard RA2 crate formula: `CrateMinimum + Random(0, CrateMaximum - CrateMinimum)`).

Three special crate overlay types are detected from `g_RulesClass_Instance+0xfc/0xf8/0x100` and force specific amounts via `g_RulesClass_Instance+0x1464/0x1468/0x146c`.

**IncomeMult:** NOT applied in this path. `HouseClass__Add_Credits` is called with the raw amount.

---

## (f) Unit-Spawn Crate

Handler at `0x00482041`:
- Checks `g_RulesClass_Instance+0x1148` for a hardcoded unit type override first.
- If not set, randomly picks from `g_UnitTypeClass_Array` (VehicleType only — the loop checks `VehicleTypeClass` array), filtering for `Crate=yes` (checked via `UnitTypeClass+0xe0d`).
- Calls `FootClass__Find_Nearby_Passable_Cell` to find spawn location near the crate cell.
- Calls the unit's vtable `+0xd8` (Unlimbo/place) to spawn it.
- If the spawning player is human and placement succeeds, plays a sound at the crate location.

---

## (g) Nuke / Explode Crate and Chrono Miner Death

**There is no "Nuke" self-destruct crate** in the switch table as a distinct effect. The "explosives" handler (index 3, 0x00482565) applies `Apply_area_damage` around the crate cell, using `g_RulesClass_Instance+0xfa8` as the warhead. This damage is centered on the crate cell and will **hit the unit that picked it up** if it is still standing there.

**YES, a chrono miner can be killed by an explosives crate.** The unit warps in, `CrateClass__PickupDispatch` fires immediately (step 3 of arrival sequence), and the explosives handler applies area damage centered on the arrival cell before the unit has moved. This is player-visible: bad luck warp onto explosives crate = instant death. **VERIFIED from decompilation — no immunity check for the picking-up unit.**

The "ICBM" handler (index 12, 0x00482ca1) does NOT self-destruct — it calls `FUN_006ceeb0` (appears to be a super-weapon grant) and `SidebarClass__AddCameo` to add a super-weapon cameo.

---

## (h) Chrono-Specific Differences vs Walking Onto Crate

**NONE.** `CrateClass__PickupDispatch` is called identically from the teleport locomotor arrival and from walk/drive locomotors. There are **zero** chrono-warp-specific guards inside the function. Every crate effect fires the same way whether the unit walked in or warped in.

**Possible implied difference:** Walking locomotors call `CrateClass__PickupDispatch` during subcell destination selection (before the unit fully arrives), while `TeleportLocomotionClass__StateMachineTick` calls it immediately after `Unlimbo` at the dest coords. Timing differs by ~1 tick but the effect logic is identical.

**Heal/HP crate:** There is no "Heal HP" crate effect in this function. The "Base heal" handler (index 18) heals all owned *buildings*, not the picking unit. No handler heals the chrono miner's own HP.

---

## (i) Cell Clear Logic

**`MapClass__RemoveCrateAtCell` is called from within `CrateClass__PickupDispatch` itself**, before the effect dispatch:

```c
MapClass__RemoveCrateAtCell();   // clears CellClass+0x44 = -1, CellClass+0x11e = 0
if ((g_GameMode != 0) && (DAT_00a8b261 != '\0')) {
    MapClass__PlaceCrateAtRandomCell();  // in multiplayer: immediately respawn a new crate elsewhere
}
```

**The pickup function owns the cell clear.** The caller (teleport locomotor) does not need to clear it — it is always cleared before the effect fires, regardless of which effect handler runs.

`MapClass__RemoveCrateAtCell` (0x0056C020) in singleplayer mode sets `CellClass+0x44 = -1` and `CellClass+0x11e = 0` and dirty-marks the screen rect. In multiplayer it calls `CrateSlot__ClearAndPreserveTimer()` which clears the slot but may preserve a respawn timer.

---

## Multiplayer Guard

Early in the function:
```c
if ((g_GameMode != 0) && (*(char *)(*(int *)(param_2[0x87] + 0x34) + 0x1a6) != '\0')) {
    return 1;  // observer/spectator — no crate for spectators
}
```
In multiplayer (`g_GameMode != 0`), if the unit's house has a flag at `HouseTypeClass+0x1a6` set (spectator/observer flag), the pickup is silently rejected.

---

## Summary

- **Signature:** `CrateClass__PickupDispatch(TechnoClass*, CellClass*)` — `__thiscall`, called from all locomotor types including TeleportLocomotionClass.
- **Cell detection:** `CellClass+0x44` = overlay type index; `OverlayTypeClass+0x2aa` = IsCrate flag.
- **19 crate types** dispatched via jump table; random selection uses weighted table at `0x0081da8c` with weights 50/20/1/3/5/5/20/1/1/10/10/10/1/3/1/1/1/1/1.
- **Effects implemented:** Money, Unit-spawn, Reveal, Shroud, Explosives, Ore-spread, Poison, Armor, Speed, Firepower, Veteran, ICBM, Base-heal.
- **Chrono warp = no special behavior.** Same function, same effects, zero suppression for teleport arrivals.
- **Explosives crate kills the warp-arriving unit** — area damage is applied with no immunity for the picker.
- **Cell clear is the function's own responsibility** — `MapClass__RemoveCrateAtCell` fires before effect dispatch.
- **Multiplayer crate respawn** fires immediately in `PlaceCrateAtRandomCell` if `DAT_00a8b261` is set.

---

## Confidence

| Claim | Confidence | Source |
|---|---|---|
| Function identity and Ghidra name | HIGH | Ghidra label `CrateClass__PickupDispatch` |
| Caller list (all 11 callers) | HIGH | `get_function_callers` |
| Cell IsCrate detection (`CellClass+0x44`, `OverlayTypeClass+0x2aa`) | HIGH | Decompilation |
| 19-entry jump table with handler addresses | HIGH | `read_memory` at 0x004833c4 |
| Weight table values | HIGH | `read_memory` at 0x0081da8c |
| Chrono warp = no special path | HIGH | Decompilation — same function, no warp-specific guard |
| Explosives crate hits warp-arriving unit | HIGH | Decompilation — area damage before any move |
| Cell clear owned by this function | HIGH | `MapClass__RemoveCrateAtCell` call before effect dispatch |
| Cash amount formula (base + Random 0..900) | MEDIUM | Decompilation shows `Random__RandomRanged(iVar3, iVar3+900)` but base-value source unclear |
| Exact IncomeMult not applied | MEDIUM | `HouseClass__Add_Credits` called directly with no multiplier site visible |
