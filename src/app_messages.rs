//! Chat/system message surface driver (study §3.1): anchors the
//! `ui::messages::MessageList` to the tactical viewport (x+3 / y / w−14),
//! posts system messages (insert sound = [AudioVisual] IncomingMessage),
//! expires rows per frame against a pause-FROZEN clock (contract §4.2 step 8
//! / §4.3: deadlines resume with remaining lifetime intact after a pause),
//! and builds the text instances drawn between the sidebar text and the
//! tooltip (study O10: chat before tooltip).
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::render::batch::SpriteInstance;
use crate::ui::game_screen::GameScreen;

/// Native Init anchors: x = tactical_x + 3, y = tactical_y, w = tactical_w − 14.
const MESSAGE_X_INSET: i32 = 3;
const MESSAGE_WIDTH_INSET: i32 = 14;
/// Interim system-message color (native rows use a color-scheme index whose
/// mapping is a plan deferred item).
const MESSAGE_RGB_SYSTEM: [f32; 3] = [1.0, 1.0, 1.0];
/// Mission/trigger text lifetime — preserves the pre-A5 banner's 4 s
/// (the native trigger-text timeout is untraced; deferred item).
const MISSION_TEXT_TIMEOUT_MS: u64 = 4_000;

/// Pause-adjusted message `now` (contract §4.2 step 8 / §4.3): the wall clock
/// minus every paused span. ALL message deadlines and expiry checks use this
/// clock — never the raw `app_tooltips::now_ms` — so a pause freezes the
/// remaining lifetime of every visible row.
pub(crate) fn message_now_ms(state: &AppState) -> u64 {
    state
        .message_clock
        .now(crate::app_tooltips::now_ms(state))
}

/// Post a system message (mission/trigger text, future house notifications).
pub(crate) fn post_system_message(state: &mut AppState, text: &str) {
    sync_view(state);
    let now = message_now_ms(state);
    let font = &state.bit_font;
    let measure = |s: &str| font.text_width(s) as i32;
    let outcome = state.message_list.add_message(
        &crate::ui::messages::MessagePost {
            prefix: None,
            text,
            rgb: MESSAGE_RGB_SYSTEM,
            timeout_ms: Some(MISSION_TEXT_TIMEOUT_MS),
            silent: false,
        },
        now,
        &measure,
    );
    if outcome.play_sound {
        let sound = state
            .rules
            .as_ref()
            .and_then(|r| r.general.incoming_message_sound.clone());
        crate::app::App::play_shell_ui_sound_by_id(state, sound.as_deref());
    }
}

/// Per-frame: feed the pause edge into the clock, then (unpaused, in-game)
/// re-anchor to the live viewport and expire rows against the FROZEN clock.
/// While paused the clock accumulates the span and `manage` is skipped — both
/// halves are required: skipping alone would let wall-time deadlines expire
/// the instant the game unpauses.
pub(crate) fn update(state: &mut AppState) {
    if state.screen != GameScreen::InGame {
        return;
    }
    let wall = crate::app_tooltips::now_ms(state);
    state.message_clock.set_paused(state.paused, wall);
    if state.paused {
        return;
    }
    sync_view(state);
    let now = message_now_ms(state);
    state.message_list.manage(now);
}

fn sync_view(state: &mut AppState) {
    // Tactical viewport = render area minus the sidebar panel width.
    let tactical_w =
        state.render_width() as i32 - state.sidebar_layout_spec.sidebar_width.round() as i32;
    state.message_list.set_view(
        MESSAGE_X_INSET,
        0,
        (tactical_w - MESSAGE_WIDTH_INSET).max(0),
    );
}

/// Text instances for the "message_text" pooled buffer (GAME.FNT atlas).
pub(crate) fn build_message_text_instances(state: &AppState) -> Vec<SpriteInstance> {
    if state.screen != GameScreen::InGame {
        return Vec::new();
    }
    let font = &state.bit_font;
    let x = state.message_list.x() as f32;
    state
        .message_list
        .messages()
        .iter()
        .flat_map(|m| {
            crate::render::sidebar_text::build_text(
                font,
                &m.text,
                x,
                m.y as f32,
                1.0,
                0.00022,
                m.rgb,
                [0.0, 0.0],
            )
        })
        .collect()
}
