//! High-level asset loading API for the RA2 engine.
//!
//! The AssetManager registers top-level and explicitly named nested MIX
//! archives in the same construction order as active retail `gamemd.exe`.
//! Registration appends to the search list and the first matching archive
//! wins. Other known nested archives are catalogued for direct access and
//! on-demand registration, but do not silently enter the global search path.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::assets::error::AssetError;
use crate::assets::mix_archive::MixArchive;
use crate::assets::mix_hash::{mix_hash, westwood_hash};

/// A MIX archive with a human-readable name for logging and diagnostics.
struct NamedArchive {
    /// Name for logs and tools, for example `ra2.mix` or `ra2.mix -> local.mix`.
    name: String,
    /// Parsed archive data.
    archive: MixArchive,
}

enum LooseBytes {
    Empty,
    Mapped(memmap2::Mmap),
    #[cfg(test)]
    Owned(Box<[u8]>),
}

impl LooseBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Mapped(data) => data,
            #[cfg(test)]
            Self::Owned(data) => data,
        }
    }
}

struct LooseAsset {
    path: PathBuf,
    source_name: String,
    bytes: LooseBytes,
}

#[derive(Clone, Copy)]
struct AssetLocation {
    archive_index: usize,
    entry_id: i32,
}

#[derive(Default)]
struct WildcardMount {
    matched: bool,
    first_registered: bool,
}

/// Borrowed first-match resolution from the production archive stack.
///
/// This is observational only: callers receive the same bytes that `get_ref`
/// would return plus the archive-chain identity and hashed entry ID that
/// selected them. It must not be used to bypass normal lookup precedence.
#[derive(Clone, Copy, Debug)]
pub struct AssetResolutionRef<'a> {
    pub bytes: &'a [u8],
    pub source_archive: &'a str,
    pub entry_id: i32,
}

/// The media-pack branch selected by active `Init_Mix_Files @ 0x00530460`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaArchiveMode {
    /// Normal startup uses the current media index plus one in `%02d` names.
    Numbered { media_index: i32 },
    /// Retail's `-CD` switch selects the wildcard-enumeration branch.
    CdWildcard,
}

impl MediaArchiveMode {
    fn from_arguments<I, S>(arguments: I, media_index: i32) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        // Native uppercases every argument after argv[0], then uses `strstr`
        // rather than whole-token equality for the literal `-CD`.
        if arguments.into_iter().any(|argument| {
            argument
                .as_ref()
                .to_string_lossy()
                .to_ascii_uppercase()
                .contains("-CD")
        }) {
            Self::CdWildcard
        } else {
            Self::Numbered { media_index }
        }
    }
}

impl Default for MediaArchiveMode {
    fn default() -> Self {
        // The active retail YR executable reports media index 2 for the stock
        // digital-install path, selecting the `03` archive family.
        Self::from_arguments(std::env::args_os().skip(1), 2)
    }
}

/// An owned result from retail's cache-first `LoadFileFromMIX` path.
///
/// The payload is owned by the cache rather than an archive borrow, so the
/// first winner remains valid after theater archives are destroyed/remounted.
#[derive(Clone, Debug)]
pub struct MixFileLoad {
    pub bytes: Arc<[u8]>,
    pub source_archive: Arc<str>,
    pub entry_id: i32,
}

/// Manages loaded MIX archives and provides name-based lookups.
///
/// Archives are searched in priority order. Earlier archives win.
pub struct AssetManager {
    /// Loaded MIX archives in search priority order.
    archives: Vec<NamedArchive>,
    /// Parsed, named nested MIX archives that retail has not registered yet.
    archive_catalog: Vec<NamedArchive>,
    /// Precomputed first-match lookup across all archives.
    lookup_index: HashMap<i32, AssetLocation>,
    /// `LoadFileFromMIX`'s process-lifetime, normalized-CRC first-winner cache.
    mix_file_cache: Mutex<HashMap<i32, MixFileLoad>>,
    /// Case-insensitive, non-recursive view of the retail executable directory.
    loose_files: HashMap<String, LooseAsset>,
    /// Currently registered theater identity and its archive names.
    active_theater: Option<String>,
    active_theater_archives: Vec<String>,
    /// Path to the RA2 installation directory.
    ra2_dir: PathBuf,
}

/// Named nested archives shipped by the active retail RA2/YR install.
///
/// Retail constructs archives by name. It does not scan arbitrary payloads
/// looking for plausible MIX headers; doing so misclassifies SHP and CSF data.
const KNOWN_NESTED_MIX_NAMES: &[&str] = &[
    "audio.mix",
    "audiomd.mix",
    "cache.mix",
    "cachemd.mix",
    "cameo.mix",
    "cameomd.mix",
    "cameos.mix",
    "conquer.mix",
    "conqmd.mix",
    "des.mix",
    "desert.mix",
    "generic.mix",
    "genermd.mix",
    "isodes.mix",
    "isodesmd.mix",
    "isogen.mix",
    "isogenmd.mix",
    "isolun.mix",
    "isolunmd.mix",
    "isosnomd.mix",
    "isosnow.mix",
    "isotemp.mix",
    "isotem.mix",
    "isotemmd.mix",
    "isourb.mix",
    "isourbmd.mix",
    "isoubn.mix",
    "isoubnmd.mix",
    "load.mix",
    "loadmd.mix",
    "local.mix",
    "localmd.mix",
    "lun.mix",
    "lunar.mix",
    "neutral.mix",
    "ntrlmd.mix",
    "sidec01.mix",
    "sidec01md.mix",
    "sidec02.mix",
    "sidec02md.mix",
    "sidenc01.mix",
    "sidenc02.mix",
    "sno.mix",
    "snow.mix",
    "snowmd.mix",
    "tem.mix",
    "temperat.mix",
    "ubn.mix",
    "urb.mix",
    "urban.mix",
    "urbann.mix",
];

impl AssetManager {
    /// Load the core runtime archive stack.
    pub fn new(ra2_dir: &Path) -> Result<Self, AssetError> {
        Self::new_with_media_mode(ra2_dir, MediaArchiveMode::default())
    }

    #[cfg(test)]
    pub(crate) fn from_loose_root_for_test(ra2_dir: &Path) -> Self {
        Self {
            archives: Vec::new(),
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: Self::open_loose_root(ra2_dir).expect("open loose test root"),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: ra2_dir.to_path_buf(),
        }
    }

    /// Load the core runtime archive stack for one native media-selection mode.
    pub fn new_with_media_mode(
        ra2_dir: &Path,
        media_mode: MediaArchiveMode,
    ) -> Result<Self, AssetError> {
        let mut manager = Self {
            archives: Vec::new(),
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: Self::open_loose_root(ra2_dir)?,
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: ra2_dir.to_path_buf(),
        };

        // WinMain registers the YR language archive before the RA2 fallback.
        manager.mount_named_archive("langmd.mix", false)?;
        manager.mount_named_archive("language.mix", false)?;

        // Init_Base_Mix_Files probes all expansion slots in descending order.
        for index in (0..=99).rev() {
            manager.mount_named_archive(&format!("expandmd{index:02}.mix"), false)?;
        }

        // Active Yuri's Revenge requires both masters, in this order.
        manager.mount_named_archive("ra2md.mix", true)?;
        manager.mount_named_archive("ra2.mix", true)?;

        for name in ["cachemd.mix", "cache.mix", "localmd.mix", "local.mix"] {
            manager.mount_named_archive(name, true)?;
        }

        // AudioSystem__Init uses exactly one archive: the MD archive when
        // present, otherwise the base archive.
        if !manager.mount_named_archive("audiomd.mix", false)? {
            manager.mount_named_archive("audio.mix", false)?;
        }

        // Init_Mix_Files constructs only these named core archives. Optional
        // constructors are allowed to fail exactly where retail does not gate
        // startup on their result.
        manager.mount_named_archive("conqmd.mix", true)?;
        manager.mount_named_archive("genermd.mix", false)?;
        manager.mount_named_archive("generic.mix", false)?;
        manager.mount_named_archive("isogenmd.mix", false)?;
        manager.mount_named_archive("isogen.mix", false)?;
        manager.mount_named_archive("conquer.mix", true)?;
        manager.mount_named_archive("cameomd.mix", true)?;
        manager.mount_named_archive("cameo.mix", true)?;

        manager.mount_native_map_media(media_mode)?;
        manager.mount_named_archive("multimd.mix", true)?;
        if !manager.mount_named_archive("thememd.mix", false)? {
            manager.mount_named_archive("theme.mix", false)?;
        }
        manager.mount_native_movie_media(media_mode)?;

        // Loose YRO archives are deliberately absent here. Retail registers
        // each one during SessionClass scenario enumeration, immediately
        // before opening its embedded PKT, and retains that registered object.
        manager.catalog_named_nested_archives();
        log::info!(
            "Retail archive search order ({} registered archives, first match wins):",
            manager.archives.len()
        );
        for (i, named) in manager.archives.iter().enumerate() {
            log::info!(
                "  [{:2}] {} ({} entries)",
                i,
                named.name,
                named.archive.entry_count()
            );
        }

        Ok(manager)
    }

    /// Look up a file by name across all loaded archives.
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        if let Some(loose) = self.loose_asset(name) {
            log::trace!("Found loose '{}' at {}", name, loose.path.display());
            return Some(loose.bytes.as_slice().to_vec());
        }
        let (named, entry_id) = self.lookup_asset_entry(name)?;
        log::trace!("Found '{}' in {}", name, named.name);
        named.archive.get_by_id(entry_id).map(|data| data.to_vec())
    }

    /// Look up a file by name without copying the asset bytes.
    pub fn get_ref(&self, name: &str) -> Option<&[u8]> {
        if let Some(loose) = self.loose_asset(name) {
            log::trace!("Found loose '{}' at {}", name, loose.path.display());
            return Some(loose.bytes.as_slice());
        }
        let (named, entry_id) = self.lookup_asset_entry(name)?;
        log::trace!("Found '{}' in {}", name, named.name);
        named.archive.get_by_id(entry_id)
    }

    /// Look up a file by name and return both the bytes and source archive name.
    pub fn get_with_source(&self, name: &str) -> Option<(Vec<u8>, String)> {
        if let Some(loose) = self.loose_asset(name) {
            return Some((loose.bytes.as_slice().to_vec(), loose.source_name.clone()));
        }
        let (named, entry_id) = self.lookup_asset_entry(name)?;
        named
            .archive
            .get_by_id(entry_id)
            .map(|data| (data.to_vec(), named.name.clone()))
    }

    /// Look up a file by name and return both the borrowed bytes and source archive name.
    pub fn get_with_source_ref(&self, name: &str) -> Option<(&[u8], &str)> {
        let resolved = self.resolve_ref(name)?;
        Some((resolved.bytes, resolved.source_archive))
    }

    /// Resolve one file through the normal first-match archive lookup.
    pub fn resolve_ref(&self, name: &str) -> Option<AssetResolutionRef<'_>> {
        if let Some(loose) = self.loose_asset(name) {
            return Some(AssetResolutionRef {
                bytes: loose.bytes.as_slice(),
                source_archive: loose.source_name.as_str(),
                entry_id: mix_hash(name),
            });
        }
        let (named, entry_id) = self.lookup_asset_entry(name)?;
        named
            .archive
            .get_by_id(entry_id)
            .map(|bytes| AssetResolutionRef {
                bytes,
                source_archive: named.name.as_str(),
                entry_id,
            })
    }

    /// Load through retail's `LoadFileFromMIX @ 0x005B40B0` boundary.
    ///
    /// This intentionally differs from [`Self::resolve_ref`]:
    ///
    /// - the uppercase-normalized filename CRC is checked first;
    /// - a cache miss searches registered MIXes before the raw filesystem;
    /// - the first payload for a CRC remains cached even after archive remounts.
    pub fn load_file_from_mix(&self, name: &str) -> Option<MixFileLoad> {
        let cache_key = mix_hash(name);
        if let Some(cached) = self
            .mix_file_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&cache_key)
            .cloned()
        {
            return Some(cached);
        }

        let candidate = if let Some((named, entry_id)) = self.lookup_asset_entry(name) {
            MixFileLoad {
                bytes: Arc::from(named.archive.get_by_id(entry_id)?),
                source_archive: Arc::from(named.name.as_str()),
                entry_id,
            }
        } else {
            let loose = self.loose_asset(name)?;
            MixFileLoad {
                bytes: Arc::from(loose.bytes.as_slice()),
                source_archive: Arc::from(loose.source_name.as_str()),
                entry_id: cache_key,
            }
        };

        // A concurrent first load wins exactly once for this CRC. Returning
        // the occupied value preserves the native cache's CRC-only identity.
        let mut cache = self
            .mix_file_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Some(cache.entry(cache_key).or_insert(candidate).clone())
    }

    /// Register the loading-screen archive pair at the scenario-load boundary.
    ///
    /// Retail constructs the MD archive first and returns immediately when
    /// either archive is unavailable.
    pub(crate) fn register_loading_archives(&mut self) -> Result<bool, AssetError> {
        if !self.mount_named_archive("loadmd.mix", false)? {
            return Ok(false);
        }
        if !self.mount_named_archive("load.mix", false)? {
            return Ok(false);
        }
        Ok(true)
    }

    /// Register the neutral shell archive pair at the shell-init boundary.
    ///
    /// `Shell_LoadNeutralMixArchives @ 0x00534E50` constructs `NTRLMD.MIX`
    /// first and returns immediately when it is unavailable, then constructs
    /// `NEUTRAL.MIX`. Their registration order makes MD duplicates win retail's
    /// first-match archive scan.
    pub(crate) fn register_neutral_archives(&mut self) -> Result<bool, AssetError> {
        if !self.mount_named_archive("ntrlmd.mix", false)? {
            return Ok(false);
        }
        if !self.mount_named_archive("neutral.mix", false)? {
            return Ok(false);
        }
        Ok(true)
    }

    /// Register an additional named archive at the tail of the search list.
    ///
    /// This matches `MixFileClass__Constructor_Registered @ 0x005B3C20`.
    /// Existing archives retain priority and an already registered name is a
    /// no-op.
    pub fn load_nested(&mut self, name: &str) -> Result<(), AssetError> {
        self.mount_named_archive(name, true).map(|_| ())
    }

    /// Replace retail's one active theater archive group.
    ///
    /// `Init_Theater @ 0x005349C0` destroys the previous group, appends the
    /// new group's named archives, and does nothing when the theater identity
    /// is unchanged.
    pub fn activate_theater_archives(
        &mut self,
        theater_name: &str,
        archive_names: &[&str],
    ) -> Result<(), AssetError> {
        if self
            .active_theater
            .as_deref()
            .is_some_and(|active| active.eq_ignore_ascii_case(theater_name))
        {
            return Ok(());
        }

        if !self.active_theater_archives.is_empty() {
            let old_names = std::mem::take(&mut self.active_theater_archives);
            let mut retained = Vec::with_capacity(self.archives.len());
            for archive in self.archives.drain(..) {
                if old_names
                    .iter()
                    .any(|name| archive_name_matches(&archive.name, name))
                {
                    self.archive_catalog.push(archive);
                } else {
                    retained.push(archive);
                }
            }
            self.archives = retained;
            self.rebuild_indexes();
        }

        for &name in archive_names {
            if self.mount_named_archive(name, false)? {
                self.active_theater_archives.push(name.to_string());
            }
        }
        self.active_theater = Some(theater_name.to_string());
        Ok(())
    }

    /// Load every additional top-level `.mix` file present in the RA2 directory.
    ///
    /// This is meant for tooling. Runtime code does not call it.
    pub fn load_all_disk_mixes(&mut self) -> Result<usize, AssetError> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&self.ra2_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(OsStr::to_str)
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mix"))
            })
            .collect();
        paths.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });

        let mut loaded_count = 0usize;
        for path in paths {
            let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            if self.archive_is_known(file_name) {
                continue;
            }

            let archive = match MixArchive::load(&path) {
                Ok(archive) => archive,
                Err(err) => {
                    log::debug!("Skipping {}: {}", path.display(), err);
                    continue;
                }
            };
            log::info!(
                "Loaded extra disk archive {} ({} entries)",
                file_name,
                archive.entry_count()
            );
            self.append_registered_archive(NamedArchive {
                name: file_name.to_string(),
                archive,
            });
            loaded_count += 1;
        }
        if loaded_count > 0 {
            self.catalog_named_nested_archives();
        }

        Ok(loaded_count)
    }

    /// Check if a file exists in any loaded archive.
    pub fn contains(&self, name: &str) -> bool {
        // Native availability checks MIX before raw. Only the boolean escapes.
        self.lookup_location_for_name(name).is_some() || self.loose_asset(name).is_some()
    }

    /// Look up a loaded archive by its display/debug name.
    pub fn archive(&self, name: &str) -> Option<&MixArchive> {
        self.archives
            .iter()
            .chain(self.archive_catalog.iter())
            .find(|archive| archive_name_matches(&archive.name, name))
            .map(|archive| &archive.archive)
    }

    /// Read one entry from a specific archive by entry hash.
    pub fn archive_entry_data(&self, archive_name: &str, entry_id: i32) -> Option<Vec<u8>> {
        let archive = self.archive(archive_name)?;
        archive.get_by_id(entry_id).map(|data| data.to_vec())
    }

    /// Visit all loaded archives in search order.
    pub fn visit_archives<F>(&self, mut visitor: F)
    where
        F: FnMut(&str, &MixArchive),
    {
        for archive in &self.archives {
            visitor(&archive.name, &archive.archive);
        }
        for archive in &self.archive_catalog {
            visitor(&archive.name, &archive.archive);
        }
    }

    /// Return the names of all loaded archives.
    pub fn loaded_archive_names(&self) -> Vec<String> {
        self.archives
            .iter()
            .chain(self.archive_catalog.iter())
            .map(|archive| archive.name.clone())
            .collect()
    }

    /// Return only the archives that participate in name lookup.
    ///
    /// [`Self::loaded_archive_names`] also reports the catalogued nested
    /// archives, whose contents are parsed and in memory but absent from the
    /// lookup index — `get`/`resolve_ref` cannot reach them. Tools that report
    /// where an asset lives need to tell the two apart, or they present
    /// unreachable bytes as if normal lookup would find them.
    pub fn registered_archive_names(&self) -> Vec<String> {
        self.archives
            .iter()
            .map(|archive| archive.name.clone())
            .collect()
    }

    /// Return the RA2 installation directory.
    pub fn ra2_dir(&self) -> &Path {
        &self.ra2_dir
    }

    fn lookup_asset_entry(&self, name: &str) -> Option<(&NamedArchive, i32)> {
        let location = self.lookup_location_for_name(name)?;
        let archive = self.archives.get(location.archive_index)?;
        Some((archive, location.entry_id))
    }

    fn lookup_location_for_name(&self, name: &str) -> Option<AssetLocation> {
        let primary_id = mix_hash(name);
        let alternate_id = westwood_hash(name);
        let primary = self.lookup_index.get(&primary_id).copied();
        let alternate = if alternate_id == primary_id {
            None
        } else {
            self.lookup_index.get(&alternate_id).copied()
        };

        match (primary, alternate) {
            (Some(primary), Some(alternate)) => {
                if primary.archive_index <= alternate.archive_index {
                    Some(primary)
                } else {
                    Some(alternate)
                }
            }
            (Some(primary), None) => Some(primary),
            (None, Some(alternate)) => Some(alternate),
            (None, None) => None,
        }
    }

    fn append_registered_archive(&mut self, named: NamedArchive) {
        let archive_index = self.archives.len();
        for entry in named.archive.entries() {
            self.lookup_index.entry(entry.id).or_insert(AssetLocation {
                archive_index,
                entry_id: entry.id,
            });
        }
        log::info!(
            "Registered {} at priority {} ({} entries)",
            named.name,
            archive_index,
            named.archive.entry_count()
        );
        self.archives.push(named);
    }

    fn mount_native_map_media(&mut self, mode: MediaArchiveMode) -> Result<(), AssetError> {
        match mode {
            MediaArchiveMode::Numbered { media_index } => {
                let number = media_index.wrapping_add(1);
                self.mount_named_archive(&format!("mapsmd{number:02}.mix"), true)?;
                self.mount_named_archive(&format!("maps{number:02}.mix"), false)?;
            }
            MediaArchiveMode::CdWildcard => {
                let md = self.mount_wildcard_archives(None, "mapsmd")?;
                if !md.matched {
                    // This is the native fallback branch, even though active
                    // YR still fails the later required-MD global check.
                    self.mount_wildcard_archives(None, "maps")?;
                }
                if !md.first_registered {
                    return Err(AssetError::AssetNotFound {
                        name: "MAPSMD*.MIX".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn mount_native_movie_media(&mut self, mode: MediaArchiveMode) -> Result<(), AssetError> {
        match mode {
            MediaArchiveMode::Numbered { media_index } => {
                let number = media_index.wrapping_add(1);
                if !self.mount_named_archive(&format!("movmd{number:02}.mix"), false)? {
                    self.mount_named_archive(&format!("movies{number:02}.mix"), true)?;
                }
            }
            MediaArchiveMode::CdWildcard => {
                // Retail starts with root MOVMD, then each successful raw-file
                // probe overwrites the selected wildcard in this exact order.
                let mut directory = None;
                let mut family = "movmd";
                for (probe, selected_directory, selected_family) in [
                    ("MOVIES01.MIX", None, "movies"),
                    ("MIXFILES/MOVIES01.MIX", Some("MIXFILES"), "movies"),
                    ("MOVMD01.MIX", None, "movmd"),
                    ("MIXFILES/MOVMD01.MIX", Some("MIXFILES"), "movmd"),
                    ("MOVMD03.MIX", None, "movmd"),
                    ("MIXFILES/MOVMD03.MIX", Some("MIXFILES"), "movmd"),
                ] {
                    if self.raw_relative_file_available(probe) {
                        directory = selected_directory;
                        family = selected_family;
                    }
                }

                let mut mounted = self.mount_wildcard_archives(directory, family)?;
                if !mounted.matched {
                    mounted = self.mount_wildcard_archives(None, "movies")?;
                }
                if !mounted.first_registered {
                    return Err(AssetError::AssetNotFound {
                        name: if family == "movmd" {
                            "MOVMD*.MIX".to_string()
                        } else {
                            "MOVIES*.MIX".to_string()
                        },
                    });
                }
            }
        }
        Ok(())
    }

    fn mount_wildcard_archives(
        &mut self,
        relative_directory: Option<&str>,
        family_prefix: &str,
    ) -> Result<WildcardMount, AssetError> {
        let directory = relative_directory
            .map(|relative| self.ra2_dir.join(relative))
            .unwrap_or_else(|| self.ra2_dir.clone());
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(WildcardMount::default());
            }
            Err(err) => return Err(err.into()),
        };

        let mut result = WildcardMount::default();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if !retail_enumerated_file(&path) {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            if !mix_wildcard_family_matches(file_name, family_prefix) {
                continue;
            }

            let first_match = !result.matched;
            result.matched = true;
            let display_name = relative_directory
                .map(|relative| format!("{relative}\\{file_name}"))
                .unwrap_or_else(|| file_name.to_string());
            let registered = self.try_register_disk_archive(&path, &display_name);
            if first_match {
                result.first_registered = registered;
            }
        }
        Ok(result)
    }

    fn try_register_disk_archive(&mut self, path: &Path, display_name: &str) -> bool {
        if self.archive_is_known(display_name) {
            return true;
        }
        let archive = match MixArchive::load(path) {
            Ok(archive) => archive,
            Err(err) => {
                log::debug!("Skipping media archive {}: {}", path.display(), err);
                return false;
            }
        };
        self.append_registered_archive(NamedArchive {
            name: display_name.to_string(),
            archive,
        });
        true
    }

    fn raw_relative_file_available(&self, relative: &str) -> bool {
        retail_enumerated_file(&self.ra2_dir.join(relative))
    }

    fn mount_named_archive(&mut self, name: &str, required: bool) -> Result<bool, AssetError> {
        if self
            .archives
            .iter()
            .any(|archive| archive_name_matches(&archive.name, name))
        {
            return Ok(true);
        }

        if let Some(disk_path) = self.loose_path(name).map(Path::to_path_buf) {
            let archive = MixArchive::load(&disk_path)?;
            if let Some(position) = self
                .archive_catalog
                .iter()
                .position(|catalogued| archive_name_matches(&catalogued.name, name))
            {
                self.archive_catalog.remove(position);
            }
            self.append_registered_archive(NamedArchive {
                name: name.to_string(),
                archive,
            });
            return Ok(true);
        }

        if let Some(position) = self
            .archive_catalog
            .iter()
            .position(|archive| archive_name_matches(&archive.name, name))
        {
            let archive = self.archive_catalog.remove(position);
            self.append_registered_archive(archive);
            return Ok(true);
        }

        if let Some((parent_index, entry_id)) = self
            .lookup_location_for_name(name)
            .map(|location| (location.archive_index, location.entry_id))
        {
            let parent_name = self.archives[parent_index].name.clone();
            let archive = self.archives[parent_index]
                .archive
                .nested_archive_by_id(entry_id)?
                .ok_or_else(|| AssetError::AssetNotFound {
                    name: name.to_string(),
                })?;
            self.append_registered_archive(NamedArchive {
                name: format!("{parent_name} -> {name}"),
                archive,
            });
            return Ok(true);
        }

        if required {
            Err(AssetError::AssetNotFound {
                name: name.to_string(),
            })
        } else {
            log::debug!("{name} not found (optional, skipping)");
            Ok(false)
        }
    }

    fn catalog_named_nested_archives(&mut self) {
        let mut discovered = Vec::new();

        for parent in &self.archives {
            for &nested_name in KNOWN_NESTED_MIX_NAMES {
                if self.archive_is_known(nested_name)
                    || discovered.iter().any(|archive: &NamedArchive| {
                        archive_name_matches(&archive.name, nested_name)
                    })
                {
                    continue;
                }

                let entry_id = mix_hash(nested_name);
                let Ok(Some(archive)) = parent.archive.nested_archive_by_id(entry_id) else {
                    continue;
                };
                if archive.entry_count() == 0 {
                    continue;
                }
                discovered.push(NamedArchive {
                    name: format!("{} -> {nested_name}", parent.name),
                    archive,
                });
            }
        }

        if !discovered.is_empty() {
            log::info!(
                "Catalogued {} unregistered named nested archives",
                discovered.len()
            );
            self.archive_catalog.extend(discovered);
        }
    }

    /// Register one loose YRO at its native scenario-enumeration point.
    ///
    /// `SessionClass__ScanMultiplayerMapFiles @ 0x00699980` performs this
    /// registration before deriving and opening the archive's embedded PKT.
    pub(crate) fn register_loose_yro_archive(&mut self, path: &Path) -> Result<bool, AssetError> {
        if !retail_enumerated_file(path)
            || !path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("yro"))
        {
            return Ok(false);
        }
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            return Ok(false);
        };
        if self.archive_is_known(file_name) {
            return Ok(true);
        }

        let archive = MixArchive::load(path)?;
        self.append_registered_archive(NamedArchive {
            name: file_name.to_string(),
            archive,
        });
        Ok(true)
    }

    fn archive_is_known(&self, name: &str) -> bool {
        self.archives
            .iter()
            .chain(self.archive_catalog.iter())
            .any(|archive| archive_name_matches(&archive.name, name))
    }

    fn open_loose_root(ra2_dir: &Path) -> Result<HashMap<String, LooseAsset>, AssetError> {
        let mut files = HashMap::new();

        for entry in std::fs::read_dir(ra2_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            let key = file_name.to_ascii_lowercase();
            if files.contains_key(&key) {
                continue;
            }

            let bytes = match entry.metadata() {
                Ok(metadata) if metadata.len() == 0 => LooseBytes::Empty,
                Ok(_) => {
                    let file = match File::open(&path) {
                        Ok(file) => file,
                        Err(err) => {
                            log::debug!("Skipping loose {}: {}", path.display(), err);
                            continue;
                        }
                    };
                    let mapped = unsafe { memmap2::MmapOptions::new().map(&file) };
                    match mapped {
                        Ok(mapped) => LooseBytes::Mapped(mapped),
                        Err(err) => {
                            log::debug!("Skipping loose {}: {}", path.display(), err);
                            continue;
                        }
                    }
                }
                Err(err) => {
                    log::debug!("Skipping loose {}: {}", path.display(), err);
                    continue;
                }
            };
            files.insert(
                key,
                LooseAsset {
                    path: path.clone(),
                    source_name: format!("loose:{}", path.display()),
                    bytes,
                },
            );
        }

        Ok(files)
    }

    fn loose_asset(&self, name: &str) -> Option<&LooseAsset> {
        let key = loose_lookup_key(name)?;
        self.loose_files.get(&key)
    }

    fn loose_path(&self, name: &str) -> Option<&Path> {
        self.loose_asset(name).map(|asset| asset.path.as_path())
    }

    fn rebuild_indexes(&mut self) {
        self.lookup_index.clear();

        for (archive_index, named) in self.archives.iter().enumerate() {
            for entry in named.archive.entries() {
                self.lookup_index.entry(entry.id).or_insert(AssetLocation {
                    archive_index,
                    entry_id: entry.id,
                });
            }
        }
    }
}

fn archive_name_matches(stored_name: &str, requested_name: &str) -> bool {
    archive_lookup_keys(stored_name)
        .iter()
        .any(|key| key.eq_ignore_ascii_case(requested_name))
}

fn loose_lookup_key(name: &str) -> Option<String> {
    let path = Path::new(name);
    if path
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty())
    {
        return None;
    }
    path.file_name()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
}

fn retail_enumerated_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x116 == 0
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn mix_wildcard_family_matches(file_name: &str, family_prefix: &str) -> bool {
    let path = Path::new(file_name);
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mix"))
        && path
            .file_stem()
            .and_then(OsStr::to_str)
            .is_some_and(|stem| {
                stem.get(..family_prefix.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(family_prefix))
            })
}

fn archive_lookup_keys(name: &str) -> Vec<String> {
    let mut keys = Vec::with_capacity(3);
    keys.push(name.to_ascii_lowercase());

    if let Some(rest) = name.strip_prefix("nested:") {
        keys.push(rest.trim().to_ascii_lowercase());
    }
    if let Some((_, leaf)) = name.rsplit_once("->") {
        keys.push(leaf.trim().to_ascii_lowercase());
    }

    keys.sort();
    keys.dedup();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vera20k-asset-manager-{label}-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write_mix(&self, archive_name: &str, entry_name: &str, body: &[u8]) {
            let path = self.0.join(archive_name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create archive parent");
            }
            std::fs::write(path, make_new_format_mix_bytes(entry_name, body))
                .expect("write test MIX");
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn empty_manager(ra2_dir: &Path) -> AssetManager {
        AssetManager {
            archives: Vec::new(),
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: AssetManager::open_loose_root(ra2_dir).expect("open loose test root"),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: ra2_dir.to_path_buf(),
        }
    }

    fn make_new_format_mix_bytes(name: &str, body: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        let entry_id = mix_hash(name);

        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&entry_id.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(body);
        data
    }

    fn make_new_format_mix(name: &str, body: &[u8]) -> MixArchive {
        MixArchive::from_bytes(make_new_format_mix_bytes(name, body))
            .expect("new-format test mix should parse")
    }

    fn make_old_format_mix(name: &str, body: &[u8]) -> MixArchive {
        let mut data = Vec::new();
        let entry_id = westwood_hash(name);

        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&entry_id.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(body);

        MixArchive::from_bytes(data).expect("old-format test mix should parse")
    }

    #[test]
    fn media_mode_matches_native_uppercase_substring_command_line_test() {
        assert_eq!(
            MediaArchiveMode::from_arguments(["-cd"], 2),
            MediaArchiveMode::CdWildcard
        );
        assert_eq!(
            MediaArchiveMode::from_arguments(["prefix-CDsuffix"], 2),
            MediaArchiveMode::CdWildcard
        );
        assert_eq!(
            MediaArchiveMode::from_arguments(["--shell-capture"], 2),
            MediaArchiveMode::Numbered { media_index: 2 }
        );
    }

    #[test]
    fn numbered_media_uses_media_plus_one_and_movie_family_fallback() {
        let directory = TestDirectory::new("numbered-media");
        directory.write_mix("MAPSMD03.MIX", "md-map.bin", b"md");
        directory.write_mix("MAPS03.MIX", "base-map.bin", b"base");
        directory.write_mix("MOVIES03.MIX", "movie.bin", b"movie");

        let mut manager = empty_manager(directory.path());
        let mode = MediaArchiveMode::Numbered { media_index: 2 };
        manager
            .mount_native_map_media(mode)
            .expect("numbered map media");
        manager
            .mount_native_movie_media(mode)
            .expect("numbered movie fallback");

        assert_eq!(
            manager.loaded_archive_names(),
            vec![
                "mapsmd03.mix".to_string(),
                "maps03.mix".to_string(),
                "movies03.mix".to_string(),
            ]
        );
    }

    #[test]
    fn cd_map_wildcard_uses_md_family_and_preserves_enumeration_order() {
        let directory = TestDirectory::new("cd-map-media");
        directory.write_mix("MAPSMD01.MIX", "one.bin", b"one");
        directory.write_mix("mapsmd03.mix", "three.bin", b"three");
        directory.write_mix("MAPS01.MIX", "base.bin", b"base");

        let expected: Vec<String> = std::fs::read_dir(directory.path())
            .expect("enumerate expected order")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_name = entry.file_name().into_string().ok()?;
                mix_wildcard_family_matches(&file_name, "mapsmd").then_some(file_name)
            })
            .collect();

        let mut manager = empty_manager(directory.path());
        manager
            .mount_native_map_media(MediaArchiveMode::CdWildcard)
            .expect("CD wildcard map media");

        assert_eq!(manager.loaded_archive_names(), expected);
        assert!(
            manager
                .loaded_archive_names()
                .iter()
                .all(|name| name.to_ascii_lowercase().starts_with("mapsmd"))
        );
    }

    #[test]
    fn cd_movie_probe_order_selects_movmd_before_movies_family() {
        let directory = TestDirectory::new("cd-movie-media");
        directory.write_mix("MOVIES01.MIX", "base-movie.bin", b"base");
        directory.write_mix("MOVMD03.MIX", "md-three.bin", b"three");
        directory.write_mix("MOVMD04.MIX", "md-four.bin", b"four");

        let expected: Vec<String> = std::fs::read_dir(directory.path())
            .expect("enumerate expected order")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_name = entry.file_name().into_string().ok()?;
                mix_wildcard_family_matches(&file_name, "movmd").then_some(file_name)
            })
            .collect();

        let mut manager = empty_manager(directory.path());
        manager
            .mount_native_movie_media(MediaArchiveMode::CdWildcard)
            .expect("CD wildcard movie media");

        assert_eq!(manager.loaded_archive_names(), expected);
    }

    #[test]
    fn loose_yro_is_absent_until_scenario_time_registration() {
        let directory = TestDirectory::new("scenario-yro");
        directory.write_mix("CUSTOM.YRO", "custom.map", b"[Basic]\nName=Custom");
        let yro_path = directory.path().join("CUSTOM.YRO");

        let mut manager = empty_manager(directory.path());
        assert!(manager.lookup_asset_entry("custom.map").is_none());

        assert!(
            manager
                .register_loose_yro_archive(&yro_path)
                .expect("register scenario YRO")
        );
        assert_eq!(
            manager.get_ref("custom.map"),
            Some(&b"[Basic]\nName=Custom"[..])
        );
    }

    #[test]
    fn loading_archives_register_md_then_base_once_at_resource_setup() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "base.mix".to_string(),
                archive: make_new_format_mix("base.bin", b"base"),
            }],
            // Deliberately reverse catalogue order: the setup function's named
            // construction order, not discovery order, controls registration.
            archive_catalog: vec![
                NamedArchive {
                    name: "master.mix -> load.mix".to_string(),
                    archive: make_new_format_mix("loading.bin", b"base-loading"),
                },
                NamedArchive {
                    name: "master.mix -> loadmd.mix".to_string(),
                    archive: make_new_format_mix("loading.bin", b"md-loading"),
                },
            ],
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(
            manager
                .register_loading_archives()
                .expect("register loading archives")
        );
        assert_eq!(
            manager.loaded_archive_names(),
            vec![
                "base.mix".to_string(),
                "master.mix -> loadmd.mix".to_string(),
                "master.mix -> load.mix".to_string(),
            ]
        );
        assert_eq!(manager.get_ref("loading.bin"), Some(&b"md-loading"[..]));

        assert!(
            manager
                .register_loading_archives()
                .expect("repeat loading setup")
        );
        assert_eq!(manager.archives.len(), 3);
    }

    #[test]
    fn loading_archive_setup_returns_at_each_missing_archive_boundary() {
        let directory = TestDirectory::new("loading-archive-failure");
        let mut missing_md = empty_manager(directory.path());
        missing_md.archive_catalog.push(NamedArchive {
            name: "master.mix -> load.mix".to_string(),
            archive: make_new_format_mix("base-loading.bin", b"base"),
        });
        assert!(
            !missing_md
                .register_loading_archives()
                .expect("missing MD archive is not an I/O failure")
        );
        assert!(missing_md.archives.is_empty());

        let mut missing_base = empty_manager(directory.path());
        missing_base.archive_catalog.push(NamedArchive {
            name: "master.mix -> loadmd.mix".to_string(),
            archive: make_new_format_mix("md-loading.bin", b"md"),
        });
        assert!(
            !missing_base
                .register_loading_archives()
                .expect("missing base archive is not an I/O failure")
        );
        assert_eq!(
            missing_base.loaded_archive_names(),
            vec!["master.mix -> loadmd.mix".to_string()]
        );
    }

    #[test]
    fn load_file_from_mix_cache_survives_theater_replacement_and_normalizes_case() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "base.mix".to_string(),
                archive: make_new_format_mix("base.bin", b"base"),
            }],
            archive_catalog: vec![
                NamedArchive {
                    name: "master.mix -> snow.mix".to_string(),
                    archive: make_new_format_mix("weather.bin", b"snow"),
                },
                NamedArchive {
                    name: "master.mix -> temperat.mix".to_string(),
                    archive: make_new_format_mix("weather.bin", b"temperate"),
                },
            ],
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();
        manager
            .activate_theater_archives("SNOW", &["snow.mix"])
            .expect("activate snow");

        let first = manager
            .load_file_from_mix("Weather.Bin")
            .expect("first MIX load");
        assert_eq!(&*first.bytes, b"snow");

        manager
            .activate_theater_archives("TEMPERATE", &["temperat.mix"])
            .expect("replace theater");
        assert_eq!(
            manager
                .resolve_ref("WEATHER.BIN")
                .map(|result| result.bytes),
            Some(&b"temperate"[..])
        );

        let cached = manager
            .load_file_from_mix("WEATHER.BIN")
            .expect("cached MIX load");
        assert_eq!(&*cached.bytes, b"snow");
        assert!(Arc::ptr_eq(&first.bytes, &cached.bytes));
        assert_eq!(cached.source_archive.as_ref(), "master.mix -> snow.mix");
    }

    #[test]
    fn indexed_lookup_prefers_earliest_archive_across_hash_fallbacks() {
        let mut manager = AssetManager {
            archives: vec![
                NamedArchive {
                    name: "theme.mix".to_string(),
                    archive: make_old_format_mix("audio.idx", b"westwood"),
                },
                NamedArchive {
                    name: "audio.mix".to_string(),
                    archive: make_new_format_mix("audio.idx", b"crc32"),
                },
            ],
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        let (bytes, source) = manager
            .get_with_source("audio.idx")
            .expect("indexed lookup should find audio.idx");
        assert_eq!(bytes, b"westwood");
        assert_eq!(source, "theme.mix");

        let resolved = manager
            .resolve_ref("audio.idx")
            .expect("observational resolution should preserve first match");
        assert_eq!(resolved.bytes, b"westwood");
        assert_eq!(resolved.source_archive, "theme.mix");
        assert_eq!(resolved.entry_id, westwood_hash("audio.idx"));
    }

    #[test]
    fn archive_lookup_is_case_insensitive() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "RA2.MIX".to_string(),
                archive: make_new_format_mix("rules.ini", b"test"),
            }],
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(manager.archive("ra2.mix").is_some());
        assert!(manager.archive("RA2.MIX").is_some());
    }

    #[test]
    fn archive_lookup_matches_nested_leaf_aliases() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "language.mix -> audio.mix".to_string(),
                archive: make_new_format_mix("audio.idx", b"test"),
            }],
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(manager.archive("audio.mix").is_some());
        assert!(manager.archive("language.mix -> audio.mix").is_some());
    }

    #[test]
    fn neutral_sidebar_mix_names_match_retail_hashes_and_catalog() {
        let expected = [("sidenc01.mix", 0x330A_4ADF), ("sidenc02.mix", 0x74AA_300F)];

        for (name, entry_id) in expected {
            assert_eq!(mix_hash(name), entry_id);
            assert!(KNOWN_NESTED_MIX_NAMES.contains(&name));
        }
    }

    #[test]
    fn registering_nested_archive_appends_without_displacing_first_winner() {
        let nested_bytes = make_new_format_mix_bytes("duplicate.bin", b"late");
        let mut manager = AssetManager {
            archives: vec![
                NamedArchive {
                    name: "early.mix".to_string(),
                    archive: make_new_format_mix("duplicate.bin", b"early"),
                },
                NamedArchive {
                    name: "master.mix".to_string(),
                    archive: make_new_format_mix("audio.mix", &nested_bytes),
                },
            ],
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();
        manager.catalog_named_nested_archives();

        manager
            .load_nested("audio.mix")
            .expect("named child archive should register");

        assert_eq!(manager.loaded_archive_names()[2], "master.mix -> audio.mix");
        let (bytes, source) = manager
            .get_with_source("duplicate.bin")
            .expect("duplicate should resolve");
        assert_eq!(bytes, b"early");
        assert_eq!(source, "early.mix");
    }

    #[test]
    fn neutral_archive_registration_preserves_md_first_winner() {
        let mut manager = AssetManager {
            archives: Vec::new(),
            archive_catalog: vec![
                NamedArchive {
                    name: "ra2md.mix -> ntrlmd.mix".to_string(),
                    archive: make_new_format_mix("duplicate.shp", b"yr"),
                },
                NamedArchive {
                    name: "ra2.mix -> neutral.mix".to_string(),
                    archive: make_new_format_mix("duplicate.shp", b"base"),
                },
            ],
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(
            manager
                .register_neutral_archives()
                .expect("register neutral pair")
        );
        assert_eq!(
            manager.loaded_archive_names(),
            [
                "ra2md.mix -> ntrlmd.mix".to_string(),
                "ra2.mix -> neutral.mix".to_string(),
            ]
        );

        let loaded = manager
            .load_file_from_mix("DUPLICATE.SHP")
            .expect("registered neutral asset");
        assert_eq!(&*loaded.bytes, b"yr");
        assert_eq!(loaded.source_archive.as_ref(), "ra2md.mix -> ntrlmd.mix");
    }

    #[test]
    fn neutral_archive_registration_stops_when_md_archive_is_missing() {
        let mut manager = AssetManager {
            archives: Vec::new(),
            archive_catalog: vec![NamedArchive {
                name: "ra2.mix -> neutral.mix".to_string(),
                archive: make_new_format_mix("base-only.shp", b"base"),
            }],
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(
            !manager
                .register_neutral_archives()
                .expect("missing MD archive is the native optional path")
        );
        assert!(manager.load_file_from_mix("BASE-ONLY.SHP").is_none());
    }

    #[test]
    fn loose_mixed_case_file_supplies_bytes_before_registered_mix() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "first.mix".to_string(),
                archive: make_new_format_mix("ra2md.csf", b"archived"),
            }],
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::from([(
                "ra2md.csf".to_string(),
                LooseAsset {
                    path: PathBuf::from("RA2MD.CSF"),
                    source_name: "loose:RA2MD.CSF".to_string(),
                    bytes: LooseBytes::Owned(Box::from(&b"loose"[..])),
                },
            )]),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        assert!(manager.contains("Ra2Md.CsF"));
        let (bytes, source) = manager
            .get_with_source("Ra2Md.CsF")
            .expect("case-insensitive loose file should resolve");
        assert_eq!(bytes, b"loose");
        assert_eq!(source, "loose:RA2MD.CSF");

        let mix_only = manager
            .load_file_from_mix("RA2MD.CSF")
            .expect("LoadFileFromMIX checks MIX before the loose fallback");
        assert_eq!(&*mix_only.bytes, b"archived");
        assert_eq!(mix_only.source_archive.as_ref(), "first.mix");
    }

    #[test]
    fn load_file_from_mix_caches_raw_fallback_before_later_mix_registration() {
        let mut manager = AssetManager {
            archives: Vec::new(),
            archive_catalog: Vec::new(),
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::from([(
                "weather.bin".to_string(),
                LooseAsset {
                    path: PathBuf::from("WEATHER.BIN"),
                    source_name: "loose:WEATHER.BIN".to_string(),
                    bytes: LooseBytes::Owned(Box::from(&b"raw-first"[..])),
                },
            )]),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };

        let first = manager
            .load_file_from_mix("Weather.Bin")
            .expect("raw fallback");
        assert_eq!(&*first.bytes, b"raw-first");
        assert_eq!(first.source_archive.as_ref(), "loose:WEATHER.BIN");

        manager.append_registered_archive(NamedArchive {
            name: "later.mix".to_string(),
            archive: make_new_format_mix("WEATHER.BIN", b"later-mix"),
        });
        let cached = manager
            .load_file_from_mix("WEATHER.BIN")
            .expect("cached raw winner");
        assert_eq!(&*cached.bytes, b"raw-first");
        assert_eq!(cached.source_archive.as_ref(), "loose:WEATHER.BIN");
    }

    #[test]
    fn theater_group_replaces_old_archives_and_repeated_identity_is_noop() {
        let mut manager = AssetManager {
            archives: vec![NamedArchive {
                name: "base.mix".to_string(),
                archive: make_new_format_mix("base.bin", b"base"),
            }],
            archive_catalog: vec![
                NamedArchive {
                    name: "master.mix -> snow.mix".to_string(),
                    archive: make_new_format_mix("snow.bin", b"snow"),
                },
                NamedArchive {
                    name: "master.mix -> temperat.mix".to_string(),
                    archive: make_new_format_mix("temperate.bin", b"temperate"),
                },
            ],
            lookup_index: HashMap::new(),
            mix_file_cache: Mutex::new(HashMap::new()),
            loose_files: HashMap::new(),
            active_theater: None,
            active_theater_archives: Vec::new(),
            ra2_dir: PathBuf::new(),
        };
        manager.rebuild_indexes();

        manager
            .activate_theater_archives("SNOW", &["snow.mix"])
            .expect("snow group");
        assert!(manager.contains("snow.bin"));
        assert_eq!(manager.archives.len(), 2);

        manager
            .activate_theater_archives("snow", &["snow.mix"])
            .expect("same theater is a no-op");
        assert_eq!(manager.archives.len(), 2);

        manager
            .activate_theater_archives("TEMPERATE", &["temperat.mix"])
            .expect("temperate group");
        assert!(!manager.contains("snow.bin"));
        assert!(manager.contains("temperate.bin"));
        assert_eq!(manager.archives.len(), 2);
    }

    #[test]
    fn neutral_sidebar_nested_names_expose_leaf_lookup_keys() {
        assert_eq!(
            archive_lookup_keys("ra2.mix -> sidenc01.mix"),
            vec![
                "ra2.mix -> sidenc01.mix".to_string(),
                "sidenc01.mix".to_string(),
            ]
        );
        assert_eq!(
            archive_lookup_keys("ra2md.mix -> sidenc02.mix"),
            vec![
                "ra2md.mix -> sidenc02.mix".to_string(),
                "sidenc02.mix".to_string(),
            ]
        );
    }
}
