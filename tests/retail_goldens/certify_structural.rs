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

/// Shared skeleton for the per-format structural certifies: walk the corpus
/// filtered to one format, collect per-file failure strings, assert none.
fn certify_format(format: &str, mut check: impl FnMut(&CorpusEntry, &[u8]) -> Result<(), String>) {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    walk_sniffed(&am, |ce, data| {
        if ce.format != format {
            return;
        }
        total += 1;
        if let Err(msg) = check(ce, data) {
            failures.push(format!(
                "{} {:#010X} ({} bytes): {msg}",
                ce.archive, ce.id as u32, ce.size
            ));
        }
    });
    assert!(
        failures.is_empty(),
        "{}: {} of {} retail files violated structural invariants:\n{}",
        format,
        failures.len(),
        total,
        failures.join("\n")
    );
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_shp_structural() {
    certify_format("shp", |_, data| {
        let shp = ShpFile::from_bytes(data).map_err(|e| e.to_string())?;
        let raw_frame_count = u16::from_le_bytes([data[6], data[7]]) as usize;
        if shp.frames.len() != raw_frame_count {
            return Err(format!(
                "frames.len() {} != raw header frame_count {raw_frame_count}",
                shp.frames.len()
            ));
        }
        for (i, fr) in shp.frames.iter().enumerate() {
            let expected = fr.frame_width as usize * fr.frame_height as usize;
            if fr.pixels.len() != expected {
                return Err(format!(
                    "frame {i}: pixels.len() {} != {}x{}",
                    fr.pixels.len(),
                    fr.frame_width,
                    fr.frame_height
                ));
            }
            if fr.frame_x as u32 + fr.frame_width as u32 > shp.width as u32
                || fr.frame_y as u32 + fr.frame_height as u32 > shp.height as u32
            {
                return Err(format!(
                    "frame {i}: bounds ({},{} {}x{}) exceed file dims {}x{}",
                    fr.frame_x, fr.frame_y, fr.frame_width, fr.frame_height, shp.width, shp.height
                ));
            }
        }
        Ok(())
    });
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_tmp_structural() {
    certify_format("tmp", |_, data| {
        let tmp = TmpFile::from_bytes(data).map_err(|e| e.to_string())?;
        let raw_w = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let raw_h = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if tmp.template_width != raw_w || tmp.template_height != raw_h {
            return Err(format!(
                "template dims {}x{} != raw header {raw_w}x{raw_h}",
                tmp.template_width, tmp.template_height
            ));
        }
        if tmp.tiles.len() != (raw_w * raw_h) as usize {
            return Err(format!(
                "tiles.len() {} != template {raw_w}x{raw_h}",
                tmp.tiles.len()
            ));
        }
        if tmp.tile_width != 60 || tmp.tile_height != 30 {
            return Err(format!(
                "tile size {}x{} != RA2 isometric 60x30",
                tmp.tile_width, tmp.tile_height
            ));
        }
        for (i, tile) in tmp.tiles.iter().enumerate() {
            let Some(tile) = tile else { continue };
            let expected = (tile.pixel_width * tile.pixel_height) as usize;
            if tile.pixels.len() != expected {
                return Err(format!(
                    "tile {i}: pixels.len() {} != {}x{}",
                    tile.pixels.len(),
                    tile.pixel_width,
                    tile.pixel_height
                ));
            }
            if tile.depth.len() != tile.pixels.len() {
                return Err(format!(
                    "tile {i}: depth.len() {} != pixels.len() {}",
                    tile.depth.len(),
                    tile.pixels.len()
                ));
            }
        }
        Ok(())
    });
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_vxl_structural() {
    // Normal-table sizes per src/render/vxl_normals.rs: mode 2 = TS table (36
    // entries), mode 4 = RA2 table (250 distinct entries; 250–255 stale).
    //
    // Machine-derived retail invariant (full-corpus evidence pass 2026-07-19):
    // every retail limb is mode 2 or 4; every voxel's normal_index is either
    // inside its mode's table OR exactly 255. Index 255 appears ONLY in
    // placeholder limbs named DUMMY01/DUMMY02 (1,931 voxels across 8 files);
    // 250–254 and 36–254 (mode 2) never occur. A new index in that gap is a
    // real finding — investigate, don't widen the exception.
    let names = xcc_name_map();
    let mut dummy_255: Vec<String> = Vec::new();
    certify_format("vxl", |ce, data| {
        let vxl = VxlFile::from_bytes(data).map_err(|e| e.to_string())?;
        if vxl.limbs.len() != vxl.limb_count as usize {
            return Err(format!(
                "limbs.len() {} != header limb_count {}",
                vxl.limbs.len(),
                vxl.limb_count
            ));
        }
        if vxl.palette.len() != 256 {
            return Err(format!("palette.len() {} != 256", vxl.palette.len()));
        }
        for limb in &vxl.limbs {
            let table_size = match limb.normals_mode {
                2 => 36u8,
                4 => 250u8,
                m => return Err(format!("limb '{}': unexpected normals mode {m}", limb.name)),
            };
            let mut saw_255 = 0usize;
            for v in &limb.voxels {
                if v.x >= limb.size_x || v.y >= limb.size_y || v.z >= limb.size_z {
                    return Err(format!(
                        "limb '{}': voxel ({},{},{}) outside grid {}x{}x{}",
                        limb.name, v.x, v.y, v.z, limb.size_x, limb.size_y, limb.size_z
                    ));
                }
                if v.normal_index == 255 {
                    saw_255 += 1;
                } else if v.normal_index >= table_size {
                    return Err(format!(
                        "limb '{}' (normals mode {}): normal_index {} in the \
                         never-observed gap {table_size}..255",
                        limb.name, limb.normals_mode, v.normal_index
                    ));
                }
            }
            if saw_255 > 0 {
                let file = names
                    .get(&ce.id)
                    .cloned()
                    .unwrap_or_else(|| format!("{:#010X}", ce.id as u32));
                dummy_255.push(format!(
                    "{} {file}: limb '{}' mode {} has {saw_255} voxels with index 255",
                    ce.archive, limb.name, limb.normals_mode
                ));
            }
        }
        Ok(())
    });
    println!("RECORD: retail voxels referencing stale normal index 255:");
    for line in &dummy_255 {
        println!("RECORD:   {line}");
    }
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_hva_structural() {
    certify_format("hva", |_, data| {
        let hva = HvaFile::from_bytes(data).map_err(|e| e.to_string())?;
        let raw_frames = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        let raw_sections = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        if hva.frame_count != raw_frames || hva.section_count != raw_sections {
            return Err(format!(
                "counts {}f/{}s != raw header {raw_frames}f/{raw_sections}s",
                hva.frame_count, hva.section_count
            ));
        }
        if hva.section_names.len() != raw_sections as usize {
            return Err(format!(
                "section_names.len() {} != {raw_sections}",
                hva.section_names.len()
            ));
        }
        if hva.transforms.len() != (raw_frames * raw_sections) as usize {
            return Err(format!(
                "transforms.len() {} != {raw_frames}*{raw_sections}",
                hva.transforms.len()
            ));
        }
        let expected_size =
            24 + raw_sections as usize * 16 + raw_frames as usize * raw_sections as usize * 48;
        if data.len() != expected_size {
            return Err(format!(
                "file size {} != exact formula {expected_size}",
                data.len()
            ));
        }
        Ok(())
    });
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_csf_structural() {
    let mut languages: Vec<u16> = Vec::new();
    certify_format("csf", |_, data| {
        let csf = CsfFile::from_bytes(data).map_err(|e| e.to_string())?;
        if csf.version != 3 {
            return Err(format!("version {} != 3", csf.version));
        }
        languages.push(csf.language);
        // Re-walk the raw label records independently to count total labels
        // and duplicate names (the parser's HashMap keeps one entry per name).
        let raw_label_count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut walked = 0usize;
        let mut pos = 24usize;
        while walked < raw_label_count {
            let (name, next) = walk_raw_csf_label(data, pos)
                .map_err(|e| format!("raw label walk failed at record {walked} (offset {pos}): {e}"))?;
            names.insert(name);
            pos = next;
            walked += 1;
        }
        if pos != data.len() {
            return Err(format!(
                "raw walk ended at {pos}, file is {} bytes (trailing garbage)",
                data.len()
            ));
        }
        if csf.len() != names.len() {
            return Err(format!(
                "parsed entries {} != {} unique raw labels ({raw_label_count} total records)",
                csf.len(),
                names.len()
            ));
        }
        Ok(())
    });
    println!("RECORD: retail CSF language field values: {languages:?}");
}

/// Walk one raw CSF label record. Returns (uppercased name, next offset).
/// Mirrors the on-disk format: " LBL" magic, u32 pair count, u32 name length,
/// name bytes, then per pair a string record (magic, u32 char count, chars×2
/// bytes, plus a length-prefixed extra blob for the "W" variants).
fn walk_raw_csf_label(data: &[u8], offset: usize) -> Result<(String, usize), String> {
    let rd = |off: usize| -> Result<u32, String> {
        data.get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| format!("read past EOF at {off}"))
    };
    let magic = rd(offset)?;
    // " LBL" and the alt spelling "LBL " (see src/assets/csf_file.rs).
    if magic != 0x4C42_4C20 && magic != 0x204C_424C {
        return Err(format!("bad label magic {magic:#010X}"));
    }
    let pair_count = rd(offset + 4)?;
    let name_len = rd(offset + 8)? as usize;
    let name_start = offset + 12;
    let name = data
        .get(name_start..name_start + name_len)
        .map(|b| String::from_utf8_lossy(b).to_ascii_uppercase())
        .ok_or("name past EOF")?;
    let mut pos = name_start + name_len;
    for _ in 0..pair_count {
        let str_magic = rd(pos)?;
        // " RTS"/" STR"/"STR " plain, "STRW"/"WRTS" with extra blob.
        let has_extra = str_magic == 0x5752_5453 || str_magic == 0x5354_5257;
        let is_plain =
            str_magic == 0x5354_5220 || str_magic == 0x5254_5320 || str_magic == 0x2052_5453;
        if !is_plain && !has_extra {
            return Err(format!("bad string magic {str_magic:#010X} at {pos}"));
        }
        let char_count = rd(pos + 4)? as usize;
        pos += 8 + char_count * 2;
        if has_extra {
            let extra_len = rd(pos)? as usize;
            pos += 4 + extra_len;
        }
        if pos > data.len() {
            return Err(format!("string data past EOF at {pos}"));
        }
    }
    Ok((name, pos))
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_fnt_structural() {
    certify_format("fnt", |_, data| {
        let fnt = FntFile::from_bytes(data).map_err(|e| e.to_string())?;
        let expected_stride = 1 + fnt.bytes_per_row * fnt.bitmap_rows;
        if fnt.glyph_stride != expected_stride {
            return Err(format!(
                "glyph_stride {} != 1 + {}*{}",
                fnt.glyph_stride, fnt.bytes_per_row, fnt.bitmap_rows
            ));
        }
        let max_width = fnt.bytes_per_row * 8;
        for cp in 0u16..=u16::MAX {
            let Some(g) = fnt.glyph(cp) else { continue };
            if g.width > max_width {
                return Err(format!(
                    "glyph {cp:#06X}: width {} > row capacity {max_width}",
                    g.width
                ));
            }
            let expected_rgba = (g.width * fnt.bitmap_rows * 4) as usize;
            if g.rgba.len() != expected_rgba {
                return Err(format!(
                    "glyph {cp:#06X}: rgba.len() {} != {expected_rgba}",
                    g.rgba.len()
                ));
            }
        }
        Ok(())
    });
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_pcx_structural() {
    certify_format("pcx", |_, data| {
        let pcx = PcxFile::from_bytes(data).map_err(|e| e.to_string())?;
        let x_min = u16::from_le_bytes([data[4], data[5]]);
        let y_min = u16::from_le_bytes([data[6], data[7]]);
        let x_max = u16::from_le_bytes([data[8], data[9]]);
        let y_max = u16::from_le_bytes([data[10], data[11]]);
        if x_min > x_max || y_min > y_max {
            return Err(format!(
                "header bounds inverted: x {x_min}..{x_max}, y {y_min}..{y_max}"
            ));
        }
        if pcx.width != x_max - x_min + 1 || pcx.height != y_max - y_min + 1 {
            return Err(format!(
                "dims {}x{} != header bounds {}x{}",
                pcx.width,
                pcx.height,
                x_max - x_min + 1,
                y_max - y_min + 1
            ));
        }
        // Plane count from the raw header decides the pixel representation:
        // 1 plane = palette indices (w*h), 3 planes = interleaved RGB (w*h*3).
        let planes = data[65];
        let expected = match planes {
            1 => pcx.width as usize * pcx.height as usize,
            3 => pcx.width as usize * pcx.height as usize * 3,
            p => return Err(format!("unexpected plane count {p}")),
        };
        if pcx.pixels.len() != expected {
            return Err(format!(
                "pixels.len() {} != expected {expected} ({planes} plane(s))",
                pcx.pixels.len()
            ));
        }
        Ok(())
    });
}
