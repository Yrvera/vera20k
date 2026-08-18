# Shell Text Renderer Parity Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Close the 8 player-visible parity gaps in our BitFont-equivalent text rendering by splitting the current `SidebarTextRenderer` into a shared lower-layer `BitFont` plus two upper-layer wrappers, mirroring gamemd.exe's BitFont/per-path-wrapper structure in observable terms.

**Architecture:** New `bit_font.rs` owns atlas + glyph table + measurement + wrap state machine + missing-glyph fallback + darken texture. New `shell_text.rs` owns Path A (bit-flag align, vcenter, per-pixel scissor clip, max_height cutoff). Existing `sidebar_text.rs` is rewritten as a free-function module owning Path B (selected-unit fade math + side-highlight color table). `AppState.sidebar_text` field renames to `AppState.bit_font`.

**Design Doc:** [docs/plans/2026-05-17-shell-text-renderer-parity-design.md](2026-05-17-shell-text-renderer-parity-design.md)

---

## Grounding Summary

**Research docs (R1):** Primary source is `ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` (HIGH confidence on all algorithm shapes, struct layouts, address bindings). Side-color fade endpoint table is in `ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md` (cross-referenced in BITFONT §3.7). Investigation plan that produced the report is `docs/plans/2026-05-17-bitfont-shell-text-investigation-plan.md`. No new Ghidra work needed for the refactor itself.

**Ghidra verification (R2):** Already done as part of the BITFONT report. The single open Ghidra item is the `fade_param` numeric value passed by `SidebarClass__DrawCameoText` to `FUN_006211D0` — needed only to wire Path B's fade to the live sidebar Ready callers. Tracked as the deferred Task 22; the fade math itself is implemented and tested in Phase 3 against synthetic values.

**Repo patterns (R3):** Atlas-building and glyph-decode mirror existing code in [src/render/sidebar_text.rs:50-153](../../src/render/sidebar_text.rs) (lifted into `bit_font.rs`). Sprite-instance emission mirrors the same file's `build_text` ([sidebar_text.rs:233-270](../../src/render/sidebar_text.rs)). Render-pass scissor application uses `wgpu::RenderPass::set_scissor_rect` — caller-side, no new `BatchRenderer` method needed (verified at [src/render/batch.rs:1364](../../src/render/batch.rs)). `SidebarTheme` enum at [src/render/sidebar_chrome.rs:35](../../src/render/sidebar_chrome.rs).

**INI keys (R4):** BitFont/BitText system has **no INI surface** per BITFONT §4. `GAME.FNT` is hardcoded by name in gamemd's `BitText__Constructor` (string literal at `0x00818b98`). CSF strings (`TXT_READY`, `GUI:StartGame`, etc.) drive label content but don't influence the font system itself. No INI parsing tasks.

**Still unknown after grounding:** `fade_param` integer value used by `SidebarClass__DrawCameoText`. Deferred to Task 22 with a fallback plan (sidebar Ready stays on single-color rendering — unchanged from today's behavior — until the value is verified). Approximate XOR constant for missing-glyph color in 32-bit RGBA (gamemd uses 16-bit RGB565 XOR 0x5555; exact translation isn't binary-defined for 32-bit destination). Flagged at Task 4.

## Key Technical Decisions

- **`SidebarTextRenderer` struct deleted, not preserved as a shim** — per CLAUDE.md "no backwards-compatibility hacks". Field rename `state.sidebar_text` → `state.bit_font` is mechanical across six files. **Confidence:** high. **Source:** design doc *Architectural Decisions*.
- **Free-function modules for `shell_text` and `sidebar_text`** — both post-refactor hold no state (everything migrates to `BitFont`). Function-on-module is more honest than `struct + impl<&Self>`. **Confidence:** high. **Source:** design doc *Architectural Decisions*.
- **Per-pixel clip implemented via `wgpu::RenderPass::set_scissor_rect`**, not per-fragment alpha clipping or pre-batched splits. **Confidence:** high. **Source:** verified `wgpu` API at [src/render/batch.rs:1364](../../src/render/batch.rs).
- **`ShellAlign` as transparent `u8` newtype with const associated values** (not the `bitflags` crate) — avoids a new dependency. **Confidence:** high. **Source:** repo Cargo.toml — `bitflags` not in approved list.
- **Missing-glyph color XOR in 32-bit RGBA destination** uses per-channel quantize-XOR-upscale to faithfully reproduce gamemd's `color ^= 0x5555` (RGB565): `R5 ^= 0x0A; G6 ^= 0x2A; B5 ^= 0x15` then upscale back to 8-bit. This produces the same large color shift the binary does (pure white → red-orange, not a barely-perceptible tint), which is the visible signal that distinguishes missing glyphs at a glance. **Confidence:** high. **Source:** BITFONT §3.3 decomposed; verified by `/review-plan`.
- **`fade_param` value is UNKNOWN.** The fade math in `sidebar_text::build_text_with_fade` is implementable + testable against arbitrary values; only the live wire-up is blocked. Deferred to Task 22. **Confidence:** low (on the value itself); **high** on the math formula. **Source:** BITFONT §3.7 — formula verified, value not extracted.

## Open Questions

### Resolved During Planning

- **`SidebarTheme` exists and exposes Allied/Soviet/Yuri** — yes, [src/render/sidebar_chrome.rs:35](../../src/render/sidebar_chrome.rs).
- **`current_sidebar_theme(state)` for sidebar fade endpoint selection** — yes, already used at [src/app_render/build_instances.rs:623](../../src/app_render/build_instances.rs).
- **Does `BatchRenderer` need a new draw method with built-in scissor?** — no; the caller sets it on the `RenderPass` before calling existing `draw_with_buffer_passthrough`. Verified at [src/render/batch.rs:1364](../../src/render/batch.rs).
- **Does `bitflags` crate need adding?** — no; transparent `u8` newtype with const associated values is sufficient and avoids new dep.

### Deferred to Implementation

- **Exact `fade_param` integer value** used by `SidebarClass__DrawCameoText` (gamemd `0x004A60E0` caller chain to `FUN_006211D0`). Needed to wire Path B fade to the live sidebar Ready callers (Task 22). Until verified, sidebar Ready continues to render single-color (unchanged from today). The fade math + side-highlight table are implemented + tested in Phase 3 regardless.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/render/bit_font.rs` | Lower layer: atlas, glyph table, missing-glyph fallback, text_width, wrap_layout state machine, build_text, darken texture, side-color XOR helper |
| Create | `src/render/shell_text.rs` | Path A wrapper: bit-flag align, vcenter via measure-then-offset, per-line h-align, scissor clip, max_height cutoff |
| Modify | `src/render/sidebar_text.rs` | Rewritten as free-function module: pass-through helpers + side_highlight_color + build_text_with_fade |
| Modify | `src/render/mod.rs` | `pub mod bit_font;` + `pub mod shell_text;` |
| Modify | `src/app.rs:50, 176, 683` | Import path + field rename (`sidebar_text: SidebarTextRenderer` → `bit_font: BitFont`) + construction call |
| Modify | `src/app_transitions.rs:113` | Construction call switches to `BitFont::from_fnt` |
| Modify | `src/app_skirmish_shell_render.rs:450-590, 661-671` | `push_centered_text` calls `shell_text::draw_in_rect`; render pass loop sets per-rect scissor |
| Modify | `src/app_sidebar_build.rs:519-632` | Field rename only — Ready text stays on `bit_font.build_text` (single-color) until Task 22 wires fade |
| Modify | `src/app_render/draw_passes.rs:451` | `state.sidebar_text.texture()` → `state.bit_font.atlas()` |
| Modify | `src/app_render/build_instances.rs:624` | Import path: `app_sidebar_text::ready_color_for_theme` → `render::sidebar_text::side_highlight_color` |
| Modify | `src/app_sidebar_text.rs:21-33` | Delete `READY_COLOR_*` constants and `ready_color_for_theme` (moved to `render::sidebar_text`) |

## Interface Changes

- **`SidebarTextRenderer` struct deleted.** No callers retain a struct handle; all access is via `&state.bit_font` (BitFont) plus the free-function modules.
- **`AppState.sidebar_text` → `AppState.bit_font`** (field rename). All six call sites mechanically updated.
- **New public types in `render::bit_font`:** `BitFont`, `GlyphEntry`, `WrapLayout`, `LineSpan`.
- **New public types in `render::shell_text`:** `ShellAlign`, `ScissorRect`, `ShellTextDraw`. New public fn `shell_text::draw_in_rect`.
- **New public fn in `render::sidebar_text`:** `build_text_with_fade`, `side_highlight_color`, plus pass-throughs `text_width`, `glyph_height`, `darken_texture`, `texture`, `build_text` (signatures take `&BitFont` first arg).
- **`app_sidebar_text::ready_color_for_theme` removed.** Single consumer at `build_instances.rs:624` switches import to `render::sidebar_text::side_highlight_color`.

## Sim Checklist

**Not applicable** — this plan touches only `render/` and `app_*` layers. No `sim/` files modified, no tick-ordering implications, no state-hash impact.

## Risk Areas

- **Field rename across six files** is mechanical but easy to miss a site. Mitigation: Task 19's `cargo check` will surface any missed call. Use `grep -rn "state.sidebar_text"` to confirm zero remaining hits before Task 19.
- **Sidebar Ready text continues rendering single-color** between Task 18 (refactor complete) and Task 22 (fade wired). This is a continuation of today's parity drift, not a regression — explicitly accepted in the design.
- **Space-glyph width fix** (FNT-measured ≠ today's `glyph_h × 0.4` fudge) will shift Ready/queue-count layout by ≤ 1 px. Accept the new baseline; if the old hardcoded position depends on the fudge, adjust the position constant at the call site rather than reintroducing the fudge.
- **Scissor-rect lifecycle** in the shell render pass: scissor is render-pass state, must be reset to full render size after the shell-text loop or subsequent draws will be clipped. Task 17 includes the explicit reset.
- **`bit_font` ownership in `AppState`**: construction order in `app.rs` must produce `bit_font` before any caller that takes `&state.bit_font`. Current construction sequence at [app.rs:683](../../src/app.rs) is the analog — keep the same position.

## Parity-Critical Items

| Task # | Item | Why it matters (trigger frequency) | Verification |
|--------|------|-----------------------------------|--------------|
| Task 2 | Space character uses real FNT glyph width, not `glyph_h × 0.4` fudge | Fires on **every** text string with a space — every shell button label ("Start Game", "Choose Map"), every sidebar Ready text, every queue badge. Misalignment visible every shell paint and every sidebar paint | `bit_font` unit test asserts FNT space width measured; in-game eyeball of "Choose Map" button kerning vs gamemd |
| Task 2 | Missing-glyph fallback exists and is the inverted '°' from FNT codepoint 0xB0 | Fires whenever a localized CSF string contains a codepoint outside the packed `0x20..0x180` range. Currently silently dropped; with fix, visibly distinct | `bit_font` test: codepoint > 0x180 produces an entry in `build_text` output; visual check of a Cyrillic CSF if one is loaded |
| Task 4 | Missing-glyph color XOR (32-bit RGBA approximation of `0x5555` RGB565) | Same fires-on-missing-codepoint frequency as above. Distinguishes "valid glyph that happens to look weird" from "missing glyph fallback" at a glance | `bit_font` test: `missing_color_xor(tint) != tint`; visual confirmation that fallback glyphs render with a desaturated/dithered tint |
| Task 6 | Word-wrap algorithm (last-space + hard-cut backtrack + single-char-overflow accept) | Currently fires zero times because nothing wraps — but the moment a long localized label appears (German/Russian "Start Game" equivalents often double in length), correct wrap or hard-cut is what makes the button readable vs garbled | `wrap_layout` tests against known inputs; visual confirmation on a deliberately-long synthetic label |
| Task 5 | Tab stops at 64-px boundaries with origin-subtract | Rare in shell, common in any future listbox column layout. Per BITFONT §3.2 the algorithm exists in the live YR draw path, so any tab in any rendered string must align correctly | `text_width` test `"a\tb"` advances to next 64-px boundary after 'a' |
| Task 6 | CRLF normalization (`\r\n` = 1 newline; bare `\n` or `\r` each advance 1 line) | Fires whenever a string with mixed line endings reaches the renderer. CSF strings can contain literal CRLF | `wrap_layout` tests with CRLF + bare CR + bare LF |
| Task 10 | Per-pixel clip via scissor rect | Fires whenever a shell button label is wider than the button rect (long localized labels). Current behavior silently drops the entire label; with fix, clips per-pixel as gamemd does | Scissor-rect unit test; visual: deliberately oversized label, verify pixel-clip not character-drop |
| Task 10 | V-center via measure-then-offset (`y += (rect.h - measured_h) / 2`) | Fires on every shell button with `V_CENTER` flag — every paint of every button. Off-by-1-px vertical drift is visible every interaction | `vcenter_offsets_correctly` unit test; visual: button label sits visibly centered in pressed and unpressed variants |
| Task 10 | Per-line h-align (each line independently centered/right-aligned) | Fires on multi-line centered/right-aligned text. Currently rare in shell, but the moment any label wraps it must center per-line not per-block | `align_center_per_line` and `align_right_per_line` unit tests with known multi-line inputs |
| Task 12 (math) / Task 22 (wire) | Selected-unit fade math: `line_offset = (9 - fade_param) * 0x1F`, lerp `tint = base + (highlight - base) * line_offset/255` per char | Sidebar Ready cameo's color pulse fires every time a production item completes (many times per match). Without fade, Ready text is a flat side-color tint vs gamemd's gradient — visible drift every build-complete event | `fade_only_first_8_chars`, `fade_lerp_endpoints` tests for math; in-game side-by-side once Task 22 wires the live caller |
| Task 12 | Side highlight color per side: Allied `RGB(164,210,255)`, Soviet `RGB(255,255,0)`, Yuri `RGB(255,255,0)` | Endpoint of the Ready fade pulse. Wrong side color = wrong pulse hue every time anything completes | `side_highlight_table` test against BITFONT §3.7 verbatim; visual confirmation per side once Task 22 wires |

---

## Tasks

### Task 1: Scaffold `bit_font.rs` with types, constants, and module registration

**Why:** Establish the public surface before any function bodies. Mirrors the established pattern of "types first, impls second" from existing render modules.

**Files:**
- Create: `src/render/bit_font.rs`
- Modify: `src/render/mod.rs`

**Pattern:** Module layout follows [src/render/sidebar_text.rs:1-45](../../src/render/sidebar_text.rs) header style.

**Step 1: Create `src/render/bit_font.rs` with module header and constants:**

```rust
//! Lower-layer bitmap font: atlas, glyph table, measurement, wrap state
//! machine, missing-glyph fallback. Owned by `AppState.bit_font` and shared
//! by `render::shell_text` (Path A) and `render::sidebar_text` (Path B).
//!
//! Glyph data comes from a parsed [`crate::assets::fnt_file::FntFile`]
//! (GAME.FNT). Falls back to a hardcoded 5×7 path when the FNT is unavailable.
//!
//! Public surface:
//!   - [`BitFont::from_fnt`] / [`BitFont::fallback_5x7`] constructors
//!   - [`BitFont::text_width`] / [`BitFont::wrap_layout`] for measurement
//!   - [`BitFont::build_text`] for sprite-instance emission
//!   - [`BitFont::missing_color_xor`] for caller-side tint adjustment

use std::collections::HashMap;

use crate::assets::fnt_file::FntFile;
use crate::render::batch::{BatchRenderer, BatchTexture, SpriteInstance};
use crate::render::gpu::GpuContext;

/// Hardcoded inter-glyph spacing — matches gamemd's BitFont outer +0x2C default.
pub const CHAR_SPACING: u32 = 1;
/// Tab stop width in pixels — matches gamemd's BitFont outer +0x28 default.
pub const TAB_WIDTH: u32 = 64;
/// Tab origin — subtracted from x before `% TAB_WIDTH`.
pub const TAB_ORIGIN: u32 = 0;
/// Cell height for GAME.FNT (line advance = bitmap_rows + 1px gap).
pub const CELL_HEIGHT: u32 = 17;
/// Bitmap rows per glyph for GAME.FNT.
pub const BITMAP_ROWS: u32 = 16;
/// Source codepoint for the missing-glyph fallback (CP1252 '°').
pub const MISSING_GLYPH_CODEPOINT: u16 = 0xB0;
/// Darken-strip alpha for sidebar Ready overlay (matches gamemd AlphaBlendRect 0xAF use).
pub const DARKEN_ALPHA: u8 = 175;
/// Default fallback space width when FNT lacks a glyph at 0x20 (defensive).
const DEFAULT_SPACE_WIDTH: u32 = 4;
/// Codepoint range packed into the atlas (ASCII + Latin-1 + Latin Extended-A).
const PACKED_CODEPOINT_RANGE: std::ops::Range<u16> = 0x20..0x0180;

/// UV + pixel-width record for a single glyph in the atlas.
#[derive(Clone, Copy, Debug)]
pub struct GlyphEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_width: f32,
}

/// One line in a wrap layout — half-open byte range into the source string.
#[derive(Clone, Copy, Debug)]
pub struct LineSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub width: u32,
}

/// Result of [`BitFont::wrap_layout`] — total bounds + per-line spans.
#[derive(Clone, Debug, Default)]
pub struct WrapLayout {
    pub width: u32,
    pub height: u32,
    pub lines: Vec<LineSpan>,
}

/// Atlas-backed bitmap font + measurement + missing-glyph fallback.
///
/// Texture fields are `Option<BatchTexture>` so pure-measurement tests can
/// construct a `BitFont` without a GPU context (`atlas`/`darken_texture`
/// accessors `expect` Some — production callers always populate via
/// `from_fnt`/`fallback_5x7`).
pub struct BitFont {
    atlas_texture: Option<BatchTexture>,
    glyphs: HashMap<u16, GlyphEntry>,
    missing_glyph: Option<GlyphEntry>,
    cell_height: u32,
    bitmap_rows: u32,
    space_width: u32,
    char_spacing: u32,
    tab_width: u32,
    tab_origin: u32,
    darken_texture: Option<BatchTexture>,
}

impl BitFont {
    pub fn atlas(&self) -> &BatchTexture {
        self.atlas_texture.as_ref().expect("BitFont atlas not populated (test-only ctor)")
    }
    pub fn darken_texture(&self) -> &BatchTexture {
        self.darken_texture.as_ref().expect("BitFont darken_texture not populated (test-only ctor)")
    }
    pub fn glyph_height(&self) -> f32 { self.bitmap_rows as f32 }
    pub fn cell_height(&self) -> f32 { self.cell_height as f32 }
}
```

**Step 2: Register module in `src/render/mod.rs`:**

```rust
pub mod bit_font;
pub mod shell_text;   // also added now to keep mod.rs edits in one place
```

(Both modules added; `shell_text.rs` is empty file for now, scaffolded in Task 9.)

**Step 3: Create empty placeholder `src/render/shell_text.rs`:**

```rust
//! Placeholder — see Task 9.
```

**Step 4: Verify**
Run: `cargo check`
Expected: PASS (no impl blocks, no errors).

**Step 5: Commit** — `render/bit_font: scaffold types + module registration`

---

### Task 2: Implement `BitFont::from_fnt` — atlas + missing-glyph build + space-width extraction

**Why:** Lift the existing atlas-building logic from `sidebar_text.rs` and extend with two parity fixes: real FNT space width (not `glyph_h × 0.4`) and missing-glyph fallback (inverted '°' from codepoint 0xB0).

**Files:**
- Modify: `src/render/bit_font.rs`

**Pattern:** Atlas shelf-packing lifted from [src/render/sidebar_text.rs:49-153](../../src/render/sidebar_text.rs).

**Step 1: Add `from_fnt` constructor to `impl BitFont`:**

```rust
impl BitFont {
    pub fn from_fnt(gpu: &GpuContext, batch: &BatchRenderer, fnt: &FntFile) -> Self {
        // Collect glyphs in the packed codepoint range.
        let mut entries: Vec<(u16, &crate::assets::fnt_file::FntGlyph)> = Vec::new();
        for cp in PACKED_CODEPOINT_RANGE {
            if let Some(g) = fnt.glyph(cp) {
                entries.push((cp, g));
            }
        }

        // Build the missing-glyph bitmap: inverted '°' (codepoint 0xB0).
        // Stored as a synthesized FntGlyph reachable via a sentinel codepoint
        // that won't collide with anything in the packed range.
        let missing_owned: Option<crate::assets::fnt_file::FntGlyph> =
            fnt.glyph(MISSING_GLYPH_CODEPOINT).map(|src| {
                let mut rgba = src.rgba.clone();
                // Invert RGB triplet on every set pixel; transparent pixels stay transparent.
                // The source is white-on-transparent (255,255,255,255 / 0,0,0,0).
                // After invert, set pixels become (0,0,0,255) and transparent stays (0,0,0,0).
                // We render via tint anyway, so use luminance toggle: keep alpha, swap value.
                let mut i = 0;
                while i + 3 < rgba.len() {
                    let a = rgba[i + 3];
                    rgba[i] = !rgba[i] & a;
                    rgba[i + 1] = !rgba[i + 1] & a;
                    rgba[i + 2] = !rgba[i + 2] & a;
                    i += 4;
                }
                crate::assets::fnt_file::FntGlyph { width: src.width, rgba }
            });
        let missing_entry_data = missing_owned.as_ref().map(|g| (u32::MAX as u16, g));

        if entries.is_empty() && missing_entry_data.is_none() {
            log::warn!("FNT has no glyphs, falling back to hardcoded font");
            return Self::fallback_5x7(gpu, batch);
        }

        let row_h = fnt.bitmap_rows;
        let pad = 1u32;
        let max_atlas_w = 512u32;

        struct Placement { x: u32, y: u32 }
        let mut placements: Vec<Placement> = Vec::with_capacity(entries.len() + 1);
        let mut cursor_x = 0u32;
        let mut cursor_y = 0u32;
        let mut atlas_w = 0u32;

        let pack_iter = entries.iter().chain(missing_entry_data.iter().copied().map(|p| &p).into_iter());
        // (Use a unified iterator that yields (cp, &FntGlyph) tuples.)
        let all_entries: Vec<(u16, &crate::assets::fnt_file::FntGlyph)> = entries
            .iter()
            .copied()
            .chain(missing_entry_data.iter().copied())
            .collect();

        for (_cp, g) in &all_entries {
            let w = g.width + pad * 2;
            if cursor_x + w > max_atlas_w {
                cursor_x = 0;
                cursor_y += row_h + pad * 2;
            }
            placements.push(Placement { x: cursor_x + pad, y: cursor_y + pad });
            cursor_x += w;
            if cursor_x > atlas_w {
                atlas_w = cursor_x;
            }
        }
        let atlas_h = cursor_y + row_h + pad * 2;

        let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs = HashMap::new();
        let mut missing_glyph = None;

        for (idx, (cp, g)) in all_entries.iter().enumerate() {
            let pl = &placements[idx];
            for row in 0..row_h {
                for col in 0..g.width {
                    let src = ((row * g.width + col) * 4) as usize;
                    if src + 3 >= g.rgba.len() {
                        continue;
                    }
                    let dst_x = pl.x + col;
                    let dst_y = pl.y + row;
                    let dst = ((dst_y * atlas_w + dst_x) * 4) as usize;
                    rgba[dst..dst + 4].copy_from_slice(&g.rgba[src..src + 4]);
                }
            }
            let entry = GlyphEntry {
                uv_origin: [pl.x as f32 / atlas_w as f32, pl.y as f32 / atlas_h as f32],
                uv_size: [g.width as f32 / atlas_w as f32, row_h as f32 / atlas_h as f32],
                pixel_width: g.width as f32,
            };
            if *cp == u32::MAX as u16 {
                missing_glyph = Some(entry);
            } else {
                glyphs.insert(*cp, entry);
            }
        }

        let space_width = fnt.glyph(0x20).map(|g| g.width).unwrap_or(DEFAULT_SPACE_WIDTH);
        let atlas_texture = batch.create_texture(gpu, &rgba, atlas_w, atlas_h);
        let darken_texture = batch.create_texture(gpu, &[0u8, 0, 0, DARKEN_ALPHA], 1, 1);

        log::info!(
            "BitFont atlas: {}×{} px, {} glyphs (+missing={}), space_width={}",
            atlas_w, atlas_h, glyphs.len(), missing_glyph.is_some(), space_width
        );

        Self {
            atlas_texture: Some(atlas_texture),
            glyphs,
            missing_glyph,
            cell_height: fnt.cell_height,
            bitmap_rows: fnt.bitmap_rows,
            space_width,
            char_spacing: CHAR_SPACING,
            tab_width: TAB_WIDTH,
            tab_origin: TAB_ORIGIN,
            darken_texture: Some(darken_texture),
        }
    }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS. Compile errors in the iterator chain (sketched above with `.chain(...)` rough syntax) should be cleaned up to a working form — pre-collect `all_entries` once, iterate by index. Resolve any `.into_iter()` / borrow issues.

**Step 3: Commit** — `render/bit_font: implement from_fnt with missing-glyph fallback + real space width`

---

### Task 3: Implement `BitFont::fallback_5x7`

**Why:** Preserve the defensive debug path used when GAME.FNT fails to load.

**Files:**
- Modify: `src/render/bit_font.rs`

**Pattern:** Lift from [src/render/sidebar_text.rs:156-195](../../src/render/sidebar_text.rs) including the `supported_glyphs()` and `write_glyph_bitmap()` helpers.

**Step 1: Add `fallback_5x7` constructor and supporting private functions:**

```rust
impl BitFont {
    pub fn fallback_5x7(gpu: &GpuContext, batch: &BatchRenderer) -> Self {
        const GLYPH_W: u32 = 5;
        const GLYPH_H: u32 = 7;
        const GLYPH_PAD: u32 = 1;
        const ATLAS_COLUMNS: usize = 8;

        let supported = fallback_5x7_glyphs();
        let rows = supported.len().div_ceil(ATLAS_COLUMNS);
        let cell_w = GLYPH_W + GLYPH_PAD * 2;
        let cell_h = GLYPH_H + GLYPH_PAD * 2;
        let atlas_w = (ATLAS_COLUMNS as u32) * cell_w;
        let atlas_h = (rows as u32) * cell_h;
        let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs = HashMap::new();

        for (idx, (ch, bitmap)) in supported.iter().enumerate() {
            let col = (idx % ATLAS_COLUMNS) as u32;
            let row = (idx / ATLAS_COLUMNS) as u32;
            let origin_x = col * cell_w + GLYPH_PAD;
            let origin_y = row * cell_h + GLYPH_PAD;
            write_5x7_glyph_bitmap(&mut rgba, atlas_w, origin_x, origin_y, bitmap);
            glyphs.insert(
                *ch as u16,
                GlyphEntry {
                    uv_origin: [origin_x as f32 / atlas_w as f32, origin_y as f32 / atlas_h as f32],
                    uv_size: [GLYPH_W as f32 / atlas_w as f32, GLYPH_H as f32 / atlas_h as f32],
                    pixel_width: GLYPH_W as f32,
                },
            );
        }

        Self {
            atlas_texture: Some(batch.create_texture(gpu, &rgba, atlas_w, atlas_h)),
            glyphs,
            missing_glyph: None,
            cell_height: GLYPH_H + 1,
            bitmap_rows: GLYPH_H,
            space_width: GLYPH_W,
            char_spacing: CHAR_SPACING,
            tab_width: TAB_WIDTH,
            tab_origin: TAB_ORIGIN,
            darken_texture: Some(batch.create_texture(gpu, &[0u8, 0, 0, DARKEN_ALPHA], 1, 1)),
        }
    }
}

fn write_5x7_glyph_bitmap(rgba: &mut [u8], atlas_w: u32, origin_x: u32, origin_y: u32, rows: &[&str; 7]) {
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.as_bytes().iter().enumerate() {
            if *pixel != b'#' { continue; }
            let idx = (((origin_y + y as u32) * atlas_w + (origin_x + x as u32)) * 4) as usize;
            rgba[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
}

fn fallback_5x7_glyphs() -> Vec<(char, [&'static str; 7])> {
    // Lift the full table from src/render/sidebar_text.rs:291-689 verbatim.
    vec![
        (' ', [".....", ".....", ".....", ".....", ".....", ".....", "....."]),
        // ... full table copied from sidebar_text.rs ...
    ]
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/bit_font: port 5x7 fallback path`

---

### Task 4: Implement `BitFont::missing_color_xor`

**Why:** Caller-side tint adjustment for missing-glyph rendering. Faithfully reproduces gamemd's RGB565 `color ^= 0x5555` by round-tripping each channel through its RGB565 quantization before applying the XOR, so a missing glyph renders with the same large color shift the binary produces (e.g. pure white → red-orange).

**Files:**
- Modify: `src/render/bit_font.rs`

**Step 1: Add the helper:**

```rust
impl BitFont {
    /// Tint adjustment for missing-glyph fallback rendering — caller XORs
    /// the input tint to produce the visible "wrong color" effect that
    /// distinguishes missing glyphs at a glance. Faithful 32-bit port of
    /// gamemd's RGB565 `color ^= 0x5555`: decomposing 0x5555 into RGB565
    /// component XOR masks gives R5 ^= 0x0A, G6 ^= 0x2A, B5 ^= 0x15.
    /// See BITFONT §3.3.
    pub fn missing_color_xor(rgb: [f32; 3]) -> [f32; 3] {
        fn xor_565(c: f32, bits: u32, mask: u8) -> f32 {
            let max_val = (1u32 << bits) - 1;
            let quantized = ((c.clamp(0.0, 1.0) * max_val as f32) as u32) as u8;
            let flipped = (quantized ^ mask) & (max_val as u8);
            (flipped as f32) / (max_val as f32)
        }
        [
            xor_565(rgb[0], 5, 0x0A),  // R5
            xor_565(rgb[1], 6, 0x2A),  // G6
            xor_565(rgb[2], 5, 0x15),  // B5
        ]
    }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/bit_font: add missing_color_xor helper`

---

### Task 5: Implement `BitFont::text_width` — single-line measurement

**Why:** Single-line width including space-as-glyph, tab math, and missing-glyph fallback. Used by both upper wrappers and by simple sidebar callers.

**Files:**
- Modify: `src/render/bit_font.rs`

**Pattern:** Direct port of BITFONT §3.2 single-line iteration, minus the wrap logic (that goes in `wrap_layout`).

**Step 1: Add `text_width`:**

```rust
impl BitFont {
    pub fn text_width(&self, text: &str) -> u32 {
        let mut x: u32 = 0;
        let mut count: u32 = 0;
        for ch in text.chars() {
            match ch {
                '\t' => {
                    let advanced = x + self.tab_width;
                    x = advanced - ((advanced.saturating_sub(self.tab_origin)) % self.tab_width);
                    continue;
                }
                '\r' | '\n' => {
                    // Single-line measurement — newlines are not counted.
                    continue;
                }
                ' ' => {
                    x += self.space_width;
                    count += 1;
                }
                other => {
                    let cp = other as u32;
                    let w = if cp <= u16::MAX as u32 {
                        self.glyphs
                            .get(&(cp as u16))
                            .map(|g| g.pixel_width as u32)
                            .or_else(|| self.missing_glyph.as_ref().map(|g| g.pixel_width as u32))
                    } else {
                        self.missing_glyph.as_ref().map(|g| g.pixel_width as u32)
                    };
                    if let Some(w) = w {
                        x += w;
                        count += 1;
                    }
                }
            }
        }
        if count > 1 {
            x += (count - 1) * self.char_spacing;
        }
        x
    }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/bit_font: implement text_width with tab/space/missing-glyph`

---

### Task 6: Implement `BitFont::wrap_layout` — full state machine

**Why:** The word-wrap algorithm from BITFONT §3.2 is the load-bearing parity item for any multi-line text. Used by `shell_text::draw_in_rect` to lay out wrapped buttons or future long-text labels. Includes CRLF normalization.

**Files:**
- Modify: `src/render/bit_font.rs`

**Pattern:** Direct port of BITFONT §3.2 `MeasureText` state machine, returning `WrapLayout` instead of two out-params.

**Step 1: Add `wrap_layout`:**

```rust
impl BitFont {
    pub fn wrap_layout(&self, text: &str, max_width: u32) -> WrapLayout {
        if text.is_empty() {
            return WrapLayout::default();
        }
        let mut lines: Vec<LineSpan> = Vec::new();
        let mut line_start_byte: usize = 0;
        let mut line_x: u32 = 0;
        let mut chars_on_line: u32 = 0;
        let mut max_line_width: u32 = 0;
        let mut last_space_byte: Option<usize> = None;
        let mut last_space_byte_after: usize = 0;
        let mut last_space_x: u32 = 0;
        let mut prev_char: Option<char> = None;
        let mut y_lines: u32 = 1;

        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (byte_off, ch) = chars[i];
            let next_byte_off = chars.get(i + 1).map(|c| c.0).unwrap_or(text.len());

            match ch {
                '\t' => {
                    let advanced = line_x + self.tab_width;
                    line_x = advanced - ((advanced.saturating_sub(self.tab_origin)) % self.tab_width);
                    prev_char = Some('\t');
                    i += 1;
                    continue;
                }
                '\r' | '\n' => {
                    // CRLF normalization: \n after \r is suppressed.
                    if ch == '\n' && prev_char == Some('\r') {
                        prev_char = Some('\n');
                        i += 1;
                        continue;
                    }
                    lines.push(LineSpan { start_byte: line_start_byte, end_byte: byte_off, width: line_x });
                    max_line_width = max_line_width.max(line_x);
                    line_start_byte = next_byte_off;
                    line_x = 0;
                    chars_on_line = 0;
                    last_space_byte = None;
                    y_lines += 1;
                    prev_char = Some(ch);
                    i += 1;
                    continue;
                }
                ' ' => {
                    last_space_byte = Some(byte_off);
                    last_space_byte_after = next_byte_off;
                    last_space_x = line_x;
                    // FALLTHROUGH: space is drawn (advance + count) like any glyph.
                }
                _ => {}
            }

            let glyph_w = self.lookup_glyph_width(ch);
            let Some(glyph_w) = glyph_w else {
                prev_char = Some(ch);
                i += 1;
                continue;
            };
            let spacing = if chars_on_line == 0 { 0 } else { self.char_spacing };
            let next_x = line_x + spacing + glyph_w;

            if max_width == 0 || next_x <= max_width {
                line_x = next_x;
                chars_on_line += 1;
                prev_char = Some(ch);
                i += 1;
            } else if chars_on_line == 0 {
                // Single char wider than max_width — accept the overflow.
                line_x = next_x;
                chars_on_line = 1;
                prev_char = Some(ch);
                i += 1;
            } else if let Some(space_b) = last_space_byte {
                lines.push(LineSpan { start_byte: line_start_byte, end_byte: space_b, width: last_space_x });
                max_line_width = max_line_width.max(last_space_x);
                line_start_byte = last_space_byte_after;
                line_x = 0;
                chars_on_line = 0;
                last_space_byte = None;
                y_lines += 1;
                // Do NOT advance i — retry this char on the new line.
            } else {
                // No space on line — hard cut before this char.
                lines.push(LineSpan { start_byte: line_start_byte, end_byte: byte_off, width: line_x });
                max_line_width = max_line_width.max(line_x);
                line_start_byte = byte_off;
                line_x = 0;
                chars_on_line = 0;
                y_lines += 1;
                // Do NOT advance i — retry this char on the new line.
            }
        }
        // Flush final line.
        lines.push(LineSpan { start_byte: line_start_byte, end_byte: text.len(), width: line_x });
        max_line_width = max_line_width.max(line_x);

        WrapLayout {
            width: max_line_width,
            height: self.cell_height * y_lines,
            lines,
        }
    }

    fn lookup_glyph_width(&self, ch: char) -> Option<u32> {
        if ch == ' ' { return Some(self.space_width); }
        let cp = ch as u32;
        if cp <= u16::MAX as u32 {
            if let Some(g) = self.glyphs.get(&(cp as u16)) {
                return Some(g.pixel_width as u32);
            }
        }
        self.missing_glyph.as_ref().map(|g| g.pixel_width as u32)
    }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/bit_font: implement wrap_layout state machine`

---

### Task 7: Implement `BitFont::build_text` — sprite-instance emission

**Why:** Emit `SpriteInstance` per visible glyph for a single-line substring at a given position. Missing-glyph chars use the XOR'd tint. Multi-line emission is the upper wrapper's job (one call per `LineSpan`).

**Files:**
- Modify: `src/render/bit_font.rs`

**Pattern:** Lifted from [src/render/sidebar_text.rs:233-270](../../src/render/sidebar_text.rs), extended with tab/missing-glyph handling.

**Step 1: Add `build_text`:**

```rust
impl BitFont {
    pub fn build_text(
        &self,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        depth: f32,
        tint: [f32; 3],
        camera_offset: [f32; 2],
    ) -> Vec<SpriteInstance> {
        let mut instances = Vec::with_capacity(text.len());
        let mut cursor_x = x;
        let spacing = self.char_spacing as f32 * scale;
        let h = self.bitmap_rows as f32 * scale;
        let missing_tint = Self::missing_color_xor(tint);
        let mut emitted = 0u32;

        for ch in text.chars() {
            match ch {
                '\t' => {
                    let cell_x = ((cursor_x - x) / scale) as u32;
                    let advanced = cell_x + self.tab_width;
                    let next_cell = advanced - ((advanced.saturating_sub(self.tab_origin)) % self.tab_width);
                    cursor_x = x + (next_cell as f32) * scale;
                    continue;
                }
                '\r' | '\n' => continue,
                ' ' => {
                    if emitted > 0 { cursor_x += spacing; }
                    cursor_x += self.space_width as f32 * scale;
                    emitted += 1;
                    continue;
                }
                _ => {}
            }
            let cp = ch as u32;
            let (entry, use_missing_tint) = if cp <= u16::MAX as u32 {
                match self.glyphs.get(&(cp as u16)) {
                    Some(g) => (Some(*g), false),
                    None => (self.missing_glyph, true),
                }
            } else {
                (self.missing_glyph, true)
            };
            let Some(entry) = entry else { continue };
            if emitted > 0 { cursor_x += spacing; }
            let w = entry.pixel_width * scale;
            instances.push(SpriteInstance {
                position: [cursor_x + camera_offset[0], y + camera_offset[1]],
                size: [w, h],
                uv_origin: entry.uv_origin,
                uv_size: entry.uv_size,
                depth,
                tint: if use_missing_tint { missing_tint } else { tint },
                alpha: 1.0,
                ..Default::default()
            });
            cursor_x += w;
            emitted += 1;
        }
        instances
    }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/bit_font: implement build_text with missing-glyph XOR tint`

---

### Task 8: Add `bit_font.rs` unit tests

**Why:** Lock in the parity-critical lower-layer behavior. Tests cover space-width, tab, missing-glyph, wrap (CRLF / hard-cut / word-wrap / single-char-overflow), and XOR.

**Files:**
- Modify: `src/render/bit_font.rs` (append `#[cfg(test)] mod tests`)

**Step 1: Add the test module. Some tests use a synthetic small BitFont built in-test (no GPU needed since measurement is pure data). Module is `pub(crate)` so `make_test_font` is reachable from `shell_text::tests` and `sidebar_text::tests` for their integration tests.**

```rust
#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Build a measurement-only BitFont without a GPU context, for pure-logic tests.
    /// Atlas/darken textures are left as None — these tests never call `atlas()`
    /// or `darken_texture()`. Production callers always go through `from_fnt` /
    /// `fallback_5x7` which populate both.
    pub(crate) fn make_test_font(glyph_widths: &[(u16, u32)], space_width: u32) -> BitFont {
        let mut glyphs = HashMap::new();
        for (cp, w) in glyph_widths {
            glyphs.insert(*cp, GlyphEntry {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_width: *w as f32,
            });
        }
        BitFont {
            atlas_texture: None,
            glyphs,
            missing_glyph: Some(GlyphEntry {
                uv_origin: [0.0, 0.0], uv_size: [0.0, 0.0], pixel_width: 5.0
            }),
            cell_height: CELL_HEIGHT,
            bitmap_rows: BITMAP_ROWS,
            space_width,
            char_spacing: CHAR_SPACING,
            tab_width: TAB_WIDTH,
            tab_origin: TAB_ORIGIN,
            darken_texture: None,
        }
    }

    #[test]
    fn text_width_uses_fnt_space_width() {
        let font = make_test_font(&[(b'a' as u16, 6), (b'b' as u16, 6)], 4);
        // "a b" = 6 + 4 + 6 + (3-1)*1 = 18
        assert_eq!(font.text_width("a b"), 18);
    }

    #[test]
    fn text_width_with_tab() {
        let font = make_test_font(&[(b'a' as u16, 6), (b'b' as u16, 6)], 4);
        // "a\tb" : a (6) → tab from x=6 → next 64 boundary = 64 → +b (6) + spacing(1, b is 2nd visible) = 71
        // Note: spacing is added at end based on count; tab does not count.
        assert_eq!(font.text_width("a\tb"), 64 + 6 + 1);
    }

    #[test]
    fn text_width_with_missing_glyph() {
        let font = make_test_font(&[(b'a' as u16, 6)], 4);
        // 'X' codepoint 0x58 not in table → missing_glyph (width 5)
        // "aX" = 6 + 5 + (2-1)*1 = 12
        assert_eq!(font.text_width("aX"), 12);
    }

    #[test]
    fn wrap_layout_breaks_at_last_space() {
        let font = make_test_font(&[(b'a' as u16, 6), (b'b' as u16, 6), (b'c' as u16, 6)], 4);
        // "ab c" with max_width 20: "ab" (6+6+1=13) + " " (4+1=5, total 18) +
        // "c" overflow at 18+1+6=25 > 20 → wrap at last space.
        // Line 1: "ab " (width up to space-position x = 13 + 1 + 4 = 18)
        // Line 2: "c" (width 6)
        let layout = font.wrap_layout("ab c", 20);
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.height, CELL_HEIGHT * 2);
    }

    #[test]
    fn wrap_layout_hard_cuts_no_space() {
        let font = make_test_font(&[(b'a' as u16, 6)], 4);
        // "aaaa" with max_width 14: "aa" (6+1+6=13 ok) "a" (+1+6 =20 > 14) → hard cut.
        let layout = font.wrap_layout("aaaa", 14);
        assert!(layout.lines.len() >= 2);
    }

    #[test]
    fn wrap_layout_single_char_overflow_accepted() {
        let font = make_test_font(&[(b'a' as u16, 20)], 4);
        // Single 'a' wider than max_width=10 → still draws on one line.
        let layout = font.wrap_layout("a", 10);
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].width, 20);
    }

    #[test]
    fn wrap_layout_crlf_one_newline() {
        let font = make_test_font(&[(b'a' as u16, 6), (b'b' as u16, 6)], 4);
        let layout = font.wrap_layout("a\r\nb", 1000);
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.height, CELL_HEIGHT * 2);
    }

    #[test]
    fn wrap_layout_bare_cr_advances() {
        let font = make_test_font(&[(b'a' as u16, 6), (b'b' as u16, 6)], 4);
        let layout = font.wrap_layout("a\rb", 1000);
        assert_eq!(layout.lines.len(), 2);
    }

    #[test]
    fn missing_color_xor_produces_large_shift_for_white() {
        // BITFONT §3.3: RGB565 0xFFFF ^ 0x5555 = 0xAAAA (red-orange).
        // Decomposed: R5 0x1F^0x0A = 0x15 (≈0.68), G6 0x3F^0x2A = 0x15 (≈0.33), B5 0x1F^0x15 = 0x0A (≈0.32).
        let xored = BitFont::missing_color_xor([1.0, 1.0, 1.0]);
        assert!((xored[0] - 21.0 / 31.0).abs() < 0.01, "R = {}", xored[0]);
        assert!((xored[1] - 21.0 / 63.0).abs() < 0.01, "G = {}", xored[1]);
        assert!((xored[2] - 10.0 / 31.0).abs() < 0.01, "B = {}", xored[2]);
    }

    #[test]
    fn missing_color_xor_produces_large_shift_for_black() {
        // 0x0000 ^ 0x5555 = 0x5555. R5 = 0x0A (10/31), G6 = 0x2A (42/63), B5 = 0x15 (21/31).
        let xored = BitFont::missing_color_xor([0.0, 0.0, 0.0]);
        assert!((xored[0] - 10.0 / 31.0).abs() < 0.01);
        assert!((xored[1] - 42.0 / 63.0).abs() < 0.01);
        assert!((xored[2] - 21.0 / 31.0).abs() < 0.01);
    }
}
```

**Step 2: Verify**
Run: `cargo test --lib bit_font`
Expected: PASS for all test cases.

**Step 3: Commit** — `render/bit_font: add unit tests for measure + wrap + xor`

---

### Task 9: Scaffold `shell_text.rs` with types

**Why:** Establish the public surface for Path A before implementing draw logic.

**Files:**
- Modify: `src/render/shell_text.rs`

**Step 1: Replace the placeholder with the full type scaffolding:**

```rust
//! Path A upper wrapper for shell controls (gamemd's `FUN_00621040` /
//! `FUN_00434CD0` equivalent). Bit-flag alignment, per-pixel scissor clip,
//! vertical center via measure-then-offset, per-line horizontal alignment,
//! `max_height` cutoff. Calls into `bit_font::BitFont` for glyph data and
//! wrap layout.

use crate::render::batch::SpriteInstance;
use crate::render::bit_font::{BitFont, LineSpan};

/// Alignment flag set for `draw_in_rect`. Matches gamemd's BITFONT §3.4/§3.5
/// bit-flag convention: 0x01 = h-center, 0x02 = h-right, 0x04 = v-center.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellAlign(pub u8);

impl ShellAlign {
    pub const NONE:     ShellAlign = ShellAlign(0);
    pub const H_CENTER: ShellAlign = ShellAlign(0x01);
    pub const H_RIGHT:  ShellAlign = ShellAlign(0x02);
    pub const V_CENTER: ShellAlign = ShellAlign(0x04);

    pub fn contains(self, flag: ShellAlign) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl std::ops::BitOr for ShellAlign {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self { ShellAlign(self.0 | rhs.0) }
}

/// Pixel-coordinate scissor rect. Apply via `wgpu::RenderPass::set_scissor_rect`.
#[derive(Copy, Clone, Debug, Default)]
pub struct ScissorRect {
    pub x: u32, pub y: u32, pub w: u32, pub h: u32,
}

/// Output of `draw_in_rect`: sprite instances plus the scissor the caller
/// must set on its render pass before drawing them.
pub struct ShellTextDraw {
    pub instances: Vec<SpriteInstance>,
    pub scissor: ScissorRect,
}

/// Pixel rect input to `draw_in_rect` — width/height in screen pixels.
#[derive(Copy, Clone, Debug)]
pub struct TextRect { pub x: i32, pub y: i32, pub w: u32, pub h: u32 }
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/shell_text: scaffold types`

---

### Task 10: Implement `shell_text::draw_in_rect`

**Why:** The Path A wrapper that the shell button-label rendering goes through. Measure → vcenter offset → per-line h-align → emit glyphs → cap at max_height.

**Files:**
- Modify: `src/render/shell_text.rs`

**Pattern:** Mirrors BITFONT §3.4 + §3.5 algorithm.

**Step 1: Add `draw_in_rect`:**

```rust
pub fn draw_in_rect(
    font: &BitFont,
    text: &str,
    rect: TextRect,
    color: [f32; 3],
    flags: ShellAlign,
    cam_offset: [f32; 2],
    depth: f32,
) -> ShellTextDraw {
    let scissor = ScissorRect {
        x: rect.x.max(0) as u32,
        y: rect.y.max(0) as u32,
        w: rect.w,
        h: rect.h,
    };
    if text.is_empty() {
        return ShellTextDraw { instances: Vec::new(), scissor };
    }
    let layout = font.wrap_layout(text, rect.w);
    let base_x = rect.x as f32;
    let mut line_y = rect.y as f32;
    if flags.contains(ShellAlign::V_CENTER) && layout.height < rect.h {
        line_y += ((rect.h - layout.height) / 2) as f32;
    }
    let line_advance = font.cell_height();

    let mut instances: Vec<SpriteInstance> = Vec::with_capacity(text.len());
    for span in &layout.lines {
        // max_height cutoff: stop if this line would exceed rect bottom.
        if (line_y + font.glyph_height()) > (rect.y as f32 + rect.h as f32) {
            break;
        }
        let line_x_offset = if flags.contains(ShellAlign::H_CENTER) && span.width < rect.w {
            ((rect.w - span.width) / 2) as f32
        } else if flags.contains(ShellAlign::H_RIGHT) && span.width < rect.w {
            (rect.w - span.width) as f32
        } else {
            0.0
        };
        let segment = &text[span.start_byte..span.end_byte];
        let mut line_instances = font.build_text(
            segment,
            base_x + line_x_offset,
            line_y,
            1.0,
            depth,
            color,
            cam_offset,
        );
        instances.append(&mut line_instances);
        line_y += line_advance;
    }
    ShellTextDraw { instances, scissor }
}
```

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/shell_text: implement draw_in_rect with vcenter/scissor/max-height`

---

### Task 11: Add `shell_text.rs` unit tests

**Why:** Lock in per-line alignment, vcenter offset, scissor passthrough, and max_height cutoff.

**Files:**
- Modify: `src/render/shell_text.rs`

**Step 1: Append test module:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::bit_font::tests::make_test_font;

    fn test_font() -> BitFont {
        make_test_font(&[(b'x' as u16, 6), (b'a' as u16, 6), (b'b' as u16, 6)], 4)
    }

    #[test]
    fn scissor_equals_rect() {
        let font = test_font();
        let draw = draw_in_rect(
            &font, "x",
            TextRect { x: 10, y: 20, w: 100, h: 30 },
            [1.0, 1.0, 1.0],
            ShellAlign::NONE,
            [0.0, 0.0],
            0.5,
        );
        assert_eq!(draw.scissor.x, 10);
        assert_eq!(draw.scissor.y, 20);
        assert_eq!(draw.scissor.w, 100);
        assert_eq!(draw.scissor.h, 30);
    }

    #[test]
    fn empty_text_returns_empty_instances() {
        let font = test_font();
        let draw = draw_in_rect(
            &font, "",
            TextRect { x: 0, y: 0, w: 100, h: 30 },
            [1.0, 1.0, 1.0],
            ShellAlign::V_CENTER | ShellAlign::H_CENTER,
            [0.0, 0.0],
            0.5,
        );
        assert!(draw.instances.is_empty());
    }

    #[test]
    fn align_combines_with_bitor() {
        let combined = ShellAlign::H_CENTER | ShellAlign::V_CENTER;
        assert!(combined.contains(ShellAlign::H_CENTER));
        assert!(combined.contains(ShellAlign::V_CENTER));
        assert!(!combined.contains(ShellAlign::H_RIGHT));
    }

    // vcenter_offsets_correctly and align_center_per_line tests defer to
    // the implementation choice in Task 8 for the test helper. Once that
    // helper is callable from shell_text::tests, add:
    //   - vcenter_offsets_correctly: build single-line text, V_CENTER, check
    //     first instance.position[1] == rect.y + (rect.h - cell_height)/2
    //   - align_center_per_line: build two-line wrapped input, H_CENTER, check
    //     each line's first instance.position[0] is centered per-line
}
```

**Note:** The pure-logic test helper from Task 8 needs to be accessible to `shell_text::tests`. Either:
- (a) Make `make_test_font` `pub(crate)` and place in a `bit_font::test_support` module, OR
- (b) Refactor `BitFont` data into a `BitFontData` sub-struct that has a public no-GPU constructor.

Pick (a) for minimum disruption. Add the visibility tweak as part of this task.

**Step 2: Verify**
Run: `cargo test --lib shell_text`
Expected: PASS.

**Step 3: Commit** — `render/shell_text: add unit tests for align/scissor/empty-text`

---

### Task 12: Rewrite `sidebar_text.rs` as free-function module

**Why:** Collapse the `SidebarTextRenderer` struct (its state migrated to `BitFont`) into free functions. Add `build_text_with_fade` (Path B fade math, BITFONT §3.7) and `side_highlight_color` (moved from `app_sidebar_text.rs`).

**Files:**
- Modify: `src/render/sidebar_text.rs` (full rewrite)

**Pattern:** Free-function module; no struct state. Side-highlight color values verified against BITFONT §3.7 verbatim.

**Step 1: Replace the entire file contents:**

```rust
//! Path B upper wrapper for sidebar text (gamemd's `FUN_006211D0` /
//! `FUN_00434500` equivalent). Single-line emission with optional
//! selected-unit fade (first N characters tinted from side-highlight color
//! toward the base text color). Side-color highlight table per BITFONT §3.7
//! (Allied / Soviet / Yuri).
//!
//! Most sidebar callers use the plain pass-through fns; only the Ready cameo
//! text needs `build_text_with_fade`.

use crate::render::batch::{BatchTexture, SpriteInstance};
use crate::render::bit_font::BitFont;
use crate::render::sidebar_chrome::SidebarTheme;

/// Side highlight colors used as fade endpoint for selected-unit text effect.
/// Values verified against BITFONT §3.7 / SIDEBAR_READY_TEXT_RENDERING.md.
const HIGHLIGHT_ALLIED: [f32; 3] = [164.0 / 255.0, 210.0 / 255.0, 1.0];
const HIGHLIGHT_SOVIET: [f32; 3] = [1.0, 1.0, 0.0];
const HIGHLIGHT_YURI:   [f32; 3] = [1.0, 1.0, 0.0];

pub fn side_highlight_color(theme: SidebarTheme) -> [f32; 3] {
    match theme {
        SidebarTheme::Allied => HIGHLIGHT_ALLIED,
        SidebarTheme::Soviet => HIGHLIGHT_SOVIET,
        SidebarTheme::Yuri   => HIGHLIGHT_YURI,
    }
}

// --- Plain pass-throughs preserved for existing single-color callers ---

pub fn text_width(font: &BitFont, text: &str) -> f32 {
    font.text_width(text) as f32
}
pub fn glyph_height(font: &BitFont) -> f32 { font.glyph_height() }
pub fn darken_texture(font: &BitFont) -> &BatchTexture { font.darken_texture() }
pub fn texture(font: &BitFont) -> &BatchTexture { font.atlas() }
pub fn build_text(
    font: &BitFont, text: &str, x: f32, y: f32, scale: f32, depth: f32,
    tint: [f32; 3], camera_offset: [f32; 2],
) -> Vec<SpriteInstance> {
    font.build_text(text, x, y, scale, depth, tint, camera_offset)
}

/// Selected-unit fade per BITFONT §3.7. First `fade_param` characters
/// (capped at 8) tint from `side_highlight` toward `base_color`; subsequent
/// characters use `base_color`. `fade_param == 0` ⇒ no fade (equivalent to
/// `build_text`).
pub fn build_text_with_fade(
    font: &BitFont,
    text: &str,
    x: f32, y: f32, scale: f32, depth: f32,
    base_color: [f32; 3],
    side_highlight: [f32; 3],
    fade_param: u32,
    camera_offset: [f32; 2],
) -> Vec<SpriteInstance> {
    if fade_param == 0 {
        return font.build_text(text, x, y, scale, depth, base_color, camera_offset);
    }
    let chars_to_fade = fade_param.min(8);
    let mut line_offset: u32 = (9u32.saturating_sub(fade_param)) * 0x1F;
    let mut out: Vec<SpriteInstance> = Vec::with_capacity(text.len());
    let mut cursor_x = x;
    let h = font.glyph_height() * scale;
    let spacing = scale; // CHAR_SPACING = 1
    let mut emitted = 0u32;

    for (char_idx, ch) in text.chars().enumerate() {
        if ch == '\r' || ch == '\n' { continue; }
        let tint = if (char_idx as u32) < chars_to_fade {
            // BITFONT §3.7: "fade from highlight color back to the normal text color".
            // line_offset starts small (=(9-fade_param)*0x1F) and grows by 0x1F per char,
            // so char 0 → small offset → mostly highlight; char 7 → large offset → mostly base.
            let t = (line_offset.min(255) as f32) / 255.0;
            lerp_rgb(side_highlight, base_color, t)
        } else {
            base_color
        };
        if char_idx < chars_to_fade as usize {
            line_offset = line_offset.saturating_add(0x1F);
        }
        // Glyph emission inlined here so per-char tint can apply.
        // For space: just advance.
        if ch == ' ' {
            if emitted > 0 { cursor_x += spacing; }
            cursor_x += font.text_width(" ") as f32 * scale;
            emitted += 1;
            continue;
        }
        // Reuse font.build_text for a single-char substring at cursor_x.
        let mut single = font.build_text(
            &ch.to_string(),
            cursor_x, y, scale, depth, tint, camera_offset,
        );
        if let Some(inst) = single.first() {
            let w = inst.size[0];
            if emitted > 0 {
                // Re-position with spacing applied.
                for s in &mut single { s.position[0] += spacing; }
                cursor_x += spacing;
            }
            cursor_x += w;
        }
        out.append(&mut single);
        emitted += 1;
    }
    out
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
```

**Note:** The per-char emission in `build_text_with_fade` calls `font.build_text` once per char to reuse glyph lookup. This is hot-path-acceptable for sidebar Ready text (≤ 8 chars per fade) but allocates per char. If this shows up in profiling, factor out a `BitFont::build_text_into(&mut Vec<SpriteInstance>, ...)` variant. Not needed for v1.

**Step 2: Verify**
Run: `cargo check`
Expected: PASS.

**Step 3: Commit** — `render/sidebar_text: rewrite as free-function module with fade math`

---

### Task 13: Add `sidebar_text.rs` unit tests

**Why:** Lock in fade math, fade boundary at 8 chars, and side-highlight color table.

**Files:**
- Modify: `src/render/sidebar_text.rs`

**Step 1: Append:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_highlight_table_matches_bitfont_doc() {
        assert_eq!(side_highlight_color(SidebarTheme::Allied), HIGHLIGHT_ALLIED);
        assert_eq!(side_highlight_color(SidebarTheme::Soviet), HIGHLIGHT_SOVIET);
        assert_eq!(side_highlight_color(SidebarTheme::Yuri),   HIGHLIGHT_YURI);
    }

    #[test]
    fn lerp_endpoints() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 1.0, 1.0];
        assert_eq!(lerp_rgb(a, b, 0.0), a);
        assert_eq!(lerp_rgb(a, b, 1.0), b);
        let mid = lerp_rgb(a, b, 0.5);
        assert!((mid[0] - 0.5).abs() < 1e-6);
    }

    // fade_only_first_8_chars / fade_lerp_direction integration tests:
    // require a BitFont — share test helper from bit_font::test_support (see Task 8).
    // Pseudocode:
    //   let font = bit_font::test_support::make_test_font(&[
    //       (b'a' as u16, 6), (b'b' as u16, 6), ... 'a'..'j' ...
    //   ], 4);
    //   let instances = build_text_with_fade(&font, "abcdefghij", 0.0, 0.0,
    //       1.0, 0.5, /*base=*/[0.0, 0.0, 0.0], /*highlight=*/[1.0, 1.0, 1.0],
    //       /*fade_param=*/8, [0.0, 0.0]);
    //   // BITFONT §3.7: first chars near highlight, last fade chars near base.
    //   // For fade_param=8: char 0 line_offset=31 (t≈0.12, tint ≈88% highlight)
    //   //                   char 7 line_offset=248 (t≈0.97, tint ≈97% base)
    //   assert!(instances[0].tint[0] > 0.8, "char 0 should be near highlight");
    //   assert!(instances[7].tint[0] < 0.2, "char 7 should be near base");
    //   // chars 8 and 9 are past fade band → pure base color.
    //   assert_eq!(instances[8].tint, [0.0, 0.0, 0.0]);
    //   assert_eq!(instances[9].tint, [0.0, 0.0, 0.0]);
}
```

**Step 2: Verify**
Run: `cargo test --lib sidebar_text`
Expected: PASS.

**Step 3: Commit** — `render/sidebar_text: add unit tests for fade + side colors`

---

### Task 14: Rename `AppState.sidebar_text` → `AppState.bit_font` + update construction

**Why:** Field rename, then re-point construction at the new `BitFont` constructor. Per CLAUDE.md no compatibility shim — change the field and migrate callers.

**Files:**
- Modify: `src/app.rs:50, 176, 683`
- Modify: `src/app_transitions.rs:113`

**Step 1: In `src/app.rs`:**

```diff
- use crate::render::sidebar_text::SidebarTextRenderer;
+ use crate::render::bit_font::BitFont;
```

```diff
-    pub(crate) sidebar_text: SidebarTextRenderer,
+    pub(crate) bit_font: BitFont,
```

```diff
-        let sidebar_text = SidebarTextRenderer::new(&gpu, &batch_renderer);
+        let bit_font = BitFont::fallback_5x7(&gpu, &batch_renderer);
```

Update the AppState struct literal to use `bit_font` field name.

**Step 2: In `src/app_transitions.rs:113`:**

```diff
-        state.sidebar_text = crate::render::sidebar_text::SidebarTextRenderer::from_fnt(
+        state.bit_font = crate::render::bit_font::BitFont::from_fnt(
             &state.gpu, &state.batch_renderer, &fnt,
         );
```

**Step 3: Verify**
Run: `cargo check`
Expected: compile errors at every `state.sidebar_text.X(...)` callsite (those are the next tasks). Field rename itself compiles cleanly in `app.rs` and `app_transitions.rs`.

**Step 4: Commit** — `app: rename sidebar_text field to bit_font, switch ctors`

---

### Task 15: Migrate `app_sidebar_build.rs` field-name updates (Ready stays single-color)

**Why:** Mechanical sweep of all `state.sidebar_text.X(...)` → `state.bit_font.X(...)`. Ready text continues to render via single-color `build_text` until Task 22 wires the fade. This keeps behavior identical to today while moving onto the new type.

**Files:**
- Modify: `src/app_sidebar_build.rs` (lines 519, 522, 524, 545, 548, 549, 579, 587, 605, 615-616, 625, 631-632)

**Step 1: Mechanical rename. For each line listed above, replace `state.sidebar_text` with `state.bit_font`. Method names (`text_width`, `glyph_height`, `darken_texture`, `build_text`) are preserved on `BitFont` — no signature changes.**

**Step 2: Add a comment at the Ready text emission site ([app_sidebar_build.rs:615-616](../../src/app_sidebar_build.rs)):**

```rust
// TODO(shell-text-parity): switch to sidebar_text::build_text_with_fade once
// fade_param value is verified — see docs/plans/2026-05-17-shell-text-renderer-parity-design.md
// "Open Follow-Ups" and Task 22 in the matching plan.
state.bit_font.build_text(ready_text, text_x, text_y, ts, 0.00042, ready_tint, co)
```

**Step 3: Verify**
Run: `cargo check`
Expected: compile errors only in files that still reference `state.sidebar_text` (the remaining call-site tasks).

**Step 4: Commit** — `app_sidebar_build: rename to bit_font, defer fade wire-up`

---

### Task 16: Migrate `app_render/draw_passes.rs` and `app_render/build_instances.rs`

**Why:** Field rename + import path swap for the side-color helper.

**Files:**
- Modify: `src/app_render/draw_passes.rs:451`
- Modify: `src/app_render/build_instances.rs:624`

**Step 1: In `src/app_render/draw_passes.rs:451`:**

```diff
-        Some(state.sidebar_text.texture()),
+        Some(state.bit_font.atlas()),
```

**Step 2: In `src/app_render/build_instances.rs:622-624`:**

```diff
     let ready_tint = {
         let theme = crate::app_sidebar_render::current_sidebar_theme(state);
-        crate::app_sidebar_text::ready_color_for_theme(theme)
+        crate::render::sidebar_text::side_highlight_color(theme)
     };
```

**Step 3: Verify**
Run: `cargo check`
Expected: errors remaining only in `app_skirmish_shell_render.rs` (next task) and the `app_sidebar_text.rs` dead-code (Task 18).

**Step 4: Commit** — `app_render: migrate texture access + side-color import`

---

### Task 17: Migrate `app_skirmish_shell_render.rs` to `shell_text::draw_in_rect` + scissor render loop

**Why:** Switch the shell button rendering to the full Path A wrapper with per-pixel scissor clip, vcenter via measure-then-offset, and per-line alignment. Start-marker labels stay on direct `bit_font.build_text` (no clip/align needed).

**Files:**
- Modify: `src/app_skirmish_shell_render.rs:450-590, 661-671`

**Step 1: Refactor `push_centered_text` to produce a `ShellTextDraw` instead of pushing `SpriteInstance`s directly. Update `build_shell_text_instances` to return `Vec<ShellTextDraw>`:**

```rust
use crate::render::shell_text::{self, ShellAlign, ShellTextDraw, TextRect};

fn shell_text_rect_for_button(rect: RectPx, y_offset: i32) -> TextRect {
    TextRect {
        x: rect.x,
        y: rect.y + y_offset,
        w: rect.w,
        h: rect.h,
    }
}

fn push_button_label_draw(
    out: &mut Vec<ShellTextDraw>,
    state: &AppState,
    label: &str,
    rect: RectPx,
    y_offset: i32,
    depth: f32,
) {
    let draw = shell_text::draw_in_rect(
        &state.bit_font,
        label,
        shell_text_rect_for_button(rect, y_offset),
        SHELL_BUTTON_TEXT_RGB_00000C05,
        ShellAlign::H_CENTER | ShellAlign::V_CENTER,
        [0.0, 0.0],
        depth,
    );
    out.push(draw);
}

fn build_shell_text_draws(
    state: &AppState,
    layout: &SkirmishShellLayout,
    shell: &SkirmishShellState,
) -> (Vec<ShellTextDraw>, Vec<SpriteInstance>) {
    let mut shell_draws: Vec<ShellTextDraw> = Vec::new();
    let mut bare_instances: Vec<SpriteInstance> = Vec::new();

    let start = localized_label(state, "GUI:StartGame", "Start Game");
    let choose = localized_label(state, "GUI:ChooseMap", "Choose Map");
    let back = localized_label(state, "GUI:Back", "Back");

    for (label, rect, button) in [
        (start.as_str(),  layout.start_button,      OwnerDrawButton::StartGame0x617),
        (choose.as_str(), layout.choose_map_button, OwnerDrawButton::ChooseMap0x5aa),
        (back.as_str(),   layout.back_button,       OwnerDrawButton::Back0x5c0),
    ] {
        let y_off = if shell.pressed_owner_draw_button == Some(button) {
            PRESSED_BUTTON_CONTENT_OFFSET_Y
        } else { 0 };
        push_button_label_draw(&mut shell_draws, state, label, rect, y_off, 0.00041);
    }
    // Start-marker labels — no clip/align, use bit_font directly.
    push_start_marker_labels(&mut bare_instances, state, layout.map_preview, &[], false, 0.00040);
    (shell_draws, bare_instances)
}
```

**Step 2: Update `render_skirmish_shell_with_atlas` to apply scissor per `ShellTextDraw`:**

```rust
let (shell_draws, bare_text_instances) = build_shell_text_draws(state, &layout, &state.skirmish_shell_state);
// ... existing chrome instance buffer creation ...

// existing chrome draw call
state.batch_renderer.draw_with_buffer_passthrough(&mut pass, &atlas.texture, &buffer, count);

// Bare-instance text (start markers, no scissor) — uses bit_font atlas
if let Some((buf, count)) = state.batch_renderer.create_instance_buffer(&state.gpu, &bare_text_instances) {
    state.batch_renderer.draw_with_buffer_passthrough(&mut pass, state.bit_font.atlas(), &buf, count);
}

// Per-rect scissored shell-text draws
for draw in &shell_draws {
    pass.set_scissor_rect(draw.scissor.x, draw.scissor.y, draw.scissor.w, draw.scissor.h);
    if let Some((buf, count)) = state.batch_renderer.create_instance_buffer(&state.gpu, &draw.instances) {
        state.batch_renderer.draw_with_buffer_passthrough(&mut pass, state.bit_font.atlas(), &buf, count);
    }
}
// Reset scissor to full render so subsequent passes/draws aren't clipped.
pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());
```

**Step 3: Update `push_start_marker_labels` to use `state.bit_font.build_text` and emit into `bare_text_instances`. Update the existing `push_centered_text` callers to use the new `push_button_label_draw` path (delete the old `push_centered_text` function and the `shell_text_origin` helper if no longer used).**

**Step 4: Verify**
Run: `cargo check`
Expected: PASS. Run `cargo test --lib` — pre-existing shell-render tests in `app_skirmish_shell_render.rs::tests` may reference the old `push_centered_text` or `shell_text_origin` helpers ([app_skirmish_shell_render.rs:799-821](../../src/app_skirmish_shell_render.rs)) — update those tests to call the new helper or remove them if they're now redundant with `shell_text` unit tests.

**Step 5: Commit** — `app_skirmish_shell_render: switch to shell_text::draw_in_rect with per-rect scissor`

---

### Task 18: Remove dead side-color helpers from `app_sidebar_text.rs`

**Why:** Single consumer (`build_instances.rs:624`) migrated in Task 16. The constants and helper are now dead.

**Files:**
- Modify: `src/app_sidebar_text.rs:21-33`

**Step 1: Delete the three `READY_COLOR_*` constants and the `ready_color_for_theme` function. Leave the rest of the file (egui credits label) intact.**

**Step 2: Verify**
Run: `cargo check && cargo build`
Expected: PASS. No remaining references to `app_sidebar_text::ready_color_for_theme` (verify with `grep -rn`).

**Step 3: Commit** — `app_sidebar_text: drop dead side-color helpers`

---

### Task 19: Run `cargo check`, `cargo test`, fix issues

**Why:** Final compile + test pass to catch any missed call sites or broken existing tests.

**Files:** any flagged by the build.

**Step 1: Run `cargo check --all-targets`**
Expected: PASS.

**Step 2: Run `cargo test --lib`**
Expected: all new tests pass; existing tests pass unchanged. Snapshot tests for shell render or sidebar may shift by ≤ 1 px due to the space-glyph fix — accept new baselines if so.

**Step 3: Run `grep -rn "state.sidebar_text" src/` — should return zero hits.** Also `grep -rn "SidebarTextRenderer" src/` — zero hits. Also `grep -rn "ready_color_for_theme" src/` — zero hits.

**Step 4: Commit** — `render/text: fix fallout from refactor` (only if anything beyond mechanical updates was needed)

---

### Task 20: Run `cargo clippy` on touched files

**Why:** Address any new lints introduced by the refactor.

**Step 1:** Run `cargo clippy --lib -- -D warnings`.

**Step 2:** Address any warnings in `bit_font.rs`, `shell_text.rs`, `sidebar_text.rs`, `app.rs`, `app_skirmish_shell_render.rs`, `app_sidebar_build.rs`, `app_render/draw_passes.rs`, `app_render/build_instances.rs`, `app_transitions.rs`, `app_sidebar_text.rs`.

**Step 3: Commit** — `render/text: clippy clean` (if changes made)

---

### Task 21: In-game visual verification on Skirmish shell and sidebar

**Why:** The parity bar is observable output. Compile-clean isn't enough — verify the actual rendered result matches gamemd.exe.

**Verify:**

1. **Skirmish shell button text** — set `RA2_DEV_SKIRMISH_SHELL=1`, launch:
   - "Start Game", "Choose Map", "Back" labels render centered horizontally and vertically in their button rects
   - Pressed state: text shifts by `PRESSED_BUTTON_CONTENT_OFFSET_Y` pixels (existing behavior preserved)
   - No regression: chrome/buttons render identically to before

2. **Space-width fix** — eyeball "Choose Map" kerning. The gap between "Choose" and "Map" should look natural (real FNT space ≈ 4-5px), not loose (`glyph_h × 0.4 ≈ 6.4px` previously).

3. **Scissor clip** — temporarily inject a deliberately oversized label (e.g., "Start Game ThisIsExtraToOverflow") via `localized_label` fallback string change. Verify the overflow is pixel-clipped at the button-rect right edge, not whole-character-dropped or rendered outside the button.

4. **In-game sidebar Ready text** — start a skirmish, build any unit:
   - "Ready" text on the cameo renders in the side-tinted color (Allied light-blue / Soviet+Yuri yellow) — unchanged from today
   - Queue count badge renders with darken-strip overlay — unchanged
   - Side-color helper lookup works through the new `render::sidebar_text::side_highlight_color` path
   - **Known drift**: Ready text does NOT yet animate the fade pulse (Task 22 pending). Accept as expected.

5. **In-game sidebar credits and other UI** — egui credits label still renders (separate path, untouched).

Document any drift in this task's commit message. If anything is broken, fix before declaring done.

**Step 1: Commit** — `render/text: visual verification on shell + sidebar (no regressions)` (or specify what was fixed)

---

### Task 22: (DEFERRED — post-Ghidra) Wire sidebar Ready cameo to fade path

**Why:** Final closure of the Path B fade parity hole. Blocked on a ~30-minute Ghidra read to extract the `fade_param` integer value passed by `SidebarClass__DrawCameoText` to `FUN_006211D0` (via `FUN_004A5EB0` / `FUN_004A60E0`).

**Prerequisite:** Targeted Ghidra MCP session — decompile the sidebar paint caller chain reaching `FUN_006211D0`, extract the integer value of the `fade_param` argument (`param_10` in `FUN_006211D0`'s signature per BITFONT §3.6). Record the value in [BITFONT_SHELL_TEXT_GHIDRA_REPORT.md §3.7](../../../ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md) as a verified addition.

**Files:**
- Modify: `src/app_sidebar_build.rs:615-616` (Ready text emission site; the TODO planted in Task 15)

**Step 1: Add a const for the verified value:**

```rust
// In app_sidebar_build.rs or render/sidebar_text.rs, near the call site:
/// Value of the `fade_param` argument that gamemd's SidebarClass__DrawCameoText
/// passes to FUN_006211D0 → FUN_00434500. Verified from Ghidra <addr>.
const READY_FADE_PARAM: u32 = /* value from Ghidra */;
```

**Step 2: Swap the call:**

```diff
-            // TODO(shell-text-parity): switch to build_text_with_fade once fade_param is verified
-            state.bit_font.build_text(ready_text, text_x, text_y, ts, 0.00042, ready_tint, co)
+            {
+                let theme = crate::app_sidebar_render::current_sidebar_theme(state);
+                let highlight = crate::render::sidebar_text::side_highlight_color(theme);
+                let base = /* the non-tinted base color for Ready text */;
+                crate::render::sidebar_text::build_text_with_fade(
+                    &state.bit_font, ready_text, text_x, text_y, ts, 0.00042,
+                    base, highlight, READY_FADE_PARAM, co,
+                )
+            }
```

Adjust `base` to whatever the post-fade resting color should be — likely the existing `ready_tint` value, or a verified base color from the gamemd caller. The Ghidra read should reveal both `fade_param` AND the base color passed as the `color` argument; capture both.

**Step 3: Verify**

- `cargo test --lib sidebar_text` still passes (fade math tests).
- In-game: build any unit, watch the Ready text animate. First ≈ 8 chars (so "Ready " then padding) transition from side-color toward base color over the fade. Side-by-side against gamemd should be visually identical pulse cadence.

**Step 4: Commit** — `render/sidebar_text: wire selected-unit fade with verified fade_param`

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-17-shell-text-renderer-parity-design.md](2026-05-17-shell-text-renderer-parity-design.md)
- **Ghidra reports:**
  - [ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md) — primary, all algorithm shapes verified HIGH
  - [ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md](../../../ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md) — side-color highlight table
- **Investigation plan:** [docs/plans/2026-05-17-bitfont-shell-text-investigation-plan.md](2026-05-17-bitfont-shell-text-investigation-plan.md)
- **gamemd.exe addresses** (kept here, not in Rust code comments):
  - `FUN_00621040` — Path A wrapper (ShellText__DrawInRect)
  - `FUN_006211D0` — Path B wrapper (ShellText__DrawWithAlign)
  - `FUN_00434CD0` — DrawWithWrap (full Path A draw)
  - `FUN_00434500` — DrawLineWithFade (Path B single-line + selected-unit fade)
  - `FUN_00434120` — DrawGlyph
  - `FUN_00434700` — BuildFallbackGlyph (inverted '°')
  - `FUN_00433CF0` — MeasureText
  - `0x004A60E0` — SidebarClass__DrawText (Task 22 caller chain root)
- **INI keys:** none — BitFont/BitText has no INI surface (BITFONT §4)
- **Related code:**
  - [src/assets/fnt_file.rs](../../src/assets/fnt_file.rs) — FNT parser (unchanged by this plan)
  - [src/render/sidebar_text.rs](../../src/render/sidebar_text.rs) — current renderer being refactored
  - [src/render/batch.rs:1364](../../src/render/batch.rs) — `draw_with_buffer_passthrough` (caller-side scissor)
  - [src/render/sidebar_chrome.rs:35](../../src/render/sidebar_chrome.rs) — `SidebarTheme` enum
