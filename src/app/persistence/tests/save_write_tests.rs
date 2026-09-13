//! VERA repository write transactions; these are filesystem regressions, not
//! comparisons with the native SAV format.

use super::*;

fn fixture(label: &str) -> PersistenceState {
    let directory = std::env::temp_dir().join(format!(
        "vera20k-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let mut state = PersistenceState::new(options_profile::RetailOptionsProfile::default());
    state.repository = SaveRepository::at(directory);
    state
}

fn snapshot(description: &str, timestamp: u64) -> Vec<u8> {
    let mut simulation = Simulation::new();
    simulation.session.map_name = "OVERWRITE.MAP".into();
    GameSnapshot::save_validated(&simulation, 1, 2, description, timestamp)
}

#[test]
fn explicit_overwrite_replaces_selected_save_and_refreshes_exact_description() {
    let mut state = fixture("explicit-overwrite");
    let original = snapshot("original", 1);
    let path = state.write_save("selected.bin", &original, 7).unwrap();
    let other = state.write_save("other.bin", &original, 8).unwrap();
    state.refresh_save_list_if_dirty();
    let exact_description = "  保存 / ridge  ";
    let replacement = snapshot(exact_description, 2);

    assert_eq!(state.overwrite_save(&path, &replacement, 19).unwrap(), path);
    assert_eq!(state.repository.read(&path).unwrap(), replacement);
    assert_eq!(state.repository.read(&other).unwrap(), original);
    assert_eq!(state.last_save_tick(), Some(19));
    assert!(state.save_list_cache.dirty);
    state.refresh_save_list_if_dirty();
    let entries = state.save_list_cache.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, path);
    assert_eq!(entries[0].header.description, exact_description);
    assert_eq!(
        std::fs::read_dir(state.repository.directory())
            .unwrap()
            .count(),
        2
    );
    std::fs::remove_dir_all(state.repository.directory()).unwrap();
}

#[test]
fn creation_collision_and_missing_overwrite_leave_previous_save_and_bookkeeping() {
    let mut state = fixture("create-collision");
    let original = snapshot("original", 1);
    let path = state
        .write_save("same-timestamp.bin", &original, 7)
        .unwrap();
    state.refresh_save_list_if_dirty();
    let saved_at = state.last_save_instant();
    let replacement = snapshot("replacement", 2);

    let collision = state
        .write_save("same-timestamp.bin", &replacement, 19)
        .unwrap_err();
    assert_eq!(collision.source.kind(), std::io::ErrorKind::AlreadyExists);
    let missing = state.repository.directory().join("removed-row.bin");
    assert!(state.overwrite_save(&missing, &replacement, 20).is_err());
    assert!(!missing.exists());
    assert_eq!(state.repository.read(&path).unwrap(), original);
    assert_eq!(state.last_save_tick(), Some(7));
    assert_eq!(state.last_save_instant(), saved_at);
    assert!(!state.save_list_cache.dirty);
    assert_eq!(
        state.save_list_cache.entries()[0].header.description,
        "original"
    );
    std::fs::remove_dir_all(state.repository.directory()).unwrap();
}

#[cfg(windows)]
#[test]
fn failed_overwrite_publish_preserves_original_and_removes_staged_file() {
    use std::os::windows::fs::OpenOptionsExt;

    let mut state = fixture("overwrite-publish-failure");
    let original = snapshot("original", 1);
    let path = state.write_save("selected.bin", &original, 7).unwrap();
    state.refresh_save_list_if_dirty();
    let saved_at = state.last_save_instant();
    // Permit the repository's existing-header read but deny replacement. This
    // fails the real final rename after the full staged snapshot is written.
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    assert!(
        state
            .overwrite_save(&path, &snapshot("replacement", 2), 19)
            .is_err()
    );
    assert_eq!(state.repository.read(&path).unwrap(), original);
    assert_eq!(state.last_save_tick(), Some(7));
    assert_eq!(state.last_save_instant(), saved_at);
    assert!(!state.save_list_cache.dirty);
    assert_eq!(
        std::fs::read_dir(state.repository.directory())
            .unwrap()
            .count(),
        1
    );
    drop(held);
    std::fs::remove_dir_all(state.repository.directory()).unwrap();
}
