//! In-game update phase — advances fixed-step simulation, triggers, path grids, and atlases.
//!
//! Camera control lives in app_camera.rs. Building animations, damage fires, sidebar
//! UI tick, and sound playback live in app_building_anim.rs.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::app::AppState;
use crate::app_commands::{preferred_local_owner, preferred_local_owner_name};

/// Minimum ticks between under-attack EVA voice lines (~30 s at 67 ms/tick).
/// The native per-house attack-voice repeat delay is UNVERIFIED-pending-trace.
const EVA_UNDER_ATTACK_COOLDOWN_TICKS: u64 = 450;
use crate::app_types::SIM_TICK_HZ;
use crate::app_types::SIM_TICK_MS;
use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::audio::events::GameSoundEvent;
use crate::map::terrain;
use crate::render::sprite_atlas;
use crate::render::unit_atlas;
use crate::sim::production;
use crate::sim::replay::{ReplayHeader, ReplayLog};
use crate::sim::trigger_runtime::TriggerEffect;
use crate::sim::world::{
    LifecycleOutput, SimFireEvent, SimFrameOutput, SimSoundEvent, TickLane, TriggerInputs,
};
use crate::ui::game_screen::GameScreen;

/// Directory for Rust-only deterministic diagnostic logs.
const REPLAYS_DIR: &str = "replays";

fn wall_sell_sound_for_local(
    receiver_name: &str,
    local_owner: Option<&str>,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> Option<GameSoundEvent> {
    if !local_owner.is_some_and(|local| local.eq_ignore_ascii_case(receiver_name)) {
        return None;
    }
    Some(GameSoundEvent::UiSound {
        sound_id: rules?.general.sell_sound.clone()?,
    })
}

struct ReplayLogFlush {
    path: PathBuf,
    tick_count: usize,
}

fn flush_replay_log_to(
    log_slot: &mut Option<ReplayLog>,
    session_tick: u64,
    replays_dir: &Path,
    unix_secs: u64,
) -> anyhow::Result<Option<ReplayLogFlush>> {
    let Some(log) = log_slot.take() else {
        return Ok(None);
    };
    if log.ticks.is_empty() {
        return Ok(None);
    }

    let result = (|| {
        std::fs::create_dir_all(replays_dir)?;
        let path = replays_dir.join(format!("replay_tick{session_tick}_{unix_secs}.json"));
        log.save(&path)?;
        Ok(Some(ReplayLogFlush {
            path,
            tick_count: log.ticks.len(),
        }))
    })();

    if result.is_err() {
        *log_slot = Some(log);
    }
    result
}

/// Persist and consume the in-memory deterministic diagnostic log.
///
/// The log lives on the sim (`sim.replay_log`) and is appended every tick but
/// is otherwise dropped when the sim is torn down. Call this on match teardown
/// so every finished match leaves a rich command+hash trace for desync
/// diagnosis. This JSON artifact is separate from the fixed native recording
/// stream in `sim::replay`. No-op when there is no active sim or no recorded
/// ticks. A successful write consumes the log so repeated teardown hooks do
/// not duplicate it; any failure restores it for a later retry. Writes
/// `replays/replay_tick{tick}_{unix_secs}.json`.
pub(crate) fn flush_replay_log(state: &mut AppState) {
    let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) else {
        return;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match flush_replay_log_to(
        &mut sim.replay_log,
        sim.session.tick,
        Path::new(REPLAYS_DIR),
        now,
    ) {
        Ok(Some(flush)) => log::info!(
            "Deterministic diagnostic log flushed: {} ticks -> {}",
            flush.tick_count,
            flush.path.display()
        ),
        Ok(None) => {}
        Err(e) => log::error!("Diagnostic-log flush failed: {e}"),
    }
}

#[cfg(test)]
mod replay_log_flush_tests {
    use super::flush_replay_log_to;
    use crate::sim::replay::{ReplayHeader, ReplayLog};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vera20k-gsi-17-08-{}-{label}-{}",
            std::process::id(),
            NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn sample_log() -> ReplayLog {
        let mut log = ReplayLog::new(ReplayHeader {
            version: 71,
            tick_hz: 15,
            seed: 0x1234,
            map_name: "gsi_17_08.map".to_owned(),
            rules_hash: 0x5678,
        });
        log.record_tick(41, Vec::new(), 0x9abc);
        log
    }

    #[test]
    fn gsi_17_08_success_writes_decodable_json_once_and_consumes_log() {
        let root = test_path("success");
        let mut slot = Some(sample_log());

        let flush = flush_replay_log_to(&mut slot, 41, &root, 1_234)
            .expect("flush succeeds")
            .expect("nonempty log flushes");
        assert!(slot.is_none());
        assert_eq!(flush.tick_count, 1);
        assert_eq!(flush.path, root.join("replay_tick41_1234.json"));

        let decoded = ReplayLog::load(&flush.path).expect("written JSON decodes");
        assert_eq!(decoded.header.seed, 0x1234);
        assert_eq!(decoded.header.map_name, "gsi_17_08.map");
        assert_eq!(decoded.ticks.len(), 1);
        assert_eq!(decoded.ticks[0].tick, 41);
        assert_eq!(decoded.ticks[0].state_hash, 0x9abc);

        assert!(
            flush_replay_log_to(&mut slot, 41, &root, 1_235)
                .expect("repeat is a no-op")
                .is_none()
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn gsi_17_08_create_and_write_failures_restore_log_for_retry() {
        let create_blocker = test_path("create-failure");
        std::fs::write(&create_blocker, b"not a directory").unwrap();
        let mut slot = Some(sample_log());
        assert!(flush_replay_log_to(&mut slot, 41, &create_blocker, 2_000).is_err());
        assert_eq!(slot.as_ref().unwrap().ticks[0].state_hash, 0x9abc);
        std::fs::remove_file(&create_blocker).unwrap();

        let root = test_path("write-failure");
        let output = root.join("replay_tick41_2001.json");
        std::fs::create_dir_all(&output).unwrap();
        let mut slot = Some(sample_log());
        assert!(flush_replay_log_to(&mut slot, 41, &root, 2_001).is_err());
        assert_eq!(slot.as_ref().unwrap().header.seed, 0x1234);
        assert_eq!(slot.as_ref().unwrap().ticks.len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}

/// App-side producers for the high-frequency EVA state cues the sim emits no
/// events for yet: "Low power", "Insufficient funds", "Unit lost".
///
/// Each cue is edge-detected against the previous frame's state (the
/// `AppState.eva_*` trackers) so it fires once per transition; the voice
/// queue's same-cue dedupe is the repeat suppressor while a cue is already
/// playing/queued. Native VoxClass priority tiers and re-announce cadence
/// remain a later parity surface (the queue bridge in `audio/sfx.rs` says the
/// same) — this wires the producers only.
fn announce_local_state_evas(state: &mut AppState) {
    let Some(owner) = crate::app_commands::preferred_local_owner_name(state) else {
        return;
    };
    // Read phase (immutable sim borrow): compute this frame's states and the
    // newly-dying set; commit to the trackers after the borrow ends.
    let (low_power, funds_stalled, current_dying, newly_dying) = {
        let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
            return;
        };
        let owner_id = sim.interner.get(&owner);
        let low_power = owner_id
            .and_then(|id| sim.power_states.get(&id))
            .is_some_and(|p| p.is_low_power);
        // Underfunded stall: any local factory holding an active object.
        let funds_stalled = owner_id.is_some_and(|id| {
            sim.production
                .factory_shadow
                .iter_insertion_ordered()
                .iter()
                .any(|f| f.owner == id && f.on_hold && f.object.is_some())
        });
        // Local mobile entities currently in their death sequence. Structures
        // have their own radar/EVA surface (not wired here); instant removals
        // that never set `dying` (e.g. crush) are a known miss until the sim
        // emits a death event.
        let current_dying: Vec<u64> = owner_id
            .map(|id| {
                sim.entities()
                    .values()
                    .filter(|e| {
                        e.dying
                            && e.owner == id
                            && e.category != crate::map::entities::EntityCategory::Structure
                    })
                    .map(|e| e.stable_id)
                    .collect()
            })
            .unwrap_or_default();
        let newly_dying: Vec<u64> = current_dying
            .iter()
            .copied()
            .filter(|id| !state.eva_announced_dying.contains(id))
            .collect();
        (low_power, funds_stalled, current_dying, newly_dying)
    };

    let mut cues: Vec<(&'static str, &'static str)> = Vec::new();
    if low_power && !state.eva_low_power_active {
        cues.push(("EVA_LowPower", "ceva053"));
    }
    state.eva_low_power_active = low_power;
    if funds_stalled && !state.eva_funds_stalled {
        cues.push(("EVA_InsufficientFunds", "ceva050"));
    }
    state.eva_funds_stalled = funds_stalled;
    if !newly_dying.is_empty() {
        cues.push(("EVA_UnitLost", "ceva064"));
    }
    // Prune despawned corpses, then record this frame's announcements.
    state
        .eva_announced_dying
        .retain(|id| current_dying.contains(id));
    state.eva_announced_dying.extend(newly_dying);

    if cues.is_empty() {
        return;
    }
    let faction = crate::app_building_anim::eva_faction_key(&owner, &state.house_roster);
    let sound_ids: Vec<String> = cues
        .iter()
        .map(|(cue, fallback)| {
            state
                .eva_registry
                .get(cue, faction)
                .unwrap_or(fallback)
                .to_string()
        })
        .collect();
    let (Some(sfx), Some(assets)) = (&mut state.sfx_player, &state.asset_manager) else {
        return;
    };
    for sound_id in &sound_ids {
        sfx.queue_eva_sound(
            sound_id,
            &state.sound_registry,
            assets,
            &state.audio_indices,
        );
    }
}

/// Drive the app-owned Vox wait after serialized HouseState reaches its exact
/// SavourDelay expiry frame. A loaded expiry latch reconstructs this wait but
/// never reconstructs the already-consumed transition EVA edge.
pub(crate) fn drive_local_player_outcome_voice_wait(state: &mut AppState, wall_ms: u64) {
    if !matches!(state.screen, GameScreen::InGame) || state.scenario_exit.is_some() {
        return;
    }
    if state.scenario_outcome.is_none() {
        let Some(owner) = state.local_player_owner.as_deref() else {
            return;
        };
        let outcome = state.sim_runtime.as_ref().map(|rt| &rt.simulation).and_then(|sim| {
            crate::sim::house_state::house_state_for_owner(&sim.houses, owner, &sim.interner)
                .and_then(|house| house.outcome_state)
                .filter(|outcome| outcome.exit_ready)
        });
        let Some(outcome) = outcome else {
            return;
        };
        log::info!(
            "Match end ready for local player '{owner}': {}",
            crate::app_scenario_exit::outcome_title(outcome.kind)
        );
        state.scenario_outcome = Some(crate::app_scenario_exit::ScenarioOutcomeVoiceWait::start(
            wall_ms,
            outcome.kind,
        ));
    }

    let voices_active = state
        .sfx_player
        .as_mut()
        .is_some_and(|sfx| sfx.pump_and_check_voices());
    let finished = state
        .scenario_outcome
        .as_ref()
        .is_some_and(|outcome| outcome.tick(wall_ms, voices_active));
    if !finished {
        return;
    }

    let outcome = state
        .scenario_outcome
        .as_ref()
        .expect("finished outcome voice wait remains present")
        .kind();
    state.scenario_outcome = None;
    state.finished_game_count = state.finished_game_count.saturating_add(1);
    let elapsed_seconds = state.scenario_elapsed_clock.stop(wall_ms);
    let model = build_score_screen_model(state, elapsed_seconds);
    // The outcome handlers at 0x00685670 / 0x00685DC0 begin only after the
    // HouseClass timer and 0x78-bucket Vox wait. From here the existing cascade
    // performs their master fade, 300-bucket tail, hard stop, and SCORE handoff.
    state.scenario_exit = Some(crate::app_scenario_exit::ScenarioExitCascade::start(
        wall_ms,
        crate::app_scenario_exit::ScenarioExitDestination::Score {
            title: crate::app_scenario_exit::outcome_title(outcome).to_string(),
            detail: crate::app_scenario_exit::outcome_detail(outcome).to_string(),
            model,
        },
    ));
}

fn outcome_eva_entry(
    kind: crate::sim::house_state::HouseOutcomeKind,
    faction: &str,
) -> (&'static str, &'static str) {
    match (kind, faction) {
        (crate::sim::house_state::HouseOutcomeKind::Victory, "Russian") => {
            ("EVA_YouAreVictorious", "csof022")
        }
        (crate::sim::house_state::HouseOutcomeKind::Victory, "Yuri") => {
            ("EVA_YouAreVictorious", "cyur022")
        }
        (crate::sim::house_state::HouseOutcomeKind::Victory, _) => {
            ("EVA_YouAreVictorious", "ceva022")
        }
        (crate::sim::house_state::HouseOutcomeKind::Defeat, "Russian") => {
            ("EVA_YouHaveLost", "csof023")
        }
        (crate::sim::house_state::HouseOutcomeKind::Defeat, "Yuri") => {
            ("EVA_YouHaveLost", "cyur023")
        }
        (crate::sim::house_state::HouseOutcomeKind::Defeat, _) => ("EVA_YouHaveLost", "ceva023"),
    }
}

/// The name one score row shows.
///
/// Native copies a stored per-house display name into every row, so no row ever
/// shows the raw house key. The local player's is the handle they launched
/// under; every other house shows its country's display name, which is what
/// native derives that slot from.
fn score_row_display_name(
    owner_name: &str,
    local_owner: &Option<String>,
    local_handle: &Option<String>,
    country_name: Option<&str>,
) -> String {
    let is_local = local_owner
        .as_deref()
        .is_some_and(|local| local.eq_ignore_ascii_case(owner_name));
    if is_local && let Some(handle) = local_handle {
        return handle.clone();
    }
    // The raw house key is the last resort: it only surfaces for a house with no
    // resolvable country at all.
    country_name.unwrap_or(owner_name).to_string()
}

/// Resolve the end-of-match score presentation from a sim-owned raw snapshot.
///
/// Simulation owns contender admission, raw statistics, displayed-score bonus
/// calculation, and its Scenario RNG draws. This app helper only resolves names,
/// colours, elapsed wall time, and display order. The existing Rust bonus formula
/// and contender admission rules are preserved, while sim now uses its canonical
/// house registration order. Exact native score-dialog traversal remains UNCHECKED.
fn build_score_screen_model(
    state: &AppState,
    elapsed_seconds: i32,
) -> crate::ui::score_shell::ScoreScreenModel {
    use crate::ui::score_shell::{ScoreRow, ScoreScreenModel};

    let local_owner = crate::app_commands::preferred_local_owner_name(state);
    // Use the launch handle while it is still available. Current map handoff
    // clears LoadingSession instead of pinning the handle for the match, so the
    // ordinary fallback remains a recorded presentation residual.
    let local_handle = crate::app_loading::launch_player_name(state);
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return ScoreScreenModel::default();
    };
    let Some(raw_snapshot) = sim.terminal_score_snapshot().cloned() else {
        log::error!("Natural match exit reached the score screen without a sim snapshot");
        return ScoreScreenModel::default();
    };
    let mut rows: Vec<ScoreRow> = Vec::with_capacity(raw_snapshot.rows.len());
    for raw in raw_snapshot.rows {
        let owner_name = sim.interner.resolve(raw.owner).to_string();
        let country_name = raw.country.and_then(|country| {
            let country = sim.interner.resolve(country);
            let (ui_key, plain) = state
                .rules
                .as_ref()
                .map(|rules| rules.country_display_name_sources(country))
                .unwrap_or((None, None));
            let localized = ui_key
                .zip(state.csf.as_ref())
                .map(|(key, csf)| csf.text(key).into_owned())
                .filter(|text| Some(text.as_str()) != ui_key);
            localized.or_else(|| plain.map(str::to_string))
        });
        let display_name = score_row_display_name(
            &owner_name,
            &local_owner,
            &local_handle,
            country_name.as_deref(),
        );
        let color_index = state
            .house_color_map
            .get(&owner_name)
            .copied()
            .unwrap_or(crate::rules::house_colors::NO_REMAP);
        // Shade 0 is the brightest band of the scheme ramp — the same colour
        // the radar draws this house's dots with.
        let rgb = state
            .rules
            .as_ref()
            .map(|rules| {
                let color = rules.house_color_ramps.ramp(color_index)[0];
                [color.r, color.g, color.b]
            })
            .unwrap_or([0xFF, 0xFF, 0xFF]);
        rows.push(ScoreRow {
            name: display_name,
            rgb,
            kills: raw.kills,
            losses: raw.losses,
            built: raw.built,
            score: raw.score,
        });
    }
    // Highest score first. A stable sort keeps ties in house order, so the table
    // is reproducible across peers rather than depending on a sort's tie-break.
    rows.sort_by(|a, b| b.score.cmp(&a.score));

    ScoreScreenModel {
        // A stock offline skirmish takes the skirmish heading; the networked
        // heading belongs to the multiplayer session type.
        title_key: "GUI:SkirmishScore",
        game_number: state.finished_game_count,
        // The clock performs native's signed division first. The existing UI
        // model is unsigned and applies the native 99:59:59 ceiling, so keep
        // the pathological rollover representation local to this boundary.
        elapsed_seconds: elapsed_seconds as u32,
        rows,
    }
}

fn anim_world_sound_screen(world: crate::sim::anim_class::AnimWorldCoord) -> (f32, f32) {
    const CELL_LEPTONS: i32 = 256;
    const HEIGHT_LEVEL_LEPTONS: i32 = 128;
    let rx = world
        .x
        .div_euclid(CELL_LEPTONS)
        .clamp(0, i32::from(u16::MAX)) as u16;
    let ry = world
        .y
        .div_euclid(CELL_LEPTONS)
        .clamp(0, i32::from(u16::MAX)) as u16;
    let sub_x = crate::util::fixed_math::SimFixed::from_num(world.x.rem_euclid(CELL_LEPTONS));
    let sub_y = crate::util::fixed_math::SimFixed::from_num(world.y.rem_euclid(CELL_LEPTONS));
    let z = world
        .z
        .div_euclid(HEIGHT_LEVEL_LEPTONS)
        .clamp(0, i32::from(u8::MAX)) as u8;
    crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeAdvanceMode {
    WallClock { now_ms: u64 },
    ExactOneStep,
}

/// App-local evidence that one tactical diagnostic pump advanced exactly one
/// gameplay frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ExactStepReceipt {
    pub tick_before: u64,
    pub tick_after: u64,
    pub binary_frame_before: u32,
    pub binary_frame_after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ExactStepError {
    #[error("exact tactical step requires an accepted explicit Rust-L0 receipt")]
    MissingAcceptedRustL0,
    #[error("exact tactical step requires the InGame screen")]
    ScreenNotInGame,
    #[error("exact tactical step requires a live simulation")]
    SimulationMissing,
    #[error("exact tactical step advanced {actual} ticks instead of exactly one")]
    TickDelta { actual: u64 },
    #[error("exact tactical step advanced gameplay frame by {actual} instead of exactly one")]
    FrameDelta { actual: u32 },
}

pub(crate) fn monotonic_frame_pacer_ms(state: &AppState, now: Instant) -> u64 {
    crate::app_frame_pacer::wall_clock_ms(state.platform.frame_pacer_epoch, now)
}

/// Front-end session mode, as the modal pump reads it to decide whether the
/// simulation advances behind an open modal dialog. Mirrors gamemd's `g_GameMode`
/// discriminator; the values are writer-proofed — the active engine only ever
/// writes 0/3/4/5, so any other raw value is a legacy (modem/serial) or
/// uninitialized mode the pump treats conservatively as non-advancing. This type
/// is read ONLY by the app loop, never by `sim/` (the layering rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Campaign / single-player. The modal pump freezes the world.
    Campaign,
    /// LAN / IPX network. The modal pump keeps the world advancing.
    Lan,
    /// WOL / Internet network. The modal pump keeps the world advancing.
    Wol,
    /// Offline skirmish. The modal pump freezes the world.
    Skirmish,
    /// Any raw value the active engine never writes (legacy modem/serial, or
    /// uninitialized). Treated as non-advancing.
    Other,
}

impl SessionMode {
    /// Map gamemd's raw `g_GameMode` int to a session mode. Writer-proofed:
    /// 0=Campaign, 3=Lan, 4=Wol, 5=Skirmish; every other value is `Other`.
    pub fn from_game_mode(raw: i32) -> Self {
        match raw {
            0 => SessionMode::Campaign,
            3 => SessionMode::Lan,
            4 => SessionMode::Wol,
            5 => SessionMode::Skirmish,
            _ => SessionMode::Other,
        }
    }

    /// Whether this is a network session (LAN/WOL). Only network sessions keep the
    /// simulation advancing behind an open modal; offline modes freeze it.
    pub fn is_network(self) -> bool {
        matches!(self, SessionMode::Lan | SessionMode::Wol)
    }
}

/// Pure modal-pump decision: should the simulation advance one frame while a
/// modal dialog is open? Mirrors the pump body: advance only on the network
/// branch (LAN/WOL), only when neither service-only blocker is set, and only
/// when no fixed tick is already in progress. Offline campaign/skirmish freeze
/// the world; message, input, and repaint still run in the surrounding loop.
/// Pure and total, so it is unit-tested without an `AppState`. The live
/// app-layer consumer is `decide_runtime_pass`, which composes it with the
/// pacer timing answer and the remaining freeze predicates to gate the
/// one-frame admission inside `advance_in_game_runtime`.
pub fn modal_pump_should_advance_sim(
    mode: SessionMode,
    service_only_blocked: bool,
    reentrancy_in_progress: bool,
) -> bool {
    mode.is_network() && !service_only_blocked && !reentrancy_in_progress
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WallClockServiceAdmission {
    simulation: bool,
    tactical_mutation: bool,
}

/// Map the app's two pause sources onto their distinct production effects.
/// A real in-scenario modal follows the native service pump for both simulation
/// and tactical-view mutation. VERA's developer pause has no native modal, so
/// it stops simulation only and leaves ordinary per-render camera/placement
/// updates admitted.
///
/// gamemd-derived: modal service pump, verified `FUN_00623120 @ 0x00623120`.
fn wall_clock_service_admission(
    paused: bool,
    modal_open: bool,
    mode: SessionMode,
    service_only_blocked: bool,
    reentrancy_in_progress: bool,
) -> WallClockServiceAdmission {
    let tactical_mutation = !modal_open
        || modal_pump_should_advance_sim(mode, service_only_blocked, reentrancy_in_progress);
    WallClockServiceAdmission {
        simulation: if modal_open {
            tactical_mutation
        } else {
            !paused
        },
        tactical_mutation,
    }
}

/// Live front-end session mode for the running client. This build is offline
/// only, and offline campaign and skirmish freeze the world identically behind a
/// modal, so it reports `Skirmish`. When networking lands, this reads the live
/// game-mode discriminator and maps it via `SessionMode::from_game_mode`.
pub(crate) fn current_session_mode(_state: &AppState) -> SessionMode {
    SessionMode::Skirmish
}

/// Raw per-pass predicates for one in-game runtime advance. The wall-clock and
/// exact-step entries fill every field from live state, so the freeze matrix
/// can be exercised from the same predicates production consults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimePassInputs {
    pub(crate) exact_step: bool,
    pub(crate) window_active: bool,
    pub(crate) startup_admitted: bool,
    pub(crate) frame_stepping: bool,
    pub(crate) paused: bool,
    pub(crate) menu_open: bool,
    pub(crate) session_mode: SessionMode,
    /// The frame pacer's pure timing answer (`should_admit` with no pause
    /// block). Pause/menu service blocking is applied by the decision, not by
    /// the pacer consult.
    pub(crate) pacer_timing_admits: bool,
}

/// The admission outcome of one runtime pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimePassDecision {
    pub(crate) run_sim: bool,
    pub(crate) tick_lane: TickLane,
    pub(crate) admitted_by_pacer: bool,
    pub(crate) tactical_mutation: bool,
}

/// Decide one in-game runtime pass from raw predicates. This is the single
/// freeze contract for simulation admission and therefore displayed-credit
/// cadence: paused/menu redraws, inactive windows, missing startup receipts,
/// and closed pacer windows all resolve to `run_sim: false`, and a committed
/// network-modal frame keeps its non-`Ordinary` lane. `sidebar_credit_gate_matrix`
/// pins the matrix through this function.
pub(crate) fn decide_runtime_pass(inputs: RuntimePassInputs) -> RuntimePassDecision {
    if inputs.exact_step {
        // Exact diagnostic stepping is an explicit request, not a wall-clock
        // modal pump: it advances exactly one Ordinary frame even while the
        // app is paused, and never records pacer admission.
        return RuntimePassDecision {
            run_sim: true,
            tick_lane: TickLane::Ordinary,
            admitted_by_pacer: false,
            tactical_mutation: true,
        };
    }
    // The current build has no network-session owner, so neither native
    // service-only blocker can be active here. Networking must supply their
    // combined state when `current_session_mode` becomes live.
    let service = wall_clock_service_admission(
        inputs.paused,
        inputs.menu_open,
        inputs.session_mode,
        false,
        false,
    );
    let pacer_admitted = service.simulation && inputs.pacer_timing_admits;
    let run_sim = inputs.window_active
        && inputs.startup_admitted
        && (inputs.frame_stepping || pacer_admitted);
    let tick_lane = if inputs.paused && inputs.session_mode.is_network() {
        TickLane::NetworkModal
    } else {
        TickLane::Ordinary
    };
    RuntimePassDecision {
        run_sim,
        tick_lane,
        admitted_by_pacer: run_sim && !inputs.frame_stepping && pacer_admitted,
        tactical_mutation: service.tactical_mutation,
    }
}

pub(crate) fn advance_in_game_runtime(state: &mut AppState, now_ms: u64) {
    let startup_admitted = crate::match_bootstrap::accepted_tick_is_admitted(
        state.loaded_startup.as_ref(),
        state.rust_l0_receipt.as_ref(),
    );
    if !startup_admitted {
        log::error!("Accepted match tick blocked: matching Rust L0 receipt is absent");
        // The whole pass is skipped: nothing below may run un-receipted. The
        // runtime decision re-checks the same predicate so the freeze contract
        // stays complete even without this call-site return.
        return;
    }

    advance_in_game_runtime_mode(state, RuntimeAdvanceMode::WallClock { now_ms }, startup_admitted);
}

/// Advance exactly one production simulation step for the hidden tactical
/// checkpoint.
///
/// This is stricter than the ordinary app pump: sandbox admission is not
/// sufficient, the screen must be `InGame`, and the live simulation clock must
/// move by exactly one gameplay frame. The local pacer is re-anchored after
/// the step so a prior wall-clock interval cannot leak into normal admission.
pub(crate) fn advance_in_game_runtime_exact_step(
    state: &mut AppState,
) -> Result<ExactStepReceipt, ExactStepError> {
    let admitted = state.loaded_startup.is_some()
        && state.rust_l0_receipt.is_some()
        && crate::match_bootstrap::accepted_tick_is_admitted(
            state.loaded_startup.as_ref(),
            state.rust_l0_receipt.as_ref(),
        );
    if !admitted {
        return Err(ExactStepError::MissingAcceptedRustL0);
    }
    if state.screen != GameScreen::InGame {
        return Err(ExactStepError::ScreenNotInGame);
    }
    let (tick_before, binary_frame_before) = state
        .sim_runtime
        .as_ref()
        .map(|rt| &rt.simulation)
        .map(|sim| (sim.session.tick, sim.session.binary_frame))
        .ok_or(ExactStepError::SimulationMissing)?;

    advance_in_game_runtime_mode(state, RuntimeAdvanceMode::ExactOneStep, admitted);
    let now_ms = monotonic_frame_pacer_ms(state, Instant::now());
    state.platform.frame_pacer.reanchor(now_ms);

    let (tick_after, binary_frame_after) = state
        .sim_runtime
        .as_ref()
        .map(|rt| &rt.simulation)
        .map(|sim| (sim.session.tick, sim.session.binary_frame))
        .ok_or(ExactStepError::SimulationMissing)?;
    let receipt = ExactStepReceipt {
        tick_before,
        tick_after,
        binary_frame_before,
        binary_frame_after,
    };
    validate_exact_step_receipt(receipt)?;
    Ok(receipt)
}

fn validate_exact_step_receipt(receipt: ExactStepReceipt) -> Result<(), ExactStepError> {
    let tick_delta = receipt.tick_after.saturating_sub(receipt.tick_before);
    if tick_delta != 1 {
        return Err(ExactStepError::TickDelta { actual: tick_delta });
    }
    let frame_delta = receipt
        .binary_frame_after
        .wrapping_sub(receipt.binary_frame_before);
    if frame_delta != 1 {
        return Err(ExactStepError::FrameDelta {
            actual: frame_delta,
        });
    }
    Ok(())
}

fn advance_in_game_runtime_mode(
    state: &mut AppState,
    mode: RuntimeAdvanceMode,
    startup_admitted: bool,
) {
    let frame_stepping =
        matches!(mode, RuntimeAdvanceMode::WallClock { .. }) && state.debug_frame_step_requested;
    let pacer_timing_admits = match mode {
        RuntimeAdvanceMode::WallClock { now_ms } => {
            let game_speed = state.sim_runtime.as_ref().map(|rt| &rt.simulation).map_or_else(
                || state.in_game_options.game_speed.min(6) as u8,
                |sim| sim.session.game_options.game_speed.clamp(0, 6) as u8,
            );
            // Pure timing consult; pause/menu blocking belongs to the decision.
            state
                .platform
                .frame_pacer
                .should_admit(now_ms, game_speed, false)
        }
        RuntimeAdvanceMode::ExactOneStep => false,
    };
    let decision = decide_runtime_pass(RuntimePassInputs {
        exact_step: matches!(mode, RuntimeAdvanceMode::ExactOneStep),
        window_active: state.platform.window_active,
        startup_admitted,
        frame_stepping,
        paused: state.paused,
        menu_open: state.in_game_menu.is_open(),
        session_mode: current_session_mode(state),
        pacer_timing_admits,
    });
    if frame_stepping {
        state.debug_frame_step_requested = false;
        if let RuntimeAdvanceMode::WallClock { now_ms } = mode {
            state.platform.frame_pacer.reanchor(now_ms);
        }
    }

    if decision.run_sim {
        let tick_lane = decision.tick_lane;
        let garrison_flash_start_tick = state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
            .map(|sim| sim.session.tick)
            .unwrap_or(0);
        let frame_committed = advance_one_simulation_frame(state, tick_lane);
        crate::app_sidebar_render::advance_sidebar_credits_after_frame(
            state,
            frame_committed,
            tick_lane,
        );
        if frame_committed && decision.admitted_by_pacer {
            let RuntimeAdvanceMode::WallClock { now_ms } = mode else {
                unreachable!("only wall-clock admission records the frame pacer");
            };
            state.platform.frame_pacer.record_admitted_frame(now_ms);
        }
        // High-frequency EVA state cues (low power / insufficient funds /
        // unit lost) — app-side edge detection over sim state.
        announce_local_state_evas(state);
        let garrison_flash_elapsed_ticks = state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
            .map(|sim| sim.session.tick.saturating_sub(garrison_flash_start_tick))
            .unwrap_or(0);
        crate::app_building_anim::drain_sound_events(state);
        // Building one-shots, refinery particles, and their logic-frame clocks
        // were finalized inside the authoritative sim transaction. Only the
        // independent wall-clock terrain-overlay timer remains app-owned.
        crate::app_building_anim::tick_terrain_overlay_animations(state, 16);
        // Looping slot animations are phased off the logic frame their building
        // was placed, so the base has to be recorded on a sim frame boundary
        // rather than on a render frame.
        crate::app_building_anim::refresh_building_anim_phase_bases(state);
        crate::app_building_anim::tick_garrison_muzzle_flashes(
            state,
            garrison_flash_elapsed_ticks.saturating_mul(u64::from(SIM_TICK_MS)) as u32,
        );
        finish_fire_effect_batch(&mut state.pending_fire_effects);
        crate::app_fire_effects::tick_weapon_muzzle_flashes(state, 16);
        crate::app_chute_anim::tick_parachute_anims(state);
    }

    // Refresh changed point-light producers after the sim step. The queued
    // Cell refresh itself remains all-gathered-before-commit.
    refresh_cell_lighting(state);

    crate::app_building_anim::update_radar_state(state, SIM_TICK_MS as f32);
    crate::app_building_anim::update_power_bar_anim(state);
    crate::app_sidebar_gadgets::update_sidebar_gadget_state(state);
    // Per-frame gadget idle tick (G22 rows 2/3 drag-off/drag-back tracking).
    crate::app_gadget_input::idle_tick(state);
    let music_now_ms = monotonic_frame_pacer_ms(state, Instant::now());
    if let (Some(player), Some(assets)) = (&mut state.music_player, &state.asset_manager) {
        player.update(assets, music_now_ms);
    }
    if decision.tactical_mutation {
        crate::app_camera::update_camera(state);
        update_building_placement_preview(state);
    }
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    state.batch_renderer.update_camera(
        &state.gpu,
        sw,
        sh,
        state.camera_x,
        state.camera_y,
        state.zoom_level,
    );
}

/// Tick simulation: advance movement and animation systems.
fn should_record_replay_tick(
    tick_result: &crate::sim::world::TickResult,
    due_commands: &[crate::sim::command::CommandEnvelope],
) -> bool {
    tick_result.frame_committed
        || !due_commands.is_empty()
        || tick_result.terminal_score_finalized
}

fn advance_one_simulation_frame(state: &mut AppState, tick_lane: TickLane) -> bool {
    let mut refresh_atlases_after_tick = false;
    let runtime_active = state.sim_runtime.is_some() || !state.trigger_graph.triggers.is_empty();
    if !runtime_active {
        return false;
    }
    let mut frame_committed = state.sim_runtime.is_none();

    if let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) {
        if sim.replay_log.is_none() {
            sim.replay_log = Some(ReplayLog::new(ReplayHeader {
                version: 1,
                tick_hz: SIM_TICK_HZ,
                seed: sim.session.seed,
                // Scenario identity is session state — the header derives
                // from the sim, not from app-resident view fields.
                map_name: sim.session.map_name.clone(),
                rules_hash: state.rules.as_ref().map(rules_hash).unwrap_or(0),
            }));
        }
    }

    begin_fire_effect_batch(&mut state.pending_fire_effects);

    for _ in 0..1 {
        // Compute local owner before mutable borrow of simulation.
        let local_owner_for_fog = preferred_local_owner_name(state);
        let trigger_inputs = TriggerInputs {
            graph: &state.trigger_graph,
            triggers: &state.triggers,
            events: &state.events,
            actions: &state.actions,
        };

        // Cache local owner name before mutable sim borrow (avoids borrow conflict).
        let local_owner_name = crate::app_commands::preferred_local_owner_name(state);
        let mut drained_fire_events: Vec<SimFireEvent> = Vec::new();
        let mut drained_lifecycle_outputs: Vec<LifecycleOutput> = Vec::new();
        let mut drained_combat_lights = Vec::new();
        let mut frame_overlay_updates = Vec::new();
        let mut trigger_effects: Vec<TriggerEffect> = Vec::new();
        // Carried out of the sim borrow so the census can read `state` freely below.
        let mut census_tick: Option<u64> = None;
        if let Some(rt) = state.sim_runtime.as_mut() {
            let resources = &rt.resources;
            let sim = &mut rt.simulation;
            // Delay-zero AnimClass construction can emit StartSound during the
            // final map-load sweep. Keep it until this first tactical drain;
            // `drain(..)` below still consumes every event exactly once.
            let due_commands = if tick_lane == TickLane::Ordinary {
                sim.take_due_commands()
            } else {
                Vec::new()
            };
            let SimFrameOutput {
                tick: tick_result,
                trigger_effects: frame_trigger_effects,
                lifecycle_outputs: frame_lifecycle_outputs,
                overlay_updates,
                sound_events: frame_sound_events,
                fire_events: frame_fire_events,
                invulnerability_impacts,
            } = sim.advance_app_frame(
                &due_commands,
                state.rules.as_ref(),
                &resources.height_map,
                state.overlay_registry.as_ref(),
                SIM_TICK_MS,
                tick_lane,
                Some(trigger_inputs),
            );
            trigger_effects = frame_trigger_effects;
            frame_overlay_updates = overlay_updates;
            frame_committed = tick_result.frame_committed;
            if tick_result.frame_committed {
                drained_combat_lights = crate::app_combat_lights::materialize_simulation_impacts(
                    invulnerability_impacts,
                    state.rules.as_ref(),
                    &sim.interner,
                );
            }
            // Parity capture, if requested. The sim has already finalized all
            // authoritative animation and particle work, so this observes the
            // exact state covered by the returned frame hash.
            if tick_result.frame_committed {
                if let Some(sink) = state.parity_digest_sink.as_mut() {
                    let digest = sim.parity_digest();
                    if let Err(error) = sink.write(&digest) {
                        // A failing diagnostic must never take the game down with it.
                        log::error!("parity digest write failed, disabling capture: {error}");
                        state.parity_digest_sink = None;
                    }
                }
            }
            census_tick = tick_result.frame_committed.then_some(tick_result.tick);
            drained_lifecycle_outputs = frame_lifecycle_outputs;
            // Pre-merge fog visibility for local owner so render queries are O(1).
            if let Some(owner) = &local_owner_for_fog {
                if sim.session.tick == 1 {
                    log::info!("Fog merged for local owner: '{}'", owner);
                }
                if let Some(owner_id) = sim.interner.get(owner) {
                    sim.fog.build_merged_for(owner_id, &sim.interner);
                }
            }
            // Drain fire events for render-side muzzle flash / projectile origin.
            drained_fire_events = frame_fire_events;
            append_fire_effect_batch(&mut state.pending_fire_effects, &drained_fire_events);
            // Convert sim sound events to app-layer sound events for playback.
            for sim_event in frame_sound_events {
                let app_event: GameSoundEvent = match sim_event {
                    SimSoundEvent::AnimationStarted {
                        anim_id,
                        sound_id,
                        world,
                    } => {
                        let (sx, sy) = anim_world_sound_screen(world);
                        GameSoundEvent::AnimationStarted {
                            anim_id,
                            sound_id: sim.interner.resolve(sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::AnimationStopped {
                        anim_id,
                        stop_sound_id,
                        world,
                    } => {
                        let (sx, sy) = anim_world_sound_screen(world);
                        GameSoundEvent::AnimationStopped {
                            anim_id,
                            stop_sound_id: stop_sound_id
                                .map(|id| sim.interner.resolve(id).to_string()),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::WeaponFired {
                        report_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::WeaponFired {
                            sound_id: sim.interner.resolve(report_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityDied {
                        die_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityDestroyed {
                            sound_id: sim.interner.resolve(die_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityCrushed {
                        crush_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityCrushed {
                            sound_id: sim.interner.resolve(crush_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityDeployed {
                        deploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityDeployed {
                            sound_id: sim.interner.resolve(deploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityUndeployed {
                        undeploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityUndeployed {
                            sound_id: sim.interner.resolve(undeploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::DockDeploy { .. } => {
                        // TODO: resolve building's deploy sound from art.ini
                        // and select healthy/damaged variant based on health ratio.
                        continue;
                    }
                    SimSoundEvent::ChronoTeleport { sound_id, rx, ry } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::ChronoTeleport {
                            sound_id: sim.interner.resolve(sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BuildingComplete { owner } => {
                        // Only play EVA for the local player's production.
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_ConstructionComplete", faction)
                            .unwrap_or("ceva048")
                            .to_string();
                        GameSoundEvent::BuildingReady { sound_id }
                    }
                    SimSoundEvent::SuperWeaponLaunched { .. } => {
                        // TODO: play EVA superweapon warning sound.
                        continue;
                    }
                    SimSoundEvent::SuperWeaponStrike { .. } => {
                        // TODO: play lightning bolt thunder sound.
                        continue;
                    }
                    SimSoundEvent::UnitComplete { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_UnitReady", faction)
                            .unwrap_or("ceva062")
                            .to_string();
                        GameSoundEvent::UnitReady { sound_id }
                    }
                    SimSoundEvent::MatchOutcome { owner, kind } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .is_some_and(|local| local.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let (eva_key, fallback) = outcome_eva_entry(kind, faction);
                        let eva_sound_id = state
                            .eva_registry
                            .get(eva_key, faction)
                            .unwrap_or(fallback)
                            .to_string();
                        GameSoundEvent::OutcomeEva { eva_sound_id }
                    }
                    SimSoundEvent::WallSold { receiver } => {
                        let receiver_name = sim.interner.resolve(receiver);
                        let Some(event) = wall_sell_sound_for_local(
                            receiver_name,
                            local_owner_name.as_deref(),
                            state.rules.as_ref(),
                        ) else {
                            continue;
                        };
                        event
                    }
                    SimSoundEvent::CannotDeployHere { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_CannotDeployHere", faction)
                            .unwrap_or("ceva063")
                            .to_string();
                        GameSoundEvent::CannotDeployHere { sound_id }
                    }
                    SimSoundEvent::StructureGarrisoned { owner } => {
                        // EVA cue: only play for the local human player.
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_StructureGarrisoned", faction)
                            .unwrap_or("ceva107")
                            .to_string();
                        GameSoundEvent::StructureGarrisoned { sound_id }
                    }
                    SimSoundEvent::StructureAbandoned { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_StructureAbandoned", faction)
                            .unwrap_or("ceva108")
                            .to_string();
                        GameSoundEvent::StructureAbandoned { sound_id }
                    }
                    SimSoundEvent::BuildingGarrisonedSfx { owner, rx, ry } => {
                        // Positional SFX: only audible to the local human player
                        // (matches gamemd VocClass::PlayAt with IsHumanPlayer gate).
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.building_garrisoned_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BuildingGarrisonedSfx {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::ChuteSound { rx, ry } => {
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.chute_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::ChuteSound {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::C4Planted { rx, ry } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::C4Planted {
                            sound_id: "SealPlaceBomb".to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::RefineryExitSfx { rx, ry } => {
                        // Positional SFX from [AudioVisual] BunkerWallsDownSound.
                        // Skip when rules don't configure the sound (matches
                        // gamemd's `RulesClass+0x244 != -1` guard).
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_down_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::RefineryExitSfx {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BunkerWallsUp { rx, ry } => {
                        // Walls-up cue on install; skip when the rules key is empty.
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_up_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BunkerWalls {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BunkerWallsDown { rx, ry } => {
                        // Walls-down cue on normal exit / clear teardown.
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_down_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BunkerWalls {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BridgeRepaired {
                        rx,
                        ry,
                        owner,
                        eva_allowed,
                    } => {
                        // Spatial SFX gated on rules.bridge_rules.repair_sound
                        // being set (the original game gates on
                        // `RulesClass+0x248 != -1`).
                        let sound_id = state
                            .rules
                            .as_ref()
                            .and_then(|r| r.bridge_rules.repair_sound.clone())
                            .unwrap_or_default();
                        let screen_pos = if sound_id.is_empty() {
                            None
                        } else {
                            let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                            Some((sx, sy))
                        };
                        // EVA cue gated on local-human owner. Resolves
                        // `EVA_BridgeRepaired` from the registry (no faction
                        // fallback needed — bridge repair is faction-agnostic).
                        let owner_str = sim.interner.resolve(owner);
                        let eva_sound_id = if eva_allowed
                            && local_owner_name
                                .as_deref()
                                .is_some_and(|l| l.eq_ignore_ascii_case(owner_str))
                        {
                            let faction = crate::app_building_anim::eva_faction_key(
                                owner_str,
                                &state.house_roster,
                            );
                            state
                                .eva_registry
                                .get("EVA_BridgeRepaired", faction)
                                .map(|s| s.to_string())
                        } else {
                            None
                        };
                        if sound_id.is_empty() && eva_sound_id.is_none() {
                            continue;
                        }
                        GameSoundEvent::BridgeRepaired {
                            sound_id,
                            screen_pos,
                            eva_sound_id,
                        }
                    }
                    SimSoundEvent::UnderAttack {
                        owner,
                        miner,
                        eva_allowed,
                        ..
                    } => {
                        // Voice for the LOCAL player only; the radar diamond is
                        // sim-side (owner-scoped) and needs nothing here.
                        let owner_str = sim.interner.resolve(owner);
                        let is_local = local_owner_name
                            .as_deref()
                            .is_some_and(|l| l.eq_ignore_ascii_case(owner_str));
                        if !eva_allowed || !is_local {
                            continue;
                        }
                        // Repeat cooldown across both cue kinds (the native
                        // per-house attack-voice delay is UNVERIFIED — see the
                        // field doc on AppState).
                        if sim.session.tick < state.eva_under_attack_block_until_tick {
                            continue;
                        }
                        state.eva_under_attack_block_until_tick =
                            sim.session.tick + EVA_UNDER_ATTACK_COOLDOWN_TICKS;
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let (cue, fallback) = if miner {
                            ("EVA_OreMinerUnderAttack", "ceva037")
                        } else {
                            ("EVA_OurBaseIsUnderAttack", "ceva054")
                        };
                        let eva_sound_id = state
                            .eva_registry
                            .get(cue, faction)
                            .unwrap_or(fallback)
                            .to_string();
                        GameSoundEvent::UnderAttackEva { eva_sound_id }
                    }
                    SimSoundEvent::WorldEffectStarted {
                        sound_id,
                        rx,
                        ry,
                        sub_x,
                        sub_y,
                        z,
                    } => {
                        let (sx, sy) =
                            crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z);
                        GameSoundEvent::WorldEffectStarted {
                            sound_id: sim.interner.resolve(sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                };
                state.sound_events.push(app_event);
            }
            if tick_result.destroyed_structure {
                refresh_atlases_after_tick = true;
            }
            if tick_result.bridge_state_changed {
                refresh_atlases_after_tick = true;
            }
            if tick_result.ownership_changed {
                refresh_atlases_after_tick = true;
            }
            if tick_result.spawned_entities {
                refresh_atlases_after_tick = true;
            }
            // Both terminal routes belong to the deterministic stream even
            // though Main_Tick skips its frame commit. EventClass EXIT carries
            // a command; natural win/loss carries the one-shot score/RNG latch.
            if should_record_replay_tick(&tick_result, &due_commands) {
                if let Some(log) = &mut sim.replay_log {
                    log.record_tick(tick_result.tick, due_commands, tick_result.state_hash);
                }
            }
        }
        if frame_committed {
            state.combat_lights.commit_frame(drained_combat_lights);
        }
        crate::app_input::reconcile_selection_order_after_sim(state);
        // Rendering is rebuilt from lifecycle facts every frame. Replay the
        // app-owned transactions in native emission order for state that has a
        // direct attachment or retained audio handle.
        for output in drained_lifecycle_outputs {
            match output {
                LifecycleOutput::DetachAttachedAnims { stable_id } => {
                    state
                        .garrison_muzzle_flashes
                        .retain(|flash| flash.building_id != stable_id);
                    state
                        .parachute_anims
                        .retain(|anim| anim.target_id != stable_id);
                }
                LifecycleOutput::StopVoc { stable_id } => {
                    if let Some(sfx) = state.sfx_player.as_mut() {
                        sfx.stop_animation_sound(stable_id);
                    }
                }
                LifecycleOutput::DisplayRemove { .. } => {
                    refresh_atlases_after_tick = true;
                }
                LifecycleOutput::RevealDisplay { .. }
                | LifecycleOutput::DirtyTacticalRect { .. }
                | LifecycleOutput::ClearDrawnState { .. }
                | LifecycleOutput::ClearRedraw { .. } => {}
            }
        }
        crate::app_fire_effects::spawn_non_garrison_fire_effects(state, &drained_fire_events);

        // Black-cell census, on a schedule rather than a keypress: nobody hits a debug key
        // at the exact moment they notice the artifact. Spread-out ticks give a time series,
        // which separates "still exploring" from "these cells will never be revealed" — a
        // shrinking count is normal, a count that plateaus with a stubborn remainder is the
        // bug. Runs after the sim borrow ends so it can read the atlas and terrain grid.
        // Early sample, then once a minute for as long as the match runs. A fixed set of
        // early ticks missed the symptom entirely: the interesting state is a mostly-explored
        // map with holes left in it, which only appears well into a session.
        if census_tick.is_some_and(|tick| tick == 150 || (tick > 150 && tick % 900 == 0)) {
            crate::app_input::report_black_cell_causes(state);
        }

        apply_trigger_effects(state, &trigger_effects);

        // Simulation has already finalized identity, passability, navigation,
        // and the returned hash. The app only updates its render-side list.
        if !frame_overlay_updates.is_empty() {
            upsert_occupied_overlay_render_entries(state, frame_overlay_updates);
        }
    }

    // Entity identity or ownership changes require presentation atlas refresh.
    if refresh_atlases_after_tick {
        refresh_entity_atlases(state);
    }
    frame_committed
}

/// Samples pending light records backward and swaps the completed grid forward,
/// matching YR `LightSourceClass::UpdateLightConverts`' all-gathered-before-commit
/// boundary. This is app-local renderer state, never deterministic simulation state.
const CELL_LIGHT_GATHER_BUDGET: usize = 8_192;

/// Per-frame point-light refresh. A changed producer set schedules a deferred
/// Cell light refresh; the visible grid remains stable until every replacement
/// cell has been sampled.
fn refresh_cell_lighting(state: &mut AppState) {
    let changed_view = {
        let (Some(sim), Some(rules), Some(terrain)) = (
            state.sim_runtime.as_ref().map(|rt| &rt.simulation),
            state.rules.as_ref(),
            state.resolved_terrain.as_ref(),
        ) else {
            return;
        };
        let view = crate::app_init::derive_lighting_view(
            &state.map_lighting_config,
            Some(sim),
            Some(rules),
            state.in_game_options.detail_level,
        );
        if state.last_lighting_view_fingerprint == Some(view.fingerprint) {
            None
        } else {
            let profile_changed = state.applied_lighting_profile != Some(view.profile)
                || state.applied_lighting_detail_level != view.detail_level;
            let affected_cells = if profile_changed {
                terrain
                    .iter()
                    .map(|cell| ((cell.rx, cell.ry), cell.level))
                    .collect()
            } else {
                // Source identity is not projected into PointLight. Enumerate
                // the union of old and new source areas so identical colocated
                // sources and multiplicity changes cannot disappear in a set diff.
                let mut seen = std::collections::BTreeSet::new();
                let mut cells = Vec::new();
                for source in state
                    .applied_lighting_sources
                    .iter()
                    .chain(view.point_lights.iter())
                {
                    for record in crate::map::lighting::point_light_area_cells(
                        source,
                        terrain.width(),
                        terrain.height(),
                        |rx, ry| terrain.cell(rx, ry).map(|cell| cell.level),
                    ) {
                        if seen.insert(record.0) {
                            cells.push(record);
                        }
                    }
                }
                cells
            };
            Some((view, affected_cells))
        }
    };

    if let Some((view, affected_cells)) = changed_view {
        // A new queued source flushes the old batch before its area is enumerated.
        if let Some(mut pending) = state.pending_lighting_refresh.take() {
            pending.gather_all();
            let committed = pending.commit_into(&mut state.lighting_grid);
            debug_assert!(committed);
        }
        state.last_lighting_view_fingerprint = Some(view.fingerprint);
        state.applied_lighting_profile = Some(view.profile);
        state.applied_lighting_detail_level = view.detail_level;
        state.applied_lighting_sources = view.point_lights.clone();
        state.pending_lighting_refresh = (!affected_cells.is_empty()).then(|| {
            crate::map::lighting::DeferredCellLightRefresh::new_with_profile(
                affected_cells,
                view.profile,
                view.detail_level,
                view.point_lights,
            )
        });
    }

    let completed = state
        .pending_lighting_refresh
        .as_mut()
        .is_some_and(|pending| pending.gather(CELL_LIGHT_GATHER_BUDGET));
    if completed {
        let pending = state
            .pending_lighting_refresh
            .take()
            .expect("completed lighting refresh remains installed");
        let committed = pending.commit_into(&mut state.lighting_grid);
        debug_assert!(committed, "completed lighting refresh commits atomically");
    }
}

fn begin_fire_effect_batch(pending: &mut Vec<SimFireEvent>) {
    pending.clear();
}

fn append_fire_effect_batch(pending: &mut Vec<SimFireEvent>, events: &[SimFireEvent]) {
    pending.extend(events.iter().cloned());
}

fn finish_fire_effect_batch(pending: &mut Vec<SimFireEvent>) {
    pending.clear();
}

fn apply_trigger_effects(state: &mut AppState, effects: &[TriggerEffect]) {
    for effect in effects {
        match effect {
            TriggerEffect::CenterCameraAtWaypoint {
                waypoint,
                immediate: _,
            } => center_camera_on_waypoint(state, *waypoint),
            TriggerEffect::MissionAnnouncement { text } => {
                // gamemd routes trigger text through the message list
                // (contract lane §4.5: the native trigger-text path is a
                // message-list producer).
                crate::app_messages::post_system_message(state, text);
            }
            TriggerEffect::MissionResult { title, detail } => {
                state.screen = GameScreen::MissionResult {
                    title: title.clone(),
                    detail: detail.clone(),
                };
            }
        }
    }
}

fn center_camera_on_waypoint(state: &mut AppState, waypoint_index: u32) {
    let Some(waypoint) = state.waypoints.get(&waypoint_index) else {
        log::warn!(
            "Trigger action requested missing waypoint {} for camera centering",
            waypoint_index
        );
        return;
    };
    let (rx, ry) = (waypoint.rx, waypoint.ry);
    // Centres on the tactical viewport, not the window.
    crate::app_camera::center_camera_on_cell(state, rx, ry);
}

pub(crate) fn update_building_placement_preview(state: &mut AppState) {
    let Some(type_id) = state.armed_building_type() else {
        state.building_placement_preview = None;
        return;
    };
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    let (Some(sim), Some(rules)) = (state.sim_runtime.as_ref().map(|rt| &rt.simulation), &state.rules) else {
        state.building_placement_preview = None;
        return;
    };
    // Offset so the foundation shadow centers on the cursor, not top-left corner.
    let (fw, fh, foundation_str) = rules
        .object(type_id)
        .map(|obj| {
            let (w, h) = production::foundation_dimensions(&obj.foundation);
            (w, h, obj.foundation.clone())
        })
        .unwrap_or((1, 1, "1x1".to_string()));
    // Log at info level once per type_id change so it's visible in console.
    static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let hash: u64 = type_id
        .as_bytes()
        .iter()
        .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
    if LAST_LOG.swap(hash, std::sync::atomic::Ordering::Relaxed) != hash {
        log::info!(
            "Placement preview: type={} foundation=\"{}\" → {}x{}",
            type_id,
            foundation_str,
            fw,
            fh,
        );
    }
    // Place the foundation with cursor cell as the top-left corner.
    // The building sprite is anchored on the north-west footprint cell's tile row
    // — the entity anchor with the render-coordinate lift taken off — and
    // `build_ghost_sprite` derives the preview from the same helper, so the
    // preview and the placed building always align.
    let (rx, ry) = screen_point_to_world_cell(state, state.cursor_x, state.cursor_y);
    state.building_placement_preview = production::placement_preview_for_owner_with_overlays(
        sim,
        rules,
        &owner,
        type_id,
        rx,
        ry,
        sim.path_grid(),
        &state.height_map(),
        state.overlay_registry.as_ref(),
    );
}

/// Refresh entity atlases after new entities are spawned.
///
/// Uses an incremental approach: first checks if the existing atlases already
/// contain all needed sprite keys. If so, skips the expensive rebuild entirely.
/// Only performs a full rebuild when genuinely new sprite types appear.
/// Reuses `state.asset_manager` instead of creating a new one (avoids re-opening
/// all MIX archives from disk).
pub(crate) fn refresh_entity_atlases(state: &mut AppState) {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else { return };
    let Some(asset_manager) = &state.asset_manager else {
        log::warn!("Atlas refresh skipped: no asset manager available");
        return;
    };

    // Check if unit atlas needs rebuilding (new voxel entity types appeared).
    let unit_needed = unit_atlas::collect_needed_unit_keys(
        sim.entities(),
        asset_manager,
        state.rules.as_ref(),
        state.rules.as_ref().map(|rules| &rules.art_registry),
        Some(&sim.interner),
    );
    let unit_rebuild: bool = match &state.unit_atlas {
        Some(atlas) => !atlas.has_all_keys(&unit_needed),
        None => !unit_needed.is_empty(),
    };

    // Check if sprite atlas needs rebuilding (new SHP entity types appeared).
    let extra_buildings: Vec<&str> = crate::app_skirmish::deployable_building_types(
        sim.entities(),
        state.rules.as_ref(),
        Some(&sim.interner),
    );
    let sprite_base_keys = sprite_atlas::collect_needed_base_keys(
        sim.entities(),
        &state.house_color_map,
        &extra_buildings,
        Some(&sim.interner),
    );
    let sprite_rebuild: bool = match &state.sprite_atlas {
        Some(atlas) => !sprite_atlas::atlas_covers_base_keys(atlas, &sprite_base_keys),
        None => !sprite_base_keys.is_empty(),
    };

    // Early out: no new sprite types → skip the expensive atlas rebuild.
    if !unit_rebuild && !sprite_rebuild {
        log::debug!("Atlas refresh: no new sprite types — skipping rebuild");
        return;
    }

    let unit_palette = load_unit_palette(asset_manager, &state.theater_ext);
    let Some(palette) = unit_palette else {
        log::warn!("Atlas refresh skipped: unit palette not found");
        return;
    };

    if unit_rebuild {
        log::info!("Rebuilding unit atlas: new voxel entity types detected");
        let existing = state.unit_atlas.take();
        if let Some(new_unit_atlas) = unit_atlas::build_unit_atlas(
            &state.gpu,
            &state.batch_renderer,
            sim.entities(),
            asset_manager,
            state.rules.as_ref(),
            state.rules.as_ref().map(|rules| &rules.art_registry),
            existing,
            state.vxl_compute.as_mut(),
            Some(&sim.interner),
        ) {
            state.unit_atlas = Some(new_unit_atlas);
        }
    }

    if sprite_rebuild {
        log::warn!(">>> SPRITE ATLAS REBUILD TRIGGERED — new SHP entity types detected <<<");
        let existing = state.sprite_atlas.take();
        let cell_drawer_type_ids: HashSet<String> = sim
            .resolved_terrain
            .as_ref()
            .into_iter()
            .flat_map(|terrain| terrain.tile_animations())
            .map(|anim| anim.anim_name.to_ascii_uppercase())
            .collect();
        let cell_palette = load_iso_palette(asset_manager, &state.theater_ext);
        if let Some(new_sprite_atlas) = sprite_atlas::build_sprite_atlas(
            &state.gpu,
            &state.batch_renderer,
            sim.entities(),
            asset_manager,
            &palette,
            &state.theater_ext,
            &state.theater_name,
            state.rules.as_ref(),
            state.rules.as_ref().map(|rules| &rules.art_registry),
            &state.house_color_map,
            &extra_buildings,
            &cell_drawer_type_ids,
            cell_palette.as_ref(),
            existing,
            Some(&sim.interner),
        ) {
            state.sprite_atlas = Some(new_sprite_atlas);
        }
    }
}

/// Upsert authoritative occupied-overlay entries into `state.overlays`.
///
/// Background: the overlay renderer iterates `state.overlays`, the static list
/// loaded from the map's `[OverlayPack]`. Sim-side mutations that create new
/// overlay cells (TIBTRE ore spawn, ore_growth spread) update `OverlayGrid`
/// but never touched `state.overlays`, so the new cells were invisible even
/// though their sim state and pathfinding were correct. A coordinate can also
/// be cleared and later receive a different overlay variant; the renderer only
/// accepts live data when the cached identity matches. This sync therefore
/// inserts absent coordinates and updates identity plus frame in place for
/// existing coordinates. Candidates may be one frame's delta or the full live
/// occupied set returned after snapshot restoration.
///
/// Cleared cached entries are render-inert because the renderer treats live
/// `OverlayGrid` state as authoritative; a later occupied update or post-load
/// snapshot replaces their stale identity. Unrelated entries retain their order
/// and fields.
pub(crate) fn upsert_occupied_overlay_render_entries(
    state: &mut AppState,
    candidates: Vec<crate::map::overlay::OverlayEntry>,
) {
    let synced = upsert_overlay_entries(&mut state.overlays, candidates);
    if synced != 0 {
        log::trace!(
            "Synced {} occupied cells from OverlayGrid to state.overlays",
            synced
        );
    }
}

/// Upsert authoritative occupied cells by coordinate. Returns the number of
/// entries inserted or changed.
fn upsert_overlay_entries(
    existing: &mut Vec<crate::map::overlay::OverlayEntry>,
    candidates: Vec<crate::map::overlay::OverlayEntry>,
) -> usize {
    let mut by_coordinate: std::collections::HashMap<(u16, u16), usize> = existing
        .iter()
        .enumerate()
        .map(|(index, entry)| ((entry.rx, entry.ry), index))
        .collect();
    let mut synced = 0;
    for candidate in candidates {
        let coordinate = (candidate.rx, candidate.ry);
        if let Some(&index) = by_coordinate.get(&coordinate) {
            let entry = &mut existing[index];
            if entry.overlay_id != candidate.overlay_id || entry.frame != candidate.frame {
                entry.overlay_id = candidate.overlay_id;
                entry.frame = candidate.frame;
                synced += 1;
            }
        } else {
            by_coordinate.insert(coordinate, existing.len());
            existing.push(candidate);
            synced += 1;
        }
    }
    synced
}

fn load_unit_palette(asset_manager: &AssetManager, theater_ext: &str) -> Option<Palette> {
    let themed = format!("unit{}.pal", theater_ext.to_ascii_lowercase());
    let candidates = [
        themed.as_str(),
        "unittem.pal",
        "unitsno.pal",
        "uniturb.pal",
        "unit.pal",
        "temperat.pal",
    ];
    for name in candidates {
        let Some(data) = asset_manager.get(name) else {
            continue;
        };
        if let Ok(pal) = Palette::from_bytes(&data) {
            return Some(pal);
        }
    }
    None
}

fn load_iso_palette(asset_manager: &AssetManager, theater_ext: &str) -> Option<Palette> {
    let name = format!("iso{}.pal", theater_ext.to_ascii_lowercase());
    asset_manager
        .get_ref(&name)
        .and_then(|bytes| Palette::from_bytes(bytes).ok())
}

/// Check if a cell is walkable on either the ground or bridge layer.
/// Delegates to the unified `PathGrid::is_any_layer_walkable()` method.
pub(crate) fn is_any_layer_walkable(
    grid: &crate::sim::pathfinding::PathGrid,
    x: u16,
    y: u16,
) -> bool {
    grid.is_any_layer_walkable(x, y)
}

pub(crate) fn screen_point_to_world(state: &AppState, screen_x: f32, screen_y: f32) -> (f32, f32) {
    // Screen pixel / zoom = world offset from camera top-left.
    (
        screen_x / state.zoom_level + state.camera_x,
        screen_y / state.zoom_level + state.camera_y,
    )
}

/// Shared owner for world-space point -> map-cell resolution in the app layer.
///
/// Any app code that already has world coordinates should use this instead of
/// re-calling the tactical inverse inline.
pub(crate) fn world_point_to_cell(
    world_x: f32,
    world_y: f32,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_cells: Option<
        &std::collections::BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    >,
) -> (u16, u16) {
    let inverse = terrain::screen_to_cell_tactical_inverse(
        world_x,
        world_y,
        terrain::TacticalInverseContext {
            height_map,
            bridge_cells,
            viewport_offset_x: 0.0,
            viewport_offset_y: 0.0,
        },
    );
    let (iso_rx, iso_ry) = match inverse {
        terrain::TacticalInverseResult::Cell { rx, ry }
        | terrain::TacticalInverseResult::Fallback { rx, ry } => (rx, ry),
    };
    (
        // Current Rust app callers expect a concrete in-map cell. Keep this
        // clamp isolated here until off-map sentinel behavior is modeled.
        iso_rx.round().max(0.0) as u16,
        iso_ry.round().max(0.0) as u16,
    )
}

/// Shared owner for screen-space cursor -> map-cell resolution in the app layer.
///
/// This is the entry point UI/input code should use when starting from screen
/// coordinates and the current camera.
pub(crate) fn screen_point_to_world_cell(
    state: &AppState,
    screen_x: f32,
    screen_y: f32,
) -> (u16, u16) {
    let (world_x, world_y) = screen_point_to_world(state, screen_x, screen_y);
    world_point_to_cell(
        world_x,
        world_y,
        &state.height_map(),
        Some(&state.tactical_bridge_inverse_map),
    )
}

pub(crate) fn nearest_walkable_cell(
    grid: &crate::sim::pathfinding::PathGrid,
    start: (u16, u16),
    max_radius: u16,
) -> Option<(u16, u16)> {
    grid.nearest_walkable_any_layer(start.0, start.1, max_radius, None, None)
}

pub(crate) fn nearest_walkable_cell_layered(
    grid: &crate::sim::pathfinding::PathGrid,
    start: (u16, u16),
    max_radius: u16,
) -> Option<(u16, u16)> {
    grid.nearest_walkable_any_layer(start.0, start.1, max_radius, None, None)
}

pub(crate) fn clamp_cell_to_grid(
    grid: &crate::sim::pathfinding::PathGrid,
    cell: (u16, u16),
) -> (u16, u16) {
    let max_x = grid.width().saturating_sub(1);
    let max_y = grid.height().saturating_sub(1);
    (cell.0.min(max_x), cell.1.min(max_y))
}

pub(crate) fn rules_hash(rules: &crate::rules::ruleset::RuleSet) -> u64 {
    // Compatibility covers the processed rules layers plus resolved entity
    // animation, effect/particle timing, terrain-spawner raw frame timing, and
    // smudge-selection frame dimensions.
    // Compatibility must distinguish static inputs that can advance the same
    // entity differently.
    rules.simulation_config_hash()
}

#[cfg(test)]
mod tests {
    use super::{
        ExactStepError, ExactStepReceipt, append_fire_effect_batch, begin_fire_effect_batch,
        finish_fire_effect_batch, outcome_eva_entry, upsert_overlay_entries,
        validate_exact_step_receipt, wall_sell_sound_for_local, world_point_to_cell,
    };
    use crate::map::entities::EntityCategory;
    use crate::map::overlay::OverlayEntry;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
    use crate::rules::locomotor_type::SpeedType;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::combat::TargetKind;
    use crate::sim::combat::combat_weapon::WeaponSlot;
    use crate::sim::intern::{InternedId, StringInterner, test_intern};
    use crate::sim::terrain_object::{
        TerrainObjectLifecycle, TerrainObjectState, mark_terrain_occupation,
        unmark_terrain_occupation,
    };
    use crate::sim::world::{FireOriginSnapshot, SimFireEvent};
    use crate::util::fixed_math::SimFixed;
    use std::collections::BTreeMap;

    fn entry(rx: u16, ry: u16, overlay_id: u8, frame: u8) -> OverlayEntry {
        OverlayEntry {
            rx,
            ry,
            overlay_id,
            frame,
        }
    }

    #[test]
    fn gsi_01_04_outcome_transition_resolves_exact_standard_eva_entries() {
        use crate::sim::house_state::HouseOutcomeKind;

        assert_eq!(
            outcome_eva_entry(HouseOutcomeKind::Victory, "Allied"),
            ("EVA_YouAreVictorious", "ceva022")
        );
        assert_eq!(
            outcome_eva_entry(HouseOutcomeKind::Victory, "Russian"),
            ("EVA_YouAreVictorious", "csof022")
        );
        assert_eq!(
            outcome_eva_entry(HouseOutcomeKind::Defeat, "Yuri"),
            ("EVA_YouHaveLost", "cyur023")
        );
    }

    #[test]
    fn gsi_04_07_wall_sell_sound_is_global_only_for_local_receiver() {
        let rules =
            crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
                "[General]\nFixtureOnly=1\n[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
                 [AudioVisual]\nSellSound=SellBuilding\n",
            ))
            .unwrap();
        let mut sim = crate::sim::world::Simulation::new();
        let receiver = sim.interner.intern("Receiver");
        let receiver_name = sim.interner.resolve(receiver);

        assert!(matches!(
            wall_sell_sound_for_local(receiver_name, Some("receiver"), Some(&rules)),
            Some(crate::audio::events::GameSoundEvent::UiSound { sound_id })
                if sound_id == "SellBuilding"
        ));
        assert!(
            wall_sell_sound_for_local(receiver_name, Some("WallOwner"), Some(&rules)).is_none()
        );
        assert!(wall_sell_sound_for_local(receiver_name, None, Some(&rules)).is_none());

        let no_sound =
            crate::rules::ruleset::RuleSet::from_ini(&crate::rules::ini_parser::IniFile::from_str(
                "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n",
            ))
            .unwrap();
        assert!(
            wall_sell_sound_for_local(receiver_name, Some("Receiver"), Some(&no_sound)).is_none()
        );
    }

    #[test]
    fn gsi_04_10_dynamic_navigation_rebuild_refreshes_path_and_track_costs() {
        let rules = crate::rules::ruleset::RuleSet::from_ini(
            &crate::rules::ini_parser::IniFile::from_str(
                "[InfantryTypes]\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n",
            ),
        )
        .unwrap();
        let speed_costs = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            float: Some(100),
            amphibious: Some(100),
            float_beach: Some(100),
            hover: Some(100),
        };
        let terrain = ResolvedTerrainGrid::from_cells(
            1,
            1,
            vec![ResolvedTerrainCell {
                rx: 0,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: None,
                land_type: LandType::Clear.as_index(),
                yr_cell_land_type: LandType::Clear.as_index(),
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs,
                is_water: false,
                is_cliff_like: false,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                allows_tiberium: false,
                height_in_pixels: 0,
                variant: 0,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                terrain_object_occupation: None,
                overlay_blocks: false,
                overlay_zone_type: None,
                outside_playfield: false,
                zone_type: zone_class::GROUND,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                base_land_type: LandType::Clear.as_index(),
                base_yr_cell_land_type: LandType::Clear.as_index(),
                base_terrain_class: TerrainClass::Clear,
                base_speed_costs: speed_costs,
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0; 3],
                radar_right: [0; 3],
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            }],
        );
        let mut sim = crate::sim::world::Simulation::new();
        sim.resolved_terrain = Some(terrain);
        let mut interner = StringInterner::default();
        let tree = TerrainObjectState {
            stable_id: 1,
            in_logic_vector: false,
            type_ref: interner.intern("TREE01"),
            rx: 0,
            ry: 0,
            health: 10,
            max_health: 10,
            occupation_bits: 7,
            lifecycle: TerrainObjectLifecycle::Live,
        };
        {
            let (production, resolved) = (&mut sim.production, &mut sim.resolved_terrain);
            mark_terrain_occupation(production, &tree, resolved.as_mut());
        }

        assert!(sim.rebuild_dynamic_navigation(&rules));
        assert!(!sim.path_grid().expect("terrain").is_walkable(0, 0));
        assert_eq!(
            sim.terrain_costs[&SpeedType::Track].cost_at(0, 0),
            0,
            "terrain object must block both navigation caches before removal"
        );

        {
            let (production, resolved) = (&mut sim.production, &mut sim.resolved_terrain);
            unmark_terrain_occupation(production, &tree, resolved.as_mut());
        }
        assert!(sim.rebuild_dynamic_navigation(&rules));

        assert!(sim.path_grid().expect("terrain").is_walkable(0, 0));
        assert_eq!(
            sim.terrain_costs[&SpeedType::Track].cost_at(0, 0),
            100,
            "terrain cost authority must be rebuilt from the cleared resolved cell"
        );
    }

    fn fire_event(attacker_id: u64, occupant_anim: Option<InternedId>) -> SimFireEvent {
        SimFireEvent {
            attacker_id,
            attacker_type_ref: test_intern("CABHUT"),
            weapon_slot: WeaponSlot::Primary,
            weapon_id: test_intern("UCWEAPON"),
            facing: 0,
            veterancy: 0,
            origin_snapshot: FireOriginSnapshot {
                rx: 10,
                ry: 20,
                sub_x: SimFixed::ZERO,
                sub_y: SimFixed::ZERO,
                z: 0,
                facing: 0,
                category: EntityCategory::Structure,
                burst_index: 0,
            },
            target: TargetKind::Cell(12, 20),
            report_sound_id: None,
            garrison_muzzle_index: occupant_anim.map(|_| 0),
            occupant_anim,
        }
    }

    #[test]
    fn fire_effect_batch_accumulates_fixed_tick_events_until_finish() {
        let mut pending = vec![fire_event(99, Some(test_intern("STALE")))];
        begin_fire_effect_batch(&mut pending);
        assert!(pending.is_empty());

        append_fire_effect_batch(&mut pending, &[fire_event(1, Some(test_intern("UCFLASH")))]);
        append_fire_effect_batch(
            &mut pending,
            &[
                fire_event(2, Some(test_intern("UCCONS"))),
                fire_event(3, None),
            ],
        );

        let attacker_ids: Vec<u64> = pending.iter().map(|ev| ev.attacker_id).collect();
        assert_eq!(attacker_ids, vec![1, 2, 3]);
        assert_eq!(pending[0].occupant_anim, Some(test_intern("UCFLASH")));
        assert_eq!(pending[1].occupant_anim, Some(test_intern("UCCONS")));
        assert_eq!(pending[2].occupant_anim, None);

        finish_fire_effect_batch(&mut pending);
        assert!(pending.is_empty());
    }

    #[test]
    fn upsert_updates_existing_and_inserts_absent_coordinates() {
        let mut existing = vec![entry(5, 5, 2, 0)];
        let candidates = vec![entry(5, 5, 2, 3), entry(6, 6, 2, 0)];
        assert_eq!(upsert_overlay_entries(&mut existing, candidates), 2);
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].frame, 3);
        assert_eq!((existing[1].rx, existing[1].ry), (6, 6));
    }

    #[test]
    fn upsert_reuses_coordinate_within_candidate_list() {
        let mut existing: Vec<OverlayEntry> = Vec::new();
        let candidates = vec![entry(7, 7, 2, 0), entry(7, 7, 2, 5), entry(8, 8, 2, 0)];
        assert_eq!(upsert_overlay_entries(&mut existing, candidates), 3);
        assert_eq!(existing.len(), 2);
        assert_eq!((existing[0].rx, existing[0].ry), (7, 7));
        assert_eq!(existing[0].frame, 5);
        assert_eq!((existing[1].rx, existing[1].ry), (8, 8));
    }

    #[test]
    fn upsert_empty_inputs() {
        let mut existing: Vec<OverlayEntry> = Vec::new();
        let candidates: Vec<OverlayEntry> = Vec::new();
        assert_eq!(upsert_overlay_entries(&mut existing, candidates), 0);
        assert!(existing.is_empty());
    }

    #[test]
    fn upsert_identical_existing_entries_is_a_noop() {
        let mut existing = vec![entry(1, 1, 2, 0), entry(2, 2, 3, 5)];
        let candidates = existing.clone();
        assert_eq!(upsert_overlay_entries(&mut existing, candidates), 0);
    }

    #[test]
    fn gsi_04_09_render_handoff_replaces_regerminated_overlay_variant() {
        let old_variant = entry(4, 4, 2, 7);
        let untouched = entry(5, 4, 99, 3);
        let mut render_entries = vec![old_variant.clone(), untouched.clone()];
        let authoritative = vec![entry(4, 4, 3, 1)];

        assert_eq!(
            upsert_overlay_entries(&mut render_entries, authoritative),
            1
        );
        assert_eq!(render_entries.len(), 2);
        assert_eq!((render_entries[0].rx, render_entries[0].ry), (4, 4));
        assert_eq!(render_entries[0].overlay_id, 3);
        assert_eq!(render_entries[0].frame, 1);
        assert_eq!(render_entries[1].overlay_id, untouched.overlay_id);
        assert_eq!(render_entries[1].frame, untouched.frame);
    }

    #[test]
    fn exact_receipt_accepts_one_wrapping_gameplay_frame() {
        let receipt = ExactStepReceipt {
            tick_before: 9,
            tick_after: 10,
            binary_frame_before: u32::MAX,
            binary_frame_after: 0,
        };
        assert_eq!(validate_exact_step_receipt(receipt), Ok(()));
    }

    #[test]
    fn exact_receipt_rejects_zero_or_multiple_frame_advances() {
        let receipt = |tick_after, binary_frame_after| ExactStepReceipt {
            tick_before: 10,
            tick_after,
            binary_frame_before: 20,
            binary_frame_after,
        };

        assert_eq!(
            validate_exact_step_receipt(receipt(10, 21)),
            Err(ExactStepError::TickDelta { actual: 0 })
        );
        assert_eq!(
            validate_exact_step_receipt(receipt(11, 20)),
            Err(ExactStepError::FrameDelta { actual: 0 })
        );
        assert_eq!(
            validate_exact_step_receipt(receipt(11, 22)),
            Err(ExactStepError::FrameDelta { actual: 2 })
        );
    }

    #[test]
    fn world_point_to_cell_round_trips_ground_iso_anchor() {
        let (rx, ry, z) = (10_u16, 5_u16, 4_u8);
        let (world_x, world_y) = (150.0, 180.0);
        let mut height_map = BTreeMap::new();
        for hx in 8..=12 {
            for hy in 3..=7 {
                height_map.insert((hx, hy), z);
            }
        }

        assert_eq!(
            world_point_to_cell(world_x, world_y, &height_map, None),
            (rx, ry)
        );
    }

    #[test]
    fn world_point_to_cell_forwards_tactical_bridge_inverse_map() {
        let (world_x, world_y) = (150.0, 180.0);
        let height_map = BTreeMap::new();
        let bridge_cells = BTreeMap::from([(
            (10, 5),
            crate::map::terrain::TacticalBridgeCell {
                deck_z: 4,
                structural: true,
                direction_zero: true,
            },
        )]);
        let expected = crate::map::terrain::screen_to_cell_tactical_inverse(
            world_x,
            world_y,
            crate::map::terrain::TacticalInverseContext {
                height_map: &height_map,
                bridge_cells: Some(&bridge_cells),
                viewport_offset_x: 0.0,
                viewport_offset_y: 0.0,
            },
        );
        let (expected_rx, expected_ry) = match expected {
            crate::map::terrain::TacticalInverseResult::Cell { rx, ry }
            | crate::map::terrain::TacticalInverseResult::Fallback { rx, ry } => (rx, ry),
        };

        assert_eq!(
            world_point_to_cell(world_x, world_y, &height_map, Some(&bridge_cells)),
            (
                expected_rx.round().max(0.0) as u16,
                expected_ry.round().max(0.0) as u16,
            )
        );
    }

    #[test]
    fn world_point_to_cell_clamps_negative_results_to_zero() {
        let height_map = BTreeMap::new();
        assert_eq!(
            world_point_to_cell(-500.0, -500.0, &height_map, None),
            (0, 0)
        );
    }
}

#[cfg(test)]
mod modal_pump_tests {
    use super::{
        SessionMode, modal_pump_should_advance_sim, score_row_display_name,
        should_record_replay_tick,
        wall_clock_service_admission,
    };

    #[test]
    fn empty_natural_terminal_score_frame_is_recorded() {
        let tick = crate::sim::world::TickResult {
            tick: 10,
            frame_committed: false,
            executed_commands: 0,
            state_hash: 123,
            terminal_score_finalized: true,
            spawned_entities: false,
            destroyed_structure: false,
            ownership_changed: false,
            bridge_state_changed: false,
            movement: Default::default(),
        };

        assert!(should_record_replay_tick(&tick, &[]));
        assert!(!should_record_replay_tick(
            &crate::sim::world::TickResult {
                terminal_score_finalized: false,
                ..tick
            },
            &[]
        ));
    }

    #[test]
    fn gsi_01_03_debug_pause_does_not_impersonate_an_offline_modal() {
        let debug_pause =
            wall_clock_service_admission(true, false, SessionMode::Skirmish, false, false);
        assert!(!debug_pause.simulation);
        assert!(debug_pause.tactical_mutation);

        let actual_modal =
            wall_clock_service_admission(true, true, SessionMode::Skirmish, false, false);
        assert!(!actual_modal.simulation);
        assert!(!actual_modal.tactical_mutation);
    }

    #[test]
    fn gsi_01_03_tactical_view_mutators_follow_modal_service_admission() {
        // No modal means the ordinary app loop owns camera/placement updates;
        // modal-pump blockers have no authority on that path.
        for mode in [
            SessionMode::Campaign,
            SessionMode::Skirmish,
            SessionMode::Lan,
            SessionMode::Wol,
        ] {
            assert_eq!(
                wall_clock_service_admission(false, false, mode, true, true),
                super::WallClockServiceAdmission {
                    simulation: true,
                    tactical_mutation: true,
                }
            );
        }

        // The active offline Menu, Abort and Options callers all enter the
        // service-only return: their frozen battlefield cannot pan or rebuild a
        // placement preview underneath the dialog.
        for mode in [SessionMode::Campaign, SessionMode::Skirmish] {
            assert_eq!(
                wall_clock_service_admission(true, true, mode, false, false),
                super::WallClockServiceAdmission {
                    simulation: false,
                    tactical_mutation: false,
                }
            );
        }

        // Preserve the existing network-modal branch, including both native
        // reasons it falls back to service-only work.
        for mode in [SessionMode::Lan, SessionMode::Wol] {
            let admitted = super::WallClockServiceAdmission {
                simulation: true,
                tactical_mutation: true,
            };
            let denied = super::WallClockServiceAdmission {
                simulation: false,
                tactical_mutation: false,
            };
            assert_eq!(
                wall_clock_service_admission(true, true, mode, false, false),
                admitted
            );
            assert_eq!(
                wall_clock_service_admission(true, true, mode, true, false),
                denied
            );
            assert_eq!(
                wall_clock_service_admission(true, true, mode, false, true),
                denied
            );
        }
    }

    #[test]
    fn session_mode_maps_writer_proofed_game_mode_values() {
        assert_eq!(SessionMode::from_game_mode(0), SessionMode::Campaign);
        assert_eq!(SessionMode::from_game_mode(3), SessionMode::Lan);
        assert_eq!(SessionMode::from_game_mode(4), SessionMode::Wol);
        assert_eq!(SessionMode::from_game_mode(5), SessionMode::Skirmish);
        // Legacy modem/serial (1/2) and any unrecognized value -> Other. The active
        // engine never writes these, so the pump treats them as non-advancing.
        assert_eq!(SessionMode::from_game_mode(1), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(2), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(-1), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(99), SessionMode::Other);
    }

    #[test]
    fn only_network_modes_advance_behind_a_modal() {
        // {3 LAN, 4 WOL} advance; {0 campaign, 5 skirmish} + Other freeze.
        assert!(modal_pump_should_advance_sim(
            SessionMode::Lan,
            false,
            false
        ));
        assert!(modal_pump_should_advance_sim(
            SessionMode::Wol,
            false,
            false
        ));
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Campaign,
            false,
            false
        ));
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Skirmish,
            false,
            false
        ));
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Other,
            false,
            false
        ));
    }

    #[test]
    fn reentrancy_guard_blocks_advance_even_on_network() {
        // The native reentrancy guard: a fixed tick already in progress means the
        // pump skips advancing, even on the network branch.
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Lan,
            false,
            true
        ));
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Wol,
            false,
            true
        ));
    }

    #[test]
    fn service_only_blockers_prevent_network_modal_advance() {
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Lan,
            true,
            false
        ));
        assert!(!modal_pump_should_advance_sim(
            SessionMode::Wol,
            true,
            false
        ));
    }

    #[test]
    fn pumped_tick_delta_is_zero_offline_and_n_on_network() {
        // C2 acceptance at the decision level: the pump decision drives whether the
        // world's fixed tick advances per pumped frame. `tick` stands in for
        // `sim.session.tick`; the full headless-World assertion (incl. "no
        // battlefield recomposite offline") lands with the live `service_tick` swap.
        const FRAMES: u64 = 7;
        let pumped = |mode: SessionMode| -> u64 {
            let mut tick = 0u64;
            for _ in 0..FRAMES {
                if modal_pump_should_advance_sim(mode, false, false) {
                    tick += 1; // one fixed step advanced this pumped frame
                }
            }
            tick
        };
        // Offline freezes: zero advance over N pumped frames.
        assert_eq!(pumped(SessionMode::Skirmish), 0);
        assert_eq!(pumped(SessionMode::Campaign), 0);
        // Network advances once per pumped frame.
        assert_eq!(pumped(SessionMode::Lan), FRAMES);
        assert_eq!(pumped(SessionMode::Wol), FRAMES);
    }

    #[test]
    fn pumped_world_tick_freezes_offline_and_advances_on_network() {
        use crate::sim::world::Simulation;
        use std::collections::BTreeMap;

        // C2 acceptance with a real headless World: drive `advance_tick` exactly
        // when the pump decision is true, and assert `session.tick` motion.
        // `advance_tick` commits one tick per call (no entities/rules needed).
        const FRAMES: u64 = 7;
        let pumped_world_delta = |mode: SessionMode| -> u64 {
            let mut sim = Simulation::new();
            let start = sim.session.tick;
            let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
            for _ in 0..FRAMES {
                if modal_pump_should_advance_sim(mode, false, false) {
                    // `tick_ms` does not affect the asserted tick delta; a literal
                    // matches the sim-test style and avoids the const dependency.
                    sim.advance_tick(&[], None, &height_map, None, None, 33);
                }
            }
            sim.session.tick - start
        };

        // Offline modes freeze the world behind the modal: zero tick advance.
        assert_eq!(pumped_world_delta(SessionMode::Skirmish), 0);
        assert_eq!(pumped_world_delta(SessionMode::Campaign), 0);
        // Network modes advance one fixed tick per pumped frame (dead code this
        // build; proves the contract for when multiplayer lands).
        assert_eq!(pumped_world_delta(SessionMode::Lan), FRAMES);
        assert_eq!(pumped_world_delta(SessionMode::Wol), FRAMES);
    }

    #[test]
    fn score_rows_never_show_the_raw_house_key() {
        let local = Some("Americans".to_string());
        let handle = Some("Commander".to_string());
        // Local player: the handle they launched under.
        assert_eq!(
            score_row_display_name("Americans", &local, &handle, Some("Americans!")),
            "Commander"
        );
        // Owner-key match is case-insensitive, as elsewhere in the owner paths.
        assert_eq!(
            score_row_display_name("AMERICANS", &local, &handle, Some("Americans!")),
            "Commander"
        );
        // Every other house shows its country's display name. Two computer
        // opponents of different countries read differently, as they do natively.
        assert_eq!(
            score_row_display_name("Russians", &local, &handle, Some("Russia")),
            "Russia"
        );
        assert_eq!(
            score_row_display_name("Africans", &local, &handle, Some("Libya")),
            "Libya"
        );
    }

    #[test]
    fn score_row_name_falls_back_when_no_launch_handle_was_recorded() {
        // Outside a skirmish launch there is no handle, so even the local row
        // takes the country name.
        assert_eq!(
            score_row_display_name("Americans", &None, &None, Some("America")),
            "America"
        );
    }

    #[test]
    fn score_row_name_uses_the_house_key_only_with_no_resolvable_country() {
        assert_eq!(
            score_row_display_name("Americans", &None, &None, None),
            "Americans"
        );
    }
}
