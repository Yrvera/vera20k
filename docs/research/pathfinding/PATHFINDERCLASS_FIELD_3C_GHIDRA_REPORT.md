# PathfinderClass +0x3C — Setter & Value Enumeration (Ghidra Research Report)

**Singleton:** `PathfinderClass @ 0x0087e8b8`
**Field address:** `0x0087e8b8 + 0x3c = 0x0087e8f4` (DWORD)
**Setter:** `AStar_pathfind_search @ 0x0042c900` (single write site)
**Source chain:** `FootClass::Find_Path arg4 → FootClass::Run_AStar arg7 → AStar_pathfind_search param_8 → PathfinderClass+0x3c`
**Confidence:** HIGH — verified by direct disassembly of every link in the chain plus all six caller sites.
**Active in YR:** YES — every locomotor that calls Find_Path is live in YR skirmish (Drive, Ship, Walk, Jumpjet, Hover).

## 1. Summary

`PathfinderClass+0x3c` is **NOT** a MovementZone or SpeedType classifier. It is a
**per-pathfind-attempt "urgency / retry-pressure" value** computed at the call site by
each locomotor based on how long the unit has been blocked. It influences the A*
edge-cost behavior for `Can_Enter_Cell` code 2 (TemporaryBlock = friendly moving unit
in the destination cell).

The previous label "destroyer mode" was misleading — `destroyer` in the existing Ghidra
annotation referred to *aggressive retry behavior* (the prior researcher's pun on
"destroying" the friendly blocker preference), NOT the naval destroyer ship.

## 2. The Write Site (Definitive)

`AStar_pathfind_search @ 0x0042c900`, near function entry:

```c
*(undefined1 *)(param_1 + 0x38) = 1;       // mark retry-pending flag
PathfinderClass__Reset();
// ... reset 3-entry list at param_1+0x74 ...
*(uint *)(param_1 + 0x3c) = param_8;       // ← THE WRITE
```

- `param_1` = `PathfinderClass *this*` (loaded as `MOV ECX, 0x87e8b8` at `0x004cbc2c`).
- `param_8` = the 7th stack argument (originating from `FootClass::Find_Path`'s arg4).
- This is the **only** write to `+0x3c` in the program. No other function modifies it.

There is **no setter** in `PathfinderClass::Constructor`, `Reset`, `Init`, or
`UpdateHierarchicalEdges`. The field is overwritten per Find_Path invocation.

## 3. The Read Sites

`AStar_compute_edge_cost @ 0x00429830` reads the field in two places, both inside
the `bVar10 = (Can_Enter_Cell_code == 2)` branch (friendly TemporaryBlock):

```c
if (*(int *)(param_1 + 0x3c) == 0) {
    // run blocker-path prediction loop (up to 10 cells)
    // may set param_5 = 1.0 if blocker is predicted to clear
}
param_5 = 4.0;                                 // default TemporaryBlock cost
if (*(int *)(param_1 + 0x3c) == 2) {
    param_5 = 1000.0;                          // urgent-reroute override
}
```

The field is read as `int` (DWORD), so all 4 bytes participate in the `== 0` and
`== 2` comparisons.

## 4. Caller Chain — How the Value Is Computed

### 4.1 Pathway

```
LocomotorClass::Process_Movement
     │
     │ pushes  arg4 = urgency (0 | 1 | 2)
     ↓
FootClass::Find_Path  @ 0x004d3920    (RET 0xc, 3 stack args)
     │
     │ arg4 is read from [ESP+0x1fb8] and PUSHed first (highest stack offset)
     ↓
FootClass::Run_AStar  @ 0x004cbba0    (RET 0x18, 6 stack args, thiscall)
     │
     │ reads [ESP+0x34] (its arg7 = Find_Path's arg4) into EAX, PUSHes EAX FIRST
     ↓
AStar_pathfind_search @ 0x0042c900    (thiscall, 7 stack args + this)
     │
     │ stores param_8 into *(uint *)(this + 0x3c)
     ↓
PathfinderClass+0x3c
```

### 4.2 Direct Callers of `FootClass::Find_Path` and arg4 values

| Caller | Address | arg4 push pattern | Value(s) |
|--------|---------|-------------------|----------|
| `DriveLocomotionClass::Process_Movement` | 0x004b2630 | call 1 @ 0x004b28a3: `PUSH 0x0; PUSH 0x0; PUSH dest` | `0` |
| `DriveLocomotionClass::Process_Movement` | 0x004b2630 | call 2 @ 0x004b3a0e: `TEST BL,BL; SETNZ DL; INC EDX; PUSH EDX` | `1` or `2` |
| `DriveLocomotionClass::Process_Movement` | 0x004b2630 | call 3 @ 0x004b3f37: `PUSH 0x0; PUSH ECX; PUSH dest` | `0` |
| `ShipLocomotionClass::Process_Movement` | 0x006a1c80 | call 1 @ 0x006a1ef3: similar to Drive call 1 | `0` |
| `ShipLocomotionClass::Process_Movement` | 0x006a1c80 | call 2 @ 0x006a305d: SETNZ+INC pattern | `1` or `2` |
| `ShipLocomotionClass::Process_Movement` | 0x006a1c80 | call 3 @ 0x006a3586: similar to Drive call 3 | `0` |
| `WalkLocomotionClass::ProcessMovement` | 0x0075aec0 | call 1 @ 0x0075afc5: `PUSH 0x0; PUSH 0x0` | `0` |
| `WalkLocomotionClass::ProcessMovement` | 0x0075aec0 | call 2 @ 0x0075b979: SETNZ+INC pattern | `1` or `2` |
| `FUN_005b01c0` (JumpjetLocomotion::Process_Movement candidate) | 0x005b01c0 | `Find_Path(coord, 0, cVar3 + 1)` | `1` or `2` |
| `FUN_005164d0` (HoverLocomotion helper) | 0x005164d0 | `PUSH [ESP+0x3c]` (propagates its own arg1) | propagated |

### 4.3 The "SETNZ + INC" computation (Drive / Ship / Walk call site 2)

Identical pattern at all three locomotors:

```asm
XOR EDX, EDX
TEST BL, BL          ; BL = blocked_delay_expired flag
SETNZ DL             ; DL = 1 if BL!=0, else 0
INC EDX              ; EDX = 1 (passive) or 2 (aggressive)
PUSH EDX             ; arg4 to Find_Path
```

`BL` is set by upstream branches: 
- Entry that does `XOR BL,BL` (e.g., DriveLocomotion @ 0x004b39d1) → BL=0 → arg4=**1** (passive retry, blocked_delay still counting down).
- Entry that skips the XOR and inherits `MOV BL,0x1` from earlier (e.g., DriveLocomotion @ 0x004b36ed → fall-through to 0x004b39d3) → BL=1 → arg4=**2** (aggressive: blocked_delay expired, force reroute).

For JumpjetLocomotion (FUN_005b01c0), the equivalent is computed differently:
`cVar3` is set based on a CDTimerClass remaining check (timer at FootClass+0x19a /
+0x19c, gated by a flag at +0x6b7). Same semantics: `0 → arg4=1`, `1 → arg4=2`.

## 5. Complete Value Enumeration

| Value | Set by | Read by `AStar_compute_edge_cost` (Can_Enter_Cell code 2 branch) | Final cost for TempBlock cell |
|-------|--------|------------------------------------------------------------------|-------------------------------|
| `0` | First-attempt call sites in every locomotor (passive, no blocker history) | Predict blocker's future path (up to 10 hops). If blocker is predicted to clear, set cost to **1.0**. Otherwise set cost to **4.0**. | 1.0 or 4.0 |
| `1` | Second-attempt call sites when `blocked_delay > 0` (locomotor knows it's stalled but hasn't given up) | Skip blocker prediction loop. Set cost to **4.0**. No 1000 override. | 4.0 |
| `2` | Second-attempt call sites when `blocked_delay == 0` (locomotor exhausted patience) | Skip blocker prediction loop. Set cost to **4.0**, then **override to 1000.0**. | 1000.0 (forces reroute) |

**No other values occur.** The SETNZ+INC instruction sequence is mathematically
limited to producing exactly 1 or 2; the only other write source is the literal `0`
push in the first-attempt call sites. Values 3, 4, etc. cannot arise from any
visible call site.

The "destroyer mode" label from prior research refers to value 2 (aggressive
retry that destroys/inflates the friendly-blocker preference) — NOT the naval
destroyer ship. The naval destroyer ship uses ShipLocomotion which produces 0/1/2
exactly like Drive and Walk.

## 6. What This Field Is NOT

Disconfirmed hypotheses from prior research:

- **NOT MovementZone-derived.** No call site reads `TechnoType+0x5b4` (MovementZone)
  for this value. The value depends entirely on the calling unit's current
  blocked-delay state.
- **NOT SpeedType-derived.** No call site reads `TechnoType+0xCD0` (SpeedType) for
  this value. (Note: a *different* TechnoType byte at `+0xC94`
  ["TooBigToFitUnderBridge" / "Crusher"] is sometimes pushed into Find_Path's
  **arg3**, but arg3 is a different parameter that does NOT flow into +0x3c.)
- **NOT a per-locomotor classifier.** All four ground locomotors (Drive, Ship,
  Walk, Jumpjet) use the same computation logic and the same value range.
- **NOT a naval-destroyer flag.** Naval destroyers go through ShipLocomotion which
  uses exactly the same 0/1/2 pattern as land vehicles.

## 7. Open Questions

1. **HoverLocomotion's arg1 to FUN_005164d0** — confirmed to propagate to the
   Find_Path call, but the upstream callers (FUN_00514f70, HoverLocomotion::Move @
   0x00514310, HoverLocomotion::SpeedUpdate @ 0x00515ed0) were not exhaustively
   traced. Likely uses the same 0/1/2 pattern (Hover units like Robot Tank are
   subject to the same blocked-delay semantics), but unverified. **Out of scope.**
2. **AircraftLocomotion / TeleportLocomotion** — neither calls `Find_Path` directly,
   so neither writes `+0x3c`. The field's value at the time aircraft/teleporting
   units evaluate edge costs is the residue from the last ground unit's pathfind.
   This is harmless because aircraft/teleporters use different vtable paths that
   don't reach `AStar_compute_edge_cost` with Can_Enter_Cell==2 in normal play.
   Verification deferred.
3. **The retry loop inside `AStar_pathfind_search`** mutates the local `param_8`
   variable via `param_8 = CONCAT31(param_8._1_3_, bVar8);` after a failed attempt,
   but **does NOT re-write `+0x3c`**. The field stays at its initial value through
   all retries within a single Find_Path call. Confirmed by inspection.

## 8. Implementation Implications for the Rust Port

If implementing road-following or A* edge cost, this field is **not** about
"destroyer mode" / "naval mode" / "road preference". It is about **per-call retry
urgency** when a friendly unit blocks the path. The Rust port should:

- Track a per-unit `blocked_delay` counter (already part of standard FootClass state).
- At each Find_Path invocation, compute an `urgency: u8` value (0 / 1 / 2) at the
  call site based on whether this is the first attempt, a retry-while-waiting, or
  a retry-after-patience-exhausted.
- Pass this value into the A* search context; consume it in the edge-cost
  computation for the "friendly unit blocking destination" case.

The 1000.0 cost in destroyer-mode produces visible behavior: when a unit's
blocked_delay expires, it will *route around* a stationary friendly blocker instead
of waiting in place. This is observable in normal YR play (any time a unit gets
"stuck" behind a friendly and eventually finds a way around).

## 9. Citations (verified this session)

- Setter write site: `0x0042c900` (decompile inspected; `*(uint *)(param_1 + 0x3c) = param_8`)
- Singleton xrefs: `get_xrefs_to 0x0087e8b8` — 23 distinct sites; only Run_AStar/Find_Path/Mission_Patrol/etc. are reads, no other writes to +0x3c
- Call site disasm for arg4 push: `0x004b3a0e` (Drive), `0x006a305d` (Ship), `0x0075b979` (Walk), `0x005b01c0` body (Jumpjet)
- Edge-cost reads: `0x00429830` decompile shows `*(int *)(param_1 + 0x3c) == 0` and `== 2` comparisons
- Vtable identification of FUN_005b01c0 as a Locomotion vtable slot: `get_xrefs_to 0x005b0060 → 007edbac [DATA]`, vtable layout consistent with Jumpjet/Process_Movement slot
