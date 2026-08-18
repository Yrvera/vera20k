# CellClass.Flags bit 0x400 — Semantic Resolution

**Date:** 2026-05-18
**Subject:** `CellClass.Flags` (offset +0x140) bit `0x400` (= "bit 10") — definitive semantic, derived from live Ghidra read of writers and readers in `gamemd.exe`.
**Active in YR:** **Yes.** Writer call sites are in the standard YR load/repair/destroy paths (`OverlayClass::Mark` for bridge overlay placement, `MapClass::Resize` for map load, `ProcessBridgeDamageStateMachine_*`, `UpdateBridgeEdgeTiles_*`, `UpdateRamp_*_Collapse*`). Readers are in `DestroyBridge_*_OnHutDeath` (C4-timer + demo-truck on `BridgeRepairHut`) and `UpdateAdjacentBridges_High`. Hut-driven destruction is standard YR multiplayer gameplay.

## TL;DR — the correct semantic

> **Bit `0x400` is the "bridge body cell, destroyed-state" marker. It occupies the same cells that bit `0x100` ("bridge body cell, alive-state") occupies, but the two are mutually exclusive — `0x100` is set when the segment is alive, `0x400` is set when the segment is collapsed. Readers test `(flags & 0x500) != 0` to mean "this cell is a bridge body in either state," then branch on which of `0x100` vs `0x400` is set to discriminate alive vs destroyed.**

Plain English: "this cell is part of a bridge segment, and right now that segment is destroyed."

## Which existing doc is right?

| Doc | Existing label | Verdict |
|-----|----------------|---------|
| `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` §2 | "bridge destroyed marker" — `SetBridgeDirection` sets when `state.byte0==0` | **CORRECT.** This matches the writer evidence exactly. Recommend keeping this entry. |
| `BRIDGE_SYSTEM.md` line 36 | "Bridge rail/guard post" | **WRONG.** No rendering or rail/guard-post reader was found. The label appears to be inferred or copied from an unrelated source. Recommend amending to "bridge body cell, destroyed state (mutually exclusive with bit 0x100)". |
| `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` §3.4 | "unknown / not pathfinding-relevant in our scope" | **DEFENSIBLE (scoped).** Correct that it is not read in pathfinding entry-tests. But it IS read in bridge destruction propagation logic. Recommend amending to "destroyed-state marker on bridge body cells; not pathfinding-relevant in entry tests, but read by `DestroyBridge_*` and `UpdateAdjacentBridges_*` to identify already-collapsed segments and bridge orientation/anchor follow-through". |

## Writer evidence — `CellClass::SetBridgeDirection_NESW` (0x47E040) and `_NWSE` (0x47E470)

Both writer functions are byte-identical (verified by prior plate comment). They write four bridge-axis cells: anchor (`param_1`), forward-1, forward-2, forward-3, and opposite (`param_2-4`). Bit `0x400` is written via:

```
cVar14 = (char)param_3;                         // low byte of "alive" param
param_3 = (uint)(cVar14 == '\0') << 10;         // bit 10 == 0x400 if destroyed
uVar9  = original_param_3 & 1;                  // "alive" bit, drives 0x80/0x100/0x200/0x1000/0x10000
```

| Cell | Mask preserving | Bits written when **alive** (cVar14!=0) | Bits written when **destroyed** (cVar14==0) | Bit 0x400 written? |
|------|-----------------|-----------------------------------------|---------------------------------------------|--------------------|
| Anchor (`param_1`)            | `0xFFFEE07F` | 0x80, 0x100, 0x200, 0x1000, 0x10000, [0x800 if dir==0] | **0x400**, [0x800 if dir==0] | **Yes** (via `param_3` term) |
| Forward-1 (`this` after step1) | `0xFFFEE8FF` then AND `0xFFFFF7FF` | 0x100, 0x200, 0x1000, 0x10000 | **0x400** | **Yes** |
| Forward-2 (`this` after step2) | `0xFFFEE8FF` then AND `0xFFFFF7FF` | 0x100, 0x1000, 0x10000 | **0x400** | **Yes** |
| Forward-3 (`this` after step3) | `0xFFFFEFFF` | 0x1000 | (none — bit 0x400 untouched) | **No** (preserved) |
| Opposite (param_2-4 step)     | `0xFFFEE7FF` | 0x100, 0x200, 0x1000, 0x10000, [0x800 if dir==0] | **0x400**, [0x800 if dir==0] | **Yes** |
| param_2==6 special tail        | `0xFFFEFFFF` | 0x10000 | (none) | **No** |

The masks (e.g., `0xFFFEE07F` = clears bits 7,8,9,10,11,12,16) explicitly clear bit 10 before re-writing it, so the alive/destroyed encoding is a hard set-or-clear, never an OR.

Writers' call sites (xrefs to 0x47E040 / 0x47E470):
- `MapClass::Resize` 0x567078 / 0x56706C — map load
- `MapClass::UpdateRamp_{NS,EW}_Collapse{A,B}_{High,Low}` — collapse animation entry
- `MapClass::UpdateBridgeEdgeTiles_{High,Low}` — edge tile recompute
- `ProcessBridgeDamageStateMachine_{High,Low}` — damage state transitions
- `OverlayClass::Mark` 0x5FC5FE/0x5FC60A/0x5FC62C — bridge overlay placement (called by the editor and at map load)

All callers pass `param_3 = 0` for the destroyed/collapse cases, which sets bit `0x400` on the body cells and clears the alive markers.

## Reader evidence — three live readers, all bridge-system

### Reader 1 — `MapClass::DestroyBridge_High_OnHutDeath` (0x574000)

Hex at 0x5742E4: `F7 81 40 01 00 00 00 04 00 00` = `TEST DWORD PTR [ECX+0x140], 0x400`.

Decompiled context (post-anchor-locate, after the `& 0x500` discrimination):

```c
uVar10 = *(uint *)(puVar6 + 0x140);
if (((uVar10 & 0x100) == 0) && ((uVar10 & 0x400) == 0)) {
    return;                              // neither alive nor destroyed bridge body -> no-op
}
if ((uVar10 & 0x100) == 0) {             // not alive -> must be destroyed (0x400)
    // walk perpendicular up to 4 cells; for each, TEST [+0x140], 0x400
    // (locating the destroyed segment's run to choose damage origin)
    ...
    if ((*(uint *)(puVar3 + 0x140) & 0x400) == 0) break;
    ...
}
```

### Reader 2 — `MapClass::DestroyBridge_Low_OnHutDeath` (0x574C20)

Hex at 0x574F00: identical to reader 1: `F7 81 40 01 00 00 00 04 00 00`.

Body is byte-identical structurally (compiled twin of reader 1 with low-bridge tile bands). Same `& 0x500` precheck, same `(& 0x100 == 0) && (& 0x400 == 0) → return`, same perpendicular walk testing `& 0x400`.

### Reader 3 — `MapClass::UpdateAdjacentBridges_High` (0x576770)

Decompiled (no direct hex hit because the inline `TEST` is fused into the load+AND idiom):

```c
if ((*(uint *)(puVar6 + 0x140) & 0x500) != 0) break;     // bridge body found in EITHER state
...
uVar8 = *(uint *)(puVar6 + 0x140);
if (((uVar8 & 0x100) == 0) && ((uVar8 & 0x400) == 0)) {
    return;                                              // neither -> bail
}
if ((uVar8 & 0x100) == 0) {                              // destroyed bridge -> walk perpendicular
    ...
    if ((*(uint *)(puVar9 + 0x140) & 0x400) == 0) break; // searching for end of destroyed run
    ...
}
```

### False positives ruled out
- **0x53AFEA** in `PsychicDominator::Process`: `A9 00 04 00 00` = `TEST EAX, 0x400` — but EAX here was loaded from `DAT_00A9FAC0`, the PD state-machine global, **not** `cell+0x140`. **Discard.**
- **0x56615C** and **0x56696E** in `MapClass::Resize`: `25 00 04 00 00` = `AND EAX, 0x400` inside the per-bit XOR-mask cell-flag copy block. These are bit-by-bit propagation **writes** during map-resize (copying flags from a temp buffer into target cells), not readers. **Discard.**

## Summary table

| Aspect | Value |
|--------|-------|
| Bit | `0x400` (bit 10) at `CellClass+0x140` |
| Semantic | "Bridge body cell, destroyed state" |
| Mutually exclusive with | `0x100` ("Bridge body cell, alive state") |
| Set by | `SetBridgeDirection_{NESW,NWSE}` when param_3 byte == 0 (collapse case), on anchor + 3 axis-neighbor cells |
| Cleared by | Same writer when param_3 byte != 0 (alive case) |
| Read by | `DestroyBridge_High_OnHutDeath` (0x5742E4, +inline), `DestroyBridge_Low_OnHutDeath` (0x574F00, +inline), `UpdateAdjacentBridges_High` (inline) |
| Read pattern | First `(flags & 0x500) != 0` to find any bridge body cell; then `(flags & 0x100) == 0` discriminates "alive vs destroyed"; if destroyed (0x400 set, 0x100 clear), walk perpendicular for up to 4 cells looking for cells with `& 0x400` set, to locate the destroyed-segment run. |
| Player-visible effect | Yes — drives hut-destruction propagation logic. If wrong, C4/demo-truck on `BridgeRepairHut` over an already-partially-destroyed bridge produces wrong damage/cell selection (or no-ops). |
| Active in YR | **Yes** — all callers are in standard YR damage/repair/load paths. |
| Rendering reader? | **None found.** Hypotheses about "rail/guard post visual" in `BRIDGE_SYSTEM.md` are unsupported by the binary in the scope examined. |

## Recommended doc amendments

1. **`BRIDGE_SYSTEM.md` line 36** — change "Bridge rail/guard post" to "Bridge body cell, destroyed state (mutually exclusive with bit 0x100 alive marker)".
2. **`BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` §3.4** — keep "not pathfinding-relevant" disclaimer but add "set by `SetBridgeDirection` when bridge collapses; read by `DestroyBridge_*_OnHutDeath` and `UpdateAdjacentBridges_High` to identify already-destroyed segments. Not relevant to pathfinding entry tests, which gate on bit 0x80 (anchor) and bit 0x100 (alive body)".
3. **`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` §2** — keep "bridge destroyed marker" — accurate.

## Confidence

- Writer encoding (anchor + neighbors, mask values, destroyed vs alive set/clear): **HIGH** — read from decompilation of both `_NESW` (0x47E040) and `_NWSE` (0x47E470), masks confirmed via direct memory read.
- Reader bytes: **HIGH** — confirmed via raw memory read at 0x5742E4 and 0x574F00 (`F7 81 40 01 00 00 00 04 00 00`) and via decompilation of all three readers.
- Mutual-exclusivity with bit 0x100: **HIGH** — single writer sets one OR the other based on `cVar14`, never both; readers consistently treat them as the two states of one logical "is bridge body" flag.
- No rendering reader exists: **MEDIUM-HIGH** — I exhaustively searched for `TEST/AND ..., 0x400` patterns against `[reg+0x140]` and `[reg]` after a `MOV r32, [reg+0x140]`, decompiled all `Bridge*`-named functions that could plausibly read it. No render-path xref was found. Possible miss: an obfuscated 16-bit `66 F7` test or a `BT` instruction. None of the patterns I scanned for matched outside the 3 known sites.
- Active-in-YR classification: **HIGH** — all callers traced to live YR paths (hut destruction by C4/demo-truck, damage state machine, map load, overlay editor placement).
