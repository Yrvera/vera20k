//! User save/load requests; input and dialogs only select the operation.

use crate::app::AppState;

use std::path::{Path, PathBuf};

/// A save request can fail before disk I/O without changing the running match.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveCommandError {
    #[error("Enter a name for the saved game.")]
    EmptyDescription,
    #[error("There is no active game to save.")]
    MissingSimulation,
    #[error("The active game has no authoritative source-map digest.")]
    MissingMapHash,
    #[error("Could not save the game: {0}")]
    Write(#[from] crate::app::persistence::SaveWriteError),
}

pub(crate) fn quicksave(state: &mut AppState) {
    log_save_result("Quicksave", save(state, None, None));
}

/// Save As retains the exact user description in the envelope; only the
/// filename is sanitized. Quick saves use the map name as their description.
pub(crate) fn save_with_name(state: &mut AppState, raw_name: &str) {
    log_save_result("Save As", save_game(state, raw_name, None));
}

/// UI save/create/explicit overwrite request. A new timestamp-name collision
/// is an error, never implicit permission to overwrite an existing save.
/// This is the VERA snapshot format; it does not claim native SAV compatibility.
pub(crate) fn save_game(
    state: &mut AppState,
    description: &str,
    overwrite: Option<&Path>,
) -> Result<PathBuf, SaveCommandError> {
    save(state, Some(description), overwrite)
}

fn save(
    state: &mut AppState,
    name: Option<&str>,
    overwrite: Option<&Path>,
) -> Result<PathBuf, SaveCommandError> {
    let sanitized = name.map(sanitize_save_name);
    if sanitized.as_deref() == Some("") {
        return Err(SaveCommandError::EmptyDescription);
    }
    let runtime = state
        .match_state
        .sim_runtime
        .as_ref()
        .ok_or(SaveCommandError::MissingSimulation)?;
    let map_hash = state
        .match_state
        .loaded_map_hash
        .ok_or(SaveCommandError::MissingMapHash)?;
    let sim = &runtime.simulation;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bytes = crate::sim::snapshot::GameSnapshot::save_validated(
        sim,
        map_hash,
        runtime.resources.rules.simulation_config_hash(),
        name.unwrap_or(&sim.session.map_name),
        now,
    );
    let tick = sim.session.tick;
    if let Some(path) = overwrite {
        return Ok(state.persistence.overwrite_save(path, &bytes, tick)?);
    }
    let filename = match sanitized {
        Some(name) => format!("save_{name}_tick{tick}_{now}.bin"),
        None => format!("save_tick{tick}_{now}.bin"),
    };
    Ok(state.persistence.write_save(&filename, &bytes, tick)?)
}

fn log_save_result(label: &str, result: Result<PathBuf, SaveCommandError>) {
    match result {
        Ok(path) => log::info!("{label}: saved game to {}", path.display()),
        Err(SaveCommandError::Write(error)) => match error.stage() {
            crate::app::persistence::SaveWriteStage::CreateDirectory => {
                log::error!("{label}: failed to create saves dir: {error}")
            }
            crate::app::persistence::SaveWriteStage::WriteFile => {
                log::error!("{label}: write failed: {error}")
            }
        },
        Err(error) => log::warn!("{label}: {error}"),
    }
}

/// Sanitize a user-typed save name for use in a filename.
///
/// Replaces Windows-reserved characters (`/ \ : * ? " < > |`) with `_`,
/// trims surrounding whitespace, then caps at 64 chars. Returns an empty
/// string for empty/whitespace-only input.
fn sanitize_save_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars().take(64) {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod save_name_tests {
    use super::sanitize_save_name;
    use crate::sim::snapshot::GameSnapshot;
    use crate::sim::world::Simulation;

    #[test]
    fn empty_returns_empty() {
        assert_eq!(sanitize_save_name(""), "");
        assert_eq!(sanitize_save_name("   "), "");
        assert_eq!(sanitize_save_name("\t\n"), "");
    }

    #[test]
    fn strips_path_separators() {
        assert_eq!(sanitize_save_name("../foo"), ".._foo");
        assert_eq!(sanitize_save_name("a/b\\c"), "a_b_c");
    }

    #[test]
    fn strips_windows_reserved_chars() {
        assert_eq!(sanitize_save_name("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
    }

    #[test]
    fn keeps_normal_chars() {
        assert_eq!(sanitize_save_name("miner stuck repro"), "miner stuck repro");
        assert_eq!(sanitize_save_name("dock_fix_a"), "dock_fix_a");
    }

    #[test]
    fn caps_at_64_chars() {
        let long: String = "x".repeat(100);
        let out = sanitize_save_name(&long);
        assert_eq!(out.len(), 64);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(sanitize_save_name("  hello  "), "hello");
    }

    #[test]
    fn gsi_17_02_unicode_filename_cap_preserves_exact_envelope_description() {
        let raw_description = "保存".repeat(40);
        let filename_part = sanitize_save_name(&raw_description);
        assert_eq!(filename_part.chars().count(), 64);

        let mut sim = Simulation::new();
        sim.session.map_name = "OFFICIAL.MAP".to_string();
        let bytes = GameSnapshot::save_validated(&sim, 1, 2, &raw_description, 3);
        let header = GameSnapshot::read_header(&bytes).expect("current VERA header");
        assert_eq!(header.description, raw_description);
    }
}

pub(crate) fn quickload(state: &mut AppState) {
    let path = match state
        .persistence
        .repository
        .quickload_path_by_modified_time()
    {
        Some(p) => p,
        None => {
            log::warn!(
                "Quickload: no save files found in {}/",
                state.persistence.repository.directory().display()
            );
            return;
        }
    };
    load_save_file(state, &path);
}

/// Load a save file by path. Used by both quickload and the save/load panel.
pub(crate) fn load_save_file(state: &mut AppState, path: &std::path::Path) {
    if let Err(error) = try_load_save_file(state, path) {
        log_prepared_load_error(path, &error);
    }
}

/// Same-content load with a result for the owning shell. Every fallible read,
/// schema/hash check and restoration step precedes the existing commit bundle.
pub(crate) fn try_load_save_file(
    state: &mut AppState,
    path: &Path,
) -> Result<(), crate::app::persistence::PreparedLoadError> {
    let prepared = crate::app::persistence::PreparedLoad::from_repository(
        crate::app::persistence::LoadPreparationView::from_runtime(
            &state.persistence.repository,
            state.match_state.sim_runtime.as_ref(),
            state.match_state.loaded_map_hash,
        ),
        path,
    )?;
    crate::app::match_runtime::restore::commit_prepared_load(state, path, prepared);
    Ok(())
}

fn log_prepared_load_error(
    path: &std::path::Path,
    error: &crate::app::persistence::PreparedLoadError,
) {
    use crate::app::persistence::PreparedLoadError;

    match error {
        PreparedLoadError::ReadFile(source) => {
            log::warn!("Load: could not read {}: {source}", path.display())
        }
        PreparedLoadError::MissingCurrentSimulation
        | PreparedLoadError::MissingMapHash
        | PreparedLoadError::MissingRules => log::warn!("Load: {error}"),
        PreparedLoadError::Snapshot(source) => log::error!("Load: {source}"),
        PreparedLoadError::MissingTerrainTemplate => {
            log::error!("Load: {error}")
        }
        PreparedLoadError::MissingOverlayRegistry => {
            log::error!("Load: restoration validation failed: {error}")
        }
        PreparedLoadError::Restore(source) => {
            log::error!("Load: restoration validation failed: {source}")
        }
        PreparedLoadError::FactoryState(source) => {
            log::error!("Load: restoration validation failed: {source}")
        }
    }
}
