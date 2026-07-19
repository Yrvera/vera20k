//! Retail byte-golden certification suite for src/assets/ parsers.
//!
//! Golden source: the retail install's own file bytes (machine-derived, never
//! hand-computed). Two evidence tiers, encoded in test names:
//!
//! - `certify_*` — the assertion's reference is the retail bytes themselves.
//!   These are citable certification checks under the project parity bar.
//! - `ratchet_*` — FNV digests of OUR decode output. Regression ratchets only;
//!   explicitly NOT parity evidence (Rust-vs-prior-Rust).
//!
//! ## What each named check certifies
//!
//! | Check | Claim |
//! |---|---|
//! | `certify_corpus_manifest` | Corpus identity: archive set + per-archive (id, size) digests match the committed manifest (drift = install change, not parser failure) |
//! | `certify_parse_total_zero_failures` | Every sniffed retail file (8,824 on the reference install) parses without error |
//! | `certify_shp_structural` | SHP frame count matches raw header; every frame's pixel buffer matches its dims; frames lie inside file bounds (2,450 files) |
//! | `certify_shp_rle_row_exactness` | SHP format-2/3 decoded pixel VALUES: no retail row under-runs, so our length-bound decoder equals the original's width-driven consumer on all retail data (grammar verified from the binary's RLE blitters) |
//! | `certify_tmp_structural` | TMP tile grid matches raw header; 60x30 tiles; pixel/depth buffers match tile dims (5,536 files) |
//! | `certify_vxl_structural` | VXL limb/palette counts match header; voxels inside grids; normal indices in-table or exactly 255 (255 only in retail DUMMY placeholder limbs) |
//! | `certify_hva_structural` | HVA counts match raw header; exact file-size formula holds (220 files) |
//! | `certify_csf_structural` | CSF parsed entry count equals unique raw label records; raw walk lands exactly on EOF; version 3 |
//! | `certify_fnt_structural` | FNT glyph stride formula; per-glyph RGBA size and width capacity |
//! | `certify_pcx_structural` | PCX dims match raw header bounds; pixel buffer matches plane count (271 files) |
//! | `certify_aud_chunk_walk` | AUD chunk walk lands exactly on EOF; chunk outputs sum to header output_size; decode consumes every input nibble |
//! | `certify_audio_bag_total` | Every audio.idx entry (3,438 across AUDIOMD/AUDIO.MIX) resolves and decodes |
//! | `certify_pal_roundtrip_bytes` | Both palette scale formulas recomputed from raw bytes match parsed output channel-for-channel; all retail bytes in 6-bit domain |
//! | `certify_hva_roundtrip_bytes` | Section names and every transform f32 are bit-identical to the raw bytes |
//! | `certify_vpl_roundtrip_bytes` | Header fields and every lighting page byte-equal the raw file |
//! | `certify_mix_known_name_resolution` | Our filename CRC resolves known shipped names against Westwood-built MIX indexes |
//!
//! ## UNVERIFIED-pending-instrument (not covered by any check here)
//!
//! Decoder OUTPUT VALUES vs the original engine's decoders for: TMP block
//! pixels, AUD ADPCM sample values, CSF text values. Upgrade path: emulation
//! vectors of the native decompressors or pixel goldens (see
//! docs/plans/2026-07-05-parity-convergence-strategy.md, P2 oracles). The
//! `ratchet_*` digests pin today's output but certify nothing about it.
//!
//! Resolved 2026-07-19: SHP pixel values are certified by
//! `certify_shp_rle_row_exactness` (grammar verified from the binary +
//! corpus no-under-run proof; formats 0/1 are byte-copies by construction) —
//! see docs/research/SHP_RLE_ZERO_VALUE_CERTIFICATION_GHIDRA_REPORT.md. The
//! trailing sample declared by 4 retail AUDs is unreachable in the original
//! engine (see docs/research/AUD_TRAILING_SAMPLE_UNREACHABLE_GHIDRA_REPORT.md).
//!
//! Run: cargo test --release --test retail_goldens -- --ignored
//! Regenerate goldens: RETAIL_GOLDENS_WRITE=1 (same invocation; one session at
//! a time, reason in the commit)

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
