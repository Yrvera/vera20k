# Adversarial verdicts — body-state-machine facet

Auditor pass: live re-decompile of every cited gamemd address + re-read of current Rust.
Burden of proof = default DRIFT; downgraded only on proven equivalence; UNCERTAIN where the
gamemd side could not be independently confirmed this pass.

Anchors re-confirmed live this pass (`get_function_by_address` name+entry then `decompile_function`):
- `ProcessBridgeDamageStateMachine_High @ 0x00576ba0` (name+entry confirmed).
- `MapClass__UpdateRamp_NS_DamageA_High @ 0x00572230`, `_DamageB_High @ 0x00572330`,
  `_CollapseA_High @ 0x00572440` (name+entry confirmed: `00572440-005727d4`).
- `ApplyDamageToCell @ 0x00587180` (proves `+0x44` = visible overlay byte, dispatched via
  `0x4A..0x63` Low / `0xCD..0xE6` High — same ranges as Rust `overlay_byte`).
- `MapClass__UpdateAdjacentBridges_High @ 0x00576770`.

Key structural correction to the finder's framing: `0x00576ba0` has TWO disjoint branches.
The OUTER `if ((local_54 & 0x100) == 0)` block is the **bridgehead/ramp-class** branch (gated
on the cell holding a bridgehead tile-class `DAT_00abad30..+3` / `DAT_00aa1028..+3`); the
recursive 3-cell BlowUpBridge + `SetOverlayAndPropagate` + two
`UpdateAdjacentBridges_High(MapCoord_Add(...))` calls live HERE. The ELSE block
(`local_54 & 0x100` set) is the **body-cell state machine** (`switch(puVar9[0x11e])`) that
`body_cell_advance_state` mirrors. The body branch reaches the perpendicular overlay/BlowUp
side effects ONLY indirectly, through the `UpdateRamp_NS/EW_*_High` helpers it calls on the
anchor's own coord (`puVar9+0x24`). D1/D2 are therefore correctly attributed to those helpers
(which the body facet owns calling), not to the bridgehead branch.

---

D1: VERDICT=REAL — `UpdateRamp_NS_CollapseA_High @ 0x00572440` (re-decompiled, entry
`00572440-005727d4`) is called by the body branch (cases 6/7/8 of `0x00576ba0`). It walks ONE
perpendicular cell, then: (a) on `[0x140] & 0x80` set does the state-byte write (`<7→7`,
`==8 → recurse + SetBridgeDirection(0,0) + [0x11e]=0 + [0x44]=-1 + MarkTerrainDirty`); (b)
INDEPENDENTLY of the `0x80` bit, on tile-class `+0x38 == DAT_00abad30+3` it RECURSES
`UpdateRamp_NS_CollapseA_High` and fires THREE `CellClass__BlowUpBridge` calls (the
`puVar4[0x11a]&1` even/odd row-vs-column mirror is present) then
`SetOverlayAndPropagate(.., DAT_00abad30+3+BridgeSet, .., (char)[0x11b]-4, 0)`. Current Rust
`update_ramp_perpendicular` (bridge_specs.rs:537-641) mutates only an `Anchor`-role target's
`damage_state` + abstract `bridgehead_anchor_class`; it issues NO `BlowUpBridge`, no recursion,
no overlay/pavement write on any target. Corrected delta: Rust [CollapseA on tile-class-`+3`
perpendicular target = no-op] -> gamemd [recurse CollapseA + 3× BlowUpBridge on the body-axis
3-cell row + SetOverlayAndPropagate(+3)]. Verified `decompile_function 0x00572440` and
`0x00576ba0`.

D2: VERDICT=REAL — `UpdateRamp_NS_DamageA_High @ 0x00572230` and `_DamageB_High @ 0x00572330`
(both re-decompiled) write the perpendicular TARGET cell's visible overlay class UNCONDITIONALLY
(after, and independent of, the `0x80` state-byte gate): `iVar3 = (target+0x38 - BridgeSet)+1`,
then pavement-class → `ToggleBridgePavement(&coord,1,0)`; DamageA: `==DAT_00abad30 →
SetOverlayAndPropagate(DAT_00abad30+BridgeSet)` + return, `==+2 → SetOverlayAndPropagate(+2)` +
return; DamageB: `==DAT_00abad30 → SetOverlayAndPropagate(+1)`, `==+1 →
SetOverlayAndPropagate(+2)`. Current Rust writes only `cell.bridgehead_anchor_class` (abstract
enum, bridge_specs.rs:602-622) and NEVER writes `overlay_byte` or toggles pavement. Corrected
delta: Rust [perpendicular DamageA/B target overlay/pavement = unwritten] -> gamemd
[SetOverlayAndPropagate(+0/+1/+2 BridgeSet class) or ToggleBridgePavement on the target's
visible tile, gated by which `+0x38` bridgehead slot it holds]. Verified
`decompile_function 0x00572230` and `0x00572330`.

D3: VERDICT=REAL — Body branch `LAB_0057778a` (and the EW twin before `LAB_005778cc`) in
`0x00576ba0` executes `puVar9[0x11e] = 0; *(undefined4 *)(puVar9 + 0x44) = 0xffffffff;` on
final collapse. `ApplyDamageToCell @ 0x00587180` reads `*(puVar6+0x44)` and dispatches
`0xCD..0xE6 → DestroyBridge_High` / `0x4A..0x63 → DestroyBridge_Low` — proving `+0x44` is the
visible overlay byte that Rust mirrors as `overlay_byte` (identical ranges). Current Rust body
collapse (mod.rs:1090-1093) sets `damage_state=Destroyed` but never touches `overlay_byte`. I
traced the only downstream `overlay_byte` writer on the collapse path —
`update_adjacent_bridges` (bridge_orchestrator.rs:1035-1108): it `continue`s past `Destroyed`
cells (line 1091) and only resets cells whose anchor span is gone (`stub_now`, line 1094), so
the just-collapsed anchor is NEVER cleared. `effective_render_state` (mod.rs:945-970) maps the
stale loaded byte (e.g. `0xD6 ∈ 0xD6..=0xD9`) to `Some(Healthy)`, so `is_bridge_walkable`
(mod.rs:972) returns true on a collapsed anchor. Finder's demote-condition (a downstream pass
forcing `0xFF`) was checked and does NOT fire. Corrected delta: Rust [collapsed body anchor
keeps loaded body overlay_byte → renders Healthy + stays walkable] -> gamemd [`+0x44 = -1`,
overlay cleared → renders collapsed/empty + impassable]. Verified `decompile_function
0x00576ba0` (LAB_0057778a) and `0x00587180` (`+0x44` semantics).

D4: VERDICT=REAL (finder under-scoped it; was UNCHECKED) — Body branch calls
`MapClass__UpdateAdjacentBridges_High(psVar11)` with `psVar11 = param_1` (the input damage
cell), so the INPUT-CELL coordinate matches Rust's `compute_adjacent_bridges_dirty(rx,ry,..)`
input. BUT `UpdateAdjacentBridges_High @ 0x00576770` (re-decompiled) is NOT a "two perpendicular
cells" producer: it scans 8 neighbors for a `0x140 & 0x500` cell, walks toward the bridge head
(up to the `0x400` ramp run), locates the bridge-end iso-tile by `+0x38` class + `+0x11a` height
byte, and conditionally calls `UpdateBridgeEdgeTiles_High` + `DirtyScreenRect` to rewrite the
END tiles. Rust `compute_adjacent_bridges_dirty` (mod.rs:2028-2043) merely returns the two
perpendicular neighbor coords of the input cell; the orchestrator's `update_adjacent_bridges`
(bridge_orchestrator.rs:1035) is a separate orphan-stub-overlay reset, NOT an edge-tile
recompute. So both the dirtied cell SET and the edge-tile-rewrite effect diverge. Corrected
delta: Rust [rim = {2 perpendicular neighbors of input cell}; no edge-tile rewrite] -> gamemd
[neighbor-scan → walk to bridge head → `UpdateBridgeEdgeTiles_High` rewrites the bridge-end
iso-tiles + `DirtyScreenRect`]. Verified `decompile_function 0x00576770` and the body-branch
`UpdateAdjacentBridges_High(psVar11)` call in `0x00576ba0`. Player-visibility: edge/end-tile
art at a collapsed span boundary; fires on every body collapse. Severity LOW-MED (visual end
tiles only), but it IS a real output divergence, not UNCHECKED.

D5: VERDICT=REAL — Confirms the High/Low overlay-propagate split is load-bearing and missing.
High `UpdateRamp_NS_DamageA_High @ 0x00572230` uses concrete-bridge constants (`DAT_00abad30`,
`DAT_00aa0e28` BridgeSet, `DAT_00abc1e8`/`DAT_00aa0e38` pavement); the `_Low` family at
`0x0056ed40` uses the wood equivalents (not re-decompiled this pass — see UNCERTAIN note). The
state-byte transitions ARE shared (PARITY — Rust's single `apply_ramp_transition` is correct).
But since D2 proves the entire overlay-propagate branch is unimplemented in Rust, `is_high_bridge`
being bound as `_is_high_bridge` (bridge_specs.rs:542) is currently moot AND wrong: once D2 is
implemented it must branch High vs Low constants, which it cannot today. Corrected delta: same
as D2, plus Rust [`is_high_bridge` discarded] -> gamemd [overlay-class constants differ
concrete(High) vs wood(Low)]. Verified High constants `decompile_function 0x00572230`; Low
constant set asserted from the High structure + doc, NOT re-decompiled this pass (the
state-byte sameness is proven; the wood-constant identity is inherited from D2, which is the
only output that matters).

---

## PARITY (spot-checked against live `0x00576ba0`, not re-litigated)

- Body state-byte switch ranges + case→phase mapping (NS 0..8, EW 9..0x11; EW 0x10→CollapseB,
  0x11→CollapseA) match the live `switch(puVar9[0x11e])`. CONFIRMED.
- Perpendicular direction args: body NS `uVar6=(8<state)-1 = -1` → `iVar2=2`(E), `uVar6&6=6`(W);
  EW `uVar6=0` → `iVar2=4`(S), `uVar6&6=0`(N). Matches Rust `perpendicular_direction`
  (bridge_specs.rs:514-522). CONFIRMED live.
- Anchor-pointer follow: `if ((local_54 & 0x80) == 0) puVar9 = *(puVar9+0x2c);` matches Rust
  `anchor_span_id → span.anchor`. CONFIRMED live.
- DamageA-before-DamageB and CollapseA-before-CollapseB call order. CONFIRMED live.

## UNCERTAIN / not independently confirmed this pass

- `0x80`-bit vs `role==Anchor` gate equivalence (finder UNCHECKED #4): the binary gates the
  perpendicular STATE-byte write on the target's `[0x140] & 0x80` (anchor-self bit). Rust gates
  on `role==Anchor`. Whether `{cells with 0x80} == {role==Anchor}` exactly in the Rust model is
  NOT proven here. If a `Body`/`Tail` cell carries `0x80` in gamemd, the binary writes its state
  byte while Rust no-ops (bridge_specs.rs:624). Mark UNCERTAIN, not REAL — needs a map fixture
  comparison. (Note: this is the STATE-byte gate only; the OVERLAY write of D2 is `0x80`-
  independent and is unconditionally REAL.)
- Low UpdateRamp families (`0x0056ed40`/`ee40`/`ef50`/`f2f0` NS, `0x0056f690`/`f7a0`/`f8b0`/
  `fc80` EW) and EW-High CollapseA/B (`0x00572da0`/`0x00573170`) not re-decompiled this pass.
  Expected to mirror NS-High; the D5 Low wood-constant identity rests on that assumption.

## MISS (new disparities the finder did not surface)

- MISS: In the body-branch DamageA/DamageB perpendicular writes, the gamemd state-byte gate is
  `[0x11e] < 4 → 4` (DamageA) / `< 4 → 5`, `==4 → 6` (DamageB) — note DamageA only advances
  `==5 → 6` (NOT `==4`), DamageB only `==4 → 6`. Rust `apply_ramp_transition` encodes this
  correctly (NS DamageA `5→6`, DamageB `4→6`). PARITY — listed only to record it was checked,
  no disparity.
- MISS: The body-branch CollapseA/CollapseB ON THE ANCHOR ITSELF (`0x00576ba0` cases 6/7/8) does
  NOT directly write the anchor's overlay; it relies on the `UpdateRamp_*` perpendicular helper
  to do overlay work on neighbors and then clears the anchor's own `+0x44` to -1 at
  `LAB_0057778a`. Rust's `update_ramp_perpendicular` performs the perpendicular state write but
  the anchor's own overlay-progression-to-destroyed-tile (the `DestroyBridge_*` walker overlay,
  `0xE7`/`0xE8` destroyed byte) is on the direct-overlay path, not here — so no new disparity
  beyond D3, but the absence of ANY overlay_byte mutation in the entire body state-machine path
  (it writes only `damage_state`) means the renderer has no destroyed-tile byte to show even
  apart from D3's stale-byte problem. This compounds D3: not just "stale healthy byte" but
  "never any destroyed byte written by the body SM path." Surfacing as a sharper restatement of
  D3's root cause.
