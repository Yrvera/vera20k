//! Match owner (F12 `MatchState`): the aggregate for everything scoped to a
//! single running match — the authoritative `SimRuntime`, the app-side input,
//! presentation, audio, and diagnostics owners, plus the scenario identity and
//! pacing facts that accompany them.
//!
//! App-side only: nothing here is serialized, hashed, or read by the
//! deterministic simulation except through `SimRuntime` itself.

use crate::map::basic::BasicSection;

pub(crate) struct MatchState {
    pub(crate) sim_runtime: Option<crate::sim::runtime::SimRuntime>,
    /// Match input owner (F12): camera, zoom, cursor, keys, hotkeys.
    pub(crate) input: crate::app::input::state::MatchInputState,
    /// Match presentation owner (F12), part 1: per-match atlases + cursor.
    pub(crate) match_presentation: crate::app::presentation::state::MatchPresentationState,
    /// Per-match audio owner (F11): sound event queue + EVA latches; resets
    /// on every match install and on leaving a match for the shell.
    pub(crate) match_audio: crate::app::match_audio::MatchAudioState,
    /// App-owned diagnostic recording (F10) — never inside the simulation, so
    /// no load/install path can silently drop an unflushed segment.
    pub(crate) match_diagnostics: crate::app::match_diagnostics::MatchDiagnosticsState,
    pub(crate) map_basic: BasicSection,
    /// Exact source whose bytes produced the active parsed map.
    pub(crate) loaded_map_source: Option<crate::app::frontend::list_maps::LoadedMapSource>,
    /// Deterministic digest of the parsed source map INI. `None` only for
    /// generated/fallback worlds without an authoritative source-map payload.
    pub(crate) loaded_map_hash: Option<u64>,
    /// App-owned wall-clock outcome-EVA drain. The deterministic accepted
    /// result and SavourDelay target live in serialized `HouseState`.
    pub(crate) scenario_outcome: Option<crate::app::match_runtime::scenario_exit::ScenarioOutcomeVoiceWait>,
    /// Active running-scenario audio teardown. While present the tactical
    /// frame remains visible but simulation is frozen; its destination is
    /// committed only after the retail fade/voice-wait sequence completes.
    pub(crate) scenario_exit: Option<crate::app::match_runtime::scenario_exit::ScenarioExitCascade>,
    /// Match elapsed wall time for the retail score screen. App-local and never
    /// serialized, hashed, or read by deterministic simulation.
    pub(crate) scenario_elapsed_clock: crate::app::match_runtime::frame_pacer::ScenarioElapsedClock,
    /// Config-sourced input delay — copied to each new Simulation instance at game start.
    pub(crate) configured_input_delay_ticks: u64,
    /// Match-scoped local player identity, pinned ONCE at match launch
    /// (skirmish session / spawn-pick) and never rewritten mid-match. All
    /// command/HUD owner resolution reads this first — selection must never
    /// repoint the local player (lockstep: each client issues commands as its
    /// fixed house). `None` only in dev/sandbox flows with no launch identity,
    /// where the legacy heuristic + debug override below take over.
    pub(crate) local_player_owner: Option<String>,
    /// Explicit local owner preference for HUD/commands (set by debug actions).
    /// Only consulted when `local_player_owner` is `None` (sandbox/dev flows).
    pub(crate) local_owner_override: Option<String>,
    /// Seeded empty-map sandbox keeps full map visibility while still locking control.
    pub(crate) sandbox_full_visibility: bool,
    /// True when the game is paused (an in-scenario modal is open, sim frozen).
    ///
    /// Derived from `in_game_menu` for every player-driven modal; the debug
    /// pause (dev overlay / hotkey) also sets it without opening a menu.
    pub(crate) paused: bool,
    /// Effective simulation ticks per second — controls game speed.
    /// Default follows retail/YR skirmish stored game speed 1.
    pub(crate) sim_speed_tps: u32,
}
