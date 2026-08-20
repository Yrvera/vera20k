//! StartUncloaking sound-argument and transition acceptance.

use super::*;

#[test]
fn start_uncloaking_arg_zero_sounds_only_on_accepted_transition() {
    let mut audible = CloakRuntime::new(0, 9);
    audible.establish_unlimbo_fully_cloaked();
    let started = audible.start_uncloaking(12, 1, false);
    assert_eq!(
        started,
        StartUncloakingResult {
            transitioned: true,
            play_sound: true,
        }
    );
    assert_eq!(
        audible.start_uncloaking(13, 1, false),
        StartUncloakingResult {
            transitioned: false,
            play_sound: false,
        },
        "a repeated state-3 visit neither transitions nor replays the cue"
    );

    let mut silent = CloakRuntime::new(0, 9);
    silent.state = 1;
    silent.visual_phase = Some(CloakVisualPhase::Cloaking);
    assert_eq!(
        silent.start_uncloaking(12, 1, true),
        StartUncloakingResult {
            transitioned: true,
            play_sound: false,
        },
        "native arg one preserves state writes while suppressing CloakSound"
    );
}

#[test]
fn cloak_tick_reports_only_arg_zero_uncloak_sound_request() {
    let facts = |health_above_red, should_uncloak| CloakTickFacts {
        current_frame: 0,
        state_zero_head_allows: true,
        can_auto_cloak: true,
        should_uncloak,
        health_above_red,
        cloaking_speed: 1,
        cloak_delay_frames: 18,
    };

    let seed = (0..100_000)
        .find(|seed| {
            let mut rng = SimRng::new(*seed);
            rng.next_range_u32_inclusive(0, 99) <= 9
        })
        .expect("bounded seed search finds the native ten-percent branch");
    let mut rng = SimRng::new(seed);
    let mut abort = CloakRuntime::new(0, 9);
    abort.state = 1;
    abort.visual_phase = Some(CloakVisualPhase::Cloaking);
    abort.depth = 3;
    abort.step_delta = 1;
    abort.step_timer = CloakStepTimer {
        start_frame: 0,
        speed: 1,
        duration_frames: 1,
    };
    let result = abort.tick(facts(false, false), &mut rng);
    assert!(result.transitioned);
    assert!(!result.play_uncloak_sound, "mid-cloak abort calls StartUncloaking(1)");

    let mut ordinary = CloakRuntime::new(0, 9);
    ordinary.establish_unlimbo_fully_cloaked();
    let result = ordinary.tick(facts(true, true), &mut rng);
    assert!(result.transitioned && result.play_uncloak_sound);
}
