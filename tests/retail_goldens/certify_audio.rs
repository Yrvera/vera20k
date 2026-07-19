//! `certify_*` AUD chunk-walk + audio.bag corpus tests.

use super::*;

use vera20k::assets::aud_file::{decode_aud, parse_header};
use vera20k::assets::audio_bag::{AudioIndex, decode_bag_audio};

/// AUD chunk header: u16 compressed size + u16 output size + u32 magic.
const AUD_HEADER_SIZE: usize = 12;
const CHUNK_HEADER_SIZE: usize = 8;
const CHUNK_MAGIC: u32 = 0x0000_DEAF;

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_aud_chunk_walk() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let names = xcc_name_map();
    let mut failures: Vec<String> = Vec::new();
    let mut trailing_sample_files: Vec<String> = Vec::new();
    let mut total = 0usize;
    walk_sniffed(&am, |ce, data| {
        if ce.format != "aud" {
            return;
        }
        total += 1;
        let result = (|| -> Result<(), String> {
            let header = parse_header(data).ok_or("header too short")?;
            // Walk every chunk from the raw bytes: the walk must land exactly
            // on EOF and the summed chunk output sizes must equal the header's
            // declared output size — this certifies our chunk accounting
            // against the retail file's own layout.
            let mut pos = AUD_HEADER_SIZE;
            let mut summed_output: u64 = 0;
            let mut chunks: Vec<(usize, u64)> = Vec::new();
            while pos < data.len() {
                if pos + CHUNK_HEADER_SIZE > data.len() {
                    return Err(format!(
                        "truncated chunk header at {pos} (file {} bytes)",
                        data.len()
                    ));
                }
                let compressed = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
                let output = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as u64;
                let magic = u32::from_le_bytes([
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                if magic != CHUNK_MAGIC {
                    return Err(format!("bad chunk magic {magic:#010X} at {pos}"));
                }
                summed_output += output;
                chunks.push((compressed, output));
                pos += CHUNK_HEADER_SIZE + compressed;
            }
            if pos != data.len() {
                return Err(format!(
                    "chunk walk overran EOF: ended at {pos}, file is {} bytes",
                    data.len()
                ));
            }
            if summed_output != header.output_size as u64 {
                return Err(format!(
                    "summed chunk outputs {summed_output} != header output_size {}",
                    header.output_size
                ));
            }
            // Machine-derived retail invariant (2026-07-19 corpus pass): for
            // format-99 IMA, every chunk declares output == 4*compressed —
            // EXCEPT that in 4 unique retail files the FINAL chunk declares
            // exactly 4*compressed + 2 (one extra trailing sample the nibble
            // stream cannot produce). Any other pattern is a new finding.
            let mut input_derived_output: u64 = 0;
            for (i, (compressed, output)) in chunks.iter().enumerate() {
                let expected = *compressed as u64 * 4;
                if *output == expected {
                    input_derived_output += expected;
                } else if i == chunks.len() - 1 && *output == expected + 2 {
                    input_derived_output += expected;
                    let file = names
                        .get(&ce.id)
                        .cloned()
                        .unwrap_or_else(|| format!("{:#010X}", ce.id as u32));
                    trailing_sample_files.push(format!("{} {file}", ce.archive));
                } else {
                    return Err(format!(
                        "chunk {i}/{}: output {output} vs compressed {compressed} \
                         (neither 4*c nor final-chunk 4*c+2)",
                        chunks.len()
                    ));
                }
            }
            // Our decoder is input-driven: it must consume every nibble and
            // emit exactly 2 samples per input byte.
            let (_, samples) = decode_aud(data).ok_or("decode_aud returned None")?;
            if samples.len() as u64 * 2 != input_derived_output {
                return Err(format!(
                    "decoded {} bytes != 4 * total compressed bytes {input_derived_output}",
                    samples.len() * 2
                ));
            }
            Ok(())
        })();
        if let Err(msg) = result {
            failures.push(format!(
                "{} {:#010X} ({} bytes): {msg}",
                ce.archive, ce.id as u32, ce.size
            ));
        }
    });
    assert!(
        failures.is_empty(),
        "aud: {} of {total} retail files failed chunk accounting:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // Resolved 2026-07-19 (see docs/research/
    // AUD_TRAILING_SAMPLE_UNREACHABLE_GHIDRA_REPORT.md): the original engine
    // never decodes these four files (installer / RA2-era shell leftovers;
    // its music is WAV and its SFX come from audio.bag), so the declared
    // trailing sample is unobservable and our input-driven decoder needs no
    // change. Recorded here so a future consumer of these files knows.
    println!("RECORD: retail AUDs declaring a final trailing sample with no input nibbles:");
    for line in &trailing_sample_files {
        println!("RECORD:   {line}");
    }
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_bag_adpcm_block_invariants() {
    // Value-parity certification for bag IMA-ADPCM decode, block level.
    // The original engine's block decoder (see docs/research/
    // ADPCM_NIBBLE_VALUE_CERTIFICATION_GHIDRA_REPORT.md):
    // - REJECTS a block whose per-channel preamble has step_index > 88 or a
    //   nonzero reserved byte (our decoder clamps/continues instead) — so
    //   equivalence needs: no retail preamble is invalid;
    // - for MONO blocks rounds the nibble payload up to 4-byte groups,
    //   reading past an unaligned block end (ours decodes exact bytes) — so
    //   equivalence needs: every mono block payload is 4-byte aligned;
    // - for STEREO blocks truncates the payload to whole 8-byte L/R groups —
    //   identical to ours by construction.
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);
    let mut failures: Vec<String> = Vec::new();
    let mut ima_entries = 0usize;
    let mut stereo_entries = 0usize;
    for mix_name in ["AUDIOMD.MIX", "AUDIO.MIX"] {
        let Some(mix) = am.archive(mix_name) else {
            continue;
        };
        let (Some(idx_data), Some(bag_data)) = (
            mix.get_by_name("audio.idx"),
            mix.get_by_name("audio.bag").map(|d| d.to_vec()),
        ) else {
            continue;
        };
        let Some(index) = AudioIndex::from_idx_bag(idx_data, bag_data) else {
            continue;
        };
        let names: Vec<String> = index
            .names_with_prefix("")
            .into_iter()
            .map(str::to_string)
            .collect();
        for name in &names {
            let Some((entry, data)) = index.get(name) else {
                continue;
            };
            if !entry.is_ima_adpcm() {
                continue;
            }
            ima_entries += 1;
            let channels = entry.channels() as usize;
            if channels == 2 {
                stereo_entries += 1;
            }
            let preamble = channels * 4;
            let block_size = if entry.chunk_size > 0 {
                entry.chunk_size as usize
            } else {
                data.len()
            };
            let mut pos = 0usize;
            while pos + preamble <= data.len() {
                let block_end = (pos + block_size).min(data.len());
                for ch in 0..channels {
                    let step = data[pos + ch * 4 + 2];
                    let reserved = data[pos + ch * 4 + 3];
                    if step > 0x58 || reserved != 0 {
                        failures.push(format!(
                            "{mix_name} '{name}': block at {pos} ch {ch} preamble \
                             step {step} reserved {reserved} — native would reject"
                        ));
                    }
                }
                let payload = block_end - pos - preamble;
                if channels == 1 && payload % 4 != 0 {
                    failures.push(format!(
                        "{mix_name} '{name}': mono block at {pos} payload {payload} \
                         not 4-aligned — native reads past block end"
                    ));
                }
                if block_size == 0 {
                    break;
                }
                pos += block_size;
            }
        }
    }
    println!("RECORD: IMA-ADPCM bag entries: {ima_entries} ({stereo_entries} stereo)");
    assert!(
        failures.is_empty(),
        "bag ADPCM block invariants: {} violations:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore] // Requires RA2_DIR (retail game files)
fn certify_audio_bag_total() {
    let Some(root) = ra2_dir() else {
        println!("SKIP: set RA2_DIR to the retail install");
        return;
    };
    let am = load_corpus(&root);

    // Mirror the runtime lookup exactly (src/app_transitions.rs
    // load_audio_indices): each of AUDIOMD.MIX / AUDIO.MIX contains entries
    // named "audio.idx" / "audio.bag" internally.
    let mut failures: Vec<String> = Vec::new();
    let mut total_entries = 0usize;
    let mut mixes_found = 0usize;
    for mix_name in ["AUDIOMD.MIX", "AUDIO.MIX"] {
        let Some(mix) = am.archive(mix_name) else {
            failures.push(format!("{mix_name}: archive not loaded"));
            continue;
        };
        let Some(idx_data) = mix.get_by_name("audio.idx") else {
            failures.push(format!("{mix_name}: no audio.idx entry"));
            continue;
        };
        let Some(bag_data) = mix.get_by_name("audio.bag").map(|d| d.to_vec()) else {
            failures.push(format!("{mix_name}: no audio.bag entry"));
            continue;
        };
        let Some(index) = AudioIndex::from_idx_bag(idx_data, bag_data) else {
            failures.push(format!("{mix_name}: audio.idx failed to parse"));
            continue;
        };
        mixes_found += 1;
        total_entries += index.len();
        let names: Vec<String> = index
            .names_with_prefix("")
            .into_iter()
            .map(str::to_string)
            .collect();
        for name in &names {
            let Some((entry, data)) = index.get(name) else {
                failures.push(format!("{mix_name}: entry '{name}' failed lookup"));
                continue;
            };
            if decode_bag_audio(entry, data).is_none() {
                failures.push(format!(
                    "{mix_name}: entry '{name}' ({} bytes, flags {:#x}) failed to decode",
                    entry.size, entry.flags
                ));
            }
        }
    }
    assert_eq!(
        mixes_found,
        2,
        "expected both audio MIXes:\n{}",
        failures.join("\n")
    );
    assert!(
        failures.is_empty(),
        "audio.bag: {} failures across {total_entries} entries:\n{}",
        failures.len(),
        failures.join("\n")
    );

    if write_mode() {
        let mut m = read_manifest().unwrap_or_default();
        m.bag_aud = total_entries;
        write_manifest(&m);
        println!("manifest updated: bag_aud = {total_entries}");
        return;
    }
    let m = read_manifest().expect("manifest.json missing — run once with RETAIL_GOLDENS_WRITE=1");
    assert_eq!(
        m.bag_aud, total_entries,
        "CORPUS-DRIFT: audio.bag entry count changed"
    );
}
