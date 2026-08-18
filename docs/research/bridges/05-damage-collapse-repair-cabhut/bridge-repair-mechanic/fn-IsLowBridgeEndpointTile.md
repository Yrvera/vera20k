# MapClass::IsLowBridgeEndpointTile — Decode Doc

Address: `0x00574600`  
Body range: `0x00574600 – 0x005746BA`  
Calling convention: `__fastcall` — `param_1` in EAX/ECX, `param_2` in EDX, `param_3` on stack.  
Scope: Full function.

## Summary

`MapClass::IsLowBridgeEndpointTile` answers: "Is this cell the endpoint (terminal ramp) tile of a
low (wooden) bridge?" Given a tile-type index (`param_1`), an axis discriminator (`param_2`: 2=NS,
4=EW), and a `CellClass*` pointer (`param_3`), it checks whether `param_1` matches one of the
known endpoint tile-type globals for that axis AND whether `CellClass+0x11A` (sub-tile index) is
the expected value. Returns 1 if endpoint, 0 otherwise.

This is the low-bridge twin of the endpoint check used in `DestroyBridge_Low_OnHutDeath` (task #3)
to halt the bridge-body walk when the destruction pass reaches a ramp tile. It is a pure predicate
with no side effects.

## Active in YR

**Yes.** Verified via `get_function_callers 0x00574600`: two callers —
`MapClass::DestroyBridge_Low_OnHutDeath @ 0x00574C20` and
`MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000`. Both are live YR paths (confirmed in
task #3 and task #4 decode docs). The function fires on every bridge collapse event triggered by
C4 on a bridge repair hut.

## Decompilation Excerpt

From `decompile_function 0x00574600`:

```c
uint __fastcall MapClass__IsLowBridgeEndpointTile(uint param_1, int param_2, int param_3)
{
    // param_1: tile type index of the cell
    // param_2: axis discriminator — 2=NS, 4=EW
    // param_3: CellClass* pointer to the cell

    cVar4 = '\x02';  // default sub-tile for EW path
    cVar3 = '\x02';

    if (param_2 == 2) {
        // NS axis: sub-tile value is 0x04
        cVar4 = '\x04';
        cVar3 = '\x04';
        uVar1 = DAT_00abc1e8;  // NS endpoint tile set A base

        // Check NS single-tile endpoints directly
        if (((param_1 == DAT_00abc1e8) && (*(char *)(param_3 + 0x11a) == '\x04')) ||
            ((in_EAX = DAT_00abad30, param_1 == DAT_00aa0e38 && (*(char *)(param_3 + 0x11a) == '\x04'))))
            goto LAB_005746ad;  // → return 1

    } else {
        if (param_2 != 4) goto LAB_005746b4;  // unrecognized axis → return 0

        // EW axis: sub-tile value is 0x02
        if ((param_1 == DAT_00abc1d0) && (*(char *)(param_3 + 0x11a) == '\x02')) {
            return CONCAT31(uVar2, 1);  // return 1
        }
        in_EAX = DAT_00aa1028;
        if ((param_1 == DAT_00aa1540) && (*(char *)(param_3 + 0x11a) == '\x02')) {
            return CONCAT31(uVar2, 1);  // return 1
        }
    }

    // Shared: check 4-tile set (in_EAX + 0/1/2/3) with matching sub-tile
    if ((((param_1 == in_EAX) || (param_1 == in_EAX + 3)) ||
          (param_1 == in_EAX + 1)) ||
         (in_EAX = in_EAX + 2, cVar3 = cVar4, param_1 == in_EAX))
        && (*(char *)(param_3 + 0x11a) == cVar3)) {
LAB_005746ad:
        return CONCAT31((int3)(uVar1 >> 8), 1);  // return 1
    }
LAB_005746b4:
    return in_EAX & 0xffffff00;  // return 0
}
```

## Behavioral Analysis

### Axis discriminator

`param_2` selects the bridge axis:

| `param_2` value | Axis | Sub-tile check value |
|---|---|---|
| `2` | NS (North-South) | `0x04` |
| `4` | EW (East-West) | `0x02` |
| Any other | — | returns 0 silently |

The sub-tile values `0x04` and `0x02` match the ramp-tile sub-tile expectations documented in
`IsBridgeRampTile` (`fn-IsBridgeRampTile.md`): theater-B 4-tile ramps use sub-tile `0x04` and
theater-D 4-tile ramps use sub-tile `0x02`.

### Tile-type globals

All six `DAT_*` globals are **runtime-populated** — they read as zero in the static binary
(`read_memory` at all addresses confirmed `00 00 00 00`). They are written by
`Read_Theater_TileSets_INI @ 0x00545B88` and `0x00545BEC` (confirmed via `get_xrefs_to
0x00abc1e8` and `get_xrefs_to 0x00abc1d0`).

| Global | Axis | Role |
|---|---|---|
| `DAT_00abc1e8` | NS | NS endpoint tile set A — single-tile check #1 |
| `DAT_00aa0e38` | NS | NS endpoint tile set B — single-tile check #2 |
| `DAT_00abad30` | NS | NS endpoint 4-tile set base (tiles +0, +1, +2, +3) |
| `DAT_00abc1d0` | EW | EW endpoint tile set A — single-tile check #1 |
| `DAT_00aa1540` | EW | EW endpoint tile set B — single-tile check #2 |
| `DAT_00aa1028` | EW | EW endpoint 4-tile set base (tiles +0, +1, +2, +3) |

Note: `DAT_00abad30` and `DAT_00aa1028` are also used by `IsBridgeRampTile` for the same
4-tile set comparisons — these globals serve both endpoint and ramp classification, the
distinction being which sub-tile value is required.

### 4-tile set matching block

After the axis-specific single-tile checks, a shared block tests `param_1` against
`in_EAX + 0`, `in_EAX + 1`, `in_EAX + 2`, `in_EAX + 3` with `cVar3` as the sub-tile
comparand (set to `0x04` for NS or `0x02` for EW by the axis branch above).

For the **NS path**: `in_EAX` is loaded with `DAT_00abad30` (the single-tile check #2
arms the variable), so the 4-tile block checks `DAT_00abad30 + {0,1,2,3}` with sub-tile
`0x04`.

For the **EW path**: `in_EAX` is loaded with `DAT_00aa1028` (loaded just before the
single-tile check #2), so the 4-tile block checks `DAT_00aa1028 + {0,1,2,3}` with
sub-tile `0x02`.

### `CellClass+0x11A` — sub-tile index field

`*(char *)(param_3 + 0x11A)` reads a signed byte from offset `0x11A` of the `CellClass`
struct. This is the **sub-tile index** — which frame within the tile type's multi-frame
tile sheet the cell is using. The same field and offset are used by `IsBridgeRampTile`
(verified in `fn-IsBridgeRampTile.md`).

### Relationship to `IsBridgeRampTile`

`IsLowBridgeEndpointTile` and `IsBridgeRampTile` (at `0x005746C0`) share several tile
globals (`DAT_00ABAD30`, `DAT_00AA1028`) and the same `CellClass+0x11A` field. The
difference: `IsBridgeRampTile` checks a broader set of ramp configurations (6 tile groups
across all theaters); `IsLowBridgeEndpointTile` checks specifically the terminal-endpoint
tiles at the ends of a low bridge span, using an axis parameter to restrict which tiles
are valid. An endpoint tile IS also a ramp tile, but not all ramp tiles are endpoint tiles.

### Return value encoding

The return is a `uint` with `CONCAT31` masking: lower byte is 1 (true) or 0 (false). The
upper 3 bytes come from incidental register state but the caller only tests the low byte.
Semantically this is a boolean.

## Struct Field Accesses

`param_3` is `CellClass*`.

| Source | Offset | Field | Role |
|---|---|---|---|
| CellClass | `+0x11A` | Sub-tile index (signed byte) | Determines which frame of the tile type; must match axis-specific expected value |

## Globals Referenced

| Global | Written by | Role |
|---|---|---|
| `DAT_00abc1e8` | `Read_Theater_TileSets_INI @ 0x00545B88` | NS endpoint tile A |
| `DAT_00aa0e38` | (runtime init) | NS endpoint tile B |
| `DAT_00abad30` | `Read_Theater_TileSets_INI @ 0x00545B88` | NS/ramp 4-tile set base (also used by IsBridgeRampTile) |
| `DAT_00abc1d0` | `Read_Theater_TileSets_INI @ 0x00545BEC` | EW endpoint tile A |
| `DAT_00aa1540` | (runtime init) | EW endpoint tile B |
| `DAT_00aa1028` | (runtime init) | EW/ramp 4-tile set base (also used by IsBridgeRampTile) |

## Callers

Verified via `get_function_callers 0x00574600`:

| Caller | Address | Context |
|---|---|---|
| `MapClass::DestroyBridge_Low_OnHutDeath` | `0x00574C20` | Halts low-bridge walk at endpoint tiles |
| `MapClass::DestroyBridge_High_OnHutDeath` | `0x00574000` | Also called during high-bridge collapse path |

## Callees

Verified via `get_function_callees 0x00574600`: none. Pure predicate — no function calls,
no side effects, reads only.

## Out-of-scope Refs

- `MapClass::IsBridgeRampTile` @ `0x005746C0` — decode task #13. Sibling predicate sharing
  tile globals `DAT_00ABAD30` and `DAT_00AA1028`.
- `Read_Theater_TileSets_INI` @ `0x00545B88` — writes the tile-type globals; not a bridge
  decode task.
- `MapClass::DestroyBridge_Low_OnHutDeath` @ `0x00574C20` — decode task #3 (primary caller).
- `MapClass::DestroyBridge_High_OnHutDeath` @ `0x00574000` — decode task #4.

## Unverified Claims (YELLOW)

- The "axis discriminator" interpretation of `param_2` (2=NS, 4=EW) is inferred from the
  `if (param_2 == 2)` / `if (param_2 != 4)` branch structure and from the sub-tile values
  `0x04` / `0x02` matching NS/EW ramp orientations in `IsBridgeRampTile`. Direct caller
  inspection would confirm the values passed.
- The exact INI / theater keys that write `DAT_00abc1e8`, `DAT_00abc1d0`, `DAT_00aa0e38`,
  `DAT_00aa1540`, and `DAT_00aa1028` via `Read_Theater_TileSets_INI` are not decoded here
  (tileset loader is out of scope).
- `CellClass+0x11A` as "sub-tile index" is inferred from `IsBridgeRampTile` using the same
  field for the same purpose; confirmed consistent but full `CellClass` layout verification
  belongs to task #21.
- The "endpoint tile" vs "ramp tile" distinction is inferred from usage context — both
  functions check overlapping globals, but this function is specifically called to terminate
  a bridge-body walk, implying it identifies the terminal tile.

## Self-Proof

### Claim 1: Function identity

`get_function_by_address 0x00574600` → `MapClass__IsLowBridgeEndpointTile`, body
`0x00574600 – 0x005746BA`. **VERIFIED — matches task spec.**

### Claim 2: Two callers only; both live YR paths

`get_function_callers 0x00574600` → `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574C20`
and `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000`. Both confirmed live YR in
task #3 and #4 docs. **VERIFIED — narrow scope, no TS dead code.**

### Claim 3: Tile globals are runtime-populated (all zero at static time)

`read_memory 0x00abc1e8` → `00 00 00 00`; `read_memory 0x00abc1d0` → `00 00 00 00`;
`read_memory 0x00aa0e38` → `00 00 00 00`; `read_memory 0x00aa1028` → `00 00 00 00`.
`get_xrefs_to 0x00abc1e8` and `get_xrefs_to 0x00abc1d0` both show WRITE xref from
`Read_Theater_TileSets_INI @ 0x00545B88 / 0x00545BEC`. **VERIFIED — runtime-populated
by theater tileset INI loader, consistent with `IsBridgeRampTile` globals behavior.**
