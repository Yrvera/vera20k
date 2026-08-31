# Options Profile Transaction Implementation Contract

Date: 2026-08-31

Scope: Phase 14 process-owned `RA2MD.INI` Options/Video/Audio load, runtime application, and coherent write transaction

Status: READY_FOR_PLAN

## Gap Being Closed

Retail YR defaults, reads, applies, retains, and writes user Options/Video/Audio values through one process-owned `OptionsClass`; current Rust discards its parsed startup screen pair, hydrates only two Options fields plus ScoreVolume, combines Sound and Voice gain, and writes partial settings through unrelated paths.

## Scope

Included:

- Active interactive-process defaults and typed reads for all persisted `[Options]`, `[Video]`, and `[Audio]` fields in `OptionsClass`.
- One Rust-native process profile owner retained across shell and match transitions.
- Early interactive screen-pair resolution before window creation and reuse on shell re-entry.
- Boot-time application to existing ScrollRate, DetailLevel, UnitActionLines, ToolTips, music, SFX, and voice consumers.
- Retention and native-form serialization of fields without a current runtime consumer.
- One byte-preserving settings commit using one file snapshot and one filesystem write on existing accepted/quit boundaries.
- Existing capture dimensions remain a higher-priority, automation-only override.
- Existing deterministic `SetGameSpeed` command seam remains authoritative once a match is active.

Excluded:

- Launcher and in-game dialog visual layout, paint order, assets, and control construction.
- Keyboard binding edit/save behavior and `KEYBOARDMD.INI`.
- Resolution-dialog mode enumeration and native failed-display-mode recovery.
- `[Network]` field semantics and online protocol support; unrelated Network bytes must only be preserved.
- Developer-overlay persistence behavior.
- Mod-only duplicate/case-variant profile exactification beyond the active ordinary retail profile.

Basis: Phase 14 disparity scan plus the exhaustive-slice `OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md`, independently checked against the active retail profile and current `origin/main` Rust.

## Evidence Baseline

| Source | Role | Use |
|---|---|---|
| active `gamemd.exe` functions `0x005FA350`, `0x005FA620`, `0x005FAD10`, `0x004E1D00`, `0x004E1DE0`, `0x0055FAA0`, `0x0055FC80` and WinMain `0x006BD94A..0x006BD9B5` | PRIMARY | defaults, read/apply/write ownership, ordering, clamps, screen fallback, dialog commit gates |
| `docs/research/OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md` | PRIMARY synthesis of live binary/profile/Rust reads | bounded field map, detailed handoff, zero-add and cold-check record; its Section 4 case/duplicate equivalence sentence is explicitly excluded as stale/conflicted below |
| active loose `C:\Users\enok\Documents\Command and Conquer Red Alert II\RA2MD.INI`, SHA-256 `9A4093A7323472217B1F5CB0C5B886DEFA6A7027698D247955AB59513BB2EC9C` | PRIMARY | active spelling, lexical shape, absence of duplicate Options/Video/Audio sections/keys; not constructor defaults |
| current Rust files cited per row at `origin/main` `054696bb91a1daf066915ecdc44364deadfba91e` | PRIMARY | actual ownership, call paths, consumers, parsers, writers, and tests |
| `docs/research/INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` | STALE_OR_CONFLICTED for store casing/duplicates; PRIMARY only where reconfirmed by live typed-reader bodies | navigation for `ReadInt`/`ReadBool`/`ReadDouble`; its older lowercase-map description no longer matches current Rust |
| `OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md`, `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` | SYNTHESIS / navigation | prior entry points and field hypotheses; both were rechecked before use |
| `docs/gap-scans/2026-08-31-disparity-scan-phase-14.md` and clean-slate order rows 299-300 | DERIVATIVE | phase ordering and candidate gap only; not behavior authority |

## gamemd Baseline Safe To Implement

- `OptionsClass__SetDefaults @ 0x005FA350` establishes the exact defaults. `CampDifficulty +0x08` is the notable zero-initialized field not explicitly written by the constructor body.
- `OptionsClass__ReadFromINI @ 0x005FA620` passes each current field as the typed reader default. Missing file/section/key preserves that field. Difficulty clamps signed values to `0..4`; CampDifficulty and DetailLevel clamp to `0..2`; other Options integers do not clamp at this layer.
- Video width/height begin at `-1`. If either is still `-1`, WinMain replaces both. The Rust-native ordinary-host fallback is the already documented built-in `800x600`; the low-memory `<= 0x200000` retail branch is not observable through wgpu and remains non-blocking platform exactification.
- Sound/Voice/Score defaults are `0.7f/0.7f/0.4f`. The native profile object upper-clamps only. The distinct runtime channels and application order are Sound -> SFX, Voice -> Vox then voice mixer, Score -> music then Theme.
- The read immediately synchronizes action-line and audio consumers. It is a process-load mechanism, not simulation state and not a save/replay field.
- `WriteToINI @ 0x005FAD10` writes 11 Options keys, 3 Video keys, and 7 Audio keys using `%d`, lowercase `yes/no`, and `%f`; `AllowHiResModes`, `AllowModeToggle`, and `AllowVRAMSidebar` are read-only and omitted.
- In-game result `1` applies then writes; result `2` does neither. Launcher applies its parent controls after each parent modal pump and performs one final write after child-dialog loops.
- The native writer also owns Network, but this contract does not introduce online semantics. Preserving untouched Network bytes is the Rust-native equivalent for this slice.

## Parity Delta Table

| Evidence class | Delivery class | Mechanism/result | gamemd.exe behavior | Current Rust behavior | Required Rust delta | Evidence | Acceptance test |
|---|---|---|---|---|---|---|---|
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | process profile ownership | one singleton holds defaults, loaded values, dialog mutations, and final write values across the whole process | `App` retains only `StartupAudioDisposition`; Options live under match presentation; Score lives in `MusicPlayer`; startup video is discarded | add one process-lifetime Rust profile owner with exact Options/Video/Audio fields; UI/audio/window state becomes a projection/consumer, not parallel authority | gamemd `0x005FA350/620/AD10`; `src/app/mod.rs:77-133`; `src/app/state.rs:15-33`; `src/ui/shell/in_game_options_state.rs:17-53` | absent-file construction yields exact defaults; profile survives shell -> match -> shell without re-defaulting or losing untouched fields |
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | one typed profile load | full read uses current-as-default typed readers for every owned field and applies documented clamps | startup video helper uses `get_i32` and has no production caller; boot reads only ScrollRate/DetailLevel; Score has a separate reader | load one byte snapshot through existing `IniFile` and `IniSection::read_int/read_bool/read_double`; populate all fields and apply only verified field clamps | gamemd `0x005FA620`, typed readers `0x005276D0/0x005295F0/0x005283D0`; `src/app/frontend/startup_options.rs:142-176`; `src/app/persistence/options.rs:22-48`; `src/audio/music.rs:569-607`; `src/rules/ini_value.rs:40-63` | fixture with missing sections, present ordinary values, malformed decimal int, first-char bool, percent float, and out-of-range difficulty/detail produces the contract values from one load |
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | early interactive screen size | profile/switch screen fields are resolved before display creation; either sentinel replaces the pair; chosen pair persists for shell mode | `RetailStartupOptions` contains the pair, but `App::new` discards it; `initialize` and `enter_shell_window_mode` hardcode 800x600 | resolve the interactive profile before creating the window, retain chosen shell size in process/platform state, and use it on shell re-entry | WinMain `0x006BD94A..0x006BD9B5`; `src/app/frontend/startup_options.rs:98-203`; `src/app/mod.rs:125-133`; `src/app/initialize.rs:35-50`; `src/app/handler.rs:31-35` | profile 640x480 creates and later re-enters 640x480; one missing dimension selects 800x600; explicit capture dimensions still win and remain hidden |
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | coherent persisted snapshot | one object is serialized in native field/key order when the settings owner commits | six Options keys are updated through one path, Score through another, and each path independently reads/writes the file | format all owned keys from one profile, transform a single raw snapshot in memory, and perform one filesystem write while preserving unrelated bytes | gamemd `0x005FAD10`; `src/app/persistence/options.rs:102-136`; `src/audio/music.rs:592-607`; `src/util/ini_writer.rs:102-150` | CRLF fixture with comments, high bytes, Skirmish and Network sections retains every unrelated byte; all owned values change together; instrumented/test seam observes one write |
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | native lexical serialization | booleans write lowercase `yes/no`; audio floats use `%f` six-decimal shape; read-only Allow* keys are omitted | Options booleans serialize as `1/0`; Score writer is independent; no common Audio writer | serialize owned bools as lowercase words and volume values with six fractional digits; never insert Allow* keys | write helpers `0x00529560`, `0x005285B0`, constant bytes; `src/app/persistence/options.rs:115-126`; `src/audio/music.rs:586-605` | golden output contains `ToolTips=no`, `AutoScroll=yes`, `VoiceVolume=0.700000`; no `AllowHiResModes`, `AllowModeToggle`, or `AllowVRAMSidebar` is introduced |
| `REQUIRED_FIX` | `MILESTONE-BLOCKING` | boot projection into Options UI and live gates | loaded ScrollRate/DetailLevel/UnitActionLines/ShowHidden/ToolTips are live before the first match frame | only ScrollRate/DetailLevel hydrate; booleans default; target lines apply only on dialog close; tooltips always start enabled | project the loaded profile into `InGameOptionsState` and seed target-line/tooltip gates during initialization | `0x005FA620` tail and `0x004E1DE0`; `src/app/initialize.rs:346-423`; `src/app/persistence/options.rs:57-100`; `src/ui/tooltips.rs:77-82,200-205` | `ToolTips=no` disables hover before any dialog; `UnitActionLines=no` initializes target lines off; loaded ScrollRate/DetailLevel drive their existing consumers |
| `REQUIRED_FIX` | `COMPOUNDING` | distinct Sound and Voice channels | SoundVolume and VoiceVolume are separate stored gains with separate mixer owners; Score remains separate | `SfxPlayer::volume` scales normal SFX, unit voices, and EVA together; only Score is persisted | represent distinct sound and voice master gains; normal SFX reads sound gain, unit/EVA queue reads voice gain, music reads score gain; initialize all from the process profile | `0x005FA620`, `0x005FA510`, `0x005FA590`; `src/audio/sfx.rs:214-230,270-358,413-544,673-674`; `src/audio/music.rs:378-418` | Sound=0/Voice=1 silences weapon/environment effects but preserves unit/EVA; Sound=1/Voice=0 does the inverse; Score independently controls music |
| `REQUIRED_FIX` | `COMPOUNDING` | retain currently unsupported persisted fields | native object retains and writes GameSpeed, Difficulty, CampDifficulty, ScrollMethod, AutoScroll, SidebarCameoText, repeat, shuffle, latency, InGameMusic even when a given UI is closed | no single owner retains most of these fields, so a later partial write cannot serialize the native snapshot | model and round-trip every verified persisted field now; expose consumers only where current architecture has a verified owner | `0x005FA350/620/AD10`; Rust absence scan in research report Section 6 | load a fixture with distinct values for every field, mutate only ToolTips, commit, and prove every other modeled field retained its loaded value |
| `REQUIRED_FIX` | `COMPOUNDING` | accepted dialog commit ordering | accepted in-game dialog mutates live object/consumers before one write; canceled result mutates/writes nothing | current close path applies/persists six fields, but writes directly from match UI state; launcher has no controls | route accepted changes into the process profile, apply projections, then commit; cancel leaves profile, consumers, and bytes unchanged | `0x004E1D00`, `0x004E1DE0`; `src/app/persistence/options.rs:82-145` | accept changes profile -> target line/tooltip/detail effects -> file; cancel produces no profile/effect/file delta |
| `REQUIRED_FIX` | `COMPOUNDING` | existing quit persistence | native confirmed exit paths call full `WriteToINI` rather than a one-key audio save | `persist_settings_on_quit` writes only ScoreVolume and is called from selected main-menu quit paths | replace the one-key exit save with the coherent process profile commit at the already confirmed quit boundaries | Main pre-session confirmed-exit call to `0x005FAD10`; `src/app/in_game.rs:552-569`; `src/app/shell_main_menu.rs:398-405,668-675` | confirmed main-menu quit invokes the complete profile writer once and preserves unrelated bytes |
| `TEST_ONLY` | `COMPOUNDING` | existing native typed scalar semantics | ordinary-domain int/bool/double parsing already matches active helpers | `IniSection::read_int/read_bool/read_double` implement the verified semantics | reuse and add profile-level regression coverage; no new scalar parser | `0x005276D0`, `0x005295F0`, `0x005283D0`; `src/rules/ini_value.rs:40-63,170-204` | profile loader tests prove it delegates the same malformed-int, first-char-bool, f32-percent behavior rather than `get_i32/get_bool` |
| `TEST_ONLY` | `COMPOUNDING` | capture override isolation | retail has no capture harness; VERA capture dimensions are an authorized sealed automation override | `initialize` already prioritizes `capture_dimensions` over fixed shell size | preserve this priority while adding profile size | `src/app/initialize.rs:35-49`; capture constructors/launch routing | existing shell/tactical capture dimension tests remain byte/pixel stable with a conflicting RA2MD screen pair |
| `REQUIRED_FIX` | `EXACTIFICATION-RESIDUAL` | negative audio profile values | native upper-clamps only and forwards negative scaled integers to legacy audio owners | Rust music/SFX setters clamp to `0..1` | keep exact parsed profile values and document a safe output-boundary clamp unless backend behavior is separately proven; do not silently claim exact negative-output parity | native setters `0x005FA4A0/510/590`; `src/audio/music.rs:378-381`; `src/audio/sfx.rs:673-674` | negative input is retained/serialized by profile; output backend receives the documented safe value; row remains residual until backend parity is researched |
| `REQUIRED_FIX` | `EXACTIFICATION-RESIDUAL` | explicit invalid non-sentinel dimensions | native passes explicit values into later display setup whose failure recovery is outside this evidence slice | test-only Rust resolver clamps each explicit dimension to at least 1 | retain the Rust-native safety boundary and document it as an exactification residual; no attempt to mimic an unproven failure path | research deferred OQ-30; `src/app/frontend/startup_options.rs:195-202` | zero/negative explicit dimensions cannot panic or create invalid wgpu surfaces; ordinary positive and sentinel cases remain exact |
| `BLOCKED` | `EXACTIFICATION-RESIDUAL` | duplicate/case-variant profile lookup | current live evidence does not establish the exact result of multi-duplicate CRC/qsort selection or whether every load path normalizes case | current parser uses exact-case names, the first exact duplicate on initial load, and first exact section lookup; writer matches case-insensitively and updates selected occurrences | no change in this mechanism: active profile has exact retail spelling and no duplicates; if mod-profile exactification is scheduled, run a narrow re-investigation first | active profile shape; `src/rules/ini_parser.rs:62-85,227-330`; `src/util/ini_writer.rs:21-64,102-150`; contradiction with older typed-INI prose | ordinary exact-spelling/no-duplicate profile fixture passes; no test asserts unproven native duplicate/case behavior |
| `DOC_ONLY` | `EXACTIFICATION-RESIDUAL` | stale case/duplicate claim | not proven by the new bounded binary pass | `OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md` Section 4 says lookup is case-insensitive/later-wins and current `IniFile` models it, which is false of current code and overstates native evidence | after this contract gate, correct that sentence to identify the active-profile condition and the blocked exactification row above | conflicting report line and direct Rust reads above | documentation review confirms no implementation task cites the stale equivalence claim |
| `BLOCKED` | `UNKNOWN-RISK` | OS window-close persistence parity | this contract did not trace native `WM_CLOSE`/termination all the way to `WriteToINI` | Rust `CloseRequested` branches exit directly without the current quit writer | do not broaden this mechanism based on assumption; a narrow owner trace is required before changing close semantics | `src/app/handler.rs:107-109,159-162,250-258`; no active-binary close-owner proof in bounded report | no current implementation test; follow-up only if Phase 14 reverse audit shows ordinary players commonly bypass confirmed quit |

The two `BLOCKED` rows do not block this contract's common-path implementation: duplicate/case variants are absent from the active profile and OS-window-close parity is outside the proven accepted/confirmed commit owners. They remain explicit reverse-audit targets, not inferred requirements.

## Required Rust Changes

1. Process profile owner

   - Owner: process/app state, available before window creation and retained in initialized `AppState`.
   - Add exact Options/Video/Audio fields and constructor defaults proven at `0x005FA350`.
   - Keep profile state out of `sim/`, snapshots, hashes, replays, and save files.
   - Keep match `SetGameSpeed` flow intact; profile GameSpeed is settings/pacing state and a dialog projection, not a shortcut around the command queue.

2. Unified loader and startup ordering

   - Resolve the RA2 root and read `RA2MD.INI` once for interactive launch before creating the window.
   - Use existing typed `read_*` services and the verified per-field clamps.
   - Preserve explicit command-line switch precedence before the RA2MD Video override, and preserve capture-dimension precedence over both.
   - Move/retain the loaded profile into initialized app state without re-reading or re-defaulting it.

3. Consumer projection

   - Initialize `InGameOptionsState` from the process profile.
   - Seed ToolTips and UnitActionLines gates before first match input/render work.
   - Continue using existing ScrollRate and DetailLevel consumers.
   - Retain unsupported values for round-trip rather than adding speculative gameplay/UI consumers.

4. Audio ownership split

   - Keep MusicPlayer as Score channel consumer.
   - Split SfxPlayer's normal-effect master from its dedicated unit/EVA voice master while preserving existing pool/voice-slot architecture.
   - Queue and play voice/EVA items using the voice master; normal SFX paths use sound master.
   - Apply the profile before initial audible output. Later launcher/in-game audio controls must reuse these owners rather than add another volume layer.

5. One preservation-safe writer

   - Format every owned key from the process profile with native lexical forms.
   - Apply all section updates to one in-memory raw byte snapshot and issue one filesystem write.
   - Preserve comments, line endings, high bytes, key order where existing, and every unrelated section/key, including Network and Skirmish.
   - Replace independent Options and Score persistence calls at accepted/confirmed owners with this transaction.

## Acceptance Tests

1. `profile_defaults_match_optionsclass`

   - Setup: no `RA2MD.INI` bytes.
   - Action: construct/load the process profile.
   - Expected: all Options/Video/Audio defaults match the field table, including CampDifficulty zero, screen sentinels, Sound/Voice/Score `0.7/0.7/0.4`, latency 9, and exact bool defaults.
   - Proves: rows 1, 2, and 8.

2. `profile_load_uses_native_typed_semantics_and_clamps`

   - Setup: ordinary exact-spelling fixture with missing keys, malformed decimal integer, first-char bool tokens, `%` audio values, negative/oversized difficulty/detail.
   - Action: load once.
   - Expected: current defaults survive misses; typed values match `read_*`; Difficulty is `0..4`, CampDifficulty/DetailLevel `0..2`; other Options ints retain parsed values; audio applies upper-bound semantics in profile storage.
   - Proves: rows 2 and 11.

3. `interactive_profile_screen_pair_precedes_window_and_survives_shell_reentry`

   - Setup: interactive profile 640x480, then one-key-missing variant; separate capture request with conflicting dimensions.
   - Action: resolve initial window and later shell target.
   - Expected: 640x480 for complete interactive pair; 800x600 when either sentinel remains; capture dimensions win and remain non-visible.
   - Proves: rows 3 and 12.

4. `profile_boot_applies_options_gates_before_first_match_frame`

   - Setup: `ScrollRate=4`, `DetailLevel=1`, `UnitActionLines=no`, `ShowHidden=yes`, `ToolTips=no`.
   - Action: initialize app/profile projections without opening Options.
   - Expected: in-game view reflects all values, camera/detail consumers receive their existing values, target lines and tooltip service are disabled before first relevant frame/input.
   - Proves: row 6.

5. `sound_and_voice_masters_are_independent`

   - Setup: decoded normal SFX plus unit/EVA voice fixtures under Sound/Voice combinations `(0,1)` and `(1,0)`.
   - Action: compute/start outputs through the existing SFX pool and dedicated voice slot/queue.
   - Expected: only the intended channel is silent in each combination; Score volume does not affect either.
   - Proves: row 7.

6. `profile_commit_is_one_write_and_preserves_unowned_bytes`

   - Setup: CRLF `RA2MD.INI` fixture containing comments, high-byte player data, all owned keys, Network, Skirmish, unknown keys, and no Allow* keys; writer seam counts filesystem replacements.
   - Action: mutate one profile field and commit.
   - Expected: one filesystem write; all owned values reflect the in-memory profile, bools use `yes/no`, floats have six decimals, all unrelated bytes remain identical, no Allow* key is inserted.
   - Proves: rows 4, 5, and 8.

7. `accepted_and_canceled_options_transactions_are_atomic`

   - Setup: initial profile/consumers/file snapshot; one accepted and one canceled dialog result.
   - Action: close each dialog path.
   - Expected: accepted path updates profile, then consumers, then commits once; canceled path changes none of them.
   - Proves: row 9.

8. `confirmed_quit_commits_full_profile_once`

   - Setup: loaded profile with changed Options and Audio fields.
   - Action: take each existing confirmed main-menu quit owner.
   - Expected: full coherent transaction called exactly once per exit path; no one-key Score write remains.
   - Proves: row 10.

9. Existing focused regression suites

   - Preserve startup switch parsing, sealed shell/tactical capture dimensions, in-game slider inversion/result handling, tooltip enable gate, target-line state, SFX voice queue behavior, music/SFX volume APIs, and raw INI byte preservation.
   - Proves: no regression in `TEST_ONLY` rows and adjacent existing behavior.

## Known Non-Requirements

- Do not recreate a C++ singleton, raw `0xB8` memory copy, COM/vtable methods, x87 volume interpolators, or Theme/Vox integer scale internals. Preserve their observable channel/default/order semantics through Rust-native owners.
- Do not add Network UI, protocol handling, or parse/write authority. Preserve its bytes.
- Do not implement the keyboard editor, hotkey persistence, resolution subdialog, or launcher visual surface in this mechanism.
- Do not create consumers for AutoScroll, ScrollMethod, SidebarCameoText, repeat, shuffle, latency, or InGameMusic without separate verified owner evidence; retain and round-trip them now.
- Do not rewrite `config.toml` from RA2MD values or make it a second authority. Existing VERA-only graphics features remain separate; the interactive shell client size comes from the retail profile boundary.
- Do not let RA2MD settings enter deterministic sim serialization/hashing.
- Do not normalize the entire profile file or add read-only Allow* keys.
- Do not implement unproven duplicate/case behavior or native invalid-display failure recovery.
- Do not change OS `CloseRequested` persistence until its active native owner is traced.

## Blockers And Follow-Ups

- Core implementation is unblocked. Next gate: `/brainstorm` the Rust-native process-profile ownership and startup injection shape, then `/design-review` before code.
- `BLOCKED / EXACTIFICATION-RESIDUAL`: exact case and duplicate selection. If later required, use `/re-investigate INIClass profile case and duplicate selection`; active exact-spelling/no-duplicate profile work proceeds now.
- `BLOCKED / UNKNOWN-RISK`: OS close persistence. Revisit during the Phase 14 reverse audit; use `/re-investigate WinMain WM_CLOSE OptionsClass WriteToINI owner` only if it remains a common player exit gap.
- After this contract gate, correct the stale case/duplicate sentence in `OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md`; this contract records the correction but cannot modify a research document under its hard gate.

## Source Ledger

- Active binary/report addresses: listed under Evidence Baseline and the source ledger of `docs/research/OPTIONS_PROFILE_TRANSACTION_GHIDRA_REPORT.md`.
- Retail profile: `C:\Users\enok\Documents\Command and Conquer Red Alert II\RA2MD.INI`, stable hash above.
- Rust startup/app: `src/main.rs:32-77`; `src/app/frontend/launch.rs:112-145`; `src/app/frontend/startup_options.rs:98-203`; `src/app/mod.rs:73-153`; `src/app/initialize.rs:35-75,346-423,560-618`; `src/app/handler.rs:31-35,107-109,159-162,250-258`; `src/app/state.rs:15-33`.
- Rust profile/UI persistence: `src/ui/shell/in_game_options_state.rs:17-53`; `src/app/persistence/options.rs:22-145`; `src/ui/tooltips.rs:77-82,200-205`; target-line calls cited in the research report.
- Rust audio: `src/app/audio_runtime.rs:13-18`; `src/audio/music.rs:378-418,569-607`; `src/audio/sfx.rs:214-230,270-358,413-544,673-674`; `src/app/in_game.rs:552-569`.
- Rust parser/writer: `src/rules/ini_value.rs:40-63,170-204`; `src/rules/ini_parser.rs:62-85,227-330`; `src/util/ini_writer.rs:18-150`.

## Ghidra Annotation Candidates

| Address/source | Current metadata | Proposed mutation | Proof | Status |
|---|---|---|---|---|
| `0x005FB050` | `FUN_005fb050` | rename to `OptionsClass__ApplyNetworkProfile` | body consumes Options Socket/DestNet; ReadFromINI invokes it after Network fields | deferred — no sync authorized |
| `0x00A8EB78` | `g_ExtraAnimationsEnabled` | comment that this is persisted `OptionsClass::DetailLevel +0x18`, launcher coarse 0/2, in-game direct 0..2 | SetDefaults/Read/Write and both apply bodies bind the same address | deferred — no sync authorized |
| `0x00A8EB64` | `DAT_00A8EB64` | label `g_OptionsDifficulty` | Options base+4, Difficulty read/write, launcher 0x50F write, campaign consumer switch | deferred — no sync authorized |
