use super::RandomMapGenerationRetention;
use super::frontend::skirmish_session::{OfflineSkirmishRuntime, skirmish_global_defaults};
use super::loading::pump::{LoadingProgressSink, LoadingRequest};
use super::shell_random_map::teardown_then;
use crate::map::rmg::{RmgConstructionPhase, RmgConstructionTrace, RmgOptions, RmgRng};
use crate::skirmish_launch::{
    AiDifficulty, LaunchCountry, LaunchStartPosition, LaunchTeam, SkirmishAiSlot,
    SkirmishLaunchMode, SkirmishLaunchOptions, SkirmishLaunchSession, SkirmishLocalSlot,
};
use crate::ui::skirmish_shell::{
    AcceptOutcome, ChooseMapSelection, RandomMapSetupModalState, SkirmishShellState,
};

struct SilentProgress;

impl LoadingProgressSink for SilentProgress {
    fn milestone(&mut self, _percent: u32) {}
}

fn launch_session(seed_name: &str) -> SkirmishLaunchSession {
    SkirmishLaunchSession {
        mode: SkirmishLaunchMode {
            id: 1,
            ui_name_key: "GUI:Battle".to_string(),
            tooltip_key: "STT:ModeBattle".to_string(),
            override_file: "MPBattleMD.ini".to_string(),
            map_filter: "standard".to_string(),
            random_maps_allowed: true,
            allies_allowed: true,
            must_ally: false,
        },
        selected_map_file: Some(seed_name.to_string()),
        player_name: "Player".to_string(),
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
        options: SkirmishLaunchOptions::default(),
    }
}

fn generate_from_sed(
    seed_dir: &std::path::Path,
    seed_name: &str,
    assets: &mut crate::assets::asset_manager::AssetManager,
    match_seed: u32,
) -> super::loading::init::RandomMapLaunchSnapshot {
    super::loading::init::load_map_initial_with_assets(
        seed_dir.to_path_buf(),
        assets,
        Some(seed_name),
        &mut SilentProgress,
    )
    .expect("production .SED generation")
    .into_random_map_launch_snapshot(assets, match_seed, None)
}

#[test]
#[ignore] // Requires RA2_DIR (active-retail YR files).
fn gsi_04_12_random_map_ui_to_sed_launch_lifecycle_converges() {
    let retail_dir = std::path::PathBuf::from(
        std::env::var("RA2_DIR").expect("set RA2_DIR to the active-retail YR directory"),
    );
    assert!(retail_dir.join("gamemd.exe").is_file());
    crate::map::rmg::trig::install_from_dir(&retail_dir);
    let mut assets = crate::assets::asset_manager::AssetManager::new(&retail_dir)
        .expect("active-retail AssetManager");
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let seed_dir = std::env::temp_dir().join(format!(
        "vera20k-rmg-ui-launch-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&seed_dir).expect("lifecycle seed directory");
    let seed_name = "Unit2Lifecycle.Sed";
    let options = RmgOptions {
        theater: 0,
        map_type: 3,
        resources: 2,
        ruggedness: 35,
        water_amount: 50,
        num_players: 2,
        tiberium: 35,
        vegetation: 30,
        urban_presence: 40,
        seed: 4242,
        description: "Unit 2 lifecycle".to_string(),
        ..Default::default()
    };
    std::fs::write(seed_dir.join(seed_name), options.to_sed_bytes()).expect("write lifecycle .SED");
    const MATCH_SEED: u32 = 0x0412_0419;

    // U2-02: instrument the actual generator entry. Every Generate repeats
    // from fresh MapGen state; accepting a valid preview adds no run, while
    // accepting an empty setup adds exactly one.
    crate::map::rmg::build::reset_test_generator_entry_count();
    let previous_selection = ChooseMapSelection {
        mode_id: 1,
        record_index: Some(7),
    };
    let mut generated_modal =
        RandomMapSetupModalState::open(options.clone(), Some(previous_selection), false);
    generated_modal.begin_generate();
    let preview_a = generate_from_sed(&seed_dir, seed_name, &mut assets, MATCH_SEED);
    generated_modal.finish_generate(None);
    assert_eq!(crate::map::rmg::build::test_generator_entry_count(), 1);
    generated_modal.begin_generate();
    let preview_b = generate_from_sed(&seed_dir, seed_name, &mut assets, MATCH_SEED);
    generated_modal.finish_generate(None);
    assert_eq!(crate::map::rmg::build::test_generator_entry_count(), 2);
    assert_eq!(
        preview_a, preview_b,
        "repeated Generate must restart MapGen"
    );
    assert!(matches!(generated_modal.accept(), AcceptOutcome::Commit(_)));
    assert_eq!(
        crate::map::rmg::build::test_generator_entry_count(),
        2,
        "Use Map with a valid preview performs no third generation"
    );

    let mut empty_modal =
        RandomMapSetupModalState::open(options.clone(), Some(previous_selection), false);
    assert_eq!(empty_modal.accept(), AcceptOutcome::NeedsGenerate);
    empty_modal.begin_generate();
    let generated_for_accept = generate_from_sed(&seed_dir, seed_name, &mut assets, MATCH_SEED);
    empty_modal.finish_generate(None);
    assert!(matches!(empty_modal.accept(), AcceptOutcome::Commit(_)));
    assert_eq!(crate::map::rmg::build::test_generator_entry_count(), 3);
    assert_eq!(preview_a, generated_for_accept);

    // U2-03: preview constructor effects remain on the process shell Scenario
    // owner after Cancel; the chooser selection and edited options survive,
    // common teardown publishes only the preview product.
    let mut runtime = OfflineSkirmishRuntime::initialize(
        0x0412_0003,
        None,
        None,
        skirmish_global_defaults(&SkirmishShellState::default()),
    );
    runtime.remember_random_map_options(&options);
    let shell_before = runtime.scenario_rng_logical_state_for_test();
    let mut cancel_trace = RmgConstructionTrace::default();
    cancel_trace.push_emitted(
        RmgConstructionPhase::BridgeRepairHut,
        "CABHUT".to_string(),
        0,
        (10, 11),
    );
    cancel_trace.push_discarded(RmgConstructionPhase::NeutralTech, "CAOILD".to_string());
    runtime.replay_random_map_preview_construction(&cancel_trace);
    let shell_after_preview = runtime.scenario_rng_logical_state_for_test();
    assert_ne!(shell_after_preview, shell_before);
    let cancel_modal =
        RandomMapSetupModalState::open(options.clone(), Some(previous_selection), false);
    assert_eq!(cancel_modal.cancel(), Some(previous_selection));
    runtime.remember_random_map_options(&cancel_modal.options);

    struct CancelTransaction {
        img: std::path::PathBuf,
        sed: std::path::PathBuf,
        selection: ChooseMapSelection,
    }
    let cancel_img = seed_dir.join("CancelPreview.img");
    let cancel_sed = seed_dir.join("CancelPreview.Sed");
    let mut cancel_transaction = CancelTransaction {
        img: cancel_img.clone(),
        sed: cancel_sed.clone(),
        selection: previous_selection,
    };
    teardown_then(
        &mut cancel_transaction,
        |transaction| {
            std::fs::write(&transaction.img, b"completed-preview")
                .expect("common teardown preview write");
        },
        |_| (),
    );
    assert_eq!(
        std::fs::read(&cancel_img).expect("cancel preview product"),
        b"completed-preview"
    );
    assert!(!cancel_transaction.sed.exists());
    assert_eq!(cancel_transaction.selection, previous_selection);
    assert_eq!(
        runtime.random_map_options_for_setup(),
        options,
        "Cancel/reopen preserves the process MapSeed record"
    );
    assert_eq!(
        runtime.scenario_rng_logical_state_for_test(),
        shell_after_preview,
        "reopen must continue the preview-advanced shell Scenario cursor"
    );

    // U2-04/U2-05/U2-19: transport a deliberately poisoned accepted preview
    // through the same retention/request boundary used by Start. The request's
    // production initial-map entry must regenerate from .SED and converge with
    // a direct .SED launch in every generated gameplay fact.
    let mut poison_options = options.clone();
    poison_options.seed = 0x7BAD;
    let mut poison_map = crate::map::rmg::emit::empty_map_file(&poison_options, 32, 32);
    poison_map.header.theater = "POISON_PREVIEW".to_string();
    let mut poison_trace = RmgConstructionTrace::default();
    poison_trace.push_discarded(RmgConstructionPhase::NeutralTech, "POISON_TECH".to_string());
    let poison_trace_reference = poison_trace.clone();
    let poison_mapgen_reference = crate::sim::rng::SimRng::from_mapgen_continuation(
        RmgRng::new(poison_options.seed_u16()).into_continuation(),
    )
    .logical_state();
    let poison = crate::map::rmg::GeneratedMap {
        map_file: poison_map,
        mapgen_continuation: RmgRng::new(poison_options.seed_u16()).into_continuation(),
        construction_trace: poison_trace,
        start_waypoints: vec![(0, 1, 1)],
        stages_run: Vec::new(),
        unfilled_start_slots: 0,
    };
    let mut retention = RandomMapGenerationRetention::default();
    retention.finish_generation(poison);
    retention.accept_setup(seed_name);
    retention.destroy_map_storage();
    let accepted_poison = retention
        .take_preview_for_loading(Some(seed_name))
        .expect("accepted setup transfers presentation preview once");
    let launch = launch_session(seed_name);
    let ui_request = LoadingRequest::unverified_legacy_skirmish(
        launch.clone(),
        crate::match_bootstrap::MatchSeed {
            value: MATCH_SEED,
            source: crate::match_bootstrap::MatchSeedSource::Controlled,
            seed_authority_certifying: false,
        },
        crate::ui::main_menu::SkirmishSettings::default(),
    )
    .with_random_map_preview(Some(accepted_poison));
    let ui_initial = ui_request
        .load_initial_with_assets(seed_dir.clone(), &mut assets, &mut SilentProgress)
        .expect("accepted UI .SED launch regeneration");
    let descriptor = crate::sim::scenario_bootstrap::MatchLaunchDescriptor::from_resolved(
        launch.clone(),
    )
    .expect("resolved random-map launch descriptor");
    let ui_plan = crate::sim::scenario_bootstrap::preload_standard_battle_start_plan(
        &descriptor,
        ui_initial.map_data(),
        MATCH_SEED,
    )
    .expect("generated map supplies complete Battle starts");
    let ui_launch = ui_initial.into_random_map_launch_snapshot(
        &mut assets,
        MATCH_SEED,
        Some(&ui_plan),
    );
    let direct_initial = super::loading::init::load_map_initial_with_assets(
        seed_dir.clone(),
        &mut assets,
        Some(seed_name),
        &mut SilentProgress,
    )
    .expect("direct .SED launch regeneration");
    let direct_plan = crate::sim::scenario_bootstrap::preload_standard_battle_start_plan(
        &descriptor,
        direct_initial.map_data(),
        MATCH_SEED,
    )
    .expect("direct generated map supplies complete Battle starts");
    let direct_launch = direct_initial.into_random_map_launch_snapshot(
        &mut assets,
        MATCH_SEED,
        Some(&direct_plan),
    );
    assert_eq!(crate::map::rmg::build::test_generator_entry_count(), 5);
    assert_eq!(ui_launch, direct_launch);
    assert_ne!(ui_launch.map.header.0, "POISON_PREVIEW");
    assert_ne!(ui_launch.trace, poison_trace_reference);
    assert_ne!(ui_launch.mapgen_continuation, poison_mapgen_reference);
    assert!(
        !ui_launch.trace.events.is_empty(),
        "retail fixture must exercise launch constructor replay"
    );
    assert_eq!(
        ui_launch.emitted_constructor_words,
        direct_launch.emitted_constructor_words
    );
    assert_eq!(
        ui_launch.scenario_after_trace,
        direct_launch.scenario_after_trace
    );

    std::fs::remove_file(seed_dir.join(seed_name)).expect("remove lifecycle .SED");
    std::fs::remove_file(cancel_img).expect("remove cancel preview");
    std::fs::remove_dir(seed_dir).expect("remove lifecycle seed directory");
}
