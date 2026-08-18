# Bridge Parity Scan — Facet: map-load-overlay

Scope: how each bridge cell gets overlay_byte (CellClass+0x44), deck_level, axis
(NS/EW), the 0x80 anchor fact, bridgehead-class-at-load, initial damage state, and the
Latin-square variant jitter at draw. Rust sources scanned (current code):
`src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs` (bridge pass + bridgehead-class
pass), `src/sim/bridge_state/mod.rs::from_resolved_terrain` (passes 1-4) and its
`walk_anchor_pattern` helper.

Authority: live Ghidra decompile this session of `CellClass__SetBridgeDirection_NESW
@0x0047E040`, `OverlayClass__Mark @0x005FC570`, `CellClass__DrawOverlay_Body @0x0047F6A0`,
`MapClass__SelectBridgeTileVariant_Low @0x0057ACF0`. Anchor addresses re-confirmed via
`get_function_by_address` (0x0047E040 → CellClass__SetBridgeDirection_NESW; 0x0057ACF0 →
MapClass__SelectBridgeTileVariant_Low).

---

### D1: AnchorSpan slot-5 (dir-6 extra) cell is placed 1 cell short of the binary

- Rust now: `walk_anchor_pattern` in `src/sim/bridge_state/mod.rs:2004-2011` places slot 5
  at `(anchor.x + 1, anchor.y)` — i.e. **anchor + 1 East** — and only when
  `direction == Direction::W`. This is the AnchorSpan slot consumed by pass-2 role
  tagging (`from_resolved_terrain`, mod.rs:669-682) and by `body_cell_repair_state`
  slot iteration (mod.rs:1305).
- gamemd: `CellClass__SetBridgeDirection_NESW @0x0047E040`, dir-6 extra block
  (`if (param_2 == 6) { ... }` near function end): the extra cell is
  `param_1 (the OPPOSITE cell) + DAT_0089f690`. The opposite cell for dir 6 (West) is
  `anchor + ((6-4)&7=2 East) = anchor + 1 East`. `DAT_0089f690` is
  `g_DirectionOffsets[2] = East (1,0)` (g_DirectionOffsets base @0x0089F688, index 2).
  So the binary extra cell = `anchor + 1E + 1E = anchor + 2 East`.
- Fixture: anchor `0x19` at cell (5,5), family NESW, dir 6 (W).
  - bridge_facts.rs `stamp_slots` extra (mod path that builds `BridgeCellFacts`):
    `opposite=(6,5)` then `step((6,5), dir 2 East)` → `(7,5)` = anchor+2E. CORRECT — matches
    binary `EXTRA_SIDE 0x10000` write at (7,5). (Confirmed by the test
    `stamp_dir6_intact_sets_west_slots_and_two_east_slots` asserting `(7,5)` has
    `BRIDGE_FLAG_EXTRA_SIDE`, bridge_facts.rs:327.)
  - `walk_anchor_pattern` slot 5: `(anchor.x+1, anchor.y) = (6,5)` = anchor+1E. WRONG —
    the binary's extra-dir6 cell is at (7,5), not (6,5). (6,5) is already the opposite
    slot (slot 4).
  - Net: the AnchorSpan's slot-5 entry duplicates slot-4's cell (6,5) and never
    references the true extra cell (7,5). `bridge_facts.rs` and `walk_anchor_pattern`
    disagree on the same binary cell.
- Player sees: AnchorSpan slot 5 is not a `BLOW_UP_SLOT` (BLOW_UP_SLOTS=[0,1,2,4]) so this
  does not change which cells get `BlowUpBridge` on collapse, and the structural facts /
  collapse cascade run off `bridge_facts.rs` (correct). The observable impact is confined
  to span-driven repair iteration (`body_cell_repair_state`): the wrong cell (6,5),
  already covered as slot 4, is re-touched and the true extra cell (7,5) is never iterated
  by the span. (7,5) only ever carried `EXTRA_SIDE`/anchor-pointer in the binary, so the
  repair miss is low-visibility, but the slot list is structurally wrong vs the binary's
  6-cell pattern for every dir-6 (EW) high-bridge anchor on every map. Triggers at load of
  every EW high bridge; visible effect only on a span repair touching that span.
- Severity: LOW
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0047E040` (dir-6 extra block uses opposite-cell +
  DAT_0089f690=East); `get_xrefs_to 0x0089f688` confirms g_DirectionOffsets is the table
  indexed there; compared against `src/sim/bridge_state/mod.rs:2004-2011`.

---

### D2: dir-6 anchor's slot-4 / slot-5 distinction is collapsed; `walk_anchor_pattern` never models the "opposite + extra" pair correctly

- Rust now: For a fact_anchor with stamp dir 6 (overlay `0x19`/`0xEE`),
  `from_resolved_terrain` (mod.rs:643-656) derives `direction =
  bridge_stamp_direction_to_direction(6) = Direction::W` and `axis = EW`, then calls
  `walk_anchor_pattern(.., axis=EW, direction=W, ..)`. `walk_anchor_pattern` produces:
  slot0=anchor, slot1..3=+W×1/2/3, slot4=opposite(W)=+E×1, slot5=anchor+1E (the D1 bug).
- gamemd: `SetBridgeDirection_NESW(dir=6)` stamps exactly 6 cells: anchor, +W×1, +W×2,
  +W×3 (Forward1/2/3), +E×1 (opposite), +E×2 (extra). The intended Rust slot map is
  `[anchor, W1, W2, W3, E1(opposite), E2(extra)]`.
- Fixture: anchor (5,5) dir 6 → binary cells {(5,5),(4,5),(3,5),(2,5),(6,5),(7,5)}.
  Rust span cells = {(5,5),(4,5),(3,5),(2,5),(6,5),(6,5)} — slot5 collides with slot4 and
  (7,5) is absent. (This is the same root cause as D1, recorded separately because it is
  the span-construction contract — the 6-cell pattern the docstring at mod.rs:256-265 and
  AnchorSpan claim to model — not just the single slot value.)
- Player sees: same low-visibility surface as D1; called out separately so the fix touches
  the slot-5 offset to `anchor + 2E` (or `opposite + 1E`) rather than `anchor + 1E`.
- Severity: LOW
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0047E040` (six stamped slots for dir 6); contrasted
  with `src/sim/bridge_state/mod.rs:2004-2011`.

---

### D3: `anchor_walk_direction` for the legacy (non-fact) anchor path uses E/S, not the binary N/W stamp directions

- Rust now: `from_resolved_terrain` legacy branch (mod.rs:649-656) computes
  `direction = anchor_walk_direction(axis)`, where `anchor_walk_direction` (mod.rs:1965)
  returns `NS → Direction::E`, `EW → Direction::S`. The walker then stamps +E×1/2/3 / +W
  opposite (NS) or +S×1/2/3 / +N opposite (EW).
- gamemd: The map-load stamp only ever uses dir 0 (N) for the NS family (`0x18`/`0xED`)
  and dir 6 (W) for the EW family (`0x19`/`0xEE`) — verified in `OverlayClass__Mark
  @0x005FC570` (`0x18 → NESW(dir=0)`, `0x19 → NESW(dir=6)`, etc.). The forward direction
  is N for NS bridges and W for EW bridges, NOT E/S.
- Fixture: a legacy NS anchor at (5,5): binary stamps anchor + N×1/2/3 = (5,4),(5,3),(5,2)
  and opposite S = (5,6). Rust legacy walker stamps anchor + E×1/2/3 = (6,5),(7,5),(8,5)
  and opposite W = (4,5) — a 90° rotation of the span. The fact-anchor path (mod.rs:643-648,
  using `bridge_stamp_direction_to_direction`) is CORRECT (N/W); only the legacy fallback
  is rotated.
- Player sees: The legacy branch fires only when `bridge_facts.family == None` AND the
  cell has a `bridge_layer` anchor overlay (mod.rs:634-639). For real YR maps the
  high-bridge overlays `0x18/0x19/0xED/0xEE` always populate `bridge_facts.family` via the
  resolved_terrain stamp pass (resolved_terrain.rs:696-708), so `fact_anchor` wins and the
  legacy branch is normally dead for high bridges; it only governs test fixtures / synthetic
  `bridge_layer`-only data. Trigger frequency in a normal YR skirmish: effectively never
  for high bridges (fact path dominates); the rotation is latent until a map provides a
  bridge_layer anchor with no stamped facts.
- Severity: LOW
- Confidence: LIKELY-DRIFT (the rotation is proven; whether any real YR map reaches the
  legacy branch is unverified — it depends on overlay-vs-bridge_layer population that the
  fact pass normally satisfies first).
- Verify-call: `decompile_function 0x005FC570` (only dir 0 / dir 6 dispatched at map load);
  `decompile_function 0x0047E040` (N forward for dir 0, W forward for dir 6); compared with
  `src/sim/bridge_state/mod.rs:1965-1970`.

---

### D4: `is_anchor_overlay` treats every dir-marked high-bridge body cell as an anchor (legacy path only)

- Rust now: `is_anchor_overlay` (mod.rs:1958) returns true for `0x18 | 0x19 | 0xED | 0xEE`.
  The code comment (mod.rs:1954-1957) acknowledges this is over-broad: "every HIGH-bridge
  deck cell with a bridge_layer becomes an anchor."
- gamemd: At map load there is exactly ONE overlay-id-`0x18`/`0x19`/`0xED`/`0xEE` cell per
  bridge span (the anchor), and `SetBridgeDirection` sets bit `0x80` only on that single
  `this` cell (verified plate comment + `(param_3&1)<<7` at the anchor write only in
  `0x0047E040`). Body cells carry `0x100`/`0x200`/etc. but NOT `0x80`.
- Fixture: A 4-cell NS high bridge stamped from one `0x18` anchor at (5,5): only (5,5) has
  bit 0x80; the binary creates one anchor span. If a map (or the resolved overlay list)
  exposed `0x18` on multiple cells, the legacy predicate would create one AnchorSpan per
  such cell.
- Player sees: Latent only. The live `fact_anchor` path (mod.rs:633, `is_anchor_self()` =
  bit 0x80) correctly selects the single anchor and is checked first; `legacy_anchor`
  additionally requires `family == None` (mod.rs:634-636), which never holds for a stamped
  high-bridge cell. So in normal YR map load this over-broad predicate cannot fire on real
  high bridges. Flagged for completeness because the predicate itself does not match the
  binary's "one 0x80 anchor per span" rule and is a trap for future callers.
- Severity: LOW
- Confidence: LIKELY-DRIFT (over-broad predicate proven; masked by the `family == None`
  gate in normal play).
- Verify-call: `decompile_function 0x0047E040` (bit 0x80 written only on the single anchor
  `this`, plate comment confirms neighbor cells preserve 0x80 untouched); compared with
  `src/sim/bridge_state/mod.rs:1958-1960`.

---

## PARITY-CONFIRMED

These sub-aspects were checked live this session and match the binary:

1. **Overlay-id → (family, direction) dispatch.** `high_bridge_stamp_for_overlay`
   (bridge_facts.rs:81-89): `0x18→(Nesw,0)`, `0x19→(Nesw,6)`, `0xED→(Nwse,0)`,
   `0xEE→(Nwse,6)`. Matches `OverlayClass__Mark @0x005FC570`
   (`0x18→NESW(0)`, `0x19→NESW(6)`, `0xED→NWSE(0)`, `0xEE→NWSE(6)`). The binary's plate
   labels `0x18/0x19` "low" and `0xED/0xEE` "high", but the dispatch (NESW vs NWSE,
   dir 0 vs 6) is what the Rust mirrors; the label text is a stale annotation, not behavior.

2. **Per-slot `+0x140` flag writes for intact stamp (`state=1`).** Verified each slot's
   set/clear mask in `decompile_function 0x0047E040` against `stamp_intact`
   (bridge_facts.rs:128-177):
   - Anchor: binary `& 0xFFFEE07F | 0x100|0x200|0x1000|0x10000 | 0x80 | (dir0?0x800)`;
     Rust sets `ANCHOR_SELF|STRUCTURAL|TRANSITION|FORWARD_SIDE|EXTRA_SIDE` + dir0 0x800,
     clears DESTROYED_OR_RAMP(0x400). MATCH.
   - Forward1: binary sets 0x100|0x200|0x1000|0x10000 (+dir0 0x800), preserves 0x80.
     Rust matches (no 0x80, no 0x400). MATCH.
   - Forward2: binary OMITS 0x200 (`| uVar11 | uVar15 | uVar16`, no uVar12), clears 0x200.
     Rust clears `TRANSITION(0x200)|DESTROYED_OR_RAMP`, sets `STRUCTURAL|FORWARD_SIDE|EXTRA_SIDE`.
     MATCH.
   - Forward3: binary `& 0xFFFFEFFF | 0x1000` (touches only 0x1000). Rust sets only
     `FORWARD_SIDE`, no attach, no state-byte write. MATCH.
   - Opposite: binary clears 0x1000 (FORWARD_SIDE) and sets 0x100|0x200|0x10000 (+dir0 0x800).
     Rust clears `DESTROYED_OR_RAMP|FORWARD_SIDE`, sets `STRUCTURAL|TRANSITION|EXTRA_SIDE`.
     MATCH.
   - Extra dir6: binary `& 0xFFFEFFFF | 0x10000` + anchor-pointer write. Rust clears+sets
     `EXTRA_SIDE` and attaches. MATCH (flag bits; cell-position is the D1/D2 issue).

3. **`0x80` anchor fact is anchor-only.** Both bridge_facts.rs (only `Anchor` slot sets
   `BRIDGE_FLAG_ANCHOR_SELF`, tested at bridge_facts.rs:372-389) and the binary (bit 0x80
   via `(param_3&1)<<7` only on the `this`/anchor write) agree. Pass-2 tags exactly one
   `Anchor` role via `is_anchor_self()`.

4. **Default `+0x11E` state byte at stamp time.** Binary writes `field_0x11E = 0` (dir 0)
   or `9` (dir 6) on anchor/forward1/forward2/opposite. Rust `write_default_state`
   (bridge_facts.rs:224-226) writes `0` if dir==0 else `9` on the same slots; Forward3 and
   Extra get no state-byte write. MATCH.

5. **Map-load OverlayDataPack overwrite of state byte.** Binary: `SetBridgeDirection`
   default first, then `[OverlayDataPack]` writes the final `+0x11E` per cell. Rust
   resolved_terrain.rs:711-715 applies `state_byte = overlay_data_at(rx,ry)` after the
   stamp pass, gated on `has_overlay_data_pack()`. Same order. When no data pack exists,
   the stamp default (0 / 9) survives and `initial_bridge_damage_state`
   (mod.rs:1910-1919) decodes both to `Healthy{variant:0}`. MATCH.

6. **`0x800` direction-zero flag polarity.** Binary `uVar13 = (param_2==0)<<0xB`
   (set for dir 0 / NS, clear for dir 6 / EW). Rust `set_direction_zero_flag` keyed on
   `relation.direction == 0` (bridge_facts.rs:129,228-234). MATCH. `BRIDGE_FLAG_DIRECTION_ZERO
   = 0x800` (bridge_facts.rs:7).

7. **axis classification from stamp direction.** `bridge_stamp_direction_to_axis`
   (mod.rs:1921-1926): dir 2|6 → EW, else NS. Consistent with binary
   `0x18(dir0)→NS`, `0x19(dir6)→EW` and the state-byte ranges (NS 0-8, EW 9-17).

8. **Latin-square variant jitter index + range.** Binary `DrawOverlay_Body @0x0047F6A0`:
   `if (state==0 || state==9) state += g_LatinSquare[((cell.Y&3)<<2 | (cell.X&3))]`, table
   `{0,1,2,3,3,2,1,0,2,3,0,1,1,0,3,2}` (range 0..3). Rust
   `compute_bridge_body_shp_frame` (app_instances/bridges.rs:64-85) uses
   `BRIDGE_BODY_LATIN_SQUARE` (identical 16 bytes, bridges.rs:29) indexed by
   `((ry&3)<<2)|(rx&3)`, applied only for `Healthy{variant:0}` at base byte 0/9. Index
   order, table contents, and gate condition MATCH. (+0x26 is the Y high-half of packed
   coord +0x24, +0x24 low is X — confirmed in the decompile.)

9. **High-bridge deck-level delta = ground + 4.** Rust sets `bridge_deck_level =
   level + 4` for structural cells (resolved_terrain.rs:723) and the render bonus is +4
   (BRIDGE_HEIGHT_BONUS, bridges.rs:41). Consistent with the verified bridge-deck height
   semantics (`Get_Effective_Height @0x005F5F00` returns +4 when OnBridge;
   `CheckBridgeTraversal` uses diff_abs==4 as the bridge entry/exit delta). Note this is a
   render/traversal height delta, not a `cell+0x10E` map-load field (see
   HIGH_BRIDGE_UNDER_DECK_OCCLUSION report's CORRECTION) — the Rust derives it as a
   compatibility view, which matches observable height.

10. **bridgehead_anchor_class_at_load pre-classification.** resolved_terrain.rs:955-976
    matches BridgeSet-tileset cells against `BridgeAnchorVariantTable`; sim reads it in
    pass 1 (mod.rs:609-611) to seed `bridgehead_anchor_class`. The 4 NS / 4 EW variant
    tile-ids and the "author pre-damaged anchor renders from frame 1" intent are consistent
    with the bridgehead state-machine class slots; the table contents themselves were not
    re-derived from theater data this session (see UNCHECKED #2).

---

## UNCHECKED

1. **Low-bridge map-load tile selection (`SelectBridgeTileVariant_Low @0x0057ACF0`).**
   Decompiled this session: it is a runtime tile-variant selector driven by
   `ComputeBridgeSurfaceMask` + a `Random__Next() & 3` / `%3` jitter and writes overlay ids
   via `ApplyBridgeTile`. This is the LOW bridge family; the facet's high-bridge map-load
   stamp model (`SetBridgeDirection`) does not run for low bridges. The Rust low-bridge
   path is tube-backed (`build_auto_low_bridge_tubes`, resolved_terrain.rs:980) and was not
   compared cell-by-cell against this selector here — it is a separate facet. The
   `Random__Next()` draws in this function are a determinism surface for low-bridge runtime
   pavement re-tiling (and the `0x7A`/`0xE9` overlay-range walkers inside
   `OverlayClass__Mark` also draw `Random__Next() & 3`), not covered by this scan.

2. **BridgeAnchorVariantTable / BridgeRampTileTable tile-id contents.** The Rust derives
   the 8 anchor variant tile-ids and the high-bridge ramp tile-ids from theater data at
   runtime (resolved_terrain.rs:742-773, 955-976). I confirmed the wiring exists and feeds
   `bridge_facts.ramp_tile` and `bridgehead_anchor_class_at_load`, but did not re-derive the
   exact BridgeSet-relative offsets against the binary's `g_ShorePieces`/`DAT_008333xx`
   tables this session.

3. **`g_DirectionOffsets` literal bytes.** `read_memory 0x0089F688` returned all zeros in
   this Ghidra image (table is runtime-populated / in an uninitialized section), so I could
   not read the literal (dx,dy) entries. I relied on the verified doc convention (0=N(0,-1),
   2=E(1,0), 4=S(0,1), 6=W(-1,0)) plus the decompile's `(param_2-4)&7` opposite math and
   `DAT_0089f690 = base+8 = index 2 = East` to resolve D1/D2. The N/E/S/W mapping itself is
   doc-sourced, not byte-read this session.
