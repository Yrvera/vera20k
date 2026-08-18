# G4 Bridge Damaged Variant — Design

## Goal

When a bridge takes damage that doesn't immediately destroy it, swap the affected bridge tiles to the baked "damaged" variant baked into the TMP files, and clear the swap back to pristine when an engineer enters a CABHUT to repair the bridge.

## Architecture Context

### gamemd.exe — the mechanism this design reproduces

The original engine uses `CellClass.Flags` bit 13 (`0x2000`) per cell to flag "render the damaged variant of this tile instead of the pristine one." The bit is propagated via the 8-neighbor flood-fill `MapClass::ToggleBridgePavement` @ `0x0056E990`, gated by the TMP per-cell flag DWORD at `+0x24` bit 2 (`HAS_DAMAGED_DATA`), and read at draw time by `CellOverlay_TileDraw` @ `0x00480350` and `CellClass::GetRadarPixelColor` @ `0x0047BDB0`. The damaged-variant pixel data lives in a **separate TMP file** with a letter suffix (`<base>NN.TEM` pristine, `<base>NNA.TEM` damaged); each variant becomes its own `IsometricTileTypeClass` instance, linked into a singly-linked chain via the `+0x2BC` (`next_variant`) pointer. `TMP_TileBlitter` walks the chain by `variant_index` steps before blitting. Damage AND collapse paths both pass `state=1`; only repair-walker paths pass `state=0`.

Verified RE references:
- [LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md §4](../../../ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md) — propagation rules, caller pattern, gate
- [TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md) — TMP file layout, variant chain, theater loader
- [ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md §14, §16, §17](../../../ra2-rust-game-docs/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md) — TMP_TileBlitter chain walk, GetTileVariantIndex PRNG, CellOverlay_TileDraw pipeline

### Rust — existing infrastructure to integrate with

**Sim layer:**
- [src/sim/bridge_state/mod.rs:403](../../../src/sim/bridge_state/mod.rs#L403) — `BridgeRuntimeCell.damaged_variant: bool` field already declared; currently always written `false` at ~24 construction sites.
- [src/sim/bridge_state/mod.rs:780](../../../src/sim/bridge_state/mod.rs#L780) — `body_cell_advance_state` is the body-cell damage entry point; on Healthy→Damaged, it currently fires `update_ramp_perpendicular(DamageA + DamageB)`. No `damaged_variant` writer yet.
- [src/sim/bridge_state/mod.rs:1111](../../../src/sim/bridge_state/mod.rs#L1111) — `bridgehead_advance_state` is the bridgehead damage entry point. Same — no writer.
- [src/sim/bridge_state/mod.rs:978](../../../src/sim/bridge_state/mod.rs#L978) — `body_cell_repair_state` is the engineer-CABHUT repair path that transitions cells back to Healthy. No `damaged_variant` clear yet.
- [src/sim/bridge_specs.rs:533](../../../src/sim/bridge_specs.rs#L533) — `update_ramp_perpendicular` is the helper that mirrors `UpdateRamp_*_High/Low` per-phase writes. The binary's `ToggleBridgePavement(state=1)` call happens INSIDE these UpdateRamp functions; this is the natural Rust hook point.
- [src/sim/world/world_hash.rs:233](../../../src/sim/world/world_hash.rs#L233) — `damaged_variant` already hashed for determinism; no state-hash work needed.

**Asset/render layer (key finding — most of subsystem B is already done):**
- [src/assets/tmp_decode.rs:21,68](../../../src/assets/tmp_decode.rs#L21) — `FLAG_HAS_DAMAGED_DATA = 0x04` parsed from each TMP cell's `+0x24` flag DWORD; exposed as `TmpTile.has_damaged_data: bool`.
- [src/map/theater.rs:210](../../../src/map/theater.rs#L210) — `variant_filenames(tile_id) -> &[String]` returns letter-suffixed variant TMP filenames.
- [src/map/theater.rs:660-703](../../../src/map/theater.rs#L660-L703) — variant TMPs ALREADY loaded; each gets a `TileKey { tile_id, sub_tile, variant: var_idx+1 }` entry in the global tile-image map. The damaged variant is variant index 1.
- [src/render/tile_atlas.rs](../../../src/render/tile_atlas.rs) — `TileKey` already includes a `variant: u8` axis; atlas already packs all variants.
- [src/map/resolved_terrain.rs:864-887](../../../src/map/resolved_terrain.rs#L864) — map-load randomly picks a `cell.variant` for cells with `variant_count > 0`. The comment at line 866 claims `HasDamagedData` tiles are "excluded" but the code doesn't enforce — safe-by-accident because retail bridge tiles don't ship FA2 visual-diversity variants. **This guard must become explicit** so the per-frame `damaged_variant` bool can drive variant pick without map-load PRNG interference.

**Render-side bridge consumer:**
- `BridgeRuntimeCell.damaged_variant` is **not read by any code in `src/render/`** today. The bridge body draw site (whose location is currently part of the normal terrain draw path, NOT [src/render/bridge_atlas.rs](../../../src/render/bridge_atlas.rs) which handles SHP overlays) must be located during /write-plan and wired to query the bool.

## Impact Analysis

| File | Change | Risk |
|------|--------|------|
| [src/sim/bridge_state/mod.rs](../../../src/sim/bridge_state/mod.rs) | New `apply_damaged_variant_flood_fill` method (~80 LOC) + 1 line in `body_cell_repair_state` | Determinism critical — flood-fill iteration order must be locked |
| [src/sim/bridge_state/mod.rs](../../../src/sim/bridge_state/mod.rs) | `body_cell_advance_state` / `body_cell_repair_state` / `bridgehead_advance_state` signatures gain `&ResolvedTerrainGrid` parameter | Caller-side: `update_ramp_perpendicular` users; mechanical |
| [src/sim/bridge_specs.rs](../../../src/sim/bridge_specs.rs) | `update_ramp_perpendicular` adds a flood-fill call on each phase write | Hook site — must match binary state-arg pattern exactly |
| [src/sim/world/](../../../src/sim/world/) | Wire the `&terrain` borrow through to all bridge_state callers | Possible borrow-checker friction; pre-existing pattern in `bridgehead_advance_state` suggests it's tractable |
| [src/map/theater.rs](../../../src/map/theater.rs) | New `tile_has_damaged_data(tile_id, sub_tile) -> bool` accessor | New API; no migration |
| [src/map/resolved_terrain.rs:864-887](../../../src/map/resolved_terrain.rs#L864) | Add explicit `if has_damaged_data { continue; }` guard in the map-load variant pick | Source-only fix; today's behavior is the same in practice |
| Bridge body render call site (TBD) | Read `damaged_variant` and select `TileKey { variant: damaged_variant as u8 }` | Render integration — location must be located during /write-plan |
| [src/sim/world/world_orders_bridge_repair_tests.rs](../../../src/sim/world/world_orders_bridge_repair_tests.rs) | Extend tests to cover `damaged_variant` clear on repair | Test extension; no production risk |
| New unit tests in `bridge_state` test module | ~6 tests for flood-fill algorithm correctness | Test addition; no production risk |

**Blast radius — sim:** contained to `bridge_state` and `bridge_specs`. The flood-fill is bounded by `tile_id` equality; practical region sizes <20 cells.

**Blast radius — render:** one bridge body draw site (TBD location during /write-plan); other consumers of `TileKey.variant` (terrain visual-diversity, MarbleMadness) unaffected.

**Determinism:** flood-fill order is locked to 8-direction fixed sequence. `damaged_variant` already in state hash → lockstep-safe.

**TS-legacy check:** The `cell.Flags & 0x2000` bit in the binary has a SECOND use (cleared by `SetBridgeDirection`) that is TS-dead per AUDIT_LOG 2026-05-12 (no readers). The render-side use of the bit IS confirmed live in YR (verified `CellOverlay_TileDraw` and `GetRadarPixelColor` callers in this session). Distinct uses of the same bit; render-side is the parity-relevant one for G4.

## Chosen Approach

**Recursive flood-fill, immediate execution, no visited-set, render-side `TileKey.variant` axis driven by `damaged_variant: bool`.** This mirrors the binary algorithm 1:1, runs the flood-fill on the same tick as the damage event (matching the binary's immediate execution), and leverages the existing Rust asset pipeline (which already loads variant TMPs and supports variant lookup via `TileKey`). Subsystem B's scope is small because the asset infrastructure is already in place.

**Why not BFS:** Diverges from binary's recursion; visited-set is dead weight given the idempotency early-return; deferred execution would mismatch the binary's same-tick semantics.

**Why not hoisting `damaged_variant` to `ResolvedTerrainCell`:** Speculative generalization. G4 has one verified use; the next-most-likely use (`SetOverlayAndPropagate` for bridge collapse tile swap) was already deferred in a prior brainstorm. YAGNI applies.

## Tiny-Detail Ledger

This is a constraint set for `/write-plan` and implementation. Every item is sourced; every approach in this design must explicitly preserve each one.

1. **State arg = 1 from damage AND collapse callers**; state arg = 0 ONLY from repair walkers. [GHIDRA UpdateRamp_*_DamageA/B + UpdateRamp_*_CollapseA/B all `PUSH 0x1`; FUN_00569760/FUN_00568E40 all `PUSH 0x0`]
2. **Gate `HasDamagedVariantAtSubTile` runs ONLY on `suppress_self == 0`** (kickoff call); recursive calls skip the gate. [GHIDRA 0x0056E990 — `if ((char)param_3 == '\0') { ... gate ... }`]
3. **Propagation criterion: `neighbor.IsoTileTypeIndex == seed.IsoTileTypeIndex`** — neighbor's CURRENT tile_id compared to the SEED cell's tile_id (NOT the changing cell's). [GHIDRA `iVar3 = *(int *)(puVar5 + 0x38)` captured before the loop]
4. **Bit-set semantics: clear-then-set, not XOR** — `Flags = (Flags & ~0x2000) | ((state & 1) << 13)`. In Rust we use `bool`, so this is trivially correct: just assign. [GHIDRA 0x0056E990 mask `0xFFFFDFFF`]
5. **Idempotency: if current bit already matches new state, skip mutation AND skip recursion** entirely (early return before flood). [GHIDRA `if ((byte)param_2 != ((byte)(... >> 0xd) & 1))`]
6. **Off-map / null-cell sentinel returns early** at every recursion level. [GHIDRA bounds check + null-pointer fallback]
7. **IsoTileTypeIndex sentinels `0xFFFF` and `0xFF`** cause early return on the kickoff call (clear / empty cells). [GHIDRA two consecutive `if (...== 0xffff) return; if (...== 0xff) return;`]
8. **8-neighbor walk (cardinals + diagonals)**; recursion uses fixed direction order matching `g_DirectionOffsets`. [GHIDRA `iVar6 = 8; do {...} while (--iVar6 != 0)`]
9. **Damage-variant bit persists through Healthy → Damaged → Destroyed**; only explicit repair clears it. [doc §4.4 caller table — collapse passes state=1, only repair passes state=0]
10. **Render-time consumer:** when `HasDamagedVariantAtSubTile` returns true, variant index = bit13 (0 or 1); when false, fallback to PRNG `GetTileVariantIndex`. [GHIDRA `CellOverlay_TileDraw` 0x00480374, `CellClass::GetRadarPixelColor` 0x47BFF5]
11. **PRNG variant jitter is LOST while in damaged state** — the binary returns the damaged-bit early, before reaching the PRNG selector. Rust must enforce the same priority. [doc §4.3]
12. **Render gate ALSO checks `tile_type.VariantCount >= 2` first** — single-variant tiles always return variant 0 regardless of bit. [GHIDRA `if (piVar5[0xbc] < 2) goto LAB_00480403;`]
13. **Repair clear semantics:** `damaged_variant` set to false on EVERY cell that transitions to Healthy in `body_cell_repair_state`. Each cleared cell fires a kickoff flood-fill (state=false) to propagate the clear across its tile_id region. [GHIDRA FUN_00569760 / FUN_00568E40 walker structure]
14. **TMP capability flag `bit 0x04` on per-cell flag DWORD at `+0x24`** is the gate — Rust already exposes this as `TmpTile.has_damaged_data`. [src/assets/tmp_decode.rs:21,68]
15. **Tick-stage placement:** writer fires inside Phase F (combat / damage application) on the same tick as the damage event; render reads it on the next frame (no sub-tick coupling). [Rust convention — sim mutations inside `World::advance_tick`]
16. **Damaged variant pixel data layout (RESOLVED):** stored in a separate `.TEM` file with `'a'` suffix; Rust theater loader already loads it as `TileKey { variant: 1 }`. [TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md §1, §4]
17. **VariantCount modulo:** the blitter does `variant_index % VariantCount`, so `damaged_variant: bool as u8` (always 0 or 1) safely modulos to a valid variant even for single-variant tiles. No need for the Rust render code to check `variant_count >= 2` before applying — but it's still correct to do so as a fast-path. [GHIDRA 0x00547CF0 modulo line; theater.rs already loads variants 1..N]
18. **Map-load `cell.variant` PRNG pick** at [resolved_terrain.rs:874](../../../src/map/resolved_terrain.rs#L874) currently runs for ALL tiles with `variant_count > 0`. Bridges with `has_damaged_data` should be excluded so the per-frame bool can drive variant pick. The comment at line 866 already states the intent; the code must be made explicit. [Source-only finding]
19. **Radar/minimap rendering** at `GetRadarPixelColor` (0x47BDB0) uses the SAME chain walk and `(Flags >> 13) & 1` pick. If/when the Rust minimap path queries tile color from variants, it must also honor `damaged_variant`. [GHIDRA 0x47BFF5]
20. **The pristine file MUST exist; engine doesn't fall back to variant `'a'`** if pristine is missing. Rust loader at [theater.rs:669](../../../src/map/theater.rs#L669) `break`s on first missing variant — matches binary. [GHIDRA 0x00545150 fallback chain]

## Design

### Components

**Subsystem A — sim writer (~250 LOC):**

New method on `BridgeRuntimeState` in [src/sim/bridge_state/mod.rs](../../../src/sim/bridge_state/mod.rs):

```rust
/// Propagate the damaged-variant bit across an 8-neighbor region bounded by
/// `tile_index` equality. Mirrors gamemd.exe `ToggleBridgePavement` @ 0x0056E990.
///
/// The kickoff call (external) checks the TMP `has_damaged_data` gate before
/// mutating; recursive calls skip the gate (the binary trusts that all cells
/// sharing a tile_id share the gate flag since they're the same TMP).
///
/// Returns the count of cells mutated.
pub fn apply_damaged_variant_flood_fill(
    &mut self,
    rx: u16,
    ry: u16,
    state: bool,
    terrain: &ResolvedTerrainGrid,
) -> u32 {
    self.apply_damaged_variant_flood_fill_internal(rx, ry, state, terrain, /*kickoff=*/true)
}

fn apply_damaged_variant_flood_fill_internal(
    &mut self,
    rx: u16,
    ry: u16,
    state: bool,
    terrain: &ResolvedTerrainGrid,
    kickoff: bool,
) -> u32 {
    // 1. Resolve bridge cell. Non-bridge → no-op.
    let cell = match self.cell(rx, ry) {
        Some(c) => c,
        None => return 0,
    };

    // 2. Idempotency early-return.
    if cell.damaged_variant == state { return 0; }

    // 3. Resolve underlying terrain tile_id. Sentinel → no-op.
    let resolved = match terrain.cell(rx, ry) {
        Some(c) => c,
        None => return 0,
    };
    let seed_tile_id = resolved.final_tile_index;
    if seed_tile_id == 0xFFFF || seed_tile_id == 0xFF { return 0; }

    // 4. Kickoff-only: check TMP capability flag.
    if kickoff {
        let sub_tile = resolved.sub_tile;
        if !terrain.theater().tile_has_damaged_data(seed_tile_id, sub_tile) {
            return 0;
        }
    }

    // 5. Mutate.
    if let Some(c) = self.cell_mut(rx, ry) {
        c.damaged_variant = state;
    }
    let mut count = 1;

    // 6. 8-neighbor recursion. Fixed direction order matching binary g_DirectionOffsets.
    for (dx, dy) in EIGHT_NEIGHBOR_OFFSETS_BINARY_ORDER {
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx < 0 || ny < 0 { continue; }
        let nx = nx as u16;
        let ny = ny as u16;
        if let Some(n_resolved) = terrain.cell(nx, ny) {
            if n_resolved.final_tile_index == seed_tile_id {
                count += self.apply_damaged_variant_flood_fill_internal(
                    nx, ny, state, terrain, /*kickoff=*/false,
                );
            }
        }
    }

    count
}
```

The direction offsets must match the binary's `g_DirectionOffsets` at `0x0089F688` — verified from prior audits to be N, NE, E, SE, S, SW, W, NW (8 entries, 4 bytes each: i16 dx, i16 dy). Captured as a module-level `const EIGHT_NEIGHBOR_OFFSETS_BINARY_ORDER: [(i32, i32); 8]`.

**Hook wiring — 3 sites:**

1. `body_cell_advance_state` [mod.rs:780](../../../src/sim/bridge_state/mod.rs#L780):
   - Signature change: add `terrain: &ResolvedTerrainGrid` parameter.
   - On `DamageState::Healthy { .. }` arm (line 831): after the existing `update_ramp_perpendicular(DamageA + DamageB)` calls, call `self.apply_damaged_variant_flood_fill(rx, ry, true, terrain)`. (`update_ramp_perpendicular` itself ALSO calls the flood-fill — see hook 2 — but the body cell needs to be flipped directly too since the binary's `UpdateRamp_*_DamageA/B` calls happen on the **perpendicular** target, not the seed.)
   - On `DamageState::Damaged` arm (line 853, full collapse): no new call here — the perpendicular `CollapseA/CollapseB` calls handle the propagation via hook 2; the binary's collapse path passes state=1 too (ledger #1).
   - On `DamageState::PartialCollapseA/B` arms: same as Damaged — perpendicular hooks handle it.

2. `update_ramp_perpendicular` [bridge_specs.rs:533](../../../src/sim/bridge_specs.rs#L533):
   - Signature change: add `terrain: &ResolvedTerrainGrid`.
   - After applying the perpendicular `+0x11E` write that mirrors `UpdateRamp_*`, call `state.apply_damaged_variant_flood_fill(perp_rx, perp_ry, true, terrain)`.
   - This fires on all four phases: DamageA, DamageB, CollapseA, CollapseB — all pass state=1 (ledger #1).

3. `bridgehead_advance_state` [mod.rs:1111](../../../src/sim/bridge_state/mod.rs#L1111):
   - Already takes `terrain` parameter — no signature change needed.
   - On bridgehead → Damaged or Destroyed: call flood-fill at each destroyed cell with state=1.

**Repair clear — 1 site:**

`body_cell_repair_state` [mod.rs:978](../../../src/sim/bridge_state/mod.rs#L978):
- Signature change: add `terrain: &ResolvedTerrainGrid`.
- Inside Step 2 loop, immediately after `cell.damage_state = Healthy { variant }` (line 1039): call `self.apply_damaged_variant_flood_fill(cell_pos.0, cell_pos.1, false, terrain)`. The kickoff call propagates the clear across the cell's tile_id region; idempotency early-return prevents redundant work when multiple cells in the same region get repaired (ledger #5).

**Subsystem B — render guard + lookup (~100 LOC):**

1. `Theater::tile_has_damaged_data(tile_id: u16, sub_tile: u8) -> bool` — new accessor on the theater registry. Reads through to `theater.tile_image_for(tile_id, sub_tile).has_damaged_data` (or equivalent — exact field path to be located during /write-plan; today the `TmpTile.has_damaged_data: bool` is parsed at TMP load but may not be retained in the per-tile theater registry — implementation step may need to thread it through).

2. `resolved_terrain.rs:874` guard: prepend `if td.lookup.tile_has_damaged_data(tile_id, sub_tile_for_cell) { cell.variant = 0; continue; }` to the loop. Makes ledger #18 explicit and idempotent under future asset changes.

3. Bridge body render site (TBD location during /write-plan): replace any `TileKey { variant: 0 }` (or `cell.variant`-driven) lookup with:
   ```rust
   let variant = if let Some(bridge_cell) = world.bridge_state.cell(rx, ry) {
       if theater.tile_has_damaged_data(tile_id, sub_tile) && bridge_cell.damaged_variant {
           1
       } else {
           0
       }
   } else {
       cell.variant  // non-bridge cell — use the existing FA2 PRNG variant
   };
   atlas.get_uv(TileKey { tile_id, sub_tile, variant })
   ```
   The exact insertion point depends on whether bridge body tiles are rendered through the normal terrain draw loop (likely yes) or through a separate bridge code path. /write-plan must locate this.

### Interfaces / Contracts

**New / changed signatures:**

```rust
// New on BridgeRuntimeState:
pub fn apply_damaged_variant_flood_fill(
    &mut self, rx: u16, ry: u16, state: bool, terrain: &ResolvedTerrainGrid,
) -> u32;

// Signature additions (terrain parameter):
pub fn body_cell_advance_state(
    &mut self, rx: u16, ry: u16, is_high_bridge: bool, terrain: &ResolvedTerrainGrid,
) -> StateOutcome;

pub fn body_cell_repair_state(
    &mut self, scan_cells: &[(u16, u16)], rng: &mut SimRng, terrain: &ResolvedTerrainGrid,
) -> RepairOutcome;

pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState, anchor_pos: (u16, u16), axis: Axis, phase: Phase,
    is_high_bridge: bool, terrain: &ResolvedTerrainGrid,
) -> /* existing return */;

// New on Theater (or wherever the tile-image registry lives):
pub fn tile_has_damaged_data(&self, tile_id: u16, sub_tile: u8) -> bool;
```

### Data flow

```
combat damage event (Phase F)
  → world.bridge_state.body_cell_advance_state(rx, ry, is_high, &terrain)
    → cell.damage_state Healthy → Damaged
    → update_ramp_perpendicular(DamageA, &terrain)
       → apply perpendicular +0x11E write
       → apply_damaged_variant_flood_fill(perp_rx, perp_ry, state=true, &terrain)
         → recursive 8-neighbor walk bounded by tile_id == seed_tile_id
         → flips damaged_variant: bool on each cell in connected region
    → update_ramp_perpendicular(DamageB, &terrain) [same]
    → apply_damaged_variant_flood_fill(rx, ry, state=true, &terrain) [seed]
    → return Absorbed

(next render frame)
  → bridge body draw site queries world.bridge_state.cell(rx, ry).damaged_variant
  → TileKey { variant: damaged_variant as u8 } → atlas UV → GPU draws damaged sprite

engineer enters CABHUT (Phase F)
  → world.bridge_state.body_cell_repair_state(scan_cells, rng, &terrain)
    → for each cell scheduled for repair:
        → cell.damage_state = Healthy { variant: rng_pick }
        → apply_damaged_variant_flood_fill(cell_pos.0, cell_pos.1, state=false, &terrain)
          → flips damaged_variant: bool back to false across region

(next render frame)
  → variant: 0 → pristine sprite restored
```

### Error handling

- Off-map / non-bridge / sentinel-tile-id cells: silent no-op (matches binary).
- Recursion stack bound: practical tile_id regions <20 cells; defensive `debug_assert!(depth < 256)` recommended.
- Missing terrain reference: not a runtime error — all sim methods that need it take it as a borrowed parameter; compile-time guarantee.
- Theater accessor for `tile_has_damaged_data`: if tile_id is out of range, return false (safe default — no flood-fill fires).

### Testing strategy

**Unit tests** in `bridge_state` test module:

| Test | Asserts |
|------|---------|
| `flood_fill_kickoff_skips_when_no_damaged_data` | TMP gate check on kickoff: false → no mutation, no propagation |
| `flood_fill_propagates_to_same_tile_id_neighbors` | All 8 neighbors with matching tile_id get the bit |
| `flood_fill_stops_at_different_tile_id_boundary` | Neighbor with different tile_id does NOT get the bit |
| `flood_fill_idempotent_on_already_target_state` | Second call with same state is no-op (returns 0) |
| `flood_fill_eight_directions_includes_diagonals` | NE/SE/SW/NW neighbors covered, not just cardinals |
| `flood_fill_recursion_no_infinite_loop` | Region with cycle visits each cell exactly once |
| `flood_fill_clears_across_region` | state=false propagates exactly like state=true |
| `flood_fill_off_map_returns_zero` | Out-of-bounds coords are no-op |

**Integration tests** in [src/sim/world/world_orders_bridge_repair_tests.rs](../../../src/sim/world/world_orders_bridge_repair_tests.rs):

| Test | Asserts |
|------|---------|
| `damage_sets_damaged_variant_bit_on_body_cell` | After `body_cell_advance_state` Healthy→Damaged, the bit is true on the seed AND on perpendicular targets |
| `collapse_preserves_damaged_variant_bit` | Healthy→Damaged→Destroyed sequence leaves bit true after the destroy step (ledger #9) |
| `repair_clears_damaged_variant_bit` | Engineer-CABHUT repair clears the bit on all repaired cells |
| `repair_propagates_clear_via_flood_fill` | A repair on cell X clears the bit on tile_id-neighbor Y too |
| `state_hash_changes_when_damaged_variant_flips` | Sim hash diverges; lockstep verified |

**Render visual check (manual):**
- Load skirmish; damage a low bridge to Damaged state.
- Visually confirm scuffed/cracked bridge texture appears on body cells.
- Engineer-repair (Allied IFV mode or Soviet engineer into CABHUT).
- Visually confirm pristine texture restored.

## Architectural Decisions

**Patterns followed:**
- `apply_damaged_variant_flood_fill` mirrors the shape of `body_cell_advance_state` / `body_cell_repair_state` / `bridgehead_advance_state` (takes `rx, ry`, mutates state, returns count).
- Reuses existing `TileKey.variant` axis instead of introducing a parallel system.
- Reuses existing `cell.damaged_variant: bool` instead of moving to a packed flag DWORD.
- Recursive immediate-execution mirrors binary's `ToggleBridgePavement` 1:1.

**Patterns deviated from:**
- Bridge sim state methods currently don't all take `&ResolvedTerrainGrid`; this design adds it. Reason: flood-fill propagation rule is `tile_id == seed_tile_id`, and tile_id lives on `ResolvedTerrainGrid`. Acceptable: deliberate cross-module read of immutable terrain data, consistent with `bridgehead_advance_state` which already takes terrain.

**Tech debt introduced:** None — design closes an existing parity gap.

## Alternatives Considered

**Approach 2 — iterative BFS with explicit visited-set, deferred to end of tick.** Rejected:
- Deferred execution mismatches binary's immediate-execution semantics; any future sim consumer in Phase F would see 1-tick delay.
- Visited-set is dead weight given idempotency early-return.
- Detail #2 (gate-only-on-kickoff) is awkward in BFS — every queue entry must carry a `kickoff` flag.

**Approach 3 — hoist `damaged_variant` to `ResolvedTerrainCell`, build generic `apply_tile_id_flood_fill` primitive.** Rejected:
- Speculative generalization — only G4 verified user today.
- Larger refactor — moves a field across module boundaries, ~24 construction sites churn.
- ResolvedTerrainCell is currently treated as immutable map-load data; mutating it requires careful determinism review.
- YAGNI — the next-most-likely reuse (`SetOverlayAndPropagate` for bridge collapse tile swap) was already deferred in a prior brainstorm.
