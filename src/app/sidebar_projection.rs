//! Retained, app-owned projection for the in-game sidebar.
//!
//! Presentation consumers only read `current_view`. Mutations happen at the
//! explicit simulation, input, resize, and match-replacement transitions that
//! rebuild the projection.

use std::collections::{BTreeSet, HashMap};

use crate::sidebar::SidebarView;
use crate::sim::intern::InternedId;
use crate::sim::world::TickLane;

#[derive(Debug, Default)]
pub(crate) struct SidebarProjectionState {
    displayed_credits: HashMap<String, i32>,
    current_view: Option<SidebarView>,
    /// The build cameos the strips held at the last projection — the
    /// `StripClass` entry list `SidebarClass::AddCameo @ 0x006A63E0` scans
    /// before inserting. `None` until the first projection of a match, which
    /// is the scenario-init window (`[0xA8E7AC] != 0`) where insertions are
    /// silent.
    cameo_types: Option<BTreeSet<InternedId>>,
}

impl SidebarProjectionState {
    /// Read the retained view without changing credit cadence, targeting, or
    /// scroll state.
    pub(crate) fn view(&self) -> Option<&SidebarView> {
        self.current_view.as_ref()
    }

    pub(crate) fn replace_view(&mut self, view: Option<SidebarView>) {
        if view.is_none() {
            self.cameo_types = None;
        }
        self.current_view = view;
    }

    /// Forget the strip entry list: the next `note_cameos` is a scenario's
    /// first projection again (`[0xA8E7AC] != 0`, silent). Called wherever a
    /// simulation is installed — a new match or an in-scenario load — so a
    /// different pre-placed base never reads as freshly inserted cameos.
    pub(crate) fn reset_cameo_seed(&mut self) {
        self.cameo_types = None;
    }

    /// Record the build cameos now shown and report whether at least one was
    /// inserted since the previous projection
    /// (`crate::app::input::sidebar_eva::new_construction_options`).
    pub(crate) fn note_cameos(&mut self, current: BTreeSet<InternedId>) -> bool {
        let (set, inserted) = crate::app::input::sidebar_eva::new_construction_options(
            self.cameo_types.as_ref(),
            current,
        );
        self.cameo_types = Some(set);
        inserted
    }

    /// Return the owner's retained display value, seeding a newly observed
    /// owner from the actual balance without animating during projection build.
    pub(crate) fn displayed_credits_or_seed(&mut self, owner: &str, actual: i32) -> i32 {
        *self
            .displayed_credits
            .entry(owner.to_string())
            .or_insert(actual)
    }

    /// Advance one native CreditsClass AI step for an already observed owner.
    /// A newly observed owner starts at the actual balance, matching the former
    /// first-view behavior without making a view read mutate state.
    pub(crate) fn advance_credits(&mut self, owner: &str, actual: i32) {
        use std::collections::hash_map::Entry;

        let mut entry = match self.displayed_credits.entry(owner.to_string()) {
            Entry::Vacant(entry) => {
                entry.insert(actual);
                return;
            }
            Entry::Occupied(entry) => entry,
        };
        let displayed = entry.get_mut();
        if *displayed == actual {
            return;
        }

        // gamemd `CreditsClass::AI / FUN_004A2600 @ 0x004A2600`:
        // |actual-displayed| / 8, clamped to [1, 143]. The stored 1/3 value
        // does not delay this call's step (SIDEBAR_SYSTEM_GHIDRA_REPORT §30).
        let difference = (i64::from(actual) - i64::from(*displayed)).unsigned_abs();
        let step = (difference / 8).clamp(1, 143) as i32;
        if actual > *displayed {
            *displayed = displayed.saturating_add(step).min(actual);
        } else {
            *displayed = displayed.saturating_sub(step).max(actual);
        }
    }

    #[cfg(test)]
    fn displayed_credits_for_test(&self, owner: &str) -> Option<i32> {
        self.displayed_credits.get(owner).copied()
    }
}

pub(crate) fn credits_advance_for_frame(frame_committed: bool, tick_lane: TickLane) -> bool {
    frame_committed && tick_lane == TickLane::Ordinary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidebar::gadget_flash::SidebarGadgetState;
    use crate::sidebar::{SidebarTab, build_sidebar_view};

    fn retained_test_view(credits: i32) -> SidebarView {
        build_sidebar_view(
            800.0,
            600.0,
            SidebarTab::Building,
            credits,
            100,
            80,
            None,
            &[],
            &[],
            &[],
            None,
            &[],
            99,
            None,
            &SidebarGadgetState::new(),
            None,
            None,
        )
    }

    #[test]
    fn sidebar_view_reads_are_pure() {
        let _pure_getter: for<'a> fn(&'a crate::app::AppState) -> Option<&'a SidebarView> =
            crate::app::presentation::sidebar_render::current_sidebar_view;
        let mut projection = SidebarProjectionState::default();
        assert_eq!(projection.displayed_credits_or_seed("Americans", 100), 100);
        projection.replace_view(Some(retained_test_view(100)));

        let first = projection.view().expect("retained view");
        let first_ptr = std::ptr::from_ref(first);
        let first_credits = first.credits;
        let first_scroll = first.scroll_rows;
        for _ in 0..8 {
            let read = projection.view().expect("retained view remains present");
            assert_eq!(std::ptr::from_ref(read), first_ptr);
            assert_eq!(read.credits, first_credits);
            assert_eq!(read.scroll_rows, first_scroll);
        }
        assert_eq!(
            projection.displayed_credits_for_test("Americans"),
            Some(100)
        );
    }

    #[test]
    fn installing_another_sim_reseeds_the_cameo_strip_silently() {
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let mut projection = SidebarProjectionState::default();
        // Sim A: the first projection seeds silently, growth speaks.
        assert!(!projection.note_cameos(BTreeSet::from([a])));
        assert!(projection.note_cameos(BTreeSet::from([a, b])));
        // Sim B with a different pre-placed base: without the reset its
        // first refresh would read `b`-less/`a`-less strips as insertions.
        projection.reset_cameo_seed();
        assert!(
            !projection.note_cameos(BTreeSet::from([b])),
            "the first refresh after a sim install is the init window"
        );
        assert!(projection.note_cameos(BTreeSet::from([a, b])));
        // Leaving the match still clears the seed.
        projection.replace_view(None);
        assert!(!projection.note_cameos(BTreeSet::from([a])));
    }

    #[test]
    fn credits_advance_once_per_committed_ordinary_frame() {
        let mut projection = SidebarProjectionState::default();
        projection.displayed_credits_or_seed("Americans", 100);

        if credits_advance_for_frame(true, TickLane::Ordinary) {
            projection.advance_credits("Americans", 900);
        }
        assert_eq!(
            projection.displayed_credits_for_test("Americans"),
            Some(200)
        );
        for _ in 0..6 {
            let _ = projection.view();
        }
        assert_eq!(
            projection.displayed_credits_for_test("Americans"),
            Some(200)
        );

        if credits_advance_for_frame(true, TickLane::Ordinary) {
            projection.advance_credits("Americans", 900);
        }
        assert_eq!(
            projection.displayed_credits_for_test("Americans"),
            Some(287)
        );

        let mut down = SidebarProjectionState::default();
        down.displayed_credits_or_seed("Americans", 900);
        down.advance_credits("Americans", 100);
        assert_eq!(down.displayed_credits_for_test("Americans"), Some(800));

        let mut clamped = SidebarProjectionState::default();
        clamped.displayed_credits_or_seed("Americans", 0);
        clamped.advance_credits("Americans", 5_000);
        assert_eq!(clamped.displayed_credits_for_test("Americans"), Some(143));
        clamped.displayed_credits_or_seed("Soviets", 10);
        clamped.advance_credits("Soviets", 11);
        assert_eq!(clamped.displayed_credits_for_test("Soviets"), Some(11));
    }

    /// Compose the production seam: credits step only when the runtime
    /// decision admits the simulation AND that pass commits an Ordinary frame.
    /// This mirrors the only advance site, which sits inside the
    /// `decision.run_sim` block of `advance_in_game_runtime_mode`.
    fn credits_step(
        inputs: crate::app::match_runtime::sim_tick::RuntimePassInputs,
        frame_committed: bool,
    ) -> bool {
        let decision = crate::app::match_runtime::sim_tick::decide_runtime_pass(inputs);
        decision.run_sim && credits_advance_for_frame(frame_committed, decision.tick_lane)
    }

    #[test]
    fn sidebar_credit_gate_matrix() {
        use crate::app::match_runtime::sim_tick::{
            RuntimePassInputs, SessionMode, decide_runtime_pass,
        };

        // Baseline wall-clock pass: active window, accepted startup receipt,
        // elapsed pacer window, nothing paused, no menu. Every freeze case
        // below flips exactly one real predicate off this baseline.
        let admitting = RuntimePassInputs {
            exact_step: false,
            window_active: true,
            startup_admitted: true,
            frame_stepping: false,
            paused: false,
            menu_open: false,
            session_mode: SessionMode::Skirmish,
            pacer_timing_admits: true,
        };
        let baseline = decide_runtime_pass(admitting);
        assert!(baseline.run_sim);
        assert_eq!(baseline.tick_lane, TickLane::Ordinary);
        assert!(baseline.admitted_by_pacer);
        assert!(credits_step(admitting, true));
        assert!(
            !credits_step(admitting, false),
            "an uncommitted frame must freeze displayed credits"
        );

        for (case, inputs) in [
            (
                "no-admit redraw",
                RuntimePassInputs {
                    pacer_timing_admits: false,
                    ..admitting
                },
            ),
            (
                "paused redraw",
                RuntimePassInputs {
                    paused: true,
                    ..admitting
                },
            ),
            (
                "menu redraw",
                RuntimePassInputs {
                    menu_open: true,
                    ..admitting
                },
            ),
            (
                "inactive redraw",
                RuntimePassInputs {
                    window_active: false,
                    ..admitting
                },
            ),
            (
                "missing startup receipt",
                RuntimePassInputs {
                    startup_admitted: false,
                    ..admitting
                },
            ),
        ] {
            let decision = decide_runtime_pass(inputs);
            assert!(!decision.run_sim, "{case} must not run the simulation");
            assert!(
                !credits_step(inputs, true),
                "{case} must freeze displayed credits"
            );
        }

        // A committed network-modal frame keeps the world advancing but must
        // not step displayed credits.
        let network_modal = RuntimePassInputs {
            paused: true,
            menu_open: true,
            session_mode: SessionMode::Lan,
            ..admitting
        };
        let decision = decide_runtime_pass(network_modal);
        assert!(
            decision.run_sim,
            "the network modal pump keeps the world advancing"
        );
        assert_eq!(decision.tick_lane, TickLane::NetworkModal);
        assert!(
            !credits_step(network_modal, true),
            "a committed network-modal frame must freeze displayed credits"
        );

        // Explicit single steps advance credits iff their one frame commits,
        // even while paused.
        let exact = RuntimePassInputs {
            exact_step: true,
            paused: true,
            ..admitting
        };
        assert!(
            credits_step(exact, true),
            "exact-step commit must advance exactly at its committed Ordinary seam"
        );
        assert!(!credits_step(exact, false));
        let debug_step = RuntimePassInputs {
            frame_stepping: true,
            paused: true,
            pacer_timing_admits: false,
            ..admitting
        };
        let debug_decision = decide_runtime_pass(debug_step);
        assert!(!debug_decision.admitted_by_pacer);
        assert!(
            credits_step(debug_step, true),
            "debug single-step commit must advance exactly at its committed Ordinary seam"
        );
    }
}
