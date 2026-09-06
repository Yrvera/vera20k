//! Save-game repository, transactional restore, and process-lifetime persistence state.
//!
//! The repository owns every save-directory filesystem operation. The two
//! existing "latest" policies are intentionally distinct: panel rows sort by
//! the embedded snapshot timestamp, while quickload selects by filesystem
//! modification time. Unifying them would be VERA-internal / gamemd equivalent
//! UNCHECKED.
//!
//! F12 owner tree: the save/load panel UI and options persistence live here
//! beside the repository they drive.

pub(crate) mod commands;
pub(crate) mod options;
pub(crate) mod options_profile;
pub(crate) mod save_load_panel;

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::sim::snapshot::{
    GameSnapshot, GameSnapshotHeader, SnapshotError, SnapshotMapRestoreOutput, SnapshotRestoreError,
};
use crate::sim::world::Simulation;

const DEFAULT_SAVES_DIRECTORY: &str = "saves";

/// Process-lifetime save-game state owned by the app persistence domain.
pub(crate) struct PersistenceState {
    /// The process-lifetime retail Options/Video/Audio profile. UI, window,
    /// render, and audio state are projections of this single retained value.
    pub(crate) options_profile: options_profile::RetailOptionsProfile,
    pub(crate) repository: SaveRepository,
    pub(crate) save_list_cache: SaveListCache,
    last_save_tick: Option<u64>,
    last_save_instant: Option<Instant>,
    pub(crate) last_loaded_save_path: Option<PathBuf>,
}

impl PersistenceState {
    pub(crate) fn new(options_profile: options_profile::RetailOptionsProfile) -> Self {
        Self {
            options_profile,
            repository: SaveRepository::new(),
            save_list_cache: SaveListCache::new(),
            last_save_tick: None,
            last_save_instant: None,
            last_loaded_save_path: None,
        }
    }

    /// A successful disk write and its UI bookkeeping are one persistence
    /// operation. Failed writes preserve the previous last-save indication.
    pub(crate) fn write_save(
        &mut self,
        filename: &str,
        bytes: &[u8],
        tick: u64,
    ) -> Result<PathBuf, SaveWriteError> {
        let path = self.repository.write_named(filename, bytes)?;
        self.last_save_tick = Some(tick);
        self.last_save_instant = Some(Instant::now());
        self.invalidate_save_list();
        Ok(path)
    }

    pub(crate) fn last_save_tick(&self) -> Option<u64> {
        self.last_save_tick
    }

    pub(crate) fn last_save_instant(&self) -> Option<Instant> {
        self.last_save_instant
    }

    pub(crate) fn refresh_save_list_if_dirty(&mut self) {
        self.save_list_cache.refresh_if_dirty(&self.repository);
    }

    pub(crate) fn invalidate_save_list(&mut self) {
        self.save_list_cache.invalidate();
    }
}

/// Every failure that can prevent an in-scenario snapshot from being committed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PreparedLoadError {
    #[error("could not read save file: {0}")]
    ReadFile(#[source] std::io::Error),
    #[error("no active simulation to restore")]
    MissingCurrentSimulation,
    #[error("active world has no authoritative source-map digest")]
    MissingMapHash,
    #[error("active rules are unavailable")]
    MissingRules,
    #[error("no resolved_terrain available")]
    MissingTerrainTemplate,
    #[error("active overlay registry is unavailable")]
    MissingOverlayRegistry,
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error(transparent)]
    Restore(#[from] SnapshotRestoreError),
    #[error(transparent)]
    FactoryState(#[from] crate::sim::production::FactoryRestoreError),
}

/// Fully validated, cache-rebuilt replacement state ready for one infallible commit.
pub(crate) struct PreparedLoad {
    simulation: Simulation,
    map_restore: SnapshotMapRestoreOutput,
}

/// Immutable production input to an in-scenario load transaction.
///
/// Preparation can inspect only these references. App-owned screen, pacing,
/// diagnostics, presentation, lighting, and panel state therefore remain
/// structurally outside the fallible part of the transaction.
pub(crate) struct LoadPreparationView<'a> {
    repository: &'a SaveRepository,
    current_simulation: Option<&'a Simulation>,
    expected_map_hash: Option<u64>,
    rules: Option<&'a RuleSet>,
    terrain_template: Option<&'a ResolvedTerrainGrid>,
    overlay_registry: Option<&'a OverlayTypeRegistry>,
}

impl<'a> LoadPreparationView<'a> {
    /// Production inputs come from the one bound runtime, never shell fallback
    /// rules or a registry/terrain borrowed from a different match.
    pub(crate) fn from_runtime(
        repository: &'a SaveRepository,
        runtime: Option<&'a crate::sim::runtime::SimRuntime>,
        expected_map_hash: Option<u64>,
    ) -> Self {
        Self {
            repository,
            current_simulation: runtime.map(|runtime| &runtime.simulation),
            expected_map_hash,
            rules: runtime.map(|runtime| &runtime.resources.rules),
            terrain_template: runtime
                .and_then(|runtime| runtime.resources.terrain_template.as_ref()),
            overlay_registry: runtime.map(|runtime| &runtime.resources.overlay_registry),
        }
    }

    // Fault injection for independently missing historical restore inputs.
    #[cfg(test)]
    fn new(
        repository: &'a SaveRepository,
        current_simulation: Option<&'a Simulation>,
        expected_map_hash: Option<u64>,
        rules: Option<&'a RuleSet>,
        terrain_template: Option<&'a ResolvedTerrainGrid>,
        overlay_registry: Option<&'a OverlayTypeRegistry>,
    ) -> Self {
        Self {
            repository,
            current_simulation,
            expected_map_hash,
            rules,
            terrain_template,
            overlay_registry,
        }
    }
}

impl PreparedLoad {
    /// Read and prepare a save while holding only immutable references.
    /// Startup admission is match-owned and outside this transaction.
    pub(crate) fn from_repository(
        view: LoadPreparationView<'_>,
        path: &Path,
    ) -> Result<Self, PreparedLoadError> {
        let bytes = view
            .repository
            .read(path)
            .map_err(PreparedLoadError::ReadFile)?;
        let (simulation, map_restore) = Self::prepare_candidate(
            &bytes,
            view.current_simulation,
            view.expected_map_hash,
            view.rules,
            view.terrain_template,
            view.overlay_registry,
        )?;
        Ok(Self {
            simulation,
            map_restore,
        })
    }

    /// Perform every fallible validation and restoration step against owned
    /// candidate state. The live simulation is borrowed only to retain the
    /// process-global seed/Main/MapGen continuation and skipped cache inputs.
    fn prepare_candidate(
        bytes: &[u8],
        current_simulation: Option<&Simulation>,
        expected_map_hash: Option<u64>,
        rules: Option<&RuleSet>,
        terrain_template: Option<&ResolvedTerrainGrid>,
        overlay_registry: Option<&OverlayTypeRegistry>,
    ) -> Result<(Simulation, SnapshotMapRestoreOutput), PreparedLoadError> {
        let current_simulation =
            current_simulation.ok_or(PreparedLoadError::MissingCurrentSimulation)?;
        let expected_map_hash = expected_map_hash.ok_or(PreparedLoadError::MissingMapHash)?;
        let rules = rules.ok_or(PreparedLoadError::MissingRules)?;
        let snapshot = GameSnapshot::load_validated(
            bytes,
            expected_map_hash,
            rules.simulation_config_hash(),
            &current_simulation.session.map_name,
        )?;
        let terrain_template = terrain_template
            .cloned()
            .ok_or(PreparedLoadError::MissingTerrainTemplate)?;

        let terrain_speed_config = current_simulation.terrain_speed_config.clone();
        let bridge_explosions = current_simulation.bridge_explosions.clone();
        let metallic_debris = current_simulation.metallic_debris.clone();
        let bridge_anim_sounds = current_simulation.bridge_anim_sounds.clone();

        let mut simulation = snapshot.sim;
        // This is the in-scenario Load Game route: native load reseeds
        // Scenario->Random after reading ScenarioClass, while the process-global
        // seed and Main/MapGen cursors retain their live values.
        simulation.retain_in_scenario_process_state_from(current_simulation);
        simulation.restore_after_snapshot_load()?;
        crate::sim::production::validate_restored_factory_state(&simulation, rules)?;
        simulation.rebuild_caches_after_load(
            terrain_template,
            terrain_speed_config,
            bridge_explosions,
            metallic_debris,
            bridge_anim_sounds,
        );

        let overlay_registry = overlay_registry.ok_or(PreparedLoadError::MissingOverlayRegistry)?;
        let map_restore =
            simulation.restore_map_authority_after_snapshot_load(rules, overlay_registry)?;
        simulation.resolve_type_handles(rules);
        simulation.restore_move_sound_handles_after_load(rules)?;

        Ok((simulation, map_restore))
    }

    pub(crate) fn native_tiberium_stats(
        &self,
    ) -> crate::sim::ore_growth::NativeTiberiumRebuildStats {
        self.map_restore.native_tiberium_stats
    }

    /// Commit into an existing runtime: immutable match resources cannot be
    /// replaced or synthesized by a same-content load.
    pub(crate) fn commit_into(
        mut self,
        runtime: &mut crate::sim::runtime::SimRuntime,
    ) -> Vec<crate::map::overlay::OverlayEntry> {
        // This is the first infallible successful-load seam. Native
        // `MouseClass::Load @ 0x005BE150` reaches
        // `MapClass::Resize @ 0x00565C10` and reconstructs the fixed fallback
        // CellClass here, including its split `+0xDC` reservation state; doing
        // it during candidate preparation would leak mutation from a rejected
        // transactional load into the running match.
        self.simulation.reconstruct_cellclass_dummy_for_map_resize();
        runtime.replace_simulation(self.simulation);
        self.map_restore.occupied_overlays
    }
}

/// One valid row in the save-game list.
pub(crate) struct SaveEntry {
    pub(crate) path: PathBuf,
    pub(crate) header: GameSnapshotHeader,
}

/// Cached panel listing, refreshed only after explicit invalidation.
pub(crate) struct SaveListCache {
    entries: Vec<SaveEntry>,
    dirty: bool,
}

impl SaveListCache {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            dirty: true,
        }
    }

    fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn entries(&self) -> &[SaveEntry] {
        &self.entries
    }

    fn refresh_if_dirty(&mut self, repository: &SaveRepository) {
        if self.dirty {
            self.entries = repository.panel_entries_by_embedded_time();
            self.dirty = false;
        }
    }
}

/// Filesystem stage that failed while writing a named save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveWriteStage {
    CreateDirectory,
    WriteFile,
}

/// Save write failure retaining the old caller-visible stage distinction.
#[derive(Debug)]
pub(crate) struct SaveWriteError {
    stage: SaveWriteStage,
    source: std::io::Error,
}

impl SaveWriteError {
    pub(crate) fn stage(&self) -> SaveWriteStage {
        self.stage
    }
}

impl std::fmt::Display for SaveWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for SaveWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Concrete owner of the save directory and every operation performed in it.
pub(crate) struct SaveRepository {
    directory: PathBuf,
}

impl SaveRepository {
    fn new() -> Self {
        Self {
            directory: PathBuf::from(DEFAULT_SAVES_DIRECTORY),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub(crate) fn write_named(
        &self,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, SaveWriteError> {
        std::fs::create_dir_all(&self.directory).map_err(|source| SaveWriteError {
            stage: SaveWriteStage::CreateDirectory,
            source,
        })?;
        let path = self.directory.join(file_name);
        std::fs::write(&path, bytes).map_err(|source| SaveWriteError {
            stage: SaveWriteStage::WriteFile,
            source,
        })?;
        Ok(path)
    }

    pub(crate) fn delete(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    pub(crate) fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    /// Panel policy: admit only valid snapshot headers and sort newest embedded
    /// save timestamp first.
    pub(crate) fn panel_entries_by_embedded_time(&self) -> Vec<SaveEntry> {
        let Ok(directory) = std::fs::read_dir(&self.directory) else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        for item in directory {
            let Ok(item) = item else { continue };
            let path = item.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("bin") {
                continue;
            }
            let Ok(bytes) = self.read(&path) else {
                continue;
            };
            let Ok(header) = GameSnapshot::read_header(&bytes) else {
                continue;
            };
            entries.push(SaveEntry { path, header });
        }
        sort_panel_entries_by_embedded_time(&mut entries);
        entries
    }

    /// Quickload policy: select the `.bin` file with the newest filesystem
    /// modification time without reading or validating its snapshot header.
    pub(crate) fn quickload_path_by_modified_time(&self) -> Option<PathBuf> {
        let directory = std::fs::read_dir(&self.directory).ok()?;
        newest_modified_path(directory.filter_map(|item| {
            let item = item.ok()?;
            let path = item.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("bin") {
                return None;
            }
            Some((path, item.metadata().ok()?.modified().ok()?))
        }))
    }
}

fn sort_panel_entries_by_embedded_time(entries: &mut [SaveEntry]) {
    entries.sort_by(|left, right| right.header.save_timestamp.cmp(&left.header.save_timestamp));
}

fn newest_modified_path(
    candidates: impl Iterator<Item = (PathBuf, SystemTime)>,
) -> Option<PathBuf> {
    candidates
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_runtime::frame_pacer::LocalFramePacer;
    use crate::app::match_runtime::startup::MatchStartup;
    use crate::map::lighting::CellLightGrid;
    use crate::map::overlay::OverlayEntry;
    use crate::map::resolved_terrain::{DynamicTerrainCellState, ResolvedTerrainCell};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::overlay_grid::OverlayGrid;
    use crate::sim::replay::{ReplayHeader, ReplayLog};
    use crate::sim::world::{Simulation, SimulationRngState};
    use crate::skirmish_launch::{
        AiDifficulty, LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishAiSlot,
        SkirmishLaunchMode, SkirmishLaunchOptions, SkirmishLaunchSession, SkirmishLocalSlot,
    };
    use crate::ui::game_screen::GameScreen;

    const LOAD_FIXTURE_MAP_HASH: u64 = 0x1234_5678_9ABC_DEF0;
    const LOAD_FIXTURE_MAP_NAME: &str = "TRANSACTION.MAP";
    const LOAD_FIXTURE_SEED: u32 = 0x1234_5678;

    struct TestClock(u32);

    impl crate::match_bootstrap::MatchSeedClock for TestClock {
        fn low_u32(&mut self) -> u32 {
            self.0
        }

        fn source(&self) -> crate::match_bootstrap::MatchSeedSource {
            crate::match_bootstrap::MatchSeedSource::Controlled
        }

        fn seed_authority_certifying(&self) -> bool {
            true
        }
    }

    fn startup_authority(seed: u32) -> MatchStartup {
        let launch = SkirmishLaunchSession {
            mode: SkirmishLaunchMode {
                id: 1,
                ui_name_key: "GUI:Battle".into(),
                tooltip_key: "STT:ModeBattle".into(),
                override_file: "MPBattleMD.ini".into(),
                map_filter: "standard".into(),
                random_maps_allowed: true,
                allies_allowed: true,
                must_ally: false,
            },
            selected_map_file: Some(LOAD_FIXTURE_MAP_NAME.into()),
            player_name: "Player".into(),
            local: SkirmishLocalSlot {
                country: LaunchCountry::America,
                country_random: false,
                color_index: 0,
                color_random: false,
                start_position: LaunchStartPosition::Position(0),
                team: LaunchTeam::None,
            },
            opponents: vec![SkirmishAiSlot {
                country: LaunchCountry::Russia,
                country_random: false,
                color_index: 1,
                color_random: false,
                start_position: LaunchStartPosition::Position(1),
                team: LaunchTeam::None,
                difficulty: AiDifficulty::Easy,
            }],
            pre_fill_house_roster:
                crate::skirmish_launch::PreFillHouseRoster::from_compact_skirmish(1),
            options: SkirmishLaunchOptions::default(),
        };
        let accepted = match crate::match_bootstrap::classify_startup_session(&launch) {
            crate::match_bootstrap::StartupSessionClassification::AcceptedExplicitFixedBattle(
                accepted,
            ) => accepted,
            other => panic!("startup fixture was not accepted: {other:?}"),
        };
        let mut next_correlation = 1;
        let correlation =
            crate::match_bootstrap::allocate_match_correlation(&mut next_correlation).unwrap();
        let startup = crate::match_bootstrap::prepare_match_startup(
            correlation,
            accepted,
            &mut TestClock(seed),
        );
        let initial_simulation = Simulation::with_seed(u64::from(seed));
        let mut authority = MatchStartup::default();
        authority.begin(Some(correlation));
        authority
            .acknowledge(startup, Some(&initial_simulation), true, false)
            .unwrap();
        authority
    }

    struct RunningMatchTestState {
        runtime: crate::sim::runtime::SimRuntime,
        /// The app-owned diagnostics slot (F10) — represented here so the
        /// baseline proves a failed load leaves the segment untouched.
        replay_log: Option<ReplayLog>,
        startup: MatchStartup,
        screen: GameScreen,
        frame_pacer: LocalFramePacer,
        overlay_render_index: Vec<OverlayEntry>,
        lighting_grid: CellLightGrid,
        show_save_load_panel: bool,
        persistence: PersistenceState,
    }

    impl RunningMatchTestState {
        fn running(rules: &RuleSet) -> Self {
            let simulation = load_fixture_simulation(true);
            let shared_cell_dummy = simulation.effective_shared_cell_dummy();
            shared_cell_dummy.set_level_slope(-7, 11);
            shared_cell_dummy.stamp_coord(7, 9);
            shared_cell_dummy
                .apply_bridge_flag_slot(crate::map::bridge_facts::BridgeStampSlot::Anchor, true);
            let mut replay = ReplayLog::new(ReplayHeader {
                version: 1,
                tick_hz: 15,
                seed: simulation.session.seed,
                map_name: simulation.session.map_name.clone(),
                rules_hash: rules.simulation_config_hash(),
            });
            replay.record_tick(1, Vec::new(), simulation.state_hash());
            let replay_log = Some(replay);

            let startup = startup_authority(LOAD_FIXTURE_SEED);
            let mut frame_pacer = LocalFramePacer::new();
            frame_pacer.record_admitted_frame(32);
            let mut lighting_grid = CellLightGrid::new();
            lighting_grid.set_compat_tint((3, 4), [0.25, 0.5, 0.75]);
            let mut persistence =
                PersistenceState::new(options_profile::RetailOptionsProfile::default());
            persistence.last_loaded_save_path = Some(PathBuf::from("before-load.bin"));
            persistence.save_list_cache.dirty = false;

            Self {
                runtime: crate::sim::runtime::SimRuntime::from_simulation(simulation),
                replay_log,
                startup,
                screen: GameScreen::InGame,
                frame_pacer,
                overlay_render_index: vec![OverlayEntry {
                    rx: 3,
                    ry: 4,
                    overlay_id: 5,
                    frame: 6,
                }],
                lighting_grid,
                show_save_load_panel: true,
                persistence,
            }
        }

        fn baseline(&self) -> RunningMatchBaseline {
            RunningMatchBaseline {
                simulation_hash: self.runtime.simulation.state_hash(),
                shared_cell_dummy: self.effective_shared_cell_dummy_snapshot(),
                rng: self.runtime.simulation.rng_state(),
                replay: self.replay_log.as_ref().map(|replay| {
                    (
                        replay.header.seed,
                        replay.ticks.len(),
                        replay.ticks.first().map(|tick| tick.state_hash),
                    )
                }),
                startup: self.startup.clone(),
                screen: self.screen.clone(),
                pacer_admits_same_bucket: self.frame_pacer.should_admit(32, 1, false),
                pacer_admits_next_bucket: self.frame_pacer.should_admit(48, 1, false),
                overlay_render_index: self
                    .overlay_render_index
                    .iter()
                    .map(|entry| (entry.rx, entry.ry, entry.overlay_id, entry.frame))
                    .collect(),
                lighting_tint: self.lighting_grid.tint_or_default((3, 4)),
                show_save_load_panel: self.show_save_load_panel,
                last_loaded_save_path: self.persistence.last_loaded_save_path.clone(),
                save_list_dirty: self.persistence.save_list_cache.dirty,
            }
        }

        fn effective_shared_cell_dummy_snapshot(
            &self,
        ) -> crate::map::resolved_terrain::SharedCellDummySnapshot {
            self.runtime
                .simulation
                .effective_shared_cell_dummy()
                .snapshot()
        }
    }

    #[derive(Debug, PartialEq)]
    struct RunningMatchBaseline {
        simulation_hash: u64,
        shared_cell_dummy: crate::map::resolved_terrain::SharedCellDummySnapshot,
        rng: SimulationRngState,
        replay: Option<(u64, usize, Option<u64>)>,
        startup: MatchStartup,
        screen: GameScreen,
        pacer_admits_same_bucket: bool,
        pacer_admits_next_bucket: bool,
        overlay_render_index: Vec<(u16, u16, u8, u8)>,
        lighting_tint: [f32; 3],
        show_save_load_panel: bool,
        last_loaded_save_path: Option<PathBuf>,
        save_list_dirty: bool,
    }

    mod factory_restore_tests;

    fn load_fixture_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [Warheads]\n\
             [OverlayTypes]\n",
        );
        RuleSet::from_ini(&ini).expect("transaction fixture rules")
    }

    fn load_fixture_simulation(with_overlay_grid: bool) -> Simulation {
        let mut simulation = Simulation::with_seed(u64::from(LOAD_FIXTURE_SEED));
        simulation.session.map_name = LOAD_FIXTURE_MAP_NAME.to_string();
        simulation.overlay_grid = with_overlay_grid.then(|| OverlayGrid::new_with_retained_wall_plane(0, 0));
        simulation
    }

    fn load_fixture_terrain() -> ResolvedTerrainGrid {
        ResolvedTerrainGrid::from_cells(0, 0, Vec::new())
    }

    fn compatibility_snapshot_cell(tile_index: i32) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx: 0,
            ry: 0,
            source_tile_index: tile_index,
            source_sub_tile: 0,
            final_tile_index: tile_index,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            height_in_pixels: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: crate::map::resolved_terrain::zone_class::GROUND,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: TerrainClass::Clear,
            base_speed_costs: SpeedCostProfile::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn snapshot_bytes(simulation: &Simulation, rules: &RuleSet) -> Vec<u8> {
        GameSnapshot::save_validated(
            simulation,
            LOAD_FIXTURE_MAP_HASH,
            rules.simulation_config_hash(),
            "transaction fixture",
            1,
        )
    }

    fn assert_transaction_failure(
        state: &RunningMatchTestState,
        repository: &SaveRepository,
        path: &Path,
        expected_map_hash: Option<u64>,
        rules: Option<&RuleSet>,
        terrain_template: Option<&ResolvedTerrainGrid>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        expected_error: impl FnOnce(&PreparedLoadError) -> bool,
    ) {
        // Capture every represented running-match owner before the production
        // seam performs file I/O or begins candidate restoration.
        let baseline = state.baseline();
        let result = PreparedLoad::from_repository(
            LoadPreparationView::new(
                repository,
                Some(&state.runtime.simulation),
                expected_map_hash,
                rules,
                terrain_template,
                overlay_registry,
            ),
            path,
        );
        let error = match result {
            Ok(_) => panic!("invalid load fixture unexpectedly prepared"),
            Err(error) => error,
        };

        assert!(expected_error(&error), "unexpected error: {error}");
        assert_eq!(state.baseline(), baseline);
    }

    fn snapshot(description: &str, save_timestamp: u64) -> Vec<u8> {
        let mut simulation = Simulation::new();
        simulation.session.map_name = "POLICY.MAP".to_string();
        GameSnapshot::save_validated(&simulation, 1, 2, description, save_timestamp)
    }

    fn isolated_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vera20k-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after Unix epoch")
                .as_nanos(),
        ))
    }

    #[test]
    fn failed_load_preserves_complete_running_match_transaction() {
        let rules = load_fixture_rules();
        let registry = OverlayTypeRegistry::empty();
        let terrain = load_fixture_terrain();
        let state = RunningMatchTestState::running(&rules);
        let directory = isolated_directory("load-transaction-failures");
        let repository = SaveRepository::at(&directory);

        let valid_saved = load_fixture_simulation(true);
        let valid_bytes = snapshot_bytes(&valid_saved, &rules);
        let valid_path = repository
            .write_named("valid.bin", &valid_bytes)
            .expect("write valid transaction fixture");
        let bad_bytes_path = repository
            .write_named("bad-bytes.bin", b"not a snapshot")
            .expect("write bad-byte transaction fixture");

        let mut invalid_identity = load_fixture_simulation(true);
        invalid_identity.substrate.next_stable_object_id = 0;
        let invalid_identity_path = repository
            .write_named(
                "invalid-identity.bin",
                &snapshot_bytes(&invalid_identity, &rules),
            )
            .expect("write identity-failure transaction fixture");

        let missing_overlay = load_fixture_simulation(false);
        let missing_overlay_path = repository
            .write_named(
                "missing-map-cache.bin",
                &snapshot_bytes(&missing_overlay, &rules),
            )
            .expect("write map/cache-failure transaction fixture");

        assert_transaction_failure(
            &state,
            &repository,
            &bad_bytes_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            Some(&terrain),
            Some(&registry),
            |error| matches!(error, PreparedLoadError::Snapshot(_)),
        );
        assert_transaction_failure(
            &state,
            &repository,
            &valid_path,
            Some(LOAD_FIXTURE_MAP_HASH ^ 1),
            Some(&rules),
            Some(&terrain),
            Some(&registry),
            |error| {
                matches!(
                    error,
                    PreparedLoadError::Snapshot(SnapshotError::MapMismatch { .. })
                )
            },
        );
        assert_transaction_failure(
            &state,
            &repository,
            &valid_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            None,
            Some(&terrain),
            Some(&registry),
            |error| matches!(error, PreparedLoadError::MissingRules),
        );
        assert_transaction_failure(
            &state,
            &repository,
            &valid_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            None,
            Some(&registry),
            |error| matches!(error, PreparedLoadError::MissingTerrainTemplate),
        );
        assert_transaction_failure(
            &state,
            &repository,
            &valid_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            Some(&terrain),
            None,
            |error| matches!(error, PreparedLoadError::MissingOverlayRegistry),
        );
        assert_transaction_failure(
            &state,
            &repository,
            &invalid_identity_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            Some(&terrain),
            Some(&registry),
            |error| {
                matches!(
                    error,
                    PreparedLoadError::Restore(SnapshotRestoreError::ObjectIdCounterBehind { .. })
                )
            },
        );
        assert_transaction_failure(
            &state,
            &repository,
            &missing_overlay_path,
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            Some(&terrain),
            Some(&registry),
            |error| {
                matches!(
                    error,
                    PreparedLoadError::Restore(
                        SnapshotRestoreError::MissingMapAuthorityComponent {
                            component: "OverlayGrid"
                        }
                    )
                )
            },
        );

        std::fs::remove_dir_all(directory).expect("remove load-transaction fixture directory");
    }

    #[test]
    fn successful_same_content_load_preserves_admission_and_bound_resources() {
        let rules = load_fixture_rules();
        let registry = OverlayTypeRegistry::empty();
        let terrain = load_fixture_terrain();
        let mut state = RunningMatchTestState::running(&rules);
        let directory = isolated_directory("load-transaction-startup");
        let repository = SaveRepository::at(&directory);
        let mut saved = load_fixture_simulation(true);
        saved
            .substrate
            .base_reservations
            .reserve(None, 3, 4, 2);
        saved
            .substrate
            .base_reservations
            .reserve(None, -1, 0, 5);
        let path = repository
            .write_named("same-content.bin", &snapshot_bytes(&saved, &rules))
            .expect("write same-content transaction fixture");

        let startup_before = state.startup.clone();
        let baseline = state.baseline();
        state.runtime.resources.rules = rules;
        state.runtime.resources.overlay_registry = registry;
        state.runtime.resources.terrain_template = Some(terrain);
        state.runtime.resources.height_map.insert((3, 4), 7);
        state.runtime.resources.waypoints.insert(
            701,
            crate::map::waypoints::Waypoint {
                index: 701,
                rx: 122,
                ry: 135,
            },
        );
        let rules_before = state.runtime.resources.rules.simulation_config_hash();
        let prepared = PreparedLoad::from_repository(
            LoadPreparationView::from_runtime(
                &repository,
                Some(&state.runtime),
                Some(LOAD_FIXTURE_MAP_HASH),
            ),
            &path,
        )
        .unwrap_or_else(|error| panic!("same-content transaction must prepare: {error}"));
        assert_eq!(state.baseline(), baseline);

        let live_dummy = state.runtime.simulation.effective_shared_cell_dummy();
        assert_eq!(live_dummy.snapshot().coord, (7, 9));
        assert_eq!(
            prepared
                .simulation
                .substrate
                .base_reservations
                .raw_mask(None, 3, 4),
            1 << 2,
            "candidate preparation restores real reservation authority verbatim"
        );
        assert_eq!(
            prepared.simulation.substrate.base_reservations.dummy_mask(),
            0,
            "raw snapshot decode reconstructs the process-global dummy cleared"
        );
        let runtime = &mut state.runtime;
        let _occupied_overlays = prepared.commit_into(runtime);
        assert_eq!(runtime.resources.height_map.get(&(3, 4)), Some(&7));
        assert_eq!(state.startup, startup_before);
        assert!(state.startup.admits_exact_step());
        assert_eq!(
            runtime.resources.rules.simulation_config_hash(),
            rules_before
        );
        assert!(runtime.resources.terrain_template.is_some());
        assert_eq!(runtime.resources.waypoints[&701].rx, 122);
        let simulation = &mut runtime.simulation;
        let restored_dummy = simulation.effective_shared_cell_dummy();
        assert!(restored_dummy.same_identity(&live_dummy));
        assert_eq!(
            restored_dummy.snapshot(),
            crate::map::resolved_terrain::SharedCellDummySnapshot {
                coord: (0, 0),
                level: 0,
                slope_type: 0,
                bridge_flags_0x1180: 0,
            },
            "successful in-scenario load reconstructs the fixed dummy at the commit seam"
        );
        assert_eq!(
            simulation
                .substrate
                .base_reservations
                .raw_mask(None, 3, 4),
            1 << 2,
            "the narrow dummy reconstruction leaves real reservation state untouched"
        );
        assert_eq!(simulation.substrate.base_reservations.dummy_mask(), 0);

        let accepted_hash = simulation.state_hash();
        simulation
            .substrate
            .base_reservations
            .reserve(None, -1, 0, 6);
        assert_ne!(simulation.state_hash(), accepted_hash);
        simulation.reconstruct_cellclass_dummy_for_map_resize();
        assert_eq!(simulation.substrate.base_reservations.dummy_mask(), 0);
        assert_eq!(
            simulation.state_hash(),
            accepted_hash,
            "with the shared dummy already zero, reconstruction removes only the hashed stale mask"
        );
        let tick_before = runtime.simulation.session.tick;
        runtime.advance_frame(&[], 33, crate::sim::world::TickLane::Ordinary);
        assert_eq!(runtime.simulation.session.tick, tick_before + 1);
        assert_eq!(state.startup, startup_before);
        assert!(state.startup.admits_exact_step());
        std::fs::remove_dir_all(directory).expect("remove startup fixture directory");
    }

    #[test]
    fn gsi_04_02_last_tiles_full_load_route_does_not_retranslate_actual_runtime_ids() {
        let compatibility = crate::map::theater::parse_tileset_ini(
            b"[TileSet0000]\nTilesInSet=3\n\
              [TileSet0001]\nTilesInSet=5\nLastTilesInSet=2\n\
              [TileSet0002]\nTilesInSet=1\n",
            "tem",
        )
        .expect("compatibility table");
        assert_eq!(compatibility.translate_legacy_map_tile_index(5), 8);

        let rules = load_fixture_rules();
        let registry = OverlayTypeRegistry::empty();
        let translated_template =
            ResolvedTerrainGrid::from_cells(1, 1, vec![compatibility_snapshot_cell(8)]);
        let mut runtime_actual = compatibility_snapshot_cell(5);
        runtime_actual.radar_left = [31, 32, 33];

        let mut saved = load_fixture_simulation(false);
        saved.overlay_grid = Some(OverlayGrid::new_with_retained_wall_plane(1, 1));
        saved.install_resolved_terrain_for_new_map(translated_template.clone());
        saved
            .dynamic_terrain_cells
            .insert((0, 0), DynamicTerrainCellState::capture(&runtime_actual));
        let bytes = snapshot_bytes(&saved, &rules);

        let mut current = load_fixture_simulation(false);
        current.overlay_grid = Some(OverlayGrid::new_with_retained_wall_plane(1, 1));
        let (restored, _) = PreparedLoad::prepare_candidate(
            &bytes,
            Some(&current),
            Some(LOAD_FIXTURE_MAP_HASH),
            Some(&rules),
            Some(&translated_template),
            Some(&registry),
        )
        .expect("full persistence prepare/serde/rebuild/restore route");

        let restored_cell = restored
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(0, 0))
            .expect("restored dynamic terrain cell");
        assert_eq!(
            restored_cell.source_tile_index, 8,
            "the retained already-translated map template is cloned verbatim"
        );
        assert_eq!(
            restored_cell.final_tile_index, 5,
            "serialized runtime actual ID must not cross legacy translation again"
        );
        assert_eq!(restored_cell.radar_left, [31, 32, 33]);
        assert_eq!(
            restored.dynamic_terrain_cells.get(&(0, 0)),
            Some(&DynamicTerrainCellState::capture(&runtime_actual))
        );
    }

    #[test]
    fn save_write_updates_readout_and_listing_only_after_success() {
        let directory = isolated_directory("save-bookkeeping");
        let mut persistence =
            PersistenceState::new(options_profile::RetailOptionsProfile::default());
        persistence.repository = SaveRepository::at(&directory);
        persistence.refresh_save_list_if_dirty();
        assert!(persistence.save_list_cache.entries().is_empty());
        let bytes = snapshot("exact description", 123);
        let path = persistence.write_save("saved.bin", &bytes, 42).unwrap();
        assert_eq!(persistence.repository.read(&path).unwrap(), bytes);
        assert_eq!(persistence.last_save_tick(), Some(42));
        assert!(persistence.last_save_instant().is_some());
        persistence.refresh_save_list_if_dirty();
        assert_eq!(persistence.save_list_cache.entries().len(), 1);
        let saved_at = persistence.last_save_instant();

        // A directory at the chosen file path forces the actual write stage to fail.
        std::fs::create_dir(directory.join("blocked.bin")).unwrap();
        let error = persistence
            .write_save("blocked.bin", &bytes, 99)
            .unwrap_err();
        assert_eq!(error.stage(), SaveWriteStage::WriteFile);
        assert_eq!(persistence.last_save_tick(), Some(42));
        assert_eq!(persistence.last_save_instant(), saved_at);
        assert!(!persistence.save_list_cache.dirty);

        persistence.repository = SaveRepository::at(&path);
        let error = persistence
            .write_save("unreachable.bin", &bytes, 100)
            .unwrap_err();
        assert_eq!(error.stage(), SaveWriteStage::CreateDirectory);
        assert_eq!(persistence.last_save_tick(), Some(42));
        assert_eq!(persistence.last_save_instant(), saved_at);
        assert!(!persistence.save_list_cache.dirty);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn quickload_and_panel_keep_explicit_latest_policies() {
        let directory = isolated_directory("save-latest-policies");
        let repository = SaveRepository::at(&directory);
        let embedded_newer_path = repository
            .write_named("embedded-newer.bin", &snapshot("newer", 200))
            .expect("write embedded-newer fixture");
        let embedded_older_path = repository
            .write_named("embedded-older.bin", &snapshot("older", 100))
            .expect("write embedded-older fixture");

        let base_modified = SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(60))
            .expect("fixture modification time");
        std::fs::File::options()
            .write(true)
            .open(&embedded_newer_path)
            .expect("open embedded-newer fixture")
            .set_times(std::fs::FileTimes::new().set_modified(base_modified))
            .expect("set embedded-newer modification time");
        std::fs::File::options()
            .write(true)
            .open(&embedded_older_path)
            .expect("open embedded-older fixture")
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(base_modified + std::time::Duration::from_secs(30)),
            )
            .expect("set embedded-older modification time");

        let panel_entries = repository.panel_entries_by_embedded_time();
        assert_eq!(panel_entries.len(), 2);
        assert_eq!(panel_entries[0].path, embedded_newer_path);
        assert_eq!(
            repository.quickload_path_by_modified_time(),
            Some(embedded_older_path)
        );

        std::fs::remove_dir_all(directory).expect("remove latest-policy fixture directory");
    }

    #[test]
    fn repository_owns_write_read_panel_scan_and_delete() {
        let directory = isolated_directory("save-repository");
        let repository = SaveRepository::at(&directory);
        let mut simulation = Simulation::new();
        simulation.session.map_name = "OFFICIAL.MAP".to_string();
        let bytes = GameSnapshot::save_validated(&simulation, 1, 2, "Northern ridge", 3);

        let path = repository
            .write_named("completely_different_name.bin", &bytes)
            .expect("write repository fixture");
        assert_eq!(
            repository.read(&path).expect("read repository fixture"),
            bytes
        );

        let entries = repository.panel_entries_by_embedded_time();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, path);
        assert_eq!(entries[0].header.description, "Northern ridge");

        repository.delete(&path).expect("delete repository fixture");
        assert!(!repository.exists(&path));
        std::fs::remove_dir_all(directory).expect("remove repository fixture directory");
    }
}
