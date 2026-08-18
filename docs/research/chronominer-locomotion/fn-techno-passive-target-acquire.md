# TechnoClass__Passive_Target_Acquire — 0x00709480

**Proposed Ghidra label:** TechnoClass__Passive_Target_Acquire (existing label authoritative — plate comment only needed)
**Active in YR:** Yes — called exclusively from TeleportLocomotionClass__TimerCheck @ 0x00719BF0 (UNCONDITIONAL_CALL confirmed via `get_xrefs_to 0x00709480`)

---

## Summary

Passive auto-target scan called from TimerCheck after warp-delay expiry when
`TechnoClass+0x2B4 == 0` (no active targeting). Delegates the "should we try to
acquire?" gate to `FUN_00709290`, then if allowed: records the current frame, calls
`GetCoords` (vtable+0x48) to build a coord triple, invokes `vtable+0x39C` (the actual
target-scan dispatcher), and if a new target was acquired (`TechnoClass+0x2B4` changed)
sets `TechnoClass+0x50C` (firing flag byte) to 1. Returns non-null if a target was
found, null/'\0' if not.

Verified via `decompile_function 0x00709480` and `get_xrefs_to 0x00709480`
(single caller: TimerCheck @ 0x00719BF0 at 0x00719C31, UNCONDITIONAL_CALL).

---

## Caller chain (Active in YR: Yes)

```
TeleportLocomotionClass__TimerCheck (0x00719BF0)   [states 1 and 6, on timer expiry]
  └─ TechnoClass__Passive_Target_Acquire (0x00709480)   [called when TechnoClass+0x2B4 == 0]
       └─ FUN_00709290 (0x00709290)   [passive-targeting gate]
```

`get_function_callers 0x00709480` → only TeleportLocomotionClass__TimerCheck.
`get_xrefs_to 0x00709480` → single UNCONDITIONAL_CALL from 0x00719C31 (inside TimerCheck body).

Note: `FUN_00709290` is also called from `TechnoClass__AI_Update @ 0x006F9E50`, so it is
general targeting infrastructure. `Passive_Target_Acquire` itself is teleport-specific
(single caller).

---

## Signature

```c
uint __fastcall TechnoClass__Passive_Target_Acquire(int *param_1)
// param_1 = TechnoClass* (int*: index N × 4 = byte offset)
// returns: non-zero if target acquired, 0 (no target) / gate blocked
```

---

## Decompilation

Source: `decompile_function 0x00709480`

```c
uint TechnoClass__Passive_Target_Acquire(int *param_1)
{
    int iVar1;            // saved old target value at param_1[0xad] = +0x2B4
    int iVar2;            // vtable ptr
    uint uVar3;           // gate result / return
    undefined4 uVar4;
    undefined1 local_c[12];  // coord buffer on stack (12 bytes = XYZ)

    uVar3 = FUN_00709290();   // passive-targeting gate check
    if ((char)uVar3 == '\0') {
        uVar3 = uVar3 & 0xffffff00;  // gate blocked → return 0
    } else {
        iVar1 = param_1[0xad];                     // save TechnoClass+0x2B4 (current target)
        iVar2 = *param_1;                           // vtable ptr
        param_1[0x13f] = g_CurrentFrameCounter;    // TechnoClass+0x4FC = last scan frame
        uVar4 = (**(code **)(iVar2 + 0x48))(local_c, 1);  // vtable+0x48 = GetCoords
        uVar3 = (**(code **)(iVar2 + 0x39c))(uVar4);       // vtable+0x39C = target scan
        if (((char)uVar3 != '\0') && (param_1[0xad] != iVar1)) {
            // scan succeeded AND target changed → fire
            *(undefined1 *)(param_1 + 0x143) = 1;  // TechnoClass+0x50C = firing_flag
            return uVar3;
        }
    }
    return uVar3;
}
```

---

## Behavioral analysis

### Gate: FUN_00709290 (0x00709290)

Called as the first check. If it returns '\0' (false), the function exits immediately
with 0 — no scan, no target. From `decompile_function 0x00709290`, the gate suppresses
passive targeting when:
- Unit already has a target (`param_1[0xAD] != 0`)
- House is player-controlled (`HouseClass__IsPlayerControl()` returns true — AI-only)
- Unit is null or the targeting-permission bits are not set
- Mission != 2 (GUARD) in many sub-branches
- Other type-specific conditions (aircraft, buildings, transporters)

In the teleport context (TimerCheck), the gate is called after warp-delay expiry.
The unit has just re-materialized and its mission is typically not GUARD, so the gate
likely returns true only under specific conditions — or TimerCheck's outer guard
`TechnoClass+0x2B4 == 0` already filters for the common case.

### Frame timestamp

`param_1[0x13F] = g_CurrentFrameCounter` writes the current game tick to
TechnoClass+0x4FC. Records when the passive scan last ran, likely to throttle
repeated scans per game tick or to avoid scanning on the same frame twice.

### Coord buffer and target scan

`vtable+0x48(local_c, 1)` — GetCoords fills a 12-byte stack buffer with the unit's
current coordinates (XYZ leptons). The coord is passed to `vtable+0x39C` as the
scan origin.

`vtable+0x39C(coord_buf)` — the target scan dispatcher. Searches for nearby hostile
units; if found, assigns the target to `param_1[0xAD]` (TechnoClass+0x2B4) and
returns non-null.

### New-target detection and firing flag

After the scan, compares `param_1[0xAD]` (current) vs `iVar1` (saved value before scan).
If both the scan succeeded AND the target changed (new target assigned):
- Sets `*(undefined1*)(param_1+0x143) = 1` — TechnoClass+0x50C firing flag byte.
- Returns the scan result (non-null).

This fires the weapon at the newly acquired target.

### Effect on TimerCheck flow

From fn-timer-check.md:
```c
cVar1 = TechnoClass__Passive_Target_Acquire();
if (cVar1 == '\0') {
    // No target: resume locomotion
    (**(code **)(...+0x484))(0, 1);  // vtable+0x484 = Resume
}
```
If `Passive_Target_Acquire` returns non-null → unit stays locked, fires at target.
If it returns '\0' → Resume called, locomotion restarts after warp.

---

## Struct fields accessed

`param_1` is `int*` (byte offset = index × 4):

| Byte offset | Index | Field name | Role |
|---|---|---|---|
| TechnoClass+0x2B4 | `[0xAD]` | targeting_active | Current target ptr; saved before scan, compared after |
| TechnoClass+0x4FC | `[0x13F]` | last_passive_scan_frame | Written with g_CurrentFrameCounter |
| TechnoClass+0x50C | `[0x143]` byte | firing_flag | Set to 1 when new target acquired |

---

## Vtable slots

| Slot offset | Called on | Meaning | Confidence |
|---|---|---|---|
| vtable+0x48 | TechnoClass | GetCoords (fills XYZ coord buffer) | HIGH — consistent with GetCoords usage across codebase |
| vtable+0x39C | TechnoClass | Target scan dispatcher | MEDIUM — inferred from context; method name not decompiled here |

---

## Globals

| Symbol | Role |
|---|---|
| `g_CurrentFrameCounter` | Written to TechnoClass+0x4FC to timestamp the scan |

---

## Out-of-scope refs

| Symbol | Address | Reason |
|---|---|---|
| `FUN_00709290` | 0x00709290 | Passive-targeting gate; called from TechnoClass__AI_Update too; general targeting infrastructure, not teleport-specific |
| `FUN_007091D0` | 0x007091D0 | Inner gate sub-called from FUN_00709290; general targeting infrastructure |
| `HouseClass__IsPlayerControl` | — | General house utility; not teleport-specific |

---

## Unverified / YELLOW

- **TechnoClass+0x4FC exact name** (`param_1[0x13F]`): Likely `LastPassiveScanFrame` or similar throttle stamp. Not verified against TechnoClass struct decode. YELLOW.
- **TechnoClass+0x50C exact flag name** (`*(undefined1*)(param_1+0x143)` — note: `param_1+0x143` when param_1 is `int*` is NOT ×4 — this is a cast to `undefined1*` after pointer arithmetic, so the byte offset is `0x143 × 4 = 0x50C`). Named `firing_flag`. Not verified against TechnoClass struct decode. YELLOW.
- **vtable+0x39C target scan identity**: "Target scan dispatcher" inferred from context. Not decompiled. YELLOW.
- **FUN_00709290 gate — full YR conditions**: Gate decompiled but complex with many branches. The summary above covers the primary paths. Edge cases (aircraft, building turrets, specific mission states) not traced. YELLOW.
