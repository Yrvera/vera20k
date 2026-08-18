# g_BridgeZOffset_Teleport — 0x00b0ec2c

**Proposed Ghidra label:** `g_BridgeZOffset_Teleport`

## Summary

A single 4-byte global holding the Z-axis offset (in leptons) added to a unit's destination Z-coordinate when the teleport destination cell has a bridge overlay (`CellClass+0x140 & 0x100`). Read once in `TeleportLocomotionClass__Update_Position` (mode B: validate/adjust path) when the bridge overlay bit is set and the unit is not already on a bridge. Written to zero by the same static initializer block that writes `g_NullCoord_Teleport_X/Y/Z`.

Verified via `read_memory 0x00b0ec2c` (4 bytes): value = `0x00000000` at runtime. The bridge Z path in `Update_Position` executes when conditions are met, but adds **zero** to the destination Z in stock YR — the offset is architecturally present but functionally inert.

## Active in YR

**Yes (conditional — executes, but adds zero in stock YR).** The read site at `0x0071870b` is inside the `param_5 != '\0'` (mode B = validate) branch of `TeleportLocomotionClass__Update_Position`, which is reachable during normal chrono miner warp sequences. However, the global value is `0x00000000`, so no Z adjustment occurs in practice. Confirmed via `get_xrefs_to 0x00b0ec2c`: 1 READ in `TeleportLocomotionClass__Update_Position`, 1 WRITE by static init at `0x00717f80`.

## Type, Address, Default Value

| Symbol | Address | Type | Default | Notes |
|---|---|---|---|---|
| `g_BridgeZOffset_Teleport` | `0x00b0ec2c` | int32 | 0x00000000 | Z offset (leptons) for bridge destination cells |

## Writers

| Address | Context | Value Written | When |
|---|---|---|---|
| `0x00717f80` | Static initializer (non-function code) | 0x00000000 | Program startup |

No function is found at `0x00717f80` — this is a static data initialization block, not a named function (confirmed via `decompile_function 0x00717f80` returning "No function found"). The same block initializes `g_NullCoord_Teleport_X/Y/Z` at `0x00717f80..0x00717f92`.

## Readers

| Address | Function | Purpose |
|---|---|---|
| `0x0071870b` | `TeleportLocomotionClass__Update_Position` | Adds offset to dest Z when bridge overlay present (mode B path) |

## Usage Context — Update_Position mode B

Source: `decompile_function 0x00718260` (mode B branch, param_5 != '\0'):

```c
// Mode B (param_5 != '\0'): validate/adjust destination in dest-cache-1 (+0x28/+0x2c/+0x30)

// Check bridge overlay at destination cell
iVar6 = CellClass__Get_Cell_At(piVar8);   // piVar8 = dest-cache-1 coord
if (((*(uint *)(iVar6 + 0x140) & 0x100) == 0) ||
   (*(char *)(*(int *)(param_1 + 0xc) + 0x8c) != '\0')) {
  // Not a bridge cell, OR unit already on a bridge:
  *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 0;   // clear bridge-on-bridge flag
}
else {
  // Bridge cell and unit not already on bridge:
  *(undefined1 *)(*(int *)(param_1 + 0xc) + 0x8c) = 1;   // set bridge-on-bridge flag
  *(int *)(param_1 + 0x30) = *(int *)(param_1 + 0x30) + g_BridgeZOffset_Teleport;
  // dest-cache-1 Z += g_BridgeZOffset_Teleport (= 0 in stock YR)
}
```

`param_1` here is the TeleportLocomotionClass pointer (direct byte offsets, not `int *`):
- `param_1 + 0x28` = dest-cache-1 X
- `param_1 + 0x2c` = dest-cache-1 Y
- `param_1 + 0x30` = dest-cache-1 Z ← modified by bridge offset

`*(int **)(param_1 + 0xc)` = TechnoClass owner pointer:
- TechnoClass+0x8c = bridge-on-bridge flag byte

## Lifecycle

- Set to `0x00000000` by static initializer at process startup (one-time write).
- Never written at runtime — read-only after init.
- Architecturally: intended to lift units to bridge height (in leptons) when warping onto a bridge overlay cell.
- In stock YR: value is zero, so no Z adjustment occurs. The bridge-on-bridge flag (`TechnoClass+0x8c`) is still set/cleared correctly regardless of the offset value.

## Relationship to g_NullCoord_Teleport

`g_BridgeZOffset_Teleport` (0x00b0ec2c) is in the same static init block as `g_NullCoord_Teleport_X/Y/Z` (0x00b0ebd8..0x00b0ebe3). The static initializer starting at `0x00717f80` covers both. Address distance: `0x00b0ec2c − 0x00b0ebd8 = 0x54` bytes apart — they are not in the same contiguous array; the gap contains other statics not decoded here.

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `CellClass__Get_Cell_At` | `0x00565730` | General cell lookup; not teleport-specific |
| `CellClass__GetGroundHeight` | (callee near read site) | General cell geometry; not teleport-specific |

## Unverified (YELLOW)

- **Static initializer at `0x00717f80`**: `decompile_function 0x00717f80` returns "No function found." The write xref at `0x00717f80` is confirmed by `get_xrefs_to 0x00b0ec2c`, but the exact instruction sequence was not directly inspected in this session. The value 0x00000000 is confirmed by `read_memory 0x00b0ec2c`. Whether the initializer uses `mov dword [0x00b0ec2c], 0` or a `rep stosd` covering a wider range is not independently verified.

- **INI key for bridge Z offset**: No `ChronoBridgeZOffset` or similar key was found in `rulesmd.ini`. The value may be hardcoded to zero and not INI-configurable. Needs `TechnoTypeClass__ReadINI` / `RulesClass__ReadINI` cross-check to confirm no INI source.
