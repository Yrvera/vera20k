# Sidebar Tab-Flash + Repair/Sell Rendering — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Bundle G1 (Repair/Sell render) and the in-scope portion of A20 (tab pressed/active-pressed frames + Defense-tab flash on SW ready) by adding a shared gamemd-mirror gadget-flash primitive, the 5-frame state-select function, the persistent Repair/Sell mode flags, and a poll-driven orchestrator that flashes the Defense tab when any superweapon is ready.

**Architecture:** Persistent UI state goes on `AppState.sidebar_gadget_state` (mirrors the existing `power_bar_anim` pattern). A sim-tick-cadence orchestrator in `src/app_sidebar_gadgets.rs` polls SuperWeapon state and ticks each gadget's flash. The per-render-frame `SidebarView` builder reads the persistent state and populates frame indices that `app_sidebar_build.rs` renders via the per-theme chrome atlas (extended from 2 → 5 frames per tab and from 1 → 5 frames per repair/sell button).

**Scope note (post-review-plan, 2026-05-20):** The Vehicle-tab flash on aircraft completion — originally part of this bundle per gamemd's StripClass::AI Tab 0 trigger — is **deferred**. The current Rust production code at [src/sim/production/production_queue.rs:464-505](../../src/sim/production/production_queue.rs#L464-L505) auto-spawns aircraft on a free helipad OR refunds them; there is no "aircraft waiting for helipad" queue state to poll. Implementing the Vehicle flash faithfully requires either an aircraft ready-queue analogous to `ready_by_owner` for buildings, or a "local owner has any idle docked aircraft entity" sim accessor. Both are larger than this plan's bundle; the Vehicle flash becomes a follow-up once those semantics land. The plan still establishes the full `GadgetFlash` primitive + orchestrator + per-tab state — wiring the Vehicle trigger later is a one-call addition.

**Design Doc:** [docs/plans/2026-05-20-sidebar-tab-flash-and-repair-sell-design.md](2026-05-20-sidebar-tab-flash-and-repair-sell-design.md)

---

## Grounding Summary

- **Docs (R1):** Four GREEN reports (all verified 2026-05-20) carry the full ledger:
  `SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md`, `SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md`,
  `SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`, `SIDEBAR_INIT_GADGET_POSITIONING_GHIDRA_REPORT.md`.
- **Ghidra (R2):** No further live verification needed for this plan — every binary
  claim in the design ledger cites a Ghidra MCP call inline in the source doc.
  If implementation surfaces an ambiguity, re-open the relevant doc first.
- **Repo pattern (R3):** Mirror `PowerBarAnimState` in `src/sidebar/power_bar_anim.rs`
  and its sim-tick driver `update_power_bar_anim` in `src/app_building_anim.rs:486-514`,
  hooked at `src/app_sim_tick.rs:207`. Persistent UI animation state on `AppState`,
  ticked from the same per-render-frame block, consumed by the per-frame view
  builder.
- **INI (R4):** None apply. Period (10), trigger RTTIs, gadget offsets are all
  binary literals; no INI key drives this behavior.
- **Stale check:** Parallel sessions have uncommitted edits to `src/app_sim_tick.rs`
  (refinery exit SFX at line 508+, doesn't conflict with the tick-hook insertion
  point at line ~207) and `src/sim/production/production_types.rs` (adds
  `BuildQueueState::NoFunds`, doesn't affect any predicate this plan uses). Tasks
  use surrounding-context anchors rather than line numbers where the parallel
  edits could shift positions.
- **Known unknowns:** SHP frame-count of retail `tab0N.shp` / `repair.shp` /
  `sell.shp` — design defers to load-time inspection with a warning fallback to
  frame 0.
- **Resolved during review-plan (2026-05-20):** the original design assumed an
  "aircraft waiting for helipad" queue state existed; verification of
  `production_queue.rs:456-518` showed it does not (auto-spawn or refund, no
  waiting). Vehicle-tab flash is consequently deferred.

## Key Technical Decisions

- **`SidebarGadgetState` lives on `AppState`, not on `Simulation`.** UI animation
  is a render concern; gadget state must not contribute to `world_hash` or save
  serialization. **Confidence:** high — **Source:** repo pattern
  `AppState.power_bar_anim` ([src/app.rs:171](../../src/app.rs#L171)).
- **Sim-tick cadence for `flash.tick()`, not render-frame cadence.** The flash
  period is 10 *game-logic* ticks in retail. The orchestrator caches `last_sim_tick`
  and iterates `tick()` once per sim-tick delta. **Confidence:** high — **Source:**
  SIDEBAR_TAB_FLASH_SCHEDULER §4 (period literal `MOV ECX, 0xa` at `006a8e58`).
- **Poll production state, no new sim events.** Mirrors `StripClass::AI`'s per-tick
  poll-and-Stop-on-condition-clear. **Confidence:** high — **Source:**
  SIDEBAR_TAB_FLASH_SCHEDULER §5, §5.3.
- **Vehicle-tab flash deferred** (originally medium-confidence aircraft-complete
  predicate). Rust production auto-spawns or refunds aircraft on completion with
  no observable "waiting" state — see Scope note at top. **Confidence:** high
  (deferral decision) — **Source:** verified in
  [src/sim/production/production_queue.rs:456-518](../../src/sim/production/production_queue.rs#L456-L518)
  during /review-plan.
- **5-frame atlas storage: `[Option<E>; 5]` for repair/sell, `[[Option<E>; 5]; 4]`
  for tabs.** Matches the existing `powerp_frames: [Option<…>; 5]` pattern at
  [src/render/sidebar_chrome.rs:91](../../src/render/sidebar_chrome.rs#L91).
  **Confidence:** high — **Source:** repo pattern.
- **Repair/Sell don't allocate a `GadgetFlash`** — they only consume `frame_select`
  with `state = 0` (no flash in retail) and `mode_active = repair_mode_on / sell_mode_on`.
  **Confidence:** high — **Source:** SIDEBAR_TAB_FLASH_SCHEDULER §6 (flash family
  only targets tab gadgets in the per-tick loop over `&DAT_00b07c48..0xb07dc8`).
- **`SidebarToggleButton` new struct for Repair/Sell view entries** (vs reusing
  `SidebarControlButton`). The control-button struct carries a `String` label
  appropriate for text buttons; SHP-driven gadgets need a frame index instead.
  **Confidence:** high — **Source:** design doc §View-builder integration.
- **Disabled bits wired but unused in v1 (`tab_disabled`, `repair_disabled`,
  `sell_disabled` hardcoded `false`).** No sim signal currently surfaces "no
  buildings to repair" / "no money to sell". When that signal lands, wiring is
  one line each. **Confidence:** high — **Source:** design doc §Tech debt.

## Open Questions

### Resolved During Planning

- **Frame-select cadence (render vs sim tick):** sim tick. The orchestrator runs
  per render frame but iterates `tick()` `sim.tick - last_sim_tick` times.
- **Where Repair/Sell mode flags live:** on `SidebarGadgetState`, not on
  `TargetingMode`. The design's "cursor mode mutex" semantics require these to be
  separate from the targeting enum because the cursor-resolution work is deferred.
- **Whether Repair/Sell need a `GadgetFlash` allocation:** no. Per
  SIDEBAR_TAB_FLASH_SCHEDULER §6, only the 4 tab gadgets are walked by the
  per-tick Flash_AI loop.

### Deferred to Implementation

- **Whether retail `tab0N.shp` actually contains 5 frames.** Loader emits a
  `tracing::warn!` and falls back to frame 0 if a frame is missing; visible
  result is that the disabled/pressed states render as the idle frame.
  Investigate only if retail observation shows the wrong frame in a pressed
  state.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sidebar/gadget_flash.rs` | `GadgetFlash` primitive + `frame_select` + `SidebarGadgetState` aggregator |
| Create | `src/app_sidebar_gadgets.rs` | Orchestrator: poll production, drive Start/Stop, tick all gadgets |
| Modify | `src/sidebar/mod.rs` | Re-export gadget_flash; add `SidebarToggleButton`; add `frame_index` field on `SidebarTabButton`; add Repair/Sell view fields; route Repair/Sell rects in `hit_test`; new `SidebarAction` variants |
| Modify | `src/sidebar/sidebar_view.rs` | Take `&SidebarGadgetState`; populate tab `frame_index`; build Repair/Sell `SidebarToggleButton`s |
| Modify | `src/render/sidebar_chrome.rs` | Load tab0N frames 0..4, repair.shp 0..4, sell.shp 0..4 into the per-theme atlas |
| Modify | `src/app_sidebar_build.rs` | Render Repair/Sell from atlas frames; render tabs by `frame_index` not `active` |
| Modify | `src/app_sidebar_render.rs` | Pass `&state.sidebar_gadget_state` into the view builder |
| Modify | `src/app.rs` | Add `sidebar_gadget_state: SidebarGadgetState` field + `Default` init |
| Modify | `src/app_sim_tick.rs` | Call `app_sidebar_gadgets::update_sidebar_gadget_state` after `update_power_bar_anim` |
| Modify | `src/app_input.rs` | Handle `ToggleRepairMode` / `ToggleSellMode`; clear repair/sell in existing targeting-mode-arm handlers |
| Modify | `src/lib.rs` | Register the new `app_sidebar_gadgets` module |

## Interface Changes

Public-ish (`pub(crate)`) APIs created or modified:

- `crate::sidebar::gadget_flash::GadgetFlash` — new public type with `start` /
  `stop` / `tick` / `is_active`.
- `crate::sidebar::gadget_flash::frame_select(disabled, mode_active, state) -> u8`
  — new pure free function.
- `crate::sidebar::gadget_flash::SidebarGadgetState` — new aggregator type with
  `new`, `tab_frame`, `repair_frame`, `sell_frame`, mutator methods.
- `SidebarTabButton` gains `frame_index: u8` field (no removals; `active: bool`
  stays for hit-test consumers).
- `SidebarView` gains `repair_button: SidebarToggleButton` and
  `sell_button: SidebarToggleButton`.
- `SidebarAction` gains `ToggleRepairMode` and `ToggleSellMode` variants.
- `build_sidebar_view_with_spec` signature gains THREE trailing parameters:
  `gadget_state: &SidebarGadgetState`, `repair_button_size: Option<[f32; 2]>`,
  `sell_button_size: Option<[f32; 2]>`. **Consumers:**
  [src/app_sidebar_render.rs:102](../../src/app_sidebar_render.rs#L102) and
  [src/app_sidebar_render.rs:122](../../src/app_sidebar_render.rs#L122)
  and [src/sidebar/sidebar_view.rs:32](../../src/sidebar/sidebar_view.rs#L32)
  (the no-spec convenience). All three are updated in Task 6.
- `SidebarChromeAtlas` field changes: `tab_buttons: Vec<…>` →
  `tab_frames: [[Option<…>; 5]; 4]`; `tab_buttons_active: Vec<…>` removed;
  `repair: Option<…>` → `repair_frames: [Option<…>; 5]`; `sell: Option<…>` →
  `sell_frames: [Option<…>; 5]`. **Consumers:**
  [src/app_sidebar_build.rs:134-152](../../src/app_sidebar_build.rs#L134-L152)
  (tab render) and
  [src/app_sidebar_build.rs:170-194](../../src/app_sidebar_build.rs#L170-L194)
  (commented repair/sell render). Also one consumer at
  [src/app_sidebar_render.rs:80](../../src/app_sidebar_render.rs#L80) reads
  `atlas.tab_buttons.first()` — switch to `atlas.tab_frames[0][0].as_ref()`.

## Sim Checklist

This plan **does not touch `sim/`**. All new state and code lives in the
sidebar/render/app layers, which sit above sim. The orchestrator reads sim
state via existing public accessors (`production::queue_view_for_owner`,
`production::ready_buildings_for_owner`,
`superweapon::superweapon_views_for_owner`) but writes only to
`AppState.sidebar_gadget_state`.

- [x] No `sim/` files modified.
- [x] No new state in `world_hash` (gadget state is on `AppState`, not `Simulation`).
- [x] No new dependency from `sim/` to anything above it.
- [x] No `f32`/`f64` introduced into game logic (the only float arithmetic is in
  `frame_select` callers consuming the resulting `u8` as an index — no math).
- [x] BTreeMap iteration: orchestrator iterates over the local owner's
  `QueueItemView` (an already-sorted Vec — see
  [src/sim/production/production_queue.rs:613](../../src/sim/production/production_queue.rs#L613)),
  so no determinism concern.

## Risk Areas

1. **Tick-rate parity.** Verified by Task 7 unit tests + Task 16 in-game observation.
2. **Atlas field rename has multiple consumers.** Task 11 collects all consumers
   before refactoring; Task 12 applies edits atomically (single commit) so an
   intermediate state with mismatched names never lands.
3. **TargetingMode mutual exclusion.** Task 13 adds the clear-the-other lines at
   each of the existing four `SidebarAction::Arm*` handlers + two new handlers.
   Regression test in Task 13 confirms all six paths produce the expected mutex.
4. **SHP frame missing in retail.** Task 11's loader emits a warning and falls
   back to frame 0 for that index; visible result is the gadget renders its idle
   frame for the missing state.
5. **Cursor mode placebo.** Per design Q1, clicking Repair/Sell sets the mode flag
   but does not change cursor or click-target resolution. The button visibly
   "stays pressed" but clicks do nothing useful until the follow-up brainstorm.
   This is a known intermediate state, called out in the design's "What this
   does NOT fix" section.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `GadgetFlash::start` countdown init = `period + extra_delay`, NOT just `period` | First toggle delayed to second-next 10-frame boundary; if wrong, first half-cycle is up to 9 ticks shorter than retail | Unit test verifying countdown after `start(10, 7, _)` = 17 |
| Task 1 | `GadgetFlash::tick` resets countdown to `+0x38` (period), NOT to `period + extra_delay` | Subsequent toggles are every 10 ticks fixed; if wrong, the steady cadence drifts | Unit test simulating 30 ticks with `start(10, 5, 0)`, asserting toggle ticks |
| Task 1 | `GadgetFlash::start` guard: if `period != 0` → no-op | Repeated Start calls during an active flash must not restart phase. Without this, the flash visibly re-syncs on every poll tick | Unit test: start, advance partial period, start again → no field changes |
| Task 1 | `Stop_Flash` field-write order: state → countdown → period | Per §3.2; the sentinel `period` is cleared last so any mid-reset reader sees consistent intermediate state | Unit test asserting all 3 fields reach 0 |
| Task 2 | `frame_select` table: disabled → 2; pressed (`state != 0`) → mode_active ? 4 : 3; idle → mode_active ? 1 : 0 | The 5 visible button states. Wrong mapping = wrong frame at every state transition | Table-driven unit test over all 8 input combinations |
| Task 7 | `extra_delay = 10 - (sim.tick % 10)` and `initial_state = ((extra_delay + frame) / 10) & 1 == 0` | Concurrent flashes phase-align in sync; first toggle lands on the second 10-tick boundary | Unit test starting two flashes one tick apart, advancing to the boundary, asserting both toggled together |
| Task 7 | Building / Infantry / Vehicle tabs always `stop()` regardless of trigger conditions | Mirrors §5.2 gate `+0x38 == 0 \|\| == 1`. Building/infantry never flash; Vehicle is deferred (no aircraft-waiting sim state) | Unit test seeding a completed building in queue, asserting `tab_flashes[0].is_active() == false` |
| Task 7 | SW-ready fires Defense tab flash | Direct mapping from gamemd Tab 1 → Rust Defense | Unit test with mock SW-ready predicate |
| Task 7 | `Stop_Flash` called when trigger condition clears (poll iteration finds nothing) | Player fires the SW → next tick poll finds no ready SW → flash stops. Without this, flash would persist forever | Unit test: start flash, clear condition, tick, assert `is_active() == false` |
| Task 11 | Load tab0N frames 0..4, repair.shp 0..4, sell.shp 0..4 | All five gadget states need their SHP frame. Missing a frame = missing visual state | Atlas-load smoke test that asserts at least frame 0 loaded; manual verification frames 2-4 present in retail |
| Task 12 | Tab render picks atlas frame by `tab.frame_index` | Frame index is the entire visual differentiator now (idle vs active vs pressed vs flashing-on). Wrong indexing = wrong sprite at every tab state | In-game observation: idle tab matches gamemd frame 0, active tab matches frame 1, flashing tab alternates 0↔3 (when inactive) or 1↔4 (when active) |
| Task 12 | Repair/Sell render uses sidebar.pal (already wired) NOT OBSERVER.PAL | Per SIDEBAR_REPAIR_SELL_BUTTON §3, sidebar.pal via `DAT_0087f6cc` is correct. Delete the wrong TODO comment | Visual: button colors match gamemd, no shifted/inverted palette artifacts |
| Task 13 | Mutual exclusion: arming any one of {BuildingPlacement, SuperWeapon, Repair-mode, Sell-mode} clears the other three | Without this, the cursor state would be ambiguous and the on-screen pressed-button visual would lie about what's armed | Manual test: arm building, click Repair → building unarmed AND button now pressed; click Sell → Repair unpressed AND Sell pressed |
| Task 14 | Sim-tick cadence (orchestrator iterates `tick()` per sim-tick delta) | Period must be exactly 10 game-logic ticks. If it ticked per render frame, period would drift with framerate | In-game observation: increase sim_speed, flash period stays at ~667ms at game-speed 1, scales with game speed at other speeds (matches retail) |
| Task 16 | End-to-end in-game observation | Final parity check against retail behavior | Run skirmish, queue aircraft → Vehicle tab pulses; place aircraft → pulse stops. Charge SW → Defense tab pulses; fire SW → pulse stops. Click Repair → button stays pressed. Click Sell → button stays pressed. Click Repair again → button unpressed. |

---

## Tasks

### Task 1: `GadgetFlash` primitive

**Why:** The shared gamemd-mirror primitive (SBGadgetClass +0x34/+0x38/+0x3c/+0x1e).
Every other task depends on its semantics. Test it before anything reads it.

**Files:**
- Create: `src/sidebar/gadget_flash.rs` (new file — only `GadgetFlash` in this task; the rest of the file accretes in Task 2 and Task 3)

**Pattern:** Follows `src/sidebar/power_bar_anim.rs` shape: data struct + start/stop/tick methods + `#[cfg(test)] mod tests` in the same file.

**Step 1: Create the file with module header and the struct.**

```rust
//! Sidebar gadget flash primitive.
//!
//! Mirrors the SBGadgetClass flash sub-struct (+0x34 state / +0x38 period /
//! +0x3c countdown / +0x1e disabled) and the three driver functions
//! Start_Flash / Stop_Flash / Flash_AI. One instance per pressable gadget
//! that may flash. See ra2-rust-game-docs/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md
//! for the source semantics.
//!
//! ## Dependency rules
//! - Part of `sidebar/` — pure data + logic, no rendering or sim dependencies.

/// Persistent flash state for one pressable gadget.
///
/// Mirrors three contiguous fields on an SBGadgetClass instance plus the
/// separate +0x1e disabled gate. All four are byte-for-byte semantic
/// counterparts of the binary; the field names use plain-English meanings
/// rather than the binary offsets.
#[derive(Debug, Clone, Copy, Default)]
pub struct GadgetFlash {
    /// "Draw as pressed" bit. 0 = idle visual, 1 = pressed-look visual.
    /// Toggled by `tick` when a flash period elapses.
    pub state: u8,

    /// Toggle interval in ticks AND the "is-flashing" sentinel (non-zero ⇒ active).
    /// Stays constant for the lifetime of an active flash; reset by `stop`.
    pub period: u32,

    /// Ticks remaining until the next toggle. On the first cycle this is
    /// `period + extra_delay`; on every subsequent cycle it resets to `period`.
    pub countdown: u32,

    /// Auto-stop gate. When set, the next `tick` zeros all three flash fields
    /// and reports a state change.
    pub disabled: bool,
}
```

**Step 2: Add `start`, `stop`, `tick`, `is_active`.**

```rust
impl GadgetFlash {
    /// Schedule a flash. No-op (returns `false`) if a flash is already active —
    /// matches the gamemd Start_Flash guard at +0x38 != 0.
    ///
    /// `extra_delay` is added to the FIRST countdown only; the steady-state
    /// toggle interval is `period`.
    pub fn start(&mut self, period: u32, extra_delay: u32, initial_state: u8) -> bool {
        if self.period != 0 {
            return false;
        }
        self.period = period;
        self.countdown = period + extra_delay;
        self.state = initial_state;
        true
    }

    /// Cancel any active flash. No-op (returns `false`) if not currently flashing.
    /// Field-write order matches the binary: state → countdown → period.
    pub fn stop(&mut self) -> bool {
        if self.period == 0 {
            return false;
        }
        self.state = 0;
        self.countdown = 0;
        self.period = 0;
        true
    }

    /// Advance one game-logic tick. Returns `true` when the visible state
    /// changed (caller marks redraw / picks a new frame index).
    pub fn tick(&mut self) -> bool {
        if self.disabled {
            if self.period != 0 {
                self.state = 0;
                self.countdown = 0;
                self.period = 0;
                return true;
            }
            return false;
        }
        if self.countdown == 0 {
            return false;
        }
        self.countdown -= 1;
        if self.countdown == 0 {
            self.state ^= 1;
            self.countdown = self.period;
            return true;
        }
        false
    }

    /// True while a flash is scheduled (period != 0).
    pub fn is_active(&self) -> bool {
        self.period != 0
    }
}
```

**Step 3: Add unit tests in the same file.**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_from_idle_initialises_all_fields() {
        let mut g = GadgetFlash::default();
        assert!(g.start(10, 7, 1));
        assert_eq!(g.period, 10);
        assert_eq!(g.countdown, 17, "first countdown is period + extra_delay");
        assert_eq!(g.state, 1);
        assert!(g.is_active());
    }

    #[test]
    fn start_while_active_is_noop() {
        let mut g = GadgetFlash::default();
        g.start(10, 7, 1);
        let snapshot = g;
        assert!(!g.start(20, 0, 0), "guard returns false");
        assert_eq!(g.period, snapshot.period);
        assert_eq!(g.countdown, snapshot.countdown);
        assert_eq!(g.state, snapshot.state);
    }

    #[test]
    fn stop_from_active_zeros_in_order() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 1);
        assert!(g.stop());
        assert_eq!(g.state, 0);
        assert_eq!(g.countdown, 0);
        assert_eq!(g.period, 0);
    }

    #[test]
    fn stop_from_idle_is_noop() {
        let mut g = GadgetFlash::default();
        assert!(!g.stop());
    }

    #[test]
    fn tick_decrements_countdown_until_toggle() {
        let mut g = GadgetFlash::default();
        // First cycle: period=10, extra_delay=5, initial=0 → countdown=15.
        g.start(10, 5, 0);
        for _ in 0..14 {
            assert!(!g.tick(), "no toggle yet");
        }
        assert!(g.tick(), "15th tick toggles");
        assert_eq!(g.state, 1, "state XOR-toggled to 1");
        assert_eq!(g.countdown, 10, "countdown resets to period, not period+extra");
    }

    #[test]
    fn tick_steady_state_toggles_every_period_ticks() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 0);
        // First cycle: countdown=10. 10 ticks → toggle.
        for _ in 0..9 {
            assert!(!g.tick());
        }
        assert!(g.tick());
        assert_eq!(g.state, 1);
        // Second cycle: countdown=10. 10 more ticks → toggle back to 0.
        for _ in 0..9 {
            assert!(!g.tick());
        }
        assert!(g.tick());
        assert_eq!(g.state, 0);
    }

    #[test]
    fn tick_when_idle_is_noop() {
        let mut g = GadgetFlash::default();
        assert!(!g.tick());
        assert_eq!(g.countdown, 0);
        assert_eq!(g.state, 0);
    }

    #[test]
    fn tick_when_disabled_auto_stops_active_flash() {
        let mut g = GadgetFlash::default();
        g.start(10, 0, 1);
        g.disabled = true;
        assert!(g.tick(), "auto-stop reports a change");
        assert_eq!(g.state, 0);
        assert_eq!(g.countdown, 0);
        assert_eq!(g.period, 0);
    }

    #[test]
    fn tick_when_disabled_and_idle_is_noop() {
        let mut g = GadgetFlash::default();
        g.disabled = true;
        assert!(!g.tick());
    }
}
```

**Step 4: Register the module in `src/sidebar/mod.rs`.**

Edit `src/sidebar/mod.rs` to add the new module alongside `power_bar_anim` and `sidebar_view`. Find the line `pub mod power_bar_anim;` near the top of the file and add directly below it:

```rust
pub mod gadget_flash;
```

**Step 5: Verify.**

Run from the repo root:

```
cargo test -p ra2-rust-game gadget_flash:: -- --nocapture
```

Expected: 8 tests pass.

Also run `cargo check` and confirm no warnings on the new file.

**Step 6: Commit.**

```
git add src/sidebar/gadget_flash.rs src/sidebar/mod.rs
git commit -m "sidebar/gadget_flash: add GadgetFlash primitive (start/stop/tick)"
```

---

### Task 2: `frame_select` free function

**Why:** The 5-frame state-select table is shared by tabs, Repair, and Sell. Pure function — easiest to unit-test before any caller exists.

**Files:**
- Modify: `src/sidebar/gadget_flash.rs` (append below the `GadgetFlash` impl, before the `#[cfg(test)]` block)

**Pattern:** Pure free function with table-driven unit tests, same file as the type it operates near.

**Step 1: Add the function.**

Append after the `impl GadgetFlash` block and before the `#[cfg(test)]` block:

```rust
/// Pick a SHP frame index for a 5-frame gadget given its three state bits.
///
/// Mirrors the SBGadgetClass::Draw conditional at gamemd's `0x0069DEB0`.
/// Output indices map to the 5-frame SHP convention used by `tab0N.shp`,
/// `repair.shp`, and `sell.shp`:
///   0 = idle, 1 = mode-active, 2 = disabled, 3 = pressed-idle, 4 = pressed-active.
///
/// Inputs:
/// - `disabled`: the gadget's disabled gate.
/// - `mode_active`: the persistent "this mode is on" / "this tab is selected" bit.
///   For tabs this is the active-tab bit; for Repair/Sell this is the
///   mode-on toggle.
/// - `state`: the transient "drawn as pressed" bit (set by mouse-down OR by
///   the flash AI's tick toggle).
///
/// The function assumes the gadget is pressable (the not-pressable / hover-static
/// branch from the binary is unused for any of our 5-frame gadgets).
pub fn frame_select(disabled: bool, mode_active: bool, state: u8) -> u8 {
    if disabled {
        return 2;
    }
    if state != 0 {
        if mode_active { 4 } else { 3 }
    } else if mode_active {
        1
    } else {
        0
    }
}
```

**Step 2: Add table-driven unit tests in the `#[cfg(test)]` block.**

Append inside `mod tests`:

```rust
    #[test]
    fn frame_select_table() {
        // (disabled, mode_active, state) → expected frame
        let cases: &[(bool, bool, u8, u8)] = &[
            (false, false, 0, 0),  // idle
            (false, true,  0, 1),  // mode-active
            (true,  false, 0, 2),  // disabled (mode and state ignored)
            (true,  true,  0, 2),
            (true,  false, 1, 2),
            (true,  true,  1, 2),
            (false, false, 1, 3),  // pressed-idle
            (false, true,  1, 4),  // pressed-active
        ];
        for &(disabled, mode_active, state, expected) in cases {
            let got = frame_select(disabled, mode_active, state);
            assert_eq!(
                got, expected,
                "frame_select(disabled={disabled}, mode_active={mode_active}, state={state}) expected {expected}, got {got}"
            );
        }
    }
```

**Step 3: Verify.**

```
cargo test -p ra2-rust-game gadget_flash::tests::frame_select_table
```

Expected: PASS.

**Step 4: Commit.**

```
git commit -am "sidebar/gadget_flash: add frame_select 5-frame state table"
```

---

### Task 3: `SidebarGadgetState` aggregator

**Why:** Holds the four tab flashes plus Repair/Sell mode flags plus the `last_sim_tick` cache. Single AppState field consumed by both orchestrator and view builder.

**Files:**
- Modify: `src/sidebar/gadget_flash.rs` (append, still before the `#[cfg(test)]` block)

**Pattern:** Aggregator struct with `new` and read-only frame accessors. Mirrors how `PowerBarAnimState` aggregates per-segment counters.

**Step 1: Add the struct + impl.**

Append after the `frame_select` function:

```rust
/// Persistent flash + mode state for the in-game sidebar gadgets.
///
/// Lives on `AppState` (one instance per app session). Ticked once per sim
/// tick by `app_sidebar_gadgets::update_sidebar_gadget_state`. Read by the
/// per-frame `SidebarView` builder to populate gadget `frame_index` fields.
#[derive(Debug, Clone, Default)]
pub struct SidebarGadgetState {
    /// One flash per tab gadget, indexed by `SidebarTab::tab_index()`.
    /// Building (0) and Infantry (2) are kept in the array for uniformity
    /// but the orchestrator unconditionally `stop()`s them — they never
    /// flash in retail.
    pub tab_flashes: [GadgetFlash; 4],

    /// Per-tab disabled bit (mirrors gadget +0x1e). v1: always false.
    /// Kept to keep the gadget tick path identical to gamemd's primitive.
    pub tab_disabled: [bool; 4],

    /// Mirrors SidebarClass +0x46c. Toggled by clicking the Repair button.
    /// Mutually exclusive with `sell_mode_on` and `AppState.targeting_mode`.
    pub repair_mode_on: bool,

    /// Mirrors SidebarClass +0x11B1. Toggled by clicking the Sell button.
    /// Mutually exclusive with `repair_mode_on` and `AppState.targeting_mode`.
    pub sell_mode_on: bool,

    /// Per-button disabled bits. v1: always false.
    pub repair_disabled: bool,
    pub sell_disabled: bool,

    /// Last sim tick the orchestrator processed; used to advance per
    /// sim-tick delta (catch-up safe).
    pub last_sim_tick: u64,
}

impl SidebarGadgetState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Frame index for a tab gadget. Caller passes whether this tab is the
    /// currently-active tab (the `+0x2D` mirror in gamemd).
    pub fn tab_frame(&self, tab_index: usize, is_active_tab: bool) -> u8 {
        let flash = &self.tab_flashes[tab_index];
        let disabled = self.tab_disabled[tab_index];
        frame_select(disabled, is_active_tab, flash.state)
    }

    /// Frame index for the Repair button. Repair has no flash AI — state is
    /// always 0; the visible "stays pressed" effect comes from `mode_active`.
    pub fn repair_frame(&self) -> u8 {
        frame_select(self.repair_disabled, self.repair_mode_on, 0)
    }

    /// Frame index for the Sell button. Same logic as `repair_frame`.
    pub fn sell_frame(&self) -> u8 {
        frame_select(self.sell_disabled, self.sell_mode_on, 0)
    }
}
```

**Step 2: Add a small unit test for the frame accessors.**

Append inside `mod tests`:

```rust
    #[test]
    fn aggregator_frame_accessors() {
        let mut s = SidebarGadgetState::new();
        // Idle tab → 0; active tab → 1.
        assert_eq!(s.tab_frame(0, false), 0);
        assert_eq!(s.tab_frame(0, true), 1);
        // Flash mid-pulse on tab 3 (Vehicle).
        s.tab_flashes[3].state = 1;
        s.tab_flashes[3].period = 10;
        assert_eq!(s.tab_frame(3, false), 3, "inactive + pressed-look");
        assert_eq!(s.tab_frame(3, true), 4, "active + pressed-look");
        // Disabled overrides everything.
        s.tab_disabled[3] = true;
        assert_eq!(s.tab_frame(3, true), 2);
        // Repair/Sell.
        assert_eq!(s.repair_frame(), 0);
        s.repair_mode_on = true;
        assert_eq!(s.repair_frame(), 1);
        s.repair_disabled = true;
        assert_eq!(s.repair_frame(), 2);
        s.sell_mode_on = true;
        s.sell_disabled = false;
        assert_eq!(s.sell_frame(), 1);
    }
```

**Step 3: Verify.**

```
cargo test -p ra2-rust-game gadget_flash::tests::aggregator_frame_accessors
```

Expected: PASS.

**Step 4: Commit.**

```
git commit -am "sidebar/gadget_flash: add SidebarGadgetState aggregator"
```

---

### Task 4: `AppState.sidebar_gadget_state` field

**Why:** Make the aggregator persistent across frames by hanging it off `AppState`. Mirrors how `power_bar_anim` is wired.

**Files:**
- Modify: `src/app.rs` (struct field + `Default`/`new` init)

**Pattern:** Direct copy of the `power_bar_anim` pattern at line 171 / line 1064.

**Step 1: Find the existing `power_bar_anim` field.** Open `src/app.rs` and search for `power_bar_anim: crate::sidebar::PowerBarAnimState`. There are two occurrences: a field declaration (≈ line 171) and an init in `Default`/`new` (≈ line 1064).

**Step 2: Add the new field directly after the `power_bar_anim` field declaration.**

```rust
    pub(crate) power_bar_anim: crate::sidebar::PowerBarAnimState,
    /// Persistent flash + mode state for in-game sidebar gadgets. Ticked from
    /// `app_sidebar_gadgets::update_sidebar_gadget_state` once per sim tick;
    /// read each frame by the sidebar view builder to pick SHP frame indices.
    pub(crate) sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState,
```

**Step 3: Add the init in the constructor.** Find the init line `power_bar_anim: crate::sidebar::PowerBarAnimState::new(),` and add directly after:

```rust
            power_bar_anim: crate::sidebar::PowerBarAnimState::new(),
            sidebar_gadget_state: crate::sidebar::gadget_flash::SidebarGadgetState::new(),
```

**Step 4: Verify.**

```
cargo check -p ra2-rust-game
```

Expected: compiles clean. The field is added but not yet read or written — `dead_code` warnings on the type are acceptable until Task 7.

**Step 5: Commit.**

```
git commit -am "app: add sidebar_gadget_state field to AppState"
```

---

### Task 5: `SidebarToggleButton` view struct + `SidebarAction` variants

**Why:** The view-side data model for Repair/Sell. Each is an SHP-driven button with a frame index (unlike `SidebarControlButton` which carries a text label). Also add the new action variants so the click handler signatures land before the routing code in Task 9.

**Files:**
- Modify: `src/sidebar/mod.rs` — add struct, modify `SidebarAction` enum.

**Pattern:** New type alongside the existing `SidebarTabButton` / `SidebarControlButton`. Action enum extension mirrors existing variants.

**Step 1: Add `frame_index` to `SidebarTabButton`.** Find the struct (around line 178):

```rust
#[derive(Debug, Clone)]
pub struct SidebarTabButton {
    pub tab: SidebarTab,
    pub rect: Rect,
    pub active: bool,
}
```

Change to:

```rust
#[derive(Debug, Clone)]
pub struct SidebarTabButton {
    pub tab: SidebarTab,
    pub rect: Rect,
    /// True when this is the currently-selected tab. Used by hit-test
    /// disambiguation; the rendered visual is driven by `frame_index`.
    pub active: bool,
    /// SHP frame index (0..=4) for the per-theme tab SHP atlas. Picked by
    /// `SidebarGadgetState::tab_frame` each frame.
    pub frame_index: u8,
}
```

**Step 2: Add `SidebarToggleButton` directly after `SidebarTabButton`.**

```rust
/// View entry for an SHP-driven toggle button (Repair, Sell).
/// Rect for hit-testing, action to dispatch on click, frame index for the
/// 5-frame SHP state table.
#[derive(Debug, Clone)]
pub struct SidebarToggleButton {
    pub rect: Rect,
    pub action: SidebarAction,
    /// SHP frame index (0..=4) for the button's per-theme SHP atlas.
    pub frame_index: u8,
}
```

**Step 3: Add the two new action variants.** Find the `SidebarAction` enum (around line 127). Add `ToggleRepairMode` and `ToggleSellMode` directly before the final `Deploy` variant:

```rust
    CycleOwner,
    PlaceStarterBase,
    SpawnTestUnits,
    /// Toggle Repair-mode (cursor stays armed for clicking buildings to repair).
    /// Mutually exclusive with `ToggleSellMode` and any active `TargetingMode`.
    ToggleRepairMode,
    /// Toggle Sell-mode (cursor stays armed for clicking buildings to sell).
    /// Mutually exclusive with `ToggleRepairMode` and any active `TargetingMode`.
    ToggleSellMode,
    Deploy,
```

**Step 4: Add the Repair/Sell fields to `SidebarView`.** Find `SidebarView` (around line 207) and add two fields directly above `cancel_button`:

```rust
    pub tabs: Vec<SidebarTabButton>,
    pub items: Vec<SidebarItem>,
    /// Repair button (toggle mode). Rendered from the per-theme atlas's
    /// `repair_frames[frame_index]`. Hit-test routes to
    /// `SidebarAction::ToggleRepairMode`.
    pub repair_button: SidebarToggleButton,
    /// Sell button (toggle mode). Rendered from the per-theme atlas's
    /// `sell_frames[frame_index]`. Hit-test routes to
    /// `SidebarAction::ToggleSellMode`.
    pub sell_button: SidebarToggleButton,
    pub cancel_button: SidebarControlButton,
```

**Step 5: Verify.**

```
cargo check -p ra2-rust-game
```

Expected: compilation errors complaining about missing `frame_index` / `repair_button` / `sell_button` fields at the view-builder construction site in `src/sidebar/sidebar_view.rs:196`. **That is intentional and expected** — Task 6 fixes it. Confirm those are the only errors; if anything else fails (mismatched variants in `app_input.rs`, etc.), inspect before proceeding.

**Step 6: Do NOT commit yet.** Hold this change uncommitted until Task 6 closes the compile gap. Same commit covers the data model + the builder population.

---

### Task 6: View builder populates `frame_index` and Repair/Sell buttons

**Why:** Closes the compile gap from Task 5 and threads `SidebarGadgetState` into the view construction.

**Files:**
- Modify: `src/sidebar/sidebar_view.rs` — add `gadget_state` parameter to `build_sidebar_view_with_spec`; populate `frame_index` on each tab; build the Repair/Sell rects from `SidebarChromeLayoutSpec` and pick their frame index from gadget state.

**Pattern:** Extends the existing builder signature; rect calculation mirrors how tab rects are built around lines 109-131.

**Step 1: Extend the `build_sidebar_view_with_spec` signature.** Find the signature at line 53:

```rust
pub(crate) fn build_sidebar_view_with_spec(
    layout_spec: SidebarChromeLayoutSpec,
    ...
    sw_views: &[SuperWeaponView],
) -> SidebarView {
```

Add THREE new trailing parameters (last positions, before `) -> SidebarView`):

```rust
pub(crate) fn build_sidebar_view_with_spec(
    layout_spec: SidebarChromeLayoutSpec,
    ...
    sw_views: &[SuperWeaponView],
    gadget_state: &crate::sidebar::gadget_flash::SidebarGadgetState,
    /// Already-scaled (× ui_scale) dimensions of the repair button SHP.
    /// `None` when the atlas isn't loaded yet — view builder uses zero-size
    /// rects so hit-test never matches before the chrome is ready.
    repair_button_size: Option<[f32; 2]>,
    /// Already-scaled (× ui_scale) dimensions of the sell button SHP.
    sell_button_size: Option<[f32; 2]>,
) -> SidebarView {
```

`repair_button_size` and `sell_button_size` follow the same convention as the
existing `tab_button_size` parameter — pre-scaled by `ui_scale` at the call
site so the view builder sees screen-pixel dimensions consistently.

Also import at the top of the file:

```rust
use super::gadget_flash::SidebarGadgetState;
```

(unused warning will resolve in Step 3).

**Step 2: Update the no-spec convenience wrapper** at line 17, `build_sidebar_view`, to also take and forward the three new parameters. It currently has the same signature minus the spec; pass through `gadget_state`, `repair_button_size`, and `sell_button_size`. **Check callers:**

```
grep -n "build_sidebar_view\b" src/
```

Expected: only the 3 test callers in `src/sidebar/sidebar_view.rs` (at lines 459, 483, 520). Update each test caller to pass `&SidebarGadgetState::new()`, `None`, `None` as the new trailing arguments:

```rust
let view = build_sidebar_view(
    1280.0,
    960.0,
    SidebarTab::Building,
    ...
    None,
    &crate::sidebar::gadget_flash::SidebarGadgetState::new(),
    None,    // repair_button_size — atlas not loaded in unit tests
    None,    // sell_button_size
);
```

**Step 3: Populate `frame_index` on tabs.** Find the `tabs: Vec<SidebarTabButton> = SidebarTab::all()` block around line 109. Replace the `SidebarTabButton { tab, rect, active }` construction with:

```rust
SidebarTabButton {
    tab,
    rect: Rect {
        x: tab_start_x + idx as f32 * tab_w + nudge,
        y: tab_y,
        w: tab_w,
        h: tab_h,
    },
    active: tab == active_tab,
    frame_index: gadget_state.tab_frame(idx, tab == active_tab),
}
```

**Step 4: Build the Repair/Sell button rects + populate frame indices.** Add a new block after the tab construction and before the cameo grid (around line 132-133):

```rust
    // Repair / Sell SHP-driven toggle buttons. Position comes from
    // SidebarChromeLayoutSpec (already-scaled). Dimensions come from the
    // chrome atlas via repair_button_size / sell_button_size (already × ui_scale
    // at the call site) — matches the tab_button_size convention so hit-test
    // and render rects agree at every UI scale. When the atlas is unavailable
    // (callers passing None), rects collapse to zero size so hit-test never
    // matches. Frame index comes from SidebarGadgetState. Per-theater positions
    // (Temperate vs others) are deferred — see disparity scan item A23.
    let [repair_w, repair_h] = repair_button_size.unwrap_or([0.0, 0.0]);
    let [sell_w, sell_h] = sell_button_size.unwrap_or([0.0, 0.0]);
    let side1_y_local = layout.side1_y;  // already in screen space.
    let repair_rect = Rect {
        x: layout.sidebar_x + layout_spec.repair_x,
        y: side1_y_local + layout_spec.repair_y,
        w: repair_w,
        h: repair_h,
    };
    let sell_rect = Rect {
        x: layout.sidebar_x + layout_spec.sell_x,
        y: side1_y_local + layout_spec.sell_y,
        w: sell_w,
        h: sell_h,
    };
    let repair_button = SidebarToggleButton {
        rect: repair_rect,
        action: SidebarAction::ToggleRepairMode,
        frame_index: gadget_state.repair_frame(),
    };
    let sell_button = SidebarToggleButton {
        rect: sell_rect,
        action: SidebarAction::ToggleSellMode,
        frame_index: gadget_state.sell_frame(),
    };
```

Also add `SidebarToggleButton` to the import block at the top of `sidebar_view.rs`:

```rust
use super::{
    CAMEO_COLUMNS, Rect, SidebarAction, SidebarChromeLayoutSpec, SidebarControlButton, SidebarItem,
    SidebarTab, SidebarTabButton, SidebarToggleButton, SidebarView, compute_layout_with_spec,
};
```

**Step 5: Add the new fields to the `SidebarView { ... }` construction** at line 196:

```rust
    SidebarView {
        panel_rect,
        layout,
        ...
        tabs,
        items,
        repair_button,
        sell_button,
        pause_button: ...,
        ...
    }
```

**Step 6: Update callers in `src/app_sidebar_render.rs`.**

Find the two `build_sidebar_view_with_spec(...)` calls around lines 102-119 and 122-139. Add three trailing arguments to both:

```rust
    let mut view = sidebar::build_sidebar_view_with_spec(
        state.sidebar_layout_spec,
        ...
        &sw_views,
        &state.sidebar_gadget_state,
        None,    // repair_button_size — wired in Task 9 once atlas.repair_frames exists.
        None,    // sell_button_size — same.
    );
    if state.sidebar_scroll_rows > view.max_scroll_rows {
        state.sidebar_scroll_rows = view.max_scroll_rows;
        view = sidebar::build_sidebar_view_with_spec(
            state.sidebar_layout_spec,
            ...
            &sw_views,
            &state.sidebar_gadget_state,
            None,
            None,
        );
    }
```

The `None` placeholders are intentional and temporary. The actual extraction
(`atlas.repair_frames[0]` × `state.ui_scale`) lands in Task 9 Step 8 once the
atlas's new `repair_frames` / `sell_frames` fields exist. At Task 6 commit time,
repair/sell view rects collapse to zero size — which is correct, because the
buttons aren't rendered yet (Task 10) and aren't hit-test-routed yet (Task 11).
Nothing observable depends on the zero rects between Task 6 and Task 10.

**Step 7: Verify.**

```
cargo check -p ra2-rust-game
cargo test -p ra2-rust-game sidebar::
```

Expected: clean compile. The two existing tab + control button tests should still pass.

**Step 8: Commit.**

```
git commit -am "sidebar: thread SidebarGadgetState into view builder + add Repair/Sell view entries"
```

(Combines Task 5 + Task 6 since Task 5 was held unsplit.)

---

### Task 7: Orchestrator — poll, Start/Stop, tick

**Why:** The brain. Each sim tick, scan SuperWeapon state, drive Start/Stop on the Defense tab, unconditionally Stop the other three tabs, then tick all four flashes the correct number of times.

**Files:**
- Create: `src/app_sidebar_gadgets.rs`
- Modify: `src/lib.rs` (register the module)

**Pattern:** Mirrors `src/app_building_anim.rs` `update_power_bar_anim` shape: a `pub(crate) fn update_xxx(state: &mut AppState)` called per render frame from `app_sim_tick.rs`.

**Step 1: Create the new file.**

```rust
//! Per-sim-tick orchestrator for sidebar gadget state.
//!
//! Mirrors a narrowed slice of gamemd's StripClass::AI poll +
//! SidebarClass::Action Flash_AI driver. Polls the local owner's SuperWeapon
//! state each call:
//!  - Defense tab flashes when any super-weapon is charged and ready
//!    (gamemd Tab 1 trigger).
//!  - Building, Infantry, and Vehicle tabs never flash in this bundle.
//!    (Vehicle-tab "aircraft waiting for helipad" trigger is deferred — the
//!    Rust sim auto-spawns or refunds aircraft on completion with no waiting
//!    state to poll. See the plan's Scope note.)
//!
//! Flash period is exactly 10 game-logic ticks. The orchestrator advances
//! `GadgetFlash::tick` by `sim.tick - last_sim_tick` per call so the period
//! is measured in sim ticks (not render frames), matching the binary.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app_commands::preferred_local_owner_name;
use crate::sidebar::SidebarTab;

/// Period (game ticks) of the per-tab pulse. Literal from gamemd
/// `MOV ECX, 0xa` at 006a8e58. Source:
/// ra2-rust-game-docs/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md §4.
const FLASH_PERIOD_TICKS: u32 = 10;

/// Drive the sidebar gadget state for this frame. Call once per render frame
/// after `update_power_bar_anim`.
pub(crate) fn update_sidebar_gadget_state(state: &mut AppState) {
    let Some(sim) = state.simulation.as_ref() else {
        return;
    };
    let Some(rules) = state.rules.as_ref() else {
        return;
    };
    let owner = preferred_local_owner_name(state).unwrap_or_else(|| "Americans".to_string());

    // --- Step 1: poll trigger conditions. ---
    let sw_ready = has_charged_sw_for_owner(sim, rules, &owner);

    // --- Step 2: compute the phase-aligned start args for THIS frame. ---
    // Mirrors StripClass::AI 006a8e52..006a8e9b. extra_delay always lands the
    // first toggle on the second-next 10-tick boundary; parity bit phase-aligns
    // concurrent flashes started in the same 10-tick window.
    let frame = sim.tick;
    let extra_delay: u32 = (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS))
        % FLASH_PERIOD_TICKS;
    let next_boundary = (extra_delay as u64 + frame) / FLASH_PERIOD_TICKS as u64;
    let initial_state: u8 = if next_boundary & 1 == 0 { 1 } else { 0 };

    // --- Step 3: drive Start/Stop on each tab. ---
    let gadgets = &mut state.sidebar_gadget_state;
    // Building (idx 0) — never flashes in retail.
    gadgets.tab_flashes[SidebarTab::Building.tab_index()].stop();
    // Defense (idx 1) — flashes on any SW ready.
    if sw_ready {
        gadgets.tab_flashes[SidebarTab::Defense.tab_index()]
            .start(FLASH_PERIOD_TICKS, extra_delay, initial_state);
    } else {
        gadgets.tab_flashes[SidebarTab::Defense.tab_index()].stop();
    }
    // Infantry (idx 2) — never flashes in retail.
    gadgets.tab_flashes[SidebarTab::Infantry.tab_index()].stop();
    // Vehicle (idx 3) — DEFERRED. Faithful trigger would be "aircraft waiting
    // for helipad," which has no current Rust sim representation. Keep
    // stopped until that semantic exists.
    gadgets.tab_flashes[SidebarTab::Vehicle.tab_index()].stop();

    // --- Step 4: advance flash AI per sim-tick delta. ---
    let last = gadgets.last_sim_tick;
    let delta = frame.saturating_sub(last);
    for _ in 0..delta {
        for f in &mut gadgets.tab_flashes {
            f.tick();
        }
    }
    gadgets.last_sim_tick = frame;
}

fn has_charged_sw_for_owner(
    sim: &crate::sim::world::Simulation,
    rules: &crate::rules::ruleset::RuleSet,
    owner: &str,
) -> bool {
    if !sim.game_options.super_weapons {
        return false;
    }
    let owner_iid = sim.interner.get(owner).unwrap_or_default();
    crate::sim::superweapon::superweapon_views_for_owner(sim, rules, &owner_iid)
        .iter()
        .any(|sw| sw.is_ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the phase math literal-for-literal against gamemd.
    /// From SIDEBAR_TAB_FLASH_SCHEDULER §4.1: at frame F, extra_delay =
    /// (10 - F % 10) % 10. At frame 0 extra_delay = 0; at frame 9 extra_delay = 1.
    #[test]
    fn phase_math_examples() {
        let cases: &[(u64, u32, u8)] = &[
            // (frame, expected_extra_delay, expected_initial_state)
            (0,  0, 1), // next_boundary = 0; 0 & 1 == 0 → 1
            (5,  5, 0), // next_boundary = (5+5)/10 = 1; 1 & 1 == 1 → 0
            (9,  1, 0), // next_boundary = (1+9)/10 = 1 → 0
            (10, 0, 0), // next_boundary = 10/10 = 1 → 0
            (15, 5, 1), // next_boundary = (5+15)/10 = 2; 2 & 1 == 0 → 1
            (20, 0, 1), // next_boundary = 20/10 = 2 → 1
        ];
        for &(frame, expected_extra, expected_state) in cases {
            let extra = (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS))
                % FLASH_PERIOD_TICKS;
            let nb = (extra as u64 + frame) / FLASH_PERIOD_TICKS as u64;
            let st: u8 = if nb & 1 == 0 { 1 } else { 0 };
            assert_eq!(extra, expected_extra, "extra_delay at frame {frame}");
            assert_eq!(st, expected_state, "initial_state at frame {frame}");
        }
    }
}
```

**Step 2: Register the module.** Open `src/lib.rs`, find the existing list of `pub mod app_*` declarations, and add (in alphabetical position):

```rust
pub mod app_sidebar_gadgets;
```

**Step 3: Verify.**

```
cargo check -p ra2-rust-game
cargo test -p ra2-rust-game app_sidebar_gadgets::
```

Expected: clean compile. The `phase_math_examples` test passes (this is a pure-math unit test; no sim setup needed).

**Step 4: Commit.**

```
git add src/app_sidebar_gadgets.rs src/lib.rs
git commit -m "app_sidebar_gadgets: add poll-driven orchestrator for tab flashes"
```

---

### Task 8: Wire the orchestrator into the sim-tick loop

**Why:** Without this hook the orchestrator never runs. One line in the existing per-render-frame block.

**Files:**
- Modify: `src/app_sim_tick.rs:207` (insertion point may shift due to the parallel session's refinery-SFX edit; use surrounding-context anchor).

**Pattern:** Same shape as the existing `update_power_bar_anim(state)` call directly above.

**Step 1: Find the insertion point.**

```
grep -n "update_power_bar_anim" src/app_sim_tick.rs
```

Expected: one line, e.g. `207:    crate::app_building_anim::update_power_bar_anim(state);`.

**Step 2: Add the new call directly after `update_power_bar_anim`.**

Open `src/app_sim_tick.rs`, find:

```rust
    crate::app_building_anim::update_power_bar_anim(state);
```

Add directly below:

```rust
    crate::app_sidebar_gadgets::update_sidebar_gadget_state(state);
```

**Step 3: Verify.**

```
cargo check -p ra2-rust-game
```

Expected: clean compile.

**Step 4: Commit.**

```
git commit -am "app_sim_tick: call update_sidebar_gadget_state each render frame"
```

---

### Task 9: Chrome atlas — load 5 frames per tab + repair/sell

**Why:** Without these frames in the atlas, the renderer has no sprite to draw for the disabled / pressed / pressed-active states.

**Files:**
- Modify: `src/render/sidebar_chrome.rs` — refactor `tab_buttons` / `tab_buttons_active` / `repair` / `sell` to per-frame arrays. Update the loader.

**Pattern:** Matches the existing `powerp_frames: [Option<SidebarChromeEntry>; 5]` array pattern at line 91.

**Step 1: Refactor the struct fields.**

Find the `SidebarChromeAtlas` struct (around line 58). Replace these four fields:

```rust
    pub tabs: Option<SidebarChromeEntry>,
    pub tab_buttons: Vec<SidebarChromeEntry>,
    pub tab_buttons_active: Vec<SidebarChromeEntry>,
    ...
    pub repair: Option<SidebarChromeEntry>,
    pub sell: Option<SidebarChromeEntry>,
```

with:

```rust
    pub tabs: Option<SidebarChromeEntry>,
    /// Tab buttons: per-tab × 5-frame SHP state table.
    /// Outer index: tab number (0..=3). Inner index: SHP frame (0..=4).
    /// Frame meanings (from SBGadgetClass::Draw, gamemd 0x0069DEB0):
    ///   0 = idle, 1 = active-tab, 2 = disabled, 3 = pressed-idle, 4 = pressed-active.
    pub tab_frames: [[Option<SidebarChromeEntry>; 5]; 4],
    ...
    /// Repair button — 5-frame SHP state table (same convention as `tab_frames`).
    pub repair_frames: [Option<SidebarChromeEntry>; 5],
    /// Sell button — 5-frame SHP state table (same convention as `tab_frames`).
    pub sell_frames: [Option<SidebarChromeEntry>; 5],
```

**Step 2: Refactor the loader.**

Find the existing per-frame loader (lines 284-293):

```rust
    let tabs = render_entry(asset_manager, &mix, "tabs.shp", &tabs_palette, 0);
    let tab_entries: Vec<RenderedChromeEntry> = (0..4)
        .filter_map(|i| render_entry(asset_manager, &mix, &format!("tab0{i}.shp"), &palette, 0))
        .collect();
    // Frame 1 is the brighter selected/highlighted tab state in the stock art.
    let tab_active_entries: Vec<RenderedChromeEntry> = (0..4)
        .filter_map(|i| render_entry(asset_manager, &mix, &format!("tab0{i}.shp"), &palette, 1))
        .collect();
    let repair = render_entry(asset_manager, &mix, "repair.shp", &palette, 0);
    let sell = render_entry(asset_manager, &mix, "sell.shp", &palette, 0);
```

Replace with:

```rust
    let tabs = render_entry(asset_manager, &mix, "tabs.shp", &tabs_palette, 0);

    // Tab buttons: 4 tabs × 5 frames each. Missing frames in retail fall
    // back to None; the renderer skips that frame's draw if not present.
    // Frame 0 = idle, 1 = active-tab, 2 = disabled, 3 = pressed-idle,
    // 4 = pressed-active. See SIDEBAR_REPAIR_SELL_BUTTON §5.
    let mut tab_frame_entries: [[Option<RenderedChromeEntry>; 5]; 4] = Default::default();
    for tab in 0..4 {
        for frame in 0..5 {
            let entry = render_entry(asset_manager, &mix, &format!("tab0{tab}.shp"), &palette, frame);
            if entry.is_none() && frame > 0 {
                tracing::warn!(
                    "tab0{tab}.shp frame {frame} missing in MIX — gadget state {frame} will fall back to idle"
                );
            }
            tab_frame_entries[tab][frame] = entry;
        }
    }

    let mut repair_frame_entries: [Option<RenderedChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        let entry = render_entry(asset_manager, &mix, "repair.shp", &palette, frame);
        if entry.is_none() && frame > 0 {
            tracing::warn!("repair.shp frame {frame} missing in MIX");
        }
        repair_frame_entries[frame] = entry;
    }

    let mut sell_frame_entries: [Option<RenderedChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        let entry = render_entry(asset_manager, &mix, "sell.shp", &palette, frame);
        if entry.is_none() && frame > 0 {
            tracing::warn!("sell.shp frame {frame} missing in MIX");
        }
        sell_frame_entries[frame] = entry;
    }
```

**Step 3: Update the sizing pass (lines ~375-388).** Find the `all_entries.push(...)` block where every rendered chrome piece is pushed for atlas-dimension computation. The current code has:

```rust
    for tab in &tab_entries {
        all_entries.push(tab);
    }
    for tab in &tab_active_entries {
        all_entries.push(tab);
    }
    ...
    if let Some(ref r) = repair {
        all_entries.push(r);
    }
    if let Some(ref s) = sell {
        all_entries.push(s);
    }
```

Replace with the array iteration:

```rust
    for tab in 0..4 {
        for frame in 0..5 {
            if let Some(ref entry) = tab_frame_entries[tab][frame] {
                all_entries.push(entry);
            }
        }
    }
    for frame in 0..5 {
        if let Some(ref entry) = repair_frame_entries[frame] {
            all_entries.push(entry);
        }
    }
    for frame in 0..5 {
        if let Some(ref entry) = sell_frame_entries[frame] {
            all_entries.push(entry);
        }
    }
```

**Step 4: Update the packing pass.** The four blocks to replace are NOT contiguous — they sit in two zones, with the `side2_uv` and `side3_uv` blits in between which must be **preserved unchanged**:

```
456-461  tab_button_uvs            ← REPLACE (Vec build)
462-467  tab_button_active_uvs     ← REPLACE (Vec build)
469-470  side2_uv = blit_entry(...)   ← KEEP AS-IS
471-472  side3_uv = blit_entry(...)   ← KEEP AS-IS
474-478  repair_uv = repair.as_ref().map(...)   ← REPLACE
479-483  sell_uv   = sell.as_ref().map(...)     ← REPLACE
```

The existing pattern in each block is `let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry); y += entry.height + CHROME_PADDING;` (the four-block group above uses small variations of this).

Replace the **four** target blocks with three new blocks. Insert the Tabs block where `tab_button_uvs` was (lines 456-461; also remove 462-467 since `tab_button_active_uvs` is gone). Insert the Repair and Sell blocks where `repair_uv` and `sell_uv` were (lines 474-483). **Leave side2_uv and side3_uv at 469-472 untouched** — they retain their current position between the tab group and the repair/sell group.

Tabs:

```rust
    let mut tab_frames_packed: [[Option<SidebarChromeEntry>; 5]; 4] = Default::default();
    for tab in 0..4 {
        for frame in 0..5 {
            if let Some(ref entry) = tab_frame_entries[tab][frame] {
                let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry);
                y += entry.height + CHROME_PADDING;
                tab_frames_packed[tab][frame] = Some(uv);
            }
        }
    }
```

Repair:

```rust
    let mut repair_frames_packed: [Option<SidebarChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        if let Some(ref entry) = repair_frame_entries[frame] {
            let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry);
            y += entry.height + CHROME_PADDING;
            repair_frames_packed[frame] = Some(uv);
        }
    }
```

Sell (identical shape to Repair, swap names):

```rust
    let mut sell_frames_packed: [Option<SidebarChromeEntry>; 5] = Default::default();
    for frame in 0..5 {
        if let Some(ref entry) = sell_frame_entries[frame] {
            let uv = blit_entry(&mut rgba, atlas_width, atlas_height, y, entry);
            y += entry.height + CHROME_PADDING;
            sell_frames_packed[frame] = Some(uv);
        }
    }
```

These three blocks REPLACE the old four target blocks. Resulting y-stack order
is: tab_frames_packed (all 20 entries) → side2_uv → side3_uv → repair_frames_packed
→ sell_frames_packed → power → powerp → extras. The side2/side3 atlas y position
shifts down by the additional tab entries (16 new — 5 frames × 4 tabs minus the
8 old entries), but that's a within-atlas y change that doesn't affect on-screen
position because all consumers read UVs from the stored `SidebarChromeEntry`.

**Step 5: Update the log block at lines ~530-535.** The existing code logs each
tab's dimensions twice (inactive + active). Since `tab_entries` /
`tab_active_entries` no longer exist, replace:

```rust
    for (i, tab) in tab_entries.iter().enumerate() {
        log::info!("  tab0{} (inactive): {}x{}", i, tab.width, tab.height);
    }
    for (i, tab) in tab_active_entries.iter().enumerate() {
        log::info!("  tab0{} (active):   {}x{}", i, tab.width, tab.height);
    }
```

with a single log over frame 0 of each tab (the dimensions are uniform across
frames in retail SHPs):

```rust
    for tab in 0..4 {
        if let Some(ref entry) = tab_frame_entries[tab][0] {
            log::info!("  tab0{} (5 frames): {}x{}", tab, entry.width, entry.height);
        }
    }
```

**Step 6: Update the atlas struct construction.** Find the `SidebarChromeAtlas { ... }` literal near the bottom of the loader and replace:

```rust
        tab_buttons: tab_button_uvs,
        tab_buttons_active: tab_button_active_uvs,
        ...
        repair: repair_uv,
        sell: sell_uv,
```

with:

```rust
        tab_frames: tab_frames_packed,
        ...
        repair_frames: repair_frames_packed,
        sell_frames: sell_frames_packed,
```

(Field order matches the struct declaration from Step 1; verify by reading the
file before saving.)

**Step 7: Update the consumer in `app_sidebar_render.rs:80`.** Find:

```rust
    let tab_btn_size = current_sidebar_chrome(state)
        .and_then(|atlas| atlas.tab_buttons.first())
        .map(|entry| {
```

Replace with:

```rust
    let tab_btn_size = current_sidebar_chrome(state)
        .and_then(|atlas| atlas.tab_frames[0][0].as_ref())
        .map(|entry| {
```

**Step 8: Wire the Repair/Sell button-size extraction now that the atlas fields exist.**

Task 6 Step 6 deferred this — it passed `None, None` for the two button-size args at both `build_sidebar_view_with_spec(...)` call sites because `atlas.repair_frames` / `atlas.sell_frames` didn't exist yet. Now that Step 1 of this task has added them, replace the `None, None` placeholders with the real atlas-derived sizes.

First, extract Repair/Sell button sizes from the atlas alongside the existing `tab_btn_size` extraction. Add directly after the `tab_btn_size` let binding (now at ~line 79-86 + a few from Step 7's edit):

```rust
    let repair_btn_size = current_sidebar_chrome(state)
        .and_then(|atlas| atlas.repair_frames[0].as_ref())
        .map(|entry| {
            [
                entry.pixel_size[0] * state.ui_scale,
                entry.pixel_size[1] * state.ui_scale,
            ]
        });
    let sell_btn_size = current_sidebar_chrome(state)
        .and_then(|atlas| atlas.sell_frames[0].as_ref())
        .map(|entry| {
            [
                entry.pixel_size[0] * state.ui_scale,
                entry.pixel_size[1] * state.ui_scale,
            ]
        });
```

Then update both `build_sidebar_view_with_spec(...)` call sites to forward the real sizes instead of `None, None`. Find:

```rust
        ...,
        &state.sidebar_gadget_state,
        None,    // repair_button_size — wired in Task 9 once atlas.repair_frames exists.
        None,    // sell_button_size — same.
```

Replace at both call sites with:

```rust
        ...,
        &state.sidebar_gadget_state,
        repair_btn_size,
        sell_btn_size,
```

**Step 9: Verify.**

```
cargo check -p ra2-rust-game
```

Expected: errors at `src/app_sidebar_build.rs` consumers (`atlas.tab_buttons`, `atlas.tab_buttons_active`, `atlas.repair`, `atlas.sell`). **That is intentional** — Task 10 fixes those. Confirm those are the only errors before moving on. The new `repair_btn_size` / `sell_btn_size` extractions compile clean because Step 1 already added the atlas fields they depend on.

**Step 10: Hold the commit.** Hold this change uncommitted; Task 10 lands the consumer fix in the same commit so the tree never compiles with mismatched field names.

---

### Task 10: Render — pick tab/repair/sell frames from view's frame_index

**Why:** Switches `app_sidebar_build.rs` from the old `tab_buttons` / `tab_buttons_active` lookup to the new `tab_frames[tab_idx][frame_index]` lookup, and uncomments + fixes the Repair/Sell render to consume the new `repair_frames` / `sell_frames` arrays via `view.repair_button.frame_index` / `view.sell_button.frame_index`.

**Files:**
- Modify: `src/app_sidebar_build.rs:134-152` (tab render block) and `:170-194` (commented Repair/Sell block).

**Pattern:** Direct atlas index — same shape as the existing `powerp_frames[frame as usize]` lookup elsewhere in the file.

**Step 1: Replace the tab render block.** Find lines 134-152:

```rust
    for tab_btn in tabs {
        let idx = tab_btn.tab.tab_index();
        let entry = if tab_btn.active {
            atlas.tab_buttons_active.get(idx).copied()
        } else {
            atlas.tab_buttons.get(idx).copied()
        };
        if let Some(e) = entry {
            push_chrome(
                &mut inst,
                e,
                tab_btn.rect.x,
                tab_btn.rect.y,
                td,
                camera_offset,
                s,
            );
        }
    }
```

Replace with:

```rust
    for tab_btn in tabs {
        let idx = tab_btn.tab.tab_index();
        let frame = tab_btn.frame_index as usize;
        // Fall back to frame 0 if the requested frame is missing in MIX.
        let entry = atlas.tab_frames[idx][frame]
            .or(atlas.tab_frames[idx][0]);
        if let Some(e) = entry {
            push_chrome(
                &mut inst,
                e,
                tab_btn.rect.x,
                tab_btn.rect.y,
                td,
                camera_offset,
                s,
            );
        }
    }
```

**Step 2: Replace the commented-out Repair/Sell block** at lines 169-193:

Delete the entire block:

```rust
    // --- Sell / Repair buttons (inside the side1 area) ---
    // TODO: these use wrong palette (sidebar.pal instead of OBSERVER.PAL) — disabled until fixed
    let _btn_depth = d - 0.00002;
    // if let Some(sell) = atlas.sell {
    //     push_chrome(
    //         &mut inst,
    //         sell,
    //         cx + spec.sell_x,
    //         layout.side1_y + spec.sell_y,
    //         _btn_depth,
    //         camera_offset,
    //         s,
    //     );
    // }
    // if let Some(repair) = atlas.repair {
    //     push_chrome(
    //         &mut inst,
    //         repair,
    //         cx + spec.repair_x,
    //         layout.side1_y + spec.repair_y,
    //         _btn_depth,
    //         camera_offset,
    //         s,
    //     );
    // }
```

Replace with:

```rust
    // --- Sell / Repair buttons (inside the side1 area). ---
    // Palette is SIDEBAR.PAL via DAT_0087f6cc — already wired through `palette`
    // in sidebar_chrome.rs. The 5-frame state machine matches gamemd's
    // SBGadgetClass::Draw conditional; the frame index is computed by
    // SidebarGadgetState::repair_frame / sell_frame.
    let btn_depth = d - 0.00002;
    let sell_frame = view.sell_button.frame_index as usize;
    if let Some(sell) = atlas.sell_frames[sell_frame].or(atlas.sell_frames[0]) {
        push_chrome(
            &mut inst,
            sell,
            view.sell_button.rect.x,
            view.sell_button.rect.y,
            btn_depth,
            camera_offset,
            s,
        );
    }
    let repair_frame = view.repair_button.frame_index as usize;
    if let Some(repair) = atlas.repair_frames[repair_frame].or(atlas.repair_frames[0]) {
        push_chrome(
            &mut inst,
            repair,
            view.repair_button.rect.x,
            view.repair_button.rect.y,
            btn_depth,
            camera_offset,
            s,
        );
    }
```

Note: the new code consumes `view.repair_button.rect` directly instead of computing the position from `spec.repair_x + cx`. This is intentional — the view builder is the single source of truth for button position.

**Step 3: Verify.**

```
cargo check -p ra2-rust-game
cargo test -p ra2-rust-game
```

Expected: clean compile. All existing tests pass. Sidebar view tests pass unchanged.

**Step 4: Commit (this is the Task 9 + Task 10 combined commit).**

```
git commit -am "render/sidebar_chrome: load 5-frame tab/repair/sell + render by frame_index"
```

---

### Task 11: Hit-test routes Repair/Sell rects

**Why:** Without this, clicking the (now-visible) Repair/Sell buttons does nothing.

**Files:**
- Modify: `src/sidebar/mod.rs:348-387` (the `hit_test` function).

**Pattern:** Same shape as the existing `view.pause_button.as_ref()` / `view.cancel_button` rect checks below.

**Step 1: Add Repair/Sell rect checks** in `hit_test`. Find the function at line 348. After the `for tab in &view.tabs` block (line 357) and before `for item in &view.items` (line 359), add:

```rust
    if view.repair_button.rect.contains(x, y) {
        return view.repair_button.action.clone();
    }
    if view.sell_button.rect.contains(x, y) {
        return view.sell_button.action.clone();
    }
```

(Order matters: Repair/Sell are positioned above the cameo grid in the side1 area, so they should be tested before cameo items. Placing them between the tab loop and the item loop matches their on-screen vertical position above the tabs in retail — Repair/Sell sit in the radar/side1 area, tabs sit between side1 and the cameo strip.)

**Step 2: Verify.**

```
cargo check -p ra2-rust-game
cargo test -p ra2-rust-game sidebar::
```

Expected: clean compile, existing tests pass.

**Step 3: Add a hit-test unit test** to `src/sidebar/mod.rs` (`mod tests` near line 419):

```rust
    #[test]
    fn hit_test_routes_repair_button() {
        let view = super::sidebar_view::build_sidebar_view(
            1280.0,
            960.0,
            super::SidebarTab::Building,
            0,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &crate::sidebar::gadget_flash::SidebarGadgetState::new(),
            // Provide non-zero button sizes so the rects are clickable.
            // Real values come from the atlas at run time; unit tests use
            // the SHP intrinsic 64×31 (from sidebar_chrome.rs header).
            Some([64.0, 31.0]),
            Some([64.0, 31.0]),
        );
        let action = super::hit_test(
            &view,
            view.repair_button.rect.x + 1.0,
            view.repair_button.rect.y + 1.0,
            false,
        );
        assert_eq!(action, super::SidebarAction::ToggleRepairMode);
    }

    #[test]
    fn hit_test_routes_sell_button() {
        let view = super::sidebar_view::build_sidebar_view(
            1280.0,
            960.0,
            super::SidebarTab::Building,
            0,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &crate::sidebar::gadget_flash::SidebarGadgetState::new(),
            Some([64.0, 31.0]),
            Some([64.0, 31.0]),
        );
        let action = super::hit_test(
            &view,
            view.sell_button.rect.x + 1.0,
            view.sell_button.rect.y + 1.0,
            false,
        );
        assert_eq!(action, super::SidebarAction::ToggleSellMode);
    }
```

If `build_sidebar_view` no longer exists (Task 6 may have deleted the convenience wrapper), use `build_sidebar_view_with_spec(SidebarChromeLayoutSpec::stock(), …)` directly.

**Step 4: Verify.**

```
cargo test -p ra2-rust-game sidebar::tests
```

Expected: both new tests pass.

**Step 5: Commit.**

```
git commit -am "sidebar: route Repair/Sell hit-test to the new toggle actions"
```

---

### Task 12: `apply_sidebar_action` handlers + mutual-exclusion edits

**Why:** Connect the click → state mutation. Also clear repair/sell when arming any of the existing targeting modes (mutual exclusion).

**Files:**
- Modify: `src/app_input.rs` — extend `apply_sidebar_action` with two new arms; edit the three existing `Arm*` handlers + `ClearSuperWeaponMode` / `ClearPlacementMode` to clear the repair/sell flags.

**Pattern:** Direct mutation of `state.sidebar_gadget_state` + clearing of other fields. Mirrors the existing `BuildingPlacement` arm which also clears `building_placement_preview`.

**Step 1: Add the two new arms.** Find the `match action` block in `apply_sidebar_action` (around line 241). Add two new arms before the final `SidebarAction::Deploy` arm:

```rust
        SidebarAction::ToggleRepairMode => {
            let g = &mut state.sidebar_gadget_state;
            g.repair_mode_on = !g.repair_mode_on;
            if g.repair_mode_on {
                g.sell_mode_on = false;
                state.targeting_mode = None;
                state.building_placement_preview = None;
            }
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

**Step 2: Add the mutex line to `ArmPlacement`.** Find:

```rust
        SidebarAction::ArmPlacement(type_id) => {
            state.targeting_mode =
                Some(crate::app_types::TargetingMode::BuildingPlacement(type_id));
        }
```

Change to:

```rust
        SidebarAction::ArmPlacement(type_id) => {
            state.targeting_mode =
                Some(crate::app_types::TargetingMode::BuildingPlacement(type_id));
            state.sidebar_gadget_state.repair_mode_on = false;
            state.sidebar_gadget_state.sell_mode_on = false;
        }
```

**Step 3: Add the mutex line to `ArmSuperWeapon`.** Find:

```rust
        SidebarAction::ArmSuperWeapon(section) => {
            state.targeting_mode = Some(crate::app_types::TargetingMode::SuperWeapon(section));
            // Mutual exclusion: clear any pending building-placement preview.
            state.building_placement_preview = None;
            log::info!(...);
        }
```

Change to:

```rust
        SidebarAction::ArmSuperWeapon(section) => {
            state.targeting_mode = Some(crate::app_types::TargetingMode::SuperWeapon(section));
            // Mutual exclusion: clear building-placement preview AND repair/sell modes.
            state.building_placement_preview = None;
            state.sidebar_gadget_state.repair_mode_on = false;
            state.sidebar_gadget_state.sell_mode_on = false;
            log::info!(
                "SuperWeapon armed: type={}",
                state.armed_super_weapon_type().unwrap_or("")
            );
        }
```

**Step 4: Verify.**

```
cargo check -p ra2-rust-game
```

Expected: clean compile. Existing `ClearPlacementMode` and `ClearSuperWeaponMode` already clear `targeting_mode`; they intentionally do NOT clear repair/sell because clearing one cursor mode shouldn't disable another.

**Step 5: Commit.**

```
git commit -am "app_input: handle ToggleRepair/Sell actions + clear modes on Arm*"
```

---

### Task 13: Documentation update — fix the stale "tab_buttons_active = frame 3" comment

**Why:** Incidental cleanup discovered during the refactor. The header comment at `src/render/sidebar_chrome.rs:9` says "tab00-03.shp (28x27 each, 5 frames)" (correct) and `tab_buttons_active` was documented at line 82-83 as "frame 3" but actually loaded frame 1. The old fields are now gone (Task 9 replaced them with `tab_frames`), so this is a verification step that no stale references remain.

**Files:**
- Verify only: `src/render/sidebar_chrome.rs`

**Step 1:** Grep for stale references:

```
grep -rn "tab_buttons_active\|tab_buttons\b" src/
```

Expected: zero hits. If any remain, fix them now and re-run before continuing.

**Step 2:** Grep for stale doc comments mentioning frame 3 as the "active" frame:

```
grep -rn "frame 3.*active\|active.*frame 3" src/
```

Expected: zero hits.

**Step 3:** No commit needed if both greps return clean. If anything required a fix, commit:

```
git commit -am "render/sidebar_chrome: clean up stale tab_buttons references"
```

---

### Task 14: SW-ready predicate sanity check

**Why:** Confirm the `is_ready` semantic on `SuperWeaponView` is "charged and waiting for the player to fire" — the actual gamemd Tab 1 trigger.

**Files:**
- Verify only: `src/sim/superweapon/mod.rs`.

**Step 1: Read `SuperWeaponView.is_ready` at line 139 and trace where it's set.**

```
grep -n "is_ready" src/sim/superweapon/
```

Expected: `is_ready` is true exactly when the SW's charge timer has reached 0 AND the SW has not been fired yet (i.e. it's available to fire on the player's next click). It must NOT be true while still charging, and must go false the moment the player fires.

**Step 2: If the semantic doesn't match,** the SW flash predicate must use a different field. Update `has_charged_sw_for_owner` in `src/app_sidebar_gadgets.rs` accordingly. If it matches, no change.

**Step 3: Commit if implementation changed.**

```
git commit -am "app_sidebar_gadgets: verify + document SW-ready predicate"
```

If no implementation change needed, no commit.

---

### Task 15: Orchestrator integration tests

**Why:** Unit-test the trigger evaluation, sim-tick-delta loop, and tab gating without a full simulation.

**Files:**
- Modify: `src/app_sidebar_gadgets.rs` (extend the `#[cfg(test)]` block from Task 7).

**Pattern:** Pure-function tests on `SidebarGadgetState` directly — the orchestrator's effects on the state struct are observable without spinning up a `Simulation`. The trigger-source predicates (which DO need `Simulation`) are NOT unit-tested here; their behavior is exercised in Task 16's in-game verification.

**Step 1: Add tests that drive `SidebarGadgetState` directly.**

Append inside the existing `#[cfg(test)] mod tests` in `src/app_sidebar_gadgets.rs`:

```rust
    use crate::sidebar::gadget_flash::SidebarGadgetState;
    use crate::sidebar::SidebarTab;

    /// Helper: simulate one orchestrator pass on a bare SidebarGadgetState
    /// without going through the AppState / Simulation indirection. Mirrors
    /// the trigger-driven body of update_sidebar_gadget_state.
    fn orchestrate(
        gadgets: &mut SidebarGadgetState,
        sim_tick: u64,
        sw_ready: bool,
    ) {
        let frame = sim_tick;
        let extra_delay: u32 = (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS))
            % FLASH_PERIOD_TICKS;
        let next_boundary = (extra_delay as u64 + frame) / FLASH_PERIOD_TICKS as u64;
        let initial_state: u8 = if next_boundary & 1 == 0 { 1 } else { 0 };

        gadgets.tab_flashes[SidebarTab::Building.tab_index()].stop();
        if sw_ready {
            gadgets.tab_flashes[SidebarTab::Defense.tab_index()]
                .start(FLASH_PERIOD_TICKS, extra_delay, initial_state);
        } else {
            gadgets.tab_flashes[SidebarTab::Defense.tab_index()].stop();
        }
        gadgets.tab_flashes[SidebarTab::Infantry.tab_index()].stop();
        gadgets.tab_flashes[SidebarTab::Vehicle.tab_index()].stop();

        let last = gadgets.last_sim_tick;
        let delta = frame.saturating_sub(last);
        for _ in 0..delta {
            for f in &mut gadgets.tab_flashes {
                f.tick();
            }
        }
        gadgets.last_sim_tick = frame;
    }

    #[test]
    fn sw_ready_starts_defense_tab_flash() {
        let mut g = SidebarGadgetState::new();
        orchestrate(&mut g, 0, true);
        let def = &g.tab_flashes[SidebarTab::Defense.tab_index()];
        assert!(def.is_active(), "defense tab should flash on SW ready");
        assert_eq!(def.period, 10);
        // At frame 0: extra_delay=0, so first countdown is 10.
        assert_eq!(def.countdown, 10);
    }

    #[test]
    fn other_three_tabs_never_flash() {
        let mut g = SidebarGadgetState::new();
        orchestrate(&mut g, 0, true);
        assert!(!g.tab_flashes[SidebarTab::Building.tab_index()].is_active());
        assert!(!g.tab_flashes[SidebarTab::Infantry.tab_index()].is_active());
        assert!(!g.tab_flashes[SidebarTab::Vehicle.tab_index()].is_active(),
            "Vehicle deferred until aircraft-waiting sim state exists");
    }

    #[test]
    fn auto_stop_on_condition_clear() {
        let mut g = SidebarGadgetState::new();
        orchestrate(&mut g, 0, true);
        assert!(g.tab_flashes[SidebarTab::Defense.tab_index()].is_active());
        // Advance 15 ticks with SW still ready → flash keeps ticking.
        orchestrate(&mut g, 15, true);
        assert!(g.tab_flashes[SidebarTab::Defense.tab_index()].is_active());
        // Player fires the SW → predicate false → Stop_Flash.
        orchestrate(&mut g, 16, false);
        assert!(!g.tab_flashes[SidebarTab::Defense.tab_index()].is_active());
        assert_eq!(g.tab_flashes[SidebarTab::Defense.tab_index()].period, 0);
    }

    #[test]
    fn repeat_start_during_active_does_not_resync_phase() {
        // Verifies the Start_Flash guard — multiple poll passes during an
        // active flash must not restart its countdown.
        let mut g = SidebarGadgetState::new();
        orchestrate(&mut g, 0, true);
        let def_after_first = g.tab_flashes[SidebarTab::Defense.tab_index()];
        // Tick 3 forward. Each call re-fires Start (predicate still true), which
        // must be a no-op because period != 0.
        orchestrate(&mut g, 1, true);
        orchestrate(&mut g, 2, true);
        orchestrate(&mut g, 3, true);
        let def_after_three = g.tab_flashes[SidebarTab::Defense.tab_index()];
        // Countdown should have decremented by 3, not reset.
        assert_eq!(def_after_three.countdown, def_after_first.countdown - 3);
        assert_eq!(def_after_three.period, def_after_first.period);
        assert_eq!(def_after_three.state, def_after_first.state);
    }

    #[test]
    fn sim_tick_delta_loop_iterates_correctly_under_catchup() {
        // Single orchestrator pass that jumps from tick 0 to tick 30 — Flash_AI
        // should iterate 30 times total and end at the correct phase.
        let mut g = SidebarGadgetState::new();
        // Start at frame 0: extra_delay=0, period=10, initial_state=1.
        // tick() called 0 times during start.
        orchestrate(&mut g, 0, true);
        let def = g.tab_flashes[SidebarTab::Defense.tab_index()];
        assert_eq!(def.state, 1);
        // Jump 30 ticks. Sequence:
        //   countdown: 10→1 (10 ticks), toggle, state=0, reset to 10.
        //   countdown: 10→1 (10 ticks), toggle, state=1, reset to 10.
        //   countdown: 10→1 (10 ticks), toggle, state=0, reset to 10.
        // → 30 ticks = 3 toggles → state = 0.
        orchestrate(&mut g, 30, true);
        let def = g.tab_flashes[SidebarTab::Defense.tab_index()];
        assert_eq!(def.state, 0, "after 3 toggles, state is back to 0");
        assert_eq!(def.countdown, 10, "countdown reset to period");
        assert!(def.is_active());
    }
```

**Step 2: Verify.**

```
cargo test -p ra2-rust-game app_sidebar_gadgets::
```

Expected: all 7 tests pass (1 from Task 7 + 6 new).

**Step 3: Commit.**

```
git commit -am "app_sidebar_gadgets: add orchestrator unit tests (trigger / mutex / phase)"
```

---

### Task 16: End-to-end in-game verification against gamemd

(Scope: Repair/Sell rendering + tab pressed/active frames + Defense-tab flash on
SW ready. Vehicle-tab flash on aircraft complete is intentionally NOT in this
verification — deferred per the plan's Scope note.)

**Why:** Confirm observable parity against retail. This is the parity-critical gate that test suites can't replace.

**Files:**
- No code changes. Observation only.

**Step 1: Run the game and verify the static states.**

```
cargo run --release
```

Start a skirmish as any faction. Confirm:

1. **Repair and Sell buttons appear in the side1 area** (above the tab strip).
   Both render in their idle (frame 0) state with no visual glitches.
   **Verification:** gamemd shows them in the same position with the same SHP
   sprite.

2. **Click the Repair button.** The button visibly transitions to its "active"
   visual (frame 1 — typically a darker / pressed-look variant). The mode
   internally is on, but the cursor does not yet change and clicking a
   building does nothing (this is the deferred follow-up).

3. **Click Repair again.** Button returns to idle (frame 0).

4. **Click Sell.** Button transitions to active. Repair is unaffected by this
   single click.

5. **Click Repair while Sell is active.** Sell returns to idle, Repair turns
   active. (Mutual exclusion.)

6. **Click on a buildable cameo to arm building placement.** Both Repair and
   Sell (whichever was active) return to idle. (TargetingMode wins.)

**Step 2: Verify tab pressed-state frames (idle path).**

7. **Hover and click on a tab.** The selected tab's visual matches gamemd
   frame 1. Other tabs match frame 0. Note: there is no "hold-down to see
   frame 3" gesture in the current input model — tabs are clicked, not
   held. So frames 3 and 4 are only visible during a flash cycle (next
   step). This matches retail.

**Step 3: Verify Defense tab flash on superweapon ready.**

8. Build a superweapon (Iron Curtain / Chronosphere / etc.). Wait for it to
   charge. Once `is_ready` becomes true:
   - The **Defense tab** alternates between frame 0/1 (idle/active depending
     on whether Defense is the currently-selected tab) and frame 3/4
     (pressed-look).
   - The cadence is **roughly 667ms per half-cycle** at game speed 1
     (~15Hz × 10 ticks).
   - Firing the SW stops the flash within ~1 tick (next poll finds no
     ready SW).

**Step 4: Verify the other three tabs never flash.**

9. Trigger conditions that would flash Building/Infantry tabs (none —
   they never flash in gamemd either). The **Vehicle tab also never
   flashes in this bundle** (deferred). Queuing and completing aircraft
   produces no visible Vehicle-tab pulse. This is intentional and
   documented in the plan's Scope note.

**Step 5: Verify pause behavior.**

10. While the Defense tab is flashing, pause the game (ESC menu). Confirm
    the **flash continues** during pause. Matches retail per
    SIDEBAR_TAB_FLASH_SCHEDULER §11.6.

**Step 6: Document the verification result.**

If all 10 checks pass, the implementation matches retail for the in-scope
items. If any check fails:
- **Static button visual wrong:** suspect the atlas load step (Task 9) or
  the frame_index lookup (Task 10).
- **Mode flag wrong / button doesn't stay pressed:** suspect handler
  (Task 12) or mutual exclusion.
- **Flash cadence wrong:** suspect the sim-tick-delta loop (Task 7) or
  the period constant.
- **Flash never starts on SW ready:** suspect the predicate in
  `has_charged_sw_for_owner` (Task 14 verification).
- **Flash never stops after fire:** suspect `is_ready` returning true
  after fire (Task 14 verification).

For any failure, capture a screenshot, diagnose, fix, and re-verify before
considering the plan complete.

**Step 7: Final commit (if any fixes landed in this step).**

```
git commit -am "sidebar: post-verification fixes"
```

If no fixes needed, no commit.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-20-sidebar-tab-flash-and-repair-sell-design.md](2026-05-20-sidebar-tab-flash-and-repair-sell-design.md)
- **Ghidra reports (all GREEN, verified 2026-05-20):**
  - `ra2-rust-game-docs/SIDEBAR_REPAIR_SELL_BUTTON_GHIDRA_REPORT.md` — palette, position, 5-frame state table, click handlers
  - `ra2-rust-game-docs/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md` — flash primitive semantics, trigger conditions, phase math
  - `ra2-rust-game-docs/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md` — cross-reference for tab flash + SidebarClass+0x5394/+0x5398 (dormant)
  - `ra2-rust-game-docs/SIDEBAR_INIT_GADGET_POSITIONING_GHIDRA_REPORT.md` — gadget X/Y/+0x40 init
- **Key gamemd.exe addresses:**
  - `FUN_0069DFC0` (Start_Flash), `FUN_0069DFF0` (Stop_Flash), `FUN_0069E010` (Flash_AI)
  - `SBGadgetClass::Draw` at `0x0069DEB0` — 5-frame state-select source
  - `StripClass::AI` at `0x006A8B30` — poll + trigger source
  - `SidebarClass::Action` at `0x006A7780` — per-tick Flash_AI driver
  - `FUN_004AC8C0` / `FUN_004AC660` — Repair/Sell mode toggles
- **INI keys:** None (all constants are binary literals).
- **Related code:**
  - `src/sidebar/power_bar_anim.rs` — pattern this design mirrors
  - `src/app_building_anim.rs:486-514` — sim-tick driver pattern
  - `src/sim/production/production_queue.rs:605-653` — `queue_view_for_owner` accessor
  - `src/sim/superweapon` — `superweapon_views_for_owner` accessor
  - `src/render/sidebar_chrome.rs:91` — `[Option<…>; 5]` array atlas pattern
- **Disparity scan tracking:** [docs/gap-scans/2026-05-20-disparity-scan-sidebar.md](../gap-scans/2026-05-20-disparity-scan-sidebar.md) — G1, A20

---

## Deferred Follow-ups (intentionally NOT in this plan)

- **Vehicle-tab flash on aircraft completion** (was originally in-scope; removed
  during /review-plan 2026-05-20). Requires a Rust sim-side representation of
  "aircraft finished, waiting for the player's command" — analogous to
  `ready_by_owner` for buildings, or a "local owner has any idle docked
  aircraft entity" accessor. Once that semantic lands, the orchestrator gets
  one new predicate + one new Start/Stop branch on `tab_flashes[Vehicle]`.
  All the supporting infrastructure (`GadgetFlash`, `SidebarGadgetState`,
  per-tab atlas frames, render path) is built and unused for this tab.
- **Cursor SHP swap when Repair/Sell mode is on** (deferred per Q1).
- **Click-target-on-tactical-map resolution** — click building → repair/sell command (deferred per Q1).
- **Theater-driven Repair/Sell positions** (disparity scan A23, LOW).
- **Disable-state wiring** for tabs / Repair / Sell (currently hardcoded `false`).
- **Click voc playback** on Repair/Sell/Tab clicks (deferred to audio brainstorm).
- **Cameo flash (G5 / A20-companion)** — can plug into the same `GadgetFlash` primitive when shipped.
