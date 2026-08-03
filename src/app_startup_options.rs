//! Retail process-start options: the `WinMain` command-line switch table and
//! the `RA2MD.INI [Video]` screen size that is read immediately after it.
//!
//! The three stages run in a fixed native order and each one can overwrite the
//! previous, so they are modelled as one pipeline rather than three lookups:
//!
//! 1. `OptionsClass::SetDefaults` seeds the screen-size fields with `-1`, the
//!    "nobody has chosen a size yet" sentinel.
//! 2. The command-line switch table may write a size over that sentinel.
//! 3. `[Video] ScreenWidth` / `ScreenHeight` are then read **using the current
//!    values as their defaults** — so a key present in `RA2MD.INI` overrides the
//!    command line, and an absent key leaves the command line's value standing.
//! 4. If either field is still the sentinel, a built-in default pair is chosen.
//!
//! Native uppercases every argument after `argv[0]` in place before matching,
//! so every comparison here is effectively case-insensitive. Matching is a mix
//! of whole-token equality and substring search, per switch; both forms are
//! reproduced rather than normalised to one.
//!
//! Native's tokenizer stops after 20 `argv` slots. That is a fixed-size stack
//! array, i.e. exactly the kind of native storage cap the engine replaces, so
//! the cap is deliberately not reproduced.
//!
//! ## Dependency rules
//! - Reads `RA2MD.INI` through `rules::ini_parser`; no render/sim/ui/audio
//!   dependencies.

use std::ffi::OsString;
use std::path::Path;

use crate::rules::ini_parser::IniFile;

const RA2MD_INI_FILENAME: &str = "RA2MD.INI";
const VIDEO_SECTION: &str = "Video";
const SCREEN_WIDTH_KEY: &str = "ScreenWidth";
const SCREEN_HEIGHT_KEY: &str = "ScreenHeight";

/// The value `OptionsClass::SetDefaults` writes into both screen-size fields
/// before anything reads them. The built-in default rule tests for exactly this.
const SCREEN_SIZE_UNSET: i32 = -1;

/// The pair native picks when no size has been chosen.
///
/// Native takes this branch when its platform-version term is true, and also
/// when that term is false but the display-memory query reports **more than**
/// 2 MiB. The 640x480 branch therefore needs the platform term false *and* the
/// query reporting 2 MiB or less — both conditions, not either. (The two fields
/// behind the platform term have no decompilable initializer, so what they hold
/// is UNCHECKED; only the branch structure is verified.)
const BUILT_IN_DEFAULT_SCREEN: ScreenSize = ScreenSize {
    width: 800,
    height: 600,
};

/// Switches whose native effect is a networking, replay, string-table or debug
/// mode this engine has no subsystem for. Native accepts them silently, so the
/// launch must not fail on them; recognising them here is what keeps that true.
/// Matched as substrings, exactly as native does.
const ACCEPTED_SUBSTRING_SWITCHES: &[&str] = &[
    "-DESTNET",
    "-SOCKET",
    "-MPDEBUG",
    "-DROP=",
    "-SPEEDCONTROL",
    "-DLINK1",
    "-NETGEAR",
    "-NOROUTER",
    "-STEALTH",
    "-MESSAGES",
    "-ATTRACT",
];

/// Switches native compares whole-token and whose effect this engine has no
/// subsystem for.
const ACCEPTED_EXACT_SWITCHES: &[&str] = &["-JABBER", "-STR", "-NOSTR", "-RECORD", "-PLAY", "-O"];

/// The four help forms. Native prints its usage strings and returns a failure
/// from the switch parser, which makes `WinMain` bail before any window exists.
const USAGE_SWITCHES: &[&str] = &["/?", "-?", "-H", "/H"];

/// The `-CD` media switch. Native searches for it as a substring, so
/// `prefix-CDsuffix` selects the wildcard media branch just as bare `-CD` does.
const CD_MEDIA_SWITCH: &str = "-CD";

/// Windowed presentation.
const WINDOWED_SWITCH: &str = "-WIN";

/// Audio off.
const NO_AUDIO_SWITCH: &str = "-NOAUDIO";

/// A resolved client size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

/// Everything the native pre-window startup path decides from the command line
/// and `RA2MD.INI`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetailStartupOptions {
    /// A help form was passed: print usage and terminate before a window opens.
    pub usage_requested: bool,
    /// `-CD` selected the wildcard media-archive branch.
    pub cd_media: bool,
    /// `-WIN` asked for windowed presentation. This engine only ever presents
    /// in a window, so the switch is accepted and has nothing left to change.
    pub windowed: bool,
    /// Cleared by `-NOAUDIO`. Parsed and reported, but **not yet applied** —
    /// no audio-init caller reads it, so the switch is accepted without effect.
    pub audio_enabled: bool,
    /// Screen width, or [`SCREEN_SIZE_UNSET`] while nothing has chosen one.
    pub screen_width: i32,
    /// Screen height, or [`SCREEN_SIZE_UNSET`] while nothing has chosen one.
    pub screen_height: i32,
}

impl Default for RetailStartupOptions {
    fn default() -> Self {
        Self {
            usage_requested: false,
            cd_media: false,
            windowed: false,
            audio_enabled: true,
            screen_width: SCREEN_SIZE_UNSET,
            screen_height: SCREEN_SIZE_UNSET,
        }
    }
}

impl RetailStartupOptions {
    /// Read the switch table straight from the process argument vector.
    ///
    /// Native's switch table writes process-global fields that every later
    /// reader consults where it needs them, so a consumer does not have to be
    /// handed the result through the launch dispatch. `AssetManager` already
    /// resolves `-CD` from the process argv the same way.
    pub fn from_process_arguments() -> Self {
        consume_retail_switches(std::env::args_os().skip(1).collect()).0
    }

    /// Apply `[Video] ScreenWidth` / `ScreenHeight` from `{ra2_dir}/RA2MD.INI`.
    ///
    /// Native passes the *current* field values as the read defaults, so a key
    /// present in the file wins over the command line and an absent key changes
    /// nothing. A missing or unparsable file leaves both fields untouched,
    /// which is what a `RawFileClass` open failure does natively.
    ///
    /// Known divergence, recorded not fixed: a key present with an unparsable
    /// value yields zero here rather than the caller's default, because that is
    /// what the shared INI integer reader does. Changing it would affect every
    /// other caller of that reader, so it does not belong in this module.
    ///
    /// **No production caller yet** — see `resolve_screen_size`.
    pub fn apply_ra2md_video_section(&mut self, ra2_dir: &Path) {
        let path = ra2_dir.join(RA2MD_INI_FILENAME);
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let Ok(ini) = IniFile::from_bytes(&bytes) else {
            log::warn!("Could not parse {} for [Video]", path.display());
            return;
        };
        self.apply_video_section(&ini);
    }

    fn apply_video_section(&mut self, ini: &IniFile) {
        let Some(video) = ini.section(VIDEO_SECTION) else {
            return;
        };
        if let Some(width) = video.get_i32(SCREEN_WIDTH_KEY) {
            self.screen_width = width;
        }
        if let Some(height) = video.get_i32(SCREEN_HEIGHT_KEY) {
            self.screen_height = height;
        }
    }

    /// The client size the window would be created at.
    ///
    /// Native applies the built-in default pair when *either* field is still
    /// the sentinel, replacing both — a file that sets only one of the two keys
    /// does not get a half-configured size.
    ///
    /// The 640x480 branch of the native rule is unreachable here: reaching it
    /// needs the platform-version term false *and* the display-memory query
    /// reporting 2 MiB or less, and wgpu exposes no equivalent query on a host
    /// that could report so little. VERA-internal substitution; the native
    /// display-memory term is UNCHECKED against a low-memory adapter.
    ///
    /// **Not yet applied to the window.** `App::initialize` still creates the
    /// window from its own constants, and `App::enter_shell_window_mode`
    /// re-applies them on every shell entry, so honouring this value needs both
    /// of those fed from one `AppState` field rather than a call here.
    pub fn resolve_screen_size(&self) -> ScreenSize {
        if self.screen_width == SCREEN_SIZE_UNSET || self.screen_height == SCREEN_SIZE_UNSET {
            return BUILT_IN_DEFAULT_SCREEN;
        }
        ScreenSize {
            width: self.screen_width.max(1) as u32,
            height: self.screen_height.max(1) as u32,
        }
    }

    fn apply(&mut self, recognized: RecognizedSwitch) {
        match recognized {
            RecognizedSwitch::Usage => self.usage_requested = true,
            RecognizedSwitch::CdMedia => self.cd_media = true,
            RecognizedSwitch::Windowed => self.windowed = true,
            RecognizedSwitch::NoAudio => self.audio_enabled = false,
            RecognizedSwitch::ScreenSize { width, height } => {
                self.screen_width = width;
                if let Some(height) = height {
                    self.screen_height = height;
                }
            }
            RecognizedSwitch::NoLocalEffect => {}
        }
    }
}

/// What one argument matched in the native switch chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecognizedSwitch {
    Usage,
    CdMedia,
    Windowed,
    NoAudio,
    ScreenSize {
        width: i32,
        height: Option<i32>,
    },
    /// Native acts on it, but the effect has no counterpart here yet. Consuming
    /// it is still the point: an accepted switch must not abort the launch.
    NoLocalEffect,
}

/// Split a raw argument vector into the retail switches it contains and the
/// arguments that are not retail switches.
///
/// Arguments that match nothing in the native chain are passed through
/// untouched so the strict capture parser still sees — and still rejects — a
/// misspelled automation flag. Native instead ignores an unrecognised argument
/// outright; keeping the strict rejection is a VERA-internal deviation that
/// preserves the sealed capture contract.
pub fn consume_retail_switches(arguments: Vec<OsString>) -> (RetailStartupOptions, Vec<OsString>) {
    let mut options = RetailStartupOptions::default();
    let mut passthrough = Vec::with_capacity(arguments.len());
    for argument in arguments {
        // Native uppercases the argument in place before every comparison.
        let normalized = argument.to_string_lossy().to_ascii_uppercase();
        match classify(&normalized) {
            Some(recognized) => options.apply(recognized),
            None => passthrough.push(argument),
        }
    }
    (options, passthrough)
}

/// Walk the native switch chain in its native order. The order is load-bearing:
/// the `-CD` substring test runs before the numeric-size test, and the numeric
/// test runs before the whole-token `-WIN` test.
fn classify(normalized: &str) -> Option<RecognizedSwitch> {
    if normalized == NO_AUDIO_SWITCH {
        return Some(RecognizedSwitch::NoAudio);
    }
    if ACCEPTED_EXACT_SWITCHES.contains(&normalized) {
        return Some(RecognizedSwitch::NoLocalEffect);
    }
    if USAGE_SWITCHES.contains(&normalized) {
        return Some(RecognizedSwitch::Usage);
    }
    if normalized.contains(CD_MEDIA_SWITCH) {
        return Some(RecognizedSwitch::CdMedia);
    }
    if ACCEPTED_SUBSTRING_SWITCHES
        .iter()
        .any(|switch| normalized.contains(switch))
    {
        return Some(RecognizedSwitch::NoLocalEffect);
    }
    // Native reaches its size parser through `isdigit(argument[1])`, then runs
    // `sscanf(argument, "-%dX%d", &width, &height)`.
    if normalized.as_bytes().get(1).is_some_and(u8::is_ascii_digit) {
        // Native consumes the argument here even when the scan assigns
        // nothing. Passing a non-matching argument through instead keeps a
        // capture option's numeric value (`--width 800`) out of this branch.
        // VERA-internal narrowing; native has no such options.
        return scan_screen_size(normalized)
            .map(|(width, height)| RecognizedSwitch::ScreenSize { width, height });
    }
    if normalized == WINDOWED_SWITCH {
        return Some(RecognizedSwitch::Windowed);
    }
    // `-X…` is a native debug form that only accepts a run of `Q` characters
    // after the prefix. Recorded divergence: on any other trailing character
    // native prints a message and aborts the launch, where this accepts the
    // argument and carries on. Chosen deliberately — the flag drives no VERA
    // subsystem, so refusing to start over it would be the worse failure.
    if normalized.starts_with("-X") {
        return Some(RecognizedSwitch::NoLocalEffect);
    }
    None
}

/// Reproduce `sscanf(argument, "-%dX%d", &width, &height)`.
///
/// The height is assigned only when a literal `X` and a second integer both
/// follow, and nothing at all is assigned when the leading `-` is missing.
/// `-480` therefore sets the *width* to 480 and `-16` sets the width to 16 —
/// the separately named `-480` and `-16` branches further down the native chain
/// are unreachable, because this numeric test runs ahead of them.
fn scan_screen_size(normalized: &str) -> Option<(i32, Option<i32>)> {
    let after_dash = normalized.strip_prefix('-')?;
    let (width, rest) = scan_i32(after_dash);
    let width = width?;
    let Some(rest) = rest.strip_prefix('X') else {
        return Some((width, None));
    };
    Some((width, scan_i32(rest).0))
}

/// One `%d` conversion: the longest run of leading ASCII digits, and the rest.
/// A sign or leading whitespace cannot occur here — the caller has already
/// established that the character after the `-` is a digit, and arguments are
/// whitespace-split before they arrive.
fn scan_i32(text: &str) -> (Option<i32>, &str) {
    let end = text
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, rest) = text.split_at(end);
    (digits.parse::<i32>().ok(), rest)
}

/// The usage text a help switch prints.
///
/// Native loads six numbered strings from an external debug string table that
/// does not ship with the retail assets, so its exact wording is UNCHECKED.
///
/// Only switches that actually change what this engine does are listed as
/// working. The rest are parsed and accepted — which is the point, since a
/// launch must never fail on a switch retail takes — but nothing reads their
/// result yet, so advertising them would promise behaviour that does not exist.
pub fn usage_text() -> String {
    [
        "Usage: vera20k [options]",
        "",
        "  -CD             Use the wildcard media-archive branch.",
        "  -WIN            Run in a window (this engine is always windowed).",
        "  -? -h /? /h     Show this message.",
        "",
        "Other retail switches are accepted so a launch never fails on them,",
        "but are not applied yet: -<W>X<H>, -480, -16, -NOAUDIO, -RECORD,",
        "-PLAY, -SOCKET, -DESTNET and the remaining net/debug flags.",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consume(arguments: &[&str]) -> (RetailStartupOptions, Vec<String>) {
        let (options, passthrough) =
            consume_retail_switches(arguments.iter().map(OsString::from).collect());
        (
            options,
            passthrough
                .into_iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
        )
    }

    #[test]
    fn defaults_leave_the_screen_size_unset_and_audio_on() {
        let options = RetailStartupOptions::default();
        assert_eq!(options.screen_width, SCREEN_SIZE_UNSET);
        assert_eq!(options.screen_height, SCREEN_SIZE_UNSET);
        assert!(options.audio_enabled);
        assert_eq!(options.resolve_screen_size(), BUILT_IN_DEFAULT_SCREEN);
    }

    #[test]
    fn every_help_form_requests_usage_in_either_case() {
        for form in ["/?", "-?", "-h", "/h", "-H", "/H"] {
            let (options, rest) = consume(&[form]);
            assert!(options.usage_requested, "{form} must request usage");
            assert!(rest.is_empty(), "{form} must be consumed");
        }
    }

    #[test]
    fn cd_matches_as_a_substring_but_win_matches_whole_token() {
        // Native searches for `-CD` with `strstr`, so an argument that merely
        // contains it selects the wildcard media branch.
        assert!(consume(&["-CD"]).0.cd_media);
        assert!(consume(&["-cd"]).0.cd_media);
        assert!(consume(&["-CDROM"]).0.cd_media);
        assert!(consume(&["prefix-CDsuffix"]).0.cd_media);
        // `-WIN` is a whole-token compare: a superstring is not the switch.
        assert!(consume(&["-win"]).0.windowed);
        assert!(consume(&["-WIN"]).0.windowed);
        let (options, rest) = consume(&["-WINDOW"]);
        assert!(!options.windowed);
        assert_eq!(rest, vec!["-WINDOW".to_string()]);
    }

    #[test]
    fn numeric_switches_follow_the_sscanf_shape() {
        let (both, _) = consume(&["-1024X768"]);
        assert_eq!((both.screen_width, both.screen_height), (1024, 768));
        assert_eq!(
            both.resolve_screen_size(),
            ScreenSize {
                width: 1024,
                height: 768
            }
        );
        // Lowercase `x` survives because native uppercases the argument first.
        let (lower, _) = consume(&["-640x480"]);
        assert_eq!((lower.screen_width, lower.screen_height), (640, 480));
        // `-480` and `-16` reach the same scan, so they set the WIDTH only and
        // leave the height at the sentinel — which then discards both.
        let (four_eighty, _) = consume(&["-480"]);
        assert_eq!(four_eighty.screen_width, 480);
        assert_eq!(four_eighty.screen_height, SCREEN_SIZE_UNSET);
        assert_eq!(four_eighty.resolve_screen_size(), BUILT_IN_DEFAULT_SCREEN);
        let (sixteen, _) = consume(&["-16"]);
        assert_eq!(sixteen.screen_width, 16);
    }

    #[test]
    fn a_numeric_capture_value_is_not_eaten_as_a_screen_size() {
        // `--width 800` must survive intact for the sealed capture parser.
        let (options, rest) = consume(&["--width", "800", "--height", "600"]);
        assert_eq!(options.screen_width, SCREEN_SIZE_UNSET);
        assert_eq!(
            rest,
            vec![
                "--width".to_string(),
                "800".to_string(),
                "--height".to_string(),
                "600".to_string()
            ]
        );
    }

    #[test]
    fn accepted_switches_are_consumed_without_changing_anything() {
        let (options, rest) = consume(&[
            "-record",
            "-play",
            "-jabber",
            "-str",
            "-nostr",
            "-ATTRACT",
            "-MESSAGES",
            "-STEALTH",
            "-NOROUTER",
            "-NETGEAR",
            "-DLINK1",
            "-SPEEDCONTROL",
            "-DROP=3",
            "-MPDEBUG",
            "-SOCKET1234",
            "-DESTNET",
            "-O",
            "-XQ",
        ]);
        assert!(rest.is_empty(), "every native switch must be consumed");
        assert_eq!(options, RetailStartupOptions::default());
    }

    #[test]
    fn noaudio_clears_audio_and_unknown_arguments_pass_through() {
        let (options, rest) = consume(&["-noaudio", "--not-a-real-option"]);
        assert!(!options.audio_enabled);
        assert_eq!(rest, vec!["--not-a-real-option".to_string()]);
    }

    #[test]
    fn video_keys_override_the_command_line_and_absent_keys_do_not() {
        let ini = IniFile::from_bytes(b"[Video]\nScreenWidth=640\nScreenHeight=480\n")
            .expect("parse video ini");
        let (mut options, _) = consume(&["-1024X768"]);
        options.apply_video_section(&ini);
        assert_eq!((options.screen_width, options.screen_height), (640, 480));

        let empty = IniFile::from_bytes(b"[Audio]\nScoreVolume=0.500000\n").expect("parse ini");
        let (mut untouched, _) = consume(&["-1024X768"]);
        untouched.apply_video_section(&empty);
        assert_eq!(
            (untouched.screen_width, untouched.screen_height),
            (1024, 768)
        );
    }

    #[test]
    fn one_video_key_alone_still_falls_back_to_the_built_in_pair() {
        let ini = IniFile::from_bytes(b"[Video]\nScreenWidth=1280\n").expect("parse video ini");
        let mut options = RetailStartupOptions::default();
        options.apply_video_section(&ini);
        assert_eq!(options.screen_width, 1280);
        assert_eq!(options.screen_height, SCREEN_SIZE_UNSET);
        assert_eq!(options.resolve_screen_size(), BUILT_IN_DEFAULT_SCREEN);
    }
}
