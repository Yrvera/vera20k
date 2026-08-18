# Slice 5 (menu modals) kickoff prompt — paste into a fresh session

## RESUME STATE (2026-06-01) — sub-steps 1, 2, 4a DONE + committed on `dev`; START AT 4b

- `d355d49` — **1+2**: new `src/ui/shell/modal.rs` (ModalKind/template-id table, native
  count-rule, `ModalResult` both conventions, count-rule descriptor builder,
  `quit_confirm_layout`) + `paint_modal_shp` mode-2 emitter in `src/render/shell_paint.rs`
  (PUDLGBGN frame 0 + MNBTTN type-3 0=up/1=disabled/2=pressed; pressed→2 correct by
  construction; now **N-button** — takes `&[ModalButton]`). No caller; zero behaviour.
- `b3d3923` — **4a**: quit-confirm egui card → 0x120 SHP modal (PUDLGBGN + MNBTTN OK/Cancel)
  over the main-menu shell. Input via shared `shell_controller` (`ensure_active(0x120)` +
  OK/Cancel feed, press-must-match-release); OK→quit (existing exit path), Cancel/ESC→stay.
  Skirmish chrome atlas loaded on demand on open. Files: `src/app.rs`,
  `src/app_main_menu_shell_render.rs`.

**4a in-game verification: PENDING** — confirm PUDLGBGN/MNBTTN composition + pressed frame,
OK quits / Cancel+ESC stay, body-text placement, AND whether gamemd draws a blank button in
the OK↔Cancel gap (the template's unpopulated `0x5AF`, currently NOT rendered — add it if the
original shows it).

**Verified-from-binary this session (load-bearing for 4b):**
- Generic message-box helper `FUN_005D3490(body, ok, third, fourth)`. Slot→control: body→`0x5B0`,
  ok→`0x5AE`(y=135), third→Cancel control `2`(y=175), fourth→`0x5AF`(y=155). Quit calls it with
  (GUI:ExitAreYouSure, TXT_OK, GUI:Cancel, NULL); `0x5AF` left untouched (no `SW_HIDE`). Result
  inits −1, OK→0. 0x120 template (DLU): dialog (0,0,300,200)=450×325px; body (40,40,220,50);
  OK (207,135,83,15); Cancel (207,175,83,15); 0x5AF (207,155,83,15).
- 4b cascade (plan §4): on OK (result 0) → `WriteToINI` to **RA2MD.INI**
  ([Options]/[Video]/[Audio]/[Network]) before teardown → state 7 → music stop
  `FUN_00720EA0(1)` → vox-pump wait **≤0xBB8 GetRadarTimer ticks** gated on
  `VoxClass__PumpAndCheckActive` → `FUN_00720EA0(0)` → fade `FUN_004A3C30(0)` → RET. No
  PostQuitMessage/ExitProcess. (Encode the cap as ticks gated on the active-check, NOT 3000 ms.)

**Architecture notes (keep consistent in sub-step 6):**
- The modal rides the shared `shell_controller` via `ensure_active(0x120)`; the menu mouse-move
  handler is gated off while `state.exit_confirm_modal.is_some()` so the gesture isn't reset.
- `draw_main_menu_dialogs(state, render_exit_confirm_egui)` — `true` ONLY on the degraded egui
  fallback path; the SHP shell + single-player paths pass `false`. The egui exit-confirm
  (`main_menu_dialogs.rs`) is intentionally KEPT for the fallback (it has no SHP shell), so
  §8.2's "remove the egui field" is partially deferred.

**Remaining this slice:** **4b** (settings-persist + graceful cascade on the OK path, facts
above), then **6** (migrate skirmish validation modal to `paint_modal_shp` + route via
`DialogController`; fix `modal_button_mnbttn_frame_index` in `app_skirmish_shell_render/chrome.rs`
pressed→2 and update the render-side test `app_skirmish_shell_render.rs` ~969-970 — NOT the
safety net; §8.1 deletion set; the 87-test safety net stays UNCHANGED/GREEN). 5a/5b → Slice 5b.
The original "START: sub-step 1" line at the bottom is superseded — begin at 4b.

---

GOAL: Implement the **menu-modal half** of shell-substrate Slice 5 — fold the quit-confirm
(0x120) and skirmish start-validation (0xCE) pop-ups onto the shared `ui::shell` substrate,
pixel/behavior INDISTINGUISHABLE from gamemd.exe, and fix the latent MNBTTN pressed-frame bug
on the way. Package is `vera20k`. The in-game Options dialog (full 0xBBB/0xF5 shell dialog) and
the sim-behind-modal pump are deliberately SPLIT OUT into Slice 5b (see the 5b stub kickoff) —
do NOT implement them here.

SOURCE OF TRUTH — READ FIRST:
- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md` — the full, fact-grounded plan (resolution
  pass; both reviews READY, zero open questions). It documents ALL 6 sub-steps, but **THIS slice
  executes only sub-steps 1, 2, 4a, 4b, 6** (skip 3, 5a, 5b → Slice 5b). Every binary fact below is
  cited there; treat the plan as authoritative.
- `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md` — substrate contracts (C2/C13) + §7 retire list.
- Recipe from prior slices: `docs/plans/2026-06-01-shell-substrate-slice2-plan.md`, `...-slice3-plan.md`.
- Substrate modules: `src/ui/shell/{geom,descriptor,layout,controller}.rs`, `src/render/shell_paint.rs`.

ALREADY DONE (confirm with `git log --grep="substrate Slice" --oneline` on `dev`):
- Substrate Slices 0–3 committed (geom, descriptor+layout, DialogController input, owner-draw paint).
- The C13 template-id pre-req and ALL prior UNCHECKED facts are RESOLVED and PROOFED in the plan
  (no Ghidra needed to start). Re-verify a binary fact only if you are about to depend on a label/address;
  the plan cites the exact Ghidra call for each.

SCOPE — sub-steps to execute (detail + citations in the plan; re-anchor by CONTENT, line numbers drift):
1. **`ModalKind` + template-id table** in NEW `src/ui/shell/modal.rs` (declare `pub mod modal;` in
   `src/ui/shell/mod.rs`). Enum {BodyOk 0xCE, Confirm 0x120, ThreeButton 0x121 (render-untested — keep or
   drop), InGameOptions 0xBBB/0xF5}, `template_id(in_active_game)`, the count-rule descriptor builder, and
   `ModalResult` carrying BOTH result conventions (plan §1.2). Render-agnostic; no sim/render/assets deps.
   Unit tests for the table + count rule. Zero behavior change.
2. **`paint_modal_shp`** in `src/render/shell_paint.rs` — mode-2 SHP composition: PUDLGBGN.SHP frame 0 +
   DIALOGN.PAL background, MNBTTN.SHP owner-draw type-3 frames **0=up / 1=disabled / 2=pressed**, body/OK
   labels after art. Unit-test that pressed → frame 2. No caller yet.
4a. **Quit-confirm UI swap** — replace the egui exit-confirm card with `ModalKind::Confirm` (0x120) via
   `paint_modal_shp` + `DialogController`. Wire results: OK `0x5AE` → **0 = quit**, Cancel control `2` →
   **1 = stay**, **ESC = Cancel**. Exit still occurs the existing way for now.
4b. **Quit-confirm persist + cascade** — on result 0, write the settings INI to **`RA2MD.INI`** (sections
   `[Options]`/`[Video]`/`[Audio]`/`[Network]`) **before** teardown, then reproduce the graceful cascade
   (music stop → vox-pump wait → fade → return). The vox-pump wait is a **cap of `0xBB8` radar ticks**
   (GetRadarTimer = timeGetTime()>>4, ~16 ms/tick) gated on a "voices still active" check — encode it as
   ticks gated on the active-check, NOT as a 3000 ms literal.
6. **Migrate the skirmish validation modal (LAST)** — re-point `push_validation_modal_instances`
   (`src/app_skirmish_shell_render/modals.rs`) to `paint_modal_shp`; route its input through
   `DialogController` instead of the bespoke `app.rs` handlers (use the plan §8.1 line-exact deletion set,
   re-anchored by content); and **fix the latent MNBTTN bug**: `modal_button_mnbttn_frame_index`
   (`src/app_skirmish_shell_render/chrome.rs`) currently maps pressed → 1 (disabled) — correct to pressed
   → 2, and update the render-side test in `src/app_skirmish_shell_render.rs` (NOT the safety net).

EXCLUDED from this slice (→ Slice 5b): sub-step 3 (pure pump decision + `service_tick` + `SessionMode`),
5a (in-game Options full-dialog chrome from the parsed 0xBBB/0xF5 layouts), 5b (`service_tick` swap +
battlefield-frozen assertion). The first batch needs NO pump work.

KEY VERIFIED FACTS to honor (do not re-introduce the old errors — all PROOFED, cited in the plan):
- Message-box result mapping is **OK → 0, Cancel(control 2) → 1, 0x5AF → 2** (an earlier draft had OK/Cancel
  inverted). Quit caller consumes 0 = QUIT, non-zero = stay.
- Quit path: 0xE2 Quit (ctrl `0x3EE`) → `FUN_00531CC0`=6 → `Main_Game` case 6 → message-box helper, template
  0x120, body `GUI:ExitAreYouSure` / `TXT_OK` / `GUI:Cancel` / NULL. INI write strictly precedes teardown; clean
  return cascade, no PostQuitMessage/ExitProcess.
- Settings filename is **`RA2MD.INI`** (uppercase). ESC on the 0x120 modal resolves to **Cancel** (only the OK
  click quits).
- In-game Options is a SEPARATE full-shell dialog (Slice 5b) — `ModalKind::InGameOptions` exists in the table
  from sub-step 1, but its render/input/pump is NOT wired in this slice.

HARD RULES / CADENCE (do not break):
- **Re-anchor by CONTENT, not line numbers** — parallel human sessions edit `src/ui/*` and `src/app_*`; the
  plan's line citations may have drifted. Confirm each target by symbol/content first. If the build fails in
  files you did NOT touch, suspect a parallel session — do not fix unrelated code.
- **ONE sub-step per cycle.** After each: run `cargo build -p vera20k` then `cargo test -p vera20k` as a
  SEPARATE bounded foreground pass (NOT inside a workflow); read the literal `test result:` line before
  reporting pass/fail.
- **The skirmish safety net `src/ui/skirmish_shell/state/tests.rs` (2147 lines, 87 tests) must stay GREEN and
  UNCHANGED.** No edit to that file is in scope. If a sub-step would require changing a safety-net test, STOP
  and re-scope.
- **`sim/` never depends on `ui/`/`render/`.** `modal.rs` is render-agnostic.
- Build clean is necessary, NOT sufficient — after a sub-step (or the whole slice) STOP and ask for in-game
  verification (PUDLGBGN/DIALOGN/MNBTTN composition with the corrected pressed frame; quit-confirm order +
  ESC=Cancel) before committing. Pixel parity is the manual in-game gate (no screenshot-diff yet).
- **Commit to `dev`** (feature work commits directly to `dev`), each sub-step or the whole slice as its own
  commit, only after build/test green AND the in-game OK.

START: re-anchor sub-step 1's targets by content (`src/ui/shell/mod.rs` exports; `descriptor.rs` `BgKind::ModalShp`
+ `RepositionPolicy::ModalCentered`; `controller.rs` `on_key`), then implement sub-step 1 (the pure `ModalKind`
table — zero behavior change), build + test as a separate pass, and report before moving to sub-step 2.
