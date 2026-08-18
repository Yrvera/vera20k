# FootClass__Set_Destination_Internal — Decode Doc

**Proposed Ghidra label:** `FootClass__Set_Destination_Internal`

## Summary

`FootClass__Set_Destination_Internal` (`0x004D94B0`) is the internal NavCom commit step
called at the end of `TechnoClass__Set_Destination` (`0x00741970`). It writes the new
destination target to `FootClass+0x5A4` (`param_1[0x169]`) and, if the destination is
non-null, calls `ILocomotion::Head_To_Coord` (vtable `+0x44`) on the active locomotor
(`FootClass+0x674`, `param_1[0x19d]`) to arm movement toward the target's coordinates.

For chrono miners with a teleport locomotor, `Head_To_Coord` arms the warp state machine.
For units that have swapped to a Drive locomotor (after piggyback), `Head_To_Coord` starts
normal drive navigation. The function also handles the `ChronoWarp` deployment cancel path
(`BuildingClass__DeployUnit_ChronoWarp`) when a pending ChronoWarp building is being
cleared by a new destination.

## Active in YR

**Yes — active in standard YR skirmish.** Verified by:
- `get_function_callers 0x004D94B0` returned `TechnoClass__Set_Destination @ 0x00741970`,
  `UnitClass__EnterBuildingOrDock @ 0x0041AA80` (3 call sites), and `FUN_0051aa40`.
  `TechnoClass__Set_Destination` is the standard vtable `+0x480` SetDestination entry
  point called by all unit movement code.
- `get_xrefs_to 0x004D94B0` confirmed DATA ref at `0x007e9114` (vtable slot; vtable
  base read via `read_memory 0x007e9108` shows `a0944d00` = LE `0x004D94A0` context and
  `b0944d00` = `0x004D94B0` at +0xC in that region).
- No TS-only gating detected.

## Decompilation Excerpt

```c
// from decompile_function 0x004D94B0 (Ghidra existing plate comment confirmed)

void __thiscall FootClass__Set_Destination_Internal(int *param_1, int param_2)
{
    // param_1 is int* — byte offset = N × 4

    param_1[0x168] = 0;  // FootClass+0x5A0: clear pending-move flag

    // Guard rails: if unit is in certain states, ignore non-null destination
    if ((*(char *)((int)param_1 + 0x6ad) != '\0') && param_2 != 0) return;  // in-deploy
    if ((*(char *)((int)param_1 + 0x82)  != '\0') && param_2 != 0) return;  // burrowed/underground
    if ((param_1[0xb9] != 0)             && param_2 != 0) return;  // in-building
    if ((param_1[0xab] != 0)             && param_2 != 0) {
        // Has a pending ChronoWarp building deployed — cancel it
        BuildingClass__DeployUnit_ChronoWarp(1);  // 0x0070FEE0
    }

    // CORE: write new NavCom destination
    param_1[0x169] = param_2;  // FootClass+0x5A4 = NavCom target ptr

    // If clearing destination (param_2 == 0) and was in deploy state:
    if ((param_2 == 0) && (*(char *)((int)param_1 + 0x6ad) != '\0') && (param_1[0xac] != 0)) {
        *(int *)(param_1[0xac] + 0x2ac) = 0;  // clear dock link
        param_1[0xac] = 0;
        *(char *)((int)param_1 + 0x6ae) = 1;  // set deploy-exit flag
    }

    if (param_1[0x169] == 0) {
        // NULL destination: stop via ILocomotion::Stop (vtable +0x48)
        // BUT: if GetMission()==2 (Harvest) AND (substate==1 OR substate_alt==1) AND NavCom!=0:
        //   skip the Stop call
        iVar4 = (*vtable + 0x2c)();  // GetMission
        if (iVar4 != 2 || (param_1[0x2b] != 1 && param_1[0x2d] != 1) || param_1[0xad] == 0) {
            (*loco_vtable + 0x48)(param_1[0x19d]);  // ILocomotion::Stop
            param_1[0x169] = param_2;  // re-clear (redundant after Stop)
        }
        goto LAB_epilog;
    }

    // Non-null destination path:
    // Cancel any existing waypoint path
    if ((int *)param_1[0xc1] != NULL) {
        (*param_1[0xc1]_vtable + 0xf8)();  // cancel path
        param_1[0xc1] = 0;
    }

    // Query active locomotor for piggyback interface
    LocomotionClass__QueryInterface_IPiggyback(param_1 + 0x19d);  // 0x0045AEA0

    // Check if locomotor CLSID == CLSID_WalkLocomotion
    // (4-DWORD GUID compare)
    if (CLSID_matches_WalkLocomotion) {
        // Walk loco: manage walk delay timer
        iVar4 = param_1[0x192];  // walk delay remaining
        if (param_1[400] == -1) {
            // no start time set
        } else {
            iVar6 = g_CurrentFrameCounter - param_1[400];
            if (iVar6 < iVar4) { iVar4 -= iVar6; /* time remaining */ }
            else { /* expired: restart */ }
        }
        param_1[400] = g_CurrentFrameCounter;
        param_1[0x191] = unaff_EBX;
        param_1[0x192] = 0;  // reset walk delay
    }

    // Head_To_Coord: arm locomotor toward destination
    if ((char)param_1[0x1ab] == '\0') {
        // Get destination coordinates
        puVar5 = (*destination_vtable + 0x4c)(&stack, param_1);  // GetCoords of dest
        (*loco_vtable + 0x44)(param_1[0x19d], uVar1, uVar2, uVar3);  // Head_To_Coord
    } else {
        *(char *)(param_1 + 0x1ab) = 0;  // clear "already armed" flag
    }

    if (piStack_4 != NULL) (*piStack_4_vtable + 8)();  // release piggybacker ref

LAB_epilog:
    *(char *)((int)param_1 + 0x6b7) = 0;  // clear move-override flag
    iVar4 = *(int *)(g_RulesClass_Instance + 0x1768);  // Rules+0x1768: locomotor delay
    param_1[0x19a] = g_CurrentFrameCounter;
    param_1[0x19b] = unaff_EBX;
    param_1[0x19c] = iVar4;     // set move timer duration
    param_1[400] = g_CurrentFrameCounter;
    param_1[0x191] = unaff_EBX;
    param_1[0x192] = 0;
    return;
}
```

## Behavioral Analysis

### Observable effect in harvest loop

This function is the final step that arms the miner's locomotor after a new destination
is committed. In the chrono miner harvest cycle:

1. `UnitClass__Mission_Harvest` state 2 calls `TechnoClass__Set_Destination` (vtable `+0x480`)
   with the chosen refinery approach cell.
2. `TechnoClass__Set_Destination` performs teleport-vs-drive decision (chrono miner CLSID
   check, QueueingCell routing) and then calls `FootClass__Set_Destination_Internal`.
3. `FootClass__Set_Destination_Internal` writes the resolved destination to `NavCom`
   (`FootClass+0x5A4`) and calls `Head_To_Coord` on the active locomotor.
4. For a chrono miner with teleport loco active: `Head_To_Coord` arms the warp to the
   refinery approach cell.
5. For a chrono miner that has swapped to drive loco (normal approach): `Head_To_Coord`
   commands the drive locomotor to navigate normally.

### Guard rails (early returns)

Three state flags cause the function to ignore a non-null destination:
- `FootClass+0x6AD` (`param_1+0x6AD`): unit is deploying in-place (e.g., IFV deploy,
  Siege Chopper). Cannot change destination while mid-deploy.
- `FootClass+0x82×4 = +0x208` (byte at `+0x82*4` interpreted as byte; actually:
  `*(char *)((int)param_1 + 0x82)` — direct byte offset `+0x82`, not `+0x208`):
  unit is burrowed/underground.
- `FootClass+0x2E4` (`param_1[0xb9]` = offset `0xB9 × 4 = 0x2E4`): unit is inside a
  building/transport.

### ChronoWarp cancel path

If `param_1[0xab]` (offset `0xAB × 4 = 0x2AC`) is non-null when a new destination is
set, the pending ChronoWarp building is cancelled via `BuildingClass__DeployUnit_ChronoWarp`
(`0x0070FEE0`). That function clears the building's deploy state and fires `SetDestination(0,1)`
on the chrono unit to release it. This is the mechanism that prevents a re-directed
chrono miner from completing a mid-flight warp to a stale refinery.

### Harvest-mission protection for null destination

The `GetMission() == 2 (Harvest) AND substate condition` check prevents the locomotor from
receiving a `Stop()` call when the harvest mission clears a queued destination internally.
If mission is Harvest (2) and substate `param_1[0x2b]` or `param_1[0x2d]` is 1, the Stop
is skipped. This protects against spurious locomotor stops during mid-harvest routing.

### Epilog timer reset

At the end (both paths), `Rules+0x1768` is read and stored in `param_1[0x19c]`
(`FootClass+0x670`). This resets the per-unit locomotion delay timer used to pace how
frequently the locomotor can receive new waypoints. Field `Rules+0x1768` is not confirmed
as a named INI key in the sessions searched; see YELLOW section.

## Struct Field Accesses

`param_1` is `int *` — byte offset = N × 4.

| Field | Byte offset | Meaning |
|-------|-------------|---------|
| `param_1[0x168]` | `+0x5A0` | Pending-move clear flag |
| `param_1[0x169]` | `+0x5A4` | **NavCom** destination target ptr (core write) |
| `param_1[0x19d]` | `+0x674` | Active locomotion interface ptr (ILocomotion) |
| `param_1[0xab]`  | `+0x2AC` | Pending ChronoWarp building ptr (if non-null) |
| `param_1[0xac]`  | `+0x2B0` | Dock link object ptr (cleared if deploy-state exit) |
| `param_1[0xb9]`  | `+0x2E4` | In-building/transport flag |
| `param_1[0xc1]`  | `+0x304` | Active waypoint path ptr (cancelled on new dest) |
| `param_1[0x2b]`  | `+0xAC`  | Mission substate 1 (Harvest protection flag) |
| `param_1[0x2d]`  | `+0xB4`  | Mission substate alt (Harvest protection flag) |
| `param_1[0xad]`  | `+0x2B4` | NavCom non-zero check (Harvest protection) |
| `param_1[0x191]` | `+0x644` | Locomotion timer aux value |
| `param_1[0x192]` | `+0x648` | Walk delay remaining |
| `param_1[0x19a]` | `+0x668` | Move timer start frame |
| `param_1[0x19b]` | `+0x66C` | Move timer aux |
| `param_1[0x19c]` | `+0x670` | Move timer duration (set from Rules+0x1768) |
| `param_1[0x1ab]` | `+0x6AC` | "Already armed" flag — skips Head_To_Coord |
| `param_1[400]`   | `+0x640` | Walk timer start frame (400 = 0x190) |
| byte `+0x82`     | direct   | Burrowed/underground byte flag |
| byte `+0x6AD`    | direct   | In-deploy byte flag |
| byte `+0x6AE`    | direct   | Deploy-exit byte flag (set when deploy link cleared) |
| byte `+0x6B7`    | direct   | Move-override byte flag (cleared in epilog) |

## Globals Referenced

| Global | Address | Meaning | Verified |
|--------|---------|---------|---------|
| `g_RulesClass_Instance` | (symbol) | Singleton RulesClass ptr | via decompile_function 0x004D94B0 |
| `Rules+0x1768` | via `g_RulesClass_Instance` | Locomotion delay (frames); exact INI key not confirmed | via decompile 0x004D94B0 |
| `g_CurrentFrameCounter` | (symbol) | Current game frame | via decompile 0x004D94B0 |
| `CLSID_WalkLocomotion` | (symbol) | 16-byte GUID for walk locomotor | via decompile 0x004D94B0 |

## Callees

| Function | Address | Description |
|----------|---------|-------------|
| `BuildingClass__DeployUnit_ChronoWarp` | `0x0070FEE0` | Cancel pending ChronoWarp building deployment (verified via get_function_by_address 0x0070FEE0) |
| `LocomotionClass__QueryInterface_IPiggyback` | `0x0045AEA0` | Query active loco for piggyback interface (verified via get_function_by_address 0x0045AEA0) |
| `GameDebugLog__Assert` | `0x007DC720` | Debug assertion |
| `ILocomotion::Stop` | via vtable `+0x48` | Stop locomotion (called on null dest) |
| `ILocomotion::Head_To_Coord` | via vtable `+0x44` | Arm locomotor with destination coords |

## Out-of-Scope Refs

- `TechnoClass__Set_Destination` (`0x00741970`) — caller (task #2, decode-fn-set-destination)
- `UnitClass__EnterBuildingOrDock` (`0x0041AA80`) — caller (task #49)
- `BuildingClass__DeployUnit_ChronoWarp` (`0x0070FEE0`) — callee; task #53
- `LocomotionClass__QueryInterface_IPiggyback` (`0x0045AEA0`) — locomotion scope
- `CLSID_WalkLocomotion` — locomotion CLSID; locomotion scope
- `CLSID_DriveLocomotion` / `CLSID_TeleportLocomotion` — referenced in TechnoClass__Set_Destination

## Unverified Claims (YELLOW)

- `Rules+0x1768` exact INI key name: searched ReadGeneral output for `0x1768` with no hit.
  The field is read and stored in `FootClass+0x670` (move timer duration). Name unknown;
  likely a locomotion-pacing interval key.
- `byte +0x82`: The decompile shows `*(char *)((int)param_1 + 0x82)` — this is a direct
  byte offset `+0x82`, not `param_1[0x82]×4`. The meaning as "burrowed/underground" is
  inferred from usage (blocks new destination when non-zero).
- `param_1[0x2b]` / `param_1[0x2d]` for Harvest protection: these are byte offsets
  `+0xAC` / `+0xB4`. Their exact meaning as sub-state counters is inferred from the
  `GetMission()==2` context. They may be the `param_1[0x2f]` substate byte variants.
- The walk-delay path (CLSID_WalkLocomotion branch) was traced but the exact timer
  semantics of `param_1[0x192]` and `param_1[400]` are inferred, not traced to INI.
