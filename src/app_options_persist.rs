//! In-game Options (0xBBB) close path: apply effects + persist `[Options]` to RA2MD.INI.
//!
//! Part of the app layer. `apply_in_game_options` runs every control's downstream
//! effect (sim cadence + live consumers) ON CLOSE only (KD-8 — never during
//! interaction); `persist_in_game_options` writes the touched `[Options]` keys to
//! `{ra2_dir}/RA2MD.INI` via the single-key in-place writer, mirroring the existing
//! `[Audio] ScoreVolume` round-trip. `in_game_options_close` ties them together
//! with the unpause/timing-reset that matches the pause-menu Resume path.

use crate::app::AppState;
use crate::app_target_lines::TargetLineState;
use crate::ui::shell::in_game_options_state::InGameOptionsState;
use crate::ui::shell::modal::ModalResult;

const RA2MD_INI_FILENAME: &str = "RA2MD.INI";
const OPTIONS_SECTION: &str = "Options";

/// Result code for a normal Back close (every close button -> result 1, which
/// persists). result 2 (game ended while the dialog was open) would skip persist;
/// no path produces it in 5a-iii's offline scope, but the gate is encoded so it is
/// correct when networked endings land.
pub(crate) const IN_GAME_OPTIONS_RESULT_BACK: i32 = 1;

/// Apply the UnitActionLines effect to the target-line render gate. Split out as a
/// pure helper over its two inputs so it is unit-testable without a GPU-backed
/// `AppState`. This is the one option with a confirmed live Rust consumer.
fn apply_target_lines(target_lines: &mut TargetLineState, opts: &InGameOptionsState) {
    target_lines.set_unit_action_lines_enabled(opts.unit_action_lines);
}

/// Apply every in-game Options control's downstream effect — ON CLOSE ONLY (KD-8).
/// During interaction only the visual/stored state changes; gamemd applies the
/// effects when the dialog closes, so the battlefield behind the non-opaque overlay
/// must not visibly change until Back.
pub(crate) fn apply_in_game_options(state: &mut AppState) {
    // GameSpeed -> sim cadence (KD-2): game_speed is the single source of truth.
    state.sim_speed_tps = crate::app_types::tps_for_game_speed(state.in_game_options.game_speed);
    // UnitActionLines -> the target-line render gate (the one confirmed live consumer).
    apply_target_lines(&mut state.target_lines, &state.in_game_options);
    // ScrollRate / DetailLevel / ShowHidden / ToolTips: persist-only — no live Rust
    // consumer wired yet (Task 8). DetailLevel is hidden in 0xBBB; ShowHidden is a
    // debug byte with no standard consumer.
}

/// Persist the six `[Options]` keys into `{ra2_dir}/RA2MD.INI`, updating each key
/// in place and preserving every other byte (mirrors the `[Audio] ScoreVolume`
/// write). Guarded by the save-on-close contract: result 1 (every Back) writes;
/// result 2 (game ended) skips. A write failure is logged, never fatal.
pub(crate) fn persist_in_game_options(state: &AppState, result: i32) {
    if !ModalResult::InGameOptions(result).options_persists() {
        return;
    }
    let Some(config) = state.game_config.as_ref() else {
        return;
    };
    let path = config.paths.ra2_dir.join(RA2MD_INI_FILENAME);
    let o = &state.in_game_options;
    // Internal values are stored verbatim: GameSpeed/ScrollRate already hold
    // `6 - slider_pos`; DetailLevel direct; checkboxes as "1"/"0".
    let pairs = [
        ("GameSpeed", o.game_speed.to_string()),
        ("ScrollRate", o.scroll_rate.to_string()),
        ("DetailLevel", o.detail_level.to_string()),
        ("UnitActionLines", (o.unit_action_lines as u8).to_string()),
        ("ShowHidden", (o.show_hidden as u8).to_string()),
        ("ToolTips", (o.tooltips as u8).to_string()),
    ];
    // Absent file -> the writer creates a fresh [Options] section.
    let mut bytes = std::fs::read(&path).unwrap_or_default();
    for (key, val) in &pairs {
        bytes = crate::util::ini_writer::set_ini_value(&bytes, OPTIONS_SECTION, key, val);
    }
    if let Err(err) = std::fs::write(&path, &bytes) {
        log::warn!("Failed to persist [Options] to RA2MD.INI: {err}");
    }
}

/// Close the in-game Options overlay: apply all effects (KD-8), persist on result
/// 1, then unpause + reset timing + re-hide the OS cursor (mirrors the pause-menu
/// Resume path so the new pace takes effect cleanly on unpause).
pub(crate) fn in_game_options_close(state: &mut AppState) {
    apply_in_game_options(state);
    persist_in_game_options(state, IN_GAME_OPTIONS_RESULT_BACK);
    state.paused = false;
    state.last_update_time = std::time::Instant::now();
    state.sim_accumulator_ms = 0;
    if state.software_cursor.is_some() {
        state.window.set_cursor_visible(false);
    }
    log::info!(
        "In-game Options closed; resumed at {} tps",
        state.sim_speed_tps
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_gate_writes_only_on_result_one() {
        // The save-on-close contract: result 1 (every Back) persists; result 2
        // (game ended) does not. No discard-without-save path exists.
        assert!(ModalResult::InGameOptions(IN_GAME_OPTIONS_RESULT_BACK).options_persists());
        assert!(ModalResult::InGameOptions(1).options_persists());
        assert!(!ModalResult::InGameOptions(2).options_persists());
    }

    #[test]
    fn apply_disables_target_lines_when_unit_action_lines_off() {
        // TargetLineState defaults to enabled; an Options apply with the checkbox
        // off must flip the live gate (the one confirmed consumer).
        let mut tl = TargetLineState::default();
        assert!(tl.unit_action_lines_enabled(), "defaults enabled");
        let opts = InGameOptionsState {
            unit_action_lines: false,
            ..Default::default()
        };
        apply_target_lines(&mut tl, &opts);
        assert!(!tl.unit_action_lines_enabled());
        // And re-enables when toggled back on.
        let opts_on = InGameOptionsState {
            unit_action_lines: true,
            ..Default::default()
        };
        apply_target_lines(&mut tl, &opts_on);
        assert!(tl.unit_action_lines_enabled());
    }
}
