//! Saved random-map seeds: the `.SED` files the setup dialog's Load / Save /
//! Delete buttons browse.
//!
//! These are not numbered slots. A saved seed is an ordinary `.SED` file in the
//! game directory under whatever name the player typed, and the browser lists
//! every one it finds — minus a few reserved names that are engine scratch
//! files rather than player-visible saves.
//!
//! Depends on the options model and the filesystem only; no UI, no assets.

use std::path::{Path, PathBuf};

use super::options::RmgOptions;

/// The extension every saved seed carries. Matched case-insensitively — the
/// engine writes mixed case and players' files come from anywhere.
pub const SEED_EXTENSION: &str = "sed";

/// Names that live in the same directory with the same extension but are not
/// player-visible saves:
/// - the setup dialog's own working file, rewritten on every accept,
/// - the engine's last-played scratch copy,
/// - the network save, which is not a seed at all.
///
/// Listing any of these would offer the player a "save" that the engine
/// overwrites behind their back.
pub const RESERVED_SEED_NAMES: [&str; 3] = ["randmap.sed", "lastmap.sed", "savegame.net"];

/// Whether a file name is one of the reserved, non-browsable ones.
pub fn is_reserved_seed_name(file_name: &str) -> bool {
    let lowered = file_name.to_ascii_lowercase();
    RESERVED_SEED_NAMES.contains(&lowered.as_str())
}

/// Whether a file name is a browsable saved seed.
pub fn is_browsable_seed(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(SEED_EXTENSION))
        && !is_reserved_seed_name(file_name)
}

/// One entry in the saved-seed browser.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SavedSeed {
    /// File name as it sits on disk, extension included.
    pub file_name: String,
    /// The name shown in the list — the file name without its extension, which
    /// is what the player typed when saving.
    pub display_name: String,
}

impl SavedSeed {
    fn from_file_name(file_name: &str) -> Self {
        let display_name = Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(file_name)
            .to_string();
        Self {
            file_name: file_name.to_string(),
            display_name,
        }
    }
}

/// List the browsable saved seeds in a directory, sorted by display name.
///
/// A directory that cannot be read yields an empty list rather than an error:
/// the browser opening empty is the same thing the player sees when nothing has
/// been saved yet, and there is nothing useful to say about the difference.
pub fn list_saved_seeds(dir: &Path) -> Vec<SavedSeed> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut seeds: Vec<SavedSeed> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_browsable_seed(name))
        .map(|name| SavedSeed::from_file_name(&name))
        .collect();
    seeds.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
    });
    seeds
}

/// Whether any saved seed exists — what the dialog's Load and Delete buttons
/// are enabled from when it opens.
pub fn saved_seeds_available(dir: &Path) -> bool {
    !list_saved_seeds(dir).is_empty()
}

/// The path a typed name saves to, with the extension supplied if the player
/// left it off.
pub fn seed_path_for_name(dir: &Path, typed_name: &str) -> Option<PathBuf> {
    let trimmed = typed_name.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Reject anything that would escape the directory: the browser is a flat
    // list of one folder, and a typed path separator is not a save name.
    if trimmed.contains(['/', '\\', ':']) {
        return None;
    }
    let has_extension = Path::new(trimmed)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(SEED_EXTENSION));
    let file_name = if has_extension {
        trimmed.to_string()
    } else {
        format!("{trimmed}.{SEED_EXTENSION}")
    };
    if is_reserved_seed_name(&file_name) {
        return None;
    }
    Some(dir.join(file_name))
}

/// Read a saved seed's options.
///
/// Missing keys keep their defaults, matching how the engine leaves its record
/// alone for anything the file does not mention. The result is normalised, so a
/// hand-edited file cannot push the dialog outside its own ranges.
pub fn load_saved_seed(path: &Path) -> std::io::Result<RmgOptions> {
    let bytes = std::fs::read(path)?;
    let mut options = RmgOptions::default();
    if let Ok(ini) = crate::rules::ini_parser::IniFile::from_bytes(&bytes) {
        options.apply_sed(&ini);
    }
    options.normalize();
    Ok(options)
}

/// Write a saved seed.
pub fn save_saved_seed(path: &Path, options: &RmgOptions) -> std::io::Result<()> {
    std::fs::write(path, options.to_sed_bytes())
}

/// Delete a saved seed.
pub fn delete_saved_seed(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_names_are_matched_regardless_of_case() {
        for name in ["RandMap.Sed", "RANDMAP.SED", "randmap.sed", "LastMap.sed"] {
            assert!(is_reserved_seed_name(name), "{name} is reserved");
            assert!(!is_browsable_seed(name), "{name} is not browsable");
        }
    }

    #[test]
    fn only_sed_files_are_browsable() {
        assert!(is_browsable_seed("mymap.sed"));
        assert!(is_browsable_seed("MyMap.SED"));
        assert!(!is_browsable_seed("mymap.map"));
        assert!(!is_browsable_seed("mymap"));
        assert!(!is_browsable_seed("savegame.net"));
    }

    #[test]
    fn the_display_name_drops_the_extension() {
        let seed = SavedSeed::from_file_name("Desert Duel.sed");
        assert_eq!(seed.display_name, "Desert Duel");
        assert_eq!(seed.file_name, "Desert Duel.sed");
    }

    #[test]
    fn a_typed_name_gains_the_extension_but_keeps_one_it_has() {
        let dir = Path::new("C:/games");
        assert_eq!(
            seed_path_for_name(dir, "duel"),
            Some(dir.join("duel.sed")),
            "extension supplied"
        );
        assert_eq!(
            seed_path_for_name(dir, "duel.SED"),
            Some(dir.join("duel.SED")),
            "existing extension kept as typed"
        );
    }

    #[test]
    fn a_typed_name_cannot_escape_the_directory_or_take_a_reserved_name() {
        let dir = Path::new("C:/games");
        assert_eq!(seed_path_for_name(dir, ""), None);
        assert_eq!(seed_path_for_name(dir, "   "), None);
        assert_eq!(seed_path_for_name(dir, "../evil"), None);
        assert_eq!(seed_path_for_name(dir, "sub/duel"), None);
        assert_eq!(seed_path_for_name(dir, "C:evil"), None);
        assert_eq!(
            seed_path_for_name(dir, "RandMap"),
            None,
            "cannot overwrite the dialog's own working file"
        );
    }

    #[test]
    fn listing_a_missing_directory_is_empty_rather_than_an_error() {
        let seeds = list_saved_seeds(Path::new("C:/definitely/not/here"));
        assert!(seeds.is_empty());
        assert!(!saved_seeds_available(Path::new("C:/definitely/not/here")));
    }

    #[test]
    fn a_saved_seed_round_trips_through_disk() {
        let dir = std::env::temp_dir().join("vera20k_saved_seed_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let options = RmgOptions {
            seed: 4242,
            num_players: 6,
            map_type: 3,
            description: "Desert Duel".to_string(),
            ..Default::default()
        };
        let path = seed_path_for_name(&dir, "Desert Duel").expect("path");
        save_saved_seed(&path, &options).expect("save");

        let listed = list_saved_seeds(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].display_name, "Desert Duel");

        let loaded = load_saved_seed(&path).expect("load");
        assert_eq!(loaded.seed, 4242);
        assert_eq!(loaded.num_players, 6);
        assert_eq!(loaded.map_type, 3);
        assert_eq!(loaded.description, "Desert Duel");

        delete_saved_seed(&path).expect("delete");
        assert!(
            list_saved_seeds(&dir).is_empty(),
            "delete removes the entry"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_working_file_never_appears_in_the_list() {
        let dir = std::env::temp_dir().join("vera20k_saved_seed_reserved");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("RandMap.Sed"), b"[RandomMap]\n").expect("write");
        std::fs::write(dir.join("lastmap.sed"), b"[RandomMap]\n").expect("write");
        std::fs::write(dir.join("Keeper.sed"), b"[RandomMap]\n").expect("write");

        let listed = list_saved_seeds(&dir);
        assert_eq!(listed.len(), 1, "only the real save is listed: {listed:?}");
        assert_eq!(listed[0].display_name, "Keeper");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
