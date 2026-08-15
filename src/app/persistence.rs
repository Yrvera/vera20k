//! Save-game repository and process-lifetime persistence state.
//!
//! The repository owns every save-directory filesystem operation. The two
//! existing "latest" policies are intentionally distinct: panel rows sort by
//! the embedded snapshot timestamp, while quickload selects by filesystem
//! modification time. Unifying them would be VERA-internal / gamemd equivalent
//! UNCHECKED.

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use crate::sim::snapshot::{GameSnapshot, GameSnapshotHeader};

const DEFAULT_SAVES_DIRECTORY: &str = "saves";

/// Process-lifetime save-game state owned by the app persistence domain.
pub(crate) struct PersistenceState {
    pub(crate) repository: SaveRepository,
    pub(crate) save_list_cache: SaveListCache,
    pub(crate) last_save_tick: Option<u64>,
    pub(crate) last_save_instant: Option<Instant>,
    pub(crate) last_loaded_save_path: Option<PathBuf>,
}

impl PersistenceState {
    pub(crate) fn new() -> Self {
        Self {
            repository: SaveRepository::new(),
            save_list_cache: SaveListCache::new(),
            last_save_tick: None,
            last_save_instant: None,
            last_loaded_save_path: None,
        }
    }

    pub(crate) fn refresh_save_list_if_dirty(&mut self) {
        self.save_list_cache.refresh_if_dirty(&self.repository);
    }

    pub(crate) fn invalidate_save_list(&mut self) {
        self.save_list_cache.invalidate();
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
    entries.sort_by(|left, right| {
        right
            .header
            .save_timestamp
            .cmp(&left.header.save_timestamp)
    });
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
    use crate::sim::world::Simulation;

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
                std::fs::FileTimes::new().set_modified(
                    base_modified + std::time::Duration::from_secs(30),
                ),
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
        assert_eq!(repository.read(&path).expect("read repository fixture"), bytes);

        let entries = repository.panel_entries_by_embedded_time();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, path);
        assert_eq!(entries[0].header.description, "Northern ridge");

        repository.delete(&path).expect("delete repository fixture");
        assert!(!repository.exists(&path));
        std::fs::remove_dir_all(directory).expect("remove repository fixture directory");
    }
}
