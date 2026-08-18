# WarpAttachClass__Detach — function decode

**Address:** `0x0062a4a0`
**Kind:** function
**Proposed Ghidra label:** `WarpAttachClass__Detach` (existing label authoritative — plate comment update only)

---

## Summary

Detaches a `WarpAttachClass` helper object from its owning `TechnoClass` unit and performs
cleanup or relocation depending on the warp state. Called from `TeleportLocomotionClass__InitiateWarp`
at `0x007195cf` as the final cleanup step before the teleport locomotor begins the warp sequence.
Also called from multiple other paths: `FootClass__ReceiveDamage`, `TechnoClass__Receive_Radio`
(radio message 0xe), `TechnoClass__PerformDeploy`, `TechnoClass__StartFidget`,
`UnitClass__PerCellProcess`, and three sites in `TemporalClass__AI`.

The function has two main outcomes:
1. **Early detach** (timer not elapsed, no valid destination, or unit is aircraft): clears the
   warp-state tracking field `TechnoClass+0x6c` (param_1[9][0x1b]) and calls the unit's
   vtable+0xf8 method (likely `Unlimbo` or `Unshroud`). Then clears `param_1[10]` (target ptr).
2. **Full detach + placement**: validates a new cell, checks `WarpAttachClass__CanPlaceAtTarget`,
   and if placement succeeds, fires a full relocation sequence including ghost-cell assignment,
   vtable+0x3c8 (Limbo), vtable+0x480, vtable+0x484, vtable+0x488, and writes the locomotor's
   three timer fields (`TechnoClass+0x1a8/+0x1a9/+0x1aa`). Clears the anim listener registration
   and internal attach pointers.

Verified via `decompile_function 0x0062a4a0` and `get_xrefs_to 0x0062a4a0`.

---

## Active in YR

**Yes — live.** Called from `TeleportLocomotionClass__InitiateWarp` at `0x007195cf`
(UNCONDITIONAL_CALL, verified via `get_xrefs_to 0x0062a4a0`). Also called from
`FootClass__ReceiveDamage` (2 sites), `TechnoClass__Receive_Radio`, `TechnoClass__PerformDeploy`,
`TechnoClass__StartFidget`, and `UnitClass__PerCellProcess` — all YR-live callers.

The three `TemporalClass__AI` callers (`0x0062985f`, `0x00629e69`, `0x00629c62`) are on the
ChronoSphere weapon path (TS legacy), but the function itself is fully active via the
teleport-locomotor path — no gating flag.

---

## Signature

```c
void __fastcall WarpAttachClass__Detach(int **param_1)
```

`param_1` — `WarpAttachClass*` (pointer-to-array-of-pointers, `int**`). All field accesses
below are **WarpAttachClass** byte offsets via `int**` indexing (`param_1[N]` = byte offset `N×4`).

---

## WarpAttachClass fields accessed

`param_1` is `int**`, so `param_1[N]` = `*(int**)(WarpAttachClass_base + N×4)`.

| Index | Byte offset | Field | Role |
|---|---|---|---|
| `param_1[9]` | `+0x24` | TechnoClass owner ptr | The warping unit |
| `param_1[10]` | `+0x28` | Attached target ptr | Cleared to null at end |
| `param_1[0xb]` | `+0x2c` | Start frame | -1 (0xffffffff) if timer not started |
| `param_1[0xd]` | `+0x34` | Delay remaining ticks | Null if no delay; adjusted by elapsed |
| `param_1[0x11]` | `+0x44` | Anim object ptr | Cleared via vtable+0xf8 call |
| `param_1[0x12]` | `+0x48` | Internal ptr 1 | Cleared to null |
| `param_1[0x13]` | `+0x4c` | Internal ptr 2 | Cleared to null |
| `param_1[0x14]` | `+0x50` | Internal ptr 3 | Cleared to null |
| `*(char*)(param_1+0x15)` | `+0x54` (byte) | Listener-registered flag | Cleared after unregistering from anim-remove listener list |

---

## TechnoClass fields written (via `param_1[9]`)

`param_1[9]` = TechnoClass owner ptr. Offsets are **TechnoClass byte offsets** (direct byte
access via `piVar6[N]` where `piVar6` is `int*` = byte offset `N×4`).

| TechnoClass offset | Name | Written to | When |
|---|---|---|---|
| `+0x6c` (`param_1[9][0x1b]`) | warp_state_local | 0 | Early-detach and final path |
| `+0x6a0` (`+0x1a8×4`) | timer_start | `g_CurrentFrameCounter` | Both paths |
| `+0x6a4` (`+0x1a9×4`) | timer_duration | `uStack_2c` (rate timer value) | Both paths |
| `+0x6a8` (`+0x1aa×4`) | timer_delay | 0 (early) or `iVar2 * 3` (full) | Branched |
| `+0x694` (`+0x1a5×4`) | warp_attach_ptr | 0 | Final cleanup |
| `+0x432` (byte) | hide_cameo_flag | 0 | If set and house is human player |

---

## Behavioral analysis

### Phase 1 — Aircraft check and rate-timer read

```c
iVar2 = (**(code **)(*param_1[9] + 0x84))();  // GetTechnoType
cVar1 = *(char *)(iVar2 + 0xcce);              // TechnoType+0xcce: IsPlane flag
puVar3 = (uint *)RateTimer__Current();
// rate-timer bits: ((*puVar3 >> 0xc) + 1 >> 1 & 7) < 3  → branch on high/low rate
```

Reads the current rate-timer value into `uStack_2c` (used later for timer writes).
Checks `TechnoType+0xcce` (aircraft/IsPlane flag — same field as PostWarpValidation).
This flag takes the non-aircraft path for most teleporting units.

### Phase 2 — Early detach gate (non-aircraft path)

```c
piVar6 = param_1[0xd];                    // delay remaining (null = no delay)
if (param_1[0xb] == (int *)0xffffffff) {  // start frame = -1 = not started
LAB_0062a593:
    if (piVar6 != null) {
        // timer correction path: write timer fields and clear target
        param_1[9][0x1b] = 0;
        piVar6 = param_1[10];
        piVar6[0x1a8] = g_CurrentFrameCounter;
        piVar6[0x1a9] = iStack_8;
        piVar6[0x1aa] = 0;
        param_1[10][0x1a5] = 0;
        (**(code **)(*param_1[9] + 0xf8))();  // vtable+0xf8 (Unlimbo/Unshroud)
        param_1[10] = null;
        return;
    }
} else {
    iVar2 = g_CurrentFrameCounter - param_1[0xb];  // elapsed since start
    if (iVar2 < param_1[0xd]) {                    // timer not yet elapsed
        piVar6 = (param_1[0xd] - iVar2);            // remaining delay
        goto LAB_0062a593;
    }
}
```

If `start_frame == -1` (not started) or the timer has not elapsed: perform early detach
with zeroed `timer_delay` (`+0x1aa = 0`). If timer has elapsed, fall through to full detach.

### Phase 3 — Destination cell lookup

```c
// aircraft path: use direction-offset table for cell offset
// non-aircraft path: use unit's vtable+0x1b8 (GetCellPacked) result
puVar5 = (undefined4 *)(**(code **)(*param_1[10] + 0x1b8))();  // target cell packed coords
uStack_28 = *puVar5;  // packed cell X/Y short pair
```

Gets the destination cell. Aircraft path applies `g_DirectionOffsets` (directional nudge
table, 8 entries × 4 bytes) to offset the cell based on rate-timer direction bits.
Non-aircraft: uses destination unit's `GetCellPacked` directly.

Verified: `g_DirectionOffsets` table used for aircraft; straight cell lookup for ground units.

### Phase 4 — Passability and null-coord guard

Checks if the resolved cell `uStack_28` equals a "null" cell pair (`DAT_00ac4928 / DAT_00ac492a`).
If the cell is `(0,0)` (null sentinel), looks up the stored backup coordinate from three globals
(`DAT_00ac4948 / DAT_00ac494c / DAT_00ac4950`). Then calls `CellClass__CheckCellPassability`
— if impassable, calls `FUN_00703590` (likely `FootClass__Find_Nearby_Passable_Cell` variant)
to find an alternate landing cell.

### Phase 5 — Full detach + relocation

```c
cVar1 = WarpAttachClass__CanPlaceAtTarget();
if (cVar1 != '\0') {
    // vtable+0xd8: some flag check on owner
    // TechnoClass+0x432: hide-cameo flag → clear, call vtable+0x14c (if human player)
    // param_1[9][0x10d]: non-null → FUN_006ea500
    // vtable+0x4ac: IsInAir/some locomotor check
    if (cVar1 == '\0') {  // not in air
        TechnoClass__SetGhostCell();
        (**(code **)(*param_1[9] + 0x3c8))();   // vtable+0x3c8 (Limbo)
        (**(code **)(*param_1[9] + 0x480))(0);  // vtable+0x480
    }
    (**(code **)(*param_1[9] + 0x484))();
    (**(code **)(*param_1[9] + 0x488))(0,0,0,0,0);
    piVar7 = (**(code **)(*param_1[9] + 0x3f8))(0);  // vtable+0x3f8 (get locomotor?)
    iVar2 = *(int *)(*piVar7 + 0xb0);
    param_1[9][0x1a8] = g_CurrentFrameCounter;
    param_1[9][0x1a9] = uStack_2c;
    param_1[9][0x1aa] = iVar2 * 3;        // timer_delay = 3 × some locomotor value
    // clear attach pointers
    param_1[0x13] = param_1[0x14] = param_1[0x12] = null;
    // clear anim listener if registered
}
```

If `WarpAttachClass__CanPlaceAtTarget` succeeds, a full relocation sequence fires: ghost-cell,
Limbo, placement calls, and timer-delay set to `iVar2 * 3` where `iVar2` comes from
`(*(locomotor_ptr + 0xb0))` — likely a speed or cooldown field.

### Phase 6 — Common cleanup

```c
LAB_0062a862:
param_1[10][0xca] = 0;           // target field 0x328 = 0
param_1[10][0x1a8] = g_CurrentFrameCounter;
param_1[10][0x1a9] = uStack_2c;
param_1[10][0x1aa] = 0;
param_1[10][0x1a5] = 0;          // warp_attach_ptr = 0
param_1[10] = null;              // clear target ptr in WarpAttachClass
```

Unconditional cleanup at end of both paths: zeros the target's timer fields and clears
the target pointer.

---

## Struct fields accessed

### WarpAttachClass (via `param_1`, `int**`):

| `param_1[N]` | Byte offset | Purpose |
|---|---|---|
| `[9]` | `+0x24` | TechnoClass owner ptr |
| `[10]` | `+0x28` | Attached target ptr (cleared) |
| `[0xb]` | `+0x2c` | Start frame (-1 = not started) |
| `[0xd]` | `+0x34` | Delay remaining ticks |
| `[0x11]` | `+0x44` | Anim ptr (cleared via vtable+0xf8) |
| `[0x12/0x13/0x14]` | `+0x48/0x4c/0x50` | Internal ptrs (cleared) |
| `*(char*)(param_1+0x15)` | `+0x54` (byte) | Anim listener flag |

### TechnoClass (via `param_1[9]`):

| Offset | Name | Role |
|---|---|---|
| `+0x6c` | warp_state_local | Cleared to 0 |
| `+0x1a8×4` = `+0x6a0` | timer_start | Set to `g_CurrentFrameCounter` |
| `+0x1a9×4` = `+0x6a4` | timer_duration | Set to `uStack_2c` (rate timer) |
| `+0x1aa×4` = `+0x6a8` | timer_delay | 0 (early) or `locomotor_val × 3` |
| `+0x1a5×4` = `+0x694` | warp_attach_ptr | Cleared to 0 |
| `+0x432` (byte) | hide_cameo_flag | Cleared if human player |

---

## Globals / enums / INI keys

| Symbol | Address | Role |
|---|---|---|
| `g_DirectionOffsets` | Referenced as `&g_DirectionOffsets + uVar8 * 4` | 8-entry direction-nudge table for aircraft path |
| `DAT_00ac4928` / `DAT_00ac492a` | `0x00ac4928` / `0x00ac492a` | Null cell sentinel (X/Y as shorts) |
| `DAT_00ac4948` / `0x00ac494c` / `0x00ac4950` | Backup cell coords | Used when resolved cell is null sentinel |
| `g_CurrentFrameCounter` | Referenced inline | Written to timer_start fields |
| `g_AnimClass_RemoveListeners` | Referenced inline | Listener array for anim remove cleanup |
| `g_AnimClass_RemoveListeners_Count` | Referenced inline | Listener count |

---

## Out-of-scope refs

| Symbol | Address | Reason |
|---|---|---|
| `RateTimer__Current` | — | General timer utility; not teleport-specific |
| `MapClass__Get_CellClass` | `0x005657a0` | General map utility; not teleport-specific |
| `CellClass__CheckCellPassability` | Referenced inline | General cell utility; not teleport-specific |
| `MapClass__GetZoneID` | `0x0056d230` | General zone query; not teleport-specific |
| `FUN_00703590` | Referenced inline | General passable-cell fallback; not teleport-specific |
| `TechnoClass__SetGhostCell` | Referenced inline | General TechnoClass utility; not teleport-specific |
| `HouseClass__IsHumanPlayer` | Referenced inline | General house utility; not teleport-specific |
| `FUN_006ea500` | `0x006ea500` | Unknown; called when `TechnoClass+0x434` non-null; not teleport-specific |
| `WarpAttachClass__CanPlaceAtTarget` | `0x0062a6e9` (inline) | Internal helper; separate decode if needed |

---

## Unverified / YELLOW

- **`WarpAttachClass` struct layout**: The `param_1[N]` indices are read from decompile but the
  WarpAttachClass struct type does not exist in Ghidra (assumed from field pattern). Offsets at
  `+0x24/+0x28/+0x2c/+0x34/+0x44/+0x48/+0x4c/+0x50/+0x54` are YELLOW on exact field names;
  byte offsets are HIGH confidence from the `int**` indexing.

- **`TechnoClass+0x6c` (`param_1[9][0x1b]`)**: Written to 0 on early-detach and on the no-
  CanPlaceAtTarget path. Called `warp_state_local` here; exact name unverified against TechnoClass
  layout. YELLOW.

- **`vtable+0xf8` call on `param_1[9]`**: Called on early-detach paths. Likely `Unlimbo` or
  `Unshroud`; exact vtable slot identity unverified. YELLOW.

- **`iVar2 * 3` timer-delay factor**: `iVar2 = *(*(locomotor_ptr + 0xb0))` where `locomotor_ptr`
  comes from `vtable+0x3f8`. The `+0xb0` field of the locomotor and the `×3` multiplier purpose
  are unverified. YELLOW.

- **`DAT_00ac4928/0x00ac492a` null-cell sentinel**: Treated as a null cell coordinate pair (two
  shorts). Address not verified via `read_memory`. YELLOW.

- **`TemporalClass__AI` callers** (`0x0062985f`, `0x00629e69`, `0x00629c62`): These are ChronoSphere
  temporal weapon callers. The function behavior on those call paths is not analyzed here. Likely
  TS-legacy context but the function itself is YR-live via the teleport path.
