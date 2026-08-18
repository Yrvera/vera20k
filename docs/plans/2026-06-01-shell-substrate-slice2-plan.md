# Shell Substrate Slice 2 — DialogController input authority (REVISED, post-review)

**Scope:** migrate the INPUT of dialog 0xE2 (main menu) + 0x100 (single player) onto one
shared `ui::shell::controller::DialogController`. Skirmish (0x102) untouched (Slice 4).
Render untouched (Slice 3). No INI/geometry changes.

Planning workflow `shell-slice2-plan` (10 agents) verified current code, drafted, and
adversarially reviewed. Judge verdict on the first draft: **NO-GO** — 6 required fixes +
2 open questions. This revision folds them in.

## Design (what gets built)

New file `src/ui/shell/controller.rs` — render-agnostic, imports only
`super::{descriptor, layout, geom}` + std (honors ui→sim-only layering):

- `DialogController { stack: Vec<DialogInstance>, kbd_route: Vec<DialogId> }` persisted in
  `AppState.shell_controller`. Per-dialog input state: `pressed/hovered/hover_started_at`
  (`Option<u16>`/`Instant`) + a `disabled: BTreeSet<u16>` runtime override.
- Input API: `on_pointer_down/up/move(x, y, &[LaidOutControl]) ` and `on_pointer_up ->
  Option<u16>` (the activated control id). `ensure_active(id)` makes the controller follow
  the app's active-shell flags (resets input state only when the dialog actually changes,
  so press state survives a down→up gesture). `set_disabled(id, bool)`, `pressed/hovered/
  hover_started_at/top_id` accessors.
- Lifecycle API (C1/C3/C5, unit-tested; forward-looking for Slice 4/5): `push/pop/reset_to`
  + `kbd_route()` registration-order list + `on_key` (returns false — no migrated shell
  registers keyboard controls; the app keeps owning Esc).

**The controller is fed BUTTON-only rects** (`layout.buttons`, adapted to `LaidOutControl`),
never `layout_pass`'s full control list. So statics (0x694 heading, 0x71b website) are never
hit-tested or hover-tracked, and `pressed` is always a button — this is how fixes #4/#5 are
met, more robustly than storing a descriptor for kind-lookup.

The legacy `MainMenuShellState`/`SinglePlayerShellState` fields (pressed/hovered/
hover_started_at/load_saved_game_enabled) are KEPT and **mirrored** from the controller after
each event so render (Slice 3) is untouched. The two `handle_*_shell_action` handlers are
**untouched**: the shim maps the activated control id → existing per-shell enum via new
`from_resource_id` + existing `action_for_control`, then calls the existing handler.

## The 6 required fixes (all verified against current code)

1. **Two hit-tests.** Press path (`on_pointer_down/up`) skips disabled ids; hover path
   (`on_pointer_move`) does NOT — current SP `state.rs:91` hover-tracks a *disabled* Load
   Saved Game (drives its tooltip/timer). Folding the enable filter into hover would drift.
2. **No SP descriptor needed / no 0xE2 descriptor exposure needed.** Resolved by feeding
   button-only rects (above) instead of the plan's "store descriptor + feed layout_pass".
   (Supersedes draft fixes #2/#3 — the underlying static-exclusion concern is met directly.)
3. **Button-only `pressed()`** — automatic: only button rects are fed, so `pressed`/the
   0xE2 down-sound never trip on the 0x71b website static. Sound still fires on Down.
4. **Static exclusion for click AND hover** — automatic (statics not fed).
5. **SP enable override current at Up** — `set_disabled(0x689, !load_saved_game_enabled)` is
   re-applied at the top of each SP down/up handler (before `on_pointer`); `ensure_active`
   only resets on dialog change, never mid-gesture, so the override persists down→up.
6. **Press-must-match-release** lifted verbatim: `on_pointer_up` takes `pressed`, requires
   `pressed.is_some() && pressed == released`, then re-applies the disabled guard — byte-
   identical to both shells (`main_menu state.rs:114-129`, `single_player 98-116`).

## 2 open questions — resolved (conservative defaults)

- **Q1 (merge handlers?)** → NO. Controller emits the activated control id; the shim maps to
  the existing per-shell enum and calls the existing `handle_*_action` byte-identical. Zero
  edits to the collision-hot 1641-1704 block (safer than the draft's "merge").
- **Q2 (retire legacy structs?)** → NO. Keep + mirror this slice; retiring is Slice 3 (render
  reads them).

## app.rs edits (contained; routers + handlers + helpers untouched)

- `AppState.shell_controller: DialogController` field + `::default()` init.
- Rewrite the 5 dispatch fns (1494-1566) + add `handle_main_menu_shell_mouse_move`; the inline
  cursor-move block (1964-1973) calls it. Each: build button rects → `ensure_active` →
  (SP only) `set_disabled` → `on_pointer_*` → mirror → (down) sound / (up) dispatch.
- `MainMenuControlId`/`SinglePlayerControlId`: add `resource_id`/`from_resource_id`.
- Delete `mouse_down/mouse_move/mouse_up/hit_test_owner_draw_button` from both `state.rs`
  (+ their now-stale tests, replaced by controller-integration tests using real
  `compute_layout` geometry) and drop them from both `mod.rs` re-exports.

## Acceptance

- `cargo build -p vera20k` clean. Controller unit tests (synthetic rects): press-match,
  stack push/pop/focus-restore, kbd registration order, ensure_active-resets-on-change.
  Real-geometry integration tests in both `state.rs`: 0xE2 hit/press at 700,210/700,537;
  SP hit at 639,204/290/540; SP disabled Load Saved Game suppresses press but still hovers.
- In-game (user): main menu + single player look and click EXACTLY as before; SP Skirmish
  enters skirmish-from-SP; Back returns to 0x100; Exit opens the confirm modal.
