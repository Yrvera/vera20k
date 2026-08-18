# High Bridge SetBridgeDirection Stamping - Ghidra Report

Date: 2026-05-15

Scope: close the Priority 1 design question from
`docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`: whether current
binary/data evidence is sufficient to replace broad Rust bridge inference with a
`CellClass::SetBridgeDirection`-equivalent fact stamping model.

No Rust code was changed.

## Executive result

There is enough binary evidence to design Priority 1 for high-bridge cell facts.

`gamemd.exe` does not derive high-bridge bridge facts from a resolved-terrain
component pass that expands side cells, normalizes deck height, or fills bridge
gaps. The live map-load path constructs overlay objects from `[OverlayPack]`.
For high-bridge overlay IDs, `OverlayClass::Mark` calls
`CellClass::SetBridgeDirection_NESW` or `CellClass::SetBridgeDirection_NWSE`.
Those functions stamp a small, fixed set of cells relative to the overlay cell,
writing specific bridge bits, an anchor pointer, and default bridge state bytes.
Then `[OverlayDataPack]` writes the final per-cell `+0x11E` state byte.

For Priority 1, the conservative implementation shape should be:

- represent first-class map-load bridge facts instead of flattened
  `has_bridge_deck` / `bridge_walkable` inference;
- stamp facts only from map overlay records that match the verified high-bridge
  overlay IDs;
- preserve separate fields for raw bridge flags, state byte, direction, anchor
  relation, and overlay family;
- derive the old flattened booleans only as temporary compatibility views for
  current sim/render consumers.

## Verified functions

Primary functions inspected in Ghidra:

| Function | Address | Role |
|---|---:|---|
| `CellClass__SetBridgeDirection_NESW` | `0x0047E040` | High/low bridge direction stamp helper |
| `CellClass__SetBridgeDirection_NWSE` | `0x0047E470` | Compiled twin of the NESW helper |
| `OverlayClass__Mark` | `0x005FC570` | Map overlay object marking; calls SetBridgeDirection for high bridge overlay IDs |
| `ReadMapOverlayPacks` | `0x005FD2E0` | Reads `[OverlayPack]` then `[OverlayDataPack]` |
| `CellClass__RecalcAttributes` | `0x0047D2B0` | Recalculates terrain attributes; does not derive high-bridge stamp bits |
| `MapClass__Resize` | `0x00565C10` | Repair/resize pass that can re-stamp existing bridge cells |
| `MapClass__UpdateBridgeEdgeTiles_High` | `0x00576200` | Runtime bridge damage/repair edge update path |

Caller/xref check for `SetBridgeDirection_NESW`:

- `OverlayClass__Mark` at `0x005FC5FE`, `0x005FC60A`
- `MapClass__Resize` at `0x00567078`
- high-bridge runtime update/damage functions:
  `MapClass__UpdateBridgeEdgeTiles_High`,
  `MapClass__UpdateRamp_*_High`, and
  `ProcessBridgeDamageStateMachine_High`

Caller/xref check for `SetBridgeDirection_NWSE`:

- `OverlayClass__Mark` at `0x005FC62C`
- `MapClass__Resize` at `0x0056706C`
- low-bridge runtime update/damage functions:
  `MapClass__UpdateBridgeEdgeTiles_Low`,
  `MapClass__UpdateRamp_*_Low`, and
  `ProcessBridgeDamageStateMachine_Low`

Active in YR: yes. `OverlayClass::Mark` is on the normal map overlay-load path.
`MapClass::Resize` is active code but is a repair/resize path, not the primary
source of normal skirmish map-load bridge facts.

## Cell fields and bits

Fields touched by the verified stamp path:

| Field | Meaning in verified path |
|---|---|
| `CellClass + 0x24` | packed map coordinate |
| `CellClass + 0x2C` | bridge anchor pointer / partner pointer |
| `CellClass + 0x44` | overlay type index |
| `CellClass + 0x11E` | bridge direction/damage state byte |
| `CellClass + 0x140` | bridge and terrain flags |

Verified flag bits in `+0x140`:

| Bit | Verified behavior in this pass |
|---:|---|
| `0x80` | Set only on the anchor cell by `SetBridgeDirection`; other stamped cells preserve their prior value |
| `0x100` | Set on anchor, forward slot 1, forward slot 2, and opposite slot |
| `0x200` | Set on anchor, forward slot 1, and opposite slot; explicitly clear/not set on forward slot 2 |
| `0x400` | Set by the destroy path (`state == 0`) on anchor, forward slots 1 and 2, and opposite slot |
| `0x800` | Set only when `direction == 0`; cleared otherwise on the cells that rewrite it |
| `0x1000` | Set on anchor, forward slots 1-3, not on the opposite slot |
| `0x10000` | Set on anchor, forward slots 1-2, opposite slot, and the extra `direction == 6` slot |

The important correction is that `0x80` is not expanded sideways or across the
whole stamp by this function. It is written on the overlay cell passed as
`this`. Neighbor cells only have `0x80` if they already had it, or if they are
also processed as overlay cells in their own `OverlayClass::Mark` call.

## Map-load flow

`ReadMapOverlayPacks` at `0x005FD2E0` reads two packed sections in order:

1. `[OverlayPack]`
2. `[OverlayDataPack]`

During `[OverlayPack]`, each non-`0xFF` overlay byte constructs an
`OverlayClass` at the cell coordinate. `OverlayClass::Mark` then marks the cell.
For high-bridge overlay IDs, it calls `SetBridgeDirection_*`.

The verified calls in `OverlayClass::Mark` are:

| Overlay ID | Call |
|---:|---|
| `0x18` | `SetBridgeDirection_NESW(direction=0, state=1)` |
| `0x19` | `SetBridgeDirection_NESW(direction=6, state=1)` |
| `0xED` | `SetBridgeDirection_NWSE(direction=0, state=1)` |
| `0xEE` | `SetBridgeDirection_NWSE(direction=6, state=1)` |

Assembly context:

- `0x005FC5FE`: `PUSH 0x1`, `PUSH 0x6`, `MOV ECX,EBP`,
  `CALL 0x0047E040`
- `0x005FC60A`: `PUSH 0x1`, `PUSH EDI` where `EDI == 0`,
  `MOV ECX,EBP`, `CALL 0x0047E040`
- `0x005FC62C`: `PUSH 0x1`, `PUSH 0x6` or `PUSH EDI`,
  `MOV ECX,EBP`, `CALL 0x0047E470`

For those bridge IDs, `ReadMapOverlayPacks` saves and restores `+0x11E` around
the overlay object construction during the first pass. The second pass,
`[OverlayDataPack]`, then writes the final byte to `cell + 0x11E` for every
in-bounds cell. Therefore map-load final bridge state bytes are data-driven by
`[OverlayDataPack]`, even though `SetBridgeDirection` writes default bytes during
the mark call.

`CellClass::RecalcAttributes` does not derive these high-bridge bits. It has
low-bridge/tube behavior and terrain recalculation, but not a replacement for
the high-bridge stamp.

## SetBridgeDirection stamp pattern

Both `SetBridgeDirection_NESW` and `SetBridgeDirection_NWSE` have the same
compiled body shape. The distinction is which caller chooses which helper for
which overlay family; the internal walk uses the same 8-direction table indexed
by the `direction` parameter.

The helper receives:

- `this`: anchor cell
- `direction`: normal map-load uses `0` or `6`
- `state`: `1` for intact/create, `0` for destroy

The direction table follows the standard 8-facing map offsets used by the Rust
`Direction` enum:

| Direction | Offset |
|---:|---|
| `0` | north `(0, -1)` |
| `2` | east `(1, 0)` |
| `4` | south `(0, 1)` |
| `6` | west `(-1, 0)` |

Slot positions:

| Slot | Coordinate relation |
|---|---|
| Anchor | `cell` |
| Forward 1 | `cell + direction` |
| Forward 2 | `cell + 2 * direction` |
| Forward 3 | `cell + 3 * direction` |
| Opposite | `cell + ((direction - 4) & 7)` |
| Extra `direction == 6` | one additional east step from the opposite slot, observed in assembly via `DAT_0089F690` |

For normal map-load directions this becomes:

| Direction | Slots |
|---:|---|
| `0` | anchor, north 1, north 2, north 3, south 1 |
| `6` | anchor, west 1, west 2, west 3, east 1, east 2 |

The extra `direction == 6` slot is verified in both helpers:

- NESW: `0x0047E3FF` compares direction to `6`; `0x0047E406-0x0047E452`
  applies the additional cell write.
- NWSE: `0x0047E82F` compares direction to `6`; `0x0047E836-0x0047E882`
  applies the additional cell write.

## Intact-state stamp table

For `state=1`, the helper computes:

- `0x100` from `(state & 1) << 8`
- `0x200` from `(state & 1) << 9`
- `0x1000` from `(state & 1) << 12`
- `0x10000` from `(state & 1) << 16`
- `0x800` only when `direction == 0`

Per-slot behavior for normal intact stamping:

| Slot | Anchor pointer | `+0x11E` write | Flags set | Flags cleared/forced off |
|---|---|---|---|---|
| Anchor | unchanged | `0` if dir 0, `9` otherwise | `0x80`, `0x100`, `0x200`, `0x1000`, `0x10000`, plus `0x800` if dir 0 | `0x400`; `0x800` if dir != 0 |
| Forward 1 | anchor pointer | `0` if dir 0, `9` otherwise | `0x100`, `0x200`, `0x1000`, `0x10000`, plus `0x800` if dir 0 | `0x400`; `0x800` if dir != 0; preserves `0x80` |
| Forward 2 | anchor pointer | `0` if dir 0, `9` otherwise | `0x100`, `0x1000`, `0x10000`, plus `0x800` if dir 0 | `0x200`, `0x400`; `0x800` if dir != 0; preserves `0x80` |
| Forward 3 | unchanged | none | `0x1000` | no other verified bridge-bit rewrite |
| Opposite | anchor pointer | `0` if dir 0, `9` otherwise | `0x100`, `0x200`, `0x10000`, plus `0x800` if dir 0 | `0x400`, `0x1000`; `0x800` if dir != 0; preserves `0x80` |
| Extra dir 6 | anchor pointer | none | `0x10000` | clears prior `0x10000` first; preserves other flags |

The helper marks terrain dirty after the anchor, forward 1, forward 2, and
opposite writes. Forward 3 and the extra dir-6 cell are flag-only writes in this
function.

## Destroy-state table

For `state=0`, the helper clears the intact bits listed above and sets `0x400`
on the anchor, forward 1, forward 2, and opposite slots. It also clears the
anchor pointer on slots that receive one in intact state.

Per-slot destroy behavior:

| Slot | Anchor pointer | `+0x11E` write | Flags set | Flags cleared/forced off | BlowUpBridge |
|---|---|---|---|---|---|
| Anchor | unchanged | `0` | `0x400` | `0x80`, `0x100`, `0x200`, `0x800`, `0x1000`, `0x10000` | yes |
| Forward 1 | `0` | `0` | `0x400` | `0x100`, `0x200`, `0x800`, `0x1000`, `0x10000`; preserves `0x80` | yes |
| Forward 2 | `0` | `0` | `0x400` | `0x100`, `0x200`, `0x800`, `0x1000`, `0x10000`; preserves `0x80` | yes |
| Forward 3 | unchanged | none | none | `0x1000` only | no |
| Opposite | `0` | `0` | `0x400` | `0x100`, `0x200`, `0x800`, `0x1000`, `0x10000`; preserves `0x80` | yes |
| Extra dir 6 | `0` | none | none | `0x10000` only | no |

This matters for current Rust collapse side effects: the binary does not treat
every relative cell in the stamp as a `BlowUpBridge` cell.

## What Priority 1 must represent separately

The Rust replacement should not collapse these into a single
`bridge_walkable: bool` too early.

Minimum separate facts:

| Fact | Why it must remain separate |
|---|---|
| `flags & 0x80` | It is an anchor/self marker written only on the overlay cell by this function; broad expansion would be wrong. |
| `flags & 0x100` | Used as a structural bridge-body/passability bit in later binary paths, but not equivalent to `0x80`. |
| `flags & 0x200` | Present on some stamped slots and absent on forward slot 2; cannot be derived from `0x100`. |
| `flags & 0x400` | Destroy/ramp-style bit set by destroy stamping; not the same as absence of bridge. |
| `flags & 0x800` | Direction-dependent bit, set for direction 0 only. |
| `flags & 0x1000` | Present on anchor/forward side, absent on opposite; also forward 3 only touches this bit. |
| `flags & 0x10000` | Present on most intact slots and the dir-6 extra slot; used independently by the helper. |
| `+0x11E` bridge state byte | Final map-load byte comes from `[OverlayDataPack]`, not from broad overlay type inference. |
| Anchor pointer relation | Several non-anchor slots store a pointer to the anchor; `flags & 0x80` selects different later behavior. |
| Direction | Normal map-load uses `0` and `6`; resize repair can use `0` or `2`; later runtime callers also depend on direction. |
| Overlay family/helper kind | `0x18/0x19` call NESW; `0xED/0xEE` call NWSE. The helper bodies match, but retaining family is useful for render/theater and later damage behavior. |

The immediate map representation should therefore resemble a per-cell
`BridgeCellFacts` record containing raw stamped bits and raw state byte, with
optional anchor metadata:

- raw bridge flags from the stamp;
- raw overlay ID;
- raw state byte from `[OverlayDataPack]`;
- stamp family (`NESW` or `NWSE`);
- stamp direction (`0`, `6`, and future-proof for `2`);
- role/slot relative to an anchor only when proven by the stamp;
- anchor coordinate for cells whose binary pointer would point at the anchor.

The existing flattened fields can be derived from this as compatibility:

- `has_bridge_deck` should be a view over the verified bridge-body bits and
  runtime state, not a terrain-wide inference pass;
- `bridge_walkable` should remain a consumer-facing traversal capability, not
  the source of bridge facts;
- `bridge_transition` should come from verified bridgehead/ramp evidence, not
  broad `BridgeSet`/`WoodBridgeSet` membership alone.

## Current Rust mismatch

Current Rust still builds bridge facts in `src/map/resolved_terrain.rs` from
broad resolved-terrain inference:

- `classify_overlay_effects` uses `is_bridge_overlay_index` to create bridge
  facts from broad overlay IDs.
- The grid builder expands side cells for high bridges.
- It normalizes connected high-bridge deck levels to the component maximum.
- It marks bridgehead transition cells from `BridgeSet` / `WoodBridgeSet`
  membership and borrows deck height from nearby bridge cells.
- It gap-fills one-cell gaps between inferred bridge deck cells.

Current consumers then rely on the flattened booleans:

- `BridgeRuntimeState::from_resolved_terrain` builds runtime bridge cells and
  anchor spans from `has_bridge_deck` / `bridge_walkable` shaped terrain data.
- `PathGrid::from_resolved_terrain_with_bridges` writes `PathCell.bridge_walkable`
  from resolved terrain or runtime bridge state.
- `movement_bridge` uses `bridge_walkable` and transition booleans for
  locomotor transition predicates.

These are reasonable scaffolding, but they are not equivalent to the verified
`gamemd.exe` source of truth.

## Conservative implementation shape

Recommended Priority 1 design:

1. Add a first-class map-load bridge fact pass adjacent to overlay parsing /
   resolved terrain construction. It should consume real map overlay IDs and
   overlay data bytes.
2. Implement a data-only `stamp_set_bridge_direction(anchor, family, direction,
   state)` equivalent using the verified table above. For map load, call it with
   `state=1` for IDs `0x18`, `0x19`, `0xED`, and `0xEE`.
3. Apply `[OverlayDataPack]` state bytes after stamping, matching the binary
   load order.
4. Store raw per-cell bridge facts in `ResolvedTerrainCell` or a parallel grid.
   Keep sim/render layering intact by making this a map data product consumed by
   sim and render, not render logic inside sim.
5. Derive existing compatibility fields from the new facts while migrating
   consumers:
   - `has_bridge_deck` from stamped structural bits, not side/gap inference;
   - `bridge_walkable` as a traversal view;
   - `bridge_transition` from later verified bridgehead/ramp facts only.
6. Remove or gate the current unverified inference passes for high bridges:
   side-cell expansion, deck-height normalization, and gap-fill.
7. Keep low bridges separate. Low bridge pathing is tube/land-type backed per
   `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`; do not fold low bridge
   overlay families into this high-bridge stamp model without a separate proven
   path.

## Remaining unknowns

No additional Ghidra pass is required before coding Priority 1 for high-bridge
map-load fact stamping. The high-confidence facts are the overlay IDs, call
order, stamp directions, per-slot flag writes, anchor pointer writes, and final
OverlayDataPack state-byte overwrite.

Remaining items that should be verified before broader bridge rewrites:

- Real stock-map data dump: confirm which high-bridge overlay cells exist in
  retail maps and how `[OverlayDataPack]` bytes line up with stamped slots.
  This is a data validation pass, not a blocker for implementing the binary
  stamping function.
- Bridgehead/ramp transition facts: this report verifies that broad
  `BridgeSet`/`WoodBridgeSet` membership is too broad, but exact Priority 1
  bridgehead replacement should use the already verified ramp/tile-index
  predicates from prior reports, or a focused follow-up if the implementation
  needs more detail.
- Low bridges: keep them out of the high-bridge stamp replacement unless a
  separate task explicitly implements the tube-backed low-bridge model.
- Runtime repair/damage callers: this report captures the shared stamp table,
  but Priority 1 should avoid changing collapse/repair behavior except where it
  consumes the new authoritative initial facts.

## Design verdict for Priority 1

Proceed with design based on this report and the two parent reports:

- `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`

The binary evidence is strong enough to replace broad high-bridge inference with
`SetBridgeDirection`-equivalent stamping for authoritative bridge cell facts.
The implementation should preserve raw bridge bits and state bytes first, then
adapt existing sim/render consumers behind compatibility accessors.
