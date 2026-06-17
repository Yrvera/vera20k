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
//! `GadgetFlash::tick` by `sim.session.tick - last_sim_tick` per call so the period
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
    let frame = sim.session.tick;
    let extra_delay: u32 =
        (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS)) % FLASH_PERIOD_TICKS;
    let next_boundary = (extra_delay as u64 + frame) / FLASH_PERIOD_TICKS as u64;
    let initial_state: u8 = if next_boundary & 1 == 0 { 1 } else { 0 };

    // --- Step 3: drive Start/Stop on each tab. ---
    let gadgets = &mut state.sidebar_gadget_state;
    // Building (idx 0) — never flashes in retail.
    gadgets.tab_flashes[SidebarTab::Building.tab_index()].stop();
    // Defense (idx 1) — flashes on any SW ready.
    if sw_ready {
        gadgets.tab_flashes[SidebarTab::Defense.tab_index()].start(
            FLASH_PERIOD_TICKS,
            extra_delay,
            initial_state,
        );
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
    if !sim.session.game_options.super_weapons {
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
    use crate::sidebar::gadget_flash::SidebarGadgetState;

    /// Helper: simulate one orchestrator pass on a bare SidebarGadgetState
    /// without going through the AppState / Simulation indirection. Mirrors
    /// the trigger-driven body of update_sidebar_gadget_state.
    fn orchestrate(gadgets: &mut SidebarGadgetState, sim_tick: u64, sw_ready: bool) {
        let frame = sim_tick;
        let extra_delay: u32 =
            (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS)) % FLASH_PERIOD_TICKS;
        let next_boundary = (extra_delay as u64 + frame) / FLASH_PERIOD_TICKS as u64;
        let initial_state: u8 = if next_boundary & 1 == 0 { 1 } else { 0 };

        gadgets.tab_flashes[SidebarTab::Building.tab_index()].stop();
        if sw_ready {
            gadgets.tab_flashes[SidebarTab::Defense.tab_index()].start(
                FLASH_PERIOD_TICKS,
                extra_delay,
                initial_state,
            );
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
        assert!(
            !g.tab_flashes[SidebarTab::Vehicle.tab_index()].is_active(),
            "Vehicle deferred until aircraft-waiting sim state exists"
        );
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

    /// Verify the phase math literal-for-literal against gamemd.
    /// From SIDEBAR_TAB_FLASH_SCHEDULER §4.1: at frame F, extra_delay =
    /// (10 - F % 10) % 10. At frame 0 extra_delay = 0; at frame 9 extra_delay = 1.
    #[test]
    fn phase_math_examples() {
        let cases: &[(u64, u32, u8)] = &[
            // (frame, expected_extra_delay, expected_initial_state)
            (0, 0, 1),  // next_boundary = 0; 0 & 1 == 0 → 1
            (5, 5, 0),  // next_boundary = (5+5)/10 = 1; 1 & 1 == 1 → 0
            (9, 1, 0),  // next_boundary = (1+9)/10 = 1 → 0
            (10, 0, 0), // next_boundary = 10/10 = 1 → 0
            (15, 5, 1), // next_boundary = (5+15)/10 = 2; 2 & 1 == 0 → 1
            (20, 0, 1), // next_boundary = 20/10 = 2 → 1
        ];
        for &(frame, expected_extra, expected_state) in cases {
            let extra =
                (FLASH_PERIOD_TICKS - (frame as u32 % FLASH_PERIOD_TICKS)) % FLASH_PERIOD_TICKS;
            let nb = (extra as u64 + frame) / FLASH_PERIOD_TICKS as u64;
            let st: u8 = if nb & 1 == 0 { 1 } else { 0 };
            assert_eq!(extra, expected_extra, "extra_delay at frame {frame}");
            assert_eq!(st, expected_state, "initial_state at frame {frame}");
        }
    }
}
