# RepairBridgeWalker_*_* Bodies — Per-Cell State Writes

**Status:** Verified from binary (Ghidra MCP, read-only). All four walker
addresses, byte-level write maps, jump-table contents, and caller chains
re-read from gamemd.exe in this session.

## 0. TL;DR

The four `RepairBridgeWalker_{NS,EW}_{Low,High}` functions are the per-cell
write engine for engineer-driven bridge repair. They walk a column or row of
3-cell-wide bridge span and **write only one cell field: `OverlayTypeIndex`
(`+0x44`)**. No direct writes occur to `+0x11E` (damage-state ladder),
`+0x11A` (Height), `+0x11B` (Level), or `+0x140` (Flags) — the only state byte
the walker touches per cell is `+0x44`. The damage→intact transition on
`+0x11E` happens **indirectly** through `CellClass__RecalcAttributes`, which
is called on each of the 3 modified cells after the overlay rewrite.

The walker's per-cell logic for each input overlay is dispatched via a
4-target jump table indexed by a per-walker byte-LUT:
- **case 0 (damaged input)** → write RNG-selected intact overlay in a 4-value range
- **case 1** → normalize half-damaged overlay pair to its base (e.g. {0x5c,0x5d}→0x5c)
- **case 2** → normalize half-damaged overlay pair to its base (e.g. {0x5e,0x5f}→0x5e)
- **case 3** → no-op (overlay is outside this walker's repairable set)

After all cells are written, the walker conditionally calls
`MapClass__UpdateBridgeZonesHelper` (only if any case-0 repair happened) and
`FUN_005868a0` (rect-region helper, probably bridge-rebuild relayer).

## 1. Walker inventory (address + dispatcher + role)

All four are labelled in the Ghidra project (do NOT rename).

| # | Address | Name | Dispatcher | Discriminator |
|---|---------|------|------------|---------------|
| 1 | `0x0057F6A0` | `MapClass__RepairBridgeWalker_NS_Low` | `MapClass__RepairBridge_Low @ 0x0057F200` | overlay `+0x44` ∈ `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}` (NS-low span set) — see §3.3 of `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` |
| 2 | `0x0057FBC0` | `MapClass__RepairBridgeWalker_EW_Low` | `MapClass__RepairBridge_Low @ 0x0057F200` | overlay `+0x44` ∈ `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}` (EW-low span set) |
| 3 | `0x005800D0` | `MapClass__RepairBridgeWalker_NS_High` | `MapClass__RepairBridge_High @ 0x0057F440` | overlay `+0x44` ∈ `[0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}` (NS-high span set) |
| 4 | `0x00580600` | `MapClass__RepairBridgeWalker_EW_High` | `MapClass__RepairBridge_High @ 0x0057F440` | overlay `+0x44` ∈ `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}` (EW-high span set) |

Caller verification (`get_function_callers`):
- `RepairBridgeWalker_NS_Low` and `RepairBridgeWalker_EW_Low` — **single
  caller** `MapClass__RepairBridge_Low @ 0x0057F200`.
- `RepairBridgeWalker_NS_High` and `RepairBridgeWalker_EW_High` — **single
  caller** `MapClass__RepairBridge_High @ 0x0057F440`.

The two dispatchers are themselves called only from
`ProcessBridgeDestruction_Low @ 0x00570050` and
`ProcessBridgeDestruction_High @ 0x00573540` (verified) — and those in turn
are called by `InfantryClass__PerCellProcess @ 0x00519630` (the engineer
walks-into-hut path) plus their own self-recursion.

Walkers do **not** call each other or any other walker; this is **not** the
shared destruction-walker family.

## 2. Per-walker per-cell write map (state-table format)

All four walkers share the same control flow. For each cell in the column/row
of the span (advanced via the inner loop and gated by `FUN_00580B20`/`B70` —
both are simple `0x4A ≤ +0x44 ≤ 0x65` / `0xCD ≤ +0x44 ≤ 0xE8` range checks),
the walker:
1. Reads the **current** cell's `+0x44` (`OverlayTypeIndex`).
2. Computes `EAX = +0x44 − base` where base = `0x4E` (NS_Low), `0x57` (EW_Low),
   `0xD1` (NS_High), `0xDA` (EW_High).
3. Looks up `LUT[EAX]` (per-walker byte table, address below) to select one
   of 4 case targets.
4. Computes the **new** `+0x44` value for the selected case.
5. If new != current, writes new to **three cells**: the walker cell (`this_01`),
   the cell at `y−1` (NS) / `x−1` (EW) (`this_00`), and the cell at `y+1` /
   `x+1` (`this`). This is the **3-wide perpendicular strip** behavior.

### Jump-table + LUT addresses (read directly from binary)

| Walker | Jump table | LUT | LUT length | LUT base offset |
|--------|-----------|-----|------------|------------------|
| NS_Low  | `0x0057FB94` | `0x0057FBA4` | 23 bytes | `+0x44 − 0x4E` |
| EW_Low  | `0x005800A8` | `0x005800B8` | 15 bytes | `+0x44 − 0x57` |
| NS_High | `0x005805D0` | `0x005805E0` | 23 bytes | `+0x44 − 0xD1` |
| EW_High | `0x00580AF4` | `0x00580B04` | 15 bytes | `+0x44 − 0xDA` |

Each LUT entry is one of `{0,1,2,3}` selecting the case target.

### Case action per walker

| Walker | case 0 (damaged) | case 1 (half-dmg pair) | case 2 (half-dmg pair) | case 3 |
|--------|-----------------|----------------------|----------------------|--------|
| NS_Low  | `+0x44 = 0x4A + RNG(0..3)` (→ `0x4A..0x4D` intact) | `+0x44 = 0x5C` | `+0x44 = 0x5E` | no-op |
| EW_Low  | `+0x44 = 0x53 + RNG(0..3)` (→ `0x53..0x56` intact) | `+0x44 = 0x60` | `+0x44 = 0x62` | no-op |
| NS_High | `+0x44 = 0xCD + RNG(0..3)` (→ `0xCD..0xD0` intact) | `+0x44 = 0xDF` | `+0x44 = 0xE1` | no-op |
| EW_High | `+0x44 = 0xD6 + RNG(0..3)` (→ `0xD6..0xD9` intact) | `+0x44 = 0xE3` | `+0x44 = 0xE5` | no-op |

The RNG is `FUN_00598030(0, 3)` — verified to be a rejection-loop around
`Random__Next()` + `Math__ftol`, returning a value in `[0..3]`.

### Per-walker LUT decode

NS_Low LUT bytes at `0x0057FBA4`:
`00 00 00 00 00  03 03 03 03 03 03 03 03 03  01 01 02 02  03 03 03 03  00`

- `0x4E..0x52` → 0 (damage → RNG repair to `0x4A..0x4D`)
- `0x53..0x5B` → 3 (EW span — no-op)
- `0x5C..0x5D` → 1 (normalize to `0x5C`)
- `0x5E..0x5F` → 2 (normalize to `0x5E`)
- `0x60..0x63` → 3 (EW half-damaged — no-op)
- `0x64`      → 0 (NS full-damaged single → RNG repair)

EW_Low LUT bytes at `0x005800B8`:
`00 00 00 00 00  03 03 03 03  01 01 02 02  03  00`

- `0x57..0x5B` → 0 (damage → RNG repair to `0x53..0x56`)
- `0x5C..0x5F` → 3 (NS half-damaged — no-op)
- `0x60..0x61` → 1 (normalize to `0x60`)
- `0x62..0x63` → 2 (normalize to `0x62`)
- `0x64`      → 3 (NS full-damaged — no-op)
- `0x65`      → 0 (EW full-damaged → RNG repair)

NS_High LUT bytes at `0x005805E0`:
`00 00 00 00 00  03 03 03 03 03 03 03 03 03  01 01 02 02  03 03 03 03  00`

- `0xD1..0xD5` → 0 (damage → RNG repair to `0xCD..0xD0`)
- `0xD6..0xDE` → 3 (EW span — no-op)
- `0xDF..0xE0` → 1 (normalize to `0xDF`)
- `0xE1..0xE2` → 2 (normalize to `0xE1`)
- `0xE3..0xE6` → 3 (EW half-damaged — no-op)
- `0xE7`      → 0 (NS full-damaged single → RNG repair)

EW_High LUT bytes at `0x00580B04`:
`00 00 00 00 00  03 03 03 03  01 01 02 02  03  00`

- `0xDA..0xDE` → 0 (damage → RNG repair to `0xD6..0xD9`)
- `0xDF..0xE2` → 3 (NS half-damaged — no-op)
- `0xE3..0xE4` → 1 (normalize to `0xE3`)
- `0xE5..0xE6` → 2 (normalize to `0xE5`)
- `0xE7`      → 3 (NS full-damaged — no-op)
- `0xE8`      → 0 (EW full-damaged → RNG repair)

### Disassembly anchors (the +0x44 writes themselves)

All four walkers write `MOV dword ptr [<cellreg> + 0x44], EAX` at three sites
per iteration. Exact addresses:

| Walker | Walker-cell write | Side-cell write A | Side-cell write B |
|--------|------------------|-------------------|-------------------|
| NS_Low  | `0x0057F995` | `0x0057F998` | `0x0057F99B` |
| EW_Low  | `0x0057FEB2` | `0x0057FEBC` | `0x0057FEBF` |
| NS_High | `0x005803CE` | `0x005803D1` | `0x005803D4` |
| EW_High | `0x005808FB` | `0x00580905` | `0x00580908` |

These are the **only** `MOV […+offset], …` writes in the walker bodies. **No
`MOV byte ptr [<reg>+0x11E]`, `+0x11A`, `+0x11B`, or any `[<reg>+0x140]
AND/OR/XOR` instructions are present in any of the four functions.** Verified
by reading the full disassembly of each (`disassemble_function` for all four
addresses; grep for `0x11E`, `0x11A`, `0x11B`, `0x140` returns zero hits).

After the writes the walker calls, for each of the 3 modified cells:
- `CellClass__RecalcAttributes(cell)` at `0x0047D2B0` — which **does** read
  the new `+0x44`, derive `LandType`/`SlopeIndex`, and **may set
  `field_0x11e = 0`** under one specific condition: when `SlopeIndex != 0`
  AND the new overlay type has flag at `OverlayTypeClass+0x2a9 != 0` (the
  bridge-overlay-clears-overlay-on-slope branch). This is the indirect path
  by which damage-state `+0x11E` is reset to 0 on repair.
- `FUN_00487a10(0)` — flag-setter on cell (does not touch `+0x11E`; appears
  to be a draw-state helper; out of scope to fully decompile).

After all iterations:
- If `bVar1` (any case-0 repair fired) → call `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`.
- If the damaged-cell rect accumulator (`local_ec`, `local_e8`) is non-empty
  → call `FUN_005868a0` with the rect (a region-iterator that walks all
  cells in the rect and calls a member function — likely re-layers objects
  on the now-repaired span).

## 3. Neighbor-step pattern

The walkers do **not** use the compass-direction table `g_DirectionOffsets @
0x0089F688`. Instead they hardcode axis-aligned neighbors via direct
`MapCoord` shorts manipulation:

- **NS walkers**: Y is incremented (`local_fc + 1` after each iteration; pre-pass
  steps Y back via `local_fc + −1` loop until exiting the overlay band).
  Side cells are at `(x, y−1)` and `(x, y+1)`.
  → Wait — re-reading: the **iteration axis** is the high short (`sStack_fa`) for
  NS; let me restate.
  - NS_Low/High: pre-pass `do { local_fc -= 1; if outside-range break } while
    in-range`. Outer iteration: `local_fc += 1` per turn (so increases X
    along the south-walker direction). Side cells at `(x, y−1)` and `(x, y+1)`.
  - EW_Low/High: pre-pass decrements `local_fc.hi` (Y). Outer iteration:
    `local_fc.hi += 1`. Side cells at `(x−1, y)` and `(x+1, y)`.

(MapCoord is encoded as `short[2] = {X, Y}` in the walker locals; the
`CONCAT22` patterns match the decompile. The naming "NS walker iterates Y"
vs "EW walker iterates X" matches the §3.3/3.4 description in
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`.)

The 3-wide perpendicular strip is written on every iteration:
- NS walker: rewrites `(x, y)`, `(x−1, y)`, `(x+1, y)` → wait, again the
  walker decompile shows `local_d8 = CONCAT22(sStack_fa + −1, local_fc)` —
  the **first** word is X (`local_fc`), the **second** is Y (`sStack_fa`).
  So `(x, y−1)` is the side cell for NS. Per-iteration write set: walker cell
  + cell at `y−1` + cell at `y+1`.

Actually re-reading carefully: the NS walker `local_d8 = CONCAT22(sStack_fa
+ -1, local_fc)` packs `(low=local_fc, high=sStack_fa−1)`. If MapCoord layout
is `(X, Y)` with X in low and Y in high, that's `(x, y−1)`. The walker is then
iterating across columns by `local_fc += 1` (incrementing X), which is an
**EW**-direction iteration with **NS** side spread. So the function name
`NS_Low` refers to the **span axis** (the bridge span runs N-S, so the strip
of intact pieces lies along that axis and the walker steps **across** the
span). This matches the parent doc's note that walker labels refer to the
bridge axis, not the iteration axis.

(This naming ambiguity is documented but not resolved further here — it is
not load-bearing for the per-cell write map.)

## 4. Recursion / cross-walker calls

- No walker self-recurses (no call to its own address within its body).
- No walker calls a sibling walker (no `CALL 0x57F6A0/0x57FBC0/0x5800D0/0x580600`
  appears in any of the four bodies — verified by full disassembly grep).
- No walker calls a `RepairBridge_*` dispatcher.

The only callees (verified by inspecting the disassembly) are:
- `MapClass__Get_CellClass @ 0x005657A0` (cell lookup)
- `FUN_00598030` (RNG-bounded; 0x00598030 in NS_Low / 0x00598030 in NS_High
  etc.) at the case-0 site
- `CellClass__RecalcAttributes @ 0x0047D2B0` ×3 (post-write)
- `FUN_00487a10` ×3 (draw/redraw helper)
- `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` (conditional; post-loop)
- `FUN_005868a0` (conditional; post-loop)
- `FUN_00580B20` (NS_Low/EW_Low) / `FUN_00580B70` (NS_High/EW_High) — loop
  guard, returns 1 iff `0x4A ≤ +0x44 ≤ 0x65` (low) / `0xCD ≤ +0x44 ≤ 0xE8`
  (high).
- Various geometry helpers (`FUN_0047fde0`, `FUN_0047fb90`, `FUN_00487f40`,
  `FUN_0045a130`, `TacticalClass__DirtyScreenRect`, `RadarClass__MarkTerrainDirty`)
  — out of scope, no bridge-state writes.

## 5. Flag-bit writes at +0x140

**None.** No `+0x140` read, write, OR, AND, or XOR appears in any of the
four walker bodies. Verified by full disassembly inspection (`disassemble_function`
on all four addresses; no operand `[..+0x140]` found).

The only `+0x140` interaction in the area is inside `RecalcAttributes` itself,
which can OR `0x10000` into `+0x140` of neighboring cells when an attached
animation table is iterated (`*(uint *)(iVar8 + 0x140) | 0x10000` at line
`0x0047D...` in the helper) — this is a side-effect of `RecalcAttributes`
**on neighbor cells**, not on the repaired cells, and only fires when the
overlay type has an attached animation-coord-list. Not directly bridge-repair
behavior.

**Bits 0x80, 0x100, 0x200, 0x400, 0x800 on `+0x140`: not touched by walkers.**

## 6. Active in YR — verdict + caller-chain evidence

**Verdict: Active in YR (all 4 walkers).**

Evidence — call chain (verified bottom-up):

```
InfantryClass__PerCellProcess @ 0x00519630   ← engineer steps onto cell
   └─→ ProcessBridgeDestruction_Low  @ 0x00570050    [or _High @ 0x00573540]
         └─→ MapClass__RepairBridge_Low  @ 0x0057F200   [or _High @ 0x0057F440]
               ├─→ RepairBridgeWalker_NS_Low  @ 0x0057F6A0
               └─→ RepairBridgeWalker_EW_Low  @ 0x0057FBC0
                   (NS_High @ 0x005800D0 / EW_High @ 0x00580600 for high-bridge variant)
```

`InfantryClass::PerCellProcess` is the standard per-tick infantry occupancy
hook, invoked every cell-enter event for every infantry. Engineer-on-hut
detection (gated by `BridgeRepairHut=yes` rule per the parent doc §3.6 and
the `[General]` parse path) is YR-active by default. No `SpecialFlags` gate
applies to these four functions or their dispatchers.

No TS-only branches inside the walker bodies — every case in the LUT is
reachable from YR-live overlay states during a normal damaged-bridge state.

## 7. Open Questions (deferred)

1. **Damage-state `+0x11E` indirect path.** Walker does not write `+0x11E`
   directly. The damage→intact transition on `+0x11E` must come from
   `RecalcAttributes`. There is exactly one site in `RecalcAttributes` that
   sets `field_0x11e = 0`: when `SlopeIndex != 0` AND
   `OverlayTypeClass[+0x2a9] != 0`. Whether the YR bridge overlays
   (`[0x4A..0x65]`, `[0xCD..0xE8]`) carry `+0x2a9 != 0` on their
   `OverlayTypeClass` is not verified here — this is the most-load-bearing
   open question for the Rust port's regression behavior. Verify by reading
   `g_OverlayTypeClass_Array[0x4A].+0x2a9` (and a few neighbours) in a
   separate investigation.

2. **`FUN_005868a0` semantics.** Called after the walk if a damaged-rect
   accumulator is non-empty. Appears to be a region-iterator that invokes a
   member function (vtable slot at PTR_FUN_007e3890) on each cell in the
   rect. Likely a per-object relayer for things sitting on the now-repaired
   span (so they re-attach to the OnBridge layer). Not decompiled in detail
   here.

3. **`FUN_00487a10(0)` semantics.** Called 3× per iteration on the modified
   cells. Probably a draw/dirty helper, not a state-write. Not decompiled
   in detail.

4. **NS/EW span-axis naming.** The walker iterates **across** the bridge,
   not along it (NS walker iterates X; EW walker iterates Y), with the
   3-wide perpendicular strip lying along the **iteration** axis. The
   walker name refers to the **bridge span** axis. This matches the parent
   doc but is worth a single sentence somewhere if it isn't already.

## 8. Sources (addresses decompiled, memory reads, strings searched)

Functions decompiled (Ghidra MCP `decompile_function`):
- `0x0057F200` — `MapClass__RepairBridge_Low` (dispatcher; for callee verification)
- `0x0057F440` — `MapClass__RepairBridge_High`
- `0x0057F6A0` — `MapClass__RepairBridgeWalker_NS_Low`
- `0x0057FBC0` — `MapClass__RepairBridgeWalker_EW_Low`
- `0x005800D0` — `MapClass__RepairBridgeWalker_NS_High`
- `0x00580600` — `MapClass__RepairBridgeWalker_EW_High`
- `0x00580B20` — `FUN_00580B20` (NS_Low/EW_Low loop guard; verified
  range-check on `+0x44`)
- `0x00580B70` — `FUN_00580B70` (NS_High/EW_High loop guard)
- `0x00598030` — `FUN_00598030` (rejection-loop RNG; bound 3 → returns 0..3)
- `0x005868A0` — `FUN_005868A0` (region rect iterator; out-of-scope)
- `0x0047D2B0` — `CellClass__RecalcAttributes` (for indirect `+0x11E` path)

Functions disassembled (`disassemble_function`) for byte-level write audit:
- `0x0057F6A0` — full NS_Low body; only `+0x44` writes at `0x0057F995/8/B`
- `0x0057FBC0` — full EW_Low body; only `+0x44` writes at `0x0057FEB2/BC/BF`
- `0x005800D0` — full NS_High body; only `+0x44` writes at `0x005803CE/D1/D4`
- `0x00580600` — full EW_High body; only `+0x44` writes at `0x005808FB/905/908`

Memory reads (`read_memory`) for jump tables + LUTs:
- `0x0057FB94` (16 bytes) — NS_Low 4-entry jump table
- `0x0057FBA4` (24 bytes) — NS_Low 23-byte LUT
- `0x005800A8` (32 bytes) — EW_Low jump table + LUT
- `0x005800B8` (16 bytes) — EW_Low 15-byte LUT (re-read for cleanliness)
- `0x005805D0` (48 bytes) — NS_High jump table + LUT
- `0x00580AF4` (32 bytes) — EW_High jump table + LUT

Caller-chain verifications (`get_function_callers`):
- `0x0057F6A0` → only `0x0057F200`
- `0x0057FBC0` → only `0x0057F200`
- `0x005800D0` → only `0x0057F440`
- `0x00580600` → only `0x0057F440`
- `0x0057F200` → only `0x00570050`
- `0x0057F440` → only `0x00573540`
- `0x00570050` → `InfantryClass__PerCellProcess @ 0x00519630` + self
- `0x00573540` → `InfantryClass__PerCellProcess @ 0x00519630` + self

Cross-references read:
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3.3, §3.4, §3.5, §7
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.1 (the
  `UpdateRamp_*_High` family — confirmed disjoint from the walker family;
  the walkers act on `+0x44`, the ramp helpers act on `+0x11E`, and they
  serve opposite paths)

No Ghidra mutations performed. Read-only session.
