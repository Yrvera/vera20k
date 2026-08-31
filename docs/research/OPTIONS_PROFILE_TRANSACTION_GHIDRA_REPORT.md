# Options Profile Transaction — Ghidra Research Report

**Address(es):** `0x005FA350` (defaults), `0x005FA620` (read/apply), `0x005FAD10` (write), `0x004E1DE0` / `0x0055FAA0` (dialog apply owners)

**Investigation Mode:** exhaustive-slice

**Claimed Scope:** the active-YR process-owned `OptionsClass` construction, early screen-only read/fallback, later full profile read/apply, dialog-to-object updates, and serialized `RA2MD.INI` write transaction for persisted `[Options]`, `[Video]`, and `[Audio]` settings.

**Non-Scope:** launcher/in-game dialog paint and geometry; the keyboard editor and `KEYBOARDMD.INI`; renderer-specific mode enumeration/failure recovery after the chosen screen pair; semantic use of `[Network]`; campaign/skirmish rules that merely reuse names such as `GameSpeed`; developer-only controls.

**Confidence:** High for the claimed slice

**Active in YR:** Yes

## 1. Overview

Retail YR owns user settings in one static `OptionsClass` object at `0x00A8EB60`. Static initialization seeds defaults, startup reads the screen pair early enough to select the display mode, `Init_Game @ 0x0052BA60` later performs the full typed read and immediately applies audio/action-line/network side effects, and accepted options dialogs mutate the same object before `OptionsClass__WriteToINI @ 0x005FAD10` serializes it.

The Phase 14 implementation gap is not six isolated checkbox keys or one music-volume key. The missing mechanism is a process-owned profile transaction: one authoritative in-memory value set, an early screen-only read/fallback followed by the later full typed read, explicit application to existing runtime consumers, and one byte-preserving commit boundary. The current Rust implementation instead parses/discards startup video values, hydrates only two `[Options]` fields, reads only `ScoreVolume`, and writes settings from several unrelated paths.

The prior-state decision was **partial report -> gaps + verification only**. `OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md` and `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` supplied navigation hypotheses, but both remain `NEVER_AUDITED`; this investigation re-derived every load-bearing field, parser, setter, owner, and write sequence below from the active retail executable.

## 2. Class Layout / Key Offsets

`OptionsClass__ShowLauncherDialog @ 0x0055FC80` copies `0x2E` dwords (`0xB8` bytes) from `0x00A8EB60` to backup object `0x00ABCE70`. The table intentionally distinguishes persisted fields from other object bytes.

| Offset | Type | Constructor/default | Persisted name / role | Tiny-detail evidence |
|---:|---|---:|---|---|
| `+0x00` | `i32` | `3` | `[Options] GameSpeed` | direct first store in `0x005FA350` |
| `+0x04` | `i32` | `1` | `[Options] Difficulty` | launcher control `0x50F` writes this exact field; campaign setup later switches on it |
| `+0x08` | `i32` | zero-initialized | `[Options] CampDifficulty` | `0x005FA350` does **not** store it; the static BSS zero is the default |
| `+0x0C` | `i32` | `0` | `[Options] ScrollMethod` | read/written without clamp |
| `+0x10` | `i32` | `3` | `[Options] ScrollRate` | dialog position is inverted as `6 - position` |
| `+0x14` | byte | `1` | `[Options] AutoScroll` | typed bool, no normalization beyond `ReadBool` |
| `+0x18` | `i32` | `2` | `[Options] DetailLevel` | Ghidra's `g_ExtraAnimationsEnabled` label aliases this field; launcher collapses nonzero to `2`, in-game stores `0..2` directly |
| `+0x1C` | byte | `1` | sidebar side | reset to `1` again during every full read; intentionally not serialized |
| `+0x1D` | byte | `1` | `[Options] SidebarCameoText` | typed bool |
| `+0x1E` | byte | `1` | `[Options] UnitActionLines` | typed bool; read/apply calls `TechnoClass__SetDrawHealthBarsFlag` |
| `+0x1F` | byte | `0` | `[Options] ShowHidden` | typed bool |
| `+0x20` | byte | `1` | `[Options] ToolTips` | in-game apply additionally calls tooltip owner when display and game are active |
| `+0x24` | `i32` | `-1` | `[Video] ScreenWidth` | persisted screen sentinel |
| `+0x28` | `i32` | `-1` | `[Video] ScreenHeight` | persisted screen sentinel |
| `+0x2C` | `i32` | `800` | built-in fallback width | not written; resolves an old report's “unknown padding” |
| `+0x30` | `i32` | `600` | built-in fallback height | not written |
| `+0x34` | byte | `0` | `[Video] StretchMovies` | final value is requested bool AND global capability byte `0x008A0DEE == 1` |
| `+0x35` | byte | `0` | read-only `AllowHiResModes` | read but deliberately omitted by writer |
| `+0x36` | byte | `0` | read-only `AllowVRAMSidebar` | read but deliberately omitted by writer |
| `+0x38` | `f32` | `0.7` | `[Audio] SoundVolume` | setter upper-clamps only and scales by `16384.0` |
| `+0x3C` | `f32` | `0.7` | `[Audio] VoiceVolume` | setter drives Vox `*255.0` and voice mixer `*16384.0` |
| `+0x40` | `f32` | `0.4` | `[Audio] ScoreVolume` | setter drives music mixer `*16384.0` and Theme `*255.0` |
| `+0x44` | byte | `0` | `[Audio] IsScoreRepeat` | mirrored into global `0x00A83D20` on read |
| `+0x45` | byte | `1` | `[Audio] InGameMusic` | written after `SoundLatency`, despite being read before it |
| `+0x46` | byte | `0` | `[Audio] IsScoreShuffle` | mirrored into global `0x00A83D22` on read |
| `+0x48` | `u16` | `9` | `[Audio] SoundLatency` | integer read narrows to 16 bits; it is read after NetID decoding |
| `+0x4A` | `u16` | `0xFFFF` | `[Network] Socket` | network semantic use is non-scope, but writer ownership is verified |
| `+0x4C` | `i32` | `-1` | first decoded NetID component | valid range is inclusive `-1..7`, else reset to `-1` |
| `+0x50` | `i32` | `-1` | second decoded NetID component | same validation |
| `+0x54` | `i32` | `0` | `[Network] NetCard` | integer read/write |
| `+0x58` | `char[64]` | empty | `[Network] DestNet` | `ReadString` receives capacity `0x40` |

`SetDefaults` also initializes non-persisted object bytes at `+0x98..+0xB4`; they are outside this transaction's claimed field semantics even though launcher backup copies them.

## 3. Core Logic

### 3.1 Construction and process ordering

The static object is zeroed before `OptionsClass__SetDefaults @ 0x005FA350`; this matters for `CampDifficulty`, because the function never explicitly stores `+0x08`. A static-initializer thunk loads `ECX = 0x00A8EB60` and tail-calls the default function. The backup object at `0x00ABCE70` is initialized through the same function.

`WinMain @ 0x006BB9A0` reads `RA2MD.INI [Video] ScreenWidth/ScreenHeight` before full `Init_Game`. If either retained field equals `-1`, the pair is replaced together. The ordinary branch tests video memory against exactly `0x200000`: `<= 2 MiB` selects `640x480` at `0x006BD985..0x006BD990`; more selects `800x600` at `0x006BD99C..0x006BD9A6`. A separate old-platform gate also selects `800x600`. Explicit non-`-1` values bypass this pair fallback.

The later full read is a second semantic read of the same Video keys, not merely the continuation of the early pass. Its missing-key defaults are the post-fallback object fields. Therefore a profile containing only `ScreenWidth=640` first produces `(640,-1)`, selects an `800x600` startup window through the pair fallback, and then the full read produces the retained/written pair `(640,600)`. This two-pass distinction is load-bearing for partial hand-edited screen pairs even when Rust performs only one physical file read and parses one immutable snapshot.

Later, active `Init_Game @ 0x0052BA60` loads MIX/campaign substrate, calls `OptionsClass__ReadFromINI` at the `0x0052C630` function body, and only then constructs tactical/audio-list consumers. The call rereads ScreenWidth/ScreenHeight along with the other fields using their then-current values as defaults. This is a process-load operation, not a simulation tick, replay datum, or save-state field.

### 3.2 Full read and immediate side effects

The exact read order in `0x005FA620` is:

1. `[Options]` integer/bool fields through `ToolTips`.
2. Force sidebar-side byte `+0x1C = 1` independently of INI content.
3. `[Video]` screen pair and video booleans.
4. `[Audio]` Sound, Voice, Score, repeat, shuffle, InGameMusic.
5. Reset and decode `[Network] NetID`.
6. Read `[Audio] SoundLatency`, then Network Socket/NetCard/DestNet.
7. Synchronize action-line state and apply Network Socket/DestNet.

Every reader receives the object's current field as its missing-key default. Missing files/sections/keys therefore retain constructor/current values. A present malformed decimal integer goes through C `atoi` and ordinarily becomes `0`; a present malformed bool returns the current default; a present float is parsed as `f32`, widened to `f64`, and multiplied by `0.01` if `%` occurs anywhere.

Only three integer fields clamp after parsing: `Difficulty` saturates signed values to inclusive `0..4`; `CampDifficulty` and `DetailLevel` saturate to inclusive `0..2`. `GameSpeed`, `ScrollMethod`, and `ScrollRate` remain unbounded at this layer. The clamp sequence is upper first, then negative-to-zero.

Each audio value is upper-clamped only: comparison `1.0 <= value` stores `1.0`; negative values survive. Sound is stored at `+0x38` and immediately sent as `ftol(value * 16384.0)` to the SFX interpolator. Voice additionally sends `ftol(value * 255.0)` to the Vox global owner before `ftol(value * 16384.0)` reaches the voice interpolator. Score sends `ftol(value * 16384.0)` to music and `ftol(value * 255.0)` to Theme, whose callee caps at 255. The binary constants were cold-read as `0x46800000` (`16384.0f`), IEEE double `255.0`, and IEEE double `1.0`.

### 3.3 Dialog application

`OptionsClass__ShowInGameDialog @ 0x004E1D00` converts modal-pump termination into result `2`. Only result `1` calls `OptionsClass__ApplyFromInGameDialog @ 0x004E1DE0`, then `WriteToINI`; result `2` closes without applying or writing.

The in-game apply order is GameSpeed, ScrollRate, DetailLevel, UnitActionLines, ShowHidden, ToolTips, then Difficulty only when `g_GameActive == 0`. Every `GetDlgItem` null result leaves the corresponding field unchanged. GameSpeed uses `6 - slider_position`; while an active non-campaign/non-mode-5 match is running, a changed speed also queues command type `0x0D` if the command ring has fewer than `0x80` entries. ScrollRate also uses `6 - position`. DetailLevel stores the raw position and refreshes display cells only on change. Checkbox truth is **exactly** `BM_GETCHECK == 1`, not merely nonzero. ToolTips immediately calls the live tooltip gate only when the display exists and the game is active.

`OptionsClass__ShowLauncherDialog @ 0x0055FC80` disables `g_GameActive`, copies `0xB8` bytes to its backup at the start of every outer primary-dialog pass, pumps the primary modal, and **always** calls `OptionsClass__ApplyFromLauncherDialog @ 0x0055FAA0` after the pump. Result `0x5CD` opens the resolution child then returns to a new primary pass; result `0x5CE` opens the keyboard child then returns; final exit writes once and restores the old `g_GameActive` byte. There is no final-result write suppression in this owner.

Launcher apply maps control `0x50F` to `Difficulty`; control `0x52A` to `ScrollRate = 6 - position`; `0x601/0x604/0x602` to UnitActionLines/ShowHidden/ToolTips; `0x52F/0x532/0x536` to Score/Sound/Voice as `position * 0.1f`. Control `0x52B` is a coarse DetailLevel surface: zero becomes `0`, any nonzero becomes `2`, and a change triggers the same display refresh. The score path contains an easily missed side effect: when the pre-update global score value is exactly `0.0`, it selects current-or-default song, queues it, then calls Theme stop with flag `0`.

### 3.4 Serialized write

`OptionsClass__WriteToINI @ 0x005FAD10` emits these keys in exact order:

- Options: `GameSpeed`, `Difficulty`, `CampDifficulty`, `ScrollMethod`, `ScrollRate`, `AutoScroll`, `DetailLevel`, `SidebarCameoText`, `UnitActionLines`, `ShowHidden`, `ToolTips`.
- Video: `ScreenWidth`, `ScreenHeight`, `StretchMovies`.
- Audio: `SoundVolume`, `VoiceVolume`, `ScoreVolume`, `IsScoreRepeat`, `IsScoreShuffle`, `SoundLatency`, `InGameMusic`.
- Network: `Socket`, `NetCard`, `DestNet`, encoded `NetID`.

Integer helper `0x005275C0` receives mode `0`, selecting `%d`. Bool helper `0x00529560` writes literal lowercase `yes` or `no`. Float helper `0x005285B0` uses `%f`, producing the CRT's ordinary six-decimal representation for these values. The relevant bytes were cold-read as `%d`, `%f`, `no`, and `yes`. The three read-only Video flags (`AllowHiResModes`, external-global `AllowModeToggle`, and `AllowVRAMSidebar`) are intentionally omitted.

The native writer also serializes Network, but interpreting or exposing those online values is outside this slice. A Rust-native profile writer may preserve untouched Network bytes instead of modeling online behavior; it must not delete or rewrite unrelated keys/sections while closing the Options/Video/Audio transaction.

## 4. INI Keys

The active retail profile was read twice without change: 1,647 bytes, CRLF, no BOM, SHA-256 `9A4093A7323472217B1F5CB0C5B886DEFA6A7027698D247955AB59513BB2EC9C`. Its values are user-mutated evidence of spelling/format only, **not** constructor defaults.

| Section/key | Type | Missing-key default | Read transform / active effect | Writer |
|---|---|---:|---|---|
| `Options/GameSpeed` | int | `3` | none | `%d` |
| `Options/Difficulty` | int | `1` | signed clamp `0..4` | `%d` |
| `Options/CampDifficulty` | int | `0` via static zero | signed clamp `0..2` | `%d` |
| `Options/ScrollMethod` | int | `0` | none | `%d` |
| `Options/ScrollRate` | int | `3` | none | `%d` |
| `Options/AutoScroll` | bool | true | first-character bool | `yes/no` |
| `Options/DetailLevel` | int | `2` | signed clamp `0..2` | `%d` |
| `Options/SidebarCameoText` | bool | true | first-character bool | `yes/no` |
| `Options/UnitActionLines` | bool | true | sync action-line gate after read/apply | `yes/no` |
| `Options/ShowHidden` | bool | false | stored profile gate | `yes/no` |
| `Options/ToolTips` | bool | true | in-game apply can update live tooltip owner | `yes/no` |
| `Video/ScreenWidth` | int | `-1` | paired startup fallback | `%d` |
| `Video/ScreenHeight` | int | `-1` | paired startup fallback | `%d` |
| `Video/StretchMovies` | bool | false | requested AND capability byte | `yes/no` |
| `Video/AllowHiResModes` | bool | false | stored at `+0x35` | omitted |
| `Video/AllowModeToggle` | bool | external zero-init global | display-mode branch | omitted |
| `Video/AllowVRAMSidebar` | bool | false | stored at `+0x36` | omitted |
| `Audio/SoundVolume` | float | `0.7f` | f32 parse, upper-only clamp, live SFX apply | `%f` |
| `Audio/VoiceVolume` | float | `0.7f` | f32 parse, upper-only clamp, Vox + voice apply | `%f` |
| `Audio/ScoreVolume` | float | `0.4f` | f32 parse, upper-only clamp, music + Theme apply | `%f` |
| `Audio/IsScoreRepeat` | bool | false | mirrors global | `yes/no` |
| `Audio/IsScoreShuffle` | bool | false | mirrors global | `yes/no` |
| `Audio/SoundLatency` | int -> u16 | `9` | truncates to low 16 bits | `%d` |
| `Audio/InGameMusic` | bool | true | stored profile gate | `yes/no` |

The active profile currently stores `GameSpeed=0`, `Difficulty=1`, `CampDifficulty=0`, `ScrollMethod=0`, `ScrollRate=4`, `AutoScroll=yes`, `DetailLevel=2`, `SidebarCameoText=yes`, `UnitActionLines=yes`, `ShowHidden=no`, `ToolTips=yes`, `ScreenWidth=640`, `ScreenHeight=480`, `StretchMovies=no`, `SoundVolume=0.700000`, `VoiceVolume=0.800000`, `ScoreVolume=0.600000`, `IsScoreRepeat=no`, `IsScoreShuffle=no`, `SoundLatency=9`, and `InGameMusic=yes`. No repository `ini/*.ini` file owns these user-profile sections. Same-named rules/art keys are different authorities.

The active retail profile uses exact retail spelling and contains no duplicate Options/Video/Audio sections or keys. Exact native case-variant and multi-duplicate CRC/qsort selection was not re-proven in this slice; current `IniFile` uses exact-case lookup and the first exact duplicate, so mod-profile case/duplicate behavior is an explicit exactification residual rather than a proven match. Ordinary values use the native typed-read rules already implemented in `src/rules/ini_value.rs`; the Options profile must reuse them rather than add a second parser.

## 5. Integration Points

| Stage | Owner / address | Ordering and effect |
|---|---|---|
| static initialization | thunk -> `0x005FA350` | seeds the singleton before normal startup; relies on BSS zero for fields not explicitly assigned |
| early display selection | `WinMain @ 0x006BB9A0`, branch `0x006BD94A..0x006BD9B5` | reads/chooses screen pair before window/display creation |
| full process load | `Init_Game @ 0x0052BA60`, call in body at `0x0052C630` | reads all persisted fields and immediately applies audio/action-line/network outputs |
| in-game accepted close | `0x004E1D00` -> `0x004E1DE0` -> `0x005FAD10` | apply first, write second; result `2` does neither |
| launcher options | Main pre-session case `5` -> `0x0055FC80` | apply after each primary pump; child dialogs loop; one final write |
| confirmed exit | Main pre-session case `6` | affirmative result calls full `WriteToINI` |
| gameplay tick/replay/save | none | profile is process/presentation state; GameSpeed changes enter the deterministic command seam separately |

## 6. Current Rust Implementation Status

| Surface | Current state at `origin/main` `054696bb91a1daf066915ecdc44364deadfba91e` | Delta |
|---|---|---|
| `src/app/frontend/startup_options.rs::RetailStartupOptions` | accurately parses early video switches/profile in tests, but `apply_ra2md_video_section` and `resolve_screen_size` have no production caller | retained startup profile and interactive window use missing |
| `src/app/mod.rs::App::new` | receives `RetailStartupOptions` but retains only the `-NOAUDIO` disposition | discards screen/profile fields before `resumed` |
| `src/app/initialize.rs::App::initialize` | creates a fixed `800x600` window before loading `GameConfig`; capture dimensions are an explicit higher-priority path | profile screen pair not consumed |
| `src/app/handler.rs::enter_shell_window_mode` | reapplies fixed `800x600` on every shell entry | retained profile size not reused |
| `src/ui/shell/in_game_options_state.rs::InGameOptionsState` | owns six UI fields with correct constructor defaults | incomplete process profile; match-presentation owner is too narrow for process settings |
| `src/app/persistence/options.rs` | boot reads only ScrollRate/DetailLevel; close writes six Options keys as `1/0` booleans | missing fields, wrong bool serialization, fragmented read/write transaction |
| `src/ui/tooltips.rs::TooltipService::set_enabled` | live gate exists | no production profile caller; boot always enables |
| target-line presentation gate | apply helper exists | initialized only after dialog close, not from profile before first match frame |
| `src/audio/music.rs::MusicPlayer` | reads/writes only `ScoreVolume`, clamped to `0..1` | duplicates profile I/O and lower-clamps contrary to native upper-only object semantics |
| `src/audio/sfx.rs::SfxPlayer` | one `0.7` master scales SFX, unit voice, and EVA together | no distinct Sound/Voice profile authority; live/queued output observation needs a separate audio mechanism |
| `src/app/in_game.rs::persist_settings_on_quit` | writes only ScoreVolume from selected main-menu quit paths | not a full profile commit; OS `CloseRequested` exits without persistence |
| `src/rules/ini_value.rs` | implements active `ReadInt`, `ReadBool`, and `ReadDouble` normal-domain semantics | reusable match; do not duplicate |
| `src/util/ini_writer.rs` | byte-preserving single/batch key transformations | usable primitive, but settings need one snapshot and one filesystem write |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OptionsClass__SetDefaults` | verified | `0x005FA350` decompile | none for persisted slice |
| static singleton and backup initialization | verified | SetDefaults xrefs/thunks; `0x0055FC80` copy | none |
| early screen-pair fallback | verified | assembly `0x006BD94A..0x006BD9B5` | renderer-specific failed-mode recovery is non-scope |
| `OptionsClass__ReadFromINI` | verified | `0x005FA620` full decompile | none |
| `CCINIClass__ReadInt` | verified | cold re-decompile `0x005276D0` | none |
| `CCINIClass__ReadBool` | verified | cold re-decompile `0x005295F0` | none |
| `CCINIClass__ReadDouble` | verified | cold re-decompile `0x005283D0`; typed-INI study | malformed `%f` no-conversion ABI artifact intentionally not reproduced |
| `OptionsClass__SetScoreVolume` | verified | cold disassembly `0x005FA4A0` | none |
| `OptionsClass__SetSoundVolume` | verified | `0x005FA510` decompile | none |
| `OptionsClass__SetVoiceVolume` | verified | cold disassembly `0x005FA590` | none |
| `OptionsClass__ApplyFromInGameDialog` | verified | `0x004E1DE0` full decompile | dialog paint/geometry non-scope |
| `OptionsClass__ShowInGameDialog` result gate | verified | `0x004E1D00` full decompile | none |
| `OptionsClass__ApplyFromLauncherDialog` | verified | `0x0055FAA0` full decompile | child dialog UI behavior non-scope |
| `OptionsClass__ShowLauncherDialog` loop/write owner | verified | `0x0055FC80` full decompile | exact backup consumers inside UI procedures non-scope |
| `OptionsClass__WriteToINI` | verified | `0x005FAD10` full decompile | none for Options/Video/Audio serialization |
| int/bool/float write helpers | verified | `0x005275C0`, `0x00529560`, `0x005285B0`; constant bytes | none |
| `Options/*` key set | verified | `0x005FA620`, `0x005FAD10`, table in Section 4 | none |
| `Video/ScreenWidth`, `ScreenHeight`, `StretchMovies` | verified | same plus WinMain assembly | none |
| read-only Video allow flags | verified | `0x005FA620` and absence from `0x005FAD10` | semantic platform use outside slice |
| `Audio/*` key set | verified | `0x005FA620`, setters, `0x005FAD10` | downstream Rust audio split is implementation work |
| Network serialization participation | verified | `0x005FA620`, `0x005FAD10`, `0x005FB050` | online semantics explicitly non-scope |
| current retail profile shape | verified | stable SHA-256 and exact file read | values are mutable, not defaults |
| current Rust ownership/callers | verified | repository scan listed in Section 6 | implementation pending |
| tick-cycle, pause, replay, save/restore relevance | verified | owner call graph; no tick/save ownership | none; process-only |
| TS legacy filter | verified | live WinMain/Init_Game/dialog paths and active keys | allow flags remain conditional platform knobs, not TS-only defaults |

The final zero-add pass re-decompiled the five primary functions (`0x005FA350`, `0x005FA620`, `0x005FAD10`, `0x0055FAA0`, `0x0055FC80`) and their typed readers/setters without adding an unresolved in-scope question. Two cold spot checks used assembly rather than decompiler output: `0x005FA4A0` reconfirmed upper-only clamp plus `16384`/`255` scaling, and `0x005FA590` reconfirmed Voice's `255`-then-`16384` side-effect order.

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-01 — What initializes the live object? -> Static zero initialization followed by SetDefaults on 0x00A8EB60.` (evidence: `0x005FA350`; singleton xref thunk)
- `[RESOLVED] OQ-02 — Which fields depend on zero initialization rather than an explicit store? -> CampDifficulty +0x08 and other false/zero bytes not touched by SetDefaults retain BSS zero; CampDifficulty is the persisted load-bearing case.` (evidence: `0x005FA350`)
- `[RESOLVED] OQ-03 — What is the complete persisted field layout? -> Section 2 records every Options/Video/Audio/Network field through DestNet.` (evidence: `0x005FA350`, `0x005FA620`, `0x005FAD10`)
- `[RESOLVED] OQ-04 — When does the screen pair affect startup? -> WinMain performs the early screen-only read before display creation; either -1 replaces the live pair together, while the later full read can reread a present single key over that fallback for retained/write state.` (evidence: `0x006BD94A..0x006BD9B5`; full-read call at `0x0052C630`)
- `[RESOLVED] OQ-05 — What chooses 640x480 versus 800x600? -> Old-platform gate selects 800x600; otherwise video memory <=0x200000 selects 640x480, greater selects 800x600.` (evidence: `0x006BD95A..0x006BD9A6`)
- `[RESOLVED] OQ-06 — When does the full read run? -> Init_Game after the early screen read/fallback and after MIX/campaign load, before later tactical/audio-list initialization; it rereads the Video pair as part of the complete profile.` (evidence: `Init_Game @ 0x0052BA60`, call in body beginning `0x0052C630`)
- `[RESOLVED] OQ-07 — What do absent file/section/key cases do? -> Typed readers return the supplied current field, preserving defaults/current values.` (evidence: `0x005276D0`, `0x005295F0`, `0x005283D0`)
- `[RESOLVED] OQ-08 — What does a present malformed integer do? -> Decimal path uses C atoi and normally returns zero; hex no-conversion retains its initialized default.` (evidence: `0x005276D0`; typed-INI study)
- `[RESOLVED] OQ-09 — What does a malformed/empty bool do? -> Only first character 1/T/Y or 0/F/N decides; otherwise current default remains.` (evidence: `0x005295F0`)
- `[RESOLVED] OQ-10 — What is float precision/percent behavior? -> sscanf %f to f32, widen to f64; any percent byte multiplies by 0.01.` (evidence: `0x005283D0`)
- `[RESOLVED] OQ-11 — Which integers clamp? -> Difficulty 0..4; CampDifficulty and DetailLevel 0..2; the others do not at read layer.` (evidence: `0x005FA620`)
- `[RESOLVED] OQ-12 — Do audio volumes lower-clamp? -> No; all three only saturate at >=1.0, so negative values survive.` (evidence: `0x005FA620`, `0x005FA4A0`, `0x005FA510`, `0x005FA590`)
- `[RESOLVED] OQ-13 — What are exact audio scaling and order? -> Sound 16384; Voice 255 then 16384; Score 16384 then 255, all via ftol.` (evidence: cold disassembly `0x005FA4A0`, `0x005FA590`; constants `0x007EF38C`, `0x007EAA50`)
- `[RESOLVED] OQ-14 — Is DetailLevel the global labeled ExtraAnimations? -> Yes for this object address; launcher writes 0/2 while in-game writes the raw slider and both refresh only on change.` (evidence: `0x0055FAA0`, `0x004E1DE0`, base `0x00A8EB60 + 0x18`)
- `[RESOLVED] OQ-15 — What does a missing dialog control do? -> Its field is skipped and remains unchanged.` (evidence: null gates in `0x0055FAA0`, `0x004E1DE0`)
- `[RESOLVED] OQ-16 — What is checkbox truth? -> Exactly BM_GETCHECK result == 1.` (evidence: both apply functions)
- `[RESOLVED] OQ-17 — How do slider inversions work? -> GameSpeed and ScrollRate store 6 minus raw position; launcher audio stores f32(position * 0.1).` (evidence: both apply functions)
- `[RESOLVED] OQ-18 — What is launcher control 0x50F? -> Difficulty at Options +0x04, consumed by campaign setup; it is not GameSpeed.` (evidence: `0x0055FAA0`, `Main__PrepareSession @ 0x0052D9A0`)
- `[RESOLVED] OQ-19 — Which in-game result commits? -> Result 1 applies then writes; pump termination becomes result 2 and skips both.` (evidence: `0x004E1D00`)
- `[RESOLVED] OQ-20 — Does launcher final result suppress writing? -> No; outer owner always performs one final WriteToINI after subdialog loops.` (evidence: `0x0055FC80`)
- `[RESOLVED] OQ-21 — What is exact key order and formatting? -> Section 3.4; integers %d, bool lowercase yes/no, floats %f.` (evidence: `0x005FAD10`, write helpers and constant bytes)
- `[RESOLVED] OQ-22 — Which read keys are deliberately not written? -> AllowHiResModes, AllowModeToggle, AllowVRAMSidebar, plus forced sidebar-side byte.` (evidence: read/write diff)
- `[DEFERRED] OQ-23 — How do native profile case variants and multi-duplicates select a value?` (category: `out-of-scope`; reason: the active retail profile uses exact spelling with no duplicates, the older typed-INI prose conflicts with current exact-case/first-exact-duplicate Rust, and this slice did not re-prove loader normalization or CRC/qsort duplicate selection; next-step-if-pursued: narrow live investigation of INI load normalization plus duplicate section/entry ordering)
- `[RESOLVED] OQ-24 — What happens at zero and maximum values? -> Zero persists normally; bounded difficulty/detail clamp as above; audio >=1 saturates to 1; other integers remain layer-unbounded or narrow to u16.` (evidence: `0x005FA620`)
- `[RESOLVED] OQ-25 — What happens on an empty profile? -> Options/Audio constructor defaults survive, while the unset screen pair takes the verified startup fallback before the later full read; read-time side effects apply the resulting values.` (evidence: SetDefaults + WinMain fallback + current-as-default Read calls)
- `[RESOLVED] OQ-26 — Is first/last tick or paused state relevant? -> No profile tick exists; dialog owner temporarily controls g_GameActive, and only the separate GameSpeed command seam enters deterministic match state.` (evidence: call graph; `0x004E1DE0`)
- `[RESOLVED] OQ-27 — Is replay/save restore relevant? -> No; the profile is process/presentation state and is not serialized into match saves/replays.` (evidence: owner call graph and Rust ownership scan)
- `[RESOLVED] OQ-28 — Which current Rust owners are partial? -> startup options, match-presentation in-game state, music, SFX, tooltip, target-line, and quit paths are fragmented as listed in Section 6.` (evidence: repository scan at origin/main SHA)
- `[RESOLVED] OQ-29 — Which INI authority owns these values? -> loose user RA2MD.INI only; same-named rules/art values are separate authorities.` (evidence: active profile plus `ini/*.ini` scan)
- `[DEFERRED] OQ-30 — What exact renderer API fallback follows an explicit invalid non--1 screen pair?` (category: `requires-different-system-context`; reason: display-device enumeration/failure recovery is beyond the profile transaction and does not change load/persist semantics; next-step-if-pursued: investigate the WinMain display-create branch from `0x006BD9B5` through failed mode setup)
- `[DEFERRED] OQ-31 — Which launcher UI action consumes/restores the 0xB8-byte backup?` (category: `out-of-scope`; reason: backup consumption belongs to launcher reset/cancel UI behavior, while the owner-level apply/write transaction is resolved; next-step-if-pursued: trace backup-object xrefs inside resource 0xD5 procedures)
- `[DEFERRED] OQ-32 — What do Network Socket/DestNet/NetID mean online?` (category: `requires-different-system-context`; reason: the Phase 14 online policy boundary is a later mechanism; next-step-if-pursued: investigate network service initialization and session protocol consumers)

Adversarial corner cases answered from the drained log:

1. If only one screen dimension is missing during the early pass, both live dimensions fall back together for window selection; the later full read then reapplies any present single key, so the retained/written pair can differ from the startup window pair (OQ-04/OQ-06).
2. A negative audio value is retained and applied rather than lower-clamped (OQ-12).
3. A missing dialog control does not zero its field (OQ-15).
4. A malformed bool retains the current default while a malformed decimal int usually becomes zero (OQ-08/OQ-09).
5. Entering a launcher child dialog applies the primary controls first, then a new primary pass backs up the now-current object, and only final exit writes (OQ-20).

Four of 32 entries are deferred (12.5%), all outside the ordinary exact-spelling/no-duplicate profile mechanism; no material in-scope unknown remains.

## 9. Visual/UI Composition Ledger

This slice covers dialog-to-profile semantics, not visual composition. Paint order, assets, frames, palette paths, and rectangles are explicitly non-scope; no visual claim is made.

## 10. Implementation Handoff

| Classification | Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| milestone-blocking | one process-owned Options/Video/Audio profile is defaulted, receives an early screen-only pass and later full typed pass, applies consumers, and commits as one settings transaction | `0x005FA350`, `0x005FA620`, `0x005FAD10`, WinMain and `0x0052C630` owner call | missing; several partial readers/writers | `src/app/`, existing persistence/options and startup-options surfaces | introduce one process-lifetime profile authority; parse one immutable snapshot, emulate both native passes, and retain the later full-read result across shell/match transitions | absent `RA2MD.INI` yields exact Options/Audio defaults and the verified ordinary-host screen fallback; a populated profile hydrates every modeled field; one quit commit preserves unrelated bytes and writes native spellings | do not create a second parser, collapse the two read points, duplicate field owners, or put profile state in `sim/` |
| milestone-blocking | early interactive screen pair is consumed before window creation and reused for shell mode; either sentinel replaces the live pair, then the full read can reapply present Video keys to retained state | WinMain `0x006BD94A..0x006BD9B5`; full read call at `0x0052C630` | parser exists but production discards it; window fixed 800x600 | `src/main.rs`, `src/app/mod.rs`, `src/app/initialize.rs`, `src/app/handler.rs` | carry both the early effective window pair and the later retained profile result while preserving capture dimensions as higher-priority automation input | complete 640x480 creates/re-enters 640x480; width-only 640 selects/re-enters 800x600 but retains/writes 640x600; capture still uses requested dimensions | do not derive the startup window from the later reread or let profile dimensions alter sealed capture contracts |
| milestone-blocking | Options booleans and Detail/Scroll values apply before first match frame | `0x005FA620` end side effects; `0x004E1DE0` | only ScrollRate/Detail read; booleans default until dialog close | process profile, in-game UI state, tooltip service, target-line state | hydrate UI view from profile and seed live tooltip/target-line gates during initialization | `ToolTips=no` suppresses tooltip service before any hover; `UnitActionLines=no` suppresses target lines before opening Options | do not make UI state the primary process owner or wait for dialog close to apply boot values |
| compounding | Sound, Voice, and Score are distinct upper-only-clamped profile gains with different consumers/scales | `0x005FA620`, setters `0x005FA4A0/510/590` | Score only; one combined SFX/voice master | `src/app/audio_runtime.rs`, `src/audio/music.rs`, `src/audio/sfx.rs` | add distinct persistent sound and voice authorities and have future/live output observe the correct channel; keep score separate | Sound=0 with Voice=1 silences effects but not unit/EVA voice; Voice=0 with Sound=1 does the inverse; negative and >1 parser boundary tests match chosen native-safe policy | do not collapse SFX and voice or re-read RA2MD independently inside each player |
| compounding | accepted in-game dialog applies before write; pump/game-termination result 2 writes nothing | `0x004E1D00`, `0x004E1DE0` | six-key close writer exists, fragmented from profile | `src/app/persistence/options.rs`, in-game dialog close path | mutate process profile through UI view, apply consumers, then commit once only on accepted result | accept changes fields/effects/file in order; result 2 leaves object, effects, and file unchanged | do not serialize directly from transient widget state through repeated per-key filesystem writes |
| compounding | launcher owner applies controls after every primary pump and performs one final profile write after subdialog loops | `0x0055FAA0`, `0x0055FC80` | launcher options dialog is a placeholder | later launcher-controls mechanism | reuse the same process profile transaction and setters when launcher controls are implemented | opening resolution/keyboard preserves applied parent changes; final exit writes one coherent profile | do not invent a second launcher-only profile or assume every final result is cancel-with-rollback |
| exactification residual | bools serialize as lowercase yes/no; floats as six-decimal `%f`; read-only Video allow keys are not written | write helpers/constant bytes | Options bools currently `1/0`; independent Score writer | common profile writer | emit native lexical forms for owned keys and preserve omitted/unrelated keys byte-for-byte | golden CRLF profile shows yes/no and `0.700000`, no inserted Allow* keys, unchanged comments/Network/Skirmish sections | do not normalize the whole file or rewrite retail/user formatting outside touched values |
| unknown-risk | explicit invalid display values may reach platform-specific native fallback beyond the profile layer | deferred OQ-30 | Rust currently clamps dimensions to at least 1 in a test-only resolver | startup/window validation | use a documented Rust-native safety boundary until display-mode investigation is scheduled | zero/negative explicit values never panic or create an invalid wgpu surface; deviation is documented | do not claim exact native invalid-mode behavior without the separate display investigation |

### Stale Docs / Follow-up Docs

- Replace any statement that `OptionsClass +0x2C/+0x30` is unknown padding with: “built-in fallback screen width/height 800/600, initialized by `0x005FA350` and consumed by startup display selection.”
- Replace any statement that launcher control `0x52B` writes an unrelated extra-animation global with: “it writes `OptionsClass::DetailLevel @ +0x18`; launcher collapses nonzero to 2, in-game stores the raw 0..2 position.”
- Replace any statement that `0x005FB050` flushes RA2MD.INI with: “it applies Network Socket/DestNet state; file serialization is owned by `OptionsClass__WriteToINI @ 0x005FAD10`.”
- Qualify the older claim that the launcher backup is restored on cancel: the outer owner copy is verified, but the exact UI procedure that consumes/restores it remains outside this slice.

## 11. Ghidra Annotation Candidates

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x005FB050` | `FUN_005fb050` / prior prose sometimes calls it persistence flush | `OptionsClass__ApplyNetworkProfile` | rename | body applies Socket and parses DestNet from Options object; ReadFromINI calls it after Network reads | deferred — report-only, no sync authorized |
| `0x00A8EB78` | `g_ExtraAnimationsEnabled` | comment: `OptionsClass::DetailLevel (+0x18); launcher coarse 0/2, in-game direct 0..2` | comment | SetDefaults/Read/Write and both apply functions bind same address | deferred — report-only, existing label may have wider consumer history |
| `0x00A8EB64` | `DAT_00A8EB64` | `g_OptionsDifficulty` | label | base+4 field, launcher control 0x50F, read/write Difficulty, campaign setup switch | deferred — report-only, no sync authorized |

## Sources

- Active retail `gamemd.exe` at `C:\Users\enok\Documents\Command and Conquer Red Alert II\gamemd.exe`, x86 image base `0x00400000`.
- Ghidra decompilation/disassembly: `0x005FA350`, `0x005FA620`, `0x005FAD10`, `0x004E1D00`, `0x004E1DE0`, `0x0055FAA0`, `0x0055FC80`, `0x005276D0`, `0x005295F0`, `0x005283D0`, `0x005275C0`, `0x00529560`, `0x005285B0`, `0x005FA4A0`, `0x005FA510`, `0x005FA590`, `0x005FB050`, `Init_Game @ 0x0052BA60`, `Main__PrepareSession @ 0x0052D9A0`, WinMain branch `0x006BD94A..0x006BD9B5`.
- Raw constant reads: `0x007EF38C`, `0x007EAA50`, `0x007E1718`, `0x00817F6C`, `0x00825BD8`, `0x00825BF4`, `0x00825BF8`.
- Retail profile: `C:\Users\enok\Documents\Command and Conquer Red Alert II\RA2MD.INI` (stable SHA-256 above).
- Existing research read fully: `OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md`, `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`, `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, `INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`.
- Rust production surfaces at `origin/main` `054696bb91a1daf066915ecdc44364deadfba91e`: files named in Sections 6 and 10.
