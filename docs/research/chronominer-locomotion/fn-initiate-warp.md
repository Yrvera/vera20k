# TeleportLocomotionClass__InitiateWarp — function decode

**Address:** `0x00719400`
**Kind:** function
**Proposed Ghidra label:** TeleportLocomotionClass__InitiateWarp (existing label is authoritative — plate comment update only)

---

## Summary

Computes the chrono warp delay from the 3D distance between source and destination,
arms the locomotor timer, plays departure sound and spawns the WarpOut anim at source,
performs the actual teleport (vtable dispatch), plays arrival sound, spawns a second
AnimClass at destination, calls CrateClass__PickupDispatch, clears TechnoClass+0x280.
Contains the chrono harvester instant-warp short-circuit: if unit type is 1 (infantry)
and TechnoType+0xe0e (Teleporter) is set, delay is forced to 0.

Verified via `decompile_function 0x00719400`.

---

## Active in YR

**Yes — live.** Called directly (inlined call site) from within
`TeleportLocomotionClass__StateMachineTick` state-0 branch when a valid destination
is cached and ChronoInTransit is set. No xrefs via normal CALL instruction; Ghidra's
`get_xrefs_to 0x00719400` returns empty, consistent with an inlined/direct-jmp call.
The vtable slot for this function is not separately exposed — it is an internal helper.
The logic path is confirmed live because StateMachineTick is vtable-live (verified in
`fn-state-machine-tick.md`).

---

## Decompile excerpt — full behavioral trace

`unaff_ESI` = `TeleportLocomotionClass*` (int*, same convention as param_1 elsewhere).
`unaff_EBP` = destination coord triple (leptons), passed in by caller.
`unaff_EBX` = comparison sentinel for conditional depart-anim.

### Step 1 — Conditional departure anim

```c
if (in_EAX != unaff_EBX) {
    // Spawn WarpOut anim at SOURCE location: TechnoClass+0x9c/+0xa0/+0xa4
    AnimClass__Constructor(g_RulesClass_Instance+0x33c, TechnoClass+0x9c);
}
```
Verified: `AnimClass__Constructor` called with `g_RulesClass_Instance+0x33c` (WarpOut
AnimType) and `&stack0x00000018` where stack locals `in_stack_0x18/1c/20` are loaded
from `unaff_ESI[2]+0x9c/+0xa0/+0xa4` (TechnoClass Location leptons, NW-cell frame).
This is the **departure-cell** anim. The `in_EAX != unaff_EBX` guard means the caller
controls whether the departure shimmer spawns; in the StateMachineTick inline clone,
the guard was always true for the normal path.

Source: `decompile_function 0x00719400`, lines 3–6.

### Step 2 — Distance computation

```c
piVar2 = GetCoords();  // vtable+0x48 on TechnoClass: returns geometric center in leptons
in_stack_14 = piVar2[0] - unaff_EBP[0];  // dX
in_stack_18 = piVar2[1] - unaff_EBP[1];  // dY
in_stack_1c = piVar2[2] - unaff_EBP[2];  // dZ
// Distance = sqrt(dX^2 + dY^2 + dZ^2)
Sqrt_Approx();         // result in FP stack
dist_int = Math__ftol();  // convert to int (in_stack_34)
```
Source: `decompile_function 0x00719400`, lines after `AnimClass__Constructor`.
Verified: `Sqrt_Approx @ 0x004cac40` and `Math__ftol @ 0x007c5f00` confirmed in
`get_function_callees 0x00719400`.

GetCoords is `vtable+0x48` on the TechnoClass (confirmed from StateMachineTick decode);
returns geometric foundation center in leptons. Destination (`unaff_EBP`) is also in
leptons (NW-cell frame). The distance is Euclidean 3D in leptons.

### Step 3 — Timer initialisation

```c
param_1[0xe] = g_CurrentFrameCounter;  // TeleportLocomotionClass+0x38 = timer start
param_1[0xf] = ...;                     // TeleportLocomotionClass+0x3c = reserved
param_1[0x10] = 0;                      // TeleportLocomotionClass+0x40 = initial ticks = 0
```

### Step 4 — ChronoTrigger distance-based delay

```c
if (*(char *)(g_RulesClass_Instance + 0xbf8) != '\0') {  // ChronoTrigger (Rules+0xbf8)
    iVar3 = *(int *)(g_RulesClass_Instance + 0xbf4);      // ChronoDistanceFactor (Rules+0xbf4)
    param_1[0x10] = dist_int / iVar3;                      // delay = distance / factor
}
```
Stock rulesmd.ini: `ChronoTrigger=yes`, `ChronoDistanceFactor=48`. Formula: `delay = dist_leptons / 48`.
Verified: `*(g_RulesClass_Instance + 0xbf8)` and `*(g_RulesClass_Instance + 0xbf4)` loads
confirmed in `decompile_function 0x00719400`.

### Step 5 — Elapsed-time adjustment

```c
iVar3 = param_1[0x10];  // computed delay ticks
if (param_1[0xe] != -1) {  // if timer was already running
    iVar6 = g_CurrentFrameCounter - param_1[0xe];
    iVar3 = (iVar6 < iVar3) ? (iVar3 - iVar6) : 0;  // subtract elapsed
}
```
Adjusts for frames already elapsed since timer started. Guards against negative result.

### Step 6 — ChronoMinimumDelay clamp

```c
if (iVar3 <= *(int *)(g_RulesClass_Instance + 0xbfc)) {  // ChronoMinimumDelay (Rules+0xbfc)
    // Copy the minimum-delay triple into param_1[0xe..0x10]
    in_stack_1c = *(int *)(g_RulesClass_Instance + 0xbfc);
    in_stack_14 = g_CurrentFrameCounter;
    piVar4 = &stack0x00000014;  // points to the clamp values
}
*piVar2 = *piVar4; param_1[0xf] = piVar4[1]; param_1[0x10] = piVar4[2];
```
If computed delay <= ChronoMinimumDelay, overwrite timer with minimum values.
Stock rulesmd.ini: `ChronoMinimumDelay=16`.

### Step 7 — ChronoRangeMinimum short-circuit

```c
if (dist_int < *(int *)(g_RulesClass_Instance + 0xc00)) {  // ChronoRangeMinimum (Rules+0xc00)
    iVar3 = *(int *)(g_RulesClass_Instance + 0xbfc);        // ChronoMinimumDelay
    param_1[0xe] = g_CurrentFrameCounter;
    param_1[0x10] = iVar3;                                  // force delay = MinimumDelay
}
```
Stock rulesmd.ini: `ChronoRangeMinimum=0` — this branch never fires in stock YR.
If non-zero, distances below this threshold (in leptons) use MinimumDelay regardless
of ChronoTrigger. Verified: `*(g_RulesClass_Instance + 0xc00)` load in decompile.

### Step 8 — Set WarpingOut flag

```c
*(TechnoClass+0x271) = 1;  // WarpingOut = true
```

### Step 9 — Chrono harvester instant-warp short-circuit

```c
iVar3 = (**(code **)(*(int *)unaff_ESI[2] + 0x2c))();  // GetTypeID vtable+0x2c
if ((iVar3 == 1) && (*(char *)(*(int *)(unaff_ESI[2] + 0x6c4) + 0xe0e) != '\0')) {
    // type==1 (infantry) AND TechnoType+0xe0e (Teleporter flag) set
    param_1[0xe] = g_CurrentFrameCounter;
    param_1[0x10] = 0;                     // force delay = 0 (instant warp)
    *(TechnoClass+0x271) = 0;              // clear WarpingOut (no animation delay)
}
```
`TechnoClass+0x6c4` = TechnoType ptr (int, direct byte offset).
`TechnoType+0xe0e` = Teleporter INI flag (bool).
**This is the chrono harvester instant-warp rule.** When Teleporter=yes is set on an
infantry unit, the warp delay collapses to 0 and the WarpingOut animation is skipped.
Verified: `*(int *)(unaff_ESI[2] + 0x6c4)` and `+0xe0e` in `decompile_function 0x00719400`.

### Step 10 — WarpAttach detach

```c
if (*(int *)(unaff_ESI[2] + 0x694) != 0) {
    WarpAttachClass__Detach();  // 0x0062a4a0
}
```
If a WarpAttachClass is linked to this unit (TechnoClass+0x694), detach it.

### Step 11 — Teleport: hide at source, compute bridge flag, unhide at dest

```c
vtable+0x124();  // Mark_Occupation_Bits (hide/remove from source)
TechnoType = (*vtable+0x84)();  // GetTechnoType
// check TechnoType+0x578 || Rules+0x21c for departure sound; play VocClass__PlayAt
vtable+0x1b4();  // UnMark_Occupation_Bits at source cell
MapClass__Get_CellClass(dest_cell);
// if dest cell has bridge overlay (CellClass+0x140 & 0x100): set TechnoClass+0x8c = 1
vtable+0x1b4();  // Mark_Occupation_Bits at dest cell
vtable+0x1cc();  // PlaceDown at dest
(*vtable+0x124)(1);  // Mark_Occupation_Bits with arrival param
TechnoType = (*vtable+0x84)();
// check TechnoType+0x574 || Rules+0x218 for arrival sound; play VocClass__PlayAt
vtable+0x18c(2);    // Set mission state
(*vtable+0x48)();   // Mark_All_Occupation_Bits
```
The hidden/unhide sequence is the actual teleport: source occupation cleared, dest
occupation set. The bridge check at dest (`CellClass+0x140 & 0x100`) updates
TechnoClass+0x8c (bridge flag). Verified from decompile.

Sound check: departure uses `TechnoType+0x578` (WarpOut sound) vs `Rules+0x21c`
(global WarpOut fallback). Arrival uses `TechnoType+0x574` vs `Rules+0x218`.

### Step 12 — Crate pickup, arrival anim, clear WarpState

```c
CrateClass__PickupDispatch(unaff_ESI[2]);  // collect any crate at dest
(*vtable+0x480)(0, 1);                     // additional placement
// Spawn second AnimClass at DESTINATION coords (TechnoClass+0x9c after teleport)
AnimClass__Constructor(g_RulesClass_Instance+0x33c, &stack0xfffffff0, 0, 1, 0x600, 0, 0);
*(TechnoClass+0x280) = 0;  // clear WarpState
return;
```
The second `AnimClass__Constructor` call uses the unit's current coords after teleport
(TechnoClass+0x9c now equals destination) — this is the **arrival-cell anim**.
**This contradicts memory `feedback_chrono_miner_no_arrival_shimmer`** which states
"WarpOut SHP at depart cell only, NOT at arrival."

Re-check: Both AnimClass calls use `g_RulesClass_Instance+0x33c` — but the args differ:
- First call (step 1): `AnimClass__Constructor(WarpOut_type, &local_18, ...)` with NO extra args
- Second call (step 12): `AnimClass__Constructor(WarpOut_type, &stack0xfffffff0, 0, 1, 0x600, 0, 0)` with 7 args

The second call with args `(type, coords, 0, 1, 0x600, 0, 0)` spawns a **looping/persistent**
anim (arg `0x600` is likely an Anim flags bitmask). The memory note's "no arrival shimmer"
likely refers specifically to the initial WarpOut flash/shimmer at departure — that matches.
The arrival-side anim is a separate persistent anim (arrival indicator, not the WarpOut SHP
flash). Both use Rules+0x33c but with different constructor arguments. YELLOW — needs
further anim-type verification to confirm both calls use the same AnimType or different ones.

---

## Struct fields accessed

**TeleportLocomotionClass** (param_1 = `unaff_ESI`, int*, index × 4):

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0x08` (param_1[2]) | 0x08 | TechnoClass ptr | Owner |
| `+0x38` (param_1[0xe]) | 0x38 | Timer start frame | Set = g_CurrentFrameCounter |
| `+0x3c` (param_1[0xf]) | 0x3c | Timer reserved | |
| `+0x40` (param_1[0x10]) | 0x40 | Timer delay ticks | Computed delay |

**TechnoClass** (piVar2 = `(int*)(unaff_ESI[2])`, int* pointer, index × 4):

| Field | Offset | Name | Role |
|---|---|---|---|
| `+0x270` | direct byte | ChronoInTransit gate | n/a in InitiateWarp (read in SMTick) |
| `+0x271` | direct byte | WarpingOut flag | Set step 8; cleared in harvester branch |
| `+0x280` | `piVar2[0xa0]`=0x280 | WarpState | Cleared end of function |
| `+0x574` | via TechnoType | WarpIn sound index | Arrival sound check |
| `+0x578` | via TechnoType | WarpOut sound index | Departure sound check |
| `+0x694` | direct | WarpAttachClass ptr | Detach check |
| `+0x6c4` | direct | TechnoType ptr | Harvester branch |
| `+0x8c` | direct byte | Bridge flag | Set from dest-cell overlay |
| `+0x9c/+0xa0/+0xa4` | direct | Location X/Y/Z leptons (NW-cell frame) | Source anim coords; dest coords after teleport |

---

## Globals / enums / INI keys

| Symbol | Address | INI key | Stock value | Role |
|---|---|---|---|---|
| `g_RulesClass_Instance` | inline | — | — | Rules singleton |
| `Rules+0xbec` | — | ChronoDelay | 60 | Post-warp delay (NOT read here; set in state 3) |
| `Rules+0xbf4` | — | ChronoDistanceFactor | 48 | Distance → delay divisor |
| `Rules+0xbf8` | — | ChronoTrigger | yes | Enable distance-based delay |
| `Rules+0xbfc` | — | ChronoMinimumDelay | 16 | Minimum warp ticks |
| `Rules+0xc00` | — | ChronoRangeMinimum | 0 | Range threshold for instant-min delay |
| `Rules+0x21c` | — | WarpOut global sound | — | Fallback departure sound |
| `Rules+0x218` | — | WarpIn global sound | — | Fallback arrival sound |
| `Rules+0x33c` | — | WarpOut anim type | — | AnimType for both depart+arrival anims |
| `TechnoType+0xe0e` | — | Teleporter | — | Instant-warp gate for chrono units |

Stock values verified via `ini/rulesmd.ini` lines 221–227 (grep confirmed).
Rules offset assignments verified via `decompile_function 0x00719400`.

---

## Out-of-scope refs

- `AnimClass__Constructor` (`0x00421ea0`) — general animation infrastructure; not teleport-specific
- `VocClass__PlayAt` (`0x007509e0`) — general sound infrastructure; not teleport-specific
- `MapClass__Get_CellClass` (`0x005657a0`) — general map utility; not teleport-specific
- `CrateClass__PickupDispatch` (`0x00481a00`) — crate pickup side-effect; not teleport-state-machine specific
- `Sqrt_Approx` (`0x004cac40`) — general math; not teleport-specific
- `Math__ftol` (`0x007c5f00`) — general math; not teleport-specific

---

## Unverified / YELLOW

- **Arrival anim vs. departure anim identity**: Both AnimClass calls use `Rules+0x33c` as AnimType.
  The second call passes extra constructor args `(0, 1, 0x600, 0, 0)`. Whether arg `0x600` selects
  a different internal frame/behaviour vs the first call (no extra args) needs AnimClass constructor
  decode to clarify. The memory note `feedback_chrono_miner_no_arrival_shimmer` says "WarpOut SHP
  at depart cell only, NOT at arrival" — this may refer to a different caller path (state 2 in
  StateMachineTick) rather than InitiateWarp itself. Both paths share the same AnimType pointer.
  YELLOW — defer to AnimClass decode or anim-type lookup.

- **`in_EAX != unaff_EBX` guard meaning**: The condition on the departure anim is opaque in the
  decompile (untyped register variables). Likely a caller-side flag indicating whether the unit
  was previously visible. Exact semantics unverified. YELLOW.

- **GetTypeID vtable+0x2c returns `1` for infantry**: Assumed from context (infantry type ID = 1).
  Not independently verified against TechnoTypeClass enum. YELLOW.
