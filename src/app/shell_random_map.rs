use super::*;

/// The random-map seed file the `.SED` launch branch recognises. Written into
/// the RA2 directory so `map_load` finds it where the original puts it.
const RANDMAP_SED_FILE: &str = "RandMap.Sed";
/// Description the setup dialog stamps onto a randomized configuration; it also
/// becomes the sentinel row's displayed name.
const RANDOM_MAP_DESCRIPTION_KEY: &str = "TXT_RANDOM_MAP_DESCRIPTION";
const RANDOM_MAP_DESCRIPTION_FALLBACK: &str = "Random Map";
/// The players slider is the last of the setup dialog's six option rows, and the
/// dialog gives it a range of 2..8 with a step of one.
const SETUP_PLAYERS_ROW: usize = 5;
const SETUP_PLAYERS_MIN: i32 = 2;
const SETUP_PLAYERS_MAX: i32 = 8;
const SETUP_PLAYERS_STEP: i32 = 1;
/// Matches the rules default the map-load path falls back to; the preview only
/// needs it because terrain resolution takes it, not because cliffs affect the
/// image.
const RANDOM_MAP_PREVIEW_CLIFF_BACK_IMPASSABILITY: u8 = 2;
/// Where the generated preview is written. The chooser's sentinel row reads this
/// back, so writing it is what makes the random-map thumbnail appear there.
const RANDMAP_PREVIEW_FILE: &str = "RandMap.img";

/// A random-map generation handed to a worker thread.
///
/// The worker only *generates*; colouring a preview needs the terrain resolver
/// and therefore the asset manager, so the main thread rasterises everything the
/// worker hands it. What matters is that the expensive part is off the UI
/// thread — that is what lets frames render while it runs.
pub(crate) struct RandomMapGenerationJob {
    receiver: std::sync::mpsc::Receiver<RandomMapUpdate>,
    /// Kept back from the worker because rasterising needs it: the resolver
    /// reads theater data to decide each cell's final tile.
    theater: Box<crate::map::theater::TheaterData>,
    terrain_rules: Box<crate::rules::terrain_rules::TerrainRules>,
    /// Set when OK started this generation. Accept cannot run until the map
    /// exists, so it is deferred to whoever collects the result.
    accept_on_finish: bool,
}

/// Generated-map ownership across the setup dialog and loading handoff.
///
/// The candidate belongs only to the open setup dialog. The accepted map is
/// retained until the matching `.SED` launch transfers it to LoadingRequest.
/// gamemd provenance: random-map setup runner FUN_00595BC0 and accepted caller
/// 0x005E8590 retain the generated scenario consumed by Scenario initialization.
#[derive(Default)]
pub(crate) struct RandomMapGenerationRetention {
    candidate: Option<crate::map::rmg::GeneratedMap>,
    accepted: Option<(String, crate::map::rmg::GeneratedMap)>,
}

impl RandomMapGenerationRetention {
    fn begin_generation(&mut self) {
        self.candidate = None;
        self.accepted = None;
    }

    fn finish_generation(&mut self, generated: crate::map::rmg::GeneratedMap) {
        self.candidate = Some(generated);
    }

    fn cancel_setup(&mut self) {
        self.candidate = None;
        self.accepted = None;
    }

    fn accept_setup(&mut self, selected_map_file: &str) {
        self.accepted = self
            .candidate
            .take()
            .map(|generated| (selected_map_file.to_owned(), generated));
    }

    pub(super) fn select_map(&mut self, selected_map_file: &str) {
        if self.accepted.as_ref().is_some_and(|(accepted_file, _)| {
            !accepted_file.eq_ignore_ascii_case(selected_map_file)
        }) {
            self.accepted = None;
        }
    }

    pub(super) fn take_for_loading(
        &mut self,
        selected_map_file: Option<&str>,
    ) -> Option<crate::map::rmg::GeneratedMap> {
        let (accepted_file, generated) = self.accepted.take()?;
        selected_map_file
            .is_some_and(|selected| accepted_file.eq_ignore_ascii_case(selected))
            .then_some(generated)
    }
}

/// What the generator worker sends back as it goes.
enum RandomMapUpdate {
    /// The map at one of the boundaries the original redraws its preview at.
    Progress(Box<crate::map::rmg::build::GenerationSnapshot>),
    /// The finished map.
    Finished(Box<crate::map::rmg::GeneratedMap>),
}

/// Whether the original redraws its preview at this generation boundary.
///
/// It draws eight times while generating, reporting 55, 60, 70, 80, 85, 90 and
/// 95 percent on the seven after the first. Two of those pairs have no
/// generation between them at all — only a progress-report helper runs — so the
/// 60 and 85 redraws reproduce the image already on screen and are dropped here:
/// eight calls, six distinct pictures.
///
/// The percentages are the anchor the boundaries below were chosen from; they
/// have no home in the port yet, because the dialog's progress bar is still
/// drawn empty.
fn draws_preview(point: crate::map::rmg::build::GenerationPoint) -> bool {
    use crate::map::rmg::Stage;
    use crate::map::rmg::build::GenerationPoint;
    matches!(
        point,
        // Clears the box before any terrain exists.
        GenerationPoint::Initial
            // 55 (and again at 60): the water is in.
            | GenerationPoint::After(Stage::WaterFinalize)
            // 70: regions, island passes and the green spread.
            | GenerationPoint::After(Stage::RecalcAfterTerrain)
            // 80 (and again at 85): starts, tech buildings and tiberium.
            | GenerationPoint::After(Stage::RecalcAfterTiberium)
            // 90: the hills.
            | GenerationPoint::After(Stage::Hills)
            // 95: LAT patches, trees and rocks.
            | GenerationPoint::After(Stage::Rocks)
    )
}

impl App {
    /// Kick off generation on a worker and return immediately.
    ///
    /// Everything that needs the asset manager is done here, up front; only
    /// plain data crosses to the worker.
    fn start_random_map_generation(
        state: &mut AppState,
        options: &crate::map::rmg::RmgOptions,
        accept_on_finish: bool,
    ) -> bool {
        // A second Generate makes the previous dialog result stale immediately,
        // even when setup cannot progress far enough to spawn the worker.
        state.random_map_retention.begin_generation();
        let Some(asset_manager) = state.asset_manager.as_mut() else {
            return false;
        };
        let settings = crate::map::rmg::RmgSettings::load(asset_manager);
        let theater_name = crate::map::rmg::emit::theater_name(options.theater);
        let Some(theater) = crate::map::theater::load_theater(asset_manager, theater_name) else {
            log::warn!("random map: theater {theater_name} unavailable");
            return false;
        };
        // Stock RMG preview publishes its resolved theater registry before the
        // later ordinary map load, even if generation subsequently fails.
        state
            .tile_variant_selector_cache
            .complete_theater_registry_load(
                theater.rmg_tiles.clear_tile,
                theater.rmg_tiles.water_set,
            );
        let terrain_rules = asset_manager
            .get_ref("rulesmd.ini")
            .and_then(|bytes| crate::rules::ini_parser::IniFile::from_bytes(bytes).ok())
            .map(|ini| crate::rules::terrain_rules::TerrainRules::from_ini(&ini))
            .unwrap_or_default();
        let resolved_inputs = crate::map::rmg::build::ResolvedTheaterInputs::from_theater(
            &theater,
            &terrain_rules,
            crate::map::rmg::trig::global().cloned(),
        );
        let blocks =
            crate::map::rmg::theater_blocks::TheaterTileBlocks::build(&theater.lookup, |name| {
                asset_manager.get(name)
            });
        // `[AI] NeutralTechBuildings` plus each type's `Foundation=`, resolved
        // here because only plain data may cross to the worker.
        let tech_types = crate::app_init_helpers::load_neutral_tech_types(asset_manager);

        let (sender, receiver) = std::sync::mpsc::channel();
        let options = options.clone();
        // Generation stays single-threaded and seed-driven; the thread changes
        // only where it runs, never the order it consumes its RNG in.
        let spawned = std::thread::Builder::new()
            .name("random-map-generate".to_string())
            .spawn(move || {
                let generated = crate::map::rmg::build::generate_map_observed(
                    &options,
                    &settings,
                    &resolved_inputs,
                    &blocks,
                    &tech_types,
                    // A closed receiver means the dialog went away; dropping
                    // what we produce is the correct outcome, not an error.
                    &mut |view| {
                        if !draws_preview(view.point()) {
                            return;
                        }
                        let _ = sender.send(RandomMapUpdate::Progress(Box::new(view.snapshot())));
                    },
                );
                if generated.unfilled_start_slots > 0 {
                    log::warn!(
                        "Random map is short of spawns: {} start slot(s) could \
                         not be filled; those players have no start position",
                        generated.unfilled_start_slots
                    );
                }
                let _ = sender.send(RandomMapUpdate::Finished(Box::new(generated)));
            });
        match spawned {
            Ok(_handle) => {
                state.random_map_generation = Some(RandomMapGenerationJob {
                    receiver,
                    theater: Box::new(theater),
                    terrain_rules: Box::new(terrain_rules),
                    accept_on_finish,
                });
                true
            }
            Err(err) => {
                log::warn!("random map: could not spawn the generator thread: {err}");
                false
            }
        }
    }

    /// Collect whatever the generator has produced since the last frame.
    ///
    /// Called every frame while a job is in flight. Returns true when the dialog
    /// changed, so the caller knows to redraw.
    ///
    /// Only the newest of several progress snapshots is rasterised. Colouring is
    /// the expensive half, and an image the worker overtook before a frame was
    /// drawn was never on screen to be seen.
    pub(crate) fn poll_random_map_generation(state: &mut AppState) -> bool {
        if state.random_map_generation.is_some()
            && state.skirmish_shell_state.random_map_setup_modal.is_none()
        {
            // The dialog went away without the job going with it. Drop it here
            // rather than trusting every close path to remember: a job with no
            // dialog has nowhere to deliver, and letting it finish would write
            // a preview file for a map nobody asked for.
            state.random_map_generation = None;
            return false;
        }
        let Some(job) = state.random_map_generation.as_ref() else {
            return false;
        };
        let mut latest_progress = None;
        let mut finished = None;
        let mut died = false;
        loop {
            match job.receiver.try_recv() {
                Ok(RandomMapUpdate::Progress(snapshot)) => latest_progress = Some(snapshot),
                Ok(RandomMapUpdate::Finished(generated)) => {
                    finished = Some(generated);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    died = true;
                    break;
                }
            }
        }

        if let Some(generated) = finished {
            let job = state
                .random_map_generation
                .take()
                .expect("checked present above");
            let preview = Self::rasterise_generated_map(state, &job, &generated);
            state.random_map_retention.finish_generation(*generated);
            if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                modal.finish_generate(preview);
            }
            if job.accept_on_finish {
                Self::accept_random_map_setup(state);
            }
            return true;
        }

        if died {
            // The worker ended without a result. Clear the job so the dialog
            // does not sit disabled forever waiting on it.
            log::warn!("random map: the generator thread ended without a result");
            state.random_map_generation = None;
            if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                modal.finish_generate(None);
            }
            return true;
        }

        let Some(snapshot) = latest_progress else {
            return false;
        };
        // Lifted out and put straight back: rasterising reads the job and the
        // rest of the app state at once, and the job lives inside that state.
        let job = state
            .random_map_generation
            .take()
            .expect("checked present above");
        let preview =
            Self::rasterise_map(state, &job, &snapshot.map_file, &snapshot.start_waypoints);
        state.random_map_generation = Some(job);
        if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
            if let Some(preview) = preview {
                modal.show_progress_preview(preview);
            }
        }
        true
    }

    /// Remove the setup dialog and any in-flight worker without changing the
    /// retention disposition already chosen by accept or cancel.
    fn dismiss_random_map_setup(state: &mut AppState) {
        state.skirmish_shell_state.random_map_setup_modal = None;
        state.random_map_generation = None;
    }

    /// Cancel the setup dialog, abandoning any generation and every retained
    /// result associated with this random-map selection.
    ///
    /// Dropping the job drops the receiver, so a worker still going finds a
    /// closed channel on its next send and its remaining output goes nowhere.
    /// That matters beyond tidiness: a late finish would otherwise overwrite
    /// `RandMap.img`, changing the chooser's thumbnail to a map the player
    /// walked away from.
    fn cancel_random_map_setup(state: &mut AppState) {
        Self::dismiss_random_map_setup(state);
        state.random_map_retention.cancel_setup();
    }

    /// Commit the dialog's options and close it. Shared by the immediate accept
    /// and the one deferred behind a generation.
    fn accept_random_map_setup(state: &mut AppState) {
        let Some(crate::ui::skirmish_shell::AcceptOutcome::Commit(options)) = state
            .skirmish_shell_state
            .random_map_setup_modal
            .as_ref()
            .map(|modal| modal.accept())
        else {
            return;
        };
        match Self::commit_random_map_setup(state, &options) {
            Ok(()) => {
                state.random_map_retention.accept_setup(RANDMAP_SED_FILE);
                // Successful OK already chose the retained result; dialog
                // teardown must not run the cancellation invalidation path.
                Self::dismiss_random_map_setup(state);
            }
            Err(err) => {
                // Staying open is deliberate: a missing seed file makes the
                // launch path fall back to defaults, which would silently
                // start a different map than the one configured.
                log::error!("random map: could not write {RANDMAP_SED_FILE}: {err}");
            }
        }
    }

    /// Rasterise the finished map and persist it as the chooser's thumbnail.
    fn rasterise_generated_map(
        state: &mut AppState,
        job: &RandomMapGenerationJob,
        generated: &crate::map::rmg::GeneratedMap,
    ) -> Option<crate::map::rmg::preview::PreviewImage> {
        let preview =
            Self::rasterise_map(state, job, &generated.map_file, &generated.start_waypoints)?;
        // Only the finished map is written out: the file is what the chooser
        // row shows later, and a half-built map is not that map.
        Self::write_random_map_preview_file(state, &preview);
        Some(preview)
    }

    /// Colour and rasterise a map. Main thread only: the resolver reads theater
    /// data and the ore/gem colours come out of overlay SHPs.
    ///
    /// Mid-generation snapshots go through here too, so an in-progress preview
    /// is coloured by exactly the path that colours the finished one.
    fn rasterise_map(
        state: &mut AppState,
        job: &RandomMapGenerationJob,
        map_file: &crate::map::map_file::MapFile,
        start_waypoints: &[(u8, u16, u16)],
    ) -> Option<crate::map::rmg::preview::PreviewImage> {
        // LAT defaults off for runtime maps, so resolve the same way the load
        // path will; a different setting here would colour cells the player
        // never sees.
        let resolved_terrain = {
            let frontend_main_rng = &mut state.frontend_main_rng;
            let selector_cache = &mut state.tile_variant_selector_cache;
            let asset_manager = state.asset_manager.as_ref();
            let mut raw_draw = || frontend_main_rng.next_u32();
            let mut selector = selector_cache.begin_load(&mut raw_draw);
            // RMG InitMap supplies explicit Clear cells. Its preview never
            // borrows a Scenario cursor; equal-bound Fill remains zero-cost.
            let mut scenario_fill_ranged = |low, high| {
                debug_assert_eq!((low, high), (0, 0));
                0
            };
            crate::map::resolved_terrain::ResolvedTerrainGrid::build_with_variant_selector(
                map_file,
                Some(&job.theater),
                asset_manager,
                Some(&job.terrain_rules),
                None,
                None,
                false,
                RANDOM_MAP_PREVIEW_CLIFF_BACK_IMPASSABILITY,
                &mut scenario_fill_ranged,
                &mut selector,
            )
        };
        // Ore and gem cells take their colour from the overlay's own SHP: the
        // growth stage indexes the frame list and the frame header carries the
        // radar triple. The artwork is never sampled for it, so there is no
        // substitute for loading the file.
        let overlay_registry = state.overlay_registry.as_ref();
        let assets = state.asset_manager.as_ref();
        let theater_ext = job.theater.extension;
        let overlay_radar = |overlay_id: u8, stage: u8| -> Option<[u8; 3]> {
            let registry = overlay_registry?;
            // The tiberium flag is the gate: walls, roads and bridges are
            // overlays too, and they keep the terrain colour underneath.
            if !registry.flags(overlay_id)?.tiberium {
                return None;
            }
            // The stage's colour out of the overlay SHP wins; the type's
            // RadarColor= stands in when that comes back essentially black,
            // which is also what happens when the art is missing entirely.
            let from_art = (|| {
                let name = registry.name(overlay_id)?;
                let bytes = crate::map::overlay_types::overlay_shp_candidates(name, theater_ext)
                    .iter()
                    .find_map(|candidate| assets?.get_ref(candidate))?;
                let shp = crate::assets::shp_file::ShpFile::from_bytes(bytes).ok()?;
                Some(shp.frames.get(stage as usize)?.radar_color)
            })()
            .filter(|rgb| *rgb != [0, 0, 0]);
            from_art.or_else(|| registry.flags(overlay_id)?.radar_color)
        };
        let cells = crate::map::rmg::preview::preview_cells_from_map(
            map_file,
            &resolved_terrain,
            &overlay_radar,
        );
        let waypoints = crate::map::rmg::preview::marker_waypoints(start_waypoints);
        crate::map::rmg::preview::render_preview(&cells, &waypoints)
    }

    /// Persist the generated preview so the chooser's random-map row can show it.
    ///
    /// Failure is logged rather than propagated: the dialog's own preview box
    /// draws from memory, so a write failure costs the chooser thumbnail and
    /// nothing else.
    fn write_random_map_preview_file(
        state: &AppState,
        preview: &crate::map::rmg::preview::PreviewImage,
    ) {
        let Some(ra2_dir) = state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
        else {
            return;
        };
        let (Ok(width), Ok(height)) = (u16::try_from(preview.width), u16::try_from(preview.height))
        else {
            log::warn!(
                "random map: preview {}x{} does not fit a PCX header",
                preview.width,
                preview.height
            );
            return;
        };
        let rgb: Vec<u8> = preview
            .rgba
            .chunks_exact(4)
            .flat_map(|px| {
                crate::render::native_surface_format::ACTIVE_RETAIL_RGB565_PRESENTATION
                    .storage_roundtrip_rgb8([px[0], px[1], px[2]])
            })
            .collect();
        match crate::assets::pcx_file::encode_direct_rgb(width, height, &rgb) {
            Ok(encoded) => {
                let path = ra2_dir.join(RANDMAP_PREVIEW_FILE);
                if let Err(err) = std::fs::write(&path, encoded) {
                    log::warn!("random map: could not write {}: {err}", path.display());
                }
            }
            Err(err) => log::warn!("random map: could not encode the preview: {err}"),
        }
    }

    /// Where saved seeds live: the game directory, the same place the dialog's
    /// own working file is written.
    fn saved_seed_dir(state: &AppState) -> Option<std::path::PathBuf> {
        state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
    }

    fn skirmish_saved_seed_layout(
        state: &AppState,
        mode: SavedSeedMode,
    ) -> crate::ui::skirmish_shell::SavedSeedLayout {
        crate::ui::skirmish_shell::compute_saved_seed_layout(
            mode,
            state.render_width(),
            state.render_height(),
        )
    }

    pub(super) fn handle_saved_seed_browser_mouse_down(state: &mut AppState) -> bool {
        let Some(mode) = state
            .skirmish_shell_state
            .saved_seed_browser
            .as_ref()
            .map(|browser| browser.mode)
        else {
            return false;
        };
        let layout = Self::skirmish_saved_seed_layout(state, mode);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let mut play_sound = false;
        if let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() {
            match crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y) {
                Some(crate::ui::skirmish_shell::SavedSeedControl::List) => {
                    if let Some(row) = crate::ui::skirmish_shell::saved_seed_list_row_at(
                        &layout,
                        browser.entries.len(),
                        browser.top_index,
                        x,
                        y,
                    ) {
                        browser.select(row);
                    }
                }
                // The list selects on press; the buttons arm instead, so
                // dragging off one cancels it.
                Some(crate::ui::skirmish_shell::SavedSeedControl::Action)
                | Some(crate::ui::skirmish_shell::SavedSeedControl::Back0x686) => {
                    browser.pressed_control =
                        crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y);
                    play_sound = true;
                }
                _ => {}
            }
        }
        if play_sound {
            Self::play_main_menu_button_sound(state);
        }
        true
    }

    pub(super) fn handle_saved_seed_browser_mouse_up(state: &mut AppState) -> bool {
        let Some(mode) = state
            .skirmish_shell_state
            .saved_seed_browser
            .as_ref()
            .map(|browser| browser.mode)
        else {
            return false;
        };
        let layout = Self::skirmish_saved_seed_layout(state, mode);
        let dir = Self::saved_seed_dir(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;

        use crate::ui::skirmish_shell::SavedSeedControl as SeedControl;
        use crate::ui::skirmish_shell::SavedSeedOutcome as Outcome;

        let outcome = {
            let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() else {
                return false;
            };
            let pressed = browser.pressed_control.take();
            let released = crate::ui::skirmish_shell::saved_seed_control_at(&layout, x, y);
            if pressed.is_none() || pressed != released {
                return true;
            }
            match released {
                Some(SeedControl::Back0x686) => Some(Outcome::Close),
                Some(SeedControl::Action) => browser.action_outcome(),
                _ => None,
            }
        };
        let Some(outcome) = outcome else {
            return true;
        };
        let Some(dir) = dir else {
            state.skirmish_shell_state.saved_seed_browser = None;
            return true;
        };

        match outcome {
            Outcome::Close => state.skirmish_shell_state.saved_seed_browser = None,
            Outcome::Load(file_name) => {
                match crate::map::rmg::saved_seeds::load_saved_seed(&dir.join(&file_name)) {
                    Ok(options) => {
                        // Loading replaces the working options and invalidates
                        // any generated result, exactly as an edit would.
                        if let Some(modal) =
                            state.skirmish_shell_state.random_map_setup_modal.as_mut()
                        {
                            modal.options = options;
                            modal.generated = false;
                            modal.generated_preview = None;
                        }
                        state.skirmish_shell_state.saved_seed_browser = None;
                    }
                    Err(err) => log::warn!("saved seed: could not read {file_name}: {err}"),
                }
            }
            Outcome::Save(name) => {
                let options = state
                    .skirmish_shell_state
                    .random_map_setup_modal
                    .as_ref()
                    .map(|modal| modal.options.clone());
                let path = crate::map::rmg::saved_seeds::seed_path_for_name(&dir, &name);
                match (options, path) {
                    (Some(options), Some(path)) => {
                        if let Err(err) =
                            crate::map::rmg::saved_seeds::save_saved_seed(&path, &options)
                        {
                            log::warn!("saved seed: could not write {name}: {err}");
                        }
                        state.skirmish_shell_state.saved_seed_browser = None;
                    }
                    // A refused name leaves the browser open so the player can
                    // retype rather than silently losing the save.
                    _ => log::warn!("saved seed: {name} is not a usable save name"),
                }
            }
            Outcome::Delete(file_name) => {
                if let Err(err) =
                    crate::map::rmg::saved_seeds::delete_saved_seed(&dir.join(&file_name))
                {
                    log::warn!("saved seed: could not delete {file_name}: {err}");
                }
                // Delete stays open so several can be removed in one visit.
                if let Some(browser) = state.skirmish_shell_state.saved_seed_browser.as_mut() {
                    browser.remove_entry(&file_name);
                }
            }
        }
        true
    }

    /// Persist accepted random-map setup, refresh the sentinel record, and
    /// select it so launch generates from it.
    ///
    /// A failed write is fatal to the commit: `map_load` treats a missing seed
    /// file as "use defaults", so committing anyway would silently start a
    /// different map than the one the player configured.
    fn commit_random_map_setup(
        state: &mut AppState,
        options: &crate::map::rmg::RmgOptions,
    ) -> anyhow::Result<()> {
        let ra2_dir = state
            .game_config
            .as_ref()
            .map(|config| config.paths.ra2_dir.clone())
            .ok_or_else(|| anyhow::anyhow!("no game config; cannot locate the RA2 directory"))?;
        std::fs::write(ra2_dir.join(RANDMAP_SED_FILE), options.to_sed_bytes())?;

        let display = if options.description.is_empty() {
            RANDOM_MAP_DESCRIPTION_FALLBACK
        } else {
            options.description.as_str()
        };
        // Reuse the modal helper: it upserts the single sentinel, honours the
        // mode's random-map admission, and refreshes the filtered record list.
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return Ok(());
        };
        let index = modal.create_random_map(
            &mut state.skirmish_scenario_records,
            &state.skirmish_modes,
            display,
            options.num_players,
        );
        let mode_id = modal.selected_mode_id;
        let _ = modal;
        if let Some(index) = index {
            // The scenario record alone is not enough to play: committing a
            // selection resolves it against the loadable map list, which has no
            // entry for a seed file until one is put there.
            let entry = state.skirmish_scenario_records[index].to_map_menu_entry();
            match state
                .skirmish_shell_maps
                .iter()
                .position(|map| map.file_name.eq_ignore_ascii_case(&entry.file_name))
            {
                Some(existing) => state.skirmish_shell_maps[existing] = entry,
                None => state.skirmish_shell_maps.push(entry),
            }
            let selection = crate::ui::skirmish_shell::ChooseMapSelection {
                mode_id,
                record_index: Some(index),
            };
            let _ = Self::commit_choose_map_selection(state, selection);
            Self::close_choose_map_modal(state);
        }
        Ok(())
    }

    fn skirmish_random_map_setup_layout(
        state: &AppState,
    ) -> crate::ui::skirmish_shell::RandomMapSetupLayout {
        crate::ui::skirmish_shell::compute_random_map_setup_layout(
            state.render_width(),
            state.render_height(),
        )
    }

    pub(super) fn handle_random_map_setup_mouse_down(state: &mut AppState) -> bool {
        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return false;
        };
        // An open list covers the rows under it, so it gets first refusal on the
        // click. Clicking anywhere else closes it without acting on whatever is
        // underneath, the way a dismissed dropdown behaves.
        if let Some(combo) = modal.open_combo {
            let items = crate::ui::skirmish_shell::setup_combo_items(combo);
            let on_list = crate::ui::skirmish_shell::random_map_setup_dropdown_row_at(
                &layout,
                combo.row(),
                items.len(),
                x,
                y,
            )
            .is_some();
            let on_face = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y)
                == Some(combo.control());
            if !on_list && !on_face {
                modal.open_combo = None;
                return true;
            }
            if on_list {
                return true;
            }
        }
        if let Some(control) = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y)
        {
            // The players slider is not a button: it acts on press, not on
            // release, so it never arms a pressed control.
            if control == crate::ui::skirmish_shell::RandomMapSetupControl::Players0x3eb {
                if modal.is_enabled(control) {
                    Self::press_setup_players_trackbar(modal, &layout, x, y);
                }
                return true;
            }
            // A disabled control swallows the click without arming a press, so
            // releasing over it cannot fire.
            if modal.is_enabled(control) {
                modal.pressed_control = Some(control);
                Self::play_main_menu_button_sound(state);
            }
            return true;
        }
        layout.dialog.contains(x, y)
    }

    /// Press behaviour for the players slider, mirroring the shell's other
    /// trackbars: grabbing the thumb starts a tracking drag, while a press on
    /// the rail jumps the value once and tracks nothing.
    fn press_setup_players_trackbar(
        modal: &mut crate::ui::skirmish_shell::RandomMapSetupModalState,
        layout: &crate::ui::skirmish_shell::RandomMapSetupLayout,
        x: i32,
        y: i32,
    ) {
        let rect = layout.control_rects[SETUP_PLAYERS_ROW];
        if !crate::ui::skirmish_shell::trackbar_mouse_allowed_y(rect, y) {
            return;
        }
        let pixel_offset = crate::ui::skirmish_shell::trackbar_pixel_offset(
            modal.options.num_players,
            SETUP_PLAYERS_MIN,
            SETUP_PLAYERS_MAX,
            SETUP_PLAYERS_STEP,
            rect,
        );
        if crate::ui::skirmish_shell::trackbar_thumb_hit(rect, pixel_offset, x, y) {
            modal.dragging_players_thumb = true;
        } else if rect.contains(x, y) {
            modal.set_num_players(crate::ui::skirmish_shell::trackbar_mouse_value(
                rect,
                x,
                SETUP_PLAYERS_MIN,
                SETUP_PLAYERS_MAX,
                SETUP_PLAYERS_STEP,
            ));
        }
    }

    pub(super) fn handle_random_map_setup_mouse_move(state: &mut AppState) {
        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return;
        };
        if !modal.dragging_players_thumb {
            return;
        }
        let rect = layout.control_rects[SETUP_PLAYERS_ROW];
        modal.set_num_players(crate::ui::skirmish_shell::trackbar_mouse_value(
            rect,
            x,
            SETUP_PLAYERS_MIN,
            SETUP_PLAYERS_MAX,
            SETUP_PLAYERS_STEP,
        ));
        state.window.request_redraw();
    }

    pub(super) fn handle_random_map_setup_mouse_up(state: &mut AppState) -> bool {
        use crate::ui::skirmish_shell::RandomMapSetupControl as Control;

        let layout = Self::skirmish_random_map_setup_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        // RMGMD.INI drives the randomizer's vegetation bounds; without it the
        // derived vegetation collapses to zero and randomized maps lose trees.
        let settings = state
            .asset_manager
            .as_ref()
            .map(crate::map::rmg::RmgSettings::load)
            .unwrap_or_default();
        let description = state
            .csf
            .as_ref()
            .map(|csf| csf.text(RANDOM_MAP_DESCRIPTION_KEY).into_owned())
            .unwrap_or_else(|| RANDOM_MAP_DESCRIPTION_FALLBACK.to_string());
        let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() else {
            return false;
        };
        if modal.dragging_players_thumb {
            modal.dragging_players_thumb = false;
            return true;
        }
        // Releasing over an open list commits that entry. The press was never
        // armed for list clicks, so this has to run before the pressed check.
        if let Some(combo) = modal.open_combo {
            let items = crate::ui::skirmish_shell::setup_combo_items(combo);
            if let Some(index) = crate::ui::skirmish_shell::random_map_setup_dropdown_row_at(
                &layout,
                combo.row(),
                items.len(),
                x,
                y,
            ) {
                modal.set_combo_value(combo, items[index].value);
                return true;
            }
        }
        let pressed = modal.pressed_control.take();
        let released = crate::ui::skirmish_shell::random_map_setup_control_at(&layout, x, y);
        if pressed.is_none() || pressed != released {
            return layout.dialog.contains(x, y) || pressed.is_some();
        }

        let mut close_setup = false;
        // Generating needs the whole app state, so it cannot run while the modal
        // is mutably borrowed; the actions below only record what to do.
        let mut generate_requested = false;
        let mut accept_requested = false;
        let mut open_browser: Option<SavedSeedMode> = None;
        match released.expect("checked equal to pressed control") {
            Control::Randomize0x621 => {
                modal.randomize_options(&settings, &mut state.frontend_main_rng, &description);
            }
            Control::Generate0x620 => {
                modal.reroll_derived_for_generate(&settings, &mut state.frontend_main_rng);
                modal.begin_generate();
                generate_requested = true;
            }
            Control::Ok0x6c5 => {
                // Accept generates first when nothing has been generated yet,
                // so the committed options always describe a map that exists.
                if matches!(
                    modal.accept(),
                    crate::ui::skirmish_shell::AcceptOutcome::NeedsGenerate
                ) {
                    modal.reroll_derived_for_generate(&settings, &mut state.frontend_main_rng);
                    modal.begin_generate();
                    generate_requested = true;
                }
                accept_requested = true;
            }
            Control::Cancel0x5c0 => {
                // Result 2 in the original: no seed file, no sentinel, no
                // selection change. The chooser underneath is left untouched.
                close_setup = true;
            }
            Control::Load0x6c2 => open_browser = Some(SavedSeedMode::Load),
            Control::Save0x6c3 => open_browser = Some(SavedSeedMode::Save),
            Control::Delete0x6c4 => open_browser = Some(SavedSeedMode::Delete),
            Control::MapType0x405
            | Control::Time0x3ea
            | Control::Theater0x407
            | Control::Size0x406
            | Control::Resources0x408 => {
                if let Some(combo) =
                    crate::ui::skirmish_shell::SetupCombo::from_control(released.expect("matched"))
                {
                    modal.toggle_combo(combo);
                }
            }
            // Dragging the players slider is a separate input mode; clicking the
            // track alone does not move it.
            Control::Players0x3eb => {}
        }

        if let Some(mode) = open_browser {
            let entries = Self::saved_seed_dir(state)
                .map(|dir| crate::map::rmg::saved_seeds::list_saved_seeds(&dir))
                .unwrap_or_default();
            state.skirmish_shell_state.saved_seed_browser =
                Some(SavedSeedBrowserState::open(mode, entries));
            return true;
        }
        if generate_requested {
            let options = state
                .skirmish_shell_state
                .random_map_setup_modal
                .as_ref()
                .map(|modal| modal.options.clone());
            let started = options.is_some_and(|options| {
                Self::start_random_map_generation(state, &options, accept_requested)
            });
            if !started {
                // Nothing will arrive, so the dialog must not be left sitting
                // in its generating state with every control disabled.
                log::warn!("random map: could not start generation for the configured options");
                if let Some(modal) = state.skirmish_shell_state.random_map_setup_modal.as_mut() {
                    modal.finish_generate(None);
                }
            }
            // Accept, if it was asked for, is now the job's responsibility.
            return true;
        }
        if accept_requested {
            Self::accept_random_map_setup(state);
        }
        if close_setup {
            Self::cancel_random_map_setup(state);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_preview_boundaries_cover_the_originals_eight_redraws() {
        use crate::map::rmg::STAGE_ORDER;
        use crate::map::rmg::build::GenerationPoint;

        let drawn: Vec<GenerationPoint> = std::iter::once(GenerationPoint::Initial)
            .chain(
                STAGE_ORDER
                    .iter()
                    .map(|stage| GenerationPoint::After(*stage)),
            )
            .filter(|point| draws_preview(*point))
            .collect();
        // Eight redraws in the original, two of them repeats of the image
        // already on screen.
        assert_eq!(drawn.len(), 6, "{drawn:?}");
        assert_eq!(drawn[0], GenerationPoint::Initial);
        // The last one precedes the final recalc, so the finished map still
        // differs from it and the closing draw is not a repeat either.
        assert_eq!(
            drawn[5],
            GenerationPoint::After(crate::map::rmg::Stage::Rocks)
        );
    }

    fn retained_map(seed: i32, start_x: u16) -> crate::map::rmg::GeneratedMap {
        let mut options = crate::map::rmg::RmgOptions::default();
        options.seed = seed;
        crate::map::rmg::GeneratedMap {
            map_file: crate::map::rmg::emit::empty_map_file(&options, 32, 32),
            mapgen_continuation:
                crate::map::rmg::MapGenRngContinuation::seeded_for_test(seed as u16),
            start_waypoints: vec![(0, start_x, 20)],
            stages_run: Vec::new(),
            unfilled_start_slots: 0,
        }
    }

    #[test]
    fn gsi_03_09_random_map_retention_invalidates_and_transfers_exactly_once() {
        let mut regenerated = RandomMapGenerationRetention::default();
        regenerated.finish_generation(retained_map(11, 10));
        regenerated.accept_setup("RandMap.Sed");
        regenerated.begin_generation();
        assert!(
            regenerated.take_for_loading(Some("RandMap.Sed")).is_none(),
            "starting a genuine regeneration invalidates accepted map A"
        );

        let mut reopened_then_cancelled_without_generate = RandomMapGenerationRetention::default();
        reopened_then_cancelled_without_generate.finish_generation(retained_map(12, 11));
        reopened_then_cancelled_without_generate.accept_setup("RandMap.Sed");
        reopened_then_cancelled_without_generate.cancel_setup();
        assert!(
            reopened_then_cancelled_without_generate
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "a genuine setup Cancel invalidates accepted map A"
        );

        let mut reopened_then_cancelled = RandomMapGenerationRetention::default();
        reopened_then_cancelled.finish_generation(retained_map(13, 12));
        reopened_then_cancelled.accept_setup("RandMap.Sed");
        reopened_then_cancelled.begin_generation();
        reopened_then_cancelled.finish_generation(retained_map(14, 13));
        reopened_then_cancelled.cancel_setup();
        assert!(
            reopened_then_cancelled
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "reopen, regenerate, then Cancel cannot resurrect accepted map A"
        );

        let mut cancelled = RandomMapGenerationRetention::default();
        cancelled.finish_generation(retained_map(22, 20));
        cancelled.cancel_setup();
        cancelled.accept_setup("RandMap.Sed");
        assert!(cancelled.take_for_loading(Some("RandMap.Sed")).is_none());

        let mut selected_elsewhere = RandomMapGenerationRetention::default();
        selected_elsewhere.finish_generation(retained_map(33, 30));
        selected_elsewhere.accept_setup("RandMap.Sed");
        selected_elsewhere.select_map("mp01t4.map");
        assert!(
            selected_elsewhere
                .take_for_loading(Some("RandMap.Sed"))
                .is_none()
        );

        let mut alternate_seed = RandomMapGenerationRetention::default();
        alternate_seed.finish_generation(retained_map(34, 35));
        alternate_seed.accept_setup("RandMap.Sed");
        // This is the retention authority used by apply_selected_shell_map_file:
        // another seed selection must invalidate, not merely refuse this call.
        alternate_seed.select_map("Other.Sed");
        assert!(
            alternate_seed
                .take_for_loading(Some("RandMap.Sed"))
                .is_none()
        );
        assert!(alternate_seed.take_for_loading(Some("Other.Sed")).is_none());

        let mut mismatched_launch = RandomMapGenerationRetention::default();
        mismatched_launch.finish_generation(retained_map(35, 36));
        mismatched_launch.accept_setup("RandMap.Sed");
        assert!(
            mismatched_launch
                .take_for_loading(Some("Other.Sed"))
                .is_none()
        );
        assert!(
            mismatched_launch
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "a refused nonmatching launch invalidates the prior accepted map"
        );

        let mut accepted_after_successful_close = RandomMapGenerationRetention::default();
        accepted_after_successful_close.finish_generation(retained_map(44, 40));
        accepted_after_successful_close.accept_setup("RandMap.Sed");
        // App::dismiss_random_map_setup has no retention side effect after OK.
        accepted_after_successful_close.select_map("RANDMAP.SED");
        let transferred = accepted_after_successful_close
            .take_for_loading(Some("randmap.sed"))
            .expect("successful accept close preserves the newly accepted map");
        assert_eq!(transferred.start_waypoints, vec![(0, 40, 20)]);
        assert!(
            accepted_after_successful_close
                .take_for_loading(Some("RandMap.Sed"))
                .is_none(),
            "successful accept still transfers exactly once"
        );
    }
}
