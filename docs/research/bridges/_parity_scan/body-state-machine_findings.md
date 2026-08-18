# Body-state-machine parity scan — High + Low body cell state machine + UpdateRamp perpendicular

Facet: `body_cell_advance_state` (src/sim/bridge_state/mod.rs) + `update_ramp_perpendicular`
/ `set_bridge_direction` (src/sim/bridge_specs.rs).

Anchors verified live this pass:
- `ProcessBridgeDamageStateMachine_High` @ `0x00576ba0` (`get_function_by_address` confirms name+entry; full `decompile_function`).
- `UpdateRamp_NS_DamageA_High` @ `0x00572230`, `_DamageB_High` @ `0x00572330`, `_CollapseA_High` @ `0x005727... (0x00572440)`, `_CollapseB_High` @ `0x005727e0`.
- `UpdateRamp_EW_DamageA_High` @ `0x00572b80`, `_DamageB_High` @ `0x00572c90`.
- `CellClass__SetBridgeDirection_NESW` @ `0x0047e040`.

Authority: live decompile > docs. All findings below are decompile-grounded.

---

### D1: Perpendicular UpdateRamp cascade only mutates a same-axis ANCHOR target; binary mutates any cell with the `0x80` bit, and chains a recursive 3-cell BlowUpBridge collapse on a parallel span

- Rust now: `update_ramp_perpendicular` (bridge_specs.rs:537-641) walks ONE perpendicular cell, then branches on `target_cell.role`. It mutates the target's `damage_state` only when `role == BridgeCellRole::Anchor` (line 569) — using `apply_ramp_transition`. For `Bridgehead` it does a tile-class write only. For every other role (`Body`, `Tail`) it is a no-op (line 624). It NEVER issues a `BlowUpBridge` on the perpendicular target, and never recurses.
- gamemd: `UpdateRamp_NS_CollapseA_High @ 0x00572440` walks to the perpendicular cell `(this + g_DirectionOffsets[param_2 & 7])` and gates ONLY on `puVar4[0x140] & 0x80` (the cell's anchor-self bit), not on a role enum: `if ((puVar4[0x140] & 0x80) != 0) { if (state < 7) state = 7; else if (state == 8) { recurse CollapseA; SetBridgeDirection_NESW(0,0); state = 0; +0x44 = -1; MarkTerrainDirty; } }`. Then a SECOND, tile-class-gated branch: when the target's `+0x38` class equals `DAT_00abad30 + 3` (the bridgehead `+3` class), it RECURSES `UpdateRamp_NS_CollapseA_High` and fires THREE `CellClass__BlowUpBridge` calls on `(this)`, `(this, y-1)`, `(this, y+1)` (or the `0x11a&1` odd-height mirror: three cells at `x-1`), then `SetOverlayAndPropagate(.., DAT_00abad30+3+BridgeSet, .., level-4, ..)`. The DamageA/DamageB helpers (`0x00572230`/`0x00572330`) similarly write the target's `+0x11E` (DamageA: `<4 → 4`, `==5 → 6`; DamageB: `<4 → 5`, `==4 → 6`) and propagate the `+2`/`+1` overlay class on the target when its `+0x38` class is `DAT_00abad30`/`+1`/`+2`.
- Fixture: two NS anchor spans laid side-by-side along X, both healthy, anchors A0=(10,10) and A1=(11,10) (A1 is one cell East of A0, i.e. the A-side perpendicular target of A0). Hit A0 twice so A0 reaches state 6 (Damaged) then collapses. On the collapse hit the binary calls `UpdateRamp_NS_CollapseA_High(A0, dir 2=E)`. That walks East to A1=(11,10); A1 has `0x140 & 0x80` set (it is its own anchor). A1's state byte advances `<7 → 7` (PartialCollapseA). If A1 was already at state 8 it would collapse-final right there (clear, BlowUp, overlay −1). The Rust path: `update_ramp_perpendicular(A0, NS, CollapseA)` reads target A1, sees `role == Anchor`, runs `apply_ramp_transition(6, NS, CollapseA) = Some(7)` → A1 becomes PartialCollapseA. So far the same byte. BUT if A1 were a `Body`/`Tail` cell of a parallel span (not its own anchor), the binary still mutates it (it has the `0x80` bit only if it is an anchor — so this sub-case matches), AND in the `+3` tile-class case the binary blows up three cells that Rust never touches.
- Player sees: a parallel high-bridge span whose ramp-end (bridgehead `+3` class) sits perpendicular to a collapsing body anchor fails to drop its three end cells / does not chain-collapse. Triggers whenever two bridge spans run parallel one cell apart, or a body collapse is perpendicular-adjacent to a bridgehead-`+3` ramp tile — uncommon on stock maps but happens on multi-lane bridges and author-damaged ramps.
- Severity: MED
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00572440` (NS CollapseA: the `+0x80` gate + the `DAT_00abad30+3` recursive 3-cell BlowUpBridge branch) and `decompile_function 0x00572230` (NS DamageA target `+0x11E` write + overlay propagate).

---

### D2: Perpendicular DamageA/DamageB overlay-class write on the TARGET cell is unimplemented; Rust models only the anchor `bridgehead_anchor_class` field, not the target's `+0x44` overlay / `+0x38` tile-class propagate

- Rust now: In `update_ramp_perpendicular` the only target overlay-ish mutation is `apply_anchor_class_transition` writing `cell_mut.bridgehead_anchor_class` (bridge_specs.rs:602-622). It writes the target's `damage_state` byte (Anchor role) and an abstract `bridgehead_anchor_class` enum. There is no write of the target's visible `overlay_byte` (the model's mirror of `+0x44`) and no `SetOverlayAndPropagate` equivalent.
- gamemd: `UpdateRamp_NS_DamageA_High @ 0x00572230` after the `+0x11E` write computes `iVar3 = (target+0x38 - BridgeSet) + 1` and: if class `== DAT_00abad30` → `SetOverlayAndPropagate(target, DAT_00abad30 + BridgeSet, ...)` and RETURN; if class `== DAT_00abad30 + 2` → `SetOverlayAndPropagate(target, DAT_00abad30 + 2 + BridgeSet, ...)` and RETURN; else (pavement class) → `ToggleBridgePavement`. DamageB (`0x00572330`) propagates `+1`/`+2` classes. So the binary writes a NEW iso-tile/overlay class onto the perpendicular ramp cell (its visible art), gated on which bridgehead-class slot the target currently holds.
- Fixture: NS anchor A0=(10,10), with a bridgehead ramp cell B=(11,10) East of it whose `+0x38` class == `DAT_00abad30` (slot +0). First hit on A0 → state 0..5 healthy → binary calls `UpdateRamp_NS_DamageA_High(A0, 2)`: walks East to B, B is not an anchor (`0x80` clear → no `+0x11E` write), class `== DAT_00abad30` → `SetOverlayAndPropagate(B, DAT_00abad30 + BridgeSet)` — B's visible tile changes to the slot-0 damaged-progress art. Rust: target B has `role == Bridgehead`, so it runs `apply_anchor_class_transition(Variant0, DamageA) = Variant0` (no-op per table line 661), leaving B's `bridgehead_anchor_class` and `overlay_byte` unchanged. So the perpendicular ramp art does not update on first body damage.
- Player sees: the ramp/bridgehead cell perpendicular to a damaged body anchor does not show its damage-progress sprite when the body takes its first hit. Triggers on the first damaging hit to any high bridge whose anchor has a perpendicular ramp neighbor — i.e. every bridge end, every match where a bridge is shot.
- Severity: MED
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00572230` (the `DAT_00abad30` / `DAT_00abad30 + 2` SetOverlayAndPropagate branches) and `decompile_function 0x00572330` (`+1`/`+2`).

---

### D3: Body-collapse driver clears nothing equivalent to overlay `+0x44 = -1` on the anchor, and writes Destroyed only to the anchor; binary clears the anchor's overlay byte (`+0x44 = 0xFFFFFFFF`) and writes state `+0x11E = 0`

- Rust now: On `DamageState::Damaged` collapse (mod.rs:1071-1121) the driver sets `c.damage_state = DamageState::Destroyed` on the anchor (line 1092) and leaves `overlay_byte` untouched. It relies on `effective_render_state` (mod.rs:944-970) to map a stale overlay byte; for a body anchor whose `overlay_byte` is still a healthy/damaged body byte (`0xCD..0xDE`), `effective_render_state` returns `Some(Healthy/Damaged)`, NOT `None`. The cell's `damage_state == Destroyed` but `overlay_byte` is unchanged, so `is_bridge_walkable` (line 972) returns `effective_render_state(cell).is_some()` → could still report walkable/healthy art from the overlay byte.
- gamemd: In the body branch final-collapse (`0x00576ba0`, cases 6/7/8 → `LAB_0057778a`, and 0xF/0x10/0x11): `CellClass__SetBridgeDirection_NESW(0,0)` (clears flags, state byte 0), then `puVar9[0x11e] = 0; *(undefined4 *)(puVar9 + 0x44) = 0xffffffff;` then `UpdateAdjacentBridges_High`. The `+0x44 = -1` (overlay clear) is explicit and is the source of "no body overlay" after collapse.
- Fixture: NS anchor A=(10,10), overlay_byte loaded as `0xD6` (NS healthy variant 0), state byte 0. Hit 1 → Healthy→Damaged (state byte → 6 logically; Rust sets `damage_state=Damaged`, overlay_byte still `0xD6`). Hit 2 → collapse: Rust sets `damage_state=Destroyed`, overlay_byte STILL `0xD6`. `effective_render_state` sees `0xD6` ∈ `0xD6..=0xD9` → `Some(Healthy{variant 0})` (mod.rs:959). So a collapsed body anchor still renders as a healthy NS body tile and `is_bridge_walkable` returns true. Binary: `+0x44 = -1` → no overlay → cell renders as collapsed/empty and is impassable.
- Player sees: a fully collapsed body anchor cell can keep drawing its intact bridge sprite and stay walkable, instead of showing the destroyed/empty span. Triggers on every body-anchor collapse where the anchor's loaded overlay byte was a raw body byte (the normal case) and nothing later overwrites `overlay_byte`.
- Severity: HIGH
- Confidence: LIKELY-DRIFT (the state byte is correct; the unproven part is whether a later orchestrator/render pass forces `overlay_byte → 0xFF`. I did not find a write to `overlay_byte` on the collapse path in `body_cell_advance_state` or the immediate orchestrator collect loop (bridge_orchestrator.rs:78-104). If a downstream walker rewrites it, demote to PARITY.)
- Verify-call: `decompile_function 0x00576ba0` — `LAB_0057778a` block: `puVar9[0x11e] = 0; *(undefined4 *)(puVar9 + 0x44) = 0xffffffff;`.

---

### D4: `UpdateAdjacentBridges_High` is fired at TWO specific perpendicular cells in the bridgehead-collapse and EW-body branches; Rust `compute_adjacent_bridges_dirty(rx, ry, axis)` derives rim cells from the input cell, not from the binary's exact `MapCoord_Add` offsets

- Rust now: On collapse the driver calls `compute_adjacent_bridges_dirty(rx, ry, axis)` (mod.rs:1113,1136,1158) passing the INPUT damage cell `(rx,ry)`, not the resolved anchor, and computes its own rim set.
- gamemd: The body collapse path calls `MapClass__UpdateAdjacentBridges_High(psVar11)` once on `psVar11` (= the original `param_1` coordinate pointer, the input cell) — see `LAB_0057778a` and the EW finalize. The bridgehead `+3` collapse branch instead calls it TWICE on `MapCoord_Add(&local_20, &g_refinery_unload_adjacent_lookup_dx)` and `MapCoord_Add(&local_20, &DAT_0089f690)` — two fixed perpendicular offsets from the resolved anchor coord `local_20`, not the input cell.
- Fixture: body case — input cell = a Body cell at (10,12) whose anchor is (10,10). Binary's body-branch `UpdateAdjacentBridges_High(psVar11)` uses `psVar11` = the input `param_1` = (10,12)-derived pointer (psVar11 is set at entry `psVar11 = param_1` and only reassigned inside the bridgehead branch). Rust passes `(rx,ry) = (10,12)` to `compute_adjacent_bridges_dirty`. The coordinate used (input cell, not anchor) matches for the body branch. The drift is the bridgehead `+3` branch (two fixed-offset rim updates from the anchor) which the body driver never reaches — that branch is owned by D1/the bridgehead facet, but the rim-cell SET produced by `compute_adjacent_bridges_dirty` is unverified against the binary's single `psVar11` update for the body path.
- Player sees: rim/edge tile re-evaluation around a collapsed span may dirty a different cell set than gamemd, producing 1-cell-off edge-tile redraw at span boundaries. Triggers on every body collapse (rim pass runs each collapse).
- Severity: LOW
- Confidence: UNCHECKED (I did not read the body of `compute_adjacent_bridges_dirty` this pass, nor `UpdateAdjacentBridges_High @ 0x00576770`. The body branch passes the input cell which matches `psVar11`; whether the derived rim SET matches is unverified.)
- Verify-call: `decompile_function 0x00576ba0` shows `MapClass__UpdateAdjacentBridges_High(psVar11)` (body) vs the two `MapCoord_Add(...)` rim calls (bridgehead). Pending: read `compute_adjacent_bridges_dirty` + `decompile_function 0x00576770`.

---

### D5: `is_high_bridge` parameter is ignored in both `body_cell_advance_state` and `update_ramp_perpendicular` — the doc claim "Low uses the same transitions as High" is TRUE for `+0x11E` but the perpendicular OVERLAY propagate and pavement-class checks differ Low vs High

- Rust now: `body_cell_advance_state` passes `is_high_bridge` straight through and `update_ramp_perpendicular` binds it as `_is_high_bridge` (unused, bridge_specs.rs:542). The state-byte transitions are shared. Comment (mod.rs:992-994) says "state transitions identical for HIGH and LOW per HIGH §11.1".
- gamemd: The `+0x11E` state-byte transitions in `UpdateRamp_*_High` (`0x00572230` etc.) ARE the same shape as the `_Low` variants (`0x0056ed40` etc.) per the doc. HOWEVER the overlay-class propagate constants differ: High uses `DAT_00abad30`/`DAT_00aa1028` (concrete bridgehead classes) + `DAT_00abc1e8`/`DAT_00aa0e38` pavement; Low uses the wood-bridge equivalents. Since D2 shows the overlay-propagate branch is unimplemented entirely, the High/Low split there is also unimplemented. So `is_high_bridge` being ignored is currently harmless ONLY because the overlay branch (where it would matter) is missing.
- Fixture: same as D2 but on a LOW (wood) bridge — the propagate would use the wood class constants; Rust does neither High nor Low propagate, so a wood ramp cell perpendicular to a damaged wood body anchor also fails to update its art.
- Player sees: identical symptom to D2 on wooden bridges. Triggers on every first-hit to a wooden bridge body with a perpendicular ramp.
- Severity: MED (folded into D2; listed separately because it confirms the High/Low parameter is load-bearing once D2 is fixed)
- Confidence: PROVEN-DRIFT (that the overlay propagate is missing; the state-byte sameness is PARITY)
- Verify-call: `decompile_function 0x00572230` (High constants) vs `decompile_function 0x0056ed40` (Low NS DamageA — pending read to confirm wood constants, but state-byte shape proven shared by doc + High decompile).

---

## PARITY-CONFIRMED

- **Body state-byte switch ranges (NS 0..8, EW 9..0x11).** `DamageState::to_state_byte`/`from_state_byte` (mod.rs:85-144) encode `0..5→0..5`, `6`, `7`, `8`, `9..14→9..14`, `0xF`, partial `0x10`/`0x11`. Matches `0x00576ba0` switch cases exactly, including the swap that EW `0x10`=PartialCollapseB and `0x11`=PartialCollapseA (mod.rs:140-141; binary case 0x10 fires CollapseB, case 0x11 fires CollapseA).
- **Healthy(0..5/9..14) absorbs into Damaged(6/0xF), returns 0 (no collapse).** Rust `Healthy → Damaged` returns `StateOutcome::Absorbed` (mod.rs:1047-1070); binary cases 0..5/9..14 write 6/0xF and `return 0`.
- **Damaged(6/0xF) → full collapse: CollapseA + CollapseB then finalize, returns 1.** Rust fires `update_ramp_perpendicular(CollapseA)` then `(CollapseB)`, sets Destroyed, returns `Collapsed{binary_success:true}` (mod.rs:1071-1121). Binary case 6 calls CollapseA(iVar2) then CollapseB(uVar6&6) then `LAB_0057778a`; case 0xF same for EW; both `return 1`.
- **PartialCollapseA(7/0x11) fires only CollapseA; PartialCollapseB(8/0x10) fires only CollapseB; then finalize.** Rust (mod.rs:1122-1166) fires the single matching phase then Destroyed. Binary case 7 → CollapseA only → `LAB_0057778a`; case 8 → CollapseB only; case 0x10 → CollapseB only; case 0x11 → CollapseA only. Order and single-side selection match.
- **DamageA-then-DamageB call ORDER, and CollapseA-then-CollapseB call ORDER.** Rust always issues phase A before phase B (mod.rs:1053/1061, 1074/1082). Binary issues DamageA(iVar2) before DamageB(uVar6&6), CollapseA before CollapseB. Match.
- **Perpendicular DIRECTION per axis/phase.** Rust `perpendicular_direction` (bridge_specs.rs:514-522): NS-A→E(2), NS-B→W(6), EW-A→S(4), EW-B→N(0). Binary body branch passes `iVar2`= `(uVar6 & ~1)+4` and `uVar6 & 6`: NS (state≤8, uVar6=−1)→ 2 and 6; EW (state>8, uVar6=0)→ 4 and 0. Then each UpdateRamp helper walks `g_DirectionOffsets[param_2 & 7]`. Directions match (2=E,6=W,4=S,0=N).
- **Perpendicular state-byte transition values.** `apply_ramp_transition` (bridge_specs.rs:353-386): NS DamageA `0..3→4`,`5→6`; DamageB `0..3→5`,`4→6`; CollapseA `0..6→7`,`8→0`; CollapseB `0..6→8`,`7→0`; EW mirror. Binary: DamageA `<4→4`,`==5→6`; DamageB `<4→5`,`==4→6`; CollapseA `<7→7`,`==8→final`; CollapseB `<7→8`,`==7→final`. Byte values and final-collapse triggers match (the `0..6` = `<7` equivalence holds: states 0..6 inclusive are exactly `<7`).
- **SetBridgeDirection destroy blow-up slots = anchor, forward1, forward2, opposite (slots 0,1,2,4); flag-only on forward3 (slot3) and dir-6 extra (slot5).** Rust `AnchorSpan::BLOW_UP_SLOTS=[0,1,2,4]` + `set_bridge_direction` (bridge_specs.rs:280,474-492). Binary `0x0047e040` destroy path (`cVar14==0`): BlowUpBridge on param_1, this(fwd1), this(fwd2), param_1-after-opposite-walk; forward-3 is `Flags & 0xffffefff | uVar15` flag-only; the `param_2==6` extra cell is `+0x140 & 0xfffeffff | uVar16` flag-only. Exact match.
- **SetBridgeDirection_NESW vs NWSE byte-identical for the destroy contract.** Plate comment + doc; not re-decompiled NWSE this pass but matches prior verification.
- **Body branch follows anchor pointer for non-anchor cells.** Rust follows `anchor_span_id → span.anchor` (mod.rs:1017-1028). Binary: `if ((local_54 & 0x80) == 0) puVar9 = *(puVar9 + 0x2c);` (follow `+0x2C` anchor ptr). Same intent (resolve to anchor before reading state).

## UNCHECKED

- **D4 rim-cell set:** `compute_adjacent_bridges_dirty` body not read; `UpdateAdjacentBridges_High @ 0x00576770` not decompiled. The body branch passes the input cell (matches `psVar11`), but the produced rim cell SET is unverified.
- **EW CollapseA/CollapseB High (`0x00572da0`/`0x00573170`) and all four Low UpdateRamp families (`0x0056ed40`/`ee40`/`ef50`/`f2f0` NS, `0x0056f690`/`f7a0`/`f8b0`/`fc80` EW)** not decompiled this pass. Expected to mirror NS-High by symmetry (doc §11.1), but the EW collapse-final recursion and Low overlay constants are unverified live.
- **D3 downstream overlay_byte rewrite:** whether any post-collapse orchestrator/render pass forces the collapsed anchor's `overlay_byte → 0xFF` was not traced beyond `body_cell_advance_state` and the immediate collect loop. If one does, D3 demotes to PARITY.
- **`+0x80` vs `role==Anchor` gate equivalence (D1):** the binary gates perpendicular mutation on the cell's `0x140 & 0x80` bit; Rust gates on `role==Anchor`. Whether every cell with the `0x80` bit is exactly the set of `role==Anchor` cells in the Rust model (and vice-versa) is unproven — a `Body`/`Tail` cell that carries `0x80` in gamemd would be mutated there but skipped in Rust (no-op branch at bridge_specs.rs:624).
