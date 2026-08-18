# TeleportLocomotionClass__StateMachineTick — function decode

**Address:** `0x007192f0`
**Kind:** function
**Proposed Ghidra label:** TeleportLocomotionClass__StateMachineTick (existing label is authoritative — plate comment update only)

---

## Summary

8-state warp dispatch function. On every locomotor tick it reads the state counter
at `TeleportLocomotionClass+0x34` (`param_1[0xd]`, param_1 is `int*`) and advances
the warp pipeline from departure-anim through position teleport, ChronoDelay wait,
and arrival-side cleanup. States 0 and 2 also contain an inline InitiateWarp path
that fires when `ChronoInTransit` is set and a valid destination is cached.

Verified via `decompile_function 0x007192f0`.

---

## Active in YR

**Yes — unconditionally live.** Called through the ILocomotion vtable slot at
`0x007f5040` (vtable base `0x007f5000`, slot index `0x10` = offset `0x40`).
Verified via `read_memory 0x007f5038` (bytes `f0 92 71 00` at offset +8) and
`get_xrefs_from 0x007f5040` (resolves to `0x007192f0`). No gating flag. Fires
every game tick while the locomotor is active.

---

## Decompile excerpt — key structural points

`param_1` is `int*`. Struct field accesses:

- `param_1[2]` → `TeleportLocomotionClass+0x08` = pointer to owning TechnoClass
- `param_1[0xd]` → `TeleportLocomotionClass+0x34` = state counter (0..7)
- `param_1[-1]` → vtable pointer of TeleportLocomotionClass itself (for self-dispatch)
- `param_1[-1]+0x28` → ILocomotion vtable slot 0x28/4 = slot 10, dispatched as the timer-check method

All TechnoClass field accesses use `piVar1 = (int*)param_1[2]` then `*(piVar1 + offset)` where param_1[2] is `int` not `int*`, so all offsets are direct byte offsets.

---

## State machine transitions (all verified from decompile)

### State 0 — Early exits and InitiateWarp inline branch

Three guards at entry:

1. **WarpingOut short-circuit** (verified: `decompile_function 0x007192f0` entry block):
   If `TechnoClass+0x271` (WarpingOut flag) is set AND `state==0` AND `TechnoClass+0x280`
   (WarpState/pending warp) is 0: dispatch `param_1[-1]+0x28` (timer-check vtable slot) and
   return. This prevents re-entry during an active warp animation. Source: `*(char*)((int)piVar1 + 0x271) != '\0' && param_1[0xd] == 0 && piVar1[0xa0] == 0`.

2. **WarpState copy** (verified): If `state==0` AND `TechnoClass+0x280 != 0`:
   copy TechnoClass+0x280 into state counter and return. This lets an external caller
   fast-forward the state machine.

3. **InTransit check** (verified): If `TechnoClass+0x270` (`ChronoInTransit` gate,
   `piVar1[0x9f]` — direct byte offset) is non-null: fall into State-0 body below.
   Otherwise proceed to uVar3 < 1 branch (InitiateWarp inline path).

**State 0 / uVar3 < 1 path — InitiateWarp inline:**
When `param_1[-1]+0x10` vtable dispatch (Is_ChronoInTransit check) returns non-zero
AND the cached destination (`param_1[9..11]`) differs from both the TechnoClass current
coords and the g_NullCoord sentinel:

- Calls `TechnoClass__StopAllTargeting` (verified callees list)
- Scans g_BulletClass_Array for bullets targeting this unit; calls `BulletClass__UpdateTarget`
  for each (verified callees list)
- Spawns `AnimClass__Constructor` at TechnoClass+0x9C/+0xA0/+0xA4 (current Location,
  **depart cell**) using `g_RulesClass_Instance+0x33c` (WarpOut anim — see memory
  `feedback_chrono_miner_no_arrival_shimmer`: anim at **depart** only)
- Computes warp delay via `InitiateWarp` inline clone (same logic as `0x00719400`,
  see Chrono harvester flag short-circuit below)
- Calls `WarpAttachClass__Detach` if `TechnoClass+0x694 != 0`
- Calls vtable+0x124 (Hide/Unlimbo), vtable+0x84 (get TechnoType), checks sound vocoder
  at `TechnoType+0x578` vs `g_RulesClass_Instance+0x21c`; plays departure sound via
  `VocClass__PlayAt`
- Sets `TechnoClass+0x8c` (bridge flag) based on destination cell overlay
  (`CellClass+0x140 & 0x100`)
- Calls vtable+0x1cc, vtable+0x124(1), vtable+0x18c(2)
- Spawns second `AnimClass__Constructor` at TechnoClass+0x9C/+0xA0/+0xA4 (current
  Location, **same departure cell again** — confirmed by the `iVar6 = param_1[2]`,
  `*(iVar6+0x9c)` pattern)
- Clears `TechnoClass+0x280`, returns

**Chrono harvester short-circuit** (verified: `decompile_function 0x007192f0`):
```c
iVar6 = (**(code **)(*(int *)param_1[2] + 0x2c))();  // GetTypeID
if ((iVar6 == 1) && (*(char *)(*(int *)(param_1[2] + 0x6c4) + 0xe0e) != '\0')) {
    // TechnoClass+0x6c4 = TechnoType*, TechnoType+0xe0e = Teleporter flag
    param_1[0xe] = g_CurrentFrameCounter;
    param_1[0xf] = ...;
    param_1[0x10] = 0;  // forces being_warped_ticks = 0
    *(undefined1 *)(param_1[2] + 0x271) = 0;  // clear WarpingOut flag
}
```
`TechnoClass+0x6c4` is `TechnoType*` pointer (int, direct byte offset). `TechnoType+0xe0e`
is the Teleporter boolean INI flag. When unit is type 1 (infantry) with Teleporter=yes,
warp delay is forced to 0 (instant teleport). Active in YR for Chrono Legionnaire.

### State 0 body (ChronoInTransit set path — `piVar1[0x9f] != 0`):

```c
*(undefined1 *)(piVar1 + 0x9c) = 1;  // TechnoClass+0x270 set
param_1[0xe] = g_CurrentFrameCounter;
param_1[0xf] = ...;
param_1[0x10] = 0x3c;  // 60 ticks hardcoded
param_1[0xd] += 1;      // → state 1
```
Sets `TechnoClass+0x270` (ChronoInTransit flag), initialises timer to 60 ticks
(hardcoded 0x3c), advances to state 1.

### State 1 — Wait for timer

```c
uVar3 = (**(code **)(param_1[-1] + 0x28))();  // TimerCheck vtable dispatch
return uVar3 & 0xffffff00;
```
Dispatches through `param_1[-1]+0x28` (ILocomotion vtable slot 10 = TimerCheck).
No field writes. Stays in state 1 until timer expires; TimerCheck advances state to 2.

### State 2 — Warp-in anim + teleport to destination

```c
// Spawn WarpIn anim at CURRENT location (departure cell)
AnimClass__Constructor(g_RulesClass_Instance+0x33c, TechnoClass+0x9c);
// vtable+0x124 (Mark_Occupation), vtable+0x84 (GetTechnoType)
// check TechnoType+0x578 vs Rules+0x21c for sound; play via VocClass__PlayAt at TechnoClass+0x27/28/29
*(TechnoClass+0x271) = 1;  // WarpingOut = 1
*(TechnoClass+0x27c) = 0;  // ChronoInTransit = 0
*(TechnoClass+0x270) = 0;  // ChronoInTransit gate = 0
*(TechnoClass+0x8c)  = 0;  // bridge flag = 0
// Teleport to destination coords
TeleportLocomotionClass__Update_Position(TechnoClass+0x288, TechnoClass+0x28c);
param_1[0xd] += 1;  // → state 3 (or state 4 if Update_Position returns true)
```
Anim spawned at `TechnoClass+0x9c/+0xa0/+0xa4` — **departure cell** (confirmed: reads
Location at +0x9c before Update_Position moves the unit). `TechnoClass+0x288/+0x28c/+0x290`
are destination coords (leptons, NW-cell frame). Sound played from `TechnoClass+0x27`
(piVar1[0x27..0x29] = TechnoClass coords at destination, confirmed `local_1c = piVar1[0x28]`
etc.). Clears `ChronoInTransit+0x27c` and gate `+0x270`, clears bridge flag.

If `Update_Position` returns true (unit is already at destination): skip to state 4
(`param_1[0xd] = iVar6 + 2`).

### State 3 — Move to dest, set ChronoDelay

```c
TeleportLocomotionClass__Update_Position(piVar1[0xa2..]);  // TechnoClass+0x288 dest
if (reached) param_1[0xd] += 1;  // → state 4
*(TechnoClass+0x284) = *(g_RulesClass_Instance+0xbec);  // ChronoDelay (ticks)
```
Sets `TechnoClass+0x284` from `Rules+0xbec` (ChronoDelay INI key) unconditionally
each tick. Advances to state 4 once destination reached.

### State 4 — Final placement + validate

```c
TeleportLocomotionClass__Update_Position(piVar1[0xa2..]);
vtable+0x1b4();  // Mark occupation bits
vtable+0x1cc();  // Place unit
vtable+0x124();  // Unlimbo
param_1[0xd] += 1;  // → state 5
```
No condition on Update_Position here — always advances. Places unit at destination.

### State 5 — PostWarpValidation + timer arm + arrival anim

```c
vtable+0x1b4();   // Unmark occupation
vtable+0x1cc();   // (1) Remove/place
vtable+0x124(1);  // Limbo flag set
// GetTechnoType; check TechnoType+0x574 vs Rules+0x218; play arrival sound
// MapClass__Is_Cell_In_Playfield; if not → clear TechnoClass+0x3d5
if (TechnoClass+0x280 == 0):
    TeleportLocomotionClass__PostWarpValidation(dest_x, dest_y, dest_z)
// if TechnoClass+0x24 (CanBeRecruited/IsBase flag) set:
//   vtable+0x18c(2), vtable+0x48 (Mark_All_Occupation_Bits)
//   *(TechnoClass+0x428) = 0; *(TechnoClass+0x42c) = 0  (source building ptrs)
//   TechnoClass__SetGhostCell(0)
//   vtable+0x480(0,1)
//   arm timer: param_1[0xe]=CurrentFrame, param_1[0x10]=TechnoClass+0x284
//   spawn AnimClass at TechnoClass+0x9c (ARRIVAL cell)
//   param_1[0xd] += 1  // → state 6
```
**IMPORTANT — anim location distinction:** In state 2 the anim is at **depart** coords
(TechnoClass+0x9c before teleport). In state 5 the anim is also at TechnoClass+0x9c —
but at this point the unit has already been teleported to the destination, so `+0x9c`
is the **arrival cell**. This is the WarpIn shimmer at the refinery pad. Consistent
with memory `feedback_chrono_miner_no_arrival_shimmer` only if the "arrival shimmer"
referenced there is the state-2 anim (which is at departure). State-5 anim is
observable at the arrival location.

Sound at state 5 uses `TechnoType+0x574` (WarpIn sound) vs `Rules+0x218`
(global WarpIn fallback).

`TechnoClass+0x280` guard: if non-zero (warp was cancelled or short-circuited) skips
PostWarpValidation. Source-building ptrs cleared from `TechnoClass+0x428/+0x42c`.

### State 6 — Wait for ChronoDelay timer

Same as state 1: dispatch `param_1[-1]+0x28` (TimerCheck vtable), stays until
timer (set from TechnoClass+0x284 = ChronoDelay ticks) expires.

### State 7 — Final cleanup, reset state

```c
*(TechnoClass+0x271) = 0;  // BeingWarped = 0
TechnoClass__SetGhostCell()
vtable+0x480()
*(TechnoClass+0xc4 offset in param_1) = 0;  // *(param_1+0xc cast byte) = 0
*(TechnoClass+0x280) = 0;  // WarpState = 0
param_1[0xd] = 0;  // reset state machine to 0
```
Note: `*(param_1[2]+0x280)` vs `param_1+0xc` — the `*(param_1+0xc)` write is
`TeleportLocomotionClass+0x30` (a cached coord, set to 0 as cleanup), not WarpState.
`TechnoClass+0x218` in the plate comment is the WarpState field accessed as
`piVar1[0xa0] = *(int*)(piVar1 + 0x280)` (direct byte, 0xa0×4 NOT applicable here —
param_1[2] is `int`, piVar1 is `int*`, so `piVar1[0xa0] = *(piVar1+0xa0)` = byte
offset `0xa0×4 = 0x280`; confirmed).

---

## Struct fields accessed

All TechnoClass fields via `piVar1 = (int*)(param_1[2])` (int* pointer, so index × 4):

| Field | Offset | Name | Role |
|---|---|---|---|
| `TechnoClass+0x270` | piVar1[0x9c] byte | ChronoInTransit gate | Cleared in state 2 |
| `TechnoClass+0x271` | `*(piVar1+0x271)` direct byte | WarpingOut flag | Set/cleared across states |
| `TechnoClass+0x27c` | `*(piVar1+0x27c)` direct byte | ChronoInTransit | Cleared in state 2 |
| `TechnoClass+0x280` | `piVar1[0xa0]` = index×4 = 0x280 | WarpState / pending counter | Read/write states 0/7 |
| `TechnoClass+0x284` | `*(piVar1+0x284)` direct | ChronoDelay countdown | Written state 3, read state 5 timer |
| `TechnoClass+0x288` | `*(piVar1+0x288)` direct | Dest X leptons (NW-cell frame) | Used states 2–4 |
| `TechnoClass+0x28c` | `*(piVar1+0x28c)` direct | Dest Y leptons (NW-cell frame) | Used states 2–4 |
| `TechnoClass+0x290` | `*(piVar1+0x290)` direct | Dest Z leptons | Used states 2–4 |
| `TechnoClass+0x3d5` | direct byte | in-playfield flag | Cleared if not in playfield (state 5) |
| `TechnoClass+0x428` | `*(piVar1+0x428)` direct | Source building ptr (primary) | Cleared state 5 |
| `TechnoClass+0x42c` | `*(piVar1+0x42c)` direct | Source building ptr (secondary) | Cleared state 5 |
| `TechnoClass+0x574` | via TechnoType sub-read | WarpIn sound index | State 5 sound check |
| `TechnoClass+0x578` | via TechnoType sub-read | WarpOut sound index | State 2 sound check |
| `TechnoClass+0x694` | `*(piVar1+0x694)` direct | WarpAttachClass ptr | WarpAttach detach check |
| `TechnoClass+0x6c4` | `*(piVar1+0x6c4)` direct | TechnoType ptr | Teleporter flag lookup |
| `TechnoClass+0x8c` | direct byte | Bridge-on-destination flag | Set/cleared in state 0/2 |
| `TechnoClass+0x9c` | `*(piVar1+0x9c)` direct | Location X leptons | Anim spawn coords |
| `TechnoClass+0xa0` | `*(piVar1+0xa0)` direct | Location Y leptons | Anim spawn coords |
| `TechnoClass+0xa4` | `*(piVar1+0xa4)` direct | Location Z leptons | Anim spawn coords |

TeleportLocomotionClass fields (param_1 is `int*`, index × 4):

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0x00` (param_1[-1] used indirectly) | 0x00 | vtable ptr | Self-dispatch |
| `+0x08` (param_1[2]) | 0x08 | TechnoClass ptr | Owner |
| `+0x24` (param_1[9]) | 0x24 | Dest cache X | Cached destination X |
| `+0x28` (param_1[10]) | 0x28 | Dest cache Y | Cached destination Y |
| `+0x2c` (param_1[11]) | 0x2c | Dest cache Z | Cached destination Z |
| `+0x34` (param_1[0xd]) | 0x34 | State counter | 0..7 |
| `+0x38` (param_1[0xe]) | 0x38 | Timer start frame | g_CurrentFrameCounter at arm |
| `+0x3c` (param_1[0xf]) | 0x3c | Timer reserved | |
| `+0x40` (param_1[0x10]) | 0x40 | Timer ticks | Duration in frames |

---

## Globals / enums / INI keys

| Symbol | Address | Role |
|---|---|---|
| `g_NullCoord_Teleport_X` | `0x00b0ebd8` | Sentinel X (no-destination check) |
| `g_NullCoord_Teleport_Y` | `0x00b0ebdc` (inferred) | Sentinel Y |
| `g_NullCoord_Teleport_Z` | `0x00b0ebe0` (inferred) | Sentinel Z |
| `g_CurrentFrameCounter` | read inline | Current game frame |
| `g_BulletClass_Array` | read inline | Bullet array base |
| `g_BulletClass_Array_Count` | read inline | Bullet count |
| `g_RulesClass_Instance` | read inline | Rules singleton |
| `Rules+0xbec` | ChronoDelay | Ticks for warp post-arrival hold (state 3 write) |
| `Rules+0xbf4` | ChronoDistanceFactor | Divisor for distance → delay (InitiateWarp inline) |
| `Rules+0xbf8` | ChronoTrigger | Bool: enable distance-based delay |
| `Rules+0xbfc` | ChronoMinimumDelay | Minimum warp delay ticks |
| `Rules+0xc00` | ChronoRangeMinimum | Distance threshold; if below, use MinimumDelay |
| `Rules+0x21c` | WarpIn fallback sound | Global fallback if TechnoType has no WarpIn |
| `Rules+0x218` | WarpOut fallback sound | Global fallback for WarpOut |
| `Rules+0x33c` | WarpOut anim type ptr | AnimType used for both depart and arrive anims |
| `TechnoType+0xe0e` | Teleporter flag | Chrono harvester instant-warp gate |

Verified: all loads from `g_RulesClass_Instance` offsets confirmed in decompile
(`decompile_function 0x007192f0`).

---

## Out-of-scope refs

- `AnimClass__Constructor` (`0x00421ea0`) — general animation infrastructure; not teleport-specific
- `VocClass__PlayAt` (`0x007509e0`) — general sound infrastructure; not teleport-specific
- `MapClass__Get_CellClass` (`0x005657a0`) — general map utility; not teleport-specific
- `CrateClass__PickupDispatch` (`0x00481a00`) — crate pickup side-effect; not teleport-specific
- `BulletClass__UpdateTarget` (`0x00468430`) — general targeting update; not teleport-specific

---

## Unverified / YELLOW

- **State 7 `param_1+0xc` write**: `*(param_1+0xc) = 0` clears `TeleportLocomotionClass+0x30`
  (Z dest cache). The plate comment says "clears WarpState(+0x218)" but the actual write is
  to `param_1+0xc` (locomotor field), not TechnoClass. TechnoClass WarpState clear at +0x280
  is a separate write confirmed above. The `+0x218` in the plate comment may refer to a different
  field; needs struct-decode clarification.

- **`TechnoClass+0x24` flag**: In state 5, `cVar2 = (char)((int*)param_1[2])[0x24]` gates the
  main path. `TechnoClass+0x24` is read as a char (direct byte offset 0x24 using int* indexing:
  the cast `(int*)param_1[2])[0x24]` means `*(int*)(param_1[2] + 0x24)` then cast to char — i.e.
  TechnoClass+0x90 direct byte). Exact flag name not verified; likely CanBeRecruited or similar.
  YELLOW.

- **Sound coords in state 2**: `VocClass__PlayAt` called with `piVar1[0x27..0x29]`. If param_1[2]
  is `int*`, then `piVar1[0x27] = *(piVar1 + 0x27) = TechnoClass+0x9c` (0x27×4=0x9c). Sound
  plays at departure cell. Consistent. Not separately verified.
