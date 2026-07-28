//! Coverage auditor for the project's asset parsers.
//!
//! Walks every entry of every loaded MIX archive, identifies each entry's
//! format by magic-byte sniffing (no dependency on XCC's filename database),
//! runs the matching parser, and tallies pass/fail per format.
//!
//! "Passed" here means `from_bytes` returned Ok — structural validity only.
//! Semantic correctness vs. gamemd is a separate question (see `/re-investigate`).
//!
//! Run with: `cargo run --release --bin audit-assets`

use std::collections::BTreeMap;
use std::path::Path;

use vera20k::assets::asset_manager::AssetManager;
use vera20k::assets::aud_file::decode_aud;
use vera20k::assets::csf_file::CsfFile;
use vera20k::assets::fnt_file::FntFile;
use vera20k::assets::format_sniff::detect_format;
use vera20k::assets::hva_file::HvaFile;
use vera20k::assets::pal_file::Palette;
use vera20k::assets::pcx_file::PcxFile;
use vera20k::assets::shp_file::ShpFile;
use vera20k::assets::tmp_file::TmpFile;
use vera20k::assets::vpl_file::VplFile;
use vera20k::assets::vxl_file::VxlFile;

#[derive(Default)]
struct ExtTally {
    ok: u32,
    fail: u32,
    failures: Vec<(String, String, usize)>, // (archive, hash_hex, size)
    failure_msgs: Vec<String>,
    total_bytes: u64,
}

const FAILURE_SAMPLE_CAP: usize = 8;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let ra2_dir = Path::new("C:/Users/enok/Documents/Command and Conquer Red Alert II/");

    println!("Loading AssetManager from {} ...", ra2_dir.display());
    let mut am = AssetManager::new(ra2_dir).expect("AssetManager init");
    let extra = am.load_all_disk_mixes().unwrap_or(0);
    println!("Loaded {} extra disk MIX(es).", extra);

    let mut tallies: BTreeMap<&'static str, ExtTally> = BTreeMap::new();
    let mut total_entries: u64 = 0;
    let mut skipped_entries: u64 = 0;
    let mut skipped_bytes: u64 = 0;

    am.visit_archives(|arch_name, archive| {
        for entry in archive.entries() {
            total_entries += 1;
            let Some(data) = archive.get_by_id(entry.id) else {
                continue;
            };
            let Some(fmt) = detect_format(data) else {
                skipped_entries += 1;
                skipped_bytes += data.len() as u64;
                continue;
            };

            let outcome: Result<(), String> = match fmt {
                "shp" => ShpFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "vxl" => VxlFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "hva" => HvaFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "tmp" => TmpFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "pal" => Palette::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "csf" => CsfFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "vpl" => VplFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "fnt" => FntFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "pcx" => PcxFile::from_bytes(data)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "aud" => match decode_aud(data) {
                    Some(_) => Ok(()),
                    None => Err("decode_aud returned None".to_string()),
                },
                _ => continue,
            };

            let tally = tallies.entry(fmt).or_default();
            tally.total_bytes += data.len() as u64;
            match outcome {
                Ok(()) => tally.ok += 1,
                Err(msg) => {
                    tally.fail += 1;
                    if tally.failures.len() < FAILURE_SAMPLE_CAP {
                        tally.failures.push((
                            arch_name.to_string(),
                            format!("{:#010X}", entry.id as u32),
                            data.len(),
                        ));
                        tally.failure_msgs.push(msg);
                    }
                }
            }
        }
    });

    println!("\n=== Per-format coverage (magic-byte sniffed) ===");
    println!(
        "{:<6}  {:>8}  {:>8}  {:>9}  {:>10}",
        "ext", "ok", "fail", "ok %", "MB"
    );
    let mut grand_ok: u32 = 0;
    let mut grand_fail: u32 = 0;
    for (fmt, t) in &tallies {
        let total = t.ok + t.fail;
        let pct = if total > 0 {
            100.0 * (t.ok as f64) / (total as f64)
        } else {
            0.0
        };
        println!(
            ".{:<5}  {:>8}  {:>8}  {:>8.2}%  {:>10.2}",
            fmt,
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
        "Total MIX entries seen: {}  |  skipped (unrecognized/text/MIX/video): {} ({:.1} MB)",
        total_entries,
        skipped_entries,
        skipped_bytes as f64 / (1024.0 * 1024.0)
    );

    println!("\n=== Failure samples ===");
    for (fmt, t) in &tallies {
        if t.failures.is_empty() {
            continue;
        }
        println!(
            "\n.{} ({} failures, showing up to {})",
            fmt, t.fail, FAILURE_SAMPLE_CAP
        );
        for ((arch, hash, size), msg) in t.failures.iter().zip(t.failure_msgs.iter()) {
            let truncated = if msg.len() > 200 {
                &msg[..200]
            } else {
                msg.as_str()
            };
            println!("  {arch:<32} hash={hash} size={size:>7}  {truncated}");
        }
    }
}
