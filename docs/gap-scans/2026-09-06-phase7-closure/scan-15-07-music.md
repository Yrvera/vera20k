# GSI-15.07 — Music catalog, selection and transitions (ThemeClass) — disparity scan

Read-only scan, 2026-09-05. Binary: gamemd.exe (image base 0x400000), Ghidra DB `testProsjekt`.
Rust: worktree `clean-slate-system-impl-891469` @ 4b89ef52 (`feature/phase-7-parity-close-284594`).
Every native claim below was read from the decompiled/disassembled body in this session unless marked
"doc-derived". The prior report `docs/research/MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` is
correct on the shell path but carries three field-label errors (corrected in §1.2).

---

## 1. Scope enumeration (native mechanisms, addresses, active-YR reachability)

### 1.1 ThemeClass singleton `g_Theme @ 0x00A83D10` (ECX literal at every call site)

| Offset | Meaning (verified from bodies) | Global alias |
|---|---|---|
| +0x00 | current playing index (`Score`), -1 none | 0xA83D10 |
| +0x04 | last requested index (`retained`; Next_Song's "previous") | 0xA83D14 |
| +0x08 | pending index: -1 none, -2 auto-pick, -3 hold, else index | 0xA83D18 |
| +0x0C | **score volume 0..255** (ctor 0x00720960 sets 0xFF; `Set_Volume` 0x00721290 clamps). Gates `AI` and `Play` (`0 < +0xC`). *Prior doc calls this "theme count" — wrong.* | 0xA83D1C |
| +0x10 | `IsScoreRepeat` (global repeat) | 0xA83D20 |
| +0x11 | fading flag | 0xA83D21 |
| +0x12 | `IsScoreShuffle` | 0xA83D22 |
| +0x18 / +0x24 | entry pointer array / entry count | 0xA83D28 / 0xA83D34 |
| +0x2C | StreamPlayer* (NULL when no audio device) | 0xA83D3C |

### 1.2 Per-theme entry (0x290 bytes; ctor 0x00720440 — DB label `TechnoClass__ClearChronoFields` is wrong; reader 0x00720480)

| Offset | Key | Default | Note |
|---|---|---|---|
| +0x000 | section key (`[Themes]` value) | | `From_Name` 0x00721210 matches this only — `Sound=` stems are **not** aliases |
| +0x100 | `Sound=` (leading `$`/`#` stripped) | "" | `.WAV` appended at play (`0x00844768`) |
| +0x200 | `Name=` (64 bytes, key @0x00817854) | | read only when `Normal=yes`. *Prior doc says "Side" — wrong.* |
| +0x280 | `Scenario=` int | 0 | |
| +0x284 | duration (float seconds) | 0 | written by Scan 0x007207F0 from the WAV header |
| +0x288 | `Normal=` bool | **1** | |
| +0x289 | `Repeat=` bool | 0 | |
| +0x28A | file-available flag | 0 | written by Scan 0x007207F0 (`CCFileClass::IsAvailable(Sound+".WAV")`); read by `Is_Allowed` |
| +0x28C | `Side=` int (key @0x00817334) | **-1** | *Prior doc says "Length-like" — wrong.* |

### 1.3 Functions

| Function | Address | Reached in stock skirmish? |
|---|---|---|
| Catalog load (`[Themes]` of **THEMEMD.INI only**, strings @0x00825D94/0x00825DA0) | 0x00720590 ← `Init_Game` 0x0052BA60 @0x0052C92A | yes (once) |
| Scan (availability + duration) | 0x007207F0 ← `Init_Game`, `Init_Mix_Files` 0x00530460 | yes |
| `From_Name` | 0x00721210 | yes |
| `Is_Allowed(idx)` | 0x00721140 ← only `Next_Song` | yes |
| `Next_Song(prev)` | 0x00720A80 ← only `AI` | yes |
| `Queue_Song(idx)` | 0x00720B20 | yes (13 callers, see 1.4) |
| `Play_Song(idx)` | 0x00720BB0 | yes |
| `Still_Playing` | 0x00720FD0 | shell exit + WOL lobby only |
| `Stop(fade)` | 0x00720EA0 | yes |
| `AI` | 0x007209D0 ← `AudioSystem__Pump` 0x00406F70 (rate-gated >33 ms) ← `Network_ServiceLoop` 0x0048D080 | yes, every screen (menu, loading, in-game, pause, score, inactive window) |
| `Set_Volume` | 0x00721290 ← `OptionsClass__ReadFromINI` 0x005FA620, options apply 0x005FB160 | yes |
| Theme::Load (savegame) | 0x00721040 | save/load only |
| StreamPlayer `PlayFile` / `IsPlaying` / completion | 0x00407B60 / 0x00408070 (`flags & 1`) / 0x00402B80 clears bit 0 when the DirectSound buffer drains | yes |
| Fade primitives | 0x004080C0 → `VolumeInterp__SetTarget` (1000 ms), 0x004080D0 = "interp not yet at target" | yes |

### 1.4 Skirmish-flow call sites (verified bodies)

- **Shell entry / every return to the main menu** — `Main__PrepareSession` 0x0052D9A0: `Play(From_Name("INTRO"))` at 0x0052D9B4 and at label 0x0052DCB6 (every loop pass that is not mode 7/8). Credits (case 0xF): `Show_Credits` then `Queue_Song(INTRO)`. Exit (case 7): `Stop(1)`, wait while `Still_Playing` ≤ 3000 wall buckets, then `Stop(0)`.
- **Scenario start** — `ScenarioClass__Start_Scenario` 0x00683AB0: `Play(From_Name("LOADING"))` **before** `Read_Scenario` (skirmish included; the movie branch `Stop(0)` needs `[Basic] Intro/Brief`, campaign only). After a successful read: `Scen+0x1C70 == -1` (no `[Basic] Theme=`) → `Stop(fade=1)`; else `Queue_Song(idx)`. Then `DAT_00A8E378 = 1` (scenario-running).
- **Every frame in-game** — `Main_Tick` 0x0055D360 head, when `DAT_00A8E378 == 1` and the device exists: `cur = +0x4, or +0x8 if +0x4 == -1`; if `Options.InGameMusic (0x00A8EBA5) == 0` and `cur != -3` → `Stop(1)` + `Queue_Song(-3)`; else if `cur == -1` → `Queue_Song(-2)`. This is what starts the first gameplay track after `Start_Scenario`'s `Stop(1)`.
- **Scenario end** — `ScoreDialog__WndProc` 0x005C9B10 plays `SCORE`; `GameExit__BattleControlTerminated` 0x00686570 `Stop(1)`; restart 0x006863E0 `Stop(1)` then `Start_Scenario`.
- **Launcher options** 0x0055FAA0 (score volume 0 → `Queue(cur)` + `Stop(0)`), in-game **Sound options dialog** (unanalysed code 0x006B6500–0x006B69F8, reached from Options→Sound; see §2.9).
- Not skirmish: WOL lobby music 0x0077B2A0/0x0077E110 (`[WOnline] LobMusic`, forces shuffle=1 inside the lobby), WDT ctor 0x0076C6E0, campaign map-select 0x005AE100/0x004F2300/0x0076EEB0, CD-swap prompt 0x00479110, movie player 0x005BFF60, trigger action "play music" @0x006DE8F0 (`Queue(action+0x90)`), team-mission re-queue after a movie @0x006E9944.

### 1.5 Stock YR catalog (`ini/thememd.ini`, `;`-truncated values are empty and skipped because `CCINIClass::ReadString` returns 0)

Index: 0 INTRO (Drok, Normal=no, Repeat=yes) · 1 SCORE (ScoreX, no/yes) · 2 LOADING (Bully, no/yes) · 3 CREDITS (OptionX, no/yes) · 4 RA2Options (**no section**: Sound="", Normal=1, `.WAV` unavailable → never allowed) · 5 BrainFreeze · 6 Drok · 7 Deceiver · 8 PhatAttack · 9 BullyKit (Bully) · 10 DefendTheBase · 11 Tactics · 12 TranceLVania. Count 13; playable playlist = indices 5..12, catalog order. No `Side=`/`Scenario=` keys anywhere in stock.

### 1.6 Rust owners

`src/audio/theme.rs` (ThemeRuntime: catalog, slots, Next/Queue/Play/Stop/AI), `src/audio/music.rs` (rodio output), `src/app/audio_runtime.rs` (gates + action application), `src/app/loading/transitions.rs:420` (Start_Scenario request), `src/app/match_runtime/sim_tick.rs:1124` (AI poll), `src/app/shell_main_menu.rs:866` (INTRO maintenance), `src/app/in_game.rs:41,599,615`, `src/app/frame.rs:104-108`, `src/app/match_runtime/scenario_exit.rs:226` (SCORE), `src/app/persistence/options_profile.rs:64-66,270-273` (IsScoreRepeat/IsScoreShuffle/InGameMusic parsed), `src/app/input/in_game_options.rs:80-105` (Sound button KD-7 stub).

---

## 2. Per-mechanism findings

### 2.1 Catalog source and identity — **DRIFT (demonstrated)**
- Native: only `THEMEMD.INI` is read (§1.3); entries are index-keyed; INTRO (idx 0) and Drok (idx 6) are distinct entries sharing `Sound=Drok`.
- Rust `theme.rs:173-198`: loads **`theme.ini` then `thememd.ini`** and unions their `[Themes]` playlists. `ini/theme.ini` `[Themes]` declares 14 RA2 tracks with `Normal=yes` (Grinder, Power, Fortification, InDeep, Tension, EagleHunter, IndustroFunk, 200Meters, BlowItUp, Destroy, Burn, Motorized, HM2 …). Resulting Rust playlist = 14 RA2 tracks followed by the 8 YR tracks; native = the 8 YR tracks only. Identity is the `Sound` stem (`aliases`, `resolve_track_name`), so INTRO and [Drok] collapse into one track.
- Trigger: every skirmish once the playlist advances. Effect: RA2 tracks play in YR skirmish; track order differs from native.

### 2.2 Per-theme `Repeat=` — **DRIFT (demonstrated)**
- Native: `Play_Song` 0x00720BB0 writes `+0x8 = idx` only when `+0x10 == 1` or `entry[idx]+0x289 == 1`; `Next_Song` returns `prev` unchanged under the same test. Keyed by entry index.
- Rust `theme.rs:628-644` `merge_repeating_tracks` keys repeat by **stem**: `[INTRO] Repeat=yes` marks stem `DROK`, so the playlist entry `[Drok]` (native Repeat=0) repeats forever once reached; `track_repeats` (`theme.rs:222`) then makes `next_song` (`theme.rs:437-441`) return it indefinitely.
- Combined with 2.3 this means: a skirmish entered from the main menu keeps Drok (the menu theme) looping for the whole match. Trigger: every skirmish. Effect: no playlist at all in-game.

### 2.3 Start_Scenario hand-off — **DRIFT (demonstrated)** — earliest divergence of the in-game music flow
- Native (§1.4): `Play(LOADING)` replaces INTRO with a hard stop when loading begins (Bully loops under the loading screen); after the read, no `[Basic] Theme=` → `Stop(fade=1)` (1000 ms fade, slots := -1); `Main_Tick` then issues `Queue_Song(-2)`; `AI` waits for the fade (`+0x11` set, 0x004080D0 reports target reached → `StreamPlayer__Stop`) and calls `Next_Song(-1)` → first allowed index (BrainFreeze, idx 5).
- Rust `transitions.rs:420` → `request_scenario_theme(None)` → `ScenarioThemeRequest::Auto` → `queue_request(ThemeRequest::Auto)` (`theme.rs:258-318`): `avoids_fade = true`, so **no fade and no stop**; INTRO keeps playing through loading and into gameplay until its natural end, then 2.2 re-selects Drok. No LOADING theme is ever played (no `"LOADING"` string outside `theme.rs` tests).
- Trigger: every skirmish start. Effect: wrong music under the loading screen, wrong first gameplay track, no fade at gameplay start.
- Note: the Rust unit test `queue_sentinels_and_same_retained_track_preserve_native_fade_rules` (`theme.rs:772`) encodes `Queue_Song(-2)` semantics, which is right for `Queue_Song` but is not what `Start_Scenario` calls for index -1.

### 2.4 `Next_Song` selection — **MISSING (shuffle, Is_Allowed) / MATCH (bounded, cyclic order)**
- Native 0x00720A80 (disassembly verified):
  - Skip selection and return `prev` when `prev >= 0 && !entry[prev].Repeat && !global_repeat` is false (i.e. repeat) — same as Rust.
  - `+0x12 == 1` (shuffle): up to 1000 draws of `Random__RandomRanged(0, count-1)` on **`g_MainRng @ 0x00886B88`** (`MOV ECX,0x886B88` at 0x00720AB5), rejecting `== prev` or `!Is_Allowed`; after 1000 failures index 0.
  - else cyclic: `idx = prev+1` wrapping at `count`, up to `count+1` probes, first `Is_Allowed`; none → 0.
- `Is_Allowed` 0x00721140: sentinels -2/-3 → allowed; `idx >= count` → no; `+0x28A == 0` (file missing) → no; `Normal == 0` → no; `Side != -1 && Player->HouseType(+0x34)->+0xBC != Side` → no; `g_GameMode == 0 && Scen+0x1254 < Scenario` → no (campaign only).
- Rust `theme.rs:437-450`: cyclic over the pre-filtered `playlist` (Normal=no removed at build time, `theme.rs:684-704`), no shuffle, no Side/Scenario/availability test, no same-as-previous rejection. `IsScoreShuffle` is parsed (`options_profile.rs:272`) and never consumed; `global_repeat` (`theme.rs:132`) is never set from `is_score_repeat`.
- Disposition: cyclic order with the stock catalog is a bounded MATCH (given 2.1 fixed); shuffle and the repeat option are MISSING. Side filter EXCLUDED in stock data (no `Side=`); Scenario filter EXCLUDED in skirmish (`g_GameMode != 0`); availability gate EXCLUDED with retail mixes (all 8 `.WAV` exist).
- **Determinism note (critical for the builder):** native draws the shuffle from `g_MainRng`, which `Init_Random_Number_System` 0x0052FC20 seeds from the match seed alongside `Scen->Random` (doc-derived: `RMG_RNG_SEED_MAPGENRNG_GHIDRA_REPORT.md` §5.2, `RMG_BRIDGE_DUAL_RNG_LIFECYCLE_REINVESTIGATION_GHIDRA_REPORT.md` §3.6), and the draw happens inside the audio pump at wall-clock track ends. Rust models that instance as `World::main_rng` (`src/sim/world/mod.rs:681`). Routing the shuffle through it would make sim RNG consumption depend on audio timing (replay/lockstep nondeterminism). The shuffle must use a presentation-side `SimRng` clone (same algorithm, seeded from the match seed at scenario start) and be labelled "VERA-internal, gamemd draw sequence deliberately not reproduced".

### 2.5 `Queue_Song` / `Play_Song` / `Stop` state rules — **MATCH (bounded)** with two residuals
- Checked against `theme.rs:281-352,452-538`: sentinel handling (-1/-2/-3), same-retained no-op while playing, fade on queue of a different index while playing, `Stop` clearing all three slots, repeat writing pending, AI restoring -2 after a play attempt. These match the bodies.
- Residual A: native `Play` and `AI` are gated on `+0xC > 0` (score volume). With `ScoreVolume=0`, `Play(idx)` stores `pending = idx` and plays nothing; `AI` returns early; raising the volume later (`Set_Volume` from the options dialog) resumes by playing `pending`. Rust has no volume gate (`music.rs` only scales gain), so the logical player keeps advancing silently and a later volume raise joins a track mid-way. Trigger: only with ScoreVolume 0 then raised — rare. **DRIFT (low)**.
- Residual B: the "CD check" branch in `Play` (`FUN_004790A0` → 0x004A80D0 returns constant 2) is dead — EXCLUDED; Rust's "acquisition failure retry" comment at `theme.rs:515` models a path native cannot take, harmless.

### 2.6 `AI` poll cadence / screens — **DRIFT (demonstrated)**
- Native: `AI` runs from `AudioSystem__Pump` on every `Network_ServiceLoop` pass (≥33 ms) on every screen, including loading, score dialog, exit cascade and an inactive window.
- Rust: `update_theme` is called only from `advance_in_game_runtime_mode` (`sim_tick.rs:1124`, gated by `frame.rs:135-142`: `InGame` && window active && no scenario_exit/outcome) and from `maintain_main_menu_theme` (`shell_main_menu.rs:866`, MainMenu only). `pump_audio_service` (`sim_tick.rs:737`) pumps SFX only. Consequences: (a) on the score screen `SCORE` (Repeat=yes) plays once and is not re-queued; (b) while the window is inactive or a scenario exit/outcome is pending, completions are not consumed; (c) the loading-screen fade of 2.3 (once fixed) would not advance until gameplay.
- Trigger: every score screen longer than ScoreX; every Alt-Tab across a track end. Effect: silence instead of repeat/next track.

### 2.7 Track-end detection and fade — **MATCH (bounded)**
- Native: completion callback 0x00402B80 clears StreamPlayer `flags & 1`; `AI` polls `IsPlaying`. Fade = `VolumeInterp` 1000 ms started by 0x004080C0; `AI`/`Queue`/`Play` call `StreamPlayer__Stop` when `+0x11` is set and 0x004080D0 says the target is reached.
- Rust: rodio `Player::empty()` → `MusicOutputState::Finished` (`music.rs:52-58`), `SCENARIO_THEME_FADE_MS = 1000` linear (`theme.rs:16,393-408`). Equivalent for the paths that exist; the fade is only ever started from `queue_request`, never from `Stop(fade=1)` (no `fade` parameter on `ThemeRuntime::stop`, `theme.rs:341`) — needed by 2.3, scenario exit already models its own master fade in `scenario_exit.rs:172-188`.

### 2.8 Main-menu INTRO, score theme, credits — **MATCH (bounded)** / not in skirmish scope
- INTRO play on shell entry and every return, loop via per-theme Repeat: Rust `maintain_main_menu_theme` — matches the prior report's chain (spot-checked at 0x0052D9B4/0x0052DCB6).
- SCORE on the score dialog: `scenario_exit.rs:226` + `in_game.rs:615` — MATCH except 2.6(a).
- `Show_Credits` → `Queue_Song(INTRO)` after credits: out of scope (main menu credits); not checked in Rust.

### 2.9 In-game Sound options dialog — **MISSING**
- Native (Options → Sound; code region 0x006B6500–0x006B69F8, not a Ghidra function; string table 0x006040B0 maps ids `STT:SoundOptListScore/CBoxShuffle/CBoxRepeat/ButtonPlay/ButtonStop/Slider*`): list of all entries (loop to `0xA83D34`, current = `+0x4` else `+0x8`); **Play** → `InGameMusic := 1`, if selection ≠ current → `Stop(1)` + `Queue_Song(sel)`; **Stop** → `Stop(1)` + `Queue_Song(-3)` + `InGameMusic := 0`; music slider → `Set_Volume`, zero → `Queue(cur)` + `Stop(0)`; Repeat/Shuffle checkboxes → setters 0x005FA470/0x005FA440 (write `+0x10`/`+0x12` and Options+0x44/+0x46; persisted by `OptionsClass__WriteToINI`).
- Rust: the Sound button is a KD-7 no-op (`input/in_game_options.rs:88-99`); no owner for list/Play/Stop/checkboxes.
- Trigger: whenever the player opens Options → Sound (common for volume). Effect: no in-game music control at all.

### 2.10 `InGameMusic` option and `Main_Tick` hold — **MISSING (low)**
- Native: `Main_Tick` head (§1.4) holds music with `Stop(1)`/`Queue(-3)` while `InGameMusic == 0`; default 1 (`SetDefaults` 0x005FA350 `+0x45 = 1`). Only the Sound dialog's Stop/Play buttons write it in stock UI.
- Rust: parsed (`options_profile.rs:66,273`) and unused. Trigger: only after 2.9 exists or via hand-edited `ra2md.ini`.

### 2.11 Savegame Theme::Load 0x00721040 — **UNRESOLVED**
- Native restores `+0x0/+0x4` from the save and replays the saved index (if `InGameMusic`). Rust save/load (`persistence/save_load_panel.rs`) has no theme field. Not in the skirmish-start path; noted for the save/load lane.

### 2.12 Trigger / team-mission music, WOL lobby, campaign map-select — **EXCLUDED (stock skirmish)**
- Trigger action `Queue(action+0x90)` @0x006DE8F0 and team-mission re-queue @0x006E9944 need map-authored triggers/scripts; the WOL lobby path (`g_GameMode == 4`) and `MapSelect` are not reachable offline.

---

## 3. Ranked implementation candidates

### Prerequisite A — index-keyed catalog from THEMEMD.INI only (size M)
Rebuild `ThemeRuntime` around a `Vec<ThemeEntry>` in `[Themes]` order with `key, sound, normal, repeat, side, scenario, available` and integer slots (`active/retained/pending: i32` with the -1/-2/-3 sentinels), dropping `theme.ini` loading, the stem `aliases`/`repeating_tracks` sets and `FALLBACK_TRACKS`. `From_Name` matches section keys only. Availability = `assets.get_ref(sound + ".wav")` at catalog build (Scan). Files: `src/audio/theme.rs` (most of 1-704 + tests), `src/app/audio_runtime.rs` (`play_theme(&str)` callers keep a name API), `src/app/match_runtime/scenario_exit.rs:125,226` (string themes still fine via From_Name). Fixes 2.1 and 2.2.

### P0 candidates (each depends on A)
1. **Start_Scenario hand-off + LOADING theme** (size S once A lands): `request_scenario_theme` → `Play(From_Name("LOADING"))` at loading start, then after load `theme == -1 ? stop(fade=true) : queue(idx)`; add a `fade` argument to `ThemeRuntime::stop` that starts the 1000 ms ramp and clears the slots (native `Stop(1)` while playing: fade flag + clear). `Main_Tick`'s `retained == -1 → Queue(-2)` rule goes into `update_with` (or a per-frame hook in `sim_tick.rs`). Files: `src/audio/theme.rs`, `src/app/loading/transitions.rs:405-425`, `src/app/loading/pump.rs` (loading start), `src/app/match_runtime/sim_tick.rs`. Fixes 2.3.
2. **AI poll on every screen** (size S): move `state.audio.update_theme(...)` out of `advance_in_game_runtime_mode` into `pump_audio_service` (`sim_tick.rs:737`, already called unconditionally at `frame.rs:125`), keeping the 33 ms gate. Verify the main-menu maintenance still takes the same-track no-op. Fixes 2.6.
3. **Next_Song parity** (size M): implement `is_allowed(idx)` (count, available, Normal, Side vs local player's side, campaign-only Scenario) and the two selection modes; wire `global_repeat` and a `shuffle` flag from `options_profile.is_score_repeat/is_score_shuffle` at startup (`initialize.rs:81-84` path) and from the launcher/in-game dialogs. Shuffle RNG: a presentation-side `SimRng` seeded from the match seed at `Start_Scenario`, never `World::main_rng` (2.4 note). Files: `src/audio/theme.rs`, `src/app/initialize.rs`, `src/app/persistence/options_profile.rs`.

### P1
4. **In-game Sound options dialog** (size L, UI): list/Play/Stop/Repeat/Shuffle/sliders per 2.9, writing `InGameMusic`, `IsScoreRepeat`, `IsScoreShuffle`, `ScoreVolume` back through `options_profile` and applying `Stop(1)+Queue(sel)`, `Stop(1)+Queue(-3)`. Files: `src/app/input/in_game_options.rs`, `src/app/frontend/skirmish_shell_render/in_game_options.rs`, `src/ui/pause_menu.rs`, `src/app/persistence/options/*`. Depends on 3.
5. **Score-volume gate and InGameMusic hold** (size S): `+0xC > 0` gate in `Play`/`AI`; `Main_Tick` hold rule when `in_game_music == false`. Files: `src/audio/theme.rs`, `src/app/match_runtime/sim_tick.rs`.

### P2
6. Theme index in savegames (2.11) — coordinate with the save/load lane.

---

## 4. Uninspected / unknown

- The in-game Sound dialog body (0x006B6500–0x006B69F8) was read only around the five Theme xrefs via assembly context; the exact control ids, list ordering (all entries vs `Is_Allowed`-filtered) and whether the Repeat/Shuffle checkboxes apply immediately or on Back are UNCHECKED (setters 0x005FA440/0x005FA470 have no analysed callers, so they are called from this unanalysed region).
- `StreamPlayer` buffer-drain timing (how many ms after the last sample `flags&1` clears) was not measured; the Rust `Player::empty()` edge may differ by a buffer length.
- Track durations (`+0x284`) are only consumed by the Sound dialog list (not verified) — not by selection.
- `Show_Credits` / `CREDITS` theme, WOL lobby music, `Theme::Load` were identified but not compared.
- Rust runtime behaviour was established by code reading only (no build/run in this scan); 2.2/2.3 should be confirmed with one release-build skirmish start (expected: Drok continues from the menu and never advances).
