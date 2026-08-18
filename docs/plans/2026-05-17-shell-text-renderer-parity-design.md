# Shell Text Renderer Parity Design

## Goal

Close the 8 player-visible parity gaps in our BitFont-equivalent text rendering — covering both Skirmish shell controls (FUN_00621040 path) and the in-game sidebar Ready cameo (FUN_006211D0 / FUN_00434500 path) — by splitting the current monolithic `SidebarTextRenderer` into a shared lower-layer `BitFont` plus two upper-layer wrappers that mirror gamemd.exe's natural class hierarchy in observable terms.

## Architecture Context

### How shell + sidebar text is rendered today

A single `SidebarTextRenderer` at [src/render/sidebar_text.rs](../../src/render/sidebar_text.rs) holds:

- A GPU sprite atlas built once at startup from a parsed `FntFile`, packing codepoints `0x20..0x180` into one 512px-wide RGBA texture (multi-page support exists in the renderer infrastructure but is not used here).
- A `HashMap<char, GlyphEntry>` of UV rects + pixel widths.
- A 1×1 RGBA(0,0,0,175) "darken" texture for cameo strip overlays.
- A hardcoded 5×7 fallback path (`SidebarTextRenderer::new`) used when GAME.FNT fails to load.

Public API:
- `text_width(text) -> f32`
- `glyph_height() -> f32`
- `darken_texture() -> Option<&BatchTexture>`
- `texture() -> &BatchTexture`
- `build_text(text, x, y, scale, depth, tint, cam_offset) -> Vec<SpriteInstance>`

It is held on `AppState.sidebar_text` and built in `app.rs:683` / promoted to the FNT-backed version in `app_transitions.rs:113`.

### Callers

| Caller | Path equivalent | What it needs |
|---|---|---|
| [app_skirmish_shell_render.rs:461-478](../../src/app_skirmish_shell_render.rs) `push_centered_text` | Path A (FUN_00621040) | Bit-flag align (h-center, v-center), per-pixel clip, wrap, missing-glyph fallback |
| [app_skirmish_shell_render.rs:558-567](../../src/app_skirmish_shell_render.rs) start-marker labels | Lower layer (raw build_text) | Just "draw '1' at (x,y)" — no clip, no align |
| [app_sidebar_build.rs:519-632](../../src/app_sidebar_build.rs) Ready cameo + queue badge | Path B (FUN_00434500) | Selected-unit fade (first 8 chars, gradient toward side-highlight color), darken strip |
| [app_render/draw_passes.rs](../../src/app_render/draw_passes.rs) | Chrome composition only | Atlas + texture reference |

### Render-pass integration

Sprite instances are batched into a `wgpu::Buffer` and drawn via `BatchRenderer::draw_with_buffer_passthrough` ([src/render/batch.rs:1364](../../src/render/batch.rs)). The render pass owner can call `wgpu::RenderPass::set_scissor_rect()` before any draw call — no new `BatchRenderer` API is required to support per-pixel clip.

### Source-of-truth research

- [ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md) — verified RE report (HIGH confidence on all algorithm shapes, struct layouts, and address bindings) covering FUN_00621040, FUN_006211D0, FUN_00434500, FUN_00434CD0, FUN_00434120, FUN_00434700.
- [ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md](../../../ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md) — side-color highlight table (Allied / Soviet / Yuri) used as fade endpoint.
- [docs/plans/2026-05-17-bitfont-shell-text-investigation-plan.md](2026-05-17-bitfont-shell-text-investigation-plan.md) — the scoping plan that produced the BITFONT report.

## Impact Analysis

### Files touched

| File | Change |
|---|---|
| [src/render/bit_font.rs](../../src/render/bit_font.rs) | NEW — lower layer (atlas, glyph table, measurement, wrap state machine, missing-glyph, tab math, darken texture) |
| [src/render/shell_text.rs](../../src/render/shell_text.rs) | NEW — Path A upper wrapper (bit-flag align, vcenter, multi-line, scissor clip) |
| [src/render/sidebar_text.rs](../../src/render/sidebar_text.rs) | REFACTORED — struct → free-function module; Path B upper wrapper (selected-unit fade); thin pass-throughs for existing single-color usage |
| [src/render/mod.rs](../../src/render/mod.rs) | Add `pub mod bit_font;` and `pub mod shell_text;` |
| [src/lib.rs](../../src/lib.rs) | No change expected (already re-exports through render::) |
| [src/app.rs](../../src/app.rs) | Rename `sidebar_text: SidebarTextRenderer` field → `bit_font: BitFont`; update construction at line 683 |
| [src/app_transitions.rs](../../src/app_transitions.rs) | Update construction at line 113 (FNT-backed builder) |
| [src/app_skirmish_shell_render.rs](../../src/app_skirmish_shell_render.rs) | `push_centered_text` → `shell_text::draw_in_rect`; start-marker labels → `state.bit_font.build_text`; render pass loop adds `set_scissor_rect` per ShellTextDraw |
| [src/app_sidebar_build.rs](../../src/app_sidebar_build.rs) | `state.sidebar_text.X(...)` → `state.bit_font.X(...)` for plain rendering; Ready-text call switches to `sidebar_text::build_text_with_fade` with the side-highlight color + fade_param |
| [src/app_sidebar_text.rs](../../src/app_sidebar_text.rs) | Move `READY_COLOR_*` constants + `ready_color_for_theme` into `render::sidebar_text::side_highlight_color` (this file otherwise stays — it owns the egui credits label, a different concern) |
| [src/app_render/draw_passes.rs](../../src/app_render/draw_passes.rs) | Field rename only (texture reference) |
| [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) | Field rename + side-highlight import path update |

### Risk areas

- **Mechanical field rename** (`state.sidebar_text` → `state.bit_font`) touches six files. No semantic change, but careful diff review needed.
- **Sidebar Ready callers currently pass a single color** ([app_sidebar_build.rs:616, 632](../../src/app_sidebar_build.rs)). The Ready path migrates to `build_text_with_fade` with both base color and side-highlight; the queue-count badge stays on single-color since gamemd does not fade queue counts.
- **5×7 fallback path** preserved as `BitFont::fallback_5x7()` — debug-only defensive path; not extended with the new features (no parity benefit when the real font is missing).
- Not a `sim/` change → no tick ordering / determinism / state-hash concerns.

## Chosen Approach

Two-layer split that mirrors gamemd's BitFont + per-path-wrapper hierarchy in observable terms while keeping each file under the CLAUDE.md ~600-line target.

**Lower `bit_font.rs`** owns everything that's shared between call paths (atlas, glyph measurement, missing-glyph fallback, tab math, CRLF normalization, wrap state machine, darken texture). **Upper `shell_text.rs`** carries Path A's distinctive details (bit-flag alignment, per-line align, vcenter, scissor clip, max_height cutoff). **Upper `sidebar_text.rs`** carries Path B's distinctive details (selected-unit fade math, side-highlight color table) and provides thin pass-through fns for the simple single-color sidebar usages.

### Why this over the alternatives

- **Extending sidebar_text.rs in place** would push it past 800 lines and mix Path A bit-flag alignment with Path B integer-mode alignment in one impl — god-class anti-pattern, alignment-flag aliasing risk.
- **Forking shell_text.rs and leaving sidebar untouched** would duplicate atlas building, glyph measurement, and missing-glyph fallback across two files — latent drift source, and Path B's fade still needs a second pass later.
- The two-layer split keeps the load-bearing parity code (lower layer) in one place exercised by both paths, while letting each upper wrapper own its distinct semantics.

## Tiny-Detail Ledger

Constraint set carried through to `/write-plan` and implementation. Each item is sourced; nothing invented.

### Shared lower layer (BitFont equivalent)

- 1px inter-character spacing, hardcoded after each glyph — **not** an FNT field. `[doc: BITFONT §3.2, §3.3]`
- Cell height = 17 px (bitmap_rows 16 + 1px line gap). `[doc: BITFONT §2 inner +0x0C / file 0x10]`
- Glyph bitmap: 16 rows × 3 bytes, 1bpp, MSB = leftmost pixel. `[doc: BITFONT §2, §3.3]`
- Space character has its own glyph in GAME.FNT with measured width; the current `glyph_h × 0.4` fudge in [sidebar_text.rs:217](../../src/render/sidebar_text.rs) is a parity defect that must be fixed. `[doc: BITFONT §3.2 — space FALLTHROUGH to default glyph path]`
- Missing-glyph fallback: at font load, build a 50-byte buffer = `~glyph(0xB0)` byte-inverted; rendered as a single glyph entry. `[doc: BITFONT §3.1 FUN_00434700]`
- Missing-glyph color XOR 0x5555: in RGB565 this is odd-bit-position swap on R/G/B. Translated to 32-bit RGBA → pre-compute once: `r ^= 0x08, g ^= 0x14, b ^= 0x08` (approximate; visual intent is a dithered/desaturated tint distinguishing missing glyphs at a glance). `[doc: BITFONT §3.3]`
- Tab handling: `x += tab_width; x -= (x - tab_origin) % tab_width;` with default `tab_width=64`, `tab_origin=0`. `[doc: BITFONT §2 outer +0x20/+0x28, §3.3 case '\t']`
- CRLF normalization: `\r\n` pair = single newline; bare `\n` or `\r` each advance one line; `prev_char` tracked across iterations. `[doc: BITFONT §3.2 case '\n' / '\r']`
- Word-wrap algorithm (measure and layout share one state machine):
  - Track `last_space_pos` + `last_space_x` while iterating.
  - Overflow + space-on-line: line ends at `last_space_x`, next line starts at char after space.
  - Overflow + no space + `chars_on_line > 1`: hard-cut before overflowing char, retry on next line, line width = pre-overflow x.
  - Overflow + `chars_on_line <= 1`: accept the overflow (single char wider than max_width). `[doc: BITFONT §3.2]`
- Per-pixel clip: clip rect set on font state; per-pixel bounds check on the source side. **GPU translation**: `RenderPass::set_scissor_rect()` matching the rect = 1px-accurate, no per-fragment overhead. `[doc: BITFONT §3.3 outer +0x30..+0x3F]`
- Total measured height = `cell_height × number_of_lines` (no extra gap after last line). `[doc: BITFONT §3.2]`

### Path A upper (shell controls — FUN_00621040 / FUN_00434CD0)

- Alignment flags are BITS: `0x01` h-center, `0x02` h-right, `0x04` v-center (NOT integer modes). `[doc: BITFONT §3.4, §3.5]`
- V-center: measure first with `max_width = rect.w` → `y += (rect.h - measured_h) / 2`. Computed BEFORE the per-line draw. `[doc: BITFONT §3.5]`
- H-center / h-right: applied PER-LINE: `line_x = base_x + (max_width - line_width) / 2` or `+ (max_width - line_width)`. `[doc: BITFONT §3.4]`
- `max_height` cutoff: stop drawing as soon as `line_y >= max_height`. `[doc: BITFONT §3.4]`
- No fade for shell controls — `FUN_00621040` always passes `fade_count=0, fade_range=0`. `shell_text` does not even accept a fade parameter. `[doc: BITFONT §3.5]`
- Clip rect = the input rect (no separate clip-vs-draw distinction). `[doc: BITFONT §3.5]`

### Path B upper (sidebar Ready cameo — FUN_006211D0 / FUN_00434500)

- Alignment is INTEGER MODES (1=center, 2=anchor-from-mid, 3=right-1, 4=right-edge-1) — different convention from Path A. `[doc: BITFONT §3.6]`
- Single-line only (sidebar text never wraps). `[doc: BITFONT §3.6 FUN_00434B90 → FUN_00434500]`
- Selected-unit fade: first 8 characters tint from `g_SelectedUnitHighlightColor` toward saved base color.
  - `line_offset = (9 - fade_param) * 0x1F`
  - Per char: `tint = lerp(base, highlight, line_offset / 255)`; then `line_offset += 0x1F`. `[doc: BITFONT §3.7]`
- Side highlight color: Allied `RGB(164, 210, 255)`, Soviet `RGB(255, 255, 0)`, Yuri `RGB(255, 255, 0)`. Three constants currently in [app_sidebar_text.rs:21-23](../../src/app_sidebar_text.rs) — moved to `render::sidebar_text::side_highlight_color`. `[doc: SIDEBAR_READY_TEXT_RENDERING.md, cited in BITFONT §3.7]`
- **`fade_param` numeric value used by `SidebarClass__DrawCameoText`** — `[UNKNOWN — needs targeted Ghidra read on the caller chain reaching 0x004A60E0 / FUN_006211D0 from the sidebar paint path before Path B fade implementation can finish]`. Blocks the fade step, not the broader refactor.

### Implementation-side notes

- `BatchRenderer::draw_with_buffer_passthrough` does not currently accept a scissor rect; the caller sets it on the `RenderPass` before invoking the draw. Verified at [src/render/batch.rs:1364](../../src/render/batch.rs).

## Design

### Components

```
src/render/
├── bit_font.rs       (NEW, ~250 lines)
│     pub struct BitFont {
│         atlas_texture: BatchTexture,
│         glyphs: HashMap<u16, GlyphEntry>,
│         missing_glyph: GlyphEntry,         // built once at load
│         cell_height: u32,                  // 17 from FNT field 3
│         bitmap_rows: u32,                  // 16 from FNT field 2
│         space_width: u32,                  // from FNT glyph(0x20)
│         tab_width: u32,                    // 64 (gamemd default)
│         tab_origin: u32,                   // 0 (gamemd default)
│         char_spacing: u32,                 // 1 (hardcoded in gamemd)
│         darken_texture: BatchTexture,      // 1×1 RGBA(0,0,0,175)
│     }
│     impl BitFont {
│         pub fn from_fnt(&GpuContext, &BatchRenderer, &FntFile) -> Self;
│         pub fn fallback_5x7(&GpuContext, &BatchRenderer) -> Self;
│         pub fn atlas(&self) -> &BatchTexture;
│         pub fn darken_texture(&self) -> &BatchTexture;
│         pub fn glyph_height(&self) -> f32;        // = bitmap_rows = 16
│         pub fn cell_height(&self) -> f32;         // = 17 (line advance)
│         pub fn text_width(&self, &str) -> u32;
│         pub fn wrap_layout(&self, &str, max_width: u32) -> WrapLayout;
│         pub fn build_text(&self, &str, x, y, scale, depth, tint, cam_offset)
│                                                     -> Vec<SpriteInstance>;
│         pub fn missing_color_xor(rgb: [f32; 3]) -> [f32; 3];
│     }
│     pub struct WrapLayout {
│         pub width: u32, pub height: u32, pub lines: Vec<LineSpan>
│     }
│     pub struct LineSpan {
│         pub start_byte: usize, pub end_byte: usize, pub width: u32
│     }
│
├── shell_text.rs     (NEW, ~250 lines)
│     bitflags! pub struct ShellAlign: u8 {
│         const NONE = 0;
│         const H_CENTER = 0x01;
│         const H_RIGHT  = 0x02;
│         const V_CENTER = 0x04;
│     }
│     pub struct ShellTextDraw {
│         pub instances: Vec<SpriteInstance>,
│         pub scissor: ScissorRect,
│     }
│     pub struct ScissorRect { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }
│     pub fn draw_in_rect(
│         font: &BitFont, text: &str, rect: RectPx,
│         color: [f32; 3], flags: ShellAlign,
│         cam_offset: [f32; 2], depth: f32,
│     ) -> ShellTextDraw;
│
└── sidebar_text.rs   (REFACTORED, ~200 lines)
      // Free-function module — no struct state (everything lives in BitFont).
      pub fn text_width(font: &BitFont, text: &str) -> f32;
      pub fn glyph_height(font: &BitFont) -> f32;
      pub fn darken_texture(font: &BitFont) -> &BatchTexture;
      pub fn texture(font: &BitFont) -> &BatchTexture;
      pub fn build_text(font: &BitFont, text, x, y, scale, depth, tint, cam)
                                                     -> Vec<SpriteInstance>;
      pub fn build_text_with_fade(
          font: &BitFont, text: &str, x: f32, y: f32, scale: f32, depth: f32,
          base_color: [f32; 3], side_highlight: [f32; 3], fade_param: u32,
          cam: [f32; 2],
      ) -> Vec<SpriteInstance>;
      pub fn side_highlight_color(theme: SidebarTheme) -> [f32; 3];
```

### Interfaces / Contracts

**`BitFont::build_text` semantics** — emits one `SpriteInstance` per visible glyph including space (zero-pixel-wide quad that just advances cursor by `space_width + char_spacing`). Missing-glyph codepoints emit the inverted-'°' quad with `tint = missing_color_xor(input_tint)`. Tab characters do not emit a quad; they only advance the cursor per tab math. Newlines do not emit a quad; multi-line emission is the caller's job via `wrap_layout` for shell, or "single line only" for sidebar.

**`shell_text::draw_in_rect` semantics** — computes a `WrapLayout` using `font.wrap_layout(text, rect.w)`, applies `V_CENTER` offset to the base y, then for each `LineSpan` applies per-line h-alignment and emits glyphs via `font.build_text` (substring + offset). Stops emitting once `line_y >= rect.bottom`. Returns the scissor rect equal to the input rect.

**`sidebar_text::build_text_with_fade` semantics** — single-line layout via `font.text_width` and direct cursor advancement. For each character index `i < min(8, fade_param)`, computes per-char tint via the BITFONT §3.7 lerp and emits a quad. Characters at `i >= fade_param` use base color.

### Data Flow

#### Path A — shell button text

```rust
// app_skirmish_shell_render.rs::build_shell_text_instances
fn build_shell_text_draws(state, layout, shell_state) -> Vec<ShellTextDraw> {
    let mut draws = Vec::new();
    for (label, rect, pressed_offset) in [
        (start_label,  layout.start_button,      ...),
        (choose_label, layout.choose_map_button, ...),
        (back_label,   layout.back_button,       ...),
    ] {
        draws.push(shell_text::draw_in_rect(
            &state.bit_font, &label,
            rect.shifted_y(pressed_offset),
            SHELL_BUTTON_TEXT_RGB_00000C05,
            ShellAlign::H_CENTER | ShellAlign::V_CENTER,
            [0.0, 0.0],
            BUTTON_TEXT_DEPTH,
        ));
    }
    draws
}

// render_skirmish_shell_with_atlas
for draw in &shell_draws {
    pass.set_scissor_rect(draw.scissor.x, draw.scissor.y,
                          draw.scissor.w, draw.scissor.h);
    let Some((buf, count)) = state.batch_renderer
        .create_instance_buffer(&state.gpu, &draw.instances) else { continue };
    state.batch_renderer.draw_with_buffer_passthrough(
        &mut pass, state.bit_font.atlas(), &buf, count);
}
pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());
```

Start-marker labels stay on `state.bit_font.build_text(...)` — no rect, no clip, no align.

#### Path B — sidebar Ready cameo

```rust
// app_sidebar_build.rs (Ready text site)
let highlight = sidebar_text::side_highlight_color(theme);
let fade_param = READY_FADE_PARAM;   // ← UNKNOWN value, ledger blocker
let instances = sidebar_text::build_text_with_fade(
    &state.bit_font, ready_text, text_x, text_y, ts, depth,
    base_color, highlight, fade_param, cam_offset,
);
// instances batched into existing sidebar render pass; no scissor change
```

Queue-count badge stays on `state.bit_font.build_text(...)` — gamemd does not fade queue counts.

### Error Handling

- `BitFont::from_fnt` returns `BitFont` directly (already infallible — the `FntFile` is validated). Atlas allocation failures bubble through `BatchRenderer::create_texture` which already handles GPU errors.
- Missing glyph at runtime → always falls back to inverted-'°' (constructed at load; never absent). No silent drops, no panics.
- Empty text → `WrapLayout { width: 0, height: 0, lines: vec![] }`; `build_text` returns empty `Vec`.
- 5×7 fallback (`BitFont::fallback_5x7`) used only when `from_fnt` is never called (GAME.FNT load failure path in `app_transitions.rs:113`).

### Testing Strategy

**`bit_font.rs`** unit tests (lifted patterns from existing `fnt_file.rs` tests):

- `text_width_uses_fnt_space_width` — "Hello World" measures via actual FNT space glyph, not `glyph_h × 0.4`.
- `text_width_with_tab` — `"a\tb"` advances to next 64-px boundary after 'a'.
- `text_width_with_missing_glyph` — codepoint outside the packed range still contributes the missing-glyph width.
- `wrap_layout_breaks_at_last_space` — known input with spaces → expected `LineSpan` boundaries.
- `wrap_layout_hard_cuts_no_space` — single overflowing word → retry-on-next-line backtrack.
- `wrap_layout_crlf_one_newline` — `"a\r\nb"` produces 2 lines (not 3).
- `wrap_layout_bare_cr_advances` — `"a\rb"` produces 2 lines.
- `wrap_layout_single_char_overflow_accepted` — a single char wider than max_width still draws.
- `missing_color_xor_round_trip` — sentinel correctness against known RGB input.

**`shell_text.rs`** unit tests:

- `vcenter_offsets_correctly` — known `rect.h` + measured `WrapLayout.height` → expected y offset.
- `align_center_per_line` — multi-line input with `H_CENTER` → each line centered independently.
- `align_right_per_line` — same for `H_RIGHT`.
- `scissor_equals_rect` — output `ScissorRect` matches input `RectPx`.
- `max_height_stops_drawing` — given a small `rect.h`, only the lines that fit are emitted.

**`sidebar_text.rs`** unit tests:

- `fade_only_first_8_chars` — character 9 and beyond use base color.
- `fade_lerp_endpoints` — `fade_param=1` → char 0 close to highlight, char 7 close to base.
- `side_highlight_table` — Allied / Soviet / Yuri tints match BITFONT §3.7 verbatim.

**Integration**: existing snapshot tests for shell render and sidebar render should pass unchanged once call sites are migrated. The space-glyph-from-FNT fix may shift Ready/queue-count layout by ≤ 1 px — accept the new baseline since the old behavior was the parity defect.

### Determinism considerations

Not a `sim/` change. Renderer-only. No tick ordering, no state hashing, no replay implications.

## Architectural Decisions

### Patterns followed

- **Low-level vs upper-layer split** mirrors the existing `assets/` (parsers) vs `app_*` (orchestration) convention and matches gamemd's `BitFont` + per-path-wrapper hierarchy in observable terms.
- **Free-function modules for stateless wrappers** — `shell_text` and `sidebar_text` post-refactor hold no state, so the function-on-module form is more honest than `struct + impl<&Self>`. The pattern is already used elsewhere in the codebase for pure transformations.
- **All measurement state on `BitFont`** — atlas, glyph table, darken texture, font defaults. Single source of truth for layout.

### Patterns deviated

- **`SidebarTextRenderer` struct is deleted, not preserved as a shim.** Per CLAUDE.md "no backwards-compatibility hacks", call sites rename `state.sidebar_text.X(...)` → `state.bit_font.X(...)` mechanically. Six files; small diff, no behavioral change beyond the parity fixes.
- **Side-color highlight table moves from `app_sidebar_text.rs` → `render::sidebar_text`.** The egui credits label stays on `app_sidebar_text.rs` since it is a separate (egui-based) concern.

### Tech debt introduced

- The 5×7 hardcoded fallback path (`BitFont::fallback_5x7`) is preserved but not extended with the new features (clip, wrap, missing-glyph, fade). It is a defensive debug-only path used only when GAME.FNT fails to load — extending it would be effort for no parity benefit. Documented in the module header.

## Alternatives Considered

### Approach 1 — Extend `sidebar_text.rs` in place

Grow the existing `SidebarTextRenderer` to ~800-1000 lines containing all 8 features. **Rejected** because:

- Violates the CLAUDE.md ~600-line file target.
- Mixes Path A bit-flag alignment (`0x01`/`0x02`/`0x04`) with Path B integer-mode alignment (`1`/`2`/`3`/`4`) in one impl — alignment-flag aliasing risk, god-class anti-pattern.
- Single source of truth for lower-layer code is a benefit, but the same benefit is available with the two-layer split without the size/coupling cost.

### Approach 2 — Fork a new `shell_text.rs`, leave `sidebar_text.rs` untouched

Add `shell_text.rs` implementing Path A; leave `sidebar_text.rs` as-is; add Path B fade in a separate later pass. **Rejected** because:

- Duplicates atlas building, glyph measurement, space-glyph handling, and missing-glyph fallback across two files — exactly the "identical implementations" surface area where drift creeps in over time.
- Sidebar Ready fade (a current player-visible parity drift) gets deferred to a second design pass that duplicates the missing-glyph work.
- "Hidden coupling" anti-pattern: an FNT format evolution requires editing two files in lockstep.

## Open Follow-Ups

- **`fade_param` numeric value used by `SidebarClass__DrawCameoText`** — needs a targeted Ghidra read on the caller chain reaching `FUN_006211D0` from the sidebar paint path. The exact integer determines the fade-gradient slope per BITFONT §3.7's `line_offset = (9 - fade_param) * 0x1F` formula. Blocks the Path B fade step in implementation, not the broader refactor. Can be done as a 30-minute follow-up before `/write-plan`.
- **Codepoint range expansion** beyond `0x20..0x180` for full localization (Cyrillic, CJK) — out of scope. Current range covers ASCII + Latin-1 Supplement + Latin Extended-A, which is sufficient for English/French/German/Spanish CSF strings. Localized builds outside this range now render correctly via missing-glyph fallback (instead of being silently dropped). Multi-page atlas infrastructure exists if needed later.
