# Slice 5 — resume kickoff (sub-step 6, then 5) — paste into a fresh session

STATUS: Slice 5 sub-steps **1, 2, 3, 4 DONE + committed on `dev`** (4 = 4a + 4b-i + 4b-ii-a +
4b-ii-b, the full graceful-quit cascade). Sub-step **6 STARTED** — the MNBTTN pressed→2 fix is
committed. **START AT 6a** (validation-modal render re-point), then **6b** (input migration). Package
`vera20k`. Parity bar: indistinguishable from gamemd.exe. Ultracode style: use workflows for
fan-out / verification / adversarial review; run cargo build/test as a separate FOREGROUND pass.

This session's commits on `dev` (your baseline — `git log` to confirm they're still the tip, another
session may have added sim work on top):
- `31cbacb` 4b-i — RA2MD.INI ScoreVolume persist (in-place writer)
- `1be4e2f` 4b-ii-a — audio quit cascade (music fade + voice wait + hard stop)
- `87a7e59` 4b-ii-b — screen fade-to-black
- `7682611` sub-step 6 partial — MNBTTN pressed-frame fix

## SOURCE OF TRUTH — READ FIRST
- **`docs/plans/2026-06-01-shell-substrate-slice5-substep6-handoff.md`** — the authoritative,
  re-anchored guide for sub-step 6: current line numbers (the `§8.1` deletion set, render path,
  DialogController), the exit-confirm `0x120` mirror, the 4 design decisions, the safety-net
  constraints, and the 6a/6b sub-stepping. **Start here.**
- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md` — the slice plan (§1, §2.1, §4.1, §8.1). Its
  app.rs line citations are STALE — trust the handoff's numbers.

## FIRST STEP — re-anchor
The handoff's line numbers were current on 2026-06-01 but app.rs lines **drift with every commit**
(function NAMES are stable). Re-grep each target before editing; if several commits landed since, run a
quick read-only re-anchor workflow (parallel locators) like the one that produced the handoff.

## 6a — render re-point (DO FIRST; render-only, safe, no input/substrate changes)
- Re-point `push_validation_modal_instances` (`src/app_skirmish_shell_render/modals.rs`) +
  `push_validation_modal_text_draws` (`src/app_skirmish_shell_render/text.rs`) to
  `shell_paint::paint_modal_shp`. Mirror `build_exit_confirm_modal_overlay`
  (`src/app_main_menu_shell_render.rs`). Reconcile the two-pass sprite/text split into the combined
  `ModalDraw` — or keep the split and call `paint_modal_sprites` + `paint_labels_at_depth`.
- Delete the now-dead chrome MNBTTN fns (`modal_button_mnbttn_frame_index`/`_entry`/`_position`,
  `push_modal_button_mnbttn` in `src/app_skirmish_shell_render/chrome.rs`); update the render tests.
- Verify: build + render tests + in-game (validation modal renders; held OK shows the pressed frame).

## 6b — input migration (THEN; has the substrate design decision)
- Route validation-modal mouse through `DialogController` (`DialogId(0x00CE)`, single OK
  `control::OK = 0x05AE`) mirroring the exit-confirm 0x120 handlers (`exit_confirm_modal_feed`,
  `handle_exit_confirm_modal_mouse_down`/`_up`).
- Implement a `DialogController::on_key` consumer (`src/ui/shell/controller.rs:192` returns `false`
  today) so an open modal swallows Enter/Escape → `dismiss_validation_modal`. **SHARED substrate** —
  verify it does NOT double-handle the already-migrated exit-confirm's Esc=Cancel.
- Apply the `§8.1` deletion set: delete the 4 bespoke handlers, re-route the 2 mouse call-sites + the
  keyboard call-site, retire ONLY the validation half of the Esc branch (keep the choose-map half —
  that's Slice-4).
- **KEEP (adapt, do NOT delete)** the board-block mouse-move/wheel guards — deleting them lets the
  skirmish board scroll/hover behind an open modal (the plan's literal "remove guards" is wrong).
- Remove/repoint the **app.rs-internal test** that calls the deleted `is_validation_modal_dismissal_key`
  (it's a `cargo test` gate, separate from the protected safety net).
- Verify: build + safety net + in-game (OK / Enter / Escape dismiss; board frozen behind the modal).

## HARD RULES (cadence — unchanged)
- ONE sub-step per cycle (6a, then 6b). `cargo check -p vera20k` then `cargo test -p vera20k` as a
  separate FOREGROUND pass; read the literal `test result:` line before reporting.
- The **87-test skirmish safety net** `src/ui/skirmish_shell/state/tests.rs` stays GREEN + UNCHANGED:
  keep the `validation_modal` field, the `SkirmishValidationModalState` struct, and
  `dismiss_validation_modal` exactly as-is. If a change would touch the net, STOP and re-scope.
- `sim/` never depends on `ui/`/`render/`.
- Build clean is necessary, not sufficient — **STOP for the user's in-game OK before committing** each
  sub-step to `dev`.
- After implementing each sub-step, run a short **adversarial review workflow** (especially 6b's
  `on_key` consumer + the guard reconciliation + no exit-confirm regression).

## PARALLEL SESSIONS
Another session owns the sim **"Slice 6"** (deferred-delete / two-phase death). If `sim/production/`
or `sim/world/` show as modified, or the build fails in files you did not touch, that's their
in-progress work — do NOT fix/revert/stash it; continue your own.

## AFTER sub-step 6 — only sub-step 5 remains in Slice 5
**Sub-step 5 = the in-game Options modal** (full-shell `0xBBB` active / `0xF5` shell chrome, own proc
selection via `g_GameActive==1`, INI-write-on-OK). It is the BIG one — standard Win32 owner-draw
control family (trackbars/checkboxes), NOT `paint_modal_shp`. Start it with `/brainstorm`; it has real
open items (DLU→pixel projection, owner-draw asset confirmation — plan §0.1/§4.2/§9). Give it its own
session; it is not a quick win.

## OPTIONAL pending check
The committed MNBTTN fix could use a quick in-game confirm: trigger a skirmish validation-error modal
(start a skirmish with an invalid config), hold the OK button, and confirm it shows the pressed cameo
(not the disabled frame).
