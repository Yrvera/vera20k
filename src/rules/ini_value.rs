//! Typed INI accessor service — the gamemd CCINIClass `ReadX` analog.
//!
//! Sits on top of the raw `IniSection` store (the "INIClass" analog). Reproduces
//! the gamemd parse CONTRACT bit-for-bit on the resolved value: $xx/xxh hex,
//! C-atoi leniency, first-char bool, '%'-anywhere ×0.01 double, strtrim ≤0x20.
//!
//! INVARIANT (P4/P18): "present" = key exists (even if value is empty). A present
//! key ALWAYS returns its parsed value (which for int may be atoi("")=0). `default`
//! is returned ONLY when section/key is ABSENT. This is NOT `unwrap_or(default)` —
//! it does not fall to default on parse failure.
//!
//! This service is ADDITIVE this slice: no consumer reads it yet. The corpus
//! harness (`corpus_tests`) proves where it agrees with — and where it deliberately
//! corrects — the existing ad-hoc `get_*` accessors, without flipping any consumer.
//!
//! ## Dependency rules
//! - rules/ only: depends on `crate::rules::ini_parser`. No sim/render/ui/audio/net.
//! - Returns un-truncated f64 from `read_double`; the single f64->SimFixed
//!   conversion stays in `util::fixed_math`. No float enters sim/.

use crate::rules::ini_parser::IniSection;

/// 0x20 = ASCII space; gamemd `strtrim` strips bytes <= 0x20 (space + all ASCII
/// control) at BOTH ends — NOT Unicode whitespace.
const STRTRIM_MAX: u8 = 0x20;

/// Smallest gamemd per-accessor ReadString buffer cap (enum/zone/action). A
/// stock value longer than this would silently truncate in gamemd; we surface it
/// via a debug_assert instead of reproducing the C buffer truncation.
const SMALLEST_READSTRING_CAP: usize = 32;

impl IniSection {
    /// ReadInt (P1–P4, P18): `$xx`/`xxh` (case-insensitive `h`) hex, else C-atoi
    /// leniency. Default ONLY on absent key. Present-but-nonnumeric -> atoi (0).
    pub fn read_int(&self, key: &str, default: i32) -> i32 {
        match self.get(key) {
            None => default,
            Some(raw) => {
                // strtrim ≤0x20 both ends (P5), matching the value gamemd parses.
                let v = strtrim_ascii(raw);
                if let Some(rest) = v.strip_prefix('$') {
                    // "$%x": parse hex; junk after digits stops the C scan -> take
                    // the leading hex run (sscanf "$%x" stops at first non-hex).
                    parse_leading_hex(rest)
                } else if ends_with_h(v) {
                    // "%xh": leading hex run, ignore the trailing 'h'/'H'.
                    parse_leading_hex(&v[..v.len() - 1])
                } else {
                    atoi_lenient(v)
                }
            }
        }
    }

    /// ReadBool (P6, P18): `toupper(first char)` in {'1','T','Y'}=true,
    /// {'0','F','N'}=false, else default. `on`/`off` (first char 'o') -> default.
    /// Present-empty (no first char) -> default.
    pub fn read_bool(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                match v.bytes().next().map(|b| b.to_ascii_uppercase()) {
                    Some(b'1') | Some(b'T') | Some(b'Y') => true,
                    Some(b'0') | Some(b'F') | Some(b'N') => false,
                    _ => default, // present-empty or any other first char
                }
            }
        }
    }

    /// ReadDouble (P7): sscanf "%f" (leading float, single-precision) widened to
    /// f64, then ×0.01 iff the value string contains '%' ANYWHERE. Returns the
    /// gamemd double UN-truncated; the consumer truncates toward zero at ITS
    /// boundary (never `.round()` / never truncate here). Default ONLY on absent.
    /// (precision pinned by the S0 gate in `util::fixed_math`)
    pub fn read_double(&self, key: &str, default: f64) -> f64 {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                let leading: f32 = parse_leading_f32(v); // f32 first (mantissa narrow)
                let widened: f64 = leading as f64;
                if v.as_bytes().contains(&b'%') {
                    widened * 0.01_f64
                } else {
                    widened
                }
            }
        }
    }

    /// ReadString (P5, P18): strtrim ≤0x20 both ends; default on ABSENT key;
    /// present-empty -> "". No C buffer cap in Rust — debug_assert at the smallest
    /// gamemd per-accessor cap (32) to surface a corpus value that WOULD truncate
    /// (design open-question 6: debug-assert, do NOT silently truncate).
    pub fn read_string<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let v = strtrim_ascii(raw);
                debug_assert!(
                    v.len() <= SMALLEST_READSTRING_CAP,
                    "INI value for {key:?} is {} chars; gamemd smallest \
                     ReadString cap is {SMALLEST_READSTRING_CAP} (enum/zone/action) \
                     — would truncate",
                    v.len()
                );
                v
            }
        }
    }

    /// Read3Int (P8): comma "%d,%d,%d". All-defaults on ABSENT key. Each field
    /// atoi-lenient; missing trailing fields keep the corresponding default.
    pub fn read_3int(&self, key: &str, default: [i32; 3]) -> [i32; 3] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(3) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                out
            }
        }
    }

    /// ReadMinMax (P8): comma "%d,%d". All-defaults on ABSENT key.
    pub fn read_minmax(&self, key: &str, default: [i32; 2]) -> [i32; 2] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(2) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                out
            }
        }
    }

    /// ReadPoint/ReadSize (P9, COMMA): "%d,%d". All-defaults on ABSENT key.
    pub fn read_point(&self, key: &str, default: (i32, i32)) -> (i32, i32) {
        let [x, y] = self.read_minmax(key, [default.0, default.1]);
        (x, y)
    }

    /// ReadRect (P9, COMMA): "%d,%d,%d,%d". gamemd seeds "0,0,0,0" so missing
    /// fields keep the default component; all-defaults on ABSENT key.
    pub fn read_rect(&self, key: &str, default: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = [default.0, default.1, default.2, default.3];
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(4) {
                    out[i] = atoi_lenient(strtrim_ascii(tok));
                }
                (out[0], out[1], out[2], out[3])
            }
        }
    }

    /// ReadColorRGB (P21): COMMA "%d,%d,%d" -> [u8;3]. Per-component plain %d
    /// (stops at first non-digit; NO atoi-leniency beyond sign, NO hex). Default
    /// RGB on absent key or short value; component byte-narrowed to u8 (gamemd
    /// packs the sscanf int into a byte).
    ///
    /// `atoi_lenient` matches sscanf `%d` over the whole stock domain (leading
    /// sign + decimal digits, stop at non-digit) — corpus-confirmed ZERO `$`/`h`
    /// triplet components (plan-review C-R3). The `$`/`h` hex lives in `read_int`,
    /// NOT in `atoi_lenient`, so reusing it here stays faithful to `%d`.
    pub fn read_color_rgb(&self, key: &str, default: [u8; 3]) -> [u8; 3] {
        match self.get(key) {
            None => default,
            Some(raw) => {
                let mut out = default;
                for (i, tok) in strtrim_ascii(raw).split(',').enumerate().take(3) {
                    out[i] = atoi_lenient(strtrim_ascii(tok)) as u8;
                }
                out
            }
        }
    }

    /// ReadSpeed (P19): `read_int(-1)` sentinel; -1 -> default; else clamp100,
    /// `(v<<8)/100` truncate-toward-zero (Rust i32 `/` truncates toward 0),
    /// clamp255. `100→255`, `50→128`, `7→17`, `0→0`. (ledger #18)
    ///
    /// NB present-empty `Speed=` -> `read_int("")` = atoi("") = 0 (NOT the -1
    /// sentinel) -> `(0<<8)/100` = 0, NOT the call-site default. Correct per
    /// P4/P18; corpus harness scans stock for present-empty Speed/Range.
    pub fn read_speed(&self, key: &str, default: i32) -> i32 {
        let raw = self.read_int(key, -1);
        if raw == -1 {
            return default;
        }
        let capped = raw.min(100);
        let scaled = (capped << 8) / 100; // i32 / truncates toward zero (ledger #18)
        scaled.min(255)
    }

    /// ReadRange (P20): `read_double(-1.0)` sentinel; ==-1.0 -> default; else ftol
    /// TRUNCATE-TOWARD-ZERO. NOT `util::sim_to_i32` (that floors toward −∞ — DRIFT
    /// on negatives, ledger #18). `f64 as i32` truncates toward zero (saturating,
    /// NaN->0), matching gamemd ftol RC=11. `5.9→5`.
    pub fn read_range(&self, key: &str, default: i32) -> i32 {
        let raw = self.read_double(key, -1.0);
        if raw == -1.0 {
            return default;
        }
        raw as i32 // truncate toward zero (gamemd ftol RC=11)
    }
}

/// strtrim equivalent (P5): strip bytes <= 0x20 from BOTH ends. ASCII-only by
/// design (RA2 INI is ASCII); does NOT use `str::trim` (Unicode whitespace).
fn strtrim_ascii(s: &str) -> &str {
    let b = s.as_bytes();
    let mut start = 0usize;
    while start < b.len() && b[start] <= STRTRIM_MAX {
        start += 1;
    }
    let mut end = b.len();
    while end > start && b[end - 1] <= STRTRIM_MAX {
        end -= 1;
    }
    &s[start..end]
}

/// tolower(last char) == 'h' (P2). Case-insensitive via ASCII.
fn ends_with_h(s: &str) -> bool {
    s.as_bytes().last().map(|b| b.to_ascii_lowercase()) == Some(b'h')
}

/// Parse a leading run of hex digits (after `$` strip or before `h` strip).
/// sscanf "$%x"/"%xh" stop at the first non-hex char. No sign (hex is unsigned
/// in gamemd's `$`/`h` branches). Empty -> 0.
fn parse_leading_hex(s: &str) -> i32 {
    let mut acc: i64 = 0;
    let mut any = false;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => (c - b'0') as i64,
            b'a'..=b'f' => (c - b'a' + 10) as i64,
            b'A'..=b'F' => (c - b'A' + 10) as i64,
            _ => break,
        };
        any = true;
        acc = acc.wrapping_mul(16).wrapping_add(d);
    }
    if any {
        acc as i32
    } else {
        0
    }
}

/// C-atoi-equivalent leading-numeric parse (P3): optional leading sign, then
/// leading decimal digits, stop at first non-digit. `5cells`->5, `abc`->0,
/// ``->0, `  7 `->7 (already strtrimmed), `-50`->-50, `+9`->9. NB `0x1A`->0
/// (atoi does NOT treat `0x` as hex; the `$`/`h` branches are separate).
pub(crate) fn atoi_lenient(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut neg = false;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    let mut any = false;
    while i < b.len() && b[i].is_ascii_digit() {
        any = true;
        acc = acc.wrapping_mul(10).wrapping_add((b[i] - b'0') as i64);
        i += 1;
    }
    if !any {
        return 0;
    }
    let v = if neg { -acc } else { acc };
    v as i32
}

/// sscanf "%f"-equivalent leading float (P7): optional sign, digits, single dot,
/// more digits; stop at first non-float char (so "12.5%"->12.5, "10%0"->10).
/// Empty/junk -> 0.0. No exponent branch: corpus-confirmed ZERO `e`/`E` stock
/// double values (plan-review C-R3).
pub(crate) fn parse_leading_f32(s: &str) -> f32 {
    let b = s.as_bytes();
    let mut end = 0usize;
    let mut seen_dot = false;
    while end < b.len() {
        let c = b[end];
        let ok = c.is_ascii_digit()
            || (end == 0 && (c == b'-' || c == b'+'))
            || (c == b'.' && !seen_dot);
        if c == b'.' {
            seen_dot = true;
        }
        if !ok {
            break;
        }
        end += 1;
    }
    s[..end].parse::<f32>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::{atoi_lenient, parse_leading_f32};
    use crate::rules::ini_parser::IniFile;

    fn sec(body: &str) -> IniFile {
        IniFile::from_str(body)
    }

    #[test] // P1/P2
    fn test_read_int_hex() {
        let ini = sec("[S]\nA=$1A\nB=1Ah\nC=0FFH\nD=$0\nE=$FF\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -9), 26);
        assert_eq!(s.read_int("B", -9), 26);
        assert_eq!(s.read_int("C", -9), 255);
        assert_eq!(s.read_int("D", -9), 0);
        assert_eq!(s.read_int("E", -9), 255);
    }

    #[test] // P3/P4/P18
    fn test_read_int_atoi_leniency() {
        let ini = sec("[S]\nA=5cells\nB=abc\nC=-50\nD=\nE=  7 \n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -9), 5);
        assert_eq!(s.read_int("B", -9), 0); // present-nonnumeric -> 0, NOT default
        assert_eq!(s.read_int("C", -9), -50);
        assert_eq!(s.read_int("D", -9), 0); // present-empty -> atoi("") = 0
        assert_eq!(s.read_int("E", -9), 7);
        assert_eq!(s.read_int("MISSING", -9), -9); // absent -> default
    }

    #[test] // OQ3: 0x is NOT hex via atoi fallback
    fn test_read_int_0x_prefix_is_zero() {
        let ini = sec("[S]\nA=0x1A\n");
        // atoi("0x1A") = 0 (stops at 'x'); $/h branches don't fire.
        assert_eq!(ini.section("S").unwrap().read_int("A", -9), 0);
    }

    #[test]
    fn test_read_int_signs_and_edges() {
        let ini = sec("[S]\nA=+9\nB=-0\nC=$\nD=h\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_int("A", -1), 9);
        assert_eq!(s.read_int("B", -1), 0);
        assert_eq!(s.read_int("C", -1), 0); // "$" with no hex digits -> 0
        // "h": ends_with_h true, leading hex of "" -> 0. No stock key is `=h`
        // (corpus-confirmed); this locks the Rust helper's defined behavior.
        assert_eq!(s.read_int("D", -1), 0);
    }

    #[test] // P6/P18
    fn test_read_bool_first_char() {
        let ini =
            sec("[S]\nA=yes\nB=Y\nC=T\nD=true\nE=1\nF=no\nG=N\nH=F\nI=false\nJ=0\nK=off\nL=xyz\nM=\n");
        let s = ini.section("S").unwrap();
        for k in ["A", "B", "C", "D", "E"] {
            assert!(s.read_bool(k, false), "{k}");
        }
        for k in ["F", "G", "H", "I", "J"] {
            assert!(!s.read_bool(k, true), "{k}");
        }
        assert!(s.read_bool("K", true)); // 'off' first char 'o' -> default
        assert!(s.read_bool("L", true)); // xyz -> default
        assert!(s.read_bool("M", true)); // present-empty -> default
        assert!(s.read_bool("MISSING", true)); // absent -> default
    }

    #[test] // P7 (after the S0 gate pins precision)
    fn test_read_double_percent() {
        let ini = sec("[S]\nA=50%\nB=100%\nC=7\nD=0.5\nE=12.5%\n");
        let s = ini.section("S").unwrap();
        assert!((s.read_double("A", -1.0) - 0.5).abs() < 1e-6);
        assert!((s.read_double("B", -1.0) - 1.0).abs() < 1e-6);
        assert!((s.read_double("C", -1.0) - 7.0).abs() < 1e-6);
        assert!((s.read_double("D", -1.0) - 0.5).abs() < 1e-6);
        assert!((s.read_double("E", -1.0) - 0.125).abs() < 1e-6);
        assert!((s.read_double("MISSING", -42.0) + 42.0).abs() < 1e-9); // absent -> default
    }

    #[test] // P5/P18
    fn test_read_string_trim_default() {
        let ini = sec("[S]\nA=  hello  \nB=\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_string("A", "D"), "hello"); // trimmed
        assert_eq!(s.read_string("B", "D"), ""); // present-empty -> ""
        assert_eq!(s.read_string("MISSING", "D"), "D"); // absent -> default
    }

    #[test]
    fn test_atoi_and_leading_f32_helpers() {
        assert_eq!(atoi_lenient("5cells"), 5);
        assert_eq!(atoi_lenient("-50"), -50);
        assert_eq!(atoi_lenient("+9"), 9);
        assert_eq!(atoi_lenient(""), 0);
        assert!((parse_leading_f32("12.5%") - 12.5).abs() < 1e-6);
        assert!((parse_leading_f32(".9") - 0.9).abs() < 1e-6);
    }

    #[test] // P9 COMMA
    fn test_read_point_comma() {
        let ini = sec("[S]\nP=3,5\nR=1,2,3,4\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_point("P", (0, 0)), (3, 5));
        assert_eq!(s.read_rect("R", (0, 0, 0, 0)), (1, 2, 3, 4));
        assert_eq!(s.read_point("MISSING", (9, 9)), (9, 9)); // absent -> default
    }

    #[test] // P8 partial keeps default component
    fn test_read_3int_partial_keeps_default() {
        let ini = sec("[S]\nA=10,20\n"); // only 2 of 3 fields
        assert_eq!(ini.section("S").unwrap().read_3int("A", [1, 2, 3]), [10, 20, 3]);
    }

    #[test] // P21
    fn test_read_color_rgb() {
        let ini = sec("[S]\nC=12,34,56\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_color_rgb("C", [0, 0, 0]), [12, 34, 56]);
        assert_eq!(s.read_color_rgb("MISSING", [1, 2, 3]), [1, 2, 3]);
    }

    #[test] // P19
    fn test_read_speed_clamp() {
        let ini = sec("[S]\nA=100\nB=50\nC=7\nD=0\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_speed("A", -1), 255); // (100<<8)/100=256 -> clamp 255
        assert_eq!(s.read_speed("B", -1), 128); // (50<<8)/100=128
        assert_eq!(s.read_speed("C", -1), 17); // (7<<8)/100=17 (trunc)
        assert_eq!(s.read_speed("D", -1), 0);
        assert_eq!(s.read_speed("MISSING", 42), 42); // absent -> default (sentinel -1)
    }

    #[test] // P20 truncate toward zero (ledger #18)
    fn test_read_range_truncates() {
        let ini = sec("[S]\nA=5.9\nB=5\nC=0.4\n");
        let s = ini.section("S").unwrap();
        assert_eq!(s.read_range("A", -1), 5); // 5.9 -> 5 (never rounds to 6)
        assert_eq!(s.read_range("B", -1), 5);
        assert_eq!(s.read_range("C", -1), 0);
        assert_eq!(s.read_range("MISSING", 7), 7); // absent -> default (sentinel -1.0)
    }
}

/// S2 corpus equivalence harness — "the shadow assert". Read-only, NOT
/// hash-relevant: builds an `IniFile` and compares accessor outputs; never
/// touches `World`, `state_hash`, or `SNAPSHOT_VERSION`. It is a test, not a
/// consumer flip.
#[cfg(test)]
mod corpus_tests {
    use crate::rules::ini_parser::IniFile;

    // `ini_value.rs` lives in `src/rules/`, so `../../ini/` reaches the repo
    // `ini/` corpus (two levels up). `skirmish_modes.rs` uses `../ini/` only
    // because it is in `src/`.
    const STOCK_RULESMD: &str = include_str!("../../ini/rulesmd.ini");
    const STOCK_ARTMD: &str = include_str!("../../ini/artmd.ini");

    /// Smallest gamemd per-accessor ReadString cap; values over (cap-1) chars
    /// would truncate in an enum/zone/action read.
    const READSTRING_CAP: usize = 32;

    /// Keys where the NEW accessor INTENTIONALLY diverges from the OLD one,
    /// each with the gamemd-correct reason. Empty: corpus-confirmed (plan-review
    /// C-R3) that stock has ZERO `$`/`h`/`0x`/exponent values, so over the stock
    /// numeric domain the new `read_int`/`read_double` agree with the old
    /// `get_i32`/`get_f32` everywhere they both return a value. Any divergence
    /// surfaced by the test below must be classified against P1–P21 and added
    /// here with a cited reason, OR proven a new accessor bug and fixed.
    /// Format: (section, key, reason).
    const DIVERGENCES: &[(&str, &str, &str)] = &[];

    fn is_documented(section: &str, key: &str) -> bool {
        DIVERGENCES
            .iter()
            .any(|(s, k, _)| s.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key))
    }

    #[test]
    fn test_ini_accessor_corpus_parity() {
        // Scans the MERGED view (rules then art on top). Production keeps them
        // separate, but a per-key parse-EQUIVALENCE scan is unaffected: the
        // old and new accessor see the same stored string for each key.
        let mut ini = IniFile::from_str(STOCK_RULESMD);
        ini.merge(&IniFile::from_str(STOCK_ARTMD));

        let mut undocumented: Vec<String> = Vec::new();
        let mut zero_x: Vec<String> = Vec::new();
        let mut present_empty_transform: Vec<String> = Vec::new();
        let mut over_cap: Vec<String> = Vec::new();
        let mut exponent: Vec<String> = Vec::new();

        // Collect section names first to avoid borrowing `ini` while iterating.
        let names: Vec<String> = ini.section_names().iter().map(|s| s.to_string()).collect();
        for name in &names {
            let section = match ini.section(name) {
                Some(s) => s,
                None => continue,
            };
            let keys: Vec<String> = section.keys().map(|k| k.to_string()).collect();
            for key in &keys {
                let raw = section.get(key).unwrap_or("");
                let trimmed = raw.trim();

                // 0x-prefix scan (OQ3): would atoi to 0 if read as an int.
                if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                    zero_x.push(format!("[{name}] {key}={raw}"));
                }
                // Buffer-cap scan (smallest gamemd cap 32).
                if trimmed.len() > READSTRING_CAP - 1 {
                    over_cap.push(format!("[{name}] {key} len={}", trimmed.len()));
                }
                // Exponent-notation scan: confirm parse_leading_f32 needs no
                // exponent branch over the stock domain.
                if (raw.contains('e') || raw.contains('E')) && looks_numeric(trimmed) {
                    exponent.push(format!("[{name}] {key}={raw}"));
                }
                // Int equivalence: old get_i32 (None on parse-fail / absent) vs
                // new read_int. Only compare where the OLD returns Some — that is
                // the set the consumers actually use today.
                if let Some(old_i) = section.get_i32(key) {
                    let new_i = section.read_int(key, i32::MIN);
                    if old_i != new_i && !is_documented(name, key) {
                        undocumented.push(format!(
                            "[{name}] {key}: old_i32={old_i} new_int={new_i} raw={raw}"
                        ));
                    }
                }
                // Bool equivalence: old get_bool (whole-word, None otherwise) vs
                // new read_bool (first-char). Compare only where old returns Some.
                if let Some(old_b) = section.get_bool(key) {
                    let new_b = section.read_bool(key, !old_b); // sentinel != old_b
                    if old_b != new_b && !is_documented(name, key) {
                        undocumented.push(format!(
                            "[{name}] {key}: old_bool={old_b} new_bool={new_b} raw={raw}"
                        ));
                    }
                }
                // Present-empty transform scan (OQ4/C4): would resolve to 0 vs
                // the call-site default in read_speed/read_range.
                if trimmed.is_empty()
                    && matches!(
                        key.to_ascii_lowercase().as_str(),
                        "speed" | "range" | "minimumrange"
                    )
                {
                    present_empty_transform.push(format!("[{name}] {key}="));
                }
            }
        }

        // Fail ONLY on undocumented parse divergences. Each must be classified
        // (gamemd-correct fix -> DIVERGENCES, or accessor bug -> fix the code).
        assert!(
            undocumented.is_empty(),
            "UNDOCUMENTED parse divergences (each must be added to DIVERGENCES \
             with a gamemd-correct reason or proven a real fix):\n{}",
            undocumented.join("\n")
        );

        // Surface the remaining scans for the later flip slices. These are the
        // precise input lists for the consumer-flip parity re-baseline.
        if !zero_x.is_empty() {
            eprintln!("0x-prefixed values:\n{}", zero_x.join("\n"));
        }
        if !present_empty_transform.is_empty() {
            eprintln!(
                "present-empty Speed/Range/MinimumRange:\n{}",
                present_empty_transform.join("\n")
            );
        }
        if !over_cap.is_empty() {
            eprintln!(">31-char values (cap-32 scan):\n{}", over_cap.join("\n"));
        }
        if !exponent.is_empty() {
            eprintln!("exponent-notation values:\n{}", exponent.join("\n"));
        }
    }

    /// Cheap "is this a numeric value" test so the exponent scan does not flag
    /// every string key that happens to contain an 'e'/'E' (image names etc.).
    fn looks_numeric(s: &str) -> bool {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'+' | b'e' | b'E' | b'%'))
    }
}
