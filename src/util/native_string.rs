//! Active gamemd narrow/wide string boundary helpers.

/// Zero-extend each narrow byte to one Unicode scalar.
///
/// This is the engine's ordinary internal conversion. It is deliberately not
/// UTF-8, CP1252, or the Windows active code page.
pub fn widen_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn CharToOemBuffA(source: *const u8, destination: *mut u8, length: u32) -> i32;
}

#[cfg(windows)]
fn ansi_byte_to_oem(byte: u8) -> u8 {
    let mut converted = byte;
    // SAFETY: source and destination are distinct live one-byte objects, and
    // the explicit length keeps Win32 from reading either as a C string.
    let succeeded = unsafe { CharToOemBuffA(&byte, &mut converted, 1) };
    if succeeded == 0 { byte } else { converted }
}

#[cfg(not(windows))]
fn ansi_byte_to_oem(byte: u8) -> u8 {
    byte
}

/// Convert score-screen text to the byte-valued glyph string used by ScoreFont.
///
/// Each UTF-16 unit is truncated independently before ANSI-to-OEM conversion;
/// this is intentionally unrelated to the ordinary byte widening, player-edit
/// ACP boundary, CSF Unicode, and shared BitFont paths. An original wide NUL
/// terminates the input, while a zero low byte from a nonzero unit remains a
/// valid converted glyph value.
///
/// Retail provenance: ScoreFont low-byte ANSI-to-OEM glyph selection — ScoreFont width slot @ `0x006907E0`.
/// Retail provenance: ScoreFont low-byte ANSI-to-OEM glyph selection — ScoreFont draw slot @ `0x00690850`.
/// Retail provenance: ScoreFont low-byte ANSI-to-OEM glyph selection — ScoreFont draw slot @ `0x00690910`.
pub fn score_font_text(text: &str) -> String {
    let mut converted = String::with_capacity(text.len());
    for unit in text.encode_utf16().take_while(|unit| *unit != 0) {
        converted.push(char::from(ansi_byte_to_oem(unit as u8)));
    }
    converted
}

/// Encode UTF-16 text through the Windows active ANSI code page.
#[cfg(windows)]
pub fn acp_encode(text: &str) -> Vec<u8> {
    const CP_ACP: u32 = 0;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn WideCharToMultiByte(
            code_page: u32,
            flags: u32,
            wide: *const u16,
            wide_len: i32,
            narrow: *mut u8,
            narrow_len: i32,
            default_char: *const u8,
            used_default_char: *mut i32,
        ) -> i32;
    }

    let wide: Vec<u16> = text.encode_utf16().collect();
    if wide.is_empty() {
        return Vec::new();
    }
    let Ok(wide_len) = i32::try_from(wide.len()) else {
        return Vec::new();
    };
    // SAFETY: the sizing call reads `wide`; the conversion call writes the
    // exactly-sized initialized Vec allocation.
    unsafe {
        let narrow_len = WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide_len,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        );
        if narrow_len <= 0 {
            return Vec::new();
        }
        let mut narrow = vec![0u8; narrow_len as usize];
        if WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            wide_len,
            narrow.as_mut_ptr(),
            narrow_len,
            std::ptr::null(),
            std::ptr::null_mut(),
        ) <= 0
        {
            return Vec::new();
        }
        narrow
    }
}

#[cfg(not(windows))]
pub fn acp_encode(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// Decode bytes returned by an ANSI Win32 API through CP_ACP.
#[cfg(windows)]
pub fn acp_decode(bytes: &[u8]) -> String {
    const CP_ACP: u32 = 0;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            flags: u32,
            narrow: *const u8,
            narrow_len: i32,
            wide: *mut u16,
            wide_len: i32,
        ) -> i32;
    }
    if bytes.is_empty() {
        return String::new();
    }
    let Ok(narrow_len) = i32::try_from(bytes.len()) else {
        return String::new();
    };
    // SAFETY: the sizing call reads `bytes`; the conversion call writes the
    // exactly-sized initialized Vec allocation.
    unsafe {
        let wide_len = MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            narrow_len,
            std::ptr::null_mut(),
            0,
        );
        if wide_len <= 0 {
            return String::new();
        }
        let mut wide = vec![0u16; wide_len as usize];
        if MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            narrow_len,
            wide.as_mut_ptr(),
            wide_len,
        ) <= 0
        {
            return String::new();
        }
        String::from_utf16_lossy(&wide)
    }
}

#[cfg(not(windows))]
pub fn acp_decode(bytes: &[u8]) -> String {
    widen_bytes(bytes)
}

/// Round-trip edit-control text through the Windows active ANSI code page.
///
/// Native owner-draw edits cross this boundary in both directions. Characters
/// that the current ACP cannot represent are replaced by the platform default
/// byte (normally `?`).
#[cfg(windows)]
pub fn acp_round_trip(text: &str) -> String {
    acp_decode(&acp_encode(text))
}

#[cfg(not(windows))]
pub fn acp_round_trip(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_font_ascii_is_identity() {
        let ascii: String = (0x20u8..=0x7E).map(char::from).collect();
        assert_eq!(score_font_text(&ascii), ascii);
    }

    #[test]
    fn score_font_truncates_euro_before_oem_mapping() {
        let expected = char::from(ansi_byte_to_oem(0xAC)).to_string();
        assert_eq!(score_font_text("\u{20AC}"), expected);
    }

    #[test]
    fn score_font_stops_at_an_original_wide_nul() {
        assert_eq!(score_font_text("A\0ignored"), "A");
        assert_eq!(score_font_text("\u{100}"), "\0");
    }

    #[cfg(windows)]
    #[test]
    fn score_font_e_acute_matches_one_byte_char_to_oem_buff() {
        let source = 0xE9u8;
        let mut expected = source;
        // SAFETY: this independently exercises the native one-byte call with
        // distinct source and destination objects, matching the verified ABI.
        let succeeded = unsafe { CharToOemBuffA(&source, &mut expected, 1) };
        if succeeded == 0 {
            expected = source;
        }

        assert_eq!(score_font_text("\u{E9}"), char::from(expected).to_string());
    }
}
