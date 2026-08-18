# CABHUT Bridge Collapse Visual Trace
## Slot 5 — Sim-to-Render/Audio Pipeline

**Mechanic:** After N bridge cells transition to `DamageState::Destroyed`, does the
render + audio layer produce N explosions, N tile swaps, N splash sounds, and N water
reveals — or does it produce 1 regardless?

**Scenario:** SEAL plants C4 on CABHUT → `dispatch_bridge_collapse_from_hut` is called
→ produces `StateOutcome::Collapsed { destroyed_cells: [N cells] }` →
`apply_hut_bridge_outcomes` runs the cascade.

**Scope:** Render/audio pipeline only. Sim correctness (slot 4) assumed correct.

**Date:** 2026-05-20

---

## Stage Results

### Stage 1 — Sim-to-render boundary: PASS (mechanism verified)

The `apply_bridge_damage_events` / `apply_hut_bridge_outcomes` functions
(`src/sim/world/bridge_orchestrator.rs`) return `bool` = `!destroyed_set.is_empty()`.
`app_sim_tick.rs:560` sets `refresh_after_tick = true` when `tick_result.bridge_state_changed`.

Render reads bridge state from `sim.bridge_state` (a `BridgeRuntimeState`) directly
each frame via `build_bridge_body_instances`, `build_bridge_shadow_instances`,
`build_bridge_railing_instances` in `src/app_instances/bridges.rs`. This is a
snapshot-read: every frame all bridge cells are iterated from `BridgeRuntimeState::iter_cells()`.
There is no per-cell dirty flag; the full set is re-scanned each frame. This means
if N cells are in state `DamageState::Destroyed` after the collapse tick, those N
cells are immediately visible to the render layer on the next frame.

**Numerical parity:** gamemd uses a per-cell dirty-rect system; our engine re-draws
all visible bridge cells unconditionally each frame — this cannot show FEWER cells
than gamemd, and the count agrees at N.

**Verdict: PASS** — all N destroyed cells are visible to render boundary every frame.

---

### Stage 2 — Per-cell tile-swap render (bridge body SHP → destroyed tile-set): PASS

`build_bridge_body_instances_inner` (`src/app_instances/bridges.rs:116`):
```rust
for ((rx, ry), cell) in bridge_state.iter_cells() {
    if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
        continue;  // ← SKIP destroyed cells
    }
```

Destroyed cells are skipped from bridge-body SHP rendering. This is correct: in
gamemd, a destroyed cell's `cell+0x44` overlay byte is set to 0xE7 (EW destroyed)
or 0xE8 (NS destroyed), which are the "fully destroyed body" overlay types that
render a dropped/broken bridge sprite. In our engine the equivalent is handled by
the terrain TMP tile swap (the `cell+0x140 & 0x2000` damage-variant bit in gamemd
selects the "alternate" sub-tile art via the IsoTileType linked list).

**Terrain TMP tile swap:** `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md §3.2` verifies that
the `cell+0x140 & 0x2000` damage-variant bit selects the destroyed tile art by passing
`param_14` to `TMP_TileBlitter`. This is set by `MapClass::ToggleBridgePavement` during
destruction events (event-driven, not per-frame). Our engine's terrain tile is rendered
via `build_instances.rs:106` using `(tile_id, sub_tile, variant: 0)` — the `TileKey`
always uses `variant: 0` (`src/map/resolved_terrain.rs:350`). The `has_damaged_data`
field in `ResolvedTerrainCell` exists but the render path never selects `variant: 1`
post-collapse.

The bridge body SHP layer correctly hides destroyed cells (DamageState::Destroyed
`continue` guard). The TMP tile layer shows the destroyed-tile art (variant=1 in
gamemd) but our engine always renders variant=0. This means the terrain UNDER the
destroyed bridge deck shows the wrong (undamaged) sub-tile art. This is a visual
discrepancy but is a "wrong art" bug not a "wrong count" bug — all N cells would
show wrong-art, not just 1.

**Verdict: PASS** for the N-cell iteration count. The `DamageState::Destroyed` guard
correctly excludes all N destroyed cells from the body SHP pass, letting the terrain
TMP pass take over. (Variant selection for TMP is a separate adjacent finding — see §9.)

---

### Stage 3 — Per-cell BridgeExplosions anim spawn: PASS (count correct, probability noted)

`spawn_bridge_debris` (`src/sim/world/bridge_orchestrator.rs:807`) iterates
`cells: &BTreeSet<(u16, u16)>` — one entry per destroyed cell. Per cell:

1. Outer 95% gate: `next_range_u32(20) == 0` → skip (~5% skip rate per cell).
2. Two jitter draws consumed.
3. MetallicDebris 50% gate + optional slot.
4. BridgeExplosions delay + slot → one `WorldEffect` pushed per cell that passes gate.

For N=8 destroyed cells, `sim.world_effects` will receive up to 8 `WorldEffect` entries
(~7.6 on average, ~0.4 suppressed by the 5% gate). The `WorldEffect` struct carries
`delay_ms` which staggers the visible effect over 1-5 frames.

`build_world_effect_instances` (`src/app_instances/overlays.rs:42`) skips effects with
`fx.delay_ms > 0`. A WorldEffect with a pending delay renders nothing until delay
expires. This means explosion sprites are visible per-cell with correct stagger.

**Numerical comparison vs gamemd:**
- gamemd `BlowUpBridge` @ `0x0047DD70` is called once per cell by the cascade walker.
  Each call spawns 1 mandatory BridgeExplosions anim + optional MetallicDebris.
- Our `spawn_bridge_debris` does the same per-cell with matching RNG draw order.

**Verdict: PASS** — N cells produce N BridgeExplosions spawns (subject to identical
~5% per-cell gate). Count equals gamemd per-cell semantics.

---

### Stage 4 — Per-cell splash sound: NOT-IMPLEMENTED

`src/audio/events.rs` defines `GameSoundEvent` variants. There is no
`BridgeCellCollapsed`, `BridgeSplash`, or equivalent variant. The `GameSoundEvent`
enum contains `BridgeRepaired` (repair SFX) but nothing for bridge destruction.

`spawn_bridge_debris` in `bridge_orchestrator.rs` produces only `WorldEffect` entries
(visual anims). It does not push any `GameSoundEvent`.

**gamemd reference:** `CellClass::BlowUpBridge @ 0x0047DD70` contains the splash-sound
call chain. From `BRIDGEEXPLOSIONS_RULES_OFFSETS_GHIDRA_REPORT.md §2` it is confirmed
active in YR. The exact sound ID (bridge collapse / water splash) is controlled by an
INI key resolved at the blast site. The binary confirmed active for YR; no TS gate.

**Player experience:** In gamemd, every destroyed bridge cell produces an audible
splash/explosion. In our engine, zero sounds play for any bridge collapse. With N=8
cells, this is 8 missing sounds per collapse event.

**Verdict: NOT-IMPLEMENTED** — no bridge collapse sound at any granularity.

---

### Stage 5 — Water/terrain reveal under destroyed bridge: FAIL

In gamemd, destroying a bridge cell causes the underlying terrain (typically water)
to become visible. This happens because:
1. The bridge deck TMP tiles are replaced with destroyed-bridge TMP tiles (via the
   `cell+0x140 & 0x2000` damage-variant bit selecting alternate sub-tile art).
2. The `cell+0x140 & 0x80` (HasBridge) flag is cleared on fully destroyed cells,
   dropping the -16px Y offset so the terrain draws at ground level.

In our engine:
- The terrain TMP tiles are static (`TerrainGrid` built at map load, never rebuilt
  after bridge collapse). `build_terrain_grid_from_resolved` runs once (`app_init.rs:328`).
  No terrain tile update happens when `bridge_state_changed`.
- `TileKey.variant` is always 0 in the render path (`resolved_terrain.rs:350`).
  The `has_damaged_data` field in `ResolvedTerrainCell` is parsed but the render path
  (`app_render/build_instances.rs:106`) never passes `variant: 1` to `TileKey`.
- The bridge body SHP render correctly skips `DamageState::Destroyed` cells, but
  without the TMP variant swap, the terrain under destroyed bridge cells still shows
  the bridge-over-water tile art (undamaged) instead of open water.

For N=8 destroyed cells, **all 8** cells show wrong terrain (no water reveal). This
is N-wide, not a 1-vs-N discrepancy in count, but it is a total failure of terrain
reveal for every destroyed cell.

**Verdict: FAIL** — destroyed bridge cells never reveal water. All N destroyed cells
are visually wrong. File: `src/app_render/build_instances.rs:106` (TileKey variant=0
hardcoded), `src/map/resolved_terrain.rs:350` (variant always 0).

---

### Stage 6 — Occupant fall visual: PASS (DropIn runs per occupant, N-wide)

`drop_in_bridge_deck_entities` (`src/sim/world/bridge_orchestrator.rs:894`) iterates
ALL entities at the cell matching `is_on_bridge_layer()`. It is called once per
destroyed cell in `destroyed_set`. For N cells, it runs N times, snapping every
deck entity at each cell to ground level. Entity positions update (`position.refresh_screen_coords()`),
so the next render frame sees all entities at their new ground positions. This
matches gamemd's `BlowUpBridge` step 2 per §11.4.

The visual "fall" is rendered by the entity's normal SHP render pipeline — no
separate fall animation sprite. gamemd also uses DropIn semantics (no fall anim
for vehicles; infantry may have a die anim on kill). Consistent.

**Verdict: PASS** — all occupants on all N destroyed cells drop. Count equals gamemd.

---

### Stage 7 — Minimap/radar bridge pixel removal: FAIL

The minimap `overlay_pixels` list is populated once at init (`minimap.rs:140`) from
the static overlay data. It is never updated after bridge collapse.

`update_unit_dots` (`minimap.rs:211`) re-stamps `overlay_pixels` every tick, but the
list is immutable after construction. No code path removes a `Bridge`-classified
overlay pixel when a cell's `DamageState` becomes `Destroyed`.

`src/sim/world/world_orders.rs:354-356` has an explicit comment: "No render-side
dirty-cell API is wired up yet for bridges; the minimap refreshes after the PathGrid
rebuild driven by `bridge_state_changed`. Reserved for a follow-up once a per-cell
radar-dirty channel exists."

**gamemd reference:** `CellClass__GetRadarColor @ 0x0047C060` reads `cell+0x140 & 0x100`
(bridge structural flag) to draw bridge color. When a bridge is destroyed, the flag is
cleared by the destruction walker, so the radar color falls back to terrain on the next
frame. Confirmed active in YR (`BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT.md §2`).

For N=8 destroyed cells, all 8 bridge pixels persist on the minimap indefinitely.

**Verdict: FAIL** — destroyed bridge cells remain as bridge-colored pixels on minimap
after collapse. File: `src/render/minimap.rs:140` (overlay_pixels built once, never
updated on collapse). `src/sim/world/world_orders.rs:354` (known deferred item).

---

### Stage 8 — Render-tick gating (`bridge_state_changed` flag): PASS

`app_sim_tick.rs:560` sets `refresh_after_tick = true` when `bridge_state_changed`.
The bridge body/shadow/railing renderers (`build_bridge_body_instances`,
`build_bridge_shadow_instances`, `build_bridge_railing_instances`) iterate
`bridge_state.iter_cells()` every frame unconditionally (not gated on a dirty flag).
They do not check `bridge_state_changed`.

This means the render is always current — there is no stale-cache bug here. A changed
bridge state is reflected on the very next frame regardless of the `refresh_after_tick`
flag (which controls PathGrid rebuild, not bridge rendering).

The loop iterates ALL cells, so if all N cells changed, all N changes are reflected.
No "only first dirty cell" pathology.

**Verdict: PASS** — per-frame re-scan of all bridge cells; no single-cell gating.

---

## Adjacent Findings (out of scope for this trace)

1. **TMP variant selection missing (Stage 5):** `TileKey.variant` is always 0
   (`resolved_terrain.rs:350`). The `has_damaged_data` bool is parsed and stored in
   `ResolvedTerrainCell` but the tile renderer never reads it post-collapse. The
   destroyed-bridge TMP variant (gamemd `cell+0x140 & 0x2000`) is never applied.
   File: `src/app_render/build_instances.rs:106`, `src/map/resolved_terrain.rs:349-350`.

2. **Minimap overlay refresh on collapse (Stage 7):** `overlay_pixels` list in
   `MinimapRenderer` is built at map init and never mutated. Destroyed bridge cells
   remain on minimap indefinitely. A `rebuild_overlay_pixels()` method would need to
   be called with updated `bridge_state` after each collapse.
   File: `src/render/minimap.rs:140`. Confirmed deferred at `src/sim/world/world_orders.rs:354`.

3. **Bridge shadow displacement direction label:** `BRIDGE_RENDERING_GHIDRA_REPORT.md §4`
   states shadow displacement applies to NS bridges (states 9-17). `bridges.rs:256`
   applies the shift for `axis == Axis::EW`, which may be a label-swap.
   Not confirmed against retail game; left as adjacent finding.

4. **Missing `BridgeExplosions` sound emission:** The `spawn_bridge_debris` function
   spawns visual `WorldEffect` entries only. The binary `BlowUpBridge` also triggers
   audio. The exact sound key from INI is not yet identified in the Rust port. Needs
   a targeted RE session at the `BlowUpBridge` sound call site.

---

## Verdict Tally

**PASS: 5 | FAIL: 2 | UNCHECKED: 0 | NOT-IMPLEMENTED: 1**

---

## Top 5 Player-Visible Failures

1. **Stage 5 — Water reveal absent:**
   After N cells destroyed, player sees undamaged bridge-deck terrain tiles instead
   of open water beneath. No "bridge drops into water" visual for any of the N cells.
   File: `src/app_render/build_instances.rs:106` + `src/map/resolved_terrain.rs:350`.
   gamemd: `TMP_TileBlitter` param_14 variant select via `cell+0x140 & 0x2000`
   (`BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md §3.2`). Fires every collapse, every cell.

2. **Stage 4 — Bridge collapse sounds completely absent:**
   Player hears nothing when the bridge collapses. In gamemd, each destroyed cell
   produces an audible splash/explosion from `BlowUpBridge @ 0x0047DD70`.
   File: `src/audio/events.rs` (no `BridgeCollapse` event variant exists).
   `src/sim/world/bridge_orchestrator.rs:807` (no sound push in `spawn_bridge_debris`).
   Fires every collapse, all N cells affected.

3. **Stage 7 — Destroyed bridge cells remain on minimap:**
   Player sees bridge pixels on minimap long after bridge is gone. In gamemd,
   radar color updates next frame via `CellClass__GetRadarColor @ 0x0047C060`
   reading the cleared `cell+0x140 & 0x100` flag.
   File: `src/render/minimap.rs:140` (overlay_pixels built once at init).
   Fires every collapse, persists until session end.

4. **Stage 5 (secondary) — Destroyed bridge deck still shows valid bridge art:**
   The bridge-body SHP pass correctly skips destroyed cells, but without the TMP
   variant swap the terrain under the "gap" shows intact bridge tiles, not water.
   This makes the bridge appear to have an invisible platform where it collapsed.
   File: `src/map/resolved_terrain.rs:350` (variant always 0).
   Fires every collapse, all N cells.

5. **Stage 4 — No per-cell audio variation:**
   Even if a single sound were added, it would need to fire once per destroyed cell
   with spatial positioning. The current `WorldEffect` spawn loop is N-wide but the
   audio layer has no equivalent N-wide bridge-collapse event loop.
   File: `src/sim/world/bridge_orchestrator.rs:807` (no sound loop).
   Fires every collapse.

---

## Report File

`docs/research/traces/CABHUT_BRIDGE_COLLAPSE_VISUAL_TRACE.md`

---

## Status: COMPLETE
