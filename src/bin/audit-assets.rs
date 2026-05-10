//! Coverage auditor for the project's asset parsers.
//!
//! Walks every entry of every loaded MIX archive, identifies the file's
//! extension via the XCC database (hash → filename), dispatches to the
//! matching parser, and tallies pass/fail per format. Used to convert
//! "the parser works on stuff we happen to render" into hard numbers.
//!
//! Run with: `cargo run --release --bin audit-assets`

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use vera20k::assets::asset_manager::AssetManager;
use vera20k::assets::aud_file::decode_aud;
use vera20k::assets::csf_file::CsfFile;
use vera20k::assets::fnt_file::FntFile;
use vera20k::assets::hva_file::HvaFile;
use vera20k::assets::pal_file::Palette;
use vera20k::assets::shp_file::ShpFile;
use vera20k::assets::tmp_file::TmpFile;
use vera20k::assets::vpl_file::VplFile;
use vera20k::assets::vxl_file::VxlFile;
use vera20k::assets::xcc_database::XccDatabase;

#[derive(Default)]
struct ExtTally {
    ok: u32,
    fail: u32,
    /// Sample failures: (filename, error message). Capped to keep output sane.
    failures: Vec<(String, String)>,
    /// Total bytes across all entries of this extension (rough coverage proxy).
    total_bytes: u64,
}

const FAILURE_SAMPLE_CAP: usize = 8;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let ra2_dir = Path::new("C:/Users/enok/Documents/Command and Conquer Red Alert II/");

    println!("Loading AssetManager from {} ...", ra2_dir.display());
    let mut am = AssetManager::new(ra2_dir).expect("AssetManager init");
    let extra = am.load_all_disk_mixes().unwrap_or(0);
    println!("Loaded extra disk MIXes: {extra}");

    // Build hash -> filename map from XCC database (so we know each entry's extension).
    println!("Loading XCC global mix database ...");
    let xcc = match XccDatabase::load_from_disk() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("XCC database not available: {e}");
            eprintln!("(Set XCC_DATABASE_PATH or install XCC Mixer to identify entries by name.)");
            return;
        }
    };
    let dict = xcc.build_hash_dictionary();
    let hash_to_name: HashMap<i32, String> = dict.into_iter().map(|(n, h)| (h, n)).collect();
    println!("XCC dictionary entries: {}", hash_to_name.len());

    let mut tallies: BTreeMap<String, ExtTally> = BTreeMap::new();
    let mut unknown_entries: u32 = 0;
    let mut unknown_bytes: u64 = 0;

    am.visit_archives(|_arch_name, archive| {
        for entry in archive.entries() {
            let Some(data) = archive.get_by_id(entry.id) else { continue };

            // Skip nested MIX archives — they're intermediate containers, not leaf assets.
            if data.len() >= 4 && data[0] == 0 && data[1] == 0 {
                // Could be a nested MIX. Not a leaf asset to parse.
                if vera20k::assets::mix_archive::MixArchive::looks_like_mix(data) {
                    continue;
                }
            }

            let Some(name) = hash_to_name.get(&entry.id) else {
                unknown_entries += 1;
                unknown_bytes += data.len() as u64;
                continue;
            };
            let Some(ext) = name.rsplit('.').next().map(|e| e.to_ascii_lowercase()) else {
                continue;
            };

            // Only audit formats we actually parse.
            let outcome = match ext.as_str() {
                "shp" => ShpFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "vxl" => VxlFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "hva" => HvaFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "tmp" => TmpFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "pal" => Palette::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "csf" => CsfFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "vpl" => VplFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "fnt" => FntFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
                "aud" => match decode_aud(data) {
                    Some(_) => Ok(()),
                    None => Err("decode_aud returned None".to_string()),
                },
                // Theater-suffixed TMPs: .urb, .sno, .tem, .lun, .des, .urn (URBANN)
                "urb" | "sno" | "tem" | "lun" | "des" | "urn" => {
                    TmpFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string())
                }
                _ => continue, // unaudited extension (.ini, .mix, .bik, etc.)
            };

            let tally = tallies.entry(ext.clone()).or_default();
            tally.total_bytes += data.len() as u64;
            match outcome {
                Ok(()) => tally.ok += 1,
                Err(msg) => {
                    tally.fail += 1;
                    if tally.failures.len() < FAILURE_SAMPLE_CAP {
                        tally.failures.push((name.clone(), msg));
                    }
                }
            }
        }
    });

    println!("\n=== Per-format coverage ===");
    println!("{:<6}  {:>8}  {:>8}  {:>10}  {:>10}", "ext", "ok", "fail", "ok %", "MB");
    let mut grand_ok: u32 = 0;
    let mut grand_fail: u32 = 0;
    for (ext, t) in &tallies {
        let total = t.ok + t.fail;
        let pct = if total > 0 {
            100.0 * (t.ok as f64) / (total as f64)
        } else {
            0.0
        };
        println!(
            ".{:<5}  {:>8}  {:>8}  {:>9.2}%  {:>10.2}",
            ext,
            t.ok,
            t.fail,
            pct,
            t.total_bytes as f64 / (1024.0 * 1024.0)
        );
        grand_ok += t.ok;
        grand_fail += t.fail;
    }
    let grand_total = grand_ok + grand_fail;
    let grand_pct = if grand_total > 0 {
        100.0 * (grand_ok as f64) / (grand_total as f64)
    } else {
        0.0
    };
    println!(
        "\nTotal audited: {} (passed {}, failed {}, {:.2}%)",
        grand_total, grand_ok, grand_fail, grand_pct
    );
    println!(
        "Unidentified entries (no XCC name): {} ({:.1} MB) — skipped",
        unknown_entries,
        unknown_bytes as f64 / (1024.0 * 1024.0)
    );

    println!("\n=== Failure samples ===");
    for (ext, t) in &tallies {
        if t.failures.is_empty() {
            continue;
        }
        println!("\n.{ext}  ({} failures, showing up to {})", t.fail, FAILURE_SAMPLE_CAP);
        for (name, err) in &t.failures {
            let truncated = if err.len() > 200 { &err[..200] } else { err.as_str() };
            println!("  {:<32} {}", name, truncated);
        }
    }
}
