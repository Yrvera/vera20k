//! `certify_*` tests: parse-total + per-format internal consistency vs raw
//! retail header bytes.

use super::*;

use vera20k::assets::aud_file::decode_aud;
use vera20k::assets::csf_file::CsfFile;
use vera20k::assets::fnt_file::FntFile;
use vera20k::assets::hva_file::HvaFile;
use vera20k::assets::pal_file::Palette;
use vera20k::assets::pcx_file::PcxFile;
use vera20k::assets::shp_file::ShpFile;
use vera20k::assets::tmp_file::TmpFile;
use vera20k::assets::vpl_file::VplFile;
use vera20k::assets::vxl_file::VxlFile;

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_corpus_manifest() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);

    let mut archives: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let mut format_counts: BTreeMap<String, usize> = BTreeMap::new();
    am.visit_archives(|arch_name, archive| {
        let mut h = FNV_OFFSET;
        for e in archive.entries() {
            h = fnv1a(&(e.id as u32).to_le_bytes(), h);
            h = fnv1a(&e.size.to_le_bytes(), h);
        }
        archives.insert(arch_name.to_string(), (archive.entry_count(), h));
    });
    walk_sniffed(&am, |ce, _| {
        *format_counts.entry(ce.format.to_string()).or_default() += 1;
    });

    if write_mode() {
        let mut m = read_manifest().unwrap_or_default();
        m.schema = 1;
        m.archives = archives;
        m.format_counts = format_counts;
        write_manifest(&m);
        println!("manifest written: {MANIFEST_PATH}");
        return;
    }

    let m =
        read_manifest().expect("manifest.json missing — run once with RETAIL_GOLDENS_WRITE=1");
    assert_eq!(
        m.archives, archives,
        "CORPUS-DRIFT: the install's archive set/index differs from the committed \
         manifest. This is an install change, not a parser failure. If intentional, \
         regenerate with RETAIL_GOLDENS_WRITE=1 (golden re-baseline discipline: one \
         session at a time, note the reason in the commit)."
    );
    assert_eq!(
        m.format_counts, format_counts,
        "CORPUS-DRIFT: sniffed format counts changed"
    );
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_parse_total_zero_failures() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    walk_sniffed(&am, |ce, data| {
        total += 1;
        let outcome: Result<(), String> = match ce.format {
            "shp" => ShpFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "vxl" => VxlFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "hva" => HvaFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "tmp" => TmpFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "pal" => Palette::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "csf" => CsfFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "vpl" => VplFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "fnt" => FntFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "pcx" => PcxFile::from_bytes(data).map(|_| ()).map_err(|e| e.to_string()),
            "aud" => decode_aud(data)
                .map(|_| ())
                .ok_or_else(|| "decode_aud returned None".to_string()),
            _ => Ok(()),
        };
        if let Err(msg) = outcome {
            failures.push(format!(
                "{} {:#010X} ({} bytes): {msg}",
                ce.archive, ce.id as u32, ce.size
            ));
        }
    });
    // Baseline 2026-07-19: 8,824/8,824 pass (incl. 271 pcx). Empty allowlist is
    // intentional — a new failure is a real finding to investigate, never to
    // allowlist blind.
    assert!(
        failures.is_empty(),
        "{} of {} retail files failed to parse:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}
