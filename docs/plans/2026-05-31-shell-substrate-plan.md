# Shell Substrate (`ui::shell`) — Implementation Plan

> **For Claude:** Slice 0 is the ONLY implement-now scope. It is a **pure refactor —
> zero behavior change, every computed rect MUST stay pixel-identical**. Tasks 0.1–0.5
> land together (a `RectPx` type move does not compile half-done); compile only after
> Task 0.5. Slices 1–6 are **PLAN ONLY — NOT APPLIED THIS RUN**; do not write any of
> their code.
> Constraints this run: Ghidra read-only; do NOT run cargo (a later separate pass
> builds). Parallel human sessions edit `src/app_*` and `src/ui/*` — Slice 0
> deliberately touches ONLY the three `ui/<shell>/layout.rs` + the new `ui/shell/`;
> if any of those four files looks mid-edit, stop and note it, do not "fix" it.

**Goal:** Replace the 3-way triplication of `RectPx` / DLU helpers / right-panel /
lower-strip / button-snap across `main_menu_shell` (0xE2), `single_player_shell`
(0x100), and `skirmish_shell` (0x102) with ONE shared `src/ui/shell/geom.rs`. The three
shells import from it and delete their private copies. **Byte-identical output is the
bar** — the three copies are NOT all identical, so geom.rs preserves every per-caller
difference (two snap algorithms, the 156-vs-168 SDBTNANM width).

**Architecture:** New `ui/shell/` submodule, render-agnostic (depends only on plain
integers — no `sim`/`render`/`assets`/`ui`-sibling deps), honoring the `ui/mod.rs:10-12`
layering rule. The three shells `pub use` the shared types so every downstream import
(`main_menu_shell::RectPx`, `skirmish_shell::RectPx`, `skirmish_shell::RIGHT_PANEL_WIDTH`,
`main_menu_shell::RIGHT_PANEL_TILE_H`/`RIGHT_PANEL_WIDTH`, and the `super::super::layout`
paths inside `skirmish_shell/state/*`) keeps resolving with no rename.

**Design Doc:** `docs/plans/2026-05-31-shell-substrate-design.md` (§3 geom API, §4 Slice 0
scope, §6 risks). **Study Doc:** `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`
(§7 retire list, §8 migration slices).

---

## Grounding Summary (live-src verified this run; file:line quoted)

All three layout files were re-read this session and the design doc's claims confirmed:

- **`RectPx` struct + `new` + `contains`** byte-identical across copies: main_menu
  `layout.rs:14-30`, skirmish `layout.rs:42-67`. single_player imports main_menu's
  (`single_player_shell/layout.rs:4`). **`RectPx::translate` exists ONLY in skirmish**
  (`layout.rs:55-62`); main_menu/SP lack it.
- **`mul_div_round`** byte-identical: main_menu `107-114`, SP `50-57`, skirmish `229-236`.
- **`dlu_rect`** byte-identical: main_menu `116-123`, SP `59-66`, skirmish `238-245`
  (all `BASE_X=6`, `BASE_Y=13`).
- **`center_offset`**: SP `68-74` is `if screen > base { (screen-base)/2 } else {0}`;
  skirmish `427-429` is `((screen-base)/2).max(0)`; main_menu has **no named fn** — it
  inlines `if screen_w > SHELL_BASE_W { (screen_w-SHELL_BASE_W)/2 } else {0}`. **All three
  are algebraically identical for every i32** (verified boundary cases below). Canonical
  form chosen: `((screen-base)/2).max(0)`.
- **`right_panel_rects`** identical output: main_menu `147-181`, SP `90-124`, skirmish
  `447-476`. The only textual variance is `bottom_h.max(0)` placement (main/SP clamp at
  assignment, skirmish clamps at construction `474`) — **algebraically identical** (`.max(0)`
  of the same value). main/SP use named consts (`RIGHT_PANEL_TILE_COUNT_BASE`,
  `RIGHT_PANEL_TILE_H`); skirmish inlines `.min(9)` / `SDBTNBKGD_H` — same values.
- **`lower_strip_rect`**: main_menu `183-208`, SP `126-149`; **skirmish has none** (0x102
  has no lower strip).
- **Owner-draw button snap — TWO algorithms (must NOT merge):**
  - main_menu `sdbtnanm_button_rect` (`289-302`): round-half-up to nearest 42-px row,
    anchored at `right_panel.tile.y`, with `cell_w = SDBTNANM_CELL_W = 156` (`layout.rs:97`).
  - SP `owner_draw_button_snap_rect` (`169-185`): `+tile_h/2` biased-truncate tile-index,
    `source.y + center_offset(h)`, with `SDBTNANM_W = 168` (`layout.rs:15`).
  - skirmish `owner_draw_button_snap_rect` (`500-516`): **same biased-truncate as SP** but
    `SDBTNANM_W = 156` (`layout.rs:6`).
- **`back_rect` / `exit_button_rect` — also divergent (kept shell-local, see Task notes):**
  main_menu `exit_button_rect` (`308-312`) uses a raw DLU top (`EXIT_DLU_TOP=330`), NOT
  last-tile; SP `back_rect` (`187-195`) and skirmish `back_rect` (`490-498`) use
  last-tile `(panel.tile_count-1)` rows. SP width 168, skirmish width 156.

**Const divergence (load-bearing):** `SDBTNANM` cell width is **156** for main_menu
(`layout.rs:97`) and skirmish (`layout.rs:6`), but **168** for single_player (`layout.rs:15`).
A single hardcoded shared width would shift SP's 4 buttons by 12 px (168→156) and move
their flush-right x from 632 to 644. **The shared snap fns take `cell_w` as a parameter.**

**Re-export consumers (Slice 0 must keep names stable):**
- `main_menu_shell/mod.rs:6-10` `pub use`s `RectPx`, `RIGHT_PANEL_TILE_H`, `RIGHT_PANEL_WIDTH`
  (+ others). External consumers found: `app_single_player_shell_render.rs:13`
  (`use crate::ui::main_menu_shell::RectPx`), `single_player_shell/layout.rs:4`. The two
  consts are re-exported but have **no external consumer** today (the only `RIGHT_PANEL_*`
  hits outside `ui/` are `app_single_player_shell_render.rs:25-26`, which are that file's OWN
  private consts) — still, the `pub use` must keep compiling, so we preserve them.
- `skirmish_shell/mod.rs:11-30` `pub use`s `RectPx`, `RIGHT_PANEL_WIDTH` (+ ~40 items).
  External consumers: `app.rs:1078`, `app_skirmish_shell_render/preview.rs:15`. Internal:
  `skirmish_shell/state/*` import `RectPx` via `super::super::layout::{...RectPx...}`
  (`state/hit_test.rs:6-7`, `state/combos.rs:7-9`, `state/trackbars.rs:10`,
  `state/player_name.rs:10`). **So `skirmish_shell::layout::RectPx` MUST keep resolving** —
  achieved by `pub use`-ing the shared type back through `layout.rs`.
- `single_player_shell/mod.rs:6` does NOT re-export consts; it re-exports `compute_layout`
  and the layout structs only. It currently imports `RectPx` from main_menu; Slice 0
  re-points it to `ui::shell::geom`.

**`RightPanelRects` struct sharing is safe:** no code outside `ui/` references the type by
name (`grep RightPanelRects` outside `ui/` = 0 hits). Render code reads only field paths
(`layout.right_panel.{top,tile,bottom,tile_count}` — `app_main_menu_shell_render.rs:234-249`,
`app_single_player_shell_render.rs:250-265`). The shared `geom::RightPanelRects` has the
same all-`pub` fields, so field access and the responsive-layout literal construction
(`main_menu_shell/layout.rs:390-395`) are unaffected.

**Unknown after grounding:** none for Slice 0. All three layout files were last touched by
the 0xE2-geometry work (`git log`: commit `8706e58`/`845ea12` area); no parallel restructure
of these specific files observed this run. If a parallel session has edited a shell's
`layout.rs` line ranges below, re-anchor on the function name (not the line number) before
deleting.

### Boundary verification of `center_offset` equivalence (canonical = `((s-b)/2).max(0)`)

| screen `s`, base `b` | `if s>b {(s-b)/2} else 0` (main/SP form) | `((s-b)/2).max(0)` (skirmish/canonical) |
|---|---|---|
| s=800,b=800 | s>b false → 0 | (0)/2=0 → 0 |
| s=801,b=800 | (1)/2=0 → 0 | 0 → 0 |
| s=1024,b=800 | (224)/2=112 | 112 |
| s=799,b=800 | false → 0 | (-1)/2=0 (trunc-to-zero) → max(0)=0 |
| s=797,b=800 | false → 0 | (-3)/2=-1 → max(0)=0 |

Identical for all i32. Slice 0 keeps the canonical `.max(0)` form. **main_menu's inline
sites are NOT rewritten to call `center_offset`** (that would be a behavior-neutral but
out-of-scope edit and risks a typo) — main_menu keeps its inline guards; only the named
`center_offset` consumers (SP, skirmish) point at the shared fn.

---

## Key Technical Decisions

- **geom.rs carries BOTH snap algorithms as two named fns** (`snap_button_round_half_up`
  for 0xE2, `snap_button_biased_truncate` for 0x100/0x102), each taking `cell_w: i32`.
  **Why:** §6 risk 1 — the two roundings are not proven equivalent across the half-row
  boundary, and the width differs 156-vs-168. Parameterizing sidesteps having to prove
  equivalence and preserves each shell's exact asserted rects. **Confidence:** high —
  source: live read of the two fns + the existing per-shell tests.
- **`back_rect`/`exit_button_rect` stay shell-local** (NOT lifted to geom this slice).
  **Why:** three different rules (main_menu raw-DLU vs SP/skirmish last-tile; widths
  168 vs 156). The design doc §3.2 lists these as shell-specific. Lifting them adds risk
  with no dedup payoff for Slice 0 (each is ≤8 lines and one-of-a-kind). They keep calling
  the shared `dlu_rect`/`center_offset`/`mul_div_round` primitives. **Confidence:** high.
- **Each `layout.rs` `pub use`s the shared `RectPx`/`RightPanelRects`** so the existing
  `mod.rs` re-exports and the `state/*` `super::super::layout::RectPx` imports keep
  resolving with zero downstream edits. **Confidence:** high — source: the consumer
  inventory above.
- **geom.rs owns the canonical consts** (`RIGHT_PANEL_WIDTH=168`, `RIGHT_PANEL_TOP_H=199`,
  `RIGHT_PANEL_TILE_H=42`, `RIGHT_PANEL_TILE_COUNT_CAP=9`, `SDBTNANM_CELL_H=42`,
  `LOWER_STRIP_H=32`, `SDBTNANM_CELL_W_NARROW=156`, `SDBTNANM_CELL_W_WIDE=168`,
  `DLU_BASE_X=6`, `DLU_BASE_Y=13`). Each shell re-aliases the names it publicly exported
  (main_menu `RIGHT_PANEL_TILE_H`/`RIGHT_PANEL_WIDTH`, skirmish `RIGHT_PANEL_WIDTH`,
  skirmish `SDBTNBKGD_H`→`RIGHT_PANEL_TILE_H`). **Confidence:** high.

---

## Slice 0 — extract `ui/shell/geom.rs` (IMPLEMENT NOW)

### Task 0.1 — create `src/ui/shell/mod.rs`

New file. Content (verbatim):

```rust
//! Shared front-end shell substrate (Framework B: Win32-native dialog shells).
//!
//! Holds the geometry primitives the three pixel-parity shells (main menu 0xE2,
//! single player 0x100, skirmish 0x102) used to each re-implement. Render-agnostic:
//! depends on nothing above this layer (no sim/render/assets), so it honors the
//! ui/ layering rule. The wider descriptor/layout/controller/modal/slide substrate
//! is roadmap (see docs/plans/2026-05-31-shell-substrate-design.md §5); only geom
//! is shared today.

pub mod geom;
```

### Task 0.2 — create `src/ui/shell/geom.rs`

New file. This is the ONE shared copy. The fns below are copied **verbatim** from the
current per-shell implementations so output is byte-identical; the canonical
`center_offset` is the `.max(0)` form (proven equal to the `if` form above). Content
(verbatim):

```rust
//! Shared front-end shell geometry: DLU->pixel, right-panel chrome, button snap.
//!
//! Render-agnostic; depends on plain integers only. The three shells (dialogs
//! 0xE2 / 0x100 / 0x102) import these instead of each keeping a private copy.
//! Two distinct owner-draw snap algorithms are preserved (the main menu rounds
//! half-up to the nearest button row; single-player and skirmish bias-truncate a
//! tile index) because they are not proven equivalent at the half-row boundary.

/// Pixel rect in window space. Fields/semantics identical to the three prior
/// per-shell copies; `translate` is hoisted from the skirmish copy so all shells
/// share one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectPx {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn translate(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            w: self.w,
            h: self.h,
        }
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

// --- DLU base metrics (MS Sans Serif 8pt) ---
pub const DLU_BASE_X: i32 = 6;
pub const DLU_BASE_Y: i32 = 13;

// --- Right-panel chrome (SDTP top / SDBTNBKGD tile / SDBTM bottom) ---
pub const RIGHT_PANEL_WIDTH: i32 = 168;
pub const RIGHT_PANEL_TOP_H: i32 = 199;
pub const RIGHT_PANEL_TILE_H: i32 = 42;
pub const RIGHT_PANEL_TILE_COUNT_CAP: i32 = 9;
pub const SDBTNANM_CELL_H: i32 = 42;
pub const LOWER_STRIP_H: i32 = 32;

/// SDBTNANM.SHP button-cell widths. main menu (0xE2) and skirmish (0x102) use the
/// 156-wide cell flush at the panel right edge; single player (0x100) uses the
/// 168-wide cell flush at the panel left edge. Load-bearing divergence: a single
/// hardcoded width would shift 0x100's buttons 12 px.
pub const SDBTNANM_CELL_W_NARROW: i32 = 156;
pub const SDBTNANM_CELL_W_WIDE: i32 = 168;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightPanelRects {
    pub top: RectPx,
    pub tile: RectPx,
    pub tile_count: i32,
    pub bottom: RectPx,
}

/// Round-half-up MulDiv (sign-correct). Byte-identical to the three prior copies.
pub fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
    let value = n * numer;
    if value >= 0 {
        (value + denom / 2) / denom
    } else {
        (value - denom / 2) / denom
    }
}

/// DLU rect -> pixel rect (MS Sans Serif 8pt). Byte-identical to the three copies.
pub fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    RectPx::new(
        mul_div_round(x, DLU_BASE_X, 4),
        mul_div_round(y, DLU_BASE_Y, 8),
        mul_div_round(w, DLU_BASE_X, 4),
        mul_div_round(h, DLU_BASE_Y, 8),
    )
}

/// `(screen - base) / 2` clamped to >= 0. Canonical form; algebraically equal to
/// the single-player `if screen > base` guard and the main-menu inline guard for
/// every i32.
pub fn center_offset(screen: i32, base: i32) -> i32 {
    ((screen - base) / 2).max(0)
}

/// Right-panel layout (SDTP top cap / SDBTNBKGD tile column / SDBTM bottom cap).
/// Reproduces all three shells' output exactly, including the `bottom_h.max(0)`
/// clamp. Same for 0xE2 / 0x100 / 0x102.
pub fn right_panel_rects(screen_w: i32, screen_h: i32) -> RightPanelRects {
    let left_margin = if screen_w > 1023 {
        (screen_w - 800) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - 600) / 2
    } else {
        0
    };
    let effective_right = screen_w - left_margin;
    let top = RectPx::new(
        effective_right - RIGHT_PANEL_WIDTH,
        top_margin,
        RIGHT_PANEL_WIDTH,
        RIGHT_PANEL_TOP_H,
    );
    let tile = RectPx::new(top.x, top.y + top.h, RIGHT_PANEL_WIDTH, RIGHT_PANEL_TILE_H);
    let effective_h = if screen_h > 767 {
        screen_h - top_margin * 2
    } else {
        screen_h
    };
    let remaining = (effective_h - top.h).max(0);
    let tile_count = (remaining / RIGHT_PANEL_TILE_H).min(RIGHT_PANEL_TILE_COUNT_CAP);
    let bottom_y = tile.y + tile_count * RIGHT_PANEL_TILE_H;
    let bottom_h = (screen_h - top_margin - bottom_y).max(0);
    RightPanelRects {
        top,
        tile,
        tile_count,
        bottom: RectPx::new(top.x, bottom_y, RIGHT_PANEL_WIDTH, bottom_h),
    }
}

/// Lower strip (LWSCRN cap) flush against the screen/shell bottom. Used by the
/// main menu (0xE2) and single player (0x100); skirmish (0x102) has no lower strip.
pub fn lower_strip_rect(screen_w: i32, screen_h: i32) -> RectPx {
    let left_margin = if screen_w > 1023 {
        (screen_w - 800) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - 600) / 2
    } else {
        0
    };
    let shell_h = if screen_h > 767 { 600 } else { screen_h };
    // LWSCRNS at 640w is 472 wide; LWSCRNL at >=800w is 632 wide.
    let w = if screen_w == 640 { 472 } else { 632 };
    RectPx::new(
        left_margin,
        top_margin + shell_h - LOWER_STRIP_H,
        w,
        LOWER_STRIP_H,
    )
}

/// Owner-draw button snap, round-half-up variant (main menu 0xE2 stacked buttons).
/// `dlu_y` is the resource DLU top; the rect is right-anchored flush to the panel
/// right edge at `cell_w` wide and the DLU top is snapped to the nearest 42-px
/// SDBTNANM row anchored at the button-column top (the SDBTNBKGD tile origin).
pub fn snap_button_round_half_up(dlu_y: i32, panel: RightPanelRects, cell_w: i32) -> RectPx {
    let dlu_top = mul_div_round(dlu_y, DLU_BASE_Y, 8) + panel.top.y;
    let panel_y = panel.tile.y;
    let row_h = RIGHT_PANEL_TILE_H;
    let delta = (dlu_top - panel_y).max(0);
    let q = delta / row_h;
    let rem = delta % row_h;
    let q = if row_h - rem <= rem { q + 1 } else { q };
    let y = q * row_h + panel_y;
    let x = panel.top.x + (RIGHT_PANEL_WIDTH - cell_w);
    RectPx::new(x, y, cell_w, SDBTNANM_CELL_H)
}

/// Owner-draw button snap, biased-truncate variant (single player 0x100 and
/// skirmish 0x102). `source` is the DLU-derived resource rect; the rect is
/// flush-left at `screen_w - center_offset - cell_w`, `cell_w` wide, snapped to a
/// 42-px tile index from the SDBTNBKGD column top via `+tile_h/2` truncation.
pub fn snap_button_biased_truncate(
    screen_w: i32,
    screen_h: i32,
    source: RectPx,
    panel: RightPanelRects,
    cell_w: i32,
) -> RectPx {
    let offset_x = center_offset(screen_w, 800);
    let source_y = source.y + center_offset(screen_h, 600);
    let tile_h = panel.tile.h.max(1);
    let tile_index = ((source_y - panel.tile.y + tile_h / 2) / tile_h).max(0);
    RectPx::new(
        screen_w - offset_x - cell_w,
        panel.tile.y + tile_index * tile_h,
        cell_w,
        SDBTNANM_CELL_H,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div_round_matches_round_half_up_muldiv_all_odd_dlu() {
        // Round-half-up MulDiv reference (i64 to avoid overflow), both signs.
        fn reference(n: i64, numer: i64, denom: i64) -> i64 {
            let v = n * numer;
            if v >= 0 {
                (v + denom / 2) / denom
            } else {
                (v - denom / 2) / denom
            }
        }
        for dlu in (-1024..=1024).filter(|d| d % 2 != 0) {
            assert_eq!(
                mul_div_round(dlu, DLU_BASE_X, 4) as i64,
                reference(dlu as i64, DLU_BASE_X as i64, 4)
            );
            assert_eq!(
                mul_div_round(dlu, DLU_BASE_Y, 8) as i64,
                reference(dlu as i64, DLU_BASE_Y as i64, 8)
            );
        }
    }

    #[test]
    fn center_offset_equals_if_guard_form_at_boundaries() {
        for (s, b) in [(800, 800), (801, 800), (1024, 800), (799, 800), (797, 800)] {
            let if_form = if s > b { (s - b) / 2 } else { 0 };
            assert_eq!(center_offset(s, b), if_form, "s={s} b={b}");
        }
    }

    #[test]
    fn right_panel_rects_byte_equal_to_pre_refactor_literals() {
        // Values asserted by the three shells' existing suites pre-refactor.
        let a = right_panel_rects(800, 600);
        assert_eq!(a.top, RectPx::new(632, 0, 168, 199));
        assert_eq!(a.tile, RectPx::new(632, 199, 168, 42));
        assert_eq!(a.tile_count, 9);
        assert_eq!(a.bottom, RectPx::new(632, 577, 168, 23));

        let b = right_panel_rects(1024, 768);
        assert_eq!(b.top, RectPx::new(744, 84, 168, 199));
        assert_eq!(b.tile, RectPx::new(744, 283, 168, 42));
        assert_eq!(b.tile_count, 9);
        assert_eq!(b.bottom, RectPx::new(744, 661, 168, 23));

        let c = right_panel_rects(640, 480);
        assert_eq!(c.top, RectPx::new(472, 0, 168, 199));
        assert_eq!(c.tile, RectPx::new(472, 199, 168, 42));
        assert_eq!(c.tile_count, 6);
        assert_eq!(c.bottom, RectPx::new(472, 451, 168, 29));
    }

    #[test]
    fn snap_round_half_up_reproduces_main_menu_0xe2_stacked_cells() {
        // 0xE2 at 800x600: cell_w=156, flush-right x=644, rows from y=199.
        let panel = right_panel_rects(800, 600);
        let dlu_y = [125, 152, 179, 206, 233];
        let expected_y = [199, 241, 283, 325, 367];
        for (dy, ey) in dlu_y.iter().zip(expected_y) {
            assert_eq!(
                snap_button_round_half_up(*dy, panel, SDBTNANM_CELL_W_NARROW),
                RectPx::new(644, ey, 156, 42)
            );
        }
    }

    #[test]
    fn snap_biased_truncate_reproduces_single_player_0x100_wide_cells() {
        // 0x100 at 800x600: cell_w=168, flush-left x=632, rows 199/241/283.
        let panel = right_panel_rects(800, 600);
        let dlu = [dlu_rect(425, 122, 108, 23), dlu_rect(425, 149, 108, 23), dlu_rect(425, 176, 108, 23)];
        let expected_y = [199, 241, 283];
        for (src, ey) in dlu.iter().zip(expected_y) {
            assert_eq!(
                snap_button_biased_truncate(800, 600, *src, panel, SDBTNANM_CELL_W_WIDE),
                RectPx::new(632, ey, 168, 42)
            );
        }
    }

    #[test]
    fn snap_biased_truncate_reproduces_skirmish_0x102_narrow_cells() {
        // 0x102 at 800x600: cell_w=156, flush-right x=644, start/choose 241/283.
        let panel = right_panel_rects(800, 600);
        assert_eq!(
            snap_button_biased_truncate(800, 600, dlu_rect(425, 149, 108, 23), panel, SDBTNANM_CELL_W_NARROW),
            RectPx::new(644, 241, 156, 42)
        );
        assert_eq!(
            snap_button_biased_truncate(800, 600, dlu_rect(425, 176, 108, 23), panel, SDBTNANM_CELL_W_NARROW),
            RectPx::new(644, 283, 156, 42)
        );
    }

    #[test]
    fn lower_strip_matches_pre_refactor_values() {
        assert_eq!(lower_strip_rect(800, 600), RectPx::new(0, 568, 632, 32));
        assert_eq!(lower_strip_rect(1024, 768), RectPx::new(112, 652, 632, 32));
        assert_eq!(lower_strip_rect(640, 480), RectPx::new(0, 448, 472, 32));
    }
}
```

> NOTE on the `snap_button_round_half_up` x value: at 800x600 `panel.top.x = 632`,
> so `x = 632 + (168 - 156) = 644` — matches main_menu's current
> `right_panel.top.x + (RIGHT_PANEL_WIDTH - SDBTNANM_CELL_W)`. The biased-truncate
> variant computes x as `screen_w - offset_x - cell_w` (= 644 for cell_w=156, 632 for
> cell_w=168 at 800 wide) — matches SP/skirmish. Both x formulas are copied verbatim
> from the respective shells; they coincide at 800x600 by construction but are NOT
> interchangeable on oversized screens, so each variant keeps its own.

### Task 0.3 — register the module in `src/ui/mod.rs`

Add `pub mod shell;` to the module list. Exact edit — after the line
`pub mod pause_menu;` (currently `ui/mod.rs:21`) insert:

```rust
pub mod shell;
```

(Placement is alphabetical-ish among the existing `pub mod` block lines 14-23; any spot
in that block compiles — insert after `pub mod pause_menu;` to keep order tidy.)

### Task 0.4 — rewire `src/ui/main_menu_shell/layout.rs`

**Delete** these items (re-anchor on the item name if line numbers drifted):
- `RectPx` struct + impl (`14-30`).
- `RightPanelRects` struct (`57-63`).
- consts `RIGHT_PANEL_WIDTH` (`86`), `RIGHT_PANEL_TOP_H` (`87`), `RIGHT_PANEL_TILE_H` (`88`),
  `RIGHT_PANEL_TILE_COUNT_BASE` (`89`), `LOWER_STRIP_H` (`91`), `SDBTNANM_CELL_H` (`98`),
  `BASE_X`/`BASE_Y` (`7-8`). **Keep** `RIGHT_PANEL_BOTTOM_H` (`90`) and `SDBTNANM_CELL_W`
  (`97`) — `SDBTNANM_CELL_W` is still referenced by `exit_button_rect`/`sdbtnanm_button_rect`
  callers; re-alias `SDBTNANM_CELL_H` from geom (see below). (`RIGHT_PANEL_BOTTOM_H` is used
  only by tests/comments — leave it; it is not in geom.)
- `fn mul_div_round` (`107-114`), `fn dlu_rect` (`116-123`), `fn right_panel_rects`
  (`147-181`), `fn lower_strip_rect` (`183-208`).

**Add** at the top of the file (after the `use super::state::MainMenuControlId;` line):

```rust
pub use crate::ui::shell::geom::{RectPx, RightPanelRects};
use crate::ui::shell::geom::{
    RIGHT_PANEL_TILE_H, RIGHT_PANEL_WIDTH, dlu_rect, lower_strip_rect, mul_div_round,
    right_panel_rects,
};
const SDBTNANM_CELL_H: i32 = crate::ui::shell::geom::SDBTNANM_CELL_H;
const BASE_Y: i32 = crate::ui::shell::geom::DLU_BASE_Y;
```

Rationale per symbol:
- `pub use ...{RectPx, RightPanelRects}` keeps `main_menu_shell/layout.rs::RectPx` and the
  `mod.rs:6-10` re-export of `RectPx` resolving for `app_*_render.rs` and SP's import.
- `use ...{RIGHT_PANEL_TILE_H, RIGHT_PANEL_WIDTH}` — `mod.rs:7` re-exports both publicly;
  this re-import (`pub use` is via `mod.rs`, the `use` here makes them in-scope for this
  file's body, e.g. `RIGHT_PANEL_WIDTH` at `160/300/310` and `right_anchor_rect`). To keep
  `mod.rs:7`'s `pub use layout::{... RIGHT_PANEL_TILE_H, RIGHT_PANEL_WIDTH ...}` valid,
  change the `use` to `pub use`:

```rust
pub use crate::ui::shell::geom::{RIGHT_PANEL_TILE_H, RIGHT_PANEL_WIDTH};
use crate::ui::shell::geom::{dlu_rect, lower_strip_rect, mul_div_round, right_panel_rects};
```

- `SDBTNANM_CELL_H` / `BASE_Y` are still referenced by `sdbtnanm_button_rect` (`290-301`)
  and `exit_button_rect` (`309-311`); re-alias as local consts from geom so those fns
  compile unchanged.

**Keep local, unchanged** (they call the now-shared primitives): `sdbtnanm_button_rect`
(`289-302`), `exit_button_rect` (`308-312`), `right_anchor_rect`, `title_rect`,
`version_line_rect`, `tooltip_line_rect`, `movie_*`, `scale_rect`, `compute_layout`,
`compute_responsive_layout`, all consts not listed above, the test module.

> Note: `sdbtnanm_button_rect` and `exit_button_rect` stay shell-local in this slice (NOT
> routed through `snap_button_round_half_up`). They already produce the asserted rects and
> are the only callers; lifting them is Slice 1 (descriptor) territory. The geom
> `snap_button_round_half_up` exists for the future descriptor pass and is unit-tested in
> geom.rs. **This keeps main_menu's existing tests byte-identical with zero call-site
> change.** (Alternative considered: replace `sdbtnanm_button_rect`'s body with a call to
> `snap_button_round_half_up(dlu_y, right_panel, SDBTNANM_CELL_W)` — output-identical, but
> an unnecessary edit this slice; defer.)

### Task 0.5 — rewire `src/ui/single_player_shell/layout.rs`

**Delete:**
- `use crate::ui::main_menu_shell::{MainMenuMovieBase, RectPx, movie_base_for_screen_width};`
  (`4`) — replace `RectPx` source (see Add).
- consts `BASE_X`/`BASE_Y` (`8-9`), `RIGHT_PANEL_WIDTH` (`10`), `RIGHT_PANEL_TOP_H` (`11`),
  `RIGHT_PANEL_TILE_H` (`12`), `RIGHT_PANEL_TILE_COUNT_BASE` (`13`), `LOWER_STRIP_H` (`14`).
  **Keep** `SDBTNANM_W=168` (`15`), `SDBTNANM_H=42` (`16`) — used by `back_rect`/snap caller;
  re-alias `RIGHT_PANEL_TILE_H` from geom (used by `back_rect` `191`).
- `RightPanelRects` struct (`28-34`).
- `fn mul_div_round` (`50-57`), `fn dlu_rect` (`59-66`), `fn center_offset` (`68-74`),
  `fn right_panel_rects` (`90-124`), `fn lower_strip_rect` (`126-149`).
- `fn owner_draw_button_snap_rect` (`169-185`) — replace its body with a call (see below),
  OR keep it as a thin wrapper. **Keep** `fn back_rect` (`187-195`) local.

**Add** at the top (replacing the deleted `use` on line 4):

```rust
use crate::ui::main_menu_shell::{MainMenuMovieBase, movie_base_for_screen_width};
use crate::ui::shell::geom::{
    RectPx, RightPanelRects, center_offset, dlu_rect, lower_strip_rect, right_panel_rects,
    snap_button_biased_truncate,
};
const RIGHT_PANEL_TILE_H: i32 = crate::ui::shell::geom::RIGHT_PANEL_TILE_H;
```

`MainMenuMovieBase` and `movie_base_for_screen_width` still come from main_menu (movie
asset choice is not geom). `RectPx` now comes from geom directly. The SP `mod.rs:6` does
NOT re-export `RectPx`, so no `pub use` needed here.

**Replace** `fn owner_draw_button_snap_rect` body — change the 4 call sites
(`223-228`, `232-237`, `241-246`) to call the shared fn directly, and delete the local
wrapper:

```rust
// at each of the 3 button call sites in compute_layout (NewCampaign/Load/Skirmish):
rect: snap_button_biased_truncate(
    screen_w,
    screen_h,
    dlu_rect(425, 122, 108, 23), // (and 149,…)/(176,…) for the other two
    panel,
    SDBTNANM_W,
),
```

i.e. the 3 calls pass `SDBTNANM_W` (= 168) as the new `cell_w` argument. `back_rect` is
unchanged (the 4th button). This produces byte-identical output: the old
`owner_draw_button_snap_rect` was exactly `snap_button_biased_truncate` with `SDBTNANM_W`
hardcoded.

**Keep local, unchanged:** `back_rect`, `right_anchor`, `status_help_rect`, `movie_origin`,
`compute_layout` (except the 3 snap call edits), the layout structs, the test module.
The existing test `key_rects_match_dialog_0x100_rows_at_800x600` (asserts buttons at
`632, 199/241/283` width 168) stays green — the 168 width is preserved via `SDBTNANM_W`.

### Task 0.6 — rewire `src/ui/skirmish_shell/layout.rs`

**Delete:**
- `RectPx` struct + impl incl. `translate` (`42-67`).
- `RightPanelRects` struct (`122-128`).
- consts `RIGHT_PANEL_WIDTH` (`5`), `SDBTNANM_H` (`7`), `SDBTNBKGD_H` (`8`),
  `BASE_X`/`BASE_Y` (`226-227`). **Keep** `SDBTNANM_W=156` (`6`) — used by `back_rect`/snap.
- `fn mul_div_round` (`229-236`), `fn dlu_rect` (`238-245`), `fn center_offset` (`427-429`),
  `fn right_panel_rects` (`447-476`).
- `fn owner_draw_button_snap_rect` (`500-516`) — replace call sites (see below).
  **Keep** `fn back_rect` (`490-498`) local.

**Add** at the top (after the `//!` header / before the consts):

```rust
pub use crate::ui::shell::geom::{RectPx, RightPanelRects};
use crate::ui::shell::geom::{
    center_offset, dlu_rect, mul_div_round, right_panel_rects, snap_button_biased_truncate,
};
pub const RIGHT_PANEL_WIDTH: i32 = crate::ui::shell::geom::RIGHT_PANEL_WIDTH;
const SDBTNANM_H: i32 = crate::ui::shell::geom::SDBTNANM_CELL_H;
const SDBTNBKGD_H: i32 = crate::ui::shell::geom::RIGHT_PANEL_TILE_H;
```

Rationale:
- `pub use ...{RectPx, RightPanelRects}` is **the critical re-export** — it keeps
  `skirmish_shell::layout::RectPx` resolving for `state/hit_test.rs:7`, `state/combos.rs:9`,
  `state/trackbars.rs`, `state/player_name.rs` (all `use super::super::layout::{...RectPx...}`)
  AND for `mod.rs:16`'s `pub use layout::{... RectPx ...}` (consumed by `app.rs:1078`,
  `app_skirmish_shell_render/preview.rs:15`). Without this `pub use`, ~278 `RectPx`
  references break.
- `pub const RIGHT_PANEL_WIDTH` keeps `mod.rs:16`'s `pub use layout::{... RIGHT_PANEL_WIDTH ...}`
  valid (re-export, external consumers exist).
- `mul_div_round` is still used by the many `dlu_rect`-adjacent fns and the inline
  fixups; re-import it. `dlu_rect` is used pervasively in `compute_layout`. `center_offset`
  used by `right_anchor`/`back_rect`/`status_help_rect`/`centered_fixed_shell_offset`.
- `SDBTNANM_H`/`SDBTNBKGD_H` re-aliased so `back_rect`, `right_panel_text` (`529`),
  and the trackbar/checkbox geometry that reference `SDBTNBKGD_H` compile unchanged.

**Replace** `fn owner_draw_button_snap_rect` — delete the local fn and change its call
sites (`compute_layout:603-604`, `compute_choose_map_modal_layout:668-675`) to call the
shared fn with `SDBTNANM_W` (= 156):

```rust
// start_button / choose_map_button in compute_layout:
start_button: snap_button_biased_truncate(screen_w, screen_h, start_base, panel, SDBTNANM_W),
choose_map_button: snap_button_biased_truncate(screen_w, screen_h, choose_base, panel, SDBTNANM_W),
// use_map_button / create_random_map_button in compute_choose_map_modal_layout:
use_map_button: snap_button_biased_truncate(screen_w, screen_h, use_map_base, panel, SDBTNANM_W),
create_random_map_button: snap_button_biased_truncate(screen_w, screen_h, create_random_map_base, panel, SDBTNANM_W),
```

Byte-identical: the old fn was exactly `snap_button_biased_truncate` with `SDBTNANM_W=156`
hardcoded.

**Keep local, unchanged (large surface — do NOT touch):** `back_rect`, `right_anchor`,
`status_help_rect`, `choose_map_status_help_rect`, `dialog_child`,
`centered_live_screen_dialog`, all the combo/trackbar/checkbox/listbox helpers
(`combo_*`, `trackbar_*`, `checkbox_*`, `player_name_edit_*`, `choose_map_listbox_*`),
`translate_layout`, `compute_fixed_800_layout`, `compute_layout` (except the 2 snap
edits), `compute_choose_map_modal_layout` (except the 2 snap edits),
`compute_validation_modal_layout`, the validation/choose-map structs, the 2147-line-ish
test module. The `RectPx::translate` method these rely on (`translate_layout`,
`offset_rect_x`, `combo_*`) now comes from the shared `RectPx` — same `const fn translate`
signature, so they compile unchanged.

### Slice 0 verification (NO cargo this run — a later separate pass does this)

1. **`cargo check -p vera20k`** — compiles. Watch for: a missed `RectPx` import in a
   `state/*` file (means the `pub use` in `layout.rs` is wrong/missing); a missed
   `RIGHT_PANEL_WIDTH`/`RIGHT_PANEL_TILE_H` re-export break in a `mod.rs`.
2. **`cargo test -p vera20k`** — every existing per-shell test stays green **unchanged**:
   - `main_menu_shell/layout.rs::tests` (`key_rects_match_800x600`,
     `buttons_grid_snap_and_exit_special_case_800x600`, `title_rect_matches_dlu_at_800x600`,
     `tooltip_line_anchors_*`, `version_line_uses_*`, `key_rects_match_640x480_movie_choice`,
     `large_screen_*`, `responsive_layout_*`).
   - `single_player_shell/layout.rs::tests` (`key_rects_match_dialog_0x100_rows_at_800x600`
     — buttons `632,199/241/283` w=168 + back `632,535`; `large_screen_*`).
   - `skirmish_shell/layout.rs::tests` (the full suite: `key_rects_match_800x600/1024/640`,
     `right_panel_globals_match_research_modes`, `choose_map_modal_*`, `validation_modal_*`,
     trackbar/checkbox/combo geometry, `fixed_800_*`).
   - `skirmish_shell/state/tests.rs` (the big suite).
   - New geom.rs tests (Task 0.2): `mul_div_round_matches_round_half_up_muldiv_all_odd_dlu`,
     `center_offset_equals_if_guard_form_at_boundaries`,
     `right_panel_rects_byte_equal_to_pre_refactor_literals`,
     `snap_round_half_up_reproduces_main_menu_0xe2_stacked_cells`,
     `snap_biased_truncate_reproduces_single_player_0x100_wide_cells`,
     `snap_biased_truncate_reproduces_skirmish_0x102_narrow_cells`,
     `lower_strip_matches_pre_refactor_values`.
3. **Manual in-game look** (the parity bar — tests can't catch a render-side regression):
   launch the app, observe at 800x600 and 1024x768:
   - Main menu (0xE2): 6 buttons flush-right at x=644 (756 at 1024), stacked
     199/241/283/325/367, Exit at 536 (620 at 1024); title, version line, tooltip line
     unchanged.
   - Single player (0x100): 4 buttons flush-LEFT at x=632 (744 at 1024), w=168,
     New/Load/Skirmish at 199/241/283, Back at 535 (619 at 1024).
   - Skirmish (0x102): Start/Choose at 644 241/283 w=156 (756 325/367 at 1024), Back at
     535 (619), choose-map modal lists + validation modal centered.
   A side-by-side screenshot diff against a pre-refactor capture must be **0 changed
   pixels** on all three shells.
4. **No render/app file changed** — confirm `git diff --stat` shows only
   `src/ui/shell/mod.rs`, `src/ui/shell/geom.rs`, `src/ui/mod.rs`, and the three
   `src/ui/<shell>/layout.rs`. The contested `src/app_*` and render files are untouched.

---

## Parity-Critical Section — Slice 0 must be pixel-identical

Every caller whose rect could shift, and why it will not (or would, if a copy were wrong):

| Caller / rect | Could shift? | Why it stays identical |
|---|---|---|
| main_menu 6 buttons (`sdbtnanm_button_rect`/`exit_button_rect`) | No | These two fns are **kept verbatim** (not routed through geom this slice); they call the byte-identical `mul_div_round`/`dlu_rect` and use local `SDBTNANM_CELL_W=156`/re-aliased `SDBTNANM_CELL_H`/`BASE_Y`. Output unchanged. |
| main_menu right_panel / lower_strip / title / version / tooltip | No | `right_panel_rects`/`lower_strip_rect` copied verbatim into geom (same `RIGHT_PANEL_*` consts = same values); `title/version/tooltip` fns unchanged, call shared `dlu_rect`. |
| SP 3 stacked buttons (NewCampaign/Load/Skirmish) | **Yes if width wrong** | Routed through `snap_button_biased_truncate(..., SDBTNANM_W=168)`. The old fn hardcoded 168; passing `SDBTNANM_W` (still 168) keeps x=632, w=168. **If a shared single-width geom were used (156), SP would shift to x=644, w=156 — 12 px wrong. Avoided by the `cell_w` param.** |
| SP `back_rect` (4th button) | No | Kept local, unchanged; uses `SDBTNANM_W=168` + re-aliased `RIGHT_PANEL_TILE_H`. |
| skirmish Start/Choose/UseMap/CreateRandom buttons | No | Routed through `snap_button_biased_truncate(..., SDBTNANM_W=156)`; old fn hardcoded 156. x=644, w=156 preserved. |
| skirmish `back_rect`, all combo/trackbar/checkbox/listbox geometry | No | Kept local; rely on shared `RectPx`/`translate`/`dlu_rect`/`center_offset` (all byte-identical) + re-aliased `SDBTNBKGD_H`/`SDBTNANM_H`. |
| `center_offset` consumers (SP `right_anchor`/`status_help`/snap; skirmish `right_anchor`/`back_rect`/`status_help`/`centered_fixed_shell_offset`/snap) | No | Canonical `((s-b)/2).max(0)` proven equal to the per-shell `if`/`.max(0)` forms for all i32 (boundary table above). main_menu does NOT use `center_offset` (inline guards untouched). |
| `right_panel_rects` `bottom_h` clamp | No | geom uses `(screen_h - top_margin - bottom_y).max(0)` (the main/SP form); algebraically equal to skirmish's `.max(0)`-at-construction form — same output. The `right_panel_rects_byte_equal_*` test pins all three resolutions. |
| Downstream `RectPx` importers (`app_*_render.rs`, `app.rs:1078`, `preview.rs:15`, skirmish `state/*`) | No (import path) | Each `layout.rs` `pub use`s `geom::RectPx`, so `main_menu_shell::RectPx` / `skirmish_shell::RectPx` / `skirmish_shell::layout::RectPx` all still resolve to a type with identical fields/methods. No call-site change. |
| `RightPanelRects` field access in render (`layout.right_panel.{top,tile,bottom,tile_count}`) | No | Shared `geom::RightPanelRects` has the same all-`pub` fields; field access and the main_menu responsive literal construction (`390-395`) compile unchanged. |

**Copies that were NOT byte-identical (and how Slice 0 reconciles to gamemd-correct):**
1. **Two snap algorithms** (main_menu round-half-up vs SP/skirmish biased-truncate) — both
   preserved as separate geom fns; NEITHER is "the correct one" to merge to, because they
   apply to different dialogs with different gamemd resource layouts. No shell's output
   changes.
2. **SDBTNANM width 156 (0xE2/0x102) vs 168 (0x100)** — both gamemd-correct for their
   dialog; the snap fns take `cell_w`. **No shell's output changes.** (A naive single-const
   share would have shifted SP 12 px — that is the only place a wrong "share" would move
   pixels, and the `cell_w` param prevents it.)
3. **`RectPx::translate`** — existed only in skirmish; now shared. main_menu/SP gain the
   method but never call it, so no behavior change; skirmish's `translate_layout`/`combo_*`
   keep working against the same `const fn translate` signature.

**Net pixel delta for all three shells: 0.** No copy was *wrong* (each produced its
dialog's correct gamemd rects); the divergences are legitimate per-dialog differences, all
preserved. Slice 0 is a pure consolidation with no observable change.

---

## Slices 1–6 — PLAN ONLY, NOT APPLIED THIS RUN

> The following are task outlines distilled from study doc §8 and design doc §5. **None is
> implemented in this run.** Each pre-req must be re-verified at the start of its slice
> (render-side line ranges and the C13 modal-template mapping are UNCHECKED). Do NOT write
> any code below this line this run.

### Slice 1 — Descriptor table + layout pass (`descriptor.rs` + `layout.rs`) — NOT APPLIED
- Add `DialogDescriptor { id, controls, bg_kind, slide_eligible, reposition_policy }`,
  `ControlDescriptor { id, kind, dlu_rect, csf_key, group, enabled }`, `ControlKind`.
- `layout_pass(&DialogDescriptor, screen_w, screen_h) -> Vec<(ControlId, RectPx)>`:
  DLU->px once, include-set re-anchor (0xE2/0x6B/0x100/0x102 only), 1-px finalizers
  (e.g. 0xE2/0x694 Y+7/H+1). Convert **0xE2 first**; route `sdbtnanm_button_rect` through
  `geom::snap_button_round_half_up`.
- **Contract:** C6 (subclass classification), C7 (DLU->px once + include-set re-anchor).
- **Pre-req/risk:** include-set gating (0x120/0xCE excluded — study §3 DRIFT-CORRECTED).
- **Acceptance:** 0xE2 button rects = (644-snapped 156×42, Exit at template rect), title
  0x694 with Y+7/H+1; **screenshot diff vs current 0xE2 == 0 changed pixels**; include-set
  gating verified.

### Slice 2 — Descriptor-driven hit/press model + `DialogController` router — NOT APPLIED
- `DialogController { stack, kbd_route }`, descriptor-driven hit-test + press-must-match,
  `on_event -> Option<ShellAction>`, result-code map + nav table.
- Migrate 0xE2 + 0x100 input; retire `main_menu_shell/state.rs:90-129`,
  `single_player_shell/state.rs:60-116` (keep the LoadSavedGame disabled guard),
  `skirmish_shell/state/hit_test.rs` press cluster, `app.rs:1426-1437`.
- **Contract:** C1 (lifecycle ordering), C3 (keyboard routing in registration order),
  C4 (result channel), C5 (focus restore), C12 (one result-routed nav loop).
- **Pre-req/risk:** SP stays an **intermediate** dialog (0x579 -> skirmish, NOT a direct
  jump); parallel human edits to `app.rs` — re-verify `:1426-1437` before deleting.
- **Acceptance:** press-must-match-release (press A, drag to B, release = no fire); SP
  0x579 routes to skirmish; focus restore returns to parent dialog; C3 test asserts
  Tab/Enter/Esc offered in registration order independent of LIFO stack.

### Slice 3 — `OwnerDrawControl` paint trait — NOT APPLIED (render-coupled)
- One paint pass over descriptors: SDBTNANM frames 2/3/4 + ~1 Hz hover flash (+0xC5),
  pressed sink +2y/+1x, parent_compose order (offscreen <- right-panel <- MNSCRN <-
  controls <- flip).
- Retire duplicated emitters (`app_main_menu_shell_render.rs:47-292`,
  `app_single_player_shell_render.rs:36-300`) + one atlas-pack copy.
- **Contract:** C8 (paint composition order), C9 (button art frames + pressed sink),
  C10 (text color permutation, yellow #FFFF00 / disabled #9F0000).
- **Pre-req/risk:** **render-coupled — re-verify the emitter line ranges first** (study §7.5
  ranges are UNCHECKED this run; a parallel session may be mid-edit).
- **Acceptance:** **screenshot diff == 0 vs current per state** (idle/hover/pressed).

### Slice 4 — Fold skirmish controls onto the substrate — NOT APPLIED
- Combo/trackbar/checkbox/listbox -> `ControlKind`s; unify the two skirmish scroll models
  (combo dropdown `state/combos.rs` vs choose-map listbox `layout.rs:702-831`).
- **Contract:** per-control behavior + C14 defaults seed.
- **Pre-req/risk:** biggest blast radius; the skirmish `state/tests.rs` suite is the safety
  net — it must stay green unchanged.
- **Acceptance:** combo open/scroll/select, trackbar drag value, checkbox icon-vs-label hit,
  choose-map listbox scroll all identical (existing tests green); `[MultiplayerDialogSettings]`
  seeds every control byte-exact (TechLevel 10, GameSpeed 1, FogOfWar off, …).

### Slice 5 — Modal substrate (`modal.rs`) + lifecycle/pump — DATA SHIPPED (5a); WIRING (5b) PENDING
> **STATUS UPDATE (2026-06-12):** the Ghidra pre-req is **RESOLVED** (was the blocker). C13
> template-id count-rule + result conventions shipped in `src/ui/shell/modal.rs` (binary-cited).
> C2 pump resolved in `docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md`
> — and it **OVERTURNS** the C2 acceptance below: offline skirmish (mode 5) and campaign (mode 0)
> **FREEZE** the world behind modals (no `Main_Tick`); only LAN 3 / WOL 4 advance. The "sim tick
> advances ≥1 / battlefield animates behind options" criterion was WRONG for the offline client.
> 5b kickoff (`docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md`) already carries the
> corrected mode partition. Remaining = the 5b wiring (render emitter + app-layer mode-gated
> pump decision + lifecycle), NOT research.
- `ModalKind::{Body, BodyOk(0xCE), Confirm(0x120), ThreeButton(0x121)}` selected by
  text-slot presence; `service_tick` runs message/input/repaint always, advances sim ONLY on
  mode-gated branches (LAN 3 / WOL 4), freezes offline {0,5}; mode-2 SHP compose
  (PUDLGBGN.SHP + DIALOGN.PAL + MNBTTN owner-draw OK).
- **Contract:** C2 (mode-gated pump — offline freeze / network advance), C13 (modal text-slot routing).
- **Pre-req/risk:** **RESOLVED** — C13 (`modal.rs`) + C2 (`MODAL_PUMP_…` report). No Ghidra blocker remains.
- **Acceptance (CORRECTED):** validation modal renders PUDLGBGN.SHP+DIALOGN.PAL + MNBTTN owner-draw OK
  (not a flat panel / not skirmish PCX); quit-confirm writes ra2md.ini before the graceful
  return cascade; **automated assertion** that during OFFLINE skirmish the world tick delta is **0**
  per pumped frame while a modal is open (world freezes; dialog stays message-responsive). The
  LAN/WOL advance branch is a separate test.

### Slice 6 — Data-driven slide eligibility (`slide.rs`) — NOT APPLIED
- Drive `app_shell_transition` off the dialog-id allow-list + per-dialog first-paint state;
  add the missing 0x100 (and 0x94 if present) first-paint slides.
- **Contract:** C11 (first-paint slide: SHP-frame sweep, 30 ms/tick, loop bound = max
  stagger + 6 = reuse `WAVE_TAIL_TICKS=6`).
- **Pre-req/risk:** **keep `app_shell_transition.rs` (generalize, don't rewrite)** — it is
  the already-generic model.
- **Acceptance:** every include-set shell slides once on first paint; 30 ms/tick, loop
  bound = max stagger + 6, SHP-frame sweep (not x/y ramp); `MenuSlideIn` at start, silent
  end cue (stock); non-listed transient dialogs do not slide.

> **Out of scope entirely** (do NOT fold into the substrate): the egui menus
> (`main_menu.rs`, `main_menu_dialogs.rs`, `pause_menu.rs`) and the in-game
> GadgetClass/sidebar (study §0 Framework A) — different service.

---

## Rollback

Slice 0 is contained to 6 files (`git diff --stat` listed in verification step 4). If
`cargo check`/`test` or the manual look shows any drift, `git restore` the three
`ui/<shell>/layout.rs` and delete `src/ui/shell/` + the `pub mod shell;` line. No
render/app/sim state is touched, so rollback is clean.
