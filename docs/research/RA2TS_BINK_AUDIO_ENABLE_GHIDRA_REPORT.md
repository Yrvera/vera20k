# RA2TS Bink Audio Enable — Ghidra Research Report

**Date:** 2026-05-19
**Primary addresses:** `0x004326C0` (Bink constructor), `0x00432750` (Bink open/init)
**Confidence:** HIGH for all major findings (verified from binary + asset header in this session)
**Active in YR:** Yes (main menu shell state `0x12`)

Parent report: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`

---

## 1. Overview

This report answers three questions:

1. Does `gamemd.exe` call any audio-enable API (`BinkSetSoundOn`, `BINKSND` flag,
   `BinkOpenWithOptions`, or similar) when opening the main-menu Bink movie?
2. Does retail `ra2ts_l.bik` / `ra2ts_s.bik` actually carry an audio track?
3. Is the Rust port's current video-only Bink playback a parity gap or faithful?

**Bottom line:** `ra2ts_l.bik` and `ra2ts_s.bik` have **zero audio tracks** (`num_audio_tracks = 0` in
the BIK container header). Even though `gamemd.exe` sets up DirectSound via
`_BinkSetSoundSystem@8` before calling `_BinkOpen@8`, there is no audio to decode from
these assets. The Rust port's current video-only Bink implementation is **already faithful**
to retail YR behavior for the main-menu shell movie. No parity gap exists here.

---

## 2. Bink Import Function Table

All Bink imports resolved from `binkw32.dll`. Import name strings verified at `0x00810bXX`:

| String address | Import name | IAT call site(s) in FUN_00432750 |
|---|---|---|
| `0x00810bae` | `_BinkOpen@8` | `[0x007e159c]` → called at `0x00432849` |
| `0x00810bbc` | `_BinkSetSoundSystem@8` | `[0x007e1590]` → called at `0x004327b4` |
| `0x00810bd4` | `_BinkOpenDirectSound@4` | `[0x007e1594]` → passed as arg to above |
| `0x00810b88` | `_BinkSetVolume@8` | `[0x007e15a0]` → called at `0x00432897` and `0x00432E40` |
| `0x00810b60` | `_BinkClose@4` | `[0x007e15a4]` → called at `0x00432783` |
| `0x00810bee` | `_BinkGoto@12` | vtable `+0x1C` thunk `0x005C05D0` |
| `0x00810bfe` | `_BinkPause@8` | vtable `+0x0C` thunk `0x005C0540` |
| `0x00810c0e` | `_BinkNextFrame@4` | `0x00432E40` update loop |
| `0x00810c22` | `_BinkCopyToBuffer@28` | `0x00432E40` update loop |
| `0x00810c3a` | `_BinkDoFrame@4` | `0x00432E40` update loop |
| `0x00810c4c` | `_BinkWait@4` | `0x00432E40` update loop |
| `0x00810b70` | `_BinkDDSurfaceType@4` | `0x00432A3E` surface-type setup |
| `0x00810b9c` | `_BinkGetError@0` | `0x00432857` error path |

**No `BinkSetSoundOn`, no `BinkOpenWithOptions`, no `BINKSND` string, no track-index
flag exists anywhere in the binary.** Confirmed via `search_strings("BINKSND")` and
`search_strings("BinkOpen")` — zero matches beyond the import name strings listed above.

---

## 3. Open-Time Audio Setup in FUN_00432750

The open/init function `FUN_00432750` (called by the Bink constructor `FUN_004326C0`)
executes this sequence:

```text
1. Clean up any prior Bink state (close existing handle, close Win32 HANDLE if open)

2. CHECK AUDIO: FUN_00407000()
   → returns (DAT_0087e728 != 0)
   → DAT_0087e728 = DirectSound device pointer, set by AudioSystem__Init @ 0x00406C00
   → non-null means audio subsystem initialized

3. IF audio available (FUN_00407000 != 0):
   → FUN_0040a7a0()  — returns DAT_0087e89c (IDirectSound* object)
   → _BinkSetSoundSystem@8(BinkOpenDirectSound, IDirectSound*)
   → registers DirectSound as Bink's audio backend

4. Resolve file via RawFileClass or MIX archive (FUN_005B4430)
   → if found in MIX archive: open Win32 HANDLE via CreateFileA, store at object+0x28
     → flags for _BinkOpen@8 = 0x800000 (= BINKOPENFILEHANDLE in Bink SDK)
   → if found as raw file: flags = 0

5. _BinkOpen@8(file_path_or_handle, flags)
   → returns Bink* handle, stored at object+0x4
   → No BINKSND or audio-enable flag is passed — Bink automatically enables audio
     decode for whatever tracks exist in the file when a sound system is registered

6. IF open succeeded:
   → copy DAT_00a8eb9c → DAT_0089c490 (cache current audio volume)
   → _BinkSetVolume@8(handle, ftol(DAT_00a8eb9c * scale))
   → compute ticks_per_frame = int(60 / fps) → object+0x24
   → set up BSurface and clip rect → object+0x10..+0x1C
   → _BinkDDSurfaceType@4 → object+0x8

7. IF open failed:
   → _BinkGetError@0 → log "Bink Error: %s\n" (string at 0x00818b2c)
   → return 0
```

### Key finding — `_BinkSetSoundSystem@8` is called before `_BinkOpen@8`

This is the correct Bink SDK call order. `BinkSetSoundSystem` installs the audio backend
globally. When `BinkOpen` is subsequently called with no special flags, Bink will
automatically decode any audio tracks present in the file. **There is no separate
"enable audio" flag needed.** Audio is on by default if a sound system is registered and
the file has tracks.

### Key finding — `0x800000` is NOT a sound flag

When the BIK file is sourced from a MIX archive, `CreateFileA` opens a physical file,
stores the handle at `object+0x28`, and passes `0x800000` to `_BinkOpen@8`. In the Bink
SDK, `0x800000` = `BINKOPENFILEHANDLE` — it tells Bink that the first argument is a Win32
HANDLE rather than a filename string. This has no relationship to audio.

### Key finding — audio gate is `DAT_0087e728`

`FUN_00407000` (tested at `0x004327A4`) reads `DAT_0087e728`. This global is written
by `AudioSystem__Init @ 0x00406C00` to the result of `FUN_00402C70()` (DirectSound
channel allocator). If the audio subsystem failed to initialize (no DirectSound device,
`-nosound` cmdline, etc.), `DAT_0087e728 == 0` and `_BinkSetSoundSystem@8` is never
called. In that case Bink opens video-only regardless of what audio tracks the file
contains.

---

## 4. Asset-Level Verification — ra2ts_l.bik / ra2ts_s.bik Have Zero Audio Tracks

Verified using `cargo run --bin bik-survey -- ra2ts_l --avsync` and `-- ra2ts_s --avsync`:

```
[AV] ra2ts_l.bik  no audio track
[AV] ra2ts_s.bik  no audio track
```

This reads `BinkHeader.num_audio_tracks` directly from the BIK container header
(`src/assets/bink_file.rs`, field at header offset `0x28`). Both files report `0`.

Consequence: even with `_BinkSetSoundSystem@8` called successfully and DirectSound
initialized, `_BinkOpen@8` opens a file that contains no audio data. Bink will output
video frames only. There is no audio to mix with the `[INTRO]` theme music.

---

## 5. Per-Frame Audio Handling in FUN_00432E40

The per-frame update (`FUN_00432E40`, called from the WM_TIMER path) contains:

```text
if (_DAT_0089c490 != DAT_00a8eb9c):     // volume changed since last frame
    _DAT_0089c490 = DAT_00a8eb9c
    _BinkSetVolume@8(handle, ftol(volume))  // update Bink's audio volume

if g_GameRunning == 0:
    if playing_flag == 1:                // game paused → pause Bink (including audio)
        BinkPause(handle, 1)
else:
    if playing_flag == 0:                // game resumed → resume Bink
        BinkPause(handle, 0)

... BinkDoFrame / BinkCopyToBuffer / BinkNextFrame loop ...
```

`_BinkPause@8` pauses/resumes both video and audio in Bink. There is no separate audio
pause call — Bink handles it internally. For `ra2ts_*.bik` (no audio), `BinkPause` and
`BinkSetVolume` are no-ops on the audio side but are still called unconditionally.

**Volume shadow copy pattern:** `DAT_0089c490` caches the last-sent volume. It is set
at open time (in `FUN_00432750`) and then checked each frame (in `FUN_00432E40`). The
comparison is floating-point equality: `_DAT_0089c490 != DAT_00a8eb9c`. `BinkSetVolume`
is only issued when the value changes — not every frame. `DAT_00a8eb9c` is the
authoritative global audio volume for Bink (set elsewhere in the audio subsystem).

---

## 6. MSBinkAnim — Separate Bink Usage (Out of Scope)

`MSAnim__Constructor @ 0x005cc760` also calls `FUN_004326C0` (same Bink constructor)
for in-game animated sprites. It uses the same open-time audio setup path. Most
in-game Bink animations (cutscene side-panel portraits, briefing movies) DO carry
audio tracks. The same `_BinkSetSoundSystem@8` call enables audio for those too.
This is out of scope for the main-menu shell investigation.

---

## 7. Current Rust Implementation Status

`src/render/bink_movie.rs` — video-only `BinkMovieSurface`.
- `BinkFile::parse` reads `num_audio_tracks` correctly (field at header offset `0x28`).
- `BinkAudioDecoder` in `src/assets/bink_audio.rs` exists and is complete.
- `bink_movie.rs::step()` calls only `file.video_packet()` and `decoder.decode_frame()`.
  It does NOT call `file.audio_packets()` or `BinkAudioDecoder::decode_packet()`.

For the **main-menu ra2ts playback specifically**, the current video-only Rust
implementation is **parity-correct**: the source assets carry no audio, so no audio
output is expected or produced.

For **other Bink files** (cutscene movies, in-game side panels) that DO carry audio
tracks, the Rust port has a parity gap — but that is a separate system from the
main-menu shell and is out of scope here.

---

## 8. Open Questions — Final State

- `[RESOLVED] OQ1 — Does _BinkOpen@8 use a BINKSND or audio-enable flag at open time?`
  → No. Flags are `0` (filename) or `0x800000` (WINFILEHANDLE). No sound flag is
  passed. Audio enable is implicit via prior `_BinkSetSoundSystem@8` call.
  (evidence: `FUN_00432750` disassembly at `0x00432849`, search_strings BINKSND → 0 hits)

- `[RESOLVED] OQ2 — Do ra2ts_l.bik / ra2ts_s.bik carry audio tracks?`
  → No. Both have `num_audio_tracks = 0` in the BIK header.
  (evidence: `bik-survey --avsync` output: `"no audio track"` for both files)

- `[RESOLVED] OQ3 — Is BinkSetSoundSystem called before or after BinkOpen?`
  → Before. Confirmed from disassembly: `_BinkSetSoundSystem@8` at `0x004327b4`,
  `_BinkOpen@8` at `0x00432849`, unconditional ordering.
  (evidence: `FUN_00432750` disassembly)

- `[RESOLVED] OQ4 — What is the audio-system availability gate?`
  → `DAT_0087e728 != 0`, read by `FUN_00407000`. Written by `AudioSystem__Init`
  to the result of `FUN_00402C70()` (DirectSound device allocator).
  (evidence: `FUN_00407000` decompile at `0x00407000`; xrefs to `0x0087e728`
  include `AudioSystem__Init` WRITE at `0x00406c1a`)

- `[RESOLVED] OQ5 — Is BinkSetVolume called at open time, at frame update, or both?`
  → Both. At open time: `0x00432877`–`0x00432897`. At frame update: `FUN_00432E40`
  delta-check `_DAT_0089c490 != DAT_00a8eb9c`.
  (evidence: `FUN_00432750` decompile; `FUN_00432E40` decompile)

- `[RESOLVED] OQ6 — Is there a BinkSetSoundOn or BinkOpenWithOptions API imported?`
  → No. Full Bink import list confirmed: 13 functions, none are `BinkSetSoundOn` or
  `BinkOpenWithOptions`. (evidence: `search_strings("BINK")` — 20 matches, all 13
  import names verified, no sound-on/options variants present)

- `[RESOLVED] OQ7 — Does the WM_TIMER Bink pause/resume path affect audio?`
  → Yes. `BinkPause(handle, 1/0)` pauses/resumes both video and audio in Bink.
  For ra2ts files (no audio), the call is a no-op on audio. For audio-bearing files,
  it properly gates audio output during game pause.
  (evidence: `FUN_00432E40` decompile; `g_GameRunning` branch at offset `+0x2c`)

- `[DEFERRED] OQ8 — Do the in-game MSBinkAnim Bink files (cutscene portraits) mix
  audio with the shell theme music?`
  (category: out-of-scope; reason: MSBinkAnim is in-game context, not main-menu shell;
  next-step-if-pursued: decompile `MSAnim__Constructor` dispatch chain and trace
  `_BinkPause` call sites in the in-game tick)

- `[DEFERRED] OQ9 — What is the exact value of DAT_00a8eb9c at the time the main-menu
  Bink opens (full-volume or attenuated)?`
  (category: needs-runtime-debugger; reason: global is written by audio volume sliders
  and options screen; static analysis cannot determine runtime value without tracing
  all writers; next-step-if-pursued: breakpoint at `0x00432881` during shell init)

---

## Sources

- Ghidra functions decompiled this session:
  - `FUN_004326C0` @ `0x004326C0` — Bink constructor (vtable dispatch target)
  - `FUN_00432750` @ `0x00432750` — Bink open/init (full disassembly + decompile)
  - `FUN_00432E40` @ `0x00432E40` — per-frame Bink update (BinkSetVolume, BinkPause)
  - `FUN_00432C50` @ `0x00432C50` — end/wrap test (no audio branches)
  - `FUN_00407000` @ `0x00407000` — audio system available check
  - `FUN_00406C00` → `AudioSystem__Init` — writes `DAT_0087e728`
  - `FUN_0040A7A0` @ `0x0040A7A0` — returns IDirectSound* from `DAT_0087e89c`
  - `FUN_00432690` @ `0x00432690` — alternate Bink constructor (no explicit clip rect)
  - `MSAnim__Constructor` @ `0x005CC760` — in-game Bink anim constructor
  - `FUN_00402C70` @ `0x00402C70` — DirectSound device allocator
- Ghidra string searches:
  - `search_strings("BINK")` — 20 matches, all listed in Section 2
  - `search_strings("BINKSND")` — 0 matches
  - `search_strings("BINKNOSOUND")` — 0 matches
- Asset verification:
  - `cargo run --bin bik-survey -- ra2ts_l --avsync` → `"no audio track"`
  - `cargo run --bin bik-survey -- ra2ts_s --avsync` → `"no audio track"`
- Rust source inspected (read-only):
  - `src/assets/bink_file.rs` — `BinkHeader.num_audio_tracks` parsed at offset `0x28`
  - `src/assets/bink_audio.rs` — `BinkAudioDecoder` (complete implementation)
  - `src/render/bink_movie.rs` — video-only `BinkMovieSurface::step()`
  - `src/bin/bik-survey.rs` — `avsync_report()` function used for asset verification
- Parent reports:
  - `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
  - `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md`
