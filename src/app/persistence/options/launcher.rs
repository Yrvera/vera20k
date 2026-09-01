//! Active-YR launcher Options parent projection and transaction ordering.
//!
//! The retained `RetailOptionsProfile` stays authoritative. This module owns
//! the pure projection plus narrow operation seams for the live preview and
//! accepted parent boundary; UI drawing, Winit enumeration, concrete audio,
//! persistence I/O, and child routing remain with their established owners.

use crate::app::persistence::options_profile::RetailOptionsProfile;
use crate::ui::main_menu_dialogs::options::{
    LauncherCue, LauncherOptionsEvent, LauncherOptionsLabels, LauncherOptionsPacked,
    LauncherOptionsValues, LauncherParentResult, LauncherResolutionRow, OptionsDialogState,
    admitted_initial_position, admitted_volume_position,
};

/// Pure result of substituting current-monitor Winit dimension pairs for the
/// obsolete DirectDraw 16-bpp enumeration used by the retail launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LauncherResolutionProjection {
    pub(crate) rows: Vec<LauncherResolutionRow>,
    pub(crate) selected_index: Option<usize>,
}

/// Project the retained signed profile into the fresh native-shaped controls.
///
/// gamemd-derived: launcher primary-proc setup at
/// `0x0055FDB0..0x0056047A`. Trackbar set-position rejects an out-of-range
/// request and leaves the fresh position at zero; it does not clamp.
pub(crate) fn launcher_values_from_profile(
    profile: &RetailOptionsProfile,
) -> LauncherOptionsValues {
    LauncherOptionsValues {
        detail_position: u8::from(profile.detail_level != 0),
        difficulty_position: admitted_initial_position(i64::from(profile.difficulty), 2),
        scroll_position: admitted_initial_position(6_i64 - i64::from(profile.scroll_rate), 6),
        tooltips: profile.tooltips,
        target_lines: profile.unit_action_lines,
        show_hidden: profile.show_hidden,
        score_position: admitted_volume_position(profile.score_volume),
        sound_position: admitted_volume_position(profile.sound_volume),
        voice_position: admitted_volume_position(profile.voice_volume),
    }
}

/// Filter/sort host mode pairs with the exact retail launcher list rules.
///
/// gamemd-derived: primary-proc slice `0x005601A0..0x00560270`. One Winit
/// dimension pair explicitly substitutes for one obsolete 16-bpp DirectDraw
/// mode. Duplicates remain; stable ascending width/height order and the last
/// displayed matching duplicate determine the initial selection.
pub(crate) fn project_launcher_resolutions(
    mode_pairs: impl IntoIterator<Item = (u32, u32)>,
    allow_hi_res_modes: bool,
    retained_pair: (i32, i32),
) -> LauncherResolutionProjection {
    let mut admitted: Vec<(u32, u32)> = mode_pairs
        .into_iter()
        .filter(|(width, height)| {
            if *width < 640 || *height < 480 {
                return false;
            }
            if allow_hi_res_modes {
                *width <= 4096 && *height <= 4096
            } else {
                matches!(*width, 640 | 800 | 1024) && *height <= 768
            }
        })
        .collect();
    admitted.sort_by_key(|&(width, height)| (width, height));

    let rows: Vec<_> = admitted
        .into_iter()
        .map(|(width, height)| LauncherResolutionRow::new(width as i32, height as i32))
        .collect();
    let selected_index = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| (row.width, row.height) == retained_pair)
        .map(|(index, _)| index)
        .last();

    LauncherResolutionProjection {
        rows,
        selected_index,
    }
}

/// Build one fresh primary snapshot from already-owned platform and text facts.
/// Child return and initial open both use this same constructor path.
pub(crate) fn launcher_dialog_from_profile(
    profile: &RetailOptionsProfile,
    labels: LauncherOptionsLabels,
    mode_pairs: impl IntoIterator<Item = (u32, u32)>,
    launcher_audio_available: bool,
) -> OptionsDialogState {
    let values = launcher_values_from_profile(profile);
    let resolutions = project_launcher_resolutions(
        mode_pairs,
        profile.allow_hi_res_modes,
        (profile.screen_width, profile.screen_height),
    );
    OptionsDialogState::new(
        labels,
        values,
        resolutions.rows,
        resolutions.selected_index,
        launcher_audio_available,
    )
}

/// Concrete effects admitted by one launcher UI frame.
///
/// The common audio predicate is deliberately queried again at dispatch so a
/// forged or stale audio event cannot store a profile value or touch output.
pub(crate) trait LauncherPreviewOperations {
    fn launcher_audio_available(&self) -> bool;
    fn play_cue(&mut self, cue: LauncherCue);
    fn store_resolution(&mut self, width: i32, height: i32);
    fn store_score_volume(&mut self, volume: f32);
    fn apply_score_output(&mut self, volume: f32);
    fn store_sound_volume(&mut self, volume: f32);
    fn apply_sound_output(&mut self, volume: f32);
    fn store_voice_volume(&mut self, volume: f32);
    fn apply_voice_output(&mut self, volume: f32);
    fn play_generic_beep(&mut self, local_multiplier: f32);
}

/// Dispatch one already-ordered UI event without an INI write or display-mode
/// change. The three audio paths preserve native profile -> output -> cue order.
pub(crate) fn dispatch_launcher_preview_event(
    operations: &mut impl LauncherPreviewOperations,
    event: LauncherOptionsEvent,
) {
    match event {
        LauncherOptionsEvent::Cue(cue) => operations.play_cue(cue),
        LauncherOptionsEvent::ResolutionSelected { width, height } => {
            operations.store_resolution(width, height);
        }
        LauncherOptionsEvent::ScorePreview(volume) => {
            if !operations.launcher_audio_available() {
                return;
            }
            operations.store_score_volume(volume);
            operations.apply_score_output(volume);
        }
        LauncherOptionsEvent::SoundPreview(volume) => {
            if !operations.launcher_audio_available() {
                return;
            }
            operations.store_sound_volume(volume);
            operations.apply_sound_output(volume);
            operations.play_generic_beep(1.0);
        }
        LauncherOptionsEvent::VoicePreview(volume) => {
            if !operations.launcher_audio_available() {
                return;
            }
            operations.store_voice_volume(volume);
            operations.apply_voice_output(volume);
            operations.play_generic_beep(volume);
        }
    }
}

/// Narrow effect boundary for the unconditional accepted parent transaction.
/// Production and the recorder both enter through
/// [`dispatch_launcher_parent_result`].
pub(crate) trait LauncherParentOperations {
    fn observe_pack(&mut self, packed: LauncherOptionsPacked);
    fn store_detail_level(&mut self, value: i32) -> bool;
    fn refresh_detail(&mut self);
    fn store_difficulty(&mut self, value: i32);
    fn store_unit_action_lines(&mut self, value: bool);
    fn refresh_unit_action_lines(&mut self);
    fn store_show_hidden(&mut self, value: bool);
    fn store_tooltips(&mut self, value: bool);
    fn store_scroll_rate(&mut self, value: i32);
    fn store_score_volume(&mut self, value: f32);
    fn apply_score_output(&mut self, value: f32);
    fn queue_then_stop_score_zero(&mut self);
    fn store_sound_volume(&mut self, value: f32);
    fn apply_sound_output(&mut self, value: f32);
    fn store_voice_volume(&mut self, value: f32);
    fn apply_voice_output(&mut self, value: f32);
    fn primary_dropped(&mut self);
    fn persist_profile(&mut self);
    fn prepare_network_settings(&mut self);
    fn route_network(&mut self);
    fn route_keyboard(&mut self);
    fn reopen_parent(&mut self);
}

/// Apply the packed parent subset in the literal native order.
///
/// gamemd-derived: `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0`.
/// Accepted audio setters are unconditional and cue-silent. Detail refresh is
/// changed-only; action-line refresh is unconditional.
fn apply_launcher_parent(
    operations: &mut impl LauncherParentOperations,
    packed: LauncherOptionsPacked,
) {
    let detail_changed = operations.store_detail_level(packed.detail_level);
    if detail_changed {
        operations.refresh_detail();
    }
    operations.store_difficulty(packed.difficulty);
    operations.store_unit_action_lines(packed.unit_action_lines);
    operations.refresh_unit_action_lines();
    operations.store_show_hidden(packed.show_hidden);
    operations.store_tooltips(packed.tooltips);
    operations.store_scroll_rate(packed.scroll_rate);
    operations.store_score_volume(packed.score_volume);
    operations.apply_score_output(packed.score_volume);
    if packed.score_volume == 0.0 {
        operations.queue_then_stop_score_zero();
    }
    operations.store_sound_volume(packed.sound_volume);
    operations.apply_sound_output(packed.sound_volume);
    operations.store_voice_volume(packed.voice_volume);
    operations.apply_voice_output(packed.voice_volume);
}

/// Consume one completed parent pass: Pack -> Apply -> physical dialog drop ->
/// result-specific continuation.
///
/// gamemd-derived: `OptionsClass__ShowLauncherDialog @ 0x0055FC80` always
/// applies and destroys the primary before child preparation/route or the one
/// final write. Network/Keyboard then rebuild a fresh primary without writing.
pub(crate) fn dispatch_launcher_parent_result(
    operations: &mut impl LauncherParentOperations,
    dialog: OptionsDialogState,
    result: LauncherParentResult,
) {
    let packed = dialog.pack();
    operations.observe_pack(packed);
    apply_launcher_parent(operations, packed);
    drop(dialog);
    operations.primary_dropped();

    match result {
        LauncherParentResult::Back | LauncherParentResult::Terminal => {
            operations.persist_profile();
        }
        LauncherParentResult::Network => {
            operations.prepare_network_settings();
            operations.route_network();
            operations.reopen_parent();
        }
        LauncherParentResult::Keyboard => {
            operations.route_keyboard();
            operations.reopen_parent();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::main_menu_dialogs::options::LauncherOptionsLabels;

    fn dialog(values: LauncherOptionsValues) -> OptionsDialogState {
        OptionsDialogState::new(
            LauncherOptionsLabels::resolve(&|_| None),
            values,
            Vec::new(),
            None,
            true,
        )
    }

    #[test]
    fn profile_projection_uses_reject_to_zero_edges() {
        let default = launcher_values_from_profile(&RetailOptionsProfile::default());
        assert_eq!(default, LauncherOptionsValues::default());

        for (input, expected) in [(0, 0), (2, 2), (3, 0), (4, 0), (-1, 0)] {
            let profile = RetailOptionsProfile {
                difficulty: input,
                ..RetailOptionsProfile::default()
            };
            assert_eq!(
                launcher_values_from_profile(&profile).difficulty_position,
                expected,
                "Difficulty={input}"
            );
        }
        for (input, expected) in [(-1, 0), (0, 6), (6, 0), (7, 0)] {
            let profile = RetailOptionsProfile {
                scroll_rate: input,
                ..RetailOptionsProfile::default()
            };
            assert_eq!(
                launcher_values_from_profile(&profile).scroll_position,
                expected,
                "ScrollRate={input}"
            );
        }
        for input in [i32::MIN, -2, -1, 1, 2, i32::MAX] {
            let profile = RetailOptionsProfile {
                detail_level: input,
                ..RetailOptionsProfile::default()
            };
            assert_eq!(launcher_values_from_profile(&profile).detail_position, 1);
        }
        assert_eq!(
            launcher_values_from_profile(&RetailOptionsProfile {
                detail_level: 0,
                ..RetailOptionsProfile::default()
            })
            .detail_position,
            0
        );

        for (input, expected) in [
            (-2.0, 0),
            (-0.05, 0),
            (0.0, 0),
            (0.04, 0),
            // Stored f32 0.95 promotes slightly below 0.95, so the native
            // x87 truncation of `v * 10 + 0.5` yields 9, not 10.
            (0.95, 9),
            (1.0, 10),
            (f32::NAN, 0),
            (f32::INFINITY, 0),
        ] {
            let profile = RetailOptionsProfile {
                score_volume: input,
                sound_volume: input,
                voice_volume: input,
                ..RetailOptionsProfile::default()
            };
            let values = launcher_values_from_profile(&profile);
            assert_eq!(values.score_position, expected, "volume={input:?}");
            assert_eq!(values.sound_position, expected, "volume={input:?}");
            assert_eq!(values.voice_position, expected, "volume={input:?}");
        }

        let booleans = launcher_values_from_profile(&RetailOptionsProfile {
            tooltips: false,
            unit_action_lines: false,
            show_hidden: true,
            ..RetailOptionsProfile::default()
        });
        assert!(!booleans.tooltips);
        assert!(!booleans.target_lines);
        assert!(booleans.show_hidden);
    }

    #[test]
    fn resolution_projection_filters_sorts_preserves_duplicates_and_selects_last_match() {
        let modes = [
            (1920, 1080),
            (800, 600),
            (639, 480),
            (800, 600),
            (1024, 769),
            (1024, 768),
            (640, 479),
            (1280, 720),
            (640, 480),
            (4097, 2160),
        ];
        let low = project_launcher_resolutions(modes, false, (800, 600));
        assert_eq!(
            low.rows
                .iter()
                .map(|row| (row.width, row.height, row.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (640, 480, "640 x 480 x 16"),
                (800, 600, "800 x 600 x 16"),
                (800, 600, "800 x 600 x 16"),
                (1024, 768, "1024 x 768 x 16"),
            ]
        );
        assert_eq!(low.selected_index, Some(2));

        let high = project_launcher_resolutions(modes, true, (1920, 1080));
        assert!(
            high.rows
                .iter()
                .any(|row| (row.width, row.height) == (1280, 720))
        );
        assert!(
            high.rows
                .iter()
                .any(|row| (row.width, row.height) == (1920, 1080))
        );
        assert!(!high.rows.iter().any(|row| row.width == 4097));
        assert_eq!(
            high.selected_index.map(|index| {
                let row = &high.rows[index];
                (row.width, row.height)
            }),
            Some((1920, 1080))
        );

        assert_eq!(
            project_launcher_resolutions([], false, (800, 600)),
            LauncherResolutionProjection {
                rows: Vec::new(),
                selected_index: None,
            }
        );
        assert_eq!(
            project_launcher_resolutions([(640, 480)], false, (800, 600)).selected_index,
            None
        );
    }

    #[test]
    fn fresh_dialog_constructor_projects_the_retained_parent_subset_and_audio_gate() {
        let profile = RetailOptionsProfile {
            detail_level: 0,
            difficulty: 2,
            scroll_rate: 0,
            tooltips: false,
            unit_action_lines: false,
            show_hidden: true,
            score_volume: 0.2,
            sound_volume: 0.5,
            voice_volume: 0.8,
            screen_width: 800,
            screen_height: 600,
            ..RetailOptionsProfile::default()
        };
        let dialog = launcher_dialog_from_profile(
            &profile,
            LauncherOptionsLabels::resolve(&|_| None),
            [(1024, 768), (800, 600), (800, 600)],
            false,
        );

        assert_eq!(
            dialog.pack(),
            LauncherOptionsPacked {
                detail_level: 0,
                difficulty: 2,
                unit_action_lines: false,
                show_hidden: true,
                tooltips: false,
                scroll_rate: 0,
                score_volume: 0.2,
                sound_volume: 0.5,
                voice_volume: 0.8,
            }
        );
        assert!(!dialog.launcher_audio_available());
    }

    #[derive(Debug, Clone, PartialEq)]
    enum PreviewCall {
        Cue(LauncherCue),
        Resolution(i32, i32),
        StoreScore(f32),
        OutputScore(f32),
        StoreSound(f32),
        OutputSound(f32),
        StoreVoice(f32),
        OutputVoice(f32),
        Beep(f32),
    }

    struct PreviewRecorder {
        available: bool,
        calls: Vec<PreviewCall>,
    }

    impl LauncherPreviewOperations for PreviewRecorder {
        fn launcher_audio_available(&self) -> bool {
            self.available
        }

        fn play_cue(&mut self, cue: LauncherCue) {
            self.calls.push(PreviewCall::Cue(cue));
        }

        fn store_resolution(&mut self, width: i32, height: i32) {
            self.calls.push(PreviewCall::Resolution(width, height));
        }

        fn store_score_volume(&mut self, volume: f32) {
            self.calls.push(PreviewCall::StoreScore(volume));
        }

        fn apply_score_output(&mut self, volume: f32) {
            self.calls.push(PreviewCall::OutputScore(volume));
        }

        fn store_sound_volume(&mut self, volume: f32) {
            self.calls.push(PreviewCall::StoreSound(volume));
        }

        fn apply_sound_output(&mut self, volume: f32) {
            self.calls.push(PreviewCall::OutputSound(volume));
        }

        fn store_voice_volume(&mut self, volume: f32) {
            self.calls.push(PreviewCall::StoreVoice(volume));
        }

        fn apply_voice_output(&mut self, volume: f32) {
            self.calls.push(PreviewCall::OutputVoice(volume));
        }

        fn play_generic_beep(&mut self, local_multiplier: f32) {
            self.calls.push(PreviewCall::Beep(local_multiplier));
        }
    }

    #[test]
    fn preview_dispatch_rechecks_common_gate_and_preserves_native_order() {
        let mut recorder = PreviewRecorder {
            available: true,
            calls: Vec::new(),
        };
        for event in [
            LauncherOptionsEvent::Cue(LauncherCue::Checkbox),
            LauncherOptionsEvent::ResolutionSelected {
                width: 1024,
                height: 768,
            },
            LauncherOptionsEvent::ScorePreview(0.4),
            LauncherOptionsEvent::SoundPreview(0.7),
            LauncherOptionsEvent::VoicePreview(0.3),
        ] {
            dispatch_launcher_preview_event(&mut recorder, event);
        }
        assert_eq!(
            recorder.calls,
            [
                PreviewCall::Cue(LauncherCue::Checkbox),
                PreviewCall::Resolution(1024, 768),
                PreviewCall::StoreScore(0.4),
                PreviewCall::OutputScore(0.4),
                PreviewCall::StoreSound(0.7),
                PreviewCall::OutputSound(0.7),
                PreviewCall::Beep(1.0),
                PreviewCall::StoreVoice(0.3),
                PreviewCall::OutputVoice(0.3),
                PreviewCall::Beep(0.3),
            ]
        );

        recorder.available = false;
        recorder.calls.clear();
        for event in [
            LauncherOptionsEvent::ScorePreview(0.1),
            LauncherOptionsEvent::SoundPreview(0.2),
            LauncherOptionsEvent::VoicePreview(0.3),
        ] {
            dispatch_launcher_preview_event(&mut recorder, event);
        }
        assert!(recorder.calls.is_empty());

        dispatch_launcher_preview_event(
            &mut recorder,
            LauncherOptionsEvent::ResolutionSelected {
                width: 640,
                height: 480,
            },
        );
        assert_eq!(recorder.calls, [PreviewCall::Resolution(640, 480)]);
    }

    #[derive(Debug, Clone, PartialEq)]
    enum ParentCall {
        Pack,
        Detail(i32),
        RefreshDetail,
        Difficulty(i32),
        TargetLines(bool),
        RefreshTargetLines,
        ShowHidden(bool),
        Tooltips(bool),
        Scroll(i32),
        StoreScore(f32),
        OutputScore(f32),
        QueueStopScoreZero,
        StoreSound(f32),
        OutputSound(f32),
        StoreVoice(f32),
        OutputVoice(f32),
        DropPrimary,
        Write,
        PrepareNetwork,
        RouteNetwork,
        RouteKeyboard,
        Reopen,
    }

    struct ParentRecorder {
        detail: i32,
        calls: Vec<ParentCall>,
    }

    impl LauncherParentOperations for ParentRecorder {
        fn observe_pack(&mut self, _packed: LauncherOptionsPacked) {
            self.calls.push(ParentCall::Pack);
        }

        fn store_detail_level(&mut self, value: i32) -> bool {
            self.calls.push(ParentCall::Detail(value));
            let changed = self.detail != value;
            self.detail = value;
            changed
        }

        fn refresh_detail(&mut self) {
            self.calls.push(ParentCall::RefreshDetail);
        }

        fn store_difficulty(&mut self, value: i32) {
            self.calls.push(ParentCall::Difficulty(value));
        }

        fn store_unit_action_lines(&mut self, value: bool) {
            self.calls.push(ParentCall::TargetLines(value));
        }

        fn refresh_unit_action_lines(&mut self) {
            self.calls.push(ParentCall::RefreshTargetLines);
        }

        fn store_show_hidden(&mut self, value: bool) {
            self.calls.push(ParentCall::ShowHidden(value));
        }

        fn store_tooltips(&mut self, value: bool) {
            self.calls.push(ParentCall::Tooltips(value));
        }

        fn store_scroll_rate(&mut self, value: i32) {
            self.calls.push(ParentCall::Scroll(value));
        }

        fn store_score_volume(&mut self, value: f32) {
            self.calls.push(ParentCall::StoreScore(value));
        }

        fn apply_score_output(&mut self, value: f32) {
            self.calls.push(ParentCall::OutputScore(value));
        }

        fn queue_then_stop_score_zero(&mut self) {
            self.calls.push(ParentCall::QueueStopScoreZero);
        }

        fn store_sound_volume(&mut self, value: f32) {
            self.calls.push(ParentCall::StoreSound(value));
        }

        fn apply_sound_output(&mut self, value: f32) {
            self.calls.push(ParentCall::OutputSound(value));
        }

        fn store_voice_volume(&mut self, value: f32) {
            self.calls.push(ParentCall::StoreVoice(value));
        }

        fn apply_voice_output(&mut self, value: f32) {
            self.calls.push(ParentCall::OutputVoice(value));
        }

        fn primary_dropped(&mut self) {
            self.calls.push(ParentCall::DropPrimary);
        }

        fn persist_profile(&mut self) {
            self.calls.push(ParentCall::Write);
        }

        fn prepare_network_settings(&mut self) {
            self.calls.push(ParentCall::PrepareNetwork);
        }

        fn route_network(&mut self) {
            self.calls.push(ParentCall::RouteNetwork);
        }

        fn route_keyboard(&mut self) {
            self.calls.push(ParentCall::RouteKeyboard);
        }

        fn reopen_parent(&mut self) {
            self.calls.push(ParentCall::Reopen);
        }
    }

    fn expected_apply(detail_changed: bool, score_zero: bool) -> Vec<ParentCall> {
        let mut calls = vec![ParentCall::Pack, ParentCall::Detail(2)];
        if detail_changed {
            calls.push(ParentCall::RefreshDetail);
        }
        calls.extend([
            ParentCall::Difficulty(2),
            ParentCall::TargetLines(false),
            ParentCall::RefreshTargetLines,
            ParentCall::ShowHidden(true),
            ParentCall::Tooltips(false),
            ParentCall::Scroll(0),
            ParentCall::StoreScore(if score_zero { 0.0 } else { 0.4 }),
            ParentCall::OutputScore(if score_zero { 0.0 } else { 0.4 }),
        ]);
        if score_zero {
            calls.push(ParentCall::QueueStopScoreZero);
        }
        calls.extend([
            ParentCall::StoreSound(0.7),
            ParentCall::OutputSound(0.7),
            ParentCall::StoreVoice(0.3),
            ParentCall::OutputVoice(0.3),
            ParentCall::DropPrimary,
        ]);
        calls
    }

    fn transaction_dialog(score_position: u8) -> OptionsDialogState {
        dialog(LauncherOptionsValues {
            detail_position: 1,
            difficulty_position: 2,
            scroll_position: 6,
            tooltips: false,
            target_lines: false,
            show_hidden: true,
            score_position,
            sound_position: 7,
            voice_position: 3,
        })
    }

    #[test]
    fn parent_apply_is_unconditional_interleaved_and_drops_before_final_write() {
        let mut changed = ParentRecorder {
            detail: 0,
            calls: Vec::new(),
        };
        dispatch_launcher_parent_result(
            &mut changed,
            transaction_dialog(0),
            LauncherParentResult::Back,
        );
        let mut expected = expected_apply(true, true);
        expected.push(ParentCall::Write);
        assert_eq!(changed.calls, expected);

        let mut unchanged = ParentRecorder {
            detail: 2,
            calls: Vec::new(),
        };
        dispatch_launcher_parent_result(
            &mut unchanged,
            transaction_dialog(4),
            LauncherParentResult::Terminal,
        );
        let mut expected = expected_apply(false, false);
        expected.push(ParentCall::Write);
        assert_eq!(unchanged.calls, expected);
    }

    #[test]
    fn child_routes_prepare_and_reopen_only_after_primary_drop_without_write() {
        let mut network = ParentRecorder {
            detail: 2,
            calls: Vec::new(),
        };
        dispatch_launcher_parent_result(
            &mut network,
            transaction_dialog(4),
            LauncherParentResult::Network,
        );
        let mut expected = expected_apply(false, false);
        expected.extend([
            ParentCall::PrepareNetwork,
            ParentCall::RouteNetwork,
            ParentCall::Reopen,
        ]);
        assert_eq!(network.calls, expected);

        let mut keyboard = ParentRecorder {
            detail: 2,
            calls: Vec::new(),
        };
        dispatch_launcher_parent_result(
            &mut keyboard,
            transaction_dialog(4),
            LauncherParentResult::Keyboard,
        );
        let mut expected = expected_apply(false, false);
        expected.extend([ParentCall::RouteKeyboard, ParentCall::Reopen]);
        assert_eq!(keyboard.calls, expected);
    }
}
