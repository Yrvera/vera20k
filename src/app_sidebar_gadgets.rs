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
            (0, 0, 1),  // next_boundary = 0; 0 & 1 == 0 → 1
            (5, 5, 0),  // next_boundary = (5+5)/10 = 1; 1 & 1 == 1 → 0
            (9, 1, 0),  // next_boundary = (1+9)/10 = 1 → 0
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
