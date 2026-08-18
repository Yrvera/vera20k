# Main Menu RA2TS Movie Playback / Archive Priority - Ghidra Report

Date: 2026-05-17

Parent reports:

- `docs/research/MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`

Scope: targeted follow-up on two open questions from the visual-assets report:
the exact `Ra2ts_s/l` Bink playback/update path and the duplicate-asset
priority between `LANGMD.MIX` and `LANGUAGE.MIX`. This report does not
re-investigate the dialog `0xE2` button routing or shell owner-draw button art.

**Address(es):** `0x006153E0`, `0x005C0580`, `0x005C0570`, `0x005C05D0`,
`0x005C05F0`, `0x00433040`, `0x00432E40`, `0x00432C50`, `0x00432BD0`,
`0x00433060`, `0x005B3C20`, `0x005B4430`, `0x006BD7EF`, `0x006BD81F`
**Confidence:** High for playback/update behavior and `LANGUAGE.MIX` before
`LANGMD.MIX` duplicate priority. Medium for full global MIX priority beyond
these two archives because this pass focused only on the RA2TS movie assets.
**Active in YR:** Yes. This is the standard shell state `0x12` main menu path.

## 1. Overview

The main menu movie panel is a Win32 static control (`0x71A`) whose owner-draw
state stores a generic movie handle at `static_state+0x58` (`piVar11[0x16]` in
Ghidra's decompile). The panel is driven by a timer, explicit draw messages,
and Bink's own readiness checks.

The important implementation findings are:

- `WM_TIMER` id `0x65` calls the Bink update path, invalidates the window when
  a frame advanced, then checks whether the movie ended.
- On loop, the shell calls `BinkGoto(handle, 1, 1)` through the movie vtable and
  logs `"Looping movie"`.
- The Bink update path does not blindly advance one frame per Win32 timer. It
  calls `_BinkWait`, `_BinkDoFrame`, `_BinkCopyToBuffer`, `_BinkNextFrame`, and
  loops while Bink says more frames are ready.
- `LANGMD.MIX` is opened before `LANGUAGE.MIX`, but the MIX file list inserts
  newly opened archives at the search head. Therefore `LANGUAGE.MIX` wins for
  duplicate filenames such as `ra2ts_l.bik`.

## 2. Static Control Playback State

`OwnerDraw_Static_006153E0` stores the main menu movie handle in the owner-draw
record at `piVar11[0x16]` (`+0x58` bytes from the record start used by the
decompiler). It stores the loop flag in `piVar11[0x17]` (`+0x5C`).

Relevant messages:

| Message | Behavior | Evidence | Confidence |
|---:|---|---|---:|
| `0x4E3` | Stores `wParam` as the loop flag. Main menu passes `1`. | `OwnerDraw_Static_006153E0`; parent report `FUN_00531CC0`. | High |
| `0x4E4` | Destroys any old movie, kills timer `0x65`, constructs a new movie handle, resizes the static to movie width/height, then starts timer `0x65` at `0x22` ms. | `OwnerDraw_Static_006153E0`. | High |
| `0x4F0` | Calls movie vtable `+0x28`, the explicit copy/draw path. | `OwnerDraw_Static_006153E0` and Bink vtable. | High |
| `WM_TIMER`, id `0x65` | Calls movie vtable `+0x04` update; if nonzero, invalidates the control; then calls movie vtable `+0x14` end/wrap test. | `OwnerDraw_Static_006153E0`. | High |
| `WM_DESTROY` / `WM_NCDESTROY` | Destroys movie handle and kills timer `0x65`. | `OwnerDraw_Static_006153E0`. | High |

Tiny ordering detail: `WM_TIMER` first updates/copies frames, then checks
end-of-movie. That means the final ready frame can be copied before the loop
decision runs.

## 3. Bink Movie Vtable Entries Used By Static `0x71A`

The generic movie handle switches to `vtable__BinkMovieHandle` at `0x007EE154`
when the resolved filename has extension `.bik`.

The static control uses these entries:

| Vtable offset | Thunk | Concrete target | Behavior |
|---:|---:|---:|---|
| `+0x04` | `0x005C0580` | `0x00433040` -> `0x00432E40` | Timer update: process/copy ready Bink frame(s) using stored rect. Return value is used as "frame changed". |
| `+0x0C` | `0x005C0540` | `0x00432C30` | `_BinkPause(handle, param)` wrapper. |
| `+0x14` | `0x005C0570` | `0x00432C50` | End/wrap test: returns true if current frame is `>= total_frames` or current frame moved below the last recorded frame. |
| `+0x18` | `0x005C05A0` | `0x00432AB0` | Target/clip setup helper. The decompiler is register-confused, but it writes object `+0x10/+0x14/+0x18/+0x1C` clipping fields. |
| `+0x1C` | `0x005C05D0` | `0x00432BD0` | `_BinkGoto(handle, frame, 1)` and clears object `+0x30`. Main menu loop passes frame argument `1`. |
| `+0x24` | `0x005C05E0` | `0x00432BF0` | Returns integer `1000 / (handle+0x14 / handle+0x18)` where handle+0x14 = fps numerator, handle+0x18 = fps denominator (corrected 2026-05-28: was `1000/fps`; binary shows ratio division via decompile of `BinkMovie_FrameDelayMs at 0x00432BF0`). For RA2TS 15/1 fps this is `66` ms. |
| `+0x28` | `0x005C05F0` | `0x00433060` | Explicit copy/draw path used by custom message `0x4F0`; copies current Bink frame buffer with `0x80000000` ORed into copy flags. |

Do not treat the vtable labels as VQA-only. These entries are Bink-specific once
the constructor installed `vtable__BinkMovieHandle`.

## 4. Timer Update Path

`OwnerDraw_Static_006153E0` handles `WM_TIMER` id `0x65` as:

```text
if movie_handle == null:
    return

changed = movie_handle.vtable[+0x04]()
if changed:
    InvalidateRect(hwnd, null, erase = 0)

ended = movie_handle.vtable[+0x14]()
if !ended:
    return

if loop_flag != 0:
    movie_handle.vtable[+0x1C](1)
    Register_heap_pool("Looping movie")
else:
    destroy movie_handle
    movie_handle = null
    KillTimer(hwnd, 0x65)
```

Constants:

| Constant | Meaning | Evidence |
|---:|---|---|
| `0x65` | Movie timer id for owner-draw static. | `OwnerDraw_Static_006153E0`. |
| `0x22` | Timer interval in milliseconds when a movie handle is assigned. | `OwnerDraw_Static_006153E0`. |
| `1` | Loop flag passed by main menu through `0x4E3`; also frame argument passed to Bink goto on loop. | `FUN_00531CC0`, `OwnerDraw_Static_006153E0`, `0x00432BD0`. |

Player-visible consequence: the shell does not wait for the static's normal
`WM_PAINT` to render the movie. The timer drives update/invalidation, and the
parent menu's paint path sends custom `0x4F0` for explicit copy.

## 5. Bink Update And Copy Algorithm

The core Bink update is `0x00432E40`. It is reached from `0x00433040` during
timer updates and also has related copy paths at `0x00433060`.

Pseudocode for the update path:

```text
if global_audio_volume changed:
    BinkSetVolume(handle, ftol(global_audio_volume))

if g_GameRunning == 0:
    if object.playing_flag == 1:
        object.playing_flag = 0
        BinkPause(handle, 1)
else:
    if object.playing_flag == 0:
        object.playing_flag = 1
        BinkPause(handle, 0)
        copy current frame to target once

if object.force_frame_flag == 0:
    if BinkWait(handle) != 0:
        return false

do:
    BinkDoFrame(handle)
    object.force_frame_flag = 0

    if object.surface_event_state exists and
       event_check(current_frame * object.ticks_per_frame):
        clear/fill affected rect

    lock destination surface
    if lock succeeded:
        copy_flags = object.dd_surface_type
        if clear/fill path fired:
            copy_flags |= 0x80000000
        BinkCopyToBuffer(handle, locked_ptr, pitch, height, x, y, copy_flags)
        unlock destination surface

    post-copy blit helper
    object.last_frame_seen = handle.current_frame
    BinkNextFrame(handle)
while BinkWait(handle) == 0

return true
```

Verified tiny details:

| Detail | Evidence | Confidence |
|---|---|---:|
| `_BinkWait` is checked before `BinkDoFrame` unless object byte `+0x2D` is set. | `0x00432E40`. | High |
| The update can process multiple frames in one call: it loops while `_BinkWait` returns `0` after `_BinkNextFrame`. | `0x00432E40`. | High |
| Object `+0x30` records the Bink current-frame value before `_BinkNextFrame`. | `0x00432E40`. | High |
| End/wrap test uses Bink handle `+0x0C` current frame and `+0x08` total frame count, plus object `+0x30` last-frame marker. | `0x00432C50`. | High |
| Loop reset calls `_BinkGoto(handle, 1, 1)` and clears object `+0x30` to `0`. | `0x00432BD0`. | High |
| Explicit draw `0x4F0` calls `0x00433060`, which copies using `copy_flags | 0x80000000`. | `OwnerDraw_Static_006153E0`, `0x00433060`. | High |

## 6. Timing Implications For RA2TS

`ra2ts_s.bik` and `ra2ts_l.bik` are 15 fps and 431 frames by current asset
survey. The Bink object computes two different timing-related values:

| Value | Formula | RA2TS result | Evidence |
|---|---|---:|---|
| Object `+0x24` ticks-per-frame | `int(0x3C / (handle+0x14 / handle+0x18))` where handle+0x14 = fps numerator, handle+0x18 = fps denominator (corrected 2026-05-28: was `int(0x3C/fps)`; binary shows ratio division via decompile of `FUN_00432750` — ROOT_CAUSE: INFERENCE_HARDENED) | `4` ticks | `0x00432750`; 15 fps header. |
| Vtable `+0x24` delay | `int(1000 / (handle+0x14 / handle+0x18))` (corrected 2026-05-28: was `int(1000/fps)`; same pattern via `BinkMovie_FrameDelayMs at 0x00432BF0` — ROOT_CAUSE: INFERENCE_HARDENED) | `66` ms | `0x00432BF0`; 15 fps header. |

The owner-draw static timer is still `0x22` ms (`34` ms), roughly 29.4 Hz. That
timer is not the movie frame rate. It is the polling/update cadence around Bink,
and Bink's `_BinkWait` decides whether a frame is ready. At 15 fps, a new movie
frame is expected about every 66.7 ms, so most timer ticks will not advance a
frame.

Implementation implication: do not advance the RA2TS movie every 34 ms. Use the
Bink frame timing/readiness model, or an equivalent 15 fps schedule with catch-up
behavior, while keeping the shell invalidation cadence separate if needed.

## 7. Archive Priority For `LANGMD.MIX` vs `LANGUAGE.MIX`

`WinMain` opens both language archives directly:

| Call site | Filename pointer | String | Open order |
|---:|---:|---|---:|
| `0x006BD7EF` | `0x00840D5C` | `LANGMD.MIX` | first |
| `0x006BD81F` | `0x00840D4C` | `LANGUAGE.MIX` | second |

The MIX file constructor at `0x005B3C20` inserts successfully opened MIX
archives into the global search list by writing the new node immediately after
the list sentinel:

```text
iVar4 = DAT_00ABEFF0
new.prev = iVar4
new.next = *(iVar4 + 4)
if old_next != 0:
    old_next.prev = new
*(iVar4 + 4) = new
```

`FUN_005B4430` performs file lookup by starting from `DAT_00ABEFE0` and walking
the `+4` next pointer. Because `DAT_00ABEFE0` is the sentinel's first node,
newly opened archives are searched first.

Therefore, although `LANGMD.MIX` is opened first, `LANGUAGE.MIX` is opened
second and becomes earlier in the search list. For duplicate entries with the
same hash, `LANGUAGE.MIX` wins over `LANGMD.MIX`.

This resolves the previous `ra2ts_l.bik` mismatch:

- Direct physical parse of `LANGMD.MIX` found `ra2ts_l.bik` header largest frame
  field `49984`.
- Current Rust `bik-survey` first-match result reports `largest=50848` for
  `ra2ts_l.bik`.
- Since gamemd searches `LANGUAGE.MIX` before `LANGMD.MIX`, the Rust first-match
  result is consistent with the expected gamemd duplicate priority for this
  asset.

## 8. Current Rust Status

No Rust code was changed during this investigation.

Relevant current behavior:

- `src/assets/asset_manager.rs` documents first-match archive lookup.
- `OPTIONAL_TOP_LEVEL` lists `language.mix` before `langmd.mix`.
- `AssetManager::rebuild_indexes` uses `or_insert`, so the first archive in the
  Rust search order wins.
- `cargo run --bin bik-survey -- ra2ts` succeeds and reports:

```text
[ OK      / 431] ra2ts_l.bik  632x570  kf=[0]  largest=50848  maxpkt=50848
[ OK      / 431] ra2ts_s.bik  472x450  kf=[0]  largest=54096  maxpkt=54096
```

`cargo run --bin bik-survey -- ra2ts --archives` confirms that both
`language.mix` and `langmd.mix` contain BIK entries and that current Rust serves
two resolvable BIK names from `language.mix` and one from `langmd.mix`.

Rust still lacks the main-menu integration:

- no dialog `0xE2` render path,
- no static `0x71A` RA2TS movie surface,
- no Bink playback cadence integrated into the shell,
- no parent `WM_PAINT` equivalent sending the explicit `0x4F0` draw.

### 2026-07-25 current-Rust correction

The preceding absence list described the 2026-05-17 checkout and is no longer
current. Production `dev` at `e726da11` now has:

- a dedicated dialog `0xE2` layout/state/render path;
- a retail-asset-backed `RA2TS_S`/`RA2TS_L` `BinkMovieSurface`;
- width-selected archive lookup with `language.mix` duplicate priority;
- a separate Single Player `0x100` shell that requests its own RA2TS session;
- owner/base-qualified decoder/texture reuse across steady paints; and
- explicit session destruction before the non-movie `0x102` Skirmish dialog.

This does **not** certify exact `WM_PAINT`/`0x4F0` equivalence, first-visible
frame, timer phase, transition pixels, or physical presentation. A post-merge
review also found that `e726da11` can preserve the old `0xE2` session during a
queued `0xE2 -> 0x100 -> Back` round trip when `0x100` never paints. Reviewed
feature commit `3a96251e` clears the session synchronously at both route edges
and makes owner/base identity atomic, but it was not yet merged into `dev` when
this correction was written. The production verdict for that collapsed route
therefore remains `DRIFT`; aggregate lifecycle pixels and timing remain
`UNVERIFIED`.

## 9. Open Questions

1. A final pixel comparison still needs a retail capture. The binary playback
   path is now understood, but capture is needed to confirm exact first visible
   frame, window coordinates, and clipping after the Win32 dialog is realized.
2. The full global MIX priority across every archive was not exhaustively
   linearized here. For RA2TS duplicates, the only priority relationship needed
   is `LANGUAGE.MIX` before `LANGMD.MIX`, which is verified.
3. `0x00432AB0` clip setup has register-confused decompilation due the thunk
   calling convention. Its side effect on object `+0x10/+0x14/+0x18/+0x1C` is
   clear, but a future pass could hand-disassemble it if exact clipping edge
   cases become suspect.

## Sources

- Ghidra/live binary functions:
  - `OwnerDraw_Static_006153E0` `0x006153E0`
  - `BinkMovieHandle` vtable `0x007EE154`
  - Bink update thunk `0x005C0580`
  - Bink end/wrap thunk `0x005C0570`
  - Bink goto thunk `0x005C05D0`
  - Bink explicit draw thunk `0x005C05F0`
  - `0x00433040` stored-rect update wrapper
  - `0x00432E40` Bink wait/do/copy/next update
  - `0x00432C50` end/wrap test
  - `0x00432BD0` `_BinkGoto(handle, frame, 1)`
  - `0x00432BF0` `1000 / fps` delay helper
  - `0x00433060` explicit copy to primary/stored target
  - `0x005B3C20` MIX file constructor/list insertion
  - `0x005B4430` global MIX file lookup
  - `WinMain` call sites `0x006BD7EF` and `0x006BD81F`
- String/memory checks:
  - `0x00840D5C` = `LANGMD.MIX`
  - `0x00840D4C` = `LANGUAGE.MIX`
  - `s_Looping_movie_00835958` used after loop reset
- Local commands:
  - `cargo run --bin bik-survey -- ra2ts`
  - `cargo run --bin bik-survey -- ra2ts --archives`
- Rust source inspected:
  - `src/assets/asset_manager.rs`
  - `src/bin/bik-survey.rs`
  - `src/bin/bik-player.rs`

## Related reports (added 2026-05-18 main-menu --area swarm)

The 2026-05-18 main-menu swarm produced five new reports. Most relevant
to this Bink-focused doc:

- `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` — the audio companion
  to the visual Ra2ts playback documented here. Shell music = `[INTRO]`
  theme, `Sound=Drok, Repeat=yes`; queued from `Main_Game @ 0x0052D9A0`
  immediately before the dialog `0xE2` movie panel is created. Loop is
  poll-based via `Theme::AI`, not a Bink completion callback.
- `EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` — verified-negative:
  no EVA voice fires alongside the Ra2ts Bink playback.

Other reports in the same swarm (less directly relevant):

- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`
- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`
