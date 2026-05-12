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
use vera20k::assets::hva_file::HvaFile;
use vera20k::assets::mix_archive::MixArchive;
use vera20k::assets::pal_file::Palette;
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

/// Detect the format of an asset by magic bytes / structural signatures.
///
/// Returns `None` for things we don't audit (nested MIX, INI text, BIK video,
/// VQA, audio bags, raw binary blobs we don't parse, etc.).
fn detect_format(data: &[u8]) -> Option<&'static str> {
    if data.len() < 4 {
        return None;
    }

    // 1. Strong magic matches (unique 4+ byte signatures).
    if &data[0..4] == b" FSC" {
        return Some("csf");
    }
    if &data[0..4] == b"fonT" {
        return Some("fnt");
    }
    if data.len() >= 16 && &data[0..16] == b"Voxel Animation\0" {
        return Some("vxl");
    }
    // BIK video — skip (separate handling).
    if &data[0..3] == b"BIK" {
        return None;
    }
    // VQA — skip.
    if &data[0..4] == b"FORM" {
        return None;
    }

    // 2. Nested MIX archive — skip (it's a container, not a leaf asset).
    if data[0] == 0 && data[1] == 0 && MixArchive::looks_like_mix(data) {
        return None;
    }

    // 3. INI / text — skip.
    if matches!(data[0], b'[' | b';') {
        return None;
    }
    // Plain ASCII letter at start with no nulls in first 32 bytes is probably text.
    if data[0].is_ascii_alphabetic() && data.iter().take(32).all(|b| *b != 0) {
        return None;
    }

    // 4. TMP: tile_width=60, tile_height=30 at offsets 8/12 (RA2 isometric).
    if data.len() >= 16 {
        let tile_w = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let tile_h = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        if tile_w == 60 && tile_h == 30 {
            return Some("tmp");
        }
    }

    // 5. SHP: bytes[0..2] = 0, reasonable width/height/frame_count.
    if data.len() >= 8 && data[0] == 0 && data[1] == 0 {
        let w = u16::from_le_bytes([data[2], data[3]]);
        let h = u16::from_le_bytes([data[4], data[5]]);
        let frames = u16::from_le_bytes([data[6], data[7]]);
        if (1..=2048).contains(&w)
            && (1..=2048).contains(&h)
            && (1..=10000).contains(&frames)
            && data.len() >= 8 + (frames as usize) * 24
        {
            return Some("shp");
        }
    }

    // 6. AUD: byte 11 is the format byte (1 = WS_ADPCM, 99 = IMA_ADPCM).
    //    Also require a reasonable sample rate.
    if data.len() >= 12 {
        let format_byte = data[11];
        let sample_rate = u16::from_le_bytes([data[0], data[1]]);
        if (format_byte == 1 || format_byte == 99)
            && matches!(sample_rate, 8000 | 11025 | 22050 | 44100 | 48000)
        {
            return Some("aud");
        }
    }

    // 7. VPL: header has first_remap, last_remap, num_sections; total size is
    //    16 + 768 (palette) + num_sections * 256.
    if data.len() >= 16 + 768 {
        let first_remap = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let last_remap = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let num_sections = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        if first_remap < 256
            && last_remap < 256
            && first_remap <= last_remap
            && (16..=512).contains(&num_sections)
            && data.len() == 16 + 768 + (num_sections as usize) * 256
        {
            return Some("vpl");
        }
    }

    // 8. PAL: exactly 768 bytes (256 RGB triplets, 6-bit values).
    if data.len() == 768 {
        return Some("pal");
    }

    // 9. HVA: no magic, but offset 16 = frame_count, offset 20 = section_count.
    //    Total size = 24 + section_count*16 + frame_count*section_count*48.
    if data.len() >= 24 {
        let frame_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let section_count = u32::from_le_bytes([data[20], data[21], data[22], data[23]]) as usize;
        if frame_count > 0
            && (1..=200).contains(&section_count)
            && frame_count <= 1000
            && data.len() == 24 + section_count * 16 + frame_count * section_count * 48
        {
            return Some("hva");
        }
    }

    None
}

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
