//! Retail-root discovery and `AssetManager` construction for the `asset` tool.
//!
//! Collapses the three incompatible conventions in the tree — hardcoded install
//! paths in tool binaries, `GameConfig::load()`, and the `RA2_DIR` env var used
//! by tests — into one resolution order.
//!
//! ## Dependency rules
//! - Depends on `assets/` and `util/config`. No render, sim, or app types.

use std::path::{Path, PathBuf};

use crate::assets::asset_manager::{AssetManager, MediaArchiveMode};
use crate::util::config::GameConfig;

/// Retail's digital-install media index. Startup selects the `03` archive family
/// from it. Pinned explicitly so the tool never takes the `-CD` branch.
const RETAIL_MEDIA_INDEX: i32 = 2;

/// Where the retail root came from, for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSource {
    Flag,
    Env,
    Config,
}

/// Resolve the retail install directory in strict priority order:
/// `--ra2-dir` → `$RA2_DIR` → `config.toml` via [`GameConfig::load`].
///
/// The directory is verified to exist, because every downstream failure from a
/// wrong root looks like "asset not found" instead of "wrong directory".
pub fn resolve_ra2_dir(explicit: Option<&Path>) -> Result<(PathBuf, RootSource), String> {
    if let Some(dir) = explicit {
        return check(dir.to_path_buf(), RootSource::Flag);
    }
    if let Ok(env_dir) = std::env::var("RA2_DIR")
        && !env_dir.trim().is_empty()
    {
        return check(PathBuf::from(env_dir), RootSource::Env);
    }
    match GameConfig::load() {
        Ok(config) => check(config.paths.ra2_dir, RootSource::Config),
        Err(err) => Err(format!(
            "no retail directory: --ra2-dir not given, RA2_DIR unset, and config.toml \
             could not be read ({err})"
        )),
    }
}

fn check(dir: PathBuf, source: RootSource) -> Result<(PathBuf, RootSource), String> {
    if !dir.is_dir() {
        return Err(format!(
            "retail directory does not exist: {} (from {source:?})",
            dir.display()
        ));
    }
    Ok((dir, source))
}

/// Build the archive stack.
///
/// Always pins [`MediaArchiveMode::Numbered`] rather than using `AssetManager::new`.
/// The default mode substring-matches `-CD` across every process argument, so a
/// perfectly ordinary flag or path containing that sequence would otherwise flip
/// the whole manager into wildcard media mode.
///
/// `all_mixes` additionally mounts archives the startup path skips. It is a
/// tooling-only widening of the search set, so results found only that way are
/// not what the game would resolve.
pub fn open_manager(ra2_dir: &Path, all_mixes: bool) -> Result<AssetManager, String> {
    let mut manager = AssetManager::new_with_media_mode(
        ra2_dir,
        MediaArchiveMode::Numbered {
            media_index: RETAIL_MEDIA_INDEX,
        },
    )
    .map_err(|err| {
        format!(
            "could not mount archives under {}: {err}",
            ra2_dir.display()
        )
    })?;

    if all_mixes {
        manager
            .load_all_disk_mixes()
            .map_err(|err| format!("--all-mixes failed: {err}"))?;
    }

    Ok(manager)
}
