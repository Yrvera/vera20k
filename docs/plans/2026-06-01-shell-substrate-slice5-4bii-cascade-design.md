# Slice 5 sub-step 4b-ii — graceful quit cascade (design + PROOFED timing)

Design spec for the graceful quit cascade that runs **after** the main-menu Exit-confirm OK,
**after** 4b-i's settings persist, and **before** the window closes. 4b-i (persist `[Audio]
ScoreVolume` to `RA2MD.INI` strictly before teardown) is implemented; this sub-step adds the
audible/visible teardown. Package `vera20k`. Parity bar: indistinguishable from gamemd.exe.

The port reproduces the **observable** cascade with its own systems via a **non-blocking
per-frame state machine** — gamemd busy-spins the thread, which the port must NOT do (the winit
event loop must keep turning). Match the player-visible result, not the C++ plumbing.

---

## 1. VERIFIED-FROM-BINARY cascade (all PROOFED this pass via Ghidra MCP)

Teardown runs in `Main_Game @0x0052D9A0` **case 7** (jump-table entry 8 → `0x0052E764`), in this
order (`disassemble_function 0x0052D9A0`):

1. **`Theme::Stop(1)` @0x0052E76B** — start the music fade-out. `Theme::Stop` = `FUN_00720EA0`;
   the char param gates the fade branch (`disassemble_function 0x00720EA0` @00720ed3
   `MOV AL,[ESP+8]; TEST AL,AL; JZ hard-stop`). **fade=1 is NON-BLOCKING**: it sets the
   theme/score volume-interpolator **target to 0** and returns (`decompile_function 0x004080c0` →
   `VolumeInterp__SetTarget 0x00407170`; no loop/wait). The actual ramp is a per-tick wall-clock
   volume interpolation (`VolumeInterp__Tick 0x00407210`, driven by `SoundSystem__UpdateTick
   0x004041d0`).
   - **Rate = full-scale (`0x4000`) over 1000 ms** ⇒ fade time = `current_volume × 1000 ms`
     (full→1000 ms, half→~500 ms, default 0.4→400 ms). PROOFED: the rate setter `FUN_004071a0`
     has exactly one caller `FUN_00401000`, which always passes divisor `0x3E8`=1000; timestamps
     are in **ms** (`Timer__InitPerformanceCounter 0x00409393` divides QPF by 1000).
2. **Vox-wait loop @0x0052E79C–0x0052E7E4** — runs **concurrently** with the music fade (control
   falls straight in after the non-blocking `Theme::Stop(1)`). Three exit conditions, exits the
   instant **any** fires (`disassemble_function 0x0052D9A0`):
   - (a) music stream no longer playing/fading — gate `FUN_00720FD0 @0x0052E79C` → `JZ` exit;
   - (b) voices done — gate `VoxClass__PumpAndCheckActive @0x007529E0` → `JZ` exit (pumps the
     voice queue, returns 1 while any voice/stream/queue active, 0 when done);
   - (c) deadline `baseline + 0xBB8` ticks exceeded → `JLE` exit.
   Each iteration calls `Network_ServiceLoop FUN_0048D080` (sound + net pump, **no render, no
   Sleep** — a tight CPU-spin service loop). The `0xBB8`=3000 ticks × 16 ms = **~48 s is a SAFETY
   CEILING only**; typical real duration = the trailing voice / short music-fade, exits at once
   when both are done.
   - `GetRadarTimer @0x006C8C40 = timeGetTime() >> 4` ⇒ **16 ms/tick** (`disassemble_function
     0x006C8C40`).
3. **`Theme::Stop(0)` @0x0052E7EC** — immediate hard music stop (`StreamPlayer__Stop 0x00407F40`,
   synchronous, in a critical section). Truncates the fade if a voice ended first.
4. **`FUN_004A3C30(palette=0x00884E80, duration=0x1E, callback=NULL)` @0x0052E805** — palette
   **fade-to-BLACK**, gated on `DAT_008175b0 == 0`. CORRECTION vs prior notes: ECX/param_1 =
   `0x00884E80` = an **all-zero (black) palette** (`read_memory 0x00884E80` = all `0x00`); the
   `0` is the NULL callback (3rd arg). `0x1E`=30 is a **DURATION in GetRadarTimer ticks**, not a
   step count (`disassemble_function 0x004A3C30` @004A3CBD `CALL GetRadarTimer; SUB; CMP ESI;
   JGE exit`). ⇒ **30 × 16 ms = ~480 ms**. BLOCKS (busy-spin on `GetRadarTimer`, no Sleep, no
   pump since callback NULL). Linear per-channel RGB lerp toward black, level =
   `elapsed × 256 / 30` (`decompile_function 0x00626120/0x006612C0`); ~31 palette presents
   (`FUN_004A4780` = SetEntries).
   - **`DAT_008175b0` gate** = `(DDraw vtable+0x70 == 2)`; the fade only runs in **8-bit
     palettized mode** (`== 1`). Standard RA2/YR menu **is** palettized → the fade runs. This is
     an internal DirectDraw display-mode artifact with no analog in the port's wgpu renderer.
5. **`XOR AL,AL; RET`** — returns 0. **No `PostQuitMessage`/`ExitProcess`.**

**Net observable:** settings saved → menu music fades out (~0.4–1 s) while any trailing
EVA/menu voice plays → music hard-stops → screen fades to black (~480 ms) → window closes.

---

## 2. Port mapping — non-blocking `QuitCascade` per-frame state machine

Modeled on `ShellFrameWave` (`src/app_shell_transition.rs`): an `Instant`-based phase animation
ticked once per frame in `render_frame` (`app.rs:2614`), advancing on elapsed wall-clock, never
blocking the loop. The continuous-redraw loop already runs (`about_to_wait → request_redraw`,
`app.rs:2250`), so the machine self-drives.

New field on `AppState`: `quit_cascade: Option<QuitCascade>` (+ an edge/`Instant` as needed).

```
enum QuitPhase {
    FadeMusicAndWaitVoices, // ramp music vol→0 at 1.0/1000ms; poll voices_active()
    StopMusic,              // MusicPlayer::stop() — instant hard stop
    FadeToBlack,            // black overlay alpha 0→1 over 480 ms
    Done,                   // event_loop.exit()
}
```

Phase transitions (reproducing the gamemd semantics above):

1. **FadeMusicAndWaitVoices** — each frame: lower `MusicPlayer` volume by `elapsed_ms × 0.001`
   (rate 1.0 per 1000 ms), clamped ≥ 0, via `set_volume`; poll `SfxPlayer::voices_active()`.
   **End** when `music_volume == 0` **OR** `!voices_active()` **OR** `elapsed ≥ 48 s` (ceiling) —
   matching gamemd's OR-exit + `0xBB8` bound.
2. **StopMusic** — `MusicPlayer::stop()` (hard). Instant; one tick.
3. **FadeToBlack** — ramp a full-screen black overlay `alpha = clamp(elapsed_ms / 480.0, 1.0)`
   each frame; draw the black quad over the composed menu (last draw in the menu pass). **End**
   when `elapsed ≥ 480 ms` (alpha 1.0, fully black). gamemd's linear `elapsed×256/30` curve = a
   linear alpha ramp; the 30-step quantization is internal — a smooth ramp matches the observable.
4. **Done** — `event_loop.exit()`.

**Input frozen** during the cascade (gate analogous to `blocks_shell_input`,
`app_shell_transition.rs:207`): swallow mouse/keyboard in `window_event` while `quit_cascade` is
`Some`, so the player can't re-enter the menu mid-fade.

**Persist ordering preserved:** the OK handler still calls `persist_settings_on_quit(state)` (4b-i)
FIRST — capturing the pre-fade volume — then starts the cascade instead of `event_loop.exit()`.

---

## 3. New facilities (small, additive)

- **`SfxPlayer::voices_active(&self) -> bool`** (`src/audio/sfx.rs`) =
  `voice_player.as_ref().is_some_and(|p| !p.empty()) || !queued_voice.is_empty()`. The non-blocking
  predicate already exists privately (sfx.rs:323/367); this exposes it. `rodio Player::empty()` is a
  non-blocking poll.
- **Music fade**: no new audio API — the cascade owns the ramp via the existing
  `MusicPlayer::set_volume` (music.rs:239), which applies live to the current track.
- **Black overlay**: reuse the already-baked 1×1 opaque `white_pixel` in the skirmish chrome atlas
  (`skirmish_shell_chrome.rs:84/319`), stretched full-screen with `tint=[0,0,0]`, `alpha=progress`,
  via the existing `draw_with_buffer_passthrough` ALPHA_BLENDING pipeline (`batch.rs:529/1364`) —
  same `push_tinted_entry` pattern used at `controls.rs:271`. Drawn as the final passthrough call in
  `render_main_menu_shell_to_target` (`app_main_menu_shell_render.rs:574`). No new
  shader/pipeline/texture. (Shader discards `color.a < 0.01`, so the **opaque** white_pixel is
  required — verified.)

---

## 4. Integration points

- **OK handlers** — replace the terminal exit with `Self::start_quit_cascade(state)`:
  - SHP path: `handle_exit_confirm_modal_mouse_up` OK arm (currently `app.rs` ~1679–1685; persist +
    `exit_confirm_modal=None` + `event_loop.exit()`). Start cascade in place of `exit()`.
  - egui-fallback path: `draw_main_menu_dialogs` Confirm arm (~1934–1941; persist + `return true` →
    caller exits at ~755/2782). The fallback fires only when SHP chrome failed to load, so the
    `white_pixel` overlay is unavailable there — see §6 open item; the audio phases still apply.
- **Per-frame tick** — advance the cascade in `render_frame` (`app.rs:2614`) alongside the existing
  `ShellFrameWave` ticks (2671/2689); call `event_loop.exit()` (in scope) when phase reaches `Done`.
- **Render** — black overlay appended in `render_main_menu_shell_to_target` before `drop(pass)`
  (`app_main_menu_shell_render.rs:574`), alpha from the cascade's `FadeToBlack` progress.

---

## 5. Sub-stepping (per kickoff "consider sub-stepping 4b")

Two independently-buildable, independently-verifiable steps:

- **4b-ii-a — audio cascade**: `voices_active()`, the `QuitCascade` machine with
  `FadeMusicAndWaitVoices → StopMusic → Done` (no screen fade yet — exit after music stop), input
  freeze, OK-handler interception. Player-observable: music fades out, trailing voice plays, then
  window closes.
- **4b-ii-b — screen fade-to-black**: add the `FadeToBlack` phase + the black overlay draw before
  `Done`. Player-observable: the ~480 ms fade-to-black before the window closes.

---

## 6. ACCEPTANCE TESTS

- **Pure phase-machine test** (headless, elapsed-ms-driven — inject a `voices_active` closure and a
  starting volume; no `&mut AppState`, no event loop): advances
  `FadeMusicAndWaitVoices → StopMusic → FadeToBlack → Done`; music volume reaches 0 by
  `min(volume×1000ms, voice_end_ms)`; `FadeToBlack` lasts 480 ms; **no exit signal before `Done`**;
  the 48 s ceiling caps a never-ending voice.
- **`voices_active()` unit test** on `SfxPlayer` (idle → false; voice playing → true; queue
  non-empty → true).
- **Music fade rate**: from volume `v`, reaches 0 in `v×1000 ms` (±1 frame).
- **Render**: black overlay `alpha = clamp(elapsed/480, 1.0)`, full-screen, `tint=[0,0,0]`.
- **Persist-before-cascade** (extends 4b-i §7.C): persist runs before the cascade mutates volume and
  before any exit.
- Skirmish safety net (`src/ui/skirmish_shell/state/tests.rs`, 87 tests) GREEN + UNCHANGED.

---

## 7. OPEN / UNCHECKED

- **Does the port queue an EVA/menu voice on quit?** If none is active, `FadeMusicAndWaitVoices`
  ends as soon as the music fade completes (no trailing-voice wait). Verify in-game; LOW.
- **egui-fallback screen fade**: `white_pixel` (skirmish chrome atlas) is unavailable on the degraded
  fallback path. Options: a `clear`-to-black / egui black panel ramp, or skip the screen fade on the
  fallback (audio phases still run, then exit). Decide in the plan; LOW (rare path).
- **Music-fade-vs-voice truncation**: modeled per gamemd's OR-exit (fade truncated by `Theme::Stop(0)`
  if the voice ended first; or the fade completes and a longer voice plays through the screen fade,
  cut at window close). Faithful but a sub-second nuance; LOW.
- **480 ms granularity**: gamemd quantizes to 30 palette steps (~16 ms apart); the port uses a smooth
  linear ramp over the same 480 ms (same linear curve, same endpoints). Observable-equivalent.

---

## 8. Cadence

- ONE sub-step per cycle (4b-ii-a, then 4b-ii-b). Each: `cargo check -p vera20k` then
  `cargo test -p vera20k` as a **separate foreground** pass; read the literal `test result:` line.
- `sim/` never depends on `ui/`/`render/`.
- **STOP for the user's in-game OK before committing** each sub-step to `dev`.
- After implementing: a short **adversarial review workflow** (order = persist→fade→wait→stop→
  black→exit; tick-cap gated on the voice/music-active checks not a fixed sleep; non-blocking against
  the frame loop; fade durations match the PROOFED ~`volume×1000 ms` / ~480 ms).
