# Audio-Bearing BIK Path And Volume - Ghidra Research Report

**Address(es):** `0x00432750`, `0x00432E40`, `0x005BED40`, `0x00432C70`, `0x00657950`, `0x00657A20`, `0x006691E0`  
**Investigation Mode:** coverage-map with exhaustive wrapper-side audio slice  
**Claimed Scope:** Bink wrapper audio enablement, volume update, pause/update ordering, active standard YR paths that can play audio-bearing BIKs, and `MovieOn`/`MovieOff` relationship to Bink audio.  
**Non-Scope:** Bink SDK internals, PCM byte parity, all campaign win/loss launchers, full radar movie visual composition.  
**Confidence:** High for wrapper call order and rule-key reads; medium-high for active path liveness.  
**Active in YR:** Yes, conditional on the movie file carrying audio and DirectSound being initialized.

## 0. Working Notes Required Before Investigation

- **Target question:** How does standard YR enable and control audio for audio-bearing BIK playback beyond silent RA2TS, and what must Rust preserve?
- **Non-goals:** Do not investigate Bink codec internals, VQA audio, full campaign result flow, or pixel/video frame-loop details beyond ordering against audio calls.
- **Evidence needed to mark COMPLETE:** live Ghidra decompile plus assembly for open/update/playback functions; caller/path proof for standard YR movie playback; INI/default proof for `MovieOn`/`MovieOff`; Rust surface scan with concrete test handoff.
- **Stop conditions:** Stop after wrapper-side Bink audio and the direct radar/movie sound distinction are proven; record Bink SDK/runtime-only PCM and global-volume writer details as uncertainty.

## 1. Overview

Standard YR does support audio-bearing BIK playback. The direct BIK path registers Bink's DirectSound backend before `_BinkOpen`, sends Bink volume as `ftol(DAT_00A8EB9C * 32768.0)`, and pauses/resumes Bink through `_BinkPause`. Retail RA2TS menu BIKs are the special silent case; the wider movie corpus is not silent.

`MovieOn` and `MovieOff` are separate rules-driven VOC sound events used by the radar/sidebar movie queue. They are not Bink-open flags and are not the embedded audio track in a `.bik`.

## 2. Key Offsets And Globals

| Offset / global | Meaning | Active in YR | Evidence |
|---:|---|---|---|
| Bink object `+0x04` | Bink SDK handle passed to `_BinkSetVolume`, `_BinkPause`, `_BinkWait`, `_BinkDoFrame`, `_BinkNextFrame`, `_BinkClose`. | Yes | `0x00432750`, `0x00432E40`, `0x00432700`. |
| Bink object `+0x2C` | Local playing/unpaused flag; constructor sets `1`; update toggles it around `_BinkPause`. | Yes | `0x00432690`, `0x004326C0`, `0x00432E40`; assembly `0x00432EA4`, `0x00432ECA`. |
| Bink object `+0x2D` | Force-frame flag; skips initial `_BinkWait` when nonzero, then clears after `_BinkDoFrame`. | Yes | `0x00432F36..0x00432F6F`. |
| Bink object `+0x30` | Last Bink frame marker used by loop/end logic. | Yes | `0x00432E40`, `0x00432C70`. |
| `DAT_0087E728` | Audio-system availability gate. | Conditional | `0x00407000`, `AudioSystem__Init 0x00406C00`. |
| `DAT_0087E89C` | IDirectSound pointer passed to `_BinkSetSoundSystem`. | Conditional | `0x0040A7A0`, `0x004327A8..0x004327B4`. |
| `DAT_00A8EB9C` | Current Bink volume float. | Yes when audio enabled | `0x00432750`, `0x00432E40`. |
| `DAT_0089C490` | Cached last-sent Bink volume float. | Yes | `0x00432E40..0x00432E7A`. |
| RulesClass `+0x6F8` / `param_1[0x1BE]` | `MovieOn` VOC index. | Yes for radar movie queue | `0x006691E0`; `0x0065795B`, `0x00657BCD`. |
| RulesClass `+0x6FC` / `param_1[0x1BF]` | `MovieOff` VOC index. | Yes for radar movie queue | `0x006691E0`; `0x00657C50`. |

## 3. Core Logic

### Bink Sound Backend Registration

Active in YR: Yes, conditional on initialized sound.

`FUN_00432750` calls `FUN_00407000()`. If `DAT_0087E728 != 0`, it calls `FUN_0040A7A0()` and then `_BinkSetSoundSystem(BinkOpenDirectSound, DAT_0087E89C)` before `_BinkOpen`.

Evidence: decompile `0x00432750`; assembly `0x0043279F..0x004327B4`; string search for Bink imports. The import set contains no `BinkSetSoundOn`, no `BinkOpenWithOptions`, no `BINKSND`, and no no-sound variant string.

### `_BinkOpen` Flags

Active in YR: Yes.

| Source mode | `_BinkOpen` first arg | `_BinkOpen` flags | Evidence |
|---|---|---:|---|
| direct filename/raw file | filename pointer | `0` | `0x004327DE..0x004327E4` pushes `0` then filename. |
| archive-backed Win32 handle | object `+0x28` | `0x800000` | `0x00432840..0x00432849` pushes `0x800000` and handle. |

`0x800000` is a file-handle/source flag, not audio. Embedded audio is enabled by the earlier sound-system registration and by the BIK file actually carrying a track.

### Volume Formula And Ordering

Active in YR: Yes.

Open-time success path:

```text
DAT_0089C490 = DAT_00A8EB9C
_BinkSetVolume(handle, ftol(DAT_0089C490 * 32768.0))
```

Per-update path:

```text
if DAT_0089C490 != DAT_00A8EB9C:
    DAT_0089C490 = DAT_00A8EB9C
    _BinkSetVolume(handle, ftol(DAT_0089C490 * 32768.0))

apply _BinkPause transitions
then initial _BinkWait gate
then decode/copy/next-frame loop
```

Evidence: assembly `0x00432877..0x00432897` and `0x00432E40..0x00432E7A` copy the float, `FMUL` by `0x007E3A70` (`32768.0f`), call `0x007C5F00` (`ftol`), then call IAT `[0x007E15A0]` (`_BinkSetVolume@8`). In the update loop, this occurs before reading `DAT_00A8ED80` at `0x00432E80` and before `_BinkWait` at `0x00432F41`.

Tiny details:

- The volume equality test is x87 `FCOMP` plus status-word check, not an epsilon compare.
- Volume update can occur on a poll that returns no new frame.
- Scale is exactly `32768.0f`; a `1.0` float maps to integer `32768`.

### Pause / Mute / Full-Screen Movie Audio Gating

Active in YR: Yes.

`FUN_00432E40` calls `_BinkPause(handle, 1)` when global run state `DAT_00A8ED80` is zero and object `+0x2C` is one, setting `+0x2C = 0` before the call. On resume, it sets `+0x2C = 1`, calls `_BinkPause(handle, 0)`, then runs the surface transition helper before wait/decode.

Evidence: decompile `0x00432E40`; assembly pause call `0x00432EA4..0x00432EAB`; resume call `0x00432ECA..0x00432ED1`.

`FUN_005BED40` wraps full-screen direct movie playback by pausing/fading other audio once per playback session, then restoring it after playback. The BIK branch opens via `0x00432690`, sets the `DAT_00ABF35C` guard, calls `VoxClass__PauseEVA`, `0x00408200`, `0x00408270`, two `VolumeInterp__SetTarget` calls, then `0x00406EA0`, `0x0040A7C0(2)`, and `0x00432C70`. After playback it calls `0x0040A850`, `0x00406EC0`, restores audio/EVA, and clears `DAT_00ABF35C`.

Evidence: decompile `0x005BED40`; assembly around `0x005BEE00..0x005BEE58`.

## 4. Active Standard YR Paths To Audio-Bearing BIK

| Path | Bink entry | Audio-bearing proof | Active in YR |
|---|---|---|---|
| Movies & Credits -> Sneak Preview / Movies list | `FUN_005BED40` -> `.bik` branch -> `0x00432690` -> `0x00432C70` | Local survey found 96 audio-bearing BIKs and only `ra2ts_l/s.bik` silent. | Yes, reachable from main menu; list source is `[Movies]` in `artmd.ini`. |
| Radar/sidebar movie queue | `RadarClass__PlayRadarMovie 0x00657950`, `RadarClass__PerFrameMovieUpdate 0x00657A20`, virtual movie handle update/end slots | Plays `MovieOn`/`MovieOff` VOC loop sounds around queue state; individual BIKs may also carry embedded audio. | Conditional on scenario/briefing/radar movie queue entries. |
| `MSBinkAnim` | `MSAnim__Constructor 0x005CC760` -> `0x004326C0` | Shares the same Bink object open path. | Conditional; per-asset liveness out of scope. |
| Main-menu RA2TS background | `0x005C07D0` Bink branch -> `0x004326C0` | `ra2ts_l.bik` and `ra2ts_s.bik` have no audio track. | Yes, but silent. |

Retail asset survey:

```text
cargo run --quiet --bin bik-survey -- --avsync
total=98 audio=96 silent=2
silent: ra2ts_l.bik, ra2ts_s.bik
```

## 5. INI Keys

| Key / section | Default in YR | Effect | Active in YR | Evidence |
|---|---|---|---|---|
| `rulesmd.ini [AudioVisual] MovieOn=MovieOn` | `MovieOn` | Resolves to RulesClass `+0x6F8` / `param_1[0x1BE]`. | Yes | INI; `0x006691E0`; `0x0066AB58..0x0066AB8E`. |
| `rulesmd.ini [AudioVisual] MovieOff=MovieOff` | `MovieOff` | Resolves to RulesClass `+0x6FC` / `param_1[0x1BF]`. | Yes | INI; `0x006691E0`; `0x0066AB9A..0x0066ABC7`. |
| `soundmd.ini [MovieOn]` | `Sounds=umovlo...`, `Control=random loop all attack decay`, `Volume=45` | Defines the loopable VOC event played around radar movie activity. | Yes | INI; `VocClass__PlayAtPos 0x00750920`; `SoundEvent__SetLoopHandle 0x004060F0`. |
| `soundmd.ini [MovieOff]` | Same sound list and volume as `MovieOn` | Defines the off/done VOC event. | Yes | INI; radar update reads `+0x6FC` at `0x00657C50`. |
| `artmd.ini [Movies]` | 59 YR movie names | Movie picker entries that can enter `FUN_005BED40`. | Yes | Prior dialog report; INI checked. |
| `battlemd.ini FinalMovie=` | Empty in checked stock YR entries | Campaign result movie key. | Conditional | INI grep; result-flow liveness deferred. |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Delta |
|---|---|---|
| `src/assets/bink_file.rs` | Parses BIK header, audio track descriptors, `audio_packets`, and strips audio packets from `video_packet`. | Header/audio packet support exists. |
| `src/assets/bink_audio.rs` | Contains a Bink audio decoder and reset support. | PCM parity against `binkw32.dll` is not proven here. |
| `src/render/bink_movie.rs` | Main-menu movie surface is video-only and elapsed-time stepped. | Correct for silent RA2TS only; missing audio-bearing playback side effects. |
| `src/bin/bik-player.rs` / `src/bin/bik_player_audio.rs` | Tool-level BIK playback with rodio sink, pause/resume, and `0.0..1.0` volume. | Useful scaffold, but native game path needs `DAT_00A8EB9C * 32768.0` semantics and Bink wait/update ordering. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Bink sound-system registration | verified | `0x0043279F..0x004327B4`; `0x00407000`; `0x0040A7A0` | none |
| `_BinkOpen` flags | verified | `0x004327DE..0x00432849` | SDK handling external |
| Bink open-time volume | verified | `0x00432877..0x00432897` | global volume writer set |
| Bink update volume/pause/wait order | verified | `0x00432E40..0x00432F55` | none for wrapper order |
| Direct `Play_Movie()` BIK path | verified | `0x005BED40`; `0x00432C70` | campaign win/loss callers not exhausted |
| Retail audio-bearing BIK presence | verified | `bik-survey --avsync`: 98 total, 96 audio, 2 silent | PCM parity runtime tests |
| `MovieOn`/`MovieOff` rule reads and radar use | verified | `0x006691E0`; `0x0065795B`, `0x00657BCD`, `0x00657C50` | full radar visual/movie queue setup |
| MSBinkAnim audio liveness | touched-not-exhausted | `0x005CC760` | per-asset callers |
| Bink SDK PCM/track mixing internals | deferred | external `binkw32.dll` | runtime or DLL-level investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-AUD-001 - Is there a separate Bink sound-enable flag? -> No; wrapper calls _BinkSetSoundSystem before _BinkOpen and passes only 0 or 0x800000 to _BinkOpen.` (evidence: `0x0043279F..0x00432849`; string search)
- `[RESOLVED] OQ-AUD-002 - What is the exact wrapper volume formula? -> _BinkSetVolume(handle, ftol(DAT_00A8EB9C * 32768.0)).` (evidence: `0x00432877..0x00432897`, `0x00432E40..0x00432E7A`)
- `[RESOLVED] OQ-AUD-003 - Does volume update happen before or after pause/wait? -> Before pause/resume and before the initial _BinkWait gate.` (evidence: `0x00432E40..0x00432F41`)
- `[RESOLVED] OQ-AUD-004 - Are RA2TS menu BIKs representative? -> No; asset survey finds only ra2ts_l/s silent, while 96 other resolvable BIKs are audio-bearing.` (evidence: `bik-survey --avsync`)
- `[RESOLVED] OQ-AUD-005 - Are MovieOn/MovieOff embedded BIK audio? -> No; they are RulesClass VOC indices played through VocClass/SoundEvent on the radar movie queue.` (evidence: `0x006691E0`, `0x0065795B`, `0x00657C50`, `0x00750920`)
- `[RESOLVED] OQ-AUD-006 - Does full-screen BIK playback also pause/fade other audio? -> Yes; FUN_005BED40 pauses/restores other audio systems around playback using DAT_00ABF35C as a guard.` (evidence: `0x005BED40`)
- `[DEFERRED] OQ-AUD-007 - Which code writes every possible DAT_00A8EB9C value?` (category: bounded-cost-too-high; reason: global volume writer set belongs to broader options/audio mixer system; next-step-if-pursued: dedicated global-volume xref trace)
- `[DEFERRED] OQ-AUD-008 - Is Rust BinkAudioDecoder PCM byte-identical to binkw32.dll?` (category: needs-runtime-debugger; reason: gamemd delegates decoding to external DLL; next-step-if-pursued: capture Bink SDK output for stock audio-bearing BIK frames and compare)
- `[DEFERRED] OQ-AUD-009 - Which standard YR scenarios instantiate MSBinkAnim with audio-bearing files?` (category: requires-different-system-context; reason: constructor liveness was proved but per-asset callers were outside this slot; next-step-if-pursued: MSBinkAnim caller/asset trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Audio-bearing BIK playback registers DirectSound before open and uses Bink audio automatically when the file has tracks. | `0x0043279F..0x004327B4`; `bik-survey --avsync` 96 audio-bearing BIKs. | Main-menu Rust is video-only; asset/audio decoder exists but is not integrated into game playback. | `src/assets/bink_file.rs`, `src/assets/bink_audio.rs`, future movie playback surface beyond `src/render/bink_movie.rs`. | Treat embedded BIK audio as part of Bink playback for non-RA2TS movies when sound is enabled. | Proposed test `audio_bearing_bik_registers_audio_track_when_sound_enabled`: load `a00_f00e.bik`, assert audio track metadata exists and playback pipeline creates an audio stream. | Do not use RA2TS silence to justify global video-only BIK support. |
| Wrapper volume is `ftol(DAT_00A8EB9C * 32768.0)` at open and on changed-volume polls before pause/wait. | `0x00432877..0x00432897`; `0x00432E40..0x00432E7A`. | `bik-player` uses rodio `0.0..1.0`; main renderer has no Bink audio volume path. | Future Bink audio player / `src/bin/bik_player_audio.rs` as reference only. | Convert engine movie volume through native scale and apply it before any wait/decode no-op decision. | Proposed test `bink_audio_volume_update_precedes_wait_noop`: changed volume plus not-ready wait updates sink volume without advancing frame. | Do not tie volume updates to successful frame upload or elapsed-frame advancement. |
| `MovieOn`/`MovieOff` are separate VOC loop sounds on radar movie queue, not Bink embedded audio or Bink open flags. | Rules read `0x006691E0`; use sites `0x0065795B`, `0x00657BCD`, `0x00657C50`; `VocClass__PlayAtPos 0x00750920`. | Rust has BIK parser/audio, but no verified radar movie VOC loop model. | Future radar/sidebar movie system and rules AudioVisual fields. | Keep radar queue activation/deactivation sounds separate from BIK track decoding. | Proposed test `radar_movie_plays_movieon_voc_independent_of_bik_audio_track`: starting a radar movie queues `MovieOn` even if the movie handle has its own audio track. | Do not map `MovieOn`/`MovieOff` to embedded audio mute/unmute or BinkSetVolume. |

## 10. Negative Facts / Do Not Do

- Do not implement `_BinkOpen` flag `0x800000` as audio. It is used only on the Win32 handle path.
- Do not treat `MovieOn` and `MovieOff` as Bink sound-enable toggles. They are VOC indices read from rules and played through `VocClass__PlayAtPos`.
- Do not apply Bink volume only when a frame uploads. Native updates volume before pause/resume and before `_BinkWait`.
- Do not preserve video-only playback for all BIKs on the basis of RA2TS.
- Do not collapse full-screen movie audio pausing/fading into Bink's own pause.

## 11. Remaining Uncertainty

- Exact `DAT_00A8EB9C` writer provenance and option-slider mapping are not exhausted.
- Exact PCM equivalence between Rust `BinkAudioDecoder` and `binkw32.dll` is runtime/DLL-level evidence.
- Full campaign result movie launcher coverage is not complete.
- MSBinkAnim per-asset liveness in standard YR is conditional and not drained here.

## 12. Stale Docs / Follow-up Docs

- Replace any broad wording equivalent to "Rust video-only BIK playback is parity-correct" with: "Video-only playback is parity-correct for `ra2ts_l.bik` and `ra2ts_s.bik` because those assets have zero audio tracks; it is not correct for the broader YR BIK corpus, where most resolvable BIKs carry audio."
- Replace any wording that implies `MovieOn`/`MovieOff` are Bink audio toggles with: "`MovieOn`/`MovieOff` are RulesClass VOC event indices used by the radar/sidebar movie queue; embedded BIK audio is controlled through Bink's DirectSound backend, `_BinkSetVolume`, and `_BinkPause`."

## Sources

- Live Ghidra MCP decompile/assembly: `0x00432750`, `0x00432E40`, `0x00407000`, `0x0040A7A0`, `0x00406C00`, `0x00432690`, `0x004326C0`, `0x00432700`, `0x00432C70`, `0x005BED40`, `0x005CC760`, `0x00657950`, `0x00657A20`, `0x006691E0`, `0x004060F0`, `0x00750920`.
- Prior docs: `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`, `BINK_UPDATE_LOOP_0X00432E40_FRESH_MCP_AUDIT_GHIDRA_REPORT.md`, `FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`, `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md`, `GLOBAL_SOUNDS_GHIDRA_REPORT.md`.
- INI files: `ini/rulesmd.ini`, `ini/soundmd.ini`, `ini/artmd.ini`, `ini/battlemd.ini`.
- Rust files scanned read-only: `src/assets/bink_file.rs`, `src/assets/bink_audio.rs`, `src/render/bink_movie.rs`, `src/bin/bik-player.rs`, `src/bin/bik_player_audio.rs`.
- Local command: `cargo run --quiet --bin bik-survey -- --avsync`.
