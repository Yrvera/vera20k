---
title: Pre-execution review — Shell Substrate Slice 0 plan
date: 2026-05-31
reviewer: review subagent (read-only Ghidra, no cargo this run)
plan: docs/plans/2026-05-31-shell-substrate-plan.md
study: docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md
verdict: READY (with 2 cosmetic fixes — neither changes a pixel)
---

# Verdict: READY

Slice 0 is a sound pure-refactor. Every load-bearing equivalence claim verified against
live src this run holds: the three `mul_div_round`/`dlu_rect` copies are byte-identical,
the three `right_panel_rects` are algebraically identical, `center_offset` `.max(0)` ==
the `if` form for all i32, and the two snap algorithms + the 156-vs-168 cell-width
divergence are correctly preserved via the `cell_w` parameter (so SP does NOT shift 12 px).
The `pub use` re-export mechanism that keeps `RectPx` resolving for ~278 downstream
references is correctly specified for all three shells.

Two findings are **cosmetic only** (an `unused import` / dead-`pub const` warning, never a
pixel change or compile error), so the plan can ship as-is or with the two trivial
adjustments below. **No CONFIRMED ISSUE that would move a pixel.** The task's central
worry — "are the three RectPx/DLU/right-panel copies equivalent, would a naive shared
extraction change a shell's pixels" — is answered: YES they are equivalent, and the plan's
`cell_w`-parameterized snap correctly handles the ONE genuine divergence (SDBTNANM width).

---

## STALE / WRONG codebase claims (file:line)

| # | Plan claim | Actual (verified this run) | Impact |
|---|---|---|---|
| S1 | `mul_div_round` in skirmish "is still used by the many `dlu_rect`-adjacent fns and the inline fixups; re-import it" (Task 0.6 rationale, plan §0.6) | `skirmish_shell/layout.rs`: `mul_div_round` is called **only** by `dlu_rect` (lines 240-243), which Task 0.6 also deletes. No other caller exists (grep: defn 229, calls 240-243, nothing else). | Cosmetic. Importing `mul_div_round` per Task 0.6 (`use …geom::{… mul_div_round, …}`) becomes an **unused import** → `warning: unused import`, NOT an error. Fix: drop `mul_div_round` from skirmish's geom `use` list. |
| S2 | `RIGHT_PANEL_BOTTOM_H` "is used only by tests/comments — leave it" (Task 0.4, plan line ~506) | `main_menu_shell/layout.rs:90` `pub const RIGHT_PANEL_BOTTOM_H` is referenced by **nothing** — not tests, not comments, not code (grep src-wide: only the definition at line 90). | Cosmetic. It is `pub`, so leaving it causes no warning. The keep decision is harmless; only the *rationale* is wrong. |
| S3 | "the `state/*` `super::super::layout::RectPx` imports … `state/trackbars.rs:10`, `state/player_name.rs:10`" (Grounding Summary + Task 0.6 rationale) | `trackbars.rs`: `RectPx` is imported at **line 6**, not 10 (block is lines 5-9). `player_name.rs:7` imports `super::super::layout::{SkirmishShellLayout, SkirmishTrackbarId}` and **does NOT import `RectPx` at all**. | None on mechanism. `hit_test.rs:7` and `combos.rs:9` DO import `RectPx`, so the `pub use` in `layout.rs` is still required and the plan's fix is still correct. Only the cited line numbers / the `player_name.rs` consumer are wrong. |
| S4 | "skirmish `RIGHT_PANEL_WIDTH` … re-export … external consumers exist" (Task 0.6 rationale) | `skirmish_shell::RIGHT_PANEL_WIDTH` is in the `mod.rs:16` `pub use` list, but grep finds **no external importer** by name (the only non-ui `RIGHT_PANEL_WIDTH` hits are `app_single_player_shell_render.rs:25-26`, that file's OWN private consts). | None. The `pub const RIGHT_PANEL_WIDTH = …geom::RIGHT_PANEL_WIDTH;` re-alias is still needed to keep the `mod.rs:16` `pub use` compiling. Over-claim only. |
| S5 | choose-map snap call sites "`compute_choose_map_modal_layout:668-675`" (Task 0.6) | Actual: `use_map_button` at **668**, `create_random_map_button` at **670-675**. Range is fine; just note `use_map_button` is a single line at 668, not part of a 668-675 block. | None. Re-anchor on the field name as the plan already instructs. |

All other line-number claims verified accurate within the stated ±10 tolerance:
RectPx struct (main 14-30, skirmish 42-67 incl. `translate` 55-62), `mul_div_round`
(main 107-114, SP 50-57, skirmish 229-236), `dlu_rect` (main 116-123, SP 59-66, skirmish
238-245), `center_offset` (SP 68-74 `if` form, skirmish 427-429 `.max(0)`), `right_panel_rects`
(main 147-181, SP 90-124, skirmish 447-476), `lower_strip_rect` (main 183-208, SP 126-149,
skirmish: none), snap fns (main `sdbtnanm_button_rect` 289-302 + `exit_button_rect` 308-312,
SP `owner_draw_button_snap_rect` 169-185 + `back_rect` 187-195, skirmish snap 500-516 +
`back_rect` 490-498), consts (main `SDBTNANM_CELL_W`=156 @97, SP `SDBTNANM_W`=168 @15,
skirmish `SDBTNANM_W`=156 @6), `ui/mod.rs:21` is `pub mod pause_menu;`, mod.rs re-exports
(main 6-10, skirmish 11-30), external `RectPx` consumers (`app_single_player_shell_render.rs:13`,
`app.rs:1078`, `app_skirmish_shell_render/preview.rs:15`), responsive literal
construction (main 390-395). All VERIFIED.

---

## Re-verification of the "three copies equivalent" question (the task's core ask)

The task asks specifically: re-verify the doc/plan claim that the three RectPx/DLU/
right-panel copies are equivalent; if any differ, that is a CONFIRMED ISSUE because a
naive shared extraction would change that shell's pixels.

**Result: the copies ARE equivalent. No CONFIRMED ISSUE.** Detail:

1. **`RectPx` + `new` + `contains`** — main (14-30) and skirmish (42-67) are field-for-field
   and method-for-method identical except skirmish adds `const fn translate` (55-62). The
   shared geom type adds `translate` to all three; main_menu/SP gain a method they never
   call → no behavior change. EQUIVALENT.
2. **`mul_div_round`** — all three identical token-for-token (`value=n*numer`; sign-split
   round-half-up). EQUIVALENT (byte-identical).
3. **`dlu_rect`** — all three call `mul_div_round(_,6,4)` / `(_,13,8)` with the same
   `BASE_X=6`/`BASE_Y=13`. EQUIVALENT (byte-identical).
4. **`center_offset`** — SP `if screen>base {(s-b)/2} else 0` vs skirmish `((s-b)/2).max(0)`.
   i32 division truncates toward zero, so for s<b the quotient is ≤0 and `.max(0)`==0==the
   `if`-else 0; for s≥b both compute `(s-b)/2`. EQUIVALENT for all i32 (the plan's boundary
   table is correct). main_menu has no named `center_offset` (inlines the same `if` guard
   in tooltip/version/right_anchor) and the plan correctly leaves those inline sites
   untouched. EQUIVALENT.
5. **`right_panel_rects`** — main (147-181) and SP (90-124) are identical and use named
   consts. skirmish (447-476) inlines `168`/`199`/`.min(9)` and uses `SDBTNBKGD_H`(=42)
   where main/SP use `RIGHT_PANEL_WIDTH`(168)/`RIGHT_PANEL_TOP_H`(199)/
   `RIGHT_PANEL_TILE_COUNT_BASE`(9)/`RIGHT_PANEL_TILE_H`(42) — **same numeric values**. The
   only structural difference is the `bottom_h.max(0)` placement (main/SP clamp at
   assignment line 174/117, skirmish clamps inside `RectPx::new` line 474) — both clamp the
   identical expression `(screen_h - top_margin - bottom_y)`, so the output rect is
   identical. EQUIVALENT. Cross-checked against the existing test literals: 800×600 →
   bottom (632,577,168,23) [main test 461]; 1024×768 → bottom (744,661,168,23) [main test
   481]; 640×480 → tile_count 6, bottom (472,451,168,29) [matches skirmish 640 back_button
   y=409 = 199+(6-1)*42]. geom.rs `right_panel_rects_byte_equal_*` asserts exactly these.
6. **`lower_strip_rect`** — main (183-208) and SP (126-149) identical; skirmish has none.
   geom values cross-checked: 1024×768 → (112,652,632,32) [main test 482]. EQUIVALENT.

**The ONE genuine divergence that WOULD move pixels if mishandled** is `SDBTNANM` cell
width: 156 in main_menu (`layout.rs:97`) and skirmish (`layout.rs:6`), 168 in SP
(`layout.rs:15`). The plan handles this correctly: both snap fns take `cell_w: i32` and
each call site passes its shell's own const (SP passes 168, skirmish passes 156, main keeps
its local `SDBTNANM_CELL_W`=156). The plan's own Parity-Critical table row "SP 3 stacked
buttons" calls this out explicitly. A naive single-const share WOULD have shifted SP's 4
buttons from x=632/w=168 to x=644/w=156 — the plan avoids it. **Correctly handled, not an
issue.**

The two snap *algorithms* (main round-half-up `sdbtnanm_button_rect` 289-302 vs SP/skirmish
biased-truncate `owner_draw_button_snap_rect`) genuinely differ and are NOT proven
equivalent at the half-row boundary; the plan keeps them as two separate geom fns
(`snap_button_round_half_up`, `snap_button_biased_truncate`) and does NOT merge. Correct.

---

## geom.rs test-value spot checks (all pass against live test literals)

- `snap_biased_truncate_reproduces_skirmish_0x102_narrow_cells`: uses `dlu_rect(425,149,…)`
  and `dlu_rect(425,176,…)` → (644,241,156,42)/(644,283,156,42). Verified: skirmish
  `start_base=dlu_rect(425,149,108,23)` (layout.rs:522), `choose_base=dlu_rect(425,176,…)`
  (523); skirmish test asserts start_button (644,241,156,42)/choose (644,283,156,42)
  (layout.rs:887-888). MATCH.
- `snap_biased_truncate_reproduces_single_player_0x100_wide_cells`: `dlu_rect(425,122/149/
  176,…)` w=168 → (632,199/241/283,168,42). Matches SP test 269-271. MATCH.
- `snap_round_half_up_reproduces_main_menu_0xe2_stacked_cells`: dlu_y [125,152,179,206,233]
  → y [199,241,283,325,367] x=644 w=156. Matches main_menu buttons (layout.rs:339-355) and
  test (433). MATCH.
- `right_panel_rects_byte_equal_*` and `lower_strip_matches_*`: cross-checked above. MATCH.

---

## False positives caught (claims that look risky but are actually fine)

- **FP1 — "SP would shift 12 px."** True only of a naive single-width share; the plan's
  `cell_w` param prevents it. Not an issue in the plan as written.
- **FP2 — adding `translate` to main_menu/SP `RectPx`.** Looks like a behavior change but
  is a pure additive method neither shell calls. Safe.
- **FP3 — demoting skirmish `pub const SDBTNANM_H`/`SDBTNBKGD_H`/`SDBTNANM_W` to private.**
  Safe: none of the three is in skirmish `mod.rs:11-30` `pub use`, and grep finds no
  external importer (the SP `SDBTNANM_H` hits are SP's own). No break.
- **FP4 — deleting main_menu `RIGHT_PANEL_TOP_H`/`RIGHT_PANEL_TILE_COUNT_BASE`.** Safe:
  used only inside the deleted `right_panel_rects`, and neither is in main_menu `mod.rs:7`
  `pub use` (only `RIGHT_PANEL_TILE_H`/`RIGHT_PANEL_WIDTH` are re-exported).
- **FP5 — render field access.** `app_main_menu_shell_render.rs:235-249` and
  `app_single_player_shell_render.rs:251-265` read `layout.right_panel.{top,tile,tile_count,
  bottom}`; the shared `geom::RightPanelRects` has the same all-`pub` fields → compiles
  unchanged. Confirmed.

---

## Recommended (cosmetic) adjustments before execution — optional

1. Task 0.6 `use` list: **drop `mul_div_round`** from the skirmish geom import (S1) — it has
   no caller after `dlu_rect` is removed, and importing it unused emits a warning.
2. Task 0.4 rationale for `RIGHT_PANEL_BOTTOM_H`: correct the note to "unused `pub const`,
   harmless to keep" (S2). No code change needed.
3. Grounding-summary/Task-0.6 prose: fix the `state/player_name.rs:10`/`trackbars.rs:10`
   `RectPx`-consumer line cites (S3) — the `pub use` mechanism is still required (hit_test.rs:7,
   combos.rs:9 DO import it), so the FIX is correct; only the citation is stale.

Neither (1) nor (2)/(3) affects a single pixel or blocks compilation. Verdict stands:
**READY**.

---

## Tree-state note (parallel sessions)

`git status` shows `src/ui/main_menu_shell/layout.rs` as ` M` (modified, unstaged) in the
working tree. The version read this run is internally consistent (all fns/tests coherent),
so it does not look mid-edit — but a parallel session may touch it. The other three target
files (`single_player_shell/layout.rs`, `skirmish_shell/layout.rs`, `ui/mod.rs`) are clean;
`src/ui/shell/` does not exist yet. Per the plan's own instruction, re-anchor edits on
function/const NAMES (not line numbers) at execution time, especially in
`main_menu_shell/layout.rs`.
