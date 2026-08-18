# Slice 5 sub-step 6 — validation-modal migration HANDOFF (re-anchored)

> **SUPERSEDED — sub-step 6 COMPLETE (verified 2026-06-17 against current code).** The substantive
> migration has landed: **6a render re-point** — `push_validation_modal_instances`
> (`src/app_skirmish_shell_render/modals.rs:187`) now delegates to the modal substrate
> (`shell_paint::paint_modal_sprites` with `ModalButtonFrames`/`ModalButton`), not the old chrome
> mnbttn path; **6b input migration** — `src/app.rs` routes the validation modal through
> `DialogController` (`route_validation_modal_key` / `route_validation_modal_mouse_down` /
> `route_validation_modal_mouse_up` → `close_validation_modal_from_controller` →
> `dismiss_validation_modal`), and the bespoke `handle_validation_modal_*` /
> `is_validation_modal_dismissal_key` handlers are gone. The 87-test skirmish safety net is intact.
> All Slice 5 sub-steps (3, 4a/4b, 5a-i/ii/iii, 6, 5b) are now in. The status line and line citations
> below are HISTORICAL and stale — kept for reference only.

STATUS (HISTORICAL — stale): **MNBTTN pressed→2 fix DONE + committed** (`fix(ui): modal MNBTTN button
shows pressed frame…`). The rest of sub-step 6 — the render re-point + the input migration — REMAINS.
All line numbers below were re-verified against current code this session (post sub-steps 4a/4b/4b-ii,
which shifted the plan §8.1 citations by +5 to +110 lines). Package `vera20k`. Parity bar:
indistinguishable from gamemd.exe. **Function NAMES are stable; re-grep before trusting the line
numbers if more commits land.**

The authoritative slice plan is `docs/plans/2026-06-01-shell-substrate-slice5-plan.md` (§1, §2.1, §4.1,
§8.1) — but its app.rs line citations are STALE; use the re-anchored numbers here.

## Recommended sub-stepping (each independently buildable; safety net stays GREEN + UNCHANGED)

- **6a — render re-point** (render-only, no input/substrate changes): re-point
  `push_validation_modal_instances` + `push_validation_modal_text_draws` to `paint_modal_shp` (this
  ALSO re-fixes the MNBTTN frame via the substrate's correct mapping and makes the old chrome mnbttn
  fns dead → delete them). Verifiable: build + the render tests + in-game (the validation modal still
  renders correctly).
- **6b — input migration**: route validation-modal input through `DialogController` (mirror the
  exit-confirm `0x120`), implement the `on_key` consumer, apply the §8.1 deletion set, reconcile the
  board-block guards, fix the app.rs-internal test. Verifiable: build + safety net + in-game
  (modal OK/Enter/Escape dismiss; board frozen behind the modal).

## DESIGN DECISIONS to resolve (surfaced by the re-anchor — the plan understated these)

1. **`DialogController::on_key` returns `false` today** (`src/ui/shell/controller.rs:192-194`). Routing
   Enter/Escape "through on_key" (plan §3.3) requires giving it a real consumer that returns `true`
   (consumes) when a modal is active and dismisses on Enter/Escape. This is SHARED substrate — it also
   affects the already-migrated exit-confirm `0x120`. Verify the exit-confirm's Esc=Cancel still works
   and isn't double-handled (exit-confirm Esc is currently handled in the main-menu Esc path, not via
   on_key). Decide: a generic on_key consumer vs a validation-scoped one.
2. **KEEP the board-block guards — do NOT delete them.** The plan §8.1 says "remove 2 in-fn guards"
   (mouse_move 1468-1473, mouse_wheel 1489-1491), but those guards suppress *board* input while the
   modal is open. Deleting them outright lets the skirmish board scroll/hover behind an open modal — a
   regression. Preserve the `validation_modal.is_some()` → block-board behavior; only the bespoke
   button HANDLERS are replaced by DialogController routing.
3. **Render is a TWO-PASS split** (sprites + text in separate passes), unlike exit-confirm's single
   `build_*_overlay`. Choose: (a) add a `build_validation_modal_overlay` returning `ModalDraw` and
   rewire both passes (matches the exit-confirm mirror), or (b) keep the split and call
   `paint_modal_sprites` for sprites + `paint_labels`/`paint_labels_at_depth` for text. (b) is less
   invasive.
4. **app.rs-internal test breaks on deletion.** `src/app.rs` `mod tests` (starts ~3282) has
   `validation_modal_dismissal_keys_match_dialog_translation` (~3285-3291) calling
   `App::is_validation_modal_dismissal_key`. Deleting that fn breaks the build — remove or re-point this
   test alongside. (This is NOT the protected 87-test safety net, but it IS a `cargo test` gate.)

## SAFETY NET — keep UNCHANGED (`src/ui/skirmish_shell/state/tests.rs`, 87 #[test])

It references the `validation_modal` field, `SkirmishValidationModalState::new`, and
`dismiss_validation_modal` (tests.rs:1245-1254). So KEEP all three intact:
- `validation_modal: Option<SkirmishValidationModalState>` field — `src/ui/skirmish_shell/state/player_name.rs:256` (decl), `:302` (Default `None`).
- `SkirmishValidationModalState { message, ok_button, ok_button_pressed }` + `::new(message, ok_button)` — `src/ui/skirmish_shell/state.rs:160-175`.
- `dismiss_validation_modal(state) -> bool` = `state.validation_modal.take().is_some()` — `player_name.rs:399-401`. The migrated path MUST still call this on OK/Enter/Escape.
If any change touches the field name/type, the struct, or this helper's signature, **STOP** — that
contradicts the slice constraint. (`ok_button_pressed` press state should be tracked via
DialogController at runtime instead of mutated by app.rs handlers, but the field stays for the tests.)

## RE-ANCHORED locations (current this session)

### §8.1 deletion set — `src/app.rs`
| Item | Current lines | Action |
|---|---|---|
| `is_validation_modal_dismissal_key` | 1311-1316 | delete (→ on_key) |
| `handle_validation_modal_key_input` | 1318-1328 | delete (→ on_key + `dismiss_validation_modal`) |
| `handle_validation_modal_mouse_down` | 1330-1345 | delete (→ `ensure_active(DialogId(0x00CE),true)` + `on_pointer_down`) |
| `handle_validation_modal_mouse_up` | 1347-1372 | delete (→ `on_pointer_up`; on `Some(id)==control::OK` call `dismiss_validation_modal`) |
| mouse_move guard | 1468-1473 | **KEEP** board-block (adapt; do NOT delete) |
| mouse_wheel guard | 1489-1491 | **KEEP** swallow |
| mouse_down call-site | 1374-1377 | re-route |
| mouse_up call-site | 1425-1428 | re-route |
| key-input call-site (KeyboardInput arm) | 2094-2100 | re-route → `on_key` |
| Esc branch (validation half) | 2102-2111 (validation `||` on line 2104) | modify — retire ONLY the validation half; route Esc via on_key |
| app.rs-internal test | 3285-3291 | delete/repoint (see decision 4) |
**Slice-4 — DO NOT TOUCH:** every `choose_map_modal` sibling — mouse-move guard 1461-1467, mouse-wheel
guard 1492-1494, mouse-down call-site 1378-1380, mouse-up dispatch 1429-1432, Esc half on line 2103.
`is_escape` def at 2075-2076 stays (used by other Esc paths).
**KEEP (not in deletion set):** validation_modal clear sites `close_native_skirmish_shell` (app.rs:548),
return-from-skirmish (app.rs:602); show path `show_skirmish_validation_modal` (879-891) +
`skirmish_validation_modal_for_error` (844-877), triggered from StartGame Err (800-808).

### Render path
- `push_validation_modal_instances` (sprite emitter) — `src/app_skirmish_shell_render/modals.rs:171-208`; called at `src/app_skirmish_shell_render.rs:375`. Emits PUDLGBGN bg (or solid-rect fallback) + one MNBTTN OK via `push_modal_button_mnbttn`. **No text.**
- `push_validation_modal_text_draws` (text emitter) — `src/app_skirmish_shell_render/text.rs:898-924`; called at `src/app_skirmish_shell_render.rs:573`. Body (`ShellAlign::NONE`) + OK (`H_CENTER|V_CENTER`), both `SHELL_LABEL_TEXT_RGB`, via `state.bit_font`.
- TARGET `paint_modal_shp` — `src/render/shell_paint.rs:464-476` → `ModalDraw{sprites,text}`. Support: `ModalButtonFrames` 386-401, `ModalDepths` 407-411, `ModalDraw` 414-417, `ModalButton` 424-428, `modal_button_frame_index(pressed,enabled)` 373-381 (canonical 0=up/1=disabled/2=pressed), `paint_modal_sprites` 435-460, `paint_labels_at_depth`.
- MIRROR `build_exit_confirm_modal_overlay` — `src/app_main_menu_shell_render.rs:631-690`; `EXIT_CONFIRM_MODAL_DEPTHS` 620-624; press-sink `modal_button_label_rect` 695-706.
- After re-point, DELETE the now-dead chrome mnbttn fns — `src/app_skirmish_shell_render/chrome.rs`: `modal_button_mnbttn_frame_index` (321-326, just fixed), `modal_button_mnbttn_position` (325-335), `modal_button_mnbttn_entry` (337-346), `push_modal_button_mnbttn` (348-…). (Grep-confirmed: only caller is modals.rs:201.)
- Layout: `ValidationModalLayout { screen, dialog, message, ok_button }` — `src/ui/skirmish_shell/layout.rs:189-194` (field is `.message`, not `.body`); `compute_validation_modal_layout(render_w, render_h)` at `layout.rs:749-761`; OK rect = `dlu_rect(207,175,83,15)`.
- Atlas (`src/render/skirmish_shell_chrome.rs`): `validation_modal_background_pudlgbgn` :52 (`pudlgbgn.shp#0` :367); `modal_button_mnbttn_frame0/1/2` :59-61 (`mnbttn.shp#0/1/2` :374-376).
- Depth bands to preserve: sprites in `SHELL_DROPDOWN_DEPTH`(=0.00034) family (bg −0.00014, button −0.00016); text in `SHELL_DROPDOWN_TEXT_DEPTH`(=0.00029) family (body −0.00012, OK −0.00013).

### DialogController + modal-id facts
- `src/ui/shell/controller.rs`: `ensure_active(DialogId, accepts_keys: bool)` 107-111 (pass `true`); `on_pointer_down(x,y,&[LaidOutControl])` 149-153; `on_pointer_up(x,y,&feed) -> Option<u16>` 158-175 (fires only when release matches press, control enabled); `on_pointer_move` 179-187; `on_key(ShellKey) -> bool` 192-194 (**returns false — needs consumer**); `ShellKey` enum 27-32; `pressed()`/`hovered()` 119-129. Lives on `state.shell_controller`.
- `src/ui/shell/modal.rs`: `control::OK = 0x05AE`, `control::CANCEL = 0x0002`, `control::BODY_STATIC = 0x05B0`; `ModalKind::BodyOk = 0xCE`; `build_message_box_descriptor(ModalKind::BodyOk, &rects)` 185-213 → `[BODY_STATIC, OK]`. Use `DialogId(0x00CE)`. The 0xCE feed has ONLY the OK control (no Cancel/0x5AF).
- Exit-confirm routing to mirror (app.rs): `exit_confirm_modal_feed` 1619-1636, `handle_exit_confirm_modal_mouse_down` 1672-1680, `handle_exit_confirm_modal_mouse_up` 1682-1707 (match `Some(id)==control::OK`/`control::CANCEL`).

### OK-click behaviour to preserve
On a valid OK click (or Enter/NumpadEnter/Escape) the bespoke path calls `dismiss_validation_modal`
(closes the modal, no other side effect; skirmish-validation callers ignore the result — plan §1.2).
The migrated DialogController path must do the same: `on_pointer_up` → `Some(OK)` OR `on_key` consuming
Enter/Escape → `dismiss_validation_modal(&mut state.skirmish_shell_state)` + `request_redraw()`.

## Cadence (unchanged)
ONE sub-step per cycle (6a then 6b). `cargo check -p vera20k` then `cargo test -p vera20k` as a
separate foreground pass; read the literal `test result:` line. `sim/` never depends on `ui/`/`render/`.
STOP for the user's in-game OK before committing each sub-step. Run an adversarial review workflow after
implementing each (especially 6b's on_key + guard reconciliation). The 87-test skirmish state safety net
stays GREEN + UNCHANGED — if a change would touch it, STOP and re-scope.
