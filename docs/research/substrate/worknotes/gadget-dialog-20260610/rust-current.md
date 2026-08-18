# Rust-current lane — UI architecture vs the two-framework substrate model

Lane: CURRENT RUST SIDE of the gadget/dialog substrate study (2026-06-10).
No Ghidra used in this lane (by assignment). All claims below are **VERIFIED-LIVE against the
working tree at commit 7b79a186 (branch `dev`)** by reading the cited file:line spans this
session, unless explicitly marked DOC-INHERITED or INFERRED. Binary-side statements (what
gamemd does) are DOC-INHERITED from the named docs/comments and are for the binary lanes to
confirm — they are *not* re-verified here.

Framework model (from the study prompt):
- **Framework A** — GadgetClass/LinkClass retained-mode gadget tree (in-game sidebar, radar
  chrome, tabs, tooltips).
- **Framework B** — Win32 RT_DIALOG shell dialogs with owner-draw subclassed controls (main
  menu, skirmish setup, options, modals).

---

## 1. ARCHITECTURE MAP

Legend: SUBSTRATE = on the shared `ui::shell` substrate; AD-HOC = bespoke per-surface logic;
EGUI = egui-drawn client UI (no gamemd counterpart attempted); DEAD = no callers.

### 1.1 Framework B territory (front-end shells)

| Module | Role | Shadows (gamemd) | Classification |
|---|---|---|---|
| `src/ui/shell/geom.rs` (301 ln) | DLU→px, right-panel chrome rects, two button-snap algorithms, `RectPx` | dialog-unit conversion + the shells' re-anchor math | SUBSTRATE (Slice 0) |
| `src/ui/shell/descriptor.rs` (110 ln) | `DialogDescriptor`/`ControlDescriptor`/`ControlKind`/`AnchorRule`/`BgKind`/`RepositionPolicy` | RT_DIALOG template + per-control subclass classification (study §2.2/C6, C7 include-set) | SUBSTRATE (Slice 1) |
| `src/ui/shell/layout.rs` (211 ln) | `layout_pass`: one-shot DLU→px + per-control re-anchor; ModalCentered skips re-anchor | the fullscreen reposition pass (contract C7) | SUBSTRATE (Slice 1) |
| `src/ui/shell/controller.rs` (322 ln) | `DialogController`: LIFO dialog stack, kbd route in registration order, press-must-match-release, hover, runtime-disable | dialog-proc input authority + focus stack (contracts C1/C3/C5) | SUBSTRATE (Slice 2) |
| `src/ui/shell/modal.rs` (454 ln) | `ModalKind` table (0xCE/0x120/0x121/0xBBB/0xF5), count-rule template selection, two result conventions, quit-confirm layout | the generic CSF message-box helper + in-game Options own-proc | SUBSTRATE (Slice 5) |
| `src/render/shell_paint.rs` | ONE owner-draw paint emitter (chrome/buttons/labels/cursor + `paint_modal_shp`); `ButtonPolicy`+`ArtFit` carry per-shell differences as data | WM_DRAWITEM owner-draw painting + mode-1/mode-2 background compose (C8) | SUBSTRATE (Slice 3 + 5) |
| `src/ui/main_menu_shell/` (layout 434, state 233 ln) | 0xE2 descriptor table → `layout_pass`; control-id↔action↔return-code↔CSF tables | dialog 0xE2 template + its dialog proc's command table | SUBSTRATE (input + layout + paint) |
| `src/ui/single_player_shell/` (layout 174, state 181 ln) | 0x100 layout via shared geom helpers (NOT via descriptor table); id/action/result tables (8/9/0x0B/0x12) | dialog 0x100 + proc 0x52D640 result codes (per test name `command_results_match_dialog_proc_0x52d640`, state.rs:110) | SUBSTRATE for input+paint; layout = shared-geom AD-HOC (no descriptor) |
| `src/ui/skirmish_shell/` (layout 1301; state 295 + 5 submodules ≈ 2.2k ln + 2147 ln tests) | 0x102 full board: combos, trackbars, checkboxes, edit, choose-map modal 0x6B, validation modal, status help, reveal | dialog 0x102 + 0x6B + the owner-draw combo/trackbar/listbox subclasses | AD-HOC (Slice 4 NOT shipped; uses shared geom only). Exception: validation-modal input routes through `DialogController` (app.rs:1361-1392) |
| `src/ui/main_menu_dialogs.rs` (435 ln) | egui modals: exit-confirm (fallback only), Options, Movies&Credits, Campaign select | 0x120 quit (now SHP on the normal path), 0xF5 options, movies/campaign dialogs | EGUI placeholders (options/movies/campaign); exit-confirm egui only on degraded fallback (app.rs:1964-1975) |
| `src/ui/main_menu.rs` (391 ln) | egui main menu + loading screen | none (client-style fallback) | EGUI fallback (only when shell chrome fails: `render_egui_main_menu_fallback`, app.rs:748-789) |
| `src/app.rs` (3321 ln) | Host: winit event routing, shell handlers, controller calls, modal opens, quit cascade | the message pump + WndProc dispatch | AD-HOC host glue |
| `src/app_main_menu_shell_render.rs`, `src/app_single_player_shell_render.rs` | per-shell render callers (build PaintButton/PaintLabel feeds, submit passes) | per-dialog WM_PAINT order | AD-HOC callers of substrate paint |
| `src/app_skirmish_shell_render.rs` + `app_skirmish_shell_render/{chrome,controls,draw_order,modals,preview,text}.rs` | skirmish render: bespoke emitters + `skirmish_shell_semantic_draw_order` role list | 0x102 paint pass | AD-HOC (validation modal re-pointed to `paint_modal_shp` per commit 635423cd) |
| `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs` | slide/wave transition + input block during slide | shell slide (+0xD5 groups) | AD-HOC |
| `src/ui/skirmish_shell/static_reveal.rs` (137 ln) | kind-1 static text reveal cursor, 30 ms/step | the 0x4B2 text-reveal statics | SUBSTRATE-adjacent primitive (self-contained) |

### 1.2 Framework A territory (in-game)

| Module | Role | Shadows (gamemd) | Classification |
|---|---|---|---|
| `src/sidebar/mod.rs` (573 ln) | sidebar geometry, tabs, items, hit-test, actions | SidebarClass/StripClass + SBGadgetClass tree | AD-HOC (no retained gadget list) |
| `src/sidebar/sidebar_view.rs` (601 ln) | per-frame `SidebarView` builder from production state | StripClass recalc | AD-HOC |
| `src/sidebar/layout_spec.rs` (242 ln) + `sidebar_layout.ron` | RON-tunable chrome layout (`SidebarChromeLayoutSpec`), `with_scale` | (none — modern tunable) | AD-HOC, deliberately non-retail-pinned |
| `src/sidebar/gadget_flash.rs` (331 ln) | `GadgetFlash` (state/period/countdown/disabled) + `frame_select` 5-frame table + `SidebarGadgetState` | SBGadgetClass flash sub-struct +0x34/+0x38/+0x3C/+0x1E; Start_Flash/Stop_Flash/Flash_AI; Draw conditional at 0x0069DEB0 (DOC-INHERITED: module header cites SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md) | Faithful PRIMITIVE port (closest thing to a Framework-A gadget object in the codebase) |
| `src/app_sidebar_gadgets.rs` | per-sim-tick flash orchestrator (Defense tab on SW ready; period=10 ticks) | StripClass::AI poll 006a8e52..9b (DOC-INHERITED via module comment) | Faithful narrowed port |
| `src/sidebar/power_bar_anim.rs` (517 ln) | PowerClass segment-slide + flash anim; FILL_SCALE 400.0 / OUTPUT_CAP 100.0 cited to 0x7ED8C8/0x7E2AC0 (DOC-INHERITED comments) | PowerClass (Power.CPP) | Faithful port w/ admitted placeholder: `SLIDE_TICKS_PER_STEP = 9` is "placeholder until verified" (power_bar_anim.rs:24-27) |
| `src/app_sidebar_render.rs`, `src/render/sidebar_chrome.rs`, `sidebar_cameo_atlas.rs`, `sidebar_text.rs`, `radar_anim.rs`, `minimap.rs` | sidebar draw + atlases | SidebarClass draw | AD-HOC render |
| `src/app_input.rs` (1117 ln) | in-game mouse/hotkeys; sidebar hit-test dispatch on mouse-DOWN | tactical input + gadget input chain | AD-HOC |
| `src/ui/in_game_hud.rs` (210 ln) | egui build/queue panel ("Temporary bridge until the custom RA2 sidebar is implemented") | none | **DEAD** — `draw_in_game_hud` has zero callers outside its own module (grep over src/: only def + mod decl) |
| `src/ui/pause_menu.rs` (211 ln) | egui pause overlay (speed presets, music) | ESC options dialog 0xBBB (own proc) | EGUI placeholder |
| `src/ui/mission_status.rs` (75 ln) | egui mission banner + result screen | score/endgame screens | EGUI placeholder |
| `src/ui/game_screen.rs` (46 ln) | `GameScreen` enum (MainMenu/Loading/SpawnPick/InGame/MissionResult) | top-level game-mode switch | neutral state machine |
| `src/ui/client_theme.rs` (68 ln) | egui light-theme palette/card helpers | none | EGUI support |

### 1.3 Input flow (verified at the winit boundary, app.rs:2067-2311)

- Every event goes to egui first (`state.egui.on_window_event`, app.rs:2071-2072); in-game the
  `consumed` flag is deliberately ignored unless paused/save-load panel (app.rs:2080-2081).
- MouseInput routing precedence (app.rs:2207-2275): shell-transition block → main-menu modal
  (exit-confirm SHP hit-test) → native skirmish shell → single-player shell → main-menu shell →
  SpawnPick → InGame (`app_input::handle_mouse_input`).
- 0xE2 / 0x100 / exit-confirm 0x120 / validation 0xCE: all four feed `DialogController`
  (`ensure_active` + `on_pointer_down/move/up`; app.rs:1580-1636, 1692-1727, 1729-1788,
  1361-1392). The skirmish 0x102 board does NOT (bespoke handlers app.rs:1394-1522).
- Keyboard: validation modal Enter/Esc via `shell_controller.on_key` (app.rs:1346-1359, the
  only `on_key` consumer in the app — grep hit app.rs:1353); main-menu-dialog Esc closes
  dialogs directly WITHOUT the controller (app.rs:2106-2112 → `close_main_menu_dialogs`
  app.rs:1950-1955 — the 0x120 instance is never `pop()`ed; it is clobbered later by
  `ensure_active(0xE2)`'s `reset_to`).
- In-game: sidebar hit-test happens on mouse-DOWN only (app_input.rs:39-43 →
  `handle_sidebar_mouse_input` app_input.rs:227-238); wheel over sidebar scrolls rows
  (app_input.rs:210-225, app.rs:2292-2302).

### 1.4 Layering invariant (verified)

- `src/sim/` has ZERO imports of ui/render/sidebar/audio/net: grep
  `use crate::(ui|render|sidebar|audio|net)::` over `src/sim` → **no matches**.
- ui→sim reads are present and allowed (ui/mod.rs:10-12 states the rule; e.g.
  in_game_hud.rs:5, skirmish_shell/state/player_name.rs:3 `use crate::sim::game_options`).
- `ui/shell/*` imports nothing above plain std + sibling shell modules (each `//!` header
  states render-agnostic; verified imports: geom none, descriptor→geom, layout→descriptor+geom,
  controller→std+descriptor+layout, modal→descriptor+geom).
- One layering wart: `ui::shell::controller` uses `std::time::Instant` (controller.rs:19,
  184) — wall-clock in a ui-model type. Fine for menus (non-sim), but it makes controller
  state non-reproducible for replay-style testing of shell behavior.

---

## 2. CONTRACT GAPS vs Framework A/B basics (verified in code)

### 2.1 Hit-testing: first-match-in-vec, no tie-break, mixed edge semantics

- Shell controller: `hit_any` = `buttons.iter().find(|c| c.rect.contains(x, y))`
  (controller.rs:210-212) — FIRST match in feed order. The doc-comment itself concedes the
  disabled-press path "does NOT skip a disabled control to a later one beneath the same point"
  (controller.rs:199-206). Today's feeds are non-overlapping, so no divergence is *currently*
  reachable — but the contract is order-dependent, not z/area-dependent. Default verdict for
  any future overlapping dialog: DRIFT risk.
- Sidebar: fixed probe order tabs → repair → sell → items → pause/producer → 4 control buttons
  (sidebar/mod.rs:379-425). First match wins; draw order and hit order are two separately
  hardcoded sequences (no retained list ties them together).
- Skirmish board hover: fixed probe order open-combo-item → status_help → player_name →
  owner-draw buttons → map_preview → checkboxes → trackbars → combos → statics
  (hit_test.rs:41-113). Comment at hit_test.rs:97-99 asserts statics don't overlap widgets so
  order is claimed immaterial (UNVERIFIED for all resolutions — see §5).
- **Edge semantics are inconsistent across frameworks**: shell `RectPx::contains` is
  right/bottom-EXCLUSIVE (`x < self.x + self.w`, geom.rs:34-36); sidebar `Rect::contains` is
  right/bottom-INCLUSIVE (`px <= self.x + self.w`, sidebar/mod.rs:61-63). A click on the
  sidebar's exact right/bottom edge pixel hits in one framework and misses in the other.
  gamemd GadgetClass edge convention must be confirmed by the binary lane; flagged DRIFT.

### 2.2 No sticky capture / hold-repeat model

- `DialogController` tracks `pressed` identity only; `on_pointer_move` updates hover, never
  routes motion to the pressed control (controller.rs:179-187). No gadget receives
  "mouse held, cursor elsewhere" events — gamemd's sticky-capture gadget semantics
  (held gadget keeps receiving input until release) have no counterpart.
- Dropdown scrollbar arrows scroll EXACTLY ONCE per mouse-down (combos.rs:645-656:
  `scroll_open_combo_by_rows(±1)` at press); `dropdown_scroll_press` is recorded but its only
  consumer is the RENDER pressed-frame (app_skirmish_shell_render/controls.rs:532-536). There
  is NO hold-repeat timer anywhere (handle_option_mouse_move handles only thumb drag,
  trackbars.rs:286-293; mouse_up clears the press, trackbars.rs:314-319). Holding an arrow in
  a Win32 scrollbar auto-repeats; here it does not. **DRIFT** (fires whenever a player holds
  the dropdown arrow — e.g. the 9-item color combo).
- Trackbar rail press jumps the value once and sets `dragging_thumb:false`; subsequent motion
  while held does nothing (trackbars.rs:263-272, 300-303 — comment documents this as the
  native behavior; binary lane to confirm).
- Trackbar thumb drag IS captured while held (trackbars.rs:258-262 + 295-311) — capture exists
  only as this one special case plus `minimap_dragging` (app_input.rs:173-176) and
  `selection_state` drag; each is a bespoke boolean, not a substrate service.
- Sidebar: action fires on mouse-DOWN, not on press-release match (app_input.rs:39-43 +
  227-238). gamemd owner-draw buttons fire on release-over-pressed; the in-game sidebar
  gadgets' down-vs-up firing convention must be confirmed per-gadget by the binary lane.
  Flagged DRIFT-risk for every sidebar button (tabs, repair, sell, cameos).

### 2.3 No retained gadget list / draw order hardcoded

- There is no LinkClass-like list anywhere. Draw order per surface is hardcoded:
  - Single-player/main-menu shells: explicit buffer sequence movie → chrome → buttons → text →
    cursor (app_single_player_shell_render.rs:215-272; depths in shell_paint.rs:36-43).
  - Skirmish: a semantic role Vec built by `skirmish_shell_semantic_draw_order`
    (app_skirmish_shell_render/draw_order.rs:74-90+) — order is data, but still a hardcoded
    list, not a gadget tree.
  - Sidebar: chrome stack order baked into `compute_layout` + render module.
- Consequence: adding/removing a control means editing hit order AND draw order separately —
  the exact divergence-class the gamemd retained list prevents.

### 2.4 Tooltip / status-help timing

- Main menu 0xE2: hover tooltip label is emitted IMMEDIATELY whenever `hovered_button` is Some
  (app_main_menu_shell_render.rs:155-162). No delay timer consulted.
- Controller arms `hover_started_at` on hover transition (controller.rs:182-185, "single-player
  hover-delay parity" per comment) — but its ONLY consumer is the 0x100 hover-flash square
  wave (`steady_frame_choice`, shell_paint.rs:223-241: `(elapsed/1000) % 2 == 1`, ~1 Hz, first
  high phase at 1s). No tooltip delay is implemented anywhere.
- Single-player 0x100: status_help rect exists but is NOT drawn at all
  (app_single_player_shell_render.rs:89-90 comment: "status_help / side_image_static rects
  exist in the layout but are NOT drawn — kept that way").
- Skirmish 0x102: status help text updates on every mouse-move with no delay (app.rs:1222-1242
  via `hovered_shell_control` → `status_help_key_for_hover`). Whether gamemd's 0x102 status
  line is immediate or delayed is for the binary lane; current Rust = immediate.
- In-game (Framework A): NO tooltip system exists at all — no cameo name/cost tooltip, no
  bottom bar. gamemd's in-game ToolTip/HelpClass surface is entirely unimplemented. GAP.

### 2.5 Event→command dispatch and sim independence

- Front-end shells never touch sim; they produce actions consumed by app.rs
  (`handle_main_menu_shell_action` app.rs:1880-1926, `handle_skirmish_shell_action`
  app.rs:806-858) which call into `skirmish_launch`/session start.
- In-game: `sidebar::hit_test` returns `SidebarAction`; `apply_sidebar_action`
  (app_input.rs:240-318) either mutates app-local UI state (tabs, targeting/repair/sell modes)
  or calls `app_commands::*` which schedule sim `Command`s. Sim stays UI-independent (§1.4
  grep). The UI→sim boundary is command-queue shaped, which is the right substrate seam.
- Dev-only actions live in the same enum as gameplay actions (`CycleOwner`,
  `PlaceStarterBase`, `SpawnTestUnits`, `CancelLastBuild` — sidebar/mod.rs:142-146) and render
  as bottom control-block buttons that have no gamemd counterpart (sidebar/mod.rs:251-256).

### 2.6 Substrate-internal contract notes (current limits, verified)

- `DialogController::on_key` is a stub policy: returns true for Enter/Escape whenever ANY
  dialog is in the kbd route (controller.rs:192-194) — it does not identify WHICH control
  resolves, nor route Tab focus. Sole consumer: validation modal (app.rs:1346-1359).
- Esc on the exit-confirm modal bypasses on_key entirely (app.rs:2106-2112) and never
  `pop()`s — the C5 LIFO teardown is not exercised; `reset_to` clobbers the stack later.
  INTERNAL inconsistency with the controller's own stack contract; observable only if stack
  depth >1 ever matters (today max depth used is 1 + ensure_active swaps).
- `ControlKind::{Checkbox,Radio,Combo,Listbox,Trackbar,Edit,ScrollBar}` are declared but
  unexercised placeholders (descriptor.rs:21-32, comment says Slice 4 fills them in).
- 0x100 and 0x102 layouts do NOT go through the descriptor table — they call geom helpers
  directly (single_player_shell/layout.rs:83-146; skirmish_shell/layout.rs:414-550). Only 0xE2
  is descriptor-driven (main_menu_shell/layout.rs:163-211, 222-280). The descriptor comment
  (AnchorRule::RightAnchor, descriptor.rs:48-53) itself warns 0x100/0x102 need their own
  anchor variant when they migrate.
- `compute_responsive_layout` (main_menu_shell/layout.rs:296-325) is an admitted intentional
  drift ("This intentionally drifts from retail pixel parity") — X/Y stretch of the 800x600
  shell. Callers must prefer `compute_layout`; the input handlers DO use `compute_layout`
  (app.rs:1581-1584), so the responsive path is render-only if used at all.
- `mirror_shell_controller_to_*` (app.rs:1556-1578) still feeds per-shell state structs that
  the render path READS (app_single_player_shell_render.rs:219-231 reads
  `single_player_shell_state.pressed/hovered/hover_started_at`). The comment at
  app.rs:1553-1555 says "render is retired off these in Slice 3, after which the controller is
  the sole authority" — that retirement did NOT land; the mirrors are still load-bearing.
  Stale comment + remaining migration step.

---

## 3. RETIRE LIST — ad hoc logic a gadget/dialog substrate service replaces

Ordered by blast radius (smallest first). "Clause" = the substrate facility that absorbs it.

| # | File:lines | What it does today | Replacing clause | Risk of behavior change |
|---|---|---|---|---|
| R1 | `src/ui/in_game_hud.rs` (whole file, 210 ln) | egui build palette; zero callers | delete (Framework-A sidebar already shipped) | None (dead code) — verify with a build after removal |
| R2 | `src/app.rs:1556-1578` (`mirror_shell_controller_to_*`) + per-shell `pressed/hovered` struct fields (main_menu_shell/state.rs:62-70, single_player_shell/state.rs:52-57) | duplicate press/hover state for render | render reads `DialogController` directly (finishing Slice 3's stated retirement) | Low — pure plumbing; pixel-identical if reads are 1:1 |
| R3 | `src/app.rs:1394-1478` skirmish bespoke gesture (`pressed_owner_draw_button` set at 1425-1426, release-match at 1456-1467) + `hit_test_owner_draw_button`/`hit_test` (skirmish_shell/state/hit_test.rs:292-331) | hand-rolled press-must-match-release for the 3 owner-draw buttons | `DialogController` feed (exactly like 0xE2/0x100); ids already exist (0x617/0x5AA/0x5C0) | Low-medium — must keep mouse-down button sound (app.rs:1432) and the choose-map/validation modal pre-empts (app.rs:1395-1400, 1446-1452) |
| R4 | `src/ui/single_player_shell/layout.rs:55-146` (private `right_anchor`, `status_help_rect`, `back_rect`, hand-built button array) and `src/ui/skirmish_shell/layout.rs:380-412` (same helpers re-duplicated) | per-shell layout passes not on the descriptor table | new `AnchorRule` variants (Y-centered offset family; back-button bottom-row rule) + descriptor tables for 0x100/0x102 | Medium — descriptor.rs:48-53 warns the anchor conventions differ from 0xE2; rect-for-rect golden tests required (existing test fixtures cover 800x600/1024x768/640x480) |
| R5 | `src/ui/skirmish_shell/state/combos.rs:635-727` (combo open/close/select + popup capture) + `state/trackbars.rs:227-334` (checkbox toggle, trackbar press/drag) + `state/player_name.rs` edit state | the entire 0x102 control set as bespoke state machines | Slice 4: `ControlKind::{Combo,Trackbar,Checkbox,Edit,ScrollBar}` behaviors + per-control substrate dispatch (the deferred OwnerDrawControl trait per slice4 kickoff) | HIGH — the 87-test safety net (`state/tests.rs`, count verified `grep -c "#[test]"` = 87) must stay green & unchanged per kickoff doc |
| R6 | two scroll models: combo dropdown scrollbar (combos.rs:643-712 + scroll math at 200-260) vs choose-map listbox scrollbar (skirmish_shell/layout.rs:593-732) | duplicated thumb/track/arrow math with separate constants | one substrate ScrollBar control | Medium — the two have different row heights (23 vs 19) and the same `COMBO_DROPDOWN_SCROLLBAR_*` constants are already shared; unify carefully |
| R7 | `src/sidebar/mod.rs:379-425` (`hit_test`) + the parallel hardcoded draw stack in `app_sidebar_render`/`render/sidebar_chrome` | in-game sidebar hit/draw as two separately-ordered hardcoded lists | Framework-A gadget service: ONE retained gadget list owning z-order, hit order, capture, repeat, disabled, flash | Medium-high — must preserve current click outcomes; gadget_flash.rs already provides the per-gadget flash primitive to hang on the list |
| R8 | `src/app_input.rs:39-43, 227-238` (sidebar fires on mouse-DOWN) | press-time dispatch | gadget service's press/release semantics (match gamemd per-gadget convention once the binary lane pins it) | DRIFT-correcting change — player-visible (button click cancels on drag-off in gamemd-style UIs) |
| R9 | `src/ui/pause_menu.rs` + `src/ui/main_menu_dialogs.rs` options dialog | egui stand-ins for the Options surface | `ModalKind::InGameOptions` (0xBBB in-game / 0xF5 shell) descriptor + own-proc result convention already modeled in modal.rs:44-78, 119-135, 162-165; needs the full-shell descriptor + renderer (slice5b plan exists, unshipped) | High visibility — replaces a whole egui surface with SHP |
| R10 | `src/ui/main_menu_dialogs.rs` movies/credits + campaign select egui panels | egui stand-ins | Framework-B dialogs (ids TBD by binary lane) | Not started; keep egui until decoded |
| R11 | `src/sidebar/layout_spec.rs` RON-tunable spec + `sidebar/mod.rs:273-333` content-adaptive row clamp (`MIN_VISIBLE_ROWS`, fill_to_bottom) | modern adaptive sidebar geometry | retail-pinned strip geometry + scroll buttons IF retail parity for the sidebar is adopted | POLICY DECISION — current sidebar deliberately diverges (scale target/modern resolutions). Do not retire without user sign-off |
| R12 | `src/ui/main_menu_shell/layout.rs:282-325` `compute_responsive_layout` (+ scale_rect) | stretch-to-window drift mode | drop or keep as explicit non-parity mode behind config | Verify no input path consumes it first (input uses compute_layout, app.rs:1581-1584) |

NOT retire (faithful primitives to keep and extend): `gadget_flash.rs` (whole file),
`power_bar_anim.rs` (fix `SLIDE_TICKS_PER_STEP` placeholder instead),
`static_reveal.rs`, `app_sidebar_gadgets.rs`, all of `ui/shell/*`.

---

## 4. Shell substrate's OWN remaining gaps (slice ledger)

Slice status from `git log --grep="substrate Slice"` (verified this session):

| Slice | Commit(s) | Status |
|---|---|---|
| 0 geom | e1b50ec4 | SHIPPED |
| 1 descriptor+layout (0xE2) | 21d3341a | SHIPPED |
| 2 DialogController (0xE2+0x100) | 71b9a3de | SHIPPED |
| 3 paint pass (0xE2+0x100) | 32f066f0 | SHIPPED (but see mirror retirement gap, §2.6) |
| 4 skirmish controls | — | **NOT SHIPPED** (no commit; kickoff doc `docs/plans/2026-06-01-shell-substrate-slice4-kickoff.md` read in full; flags it "BIGGEST BLAST RADIUS", 87-test safety net must stay green/unchanged) |
| 5 modal substrate | d355d495 (ModalKind+paint_modal_shp), b3d39232 (quit-confirm SHP, 4a), 31cbacbf+1be4e2ff (persist settings + audio cascade, 4b-i/4b-ii-a), 87a7e598 (fade-to-black, 4b-ii-b), 76826114 (MNBTTN frame fix, sub-step 6 partial), 635423cd (validation render re-point, 6a), 54de2fd3 (validation input via controller, 6b) | LARGELY SHIPPED |
| 5b options dialog | — | NOT SHIPPED (plan `2026-06-02-shell-substrate-slice5b-options-plan.md` exists; `ModalKind::InGameOptions` has zero consumers in app.rs — grep returned none) |

Remaining gaps inside the shipped substrate:

1. **Slice 4 wholesale** — skirmish 0x102 controls off-substrate (the largest single gap).
2. **InGameOptions unconsumed** — modal.rs models 0xBBB/0xF5 + OwnProc results (modal.rs:44-78,
   119-135) but nothing builds its descriptor or renders it; `build_message_box_descriptor`
   debug_asserts it away (modal.rs:185-189).
3. **ModalKind::ThreeButton (0x121)** — reachable through the count rule but render-UNTESTED
   (modal.rs:38-40 comment).
4. **on_key is a placeholder** (controller.rs:192-194): no per-control keyboard resolution, no
   Tab order, `ShellKey::Tab` always unconsumed; kbd_route registration-order plumbing exists
   but only Enter/Escape-as-dismiss is implemented (validation modal only).
5. **Exit-confirm Esc bypasses the controller** (app.rs:2106-2112) and the stack is never
   popped on that path (§2.6) — C5 teardown unexercised.
6. **Mirror retirement** promised by the Slice-3 boundary comment (app.rs:1553-1555) not done
   (render still reads per-shell mirrors, app_single_player_shell_render.rs:219-231).
7. **hover_started_at has no tooltip consumer** — armed (controller.rs:184) and threaded, used
   only for the 0x100 hover flash; tooltip-delay behavior (if gamemd has one) unimplemented.
8. **Validation modal pixel size unconfirmed** — skirmish layout.rs:40-45 comment: 450x325 is a
   derived candidate, "treat as unconfirmed pending a native GetClientRect/screenshot".
9. **Slide/`group` byte** — descriptor carries `group: u8` (study +0xD5, descriptor.rs:94-95)
   but every control sets 0; slide grouping is handled by the separate
   `app_shell_transition`/wave machinery rather than the descriptor.
10. **Two snap algorithms kept un-unified on purpose** (geom.rs:5-8: "not proven equivalent at
    the half-row boundary") — fine, but it means AnchorRule must stay two variants until an
    algebraic/exhaustive proof exists.

---

## 5. UNVERIFIED (YELLOW) — claims this lane could not verify

- Any statement about what gamemd.exe actually does (button release semantics for Framework-A
  gadgets, scrollbar hold-repeat, status-help delay, 0x102 statics overlap behavior at all
  template sizes, GadgetClass hit-test edge convention, the 0x52D640 proc identity, the
  0x0069DEB0 frame conditional, StripClass::AI 006a8e52 literal, PowerClass constants at
  0x7ED8C8/0x7E2AC0, the SBGadgetClass +0x34/+0x38/+0x3C/+0x1E layout) — DOC-INHERITED from
  in-repo comments/docs; binary lanes must confirm.
- hit_test.rs:97-99's claim that skirmish statics never overlap interactive widgets — asserted
  in a comment; not exhaustively checked across all three supported resolutions by this lane.
- Whether `compute_responsive_layout` is reachable from any live render path (input paths use
  `compute_layout`; I did not enumerate every render caller).
- The "87-test safety net" semantic coverage (count verified = 87; content not re-read in
  full).
