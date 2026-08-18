# MapClass::DestroyBridge_Low_OnHutDeath — Decode Doc

Address: `0x00574C20`  
Scope: Full function.

## Summary

`MapClass::DestroyBridge_Low_OnHutDeath` is called when a BridgeRepairHut (low-bridge type)
has its C4 charge expire or a bomb detonates on it. Given the hut's cell coordinate as
`param_2`, it:

1. Scans a 5×5 neighbourhood for any cell with `CellClass+0x44` (overlay subtype) in
   `[0x4A, 0x65]`. On first match: calls `DestroyBridgeFromCell_Low` and returns immediately.
2. If no overlay match: falls back to reading `CellClass+0x140` flags on the hut cell and
   searching 8 compass directions up to 3 cells out for a cell with flags `& 0x500 != 0`.
3. Based on anchor cell flags, computes a bridge-segment start cell (anchor), then walks
   the bridge in direction `(flags & 0x800) ? dir+2 : dir`, calling `ApplyDamageToCell`
   up to 3 times per ramp tile found, until `IsLowBridgeEndpointTile` fires.
4. At end of walk: calls `UpdateAdjacentBridges_High`, sets `g_Tactical+0xD7C = 1` (marks
   tactical display dirty), and unconditionally calls `UpdateBridgeZonesHelper`.

This produces the observable "low bridge collapses when C4 is planted on its repair hut"
behavior.

## Active in YR

**Yes.** Callers verified via `get_function_callers 0x00574C20`:
- `BombClass::Detonate` @ `0x00438720`
- `BuildingClass::Update` @ `0x0043FB20`

Both are live YR skirmish paths. No TS-only gating flags visible in decompilation.

## Decompilation Excerpt

From `decompile_function 0x00574C20`:

```c
void __thiscall MapClass__DestroyBridge_Low_OnHutDeath(int param_1, short *param_2)
{
    // ---- Phase 1: 5x5 overlay scan ----
    // iVar9 = dx, iVar8 = dy, each -2..+2
    iVar9 = -2;
    do {
        iVar8 = -2;
        do {
            psVar5 = param_2;
            local_2c = CONCAT22(param_2[1] + (short)iVar8, *param_2 + (short)iVar9);
            local_1c = local_2c;
            iVar2 = MapClass__Get_CellClass(&local_1c);
            // CellClass+0x44: overlay/subtype field; range [0x4A..0x65] = low-bridge
            if ((0x49 < *(int *)(iVar2 + 0x44)) && (*(int *)(iVar2 + 0x44) < 0x66)) {
                param_2 = CONCAT22(psVar5[1] + (short)iVar8, *psVar5 + (short)iVar9);
                MapClass__DestroyBridgeFromCell_Low(&param_2);  // @ 0x00574780
                return;   // early exit on first match
            }
            iVar8++;
        } while (iVar8 < 3);
        iVar9++;
    } while (iVar9 < 3);

    // ---- Phase 2: fallback — read anchor from hut cell flags ----
    // psVar5 still = param_2 (original hut cell)
    iVar9 = psVar5[1] * 0x200 + (int)*psVar5;
    if ((iVar9 < 0) || (0x3ffff < iVar9) ||
        (puVar6 = *(CellClass**)(g_CellArray_Base + iVar9*4), puVar6 == NULL)) {
        DAT_00abdc74 = *(short**)psVar5;   // out-of-bounds: use sentinel
        puVar6 = &DAT_00abdc50;
    }
    local_2c = 0;
    if ((*(uint*)(puVar6 + 0x140) & 0x500) == 0) {
        // Hut cell has no bridge flags → search 8 directions, up to 3 cells
        local_1c = 0;
        do {
            // Step 1 in dir
            sVar4 = *psVar5 + *(short*)(&g_DirectionOffsets + (int)local_2c);
            sVar7 = *(short*)(&g_DirectionOffsets + (int)local_2c*4 + 2) + psVar5[1];
            // ... (3 steps, each checks & 0x500; first match breaks)
            local_2c = (local_2c + 1) & 7;
            local_1c++;
            psVar5 = param_2;
        } while ((int)local_1c < 8);
    }

    // ---- Phase 3: classify anchor and derive bridge start cell ----
    uVar10 = *(uint*)(puVar6 + 0x140);
    if (((uVar10 & 0x100) == 0) && ((uVar10 & 0x400) == 0)) return;

    if ((uVar10 & 0x100) == 0) {
        // Pure bridgehead (0x400 set, 0x100 clear): walk perp until non-0x400 cell
        local_1c = *(short**)(puVar6 + 0x24);
        uVar10 = -(uint)((uVar10 & 0x800) != 0) & 2;  // 0 or 2
        local_24 = uVar10 + 2;
        // ... walk up to 4 cells in direction local_24; if still 0x400 after 4 → return
        // offsets 2 more cells in reflected direction to get bridge start
    } else if ((uVar10 & 0x80) == 0) {
        // Bridge cell without ramp flag: anchor = *(cell+0x2c)->coord (+0x24)
        local_30 = *(short**)(*(int*)(puVar6 + 0x2c) + 0x24);
    } else {
        // Bridge cell with ramp flag (0x80): anchor = cell->coord (+0x24)
        local_30 = *(short**)(puVar6 + 0x24);
    }

    // ---- Phase 4: walk bridge, call ApplyDamageToCell on ramp tiles ----
    uVar10 = -(uint)((*(uint*)(puVar6 + 0x140) & 0x800) != 0) & 6;  // direction: 0 or 6
    FUN_0042fcb0(0, 0);  // unknown setup call
    // Walk forward from local_30 in direction uVar10:
    // On IsBridgeRampTile: call ApplyDamageToCell up to 3 times in reversed direction
    // Continue until IsLowBridgeEndpointTile or map bounds exceeded

    // ---- Phase 5: post-destruction bookkeeping ----
LAB_005751c9:
    MapClass__UpdateAdjacentBridges_High(&local_30);
    *(byte*)(g_Tactical + 0xd7c) = 1;  // mark tactical dirty
LAB_005751e6:
    MapClass__UpdateBridgeZonesHelper();
    // destructor cleanup (local_18 / FUN_007c8b3d)
    return;
}
```

## Behavioral Analysis

### Phase 1 — 5×5 overlay scan

The scan is column-major: outer loop is `dx` (iVar9), inner is `dy` (iVar8), each from
−2 to +2 inclusive (25 cells total). The single check is `CellClass+0x44 ∈ (0x49, 0x66)`.
On the first matching cell (in column-major order), `DestroyBridgeFromCell_Low` is called
with that cell's coordinates and the function returns. No further processing.

This is the fast path for huts already on top of a bridge tile.

Note: the scan here uses only `CellClass+0x44` (overlay subtype), whereas the parent
`BuildingClass::Update` scan (task #1) checked BOTH `CellClass+0x38` (tile-type index vs
`DAT_00abad1c`) AND `CellClass+0x44`. This function performs the overlay-only check.

### Phase 2 — Cell flag search

If Phase 1 finds nothing (hut is not directly over a low-bridge tile), the function reads
`CellClass+0x140` (bridge flags bitmask) on the hut cell. If `flags & 0x500 == 0`
(no bridge flags), it walks all 8 compass directions up to 3 cells each, stopping when any
cell has `flags & 0x500 != 0`. This locates the nearest bridge cell adjacent to the hut.

`g_DirectionOffsets` is a table of (dx, dy) pairs for 8 compass directions (index 0–7).

### Phase 3 — Anchor classification

The anchor cell (from Phase 2, or the hut cell if it had flags) is classified by its
`CellClass+0x140` flags:

| Flags | Interpretation | Bridge start derivation |
|---|---|---|
| `0x100 == 0` AND `0x400 == 0` | Not a bridge cell at all | Return immediately (no-op) |
| `0x400 set`, `0x100 clear` | Pure bridgehead | Walk perpendicular until non-0x400; offset 2 more to get bridge start |
| `0x100 set`, `0x80 clear` | Bridge cell, not a ramp | Start = `*(cell+0x2C)->coord (+0x24)` (linked bridge-segment pointer) |
| `0x100 set`, `0x80 set` | Bridge cell with ramp | Start = `cell->coord (+0x24)` |

The `0x800` flag controls walk direction: `0x800` set → direction index `+= 2`; clear →
direction base. Maps to NS vs EW axis of the bridge.

### Phase 4 — Walk and damage

Starting from `local_30` (computed start cell), the function walks in direction `uVar10`
(derived from `0x800` flag of anchor cell: either 0 or 6 from the `& 6` mask). At each
step:
- Checks `MapClass::IsBridgeRampTile` on the current cell.
- If ramp: calls `ApplyDamageToCell` up to 3 times in the **reversed** direction
  (`uVar10 - 4 & 7`).
- Continues until `MapClass::IsLowBridgeEndpointTile` returns non-zero or map bounds
  exceeded.
- When endpoint reached and tile offset `iVar9 != -2` (not at bridge start), reverses and
  calls `ApplyDamageToCell` up to 3 more times for the endpoint.

`ApplyDamageToCell` is the function that actually collapses individual bridge-tile objects.

### Phase 5 — Post-destruction bookkeeping

Regardless of which path was taken in Phases 3–4:
- `MapClass::UpdateAdjacentBridges_High` is called with `&local_30` (the start cell).
- `g_Tactical + 0xD7C` is set to `1` (marks the tactical display dirty for redraw).
- `MapClass::UpdateBridgeZonesHelper` is called unconditionally.

These three steps always execute for every successful bridge destruction.

## Struct Field Accesses

`param_2` is `short*` — a cell coordinate pair (X=`*param_2`, Y=`param_2[1]`), in cell
units (NW-corner reference frame per CLAUDE.md conventions).

`param_1` is `int` — the MapClass `this` pointer; direct byte offsets apply.

| Source | Offset | Field | Role |
|---|---|---|---|
| CellClass (return of Get_CellClass) | `+0x44` | Overlay subtype | Low-bridge overlay range check [0x4A..0x65] |
| CellClass | `+0x140` | Bridge flags bitmask | `0x100`=bridge, `0x400`=bridgehead, `0x80`=ramp, `0x800`=axis, `0x500`=any-bridge |
| CellClass | `+0x24` | Cell coordinate | Starting coord for bridge walk |
| CellClass | `+0x2C` | Linked segment ptr | `*(cell+0x2C)+0x24` = coord of the start of linked bridge segment |
| CellClass | `+0x38` | Tile type index | Used in `IsLowBridgeEndpointTile` check (`iVar9 = +0x38 - DAT_00abad1c`) |
| MapClass (param_1) | `+0x124` | Map X origin | Map cell origin X for bounds checking |
| MapClass (param_1) | `+0x128` | Map Y origin | Map cell origin Y for bounds checking |
| MapClass (param_1) | `+0x12C` (`+300`) | Map X extent | Width for bounds |
| MapClass (param_1) | `+0x130` | Map Y extent | Height for bounds |
| MapClass (param_1) | `+0x13C` | Visibility/passability grid | Used in bridge-walk loop |

All CellClass offsets are direct byte offsets (param to Get_CellClass is `int`, not `int*`).

## Globals Referenced

| Global | Address | Role |
|---|---|---|
| `g_CellArray_Base` | (named) | Array of CellClass ptrs indexed by `Y*0x200 + X` |
| `g_DirectionOffsets` | (named) | 8-entry table of (short dx, short dy) pairs for compass dirs |
| `DAT_00abdc50` | `0x00ABDC50` | Sentinel null-cell used as fallback on out-of-bounds coord |
| `DAT_00abdc74` | `0x00ABDC74` | Scratch coord storage; written by `Get_CellClass` on out-of-bounds |
| `DAT_00abad1c` | `0x00ABAD1C` | Low-bridge tile-type table base (runtime-populated) |
| `g_Tactical` | (named) | Tactical display object; `+0xD7C` = dirty flag |

`DAT_00abdc50` is referenced as a safe fallback cell object written also in
`MapClass__DestroyBridgeWalker_NS_Low` (verified via `get_xrefs_to 0x00ABDC50`).

## Out-of-scope Refs

- `MapClass::DestroyBridgeFromCell_Low` @ `0x00574780` — decode task #5
- `ApplyDamageToCell` @ `0x00587180` — applies damage to a single bridge tile cell
- `MapClass::IsBridgeRampTile` @ `0x005746C0` — decode task #13
- `MapClass::IsLowBridgeEndpointTile` @ `0x00574600` — decode task #14
- `MapClass::UpdateAdjacentBridges_High` @ `0x00576770` — decode task #12
- `MapClass::UpdateBridgeZonesHelper` @ `0x0056C510` — decode task #11
- `FUN_0042FCB0` — called with `(0,0)` before the walk phase; purpose unknown
- `BombClass::Detonate` @ `0x00438720` — other caller; shares the same bridge-destroy path

## Unverified Claims (YELLOW)

- `CellClass+0x140` field name as "bridge flags bitmask" is inferred from bit-pattern
  usage (`0x100`, `0x400`, `0x80`, `0x800`, `0x500`) and the bridge context. Not verified
  via struct layout decode; `decode-struct-CellClass_BridgeFields` (task #21) will confirm.
- `CellClass+0x24` is identified as "cell coordinate" (short pair) from the pattern of
  adding direction offsets to it and using it in the Y*0x200+X cell-index formula. Not
  independently confirmed via struct layout.
- `CellClass+0x2C` is identified as a "linked bridge segment pointer" (another CellClass*)
  from the `*(int*)(puVar6+0x2C)` dereference to get another `+0x24` coord. The linked-
  segment semantics are inferred from the bridge-walk context.
- `g_DirectionOffsets` at-name is from Ghidra label; the 8-entry (dx,dy) structure is
  inferred from usage (index `&7`, stride `*4`, half-word reads at `+0` and `+2`).
- `FUN_0042FCB0` purpose is unknown; called with `(0,0)` before the walk loop. Not decoded.
