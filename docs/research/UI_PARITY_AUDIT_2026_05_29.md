# UI Parity Audit — Main Menu / Skirmish Shell / Loading Screen vs gamemd.exe

**Date:** 2026-05-29
**Status:** ghidra/verified (adversarial pass complete)

## 1. Scope & Method

This audit covers three shell-layer surfaces of the RA2 Rust engine and measures their
player-visible output against retail `gamemd.exe`:

- **Main Menu** (dialog 0xE2 shell: background, movie panel, right-panel button stack, title, version line, hover/tooltip, button-click transition, menu music/EVA, downstream menu dialogs).
- **Skirmish Shell** (dialog 0x102 setup: combos, trackbars, player-name edit, choose-map modal, validation modal, hit-testing, start→loading handoff).
- **Loading Screen** (the native skirmish loading composition: LS country background, mmpb assigned-player markers, PROGBARM progress bar, side icon, progress text, color formulas).

Method: docs-first (verified Ghidra reports under `docs/research/`) with live Ghidra escalation
for ambiguous claims, then an adversarial verification pass that defaulted every difference to
DRIFT and demanded algebraic-proof / bit-identical / exhaustive-caller evidence to downgrade.
Each finding below was independently re-verified; **verifier corrections are folded in** and
flagged. Per CLAUDE.md, severity = player-visibility × trigger-frequency; "rare/edge" does **not**
downgrade a verdict, only its fix priority.

Tally: **74 confirmed drifts**, **1 needs-research (UNCHECKED)**, **49 false positives checked**.

> **Note on the audit's own severity labels:** several findings carried inflated
> `trigger_frequency` claims that the verifier corrected (e.g. "every-match" → conditional).
> Final severity columns below reflect the *verified* trigger frequency, not the original claim.

---

## 2. Executive Summary

### Counts by surface

| Surface | Confirmed drifts | Needs-research |
|---|---|---|
| Main Menu | 31 | 0 |
| Skirmish Shell | 28 | 1 |
| Loading Screen | 15 | 0 |
| **Total** | **74** | **1** |

### Counts by final severity

| Severity | Count |
|---|---|
| HIGH | 27 |
| MEDIUM | 21 |
| LOW | 26 |

### Top 5 highest-priority holes (player-visibility × trigger-frequency)

1. **`mm-composition-missing-mnscrnl-background`** — The parent shell background
   (MNSCRNL.SHP / MNSCRNS.SHP) is never composited. Every screen region outside the movie
   panel and right panel renders CLEAR_COLOR instead of retail art. *Fires on every main-menu
   load; the menu looks fundamentally wrong.*
2. **`mm-hover-tooltip-no-software-shp-cursor`** — Main menu shows the native Windows cursor;
   gamemd hides the OS cursor (`ShowCursor(0)`) and software-blits MOUSE.SHA frame 0. *Wrong
   cursor on every menu, every frame.*
3. **`mm-dialogs-quit-confirm-missing` + `mm-dialogs-options-case5-missing` +
   `mm-dialogs-movies-credits-case4-missing` + `mm-dialogs-sp-newcampaign-noop`** — Four of the
   six main-menu buttons are no-ops or skip their dialog (Exit quits with no confirm/options-save;
   Options, Movies & Credits, New Campaign open nothing). *Half the menu is dead.*
4. **`ss-launch-random-country-rejected`** — Clicking Start with any slot on "Random" hard-errors
   in Rust (`RandomSelectionUnverified`) instead of resolving via native RNG. *Blocks a common
   skirmish setup that gamemd starts normally.*
5. **`ls-progress-bar-fill-tint-not-remap` + `ls-progress-mmpb-marker-missing` +
   `ls-progress-backing-shade-approximated`** — The loading screen omits the assigned-player
   mmpb markers entirely and uses ramp-shade tint approximations for the bar fill/backing instead
   of the native draw. *The loading screen is visibly wrong every match.*

### Architecture-rework flags (not point fixes)

- **Main-menu→Skirmish transition system** (`mm-transition-*`, 9 findings) — the entire
  `ShellBridgeTransition` whole-screen compositor is a self-labeled DRIFT bridge that does not
  exist in gamemd on this path. gamemd shows an **instant dialog swap**. This is not a tuning
  fix; the compositor should be removed for this path (or re-architected to model the native
  per-cell SHP-frame slide that runs only on *other* dialogs).
- **Options persistence layer** (`mm-audio-eva-*`) — there is no `RA2MD.INI [Options]` read path
  at all; music volume and all option state are hardcoded. This is a missing subsystem, not a
  constant.
- **Native cursor pipeline on shell screens** (`mm-hover-tooltip-no-software-shp-cursor`) — the
  software MOUSE.SHA cursor only activates on the in-game map transition; the shell layer needs
  it wired in.
- **Loading-screen color pipeline** (`ls-progress-*`, `ss-launch-bar-colors-*`) — the bar
  fill/backing use ramp-shade approximations and a flat tint instead of the native raw-SHP /
  HSV-from-scheme draw. Multiple findings share this root cause (merged below).

---

## 3. Main Menu

### 3.1 Composition (`mm-composition`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-missing-mnscrnl-background | Parent shell background MNSCRNL/MNSCRNS.SHP never composited | missing | HIGH | every-match |

- **gamemd:** `WM_PAINT_Handler @0x00621E90` draws the generic parent background via
  `Background_Overlay @0x0072E730` (MNSCRNS.SHP at width 640, else MNSCRNL.SHP) through SHELL.PAL
  into the per-dialog offscreen BSurface, filling the whole screen behind the movie + right panel.
- **Rust:** `build_main_menu_shell_chrome_atlas` (src/render/main_menu_shell_chrome.rs:48-109)
  loads only SDTP/SDBTNBKGD/SDBTM/LWSCRNS/L/SDBTNANM; `build_chrome_instances`
  (src/app_main_menu_shell_render.rs:210-241) draws no parent background. The render pass clears
  to CLEAR_COLOR, leaving uncovered regions blank.
- **Evidence:** MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md (record[0x39]=
  MNSCRNS, [0x3A]=MNSCRNL); SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md §2.Q4.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-version-line-uses-versiontxt-contents | Version line pastes VERSION.TXT contents instead of fixed VersionClass label | drift-other | LOW | conditional |

- **gamemd:** `VersionClass @0x00A8ECE0 / FUN_0074FAE0` always formats the uint16 build pair with
  `%d.%3.3dTUC` (defaults → `1.001TUC`). VERSION.TXT bytes go to a separate buffer and are never
  pasted into the visible label.
- **Rust:** src/app.rs:663-679 reads VERSION.TXT and uses its trimmed contents verbatim as the
  second token; only the absent-file case falls back to `1.001TUC`.
- **Verifier correction:** trigger is **conditional on VERSION.TXT present**, not every-match.
  The retail install has no VERSION.TXT, so both sides currently output `1.001TUC` and the player
  sees no difference. The OWNERDRAW doc §10.3 ("final text = VERSION.TXT contents") is an
  imprecise paraphrase; VERSION_TXT doc §6 (verified via `decompile_function 0x0074FAE0`) is
  authoritative.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-egui-fallback-menu-not-gamemd | Fallback main menu is an egui placeholder, not dialog 0xE2 | drift-other | MEDIUM | rare |

- **gamemd:** Dialog 0xE2 = Bink movie + MNSCRNL background + right-panel SHP stack + six
  SDBTNANM buttons + GAME.FNT title + version line. No map dropdown, credits dropdown, or
  Allow-Zoom checkbox.
- **Rust:** On shell-asset load failure (`main_menu_shell_failed` or chrome `None`),
  `render_egui_main_menu_fallback` (app.rs:696-724) → `draw_main_menu_with_maps`
  (src/ui/main_menu.rs:163-298) renders a wholly non-parity egui layout. (Note: the
  `main_menu_show_skirmish_setup` branch at app.rs:2352 is dead code — never set true.)
- **Evidence:** src/app_main_menu_shell_render.rs:392-396; MAIN_MENU_DIALOG_0XE2 control table.
- **Trigger:** only on retail-asset load failure (error condition).

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-loading-screen-overlay-non-gamemd | Loading screen uses egui rounded-rect + English strings | drift-other | HIGH | every-match |

- **gamemd:** WM_PAINT mode-2 (`@0x00621E90`) draws frame 0 of the side-selected PUDLGBG*.SHP
  (Allied/Soviet/Yuri/Neutral) at (0,0) via CC_Draw_Shape — no text, no progress overlay, no
  English strings (zero text-draw calls between 0x006221F4 and 0x006222AD).
- **Rust:** src/ui/main_menu.rs:300-391 draws a 430×132px rounded dark egui overlay with
  hardcoded English strings ("Mission deployment", "Loading...", "Map: {name}", "Parsing map...")
  in proportional fonts; `loading_screen_image()` always returns `None`.
- **Verifier correction:** the finding's "uncertain" confidence on the gamemd side is
  **unwarranted** — multiple verified reports (LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION,
  PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE, LOADING_PROGRESS_CALLBACK_VISIBLE_UI) fully establish it.
  Severity is HIGH.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-button-art-sdbtnanm-frames-correct | Button art SOURCE correct, but hover frame-3 selection is WRONG | unchecked→drift | MEDIUM | every-match |

- **Asset source is correct:** `LAB_0060A330` writes `record+0xB0=1` for all six buttons → the
  `iVar14==1` branch of `OwnerDraw_Button_00612B70` uses SDBTNANM frames 2/3/4 with the SDBTNANM
  palette convert. Rust loads frames 2/3/4 with SDBTNANM.PAL identically. (The greyscale
  bue/bde PCX path is never reached on 0xE2.)
- **Verifier correction (now DRIFT):** the hover-frame *selection* is wrong. Frame 3 is gated on
  `button+0xC5`, set only by the WM_TIMER toggle armed by message `0x4DC`. HOVER_DISPATCHER_
  FUN_007B66C0_FAMILY §3 enumerates all six `push 0x4DC` sites — **all target GetDlgItem(_, 0x59F)
  (a network-dialog control)**, never a dialog-0xE2 button. So `+0xC5` is never set; gamemd shows
  frame 2 (default) until pressed (frame 4). Rust (src/app_main_menu_shell_render.rs:150-153)
  shows frame 3 after 1s hover. (Root cause is shared with `mm-buttons-fabricated-hover-flash`.)

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-sdbtnanm-palette-fallback-shell2 | SDBTNANM.PAL falls back to SHELL2.PAL when missing | drift-asset | LOW | rare |

- **gamemd:** Button art is colorized through the dedicated SDBTNANM.PAL convert
  (`DAT_00B0FBDC`, distinct pointer from SHELL2.PAL's `DAT_00B0FBD4`). No SHELL2.PAL substitution
  exists; a missing SDBTNANM.PAL would null-deref/crash, not silently substitute.
- **Rust:** src/render/main_menu_shell_chrome.rs:58-59 falls back to SHELL2.PAL, which renders
  the gradient in wrong colors. Retail ships SDBTNANM.PAL so this fires only on load failure.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-button-art-rightanchored-scaled-not-centered | Button art uses WRONG asset family (SDBTNANM SHP, not PCX), scaled+right-anchored | drift-layout | HIGH | every-match |

- **Verifier correction (drift is LARGER than stated):** gamemd does **not** draw SDBTNANM frames
  for main-menu buttons. `OwnerDraw_Button_00612B70` draws SDBTNANM only when `state+0xB0==1`;
  dialog-0xE2 buttons take the `state+0xB0==0` **PCX path**, blitting `bue_li30 / bue_mi30 /
  bue_ri30.pcx` at native size (30px released / 27px pressed, middle tiled by FUN_006BA3E0),
  vertically centered in the **162×37px** client (not 168×42). Evidence:
  BUTTON_FADE_EFFECT_VISUAL §3; SKIRMISH_OWNERDRAW_BUTTON_GEOMETRY_DOC_RECONCILIATION §7 asset
  matrix; MAIN_MENU_DIALOG_0XE2 button-rendering section.
- **Rust:** src/app_main_menu_shell_render.rs:78-99 scales the SDBTNANM frame by rect.w/168 ×
  rect.h/42 and right-anchors with a 12px left bevel.
- **Net:** wrong asset drawn, plus wrong scaling and anchoring on top — visible pixel mismatch on
  every button.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-button-rect-snapped-to-168x42-tile | Button hit/draw rect snapped to 168×42 chrome tile, not 162×37 client | drift-layout | HIGH | every-match |

- **gamemd:** Six owner-draw controls at DLU 425,Y,108,23 → pixel ~162×37, at distinct Y
  (203/247/291/335/379/536 at 800×600). The SDBTNBKGD tile (168×42) is a *separate* background
  element drawn behind them.
- **Rust:** src/ui/main_menu_shell/layout.rs:267-281 replaces each button's true client rect with
  the nearest 168×42 tile rect (width 168 vs 162, height 42 vs 37, Y tile-snapped 199/241/283…),
  shifting both draw and hit-test boundaries. The test at layout.rs:393 encodes the wrong values
  `(632,199,168,42)`.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-composition-button-x-632-vs-638 | Buttons/title anchored at x=632 (tile left) vs DLU-derived 638 | drift-layout | LOW | every-match |

- **gamemd:** Button client left = DLU 425 → `MulDiv(425,6,4)=638` px at 800×600, w=162. The
  168px tile (x=632) extends ~6px further left as bevel.
- **Rust:** button rect = tile rect (x=632, w=168), a 6px leftward shift of the whole column and
  a 6px wider click target. Evidence: MAIN_MENU_DIALOG_0XE2 ("425 DLU → 638 px under MulDiv");
  layout.rs:393.

### 3.2 Buttons (`mm-buttons`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-buttons-fabricated-hover-flash | Buttons show a 1 Hz frame-3 hover flash gamemd never produces on 0xE2 | drift-other | HIGH | every-match |

- **gamemd:** SDBTNANM frame 3 painted only when `record+0xC5 != 0`, set only by the WM_TIMER
  toggle armed by `0x4DC`. `MainMenuDialog0xE2_Proc (0x00531F60)` and launcher (0x00531CC0) send
  no 0x4DC, no SetTimer, no WM_MOUSEMOVE. All six `push 0x4DC` sites are lobby-only
  (netdlg2/wonline, GetDlgItem 0x59F). gamemd buttons sit at frame 2 on hover, frame 4 on press.
- **Rust:** `build_button_instances` selects frame 3 for the hovered button via
  `elapsed_ms/1000 % 2 == 1` (src/app_main_menu_shell_render.rs:146-164), a visible red/orange
  flash. Verified via `decompile_function 0x00531F60`, `0x00612B70`, `0x00531CC0`;
  `search_byte_patterns 68 dc 04 00 00` → 6 lobby-only sites;
  HOVER_DISPATCHER_FUN_007B66C0_FAMILY §1.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-buttons-website-control-id-mislabel | Website control modeled as ID 0x71b; gamemd's website button is 0x55F | drift-other | LOW | rare |

- **gamemd:** `FUN_00608CD0` matches the website button as control **0x55F**; 0x71B is an
  unrelated static. WM_COMMAND on 0xE2 handles only 0x683/0x684/0x686/0x578/0x55c/0x3ee.
- **Rust:** state.rs:15 names the variant `YuriWebsite0x71b`. Functionally inert — the variant
  is not in the 6-element hit-tested `buttons` array (it is a separate `website_static`), so no
  live hit-test depends on the wrong ID. Documentation/naming defect only.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-buttons-pressed-text-right-shift-uses-scaled-offset | Pressed text right-shift scales pressed_content_offset_x in responsive path | drift-layout | LOW | common |

- **gamemd:** Press shifts the text rect (left+=2, top+=4) → net **+1px right, +2px down**, fixed
  pixel deltas regardless of screen size.
- **Rust:** `compute_layout` (fixed path) sets offset_x=1 (correct). `compute_responsive_layout`
  (layout.rs:376) scales it: `((1.0*scale_x).round()).max(1)` → 2px at 1600-wide.
- **Verifier correction:** the Y press offset is **not** scaled — `PRESSED_CONTENT_OFFSET_Y=2.0f32`
  is a hardcoded constant returning 2.0 for all sizes. Only X drifts, and only on the responsive
  path.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-buttons-mousedown-sound-missing-disabled-gate | Click sound plays without the not-disabled gate | drift-other | LOW | rare |

- **gamemd:** `OwnerDraw_Button_00612B70` plays the click sound on WM_LBUTTONDOWN/DBLCLK only when
  `(char)piVar17[0x2f] == 0` (not disabled); a disabled button is silent.
- **Rust:** src/app.rs:1459-1467 plays the sound on any owner-draw hit, no disabled check. No
  main-menu button is disabled in standard YR today, so audible output currently matches; the gate
  is structurally absent (WwOnline 0x684 is the canonical offline-disabled candidate).

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-buttons-stale-state-comments-claim-mainmenu-timer | state.rs comments assert gamemd arms a hover SetTimer on the main menu | drift-other | MEDIUM | every-match |

- **gamemd:** The 1000ms SetTimer / `+0xC5` mechanism lives in `OwnerDraw_Button_00612B70`'s
  `0x4DC` handler, reached only by network-lobby dialogs; dialog 0xE2 never arms it.
- **Rust:** src/ui/main_menu_shell/state.rs:36-42,111-122 comments state the timer is armed "in
  the WM_LBUTTONDOWN/hover-mutator path" and cite gamemd's `piVar17[0x31]` guard as if it applied
  to the main menu. This is the documentation face of `mm-buttons-fabricated-hover-flash`; fix
  together.

### 3.3 Transition (`mm-transition`) — ARCHITECTURE

> All nine transition findings share one root cause: the `ShellBridgeTransition` whole-screen
> compositor is a self-labeled DRIFT bridge. gamemd shows an **instant dialog swap** on the
> main-menu→Skirmish click (no animation). On *other* dialogs where a slide does run, it is an
> SHP-frame-index wave on Win32 child controls, not a positional pixel slide + crossfade. The
> recommended action is to remove the compositor for this path, not tune it.

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-transition-should-not-exist-on-this-path | No between-page transition should exist on this path | drift-other | HIGH | rare |
| mm-transition-mechanism-is-pixel-slide-crossfade-not-shp-frame-wave | Transition is a pixel-slide+crossfade; gamemd's is an SHP-frame wave | drift-other | HIGH | rare |
| mm-transition-crossfade-and-alpha-not-in-gamemd | Source 30% fade + dest 0→1 crossfade have no gamemd counterpart | drift-other | MEDIUM | rare |
| mm-transition-destination-uses-skirmish-shell-not-shp-chrome | Destination is a full Skirmish render target, not SHP transition frames | drift-asset | MEDIUM | rare |
| mm-transition-input-blocked-whole-transition | All shell input blocked for the full ~420ms transition | drift-other | MEDIUM | rare |
| mm-transition-per-frame-30ms-tick-source | 30ms tick value correct but driven by wall-clock catch-up, not blocking Sleep | drift-timing | LOW | rare |
| mm-transition-resize-resolution-50pct-snap-no-gamemd-analog | Resize-during-transition 50%-progress snap has no gamemd analog | drift-other | LOW | rare |

- **`mm-transition-should-not-exist-on-this-path`:** Dialog 0xE2 click never sets the `+0xC1`
  slide gate (only writer is `FUN_00608380 @0x006083D3`, reachable only from
  `CDFileClass__Constructor` save-success). `FUN_00608070` bails; `DestroyWindow` is instant.
  Route `0x683→1→dialog0x100→0x579→0x0B→0x102` writes the result directly. Rust runs a 14×30ms
  compositor (app_shell_transition.rs:42-93). Evidence:
  SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md §1/§8 ("do NOT implement a between-page
  transition").
- **`mm-transition-mechanism-...`:** `FUN_006071E0` advances per-cell SHP frame indices
  (10→5→settle on show, 5→10→settle on close) with no per-frame X/Y translation and no crossfade;
  the only spatial offset is a single discrete +0x50 SDTP shift keyed by phase. Rust shader
  (shell_transition.wgsl:42-55) translates source −round(progress·32)px, dest
  +round((1−progress)·64)px and crossfades alpha. Evidence:
  SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md §2/§3.
- **`mm-transition-input-blocked-...`:** `transition_blocks_shell_input` returns
  `transition.is_some()` (app_shell_transition.rs:99-101); the transition IS already wired
  (`start_main_menu_to_skirmish`), so every click produces a 420ms input-dead window gamemd does
  not have. *(Verifier correction: the finding hedged "if ever wired" — it is wired now.)*
- **`mm-transition-per-frame-30ms-tick-source`:** `FUN_006071E0` runs a synchronous
  `Sleep(0x1E)=30ms` loop advancing one frame per iteration; Rust `advance_to` can collapse
  several steps in one slow render frame (app_shell_transition.rs:65-73). Output identical when
  render < 30ms/frame.

### 3.4 Hover / Tooltip (`mm-hover-tooltip`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-hover-tooltip-no-software-shp-cursor | Main menu shows OS cursor; gamemd hides it and blits MOUSE.SHA frame 0 | drift-asset | HIGH | every-match |

- **gamemd:** `WWMouseClass::Constructor (0x007B8730)` calls `ShowCursor(0)` for process lifetime
  and software-blits MOUSE.SHA cursor ID 0 frame 0 (hotspot 0,0, no animation) every flush.
- **Rust:** The egui/winit shell does not hide the OS cursor or draw MOUSE.SHA; the software
  cursor (`software_cursor`) is `None` until the in-game map transition (app_transitions.rs:130).
  Player sees the native Windows arrow. Evidence: MAIN_MENU_CURSOR_SHP_AND_RULES_GHIDRA_REPORT.md.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-hover-tooltip-phantom-hover-timer-state | hover_started_at timer state based on a refuted mechanism | drift-timing | HIGH | every-match |

- Same root cause as `mm-buttons-fabricated-hover-flash` / `mm-buttons-stale-state-comments`. The
  `hover_started_at` field (state.rs:36-42,116-122) drives the phantom 1 Hz frame toggle; the
  `piVar17[0x31]` guard the comment cites is the **error-set** guard (input-validation blink for
  network dialogs), not a hover guard. `+0xC5` stays 0 forever on dialog 0xE2.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-hover-tooltip-status-line-alignment-unverified | Tooltip status line H-centered; gamemd 0x695 is LEFT-aligned | unchecked→drift | HIGH | every-match |

- **Verifier resolution (now confirmed):** the 0xE2 template control 0x695 was read from the
  binary RT_DIALOG resource at VA `0x00BF7660`: style DWORD `0x50000200`
  (WS_CHILD|WS_VISIBLE|SS_CENTERIMAGE), bits 0 and 1 both 0. In `OwnerDraw_Static_006153E0`'s
  WM_PAINT, `GetWindowLongA(_, -0x10)` with bits 0,1 = 0 yields `uVar10 = 0x10` = **SS_LEFT**.
- **Rust:** src/app_main_menu_shell_render.rs:291-301 hard-codes `ShellAlign::H_CENTER` over the
  ~455px tooltip line — up to ~200px horizontal offset for short strings, on every button hover.

---

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-hover-tooltip-yuriwebsite-control-id-and-no-hover-tooltip | Website tooltip (STT:MainButtonYuriWebSite) unreachable; control mislabeled 0x71B | drift-other | MEDIUM | common |

- **gamemd:** `FUN_006040B0`'s 0xE2 branch maps control **0x55F → STT:MainButtonYuriWebSite**;
  the website button is hit-tested by `ChildWindowFromPointEx` like other buttons.
- **Rust:** the website control is absent from the 6-entry `buttons` array (layout.rs:305-330), so
  `hit_test_owner_draw_button` never returns it and the (correct) CSF key in
  `tooltip_csf_key_for_control` is dead code. Two drifts: wrong enum name (cosmetic) + missing
  hover tooltip (player-visible). Evidence:
  MAIN_MENU_DIALOG_0XE2_TOOLTIP_HOVER_FLOW_GHIDRA_REPORT.md §1.

### 3.5 Audio / EVA (`mm-audio-eva`) — ARCHITECTURE (options layer)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-audio-eva-music-volume-default-0.5-vs-0.4 | Menu INTRO music plays at hardcoded 0.5; ScoreVolume default is 0.4 | drift-constant | HIGH | every-match |
| mm-audio-eva-music-volume-not-read-from-options | Music volume never loaded from RA2MD.INI [Options] ScoreVolume | missing | HIGH | every-match |
| mm-audio-eva-loop-restart-not-instantaneous-poll-gap | INTRO loop seam gap sized by render-frame poll, not audio-pump poll | drift-timing | LOW | common |

- **gamemd:** `OptionsClass__SetDefaults (0x005FA350)` sets ScoreVolume = `param_1[0x10] =
  0x3ecccccd = 0.4` (Sound/Voice = 0.7); `OptionsClass__ReadFromINI (0x005fa620)` reads
  `[Audio] ScoreVolume` at struct +0x40 at startup and applies it to ThemeClass playback.
- **Rust:** `MusicPlayer::new()` hardcodes `volume: 0.5` (src/audio/music.rs:75); no options-INI
  read path exists anywhere in `src/`. INTRO is 25% louder than gamemd's default and ignores the
  player's saved level (and a music=0 player hears menu music). The dev-overlay slider is the only
  control and it does not persist. Evidence: `decompile_function 0x005FA350`;
  OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md.
- **`...loop-restart...`:** both poll-based with no gapless callback; Rust polls `player.empty()`
  per render frame (music.rs:204-223), gamemd polls `Theme::AI` per Win32 message-pump iteration.
  Poll periods differ; equivalence unproven.

### 3.6 Downstream Dialogs (`mm-dialogs`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| mm-dialogs-quit-confirm-missing | Exit skips GUI:ExitAreYouSure confirm + options-save + fade/vox shutdown | missing | HIGH | every-match |
| mm-dialogs-options-case5-missing | Options button no-op; launcher dialog 0xD5 unimplemented | missing | HIGH | every-match |
| mm-dialogs-movies-credits-case4-missing | Movies & Credits no-op; sub-panel/picker/credits unimplemented | missing | HIGH | common |
| mm-dialogs-sp-newcampaign-noop | New Campaign no-op; selector dialog 0x94 unimplemented | missing | HIGH | common |
| mm-dialogs-sp-submenu-movie-no-restart | SP/Movies submenu does not reposition+restart the intro movie | drift-other | MEDIUM | common |
| mm-dialogs-sp-back-no-movie-rearm | Back to main menu does not destroy-and-recreate the movie | drift-other | MEDIUM | common |

- **quit-confirm:** Button 0x3EE returns code 6 → case 6 pops CSF message-box (template 0x120,
  `FUN_005D3490`) with GUI:ExitAreYouSure / TXT_OK / GUI:Cancel; OK → case-7 shutdown
  (`OptionsClass__WriteToINI`, fade, vox pump ≤3s). Rust (app.rs:1632) calls `event_loop.exit()`
  directly — no confirm, no persistence, no shutdown sequence. Evidence:
  QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md.
- **options-case5:** Control 0x55C → case 5 → `OptionsClass__ShowLauncherDialog (0x0055FC80)`,
  dialog 0xD5 (difficulty/speed/scroll/volume sliders, health-bars/action-lines checkboxes,
  resolution combo, 0xB8-byte cancel-restore, INI write). Rust (app.rs:1639-1645) logs
  "not implemented." Evidence: OPTIONS_DIALOG_CASE5_AND_FIELD_MAP_GHIDRA_REPORT.md.
- **movies-credits-case4:** Control 0x686 → case 4 → dialog 0x101 (Sneak Preview→RENEGADE.BIK,
  Movies→picker dialog 0x129 from [Movies] in artmd.ini, Credits→CREDITSMD.TXT + INTRO music).
  Rust logs "not implemented." Evidence: MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md.
- **sp-newcampaign:** Control 0x688 → case 8 → campaign selector dialog 0x94 (Allied/Soviet icons
  0x6EA/0x6EC, difficulty slider 0x50F). Rust logs "not implemented." Evidence:
  SINGLE_PLAYER_SUBMENU_DIALOG_CASE1_GHIDRA_REPORT.md.
- **sp-submenu-movie-no-restart:** `Main_Game` cases 1 & 4 call `FUN_0060D380(1)` →
  `FUN_0052b9b0`: GetDlgItem(0x71A), SetWindowPos to centered origin, SendMessage 0x4e3 (restart)
  + 0x4e4 (Ra2ts_l/s name). Rust (app_single_player_shell_render.rs:305-312) only steps the shared
  movie by elapsed time. Verified via `decompile_function 0x0060D380`, `0x0052b9b0`.
- **sp-back-no-movie-rearm:** Back (0x12) re-calls `FUN_00531CC0` which fully reconstructs dialog
  0xE2; message 0x4E4 destroys+recreates the movie handle (restart from frame 0). Rust
  `close_single_player_shell` (app.rs:549) just toggles a bool, resuming mid-stream. *(Verifier
  correction: 0x4E4 is a full destroy-and-recreate, not a reposition.)*

---

## 4. Skirmish Shell

### 4.1 Layout (`ss-layout`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-layout-validation-modal-missing-plus-one-centering | Validation modal centering omits the +1 term of CenterChildWindow | drift-constant | MEDIUM | common |
| ss-layout-validation-modal-double-center-highres | Modal double-centers (800-box + screen offset) instead of in live screen | drift-layout | MEDIUM | common |
| ss-layout-validation-modal-size-451x326-invented | Modal size 451×326 not binary-proven (candidate 450×325) | drift-constant | MEDIUM | common |

- **gamemd:** `CenterChildWindow @0x00777080` = `X = max(0, ((g_ScreenWidth - child_w)+1)/2)`,
  centering directly in the live screen. At 800×600 with a 451-wide modal: X = (800−451+1)/2 = 175.
- **Rust:** `centered_shell_dialog` (layout.rs:427-434) = `center_offset(screen,800) +
  (800−w)/2`, no +1. At 800×600: 0 + (800−451)/2 = **174** (1px off). At 1024×768: 112+174=286 vs
  287 — and the double-nesting diverges further at any resolution where (screen−800) is odd.
- **size 451×326:** RT_DIALOG 0xCE is 300×200 DLU; `mul_div_round` → 450×325 (the doc's
  Medium/unverified candidate). The +1 in each dimension (451/326) is not derivable from any
  binary-proven formula — invented. Wrong size shifts every centered child and modal edge.
  Evidence: `decompile_function 0x00777080`;
  SKIRMISH_VALIDATION_MODAL_NATIVE_PIXEL_RECTS_GHIDRA_REPORT.md §4/§5/§8.

### 4.2 Combos (`ss-combos`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-combos-open-sound-only-on-arrow-zone | Open sound only on rightmost-20px arrow, not whole collapsed face | drift-timing | MEDIUM | common |
| ss-combos-face-click-does-not-open | Non-arrow face clicks: missing click sound (consumed differently) | drift-other | MEDIUM | common |
| ss-combos-switch-combo-plays-close-and-open-and-opens-second | Clicking a 2nd combo while one is open plays Close+Open and opens it | drift-timing | MEDIUM | common |
| ss-combos-mouse-wheel-scroll-unverified | Rust adds wheel-scroll of open dropdown; no gamemd WM_MOUSEWHEEL handler | unchecked→drift | MEDIUM | common |
| ss-combos-ai-type-default-selection-easy-vs-none | Row-1 AI defaults to Easy; gamemd resets every AI combo to None | drift-other | HIGH | every-match |

- **open-sound:** `OwnerDraw_ComboBox_00617250` plays `GUIComboOpenSound` **unconditionally first**
  on WM_LBUTTONDOWN, *then* tests `client_width-0x14 < mouse_x` to toggle. Rust gates BOTH sound
  and open on `combo_arrow_at` (rightmost 20px only, combos.rs:703-709) — a face/text click is
  silent. Verified via `decompile_function 0x00617250`.
- **face-click:** gamemd's `return 0` consumes the message unconditionally and the sound fired
  first. Rust returns false on non-arrow clicks. *(Verifier correction: the click falls through to
  checkbox/trackbar hit-testing inside `handle_option_mouse_down`, NOT player-name edit — but
  those regions don't overlap, so no mis-activation; the real drift is the missing click sound.)*
- **switch-combo:** When a dropdown is open it holds `SetCapture`; a click on another combo's
  arrow is routed to the captured `ComboDropWin`, which plays only `GUIComboCloseSound` and closes
  — the second combo does **not** open in the same click. Rust (combos.rs:683-695) plays
  Close+Open and immediately opens the second. Verified via `decompile_function 0x00617250` and
  ComboDropWin WndProc at 0x0060E4A0.
- **mouse-wheel:** No WM_MOUSEWHEEL (0x20A) handler exists in `OwnerDraw_ComboBox_00617250`,
  ComboDropWin, or `OwnerDraw_ScrollBar_0061C690` (verified by decompile + byte-pattern search for
  `0a 02 00 00` → zero matches). Rust (trackbars.rs `handle_option_mouse_wheel`) explicitly
  scrolls the open dropdown — an added feature.
- **ai-type-default:** `FUN_006AE6E0` sends `CB_SETCURSEL` index 0 (GUI:None, -1) to every AI row,
  then applies persisted slot state; a fresh profile shows all 7 rows as None (disabled siblings).
  Rust (state.rs:263-264) hardcodes opponent 0 = Easy/enabled. On first launch gamemd shows an
  empty lobby; Rust shows one active Easy opponent. Verified via `decompile_function 0x006AE6E0`,
  `0x00697F10`, `0x00477440`.

### 4.3 Trackbars (`ss-trackbars`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-trackbars-rail-click-starts-drag | Rail click wrongly begins a continuous drag | drift-other | MEDIUM | common |
| ss-trackbars-hardcoded-ranges-bypass-ini | Credits/unit-count ranges/step/defaults hardcoded, not from rules INI | drift-constant | LOW | rare |
| ss-trackbars-doubleclick-rail-not-handled | DBLCLK on thumb: gamemd releases capture (no drag); Rust re-enters drag | drift-other | LOW | rare |

- **rail-click-drag:** In `OwnerDraw_Trackbar_0061D950`, a mouse-down outside the thumb remaps
  once and does NOT set the drag flag `iStack_124`; WM_MOUSEMOVE remaps only when `iStack_124 != 0`
  (i.e. only a thumb-grab drags). Rust (trackbars.rs:196-205,228-238) sets
  `trackbar_drag=Some{dragging_thumb:false}` on a rail click and `handle_option_mouse_move` remaps
  on every move whenever `trackbar_drag.is_some()`, ignoring `dragging_thumb`. Verified via
  `decompile_function 0x0061D950`.
- **hardcoded-ranges:** `FUN_006AE6E0` seeds ranges/step/initial from RulesClass MinMoney(+0x1480)/
  MaxMoney(+0x1488)/MoneyIncrement(+0x148C)/MinUnitCount(+0x1490)/MaxUnitCount(+0x1498)/GameSpeed
  — i.e. `[MultiplayerDialogSettings]`. Rust hardcodes 5000/10000/100, 0/10, 0..6 (trackbars.rs:
  17-25; game_options.rs:72-75). Equals stock YR values, so stock output is bit-identical, but
  modded rules would not change the sliders.
- **doubleclick:** *(Verifier correction.)* gamemd's WM_LBUTTONDBLCLK at LAB_0061e4ce calls
  `ReleaseCapture` and does NOT set `iStack_124` — no drag starts, mouse-move after dblclk does not
  move the thumb. Rust (winit delivers a second Pressed) re-enters drag mode, so post-dblclk
  mouse-move DOES move the thumb. The finding's "observably indistinguishable" claim was wrong.

### 4.4 Player Name / Map List (`ss-playername-map`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-playername-map-default-name-hardcoded | Default player name is literal "Player", not the persisted profile global | drift-other | HIGH | every-match |
| ss-playername-map-mainmenu-maplist-sorted | Map list sorted alphabetically; gamemd never sorts the scenario list | drift-ordering | HIGH | common |
| ss-playername-map-mode-change-selection-reset-index0 | Mode switch resets map highlight to row 0; gamemd preserves by saved text key | drift-other | LOW | common |
| ss-playername-map-insert-control-char-filter | Insert drops all is_control() chars; native edit accepts DEL etc. | drift-other | LOW | rare |
| ss-playername-map-tab-blurs-no-next-control | Tab blurs the field but does not advance focus to the next tab control | drift-other | LOW | rare |
| ss-playername-map-refresh-records-clamp-visible-rows-1 | Map list does not scroll the selected row into view after repopulation | drift-other | LOW | rare |
| ss-playername-map-move-left-right-collapse-selection | Arrow Left/Right collapse to moved caret, not to selection edge | drift-other | LOW | rare |

- **default-name:** Setup `FUN_006AE6E0` seeds edit 0x6A0 from the persistent global
  `DAT_00A8B380` (last-used / profile name) via FUN_00735120 + message 0x4B2; Start `FUN_006ACEE0`
  reads it back. Rust `PlayerNameEditState::default()` always uses the literal "Player"
  (player_name.rs:16,32). Edit/caret/cap/readback are implemented; only the **seed value** is
  wrong. Evidence: SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md §3.1.
- **maplist-sorted:** `list_available_maps()` (app_list_maps.rs:49) sorts by lowercased display
  name. gamemd builds the scenario list in append order (MISSIONSMD.PKT MultiMaps first, then
  loose PKT/YRO/YRM in filesystem-enumeration order) and **never sorts** (verified at
  0x005E6F17..0x005E6F45; append helper 0x005EEE40). *(Verifier correction: this is the skirmish
  selector itself — `list_available_maps` feeds `available_maps` at app.rs:2016 — not a separate
  main-menu surface.)* Evidence: SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md §5.
- **mode-change-reset:** `select_mode` calls `refresh_records(..., None)` → falls to row 0
  (choose_map.rs:59-72,169-175). gamemd's 0x6EB category-change preserves the row by saved text key
  (DAT_00AC0EC8) via LB_SETCURSEL/LB_SETTOPINDEX. Evidence:
  SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md §3.5.
- **insert-control-char:** *(Verifier correction.)* gamemd's edit (`OwnerDraw_Edit_00614190`)
  forwards WM_CHAR to the native WndProc via `CallWindowProcA` at LAB_00614872; the realistic
  divergence is **DEL (0x7F)** only — Rust's `is_control()` drops it, native passes it through.
  The Unicode-control-range claim is overstated (ANSI WM_CHAR path). Paste-collapse claim is
  unverified.
- **tab:** gamemd's WM_CHAR Tab calls GetParent + GetNextDlgTabItem + SetFocus (focus-advance);
  Rust `handle_player_name_tab` (player_name.rs:445-447) only blurs.
- **refresh-records-clamp:** *(Verifier correction.)* The `visible_rows=1` clamp is NOT the bug —
  `map_top_index` is always 0 when `refresh_records` runs. The real drift: gamemd passes the
  **selected-row index** to LB_SETTOPINDEX (binary read 0x005E6C80..0x005E6DB3) so Windows clamps
  to show the selection; Rust passes the retained 0, leaving the selected map possibly off-screen.
- **move-left-right:** gamemd forwards arrows to the native single-line edit (collapse-to-edge);
  Rust (player_name.rs:173-185) clears selection then steps caret by 1, so after select-all,
  Left → len-1 (wrong; native = 0).

### 4.5 Hit-Test (`ss-hittest`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-hittest-modal-blocks-parent-but-gamemd-uses-separate-dialog | Hover suppressed under both modals; validation modal leaves parent visible | drift-other | MEDIUM | common |

- **Verifier split:** For **Choose Map** (0x5AA), gamemd hides setup 0x102 (`ShowWindow(setup,0)`
  at 0x006AD93C), so Rust's blanket `None` (hit_test.rs:48-50) is **equivalent**. For the
  **validation modal** (0xCE), gamemd does NOT hide the parent — only the Start button (0x617) is
  `EnableWindow(FALSE)`; the ~450×325 modal leaves most of the 800×600 parent visible, and Win32
  delivers WM_MOUSEMOVE to parent controls in the uncovered region (status text via
  `FUN_00622B50`, 0x00622CCB..0x00622E83). Rust suppresses hover for the whole screen during
  validation — a real output drift (hovering visible trackbars/combos shows no status text). The
  finding's "behaviorally equivalent" label was correct only for Choose Map.

### 4.6 Start → Loading Handoff (`ss-launch`)

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ss-launch-random-country-rejected | Random country/color hard-errors instead of native RNG resolution | missing | HIGH | common |
| ss-launch-mmpb-markers-missing | First loading renderer omits the mmpb.shp assigned-player marker pass | missing | HIGH | every-match |
| ss-launch-bar-colors-ramp-approximation | Bar fill/backing use ramp shades, not ColorScheme +0x308/+0x30C | drift-constant | HIGH | every-match |
| ss-launch-start-packing-incomplete | Start handoff skips AI arrays, color randomization, launch table, reset flags | missing | HIGH | common |
| ss-launch-generic-fallback-egui-text | Generic-map-load path renders egui "Mission deployment"/map-name text | drift-other | HIGH | common |
| ss-launch-progress-origin-omits-loadmgr-point | Progress origin omits the upstream LoadProgressMgr+0x1C point | drift-layout | LOW | non-standard-res |
| ss-launch-row-height-fonth-standin | Row height uses bar height as a font-height stand-in | drift-layout | LOW | every-match |
| ss-launch-selected-map-from-ui-index | Launch resolves the file from the UI index, not the persisted accepted-record buffer | drift-other | LOW | rare |

- **random-country:** `launch_country_from_menu` (launch.rs:14-34) returns
  `RandomSelectionUnverified` when random is set, so Start returns Err. gamemd
  `SessionClass__ProcessRandomAssignments (0x0069B8C0)` resolves country via
  `Random__RandomRanged(0,9)` and color via `(0,7)` with uniqueness checks (FUN_0069B7E0 for
  color -2) and launches normally. Verified via `decompile_function 0x0069B8C0`.
- **mmpb-markers (loading):** First renderer `0x00552D60` calls `FUN_00640A40` at 0x00553687,
  drawing mmpb.shp frame 0 at each assigned start (offsets −3X/−2Y, color from house scheme
  +0x30C). Rust `build_native_loading_instances` (app_loading.rs:776-837) builds only background/
  fill/bar/icon. Fires for the human player's marker every match. (Shares root cause with
  `ls-progress-mmpb-marker-missing` / `ls-composition-text-no-mmpb-marker-overlay`.)
- **bar-colors:** `FUN_00643400` fills the backing from ColorScheme color and draws the bar span
  via the scheme convert. Rust uses ramp shade 0 (fill) / 11 (backing) as approximations
  (app_loading.rs:31-32,752-771, self-flagged). *(Note: the exact gamemd backing mechanism is
  further corrected under `ls-progress-backing-shade-approximated` — it is an HSV transform of a
  static-address / player-data value, not ColorScheme+0x308. Either way, ramp shades drift.)*
- **start-packing:** `FUN_006ACEE0` packs five per-AI arrays, the 8-row launch table
  (DAT_00A8B3F0, item-data→type-code mapping), a 0x85-byte node record, runs ProcessRandomAssignments,
  mirrors checkboxes/trackbars, and forces reset flags. Rust `launch_session` (launch.rs:78-162)
  builds a `SkirmishLaunchSession` and skips the launch-table mapping, color-uniqueness, node-record
  layout, and reset flags. *(Verifier: the two player-visible gaps are random-country and
  random-color blocking; the array/table/flag differences are internal-mechanism, observability
  depends on downstream spawn.)*
- **generic-fallback-text:** The main-menu `StartSelected` path uses `generic_map_load` →
  `draw_loading_screen` egui text ("Mission deployment"/"Loading..."/`Map: {name}`). Standard
  Skirmish loading (g_GameMode==5) reads no LSLoadMessage/Briefing/UIName and shows no map name.
  Evidence: LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md.
- **progress-origin:** *(Verifier correction.)* `FUN_00552B10` writes LoadProgressMgr+0x1C/+0x20
  = `(g_ScreenWidth - refWidth)/2`; for 640/800 (both standard YR resolutions) this is **(0,0)**, so
  Rust's hardcoded (12,256)/(16,321) is pixel-exact. Drift only at non-standard resolutions
  (1024+). Trigger downgraded from every-match.
- **row-height-fonth-standin:** gamemd `row_h = max(side_icon_h, H+6, font_h)+4` with `font_h =
  GAME.FNT height` (`*(g_GAME_FNT+0x1c)`). Rust substitutes `font_h = bar_height`
  (app_loading.rs:711-723); diverges only if real font > H+6.
- **selected-map-from-ui-index:** gamemd's post-shell file arg is `ScenarioClass+0x125C`, set at
  Choose-Map-accept time by `0x005E7BF0` from the accepted record's +0x58 path (Start only mirrors
  the index, clamp-to-0 on out-of-range). Rust re-resolves `maps[selected_map_idx].file_name` at
  launch (launch.rs:83-85; app.rs:609-612), with no clamp-to-0 (aborts instead).

---

## 5. Loading Screen

> **Color-pipeline cluster (merged root cause):** `ls-progress-bar-fill-tint-not-remap`,
> `ls-composition-text-bar-fill-tint-not-shp-remap`, `ls-progress-backing-shade-approximated`,
> `ls-composition-text-backing-fill-color-formula`, and `ls-composition-text-fill-color-shade0-vs-bar-shade`
> are five views of the same problem: Rust uses flat ramp-shade tints over the baked PROGBARM
> frame, while gamemd draws a solid scheme-derived backing fill + the PROGBARM SHP. The verifier
> corrected the *mechanism* description on several of these (see below) — none of the corrections
> downgrade the drift.

> **mmpb cluster (merged):** `ss-launch-mmpb-markers-missing`, `ls-progress-mmpb-marker-missing`,
> and `ls-composition-text-no-mmpb-marker-overlay` are the **same missing layer** reported under
> three dimensions. One fix closes all three.

### 5.1 Progress / Composition

| id | title | cat | sev | freq |
|---|---|---|---|---|
| ls-progress-bar-fill-tint-not-remap | Bar fill is a flat tint over PROGBARM frame0; gamemd draws raw SHP (NOT remap) | drift-other | HIGH | every-match |
| ls-progress-backing-shade-approximated | Empty-bar backing uses ramp shade 11, not the HSV-from-scheme color | drift-constant | HIGH | every-match |
| ls-progress-mmpb-marker-missing | Assigned-player mmpb.shp marker overlay not drawn | missing | HIGH | every-match |
| ls-composition-text-no-mmpb-marker-overlay | (duplicate of above under composition-text dimension) | missing | HIGH | every-match |
| ls-composition-text-no-progress-row-text-label | Missing bitfont node/player text label on the progress row | missing | HIGH | every-match |
| ls-composition-text-backing-fill-color-formula | Backing fill uses ramp shade 11 instead of HSV-derived scheme color | drift-constant | HIGH | every-match |
| ls-composition-text-fill-color-shade0-vs-bar-shade | Bar fill uses ramp shade 0 instead of scheme convert (+0x30C) | drift-constant | HIGH | every-match |
| ls-composition-text-bar-fill-tint-not-shp-remap | Bar fill is tinted quad, not color-remapped/raw SHP | drift-asset | MEDIUM | every-match |
| ls-progress-theater-ramp-not-emitted-live | Dynamic theater ramp 13..24 (and 25) not emitted; live loader jumps 12→30 | unchecked→drift | HIGH | every-match |
| ls-progress-rowheight-fontheight-standin | Row height / centering uses bar height as font-height stand-in | drift-layout | LOW | every-match |
| ls-progress-side-icon-vertical-center-basis | Side-icon Y uses (row_h-icon_h)/2 instead of font-band centering | drift-layout | LOW | every-match |
| ls-composition-text-ftol-fractional-rounding | ftol adds epsilon round-to-nearest; gamemd Math__ftol always truncates | drift-rng | LOW | common |
| ls-composition-text-render-clears-no-surface-seed | Per-frame render clears to CLEAR_COLOR; benign at native res | drift-other | LOW | non-native-res |

- **bar-fill-tint (verifier-corrected mechanism):** `CC_Draw_Shape` in `FUN_00643400` uses flag
  `0x400` (documented "Unused/reserved" per ANIM_CLASS draw-flag table — NOT the `0x800` remap
  bit) with **null convert args**. So gamemd draws PROGBARM.SHP frame 0 **raw** (no tint, no
  remap), with a solid `ColorScheme`-derived rectangle behind it (surface vtable +0x58). Rust
  multiplies the baked frame by a flat ramp tint (app_loading.rs:769-822). The original finding's
  "remap" claim was wrong; the drift (flat tint vs raw SHP pixels) is still real.
- **backing-shade (verifier-corrected mechanism):** The backing color does NOT come from
  ColorScheme+0x308 directly. Per disassembly of `FUN_00643400`/`FUN_00643720`/`FUN_00643ae0`/
  `FUN_00642c80`: ProgressClass+0x50 is null in skirmish, defaulting to the static address
  `&DAT_00887734`, whose pointer value is read by `FUN_00517440` as 3 HSV bytes
  (H=0x34,S=0x87,V=0x88). It is a fixed color from a static address, unrelated to the house ramp.
  *(Note: `ls-composition-text-backing-fill-color-formula` separately read it as
  `LEA ECX,[EAX+0x308]` → an HSV transform of ColorScheme+0x308. The two reads disagree; both
  agree it is an HSV transform, not a ramp index, and Rust's ramp shade 11 drifts from either. The
  exact source is worth a single follow-up confirmation.)*
- **no-progress-row-text-label:** In the single-lane Skirmish branch (`+0x61==1`), `FUN_00643AE0`
  substitutes `*DAT_00a8da78` for the null +0x50 text pointer and `FUN_00643720` calls
  `FUN_00643670`, which builds a 15-char string, measures via `BitFont__GetTextWidth`, and draws
  via `FUN_004A61C0`. With `+0x70=1`, a BitFont label renders to the right of the bar. Rust draws
  none (app_loading.rs:824-825). Verified via `decompile_function 0x00643AE0`, `0x00643670`,
  `0x00643720`.
- **theater-ramp (verifier-corrected, now HIGH):** `Init_Theater (0x005349C0)` emits 8, (6,) 12,
  then a dynamic palette ramp `min(i/(DAT_00B054E0/13)+0x0C, 0x19)` advancing across 13..25, then
  a final 25 — visible on every cold-theater start (always true first load). Rust's live loader
  (app_init.rs:395-396) emits `milestone(12)` then `milestone(30)`, skipping 13..24 **and** 25;
  `theater_ramp_changed_values` (app_loading.rs:521-530) is an unwired helper. ~12 missing visible
  progress advances per cold start. Verified via asm 0x00534BE2.., 0x00534D84.., 0x00534DB9...
- **ftol (verifier-corrected mechanism):** `Math__ftol @0x007C5F00` loads CW `0x0E7F` (RC=11 =
  truncate toward zero) before FISTP — it always truncates, never round-to-nearest. Rust adds an
  epsilon round-to-nearest within 1e-6 of an integer (app_loading.rs:987-996) that gamemd never
  does. Narrow 1px drift only when the product lands within 1e-6 of an integer.

---

## 6. Needs Research (UNCHECKED)

| id | title | sev | missing evidence |
|---|---|---|---|
| ss-launch-single-player-skirmish-no-transition-helper | SP→Skirmish route correctly skips FUN_006071E0 (claimed no-disparity) | MEDIUM | live-debugger trace |

- The finding claimed this route is proven to have no transition. The result-write proc
  `0x0052D640` indeed never calls `FUN_00608260`/`FUN_006071E0`. **But** the `0x00612690 →
  FUN_00608260` slide path lives in the generic shell child subclass dispatcher
  (`0x00610CA0..0x006128FE`) installed on **all** shell child windows (including dialog 0x100
  buttons, style 0x5000000B) by `FUN_0060F9A0`, and fires during WM_PAINT when `record+0x1FC==1`
  — a paint-phase trigger orthogonal to the WM_COMMAND result write. Whether a retail `0x579`
  click activates that paint-phase state is open (research OQ-09, status: needs-runtime-debugger).
  Rust currently nulls the transition on this route (app.rs:556-564), which matches *if* gamemd
  also shows no slide — unproven. **Missing:** a live-debugger trace of `FUN_00608260` while
  clicking 0x579 in retail. ShellButtonSlideSound being empty removes only the audio cue, not the
  visual.

---

## 7. False Positives Checked (49)

Listed so the user knows they were verified, not skipped. Each was refuted by algebraic proof,
exhaustive caller verification, dead-code analysis, or a corrected gamemd reading.

**Main Menu (18):**
- `mm-composition-no-button-click-fade-effect` — ButtonFadeEffect containers have no code xrefs; FUN_006071e0 gated on +0xC1 (never set on 0xE2). Rust matches the instant snap.
- `mm-composition-website-button-0x55f-not-rendered` — 0x55F is not a child of dialog 0xE2's RT_DIALOG; Rust correctly renders 6 buttons.
- `mm-composition-responsive-layout-stretch-drift` — `compute_responsive_layout` has zero production callers (tests only); production uses `compute_layout`.
- `mm-composition-version-color-yellow-vs-cyan-open` — `FUN_00621040` byte order `0x00BBGGRR`; 0x0000FFFF → yellow #FFFF00 in both RGB565/555. Rust bit-identical.
- `mm-buttons-fallback-shell2-palette` — SDBTNANM.PAL always present in retail; fallback is dead code (gamemd would crash, not substitute).
- `mm-buttons-no-disabled-state-path` — dark-red/overlay disabled logic is the PCX path; SDBTNANM path (used by 0xE2) draws no disabled variant. Both engines match.
- `mm-buttons-pressed-text-y-uses-art-sink-not-rect-shrink` — algebraic proof: shrink-rect-then-center ≡ translate-rect-then-center for all single-line slack values. Bit-identical.
- `mm-transition-frame-count-14-vs-N-plus-8` — slide system not active on main-menu clicks; no gamemd animation to be off-by-frames.
- `mm-transition-frame-staggering-absent` — same: no per-cell wave on main-menu clicks (instant swap).
- `mm-transition-no-slide-sound-correct-but-verify-empty-key` — dialog 0xE2 calls FUN_006071E0 only with DL=0 (close dir), bypassing the sound; INI empty too. Both silent.
- `mm-transition-linear-sampler-resamples-pixel-art` — gamemd does no positional slide/blit; no nearest-vs-linear referent exists. (Compositor is acknowledged placeholder.)
- `mm-hover-tooltip-phantom-hover-frame-swap` — refuted as FP but the *opposite* is true: see confirmed `mm-composition-button-art-sdbtnanm-frames-correct` correction (frame-3 IS wrong). [Listed here per original audit; the hover flash IS drift.]
- `mm-hover-tooltip-status-line-update-not-continuous` — Rust repaints every GPU frame; same as gamemd's per-WM_NCHITTEST invalidation. No skip.
- `mm-hover-tooltip-missing-blank-clear-of-status-line` — full-frame GPU clear ≡ per-control InvalidateRect+WM_PAINT; identical blank output.
- `mm-audio-eva-track-extension-wav-only-in-gamemd` — Rust mix_hash uppercases identically; "drok.wav" and "DROK.WAV" hash to the same MIX entry. .aud fallback never fires.
- `mm-audio-eva-no-welcome-eva-correct` — verified-negative; no EVA on menu entry either side.
- `mm-dialogs-doc-eva-welcomeback-wrong` — FUN_0052B9B0 has no audio call (movie reposition only); doc label wrong but no Rust-vs-gamemd output diff.

**Skirmish Shell (24):**
- `ss-layout-choosemap-listbox-uses-combo-scrollbar-constants` — both paths delegate to the same `OwnerDraw_ScrollBar_0061C690`; 22/14 verified by binary identity.
- `ss-layout-combo-track-click-pages-instead-of-absolute-jump` — current code does absolute thumb-centered jump; doc §4 note was stale.
- `ss-combos-reopen-same-combo-missing-open-sound-semantics` — captured ComboDropWin plays one Close, closes; Rust output identical.
- `ss-combos-arrow-hit-rect-uses-reserve-width-not-19px-from-right` — `client_width-0x14` ≡ rightmost-20px reserve. Bit-identical.
- `ss-combos-track-click-absolute-jump-correct` — matches corrected gamemd.
- `ss-combos-thumb-min-height-and-arrow-button-constants-correct` — 20/22/14/23 all verified.
- `ss-combos-color-sentinel-row8-correctly-omitted` — sentinel -2 + colors 0..7, row 8 omitted, matches FUN_004E45A0.
- `ss-combos-ai-type-order-and-itemdata-correct` — None/Easy/Normal/Hard with data -1/2/1/0, verified.
- `ss-combos-side-combo-no-observer-row-correct` — Random + countries 0..9, observer gated; matches standard offline.
- `ss-combos-side-combo-cap-7-scrollbar` — cap 7, 11 items, scrollbar both sides.
- `ss-combos-team-cap-uncapped-vs-9` — Team items fixed at {4,5}; min(x,9)=x. Identical.
- `ss-combos-aitype-cap-uncapped-vs-none-correct` — 4 rows uncapped, no 0x4DE cap in gamemd.
- `ss-combos-start-position-list-random-plus-numbered` — Auto first + ownership-filtered numbered entries; matches FUN_004E50C0.
- `ss-combos-team-none-row-gated-by-must-ally` — vtable +0x2C / MustAlly +0x3F semantics reproduced incl. AlliesAllowed boundary.
- `ss-trackbars-hscroll-highword-no-mask` — `&0xffff` is identity for all clamped values ≤10000. Bit-identical wParam.
- `ss-playername-map-focus-scroll-reset` — scroll correction runs before request_redraw; no inconsistent frame; caret-margin logic matches.
- `ss-playername-map-empty-select-all-no-selection` — Some((0,0)) vs None indistinguishable via normalized_selection() to all consumers.
- `ss-playername-map-choose-map-row-height-16` — Rust constant is 19, never 16; matches gamemd font_height+2.
- `ss-playername-map-default-selected-idx-source` — `current_choose_map_record_index` bridges UI index → record by file_name; equivalent to gamemd pointer scan.
- `ss-hittest-zorder-model-vs-childwindowfrompointex` — all controls geometrically disjoint at 800/1024; priority list == z-order result.
- `ss-hittest-static-controls-tested-last-assumes-nonoverlap` — exhaustive rect audit: flags in a 77px gap between disjoint x-bands; structurally non-overlapping at all res.
- `ss-hittest-choosemap-modal-status-precedence-order` — verified rects disjoint; fixed order == ChildWindowFromPointEx for every cursor point.
- `ss-hittest-disabled-combos-still-hovered-matches-enablewindow` — disabled-but-visible combos still hover (CWP flag=1 = SKIPINVISIBLE only). Matches.
- `ss-hittest-side-color-item-status-now-resolved` — current hit_test.rs:161-168 dispatches item-specific STT for Side/Color; doc delta was stale.
- `ss-hittest-start-team-open-rows-no-item-specific` — Start/Team fall to generic combo help on both sides; algebraic match.

**Loading Screen (7):**
- `ss-launch-side-icon-content-unverified` — `FUN_004e3560` is an exact country→PCX switch; every Rust filename matches.
- `ss-launch-ftol-fractional-x87` — Math__ftol always truncates (CW 0x0E7F); Rust fallback truncation matches mid-fill. (Narrow epsilon drift tracked under `ls-composition-text-ftol-fractional-rounding`.)
- `ss-launch-progress-message-0x11ae-absent` — 0x11AE handler `FUN_00554400` is gated on LoadProgressMgr+0x60, never set in Skirmish; the call is a no-op.
- `ls-progress-ftol-fractional-truncates` — same as above; gamemd truncates, Rust truncates mid-fill. Match (finding's gamemd claim was inverted).
- `ls-composition-text-background-not-centered` — gamemd centering offset = 0 at both supported resolutions (640/800); Rust (0,0) is bit-identical.
- `ls-composition-text-vertical-centering-uses-bar-h-not-font-h` — with H=14, H+6=20 dominates font_h=17 in both formulas; no divergence for the real asset.
- `ls-composition-text-side-icon-presence-mismatch` — binary directly confirms filenames, x=base+bar_w+0x15, and centering all match Rust.

---

## 8. Prioritized Fix Order

**Tier 1 — fix first (every-match, high player-visibility, isolated point fixes):**
1. `mm-composition-missing-mnscrnl-background` — load + draw MNSCRNL/MNSCRNS behind the shell.
2. `ls-progress-mmpb-marker-missing` cluster — add the mmpb assigned-player marker layer (one fix closes 3 reported findings + `ss-launch-mmpb-markers-missing`).
3. `ls-composition-text-no-progress-row-text-label` — add the BitFont node-name label.
4. `mm-hover-tooltip-no-software-shp-cursor` — wire the MOUSE.SHA software cursor into the shell layer.
5. `mm-buttons-fabricated-hover-flash` + `mm-hover-tooltip-phantom-hover-timer-state` + `mm-buttons-stale-state-comments` + `mm-composition-button-art-sdbtnanm-frames-correct` — remove the phantom 1 Hz frame-3 hover flash and fix the comments (one change, four findings).

**Tier 2 — high-value but larger or multi-finding:**
6. `mm-composition-button-art-rightanchored-scaled-not-centered` + `mm-composition-button-rect-snapped-to-168x42-tile` + `mm-composition-button-x-632-vs-638` — rework button geometry to the 162×37 client rect and the PCX-piece asset family (the audit found the wrong asset family is drawn; verify PCX vs SDBTNANM intent before implementing).
7. Loading-screen color cluster (`ls-progress-bar-fill-tint-not-remap`, `ls-progress-backing-shade-approximated`, `ls-composition-text-*-color-*`) — implement the raw-SHP draw + scheme-derived solid backing. **First resolve the backing-color source disagreement** (static-address HSV vs ColorScheme+0x308) with one Ghidra confirmation.
8. `ls-progress-theater-ramp-not-emitted-live` — emit the 13..25 theater palette ramp.
9. `ss-launch-random-country-rejected` + `ss-launch-start-packing-incomplete` — implement `ProcessRandomAssignments` (RandomRanged 0,9 / 0,7 + uniqueness) so Random country/color launches.
10. `ss-combos-ai-type-default-selection-easy-vs-none` — reset AI rows to None on a fresh profile.
11. `ss-playername-map-default-name-hardcoded` + `ss-playername-map-mainmenu-maplist-sorted` — seed the player name from a persistent global; remove the alphabetical map sort.

**Tier 3 — ARCHITECTURE rework (not point fixes):**
12. **Remove the main-menu→Skirmish `ShellBridgeTransition`** for this path (instant dialog swap). This retires all 7 `mm-transition-*` confirmed drifts plus the input-block window. Re-architect only if a native slide is later wanted on a dialog that actually sets `+0xC1`.
13. **Build the options-persistence layer** (`RA2MD.INI [Options]` read/write): closes `mm-audio-eva-music-volume-default`, `...not-read-from-options`, and is a prerequisite for the missing Options dialog (`mm-dialogs-options-case5`) and the quit-time options save (`mm-dialogs-quit-confirm`).
14. **Implement the four missing menu dialogs** (`mm-dialogs-quit-confirm`, `...options-case5`, `...movies-credits-case4`, `...sp-newcampaign`) — substantial but each is a self-contained dialog port.

**Lower priority (LOW severity / rare / non-standard-resolution):** the validation-modal pixel/centering trio, trackbar rail-click drag and dblclk, player-name edit edge cases, combo wheel-scroll and face-click sound, loading-screen row-height/side-icon/ftol precision, progress-origin at non-standard resolutions. Fix opportunistically alongside the surrounding system.

**Resolve before implementing:** `ss-launch-single-player-skirmish-no-transition-helper` (needs a live-debugger trace, OQ-09) — do not assume the SP→Skirmish route is animation-free until confirmed.
