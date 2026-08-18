# Bridgehead Slot +3 Collapse — Adversarial Verdicts

Facet: bridgehead-slot3 — direct-damage slot+3 collapse branch of
`ProcessBridgeDamageStateMachine_High@0x00576ba0` / `_Low@0x00571490`.

Method: re-decompiled both state machines live this session (`decompile_function 0x00576ba0`,
`0x00571490`, `0x00572230`, `0x00572330`); re-read current Rust
(`src/sim/bridge_state/mod.rs::bridgehead_advance_state`,
`src/sim/bridge_specs.rs`, `src/sim/world/bridge_orchestrator.rs`,
`src/map/resolved_terrain.rs`, `src/map/theater.rs`). Addresses re-confirmed via
`get_function_by_address 0x00576ba0` (= ProcessBridgeDamageStateMachine_High) and
`0x00572330` (= MapClass__UpdateRamp_NS_DamageB_High).

---

D1: VERDICT=REAL — Live `0x00576ba0`: the absorb/collapse decision is a single read of the
INPUT cell's own tile class `iVar2 = (puVar9[0x38] - DAT_00aa0e28) + 1`; only `iVar2 ==
DAT_00abad30 + 3` (NS) / `+3` (EW) runs the BlowUpBridge collapse and `return 1`, while
slots +0/+1/+2 take the absorb branch (`SetOverlayAndPropagate(..., DAT_00abad30 + 2 ...)`)
and `return 0`. Repeating the hit re-reads the same unchanged input class — a healthy
bridgehead never collapses via this path. Rust (`mod.rs:1449-1535`) collapses when
`input_is_final || anchor_is_final` where `is_final ≡ bridgehead_anchor_class==AboutToFall`,
and the absorb path at `mod.rs:1543-1545` writes the *anchor* to `AboutToFall`, so a 2nd hit
collapses. Confirmed by `tests.rs:1282` (first `Absorbed`, second `Collapsed`) and by the
orchestrator (`bridge_orchestrator.rs:1434-1470`): `Absorbed` has no `apply_damage_success`,
so the path simply re-runs on the next event with no guard. Low `0x00571490` is the same
shape (collapse gated on `uVar3 == DAT_00abad30 + 3`; absorb writes slot+2).
  Corrected delta: Rust = healthy bridgehead collapses on the 2nd direct state-machine hit
  (1st hit writes anchor to AboutToFall, 2nd collapses) -> gamemd = healthy bridgehead
  (input own class slot +0/+1/+2) NEVER collapses via this path; it only chips ramp/anchor to
  slot +2 (Damaged) and returns 0 every hit.

D2: VERDICT=REFUTED — Finder's Rust claim is STALE. Finder marked as UNCHECKED whether any
loader maps an authored slot+3 tile to `AboutToFall`; it does. `resolved_terrain.rs:955-975`
walks every BridgeSet cell, calls `BridgeAnchorVariantTable::match_tile_id(tid)`, and stores
`cell.bridgehead_anchor_class_at_load = Some(class)`; `theater.rs:720,737` shows that table
includes the slot-3 `AboutToFall` variant. `mod.rs:609-611` seeds the runtime cell with that
loaded class (default `Variant0` only when `None`). So an authored slot+3 bridgehead loads
with `bridgehead_anchor_class == AboutToFall` → `input_is_final == true` (`mod.rs:1449-1452`)
→ collapses on the FIRST hit, which MATCHES the binary's `iVar2 == DAT_00abad30 + 3` first-hit
collapse (live `0x00576ba0`). The premise "loaded as Variant0, follows 2-hit path" is wrong
against current code; the binary side is correct but the disparity does not exist.
  (Note: any geometry/level difference on that first-hit collapse is covered by D4/D5, not D2.)

D3: VERDICT=REAL — Live `0x00576ba0` absorb branch (NS): `param_1 = *(puVar9+0x24);
MapClass__SetOverlayAndPropagate(&param_1, DAT_00abad30 + 2 + DAT_00aa0e28, 0xffffffff,
0xffffffff, 0)` — anchor written to slot +2. The two neighbor helpers confirm the cap:
`UpdateRamp_NS_DamageB_High@0x00572330` writes only `slot+0→+1` (`DAT_00abad30+1`) and
`slot+1→+2` (`DAT_00abad30+2`); `UpdateRamp_NS_DamageA_High@0x00572230` writes `slot+0→+0`
and `slot+2→+2`. No bridgehead-state-machine path writes slot +3. Rust `mod.rs:1543-1544`
writes the anchor to `AboutToFall` (slot +3, per enum order `mod.rs:172-177`), and the
doc-comment `mod.rs:1538-1540` acknowledges the "skip to most-damaged" divergence.
  Corrected delta: Rust = absorb sets anchor `bridgehead_anchor_class=AboutToFall` (slot +3)
  -> gamemd = absorb sets anchor tile class to slot +2 (Damaged), never slot +3. (Root cause
  of D1's spurious 2nd-hit collapse, plus a 1-stage-too-advanced anchor SHP frame after one hit.)

D4: VERDICT=REAL (sim-state); player-visibility UNCERTAIN — Live `0x00576ba0` collapse:
`cVar1 = puVar9[0x11b]` (anchor/odd-shift-neighbor deck-height byte), then
`MapClass__SetOverlayAndPropagate(&param_1, DAT_00abad30 + 3 + DAT_00aa0e28 /*slot+3 tile*/,
0xffffffff, cVar1 + -4 /*level*/, 0)`. Low `0x00571490` does the same (`cVar1 = puVar8[0x11b]`,
`iVar2 = DAT_00abad30 + 3 + DAT_00abad1c`, `level = cVar1 - 4`). Rust collapse
(`mod.rs:1461-1535`) only sets `bridgehead_anchor_class = AboutToFall` and `damage_state =
Destroyed` on the BlowUpBridge row; there is no `overlay_byte`/Z/level write anywhere on the
collapse path (re-read full block — no level arg is carried). So the slot+3 overlay re-skin +
`level = deck_height - 4` drop is not modeled. REAL on sim state; whether it produces a wrong
collapsed-tile frame/elevation depends on how the (out-of-scope) renderer derives the tile from
`bridgehead_anchor_class`/`damage_state` — the renderer wasn't read, so visibility magnitude
stays UNCERTAIN, not the disparity itself.
  Corrected delta: Rust = no overlay/level write on collapse -> gamemd = collapse writes the
  anchor overlay to slot+3 at `level = (Cell deck-height byte 0x11B) - 4`.

D5: VERDICT=UNCERTAIN — Binary side holds: live `0x00576ba0`/`0x00571490` make exactly three
`CellClass__BlowUpBridge` calls, then build a fixed 10-cell (2×5) recalc list via the
`iVar8=-2..<3` / `iVar10..<2` nested loop into `PTR_FUN_007e3890` and
`MapClass__RecalcCellsAndRebuildZones` — there is NO four-neighbor "already-Destroyed" scan to
assemble a destroyed-set. Rust (`mod.rs:1510-1525`) appends already-`Destroyed` E/W/N/S
neighbors of the anchor to `destroyed_cells`, which has no binary analog. But this stays
UNCERTAIN (not REAL): `destroyed_cells` is a Rust-model aggregation feeding downstream
consumers (e.g. `refresh_endpoint_active_flags`), and I did not trace every consumer to prove
the extra/missing entries ever change observable output vs the binary's recalc rectangle. The
divergence is structural and only triggers when a perpendicular neighbor was pre-Destroyed at
collapse time; output-equivalence is unproven in either direction.

---

PARITY-CONFIRMED items spot-checked and upheld:
- BlowUpBridge 3-cell geometry NS-even (column at anchor.X, Y∈{Y-1,Y,Y+1}) verified in
  `0x00576ba0`; matches `bridgehead_blow_up_row` (`bridge_specs.rs:788-817`). NS-odd /
  EW<5 / EW>=5 shift branches present in binary and Rust, dead at the walked anchor (NS walk
  converges to h=4 even; EW to h=2 <5).
- Start-cell gates: High NS `if ((puVar9[0x11a] & 1) != 0) return 0`; High EW `if (4 < uVar6)
  return 0` — match `bridgehead_walk_to_anchor` (`bridge_specs.rs:716-728`).
- Walk targets NS=4, EW=2 — match `bridge_specs.rs:709-712`.
- Low collapse returns 0 / High returns 1: live `0x00571490` collapse blocks fall to
  `switchD_00572019_default: return 0`; `0x00576ba0` collapse paths `return 1`. Rust
  `Collapsed{binary_success: is_high_bridge}` (`mod.rs:1529`) reproduces it.
- IonCannon state-machine 4 attempts / direct 1: `bridge_orchestrator.rs:1429-1433`.

---

MISS (new, finder did not surface):

MISS-1 [LIKELY-REAL, EW absorb tail-call]: In `ProcessBridgeDamageStateMachine_Low@0x00571490`
the EW absorb block writes the anchor to slot+2 then calls **`MapClass__UpdateRamp_EW_DamageA_High`
and `MapClass__UpdateRamp_EW_DamageB_High`** (the *High* perpendicular helpers) — not the `_Low`
variants — and then `return 0`. Compare the Low NS absorb block immediately above it, which calls
`UpdateRamp_NS_DamageA_Low` / `_Low`. This is an EW-vs-NS asymmetry inside the Low machine that
the Rust `update_ramp_perpendicular` (which takes `_is_high_bridge` and is documented "unused …
state transitions identical for HIGH and LOW", `bridge_specs.rs:535-536`) cannot reproduce if
High and Low EW DamageA/B helpers ever diverge in their overlay tile targets. Needs a diff of
`UpdateRamp_EW_DamageA_High@0x00572b80` vs `_Low@0x0056f690` (and the B pair) to confirm whether
the High/Low EW absorb writes actually differ in output; if they do, this is a real EW-Low absorb
drift. (Either an intentional gamemd quirk or a decomp artifact — unverified this pass.)

MISS-2 [LOW, absorb does not bump input-cell state]: Both binary machines compute the
absorb/collapse purely from the INPUT cell's tile class and the walked ANCHOR; the hit
bridgehead cell's own `puVar9[0x11e]` state byte / `Cell+0x44` overlay is never advanced on the
absorb path (only the anchor + its two perpendicular neighbors change). Rust matches this
(`mod.rs` absorb touches only the anchor via `cell_mut(anchor_pos)` + the two
`update_ramp_perpendicular` calls; the input cell is untouched). Noted as confirmed-parity, not
a drift — surfaced because the finder's report did not explicitly assert the input cell stays
unmodified on absorb.
