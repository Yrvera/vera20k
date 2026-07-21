//! Random-map setup modal state (the Create Random Map dialog).
//!
//! Render-agnostic: owns the working option set, the enable/disable state
//! machine, and the accept/cancel outcome. Depends on the rmg options model and
//! the layout control enum only — no assets, no wgpu.

use crate::map::rmg::options::RmgOptions;
use crate::map::rmg::randomize::{RandomRanged, randomize};
use crate::map::rmg::settings::RmgSettings;

use super::super::layout::RandomMapSetupControl;
use super::choose_map::ChooseMapSelection;

/// Sentinel meaning "no seed chosen yet"; replaced with a random one on open.
const UNSET_SEED: i32 = -1;

/// Which combo is currently dropped open, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupCombo {
    MapType,
    Time,
    Theater,
    Size,
    Resources,
}

/// What closing the dialog should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Accepted: commit these options.
    Commit(Box<RmgOptions>),
    /// Rejected because nothing has been generated yet.
    NeedsGenerate,
}

/// The Create Random Map dialog.
#[derive(Debug, Clone)]
pub struct RandomMapSetupModalState {
    /// The working option set the controls edit.
    pub options: RmgOptions,
    /// True once Generate has run and no option has changed since.
    pub generated: bool,
    /// True while the synchronous generate block owns the dialog.
    pub generating: bool,
    /// Load/Delete enablement. Starts at saved-seed availability, but the
    /// generate action turns it on unconditionally, as the original does.
    pub saved_seed_buttons_enabled: bool,
    pub open_combo: Option<SetupCombo>,
    pub pressed_control: Option<RandomMapSetupControl>,
    /// Restored verbatim if the player cancels.
    pub previous_selection: Option<ChooseMapSelection>,
}

impl RandomMapSetupModalState {
    /// Open the dialog over the current selection.
    ///
    /// An unset seed is replaced with a fresh random one, matching the
    /// original's init. Accept starts disabled: the player must generate first.
    pub fn open(
        mut options: RmgOptions,
        previous_selection: Option<ChooseMapSelection>,
        saved_seeds_available: bool,
        rng: &mut impl RandomRanged,
    ) -> Self {
        if options.seed == UNSET_SEED {
            options.seed = rng.ranged(0, 0xFFFF);
        }
        options.normalize();
        Self {
            options,
            generated: false,
            generating: false,
            saved_seed_buttons_enabled: saved_seeds_available,
            open_combo: None,
            pressed_control: None,
            previous_selection,
        }
    }

    /// Whether a control is currently interactive. Every control is inert
    /// during the synchronous generate block, including Cancel.
    pub fn is_enabled(&self, control: RandomMapSetupControl) -> bool {
        use RandomMapSetupControl as C;
        if self.generating {
            return false;
        }
        match control {
            // Accept and save both require a generated result.
            C::Ok0x6c5 | C::Save0x6c3 => self.generated,
            C::Load0x6c2 | C::Delete0x6c4 => self.saved_seed_buttons_enabled,
            _ => true,
        }
    }

    /// Apply an option edit. Any change invalidates the generated result, so
    /// accept is disabled until the next generate.
    pub fn set_map_type(&mut self, value: i32) {
        self.options.map_type = value;
        self.on_option_changed();
    }

    pub fn set_time(&mut self, value: i32) {
        self.options.time = value;
        self.on_option_changed();
    }

    pub fn set_theater(&mut self, value: i32) {
        self.options.theater = value;
        self.on_option_changed();
    }

    /// One size selection drives both axes.
    pub fn set_size(&mut self, value: i32) {
        self.options.width = value;
        self.options.height = value;
        self.on_option_changed();
    }

    pub fn set_resources(&mut self, value: i32) {
        self.options.resources = value;
        self.on_option_changed();
    }

    pub fn set_num_players(&mut self, value: i32) {
        self.options.num_players = value;
        self.on_option_changed();
    }

    fn on_option_changed(&mut self) {
        self.options.normalize();
        self.generated = false;
    }

    /// Surprise Me: randomize the option subset and invalidate the result.
    pub fn randomize_options(
        &mut self,
        settings: &RmgSettings,
        rng: &mut impl RandomRanged,
        description: &str,
    ) {
        randomize(&mut self.options, settings, rng, description);
        self.generated = false;
        self.open_combo = None;
    }

    /// Begin the synchronous generate block: every control goes inert.
    pub fn begin_generate(&mut self) {
        self.generating = true;
        self.open_combo = None;
    }

    /// End the generate block, marking a result available so accept unlocks.
    ///
    /// This also switches Load/Delete on unconditionally: the original
    /// re-enables the whole control set afterwards without re-testing whether
    /// any saved seed actually exists.
    pub fn finish_generate(&mut self) {
        self.generating = false;
        self.generated = true;
        self.saved_seed_buttons_enabled = true;
    }

    /// Accept. The original generates first when nothing has been generated
    /// yet, so a caller receiving `NeedsGenerate` must generate then retry.
    pub fn accept(&self) -> AcceptOutcome {
        if !self.generated {
            return AcceptOutcome::NeedsGenerate;
        }
        let mut committed = self.options.clone();
        committed.normalize();
        AcceptOutcome::Commit(Box::new(committed))
    }

    /// Cancel. Returns the selection to restore; the caller performs no other
    /// side effects.
    pub const fn cancel(&self) -> Option<ChooseMapSelection> {
        self.previous_selection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Always returns `max`, so draws are identifiable.
    struct MaxRng;
    impl RandomRanged for MaxRng {
        fn ranged(&mut self, _min: i32, max: i32) -> i32 {
            max
        }
    }

    fn opened() -> RandomMapSetupModalState {
        RandomMapSetupModalState::open(RmgOptions::default(), None, false, &mut MaxRng)
    }

    #[test]
    fn open_replaces_the_unset_seed() {
        assert_eq!(RmgOptions::default().seed, UNSET_SEED);
        assert_eq!(opened().options.seed, 0xFFFF, "unset seed is randomized");
    }

    #[test]
    fn open_keeps_an_existing_seed() {
        let options = RmgOptions {
            seed: 4321,
            ..Default::default()
        };
        let state = RandomMapSetupModalState::open(options, None, false, &mut MaxRng);
        assert_eq!(state.options.seed, 4321);
    }

    #[test]
    fn ok_and_save_start_disabled() {
        let state = opened();
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert!(!state.is_enabled(RandomMapSetupControl::Save0x6c3));
    }

    #[test]
    fn load_and_delete_follow_saved_seed_availability_at_open() {
        let none = opened();
        assert!(!none.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(!none.is_enabled(RandomMapSetupControl::Delete0x6c4));

        let some = RandomMapSetupModalState::open(RmgOptions::default(), None, true, &mut MaxRng);
        assert!(some.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(some.is_enabled(RandomMapSetupControl::Delete0x6c4));
    }

    #[test]
    fn generate_and_cancel_start_enabled() {
        let state = opened();
        assert!(state.is_enabled(RandomMapSetupControl::Generate0x620));
        assert!(state.is_enabled(RandomMapSetupControl::Cancel0x5c0));
    }

    #[test]
    fn changing_an_option_disables_accept_again() {
        let mut state = opened();
        state.generated = true;
        state.set_resources(2);
        assert!(!state.generated, "an edit invalidates the generated result");
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
    }

    #[test]
    fn size_writes_both_axes() {
        let mut state = opened();
        state.set_size(3);
        assert_eq!((state.options.width, state.options.height), (3, 3));
    }

    #[test]
    fn mutators_clamp_through_normalize() {
        let mut state = opened();
        state.set_num_players(99);
        assert_eq!(state.options.num_players, 8);
        state.set_num_players(0);
        assert_eq!(state.options.num_players, 2);
    }

    #[test]
    fn randomize_invalidates_the_generated_result() {
        let mut state = opened();
        state.generated = true;
        state.randomize_options(&RmgSettings::default(), &mut MaxRng, "Random Map");
        assert!(!state.generated);
        assert!(!state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert_eq!(state.options.description, "Random Map");
    }

    #[test]
    fn every_control_is_inert_during_generate_including_cancel() {
        let mut state = opened();
        state.begin_generate();
        for control in [
            RandomMapSetupControl::MapType0x405,
            RandomMapSetupControl::Theater0x407,
            RandomMapSetupControl::Players0x3eb,
            RandomMapSetupControl::Randomize0x621,
            RandomMapSetupControl::Generate0x620,
            RandomMapSetupControl::Ok0x6c5,
            RandomMapSetupControl::Cancel0x5c0,
        ] {
            assert!(!state.is_enabled(control), "{control:?} must be inert");
        }
    }

    #[test]
    fn finishing_generate_unlocks_accept() {
        let mut state = opened();
        state.begin_generate();
        state.finish_generate();
        assert!(state.is_enabled(RandomMapSetupControl::Ok0x6c5));
        assert!(state.is_enabled(RandomMapSetupControl::Cancel0x5c0));
    }

    #[test]
    fn generate_enables_load_and_delete_even_with_no_saved_seeds() {
        // The original re-enables the whole control set after generating
        // without re-testing saved-seed availability.
        let mut state = opened();
        assert!(!state.is_enabled(RandomMapSetupControl::Load0x6c2));
        state.begin_generate();
        state.finish_generate();
        assert!(state.is_enabled(RandomMapSetupControl::Load0x6c2));
        assert!(state.is_enabled(RandomMapSetupControl::Delete0x6c4));
    }

    #[test]
    fn accept_before_generate_asks_for_generation() {
        assert_eq!(opened().accept(), AcceptOutcome::NeedsGenerate);
    }

    #[test]
    fn accept_after_generate_commits_normalized_options() {
        let mut state = opened();
        state.finish_generate();
        match state.accept() {
            AcceptOutcome::Commit(options) => {
                assert!(options.tiberium >= 1, "committed options are normalized");
                assert_eq!(options.seed, state.options.seed);
            }
            other => panic!("expected a commit, got {other:?}"),
        }
    }

    #[test]
    fn cancelling_returns_the_previous_selection_untouched() {
        let previous = ChooseMapSelection {
            mode_id: 1,
            record_index: Some(3),
        };
        let state = RandomMapSetupModalState::open(
            RmgOptions::default(),
            Some(previous),
            false,
            &mut MaxRng,
        );
        assert_eq!(state.cancel(), Some(previous));
    }
}
