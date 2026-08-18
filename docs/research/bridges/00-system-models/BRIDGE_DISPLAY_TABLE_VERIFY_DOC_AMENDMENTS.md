# BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md — Verification Amendments

**Audit date:** 2026-05-18
**Audited doc:** `ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
**Method:** Read-only Ghidra MCP decompilation against live `gamemd.exe`.
**Cross-references:** `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md` (slot-2 verdict).

---

## Tally

- **VERIFIED:** 18 load-bearing claims
- **WRONG — must amend:** 2 (one primary, one cascading wording)
- **UNVERIFIABLE — flag for user:** 0

---

## WRONG — must amend

### W1 — §10.3 Open Question #3: `cell+0x11A` Phase 2B+2C "damage_state_1" claim

**Doc location (lines 803–804):**
> "**`cell+0x11A` semantics.** Phase 1C says 'sub_tile (icon idx)'; Phase 2B+2C says
> `UpdateAdjacentBridges_High` reads it as 'damage_state_1'. One of these is misreading
> the disassembly. Phase 1C's read (consumed by `TMP_TileBlitter` as sub_tile_idx) is
> more likely correct. Re-verify in `UpdateAdjacentBridges_High` if this matters for
> the orchestrator port."

**Doc location (lines 522–523, §4 field-map note):**
> "Important: there is field-offset disagreement between the Phase 1 and Phase 2
> reports about whether `cell+0x11A` is 'sub_tile' or 'damage_state_1'. Phase 1C's
> direct decompilation of `DrawOverlay_Body` (consumes `+0x11E`) and
> `CellOverlay_TileDraw` (consumes `+0x11A` as sub_tile passed to `TMP_TileBlitter`)
> is authoritative. The Phase 2 report's claim that `UpdateAdjacentBridges_High` reads
> `+0x11A` as 'damage_state_1' appears to be a misreading — re-verify if precision is
> needed (see §10)."

**Binary evidence (decompiled `MapClass__UpdateAdjacentBridges_High @ 0x00576770`):**
The function reads `puVar6[0x11a]` and compares against character literals `'\b'` (8),
`'\x05'` (5), `'\f'` (12), and `'\a'` (7). The `_High` callee `UpdateBridgeEdgeTiles_High
@ 0x00576200` (cited inside the same chain) compares the same byte to `'\x02'` and
`'\x04'`. The damage-state machine spans 0..17 and lives at `+0x11E` (verified by
`DrawOverlay_Body @ 0x47F6A0` at `uVar7 = (uint)*(byte *)(param_1 + 0x11e)`). The
constants 2, 4, 5, 7, 8, 12 are sub-tile slot indices within the bridge IsoTileType.

**Corrected wording (replace §10.3 OQ#3 entirely):**
> 3. ~~**`cell+0x11A` semantics.**~~ **RESOLVED 2026-05-18** (see
>    `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md`). `cell+0x11A` is the per-cell
>    IsoTileType sub-tile (icon) index. Phase 1C's "sub_tile (icon idx)" label is
>    correct. The Phase 2B+2C "damage_state_1" interpretation is **wrong**:
>    `UpdateAdjacentBridges_High @ 0x576770` compares `puVar6[0x11A]` to literal
>    sub-tile slot values {5, 7, 8, 12}, and `UpdateBridgeEdgeTiles_High @ 0x576200`
>    compares it to {2, 4}. Those are sub-tile slot numbers within the bridge
>    IsoTileType, not damage states. The damage state byte lives at `cell+0x11E`
>    and spans 0..17.

**Corrected wording for the §4 cross-reference note (lines 522–523):**
> "Field-offset reconciliation: `cell+0x11A` is the per-cell IsoTileType sub-tile
> (icon) index — consumed by `TMP_TileBlitter` (via `CellOverlay_TileDraw`),
> `TMP_ReadSlopeType`, `FUN_005471F0` (pavement bit check), and the bridge-rim
> sub-tile matchers in `UpdateAdjacentBridges_High` / `UpdateBridgeEdgeTiles_High`.
> Phase 2B+2C's transient 'damage_state_1' relabeling was a misreading; the damage
> state byte is at `cell+0x11E`. See `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md`."

### W2 — Cascading wording inside §7.1 (`UpdateAdjacentBridges_High`)

**Doc location (line 599):**
> "`if ((normalized == DAT_00abc2b4 || normalized == DAT_00aa1130) && cell+0x11A == 8)`"

Plus lines 601, 603, 605 — pattern-match block. These lines are factually correct
(they compare `cell+0x11A` to 5/7/8/12). However, §10.3 OQ#3 still flagged this read
as ambiguous. With W1 resolved, no rewording is strictly required here, but a
one-line clarifying note above the snippet would prevent re-litigation:

**Suggested insertion (after line 595):**
> "Note: `cell+0x11A` here is the IsoTileType sub-tile index (icon idx), not a
> damage state. The literal values 5/7/8/12 are sub-tile slot numbers within the
> bridge IsoTileType. See `CELL_0x11A_POLARITY_RECONCILE_GHIDRA_REPORT.md`."

---

## VERIFIED — load-bearing claims spot-checked

1. **§3.1** — `TacticalClass::Draw @ 0x6D3D10` — function exists, body 0x6D3D10–0x6D4B4A.
2. **§2.2 / §3.2** — `Tactical_layer_terrain_shadows @ 0x6D2DE0` — verified.
3. **§2.2 / §3.3** — `Tactical_layer_smudges @ 0x6D3290` — verified.
4. **§2.2 / §3.4** — `Tactical_layer_overlays @ 0x6D3040` — verified.
5. **§3.2** — `CellOverlay_TileDraw @ 0x480350` reads `*(undefined1 *)(param_1 + 0x11a)`
   and passes it as `uVar1` (sub_tile) to `TMP_TileBlitter`. Confirmed by decompilation.
   This is Phase 1C's authoritative read.
6. **§3.3.1** — `DrawOverlay_Body @ 0x47F6A0` — Latin-square branch fires only when
   `*(byte *)(param_1 + 0x11e) == 0 || == 9`. Confirmed by decompilation.
7. **§3.3.1** — Latin square indexed via `((cell+0x26 & 3) << 2) | (cell+0x24 & 3)`.
   Confirmed; Ghidra symbol is `g_OverlayVarietyLatinSquare`.
8. **§3.3.2** — `DrawOverlay_Shadow @ 0x47F510` HIGH-bridge shift `iStack_10 += -0xF`
   (x -= 15), `iStack_C += 7`, gated on `(cell+0x140 & 0x80) && 8 < state < 0x12` —
   verified literal-for-literal.
9. **§5** — Latin square at `0x0081CC30` is `g_OverlayVarietyLatinSquare`. Ghidra
   symbol resolves; the doc's `DAT_0081CC30` address matches.
10. **§3.4.1** — `FUN_00547230 @ 0x547230` (railing emit) — function exists.
11. **§3.4.2** — `FUN_005471F0 @ 0x5471F0` (pavement bit pre-check) — function exists,
    body 0x5471F0–0x547225.
12. **§7.1** — `UpdateAdjacentBridges_High @ 0x576770` — function exists, body
    0x576770–0x576B99. 8-direction walk + `(flags & 0x500)` break is verified.
13. **§7.2** — `UpdateBridgeEdgeTiles_High @ 0x576200` — function exists, body
    0x576200–0x576764.
14. **§2.3** — Name searches confirm **only `_Low` variants** exist:
    `MapClass__SelectBridgeTileVariant_Low @ 0x57ACF0`,
    `MapClass__UpdateBridgeTile_Low @ 0x57A430`,
    `MapClass__SelectDestroyedBridgeTile_Low @ 0x579620`. No `_High` symbols.
15. **§2.5** — `FUN_004863D0 @ 0x4863D0` — function exists, body 0x4863D0–0x4865AC.
16. **§2.7** — `CellClass__HasBridgeOverlay @ 0x4865D0` — function exists, body
    0x4865D0–0x486641 (Ghidra retains the misleading name).
17. **§2.6** — `FUN_0059E740 @ 0x59E740` (RMG_PlaceBridge) — function exists, body
    0x59E740–0x5A004C.
18. **§3.3.3 / §3.3.4 / §3.3.5** — `Get_Draw_Offset @ 0x480110`, `FUN_005FDCC0 @
    0x5FDCC0`, `FUN_00483E30 @ 0x483E30` — all three functions exist at the cited
    addresses.

---

## UNVERIFIABLE — flag for user

None. Every load-bearing claim spot-checked resolved cleanly.

---

## Summary recommendation

One substantive amendment (W1) plus one optional clarifying insertion (W2). The doc
is otherwise solid on the per-frame draw chain, the layer-step mapping, the Latin
square mechanic, the shadow shift, and the `_High`-selector-doesn't-exist claim. The
only landmine — the `cell+0x11A` "damage_state_1" mislabel in §10.3 OQ#3 — should be
fixed in place to prevent future readers from being misled. The §6 cell-offset table
(line 510) already labels `+0x11A` correctly as "sub_tile"; only the OQ#3 prose and
the §4 cross-reference note (lines 522–523) need rewording.
