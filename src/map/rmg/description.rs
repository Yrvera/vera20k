//! Fresh-file MapSeed Description decoding.
//!
//! Native identity: INIClass__ReadCommaHexUTF16 0x00528F00, called first on a
//! freshly loaded INI by MapSeed Load 0x00597A30 and metadata 0x00597D60.
//! See docs/research/skirmish-ui/2026-09-12-seed-description-reader.md.

const DESCRIPTION_UNITS: usize = 128;
const ENCODED_BYTES: usize = 0x4fff;
// On a section-pointer cache miss, 0x00529023 leaves this native CRC in the
// sscanf destination. An initial failed conversion therefore emits 0xB573.
const RANDOM_MAP_SECTION_CRC: u32 = 0x1597_b573;

/// Decode the visible UTF-16 units. Preserve raw units for native comparisons;
/// the existing options String converts unpaired surrogates lossily separately.
fn decode_units(raw: Option<&str>, default: &[u16]) -> Vec<u16> {
    let bytes = raw.unwrap_or_default().as_bytes();
    let bytes = &bytes[..bytes.len().min(ENCODED_BYTES)];
    let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    // strtrim at 0x0052909F removes bytes <= 0x20, unlike sscanf whitespace.
    let start = bytes.iter().position(|b| *b > 0x20).unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| *b > 0x20)
        .map_or(start, |i| i + 1);
    let bytes = &bytes[start..end];
    if bytes.is_empty() {
        return default
            .iter()
            .copied()
            .take(DESCRIPTION_UNITS)
            .take_while(|u| *u != 0)
            .collect();
    }
    let mut scratch = RANDOM_MAP_SECTION_CRC;
    let mut units = Vec::new();
    // strtok skips empty comma-delimited tokens. A whitespace-only token is
    // still a token and a failed conversion repeats the preceding value.
    for token in bytes
        .split(|b| *b == b',')
        .filter(|token| !token.is_empty())
        .take(DESCRIPTION_UNITS)
    {
        if let Some(value) = scan_hex(token) {
            scratch = value;
        }
        units.push(scratch as u16);
    }
    // The original scans up to its token count, appends NUL, then returns wcslen. Keep its
    // visible result without reproducing the caller's 129th-unit overwrite.
    units.truncate(units.iter().position(|u| *u == 0).unwrap_or(units.len()));
    units
}

fn scan_hex(token: &[u8]) -> Option<u32> {
    let start = token
        .iter()
        .position(|b| !matches!(*b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c))?;
    let mut bytes = &token[start..];
    let negative = bytes.first() == Some(&b'-');
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        bytes = &bytes[1..];
    }
    // CRT sscanf rejects a bare/invalid 0x prefix; parsing just its leading
    // zero would incorrectly replace the prior conversion with zero.
    if bytes.starts_with(b"0x") || bytes.starts_with(b"0X") {
        bytes = &bytes[2..];
    }
    let mut any = false;
    let mut value = 0u32;
    for byte in bytes {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => break,
        };
        any = true;
        value = value.wrapping_mul(16).wrapping_add(u32::from(digit));
    }
    any.then_some(if negative {
        value.wrapping_neg()
    } else {
        value
    })
}

pub(super) fn read_description(raw: Option<&str>, default: &str) -> String {
    String::from_utf16_lossy(&decode_units(
        raw,
        &default.encode_utf16().collect::<Vec<_>>(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_seed_descriptions_match_original_reader_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tools/storage_oracle/sed_description.json"
        ))
        .unwrap();
        let cases = vectors["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 29);
        let default: Vec<u16> = "DEFAULT".encode_utf16().collect();
        let mut compared = 0;
        for case in cases {
            if case["cached_section_pointer"].as_bool().unwrap() {
                assert_eq!(case["name"], "cached_section_diagnostic");
                continue;
            }
            assert_eq!(case["count"], 128);
            let expected: Vec<u16> = case["visible_units"]
                .as_array()
                .unwrap()
                .iter()
                .map(|unit| u16::try_from(unit.as_u64().unwrap()).unwrap())
                .collect();
            let raw = case["raw"].as_str();
            assert_eq!(decode_units(raw, &default), expected, "{}", case["name"]);
            assert_eq!(
                read_description(raw, "DEFAULT"),
                String::from_utf16_lossy(&expected)
            );
            compared += 1;
        }
        assert_eq!(compared, 28);
        assert_eq!(read_description(None, "Default map"), "Default map");
    }
}
