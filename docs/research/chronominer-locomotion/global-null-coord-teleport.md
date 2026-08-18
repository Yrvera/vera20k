# g_NullCoord_Teleport — 0x00b0ebf8 / 0x00b0ebfc / 0x00b0ec00

**Proposed Ghidra label:** `g_NullCoord_Teleport_X` (for 0x00b0ebf8), `g_NullCoord_Teleport_Y` (for 0x00b0ebfc), `g_NullCoord_Teleport_Z` (for 0x00b0ec00)

> **Address correction:** earlier revisions of this doc cited `0x00b0ebd8 / dc / e0` — that was wrong. `0x00b0ebd8` is an unrelated global with only 3 reads inside Process body (0x00719014, 0x007190d5, 0x00719190) and does NOT match the locomotor's behavioral pattern. The labeler caught the discrepancy when applying Ghidra annotations and surfaced it before any downstream parity work was contaminated. Confirmed via `get_xrefs_to 0x00b0ebf8` returning 13 READ xrefs across Constructor / HeadToCoord / Process (5 sites) / Stop_Moving / StateMachineTick / Update_Position (2 sites) + 1 DATA xref + 1 WRITE — this xref signature matches the doc's behavioral description exactly.

## Summary

Three consecutive 4-byte globals forming a sentinel coordinate triple `(X=0, Y=0, Z=0)` used throughout the TeleportLocomotionClass to represent "no destination cached." The locomotor compares its destination-cache fields against these sentinel values to determine whether a valid warp target has been set.

Verified via `read_memory 0x00b0ebf8` (12 bytes): all zeros at runtime.

## Active in YR

**Yes.** Read across all 8 TeleportLocomotionClass methods that touch destination state: `Constructor` (sets all 6 cache slots from sentinel), `HeadToCoord`, `Process` (5 distinct READ sites), `Stop_Moving` (writes sentinel to clear cache), `StateMachineTick`, `Update_Position` (2 READ sites). 13 READ xrefs total. Static-initialised at program startup by code at `0x00717fa2` (WRITE xref). All confirmed via `get_xrefs_to 0x00b0ebf8`.

## Type, Address, Default Value

| Symbol | Address | Type | Default | Notes |
|---|---|---|---|---|
| `g_NullCoord_Teleport_X` | `0x00b0ebf8` | int32 | 0x00000000 | Sentinel X coord |
| `g_NullCoord_Teleport_Y` | `0x00b0ebfc` | int32 | 0x00000000 | Sentinel Y coord |
| `g_NullCoord_Teleport_Z` | `0x00b0ec00` | int32 | 0x00000000 | Sentinel Z coord |

Memory layout: three consecutive 4-byte words at `0x00b0ebf8..0x00b0ec03`, confirmed via `read_memory 0x00b0ebf8` (12 bytes, all zeros).

## Writers

| Address | Context | Value Written | When |
|---|---|---|---|
| `0x00717fa2` | Static initializer (non-function code) | 0x00000000 | Program startup |

Y (`0x00b0ebfc`) and Z (`0x00b0ec00`) have no direct DATA xrefs in Ghidra. This is because Ghidra only annotates the first address of a multi-field access — the Y and Z writes are part of the same `memset` / `rep stosd` that initializes all three fields at once. The static initializer at `0x00717f80..0x00717fa2` covers all three.

## Readers

| Address | Function | Purpose |
|---|---|---|
| `0x00718008`, `0x00718029` | `TeleportLocomotionClass__Constructor` | Reads sentinel to seed both destination-cache slots at +0x1c..+0x30 |
| `0x007181b4` | `TeleportLocomotionClass__HeadToCoord` | Compares incoming dest against sentinel (no-dest gate) |
| `0x00718b76`, `0x00718bf3`, `0x00718e95`, `0x00718ecd`, `0x00719249` | `TeleportLocomotionClass__Process` | Compares dest-cache against sentinel at 5 distinct branch points |
| `0x00718234` | `TeleportLocomotionClass__Stop_Moving` | Reads sentinel before writing it to clear the cache |
| `0x007193a1` | `TeleportLocomotionClass__StateMachineTick` | Compares ChronoInTransit dest against sentinel before warp commit |
| `0x0071865e`, `0x0071872f` | `TeleportLocomotionClass__Update_Position` | Compares cache-1 fields against sentinel |
| `0x00718198` | `TeleportLocomotionClass__HeadToCoord` | DATA xref (constant pool reference, used by adjacent code) |

## Lifecycle

- Set to `(0,0,0)` by static initializer at process startup (one-time write at `0x00717fa2`).
- Never written at runtime — read-only sentinel.
- Used as the canonical "null destination" marker. The locomotor compares both dest-cache slots against this triple to decide whether to fall back to TechnoClass current location or to commit the cached warp destination.
- Value `(0,0,0)` works as sentinel because no valid in-map unit position can be (0,0,0) in lepton space: all playable cells have a 1-cell impassable border, so the minimum lepton coordinate for any unit is (256, 256) = `(0x100, 0x100)`. The Constructor relies on this invariant when seeding the cache.

## Out-of-Scope Refs

None — the sentinel is used only within TeleportLocomotionClass.

## Unverified (YELLOW)

- **Y address `0x00b0ebfc` and Z address `0x00b0ec00`**: no direct Ghidra DATA xrefs. Addresses inferred from memory layout (12-byte read from `0x00b0ebf8` shows all zeros; Y is at +4, Z at +8). Confirmed indirectly by Constructor decompile writing `g_NullCoord_Teleport_Y` and `_Z` symbolically — Ghidra resolves these symbols to the adjacent addresses. No independent static-analysis xref verification.
- **Static initializer extent (`0x00717f80..0x00717fa2`)**: not independently decoded. The address `0x00717fa2` is confirmed as a WRITE xref to `0x00b0ebf8`; the surrounding init block may also initialise neighbouring sentinels (`g_BridgeZOffset_Teleport` at `0x00b0ec2c` is one such candidate, 0x34 bytes further on). Not verified in this decode.
