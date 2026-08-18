# Smudge Atlas Registration Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Register smudge SHPs into the existing OverlayAtlas at map-load and replace the placeholder lookup closure in `build_smudge_instances` with a real atlas query, making craters and scorches visible end-to-end.

**Architecture:** Smudge SHPs join the OverlayAtlas (single shelf-packed texture, single GPU bind, single map-load build). Atlas keys are namespaced under `__smudge::` to eliminate any name collision with overlays. Render layer skips non-origin footprint cells so each multi-cell smudge draws once at the footprint origin (matching gamemd's net pixel result).

**Design Doc:** [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md) — Out-of-scope item §"Render path for the smudge SHP atlas". This plan implements that follow-up.

---

## Grounding Summary

**Docs:**
- `ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md` — §7 "Rendering — Pointer Only" said full render trace was deferred. This plan completes that trace.
- `ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md` — spawn-side covered by prior plan.

**Ghidra-verified (this session):**
- `Tactical_layer_smudges @ 0x006D3290` — called from `TacticalClass::Draw` between `Tactical_layer_base_terrain` and `Tactical_layer_overlays`. Confirms render-pass placement: smudges draw after terrain, before overlays/animations.
- `CellOverlay_TileDraw @ 0x00480350` — per-cell render. After blitting the iso tile, if `Cell+0x48 != -1` (smudge index), it invokes `SmudgeTypeClass` vtable+0xa0 (the smudge draw method).
- `SmudgeTypeClass::Draw_It @ 0x006B55F0` (newly created in Ghidra) — calls `CC_Draw_Shape(shp, frame=0, &screen_pos, viewport, 0xe00, 0, depth, 0, light, 0,0,0,0,0)`. **No explicit palette argument — palette is implicit global render state (iso theater palette during the world pass).**
- **Frame index passed to CC_Draw_Shape is 0 always.** The `frame_offset` shifts the screen position back to the footprint origin via `iStack_4 += (y+x)*-15; iStack_8 += (y-x)*30;` — verified by working iso math for all 4 cells of a 2×2 footprint at origin (5,5): every cell's draw position cancels back to origin (0, 150).
- **Implication:** multi-cell smudges register exactly **one** atlas entry per SmudgeType (frame=0). Render skips non-origin footprint cells (`frame_offset != 0`) — visually identical to gamemd's redundant per-cell draws stacking on the same pixels.

**Repo pattern this mirrors:**
- [src/render/overlay_atlas.rs:100-302](src/render/overlay_atlas.rs#L100-L302) — `build_overlay_atlas` shape: collect needed `(name, frame)` keys → render each to RGBA → shelf-pack → produce GPU texture + HashMap lookup.
- [src/render/overlay_atlas.rs:308-475](src/render/overlay_atlas.rs#L308-L475) — `render_overlay_sprite` shape: pick palette + candidate filenames → load SHP → render frame to RGBA → blit into full SHP bounds → return `RenderedOverlay`.
- [src/app_skirmish.rs:269-487](src/app_skirmish.rs#L269-L487) — `build_overlay_atlas_from_map` orchestration: assembles palettes, calls `overlay_atlas::build_overlay_atlas(...)`.
- [src/app_render/draw_passes.rs:101-107](src/app_render/draw_passes.rs#L101-L107) — smudge draw pass ALREADY binds `state.overlay_atlas` and uses `draw_pooled_passthrough_overlay`. No GPU-pipeline wiring changes needed.

**INI keys driving behavior:**
- `rulesmd.ini` `[SmudgeTypes]` numeric list (lines 1683-1727) → 46 SmudgeTypes (CR1..CR6, BURN01..BURN16, BURNT01..BURNT12, CRATER01..CRATER12).
- `rulesmd.ini` per-name sections — `Crater=`, `Burn=`, `Width=`, `Height=`, `Image=`. Width/Height default 1.
- `artmd.ini` per-name sections — `Theater=yes` lives here for CRATER01-12 and BURNT01-12 (NOT in rulesmd.ini). The `SmudgeTypeDef.is_theater` field on the existing parser is read from rulesmd and is therefore always `false` — vestigial, ignored by this plan, not removed (separate cleanup).

**Unknowns after grounding:**
- None blocking. Multi-cell SmudgeType SHP frame count is asserted at runtime (warn-log if a `Width>1 || Height>1` SmudgeType's SHP has more than 1 frame, to catch unexpected mod assets). Doesn't gate the plan.

## Key Technical Decisions

- **Approach (a) — pack into existing OverlayAtlas with `__smudge::` namespace prefix.** **Confidence:** high. **Source:** brainstorm session 2026-05-07; OverlayAtlas is non-dynamic at runtime (built once at map-load, never rebuilt) so the "lifecycle coupling" concern of (b) is theoretical. Reuses shelf packer, GPU upload, draw-pipeline binding. ~50 LOC delta.
- **Atlas registration timing: at map-load, inside `build_overlay_atlas_from_map`.** **Confidence:** high. **Source:** theater extension is map-specific; OverlayAtlas already builds there; no value in delaying.
- **Iso theater palette for smudges.** **Confidence:** high. **Source:** Ghidra `SmudgeTypeClass::Draw_It @ 0x006B55F0` — `CC_Draw_Shape` takes no explicit palette arg; palette is implicit global state set up by the world pass; smudges sit in that pass between terrain and overlays.
- **One atlas entry per SmudgeType, frame=0.** **Confidence:** high. **Source:** Ghidra — frame index passed to CC_Draw_Shape is always 0; `frame_offset` is a screen-position shift that cancels back to footprint origin. Verified by iso math for 2×2 footprint.
- **Render-side: skip non-origin footprint cells (`frame_offset != 0`).** **Confidence:** high. **Source:** identical pixels to gamemd's redundant per-cell stacking (every cell would draw the same SHP at the same position with the same depth and palette); skipping non-origin cells is a strict optimization. Origin cell of any placed footprint always has `frame_offset == 0` per [src/sim/smudge_grid.rs:195](src/sim/smudge_grid.rs#L195) (`(dx as u8) + (dy as u8) * w` is 0 when dx=dy=0).
- **Anchor offset: `offset_x = -full_w/2`, `offset_y = -full_h/2`.** **Confidence:** high. **Source:** mirrors OverlayAtlas's non-tiberium centered overlay anchor at [src/render/overlay_atlas.rs:464-465](src/render/overlay_atlas.rs#L464-L465); SHP's internal `frame_x`/`frame_y` already provides the per-frame offset within the canvas after the blit-to-full-bounds step.
- **Failure-isolation log: separate "Smudge sprites: N rendered, M failed" log line.** **Confidence:** high. **Source:** brainstorm hardening item #2; addresses log-noise concern without architectural cost.
- **Namespace prefix `__smudge::`.** **Confidence:** high. **Source:** brainstorm hardening item #1; eliminates modded-collision class. Implemented via a `smudge_key(name, frame)` helper used by both insertion and lookup so prefix can never drift.

## Open Questions

### Resolved During Planning
- **Where does palette logic live?** → No explicit palette argument in `CC_Draw_Shape` for smudges; iso theater palette is the implicit global. We pass `theater_palette` to the smudge sprite renderer and never branch on flags.
- **How many atlas entries per SmudgeType?** → Exactly one (frame=0). Verified via Ghidra.
- **Should the render layer iterate every occupied cell or only origin cells?** → Origin cells only (`frame_offset == 0`). Strictly fewer GPU instances; visually identical.
- **Where to thread `&SmudgeTypeRegistry`?** → `app_init.rs:543` → `build_overlay_atlas_from_map` → `overlay_atlas::build_overlay_atlas`. Add as the final optional parameter on each.

### Deferred to Implementation
- **Multi-cell SHP frame count assertion threshold.** If a Width>1 or Height>1 SmudgeType's SHP turns out to have multiple frames in retail content, downgrade the warn to debug. Decided at runtime once we observe live data; not a blocker.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/overlay_atlas.rs` | Add `SMUDGE_KEY_PREFIX` const, `smudge_key()` helper, `smudge_shp_candidates()` helper, `render_smudge_sprite()` helper. Extend `build_overlay_atlas` signature with `smudge_types: Option<&SmudgeTypeRegistry>` and a smudge collection+render block before the shelf-pack step. |
| Modify | `src/app_skirmish.rs` | Thread `smudge_types: Option<&SmudgeTypeRegistry>` parameter through `build_overlay_atlas_from_map`; forward to `overlay_atlas::build_overlay_atlas`. |
| Modify | `src/app_init.rs` | Pass `rules.as_ref().map(\|r\| &r.smudge_types)` into `build_overlay_atlas_from_map` at the existing call site. |
| Modify | `src/app_render/build_instances.rs` | Replace the placeholder `lookup` closure in `build_smudge_instances` with a real OverlayAtlas query via `smudge_key()`; skip cells where `frame_offset != 0`. |
| Modify | `src/render/smudge.rs` | Add `frame_offset != 0` skip to `build_visible_instances`; add a unit test covering the skip. |

## Interface Changes

- **`overlay_atlas::build_overlay_atlas`** gains a final optional parameter `smudge_types: Option<&SmudgeTypeRegistry>`. Call sites: `src/app_skirmish.rs:453`. New public re-exports from `overlay_atlas`: `pub const SMUDGE_KEY_PREFIX: &str = "__smudge::";` and `pub fn smudge_key(name: &str) -> OverlaySpriteKey`.
- **`app_skirmish::build_overlay_atlas_from_map`** gains a final optional parameter `smudge_types: Option<&SmudgeTypeRegistry>`. Call sites: `src/app_init.rs:544`.
- No changes to public types (`OverlaySpriteKey`, `OverlaySpriteEntry`, `OverlayAtlas`).
- No changes to `SmudgeTypeRegistry`, `SmudgeTypeDef`, `SmudgeGrid`, `SmudgeCell`.
- No changes to draw-pipeline binding ([src/app_render/draw_passes.rs:101-107](src/app_render/draw_passes.rs#L101-L107) already routes via `state.overlay_atlas`).

## Sim Checklist

Not applicable — this plan is render-side only. No `sim/` files touched. The sim/render boundary is preserved: render reads `&SmudgeGrid` immutably, sim mutations remain in their existing `SmudgeGrid::try_place` and `SmudgeGrid::from_map_entries` paths.

## Risk Areas

1. **Atlas size pressure.** Adding ~46 single-frame entries (~46 sprites at typical ~60×30 to ~120×60 pixels) to the OverlayAtlas. Existing OverlayAtlas already shelf-packs hundreds of overlay+wall+terrain entries. Negligible additive size; `pack_overlay_sprites` handles atlas growth automatically up to GPU limits.
2. **Theater-extension SHP load failures.** Smudges in retail YR have theater-specific filenames (`crater01.tem` vs `crater01.sno`). Wrong theater_ext or missing palette → all smudges fail to load silently. Mitigation: separate "Smudge sprites: N rendered, M failed" log; lookup-side `None` already skips the cell without panic.
3. **Namespace prefix drift.** If insertion uses `format!("__smudge::{}", x)` and lookup uses some other shape, every smudge silently disappears. Mitigation: single `smudge_key(name)` helper used by both sides.
4. **`frame_offset != 0` skip masks bugs.** If `SmudgeGrid::write_footprint` were ever to misorder cells such that the origin cell ends up with `frame_offset != 0`, the entire smudge becomes invisible. Mitigation: existing test [src/sim/smudge_grid.rs:401-403](src/sim/smudge_grid.rs#L401-L403) (`empty_filter_falls_back_to_unfiltered` writes a 2×2 and asserts 4 cells occupied) implicitly verifies origin cell has frame_offset=0; add an explicit assertion in this plan's test.
5. **Multi-cell SHP frame count.** If a Width>1 SmudgeType's SHP has more than 1 frame, our atlas registers only frame 0 — gamemd also draws frame 0 (verified). No drift, but log a debug-level note for visibility.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 2 | SmudgeType SHP rendered with **iso theater palette** (not unit pal, not tiberium pal) | Wrong palette → wrong colors on every visible smudge in every match | Ghidra `SmudgeTypeClass::Draw_It @ 0x006B55F0` confirms no explicit palette arg, implicit iso palette during world pass. Visual: in-game side-by-side at Task 5. |
| Task 2 | Anchor offset `-full_w/2, -full_h/2` (centered on cell, no Y-shift) | Wrong anchor → smudges drift visibly off the impact cell | Mirrors OverlayAtlas non-tiberium overlay anchor. Visual: Task 5 in-game V3 strike, smudge centered on impact cell. |
| Task 4 | Skip non-origin footprint cells (`frame_offset != 0`) | Without skip, multi-cell smudges draw 4× redundantly at same pixels (matches gamemd, but wastes draw calls; with skip, identical pixels and 4× fewer instances). With skip + bug in origin-tracking, smudges go invisible. | Unit test in Task 4: place a 2×2 footprint, verify exactly 1 SpriteInstance is emitted. |
| Task 1 | Theater-ext fallback chain: `name.tem` → `name.shp` → lowercase variants | Wrong order → wrong-theater smudge graphics. Many smudge SHPs are theater-specific (`Theater=yes` in artmd) so the `.tem`/`.sno`/`.urb` extension MUST be tried before `.shp`. | Mirrors `art_data::overlay_shp_candidates` shape verified in OverlayAtlas. Asset-load logging will confirm which file each SmudgeType resolves to. |
| Task 5 | V3 strike on clear ground produces a visible crater at the impact cell | Headline parity check. The whole point of the work. | In-game observation; side-by-side vs retail YR. |

---

## Tasks

### Task 1: Add smudge atlas key + filename-candidate helpers

**Why:** Foundation. Establishes the namespace-prefix invariant and the theater-fallback filename pattern. Both insertion and lookup will go through these helpers so prefix and candidate order can never drift between sides.

**Files:**
- Modify: `src/render/overlay_atlas.rs` (add public consts/helpers near the top of the file, after `SPRITE_PADDING`)

**Pattern:** Mirrors `art_data::overlay_shp_candidates` for the candidate-filename shape; adds a fixed namespace prefix to keep smudge keys collision-free with overlay keys.

**Step 1: Add public namespace constants and key helper**

```rust
// src/render/overlay_atlas.rs — insert after `const SPRITE_PADDING: u32 = 1;`

/// Namespace prefix for smudge atlas keys.
///
/// Smudges share the OverlayAtlas (single texture, single bind group) but are
/// keyed under this prefix so a SmudgeType named `CRATER01` cannot collide
/// with an overlay named `CRATER01` (modded content is the realistic
/// concern). All smudge insertions and lookups MUST go through `smudge_key()`
/// so the prefix can never drift between sides.
pub const SMUDGE_KEY_PREFIX: &str = "__smudge::";

/// Build the canonical OverlayAtlas key for a smudge SHP.
///
/// Frame is always 0 — gamemd's `SmudgeTypeClass::Draw_It` passes frame 0 to
/// `CC_Draw_Shape` for every cell of every multi-cell footprint. The
/// `frame_offset` on `SmudgeCell` is a screen-position shift, not a frame
/// index.
pub fn smudge_key(name: &str) -> OverlaySpriteKey {
    OverlaySpriteKey {
        name: format!("{}{}", SMUDGE_KEY_PREFIX, name.to_uppercase()),
        frame: 0,
    }
}
```

**Step 2: Add the smudge SHP-candidate filename helper**

Insert below `decrement_numeric_suffix` near the bottom of the existing helpers section.

```rust
/// Build the candidate SHP filename list for a SmudgeType.
///
/// Theater-extension first, then `.shp` fallback. Lowercase variants too —
/// asset_manager treats names case-sensitively in some code paths, and SHP
/// files in retail mix archives are lowercase.
fn smudge_shp_candidates(name: &str, theater_ext: &str) -> Vec<String> {
    let upper = name.to_string();
    let lower = name.to_ascii_lowercase();
    vec![
        format!("{}.{}", lower, theater_ext),
        format!("{}.shp", lower),
        format!("{}.{}", upper, theater_ext),
        format!("{}.shp", upper),
    ]
}
```

**Step 3: Verify**

Run: `cargo check --package vera20k`
Expected: PASS (no behavior change yet — only added unused public/private items).

**Step 4: Commit**

`render: smudge atlas key namespace + filename candidates`

---

### Task 2: Add render_smudge_sprite helper

**Why:** Renders one SmudgeType SHP frame 0 to RGBA using the iso theater palette and produces a `RenderedOverlay` keyed under the smudge namespace. Reuses the same packing path as overlays without touching overlay logic.

**Files:**
- Modify: `src/render/overlay_atlas.rs` (insert after `render_overlay_sprite` function, near the existing `decrement_numeric_suffix` helper)

**Pattern:** Mirrors [src/render/overlay_atlas.rs:308-475](src/render/overlay_atlas.rs#L308-L475) `render_overlay_sprite` — load SHP via candidate filename list → render frame 0 to RGBA via palette → blit into full SHP bounds → centered anchor offset.

**Step 1: Add the helper**

```rust
// src/render/overlay_atlas.rs — insert before the existing `#[cfg(test)]` block

/// Load and render a single SmudgeType SHP frame 0 to RGBA pixels.
///
/// Uses the iso theater palette unconditionally — gamemd's
/// `SmudgeTypeClass::Draw_It @ 0x006B55F0` calls `CC_Draw_Shape` with no
/// explicit palette argument; the active palette during the world pass is
/// the iso theater palette.
///
/// The anchor offset is `(-full_w/2, -full_h/2)` — centered on the
/// footprint-origin cell. SHPs for multi-cell SmudgeTypes have a single
/// composite frame whose internal `frame_x`/`frame_y` already place the
/// visual correctly relative to the canvas center.
fn render_smudge_sprite(
    asset_manager: &AssetManager,
    palette: &Palette,
    name: &str,
    theater_ext: &str,
) -> Option<RenderedOverlay> {
    let candidates: Vec<String> = smudge_shp_candidates(name, theater_ext);

    let mut found_name: String = String::new();
    let mut shp_opt: Option<ShpFile> = None;
    for candidate in &candidates {
        let Some(data) = asset_manager.get_ref(candidate) else {
            continue;
        };
        let Ok(shp) = ShpFile::from_bytes(data) else {
            continue;
        };
        let has_drawable = shp
            .frames
            .iter()
            .any(|fr| fr.frame_width > 0 && fr.frame_height > 0);
        if !has_drawable {
            continue;
        }
        found_name = candidate.clone();
        shp_opt = Some(shp);
        break;
    }
    let shp: ShpFile = shp_opt?;
    log::trace!("Smudge sprite {} uses {}", name, found_name);

    if shp.frames.is_empty() {
        return None;
    }
    let frame = &shp.frames[0];
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }

    let frame_rgba: Vec<u8> = match shp.frame_to_rgba(0, palette) {
        Ok(rgba) => rgba,
        Err(_) => return None,
    };

    let full_w: u32 = shp.width as u32;
    let full_h: u32 = shp.height as u32;
    let mut full_rgba: Vec<u8> = vec![0u8; (full_w * full_h * 4) as usize];

    let fw: u32 = frame.frame_width as u32;
    let fh: u32 = frame.frame_height as u32;
    let fx: u32 = frame.frame_x as u32;
    let fy: u32 = frame.frame_y as u32;

    for y in 0..fh {
        let dst_y: u32 = fy + y;
        if dst_y >= full_h {
            break;
        }
        let src_off: usize = (y * fw * 4) as usize;
        let copy_w: u32 = fw.min(full_w.saturating_sub(fx));
        let dst_off: usize = ((dst_y * full_w + fx) * 4) as usize;
        let bytes: usize = (copy_w * 4) as usize;
        if src_off + bytes <= frame_rgba.len() && dst_off + bytes <= full_rgba.len() {
            full_rgba[dst_off..dst_off + bytes]
                .copy_from_slice(&frame_rgba[src_off..src_off + bytes]);
        }
    }

    let offset_x: f32 = -(full_w as f32) / 2.0;
    let offset_y: f32 = -(full_h as f32) / 2.0;

    Some(RenderedOverlay {
        key: smudge_key(name),
        rgba: full_rgba,
        width: full_w,
        height: full_h,
        offset_x,
        offset_y,
    })
}
```

**Step 2: Verify**

Run: `cargo check --package vera20k`
Expected: PASS. Compiler may warn `render_smudge_sprite` is unused — that's expected; Task 3 wires it in.

**Step 3: Commit**

`render: render_smudge_sprite helper for SmudgeType frame 0`

---

### Task 3: Wire SmudgeTypeRegistry into build_overlay_atlas

**Why:** Threads `&SmudgeTypeRegistry` through the atlas-build call chain and integrates `render_smudge_sprite` into the existing collect → render → pack flow. After this task, smudge SHPs are loaded into the atlas at map-load.

**Files:**
- Modify: `src/render/overlay_atlas.rs` (`build_overlay_atlas` signature + smudge collection block)
- Modify: `src/app_skirmish.rs:269-487` (`build_overlay_atlas_from_map` signature + forward)
- Modify: `src/app_init.rs:543-555` (pass `smudge_types` at the existing call site)

**Pattern:** New parameter threaded as the final optional argument; insertion of a new collection+render block parallel to the existing wall and terrain blocks.

**Step 1: Extend `build_overlay_atlas` signature and insert smudge block**

Add the parameter to the function signature at [src/render/overlay_atlas.rs:100](src/render/overlay_atlas.rs#L100):

```rust
// src/render/overlay_atlas.rs — modify the existing fn signature

pub fn build_overlay_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    overlays: &[OverlayEntry],
    terrain_objects: &[TerrainObject],
    asset_manager: &AssetManager,
    theater_palette: &Palette,
    unit_palette: &Palette,
    tiberium_palette: &Palette,
    theater_ext: &str,
    theater_name: &str,
    overlay_registry: &OverlayTypeRegistry,
    rules_ini: &IniFile,
    art_registry: &ArtRegistry,
    smudge_types: Option<&crate::rules::smudge_type::SmudgeTypeRegistry>,
) -> Option<OverlayAtlas> {
```

Then, after the `for key in &needed { ... render_overlay_sprite ... }` block (around [src/render/overlay_atlas.rs:275](src/render/overlay_atlas.rs#L275)) and BEFORE the `if rendered.is_empty() { return None; }` check, insert the smudge render block:

```rust
    // --- Smudge SHPs ---
    // Smudges share this atlas (single texture / single bind group) but are
    // keyed under SMUDGE_KEY_PREFIX to keep the namespace collision-free with
    // overlays. Always rendered with the iso theater palette per gamemd's
    // implicit world-pass palette state.
    let mut smudge_rendered_count: u32 = 0;
    let mut smudge_failed_count: u32 = 0;
    if let Some(smudge_reg) = smudge_types {
        for (_id, def) in smudge_reg.iter_with_id() {
            match render_smudge_sprite(asset_manager, theater_palette, &def.name, theater_ext) {
                Some(sprite) => {
                    rendered.push(sprite);
                    smudge_rendered_count += 1;
                }
                None => {
                    smudge_failed_count += 1;
                    let candidates: Vec<String> = smudge_shp_candidates(&def.name, theater_ext);
                    log::debug!(
                        "Smudge sprite not found: name={} (tried: {:?})",
                        def.name,
                        candidates,
                    );
                }
            }
        }
        log::info!(
            "Smudge sprites: {} rendered, {} failed (of {} types)",
            smudge_rendered_count,
            smudge_failed_count,
            smudge_reg.len(),
        );
    }
```

**Step 2: Update `build_overlay_atlas_from_map` to forward the parameter**

Modify the function signature at [src/app_skirmish.rs:269](src/app_skirmish.rs#L269):

```rust
// src/app_skirmish.rs — modify the existing fn signature

pub(crate) fn build_overlay_atlas_from_map(
    map_data: &MapFile,
    asset_manager: &AssetManager,
    gpu: &GpuContext,
    batch: &BatchRenderer,
    theater_ext: &str,
    rules_ini: &IniFile,
    art_registry: &ArtRegistry,
    theater_iso_palette: Option<&Palette>,
    theater_unit_palette: Option<&Palette>,
    theater_tiberium_palette: Option<&Palette>,
    smudge_types: Option<&crate::rules::smudge_type::SmudgeTypeRegistry>,
) -> (
    Option<OverlayAtlas>,
    Option<BridgeAtlas>,
    BTreeMap<u8, String>,
    Vec<OverlayEntry>,
    HashMap<(u8, u8), [u8; 3]>,
) {
```

Then forward at the existing `overlay_atlas::build_overlay_atlas(...)` call site at [src/app_skirmish.rs:453](src/app_skirmish.rs#L453) — append `smudge_types` as the final argument:

```rust
        overlay_atlas::build_overlay_atlas(
            gpu,
            batch,
            &wall_overlays,
            &map_data.terrain_objects,
            asset_manager,
            theater_pal,
            unit_pal,
            tib_pal,
            theater_ext,
            &map_data.header.theater,
            &overlay_registry,
            rules_ini,
            art_registry,
            smudge_types,
        )
```

**Step 3: Pass smudge_types from app_init.rs**

At the existing call site [src/app_init.rs:543-555](src/app_init.rs#L543-L555), add the smudge_types argument:

```rust
    let (overlay_atlas, bridge_atlas, overlay_names, overlays_connected, tiberium_radar_colors) =
        build_overlay_atlas_from_map(
            &map_data,
            &asset_manager,
            gpu,
            batch,
            theater_ext,
            &rules_ini,
            art.as_ref().unwrap_or(&art_fallback),
            overlay_iso_palette.as_ref(),
            unit_palette.as_ref(),
            overlay_tiberium_palette.as_ref(),
            rules.as_ref().map(|r| &r.smudge_types),
        );
```

**Step 4: Verify**

Run: `cargo build --package vera20k`
Expected: PASS. Map-load logs should show e.g. `Smudge sprites: 46 rendered, 0 failed (of 46 types)` (or similar, depending on retail asset coverage). Smudges still don't render visibly yet — Task 4 closes that gap.

**Step 5: Commit**

`render: register SmudgeType SHPs into OverlayAtlas at map-load`

---

### Task 4: Replace placeholder lookup closure + skip non-origin cells

**Why:** Closes the loop. After this task, smudges are visible end-to-end.

**Files:**
- Modify: `src/app_render/build_instances.rs:259-279` (replace placeholder closure)
- Modify: `src/render/smudge.rs` (add `frame_offset != 0` skip + test)

**Pattern:** Lookup goes through `smudge_key(name)` to match the insertion key shape; skip is a single-line guard at the top of the per-cell loop in `build_visible_instances`.

**Step 1: Replace the placeholder closure in `build_smudge_instances`**

Modify [src/app_render/build_instances.rs:259-279](src/app_render/build_instances.rs#L259-L279):

```rust
// src/app_render/build_instances.rs

fn build_smudge_instances(state: &AppState, sw: f32, sh: f32) -> Vec<SpriteInstance> {
    let (sim, rules) = match (&state.simulation, &state.rules) {
        (Some(s), Some(r)) => (s, r),
        _ => return Vec::new(),
    };
    let Some(grid) = sim.smudge_grid.as_ref() else {
        return Vec::new();
    };
    let Some(atlas) = state.overlay_atlas.as_ref() else {
        return Vec::new();
    };
    // Resolve (smudge_type_id, frame_offset) → atlas placement.
    // Smudge SHPs are registered into the OverlayAtlas under
    // `crate::render::overlay_atlas::SMUDGE_KEY_PREFIX` at map-load time
    // (see render/overlay_atlas.rs render_smudge_sprite). Frame is always 0
    // because gamemd draws every footprint cell with frame 0 and shifts
    // screen position back to footprint origin — render-side handles the
    // shift cancellation by skipping non-origin cells inside
    // build_visible_instances.
    let lookup = |type_id: u16, _frame: u8| -> Option<TilePlacement> {
        let def = rules.smudge_types.get(type_id)?;
        let entry = atlas.get(&crate::render::overlay_atlas::smudge_key(&def.name))?;
        Some(TilePlacement {
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            pixel_size: entry.pixel_size,
            draw_offset: [entry.offset_x, entry.offset_y],
        })
    };
    crate::render::smudge::build_visible_instances(
        grid,
        &rules.smudge_types,
        &lookup,
        state.camera_x,
        state.camera_y,
        sw,
        sh,
    )
}
```

**Step 2: Add the `frame_offset != 0` skip in `build_visible_instances`**

Modify [src/render/smudge.rs:63-83](src/render/smudge.rs#L63-L83) — add the skip immediately after the `let Some(type_id) = cell.type_id` guard:

```rust
    for (rx, ry, cell) in grid.iter_occupied() {
        let Some(type_id) = cell.type_id else {
            continue;
        };
        // Multi-cell smudge footprints are stored as W×H occupied cells, but
        // gamemd draws the SHP once at the footprint origin (per-cell
        // SmudgeTypeClass::Draw_It calls cancel back to the same screen
        // position with frame=0). Skipping non-origin cells produces
        // visually identical pixels and avoids redundant SpriteInstances.
        if cell.frame_offset != 0 {
            continue;
        }
        // Confirm the type still exists in the registry — defensive against
        // map/rules mismatches; an unknown id is silently skipped.
        if registry.get(type_id).is_none() {
            continue;
        }
        // ... rest of loop unchanged ...
```

**Step 3: Add a unit test for the skip behavior**

Append to the existing `#[cfg(test)] mod tests { ... }` in [src/render/smudge.rs:101-219](src/render/smudge.rs#L101-L219):

```rust
    #[test]
    fn skips_non_origin_footprint_cells() {
        // 2x2 smudge: 4 cells occupied, frame_offsets 0..3. Only the
        // frame_offset==0 cell (footprint origin) should emit an instance.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR2\n[CR2]\nCrater=yes\nWidth=2\nHeight=2\n",
        )
        .unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let type_id = registry.find_by_name("CR2").unwrap();
        // Manually seed all 4 cells of a 2x2 footprint at origin (3,3).
        for (dx, dy) in &[(0u16, 0u16), (1, 0), (0, 1), (1, 1)] {
            let frame_offset = (*dx as u8) + (*dy as u8) * 2;
            grid.test_force_set(
                3 + dx,
                3 + dy,
                SmudgeCell {
                    type_id: Some(type_id),
                    footprint_origin: Some((3, 3)),
                    frame_offset,
                },
            );
        }
        let lookup = |_id: u16, _frame: u8| -> Option<TilePlacement> {
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [0.1, 0.1],
                pixel_size: [120.0, 60.0],
                draw_offset: [-60.0, -30.0],
            })
        };
        let v = build_visible_instances(&grid, &registry, &lookup, 0.0, 0.0, 800.0, 600.0);
        assert_eq!(
            v.len(),
            1,
            "expected 1 SpriteInstance (origin cell only); got {}",
            v.len(),
        );
    }
```

**Step 4: Verify**

Run: `cargo test --package vera20k smudge -- --nocapture`
Expected: all smudge tests PASS, including new `skips_non_origin_footprint_cells`.

Run: `cargo build --package vera20k`
Expected: PASS, no warnings about unused items in overlay_atlas.rs.

**Step 5: Commit**

`render: smudge atlas lookup + skip non-origin footprint cells`

---

### Task 5: In-game verification

**Why:** Confirms the implementation produces visible smudges that match gamemd.exe end-to-end. This is the parity-bar checkpoint — passing tests doesn't equal player-visible correctness.

**Files:**
- None modified. Manual verification only; results captured in commit message or separate notes.

**Step 1: Boot a skirmish on a temperate map**

The existing in-game launch entrypoint should pick up the changes. If a specific skirmish setup script is available, use the one with V3 launchers or a unit that fires a Crater-flagged anim.

**Step 2: Verify map-load smudges**

Open the Windows debug log (typical `RUST_LOG=info`). Confirm the boot log includes a line like:
```
Smudge sprites: N rendered, M failed (of 46 types)
```
Where N + M = 46 and N is most of them. If M is more than ~5, theater filename resolution may have a mismatch — investigate.

**Step 3: Verify in-game smudge appearance**

Run the parity checklist from the original smudge plan ([docs/plans/2026-05-06-smudge-system-plan.md:2546-2557](docs/plans/2026-05-06-smudge-system-plan.md#L2546-L2557)):

- [ ] Pre-placed smudges from `[Smudge]` map entries appear at their cell coords on the test map.
- [ ] Fire a V3 (or any Crater=yes anim warhead) at clear ground. A crater appears at the impact cell.
- [ ] V3 against water: NO smudge appears.
- [ ] V3 against an ore patch: ore is reduced (Reduce_Tiberium(6) ran) but NO smudge placed.
- [ ] Air-burst weapon detonating mid-air (z >= 30): NO smudge.
- [ ] Destroy a 4×4 conyard: multiple smudges scattered across and around the foundation.
- [ ] Destroy a 1×1 Sentry Gun: only per-cell SpawnSurvivors smudges; no center forceBig smudge.

**Step 4: Side-by-side comparison vs retail YR**

Run the same scenario in the original gamemd.exe. Compare:
- Smudge type picked at the impact cell (same name? same Crater/Burn category?)
- Smudge size (1×1 vs 2×2 footprint visually)
- Smudge offset relative to the impact cell (centered? off by one cell?)
- Color/palette (matches the iso palette tones?)

**Step 5: Commit**

`smudge: in-game verification — visible craters/scorches on V3 strikes`

(If verification reveals a parity drift, STOP and report findings rather than papering over. Root-causing must come before any fix.)

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md) — out-of-scope §"Render path for the smudge SHP atlas".
- **Prior plan:** [docs/plans/2026-05-06-smudge-system-plan.md](docs/plans/2026-05-06-smudge-system-plan.md) — Tasks 1-15 shipped (commit history: `cef4b1f` through `27dfa49`). This plan is the deferred follow-up #2 from that plan's "Follow-up tasks" section.
- **Ghidra reports (research base):**
  - [ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md](ra2-rust-game-docs/SMUDGE_CLASS_GHIDRA_REPORT.md) — §7 "Rendering — Pointer Only" (full render trace deferred there; completed by this plan's grounding).
  - [ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md](ra2-rust-game-docs/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md) — spawn-side, covered by prior plan.
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `0x006D3290` — `Tactical_layer_smudges` (smudge layer entry, called between base_terrain and overlays).
  - `0x00480350` — `CellOverlay_TileDraw` (per-cell render; dispatches to `SmudgeTypeClass` vtable+0xa0 when `Cell+0x48 != -1`).
  - `0x006B55F0` — `SmudgeTypeClass::Draw_It` (the actual smudge SHP blit; CC_Draw_Shape with frame=0, no explicit palette).
  - `0x006B5260` — `SmudgeTypeClass::Constructor` (Width/Height defaults of 1).
  - SmudgeTypeClass `+0x294` = ArrayIndex, `+0x298` = Width, `+0x29C` = Height, `+0x2A0` = Crater bool.
- **INI keys:**
  - `rulesmd.ini` `[SmudgeTypes]` numeric list (lines 1683-1727).
  - `rulesmd.ini` per-name sections — `Crater=`, `Burn=`, `Width=`, `Height=`, `Image=`.
  - `artmd.ini` per-name sections — `Theater=yes` (vestigial in our parser; we always try theater_ext first then `.shp` fallback regardless).
- **Related code:**
  - [src/render/overlay_atlas.rs](src/render/overlay_atlas.rs) — atlas-build pattern this extends.
  - [src/app_skirmish.rs:269](src/app_skirmish.rs#L269) — `build_overlay_atlas_from_map` orchestration.
  - [src/app_render/build_instances.rs:259](src/app_render/build_instances.rs#L259) — `build_smudge_instances` call site.
  - [src/app_render/draw_passes.rs:101](src/app_render/draw_passes.rs#L101) — smudge draw pass (already wired to `state.overlay_atlas`).
  - [src/render/smudge.rs:48](src/render/smudge.rs#L48) — `build_visible_instances` (closure consumer).
  - [src/sim/smudge_grid.rs:186](src/sim/smudge_grid.rs#L186) — `write_footprint` (origin cell always has `frame_offset == 0`).
- **Brainstorm session:** 2026-05-07 (this conversation) — recorded in this plan's Key Technical Decisions.

## Follow-up tasks (not in this plan)

1. **Update design ledger #26** in [docs/plans/2026-05-06-smudge-system-design.md](docs/plans/2026-05-06-smudge-system-design.md). Current text: "frame index = cell offset within W×H grid." Correct text: "frame index = 0 always; the cell offset within W×H grid is a screen-position shift that cancels back to footprint origin (cells render redundantly at the same pixels in gamemd, optimized to origin-only in our render)."
2. **Remove vestigial `SmudgeTypeDef.is_theater`** in [src/rules/smudge_type.rs](src/rules/smudge_type.rs). The field reads `Theater=` from rulesmd.ini but the actual key lives in artmd.ini, so it's always `false`. The atlas-load path doesn't need it (always tries theater_ext first). Touch covers parser + tests + struct field.
3. **Investigate logging-level for smudge SHP load failures.** If retail asset coverage is consistently 46/46, downgrade the per-failure debug log to trace. If coverage is partial, surface failed names at warn-level so missing assets are visible.
4. **(Optional) Multi-page overlay atlas if size pressure becomes real.** Current atlas builds one shelf-packed texture; if smudges + walls + tiberium + terrain anim push past GPU `max_texture_dimension_2d`, refactor to multi-page (precedent: sprite_atlas already does this).
