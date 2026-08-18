# Plan-grounding lane — RUST ANCHORS + INTEGRATION CONTEXT

Lane: grounding for the A0/A1/A4/A5 + D-B3 + R1 implementation plan.
ALL claims below verified **live this session** against the WORKTREE checkout
`<local>/Documents/ra2-uigadget-worktree` (branch `ui-gadget-substrate`,
HEAD `7b79a186`, clean `git status`). File:line cites are worktree lines.
No Ghidra used (by assignment). Companion doc: `rust-current.md` (same dir).

Status: IN PROGRESS — sections appended incrementally.

---

## 1. R8 flip site — sidebar input path (A1 target)

### 1.1 Dispatch today (fire on mouse-DOWN, press only)

`src/app_input.rs:34-43` — `handle_mouse_input` entry:

```rust
pub(crate) fn handle_mouse_input(
    state: &mut AppState,
    button: MouseButton,
    btn_state: ElementState,
) {
    if btn_state.is_pressed() {
        if handle_sidebar_mouse_input(state, button) {
            return;
        }
    }
```

- VERIFIED: sidebar probe happens **only on press** (`btn_state.is_pressed()`);
  releases never reach the sidebar. Both Left AND Right press are probed
  (right_click drives cancel-build semantics).
- Caller: `src/app.rs:2272-2274` —
  `else if !egui_consumed && state.screen == GameScreen::InGame {
   app_input::handle_mouse_input(state, button, btn_state); }` (last arm of
  MouseInput routing; full precedence in §3.4).

`src/app_input.rs:227-238` — exact text:

```rust
fn handle_sidebar_mouse_input(state: &mut AppState, button: MouseButton) -> bool {
    let Some(view) = current_sidebar_view(state) else {
        return false;
    };
    let right_click = button == MouseButton::Right;
    let action = sidebar::hit_test(&view, state.cursor_x, state.cursor_y, right_click);
    if action == SidebarAction::None {
        return false;
    }
    apply_sidebar_action(state, action);
    true
}
```

Return contract: `true` = consumed (an action other than `None` was produced and
applied immediately); `false` = not over sidebar / no actionable element → input
falls through to minimap-drag → targeting → selection drag (app_input.rs:45-55).
NOTE the fall-through subtlety: a click on the sidebar *panel* but on no button
returns `false` today, so it can start a selection drag underneath. A gadget
"panel catcher" would change that (catchers are OUT OF SCOPE per A3, but the
fire-on-release flip must keep this fall-through behavior or note the change).

### 1.2 Action application — `apply_sidebar_action` (app_input.rs:240-318)

Single `match action` with these arms (all verified):
- `SelectTab(tab)` → `state.active_sidebar_tab = tab; state.sidebar_scroll_rows = 0;`
  (`app_input.rs:243-246`) — tab switch RESETS scroll to 0.
- `BuildType(type_id)` → `queue_build_by_type(state, &type_id)` (app_commands).
- `ArmPlacement` / `ClearPlacementMode` / `ArmSuperWeapon` / `ClearSuperWeaponMode`
  → mutate `state.targeting_mode`, `building_placement_preview`, and clear
  `sidebar_gadget_state.repair_mode_on/sell_mode_on` (mutual exclusion,
  app_input.rs:250-274).
- `TogglePauseQueue`/`CycleProducer`/`CancelBuild`/`CancelLastBuild`/`CycleOwner`/
  `PlaceStarterBase`/`SpawnTestUnits` → `app_commands::*` calls (queue sim Commands).
- `ToggleRepairMode` (app_input.rs:296-304): flips
  `state.sidebar_gadget_state.repair_mode_on`; when turning ON also forces
  `sell_mode_on=false`, `targeting_mode=None`, `building_placement_preview=None`.
- `ToggleSellMode` (app_input.rs:305-313): symmetric.
- `Deploy` → `queue_deploy_undeploy_for_selected`.

**Sounds/flash on sidebar click: NONE today.** Verified: `apply_sidebar_action`
plays no sound (no `GameSoundEvent`/sfx call in the function); no press-frame is
set anywhere on mouse-down (pressed visuals come ONLY from mode bits / flash,
§2.4). So the fire-on-release rework does NOT need to preserve any click sound
or down-press visual — there is none to preserve. (EVA "Building"/"Training" on
build start comes from the sim/audio side via `queue_build_by_type` →
sim events, not from the input layer.)

### 1.3 Wheel scroll (relevant to A1 strip-scroll gadgets)

`src/app_input.rs:210-225` — `try_sidebar_scroll(state, delta_lines) -> bool`:
over `view.panel_rect` only; up → `sidebar_scroll_rows.saturating_sub(step)`,
down → `min(view.max_scroll_rows)`; step = `ceil(|lines|).max(1)`.
Caller: `src/app.rs:2292-2302` (MouseWheel arm; falls back to zoom).
**There are NO strip-scroll BUTTONS anywhere in the worktree today** — verified
by grep `scroll` over `src/sidebar/`, `src/app_sidebar_render.rs`,
`src/render/sidebar_chrome.rs`: only `scroll_rows`/`max_scroll_rows` state.
A1's "2 strip-scroll buttons" are NEW gadgets; their click handler should mutate
the same `state.sidebar_scroll_rows` (clamped by `view.max_scroll_rows`), one
page/row per click, no hold-repeat (G23 mask without held bits).
Scroll state: `AppState.sidebar_scroll_rows: usize` (re-clamped every frame in
`current_sidebar_view`, app_sidebar_render.rs:139-162 rebuilds the view if
clamping changed it).

### 1.4 What a fire-on-release flip must preserve

- Right-click cancel-build path (`hit_test(..., right_click=true)` →
  `CancelBuild`): today fires on right-press. Decide: release-inside applies to
  the right button too, or right stays press-fire (binary lane pins gamemd).
- Tab-switch scroll reset (`sidebar_scroll_rows = 0` on SelectTab).
- Mutual-exclusion side effects on Repair/Sell/Arm* (exact order above).
- The `false`-return fall-through to selection/minimap (§1.1 note).
- Wheel scroll continues to work independently of the new scroll gadgets.
- `current_sidebar_view(state)` is rebuilt per event (it is `Option` — None
  before sim exists; the gadget tick must tolerate that).

---

## 2. Sidebar hit/draw loop today (A1 integration surface)

### 2.1 Geometry types

`src/sidebar/mod.rs:52-64` — `Rect { x, y, w, h }` (f32) with **INCLUSIVE**
contains (both edges):

```rust
pub fn contains(&self, px: f32, py: f32) -> bool {
    px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
}
```
(`sidebar/mod.rs:61-63` — verified verbatim; differs from shell `RectPx::contains`
which is right/bottom-exclusive, `ui/shell/geom.rs` — and from the study's G14
HALF-OPEN gadget rule. The A0 gadget rect must be a NEW type or the gadget tick
must not reuse `sidebar::Rect::contains`.)

### 2.2 hit_test — fixed probe order (sidebar/mod.rs:379-425)

VERIFIED at exactly :379-425. Order: `panel_rect` gate → tabs (Vec iter) →
`repair_button` → `sell_button` → items (Vec iter) → `pause_button?` →
`producer_button?` → `[cancel, cycle_owner, starter_base, spawn_test_units]`.
First match wins; returns `SidebarAction` (clone of the button's action field).
Item clicks route through `hit_test_item(item, right_click)`
(sidebar/mod.rs:335-377): SW ready/armed arm/clear; build ready→Arm/Clear,
enabled→BuildType, else None; right-click → CancelBuild if `queued_count>0 ||
is_ready` (None for SW).

### 2.3 View model (the rect source a gadget list would be built FROM)

`src/sidebar/mod.rs:229-257` — `SidebarView` (built per frame AND per input
event by `current_sidebar_view`, `src/app_sidebar_render.rs:29-169`):
- `panel_rect: Rect`, `layout: SidebarLayout`, money/power fields,
- `scroll_rows`/`max_scroll_rows: usize`,
- `tabs: Vec<SidebarTabButton>` — `{ tab, rect, active, frame_index: u8 }`
  (mod.rs:185-195); 4 tabs from `SidebarTab::all()`, rects computed in
  `build_sidebar_view_with_spec` (sidebar_view.rs:113-142; bottom-aligned to
  `cameo_grid_top`, per-tab X nudges −2/−1/0/+2).
- `repair_button`/`sell_button: SidebarToggleButton` — `{ rect, action,
  frame_index: u8 }` (mod.rs:200-206); rects from layout_spec
  `repair_x/y=8,20`, `sell_x/y=96,20` relative to `side1_y`
  (sidebar_view.rs:144-175); **size comes from the chrome atlas frame ×
  ui_scale, or [0,0] when atlas missing → unhittable** (sidebar_view.rs:151-152
  + app_sidebar_render.rs:87-102).
- `items: Vec<SidebarItem>` (cameo grid, scrolled window of 2-col rows),
- `pause_button`/`producer_button: Option<SidebarControlButton>` +
  4 always-present dev `SidebarControlButton`s (cancel/cycle_owner/
  starter_base/spawn_test_units) — `{ rect, action, label: String }`.

`SidebarAction` enum: `src/sidebar/mod.rs:127-153` (None, SelectTab, BuildType,
ArmPlacement, ClearPlacementMode, ArmSuperWeapon, ClearSuperWeaponMode,
TogglePauseQueue, CycleProducer, CancelBuild, CancelLastBuild, CycleOwner,
PlaceStarterBase, SpawnTestUnits, ToggleRepairMode, ToggleSellMode, Deploy).

`compute_layout_with_spec` (mod.rs:284-333): adaptive rows —
`MIN_VISIBLE_ROWS=4` (mod.rs:35), clamp `item_rows.clamp(MIN_VISIBLE_ROWS,
max_rows)` unless `fill_to_bottom`. R11 (geometry policy) is OUT of plan scope —
the gadget layer must take rects from this existing machinery as-is.

### 2.4 Pressed/hover visuals today (what frames are drawn per state)

There is **no hover state and no transient press state** in the sidebar today.
Visual state = `frame_select(disabled, mode_active, state)` →
5-frame SHP convention `0=idle, 1=mode-active, 2=disabled, 3=pressed-idle,
4=pressed-active` (`src/sidebar/gadget_flash.rs:95-123`, verbatim table).
- Tabs: `SidebarGadgetState::tab_frame(idx, is_active_tab)`
  (gadget_flash.rs:166-170) — `mode_active` = is-active-tab, `state` = flash
  pulse bit (Defense tab flashes when a SW is ready; period exactly 10 sim
  ticks — `app_sidebar_gadgets.rs:27` `FLASH_PERIOD_TICKS: u32 = 10`).
- Repair/Sell: `repair_frame()`/`sell_frame()` (gadget_flash.rs:174-181) —
  `mode_active` = mode-on toggle bit, `state` always 0 (no flash AI).
- frame_index is computed in the VIEW BUILDER (sidebar_view.rs:139, 169, 174),
  consumed by the render layer. A gadget-backed press visual (G22 silent press)
  would set `state=1` while held-inside — exactly the `3/4` frames that
  `frame_select` already maps. **The frame table needs no change for A1; only a
  source for the transient press bit.**
- `GadgetFlash` primitive (gadget_flash.rs:19-93): `state: u8`, `period: u32`
  (non-zero ⇒ active), `countdown: u32`, `disabled: bool`; `start(period,
  extra_delay, initial_state) -> bool` (guard: no-op if active), `stop() ->
  bool` (write order state→countdown→period), `tick() -> bool`. Ticked per SIM
  tick by `app_sidebar_gadgets::update_sidebar_gadget_state` called from
  `src/app_sim_tick.rs:218` (right after `update_power_bar_anim` :217).
  KEEP as-is per study §7 (faithful primitive); the A0 gadget object should
  *carry* one, not replace it.
- `SidebarGadgetState` (gadget_flash.rs:130-157): `tab_flashes: [GadgetFlash;4]`,
  `tab_disabled: [bool;4]`, `repair_mode_on`, `sell_mode_on`,
  `repair_disabled`, `sell_disabled`, `last_sim_tick: u64`. Lives on `AppState`
  (`state.sidebar_gadget_state`).

### 2.5 Draw order (where it's established)

Sidebar render = instance builders re-exported by `app_sidebar_render.rs:20-23`
from `src/app_sidebar_build.rs` (640 ln): `build_sidebar_chrome_instances`,
`build_sidebar_cameo_instances`, `build_sidebar_instances`,
`build_sidebar_text_instances`. Chrome SHP atlas:
`src/render/sidebar_chrome.rs` (900 ln, `SidebarChromeAtlas` — `tab_frames`,
`repair_frames`, `sell_frames` 5-frame arrays, per-theme
Allied/Soviet/Yuri via `current_sidebar_chrome`, app_sidebar_render.rs:413-419).
Draw order is the hardcoded buffer-emission order inside those builders + the
pass order in app_render — i.e. hit order (mod.rs:379-425) and draw order are
two separate hardcoded sequences (the divergence class A0's retained list ties
together). A1 keeps the existing builders as the draw side; the gadget list
must be constructed in the same order the builders draw chrome buttons so
retained order = draw order = hit priority holds.

---

## 3. D-B3 site — Esc routing for exit-confirm (current flow, verbatim)

### 3.1 The bypass (app.rs:2106-2112) — VERIFIED verbatim

Inside `WindowEvent::KeyboardInput` (app.rs:2091-2159):

```rust
// A main-menu modal dialog (exit confirm, options, movies,
// campaign select) takes ESC first: close it and stay,
// never propagating to the shell-close handlers below.
if Self::main_menu_dialog_open(state) {
    if is_escape {
        Self::close_main_menu_dialogs(state);
        state.window.request_redraw();
    }
    return;
}
```
`is_escape` = `code == KeyCode::Escape && pressed && !repeat` (app.rs:2095-2096).
Note: this arm swallows **every** key while any main-menu dialog is open (the
`return` is outside the `is_escape` check).

### 3.2 close_main_menu_dialogs (app.rs:1949-1955) — VERIFIED verbatim

```rust
/// Close every open main-menu modal dialog (e.g. on ESC).
pub(crate) fn close_main_menu_dialogs(state: &mut AppState) {
    state.exit_confirm_modal = None;
    state.options_dialog = None;
    state.movies_credits_dialog = None;
    state.campaign_select = None;
}
```
No `shell_controller.pop()` anywhere in it — the 0x120 DialogInstance stays on
the stack until something else calls `ensure_active(0xE2)` → `reset_to` clobber.
(Other callers of `close_main_menu_dialogs`: exit-confirm mouse-up Cancel arm,
app.rs:1721-1724.)

### 3.3 How 0x120 gets ON the stack

- Open: `open_exit_confirm_modal` (app.rs:1928-1941) —
  `shell_controller.ensure_active(DialogId(0x0120), /*accepts_keys=*/true)` +
  `state.exit_confirm_modal = Some(modal)`. NOTE: `ensure_active` on a stack
  whose top is 0xE2 **reset_to-clobbers** the stack to [0x120] — it never
  push()es 0x120 over 0xE2 (controller.rs:107-111 + 97-101), despite the
  comment at app.rs:1935-1936 saying "0x120 over the menu's 0xE2". Stack depth
  never exceeds 1 on this path.
- Mouse: `handle_exit_confirm_modal_mouse_down/_up` (app.rs:1692-1727) route
  through the controller (`ensure_active(0x120, true)` + on_pointer_down/up
  against `exit_confirm_modal_feed` = OK/CANCEL `LaidOutControl`s,
  app.rs:1640-1657). Mouse-up OK → persist + `state.exit_confirm_modal = None`
  + quit cascade (no pop); CANCEL → `close_main_menu_dialogs` (no pop).
- Esc: **never reaches the controller** — swallowed at §3.1 before
  `route_validation_modal_key`-style handling; `on_key` is only ever called by
  the validation-modal path (app.rs:1346-1359, the sole `on_key` consumer —
  verified by grep).

### 3.4 Controller stack API (ui/shell/controller.rs) — what a routed pop uses

Verified signatures (controller.rs:77-212):
- `push(&mut self, id: DialogId, accepts_keys: bool)` :77-82
- `pop(&mut self) -> Option<DialogId>` :86-94 (prunes kbd_route by rposition)
- `reset_to(&mut self, id, accepts_keys)` :97-101 (clear stack+route, push)
- `ensure_active(&mut self, id, accepts_keys)` :107-111 (reset_to ONLY if
  `top_id() != Some(id)`)
- `top_id() -> Option<DialogId>` :113-115; `pressed()/hovered() -> Option<u16>`;
  `hover_started_at() -> Option<Instant>` :127-129; `kbd_route() -> &[DialogId]`
- `set_disabled(&mut self, control: u16, disabled: bool)` :138-146
- `on_pointer_down(x: i32, y: i32, buttons: &[LaidOutControl])` :149-153
- `on_pointer_up(...) -> Option<u16>` :158-175 (press-must-match-release,
  release hit UNfiltered, disabled re-guard)
- `on_pointer_move(...)` :179-187 (hover + `hover_started_at = Instant::now()`
  on transition — the wall-clock source, std::time::Instant, controller.rs:19+184)
- `on_key(&mut self, key: ShellKey) -> bool` :192-194 — **placeholder**:
  `matches!(key, ShellKey::Enter | ShellKey::Escape) && !self.kbd_route.is_empty()`
  — does not identify a control, dialog, or pop anything.
- `hit_any` :210-212 = `buttons.iter().find(|c| c.rect.contains(x, y))` —
  first-in-feed-order, no area tie-break.
- `ShellKey` enum :27-32 = `{ Tab, Enter, Escape }`;
  `Self::shell_key_for_code(code)` maps winit code → ShellKey (app.rs, used at
  :2117).

Existing model for a CONSISTENT controller-routed close (the pattern D-B3
should follow) — validation modal:
- `route_validation_modal_key` (app.rs:1346-1359): guard state → `ensure_active`
  → `on_key(key)` → on true `close_validation_modal_from_controller`.
- `close_validation_modal_from_controller` (app.rs:1339-1344): dismiss UI state,
  then `if state.shell_controller.top_id() == Some(id) { state.shell_controller.pop(); }`.
This is the only place in the app that ever calls `pop()` (grep verified:
controller.rs internals + app.rs:1342 only).

D-B3 shape: give exit-confirm the same treatment — when
`state.exit_confirm_modal.is_some()`, Esc routes
`ensure_active(0x120, true)` → `on_key(Escape)` → on consume: clear
`exit_confirm_modal` + `pop()` if `top_id()==0x120` (Esc = IDCANCEL → result 1
→ stay, per modal.rs:152-157 `quit_confirm_quits` = MessageBox(0) only). The
egui-only dialogs (options/movies/campaign) are NOT on the controller stack
(never pushed — grep: no ensure_active/push for them), so `close_main_menu_dialogs`
remains correct for them; only the exit-confirm arm needs controller routing.
Mouse-up CANCEL (app.rs:1721-1724) should pop too for consistency.

### 3.5 Esc-flow context for the other shells (unchanged by D-B3, for reference)

app.rs:2114-2144 (KeyboardInput, after the modal arm): validation-modal key
route (skirmish) → skirmish Esc (`close_native_skirmish_shell`; choose-map modal
open ⇒ swallow) → single-player Esc (`close_single_player_shell`) → skirmish
typed-key input → in-game hotkeys (`app_input::handle_hotkey_pressed`).
None of these touch the controller stack on Esc either, but they're out of
D-B3's stated scope (exit-confirm only).

---

## 4. Shell tooltip today (A4 ground truth)

### 4.1 Main menu 0xE2 — tooltip with NO delay

`src/app_main_menu_shell_render.rs:155-162` — VERIFIED verbatim (anchor exact):

```rust
if let Some(id) = hovered_button {
    out.push(PaintLabel {
        text: resolve_csf(state, tooltip_csf_key_for_control(id)),
        rect: layout.tooltip_line,
        align: ShellAlign::H_CENTER,
        rgb: SHELL_TEXT_RGB_ENABLED,
    });
}
```
Inside `main_menu_paint_labels` (:105-164). `hovered_button` comes from the
mirror `state.main_menu_shell_state.hovered_owner_draw_button` (fed by
`mirror_shell_controller_to_main_menu`, app.rs:1556-1565). Emitted the same
frame hover starts — no timer consulted. The 0xE2 button-paint call passes
`hover_started_at = None` outright (app_main_menu_shell_render.rs:389 +
comment :387).

### 4.2 hover_started_at — armed but tooltip-unused

- Armed: `DialogController::on_pointer_move` sets
  `hover_started_at = new_hover.map(|_| Instant::now())` on hover TRANSITION
  only (controller.rs:179-187). Accessor `hover_started_at() -> Option<Instant>`
  (controller.rs:127-129).
- Mirrored: app.rs:1576-1577 into
  `single_player_shell_state.hover_started_at: Option<Instant>`
  (ui/single_player_shell/state.rs:55); cleared on shell open/close
  (app.rs:597, 605).
- SOLE consumer: 0x100 hover-flash square wave `steady_frame_choice`
  (`src/render/shell_paint.rs:223-241`):
  `duration_since(start).as_millis() / 1000 % 2 == 1` (~1 Hz, first high phase
  at 1s), gated by `policy.hover_flash` (0xE2's policy disables it).
  Render passes `now = Instant::now()` at app_single_player_shell_render.rs:230.
- **No tooltip-delay timer exists anywhere** (grep `tooltip` across src/: only
  CSF key tables `tooltip_csf_key_for_control`, descriptor `tooltip_key` field,
  layout `tooltip_line` rects, and skirmish status-help — no service module).

### 4.3 Wall-clock sources for A4's injectable clock

All shell timing today is raw `std::time::Instant::now()` call sites:
controller.rs:184; app_single_player_shell_render.rs:184, 230;
app_main_menu_shell_render.rs:293, 350, 389; mission-announcement deadline
app_sim_tick.rs:849. There is NO shared clock abstraction — A4's injectable
clock is new; the existing testable-time pattern to mirror is
`steady_frame_choice(b, policy, now: Instant, hover_started_at)` — pass `now`
in as a parameter, never call `Instant::now()` inside the model.

### 4.4 Skirmish status-help (immediate, for contrast)

`hovered_shell_control` → `status_help_key_for_hover` updates per mouse-move
(app.rs ~1222-1242 per rust-current.md §2.4; not re-read line-by-line this
session — status-help is outside A4's stated sidebar+shells scope but shares
the "immediate" behavior A4 changes for the shells).

### 4.5 In-game tooltips

NONE exist (no cameo name/cost tooltip; grep confirms no tooltip code under
src/sidebar/ or app_sidebar_*). A4's sidebar consumer starts from zero; content
source precedent: `resolve_csf_name` (app_sidebar_render.rs:465-472 — `Name:`
CSF keys via `state.csf: Option<CsfFile>`, `csf.get(key) -> Option<&str>`),
and `SidebarItem.display_name`/`cost` already carried on the view
(sidebar/mod.rs:155-176).

---

## 5. Text rendering + audio facilities (A4/A5 building blocks)

### 5.1 Text — two wrappers over one BitFont

- Font: `AppState.bit_font: BitFont` (app.rs:241) — built from retail
  `GAME.FNT` (`BitFont::from_fnt`, app.rs:2392-2400; fallback 5x7 at :2333).
  Always present (not Option).
- **Path A (shells):** `src/render/shell_text.rs` — `ShellAlign` bitflags
  (0x01 h-center / 0x02 h-right / 0x04 v-center), `draw_in_rect` →
  `ShellTextDraw { instances: Vec<SpriteInstance>, scissor: ScissorRect }`
  (per-pixel scissor clip, wrap layout, vertical center, `max_height` cutoff,
  `Reveal` window). Consumed via `shell_paint::paint_labels(&state.bit_font,
  &labels)` where `PaintLabel { text, rect: RectPx, align, rgb }`
  (app_single_player_shell_render.rs:233-234). Text color const:
  `SHELL_TEXT_RGB_ENABLED: [f32;3] = [1.0,1.0,0.0]` (shell_paint.rs:46).
  **A5's text-wrap requirement is already implemented here** — the TextLabel
  surface should reuse Path A.
- **Path B (sidebar):** `src/render/sidebar_text.rs` — single-line
  `build_text(font, text, x, y, scale, depth, tint, camera_offset) ->
  Vec<SpriteInstance>` + `build_text_with_fade` (Ready-text fade),
  `text_width`, `glyph_height`. Consumed by `app_sidebar_build.rs`
  (`build_sidebar_text_instances` :576+) and drawn in the `"sidebar_text"`
  pooled buffer (app_render/draw_passes.rs:516-522).

### 5.2 UI sound plumbing (A4 kill-on-press / A5 insert-sound call path)

- Core: `AppState::play_shell_ui_sound_by_id(state, Option<&str>)`
  (app.rs:1848-1861) → `SfxPlayer::play_sound(&mut self, sound_id: &str,
  registry: &SoundRegistry, assets: &AssetManager, audio_indices:
  &[AudioIndex]) -> bool` (src/audio/sfx.rs:168-199; resolves sound.ini
  registry first, then audio.bag direct — the EVA fallback).
- Shell button click: `play_main_menu_button_sound` (app.rs:1790-1797) reads
  `rules.general.gui_main_button_sound` ([AudioVisual] GUIMainButtonSound,
  ruleset.rs:299/972). Other parsed UI sounds: `generic_click_sound`,
  `gui_checkbox_sound`, `gui_combo_open_sound`, `gui_combo_close_sound`
  (ruleset.rs:299-307). Shells play it on mouse-DOWN-over-button
  (app.rs:1592-1599 — distinct from G22's silent-press for sidebar gadgets).
- A5 message-insert sound: NO message-sound rules key is parsed yet (grep
  `gui_` in ruleset.rs → only the five above). The gamemd key must come from
  the binary/INI lane; Rust plumbing = new GeneralRules field +
  `play_shell_ui_sound_by_id`.
- Sim-side sounds: `GameSoundEvent` enum (src/audio/events.rs) +
  `state.sound_events: SoundEventQueue` drained per frame; EVA ids resolved
  via `state.eva_registry.get("EVA_...", faction)` (EvaRegistry from
  eva.ini/evamd.ini, app.rs:330-332; example app_sim_tick.rs:416-420).
  Sim-originated sounds go through this; UI-originated ones call sfx directly.

### 5.3 CSF strings

`AppState.csf: Option<CsfFile>`; `csf.get(key) -> Option<&str>`. Helpers:
`AppState::csf_label(state, key, fallback) -> String` (app.rs:860-867);
render-side `resolve_csf(state, key) -> &str`
(app_main_menu_shell_render.rs:91-97); sidebar `resolve_csf_name`
(app_sidebar_render.rs:465-472).

### 5.4 Existing message/toast surfaces A5 must coexist with

- `state.mission_announcement: Option<String>` +
  `mission_announcement_deadline: Option<Instant>` (app.rs:315-318); set by
  `TriggerEffect::MissionAnnouncement` with a FIXED 4-second deadline
  (app_sim_tick.rs:846-850); cleared at deadline (app_sim_tick.rs:171-172) and
  on transitions (app_transitions.rs:166-167); drawn as an egui banner
  (`ui/mission_status.rs:6-30`, `draw_mission_banner`, top-center Area,
  `interactable(false)`) from app.rs:2926. A5's 14-slot TextLabel list is the
  substrate-shaped replacement target, but no other chat/system-message system
  exists (grep: no chat module, no message list).
- EVA voice cues (§5.2) are independent of any text surface.

---

## 6. Module conventions, input state, frame hooks (A0 placement)

### 6.1 Module layout (src/lib.rs verified)

- Reusable layers: `pub mod assets, render, rules, sim, ui, sidebar, audio,
  map, net, util` (lib.rs:9-39).
- App layer = FLAT `app_*.rs` modules at crate root (lib.rs:42-150): app,
  app_init, app_input, app_render (dir: mod/build_instances/draw_passes/
  merge_passes), app_sidebar_render, app_sidebar_build, app_sidebar_gadgets,
  app_sidebar_text, app_commands, app_sim_tick, app_main_menu_shell_render,
  app_single_player_shell_render, app_skirmish_shell_render (+subdir), etc.
- `ui/mod.rs` (27 ln, verified): client_theme, game_screen, in_game_hud,
  main_menu, main_menu_dialogs, main_menu_shell, mission_status, pause_menu,
  shell, single_player_shell, skirmish_shell. Layering header: "ui/ depends
  on: sim/; does NOT depend on: assets/, render/, sidebar/, audio/, net/".
- `ui/shell/mod.rs`: controller, descriptor, geom, layout, modal.
- → A0's `ui::gadget` = new `src/ui/gadget/` submodule tree declared in
  ui/mod.rs (render-agnostic like ui/shell; ~600 ln/file convention). Its
  app-side driver = a new flat `app_*` module (pattern: app_sidebar_gadgets.rs).
  A4 tooltip service: ui-layer model + app/render consumer, same split.

### 6.2 Input state available to a gadget tick (G6/G8/G9 inputs)

There is NO InputSnapshot struct. Raw winit events are dispatched per-arm in
`window_event` (app.rs:2067-2311). Persistent state on AppState:
- `cursor_x/cursor_y: f32` (app.rs:138-139) — RENDER-TARGET coords (upscale
  remap at app.rs:2161-2175, InGame/SpawnPick only). G6 coordinate source.
- `keys_held: HashSet<KeyCode>` (app.rs:140); modifier helpers
  `is_shift_held/is_ctrl_held/is_alt_held` (app_input.rs:795-810) — G9
  modifier-word source.
- Mouse-button HELD state is NOT tracked for left/right (only event edges);
  `middle_mouse_panning`, `minimap_dragging`, `selection_state` are bespoke
  per-gesture booleans. **A0's G8 held bits need a new per-frame button-held
  record owned by the gadget service** (fed from MouseInput edges) — nothing
  exists to reuse.
- egui sees every event first; `egui_consumed` is ignored in-game except
  paused/save-load-panel (app.rs:2070-2081).

### 6.3 Frame/render hook points

- Per-SIM-tick hook (flash cadence): `app_sim_tick.rs:217-218`
  (`update_power_bar_anim` → `update_sidebar_gadget_state`).
- The A0 event-flag tick is per-frame/per-event; the per-frame host for the
  in-game surface is `render_frame` (app.rs:2678) → InGame arm app.rs:2904-2914
  (`app_render::render_game`).
- In-game screen-fixed UI draw order (`app_render/draw_passes.rs:452-529`, all
  `draw_pooled_ui`, zoom=1.0 camera): minimap → viewport_rect → sidebar(fills)
  → sidebar_chrome → radar_anim → sidebar_cameo → sidebar_gclock →
  sidebar_cameo_overlay → sidebar_text → **software_cursor (LAST)**.
  → A1's gadget-backed buttons keep drawing through the existing sidebar
  buffers; the A4 tooltip draw-last slot (study O10) = a new pooled buffer
  drawn between `sidebar_text` (:516-522) and `software_cursor` (:523-529).
  Buffers upload by name in `app_render/mod.rs:227-245` (`pool.upload`), built
  in `app_render/build_instances.rs:674-751`.
- Shell render paths (A4 shell consumer): main menu `render_main_menu_shell`
  (app.rs:2829), single-player `render_single_player_shell` (app.rs:2797;
  buffer order movie → chrome → buttons → text → cursor,
  app_single_player_shell_render.rs:215-272). The shell tooltip is just a
  PaintLabel in the text buffer (already before cursor) — A4 changes WHEN it
  is emitted, not the pass structure.

---

## 7. R1 — in_game_hud.rs dead-code verification

Grep `in_game_hud|draw_in_game_hud` over worktree `src/`: exactly 3 hits —
`src/ui/in_game_hud.rs:26` (definition `pub fn draw_in_game_hud(...) ->
InGameHudAction`), `src/ui/in_game_hud.rs:43` (internal egui Area id string),
`src/ui/mod.rs:16` (`pub mod in_game_hud;`). `InGameHudAction` has no external
referent (any `use ...in_game_hud...` path would have matched the grep). File
is 210 lines; header self-describes as "Temporary bridge until the custom RA2
sidebar is implemented". **CONFIRMED zero callers.** Delete the file + the
`pub mod in_game_hud;` line (ui/mod.rs:16); verify with
`cargo check -p vera20k` (package name per CLAUDE.md).

---

## 8. Test patterns to mirror

- **In-module `#[cfg(test)] mod tests`** is the universal pattern (no separate
  tests/ dir for UI): controller.rs (6 tests; pure structs + hand-built
  `Vec<LaidOutControl>` feeds, `RectPx::new`, controller.rs:215-322), modal.rs
  (11), geom.rs (7), layout.rs (3), gadget_flash.rs (11, pure state-machine
  cycle tests), app_sidebar_gadgets.rs (6 — private `orchestrate` helper
  mirrors the AppState-coupled body on a bare struct: THE pattern for testing
  app-layer logic headlessly), sidebar/mod.rs + sidebar_view.rs
  (build_sidebar_view with empty slices + None atlas sizes).
- **The 87-test net:** `src/ui/skirmish_shell/state/tests.rs` —
  `grep -c "#[test]"` = **87** (verified this session). Headless fixtures
  (`test_map_entry`, `test_scenario_record` via `IniFile::from_str`,
  tests.rs:26-62) — no GPU/AppState. Untouched by this plan's slices but must
  stay green.
- Parity-pinned test naming cites the native artifact:
  `command_results_match_dialog_proc_0x52d640`
  (ui/single_player_shell/state.rs:110).
- A0's exhaustive G-clause tests: pure `ui::gadget` module tests with
  hand-built gadget lists + injected `now`/tick params (mirror controller.rs
  tests + the pass-now-as-param pattern of `steady_frame_choice`).
- `LaidOutControl { pub id: u16, pub rect: RectPx }` (ui/shell/layout.rs:17-20)
  — the minimal feed-struct precedent for the gadget list's entries.
- `RectPx::contains` is HALF-OPEN (`x < self.x + self.w`,
  ui/shell/geom.rs:34-36, verified) — matches the study's G14 gadget rule;
  `sidebar::Rect::contains` is INCLUSIVE (§2.1). A0 should reuse/mirror the
  RectPx convention, NOT sidebar::Rect.

---

## 9. Anchor verification ledger (study/plan cites vs worktree)

| Plan/study anchor | Worktree verdict |
|---|---|
| app_input.rs:39-43 sidebar fires on press | VERIFIED exact |
| app_input.rs:227-238 handle_sidebar_mouse_input | VERIFIED exact |
| app_input.rs:240-318 apply_sidebar_action | VERIFIED exact |
| app_input.rs:210-225 wheel scroll | VERIFIED exact |
| sidebar/mod.rs:379-425 hit_test | VERIFIED exact |
| sidebar/mod.rs:61-63 Rect::contains INCLUSIVE | VERIFIED exact |
| sidebar/mod.rs:273-333 adaptive rows | VERIFIED (fn body 275-333; MIN_VISIBLE_ROWS at :35) |
| app.rs:1950-1955 close_main_menu_dialogs | VERIFIED (body 1950-1955, doc comment 1949) |
| app.rs:2106-2112 Esc bypass | VERIFIED exact |
| controller.rs:192-194 on_key placeholder | VERIFIED exact |
| controller.rs:210-212 hit_any | VERIFIED exact |
| app_main_menu_shell_render.rs:155-162 tooltip emission | VERIFIED exact |
| ui/in_game_hud.rs zero callers (R1) | VERIFIED (3 hits: def, internal id string, mod decl) |
| 87-test net location/count | VERIFIED = 87 in skirmish_shell/state/tests.rs |
| app.rs:1935-1936 "0x120 over the menu's 0xE2" comment | MISLEADING: `ensure_active` reset_to-CLOBBERS the stack to [0x120]; nothing pushes over 0xE2; depth stays 1 (controller.rs:97-111) |
| A1 "2 strip-scroll" buttons | DO NOT EXIST in worktree — new gadgets; wire to `sidebar_scroll_rows`/`max_scroll_rows` (§1.3) |
| Sidebar click sound/press-flash to preserve in R8 flip | NONE exists today — nothing to preserve (§1.2) |

NO line drift found on any in-scope anchor: the worktree (7b79a186) matches
every rust-current.md cite re-checked above.
