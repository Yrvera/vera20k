---
title: Sidebar Tab-Flash + Repair/Sell Rendering Design
date: 2026-05-20
status: draft (awaiting plan)
scope: G1 (Repair/Sell render) + A20 (tab flash + pressed/active SHP frames)
related:
  - ra2-rust-game-docs/SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md
  - ra2-rust-game-docs/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md
  - ra2-rust-game-docs/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md
  - ra2-rust-game-docs/SIDEBAR_INIT_GADGET_POSITIONING_GHIDRA_REPORT.md
  - docs/gap-scans/2026-05-20-disparity-scan-sidebar.md
---

# Sidebar Tab-Flash + Repair/Sell Rendering Design

## Goal

Bundle two HIGH-severity sidebar disparities (G1 — Repair/Sell never render; A20 — tab
buttons stuck on idle/active with no flash or pressed-look state) by introducing a
single shared gamemd-mirror gadget-flash primitive, the 5-frame state-select function,
and the persistent Repair-mode / Sell-mode flags, so the in-game sidebar renders the
state-driven overlay layer that gamemd uses to communicate production progress and
input affordance.

## Architecture Context

**Where the sidebar lives.** `src/sidebar/` is render-agnostic state + hit-testing
(geometry, tab/item view model, item enabled/queued flags). `src/render/sidebar_chrome.rs`
loads MIX assets into an atlas texture (per-theme: Allied / Soviet / Yuri). The actual
draw calls live in `src/app_sidebar_build.rs`, which builds wgpu instances from the
chrome atlas + a per-frame `SidebarView` snapshot.

**View lifecycle.** `SidebarView` is **rebuilt every render frame** in
[src/app_sidebar_render.rs:102-119](../../src/app_sidebar_render.rs#L102-L119)
inside `current_sidebar_view`. So `SidebarTabButton` is a frame-local snapshot, not
durable state — any tab-button state that must survive across frames lives on
`AppState`, not on the view.

**Persistent UI-animation precedent.** [src/sidebar/power_bar_anim.rs](../../src/sidebar/power_bar_anim.rs)
defines `PowerBarAnimState`: a self-contained struct with internal counters, a
`tick()` advancing them, an `update()` called with current sim values, and a
read-only API (`segment_counts`, `is_flashing`) consumed by the renderer.
`AppState.power_bar_anim` ([src/app.rs:171](../../src/app.rs#L171)) holds it;
[src/app_sim_tick.rs:207](../../src/app_sim_tick.rs#L207) ticks it once per render
frame. This is the model the gadget-flash primitive follows.

**Sim → app flow.** Production state already exposes everything we need by polling:
- `production::queue_view_for_owner(sim, rules, owner) -> Vec<QueueItemView>` —
  every queued item with `queue_category`, `state`, `progress`. Aircraft completion
  shows up as `QueueItemView { queue_category: Aircraft, state: Done, .. }` or as a
  Vehicle-tab item with `progress == 1.0`.
- `production::ready_buildings_for_owner(...) -> Vec<ReadyBuildingView>` — already
  used for the "Ready" badge.
- `crate::sim::superweapon::superweapon_views_for_owner(...) -> Vec<SuperWeaponView>` —
  each carries `is_ready: bool`.

These three views are already computed each frame by `current_sidebar_view`. The
gadget-state orchestrator can call the same accessors without sim hooks.

**Tab ↔ gamemd-tab mapping.** Rust uses 4 tabs (Building/Defense/Infantry/Vehicle).
Aircraft cameos live on the Vehicle tab (per [src/sidebar/sidebar_view.rs:347-349](../../src/sidebar/sidebar_view.rs#L347-L349)),
SuperWeapon cameos live on the Defense tab (per
[src/sidebar/sidebar_view.rs:316-339](../../src/sidebar/sidebar_view.rs#L316-L339)).
So gamemd's "Tab 0 = Aircraft → flash on aircraft complete" maps to **Vehicle tab
flashes on aircraft complete**, and gamemd's "Tab 1 = Defense → flash on SW ready"
maps to **Defense tab flashes on SW ready**. Building and Infantry tabs never flash.

**Targeting mode.** [src/app_types.rs](../../src/app_types.rs) `TargetingMode` is the
existing mutex for "cursor is in a special arming state": `BuildingPlacement(String)`
or `SuperWeapon(String)`. Repair-mode / Sell-mode are conceptually the same
("clicking on the tactical map does something other than select"), but per Q1 answer
in the brainstorm, the cursor swap and click-target resolution are **deferred**. This
design only adds the persistent on/off flag and wires it to the gadget's frame-select.

## Impact Analysis

### Touched files (writes)

| File | Change |
|---|---|
| `src/sidebar/mod.rs:178-183` | `SidebarTabButton` gains `frame_index: u8` (replaces `active: bool` as the render-facing field — `active` is still computed inside the view builder, just not exposed). |
| `src/sidebar/mod.rs:127-146` | Add `SidebarAction::ToggleRepairMode` and `SidebarAction::ToggleSellMode` variants. |
| `src/sidebar/mod.rs` | Add `pub mod gadget_flash;` and re-export `GadgetFlash` + `SidebarGadgetState`. |
| **NEW** `src/sidebar/gadget_flash.rs` | The shared primitive: `GadgetFlash { state, period, countdown, disabled }` with `start`/`stop`/`tick`. Free function `frame_select(disabled, mode_active, pressed_state) -> u8` implementing the 5-frame state table. Plus `SidebarGadgetState` aggregating per-tab flash + repair/sell mode flags. |
| `src/sidebar/sidebar_view.rs:109-131` | Tab population reads from `SidebarGadgetState` (passed in) and computes `frame_index` via `frame_select`. |
| `src/sidebar/sidebar_view.rs:53-70` | `build_sidebar_view_with_spec` signature gains `gadget_state: &SidebarGadgetState`. |
| `src/app.rs:171` neighborhood | Add `pub(crate) sidebar_gadget_state: SidebarGadgetState`. Initialise in `Default`. |
| **NEW** `src/app_sidebar_gadgets.rs` | Orchestrator: `update_sidebar_gadget_state(state)` — polls production for the local owner, computes triggers, calls `start`/`stop` on the 4 tab flashes, ticks all gadgets. Mirrors `app_building_anim::update_power_bar_anim`. |
| `src/app_sim_tick.rs:208` | Add call to `app_sidebar_gadgets::update_sidebar_gadget_state(state)` right after `update_power_bar_anim`. |
| `src/app_sidebar_render.rs:102-119` | Pass `&state.sidebar_gadget_state` into `build_sidebar_view_with_spec`. |
| `src/render/sidebar_chrome.rs:285-293` | Load tab0N frames 0–4 (was 0+1); load repair.shp frames 0–4 (was 0); load sell.shp frames 0–4 (was 0). Replace `tab_entries` + `tab_active_entries` with `tab_frames: [[RenderedChromeEntry; 5]; 4]`. Replace `repair` / `sell` Option<single-frame> with `repair_frames: [RenderedChromeEntry; 5]` / `sell_frames: [RenderedChromeEntry; 5]`. |
| `src/app_sidebar_build.rs:170-193` | Delete the OBSERVER.PAL TODO comment block. Uncomment Repair/Sell render. Pick frame index from `state.sidebar_gadget_state` (repair_mode_on / sell_mode_on / disabled). |
| `src/app_sidebar_build.rs` tab render block | Pick tab frame from `tab.frame_index` instead of `if tab.active { tab_active } else { tab }`. |
| `src/app_input.rs` or `src/app_commands.rs` (whichever owns sidebar-click dispatch) | Add handlers for `ToggleRepairMode` / `ToggleSellMode` — flip the persistent flag, clear the other, clear `targeting_mode`. |
| `src/sidebar/mod.rs` `hit_test` | Route Repair/Sell button rect hits to the new action variants. (The chrome already has a clickable region; needs to be added to the view.) |

### Reverse-dependencies that change behavior subtly

- **TargetingMode clearing.** Arming building placement or a SW currently doesn't
  know about repair/sell mode. We need a single helper (`arm_targeting_mode`?) that
  clears all three classes of cursor-mode (TargetingMode + repair_mode_on + sell_mode_on)
  in one place. Otherwise it's easy to end up with Repair-mode "on" while a
  building is also armed.
- **Sidebar action hit-test.** Today's `hit_test` does not test Repair/Sell rects
  because they don't exist in `SidebarView`. The view builder needs to compute and
  expose these rects (mirroring the Tab 0/Tab 1 layout but for repair/sell).

### Risk areas

1. **Tick-rate parity.** Per the ledger, the gamemd flash period is **exactly 10
   game-logic ticks**. `power_bar_anim.tick()` runs once per render frame,
   regardless of sim speed — which is fine for the power bar but would be wrong for
   tab flash (would tick faster/slower than gamemd at non-default speeds).
   **Mitigation:** the orchestrator caches `last_sim_tick`. On each render-frame
   call, it iterates `flash.tick()` exactly `sim.tick - last_sim_tick` times. At
   default speed this is 0–1 per frame; under catch-up it could be more, and the
   loop correctly preserves phase. Phase parity uses `sim.tick` as the boundary
   counter, not render-frame count.

2. **First-frame ordering.** `update_sidebar_gadget_state` must run **before**
   `current_sidebar_view` in the same frame, otherwise the view reads stale frame
   indices. Already true: `app_sim_tick.rs:207` is well before render-pass build,
   which is where `current_sidebar_view` is called.

3. **SHP frame count in retail assets.** Per the docs (YELLOW in §10 of both reports),
   it is not yet verified that the retail `tab00..tab03.shp` actually contain 5
   frames, and likewise for `repair.shp` / `sell.shp`. If a frame is missing, the
   load step will return None for that index and we fall back to frame 0 for that
   state. **Mitigation:** in the chrome loader, log a warning when a requested
   frame is missing and substitute frame 0 + use the gadget's idle path. This is a
   one-time per-theme-load check, not a hot-path branch.

4. **Mutex with cursor system.** Per Q1, cursor swap is deferred — but Repair-mode-on
   has zero observable effect in v1 beyond the button frame. We must not advertise
   "Repair-mode is on" to anything else until the cursor work is designed in a
   follow-up. The flag is therefore `pub(crate)` and only consumed by the view
   builder + the click handler (clear-the-other logic).

5. **Save/load.** Gadget state is on `AppState`, not in any Simulation field — so
   it is NOT serialized. On load, the orchestrator's next tick re-triggers based on
   the loaded sim's production state. Matches retail (per §11.7 of the tab-flash
   doc).

6. **Determinism.** Gadget state is a render/UI concern. It must NOT contribute to
   `world_hash` and must NOT be carried in any sim-snapshot. The poll is read-only
   on the sim; the orchestrator writes only to `AppState.sidebar_gadget_state`,
   which is outside the sim. No lockstep impact.

## Chosen Approach (Approach A — gamemd-mirror primitive)

Per the brainstorm's Approaches section. Three pieces:

1. **`GadgetFlash`** — a 4-field struct (state, period, countdown, disabled)
   mirroring SBGadgetClass +0x34/+0x38/+0x3c/+0x1e — with `start`/`stop`/`tick`.
2. **`frame_select(disabled, mode_active, pressed_or_flash) -> u8`** — a pure
   function implementing the 5-frame state table from
   `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md §5`. Shared by Repair, Sell, and
   the 4 tabs.
3. **`SidebarGadgetState`** — aggregates the 4 tab flashes + 2 mode flags
   (`repair_mode_on`, `sell_mode_on`) on `AppState`. Ticked from `app_sim_tick`,
   polled against the local owner's production state to compute trigger
   start/stop, read by the sidebar view builder.

Rejected Approach B (derive-frame-from-sim.tick) on parity grounds — see brainstorm.

## Tiny-Detail Ledger

Reproduced from the brainstorm; every line must have a concrete home in the
design below. Source citations follow each line.

**Gadget state model:**
- 3 fields per pressable gadget: `state_byte` (`+0x34`, u8, 0|1), `period` (`+0x38`,
  i32, also "is-flashing" sentinel ≠ 0), `countdown` (`+0x3c`, i32).
  [doc: SIDEBAR_TAB_FLASH_SCHEDULER §2]
- `disabled_byte` (`+0x1e`) is a separate gate; auto-stops the flash.
  [doc: §2]
- `pressable` (`+0x40`) is fixed at init for tabs / Repair / Sell.
  [doc: SIDEBAR_REPAIR_SELL_BUTTON §2, SIDEBAR_TAB_FLASH_SCHEDULER §7]
- `mode_active` (`+0x2D`) is a persistent toggle, NOT a transient hover bit. Tabs:
  active-tab bit. Repair/Sell: mode-on bit.
  [doc: SIDEBAR_REPAIR_SELL_BUTTON §5]

**Frame-select function (5-frame table):**
- Disabled → frame 2.
- Not pressable → 0 or 1 depending on hover-static (not relevant here).
- Pressable + `state_byte != 0` (pressed-look / flashing-on) →
  `mode_active ? 4 : 3`.
- Pressable + `state_byte == 0` → `mode_active ? 1 : 0`.
  [doc: SIDEBAR_REPAIR_SELL_BUTTON §5]

**Period and phase:**
- Period = exactly 10 ticks.
  [doc: SIDEBAR_TAB_FLASH_SCHEDULER §4, lit `MOV ECX, 0xa` at `006a8e58`]
- After the first toggle, countdown resets to `period`, not `period + extra_delay`.
  [doc: §4.3]
- First toggle is delayed by `period + extra_delay = 10 + (10 - frame % 10)`. Lands
  on the second 10-frame boundary after the call.
  [doc: §4.1]
- Initial state = `((next_boundary_index) & 1) == 0`. Concurrent flashes in the same
  10-frame phase share initial state (sync blink).
  [doc: §4.2]
- The boundary counter is the game-logic tick (`g_CurrentFrameCounter`).
  [doc: §4.1]

**Start guard:**
- If `period != 0`, Start returns no-op without mutating state.
  [doc: §3.1, §11.1]

**Stop semantics:**
- Idempotent (no-op when `period == 0`).
- Resets in order `state → countdown → period`.
  [doc: §3.2]

**Tick (Flash_AI) semantics:**
- If `disabled` → zero all 3 fields and signal-changed.
- Else if `countdown == 0` → no-op (return 0).
- Else decrement; if it hits zero, XOR-toggle `state` (1 → 0 → 1) and reset
  `countdown = period`.
  [doc: §3.3]

**Trigger conditions (StripClass::AI poll):**
- Tabs 2/3 (Building/Infantry) never trigger.
- Tab 0 (Vehicle in Rust): any in-progress aircraft with `IsComplete == true`.
- Tab 1 (Defense in Rust): any SW with `Available != 0 AND FUN_006ce1a0() != 0`
  (i.e., charged-and-ready).
- Building / infantry / non-aircraft vehicle completions don't trigger.
- No trigger found → call `Stop_Flash` on this tab gadget.
- Poll runs every game tick.
  [doc: §5, §5.1, §5.2, §5.3]

**Per-tick driver:**
- `Flash_AI` is called once per game tick from `SidebarClass::Action` over all 4 tab
  gadgets. Any non-zero return → set `NeedsRedraw`.
  [doc: §6]
- The separate SidebarClass+0x5394/+0x5398 frame-anim system is dormant in YR
  (null output SHP). Ignore.
  [doc: §10]

**Repair/Sell click → mode toggle:**
- Click 0x8065 (Repair) → toggles SidebarClass `+0x46c` AND mirrors to gadget
  `+0x2D` (`DAT_00b0b3cd`). Plays click voc.
- Click 0x8066 (Sell) → toggles SidebarClass `+0x11B1` AND mirrors to gadget
  `+0x2D` (`DAT_00b07e25`). Plays click voc.
- The mirror is what keeps the button visually pressed (frame 1) while mode is on.
  [doc: SIDEBAR_REPAIR_SELL_BUTTON §5]

**Palette / draw flag:**
- All three gadget kinds use SIDEBAR.PAL (the global `DAT_0087f6cc` ConvertClass).
  [doc: §3]
- Draw flag = 0 in normal state. `0x800` only when `+0x55` is set — never observed,
  skip in v1.
  [doc: §4, §10]
- Do NOT use the chrome's `0x400` (that's for SidebarClass::Draw chrome blits, not
  gadget blits).
  [doc: §4]

**SHP frame requirements (asset-side YELLOW):**
- `tab0N.shp` needs frames 0..4. Currently only 0+1 loaded.
  [doc: §7.1, §10]
- `repair.shp` / `sell.shp` need frames 0..4. Currently only 0 loaded.
  [doc: SIDEBAR_REPAIR_SELL_BUTTON §5.1, §10]
- File-side verification (frame_count in SHP header) is a load-time check, not in
  binary scope.

**Save/load:**
- Flash state not serialized in retail; on load, init clears, poll re-triggers.
  [doc: §11.7]

**Pause:**
- Flash continues during pause (Action ticks from input, not from sim).
  [doc: §11.6]

**Out of scope per Q1 / Q3:**
- Cursor SHP swap when Repair/Sell mode is on.
- Click-target-on-tactical-map for repair/sell commands.
- 0x400 / 0x800 highlight modes.
- The `+0x44 = -480` offset sentinel.

## Design

### Components

```
src/sidebar/
├── mod.rs                    (existing — small edits)
├── gadget_flash.rs           NEW — primitive + state aggregator + frame_select
├── power_bar_anim.rs         (existing — unchanged, kept for cross-reference)
├── layout_spec.rs            (existing — unchanged)
└── sidebar_view.rs           (existing — extended signature, frame_index pop)

src/app_sidebar_gadgets.rs    NEW — orchestrator (poll + tick)

(existing files lightly touched: app.rs, app_sim_tick.rs, app_sidebar_render.rs,
 app_sidebar_build.rs, render/sidebar_chrome.rs, plus the click-dispatch site)
```

### `GadgetFlash` primitive

Mirrors gamemd's three functions `FUN_0069DFC0` / `FUN_0069DFF0` / `FUN_0069E010`
exactly. All four fields map 1:1 to SBGadgetClass byte offsets:

```
struct GadgetFlash {
    /// State byte mirror of SBGadgetClass +0x34.
    /// 0 = idle visual, 1 = pressed-look visual.
    state: u8,

    /// Period mirror of SBGadgetClass +0x38.
    /// Also the "is-flashing" sentinel — non-zero ⇒ flash active.
    period: u32,

    /// Countdown mirror of SBGadgetClass +0x3c.
    /// Decremented each tick; on hit-zero, toggles state and resets to period.
    countdown: u32,

    /// Disabled mirror of SBGadgetClass +0x1e.
    /// When set, the next tick auto-stops the flash and reports a change.
    disabled: bool,
}
```

Methods:

```
// Mirrors FUN_0069DFC0. Returns true on actual start, false if already flashing.
// extra_delay is added to the FIRST countdown only.
fn start(&mut self, period: u32, extra_delay: u32, initial_state: u8) -> bool;

// Mirrors FUN_0069DFF0. Returns true if state changed.
fn stop(&mut self) -> bool;

// Mirrors FUN_0069E010. Returns true if the visible state changed
// (caller marks NeedsRedraw / triggers re-render).
fn tick(&mut self) -> bool;

// Convenience read used by the view builder.
fn is_active(&self) -> bool { self.period != 0 }
```

`tick` semantics (verbatim from §3.3):

```
fn tick(&mut self) -> bool {
    if self.disabled {
        if self.period != 0 {
            self.state = 0;
            self.countdown = 0;
            self.period = 0;
            return true;
        }
        return false;
    }
    if self.countdown == 0 { return false; }
    self.countdown -= 1;
    if self.countdown == 0 {
        self.state ^= 1;
        self.countdown = self.period;
        return true;
    }
    false
}
```

`start` semantics (verbatim from §3.1):

```
fn start(&mut self, period: u32, extra_delay: u32, initial_state: u8) -> bool {
    if self.period != 0 { return false; }
    self.period = period;
    self.countdown = period + extra_delay;
    self.state = initial_state;
    true
}
```

`stop` semantics (verbatim from §3.2 — note the field-write order matches the binary):

```
fn stop(&mut self) -> bool {
    if self.period == 0 { return false; }
    self.state = 0;
    self.countdown = 0;
    self.period = 0;
    true
}
```

### `frame_select` — the shared 5-frame state table

Pure free function, applicable to any 5-frame gadget (tabs, Repair, Sell, and
future cameo flash if it adopts the same pattern):

```
// Mirrors the SBGadgetClass::Draw conditional at 0x0069DEB0.
// `state` here is the +0x34 mirror (mouse-down OR flash toggle).
// `mode_active` here is the +0x2D mirror (mode-on / active-tab bit).
// All gadgets passed here are assumed pressable (+0x40 == 1) — they are.
fn frame_select(disabled: bool, mode_active: bool, state: u8) -> u8 {
    if disabled { return 2; }
    if state != 0 {
        if mode_active { 4 } else { 3 }
    } else {
        if mode_active { 1 } else { 0 }
    }
}
```

Five outputs map to the five SHP frame indices: 0 = idle, 1 = mode-active /
tab-active, 2 = disabled, 3 = pressed-idle, 4 = pressed-active.

### `SidebarGadgetState` — the per-AppState aggregator

```
pub struct SidebarGadgetState {
    /// Persistent flash state for the 4 tab gadgets, indexed by SidebarTab::tab_index().
    /// [0] = Building (never flashes — gated by orchestrator)
    /// [1] = Defense (flashes on SW ready)
    /// [2] = Infantry (never flashes)
    /// [3] = Vehicle (flashes on aircraft complete)
    tab_flashes: [GadgetFlash; 4],

    /// Per-tab disabled bit (mirrors +0x1e). v1: always false — tabs are never
    /// disabled by anything sim-side yet. Kept as a field so the gadget tick
    /// path is identical to the gamemd primitive (no special-casing).
    tab_disabled: [bool; 4],

    /// Repair-mode flag. Mirrors SidebarClass +0x46c. Toggled by click on the
    /// Repair button. Mutually exclusive with sell_mode_on and TargetingMode.
    pub repair_mode_on: bool,

    /// Sell-mode flag. Mirrors SidebarClass +0x11B1. Toggled by click on the
    /// Sell button. Mutually exclusive with repair_mode_on and TargetingMode.
    pub sell_mode_on: bool,

    /// Per-button disabled bits (v1: always false — neither button is
    /// disable-gated yet, though they would be when a no-buildings or
    /// no-credits state is implemented).
    pub repair_disabled: bool,
    pub sell_disabled: bool,

    /// The last sim.tick the orchestrator processed. Used to advance gadget
    /// ticks the correct number of times per render frame (catch-up safe).
    last_sim_tick: u64,
}

impl SidebarGadgetState {
    pub fn new() -> Self { /* all-zero / off */ }

    /// Read-only frame index for a tab. Combines mode_active (= is-active-tab) + flash state.
    pub fn tab_frame(&self, tab_index: usize, is_active_tab: bool) -> u8 {
        frame_select(self.tab_disabled[tab_index], is_active_tab,
                     self.tab_flashes[tab_index].state)
    }

    /// Read-only frame index for Repair (mode_active = repair_mode_on; no flash).
    pub fn repair_frame(&self) -> u8 {
        frame_select(self.repair_disabled, self.repair_mode_on, /* state */ 0)
    }

    pub fn sell_frame(&self) -> u8 {
        frame_select(self.sell_disabled, self.sell_mode_on, /* state */ 0)
    }
}
```

Note: Repair / Sell do not flash in retail (their `+0x34` is only ever written by
mouse-down, not Flash_AI; the gadget flash family targets tabs only per §6 of
the tab-flash doc). So no `GadgetFlash` instance is allocated for them — they
just feed `mode_active` to `frame_select` and `state` is fixed at 0.

### `update_sidebar_gadget_state` — the orchestrator

New module `src/app_sidebar_gadgets.rs`. Mirrors `app_building_anim::update_power_bar_anim`
in shape.

```
pub(crate) fn update_sidebar_gadget_state(state: &mut AppState) {
    let Some(sim) = state.simulation.as_ref() else { return; };
    let owner = preferred_local_owner_name(state)
        .unwrap_or_else(|| "Americans".to_string());

    // --- Step 1: poll for trigger conditions. ---
    let rules = state.rules.as_ref();
    let aircraft_complete = match rules {
        Some(r) => has_aircraft_complete_for_owner(sim, r, &owner),
        None => false,
    };
    let sw_ready = match rules {
        Some(r) => has_charged_sw_for_owner(sim, r, &owner),
        None => false,
    };

    // --- Step 2: drive Start/Stop on the 4 tab flashes. ---
    let gadgets = &mut state.sidebar_gadget_state;
    let frame = sim.tick;
    let extra_delay = (10 - (frame % 10)) as u32;          // matches 006a8e5d-006a8e69
    let parity_boundary = ((extra_delay as u64) + frame) / 10;
    let initial_state: u8 = if parity_boundary & 1 == 0 { 1 } else { 0 };

    // Building (index 0) — never flashes.
    gadgets.tab_flashes[SidebarTab::Building.tab_index()].stop();
    // Defense (index 1) — flashes on SW ready.
    if sw_ready {
        gadgets.tab_flashes[1].start(10, extra_delay, initial_state);
    } else {
        gadgets.tab_flashes[1].stop();
    }
    // Infantry (index 2) — never flashes.
    gadgets.tab_flashes[2].stop();
    // Vehicle (index 3) — flashes on aircraft complete.
    if aircraft_complete {
        gadgets.tab_flashes[3].start(10, extra_delay, initial_state);
    } else {
        gadgets.tab_flashes[3].stop();
    }

    // --- Step 3: advance per-sim-tick ticks (catch-up safe). ---
    // tick() runs once per game-logic tick, NOT per render frame, to match the
    // exact 10-game-tick period from the binary.
    let tick_delta = sim.tick.saturating_sub(gadgets.last_sim_tick);
    for _ in 0..tick_delta {
        for f in &mut gadgets.tab_flashes {
            f.tick();
        }
    }
    gadgets.last_sim_tick = sim.tick;
}

// Polling predicates — read-only on Simulation.
fn has_aircraft_complete_for_owner(sim, rules, owner) -> bool {
    // True if the owner has any queue item with queue_category == Aircraft
    // AND state == Done (i.e. finished but unplaced/un-spawned). The exact
    // BuildQueueState semantics for "aircraft waiting for helipad" are
    // resolved in /write-plan against the current production code; the
    // brainstorm-time predicate is "an aircraft has finished production but
    // has not yet been placed/launched."
    production::queue_view_for_owner(sim, rules, owner).iter().any(|q|
        q.queue_category == ProductionCategory::Aircraft
        && q.state == BuildQueueState::Done
    )
}

fn has_charged_sw_for_owner(sim, rules, owner) -> bool {
    superweapon::superweapon_views_for_owner(sim, rules, owner_iid)
        .iter().any(|sw| sw.is_ready)
}
```

The orchestrator is called from `app_sim_tick.rs:208`, right after
`update_power_bar_anim(state)`. Same lifecycle, same cadence rules. The `tick()`
loop iterates per sim-tick delta so the flash period is measured against
sim ticks, exactly like gamemd measures it against `g_CurrentFrameCounter`.

### View-builder integration

`src/sidebar/sidebar_view.rs:53` `build_sidebar_view_with_spec` signature gains:

```
gadget_state: &SidebarGadgetState,
```

`SidebarTabButton` (now in `src/sidebar/mod.rs:178-183`):

```
pub struct SidebarTabButton {
    pub tab: SidebarTab,
    pub rect: Rect,
    pub active: bool,        // kept for hit-test logic
    pub frame_index: u8,     // NEW — pre-computed via gadget_state.tab_frame()
}
```

`SidebarView` (in `src/sidebar/mod.rs:207`) gains:

```
pub repair_button: SidebarTabButton-like { rect, frame_index, disabled, action },
pub sell_button:   SidebarTabButton-like { rect, frame_index, disabled, action },
```

(Or a new `SidebarToggleButton` struct that fits both Repair and Sell. The two share
all behaviour.)

Their rects come from `sidebar_layout_spec` (using the same theater-adjusted
position formulas in `SIDEBAR_REPAIR_SELL_BUTTON §6` — `g_SidebarWidth + 8`/`+ 7`
for Y; SidebarX + 20-or-33 for X. The Rust port may continue to use its existing
RON layout values, with a note that exact theater-driven positioning is A23 — a
separate LOW-severity follow-up).

### Render

[src/render/sidebar_chrome.rs:285-293](../../src/render/sidebar_chrome.rs#L285-L293)
extends the per-theme atlas loader:

```
// Before:
//   tab_entries: Vec<RenderedChromeEntry>      (4 entries, frame 0 each)
//   tab_active_entries: Vec<RenderedChromeEntry> (4 entries, frame 1 each)
//   repair: Option<RenderedChromeEntry>         (1 entry, frame 0)
//   sell: Option<RenderedChromeEntry>           (1 entry, frame 0)
//
// After:
//   tab_frames: [[Option<RenderedChromeEntry>; 5]; 4]   (4 tabs × 5 frames)
//   repair_frames: [Option<RenderedChromeEntry>; 5]
//   sell_frames: [Option<RenderedChromeEntry>; 5]
```

Loader emits a warning when a frame is missing and falls back to frame 0 for that
state. This is the YELLOW-flag mitigation; in retail we expect all 5 frames to be
present (gamemd hardcodes the indices, so retail must contain them).

[src/app_sidebar_build.rs:170-193](../../src/app_sidebar_build.rs#L170-L193): the
commented-out block becomes:

```
if let Some(sell) = atlas.sell_frames[view.sell_button.frame_index as usize].as_ref() {
    push_chrome(&mut inst, sell, /* pos */ ..., _btn_depth, camera_offset, s);
}
if let Some(repair) = atlas.repair_frames[view.repair_button.frame_index as usize].as_ref() {
    push_chrome(&mut inst, repair, /* pos */ ..., _btn_depth, camera_offset, s);
}
```

And the existing tab render code that currently picks between `tab` / `tab_active`
becomes:

```
let frame = tab.frame_index as usize;
if let Some(entry) = atlas.tab_frames[tab_idx][frame].as_ref() {
    push_chrome(&mut inst, entry, ...);
}
```

### Click handlers

The sidebar already routes button clicks through `SidebarAction`. Two new variants:

```
SidebarAction::ToggleRepairMode,
SidebarAction::ToggleSellMode,
```

Handler (in `app_input.rs` or wherever sidebar actions land; one location, mirrors
the `SidebarAction::SelectTab` etc. dispatch):

```
SidebarAction::ToggleRepairMode => {
    let g = &mut state.sidebar_gadget_state;
    g.repair_mode_on = !g.repair_mode_on;
    if g.repair_mode_on {
        g.sell_mode_on = false;
        state.targeting_mode = None;
        state.building_placement_preview = None;
    }
    // (Click voc playback deferred to audio brainstorm.)
}
SidebarAction::ToggleSellMode => {
    let g = &mut state.sidebar_gadget_state;
    g.sell_mode_on = !g.sell_mode_on;
    if g.sell_mode_on {
        g.repair_mode_on = false;
        state.targeting_mode = None;
        state.building_placement_preview = None;
    }
}
```

Mutual exclusion is bidirectional: when `targeting_mode` becomes Some(_) via the
existing path (`app_commands.rs` building-placement arm, SW arm), the same helper
must clear `repair_mode_on` / `sell_mode_on`. This is one extra line at each
arming site — call out in /write-plan.

### Data flow per frame

```
sim.tick advances (sim ticks)
   │
   ▼
app_sim_tick.rs:208  →  update_sidebar_gadget_state(state)
   │                       │
   │                       ├─ poll queue_view + sw_views
   │                       ├─ Start/Stop tab_flashes[0..4]
   │                       └─ tick() each flash (sim-tick delta times)
   │
   ▼
current_sidebar_view(state)  →  build_sidebar_view_with_spec(..., &state.sidebar_gadget_state)
   │                                     │
   │                                     ├─ tabs[i].frame_index = gadget_state.tab_frame(i, active)
   │                                     ├─ repair_button.frame_index = gadget_state.repair_frame()
   │                                     └─ sell_button.frame_index = gadget_state.sell_frame()
   │
   ▼
app_sidebar_build.rs  →  reads view.tabs[i].frame_index, atlas.tab_frames[i][frame]
                         reads view.repair_button.frame_index, atlas.repair_frames[frame]
                         reads view.sell_button.frame_index, atlas.sell_frames[frame]
```

### Error handling

- **Missing SHP frames at load time:** `render_entry(..., frame_N)` returns `Option`.
  When `None`, the atlas keeps `[..][N] = None`; the build path's `if let Some()`
  pattern simply skips that frame. Player sees nothing for that state (preferable
  to garbage). One-time `tracing::warn!` per missing frame so we know to investigate.
- **Sim missing (during loading / between scenarios):** `update_sidebar_gadget_state`
  early-returns on `state.simulation.is_none()`. No flash, no state change.
- **No local owner name:** orchestrator early-returns. Tab flashes stay in their
  previous state (which is "stopped" at startup).
- **Sim tick going backwards (load-game):** `saturating_sub` makes `tick_delta = 0`.
  The orchestrator re-evaluates Start/Stop based on the loaded sim's state. Matches
  retail (per §11.7).

### Testing strategy

Unit tests in `gadget_flash.rs`:

1. `start` from idle sets all 3 fields per §3.1; `state = initial_state`, `period`
   = arg1, `countdown` = arg1 + arg2.
2. `start` while already flashing returns false and mutates nothing.
3. `stop` from active resets all 3 in the correct order; returns true.
4. `stop` from idle is a no-op; returns false.
5. `tick` while disabled: from active → all 3 zeroed, returns true; from idle →
   no-op, returns false.
6. `tick` from idle (`countdown == 0`) → no-op, returns false.
7. `tick` decrements countdown by 1; on hit-zero, toggles state and resets
   countdown to `period`. State sequence over 30 ticks with period=10 and
   extra_delay=5 follows: countdown[15→1, toggle, countdown=10, 10→1, toggle,
   countdown=10, 10→1, toggle].
8. `frame_select` table: enumerate all 6 input combinations (2² × 2 with
   disabled override) and assert the matrix 0/1/2/3/4.
9. Two flashes with the same period and matched extra_delay computed from the
   same frame stay phase-aligned through multiple toggles.

Unit tests in `app_sidebar_gadgets.rs`:

10. Orchestrator with no sim → no-op (no panic).
11. Orchestrator with aircraft Done → Vehicle tab flash starts; calling again with
    same condition does NOT re-init (Start guard); after `sim.tick` advances 20,
    state byte has toggled twice.
12. Orchestrator with SW becoming ready → Defense tab flash starts; clearing
    is_ready → next call stops the flash, state and period zeroed.
13. Building / Infantry tabs always have `tab_flashes[i].is_active() == false`
    even when matching conditions exist (the trigger gating per §5.2).
14. Catch-up: a single render frame where `sim.tick` jumped 30 — confirm `tick()`
    is called 30 times and state ends at the right value.

Integration / golden tests:

15. With `extra_delay = 0` and period = 10, the first toggle lands exactly 10
    ticks after start. With `extra_delay = 7`, first toggle is at tick 17 from
    start; subsequent toggles every 10 ticks.

### Determinism considerations

`SidebarGadgetState` is on `AppState`, not in any field that contributes to:
- `world_hash` (computed in `src/sim/world/world_hash.rs`).
- Sim snapshots / save serialisation.
- The lockstep command stream.

Gadget state mutation reads `sim.tick`, `production::queue_view_for_owner`, and
`superweapon::superweapon_views_for_owner` — all read-only on Simulation. No
write-back into sim. The Vec<QueueItemView> result is iterated and dropped before
the gadget mutation happens. No determinism impact, no replay/lockstep impact.

## Architectural Decisions

**Patterns followed:**
- **PowerBarAnimState mirror.** `SidebarGadgetState` follows the same shape as
  `PowerBarAnimState`: persistent struct on `AppState`, tick driven from
  `app_sim_tick`, read by view builder. One conventional way to do UI animation
  in this codebase, applied to a new UI animation.
- **Polling production state** (rather than event bus). Per Q3 + brainstorm:
  matches `StripClass::AI` 1:1 — including the auto-stop on condition clear.
- **Pure 5-frame state table as a free function** (`frame_select`). Mirrors the
  `SBGadgetClass::Draw` conditional verbatim. Reusable by future gadgets.
- **Sim-tick cadence for gadget AI** (not render-frame cadence). The flash period
  is 10 *game-logic* ticks in retail; we tick the same way.

**Patterns deviated from:**
- **No sim event hooks added.** Other UI features (audio, fire effects) use an
  event queue. The flash trigger is a poll. Justified by: (a) the gamemd primitive
  is a poll, (b) the trigger conditions are already cheap to derive from existing
  views, (c) auto-stop-on-condition-clear is implicit and free in the poll model
  but would require N "trigger cleared" event types in the event model.

**Tech debt:**
- The `tab_disabled` / `repair_disabled` / `sell_disabled` fields are wired but
  not driven by any condition in v1 (no sim signal currently flags "tab disabled"
  or "no buildings to repair"). They stay `false`. When the disabled-state
  infrastructure ships (separate brainstorm), wiring is one line per condition.
- The cursor-mode mutex helper (clear all three of TargetingMode + Repair-mode +
  Sell-mode) is inlined here at three call sites. If a fourth cursor mode ships,
  extract a helper. v1 does not.

## Alternatives Considered

**Approach B (derive frame-from-sim-tick, no persistent gadget state).** Rejected
on parity: would skip the `period + extra_delay` first-cycle math from §4.1, so
the first half-cycle is up to 9 ticks shorter than retail. Per CLAUDE.md "default
to modelling the gamemd primitive, not approximating it" — this is the named
anti-pattern.

**Inline flash fields on `SidebarTabButton`.** Rejected: `SidebarTabButton` is
rebuilt per render frame inside `build_sidebar_view_with_spec`, so any state
written there is lost on the next frame. Would require also refactoring the view
lifecycle to be persistent — much larger blast radius than just putting state on
AppState (which is already the pattern).

**Event-bus trigger (sim emits "AircraftCompleted(Tab)" / "SuperWeaponReady" /
"AircraftPlaced" / "SuperWeaponFired").** Rejected per Q3: matches retail less
faithfully than the poll, requires more new sim hooks, and the symmetric
"condition cleared" events are needed for auto-stop. The poll gets auto-stop for
free.

**Extending `TargetingMode` enum with `Repair` / `Sell` variants instead of two
separate flags.** Considered. The enum currently carries a `String` payload (the
target type name / SW section); Repair / Sell have no payload, which would
require Option<String> or unit variants. More importantly, the cursor-resolution
work (deferred) might need to distinguish "armed with a tactical-map target type"
(BuildingPlacement, SuperWeapon) from "armed without a target" (Repair, Sell) —
the call sites that consume `TargetingMode` today all expect a payload. So we
keep the two as separate `bool` flags on `SidebarGadgetState` and revisit unification
when the cursor brainstorm lands.

## Follow-ups (explicitly deferred)

- **Cursor SHP swap when Repair-mode / Sell-mode is on** (per Q1 answer).
- **Click-target-on-tactical-map resolution** — clicking a friendly building in
  repair-mode issues a repair command; in sell-mode issues a sell command.
- **Theater-driven Repair/Sell positions** (A23 in the disparity scan — LOW).
- **Disable-state wiring** for tabs and Repair/Sell when sim conditions warrant
  (no buildings → Repair disabled; nothing to sell → Sell disabled). Currently
  all `disabled` flags hardcoded to `false`.
- **0x800 highlight mode** — per the docs, never observed being written in
  normal play. Add only if a future RE pass identifies the writer.
- **Cameo flash (G5 / A20-companion)** — once shipped, the same `GadgetFlash`
  primitive can host the per-CameoEntry FlashEndFrame countdown. This design
  leaves room for that without changes.
- **Click voc playback** — gamemd plays a click sound on both Repair and Sell;
  ours doesn't yet (deferred to audio brainstorm with EVA events).
