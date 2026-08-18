# A2/A3 grounding — current Rust architecture map (HEAD 801bc09e)

Lane: Rust-codebase grounding for the A2 (cameo strip click surface) and A3
(tactical + minimap click regions) implementation plan, flipping them onto the
already-shipped gadget substrate (`src/ui/gadget/` + `src/app_gadget_input.rs`).

ALL file:line cites verified live this session against the main checkout
`C:/Users/enok/Documents/ra2-rust-game`, branch `dev`, HEAD `801bc09e`, clean
`git status`. NO Ghidra (by assignment). Companion docs in same dir:
`plan-grounding-rust.md` (A0/A1/A4/A5 anchors), `rust-current.md`.

Substrate status: A0/A1/A4/A5 merged — the in-game gadget list today owns ONLY
the 8 chrome buttons (4 tabs, repair, sell, scroll-down, scroll-up). Cameos,
tactical, minimap, and the dev/control/pause/producer buttons are all still on
the legacy press path.

---

## 0. The event-loop seam (where everything enters)

- `src/app.rs:2332-2334` — in-game mouse buttons dispatch to
  `app_input::handle_mouse_input(state, button, btn_state)` (last arm, only when
  `!egui_consumed && screen == InGame`).
- `src/app.rs:2243` — cursor moves dispatch to
  `app_input::handle_cursor_moved_in_game(state)` (InGame or SpawnPick).
- `src/app_sim_tick.rs:212` — `app_gadget_input::idle_tick(state)` runs once per
  frame (drives the masked-0 sticky re-dispatch for the G22 drag-off/back visuals).

`handle_mouse_input` (`src/app_input.rs:34-169`) order, TODAY:
1. `app_gadget_input::handle_mouse_button_event` (gadget substrate) — `:43`. If
   it returns `true` → `return` (consumed; never falls through). This is the seam
   A2/A3 extend.
2. On press only: `handle_sidebar_mouse_input` (`:46-50`) → legacy
   `sidebar::hit_test` press path (cameos + pause/producer/dev buttons).
3. Left press: `try_begin_minimap_drag` (`:54`) → targeting-mode suppress (`:57`)
   → `selection_state.begin_drag` (`:60`).
4. Left release: minimap-drag end (`:64`) → SW launch (`:68`) → building place
   (`:72`) → `end_drag` → context-order / selection (`:76-145`).
5. Middle: pan (`:148-156`). Right press: targeting cancel / deselect (`:157-166`).

---

## 1. A2 — current cameo click path (legacy, fire-on-PRESS)

### 1.1 Where a cameo click fires today

Cameos fire on **mouse-DOWN** via `handle_sidebar_mouse_input`
(`src/app_input.rs:234-245`), reached only on press (`:46`
`if btn_state.is_pressed()`). Both Left and Right press are probed;
`right_click = button == MouseButton::Right` (`:238`).

```rust
let action = sidebar::hit_test(&view, state.cursor_x, state.cursor_y, right_click);
if action == SidebarAction::None { return false; }
apply_sidebar_action(state, action);
true
```

`sidebar::hit_test` (`src/sidebar/mod.rs:405-438`) walks, in order:
panel-rect gate (`:406`) → **`view.items` (cameos)** (`:410-414`) →
pause_button (`:416`) → producer_button (`:421`) → cancel/cycle_owner/
starter_base/spawn_test_units dev buttons (`:426-435`). The cameo branch is the
FIRST hit after the panel gate, so cameos take priority over the dev buttons.

### 1.2 Cameo click → SidebarAction (`hit_test_item`, `src/sidebar/mod.rs:357-399`)

Per the SidebarItem flags (`is_superweapon`, `is_ready`, `is_armed`, `enabled`,
`queued_count`):

| Click | Item state | SidebarAction |
|---|---|---|
| RIGHT | superweapon | `None` (SW has no queue) — `:360-362` |
| RIGHT | build, `queued_count>0` OR `is_ready` | `CancelBuild(type_id)` — `:364-366` |
| RIGHT | build, nothing queued | `None` — `:367` |
| LEFT | SW, not ready | `None` — `:372-374` |
| LEFT | SW, ready, armed | `ClearSuperWeaponMode` — `:375-376` |
| LEFT | SW, ready, not armed | `ArmSuperWeapon(section)` — `:377-385` (section ← `super_weapon_section`, fallback `display_name`) |
| LEFT | build, ready, armed | `ClearPlacementMode` — `:389-390` |
| LEFT | build, ready, not armed | `ArmPlacement(type_id)` — `:391-392` |
| LEFT | build, not ready, enabled | `BuildType(type_id)` — `:394` |
| LEFT | build, not ready, disabled | `None` — `:396` |

EVERY SidebarItem-derived action (the complete A2 surface):
`CancelBuild`, `ClearSuperWeaponMode`, `ArmSuperWeapon`, `ClearPlacementMode`,
`ArmPlacement`, `BuildType`, plus the `None` no-ops.

### 1.3 How `apply_sidebar_action` handles each (`src/app_input.rs:247-325`)

- `BuildType(id)` → `queue_build_by_type(state, &id)` (`:254`).
- `ArmPlacement(id)` → set `targeting_mode = BuildingPlacement(id)`, clear
  repair/sell-mode flags (`:257-261`).
- `ClearPlacementMode` → clear `targeting_mode` + `building_placement_preview`
  (`:263-265`).
- `ArmSuperWeapon(section)` → `targeting_mode = SuperWeapon(section)`, clear
  preview + repair/sell (`:267-277`).
- `ClearSuperWeaponMode` → clear `targeting_mode` (`:278-280`).
- `CancelBuild(id)` → `cancel_build_by_type(state, &id)` (`:288-289`).

ALL six route to `app_commands` (queue_build / cancel_build) or flip app-layer
`targeting_mode` / `sidebar_gadget_state` flags. **None touch `sim/` directly** —
build/cancel go through `app_commands` which schedules `Command`s into the sim
command queue. Confirms the sim seam: cameo clicks queue commands, no new `sim/`
dependency.

### 1.4 Press-vs-release: cameos are Control-style, NOT Toggle (flag for binary lane)

TODAY cameos fire on press (§1.1). The gadget substrate fires CONTROLS on press
(silent never — `GadgetBehavior::Control` posts `id|0x8000` immediately on any
masked bit) and TOGGLE BUTTONS on release (`GadgetBehavior::Button`, the G22
silent-press/fire-on-release machine — `src/ui/gadget/button.rs:75-141`).

The A2 acceptance tests in the study §8 mention O7 (retained order = hit
priority), G12 (broadcast walk stops at first consumer), tab-swap, and
Mouse_Enter/Mouse_Leave (hover) — they do **NOT** mention the G22 toggle machine
(silent press / drag-off cancel / is_on latch). The chrome buttons that DO use
G22 are tabs/repair/sell (§2). The absence of G22 from the A2 tests is evidence
cameos are modeled as `GadgetBehavior::Control` (fire-on-the-event, no pressed/on
state machine), which would make them fire on PRESS — matching today's behavior.

OPEN QUESTION for the binary lane: does the gamemd cameo gadget post its result
on left-DOWN (Control-style) or left-UP (toggle-style)? If Control on the
left-press event mask (`0x01`), it fires on press and the right-cancel needs the
right-press bit (`0x10`) in the mask plus the `RESULT_RIGHT` (`|0x4000`) marker
that `control_action` already emits (`src/ui/gadget/button.rs:38-51`). This is
the single load-bearing unknown for A2; do NOT assume.

---

## 2. A2 — visible cameo set + scroll + tab computation (driver build/sync source)

`current_sidebar_view` (`src/app_sidebar_render.rs:29-169`) builds the
`SidebarView` each frame from production state; the cameo geometry comes from
`build_sidebar_view_with_spec` (`src/sidebar/sidebar_view.rs:60-323`).

### 2.1 Visible item set + grid rects

- `collect_build_entries` (`sidebar_view.rs:340-488`) gathers ALL entries for the
  active tab's category (SW cameos prepended on the Defense tab, `:360-385`).
- `total_rows = ceil(total_items / CAMEO_COLUMNS)` (`:93`); `CAMEO_COLUMNS = 2`
  (`sidebar/mod.rs:42`).
- `visible_rows = layout.side2_tile_count` (`:180`) — the number of cameo rows
  the chrome shows; computed by `compute_layout_with_spec`
  (`sidebar/mod.rs:284-333`) as `visible_rows` clamped `[MIN_VISIBLE_ROWS=4,
  max_rows]` (stock 1024×768 = 4 rows; test `:452`).
- `max_visible = visible_rows * CAMEO_COLUMNS` items (`:185`).
- The visible slice is `all_entries.skip(visible_items).take(max_visible)`
  (`:184-190`) where `visible_items = scroll_rows * CAMEO_COLUMNS` (`:184`).
- Per-item rect (`:194-205`): `x = sidebar_x + cameo_inset_x + col*(cameo_width +
  cameo_gap_x)`, `y = grid_top + row*row_height`, size `cameo_width × cameo_height`
  — all `.round()`ed. `SidebarItem::cameo_rect()` (`sidebar/mod.rs:180-182`)
  returns `self.rect` verbatim (the full item rect IS the cameo).

So the driver builds at most `2 × visible_rows` cameo gadgets (stock: 8), one per
`view.items` entry, with `rect = rect_px(item.rect)`. Items count and rects are
already computed in the view — the driver mirrors `view.items` exactly (same
pattern as `sync_gadgets` mirroring `view.tabs`, `app_gadget_input.rs:114-117`).

### 2.2 Scroll state

- `state.sidebar_scroll_rows` (app field) is the scroll offset in ROWS.
- `view.max_scroll_rows = total_rows.saturating_sub(visible_rows)` (`:181`).
- The driver already moves it: `apply_gadget_result` ID_SCROLL_DOWN/UP add/subtract
  `view.layout.side2_tile_count.max(1)` (one page = visible rows) clamped to
  `[0, max_scroll_rows]` (`app_gadget_input.rs:278-288`).
- Mouse-wheel path: `try_sidebar_scroll` (`app_input.rs:217-232`) — independent,
  unaffected by A2/A3.

### 2.3 Tab switch

Today `SelectTab` flows through `apply_sidebar_action`
(`app_input.rs:250-253`): sets `state.active_sidebar_tab = tab` AND
`state.sidebar_scroll_rows = 0` (scroll reset on every tab change). The gadget
tab path already calls this via `apply_gadget_result` →
`apply_sidebar_action(SelectTab)` (`app_gadget_input.rs:261-265`). So when the
active tab changes, `current_sidebar_view` rebuilds `view.items` for the new
category and the driver re-syncs the cameo gadget rects/count next frame.

Note: the cameo SET changes per tab and per scroll, so the cameo gadgets are NOT
a fixed-count retained set like the 8 chrome buttons. The driver must
add/remove/re-sync cameo gadgets each frame to match `view.items.len()` (or
maintain a fixed `max_visible` pool and disable the unused tail). `GadgetList`
supports `add_tail`/`remove`/`clear`/`extract_by_id` (`list.rs:148-196`) for
this; `set_enabled` (`list.rs:245-256`) for the pool approach.

---

## 3. A3 — current tactical click path (what MUST be preserved)

All in `handle_mouse_input` (`src/app_input.rs:51-168`). A gadget ClickRegion at
the bottom of the retained list (largest area, so it loses every hit-test tie to
real gadgets but catches the open tactical area) would route ENTRY here; the
logic below must run unchanged when the region "consumes".

### 3.1 Left PRESS (`:52-62`)
- `try_begin_minimap_drag` (`:54`) — see §4; returns early if over minimap.
- `if state.targeting_mode.is_some() { return; }` (`:57-59`) — suppress selection
  drag while building-placement OR SW targeting is armed.
- else `state.selection_state.begin_drag(cursor_x, cursor_y)` (`:60-62`).

### 3.2 Left RELEASE (`:63-146`)
- minimap-drag end: `if state.minimap_dragging { minimap_dragging=false; return; }`
  (`:64-67`).
- SW launch: `if armed_super_weapon_type() ... launch_super_weapon_at_cursor;
  return` (`:68-71`) — `armed_super_weapon_type` = `app.rs:462-466`.
- Building place: `if armed_building_type() ... place_ready_building_at_cursor;
  return` (`:72-75`) — `armed_building_type` = `app.rs:454-458`.
- else `action = selection_state.end_drag(...)` (`:76-78`):
  - `SelectAction::Click(sx,sy)` (`:83-93`): try
    `try_queue_context_order_at_screen_point(state, cx, cy, true)`
    (`app_context_order.rs`); if `commanded` → return. `true` =
    select_friendly_clicks (friendly clicks fall through to selection).
  - Then `Click` → `compute_click_selection_snapshot` (`:97-118`),
    `BoxSelect` → `compute_box_selection_snapshot` (`:119-137`).
  - `emit_selection_voice` + `queue_selection_snapshot_command` (`:142-144`).

### 3.3 Right PRESS (`:157-166`)
- `if targeting_mode.is_some() { targeting_mode=None; building_placement_preview=
  None; return; }` (`:159-163`) — cancel arm.
- else `queue_selection_snapshot_command(state, Vec::new(), false)` (`:165`) —
  clear selection (deselect).

### 3.4 Sim seam
All tactical outputs go through `schedule_command` / `sim.pending_commands` /
`app_commands` (Move, AttackMove, Select, Stop, Deploy, SW launch, building
place). No direct `sim/` mutation from input except selection-snapshot
bookkeeping. Confirms: tactical clicks queue commands, no new `sim/` dependency.

---

## 4. A3 — current minimap path (what MUST be preserved/routed)

- `try_begin_minimap_drag` (`src/app_sidebar_render.rs:224-237`): if
  `is_cursor_over_minimap` (`:201-222`, gated on radar online + minimap present +
  `active_minimap_screen_rect`): if units selected →
  `minimap_move_order_if_selected` (`:241-311`, issues Move/AttackMove to the
  clicked iso cell via `pending_commands`); else set `minimap_dragging = true`,
  cancel selection drag, `update_camera_from_minimap_cursor`.
- `state.minimap_dragging` flag: set here, serviced in
  `handle_cursor_moved_in_game` (`app_input.rs:179-183` →
  `update_camera_from_minimap_cursor`), cleared on left-release
  (`app_input.rs:64-66`).
- `update_camera_from_minimap_cursor` (`app_sidebar_render.rs:345-366`): maps
  cursor over `active_minimap_screen_rect` to camera top-left, clamps.
- `radar_minimap_rect` (`sidebar/mod.rs:66-79`) — the stock-spec minimap rect
  helper (note: the LIVE path uses `active_minimap_screen_rect` in
  app_sidebar_render, NOT `radar_minimap_rect`; the latter is a public
  stock-geometry helper used by tests/callers).

The minimap drag is a STATEFUL gesture (press begins, move services, release
ends) that crosses three handlers. A gadget ClickRegion for the minimap would
need to either (a) own the whole gesture via sticky capture (press acquires,
idle ticks service via the driver, release releases) OR (b) just route the press
into `try_begin_minimap_drag` and leave the existing `minimap_dragging`-flag
move/release path intact. Option (b) is the lower-risk minimal change.

---

## 5. Integration seam — how "consumed" works and where A2/A3 slot in

### 5.1 Consumed contract (TODAY)
`handle_mouse_button_event` (`app_gadget_input.rs:169-192`) → `run_tick`
(`:205-233`) returns `fired || consumed_walk || was_captured`:
- `fired` = `(result & RESULT_BUTTON) != 0` (a control posted `id|0x8000`).
- `consumed_walk` = `out.consumed_by.is_some()` (broadcast walk stopped at a
  gadget — `tick.rs:250`).
- `was_captured` = `focus.sticky.is_some()` at tick start (a release completing a
  captured gesture is always consumed; the sticky tier is exclusive — `tick.rs:210-224`).

`handle_mouse_input` short-circuits on `true` (`app_input.rs:43-45`: `if ...
handle_mouse_button_event(...) { return; }`).

### 5.2 Where the new gadgets slot in (retained order = hit priority = draw order)

The retained list (built once in `sync_gadgets`, `app_gadget_input.rs:121-142`)
is today: tabs[0..3], repair, sell, scroll_down, scroll_up. Hit-test
`g14_smallest_area_wins` with a `<=` tie-break = later-in-list wins on equal area
(`tick.rs:79-93`); the broadcast walk dispatches in LIST order and the first
consumer stops the walk (`tick.rs:242-253`).

Proposed retained order (O7 — one pinned order for hit priority + draw):
1. **chrome buttons** (existing: tabs, repair, sell, scroll pair) — smallest, win
   ties, stay first.
2. **cameo gadgets** (A2) — `view.items.len()` of them, rects from `item.rect`,
   `Control` behavior (pending §1.4 binary confirmation). Disjoint from chrome,
   smaller than the tactical catcher.
3. **minimap ClickRegion** (A3) — `active_minimap_screen_rect`, routes press into
   `try_begin_minimap_drag`. Above the tactical catcher (smaller area).
4. **tactical ClickRegion** (A3) — the open viewport catcher, LARGEST area, LAST
   in list so it loses every hit-test tie and catches only what nothing else does.
   Must be ≤ `HIT_SEED_AREA` (786,432 px²) or it can never win (`tick.rs:81`,
   `mod.rs:53`) — the tactical viewport at high res EXCEEDS this; flag below.

### 5.3 Mapping consumed_by / dispatch back to existing handlers WITHOUT duplicating

`apply_gadget_result` (`app_gadget_input.rs:258-291`) maps a fired `id|0x8000` by
id. Extend it (do NOT re-implement the handlers):
- Cameo gadget id → look the SidebarItem back up (by retained index or stored
  type_id) → call the EXISTING `hit_test_item(item, right_click)` to derive the
  `SidebarAction`, then `apply_sidebar_action(state, action)` (already imported,
  used at `:263`). The right-vs-left split comes from the `RESULT_RIGHT`
  (`|0x4000`) marker `control_action` already sets (`button.rs:46-48`); mask it
  off the id (`:259` already does `& !(RESULT_BUTTON | RESULT_RIGHT)`).
- Minimap region id → call `try_begin_minimap_drag(state)` on the left-press
  dispatch (or route into the existing flag-based gesture).
- Tactical region id → re-enter the §3 logic. Cleanest: keep `handle_mouse_input`
  steps 3-5 as the tactical handler and have the region's "consumed" simply NOT
  short-circuit them (i.e., the region is an observability/ordering wrapper, with
  the actual selection/order code unchanged). This avoids duplicating the
  begin_drag/end_drag/context-order/selection logic.

The honest minimal-risk reading: A2 cameos genuinely move onto the substrate (the
gadget result drives `hit_test_item`+`apply_sidebar_action`). A3 tactical/minimap
"regions" are primarily about *ordering/consumption authority* — they decide
whether the tactical/minimap code runs, but the begin_drag/end_drag/order/launch/
place code in §3-4 stays put and is CALLED from the dispatch, not copied.

---

## 6. R7 retirement — what A2/A3 retire vs preserve

### Retire / narrow
- `sidebar::hit_test` (`sidebar/mod.rs:405-438`): the **cameo branch** (`:410-414`,
  the `for item in &view.items` loop) is retired by A2 — cameos move to the
  gadget list.
- `handle_sidebar_mouse_input` (`app_input.rs:234-245`): narrows to only the
  surfaces still on `hit_test` (pause/producer/dev buttons) once cameos leave.
  It does NOT disappear until A6 (dev/control buttons).
- `handle_mouse_input` tactical/minimap special-casing (`app_input.rs:51-168`):
  ENTRY is gated through A3 ClickRegions, but the BODY is preserved (§5.2-5.3).

### Preserve (stay legacy until A6)
- `sidebar::hit_test` panel gate + pause_button + producer_button +
  cancel/cycle_owner/starter_base/spawn_test_units branches (`:406`, `:416-435`).
- The whole §3 tactical body (begin/end drag, context order, SW launch, building
  place, selection, right-cancel) — moved-under-region, not deleted.
- The whole §4 minimap body (`try_begin_minimap_drag`,
  `minimap_move_order_if_selected`, `update_camera_from_minimap_cursor`,
  `handle_cursor_moved_in_game` drag servicing).
- `try_sidebar_scroll` (mouse-wheel, `app_input.rs:217-232`) — orthogonal.

### Retire / Preserve / Add table

| Element | file:line | A2/A3 verdict |
|---|---|---|
| `sidebar::hit_test` cameo loop | `sidebar/mod.rs:410-414` | RETIRE (A2) |
| `hit_test_item` | `sidebar/mod.rs:357-399` | PRESERVE — reused to map cameo gadget id→action |
| `handle_sidebar_mouse_input` | `app_input.rs:234-245` | NARROW — drops cameos, keeps dev/pause/producer until A6 |
| `sidebar::hit_test` pause/producer/dev branches | `sidebar/mod.rs:416-435` | PRESERVE (until A6) |
| `apply_sidebar_action` | `app_input.rs:247-325` | PRESERVE — gadget path already calls it |
| Tactical body (begin/end drag, order, SW, place, select, right-cancel) | `app_input.rs:51-168` | PRESERVE — moved under A3 ClickRegion, not duplicated |
| `try_begin_minimap_drag` + minimap gesture | `app_sidebar_render.rs:224-366` | PRESERVE — routed via A3 region |
| `handle_cursor_moved_in_game` minimap/selection drag | `app_input.rs:179-213` | PRESERVE |
| Cameo gadgets (Control, `view.items`-driven) | new | ADD (A2) |
| Minimap ClickRegion | new | ADD (A3) |
| Tactical ClickRegion (catcher) | new | ADD (A3) |
| `apply_gadget_result` cameo/region arms | `app_gadget_input.rs:258-291` | ADD — extend, don't rewrite |
| `sync_gadgets` cameo/region sync | `app_gadget_input.rs:106-162` | ADD — per-frame cameo add/remove/sync |

---

## 7. cfg(test) / sim fixtures

No `sim/` fixture changes needed for A2/A3. Both surfaces queue commands through
`app_commands` / `schedule_command` / `sim.pending_commands` — the sim command
API is unchanged; only the app-layer input routing moves. Existing tests:
`sidebar/mod.rs:440-524` (hit_test_item SW cases), `sidebar_view.rs:490-601`
(layout + `hit_test` control-button), and the gadget substrate tests in
`ui/gadget/*` remain valid. New tests would be app/gadget-layer (cameo gadget
id→`SidebarAction` mapping; region consumption ordering), not `sim/`.

---

## 8. Open questions for the binary lanes

1. **A2 press-vs-release (LOAD-BEARING).** Does the gamemd cameo gadget post its
   result on left-DOWN (Control / fire-on-press, matching today) or left-UP
   (toggle / fire-on-release)? §1.4. If Control: mask `0x01` (left press) for the
   build/arm action, mask `0x10` (right press) + `RESULT_RIGHT` for cancel.
2. **A2 cameo right-click marker.** Does the cameo gadget's mask include the
   right-press bit so `control_action` emits `id|0xC000` for the cancel branch
   (`button.rs:46-48`)? Or is right-cancel a separate gadget / separate path?
3. **A2 cameo gadget identity/Kind.** Are cameos plain `Control` (no
   pressed/on-state) or do they carry any `is_on`/latch state (e.g., the armed
   cameo highlight)? If the armed highlight is a button `is_on` latch, they may be
   `ToggleKind::Flip`/`LatchOn` after all — confirm against the binary.
4. **A2 hit priority / Mouse_Enter-Leave.** Study §8 names O7 + Mouse_Enter/Leave
   for cameos. Confirm the cameo gadgets sit AFTER the chrome buttons in retained
   order and whether they emit hover (the substrate already tracks hover via
   `out.hover_entered`/`hover_left`, `tick.rs:178-184`).
5. **A3 tactical catcher area vs HIT_SEED_AREA.** The hit-test seed caps winnable
   area at 786,432 px² (`mod.rs:53`, `tick.rs:81`); a full-screen tactical
   viewport at high res exceeds this. How does gamemd's tactical catcher win the
   hit-test — is it the seed-area trick (it never wins hit-test but IS the
   broadcast-walk fallback?), or a separate non-gadget path? §5.2.
6. **A3 minimap as gadget vs flag-gesture.** Does gamemd model the minimap as a
   sticky-capture gadget owning the whole drag gesture, or as a press-detect that
   hands off to a camera-follow flag (today's `minimap_dragging`)? §4.
7. **A3 tactical/minimap consume semantics.** Does the tactical/minimap region
   "consume" in the gadget sense (post `id|0x8000`, stop the walk), or is it a
   pass-through that only orders the dispatch? This decides whether §3-4 code runs
   from `apply_gadget_result` or stays in `handle_mouse_input` post-substrate.
