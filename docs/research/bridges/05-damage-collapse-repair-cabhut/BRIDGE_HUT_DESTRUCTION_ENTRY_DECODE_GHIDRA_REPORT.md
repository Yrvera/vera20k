# Bridge Hut-Destruction Entry — Exact Decode

**Date:** 2026-05-17
**Scope:** Three questions left open at the end of
[BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md](BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md)
§13/§18/§19:

1. **Exact entry behavior of `MapClass::DestroyBridge_{Low,High}_MapInit`** —
   the 5×5 scan: which tile-ranges/flags count as a valid start, the
   priority/order of acceptance, what fallback paths exist when no overlay
   match is found.
2. **`DestroyBridgeFromCell_High` (0x5749C0) and `_Low` (0x574780)** — the
   "destruction-side direction-detect" dispatchers Phase 2 located but did
   not decompile. These are the entries to the actual `CollapseBridge_*`
   destruction walker family.
3. **Difference between delayed-C4-hut branch and infantry-enter-hut
   branch** — they superficially look identical (both have 5×5 scans and
   both end in bridge destruction-or-repair) but diverge in the second
   function-tier and again at the walker family.

**Verdict (TL;DR):**

- All three "5×5 scan" sites (`InfantryClass::PerCellProcess` engineer-enter,
  `BuildingClass::Update` C4-timer expiry, `BombClass::Detonate` demo-truck)
  use the **same** outer scan: per-cell test is `tile_index ∈ [DAT_00abad1c,
  +0x10)` OR `overlay ∈ [0x4A..0x65]`, **no short-circuit**, decision is a
  single bool "low-bridge-found-anywhere-in-25-cells" → low vs high dispatcher.
- The inner 5×5 scan inside `DestroyBridge_{Low,High}_MapInit` is **different**:
  it tests overlay-only (`[0x4A..0x65]` low, `[0xCD..0xE8]` high), it
  **does** short-circuit on first match (column-major: dx outer, dy inner),
  and on match it calls `DestroyBridgeFromCell_{Low,High}` and returns
  immediately.
- If the inner overlay scan finds nothing, the function falls into a
  **flag-based ramp/bridgehead walk** keyed off `cell.+0x140` bits
  `0x80/0x100/0x400/0x500/0x800`.
- `DestroyBridgeFromCell_{Low,High}` are line-by-line twins of
  `RepairBridge_{Low,High}` (§3.3/§3.4 of the parent doc) with identical
  anchor selection, differing only in which walker family they invoke —
  `CollapseBridge_{NS,EW}_{Low,High}` instead of
  `RepairBridgeWalker_{NS,EW}_{Low,High}`.
- The delayed-C4 branch and the infantry-enter branch share the **outer
  5×5 entry-scan shape exactly**, but diverge at the second function tier
  and never recombine: delayed-C4 → `DestroyBridge_*_MapInit` →
  `DestroyBridgeFromCell_*` → `CollapseBridge_*` (destruction); infantry-enter
  → `ProcessBridgeDestruction_*` → `RepairBridge_*` →
  `RepairBridgeWalker_*` (repair).

---

## 1. Question 1 — Exact `DestroyBridge_*_MapInit` entry behavior

### 1.1 What the caller does first — the outer 5×5

The outer 5×5 scan that decides Low vs High dispatcher lives in the
**caller** (`BuildingClass::Update` or `BombClass::Detonate`), not in
`DestroyBridge_*_MapInit` itself. I re-verified it directly:

```asm
; BuildingClass::Update tail, 0x440226..0x4402de
0x440226: MOV EBX, -2                  ; dy = -2 (outer)
0x44022b: MOV EBP, -2                  ; dx = -2 (inner)
0x440230: MOV EDX, [ESI]               ; this.vtable
0x440236: PUSH ...                     ; out coord buffer
0x440239: CALL [EDX + 0x1b8]           ; vtable[0x1b8] = GetCoord
0x44023f: MOV DX, [EAX + 2]            ; building.coord.y
0x440246: ADD CX, [EAX]                ; building.coord.x + dx
0x440249: ADD DX, BX                   ; building.coord.y + dy
0x440262: PUSH ECX
0x440263: MOV ECX, 0x87f7e8            ; MapClass instance
0x440268: CALL 0x005657a0              ; MapClass::Get_CellClass(&coord)
0x44026d: MOV EDI, [EAX + 0x38]        ; tile_index = cell.field_0x38
                                       ; (second GetCoord + Get_CellClass
                                       ;  immediately afterward; compiler
                                       ;  did not fold)
0x4402ad: MOV ECX, [0x00abad1c]        ; load low-bridge tile-base
0x4402b3: MOV EAX, [EAX + 0x44]        ; overlay = cell.field_0x44
0x4402b6: CMP EDI, ECX                 ; tile vs base
0x4402b8: JL  0x004402c1               ; if tile < base, fall to overlay test
0x4402ba: ADD ECX, 0x10
0x4402bd: CMP EDI, ECX
0x4402bf: JL  0x004402cb               ; if tile in [base, base+0x10) -> match
0x4402c1: CMP EAX, 0x4a
0x4402c4: JL  0x004402d0               ; overlay < 0x4a -> no match
0x4402c6: CMP EAX, 0x65
0x4402c9: JG  0x004402d0               ; overlay > 0x65 -> no match
0x4402cb: MOV byte [ESP+0x13], 0x1     ; low_bridge_found = true
0x4402d0: INC EBP                      ; dx++
0x4402d4: JL  0x00440230               ; while dx < 3
0x4402da: INC EBX                      ; dy++
0x4402de: JL  0x0044022b               ; while dy < 3

0x4402e4: MOV AL, [ESP+0x13]
0x4402ec: JZ  0x00440308               ; not low -> high
0x440301: CALL 0x00574c20              ; DestroyBridge_Low_MapInit
0x440306: JMP 0x00440320
0x44031b: CALL 0x00574000              ; DestroyBridge_High_MapInit
0x440320: MOV byte [ESI+0x6df], 0x0    ; field_0x6DF cleared
0x440327: MOV [ESI+0x540], 0x0         ; engineer kill-credit cleared
```

**Properties of the outer 5×5:**

- **Iteration order (BuildingClass::Update):** outer = dy (`-2..+2`),
  inner = dx (`-2..+2`). 25 cells total, row-major (north to south,
  each row west to east).
- **Match condition:** `cell.tile_index ∈ [DAT_00abad1c, +0x10)` **OR**
  `cell.overlay ∈ [0x4A..0x65]`. (Single bool, low-only.)
- **No short-circuit.** The bool is only used after the loop ends; all
  25 cells are tested even if cell #1 matches. So iteration order is
  parity-irrelevant for the outer scan.
- **Decision:** if `low_bridge_found == true` → `DestroyBridge_Low_MapInit`,
  else → `DestroyBridge_High_MapInit`. There is no "ambiguous" branch
  and no high-bridge match-test; high is the default when no low-bridge
  evidence is found in 25 cells.
- After the dispatch the building's `field_0x6DF` (C4-plant flag) and
  `field_0x540` (engineer kill-credit ptr) are zeroed.

The `InfantryClass::PerCellProcess` engineer-enter scan at `0x519c14..0x519cd3`
and the `BombClass::Detonate` scan at the equivalent site are
**byte-for-byte equivalent** with constant substitution; the only
difference is which symbol is called at the bottom. (Verified: I read the
PerCellProcess slice and confirmed the `[0x4A..0x65]` and
`[DAT_00abad1c, +0x10)` constants line up.)

### 1.2 What `DestroyBridge_{Low,High}_MapInit` itself does

Decompilation of `0x574C20` (Low) and `0x574000` (High) shows they are
strict compiled twins differing only in the low-band vs high-band constants
(`0x4A..0x65` ↔ `0xCD..0xE8`; `DAT_00abad1c` ↔ `DAT_00aa0e28`). I describe
the Low version; substitute constants for High.

Functional steps in order:

#### Step A — Inner 5×5 scan (overlay-only, **short-circuiting**)

```c
for (dx = -2; dx < 3; dx++) {         // outer (iVar9)
    for (dy = -2; dy < 3; dy++) {     // inner (iVar8)
        coord = (param_2.x + dx, param_2.y + dy);
        cell  = MapClass::Get_CellClass(coord);
        if (0x4A <= cell.field_0x44 <= 0x65) {     // overlay in low band
            DestroyBridgeFromCell_Low(coord);
            return;                                // first match wins
        }
    }
}
// fall through to Step B
```

**Differences from the outer scan:**
- **Match condition is narrower:** overlay-only, no tile-index OR.
- **Iteration order:** outer = dx, inner = dy (column-major). For the
  outer scan it was the reverse. Order matters here because this scan
  **does** short-circuit.
- **Action on match:** dispatch to `DestroyBridgeFromCell_Low` and
  immediate `return` — Steps B/C/D below are skipped entirely.

**Priority order — answer to user's question:** first cell satisfying
`overlay ∈ [0x4A..0x65]` (low) or `[0xCD..0xE8]` (high) in column-major
sweep (`(x-2,y-2), (x-2,y-1), (x-2,y), (x-2,y+1), (x-2,y+2), (x-1,y-2), …`)
wins.

#### Step B — Flag-based "starter cell" search (only if Step A found nothing)

```c
// Start at original input cell
puVar6 = g_CellArray[input.y * 0x200 + input.x];
if (oob) puVar6 = &DAT_00abdc50;             // sentinel cell

if ((puVar6.field_0x140 & 0x500) == 0) {     // 0x500 = 0x100 | 0x400
    // Walk 8 directions looking up to 3 cells out
    for (dir = 0; dir < 8; dir++) {
        // try cell +1 step in dir
        c1 = cell at (input + g_DirectionOffsets[dir]);
        if ((c1.field_0x140 & 0x500) != 0) { puVar6 = c1; break; }
        // try cell +2 step
        c2 = cell at (input + 2 * g_DirectionOffsets[dir]);
        if ((c2.field_0x140 & 0x500) != 0) { puVar6 = c2; break; }
        // try cell +3 step
        c3 = cell at (input + 3 * g_DirectionOffsets[dir]);
        if ((c3.field_0x140 & 0x500) != 0) { puVar6 = c3; break; }
    }
}

if ((puVar6.field_0x140 & (0x100 | 0x400)) == 0) return;   // no bridge nearby
```

**Flags on `cell.+0x140` used here (semantics from
parent doc §3.6 / §13):**

| Bit | Meaning |
|----:|---------|
| `0x80`  | bridge-walkable (you can stand on it) |
| `0x100` | bridge cell layer flag (part of bridge body) |
| `0x400` | bridgehead/anchor marker |
| `0x500` | `0x100 \| 0x400` — "this is bridge OR bridgehead" |
| `0x800` | orientation/winding bit (flips direction by 180°) |

**Search priority for the flag-based fallback:**
1. Original input cell, if `(flags & 0x500) != 0`.
2. Else iterate `dir = 0..7` of `g_DirectionOffsets[8]`; for each direction
   try cells 1, 2, 3 steps out. First cell with `(flags & 0x500) != 0`
   wins. The loop **breaks** out of the 8-direction sweep on first hit.
3. If still nothing → `return` (no destruction).

#### Step C — Anchor selection from the starter cell

```c
flags = puVar6.field_0x140;
if ((flags & 0x100) == 0) {
    // pure bridgehead (0x400 only, no main-deck flag)
    coord = puVar6.field_0x24;                 // cell's own coord
    dir   = (flags & 0x800) ? 2 : 0;
    dir2  = dir + 2;
    // Walk up to 4 cells in dir2 looking for 0x400-tagged cell;
    // bail (return) if none found in 4 steps
    for (i = 0; i <= 3; i++) {
        coord = coord + g_DirectionOffsets[dir2];
        c = cell at coord;
        if ((c.field_0x140 & 0x400) == 0) break;
        // continue
    }
    if (i > 3) return;
    // anchor = coord + 2 more steps in (dir2 - 2) & 7
    anchor = coord + 2 * g_DirectionOffsets[(dir2 - 2) & 7];
}
else if ((flags & 0x80) == 0) {
    // bridge cell, but not walkable -> use the linked bridge's home coord
    anchor = (*(puVar6 + 0x2c)).field_0x24;    // dereferenced "owning object"
}
else {
    // bridge cell with walkable bit -> use cell's own coord
    anchor = puVar6.field_0x24;
}
direction = (flags & 0x800) ? 6 : 0;           // walking axis
```

So the anchor-resolution priority — **the answer to "what counts as a
valid start besides anchor_span_id":**

1. **Overlay-in-band cell within 5×5** (Step A) — primary fast-path.
2. **Cell whose `+0x140` has the 0x100 bridge-cell-layer bit set, plus
   the 0x80 walkable bit** — anchor = the cell itself.
3. **Cell with 0x100 but not 0x80** (bridge cell that you can't stand on —
   typically a railing/edge cell) — anchor = the linked owning bridge
   object's home cell, via `*(cell + 0x2c).+0x24`.
4. **Cell with only 0x400** (pure bridgehead, no main-deck overlay) —
   walk further along the perpendicular axis to find more 0x400 cells,
   then offset back by 2 cells along the bridge axis to land on the deck.
5. **None of the above within 3 steps in any of 8 directions** — silent
   no-op.

The orientation bit `0x800` flips the walking direction by 180° (uVar10 = 6
vs 0; same offset table indexed with the high pair).

#### Step D — Walk the bridge applying damage

After anchor selection:

```c
DynamicVectorClass local_vec; FUN_0042fcb0(0, 0);
coord  = anchor;
left_x = MapClass.LeftCellX (param_1 + 0x124);
loop {
    if (coord.x < left_x) break;
    if (coord.x > left_x + MapClass.Width) break;
    if (coord.y out of [TopY, TopY + Height]) break;

    cell_idx = coord.y * 0x200 + coord.x;
    if (*(param_1 + 0x13c)[cell_idx * 4] != 0) {       // some object layer
        // Check if this cell is a ramp tile
        if (IsBridgeRampTile(tile, cell) != 0) {
            // Apply damage to current + up to 2 perp cells (3 total)
            for (i = 0; i < 3; i++) {
                if (ApplyDamageToCell(&coord) != 0) break;
            }
            // Walk forward through endpoint cells
            loop {
                coord = coord + g_DirectionOffsets[dir];
                cell  = cell at coord;
                if (oob) goto cleanup;
                tile_rel = cell.field_0x38 - DAT_00abad1c;
                if (IsLowBridgeEndpointTile(cell, ...) != 0) break;
            }
            if (tile_rel != -2) {
                dir = (dir - 4) & 7;          // reverse direction
                coord = coord + g_DirectionOffsets[dir];
                for (i = 0; i < 3; i++) {
                    if (ApplyDamageToCell(&coord) != 0) break;
                }
            }
            goto cleanup;
        }
    }
    coord = coord + g_DirectionOffsets[dir];
}

cleanup:
UpdateAdjacentBridges_High(&anchor);            // **note: _High both paths**
*(Tactical + 0xD7C) = 1;                        // renderer-dirty flag
UpdateBridgeZonesHelper();                       // pathfinding zone rebuild (unconditional)
```

**Per-cell damage application** (`ApplyDamageToCell` @ `0x587180`) dispatches
to the per-cell-damage destruction family (`DestroyBridge_Low` @ `0x57BAA0`
or `DestroyBridge_High` @ `0x57CCF0`, distinct from this entry function)
for overlay-bearing cells, or to the bridge-damage state machine for ramp
tiles. The parent doc §13.3 already covers that subtree.

#### Step E — Tail (always runs unless Step A short-circuited)

- `MapClass::UpdateAdjacentBridges_High(&anchor)` — neighbor refresh, only
  the `_High` version exists; both Low and High paths reuse it (parent
  doc §13.4 documents this as a vanilla copy-paste bug).
- `*(Tactical + 0xD7C) = 1` — global "redraw all bridges" flag for the
  renderer.
- `MapClass::UpdateBridgeZonesHelper()` — full pathfinding zone rebuild.
  Unconditional in this branch (cf. the repair walker, which only fires
  it when a main-deck overlay was actually written).

Note: when Step A short-circuits (overlay found in 5×5), Steps B/C/D/E are
**skipped entirely** inside *this* function, including its
`UpdateBridgeZonesHelper` call. However, the zone rebuild still fires in
the short-circuit path — it comes from inside `CollapseBridge_*` itself
(verified 2026-05-18; see §4.3 correction below). **There is no
asymmetry in the final outcome:** both paths fire `UpdateBridgeZonesHelper`
exactly once before returning to the caller. A previous draft of this
note flagged a parity-porting asymmetry that does not exist.

### 1.3 Summary for the Rust port

**What `DestroyBridge_Low_MapInit` accepts as a "valid start":**

| Priority | Acceptance criterion | Action |
|---------:|----------------------|--------|
| 1 | Any cell within 5×5 of input with overlay in `[0x4A..0x65]` (low) / `[0xCD..0xE8]` (high), found in column-major order (dx outer −2..+2, dy inner −2..+2) | `DestroyBridgeFromCell_Low/High(found_coord)`; **return immediately** — no further work |
| 2 | Input cell has `+0x140 & 0x500 != 0` | Use as starter for ramp walk |
| 3 | Any cell 1, 2, or 3 steps from input along any of the 8 compass directions with `+0x140 & 0x500 != 0` (first direction with first such cell wins) | Use as starter for ramp walk |
| 4 | None of the above | **No-op return** — bridge is not destroyed |

Cases 2–4 dispatch through Step C anchor resolution and Step D ramp walk
+ unconditional `UpdateBridgeZonesHelper`. Case 1 hands off to
`DestroyBridgeFromCell_*` and does no further work in this function.

So the Rust port's "accept anchor_span_id" path covers case 1; the Rust
port currently does **not** model cases 2/3 (the flag-based ramp fallback).
For a destroy_hut order that targets the cabhut directly, case 1 will
almost always fire — the cabhut footprint cell *is* the bridge cell with
overlay in band. Cases 2/3 fire when the hut is destroyed by some other
mechanism that puts the input coord adjacent to (but not on) a bridge
overlay — e.g., a CABHUT sitting on a non-overlay anchor cell whose own
+0x140 flags identify it as part of the bridge.

In vanilla YR with the standard CABHUT placement (foundation overlays the
bridge head), case 1 dominates. Case 2 would matter only if a CABHUT
foundation cell ever held a bridge flag without an overlay — possible for
custom maps or modded CABHUTs.

---

## 2. Question 2 — `DestroyBridgeFromCell_{Low,High}` bodies

These are the **destruction-side compiled twins** of `RepairBridge_Low`
(`0x57F200`) and `RepairBridge_High` (`0x57F440`). I decompiled both.

### 2.1 `DestroyBridgeFromCell_Low` @ `0x574780`

```c
void DestroyBridgeFromCell_Low(short *param_1)
{
    cell = Get_CellClass(param_1);
    overlay = cell.field_0x44;

    bool is_ns =
        (0x4A <= overlay && overlay <= 0x52)     // NS main healthy+damaged
     || (0x5C <= overlay && overlay <= 0x5F)     // NS bridgehead A/B healthy+damaged
     ||  overlay == 0x64;                        // NS destroyed anchor

    if (is_ns) {
        // NS bridge -> walk back along Y (north)
        c_n1 = cell at (x, y - 1);
        if (c_n1.overlay NOT in [0x4A..0x65]) {
            CollapseBridge_EW_Low(x, y + 1);     // we're at NORTH edge -> start one south
            return;
        }
        c_n2 = cell at (x, y - 2);
        if (c_n2.overlay NOT in [0x4A..0x65]) {
            CollapseBridge_EW_Low(x, y);         // 1-south-of-edge -> start here
            return;
        }
        // 2+ south of edge -> walker starts 1 north of us
        local_8 = FUN_00588c60(buf, &one);       // returns coord with neg-y, ≈ (x, y-1)
        CollapseBridge_EW_Low(local_8);
        return;
    }

    bool is_ew =
        (0x53 <= overlay && overlay <= 0x5B)
     || (0x60 <= overlay && overlay <= 0x63)
     ||  overlay == 0x65;

    if (!is_ew) return;                          // overlay out of band -> no-op

    // EW bridge -> walk back along X (west)
    c_w1 = cell at (x - 1, y);
    if (c_w1.overlay NOT in [0x4A..0x65]) {
        CollapseBridge_NS_Low(x + 1, y);
        return;
    }
    c_w2 = cell at (x - 2, y);
    if (c_w2.overlay IS in [0x4A..0x65]) {
        CollapseBridge_NS_Low(x - 1, y);
        return;
    }
    CollapseBridge_NS_Low(x, y);
}
```

Key facts:

- Same direction-detect dispatching as `RepairBridge_Low` (§3.3 of parent
  doc), with **identical sub-range partitioning**:
  - NS: `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}` (= 14 cells)
  - EW: `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}` (= 14 cells)
- Same anchor-selection rule (0/1/2 cells back along the bridge axis).
- Crucially: the **`_NS_*` walker walks along the EW axis**, and vice
  versa — same Phase-2 §12.1 naming convention. So NS-overlay-cell →
  call `CollapseBridge_EW_Low` (the walker named EW walks NS).
- `FUN_00588c60(buf, &one)` returns a coord that is `(x, y - 1)` for the
  NS branch and `(x - 1, y)` for the EW branch — the "one cell back along
  walking axis" coord. Parent doc §12.7 already characterised this
  function as "negates a coord" but the calling pattern here makes the
  precise semantics clearer.

### 2.2 `DestroyBridgeFromCell_High` @ `0x5749C0`

Byte-equivalent compiled twin of `_Low` with high-band constants:

- NS overlay: `[0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}`
- EW overlay: `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}`
- Band check: `[0xCD..0xE8]`
- Calls `CollapseBridge_EW_High` / `CollapseBridge_NS_High`

Same anchor selection, same direction convention.

### 2.3 The destruction walker family

The four `CollapseBridge_*` functions exist in Ghidra (labels already
applied; not decompiled yet — open for a future investigation):

| Address    | Symbol                                |
|------------|---------------------------------------|
| `0x575540` | `MapClass::CollapseBridge_NS_Low`     |
| `0x575220` | `MapClass::CollapseBridge_EW_Low`     |
| `0x575BA0` | `MapClass::CollapseBridge_NS_High`    |
| `0x575870` | `MapClass::CollapseBridge_EW_High`    |

These are the per-cell overlay-mutating walkers that produce the actual
"bridge collapses" visual (overlay → destroyed anchor `0x64/0x65/0xE7/0xE8`,
+0x140 flag changes, radar-dirty propagation, etc.). They are the inverse
of `RepairBridgeWalker_*` (parent doc §12) — same walking convention,
opposite state transitions. Their bodies are still parent-doc open
question §19.2; this report only confirms entry points and call graph.

### 2.4 Where bodies are reached from

Verified call graph (via `get_function_callers`):

```
BombClass::Detonate (0x438720)                  [demo truck on CABHUT]
BuildingClass::Update (0x43FB20)                [C4 timer expiry on CABHUT]
  └─→ DestroyBridge_Low_MapInit  (0x574C20)
       └─→ DestroyBridgeFromCell_Low (0x574780)
            ├─→ CollapseBridge_NS_Low  (0x575540)
            └─→ CollapseBridge_EW_Low  (0x575220)
  └─→ DestroyBridge_High_MapInit (0x574000)
       └─→ DestroyBridgeFromCell_High (0x5749C0)
            ├─→ CollapseBridge_NS_High (0x575BA0)
            └─→ CollapseBridge_EW_High (0x575870)
```

No other callers exist for any function in this subtree (Ghidra
`get_function_callers` returned only these two top-level call sites for
both `DestroyBridge_*_MapInit` functions).

---

## 3. Question 3 — Delayed-C4 vs infantry-enter hut branches

Both branches enter through the same shape (5×5 scan in the caller →
low/high dispatcher), but the call graphs diverge fully at the second
function tier and never recombine.

### 3.1 Side-by-side call graph

```
INFANTRY-ENTER (engineer steps onto BridgeRepairHut cell):
─────────────────────────────────────────────────────────
InfantryClass::PerCellProcess @ 0x519630
  ├─ Gate: mission ∈ {8, 0xB, 0x19}, InfantryType.Engineer (+0xEC3) != 0,
  │        target Building has Type.BridgeRepairHut (+0x16B6) != 0
  ├─ EVA + sound dispatch (parent doc §3.1 step A/B)
  ├─ 5×5 scan (outer, tile-OR-overlay):
  │    tile ∈ [DAT_00abad1c, +0x10) OR overlay ∈ [0x4A..0x65] → low_found
  ├─ if low_found: CALL ProcessBridgeDestruction_Low @ 0x570050
  │                  ├─ 5×5 inner scan (overlay-only, [0x4A..0x65]):
  │                  │    on match → CALL RepairBridge_Low (0x57F200)
  │                  │                  └─ direction-detect → walker:
  │                  │                     RepairBridgeWalker_NS_Low (0x57F6A0)
  │                  │                     RepairBridgeWalker_EW_Low (0x57FBC0)
  │                  └─ on no-match → ramp branch:
  │                     ToggleBridgePavement / SetOverlayAndPropagate /
  │                     FUN_00569760 (pavement walker) /
  │                     ValidateBridgeZones (conditional zones rebuild) /
  │                     recursive ProcessBridgeDestruction_Low(coord ± 2)
  ├─ else:           CALL ProcessBridgeDestruction_High @ 0x573540 (twin)
  ├─ Bridge-repair listener registry (DAT_00a83dec) callback
  ├─ Engineer attached-trigger ProcessCellAction(0x30)
  └─ Engineer.vtable[0xF8]() — Limbo (consume)

DELAYED-C4 (planted C4 timer expires on a BridgeRepairHut):
───────────────────────────────────────────────────────────
BuildingClass::Update @ 0x43FB20 (tail, after generic timer/anim updates)
  ├─ Gate: field_0x6DF != 0 AND timer (field_0x528/+0x530) expired
  ├─ If Type.BridgeRepairHut (+0x16B6) == 0:
  │     CALL this.vtable[0x16C] with C4 warhead (RulesClass+0xFA8)
  │     — self-damage path; NOT bridge-destruction
  ├─ Else (BridgeRepairHut == 1):
  │   ├─ 5×5 scan (outer, tile-OR-overlay): same shape as infantry path
  │   ├─ if low_found: CALL DestroyBridge_Low_MapInit @ 0x574C20
  │   │                  ├─ 5×5 inner scan (overlay-only, [0x4A..0x65]):
  │   │                  │    on match → CALL DestroyBridgeFromCell_Low (0x574780)
  │   │                  │                  └─ direction-detect → walker:
  │   │                  │                     CollapseBridge_NS_Low (0x575540)
  │   │                  │                     CollapseBridge_EW_Low (0x575220)
  │   │                  └─ on no-match → flag-based ramp walk:
  │   │                     starter via cell+0x140 (& 0x500) /
  │   │                     anchor by 0x100/0x80/0x400 sub-branches /
  │   │                     forward walk + IsBridgeRampTile / ApplyDamageToCell /
  │   │                     UpdateAdjacentBridges_High /
  │   │                     Tactical+0xD7C = 1 /
  │   │                     UpdateBridgeZonesHelper (unconditional)
  │   └─ else:         CALL DestroyBridge_High_MapInit @ 0x574000 (twin)
  └─ Clear field_0x6DF and field_0x540

DEMO-TRUCK (BombClass::Detonate on a BridgeRepairHut):
──────────────────────────────────────────────────────
BombClass::Detonate @ 0x438720
  ├─ apply_area_damage (BombWarhead, RulesClass+0xFC8)  ← different warhead than C4!
  ├─ explosion animation
  ├─ if Target.RTTI == 6 (Building) AND Target.Type.BridgeRepairHut != 0:
  │   └─ 5×5 outer scan + dispatch — IDENTICAL to delayed-C4 path from here
  └─ (BombClass::Detonate does NOT touch field_0x6DF — direct dispatch)
```

### 3.2 Where the two branches actually differ

| Layer | Delayed-C4 (BuildingClass::Update) | Infantry-enter (PerCellProcess) |
|-------|-------------------------------------|---------------------------------|
| Trigger | `field_0x6DF == 1` AND timer expired | Mission ∈ {8, 0xB, 0x19} + Engineer + BridgeRepairHut |
| Pre-flight side effects | Clears `field_0x6DF` / `field_0x540` after dispatch | EVA event, RepairBridgeSound, listener registry callback, ProcessCellAction(0x30), engineer Limbo |
| 5×5 outer scan | **Identical** (tile-OR-overlay, no short-circuit, dy outer / dx inner, low-only bool) | **Identical** |
| Second-tier dispatcher | `DestroyBridge_{Low,High}_MapInit` (0x574C20 / 0x574000) | `ProcessBridgeDestruction_{Low,High}` (0x570050 / 0x573540) |
| Inner 5×5 scan | overlay-only, `[0x4A..0x65]` / `[0xCD..0xE8]`, **short-circuit**, dx outer / dy inner | overlay-only, **identical** scan |
| On overlay match | `DestroyBridgeFromCell_{Low,High}` (0x574780 / 0x5749C0) → `CollapseBridge_*` walker family | `RepairBridge_{Low,High}` (0x57F200 / 0x57F440) → `RepairBridgeWalker_*` walker family |
| On overlay miss (ramp path) | Flag-based ramp walk (Step B/C/D of §1.2), `ApplyDamageToCell` per cell | Repair-side ramp walk via `ToggleBridgePavement` / `SetOverlayAndPropagate` / recursive self-call |
| Zone rebuild | `UpdateBridgeZonesHelper` **unconditional** at end of ramp path (no zone-rebuild when overlay-match path is taken; that lives in `CollapseBridge_*`) | `UpdateBridgeZonesHelper` **conditional** on `ValidateBridgeZones` + `bVar1` (main-deck repair actually fired) |
| Adjacency refresh | `UpdateAdjacentBridges_High` only on ramp path | None (no destruction; no neighbor edge-update needed for repair) |
| Visual outcome | Bridge collapses; hut survives at full HP | Bridge repaired; engineer consumed |

The structural symmetry is the point: the outer 5×5 scan is "find any
low-bridge evidence within 5×5 of the hut, else assume high"; both
branches inherit this from a common idiom. The divergence is purely
**what to do with the result** — engineer-enter → repair walker tree;
C4-timer-expiry → collapse walker tree.

### 3.3 Why `field_0x6DF` and the dual-purpose flag are not relevant to this question

The Phase-2 finding that `field_0x6DF` is dual-purpose (C4-plant-pending /
Crewed-survivor cooldown, parent doc §14) lives on the **gate** for the
delayed-C4 branch, not on the dispatcher choice. Both purposes lead to
the same `BuildingClass::Update` timer-expiry block; only the
`Type.BridgeRepairHut` test inside it routes to bridge destruction vs
self-damage. The Crewed-survivor case for a non-CABHUT building hits the
self-damage `vtable[0x16C]` branch; the C4-on-CABHUT case hits the
bridge-destruction branch we just decoded.

### 3.4 Confidence

Per `feedback_research_confidence_axes`:

| Claim | Content | Identity | Binding |
|-------|---------|----------|---------|
| `DestroyBridge_*_MapInit` callers are exactly `BombClass::Detonate` and `BuildingClass::Update` | HIGH (full decompile of both functions verifies dispatch sites at 0x438982/0x44031b/0x44031b) | HIGH (Ghidra labels match Phase-1 verification) | HIGH (`get_function_callers` returned only those two) |
| `ProcessBridgeDestruction_*` callers are exactly `InfantryClass::PerCellProcess` + recursive self | HIGH | HIGH | HIGH (`get_function_callers`) |
| Outer 5×5 scan is byte-equivalent across all three caller sites | HIGH (disassembly of BuildingClass::Update verified directly; PerCellProcess slice verified via `read_memory`; BombClass::Detonate per parent doc §3.7) | HIGH | HIGH |
| Inner 5×5 scan in `DestroyBridge_*_MapInit` is overlay-only, short-circuiting, column-major | HIGH (direct decompile of 0x574C20 / 0x574000) | HIGH | HIGH |
| `DestroyBridgeFromCell_*` is a compiled twin of `RepairBridge_*` with `CollapseBridge_*` walkers | HIGH (side-by-side decompile of 0x574780 and 0x57F200) | HIGH | HIGH (call targets verified via callee names in decompile) |
| `CollapseBridge_*` walkers exist at the cited addresses | HIGH | MEDIUM-HIGH (labels in place; bodies not yet decompiled, so identity-as-destruction-walker rests on `RepairBridge_*`-twinning evidence and the direction-detect dispatcher choosing them) | HIGH (callee resolution in `DestroyBridgeFromCell_*` decompile) |

The remaining open work for a future investigation is documenting the
`CollapseBridge_*` walker bodies themselves (analogous to parent doc
§12's repair walker state table) — that would close the loop on
"destroyed-anchor → ??" overlay state transitions and confirm the
zone-rebuild trigger fires from inside the walker on the overlay-match
fast path.

---

## 4. Implications for the Rust port

This is research only; per CLAUDE.md "REVERSE ENGINEERING RULES" no
implementation follows in this report. Notes for whoever picks up the
parity-port:

### 4.1 What the dispatcher accepts besides `anchor_span_id`

The Rust port's `apply_destroy_hut_to_bridge` (or equivalent) currently
needs only an anchor span identifier. To match gamemd, it should accept:

1. **(case 1, fast path)** A cell-coord whose overlay-grid value is in
   `[0x4A..0x65]` for low / `[0xCD..0xE8]` for high — found by 5×5 sweep
   in column-major order around the hut's center cell. Hand this to the
   equivalent of `DestroyBridgeFromCell_*`.
2. **(case 2, slow path)** A cell-coord whose tile flags identify it as
   a bridge cell (`bridge_layer` bit) or bridgehead (`anchor` bit). If
   the hut center has neither, sweep 8 compass directions up to 3 cells
   out looking for any cell with either bit.
3. **(case 3, anchor refinement)** Depending on which flags are set,
   resolve the actual walker-start anchor:
   - bridge cell + walkable bit → cell itself
   - bridge cell, no walkable bit → linked bridge-object home cell
   - bridgehead only → walk 4 perp cells then offset 2 back
4. **(no-op)** No bridge evidence within 3 cells in any direction → silent
   return, no destruction.

The current `anchor_span_id`-only API covers case 1; cases 2–4 are
currently un-modeled but rarely fire in vanilla maps. The existing
ignored test
`c4_on_cabhut_destroys_bridge_when_upstream_immune_lifted`
([src/sim/world/world_orders_bridge_repair_tests.rs:265](../../ra2-rust-game/src/sim/world/world_orders_bridge_repair_tests.rs#L265))
should be re-titled (per the C4 investigation doc) and assert case 1.

### 4.2 Outer-scan dx/dy iteration order

The outer 5×5 in BuildingClass::Update is **dy outer, dx inner**, but
it does not short-circuit, so the iteration order is parity-irrelevant.
The inner 5×5 in `DestroyBridge_*_MapInit` is **dx outer, dy inner** and
**does** short-circuit. For a Rust port to choose the exact same
"first matching cell" anchor as gamemd in pathological cases (multiple
overlay-in-band cells in the 5×5), the inner scan must use column-major
order. In vanilla maps, this rarely matters — most CABHUT cells have
exactly one overlay-in-band neighbor.

### 4.3 The zone rebuild fires on BOTH paths — corrected 2026-05-18

**Original claim (kept for history):** "the unconditional zone rebuild lives
only on the ramp path; verifying that the short-circuit path also fires it
from inside `CollapseBridge_*` is open work."

**Correction (2026-05-18):** Open work resolved. All four
`CollapseBridge_{NS,EW}_{Low,High}` bodies have been decompiled
(addresses `0x575540`, `0x575220`, `0x575BA0`, `0x575870`) and **every one
of them calls `MapClass::UpdateBridgeZonesHelper()` followed by
`*(g_Tactical + 0xD7C) = 1` at its function tail, unconditionally.** See
[CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md](CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md)
§4.2 and the plate comments now applied to those addresses in `gamemd.exe`.

**Net for the Rust port:** there is no asymmetry to worry about. The zone
rebuild fires once per cascade, on both the overlay-match fast path (from
inside `CollapseBridge_*`) and the ramp-fallback path (from inside
`DestroyBridge_*_MapInit`). A port that fires the rebuild once after the
dispatch — regardless of which path was taken — matches gamemd.

This also closes parent-doc open question §19.2's "where exactly does the
zone rebuild come from on the overlay-match path."

---

## 5. Ghidra annotations applied

Per CLAUDE.md "Only label what you understand with ~90% confidence" — the
following PRE_COMMENTs were applied and the program saved. No renames
(all relevant functions already carry Phase-2 labels):

| Address | Comment summary |
|---------|-----------------|
| `0x574C20` | `DestroyBridge_Low` entry behavior: inner 5×5, fallback walk, anchor sub-branches, tail |
| `0x574000` | Twin of 0x574C20 with high-band constants |
| `0x574780` | `DestroyBridgeFromCell_Low` direction-detect to `CollapseBridge_*_Low` |
| `0x5749C0` | Twin of 0x574780 with high-band constants |

`MapClass::DestroyBridgeFromCell_{Low,High}`,
`MapClass::CollapseBridge_{NS,EW}_{Low,High}`,
`MapClass::IsBridgeRampTile`, and `MapClass::IsLowBridgeEndpointTile`
labels were already applied by Phase 2 work (verified via
`search_functions`) — no rename needed.

The Phase-1 / Phase-2 proposal to drop the `_MapInit` suffix on
`DestroyBridge_{Low,High}_MapInit` was not enacted because there is
already a separate function family at `0x57BAA0` / `0x57CCF0` named
`DestroyBridge_{Low,High}` (the per-cell-damage path — parent doc §13.3).
Dropping the suffix would cause a name collision. Suggested instead:
either keep the misleading `_MapInit` suffix in place with a comment
flagging it, or rename to e.g. `DestroyBridgeFromHut_{Low,High}` to make
the entry-from-hut intent explicit. **Not renamed** — the user should
decide the convention.

---

## 6. Sources

**Ghidra addresses decompiled in this report:**

- `0x574C20` — `DestroyBridge_Low_MapInit` (full)
- `0x574000` — `DestroyBridge_High_MapInit` (full)
- `0x574780` — `DestroyBridgeFromCell_Low` (full, NEW)
- `0x5749C0` — `DestroyBridgeFromCell_High` (full, NEW)
- `0x5746C0` — `IsBridgeRampTile` (full, NEW)
- `0x574600` — `IsLowBridgeEndpointTile` (full, NEW)
- `0x43FB20` — `BuildingClass::Update` (disassembly slice around delayed-C4
  branch verified directly at `0x440221..0x440331`)

**xrefs verified:**

- callers of `0x574000`, `0x574C20` — only `BombClass::Detonate` +
  `BuildingClass::Update`
- callers of `0x570050`, `0x573540` — only `InfantryClass::PerCellProcess`
  + recursive self
- `search_functions("CollapseBridge")` confirmed the four walker addresses

**Memory verified:**

- `read_memory(0x519c80, 160)` — `InfantryClass::PerCellProcess` 5×5 scan
  bytes match the BuildingClass::Update pattern exactly

**Prior docs cross-referenced:**

- [BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md](BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md)
  — §3.1, §3.2, §3.6, §3.7, §11, §13, §14 (parent)
- [C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md](C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md)
  — orthogonal upstream-gate refutation (not load-bearing here)
- [HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md)
  — per-cell warhead-damage tree at `ApplyDamageToCell` / `DestroyBridge_*`
  family (distinct from this entry path)
