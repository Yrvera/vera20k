# Module map

Generated snapshot: `ca38a5824bb467fa6cb31167ecbf4e93e0a80152` (2026-09-14), cargo-modules 0.26.0.

Scope: `vera20k` library, default features, `x86_64-pc-windows-msvc`, no depth limit.
Test-only, binary-specific and inactive conditional modules are excluded.
This snapshot contains 808 modules; check its source commit against your checkout.

Regenerate from the repository root with the command below, replace the tree, and
update the source commit and build scope above.

```powershell
$env:NO_COLOR = "1"
cargo modules structure --lib -p vera20k --no-fns --no-traits --no-types
```

```text
crate vera20k
├── mod app: pub
│   ├── mod audio_runtime: pub(crate)
│   ├── mod diagnostics: pub(crate)
│   │   ├── mod debug_overlays: pub(crate)
│   │   ├── mod debug_panel: pub(crate)
│   │   ├── mod dev_overlay: pub(crate)
│   │   ├── mod shell_capture: pub(crate)
│   │   │   └── mod skirmish: pub(self)
│   │   ├── mod state: pub(crate)
│   │   └── mod tactical_capture: pub(crate)
│   │       ├── mod evidence: pub(crate)
│   │       ├── mod integrity: pub(crate)
│   │       ├── mod manifest: pub(crate)
│   │       ├── mod placement: pub(crate)
│   │       ├── mod profile: pub(crate)
│   │       ├── mod script: pub(crate)
│   │       └── mod session: pub(crate)
│   ├── mod frame: pub(self)
│   ├── mod frontend: pub
│   │   ├── mod launch: pub
│   │   ├── mod list_maps: pub(crate)
│   │   ├── mod main_menu_shell_render: pub(crate)
│   │   ├── mod quit_cascade: pub(crate)
│   │   ├── mod score_shell_render: pub(crate)
│   │   ├── mod shell_transition: pub(crate)
│   │   ├── mod single_player_shell_render: pub(crate)
│   │   ├── mod skirmish: pub(crate)
│   │   ├── mod skirmish_session: pub(crate)
│   │   ├── mod skirmish_shell_render: pub(crate)
│   │   │   ├── mod abort: pub(self)
│   │   │   ├── mod chrome: pub(self)
│   │   │   ├── mod controls: pub(self)
│   │   │   ├── mod draw_order: pub(self)
│   │   │   ├── mod in_game_options: pub(self)
│   │   │   ├── mod in_game_shell: pub(self)
│   │   │   ├── mod keyboard: pub(self)
│   │   │   ├── mod launcher_options: pub(self)
│   │   │   ├── mod list: pub(self)
│   │   │   ├── mod modals: pub(self)
│   │   │   ├── mod pause_menu: pub(self)
│   │   │   ├── mod preview: pub(self)
│   │   │   ├── mod saved_games: pub(self)
│   │   │   ├── mod sound: pub(self)
│   │   │   └── mod text: pub(self)
│   │   ├── mod startup_options: pub
│   │   ├── mod startup_splash: pub(super)
│   │   └── mod state: pub(crate)
│   ├── mod handler: pub(self)
│   ├── mod in_game: pub(self)
│   ├── mod initialize: pub(self)
│   ├── mod input: pub(crate)
│   │   ├── mod abort: pub(crate)
│   │   ├── mod camera: pub(crate)
│   │   ├── mod commands: pub(crate)
│   │   ├── mod context_order: pub(crate)
│   │   ├── mod cursor: pub(crate)
│   │   ├── mod dispatch: pub(crate)
│   │   │   └── mod selection_navigation: pub(crate)
│   │   ├── mod entity_pick: pub(crate)
│   │   ├── mod gadget_input: pub(crate)
│   │   │   └── mod command_bar: pub(self)
│   │   ├── mod hotkeys: pub(crate)
│   │   │   └── mod catalog: pub(crate)
│   │   ├── mod in_game_options: pub(crate)
│   │   ├── mod keyboard: pub(crate)
│   │   ├── mod messages: pub(crate)
│   │   ├── mod pause_menu: pub(crate)
│   │   ├── mod sidebar_eva: pub(crate)
│   │   ├── mod sound: pub(crate)
│   │   ├── mod state: pub(crate)
│   │   ├── mod tooltips: pub(crate)
│   │   └── mod transport_orders: pub(crate)
│   ├── mod loading: pub(crate)
│   │   ├── mod composition: pub(crate)
│   │   ├── mod fresh_scenario: pub(crate)
│   │   ├── mod init: pub(crate)
│   │   ├── mod init_helpers: pub(crate)
│   │   ├── mod progress_row: pub(crate)
│   │   ├── mod pump: pub(crate)
│   │   │   └── mod render: pub(self)
│   │   └── mod transitions: pub(crate)
│   ├── mod match_audio: pub(crate)
│   ├── mod match_diagnostics: pub(crate)
│   ├── mod match_runtime: pub(crate)
│   │   ├── mod eva_producers: pub(crate)
│   │   ├── mod frame_pacer: pub(crate)
│   │   ├── mod restore: pub(crate)
│   │   ├── mod scenario_exit: pub(crate)
│   │   ├── mod sim_tick: pub(crate)
│   │   ├── mod sound_dispatch: pub(crate)
│   │   ├── mod startup: pub(crate)
│   │   └── mod state: pub(crate)
│   ├── mod persistence: pub(crate)
│   │   ├── mod commands: pub(crate)
│   │   ├── mod keyboard: pub(crate)
│   │   ├── mod options: pub(crate)
│   │   │   ├── mod audio: pub(crate)
│   │   │   └── mod launcher: pub(crate)
│   │   ├── mod options_profile: pub(crate)
│   │   └── mod save_load_panel: pub(crate)
│   ├── mod presentation: pub(crate)
│   │   ├── mod building_anim: pub(crate)
│   │   ├── mod chute_anim: pub(crate)
│   │   ├── mod combat_lights: pub(crate)
│   │   ├── mod fire_effects: pub(crate)
│   │   ├── mod instances: pub(crate)
│   │   │   ├── mod bridges: pub
│   │   │   ├── mod foot_depth: pub(self)
│   │   │   ├── mod helpers: pub(self)
│   │   │   ├── mod overlays: pub(self)
│   │   │   ├── mod particles: pub(self)
│   │   │   ├── mod shp: pub(self)
│   │   │   └── mod units: pub(self)
│   │   ├── mod lighting: pub(crate)
│   │   ├── mod overlay_index: pub(crate)
│   │   ├── mod radiation_light: pub(crate)
│   │   ├── mod render: pub(crate)
│   │   │   ├── mod build_instances: pub(self)
│   │   │   ├── mod draw_passes: pub(self)
│   │   │   ├── mod draw_plan_lowering: pub(crate)
│   │   │   ├── mod merge_passes: pub(self)
│   │   │   └── mod minimap_transaction: pub(crate)
│   │   ├── mod selection_brackets: pub(crate)
│   │   ├── mod sidebar_build: pub(crate)
│   │   │   └── mod command_bar: pub(crate)
│   │   ├── mod sidebar_gadgets: pub(crate)
│   │   ├── mod sidebar_render: pub(crate)
│   │   ├── mod sidebar_text: pub(crate)
│   │   ├── mod spawn_pick: pub(crate)
│   │   ├── mod state: pub(crate)
│   │   ├── mod target_lines: pub(crate)
│   │   └── mod ui_overlays: pub(crate)
│   ├── mod process_assets: pub(crate)
│   ├── mod renderer_state: pub(crate)
│   ├── mod scenario_catalog: pub(crate)
│   ├── mod shell_main_menu: pub(self)
│   ├── mod shell_random_map: pub(self)
│   ├── mod shell_route: pub(crate)
│   ├── mod shell_saved_games: pub(self)
│   ├── mod shell_saved_seeds: pub(self)
│   ├── mod shell_skirmish: pub(self)
│   ├── mod sidebar_projection: pub(crate)
│   ├── mod state: pub(self)
│   │   └── mod platform: pub(self)
│   └── mod types: pub(crate)
├── mod asset_tools: pub
│   ├── mod args: pub
│   ├── mod canvas: pub
│   ├── mod identify: pub
│   ├── mod locate: pub
│   ├── mod names: pub
│   ├── mod palette: pub
│   ├── mod palette_production: pub
│   ├── mod render_dispatch: pub
│   ├── mod render_still: pub
│   ├── mod render_tmp: pub
│   ├── mod render_vxl: pub
│   ├── mod report: pub
│   ├── mod root: pub
│   ├── mod verb_art: pub
│   ├── mod verb_compare: pub
│   ├── mod verb_csf: pub
│   ├── mod verb_extract: pub
│   ├── mod verb_find: pub
│   ├── mod verb_info: pub
│   ├── mod verb_ls: pub
│   ├── mod verb_palette: pub
│   ├── mod verb_parse_check: pub
│   ├── mod verb_render: pub
│   ├── mod verb_scan: pub
│   └── mod verb_sound: pub
├── mod assets: pub
│   ├── mod asset_manager: pub
│   ├── mod aud_file: pub
│   ├── mod audio_bag: pub
│   ├── mod bink_audio: pub
│   ├── mod bink_audio_data: pub
│   ├── mod bink_bits: pub
│   ├── mod bink_data: pub
│   ├── mod bink_decode: pub
│   ├── mod bink_file: pub
│   ├── mod csf_file: pub
│   ├── mod error: pub
│   ├── mod fnt_file: pub
│   ├── mod format_sniff: pub
│   ├── mod hva_file: pub
│   ├── mod ima_adpcm: pub
│   ├── mod mix_archive: pub
│   ├── mod mix_crypto: pub
│   ├── mod mix_hash: pub
│   ├── mod pal_file: pub
│   ├── mod pcx_file: pub
│   ├── mod shp_decode: pub
│   ├── mod shp_file: pub
│   ├── mod tmp_decode: pub
│   ├── mod tmp_file: pub
│   ├── mod vpl_file: pub
│   ├── mod vxl_decode: pub
│   ├── mod vxl_file: pub
│   ├── mod wav_file: pub(crate)
│   └── mod xcc_database: pub
├── mod audio: pub
│   ├── mod arbiter: pub
│   │   └── mod event_flags: pub(self)
│   ├── mod events: pub
│   ├── mod music: pub
│   ├── mod sfx: pub
│   ├── mod theme: pub(crate)
│   ├── mod voice_queue: pub
│   └── mod vox: pub
├── mod headless_scenario: pub
├── mod map: pub
│   ├── mod actions: pub
│   ├── mod authored_overlay: pub(crate)
│   ├── mod basic: pub
│   ├── mod bridge_facts: pub
│   ├── mod bridge_pavement: pub(crate)
│   ├── mod bridge_rim_tiles: pub(crate)
│   ├── mod briefing: pub
│   ├── mod cell_index: pub
│   ├── mod cell_tags: pub
│   ├── mod construction_trace: pub(crate)
│   ├── mod entities: pub
│   ├── mod events: pub
│   ├── mod houses: pub
│   ├── mod iso_tile_flood: pub(crate)
│   ├── mod lat: pub
│   ├── mod lighting: pub
│   ├── mod map_file: pub
│   ├── mod overlay: pub
│   ├── mod overlay_types: pub
│   ├── mod playfield: pub
│   ├── mod preview: pub
│   ├── mod resolved_terrain: pub
│   │   ├── mod mutation: pub(self)
│   │   ├── mod pavement: pub(self)
│   │   ├── mod recalc_catalog: pub(self)
│   │   └── mod zone_class: pub
│   ├── mod retail_trig: pub
│   ├── mod rmg: pub
│   │   ├── mod build: pub
│   │   ├── mod description: pub(self)
│   │   ├── mod emit: pub
│   │   ├── mod grid: pub
│   │   ├── mod options: pub
│   │   ├── mod phases: pub
│   │   │   ├── mod adjacency: pub
│   │   │   ├── mod area: pub
│   │   │   ├── mod blob: pub
│   │   │   ├── mod bridge: pub
│   │   │   ├── mod bridge_deck: pub
│   │   │   ├── mod carve: pub
│   │   │   ├── mod carve_driver: pub
│   │   │   ├── mod connector: pub
│   │   │   ├── mod green_spread: pub
│   │   │   ├── mod hills: pub
│   │   │   ├── mod hills_corners: pub
│   │   │   ├── mod island_passes: pub
│   │   │   ├── mod lake: pub
│   │   │   ├── mod lat_fixup: pub
│   │   │   ├── mod lat_patches: pub
│   │   │   ├── mod meander: pub
│   │   │   ├── mod ramp: pub
│   │   │   ├── mod regions: pub
│   │   │   ├── mod river: pub
│   │   │   ├── mod rocks: pub
│   │   │   ├── mod shore: pub
│   │   │   ├── mod starts: pub
│   │   │   ├── mod tech_buildings: pub
│   │   │   ├── mod tiberium: pub
│   │   │   ├── mod trees: pub
│   │   │   ├── mod water: pub
│   │   │   ├── mod water_finalize: pub
│   │   │   └── mod zones: pub
│   │   ├── mod pipeline: pub
│   │   ├── mod preview: pub
│   │   ├── mod randomize: pub
│   │   ├── mod rng: pub
│   │   ├── mod saved_seeds: pub
│   │   ├── mod scratch: pub
│   │   ├── mod settings: pub
│   │   ├── mod sqrt_table: pub
│   │   ├── mod tech_catalog: pub
│   │   ├── mod theater_blocks: pub
│   │   ├── mod tiles: pub
│   │   ├── mod trig: pub
│   │   └── mod x87: pub
│   ├── mod scenario_menu: pub
│   ├── mod skirmish_scenarios: pub
│   ├── mod tags: pub
│   ├── mod terrain: pub
│   ├── mod theater: pub
│   ├── mod tile_variant_selector: pub
│   ├── mod trigger_graph: pub
│   ├── mod triggers: pub
│   ├── mod tube_facts: pub
│   ├── mod tubes: pub
│   ├── mod variable_names: pub
│   └── mod waypoints: pub
├── mod match_bootstrap: pub
├── mod net: pub
│   └── mod lockstep: pub
├── mod render: pub
│   ├── mod batch: pub
│   ├── mod bink_movie: pub
│   ├── mod bit_font: pub
│   ├── mod bridge_atlas: pub
│   ├── mod bridge_railing_atlas: pub
│   ├── mod building_light: pub
│   ├── mod building_zshape: pub
│   ├── mod combat_light: pub
│   ├── mod current_radar_cell: pub(self)
│   ├── mod cursor_atlas: pub
│   ├── mod draw_state: pub
│   ├── mod egui_integration: pub
│   ├── mod foot_depth: pub(crate)
│   ├── mod frame_readback: pub
│   ├── mod gpu: pub
│   ├── mod loading_screen_chrome: pub
│   ├── mod locomotor_visual: pub
│   ├── mod main_menu_shell_chrome: pub
│   ├── mod minimap: pub
│   ├── mod minimap_helpers: pub(self)
│   ├── mod minimap_interaction: pub(self)
│   ├── mod minimap_legacy_events: pub(self)
│   ├── mod minimap_projection: pub(self)
│   ├── mod native_radar_surface: pub(self)
│   ├── mod native_radar_terrain: pub(self)
│   ├── mod native_radar_viewport: pub(self)
│   ├── mod native_surface_format: pub
│   ├── mod native_z: pub
│   ├── mod overlay_assets: pub
│   ├── mod overlay_atlas: pub
│   ├── mod palette_light: pub
│   ├── mod palette_textures: pub
│   ├── mod pixel_fx_sparkles: pub
│   ├── mod radar_anim: pub
│   ├── mod radar_animation: pub(self)
│   ├── mod radar_events: pub(self)
│   ├── mod radar_surface: pub(self)
│   ├── mod radar_terrain_updates: pub(self)
│   ├── mod radar_tracker: pub(self)
│   ├── mod radar_visibility: pub(self)
│   ├── mod screenshot: pub
│   ├── mod selection_overlay: pub
│   ├── mod shell_paint: pub
│   ├── mod shell_surface_present: pub
│   ├── mod shell_text: pub
│   ├── mod shell_text_reveal: pub
│   ├── mod shell_transition_pass: pub
│   ├── mod shroud_buffer: pub
│   ├── mod sidebar_cameo_atlas: pub
│   ├── mod sidebar_chrome: pub
│   │   ├── mod command_bar: pub(self)
│   │   └── mod in_game_shell: pub(self)
│   ├── mod sidebar_text: pub
│   ├── mod skirmish_shell_chrome: pub
│   ├── mod smudge: pub
│   ├── mod sprite_atlas: pub
│   ├── mod tactical_compat: pub
│   ├── mod tactical_draw_plan: pub
│   ├── mod tactical_shader: pub(crate)
│   ├── mod terrain_draw: pub(crate)
│   │   └── mod batching: pub(self)
│   ├── mod terrain_instances: pub
│   ├── mod tile_atlas: pub
│   ├── mod unit_atlas: pub
│   │   └── mod shadow_cache: pub(self)
│   ├── mod unit_slope_transition_cache: pub
│   ├── mod upscale_pass: pub
│   ├── mod vxl_compute: pub
│   ├── mod vxl_normals: pub
│   ├── mod vxl_raster: pub
│   │   ├── mod native: pub(self)
│   │   └── mod shadow: pub(crate)
│   └── mod wave_geometry: pub
├── mod rng_continuation: pub(crate)
├── mod rules: pub
│   ├── mod animation_sequence: pub
│   ├── mod art_data: pub
│   ├── mod bridge_warheads: pub
│   ├── mod color_add: pub
│   ├── mod color_scheme: pub
│   ├── mod combat_damage: pub
│   ├── mod crate_rules: pub
│   ├── mod effect_asset_catalog: pub
│   ├── mod error: pub
│   ├── mod flh: pub
│   ├── mod foundation: pub
│   ├── mod house_colors: pub
│   ├── mod infantry_sequence: pub
│   ├── mod ini_enum: pub
│   ├── mod ini_parser: pub
│   ├── mod ini_value: pub
│   ├── mod jumpjet_params: pub
│   ├── mod locomotor_type: pub
│   ├── mod missile_spawn: pub
│   │   └── mod retail_defaults: pub(self)
│   ├── mod mission_data: pub
│   ├── mod native_processing: pub
│   ├── mod object_type: pub
│   ├── mod overlay_types: pub
│   ├── mod particle_system_type: pub
│   ├── mod particle_type: pub
│   ├── mod powerups: pub
│   ├── mod process_owner: pub(crate)
│   ├── mod projectile_type: pub
│   ├── mod radar_event_config: pub
│   ├── mod ruleset: pub
│   ├── mod shp_vehicle_sequence: pub
│   ├── mod smudge_type: pub
│   ├── mod sound_ini: pub
│   │   ├── mod control: pub
│   │   └── mod sound_type: pub
│   ├── mod superweapon_type: pub
│   ├── mod team_ai_ini: pub
│   ├── mod terrain_asset_catalog: pub
│   ├── mod terrain_object_type: pub
│   ├── mod terrain_rules: pub
│   ├── mod tiberium_type: pub
│   ├── mod voxel_anim_type: pub
│   ├── mod warhead_type: pub
│   └── mod weapon_type: pub
├── mod sidebar: pub
│   ├── mod command_bar: pub
│   ├── mod gadget_flash: pub
│   ├── mod layout_spec: pub(self)
│   ├── mod power_bar_anim: pub
│   └── mod sidebar_view: pub(self)
├── mod sim: pub
│   ├── mod ai: pub
│   ├── mod ai_buildable: pub(crate)
│   ├── mod aircraft: pub
│   │   ├── mod attack_mission: pub
│   │   ├── mod drop_payload: pub
│   │   ├── mod idle_mode: pub
│   │   ├── mod paradrop_mission: pub
│   │   └── mod runtime_contract: pub
│   ├── mod anim_class: pub
│   ├── mod animation: pub
│   ├── mod base_plan: pub(crate)
│   ├── mod base_plan_generation: pub(crate)
│   ├── mod bounce: pub
│   ├── mod bridge_specs: pub
│   ├── mod bridge_state: pub
│   │   ├── mod damaged_variant: pub(self)
│   │   ├── mod gap_restamp: pub(crate)
│   │   ├── mod publication: pub(crate)
│   │   ├── mod ramp_repair: pub(crate)
│   │   ├── mod record_scan: pub(self)
│   │   ├── mod repair_occupants: pub(crate)
│   │   ├── mod rim: pub(crate)
│   │   ├── mod walker: pub
│   │   └── mod zone_activation: pub(self)
│   ├── mod capture_manager: pub
│   ├── mod cell_kernel: pub
│   ├── mod cell_rect: pub
│   ├── mod cloak_disguise: pub
│   │   └── mod transitions: pub(crate)
│   ├── mod combat: pub
│   │   ├── mod base_defense_response: pub(crate)
│   │   │   └── mod admission: pub(self)
│   │   ├── mod cell_spread: pub(crate)
│   │   ├── mod combat_aoe: pub(crate)
│   │   ├── mod combat_fire_gate: pub(crate)
│   │   ├── mod combat_targeting: pub(crate)
│   │   ├── mod combat_weapon: pub(crate)
│   │   ├── mod damage: pub(crate)
│   │   │   ├── mod attacker: pub(crate)
│   │   │   ├── mod gates: pub(crate)
│   │   │   ├── mod kernel: pub(crate)
│   │   │   └── mod receive: pub(crate)
│   │   ├── mod fire_decision: pub(crate)
│   │   ├── mod greatest_threat: pub(crate)
│   │   ├── mod in_range: pub(crate)
│   │   ├── mod inviso_scatter: pub(self)
│   │   ├── mod line_of_fire: pub(crate)
│   │   ├── mod receiver_health: pub(self)
│   │   ├── mod smudge_dispatch: pub
│   │   ├── mod threat_range: pub(crate)
│   │   ├── mod veterancy: pub(crate)
│   │   └── mod world_receiver: pub(crate)
│   ├── mod command: pub
│   ├── mod components: pub
│   ├── mod crates: pub
│   │   ├── mod runtime: pub(self)
│   │   └── mod state: pub(self)
│   │       └── mod crate_slot_array_serde: pub(self)
│   ├── mod credit_income: pub
│   ├── mod debug_event_log: pub
│   ├── mod deploy: pub
│   ├── mod docking: pub
│   │   ├── mod aircraft_dock: pub
│   │   ├── mod building_dock: pub
│   │   ├── mod bunker_install: pub
│   │   ├── mod bunker_link: pub
│   │   └── mod pad_geometry: pub
│   ├── mod economy: pub
│   ├── mod entity_store: pub
│   ├── mod find_nearby_cell: pub
│   ├── mod game_entity: pub
│   ├── mod game_options: pub
│   ├── mod gate_runtime: pub
│   ├── mod house_eva: pub
│   ├── mod house_state: pub
│   ├── mod house_strategy: pub(crate)
│   ├── mod infantry: pub
│   ├── mod intern: pub
│   ├── mod lifecycle_request: pub(crate)
│   ├── mod light_sources: pub(crate)
│   ├── mod map: pub
│   │   ├── mod bridge_occupancy_shadow: pub
│   │   └── mod bridge_topology: pub
│   ├── mod mcv_deploy: pub(crate)
│   ├── mod miner: pub
│   │   ├── mod harvest_mission: pub(self)
│   │   ├── mod miner_dock: pub
│   │   ├── mod miner_dock_sequence: pub(self)
│   │   └── mod miner_system: pub(crate)
│   ├── mod mission: pub
│   │   ├── mod authority: pub(crate)
│   │   │   └── mod ready_private: pub(self)
│   │   ├── mod concrete_effects: pub(crate)
│   │   │   └── mod private: pub(self)
│   │   ├── mod control: pub
│   │   ├── mod leaf: pub(crate)
│   │   ├── mod readiness: pub(crate)
│   │   ├── mod retask: pub
│   │   ├── mod state: pub
│   │   ├── mod timer: pub
│   │   └── mod verb: pub
│   ├── mod movement: pub
│   │   ├── mod air_movement: pub
│   │   ├── mod at_coord: pub(crate)
│   │   ├── mod bump_crush: pub
│   │   ├── mod cell_arrival: pub(self)
│   │   ├── mod drive_locomotion: pub(self)
│   │   ├── mod drive_track: pub
│   │   ├── mod drop_pod_movement: pub
│   │   ├── mod facing_class: pub
│   │   ├── mod ground_pose: pub(crate)
│   │   ├── mod group_destination: pub
│   │   ├── mod homing_movement: pub
│   │   ├── mod hover: pub
│   │   ├── mod jumpjet_movement: pub
│   │   ├── mod locomotion: pub
│   │   │   ├── mod install: pub
│   │   │   ├── mod piggyback: pub
│   │   │   ├── mod power: pub
│   │   │   └── mod slot: pub
│   │   ├── mod locomotor: pub
│   │   ├── mod locomotor_owner: pub(crate)
│   │   ├── mod locomotor_ready: pub(crate)
│   │   ├── mod movement_blocked: pub(self)
│   │   ├── mod movement_bridge: pub(crate)
│   │   ├── mod movement_commands: pub(self)
│   │   ├── mod movement_occupancy: pub(self)
│   │   ├── mod movement_path: pub(self)
│   │   ├── mod movement_step: pub(self)
│   │   ├── mod movement_tick: pub(crate)
│   │   ├── mod navcom: pub(self)
│   │   ├── mod parachute_descent: pub
│   │   ├── mod path_markers: pub(self)
│   │   ├── mod ready_producer: pub(crate)
│   │   ├── mod rocket_movement: pub
│   │   ├── mod scatter: pub
│   │   ├── mod slope_transition: pub(crate)
│   │   ├── mod teleport_movement: pub
│   │   ├── mod track_head: pub(self)
│   │   ├── mod track_host: pub(self)
│   │   ├── mod track_process: pub(crate)
│   │   ├── mod tube_movement: pub
│   │   ├── mod tunnel_movement: pub
│   │   └── mod turret: pub
│   ├── mod multiplayer_checksum: pub
│   ├── mod native_identity: pub(crate)
│   ├── mod naval_base_placement: pub(crate)
│   ├── mod occupancy: pub
│   ├── mod ore_growth: pub
│   ├── mod ore_twinkle: pub(crate)
│   ├── mod overlay_grid: pub
│   ├── mod parity_digest: pub
│   ├── mod particles: pub
│   │   ├── mod fire: pub
│   │   ├── mod gas: pub
│   │   ├── mod ivec3_serde: pub(self)
│   │   ├── mod smoke: pub
│   │   ├── mod spark: pub
│   │   ├── mod spark_spawn: pub
│   │   ├── mod spark_world: pub
│   │   ├── mod spawn: pub
│   │   ├── mod system_ai: pub
│   │   └── mod wind: pub
│   ├── mod passenger: pub
│   │   └── mod departure: pub(self)
│   ├── mod pathfinding: pub
│   │   ├── mod cell_entry: pub
│   │   ├── mod core: pub(self)
│   │   ├── mod passability: pub
│   │   ├── mod path_smooth: pub
│   │   ├── mod terrain_cost: pub
│   │   ├── mod terrain_speed: pub
│   │   ├── mod zone_build: pub(crate)
│   │   ├── mod zone_hierarchy: pub(crate)
│   │   ├── mod zone_incremental: pub(crate)
│   │   ├── mod zone_map: pub
│   │   │   └── mod bridge_repair_zones: pub(self)
│   │   └── mod zone_search: pub
│   ├── mod power_system: pub
│   ├── mod production: pub
│   │   ├── mod factory: pub(self)
│   │   ├── mod factory_lifecycle: pub(self)
│   │   ├── mod production_economy: pub(self)
│   │   ├── mod production_placement: pub(self)
│   │   ├── mod production_queue: pub(self)
│   │   ├── mod production_refinery: pub(self)
│   │   ├── mod production_sell: pub(self)
│   │   ├── mod production_spawn: pub(self)
│   │   ├── mod production_tech: pub(self)
│   │   ├── mod production_types: pub(self)
│   │   ├── mod wall_placement: pub(self)
│   │   └── mod war_factory_exit: pub(self)
│   ├── mod projectile: pub
│   │   └── mod launch: pub(crate)
│   ├── mod radar: pub
│   ├── mod radiation: pub
│   ├── mod radiation_light: pub(crate)
│   ├── mod radio: pub
│   │   ├── mod contacts: pub
│   │   └── mod receive: pub
│   ├── mod replay: pub
│   ├── mod rng: pub
│   ├── mod rocking: pub
│   │   ├── mod impulse: pub
│   │   ├── mod rocking_system: pub
│   │   └── mod self_destruct: pub
│   ├── mod runtime: pub
│   ├── mod scenario_bootstrap: pub(crate)
│   ├── mod scenario_post_map: pub(crate)
│   ├── mod scenario_session: pub
│   ├── mod score: pub(crate)
│   ├── mod selection: pub
│   ├── mod sensor_lifecycle: pub
│   ├── mod slave_miner: pub
│   ├── mod smudge_grid: pub
│   ├── mod snapshot: pub
│   ├── mod spawn_manager: pub
│   ├── mod substrate: pub
│   │   └── mod locomotion: pub
│   │       ├── mod capability: pub
│   │       ├── mod class: pub
│   │       └── mod defaults: pub
│   ├── mod superweapon: pub
│   │   ├── mod cell_grid: pub
│   │   ├── mod force_shield: pub
│   │   ├── mod genetic_converter: pub
│   │   │   └── mod mutation: pub(self)
│   │   ├── mod invulnerability: pub
│   │   ├── mod iron_curtain: pub
│   │   ├── mod lightning_storm: pub
│   │   ├── mod paradrop: pub
│   │   └── mod psychic_reveal: pub
│   ├── mod team_script_vm: pub
│   │   └── mod registry_install: pub(self)
│   ├── mod terrain_object: pub
│   ├── mod terrain_spawn: pub
│   ├── mod tiberium: pub
│   ├── mod tiberium_germinate: pub(crate)
│   ├── mod timer: pub
│   ├── mod transport_unload: pub
│   ├── mod trigger_runtime: pub
│   ├── mod type_handle_table: pub
│   ├── mod vision: pub
│   │   ├── mod gap_source: pub(self)
│   │   ├── mod map_reveal: pub(self)
│   │   └── mod shroud_knowledge: pub(self)
│   ├── mod voxel_anim: pub
│   ├── mod voxel_frame_catalog: pub
│   ├── mod wave: pub
│   └── mod world: pub
│       ├── mod authored_load_host: pub(crate)
│       ├── mod bridge_orchestrator: pub(crate)
│       │   ├── mod ground_fallout: pub(self)
│       │   └── mod live_publication: pub(self)
│       │       ├── mod constructor_publication: pub(self)
│       │       ├── mod pavement_publication: pub(self)
│       │       ├── mod rim_publication: pub(self)
│       │       ├── mod tile_publication: pub(self)
│       │       └── mod zone_publication: pub(self)
│       ├── mod building_anim: pub(crate)
│       ├── mod command_schedule: pub(self)
│       ├── mod damage_consequences: pub(crate)
│       ├── mod edge_cell: pub
│       ├── mod gap_generator: pub(self)
│       ├── mod hash_schema: pub(self)
│       ├── mod infantry_terminal: pub(self)
│       ├── mod lifecycle: pub(self)
│       ├── mod load_object_lifecycle: pub(self)
│       ├── mod logic_vector: pub(self)
│       ├── mod navigation: pub(self)
│       ├── mod object_turn: pub(self)
│       ├── mod projectile_collision: pub(self)
│       ├── mod shroud_refresh: pub(self)
│       ├── mod substrate: pub(self)
│       ├── mod techno_ai: pub(self)
│       │   ├── mod bounce_terrain: pub(self)
│       │   └── mod mission_handlers: pub(self)
│       ├── mod techno_ai_cloak: pub(crate)
│       ├── mod track_cell_recalc: pub(self)
│       ├── mod unit_post: pub(crate)
│       ├── mod world_commands: pub(self)
│       ├── mod world_hash: pub(self)
│       ├── mod world_orders: pub(self)
│       └── mod world_spawn: pub(self)
│           └── mod construction: pub(self)
├── mod skirmish_cooperative: pub
├── mod skirmish_launch: pub
├── mod skirmish_modes: pub
├── mod skirmish_persistence: pub
├── mod ui: pub
│   ├── mod client_theme: pub
│   ├── mod gadget: pub
│   │   ├── mod button: pub
│   │   ├── mod focus: pub
│   │   ├── mod list: pub
│   │   └── mod tick: pub
│   ├── mod game_screen: pub
│   ├── mod main_menu: pub
│   ├── mod main_menu_dialogs: pub
│   │   └── mod options: pub(crate)
│   │       └── mod shell: pub(crate)
│   ├── mod main_menu_shell: pub
│   │   ├── mod layout: pub(self)
│   │   └── mod state: pub(self)
│   ├── mod messages: pub
│   ├── mod mission_status: pub
│   ├── mod pause_menu: pub
│   ├── mod score_shell: pub
│   ├── mod shell: pub
│   │   ├── mod abort: pub
│   │   ├── mod button: pub
│   │   ├── mod controller: pub
│   │   ├── mod descriptor: pub
│   │   ├── mod geom: pub
│   │   ├── mod in_game_options: pub
│   │   │   └── mod control: pub
│   │   ├── mod in_game_options_state: pub
│   │   ├── mod in_game_shell: pub
│   │   ├── mod keyboard: pub
│   │   ├── mod layout: pub
│   │   ├── mod list: pub
│   │   ├── mod modal: pub
│   │   │   └── mod control: pub
│   │   ├── mod pause_menu: pub
│   │   ├── mod saved_file_input: pub
│   │   ├── mod saved_games: pub
│   │   ├── mod slide: pub
│   │   ├── mod sound: pub
│   │   └── mod static_reveal: pub
│   ├── mod single_player_shell: pub
│   │   ├── mod layout: pub(self)
│   │   └── mod state: pub(self)
│   ├── mod skirmish_shell: pub
│   │   ├── mod layout: pub(self)
│   │   ├── mod scroll: pub(self)
│   │   ├── mod seed_list: pub
│   │   ├── mod state: pub(self)
│   │   │   ├── mod choose_map: pub(self)
│   │   │   ├── mod combos: pub(self)
│   │   │   ├── mod hit_test: pub(self)
│   │   │   ├── mod launch: pub(self)
│   │   │   ├── mod player_name: pub(self)
│   │   │   ├── mod random_map_setup: pub(self)
│   │   │   ├── mod saved_seed_browser: pub(self)
│   │   │   └── mod trackbars: pub(self)
│   │   └── mod static_reveal: pub
│   └── mod tooltips: pub
└── mod util: pub
    ├── mod base64: pub
    ├── mod config: pub
    ├── mod direction: pub
    ├── mod direction_tables: pub
    │   ├── mod cell: pub
    │   ├── mod dragon: pub
    │   ├── mod lepton: pub
    │   ├── mod native_angle: pub(self)
    │   ├── mod native_angle_table: pub(self)
    │   └── mod quantize: pub
    ├── mod facing_table: pub
    ├── mod fixed_math: pub
    ├── mod flh_transform: pub
    ├── mod fnv: pub
    ├── mod ini_writer: pub
    ├── mod lcw: pub
    ├── mod legacy_crt_rng: pub
    ├── mod lepton: pub
    ├── mod logging: pub
    ├── mod lzo: pub
    ├── mod native_file_name: pub
    ├── mod native_file_time: pub(crate)
    ├── mod native_string: pub
    ├── mod native_trig: pub
    ├── mod native_x87: pub
    ├── mod read_helpers: pub
    ├── mod retail_pointer_sort: pub(crate)
    ├── mod sha256: pub(crate)
    ├── mod single_instance: pub
    └── mod version: pub
```
