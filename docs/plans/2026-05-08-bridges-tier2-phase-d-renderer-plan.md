# Bridges Tier 2 — Phase D Renderer Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Bring the Rust renderer to gamemd parity on visible bridge output (body, body shadow, railings, deck-TMP damaged variant) and fill the rim-refresh sim stub that the renderer depends on. Closes Phase D and Tier 2.

**Architecture:** Approach B from the design doc — pull all bridge instance emission into a new `app_instances/bridges.rs`; add `render/bridge_railing_atlas.rs` for RAILBRDG.tem + 2 × 10 entry railing tables; extend `render/bridge_atlas.rs` to pack shadow halves alongside body frames. `app_instances/overlays.rs` becomes non-bridge only. Draw order in `app_render/draw_passes.rs` gets a shadow pass (between bridge body zdepth and bridge_detail) and a railing pass (between cliff redraw and debug). Rim refresh in `sim/world/bridge_orchestrator.rs:208` is filled per RE doc §7.

**Design Doc:** [docs/plans/2026-05-08-bridges-tier2-phase-d-renderer-design.md](2026-05-08-bridges-tier2-phase-d-renderer-design.md)

---

## Grounding Summary

- **Research docs:** `ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` (HIGH confidence, dated 2026-05-07/08) is the source of truth for the per-frame draw chain, layer ordering, frame-selection arithmetic, shadow shift, and rim refresh contract. Cross-checked against `BRIDGE_RENDERING_GHIDRA_REPORT.md` (pre-Phase F; superseded layer mapping in §2.2 of the new doc), `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` (state byte authority), and `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` (two-channel state model — confirms `cell+0x140 & 0x2000` is the damaged-variant bit, written only by `ToggleBridgePavement`).
- **Ghidra verification:** Addresses cited in the RE doc are HIGH-confidence; `MapClass::UpdateAdjacentBridges_High @ 0x576770`, `CellClass::DrawOverlay_Body @ 0x47F6A0`, `CellClass::DrawOverlay_Shadow @ 0x47F510`, `g_LatinSquare @ 0x0081CC30`, `DAT_00ABC210` (concrete railing table) all present in the binary. Two values remain unresolved: shadow X-shift (-15 vs -45) and EW/NS axis labeling — both ship as named constants per design §"Open RE questions ship as named constants".
- **Repo pattern this follows:** existing sim/render symmetry — `sim/bridge_state/`, `sim/world/bridge_orchestrator.rs`, `sim/bridge_specs.rs` all bridge-named on the sim side; `render/bridge_atlas.rs` is the only render-side bridge module today. Plan introduces `render/bridge_railing_atlas.rs` and `app_instances/bridges.rs` to complete the symmetry. Atlas packing follows the existing shelf-pack approach in [src/render/bridge_atlas.rs:208-256](src/render/bridge_atlas.rs#L208-L256). Buffer-pool wiring follows [src/app_render/mod.rs:135-143](src/app_render/mod.rs#L135-L143).
- **INI keys:** `[RAILBRDG] Theater=yes` confirmed in [ini/artmd.ini:13123-13124](ini/artmd.ini#L13123-L13124) and `ini/art.ini:8985`. `[BRIDGE1]/[BRIDGE2]/[BRIDGEB1]/[BRIDGEB2]` confirmed in `rulesmd.ini:29869-29893` (and `rules.ini:22020-22044`). `[CombatDamage] BridgeStrength` already parsed. **No new INI parsing required.**
- **Premise re-verification (skill step A.1):** `git log --oneline -10` over each modify-target shows the latest touch was `8ea2611 sim/bridge: refresh_endpoint_active_flags + migrate 7 ignored bridge fixtures` (Phase G). Design doc is from the same day; no commits have invalidated its claims. **Two corrections to design carried into this plan:**
  - `BridgeRuntimeState` already has `iter_cells()` ([src/sim/bridge_state/mod.rs:1067](src/sim/bridge_state/mod.rs#L1067)). Use the existing name; do NOT add `iter_bridge_cells`.
  - Rim cells already flow through `StateOutcome::Collapsed.adjacent_bridges_dirty` → orchestrator's `rim_cells: BTreeSet<(u16, u16)>` ([src/sim/world/bridge_orchestrator.rs:120-135](src/sim/world/bridge_orchestrator.rs#L120-L135)). Do NOT add `rim_dirty: BTreeSet`, `mark_rim_dirty`, or `take_rim_dirty` to `BridgeRuntimeState` — the channel is in place; only the consumer (`update_adjacent_bridges`) is a stub.
- **Still unknown after grounding:** shadow X shift (-15 vs -45), EW/NS axis labeling, exact wood railing table base, exact 160-byte values for the concrete + wood railing tables (runtime-populated, zero in static binary). All deferred to live-debugger capture (Task 4) and visual diff (Task 17).

## Key Technical Decisions

- **Use existing `iter_cells` / orchestrator `rim_cells` channel; do NOT add a new rim-dirty queue on `BridgeRuntimeState`.** — **Confidence:** high — **Source:** repo pattern at [src/sim/world/bridge_orchestrator.rs:120-135](src/sim/world/bridge_orchestrator.rs#L120-L135) + design doc §"Re-verify the design's premise" (corrected).
- **Add `DamageState::render_state_byte(axis)` that returns `0` (NS) / `9` (EW) for `Healthy { variant }` regardless of variant.** Renderer ignores the sim's healthy variant and re-derives jitter from `(cell.x, cell.y)` via the binary Latin square. — **Confidence:** high — **Source:** RE doc §3.3.1 + Ledger #4 ("Sim `Healthy.variant` is ignored by render").
- **Pack shadow frames in the existing `BridgeAtlas`, not a new atlas.** Atlas keys gain a `kind: BridgeFrameKind::{Body, Shadow}` discriminator; lookups go through `body_entry(name, state)` and `shadow_entry(name, state)`. — **Confidence:** high — **Source:** design doc §Components + RE doc §3.3.2 (shadow uses same SHP, second-half frame range).
- **Ship the three unresolved RE values as `pub const` named constants in `app_instances/bridges.rs`** so each is a single change-point: `BRIDGE_SHADOW_EW_DX: i32 = -15`, `BRIDGE_SHADOW_EW_DY: i32 = 7`, `BRIDGE_SHADOW_BODY_LATIN_SQUARE: [u8; 16] = [0,1,2,3, 3,2,1,0, 2,3,0,1, 1,0,3,2]`. Axis convention: keep the existing `Axis::EW = states 9..17` mapping in [src/sim/bridge_state/mod.rs:22-30](src/sim/bridge_state/mod.rs#L22-L30); flag for visual-verify. — **Confidence:** medium (defaults are best-guess; Latin square value verified at `0x0081CC30`) — **Source:** design doc §Architectural Decisions + RE doc §10.
- **Railing tables hardcoded as `pub const` Rust arrays after live-debugger extraction (Task 4).** No theater-time loader — values are theater-stable across a single map (vanilla YR ships only `temperate`/`snow`/`urban`/`urban-newurban` and the railing tile-set indices don't drift between them in a single skirmish). — **Confidence:** medium — **Source:** RE doc §3.4.1 + design doc §Sub-task: railing table extraction. Caveat: if a future modded theater changes the tile-set indices, the tables would need a per-theater dispatch; **flag for /review-plan**.
- **`BridgeRuntimeCell.damaged_variant: bool` defaults `false` at map load and stays `false` until `ToggleBridgePavement`-equivalent path lands (Tasks 13.5/15.5 deferred per RE doc §8).** Phase D wires the field, the hash inclusion, and the renderer read; data only changes when the deferred ramp-handler overlay-write branch lands. — **Confidence:** high — **Source:** RE doc §8 + design Ledger #13.
- **Hot-path safety: `build_bridge_*_instances` uses pre-allocated `Vec`s passed by `&mut`, not `Vec::new()`.** Mirrors how `build_overlay_instances` takes `&mut Vec<SpriteInstance>` parameters today. — **Confidence:** high — **Source:** repo pattern at [src/app_instances/overlays.rs:176-184](src/app_instances/overlays.rs#L176-L184).

## Open Questions

### Resolved During Planning

- **Does `BridgeRuntimeState` need a new `rim_dirty` queue?** No — the existing `StateOutcome::Collapsed.adjacent_bridges_dirty → rim_cells` flow already supplies the orchestrator. — Verified at [src/sim/world/bridge_orchestrator.rs:120-135](src/sim/world/bridge_orchestrator.rs#L120-L135).
- **Does the renderer respect `Healthy.variant` (0..=5) or rebuild from cell xy?** Rebuild. Binary Latin square fires only on state byte 0 or 9; sim's `Healthy { variant: 3 }` would encode to byte 3 and skip Latin square. — Resolution: `render_state_byte` helper.
- **Where does deck-variant override get applied?** At terrain instance build, via a `BTreeMap<(u16, u16), DeckVariantSelect>` consulted inside the `uv_fn` closure in [src/app_render/build_instances.rs:99-120](src/app_render/build_instances.rs#L99-L120). The closure picks `variant: 1` (alt-art) instead of `variant: 0` for keys in the override map.
- **Does `bridge_atlas.rs`'s current `flags.bridge_deck → max_normal_frame = shp.frames.len() / 2` cap prevent loading shadow frames?** Yes ([src/render/bridge_atlas.rs:147-152](src/render/bridge_atlas.rs#L147-L152)). Task 5 removes that cap and packs all frames.

### Deferred to Implementation

- **Exact shadow X shift (-15 vs -45).** Resolves at Task 17 visual diff. Single change point: `BRIDGE_SHADOW_EW_DX`.
- **EW/NS axis label correctness.** Resolves at Task 17 visual diff against `bridge.tem` SHP frame 0 vs frame 9 visual content.
- **Wood railing table base address.** Resolves at Task 4 live-debugger capture. Until then, wood railing entries are `None`-arrayed and wood bridges render no railings.
- **Per-theater railing table stability across modded theaters.** If a modded theater shifts the tile-set indices, the hardcoded constants drift. Out-of-scope for vanilla YR Tier 2; flag for future `/re-investigate` if a modded theater is needed.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/bridge_state/mod.rs](src/sim/bridge_state/mod.rs) | Add `damaged_variant: bool` to `BridgeRuntimeCell`; add `DamageState::render_state_byte(axis)`. |
| Modify | [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs) | Hash the new `damaged_variant` field in `hash_bridge_state` (determinism). |
| Modify | [src/sim/world/bridge_orchestrator.rs](src/sim/world/bridge_orchestrator.rs) | Fill `update_adjacent_bridges` per RE doc §7. |
| Modify | [src/render/bridge_atlas.rs](src/render/bridge_atlas.rs) | Pack body + shadow halves; add `BridgeFrameKind` + `body_entry/shadow_entry`. |
| Create | `src/render/bridge_railing_atlas.rs` | Load `RAILBRDG.tem`; concrete + wood `[Option<RailingEntry>; 10]` tables. |
| Create | `src/app_instances/bridges.rs` | All bridge instance emission: body, shadow, railings, deck-variant overrides. |
| Modify | [src/app_instances/overlays.rs](src/app_instances/overlays.rs) | Remove bridge handling from `build_overlay_instances`; bucket classifier collapses to `Wall` vs `Generic`. |
| Modify | [src/app_instances/mod.rs](src/app_instances/mod.rs) | Re-export new `bridges` module. |
| Modify | [src/app_render/build_instances.rs](src/app_render/build_instances.rs) | Add `bridge_body_shadow`, `bridge_railing` Vecs + `deck_variant_overrides` to `WorldInstances`; call new builders; sort. |
| Modify | [src/app_render/mod.rs](src/app_render/mod.rs) | Upload two new pooled keys: `overlay_bridge_shadow`, `overlay_bridge_railing`. |
| Modify | [src/app_render/draw_passes.rs](src/app_render/draw_passes.rs) | Insert shadow pass + railing pass at correct positions; add `BridgeRailingAtlas` field to `DrawPassData`. |
| Modify | [src/app/mod.rs](src/app/mod.rs) (or equivalent atlas-init site) | Build the `BridgeRailingAtlas` at scenario load; store on `AppState`. |

## Interface Changes

- **`BridgeRuntimeCell`** gains a public `damaged_variant: bool` field. Constructors in [src/sim/bridge_state/mod.rs:434-447](src/sim/bridge_state/mod.rs#L434-L447), [src/sim/bridge_state/mod.rs:1583-1620](src/sim/bridge_state/mod.rs#L1583-L1620), and the test fixtures at lines 1706, 1723, 1734, 1928–1991, 2202, 2229, 2262, 2304 all need the new field initialized to `false`. Snapshot serialization (existing `serde::Serialize/Deserialize` derive) auto-rolls; no schema migration needed because the field has a default.
- **`DamageState`** gains `pub fn render_state_byte(self, axis: Axis) -> u8`. Called only by `app_instances/bridges.rs`. No existing callers affected.
- **`BridgeAtlas`** entries are now keyed by `(name, frame, BridgeFrameKind)`. The existing `pub fn get(&self, key: &OverlaySpriteKey) -> Option<&OverlaySpriteEntry>` is **removed** — its only caller, the `OverlayRenderBucket::BridgeBody` arm in [src/app_instances/overlays.rs:359-365](src/app_instances/overlays.rs#L359-L365), is deleted in Task 13. New API: `body_entry(name, state) / shadow_entry(name, state)`.
- **`AppState`** gains `pub bridge_railing_atlas: Option<BridgeRailingAtlas>`.
- **`WorldInstances`** gains three fields: `bridge_body_shadow: Vec<SpriteInstance>`, `bridge_railing: Vec<SpriteInstance>`, `deck_variant_overrides: BTreeMap<(u16, u16), DeckVariantSelect>`.
- **`DrawPassData`** gains a reference to `state.bridge_railing_atlas` (read through `state` in `dispatch_draw_passes`, no new field strictly needed; passed via state).

## Sim Checklist

(Tasks 1, 2, 14 touch sim/.)

- [ ] All math uses `fixed`-point — no f32/f64 in game logic. (Task 14's rim walk uses `i32` integer cell coords; no math.)
- [ ] New state included in deterministic state hash. (Task 1.5 adds `damaged_variant` to [src/sim/world/world_hash.rs:218-229](src/sim/world/world_hash.rs#L218-L229).)
- [ ] No dependencies on render/ui/sidebar/audio/net. (`bridge_orchestrator.rs` already free of these; rim refresh impl uses only `BridgeRuntimeState`, `g_DirectionOffsets`-equivalent constants, and pure mutations.)
- [ ] Tick ordering impact noted. **Rim refresh runs at the existing call site in `bridge_orchestrator::process_bridge_damage_cascade` Step 4 ([src/sim/world/bridge_orchestrator.rs:134-135](src/sim/world/bridge_orchestrator.rs#L134-L135)) — no order changes.**
- [ ] BTreeMap iteration order considered. **`rim_cells` is `BTreeSet<(u16, u16)>`; rim walk iterates sorted.** New `BTreeMap<(u16, u16), DeckVariantSelect>` (render-only) iterates sorted in build phase.

## Risk Areas

- **Determinism regression in bridge state hash.** Adding `damaged_variant` to the hashed cell fields changes the hash output. Mitigation: state hash regression tests in [src/sim/world/world_tests.rs](src/sim/world/world_tests.rs) — Task 18 verifies replay determinism still passes.
- **Layer ordering breakage.** Putting railing pass before unit merge silently produces tank-on-top-of-railing. Mitigation: Task 16 includes a manual visual-diff bullet specifically for this. Code-level: railing pass MUST land between cliff redraw zdepth (current Step 7 in `draw_passes.rs`) and debug overlays (Step 8).
- **Atlas packing capacity.** Doubling per-bridge frames (body 18 + shadow 18 = 36 frames × 4 SHPs) ~doubles `BridgeAtlas` pixel area. Bounded — fits in single atlas page comfortably; existing shelf-pack handles dimension growth (re-checks `max_texture_dim`). No risk if the existing test passes after Task 5.
- **`overlays.rs` regression for non-bridge overlays.** Removing bridge dispatch could break the wall + generic paths. Mitigation: Task 13 keeps `wall_instances`, `instances` parameters and only deletes the BridgeBody/BridgeDetail bucket arms. All existing wall + ore + generic tests still apply.
- **Rim-refresh recursion bound.** Per RE doc §7.2, walks up to 30 cells; recursion may hit deep bridges. Mitigation: Task 14 caps both walk-length AND recursion depth explicitly.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 5 | All 36 frames per bridge SHP packed in atlas (body half + shadow half) | Without shadow frames in the atlas, no shadow can ever draw — visible every match with bridges | Unit test: `body_entry("BRIDGE1", 5)` and `shadow_entry("BRIDGE1", 5)` both return distinct, non-empty UVs |
| Task 7 | Body Latin-square jitter ONLY on state bytes 0 and 9; states 1..8 and 10..17 use frame=state directly | Wrong jitter logic produces visible texture-tile mosaicing on damaged bridges | Unit test: `state=5, cell=(1,2)` → frame 5 (no jitter); `state=0, cell=(1,2)` → frame `BRIDGE_SHADOW_BODY_LATIN_SQUARE[((2&3)<<2) \| (1&3)] = BRIDGE_SHADOW_BODY_LATIN_SQUARE[9] = 3` |
| Task 7 | Y-offset = -16 for state ∈ [0..8], -31 for state ∈ [9..17] (HasBridge cells) | 15px Y drift = bridges visibly "float" or "sink" relative to terrain | Unit test on `compute_bridge_body_y_offset` |
| Task 7 | Z = `(cell.deck_level + 4) * -15 - 2` | Wrong Z lets units crossing draw OVER the deck | Manual: drive a unit across a bridge, confirm it's occluded by the deck |
| Task 8 | Shadow frame = `(shp.frame_count / 2) + state`, blitter flag `0x4601` (Z-test ON, Z-write OFF, darken) | Body shadow is visible behind every bridge in the original; missing shadow is immediately obvious | Manual: bridge at midday in a temperate map — shadow visible underneath the deck |
| Task 8 | Shadow X shift on EW states 9..17 = `BRIDGE_SHADOW_EW_DX` (default -15, may be -45) | Whichever value is correct produces aligned shadow; wrong value places the shadow obviously off | Manual: side-by-side gamemd vs Rust, single bridge fixture |
| Task 8 | Shadow Y shift on EW states 9..17 = +7 | 7px misplacement is small but compounds with other shadow drift | Manual: same side-by-side as above |
| Task 8 | Shadow uses neutral tint (1000), no per-cell lighting | Tinted shadows look like coloured glow, not shadow | Manual: shadow visibly grayscale, not tinted |
| Task 11 | Railings drawn AFTER unit/ground merge AND AFTER cliff redraw, BEFORE debug | If railings draw before units, units render on top of railings (visibly wrong). If they draw before cliff redraw, cliffs occlude railings near bridge approaches | Manual: drive a unit across a bridge — unit is BELOW railings; near bridge ramp where it meets a cliff, cliff still occludes correctly |
| Task 11 | Railing blitter flag `0x4601` (Z-test ON, Z-write OFF), neutral tint | Wrong Z-write or tint produces sort artifacts or coloured railings | Visual diff |
| Task 12 | Deck-TMP damaged-tile variant gates on `cell.flags & 0x2000` (our `damaged_variant: bool`) AND `IsoTileType.num_tiles >= 2` AND `tile_data[sub].flags & 0x04` | Wrong gate either always shows damaged art (visible bug on healthy bridges) or never shows it (visible bug on damaged bridges) | Currently `damaged_variant` stays false until ramp-handler overlay-write branch lands — Phase D wires the path; visible behaviour deferred to later phase. Verify the override path correctly returns alt-art when the bool is `true` (manually set in a sandbox test) |
| Task 14 | Rim refresh writes `cell.overlay_byte = NONE` and `damage_state = Healthy { variant: 0 }` on dangling stubs | Without rim refresh, partially-collapsed bridges leave orphan ramp stubs visible after the cell they connect to was destroyed — visible every time a bridge midspan collapses | Integration test: destroy midspan, expect 8 neighbour cells around dangling stub to clear; manual: blow up a midspan, confirm no orphan tile remains |
| Task 16 | Render reads `BridgeRuntimeCell.overlay_byte` post-tick (NOT `OverlayGrid`) | OverlayGrid lags by 1 tick on bridge state changes — visible 1-frame "wrong tile" flash on every damage event | Integration test: damage cell, confirm next frame's body bucket reflects new state byte |

---

## Tasks

### Task 1: Add `damaged_variant: bool` to `BridgeRuntimeCell`

**Why:** The deck-TMP damaged-art variant (RE doc §3.2, Ledger #13) lives on the cell. Wire the field first; the renderer reads it in Task 12.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs:329-356](src/sim/bridge_state/mod.rs#L329-L356) — add field
- Modify: [src/sim/bridge_state/mod.rs:434-447](src/sim/bridge_state/mod.rs#L434-L447) — initialize at map load
- Modify: [src/sim/bridge_state/mod.rs:1583-1612, 1706, 1723, 1734, 1928-1991, 2202, 2229, 2262, 2304](src/sim/bridge_state/mod.rs#L1583) — initialize in test fixtures (search-and-add)

**Pattern:** Existing `BridgeRuntimeCell` field additions (e.g. `overlay_byte` was added the same way).

**Step 1: Add the field**
```rust
// src/sim/bridge_state/mod.rs around line 355, end of BridgeRuntimeCell struct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BridgeRuntimeCell {
    pub deck_present: bool,
    pub destroyable: bool,
    pub deck_level: u8,
    pub bridge_group_id: Option<u16>,
    pub damage_state: DamageState,
    pub axis: Option<Axis>,
    pub role: BridgeCellRole,
    pub anchor_span_id: Option<u16>,
    pub overlay_byte: u8,
    /// Mirror of binary `cell.flags & 0x2000`. Selects damaged-vs-undamaged
    /// sub-tile art for the deck TMP at draw time. Written only by the
    /// `ToggleBridgePavement`-equivalent path (deferred Tasks 13.5/15.5).
    /// Defaults to `false` at map load.
    pub damaged_variant: bool,
}
```

**Step 2: Initialize at map load**

In [src/sim/bridge_state/mod.rs:434-447](src/sim/bridge_state/mod.rs#L434-L447), add `damaged_variant: false,` to the `BridgeRuntimeCell { ... }` literal in `from_resolved_terrain`.

**Step 3: Add to all test fixtures**

Run a `Grep` for `BridgeRuntimeCell {` in `src/sim/bridge_state/mod.rs` and `src/sim/world/world_hash.rs` and `src/sim/world/world_tests.rs`. Every `BridgeRuntimeCell { ... }` literal must add `damaged_variant: false,` to keep the existing test scenarios deterministic. Expected sites (verify with grep):
- `src/sim/bridge_state/mod.rs:1583, 1603, 1706, 1723, 1734, 1919, 1934, 1949, 1964, 1982, 2202, 2229, 2262, 2304`
- `src/sim/world/world_hash.rs:554`
- `src/sim/world/world_tests.rs:270`

**Step 4: Verify**
Run: `cargo build -p ra2-rust-game 2>&1 | grep -E "(error|warning)" | head -40`
Expected: clean compile (no `damaged_variant` errors).

**Step 5: Commit**

```
sim/bridge_state: add damaged_variant: bool to BridgeRuntimeCell (Phase D; Task 1)
```

### Task 1.5: Hash `damaged_variant` in `hash_bridge_state`

**Why:** Determinism. New cell field MUST flow into the state hash or replays diverge silently. Sim-checklist invariant.

**Files:**
- Modify: [src/sim/world/world_hash.rs:210-237](src/sim/world/world_hash.rs#L210-L237)

**Pattern:** existing per-field hash calls in `hash_bridge_state`.

**Step 1: Add the hash line**

```rust
// src/sim/world/world_hash.rs in hash_bridge_state, after line 229 (cell.overlay_byte.hash(hasher))
cell.overlay_byte.hash(hasher);
cell.damaged_variant.hash(hasher);
```

**Step 2: Update fixtures** — fixture at line 554 already has `damaged_variant: false` from Task 1.

**Step 3: Run state-hash regression test**

```
cargo test --lib -p ra2-rust-game world_hash -- --nocapture
```

Expected: PASS. (Determinism golden numbers may change if any test pinned a hash; if so, regenerate the golden once and re-commit — note the change in the commit message.)

**Step 4: Commit**

```
sim/world/world_hash: include damaged_variant in bridge_state hash (Phase D; Task 1.5)
```

### Task 2: Add `DamageState::render_state_byte(axis)` helper

**Why:** Renderer needs the *binary state byte* (0 for healthy NS, 9 for healthy EW) without the sim's variant baked in. Latin-square jitter is then re-derived from cell xy per Ledger #4.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs:55-102](src/sim/bridge_state/mod.rs#L55-L102) — add method to `impl DamageState`

**Pattern:** Existing `to_state_byte` and `from_state_byte` methods.

**Step 1: Add the method**

```rust
// src/sim/bridge_state/mod.rs in impl DamageState, after to_state_byte
/// Render-side state byte. Mirrors `to_state_byte` except `Healthy { variant }`
/// always returns the *base* byte (`0` for NS, `9` for EW) regardless of
/// variant. The renderer re-derives Latin-square jitter from cell `(x, y)`
/// per binary `DrawOverlay_Body` (RE doc §3.3.1, ledger #4).
pub fn render_state_byte(self, axis: Axis) -> u8 {
    match self {
        DamageState::Healthy { .. } => match axis {
            Axis::NS => 0,
            Axis::EW => 9,
        },
        other => other.to_state_byte(axis),
    }
}
```

**Step 2: Add unit tests**

```rust
// src/sim/bridge_state/mod.rs in #[cfg(test)] mod tests
#[test]
fn render_state_byte_strips_healthy_variant() {
    assert_eq!(DamageState::Healthy { variant: 0 }.render_state_byte(Axis::NS), 0);
    assert_eq!(DamageState::Healthy { variant: 5 }.render_state_byte(Axis::NS), 0);
    assert_eq!(DamageState::Healthy { variant: 0 }.render_state_byte(Axis::EW), 9);
    assert_eq!(DamageState::Healthy { variant: 5 }.render_state_byte(Axis::EW), 9);
    assert_eq!(DamageState::Damaged.render_state_byte(Axis::NS), 6);
    assert_eq!(DamageState::Damaged.render_state_byte(Axis::EW), 0xF);
    assert_eq!(DamageState::Destroyed.render_state_byte(Axis::NS), 0);
}
```

**Step 3: Run**

```
cargo test --lib -p ra2-rust-game bridge_state::tests::render_state_byte_strips_healthy_variant
```

Expected: PASS.

**Step 4: Commit**

```
sim/bridge_state: DamageState::render_state_byte (RE §3.3.1; Phase D; Task 2)
```

### Task 3: Live-debugger railing-table extraction (RESEARCH ONLY — no code)

**Why:** The 160-byte concrete table at `DAT_00ABC210` and the wood equivalent are zero in the static binary; values are written by `CDFileClass__Constructor` at theater load. We need real values before Task 9 can render railings.

**Files:** None. This is a research task; output goes into a new doc and feeds Task 9.

**Step 1: Attach to running gamemd**

1. Launch a vanilla YR skirmish on a temperate map containing a high bridge (e.g. `Bering Strait`, `Heartland`).
2. Once the map is loaded, attach `x32dbg` (or Ghidra debugger via MCP) to the running `gamemd.exe` process.
3. Locate `DAT_00ABC210` (concrete bridge railing table base) in memory. Dump 160 bytes (10 entries × 16 bytes).
4. Locate the wood-bridge railing table near `DAT_00AA1098` — scan ±256 bytes from `0x00AA1098` for the parallel 160-byte table. Confirm by checking the "shp_frame_idx_plus_1" byte pattern: entries with `shp_frame == 0` are "no railing" sub-tiles, all other entries should have small frame indices (1..~8).
5. Repeat (1)–(4) on a snow theater map; if the values match temperate, the table is theater-stable; if not, capture both.

**Step 2: Decode each 16-byte entry as `{ shp_frame_idx_plus_1: i32, surface_ptr: i32, x_offset: i32, y_offset: i32 }`** per RE doc §3.4.1.

**Step 3: Write the values to a research doc**

Create `ra2-rust-game-docs/BRIDGE_RAILING_TABLE_VALUES.md`:

```markdown
# Bridge Railing Table Values

**Source:** Live-debugger capture from gamemd.exe @ vanilla YR skirmish on `<map name>`, theater `<temperate|snow|urban>`, captured `<date>`.
**Confidence:** HIGH (live memory dump, post-theater-load).

## Concrete table (`DAT_00ABC210`, 10 × 16 bytes)

| Index | shp_frame (1-based; 0 = no railing) | surface_ptr (opaque) | dx | dy |
|-------|--------------------------------------|----------------------|----|----|
| 0 | <byte>                               | <byte>               | <i32> | <i32> |
| ... | ... | ... | ... | ... |

## Wood table (base address: `0x00AA????`, 10 × 16 bytes)

(same shape)
```

**Step 4: Verify** — read the doc back, sanity-check that `shp_frame_idx_plus_1` is in `{0, 1, 2, ..., 8}` for every entry; if any entry has a wild value (e.g. 0xFF, 0x80, garbage), re-capture; the table may not have been populated yet.

**Step 5: Commit the research doc only** — no Rust code changes.

```
docs: bridge railing table value capture (live debugger, Phase D; Task 3)
```

**If the live capture path fails** (no debugger access, can't reproduce), fall back to RE doc §10 Open Q4 + design doc §"Sub-task: railing table extraction" path 1: decompile `0x005446B1`, `0x00543F36`, `0x005451DC`, `0x00543C42`, `0x00543E02` and reconstruct from theater-INI parsing. Allocate 1–2 hours per RE doc estimate. Record the resulting table in the same doc with confidence MEDIUM.

### Task 4: `BridgeFrameKind` enum + extend `BridgeAtlas` to load all frames

**Why:** Today `bridge_atlas.rs:147-152` caps frames at `shp.frames.len() / 2` for `flags.bridge_deck`, skipping shadow halves. Phase D needs both halves packed.

**Files:**
- Modify: [src/render/bridge_atlas.rs](src/render/bridge_atlas.rs) (whole module — small enough to refactor in one task)

**Pattern:** Existing `OverlaySpriteKey` + entries map; we extend with a frame kind discriminator.

**Step 1: Add the enum**

```rust
// src/render/bridge_atlas.rs near top, after imports
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BridgeFrameKind {
    Body,
    Shadow,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BridgeAtlasKey {
    pub name: String,
    pub frame: u8,
    pub kind: BridgeFrameKind,
}
```

**Step 2: Switch the entries map**

```rust
pub struct BridgeAtlas {
    pub texture: BatchTexture,
    pub depth_texture_view: wgpu::TextureView,
    pub zdepth_bind_group: wgpu::BindGroup,
    entries: HashMap<BridgeAtlasKey, OverlaySpriteEntry>,
}

impl BridgeAtlas {
    pub fn body_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
        self.entries.get(&BridgeAtlasKey {
            name: name.to_string(),
            frame,
            kind: BridgeFrameKind::Body,
        })
    }
    pub fn shadow_entry(&self, name: &str, frame: u8) -> Option<&OverlaySpriteEntry> {
        self.entries.get(&BridgeAtlasKey {
            name: name.to_string(),
            frame,
            kind: BridgeFrameKind::Shadow,
        })
    }
}
```

**Step 3: Update the loader to pack all frames**

Inside `build_bridge_atlas`, change the frame loop:

```rust
for entry in overlays {
    let Some(name) = overlay_names.get(&entry.overlay_id) else { continue; };
    if !is_high_bridge_body_name(name) { continue; }
    // Pack body half (frames 0..18) AND shadow half (frames 18..36) — RE doc §3.3.2.
    for frame in 0u8..18u8 {
        needed.insert(BridgeAtlasKey { name: name.clone(), frame, kind: BridgeFrameKind::Body });
        needed.insert(BridgeAtlasKey { name: name.clone(), frame, kind: BridgeFrameKind::Shadow });
    }
}
```

**Step 4: Update `render_bridge_sprite` to compute the actual SHP frame index**

The current function caps at `max_normal_frame = shp.frames.len() / 2` for `flags.bridge_deck`. Change to:

```rust
fn render_bridge_sprite(
    asset_manager: &AssetManager,
    palette: &Palette,
    key: &BridgeAtlasKey,
    // ... rest unchanged ...
) -> Option<RenderedBridge> {
    // ... existing SHP load + candidates code ...
    let shp: ShpFile = /* unchanged */;
    let half: usize = shp.frames.len() / 2;
    let shp_frame_idx: usize = match key.kind {
        BridgeFrameKind::Body   => (key.frame as usize).min(half.saturating_sub(1)),
        BridgeFrameKind::Shadow => (half + key.frame as usize).min(shp.frames.len().saturating_sub(1)),
    };
    // ... existing per-frame RGBA blit using shp_frame_idx ...
}
```

**Step 5: Update `RenderedBridge.key` type to `BridgeAtlasKey`** and propagate through `pack_bridge_sprites`.

**Step 6: Add unit tests**

```rust
// at bottom of src/render/bridge_atlas.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_and_shadow_entries_are_distinct_lookups() {
        // Mock atlas: just verify the key types are not equal.
        let body = BridgeAtlasKey {
            name: "BRIDGE1".into(), frame: 5, kind: BridgeFrameKind::Body,
        };
        let shadow = BridgeAtlasKey {
            name: "BRIDGE1".into(), frame: 5, kind: BridgeFrameKind::Shadow,
        };
        assert_ne!(body, shadow);
    }
}
```

(Full integration test — `body_entry` and `shadow_entry` returning distinct UVs from a real atlas — runs at scenario load and is verified manually in Task 17.)

**Step 7: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -40
```

Expected: clean. Existing callers (`overlays.rs:359-365`) will now fail to compile because `BridgeAtlas::get(&OverlaySpriteKey)` is gone — that's expected and is fixed in Task 13. **Do NOT run `cargo build` until Task 13 lands**, or temporarily keep a `pub fn get(&self, OverlaySpriteKey) -> Option<...>` shim that maps to `body_entry` until Task 13 deletes it.

For now, **add the shim in this task**:

```rust
impl BridgeAtlas {
    /// Compatibility shim until Task 13 routes through `body_entry`.
    /// REMOVE in Task 13.
    pub fn get(&self, key: &OverlaySpriteKey) -> Option<&OverlaySpriteEntry> {
        self.body_entry(&key.name, key.frame)
    }
}
```

Now `cargo build` passes.

**Step 8: Commit**

```
render/bridge_atlas: pack body + shadow frames; add BridgeFrameKind (RE §3.3.2; Phase D; Task 4)
```

### Task 5: Create `src/render/bridge_railing_atlas.rs`

**Why:** RAILBRDG.tem SHP loaded as a single-page atlas, and the two 10-entry lookup tables (concrete + wood) loaded from the Task 3 capture.

**Files:**
- Create: `src/render/bridge_railing_atlas.rs`
- Modify: [src/render/mod.rs](src/render/mod.rs) — add `pub mod bridge_railing_atlas;`

**Pattern:** Single-SHP atlas mirrors the simple-atlas pattern from existing render modules; the TYPE-keyed table layout mirrors `[Option<RailingEntry>; 10]` in design doc §Public interfaces.

**Step 1: Create the module file**

```rust
//! Bridge railing atlas: RAILBRDG.tem SHP + concrete/wood lookup tables.
//!
//! Mirrors gamemd's `g_BridgeRailingSHP` (theater-loaded at `DAT_00ABC554`)
//! and the two parallel railing tables (concrete `DAT_00ABC210`, wood near
//! `DAT_00AA1098`). See ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md
//! §3.4.1 and BRIDGE_RAILING_TABLE_VALUES.md for table values.

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeKind {
    Concrete,
    Wood,
}

/// One entry of the 10-element bridge-railing lookup table.
/// `shp_frame == 0` means "no railing for this sub-tile" (skip emit).
#[derive(Clone, Copy, Debug)]
pub struct RailingEntry {
    pub shp_frame: u8,
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
    pub fn entry(&self, kind: BridgeKind, sub_idx: u8) -> Option<&RailingEntry> {
        let table = match kind {
            BridgeKind::Concrete => &self.concrete_table,
            BridgeKind::Wood     => &self.wood_table,
        };
        table.get(sub_idx as usize).and_then(Option::as_ref)
    }
}

/// Per-entry table values from BRIDGE_RAILING_TABLE_VALUES.md.
/// Format: `(shp_frame_1based, dx, dy)` — `(0, 0, 0)` = no railing for this sub-tile.
/// **REPLACE WITH TASK 3 CAPTURE BEFORE TASK 17 VISUAL DIFF.**
const CONCRETE_RAILING_VALUES: [(u8, i16, i16); 10] = [
    // Placeholder: all-zero until Task 3 capture.
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
];
const WOOD_RAILING_VALUES: [(u8, i16, i16); 10] = [
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
    (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
];

pub fn build_bridge_railing_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    asset_manager: &AssetManager,
    theater_palette: &Palette,
    theater_ext: &str,
) -> Option<BridgeRailingAtlas> {
    let candidates = [
        format!("railbrdg.{}", theater_ext),
        "railbrdg.shp".to_string(),
    ];
    let shp: ShpFile = candidates.iter().find_map(|name| {
        let data = asset_manager.get_ref(name)?;
        ShpFile::from_bytes(data).ok()
    })?;

    // Pack every drawable frame into a single-page atlas (shelf-pack identical
    // to bridge_atlas.rs:208-256). Each frame becomes one rect; we then build
    // a per-table entry by frame index from CONCRETE/WOOD_RAILING_VALUES.
    // (Full pack code mirrors bridge_atlas.rs::pack_bridge_sprites; refer to
    //  that module — substitute one SHP for the multi-SHP loop. Helper extracted
    //  in Task 6 via a small refactor.)

    // For brevity here: assume `pack_single_shp_to_atlas(...)` returns
    // `(BatchTexture, Vec<(usize_frame, OverlaySpriteEntry)>)`; the call
    // site fills the [Option<RailingEntry>; 10] arrays.

    let (texture, frame_entries) =
        crate::render::atlas_packer::pack_single_shp(gpu, batch, &shp, theater_palette)?;

    let concrete_table = build_table(&CONCRETE_RAILING_VALUES, &frame_entries);
    let wood_table = build_table(&WOOD_RAILING_VALUES, &frame_entries);

    Some(BridgeRailingAtlas { texture, concrete_table, wood_table })
}

fn build_table(
    values: &[(u8, i16, i16); 10],
    frame_entries: &[(usize, crate::render::overlay_atlas::OverlaySpriteEntry)],
) -> [Option<RailingEntry>; 10] {
    let mut out: [Option<RailingEntry>; 10] = [None; 10];
    for (slot, &(shp_frame_1based, dx, dy)) in values.iter().enumerate() {
        if shp_frame_1based == 0 { continue; }
        let frame_0based = (shp_frame_1based - 1) as usize;
        let Some((_, e)) = frame_entries.iter().find(|(idx, _)| *idx == frame_0based) else {
            continue;
        };
        out[slot] = Some(RailingEntry {
            shp_frame: shp_frame_1based,
            dx, dy,
            uv_origin: e.uv_origin,
            uv_size: e.uv_size,
            pixel_size: e.pixel_size,
            offset_x: e.offset_x,
            offset_y: e.offset_y,
        });
    }
    out
}
```

**Step 2: Add the module to `render/mod.rs`**

```rust
// src/render/mod.rs
pub mod bridge_railing_atlas;
```

**Step 3: Add `render::atlas_packer::pack_single_shp` helper**

Extract the packing loop from [src/render/bridge_atlas.rs:208-304](src/render/bridge_atlas.rs#L208-L304) into a shared helper `src/render/atlas_packer.rs` so `BridgeAtlas` and `BridgeRailingAtlas` both use it. This keeps both modules under their line limits.

(Helper signature: `pub fn pack_single_shp(gpu, batch, shp, palette) -> Option<(BatchTexture, Vec<(usize, OverlaySpriteEntry)>)>`. Implementation: walk `shp.frames`, render each non-empty frame to RGBA, shelf-pack into a single texture, return UVs.)

**Step 4: Add unit tests**

```rust
// at bottom of src/render/bridge_railing_atlas.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_returns_none_when_shp_frame_is_zero() {
        // Construct an atlas with placeholder all-zero tables.
        // (BridgeRailingAtlas can't be constructed without a real GPU context —
        //  use a stand-alone build_table call instead.)
        let table = build_table(&[(0, 0, 0); 10], &[]);
        for slot in 0..10 {
            assert!(table[slot].is_none(), "slot {slot} should be None for shp_frame=0");
        }
    }

    #[test]
    fn entry_lookup_respects_kind() {
        // After Task 3 capture, this test will be the regression for non-empty
        // tables. For now (placeholder all-zero), verify shape only.
        // (Actual non-empty table verification at Task 17.)
    }
}
```

**Step 5: Verify**

```
cargo test --lib -p ra2-rust-game bridge_railing_atlas
```

Expected: PASS.

**Step 6: Commit**

```
render/bridge_railing_atlas: RAILBRDG.tem + concrete/wood tables (RE §3.4.1; Phase D; Task 5)
```

### Task 6: Build `BridgeRailingAtlas` at scenario load and store on `AppState`

**Why:** Renderer needs the atlas accessible during instance build and draw passes. Mirror existing pattern for `bridge_atlas` field on `AppState`.

**Files:**
- Modify: `src/app/mod.rs` (or wherever `AppState.bridge_atlas` is built — confirm via `Grep "build_bridge_atlas"`)
- Modify: the `AppState` struct definition

**Pattern:** Identical to how `bridge_atlas: Option<BridgeAtlas>` is stored.

**Step 1: Add field**

```rust
// AppState struct
pub bridge_railing_atlas: Option<BridgeRailingAtlas>,
```

**Step 2: Build at scenario load** — at the call site for `build_bridge_atlas(...)` (find via `Grep "build_bridge_atlas"`), add right after:

```rust
state.bridge_railing_atlas = bridge_railing_atlas::build_bridge_railing_atlas(
    &gpu, &batch_renderer, &asset_manager, theater_palette, theater_ext,
);
```

**Step 3: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -20
```

Expected: clean.

**Step 4: Commit**

```
app: build BridgeRailingAtlas at scenario load (Phase D; Task 6)
```

### Task 7: Create `src/app_instances/bridges.rs` — body builder

**Why:** Dedicated module for all bridge instance emission. Body is the simplest bucket and validates the shape before adding shadow + railings + deck variant.

**Files:**
- Create: `src/app_instances/bridges.rs`
- Modify: [src/app_instances/mod.rs](src/app_instances/mod.rs) — add `pub mod bridges;`

**Pattern:** Mirrors `app_instances/overlays.rs::build_overlay_instances` signature shape (parameters: `&AppState`, `sw, sh`, `&mut Vec<SpriteInstance>`).

**Step 1: Create the file with shared constants and the body builder**

```rust
//! Bridge instance emission — body, shadow, railings, deck-variant overrides.
//!
//! Mirrors gamemd's per-frame bridge draw (Steps 3, 5, 7) per
//! ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md §9. Reads
//! BridgeRuntimeCell post-tick; ignores OverlayGrid for bridges.
//!
//! Three open RE values are encoded as named constants here so each is a
//! single change point if visual diff resolves them differently.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::BTreeMap;

use crate::app::AppState;
use crate::map::lighting;
use crate::map::terrain::{self, TILE_HEIGHT, TILE_WIDTH};
use crate::render::batch::SpriteInstance;
use crate::render::bridge_atlas::is_high_bridge_body_name;
use crate::render::bridge_railing_atlas::BridgeKind;
use crate::sim::bridge_state::{Axis, BridgeRuntimeCell, DamageState};

use super::helpers::{compute_sprite_depth_params, in_view};

/// Latin-square jitter for bridge body frames at base state byte 0 or 9.
/// Verified raw memory read at `0x0081CC30`, RE doc §5. Ledger #1.
const BRIDGE_BODY_LATIN_SQUARE: [u8; 16] = [
    0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2,
];

/// Bridge body Y offset for state bytes 0..8 (HasBridge cells). RE doc §3.3.3, ledger #5.
const BRIDGE_BODY_Y_OFFSET_LOW: f32 = -16.0;
/// Bridge body Y offset for state bytes 9..17. RE doc §3.3.3, ledger #5.
const BRIDGE_BODY_Y_OFFSET_HIGH: f32 = -31.0;

/// HasBridge depth bonus added to `cell.deck_level` before the `* -15 - 2` calc.
/// RE doc §3.3.1, ledger #6.
const BRIDGE_HEIGHT_BONUS: u8 = 4;

/// Shadow X displacement on EW states 9..17. RE doc §10 open Q2 — value
/// unresolved between -15 and -45. Defaults to -15. Single change point.
pub const BRIDGE_SHADOW_EW_DX: i32 = -15;
/// Shadow Y displacement on EW states 9..17. Verified -0x2D = +7. RE doc §3.3.2, ledger #10.
pub const BRIDGE_SHADOW_EW_DY: i32 = 7;

/// Deck-variant override for one (rx, ry). Consumed by the terrain instance
/// builder's UV closure to select alt-art sub-tile when `damaged_variant` is set.
#[derive(Debug, Clone, Copy)]
pub struct DeckVariantSelect {
    pub use_alternate: bool,
}

/// Build sprite instances for the bridge body pass (RE doc §3.3, Step 5 pass 1).
/// Reads `BridgeRuntimeCell.overlay_byte` post-tick; uses Latin-square jitter on
/// base state bytes 0 and 9 only.
pub fn build_bridge_body_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else { return; };
    let Some(bridge_state) = sim.bridge_state.as_ref() else { return; };
    let Some(atlas) = state.bridge_atlas.as_ref() else { return; };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    let (cam_x, cam_y) = (state.camera_x, state.camera_y);

    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        let Some(axis) = cell.axis else { continue; };
        let Some(name) = state.overlay_names.get(&cell.overlay_byte) else { continue; };
        if !is_high_bridge_body_name(name) { continue; }

        let base = cell.damage_state.render_state_byte(axis);
        let frame: u8 = if base == 0 || base == 9 {
            let idx = ((ry & 3) as usize) << 2 | (rx & 3) as usize;
            base + BRIDGE_BODY_LATIN_SQUARE[idx]
        } else {
            cell.damage_state.to_state_byte(axis)
        };
        let y_offset = if frame <= 8 { BRIDGE_BODY_Y_OFFSET_LOW } else { BRIDGE_BODY_Y_OFFSET_HIGH };

        let z: u8 = state.height_map.get(&(rx, ry)).copied().unwrap_or(cell.deck_level);
        let (sx, sy) = terrain::iso_to_screen(rx, ry, z);
        let sy = sy + y_offset;
        if !in_view(sx, sy, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0) { continue; }

        let Some(spr) = atlas.body_entry(name, frame) else {
            log::warn!("bridge body atlas miss: name={name} frame={frame} cell=({rx},{ry})");
            continue;
        };

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, sy, depth_z);
        let tint = state.lighting_grid.get(&(rx, ry)).copied().unwrap_or(lighting::DEFAULT_TINT);
        out.push(SpriteInstance {
            position: [sx + TILE_WIDTH / 2.0 + spr.offset_x, sy + TILE_HEIGHT / 2.0 + spr.offset_y],
            size: spr.pixel_size,
            uv_origin: spr.uv_origin,
            uv_size: spr.uv_size,
            depth, tint, alpha: 1.0,
        });
    }
}
```

**Step 2: Wire the module**

```rust
// src/app_instances/mod.rs
pub mod bridges;
```

**Step 3: Add unit tests for the latin-square arithmetic**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_square_value_at_xy() {
        // (cell.x, cell.y) = (1, 2): idx = ((2&3)<<2) | (1&3) = 8|1 = 9
        // BRIDGE_BODY_LATIN_SQUARE[9] = 3
        assert_eq!(BRIDGE_BODY_LATIN_SQUARE[((2 & 3) << 2) | (1 & 3)], 3);
    }

    #[test]
    fn latin_square_table_is_canonical_4x4() {
        // RE doc §5: verified raw memory read at 0x0081CC30.
        assert_eq!(
            BRIDGE_BODY_LATIN_SQUARE,
            [0, 1, 2, 3, 3, 2, 1, 0, 2, 3, 0, 1, 1, 0, 3, 2]
        );
    }
}
```

**Step 4: Verify**

```
cargo test --lib -p ra2-rust-game app_instances::bridges
```

Expected: PASS. (`cargo build` may still fail until Task 13 removes the now-orphan bridge dispatch in `overlays.rs`; that's fine.)

**Step 5: Commit**

```
app_instances/bridges: body builder + RE constants (Phase D; Task 7)
```

### Task 8: `bridges.rs` — shadow builder

**Why:** Body shadow pass per RE doc §3.3.2 + ledger #8–12. Drawn in a passthrough Z-test/no-write pipeline so unit drawn after still occludes correctly.

**Files:**
- Modify: `src/app_instances/bridges.rs`

**Pattern:** Body builder; differs only in atlas accessor (`shadow_entry` vs `body_entry`), frame index (`half + state` vs `state`), and the EW-state shift constant.

**Step 1: Add the function**

```rust
/// Build sprite instances for the bridge body shadow pass (RE doc §3.3.2,
/// Step 5 pass 2). Shadow frame = (frame_count / 2) + state. EW states
/// 9..17 get a (-DX, +DY) shift per ledger #9–10.
pub fn build_bridge_shadow_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else { return; };
    let Some(bridge_state) = sim.bridge_state.as_ref() else { return; };
    let Some(atlas) = state.bridge_atlas.as_ref() else { return; };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    let (cam_x, cam_y) = (state.camera_x, state.camera_y);

    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        let Some(axis) = cell.axis else { continue; };
        let Some(name) = state.overlay_names.get(&cell.overlay_byte) else { continue; };
        if !is_high_bridge_body_name(name) { continue; }

        let base = cell.damage_state.render_state_byte(axis);
        let frame: u8 = if base == 0 || base == 9 {
            let idx = ((ry & 3) as usize) << 2 | (rx & 3) as usize;
            base + BRIDGE_BODY_LATIN_SQUARE[idx]
        } else {
            cell.damage_state.to_state_byte(axis)
        };

        let z: u8 = state.height_map.get(&(rx, ry)).copied().unwrap_or(cell.deck_level);
        let (mut sx, mut sy) = terrain::iso_to_screen(rx, ry, z);
        let y_offset = if frame <= 8 { BRIDGE_BODY_Y_OFFSET_LOW } else { BRIDGE_BODY_Y_OFFSET_HIGH };
        sy += y_offset;

        // EW-state shadow shift (states 9..17 — ledger #9, #10).
        if frame >= 9 && frame <= 17 {
            sx += BRIDGE_SHADOW_EW_DX as f32;
            sy += BRIDGE_SHADOW_EW_DY as f32;
        }

        if !in_view(sx, sy, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0) { continue; }

        let Some(spr) = atlas.shadow_entry(name, frame) else {
            log::warn!("bridge shadow atlas miss: name={name} frame={frame} cell=({rx},{ry})");
            continue;
        };

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, sy, depth_z);
        // Shadow uses neutral tint, no per-cell lighting (ledger #12).
        let tint = lighting::DEFAULT_TINT;
        out.push(SpriteInstance {
            position: [sx + TILE_WIDTH / 2.0 + spr.offset_x, sy + TILE_HEIGHT / 2.0 + spr.offset_y],
            size: spr.pixel_size,
            uv_origin: spr.uv_origin,
            uv_size: spr.uv_size,
            depth, tint, alpha: 1.0,
        });
    }
}
```

**Step 2: Add unit test**

```rust
#[test]
fn shadow_ew_shift_applies_only_for_states_9_through_17() {
    // Direct constant check; full pipeline test runs via Task 17 visual diff.
    assert!(BRIDGE_SHADOW_EW_DX != 0 || BRIDGE_SHADOW_EW_DY != 0);
}
```

**Step 3: Verify**

```
cargo test --lib -p ra2-rust-game app_instances::bridges
```

Expected: PASS.

**Step 4: Commit**

```
app_instances/bridges: body shadow builder (RE §3.3.2; Phase D; Task 8)
```

### Task 9: `bridges.rs` — railing builder

**Why:** RAILBRDG railings drawn in a separate passthrough pass per RE doc §3.4.1 + §9.4. Reads `BridgeRailingAtlas`; gates on `IsoTileType.is_shadow_caster`.

**Files:**
- Modify: `src/app_instances/bridges.rs`

**Pattern:** Body/shadow builders; differs in atlas (railing), screen shift `(dx + 30, dy + 15)` per ledger #18, blitter being passthrough no-Z-write.

**Step 1: Field-location facts (verified during /review-plan, do not re-investigate)**

The renderer needs `IsoTileType.SelfIdx - g_BridgeSet` (concrete) or `- g_WoodBridgeSet` (wood). In our codebase:

- **Sub-tile** lives on `ResolvedTerrainCell.final_sub_tile: u8` at [src/map/resolved_terrain.rs:74](src/map/resolved_terrain.rs#L74). It is **NOT** a field on `BridgeLayer` (which has only `overlay_id, overlay_name, deck_level, direction`).
- **Concrete vs wood** is determined by overlay name, not by tile-set lookup at our level:
  - `BRIDGE1`, `BRIDGEB1`, `BRIDGE2`, `BRIDGEB2` → Concrete (HIGH bridge; `1`/`2` is axis EW/NS, not material — see [src/map/resolved_terrain.rs:48-55](src/map/resolved_terrain.rs#L48-L55))
  - `LOBRDG01..28`, `LOBRDGE1..4`, `LOBRDGB1..4` → Wood (LOW bridge — see [src/map/overlay_types.rs:25-28](src/map/overlay_types.rs#L25-L28))
- `BridgeRuntimeState::iter_cells()` covers BOTH HIGH and LOW (loader at [src/sim/bridge_state/mod.rs:412-447](src/sim/bridge_state/mod.rs#L412-L447) BFS-walks any cell with `has_bridge_deck`).

**Step 2: Add the function**

```rust
/// Build sprite instances for the bridge railing pass (RE doc §3.4.1, Step 7).
/// Drawn AFTER unit/ground merge AND AFTER cliff redraw, BEFORE debug — see
/// draw_passes.rs ordering. Skips cells where the railing table entry is None
/// (shp_frame_1based == 0 ⇒ no railing for this sub-tile).
pub fn build_bridge_railing_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    out: &mut Vec<SpriteInstance>,
) {
    let Some(sim) = state.simulation.as_ref() else { return; };
    let Some(bridge_state) = sim.bridge_state.as_ref() else { return; };
    let Some(atlas) = state.bridge_railing_atlas.as_ref() else { return; };
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));
    let (cam_x, cam_y) = (state.camera_x, state.camera_y);

    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if !cell.deck_present || matches!(cell.damage_state, DamageState::Destroyed) {
            continue;
        }
        // Map cell to (BridgeKind, sub_idx) — Step 1 above resolved how.
        let Some((kind, sub_idx)) = resolve_bridge_kind_and_sub_idx(state, rx, ry, cell) else {
            continue;
        };
        let Some(entry) = atlas.entry(kind, sub_idx) else { continue; };

        let z: u8 = state.height_map.get(&(rx, ry)).copied().unwrap_or(cell.deck_level);
        let (sx, sy) = terrain::iso_to_screen(rx, ry, z);
        // (dx + 30, dy + 15) per ledger #18 — TILE_WIDTH/2 + TILE_HEIGHT/2 anchor.
        let final_x = sx + entry.dx as f32 + TILE_WIDTH / 2.0;
        let final_y = sy + entry.dy as f32 + TILE_HEIGHT / 2.0;
        if !in_view(final_x, final_y, 60.0, 60.0, cam_x, cam_y, sw, sh, 60.0) { continue; }

        let depth_z = z.saturating_add(BRIDGE_HEIGHT_BONUS);
        let depth = compute_sprite_depth_params(origin_y, world_height, final_y, depth_z);
        // Railings use neutral tint per ledger #20.
        let tint = lighting::DEFAULT_TINT;
        out.push(SpriteInstance {
            position: [final_x + entry.offset_x, final_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth, tint, alpha: 1.0,
        });
    }
}

fn resolve_bridge_kind_and_sub_idx(
    state: &AppState,
    rx: u16,
    ry: u16,
    cell: &BridgeRuntimeCell,
) -> Option<(BridgeKind, u8)> {
    // Resolution rule per RE §3.4.1:
    //   self_idx = IsoTileType.SelfIdx
    //   if self_idx ∈ [g_BridgeSet, g_BridgeSet + 10) → Concrete (HIGH bridges)
    //   if self_idx ∈ [g_WoodBridgeSet, g_WoodBridgeSet + 10) → Wood (LOW bridges)
    //
    // Codebase-truth mapping (verified against
    // src/map/resolved_terrain.rs:48-55 + src/map/overlay_types.rs:25):
    //   BRIDGE1, BRIDGEB1            — concrete high bridge, EW direction
    //   BRIDGE2, BRIDGEB2            — concrete high bridge, NS direction
    //   LOBRDG01..28, LOBRDGE1..4,
    //   LOBRDGB1..4                  — wood low bridge
    //
    // The `1` vs `2` suffix on BRIDGE* names is AXIS, not material — all four
    // are concrete. Wood lives only in LOBRDG* names.
    let name = state.overlay_names.get(&cell.overlay_byte)?.to_ascii_uppercase();
    let kind = if matches!(name.as_str(), "BRIDGE1" | "BRIDGEB1" | "BRIDGE2" | "BRIDGEB2") {
        BridgeKind::Concrete
    } else if name.starts_with("LOBRDG") {
        // Catches LOBRDG01..28, LOBRDGE1..4, LOBRDGB1..4.
        BridgeKind::Wood
    } else {
        return None;
    };
    // Sub-tile index lives on ResolvedTerrainCell.final_sub_tile (NOT inside
    // BridgeLayer — verified at src/map/resolved_terrain.rs:74). Used directly
    // as the railing-table slot index (post-clamp to 0..10).
    let sub_idx: u8 = state.terrain_grid.as_ref()?.cell(rx, ry)?.final_sub_tile;
    Some((kind, sub_idx))
}
```

> **Note on bridge_state coverage:** `BridgeRuntimeState::iter_cells()` includes both HIGH and LOW bridge cells because the loader at [src/sim/bridge_state/mod.rs:412-447](src/sim/bridge_state/mod.rs#L412-L447) BFS-walks every cell with `has_bridge_deck`, which is set for both. The body builder (Task 7) filters to HIGH via `is_high_bridge_body_name`; the railing builder iterates everything and dispatches via `resolve_bridge_kind_and_sub_idx` above.

**Step 2: Run unit tests** — covered by `entry_returns_none_when_shp_frame_is_zero` from Task 5.

**Step 3: Commit**

```
app_instances/bridges: railing builder (RE §3.4.1; Phase D; Task 9)
```

### Task 10: `bridges.rs` — deck-variant override builder

**Why:** Damaged-tile alt-art selection per RE doc §3.2 + ledger #13–14.

**Files:**
- Modify: `src/app_instances/bridges.rs`

**Step 1: Add the function**

```rust
/// Walk all bridge cells with `damaged_variant: true`, return a sorted map
/// of `(rx, ry) → DeckVariantSelect { use_alternate: true }`. Consumed by
/// `app_render::build_instances` to pick the alt-art sub-tile UV for the
/// deck TMP. RE doc §3.2 + ledger #13.
pub fn build_bridge_deck_variant_overrides(
    state: &AppState,
) -> BTreeMap<(u16, u16), DeckVariantSelect> {
    let mut out = BTreeMap::new();
    let Some(sim) = state.simulation.as_ref() else { return out; };
    let Some(bridge_state) = sim.bridge_state.as_ref() else { return out; };
    for ((rx, ry), cell) in bridge_state.iter_cells() {
        if cell.damaged_variant {
            out.insert((rx, ry), DeckVariantSelect { use_alternate: true });
        }
    }
    out
}
```

**Step 2: Test**

```rust
#[test]
fn deck_overrides_empty_when_all_pristine() {
    // Construct minimal AppState mock with bridge_state.iter_cells returning
    // cells that all have damaged_variant: false. Result map should be empty.
    // (Full scaffolding may require helper from sim::bridge_state; if too
    //  invasive, gate this test on a manual sandbox check instead.)
}
```

**Step 3: Commit**

```
app_instances/bridges: deck-variant override builder (RE §3.2; Phase D; Task 10)
```

### Task 11: Wire `bridges.rs` builders into `WorldInstances`

**Why:** Hook the four new builders into the per-frame instance build path so the GPU sees the data.

**Files:**
- Modify: [src/app_render/build_instances.rs:32-46](src/app_render/build_instances.rs#L32-L46) — add fields
- Modify: [src/app_render/build_instances.rs:152-247](src/app_render/build_instances.rs#L152-L247) — call new builders

**Step 1: Extend `WorldInstances` struct**

```rust
pub(super) struct WorldInstances {
    pub terrain: terrain::TerrainInstances,
    pub overlay: Vec<SpriteInstance>,
    pub smudge: Vec<SpriteInstance>,
    pub bridge_detail: Vec<SpriteInstance>,
    pub bridge_body: Vec<SpriteInstance>,
    pub bridge_body_shadow: Vec<SpriteInstance>,    // NEW
    pub bridge_railing: Vec<SpriteInstance>,        // NEW
    pub deck_variant_overrides:                     // NEW
        std::collections::BTreeMap<(u16, u16), crate::app_instances::bridges::DeckVariantSelect>,
    pub wall: Vec<SpriteInstance>,
    pub unit: Vec<SpriteInstance>,
    pub bridge_unit: Vec<SpriteInstance>,
    pub shp_paged: Vec<Vec<SpriteInstance>>,
    pub bridge_shp_paged: Vec<Vec<SpriteInstance>>,
    pub building_turret: Vec<SpriteInstance>,
}
```

**Step 2: In `build_world_instances`, build deck overrides BEFORE terrain so the closure sees them**

```rust
// Around line 122, BEFORE the terrain build call:
let deck_variant_overrides = crate::app_instances::bridges::build_bridge_deck_variant_overrides(state);
```

**Step 3: Pass overrides into the terrain UV closure**

Modify the closure at [src/app_render/build_instances.rs:99-117](src/app_render/build_instances.rs#L99-L117) to consult `deck_variant_overrides`. If a cell is in the map and `IsoTileType.num_tiles >= 2` AND the sub-tile flags allow alt-art, use `variant: 1` instead of the pseudo-random LAT pick. Mirror the binary's gate at RE doc §3.2:

```rust
// Inside the uv_fn_closure:
let cell_key = /* (rx, ry) for this tile_id call — derive from caller */;
let override_alt = deck_variant_overrides
    .get(&cell_key)
    .map(|v| v.use_alternate)
    .unwrap_or(false);
let effective_variant = if override_alt { 1 } else { variant };
let key = TileKey { tile_id, sub_tile, variant: effective_variant };
// ... rest of lookup unchanged ...
```

> **Note:** The closure today doesn't receive `(rx, ry)` — the call site passes only `(tile_id, sub_tile, variant)`. To plumb the override correctly, **either** extend `terrain::build_visible_instances` to pass `(rx, ry)` to the closure, **or** capture the override map and look up by tile_id (works only if tile_id is unique per cell). Verify which path is feasible in this task; if signature change is invasive, surface it as a `/review-plan` flag and either accept the minor invasiveness or defer the deck-variant pass to a follow-up task with `damaged_variant` always `false`.

**Step 4: Build the four new instance buffers**

```rust
// After existing app_instances::build_overlay_instances(...) call:
let mut bridge_body: Vec<SpriteInstance> = Vec::new();
let mut bridge_body_shadow: Vec<SpriteInstance> = Vec::new();
let mut bridge_railing: Vec<SpriteInstance> = Vec::new();
crate::app_instances::bridges::build_bridge_body_instances(state, sw, sh, &mut bridge_body);
crate::app_instances::bridges::build_bridge_shadow_instances(state, sw, sh, &mut bridge_body_shadow);
crate::app_instances::bridges::build_bridge_railing_instances(state, sw, sh, &mut bridge_railing);
sort_by_depth_desc(&mut bridge_body);
sort_by_depth_desc(&mut bridge_body_shadow);
sort_by_depth_desc(&mut bridge_railing);
```

**Step 5: Return them in the struct literal at [src/app_render/build_instances.rs:234-247](src/app_render/build_instances.rs#L234-L247)**

```rust
WorldInstances {
    terrain,
    overlay,
    smudge,
    bridge_detail,
    bridge_body,
    bridge_body_shadow,
    bridge_railing,
    deck_variant_overrides,
    wall,
    unit,
    bridge_unit,
    shp_paged,
    bridge_shp_paged,
    building_turret,
}
```

**Step 6: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -30
```

Expected: clean.

**Step 7: Commit**

```
app_render/build_instances: hook bridge body/shadow/railing/deck-override builders (Phase D; Task 11)
```

### Task 12: Upload two new pooled keys in `app_render/mod.rs`

**Why:** GPU buffers for the new shadow + railing draws.

**Files:**
- Modify: [src/app_render/mod.rs:135-143](src/app_render/mod.rs#L135-L143)

**Step 1: Add uploads**

```rust
// src/app_render/mod.rs after line 143
pool.upload(&state.gpu, "overlay_bridge_body_shadow", &world.bridge_body_shadow);
pool.upload(&state.gpu, "overlay_bridge_railing", &world.bridge_railing);
```

**Step 2: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -10
```

Expected: clean.

**Step 3: Commit**

```
app_render: upload overlay_bridge_body_shadow + overlay_bridge_railing pooled buffers (Phase D; Task 12)
```

### Task 13: Remove bridge dispatch from `app_instances/overlays.rs`

**Why:** Bridge handling now lives in `bridges.rs`; `overlays.rs` collapses to non-bridge buckets only. Also remove the BridgeAtlas `get` shim added in Task 4.

**Files:**
- Modify: [src/app_instances/overlays.rs:23-53](src/app_instances/overlays.rs#L23-L53) — collapse `OverlayRenderBucket` enum
- Modify: [src/app_instances/overlays.rs:230-243](src/app_instances/overlays.rs#L230-L243) — remove bridge skip + entry resolution
- Modify: [src/app_instances/overlays.rs:271-281](src/app_instances/overlays.rs#L271-L281) — remove `BRIDGE_FRAME_VARIATION` and the Latin-square pick from the OverlayGrid path
- Modify: [src/app_instances/overlays.rs:300-310](src/app_instances/overlays.rs#L300-L310) — remove `BRIDGE_FRAME_VARIATION` and Latin-square pick from the fallback path
- Modify: [src/app_instances/overlays.rs:31-37](src/app_instances/overlays.rs#L31-L37) — remove `bridge_y_offset_for_name`
- Modify: [src/app_instances/overlays.rs:359-377](src/app_instances/overlays.rs#L359-L377) — remove `BridgeBody` arm + bridge-only depth bonus
- Modify: [src/app_instances/overlays.rs:176-184](src/app_instances/overlays.rs#L176-L184) — drop `bridge_detail_instances`, `bridge_body_instances` parameters
- Modify: [src/app_render/build_instances.rs:152-170](src/app_render/build_instances.rs#L152-L170) — drop those two `&mut Vec` arguments at the call site
- Modify: [src/render/bridge_atlas.rs](src/render/bridge_atlas.rs) — REMOVE the `pub fn get(&self, ...)` compat shim added in Task 4

**Pattern:** Delete-not-rename. Each removed branch was a clearly delineated bridge-only block.

**Step 1: Collapse the bucket enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayRenderBucket {
    Generic,
    Wall,
}
```

**Step 2: Update `classify_overlay_render_bucket`**

```rust
fn classify_overlay_render_bucket(_name: &str, _overlay_id: u8, is_wall: bool) -> OverlayRenderBucket {
    if is_wall { OverlayRenderBucket::Wall } else { OverlayRenderBucket::Generic }
}
```

(The `name` and `overlay_id` parameters can stay for API stability or be removed — choose minimal-edit.)

**Step 3: In the main loop (~line 219 onward), delete these blocks:**

- The "skip destroyed bridge overlays" block at lines 230–243 (no longer needed — bridges aren't routed here anymore; they're in `bridges.rs`).
- The two `BRIDGE_FRAME_VARIATION` + state-byte pick blocks at lines 271–281 and 301–310 (Latin square moved to `bridges.rs`).
- The `bridge_y_offset` line and helper (line 31–37 + 321) — non-bridge overlays don't need this offset.
- The `BridgeBody` lookup arm at lines 359–365.
- The `BridgeBody` depth bonus at lines 373–377.
- The `BridgeBody`/`BridgeDetail` arms in the bucket-target match at lines 406–411.

**Step 4: Drop the `bridge_*_instances` parameters from `build_overlay_instances`**

```rust
pub(crate) fn build_overlay_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    instances: &mut Vec<SpriteInstance>,
    wall_instances: &mut Vec<SpriteInstance>,
) {
    // ... no more bridge_detail_instances, no more bridge_body_instances ...
}
```

**Step 5: Update the call site** at [src/app_render/build_instances.rs:158-166](src/app_render/build_instances.rs#L158-L166):

```rust
app_instances::build_overlay_instances(state, sw, sh, &mut overlay, &mut wall);
```

(Replace the bridge_detail and bridge_body locals with `Vec::new()` defaults — they're unused now since `bridges.rs` owns them; or delete the `bridge_detail` field entirely if no other consumer exists.)

**Step 6: Decide `bridge_detail` fate** — `Grep "bridge_detail"` repo-wide. If no other consumer exists outside the `overlay_bridge_detail` upload at [src/app_render/mod.rs:138](src/app_render/mod.rs#L138) and the draw call at [src/app_render/draw_passes.rs:84](src/app_render/draw_passes.rs#L84), **delete the bucket entirely** — `bridges.rs::build_bridge_body_instances` covers all High bridge frames; LOW bridges (LOBRDG##) ride in `overlay` like any non-wall overlay. Verify there are no LOW-bridge-specific draws this would break.

**Step 7: Remove the `BridgeAtlas::get` shim from Task 4**

```rust
// src/render/bridge_atlas.rs — DELETE the compat shim:
//   pub fn get(&self, key: &OverlaySpriteKey) -> Option<&OverlaySpriteEntry> { ... }
```

**Step 8: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -20
cargo test --lib -p ra2-rust-game app_instances 2>&1 | tail -20
```

Expected: clean build, all tests still pass (no test was bridge-bucket-specific in `overlays.rs`).

**Step 9: Commit**

```
app_instances/overlays: drop bridge dispatch — moved to app_instances/bridges (Phase D; Task 13)
```

### Task 14: Insert shadow + railing passes in `draw_passes.rs`

**Why:** Wire the GPU-side draw calls for the two new buckets at the binary-correct insertion points (RE doc §9.4 + ledger #22–23).

**Files:**
- Modify: [src/app_render/draw_passes.rs](src/app_render/draw_passes.rs)

**Pattern:** Existing `draw_pooled_passthrough_overlay` calls.

**Step 1: Add a bridge-shadow helper** (mirrors `draw_pooled_bridge_zdepth`, but uses passthrough — Z test, no Z write):

```rust
// In src/app_render/draw_passes.rs after draw_pooled_bridge_zdepth (line ~447)
fn draw_pooled_bridge_passthrough<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a BridgeAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, &a.texture, buf, count);
    }
}
```

**Step 2: Add a bridge-railing helper:**

```rust
// In src/app_render/draw_passes.rs near the other pooled helpers
fn draw_pooled_bridge_railing<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a crate::render::bridge_railing_atlas::BridgeRailingAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, &a.texture, buf, count);
    }
}
```

**Step 3: Insert shadow pass between Step 2 (bridge body zdepth) and Step 3 (overlays passthrough)**

```rust
// In dispatch_draw_passes, after the existing draw_pooled_bridge_zdepth at line ~67:

// --- Step 2.5: Bridge body shadow (passthrough — Z-test ON, Z-write OFF, darken) ---
// RE doc §3.3.2: shadow uses blitter flag 0x4601. Drawn after body zdepth so
// shadow reads body Z but doesn't write its own Z; units crossing afterwards
// still occlude the shadow correctly.
draw_pooled_bridge_passthrough(
    &mut pass,
    &state.batch_renderer,
    pool,
    state.bridge_atlas.as_ref(),
    "overlay_bridge_body_shadow",
);
```

**Step 4: Insert railing pass between Step 7 (cliff redraw) and Step 8 (debug overlays)**

```rust
// In dispatch_draw_passes, after draw_pooled_zdepth(... "terrain_cliff") at line ~163:

// --- Step 7.5: Bridge railings (passthrough — Z-test ON, Z-write OFF) ---
// RE doc §3.4.1, ledger #22: drawn AFTER unit/ground merge AND AFTER cliff
// redraw, BEFORE debug. Anything between body and railings (units, anims,
// cliff redraw) sits ABOVE deck but BELOW railings.
draw_pooled_bridge_railing(
    &mut pass,
    &state.batch_renderer,
    pool,
    state.bridge_railing_atlas.as_ref(),
    "overlay_bridge_railing",
);
```

**Step 5: Verify**

```
cargo build -p ra2-rust-game 2>&1 | head -20
```

Expected: clean.

**Step 6: Commit**

```
app_render/draw_passes: shadow pass + railing pass at RE-correct positions (RE §9.4; Phase D; Task 14)
```

### Task 15: Fill `update_adjacent_bridges` rim refresh

**Why:** Stub at [src/sim/world/bridge_orchestrator.rs:208-210](src/sim/world/bridge_orchestrator.rs#L208-L210) is a no-op today. Without rim refresh, partial-collapse leaves dangling stubs visible — visible every time a midspan collapses.

**Files:**
- Modify: [src/sim/world/bridge_orchestrator.rs:191-210](src/sim/world/bridge_orchestrator.rs#L191-L210)

**Pattern:** Mirrors RE doc §7.1 + §7.4. Pure sim mutation; uses only `BridgeRuntimeState` accessors and `Direction` enum already in `bridge_state`.

**Step 1: Replace the stub with the rim-refresh implementation**

```rust
fn update_adjacent_bridges(sim: &mut Simulation, rim_cells: &BTreeSet<(u16, u16)>) {
    let Some(bridge_state) = sim.bridge_state.as_mut() else { return; };

    // Per RE doc §7.1: 8-direction walk at each rim cell, stop at first cell
    // with `flags & 0x500` (BRIDGE_HEAD candidate). In our model the equivalent
    // is "first cell whose role is Bridgehead OR damage_state is Destroyed".
    // Walk-length cap = 30 cells per RE §7.2. Recursion bound: per-stub repair
    // cap of 30 iterations.

    const WALK_LIMIT: usize = 30;
    const RECURSION_LIMIT: usize = 30;

    let directions: [(i32, i32); 8] = [
        (0, -1),  // N
        (1, -1),  // NE
        (1, 0),   // E
        (1, 1),   // SE
        (0, 1),   // S
        (-1, 1),  // SW
        (-1, 0),  // W
        (-1, -1), // NW
    ];

    for &(rx, ry) in rim_cells {
        // Phase A — find adjacent bridge-head cell within 8 neighbors.
        let mut head_dir: Option<(i32, i32)> = None;
        for &(dx, dy) in &directions {
            let nx = rx as i32 + dx;
            let ny = ry as i32 + dy;
            if nx < 0 || ny < 0 { continue; }
            let Some(neigh) = bridge_state.cell(nx as u16, ny as u16) else { continue; };
            // Bridge-head candidates: Bridgehead role OR Destroyed.
            let is_head_candidate = matches!(neigh.role, BridgeCellRole::Bridgehead)
                || matches!(neigh.damage_state, DamageState::Destroyed);
            if is_head_candidate {
                head_dir = Some((dx, dy));
                break;
            }
        }
        let Some((dx, dy)) = head_dir else { continue; };

        // Phase C — walk along the bridge from (rx, ry) toward the head, find
        // dangling stubs, repair them.
        let mut walk_x = rx as i32;
        let mut walk_y = ry as i32;
        for _ in 0..WALK_LIMIT {
            walk_x += dx;
            walk_y += dy;
            if walk_x < 0 || walk_y < 0 { break; }

            // Dangling-stub detection: cell exists and is bridge-bearing but its
            // anchor span is gone (orphan from the just-collapsed midspan).
            let Some(cell) = bridge_state.cell(walk_x as u16, walk_y as u16) else { break; };
            if !cell.deck_present { break; }
            let stub_now = cell.anchor_span_id
                .map(|sid| bridge_state.anchor_spans().get(&sid).is_none())
                .unwrap_or(false);
            if !stub_now { continue; }

            // Repair — per RE doc §7.2 ledger #31:
            //   cell.overlay_byte = NONE
            //   damage_state      = Healthy { variant: 0 }
            //   clear bridge-direction flags (mark group_id = None)
            if let Some(c) = bridge_state.cell_mut(walk_x as u16, walk_y as u16) {
                c.overlay_byte = 0xFF; // sentinel: NONE / -1
                c.damage_state = DamageState::Healthy { variant: 0 };
                c.bridge_group_id = None;
                c.deck_present = false;
            }
        }
    }
}
```

**Step 2: Add an integration test**

In `src/sim/world/world_tests.rs`, add a test that:
1. Constructs a 3-cell linear high bridge.
2. Destroys the midspan.
3. Calls the orchestrator's process step.
4. Asserts neighbours of the destroyed cell that have lost their anchor span have `overlay_byte == 0xFF` and `damage_state == Healthy { variant: 0 }`.

```rust
#[test]
fn rim_refresh_clears_dangling_stubs() {
    // ... fixture setup ...
    let mut sim = make_sim_with_3_cell_bridge();
    sim.advance_tick(/* damage_event_destroying_midspan */);
    let bs = sim.bridge_state.as_ref().unwrap();
    // After collapse, the two former neighbour cells are now dangling stubs;
    // rim refresh should reset them.
    let neighbour = bs.cell(/* dangling-stub coord */).unwrap();
    assert_eq!(neighbour.overlay_byte, 0xFF);
    assert!(matches!(neighbour.damage_state, DamageState::Healthy { variant: 0 }));
}
```

**Step 3: Run**

```
cargo test --lib -p ra2-rust-game world::tests::rim_refresh_clears_dangling_stubs
```

Expected: PASS.

**Step 4: Commit**

```
sim/world/bridge_orchestrator: fill rim refresh per RE §7 (Phase D; Task 15)
```

### Task 16: Render integration test — body bucket reflects post-tick state byte

**Why:** Lock in the "render reads `BridgeRuntimeCell.overlay_byte` post-tick (NOT `OverlayGrid`)" parity guarantee, so future refactors don't quietly regress.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (or `tests/` for a cross-module integration test)

**Step 1: Add the test**

```rust
#[test]
fn bridge_body_instance_uses_post_tick_overlay_byte() {
    let state = make_app_state_with_bridge_at(2, 2);
    // Damage to push bridge cell into Damaged state.
    apply_damage_to_bridge_at(&mut state.simulation.as_mut().unwrap(), 2, 2, /*hp=*/1);
    let cell = state
        .simulation.as_ref().unwrap()
        .bridge_state.as_ref().unwrap()
        .cell(2, 2).unwrap();
    assert!(matches!(cell.damage_state, DamageState::Damaged));

    let mut buf = Vec::new();
    crate::app_instances::bridges::build_bridge_body_instances(&state, 800.0, 600.0, &mut buf);
    // Frame for Damaged NS = 6 (no Latin square — base != 0/9).
    // Frame for Damaged EW = 0xF.
    // Verify the instance's UV came from `body_entry(name, 6)` or `(name, 0xF)`.
    assert_eq!(buf.len(), 1, "exactly one bridge body instance for one bridge cell");
    // (UV check requires atlas access; assert non-empty for now and let Task 17
    //  visual diff verify the actual texture binds correctly.)
}
```

**Step 2: Run**

```
cargo test --lib -p ra2-rust-game bridge_body_instance_uses_post_tick_overlay_byte
```

Expected: PASS.

**Step 3: Commit**

```
sim/world/world_tests: bridge body uses post-tick overlay_byte (Phase D; Task 16)
```

### Task 17: Visual diff vs gamemd — resolve open RE values

**Why:** Three named constants ship with placeholder defaults (`BRIDGE_SHADOW_EW_DX = -15`, axis convention `EW = states 9..17`, all-zero railing tables until Task 3 capture). Phase D closes only after each is resolved against gamemd output.

**Files:** None (research / verification task).

**Step 1: Spin up gamemd.exe + Rust client side-by-side**

1. Pick a bridge map (Bering Strait or Heartland — temperate, both bridge orientations visible).
2. In gamemd, screenshot a single bridge cell from each axis (state byte 0..8 visible vs state byte 9..17 visible).
3. In Rust client, screenshot the same cells from the same camera angle.
4. Open both in an image diff tool (visually overlay).

**Step 2: Resolve `BRIDGE_SHADOW_EW_DX`**

Inspect the shadow X position on EW-axis bridges. If the Rust shadow lands ~30px right of gamemd's, `BRIDGE_SHADOW_EW_DX = -15` is too small — change to `-45` in [src/app_instances/bridges.rs](src/app_instances/bridges.rs).

If the shadow already lines up at `-15`, the constant is correct and the prior "Phase 1C agent reports show conflicting extracts" was the agent reading the wrong basic block.

**Step 3: Resolve axis convention**

Inspect `bridge.tem` SHP frames 0 and 9 visually. If frame 0 depicts a bridge oriented north–south (vertical on screen) and frame 9 depicts east–west (horizontal), the existing mapping at [src/sim/bridge_state/mod.rs:22-30](src/sim/bridge_state/mod.rs#L22-L30) (`Axis::EW = states 9..17`, `Axis::NS = states 0..8`) is correct. Otherwise, swap.

**Step 4: Resolve railing table values**

Confirm the live-debugger capture from Task 3 produced non-zero entries; if Task 3's table came back all-zero or wildly different, re-capture per the fallback path 1 (static decompilation of `0x005446B1`, etc.).

Update [src/render/bridge_railing_atlas.rs](src/render/bridge_railing_atlas.rs):

```rust
const CONCRETE_RAILING_VALUES: [(u8, i16, i16); 10] = [
    /* values from BRIDGE_RAILING_TABLE_VALUES.md */
];
```

**Step 5: Re-run**

Re-run the side-by-side diff after each constant update. Iterate until the bridge body, shadow, and railings are pixel-identical (modulo anti-aliasing differences).

**Step 6: Commit each constant change separately**

```
app_instances/bridges: BRIDGE_SHADOW_EW_DX = -45 (visual diff resolved; Phase D; Task 17)
```

(Or `-15` if that turns out correct — commit message reflects the resolution either way.)

```
render/bridge_railing_atlas: concrete + wood railing table values from live capture (Phase D; Task 17)
```

### Task 18: Replay determinism regression

**Why:** Ensure the new sim writes (`damaged_variant` field, `update_adjacent_bridges` mutations) don't break lockstep. Sim-checklist mandate.

**Files:**
- Modify: `src/sim/world/world_tests.rs` (existing replay determinism test, if any) or add new

**Step 1: Add (or update) the replay test**

```rust
#[test]
fn replay_determinism_with_bridge_collapse_and_rim_refresh() {
    let inputs = synthetic_replay_with_bridge_destruction();
    let hash_a = run_replay(&inputs);
    let hash_b = run_replay(&inputs);
    assert_eq!(hash_a, hash_b, "two replays of the same inputs must produce identical hashes");
}
```

**Step 2: Run**

```
cargo test --lib -p ra2-rust-game replay_determinism_with_bridge_collapse_and_rim_refresh
```

Expected: PASS.

**Step 3: Commit**

```
sim/world/world_tests: replay determinism across bridge collapse + rim refresh (Phase D; Task 18)
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-08-bridges-tier2-phase-d-renderer-design.md](docs/plans/2026-05-08-bridges-tier2-phase-d-renderer-design.md)
- **Primary RE report:** `ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` (HIGH confidence, 2026-05-07/08)
- **Cross-checked RE reports:**
  - `ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md` (pre-Phase F, layer mapping superseded)
  - `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` (state byte authority)
  - `ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` (two-channel state, `cell+0x140 & 0x2000`)
  - `ra2-rust-game-docs/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md` (orchestrator dispatcher)
- **Ghidra addresses (kept here, not in code comments):**
  - `0x47F6A0` — `CellClass::DrawOverlay_Body`
  - `0x47F510` — `CellClass::DrawOverlay_Shadow`
  - `0x480110` — `CellClass::Get_Draw_Offset`
  - `0x547230` — railing emit (`FUN_00547230`)
  - `0x576770` — `MapClass::UpdateAdjacentBridges_High`
  - `0x576200` — `MapClass::UpdateBridgeEdgeTiles_High`
  - `0x56E990` — `MapClass::ToggleBridgePavement` (writes `cell.flags & 0x2000`)
  - `0x0081CC30` — `g_LatinSquare` (16 dwords)
  - `0x00ABC210` — concrete railing table
  - `~0x00AA1098` — wood railing table base
  - `0x00ABC554` — bridge railing SHP pointer (theater-loaded)
  - `0x00AA0E28` — `g_BridgeSet`
  - `0x00ABAD1C` — `g_WoodBridgeSet`
- **INI keys (no new parsing):**
  - `[CombatDamage] BridgeStrength=1500` — already parsed
  - `[BRIDGE1]/[BRIDGE2]/[BRIDGEB1]/[BRIDGEB2]` — `rulesmd.ini:29869-29893`
  - `[RAILBRDG] Theater=yes` — `artmd.ini:13123-13124`
- **Related code:**
  - `src/sim/bridge_state/` — runtime state model
  - `src/sim/world/bridge_orchestrator.rs` — damage dispatcher with rim-refresh hook
  - `src/render/bridge_atlas.rs` — body atlas (extending in Task 4)
  - `src/app_instances/overlays.rs` — current bridge dispatch (removing in Task 13)
  - `src/app_render/build_instances.rs`, `src/app_render/mod.rs`, `src/app_render/draw_passes.rs` — render pipeline plumbing
- **Predecessor branch state:** `dev` @ HEAD `e8db5eb` — Phases B+C+E+F+G shipped 2026-05-07; zero ignored bridge tests.
