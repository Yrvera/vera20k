# Bridges Tier 2 — Phase D Renderer Design

**Date:** 2026-05-08
**Author:** brainstorm session (Approach B selected)
**Status:** approved design — ready for `/write-plan`
**Predecessor RE doc:** `ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` (2026-05-07/08, HIGH confidence)
**Predecessor branch state:** `dev` @ HEAD `e8db5eb` (Phases B+C+E+F+G shipped; zero ignored bridge tests)

---

## Goal

Bring the Rust renderer to gamemd parity on visible bridge output: bridge body (already partial), bridge body shadow, bridge railings, deck-TMP damaged-tile variant, and the rim-refresh write set that the renderer depends on. Remaining work after Phase D: none — Tier 2 closes.

---

## Architecture Context

### Sim side (post-tick, available to render)

- `BridgeRuntimeCell { damage_state, axis, role, anchor_span_id, overlay_byte, deck_present, deck_level, … }` is the binary-faithful post-tick state.
- `damage_state.to_state_byte(axis)` mirrors binary `cell+0x11E` (0..17).
- `overlay_byte` mirrors binary `cell+0x44`.
- `BridgeRuntimeState::is_bridge_walkable(rx, ry)` already filters destroyed cells.
- Orchestrator stub `bridge_orchestrator.rs:208 update_adjacent_bridges` is currently a no-op. RE doc §7 specifies the rim-refresh writes.

### Render side (current)

- `src/render/bridge_atlas.rs` — packs HIGH bridge body SHPs (BRIDGE1/BRIDGE2/BRIDGEB1/BRIDGEB2), frames 0..17 only (skips shadow half), z-depth bind group ready. No railings, no shadow pass.
- `src/render/overlay_atlas.rs` — generic atlas; LOW bridge SHPs (LOBRDG##/LOBRDGE#/LOBRDGB#) live here. No special bridge handling.
- `src/app_instances/overlays.rs:217-424` — current consumer reads `OverlayGrid` + static `OverlayPack` for the live frame, applies the 16-entry Latin square to base frames 0 and 9, picks bucket BridgeBody/BridgeDetail/Wall/Generic, applies `-16/-31` Y offsets, depth `+4` for HighBridgeHeight. Skips destroyed cells via `bridge_state.is_bridge_walkable`.
- `src/app_render/draw_passes.rs:60-93` — order: terrain zdepth → bridge body zdepth → bridge-detail passthrough → overlay passthrough → smudge → bridge-entity merge → ground merge → cliff zdepth.

### Gaps vs. RE doc §9 (player-visible)

| Step | RE element | Status |
|------|-----------|--------|
| 3 | Deck TMP `cell.flags & 0x2000` damaged-tile variant select | Missing |
| 5 body | Latin square only on states 0 & 9, frame=state otherwise | ✅ correct |
| 5 body | `-16/-31` Y, `(level+4)*-15-2` Z | ✅ correct |
| 5 body | Read from `BridgeRuntimeCell.overlay_byte` post-tick | ✗ reads from OverlayGrid |
| 5 shadow | shadow-half frame `(N/2 + state)`, EW shift `(-45,+7)` (or `-15`, RE §10 open Q) | Missing |
| 7 | Railings (RAILBRDG.tem) — concrete/wood tables × 10 entries `{frame, surface, dx, dy}` | Missing entirely |
| 7 | Layer ordering: railings drawn AFTER units/animations | Pipeline missing |
| Sim | Rim refresh: per-cell `overlay_byte = NONE`, `damage_state = 0` on dangling stubs | Stub no-op |

---

## Impact Analysis

| File | Change | LOC delta |
|------|--------|-----------|
| **NEW** `src/app_instances/bridges.rs` | Owns all bridge instance emission | +~400 |
| **NEW** `src/render/bridge_railing_atlas.rs` | Loads RAILBRDG.tem + 2 × 10-entry railing tables | +~200 |
| `src/render/bridge_atlas.rs` | Pack ALL frames (body + shadow halves); add body/shadow accessors | +~80 |
| `src/sim/bridge_state/mod.rs` | Add `damaged_variant: bool`; `rim_dirty: BTreeSet`; `iter_bridge_cells`; `mark_rim_dirty` / `take_rim_dirty` | +~50 |
| `src/sim/world/bridge_orchestrator.rs` | Fill `update_adjacent_bridges` per RE §7 | +~150 |
| `src/app_instances/overlays.rs` | Remove all bridge handling | -~180 |
| `src/app_render/build_instances.rs` | Add `bridge_body_shadow`, `bridge_railings` buckets | +~50 |
| `src/app_render/draw_passes.rs` | Insert shadow pass + railing pass at correct insertion points | +~60 |

**Determinism / lockstep risk:** rim-refresh walk and deck-variant write happen in sim — must use sorted iteration (`BTreeMap` / `BTreeSet`) so multi-client state hash matches. Renderer-side reads are pure.

**Failure modes:**
- Body shadow X shift (`-45` vs `-15`) is unresolved in RE doc §10 — single-named-constant escape hatch.
- Sim's `Healthy.variant` (0..=5 model) vs binary Latin square (0..=3 model): renderer ignores `variant` and re-derives from `(cell.x, cell.y)` like the binary does.
- Layer ordering: railing pass before unit merge silently produces "tank-on-top-of-railing" — visibly wrong but won't fail tests.
- Atlas size: doubling `bridge_atlas` (~bounded; 4 SHPs × 36 frames). Fine.

---

## Chosen Approach (Approach B)

Pull all bridge instance-emission into a new `app_instances/bridges.rs`. New `render/bridge_railing_atlas.rs` module. `bridge_atlas.rs` extended for shadow frames. `overlays.rs` becomes non-bridge only — its bucket classifier collapses to `Wall` vs `Generic`. `bridges.rs` exports four instance-builder fns returning four buckets: deck-variant overrides, body, shadow, railings. Build phase calls them after `build_overlay_instances`. Draw passes call shadow + railing draws at the binary-correct insertion points.

### Why not Approach A (in-place extension)

Bridge code already justifies its own module: existing `bridge_atlas.rs`, sim-side `bridge_state/`, `bridge_orchestrator.rs`, `bridge_specs.rs`, `walker.rs`. Render side has been the odd one out. Approach B brings render into structural symmetry with sim. `overlays.rs` is already ~720 lines; approach A pushes it past 870.

### Why not Approach C (single OverlayAtlas)

Folding `bridge_atlas` back into `OverlayAtlas` requires either (a) special-casing inside `overlay_atlas.rs` for bridge entries (reinventing `bridge_atlas`), or (b) forcing bridges to passthrough (parity-broken — body would no longer write Z, breaking sort against units crossing). Breaks ledger item #7.

---

## Tiny-Detail Ledger

Every implementation step must preserve these. Sourced from RE doc; nothing invented.

### Frame selection (body)

1. Latin-square table = `{0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2}` — 16 entries, 4 dwords/row [doc §5; verified at `0x0081CC30`].
2. Latin square applies ONLY when state byte ∈ {0, 9}; states 1..8 and 10..17 use frame = state directly [doc §3.3.1, §5].
3. Latin index = `((cell.y & 3) << 2) | (cell.x & 3)` [doc §3.3.1].
4. Sim `Healthy.variant` is ignored by render; renderer derives its own jitter from cell xy [reconciliation note].

### Body geometry & Z

5. Y offset = `-16` for states 0..8, `-31` for states 9..17 (HasBridge cells) [doc §3.3.3].
6. Z = `(cell.height_level + 4) * -15 - 2` (+4 = HasBridge bonus) [doc §3.3.1].
7. Body blitter flag 0x4E00 → Z-test ON, Z-write ON → use existing `bridge_atlas.zdepth_bind_group` path [doc §3.3.1].

### Shadow

8. Shadow frame = `(shp.frame_count / 2) + state` [doc §3.3.2].
9. Shadow X shift on EW states 9..17 = `-45` OR `-15` — `UNKNOWN — needs visual verification against gamemd` [doc §10 open question 2].
10. Shadow Y shift on EW states 9..17 = `+7` [doc §3.3.2].
11. Shadow blitter flag 0x4601 → Z-test ON, Z-write OFF, darken pass → passthrough pipeline with depth read but no write.
12. Shadow uses neutral tint (1000) — no per-cell lighting tint [doc §3.3.2].

### Damaged-tile deck variant (Step 3)

13. Bit source: `cell.flags & 0x2000`, written by `ToggleBridgePavement` only [doc §3.2, §4]. Sim adds `BridgeRuntimeCell.damaged_variant: bool`.
14. When set AND `IsoTileType.num_tiles >= 2` AND `tile_data[sub].flags & 0x04`: pick alternate sub-tile via `IsoTileType` linked-list (`IsoTileType + 0xAF * 4`) [doc §3.2]. For our terrain pipeline this means selecting the alt-art deck tile UV at instance build via a `BTreeMap<(rx,ry), DeckVariantSelect>` override map.

### Railings (Step 7)

15. Tables: 10 × `{shp_frame_idx_plus_1, surface, dx, dy}` × 2 (concrete + wood) [doc §3.4.1].
16. Index = `IsoTileType.SelfIdx - g_BridgeSet` (concrete) or `- g_WoodBridgeSet` (wood) [doc §3.4.1].
17. `shp_frame == 0` ⇒ no railing for this sub-tile, skip [doc §3.4.1].
18. Railing screen pos = `(screen_x + dx + 30, screen_y + dy + 15)` (the 30/15 are TILE_WIDTH/2 and TILE_HEIGHT/2, already in our overlay anchor) [doc §3.4.1].
19. Railing blitter flag 0x4601 → Z-test, no Z-write [doc §3.4.1].
20. Railing tint = neutral 1000 [doc §3.4.1].
21. Railing SHP = theater-loaded `RAILBRDG.tem` (verified `[RAILBRDG] Theater=yes` in art.ini).

### Layer ordering

22. Order: terrain zdepth → bridge body zdepth → bridge body shadow passthrough → overlay/smudge passthrough → bridge entities merge → ground/units merge → cliff redraw zdepth → bridge railings passthrough → debug → shroud → UI [doc §2.2 + §9.4 + user choice].
23. Anything drawn between body and railings (units, anims) must appear ABOVE deck but BELOW railings — invariant from §2.2.

### Read path (sim → render)

24. Render reads `BridgeRuntimeCell.overlay_byte` (post-tick) — mirrors binary `cell+0x44`. SHP name lookup `overlay_byte → name` via existing `state.overlay_names` map.
25. Render reads `damage_state.to_state_byte(axis)` — mirrors binary `cell+0x11E`. Source for body frame and Y-offset bucket.
26. Render reads `damaged_variant` (new field) for deck TMP alt-art selection.
27. Render reads `is_bridge_walkable()` for the destroyed-cell skip — already in place.

### Rim refresh (sim, bundled into Phase D)

28. Trigger: any cell write that changes `damage_state` or `overlay_byte` queues its 8 neighbors into `BridgeRuntimeState::rim_dirty: BTreeSet<(u16,u16)>` for orchestrator drain.
29. Walk: 8-direction, stop at first cell with `flags & 0x500` (bridge head OR destroyed) [doc §7.1].
30. Walk extent: up to 30 cells along walk direction [doc §7.2].
31. Dangling-stub action: `cell.overlay_byte = NONE`, `damage_state = Healthy{variant: 0}`, clear bridge-direction flags, mark zone-rebuild dirty [doc §7.2].
32. Recursion: re-invoke on next stub until walk terminates [doc §7.2].
33. Iteration order: deterministic via `BTreeSet` (sorted (rx, ry)).

### Things to drop (TS-legacy)

34. No FoggedObject display-table walker — TS-legacy, FogOfWar=false in YR [doc §2.1].
35. No `cell+0x118` byte cache — DAT_00880940 always 0 [doc §2.8].
36. No high-bridge runtime tile selector — tile_index event-driven only [doc §2.3].
37. No RMG bridge placer — TS-legacy [doc §2.6].

---

## Design

### Components

```
src/
  sim/
    bridge_state/
      mod.rs                 ← + damaged_variant, rim_dirty queue, iter accessor
    world/
      bridge_orchestrator.rs ← fill update_adjacent_bridges (line 208)
  render/
    bridge_atlas.rs          ← pack body + shadow halves; body/shadow accessors
    bridge_railing_atlas.rs  ← NEW: RAILBRDG.tem + 2 × 10-entry tables
  app_instances/
    bridges.rs               ← NEW: 4 instance-builder fns
    overlays.rs              ← - remove bridge handling
  app_render/
    build_instances.rs       ← + 2 buckets (body_shadow, railings)
    draw_passes.rs           ← + 2 passes (shadow, railing) at correct order
```

### Public interfaces

```rust
// sim/bridge_state/mod.rs
pub struct BridgeRuntimeCell {
    // …existing fields…
    /// Mirror of binary cell.flags & 0x2000. Written by ToggleBridgePavement
    /// equivalent. Selects damaged sub-tile art for deck TMP at draw time.
    pub damaged_variant: bool,
}

impl BridgeRuntimeState {
    /// Iterate bridge-bearing cells, sorted by (rx, ry) for determinism.
    pub fn iter_bridge_cells(&self)
        -> impl Iterator<Item = ((u16, u16), &BridgeRuntimeCell)>;

    /// Add 8 neighbors of (rx, ry) to rim_dirty. Called by every mutator that
    /// changes damage_state or overlay_byte.
    pub fn mark_rim_dirty(&mut self, rx: u16, ry: u16);

    /// Drain rim_dirty post-tick, returning sorted set.
    pub fn take_rim_dirty(&mut self) -> BTreeSet<(u16, u16)>;
}

// render/bridge_atlas.rs
impl BridgeAtlas {
    pub fn body_entry(&self, name: &str, state_byte: u8) -> Option<&OverlaySpriteEntry>;
    pub fn shadow_entry(&self, name: &str, state_byte: u8) -> Option<&OverlaySpriteEntry>;
}

// render/bridge_railing_atlas.rs
#[derive(Clone, Copy)]
pub enum BridgeKind { Concrete, Wood }

pub struct RailingEntry {
    pub shp_frame: u8,         // 1-based in source table; 0 = "no railing"
    pub dx: i16,
    pub dy: i16,
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
    pub offset_x: f32,
    pub offset_y: f32,
}

pub struct BridgeRailingAtlas {
    pub texture: BatchTexture,
    concrete_table: [Option<RailingEntry>; 10],
    wood_table: [Option<RailingEntry>; 10],
}

impl BridgeRailingAtlas {
    pub fn entry(&self, kind: BridgeKind, sub_idx: u8) -> Option<&RailingEntry>;
}

// app_instances/bridges.rs
pub struct DeckVariantSelect {
    pub use_alternate: bool,
}

pub fn build_bridge_deck_variant_overrides(state: &AppState)
    -> BTreeMap<(u16, u16), DeckVariantSelect>;

pub fn build_bridge_body_instances(state: &AppState, sw: f32, sh: f32)
    -> Vec<SpriteInstance>;

pub fn build_bridge_shadow_instances(state: &AppState, sw: f32, sh: f32)
    -> Vec<SpriteInstance>;

pub fn build_bridge_railing_instances(state: &AppState, sw: f32, sh: f32)
    -> Vec<SpriteInstance>;
```

### Data flow

```
SIM TICK
  combat → bridge events → bridge_orchestrator dispatcher
    body_cell_advance_state / bridgehead_advance_state mutate:
      cell.damage_state, cell.overlay_byte, cell.damaged_variant
    mark_rim_dirty(rx, ry)            ← every mutation queues 8 neighbors
  bridge_orchestrator drain (post body):
    rim = take_rim_dirty()           ← sorted, deterministic
    update_adjacent_bridges(rim):
      for each rim cell:
        8-dir walk → first cell w/ flags & 0x500
        determine walk dir from flags (bits 0x100/0x400/0x80/0x800)
        walk up to 30 cells along direction
        on dangling-stub match:
          cell.overlay_byte    = NONE
          cell.damage_state    = Healthy{0}
          clear bridge-direction flags
          mark zone-rebuild dirty
          recurse on next stub
    refresh_endpoint_active_flags
    rebuild zones if dirty

RENDER FRAME
  build_instances:
    build_overlay_instances     (non-bridge: ore, walls, terrain)
    build_bridge_deck_variant_overrides → terrain instance builder consumes
    build_bridge_body_instances           → world.bridge_body
    build_bridge_shadow_instances         → world.bridge_body_shadow
    build_bridge_railing_instances        → world.bridge_railings

  draw_passes (in order):
    1. terrain zdepth                                  (deck-variant overrides applied)
    2. bridge body zdepth                              (Z R+W)
    3. bridge body shadow passthrough                  (Z-test, NO Z-write)         ← NEW
    4. overlay passthrough (bridge_detail + overlay)
    5. smudge passthrough
    6. bridge entities Y-merge
    7. ground/units Y-merge
    8. cliff redraw zdepth
    9. bridge railings passthrough                     (Z-test, NO Z-write)         ← NEW
    10. debug, shroud, UI
```

### Error handling

- Missing SHP for an overlay byte → `log::warn!` once per cell, skip cell. (Per project memory `feedback_silent_render_failures`.)
- `entry.shp_frame == 0` (no railing for sub-tile) → skip railing only; body and shadow still draw.
- `state.simulation.bridge_state` is `None` → all `build_bridge_*` early-return.
- Atlas missing at scenario load → `log::warn!`, bridges render via fallback (no railings, no shadow).
- Rim-walk dangling-stub match fails after 30 cells → terminate walk; no panic.

### Testing strategy

| Layer | Tests |
|-------|-------|
| Unit (`bridge_atlas`) | Body/shadow UV lookup for each `(name, state)`; assert `body_entry("BRIDGE1", 5)` and `shadow_entry("BRIDGE1", 5)` return distinct UVs. |
| Unit (`bridge_railing_atlas`) | Entry lookup for each `(kind, sub_idx)` in 0..10; entry == `None` when `shp_frame == 0`. |
| Unit (`bridges.rs`) | Latin square: `state=0, cell=(1,2)` → frame index = `((2&3)<<2) \| (1&3) = 9` → table[9] = 3 → frame 3. `state=5` → frame 5 (NO jitter). Y-offset `-16` for state 5, `-31` for state 13. |
| Integration (`sim/world/world_tests/`) | Damage cell → next frame's body bucket reflects new state byte. Destroy cell → drops from buckets AND rim refresh resets 8 neighbors' `overlay_byte`/`damage_state`. Toggle `damaged_variant` → terrain override appears. |
| Determinism | Replay same map+inputs twice → identical instance buffers; state hash stable across rim-refresh ticks. |
| Visual (manual) | Side-by-side gamemd vs Rust on a fixed bridge map to resolve §10 open Qs (shadow X shift `-15`/`-45`, axis convention). |

---

## Architectural Decisions

### Patterns followed

- **Sim/render symmetry.** Sim has `bridge_state/`, `bridge_orchestrator.rs`, `bridge_specs.rs`, `walker.rs`. Render now has `bridge_atlas.rs`, `bridge_railing_atlas.rs`, `bridges.rs`. Mirrored structure.
- **Atlas-per-pipeline.** Separate `BridgeAtlas` (zdepth body) from `OverlayAtlas` (passthrough). Required by ledger #7. Same separation principle as `TileAtlas` (terrain zdepth) vs `OverlayAtlas`.
- **Module size.** `overlays.rs` shrinks by ~180 lines (drops bridge code); `bridges.rs` grows ~400 lines. Both stay under 600.
- **Deterministic iteration.** All sim mutations and render reads use `BTreeMap`/`BTreeSet`. No `HashMap` in the rim-refresh or render-iteration paths.

### Patterns deviated from

None.

### Tech debt introduced

- **Shadow X shift unresolved value** — single named constant `BRIDGE_SHADOW_EW_DX`, defaults to `-15`, marked TODO with visual-verify directive. One change point if `-45` turns out correct.
- **EW vs NS axis convention** — current Rust mapping is `Axis::EW = state 9..17`. Verify against bridge.tem frame 0 vs 9; tag as test-pending until visual diff completes.
- **Railing table values** — runtime-populated in gamemd from theater data tables (`DAT_00ABC210` concrete, near `DAT_00AA1098` wood). Phase D minimum: extract via static memory dump or live debugger capture, hardcode as theater-keyed const tables. Sub-task documented below.

### Sub-task: railing table extraction

The railing tables are zero in the static binary — populated by `CDFileClass__Constructor` at theater load (writes at `0x005446B1`, `0x00543F36`, `0x005451DC`, `0x00543C42`, `0x00543E02`). Two paths:

1. **Static analysis path:** decompile the writers and reconstruct the runtime values from the theater `.ini` parsing. ~1-2 hour investigation per RE doc §10 open Q4.
2. **Live debugger path:** attach to running gamemd, dump 160 bytes (10 × 16) at `DAT_00ABC210` and the wood equivalent post-theater-load. ~10 min.

Recommendation: do path 2 first to unblock implementation; path 1 as a follow-up `/re-investigate` if the live values look surprising.

### Open RE questions ship as named constants

| Open Q | Constant | Default | Resolution |
|--------|----------|---------|------------|
| Shadow X shift | `BRIDGE_SHADOW_EW_DX` | `-15` | Visual diff vs gamemd |
| Axis convention (EW/NS labeling) | (existing `Axis` enum mapping) | `Axis::EW = states 9..17` | Visual diff bridge.tem frames 0 vs 9 |
| `cell+0x11A` semantics (sub_tile vs damage_state_1) | (n/a — Phase 1C reading is authoritative) | `sub_tile` | Re-decompile if rim refresh disagrees |
| Wood railing table base address | `WOOD_RAILING_TABLE_BASE` | `DAT_00AA1098` | Live-debugger memory dump |

---

## Alternatives Considered

### Approach A — Bucket extension in-place

Keep all bridge instance-emission inside `overlays.rs`. Add two more buckets (`bridge_body_shadow`, `bridge_railings`). Switch read source from `OverlayGrid` to `BridgeRuntimeCell` inside the existing loop.

**Rejected:** `overlays.rs` grows past the 600-line guideline (currently ~720, would hit ~870). Bridge logic stays mixed with non-bridge overlay logic. Same parity outcome; worse cohesion.

### Approach C — Single OverlayAtlas

Throw out `bridge_atlas.rs`. Pack everything into the generic `overlay_atlas.rs`. All bridge logic in `overlays.rs`.

**Rejected:** breaks ledger item #7 (body must Z-write). Removing `zdepth_bind_group` for body lets units crossing a bridge sometimes draw OVER the bridge body deck — per-pixel parity drift. Reintroducing the special bind group reinvents `bridge_atlas`. Net negative.

---

## Hand-off — next steps

1. `/write-plan` with this design as input → bite-sized task list with exact file paths, signatures, and verification steps.
2. Execute tasks on `dev` (no branch, no PR, per project memory `feedback_branches_and_prs`).
3. Resolve the three open Qs (shadow X shift, axis convention, railing table values) during implementation — single change points each.
4. End-to-end visual diff vs gamemd on a fixed bridge map closes Phase D and Tier 2.
