//! Retail byte-golden certification suite for src/assets/ parsers.
//!
//! Golden source: the retail install's own file bytes (machine-derived, never
//! hand-computed). Two evidence tiers, encoded in test names:
//!
//! - `certify_*` — the assertion's reference is the retail bytes themselves
//!   (parse-total, header-consistency, byte round-trip). These are citable
//!   certification checks under the CLAUDE.md parity bar.
//! - `ratchet_*` — FNV digests of OUR decode output. Regression ratchets only;
//!   explicitly NOT parity evidence (Rust-vs-prior-Rust).
//!
//! Decoder VALUE parity vs gamemd (RLE/LCW pixels, ADPCM samples, CSF text) is
//! UNVERIFIED-pending-instrument: it needs gamemd-derived vectors (Ghidra
//! emulation of the native decompressors) or pixel goldens.
//!
//! Run: cargo test --release --test retail_goldens -- --ignored
//! Regenerate manifest: RETAIL_GOLDENS_WRITE=1 cargo test --release --test retail_goldens -- --ignored certify_corpus_manifest

mod certify_audio;
mod certify_roundtrip;
mod certify_structural;
mod ratchet_digests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vera20k::assets::asset_manager::AssetManager;
use vera20k::assets::format_sniff::detect_format;

/// FNV-1a 64-bit. Stable across platforms and Rust releases (unlike
/// DefaultHasher/SipHash), which committed goldens require.
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn fnv1a(bytes: &[u8], mut h: u64) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

pub fn fnv1a_start(bytes: &[u8]) -> u64 {
    fnv1a(bytes, FNV_OFFSET)
}

/// Retail install root from RA2_DIR, or None → caller prints SKIP and returns.
/// Mirrors tests/tile_coverage.rs gating so CI (no retail assets) stays green.
pub fn ra2_dir() -> Option<PathBuf> {
    let dir = std::env::var("RA2_DIR").ok()?;
    let p = PathBuf::from(dir);
    p.exists().then_some(p)
}

/// Loads the full archive chain the engine itself loads (core chain + extra
/// disk MIXes), identical to src/bin/audit-assets.rs.
pub fn load_corpus(root: &Path) -> AssetManager {
    let mut am = AssetManager::new(root).expect("AssetManager::new");
    am.load_all_disk_mixes().expect("load_all_disk_mixes");
    am
}

/// One sniffed leaf entry: (archive name, entry id, size, format tag).
pub struct CorpusEntry {
    pub archive: String,
    pub id: i32,
    pub size: usize,
    pub format: &'static str,
}

/// Walk every archive entry, sniff formats, and hand (entry, bytes) to `f`.
/// Iteration order is deterministic: visit_archives order × index order.
pub fn walk_sniffed(am: &AssetManager, mut f: impl FnMut(&CorpusEntry, &[u8])) {
    am.visit_archives(|arch_name, archive| {
        for entry in archive.entries() {
            let Some(data) = archive.get_by_id(entry.id) else {
                continue;
            };
            let Some(fmt) = detect_format(data) else {
                continue;
            };
            let ce = CorpusEntry {
                archive: arch_name.to_string(),
                id: entry.id,
                size: data.len(),
                format: fmt,
            };
            f(&ce, data);
        }
    });
}

/// Committed golden manifest. Counts + 64-bit digests + names only — NEVER raw
/// retail bytes (public repo; digests don't reproduce content, bytes would).
#[derive(Serialize, Deserialize, Default)]
pub struct Manifest {
    pub schema: u32,
    /// archive name -> (entry_count, digest over sorted (id, size) pairs).
    pub archives: BTreeMap<String, (usize, u64)>,
    /// format tag -> sniffed file count.
    pub format_counts: BTreeMap<String, usize>,
    /// Entry count across the audio.idx/audio.bag pairs (AUDIOMD.MIX +
    /// AUDIO.MIX) — separate from format_counts, which is sniffer-derived.
    #[serde(default)]
    pub bag_aud: usize,
    /// format tag -> ratchet rollup digest over all decoded outputs.
    pub decode_rollups: BTreeMap<String, u64>,
    /// Curated per-file ratchet rows: (format, filename, decode digest).
    pub files: Vec<(String, String, u64)>,
}

pub const MANIFEST_PATH: &str = "tests/fixtures/retail_goldens/manifest.json";

pub fn read_manifest() -> Option<Manifest> {
    let text = std::fs::read_to_string(MANIFEST_PATH).ok()?;
    Some(serde_json::from_str(&text).expect("manifest.json parses"))
}

pub fn write_manifest(m: &Manifest) {
    std::fs::create_dir_all("tests/fixtures/retail_goldens").expect("fixture dir");
    let text = serde_json::to_string_pretty(m).expect("serialize manifest");
    std::fs::write(MANIFEST_PATH, text).expect("write manifest");
}

/// True when the run should (re)generate goldens instead of asserting them.
pub fn write_mode() -> bool {
    std::env::var("RETAIL_GOLDENS_WRITE").is_ok_and(|v| v == "1")
}

/// Best-effort MIX id -> filename map from the local XCC database, for legible
/// failure/record output. Empty (ids print as hashes) when the DB is absent.
pub fn xcc_name_map() -> std::collections::HashMap<i32, String> {
    let Ok(db) = vera20k::assets::xcc_database::XccDatabase::load_from_disk() else {
        return Default::default();
    };
    db.build_hash_dictionary()
        .into_iter()
        .map(|(name, id)| (id, name))
        .collect()
}
