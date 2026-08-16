//! In-game Options (0xBBB) close path: apply effects + persist `[Options]` to RA2MD.INI.
//!
//! Part of the app layer. `apply_in_game_options` runs every control's downstream
//! effect (queues sim cadence + updates live consumers) ON CLOSE only (KD-8 — never during
//! interaction); `persist_in_game_options` writes the touched `[Options]` keys to
//! `{ra2_dir}/RA2MD.INI` via the single-key in-place writer, mirroring the existing
//! `[Audio] ScoreVolume` round-trip. `in_game_options_close` ties them together
//! with the unpause/timing-reset that matches the pause-menu Resume path.

use crate::app::AppState;
use crate::app::presentation::target_lines::TargetLineState;
use crate::rules::ini_parser::IniFile;
use crate::ui::shell::in_game_options_state::InGameOptionsState;
use crate::ui::shell::modal::ModalResult;

const RA2MD_INI_FILENAME: &str = "RA2MD.INI";
const OPTIONS_SECTION: &str = "Options";

/// Read `[Options] ScrollRate` from the retail user settings file. Missing,
/// negative, or malformed values return `None` so startup keeps the constructor
/// default; non-negative native integers fit the current unsigned state.
pub(crate) fn read_scroll_rate_from_ra2md(ra2_dir: &std::path::Path) -> Option<u32> {
    let bytes = std::fs::read(ra2_dir.join(RA2MD_INI_FILENAME)).ok()?;
    let ini = IniFile::from_bytes(&bytes).ok()?;
    scroll_rate_from_ini(&ini)
}

/// Read and native-clamp `[Options] DetailLevel` from the retail settings file.
/// Malformed or missing values keep the constructor default at the caller.
pub(crate) fn read_detail_level_from_ra2md(ra2_dir: &std::path::Path) -> Option<u32> {
    let bytes = std::fs::read(ra2_dir.join(RA2MD_INI_FILENAME)).ok()?;
    let ini = IniFile::from_bytes(&bytes).ok()?;
    detail_level_from_ini(&ini)
}

fn scroll_rate_from_ini(ini: &IniFile) -> Option<u32> {
    let raw = ini.section(OPTIONS_SECTION)?.get("ScrollRate")?;
    let value = raw.trim().parse::<i32>().ok()?;
    u32::try_from(value).ok()
}

fn detail_level_from_ini(ini: &IniFile) -> Option<u32> {
    let raw = ini.section(OPTIONS_SECTION)?.get("DetailLevel")?;
    let value = raw.trim().parse::<i32>().ok()?;
    Some(value.clamp(
        0,
        crate::ui::shell::in_game_options_state::OPTIONS_DETAIL_MAX as i32,
    ) as u32)
}

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
    // Keep the UI readout in sync immediately. The sim-owned speed changes only
    // when the replayable transition is admitted before the next logic frame.
    state.match_state.sim_speed_tps = crate::app::types::tps_for_game_speed(state.match_state.match_presentation.in_game_options.game_speed);
    if state.match_state.sim_runtime.is_some() {
        let owner = crate::app::input::commands::preferred_local_owner_name(state);
        let speed = u8::try_from(state.match_state.match_presentation.in_game_options.game_speed).ok();
        let scheduled = match (owner, speed) {
            (Some(owner), Some(speed)) => crate::app::input::commands::try_schedule_command(
                state,
                &owner,
                crate::sim::command::Command::SetGameSpeed { speed },
            ),
            _ => None,
        };
        if scheduled.is_none() {
            log::warn!(
                "In-game Options could not queue GameSpeed={} for the local house",
                state.match_state.match_presentation.in_game_options.game_speed
            );
            crate::app::loading::transitions::sync_in_game_options_speed_from_sim(state);
        }
    }
    // UnitActionLines -> the target-line render gate (the one confirmed live consumer).
    apply_target_lines(&mut state.match_state.match_presentation.target_lines, &state.match_state.match_presentation.in_game_options);
    // ScrollRate is read at startup and consumed directly by the camera each
    // frame, so nothing has to be pushed from this close path.
    // The remaining two are persist-only; no render behavior is fabricated:
    //   ToolTips    -> the tooltip service (`input::tooltips`) has no enable gate to flip.
    // DetailLevel is consumed directly by the per-frame lighting refresh, so
    // it needs no close-time push beyond the stored option changing.
    //   ShowHidden  -> a debug byte with no standard consumer.
}

/// Persist the six `[Options]` keys into `{ra2_dir}/RA2MD.INI`, updating each key
/// in place and preserving every other byte (mirrors the `[Audio] ScoreVolume`
/// write). Guarded by the save-on-close contract: result 1 (every Back) writes;
/// result 2 (game ended) skips. A write failure is logged, never fatal.
pub(crate) fn persist_in_game_options(state: &AppState, result: i32) {
    if !ModalResult::InGameOptions(result).options_persists() {
        return;
    }
    let Some(config) = state.platform.game_config.as_ref() else {
        return;
    };
    let path = config.paths.ra2_dir.join(RA2MD_INI_FILENAME);
    let o = &state.match_state.match_presentation.in_game_options;
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
    state.match_state.paused = false;
    state.platform.frame_pacer.reset_for_immediate_frame();
    if state.match_state.match_presentation.software_cursor.is_some() {
        state.platform.window.set_cursor_visible(false);
    }
    log::info!(
        "In-game Options closed; resumed at {} tps",
        state.match_state.sim_speed_tps
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item82_scroll_rate_reads_retail_value_and_rejects_missing_or_malformed() {
        let retail = IniFile::from_str("[Options]\nScrollRate=4\n");
        assert_eq!(scroll_rate_from_ini(&retail), Some(4));

        let absent = IniFile::from_str("[Options]\nGameSpeed=1\n");
        assert_eq!(scroll_rate_from_ini(&absent), None);

        let malformed = IniFile::from_str("[Options]\nScrollRate=fast\n");
        assert_eq!(scroll_rate_from_ini(&malformed), None);

        let negative = IniFile::from_str("[Options]\nScrollRate=-1\n");
        assert_eq!(scroll_rate_from_ini(&negative), None);
        assert_eq!(InGameOptionsState::default().scroll_rate, 3);
    }

    #[test]
    fn gsi_04_20_detail_level_reads_retail_value_and_clamps() {
        assert_eq!(
            detail_level_from_ini(&IniFile::from_str("[Options]\nDetailLevel=1\n")),
            Some(1)
        );
        assert_eq!(
            detail_level_from_ini(&IniFile::from_str("[Options]\nDetailLevel=-4\n")),
            Some(0)
        );
        assert_eq!(
            detail_level_from_ini(&IniFile::from_str("[Options]\nDetailLevel=9\n")),
            Some(2)
        );
        assert_eq!(
            detail_level_from_ini(&IniFile::from_str("[Options]\nDetailLevel=medium\n")),
            None
        );
    }

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
