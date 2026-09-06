//! Successful same-content world replacement and its presentation lifecycle.

use crate::app::AppState;

/// Apply the enumerated post-prepare replacement bundle. This function has no
/// recoverable failure path; best-effort presentation rebuilds retain their
/// prior valid resources when replacement is unavailable.
pub(crate) fn commit_prepared_load(
    state: &mut AppState,
    path: &std::path::Path,
    prepared: crate::app::persistence::PreparedLoad,
) {
    let native_tiberium_stats = prepared.native_tiberium_stats();
    log::info!(
        "Load: rebuilt native tiberium queues ({} growth, {} spread)",
        native_tiberium_stats.growth_entries,
        native_tiberium_stats.spread_entries,
    );

    crate::app::reset_scenario_exit_runtime(state);
    // F10 lifecycle: a successful in-scenario load closes the outgoing
    // timeline's diagnostic segment BEFORE the restored simulation commits;
    // the restored timeline lazily opens its own segment on the next frame.
    // A failed close discards rather than retains (no mixed-header artifact).
    crate::app::match_runtime::sim_tick::close_replay_segment_for_new_timeline(state);
    let runtime = state
        .match_state
        .sim_runtime
        .as_mut()
        .expect("prepared load requires a live runtime");
    // The close above reads the outgoing tick and its pre-recorded replay only;
    // it never observes the shared CellClass dummy reconstructed by this commit.
    let occupied_overlays = prepared.commit_into(runtime);
    crate::app::loading::transitions::sync_in_game_options_speed_from_sim(state);
    state.match_state.match_presentation.combat_lights.clear();
    // The restored world's strips are seeded silently on the first refresh
    // below (`SidebarClass::AddCameo` init gate), not read as insertions
    // against the outgoing timeline's cameos.
    state
        .match_state
        .match_presentation
        .sidebar_projection
        .reset_cameo_seed();
    crate::app::match_runtime::sim_tick::upsert_occupied_overlay_render_entries(
        state,
        occupied_overlays,
    );

    // F10: the fog view cache was discarded with the load (nonserialized) —
    // rebuild it for the local owner BEFORE the first tactical render, and
    // invalidate the render dirty-gates: the view generation restarts from
    // zero, so an equal counter no longer proves an unchanged view.
    if let Some(owner) = crate::app::input::commands::preferred_local_owner_name(state) {
        if let Some(sim) = state
            .match_state
            .sim_runtime
            .as_mut()
            .map(|rt| &mut rt.simulation)
        {
            sim.prepare_fog_view_for(&owner);
        }
    }
    if let Some(shroud) = state.match_state.match_presentation.shroud_buffer.as_mut() {
        shroud.mark_stale();
    }
    if let Some(minimap) = state.match_state.match_presentation.minimap.as_mut() {
        minimap.mark_stale();
    }
    state
        .match_state
        .match_presentation
        .installed_playfield_authority = None;

    // Rebuild sprite/unit atlases so all entity types in the loaded save have
    // atlas entries before the first render frame.
    crate::app::match_runtime::sim_tick::refresh_entity_atlases(state);

    // Rebuild transient lighting from the loaded live entity set so destroyed
    // light-source buildings do not leave stale point lights behind.
    if let Some(resolved_terrain) = state.terrain_template() {
        state.match_state.match_presentation.lighting_grid =
            crate::app::loading::init::rebuild_lighting_grid_from_sim(
                resolved_terrain,
                &state.match_state.match_presentation.map_lighting_config,
                state
                    .match_state
                    .sim_runtime
                    .as_ref()
                    .map(|rt| &rt.simulation),
                state.rules(),
                state
                    .match_state
                    .match_presentation
                    .in_game_options
                    .detail_level,
            );
        state
            .match_state
            .match_presentation
            .pending_lighting_refresh = None;
        state
            .match_state
            .match_presentation
            .applied_lighting_sources
            .clear();
        state
            .match_state
            .match_presentation
            .applied_lighting_profile = None;
        state
            .match_state
            .match_presentation
            .applied_lighting_detail_level = state
            .match_state
            .match_presentation
            .in_game_options
            .detail_level
            .min(2);
        state
            .match_state
            .match_presentation
            .last_lighting_view_fingerprint = None;
    }

    // Reset timing to prevent a burst of ticks after the load.
    state.platform.frame_pacer.reset_for_immediate_frame();

    // Close the save/load panel after loading.
    state.match_state.match_presentation.show_save_load_panel = false;

    // Same-content restoration leaves match-owned startup admission untouched.
    state.persistence.last_loaded_save_path = Some(path.to_path_buf());
    crate::app::presentation::sidebar_render::refresh_sidebar_projection(state);
    log::info!("Load: restored simulation from {}", path.display());
}
