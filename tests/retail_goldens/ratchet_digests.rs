//! `ratchet_*` decode-output digest tests — regression ratchets ONLY, never
//! parity evidence (Rust-vs-prior-Rust comparison). A digest mismatch means
//! "our decoder's output changed", not "the output is wrong": if the change is
//! an intended decoder fix, re-baseline with RETAIL_GOLDENS_WRITE=1 (one
//! session at a time, reason in the commit).

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

/// Digest one file's DECODED output (not its raw bytes) with FNV-1a.
///
/// Every fold is length- or field-ordered deterministically so the digest is
/// stable across runs and platforms. CSF pairs are sorted by key first (the
/// parser stores them in a HashMap, whose iteration order is not stable).
fn digest_decode(format: &str, data: &[u8], mut h: u64) -> Result<u64, String> {
    match format {
        "shp" => {
            let shp = ShpFile::from_bytes(data).map_err(|e| e.to_string())?;
            for fr in &shp.frames {
                for v in [fr.frame_x, fr.frame_y, fr.frame_width, fr.frame_height] {
                    h = fnv1a(&v.to_le_bytes(), h);
                }
                h = fnv1a(&fr.pixels, h);
            }
        }
        "tmp" => {
            let tmp = TmpFile::from_bytes(data).map_err(|e| e.to_string())?;
            for tile in tmp.tiles.iter().flatten() {
                for v in [tile.pixel_width, tile.pixel_height] {
                    h = fnv1a(&v.to_le_bytes(), h);
                }
                for v in [tile.offset_x, tile.offset_y] {
                    h = fnv1a(&v.to_le_bytes(), h);
                }
                h = fnv1a(&tile.pixels, h);
                h = fnv1a(&tile.depth, h);
            }
        }
        "vxl" => {
            let vxl = VxlFile::from_bytes(data).map_err(|e| e.to_string())?;
            for limb in &vxl.limbs {
                for v in &limb.voxels {
                    h = fnv1a(&[v.x, v.y, v.z, v.color_index, v.normal_index], h);
                }
            }
        }
        "hva" => {
            let hva = HvaFile::from_bytes(data).map_err(|e| e.to_string())?;
            for matrix in &hva.transforms {
                for f in matrix {
                    h = fnv1a(&f.to_le_bytes(), h);
                }
            }
        }
        "pal" => {
            let pal = Palette::from_bytes(data).map_err(|e| e.to_string())?;
            h = fnv1a(&pal.to_rgba_bytes(), h);
        }
        "csf" => {
            let csf = CsfFile::from_bytes(data).map_err(|e| e.to_string())?;
            let mut pairs: Vec<(&str, &str)> = csf.entries().collect();
            pairs.sort_unstable();
            for (k, v) in pairs {
                h = fnv1a(&(k.len() as u32).to_le_bytes(), h);
                h = fnv1a(k.as_bytes(), h);
                h = fnv1a(&(v.len() as u32).to_le_bytes(), h);
                h = fnv1a(v.as_bytes(), h);
            }
        }
        "aud" => {
            let (_, samples) = decode_aud(data).ok_or("decode_aud returned None")?;
            for s in samples {
                h = fnv1a(&s.to_le_bytes(), h);
            }
        }
        "fnt" => {
            let fnt = FntFile::from_bytes(data).map_err(|e| e.to_string())?;
            for cp in 0u16..=u16::MAX {
                let Some(g) = fnt.glyph(cp) else { continue };
                h = fnv1a(&cp.to_le_bytes(), h);
                h = fnv1a(&g.width.to_le_bytes(), h);
                h = fnv1a(&g.rgba, h);
            }
        }
        "pcx" => {
            let pcx = PcxFile::from_bytes(data).map_err(|e| e.to_string())?;
            for v in [pcx.width, pcx.height] {
                h = fnv1a(&v.to_le_bytes(), h);
            }
            h = fnv1a(&pcx.pixels, h);
            for rgb in &pcx.palette {
                h = fnv1a(rgb, h);
            }
        }
        "vpl" => {
            let vpl = VplFile::from_bytes(data).map_err(|e| e.to_string())?;
            for page in vpl.pages_slice() {
                h = fnv1a(page, h);
            }
        }
        other => return Err(format!("no digest rule for format '{other}'")),
    }
    Ok(h)
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn ratchet_decode_digests() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let mut rollups: BTreeMap<String, u64> = BTreeMap::new();
    let mut errors: Vec<String> = Vec::new();
    walk_sniffed(&am, |ce, data| {
        let h = rollups
            .entry(ce.format.to_string())
            .or_insert(FNV_OFFSET);
        // Fold the entry id first so a same-bytes file moving between ids is
        // visible, then the decoded output.
        *h = fnv1a(&(ce.id as u32).to_le_bytes(), *h);
        match digest_decode(ce.format, data, *h) {
            Ok(next) => *h = next,
            Err(msg) => errors.push(format!(
                "{} {:#010X}: {msg}",
                ce.archive, ce.id as u32
            )),
        }
    });
    assert!(errors.is_empty(), "decode errors during digest:\n{}", errors.join("\n"));

    if write_mode() {
        let mut m = read_manifest().unwrap_or_default();
        m.decode_rollups = rollups;
        write_manifest(&m);
        println!("manifest updated: decode_rollups ({} formats)", m.decode_rollups.len());
        return;
    }
    let m = read_manifest().expect("manifest.json missing — run once with RETAIL_GOLDENS_WRITE=1");
    for (fmt, digest) in &rollups {
        let stored = m.decode_rollups.get(fmt);
        assert_eq!(
            stored,
            Some(digest),
            "RATCHET: {fmt} decode output changed (stored {stored:?}, computed {digest:#018x}). \
             If this is an intended decoder fix, re-baseline with RETAIL_GOLDENS_WRITE=1 \
             (one session at a time, reason in commit). Ratchet digests are NOT parity evidence."
        );
    }
    assert_eq!(
        m.decode_rollups.len(),
        rollups.len(),
        "RATCHET: stored format set differs from computed set"
    );
}

/// Curated player-facing assets, resolved by name so a ratchet failure is
/// immediately legible. Every name was verified to resolve on the reference
/// install when the list was authored.
const NAMED_FILES: &[(&str, &str)] = &[
    ("shp", "gtnkicon.shp"),
    ("pal", "cameo.pal"),
    ("pal", "unittem.pal"),
    ("pal", "isotem.pal"),
    ("csf", "ra2md.csf"),
    ("vxl", "bus.vxl"),
    ("hva", "bus.hva"),
    ("tmp", "clear01.tem"),
    ("tmp", "clear01.sno"),
    ("aud", "intro.aud"),
];

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn ratchet_named_file_digests() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let mut rows: Vec<(String, String, u64)> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for (fmt, name) in NAMED_FILES {
        let Some((data, source)) = am.get_with_source_ref(name) else {
            failures.push(format!("{name}: does not resolve by name"));
            continue;
        };
        match digest_decode(fmt, data, FNV_OFFSET) {
            Ok(h) => {
                println!("RECORD: {name} -> {source}: {h:#018x}");
                rows.push((fmt.to_string(), name.to_string(), h));
            }
            Err(msg) => failures.push(format!("{name}: {msg}")),
        }
    }
    assert!(
        failures.is_empty(),
        "named ratchet files: {} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );

    if write_mode() {
        let mut m = read_manifest().unwrap_or_default();
        m.files = rows;
        write_manifest(&m);
        println!("manifest updated: {} named file digests", m.files.len());
        return;
    }
    let m = read_manifest().expect("manifest.json missing — run once with RETAIL_GOLDENS_WRITE=1");
    assert_eq!(
        m.files, rows,
        "RATCHET: a named file's decode digest changed. If this is an intended \
         decoder fix, re-baseline with RETAIL_GOLDENS_WRITE=1 (one session at a \
         time, reason in commit). Ratchet digests are NOT parity evidence."
    );
}
