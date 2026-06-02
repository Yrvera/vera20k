//! In-place INI value writer.
//!
//! Updates (or inserts) a single `key=value` under a named `[section]` in the
//! raw bytes of an INI file, preserving every other byte exactly: comments,
//! key order, casing, blank lines, and the existing line-ending style all
//! round-trip untouched. This mirrors the original game writing individual
//! settings keys in place rather than rewriting the whole file — a naive full
//! rewrite would discard sections the engine does not yet model (e.g.
//! `[Skirmish]`, `[MultiPlayer]`).
//!
//! Operates on raw bytes so content that is not valid UTF-8 elsewhere in the
//! file (e.g. a player name with high-byte characters in another section)
//! round-trips verbatim and is never matched.
//!
//! ## Dependency rules
//! - Part of util/ — no dependencies on game modules.

/// Update (or insert) `[section] key=value` in `content`, returning the new
/// file bytes.
///
/// Matching is case-insensitive on the section and key names (as INI requires);
/// the names are written using the casing passed in. When the key already
/// exists in the section, only its value is replaced and the line's terminator
/// is kept. When the section exists but the key does not, the key is appended
/// to the end of that section. When the section is absent, a new section is
/// appended at the end of the file. An empty input yields a fresh section.
/// Lines that are not valid UTF-8 are passed through verbatim, never matched.
pub fn set_ini_value(content: &[u8], section: &str, key: &str, value: &str) -> Vec<u8> {
    let target_section = section.trim();
    let target_key = key.trim();
    let new_line = format!("{key}={value}");
    let lines = split_lines(content);

    // Locate the FIRST matching section block, the key within it (if present),
    // and where a missing key would be appended (the end of that block's keys).
    // Win32's WritePrivateProfileString operates on the first matching section
    // span, so any later duplicate `[section]` blocks are ignored for both the
    // lookup and the insert position.
    let mut section_found = false;
    let mut key_idx: Option<usize> = None;
    let mut insert_after: Option<usize> = None;
    let mut in_first_block = false;
    for (idx, (text, _)) in lines.iter().enumerate() {
        let Ok(line) = std::str::from_utf8(text) else {
            continue; // non-UTF-8: never a header/key, leave untouched
        };
        if let Some(name) = section_header_name(line) {
            if !section_found && name.eq_ignore_ascii_case(target_section) {
                section_found = true;
                in_first_block = true;
                insert_after = Some(idx);
            } else {
                // Any later header (including a duplicate target) ends the block.
                in_first_block = false;
            }
            continue;
        }
        if in_first_block {
            if let Some(k) = key_name(line) {
                if key_idx.is_none() && k.eq_ignore_ascii_case(target_key) {
                    key_idx = Some(idx);
                }
                insert_after = Some(idx);
            }
        }
    }

    let mut out = Vec::with_capacity(content.len() + new_line.len() + CRLF.len() + 4);
    for (idx, (text, terminator)) in lines.iter().enumerate() {
        if Some(idx) == key_idx {
            // Replace the value in place, keeping this line's own terminator.
            out.extend_from_slice(new_line.as_bytes());
            out.extend_from_slice(terminator);
            continue;
        }
        out.extend_from_slice(text);
        out.extend_from_slice(terminator);
        if key_idx.is_none() && section_found && Some(idx) == insert_after {
            // Section exists but the key does not — append it here. If the
            // anchor line was the unterminated last line, terminate it first.
            if terminator.is_empty() {
                out.extend_from_slice(CRLF);
            }
            out.extend_from_slice(new_line.as_bytes());
            out.extend_from_slice(CRLF);
        }
    }

    if !section_found {
        if !out.is_empty() && !out.ends_with(b"\n") {
            out.extend_from_slice(CRLF);
        }
        out.extend_from_slice(format!("[{section}]").as_bytes());
        out.extend_from_slice(CRLF);
        out.extend_from_slice(new_line.as_bytes());
        out.extend_from_slice(CRLF);
    }

    out
}

/// Terminator for lines this writer *adds* (an inserted key or a created
/// section): always CRLF, the convention the original game's settings writer
/// (Win32 `WritePrivateProfileString`) emits regardless of the file's existing
/// style. Lines that already exist keep their own terminator on a value
/// replace.
const CRLF: &[u8] = b"\r\n";

/// Split `content` into `(text, terminator)` pairs, one per line, where `text`
/// excludes the line ending and `terminator` is the exact ending bytes
/// (`"\r\n"`, `"\n"`, or empty for a final line with no trailing newline).
fn split_lines(content: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < content.len() {
        if content[i] == b'\n' {
            let has_cr = i > start && content[i - 1] == b'\r';
            let text_end = if has_cr { i - 1 } else { i };
            lines.push((&content[start..text_end], &content[text_end..=i]));
            start = i + 1;
        }
        i += 1;
    }
    if start < content.len() {
        lines.push((&content[start..], &content[content.len()..]));
    }
    lines
}

/// The section name inside `[...]`, trimmed, or `None` if `line` is not a clean
/// section header.
fn section_header_name(line: &str) -> Option<&str> {
    let t = line.trim();
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

/// The key name before `=`, trimmed, or `None` if `line` is blank, a comment,
/// a section header, or has no `=`.
fn key_name(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let eq = t.find('=')?;
    let k = t[..eq].trim();
    if k.is_empty() { None } else { Some(k) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
    }

    /// Real RA2MD.INI shape: replacing ScoreVolume keeps every sibling key and
    /// every other section byte-for-byte, drops the old value, and keeps CRLF.
    #[test]
    fn replaces_value_preserving_other_keys_and_sections() {
        let input = b"[Options]\r\nGameSpeed=3\r\n[Audio]\r\nSoundVolume=0.700000\r\n\
ScoreVolume=0.600000\r\nInGameMusic=yes\r\n[Network]\r\nNetID=ffff,ffff,ffff,\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.250000"));
        assert!(out.contains("ScoreVolume=0.250000\r\n"));
        assert!(out.contains("SoundVolume=0.700000\r\n"));
        assert!(out.contains("InGameMusic=yes\r\n"));
        assert!(out.contains("[Network]\r\nNetID=ffff,ffff,ffff,\r\n"));
        assert!(!out.contains("0.600000"), "old value must be gone");
    }

    /// The same key name in two sections only updates the targeted section.
    #[test]
    fn updates_only_the_targeted_section() {
        let input = b"[A]\r\nVol=1\r\n[B]\r\nVol=2\r\n";
        let out = s(set_ini_value(input, "B", "Vol", "9"));
        assert_eq!(out, "[A]\r\nVol=1\r\n[B]\r\nVol=9\r\n");
    }

    /// Section present, key absent: the key is appended within that section.
    #[test]
    fn appends_missing_key_within_section() {
        let input = b"[Audio]\r\nSoundVolume=0.7\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.5"));
        assert_eq!(out, "[Audio]\r\nSoundVolume=0.7\r\nScoreVolume=0.5\r\n");
    }

    /// Section absent: a new section is appended at the end of the file.
    #[test]
    fn appends_missing_section_at_eof() {
        let input = b"[Options]\r\nGameSpeed=3\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.5"));
        assert_eq!(out, "[Options]\r\nGameSpeed=3\r\n[Audio]\r\nScoreVolume=0.5\r\n");
    }

    /// Empty input yields a fresh section (CRLF default).
    #[test]
    fn empty_input_creates_section() {
        let out = s(set_ini_value(b"", "Audio", "ScoreVolume", "0.4"));
        assert_eq!(out, "[Audio]\r\nScoreVolume=0.4\r\n");
    }

    /// A file using LF endings keeps LF for the rewritten line.
    #[test]
    fn preserves_lf_line_endings() {
        let input = b"[Audio]\nScoreVolume=0.6\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.3"));
        assert_eq!(out, "[Audio]\nScoreVolume=0.3\n");
    }

    /// Section and key names match case-insensitively.
    #[test]
    fn matches_section_and_key_case_insensitively() {
        let input = b"[audio]\r\nscorevolume=0.6\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.3"));
        assert_eq!(out, "[audio]\r\nScoreVolume=0.3\r\n");
    }

    /// An unterminated final header line gets terminated before the inserted key.
    #[test]
    fn inserts_after_unterminated_header() {
        let out = s(set_ini_value(b"[Audio]", "Audio", "ScoreVolume", "0.4"));
        assert_eq!(out, "[Audio]\r\nScoreVolume=0.4\r\n");
    }

    /// Comment lines inside the section are preserved on a value replace.
    #[test]
    fn preserves_comment_lines() {
        let input = b"[Audio]\r\n; music level\r\nScoreVolume=0.6\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.1"));
        assert_eq!(out, "[Audio]\r\n; music level\r\nScoreVolume=0.1\r\n");
    }

    /// With a duplicated section header, a missing key is appended into the
    /// FIRST matching block (Win32 first-section semantics), not the last.
    #[test]
    fn appends_missing_key_into_first_duplicate_section() {
        let input = b"[Audio]\r\nSoundVolume=0.7\r\n[Audio]\r\nMusicVolume=0.5\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.250000"));
        assert_eq!(
            out,
            "[Audio]\r\nSoundVolume=0.7\r\nScoreVolume=0.250000\r\n\
[Audio]\r\nMusicVolume=0.5\r\n"
        );
    }

    /// With the key duplicated across two same-named sections, only the first
    /// occurrence is replaced (first-match-wins, matching the boot reader's
    /// first-section preference).
    #[test]
    fn replaces_only_first_duplicate_section_key() {
        let input = b"[Audio]\r\nScoreVolume=0.6\r\n[Audio]\r\nScoreVolume=0.9\r\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.1"));
        assert_eq!(out, "[Audio]\r\nScoreVolume=0.1\r\n[Audio]\r\nScoreVolume=0.9\r\n");
    }

    /// A key appended into an LF-only file is written with CRLF (the original
    /// writer always emits CRLF for new lines); existing lines keep their LF.
    #[test]
    fn appended_key_uses_crlf_even_in_lf_file() {
        let input = b"[Audio]\nSoundVolume=0.7\n";
        let out = s(set_ini_value(input, "Audio", "ScoreVolume", "0.5"));
        assert_eq!(out, "[Audio]\nSoundVolume=0.7\nScoreVolume=0.5\r\n");
    }
}
