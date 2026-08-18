# Main Menu Visual Assets / RA2TS Movie Panel - Ghidra Report

Date: 2026-05-17

Parent report: `C:/Users/enok/Documents/ra2-rust-game-docs/MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`

Scope: targeted follow-up on the standard Yuri's Revenge initial main menu
visual assets: the `Ra2ts_s/l` left movie panel, Bink vs VQA selection, shell
button PCX names, layout consequences, and current Rust asset support. This
does not re-investigate the full shell menu routing already covered by the
parent report.

**Address(es):** `0x00531CC0`, `0x0052B9B0`, `0x006153E0`, `0x005C0640`,
`0x005C07D0`, `0x004326C0`, `0x00432750`, `0x00612B70`
**Confidence:** High for Bink/VQA resolution and button filename generation.
Follow-up `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
resolved the duplicate `LANGUAGE.MIX` / `LANGMD.MIX` priority question for
RA2TS assets.
**Active in YR:** Yes. The path is the standard shell state `0x12` initial main
menu reached by `Main_Game`.

## 1. Overview

The initial main menu's left visual panel is not a `GraphicMenu` background and
not an in-game sidebar. It is dialog child static `0x71A`, subclassed to
`OwnerDraw_Static_006153E0`, then commanded with custom messages `0x4E3` and
`0x4E4` to loop a movie base name.

The important correction from the first report is that "VQ movie" is only the
legacy naming of the generic handle path. The constructor resolves the provided
base name by trying `.BIK` first and `.VQA` second. If the resolved filename has
extension `.bik`, the handle becomes a Bink-backed movie handle. Retail asset
probing confirms `ra2ts_s.bik` and `ra2ts_l.bik` exist and decode successfully.

## 2. Verified Movie Selection

Main menu creation and refresh both send the same sequence to child `0x71A`:

```text
SendMessage(0x71A, 0x4E3, 1, 0)
SendMessage(0x71A, 0x4E4, 0, screen_width == 640 ? "Ra2ts_s" : "Ra2ts_l")
```

Findings:

| What | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| 640-wide mode uses base name `Ra2ts_s`; other widths use `Ra2ts_l`. | `FUN_00531CC0`, `FUN_0052B9B0`; strings at `0x00825CE8` and `0x00825CE0`. | High | Yes |
| Main menu sets loop flag before assigning the movie. | `SendMessage` order in `FUN_00531CC0` and `FUN_0052B9B0`; `0x4E3` precedes `0x4E4`. | High | Yes |
| The static owner-draw path uses timer id `0x65` and interval `0x22` ms around the movie handle. | `OwnerDraw_Static_006153E0`. | High | Yes |
| The initial template rect of static `0x71A` is not the final movie size source. `0x4E4` creates a handle and then the static path uses movie dimensions/vtable methods. | `OwnerDraw_Static_006153E0` plus parent report's dialog resource parse. | High | Yes |

Player-visible consequence: the left panel is a looping movie sized by the
movie asset, selected by screen-width mode, and refreshed through a shell static
control rather than by the Rust egui main menu placeholder.

## 3. Bink vs VQA Resolution

`VQMovieHandle__Constructor` at `0x005C07D0` calls helper `0x005C0640` to resolve
the physical asset name from the caller-provided base name. The helper strips an
extension if one is present, performs archive/file lookup, and tries extensions
in this order:

| Address | String | Role |
|---:|---|---|
| `0x0082419C` | `.BIK` | first extension attempted |
| `0x008241A4` | `.VQA` | fallback extension attempted |

After a filename resolves, `0x005C07D0` checks the extension:

| Address | String | Role |
|---:|---|---|
| `0x0082D9CC` | `.bik` | case-insensitive Bink branch test target |
| `0x0082D9B4` | `Play_Movie() as Bink!\n` | Bink branch debug/log string |

If the extension is `.bik`, the constructor calls `FUN_004326C0`, stores the
returned Bink object in the movie handle, and uses `vtable__BinkMovieHandle`
(`0x007EE154`). Otherwise it follows the legacy VQA path through
`CDFileClass__Constructor` / `FUN_005BFAA0` and uses `vtable__VQMovieHandle`
(`0x007EE0F4`).

Finding:

| What | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| `Ra2ts_s/l` are generic movie base names, not VQA-only filenames. | `0x005C0640` extension order and `0x005C07D0` branch. | High | Yes |
| Retail YR uses the Bink branch for these assets when `.BIK` is found. | Binary branch plus retail asset probe finding `ra2ts_s.bik` / `ra2ts_l.bik`. | High | Yes |
| The legacy class/function names are misleading for implementation naming. | Constructor name says VQ, but branch installs Bink vtable on `.bik`. | High | Yes |

## 4. Bink Object Timing And Size

`FUN_004326C0` initializes the Bink movie object and delegates file opening to
`FUN_00432750`. Important fields and side effects observed in the constructor
path:

| Offset / constant | Verified behavior | Evidence |
|---:|---|---|
| object `+4` | Bink handle slot | `FUN_004326C0`, `FUN_00432750` |
| object `+0x20` | BSurface pointer slot | `FUN_00432750` |
| object `+0x28` | file handle initialized to `-1` | `FUN_004326C0` |
| object `+0x2C` | byte initialized to `1` | `FUN_004326C0` |
| object `+0x2D` | byte initialized to `0` | `FUN_004326C0` |
| `0x3C` | 60-tick basis used to derive ticks per movie frame | `FUN_00432750` |

`FUN_00432750` behavior that affects the main menu panel:

- Closes any old Bink handle with `_BinkClose@4` before opening a new one.
- Closes the old file handle if one is present.
- If the audio system is available, calls `_BinkSetSoundSystem@8` with
  `_BinkOpenDirectSound@4`.
- Tries the game raw-file path first. If that fails, it tries a disk path with
  `CreateFileA` and `_BinkOpen@8`.
- On success, calls `_BinkSetVolume@8` using the global audio volume.
- Computes a per-frame tick value as `int(0x3C / (fps_num / fps_den))`.
- Creates a `BSurface`, reads movie width/height from the Bink header, and
  calculates clip/window fields at object `+0x10/+0x14/+0x18/+0x1C`.
- Calls `_BinkDDSurfaceType@4` and stores the DirectDraw surface type at object
  `+8`.
- On failure, logs `Bink Error: %s\n` with `_BinkGetError@0`.

For the physical `langmd.mix` `ra2ts_l.bik` header, the movie is 15 fps. The
binary formula therefore yields `60 / 15 = 4` ticks per movie frame. This is
separate from the owner-draw static timer interval of `0x22` ms, which appears
to drive periodic invalidation/update checks around the movie object.

## 5. Retail Movie Asset Inventory

Asset probing was performed against the configured retail install at
`C:/Users/enok/Documents/Command and Conquer Red Alert II/`.

`cargo run --bin bik-survey -- ra2ts` succeeded:

| Asset | Dimensions | Frames | Keyframes | Largest/max packet | Audio |
|---|---:|---:|---|---:|---|
| `ra2ts_l.bik` | `632x570` | `431` | `[0]` | `50848` | none by `--avsync` |
| `ra2ts_s.bik` | `472x450` | `431` | `[0]` | `54096` | none by `--avsync` |

`cargo run --bin bik-survey -- ra2ts --avsync` reported:

```text
[AV] ra2ts_l.bik  no audio track
[AV] ra2ts_s.bik  no audio track
```

Direct parsing of one non-encrypted physical copy in `langmd.mix` found:

| Field | `ra2ts_l.bik` physical `langmd.mix` copy |
|---|---|
| MIX flags | `1` |
| XCC id/hash | `0x33665128` |
| File size | `7185136` bytes |
| Header version | `BIKi` |
| Frames | `431` |
| Width x height | `632x570` |
| FPS | `15/1` |
| Video flags | `0` |
| Audio tracks | `0` |
| Header largest-frame field | `49984` |

The first-match survey value for `ra2ts_l.bik` reports largest/max packet
`50848`, while the inspected `langmd.mix` physical copy header reports `49984`.
The follow-up playback/archive report resolved this as expected duplicate
priority: gamemd opens `LANGMD.MIX` first and `LANGUAGE.MIX` second, but each
new MIX is inserted at the search-list head, so `LANGUAGE.MIX` wins for
duplicates. Current Rust first-match behavior is therefore consistent for
`ra2ts_l.bik`.

No loose `ra2ts*` files were found under the retail install tree.

## 6. Button PCX Asset Selection

`OwnerDraw_Button_00612B70` formats the shell button art pieces from three
strings:

| Address | Format string | Piece |
|---:|---|---|
| `0x0083589C` | `b%c%c_li%d.pcx` | left cap |
| `0x0083588C` | `b%c%c_mi%d.pcx` | middle tile |
| `0x0083587C` | `b%c%c_ri%d.pcx` | right cap |

Verified state selection:

- First `%c` is `'u'` for up/unpressed or `'d'` for down/pressed.
- Second `%c` is fixed `'e'` on the default enabled path.
- `%d` is the height family selected from control height thresholds `0x18`
  (24) and `0x1E` (30).
- Disabled style forces the up-state path and applies an alpha blend of `0x80`.
- Pressed state offsets content/text downward by 2 pixels.
- Left and right caps are direct blits; the middle piece is tiled.
- The normal path expects the PCX pieces to exist; it is not a primitive button
  fallback.

Dialog `0xE2` main menu buttons are `108x23` DLU. With the MS Sans Serif 8 pt
shell conversion used in the prior dialog parse (`base_x=6`, `base_y=13`), this
becomes approximately `162x37` px:

```text
width  = MulDiv(108, 6, 4)  = 162
height = MulDiv(23, 13, 8)  = 37
```

Height `37` selects the `30` family. Therefore the standard main menu right-side
buttons use these enabled pieces:

| State | Left | Middle | Right |
|---|---|---|---|
| Up | `bue_li30.pcx` | `bue_mi30.pcx` | `bue_ri30.pcx` |
| Down | `bde_li30.pcx` | `bde_mi30.pcx` | `bde_ri30.pcx` |

## 7. Layout Consequences

The dialog resource is `DIALOGEX` id `0xE2`, rect `0,0,533,369`, font
`MS Sans Serif`, 8 pt. The movie panel's template child rect is
`0,0,304,266`, but the runtime message path replaces the visual sizing with
the movie handle dimensions.

Runtime positioning:

| Condition | Movie base | Static position before movie assignment |
|---|---|---|
| `screen_width == 640` | `Ra2ts_s` | `x=0`, `y=0` |
| other widths | `Ra2ts_l` | `x=0/y=0` for <= `800x600`; otherwise centered against an `800x600` shell rectangle |

The Bink dimensions observed by the current asset tooling are:

| Mode | Asset | Movie size |
|---|---|---:|
| 640-wide | `ra2ts_s.bik` | `472x450` |
| larger shell | `ra2ts_l.bik` | `632x570` |

The right-column button x/y DLU values from dialog `0xE2` convert to roughly:

| Control | DLU rect | Approx px rect |
|---|---|---|
| Single Player `0x683` | `425,125,108,23` | `637,203,162,37` |
| WW Online `0x684` | `425,152,108,23` | `637,247,162,37` |
| Network `0x578` | `425,179,108,23` | `637,290,162,37` |
| Movies/Credits `0x686` | `425,206,108,23` | `637,334,162,37` |
| Options `0x55C` | `425,233,108,23` | `637,378,162,37` |
| Exit `0x3EE` | `425,330,108,23` | `637,536,162,37` |

These pixel positions are derived from the same DLU conversion as the parent
report and should still be checked against a captured retail screenshot before
final pixel-perfect implementation.

## 8. Current Rust Implementation Status

Rust asset support exists for the individual asset formats:

- `src/assets/mod.rs` exports Bink modules including `bink_file`, `bink_decode`,
  and `bink_audio`.
- `src/bin/bik-survey.rs` and `src/bin/bik-player.rs` can load and inspect Bink
  assets; the survey successfully decoded both `ra2ts_*` files.
- `src/assets/pcx_file.rs` supports the 8-bit one-plane PCX shape used by shell
  art with an embedded VGA palette.
- `src/render/skirmish_shell_chrome.rs` already loads the `bue_*30` and
  `bde_*30` shell button PCX family for the Skirmish shell atlas.
- `src/app_skirmish_shell_render.rs` has adjacent shell owner-draw button logic,
  but it is Skirmish-shell specific.

The standard initial main menu is still missing:

- Dialog `0xE2` layout/rendering.
- Static `0x71A` movie panel using `ra2ts_s/l.bik`.
- Bink-frame presentation integrated into the main menu surface.
- Main menu reuse of the shell owner-draw `30`-height PCX buttons.
- Exact `GUIMainButtonSound` trigger on these right-column buttons.

No Rust code was changed by this investigation.

## 9. Open Questions

1. A retail screenshot or video capture is still needed to validate final
   DLU-to-pixel positions, centering, and any one-pixel shell font metric
   differences.
2. The second BIK entry seen in `langmd.mix` with id `0x48113C24` was not mapped
   to a candidate filename in this pass.
3. Full global MIX archive priority beyond the RA2TS-relevant
   `LANGUAGE.MIX`/`LANGMD.MIX` relationship is not fully linearized here.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- Ghidra/live binary findings:
  - `FUN_00531CC0` main menu creation and `Ra2ts_s/l` selection
  - `FUN_0052B9B0` movie panel refresh/reposition helper
  - `OwnerDraw_Static_006153E0` custom static/movie messages
  - `0x005C0640` movie extension resolver
  - `VQMovieHandle__Constructor` `0x005C07D0`
  - `FUN_004326C0` Bink object constructor
  - `FUN_00432750` Bink open/timing/surface setup
  - `OwnerDraw_Button_00612B70` shell button PCX selection
- PE string checks from `gamemd.exe`:
  - `.BIK` at `0x0082419C`
  - `.VQA` at `0x008241A4`
  - `.bik` at `0x0082D9CC`
  - `Play_Movie() as Bink!\n` at `0x0082D9B4`
  - `Ra2ts_l` at `0x00825CE0`
  - `Ra2ts_s` at `0x00825CE8`
  - `b%c%c_ri%d.pcx` at `0x0083587C`
  - `b%c%c_mi%d.pcx` at `0x0083588C`
  - `b%c%c_li%d.pcx` at `0x0083589C`
- Local verification commands:
  - `cargo run --bin bik-survey -- ra2ts`
  - `cargo run --bin bik-survey -- ra2ts --avsync`
  - Direct non-encrypted `langmd.mix` header parse for `ra2ts_l.bik`
- Rust files inspected:
  - `src/assets/mod.rs`
  - `src/assets/pcx_file.rs`
  - `src/bin/bik-survey.rs`
  - `src/bin/bik-player.rs`
  - `src/render/skirmish_shell_chrome.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/ui/main_menu.rs`

## Related reports (added 2026-05-18 main-menu --area swarm)

The 2026-05-18 main-menu swarm produced five new reports covering audio
and interactive surfaces adjacent to the visual-asset path documented
here:

- `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` — shell music is the
  `[INTRO]` theme (`Sound=Drok`); started from `Main_Game @ 0x0052D9A0`
  before the dialog `0xE2` is created. Companion to the visual side
  documented here.
- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` — `ShellButtonSlideSound`
  consumer at `RulesClass + 0x750 @ 0x00607F59`; not triggered on initial
  menu entry.
- `EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` — verified-negative:
  no EVA welcome cue exists; the only entry audio is the INTRO theme.
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md` — Quit button confirm
  modal uses RT_DIALOG `0x120` and CSF `GUI:ExitAreYouSure / TXT_OK /
  GUI:Cancel`.
- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md` — partial: the
  frame-10 row on the right-panel chrome is a binary highlight-vs-default
  selector; predicate is byte `+0xD8` of the WindowExtra record.

## Related reports (added 2026-05-19 main-menu --area swarm #3)

The 2026-05-19 main-menu swarm produced five reports on residual gaps. Most
relevant to this visual-assets doc:

- `RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md` — full draw stack for
  `RightPanel__Draw 0x0072E450` + `RightPanel__ComputeLayoutRects 0x0072EC70`
  + `Background_Overlay 0x0072E730`: 6-step paint order; SDBTNBKGD uses
  SHELL2.PAL (all other right-panel SHPs use SHELL.PAL); tile-row count
  formula `(min(screen_h, 600) - SDTP_h) / SDBTNBKGD_tile_h`; centering
  origin `(max((screen_w-800)/2, 0), max((screen_h-600)/2, 0))`.
- `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` — full WM_PAINT
  dispatch for Static controls (4 kinds: text / text-anim / image / SHP-anim);
  movie static `0x71A` suppresses WM_PAINT and is drawn via explicit `0x4F0`
  from the parent dialog proc; Bink loop timer interval 34 ms.
- `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md` — full 12-branch
  dispatch table mapping Win32 control classes (Static / Button / Checkbox /
  Edit / ListBox / ComboBox / ScrollBar / Trackbar / Progress / Tab / Hotkey)
  to their owner-draw paint procs; universal subclass proc `0x610CA0`
  installed on every control.
- `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md` — per-control
  resize math for non-default resolutions: `0x694` heading gets Y+7 / H+1
  nudge; `0x695` tooltip bottom-left anchored at `+10` px; `0x71D` version
  uses sidebar inset `(168 - ctrl_w) / 2`; `0x71A` movie repositioned via
  `FUN_0052B9B0` with threshold `< 801 / < 601`.
- `MAIN_MENU_DIALOG_0XE2_TOOLTIP_HOVER_FLOW_GHIDRA_REPORT.md` — **resolves
  this doc's open question Q5.** Control `0x55F` (STT:MainButtonYuriWebSite,
  "Yuri Website") IS present on dialog `0xE2`, alongside the other 6 buttons:
  `0x683` SinglePlayer, `0x684` WWOnline, `0x578` Network, `0x686` Movies,
  `0x55C` Options, `0x3EE` ExitGame. All 7 are wired into the
  `FUN_006040B0 @ 0x006040B0` tooltip-string registry (case `iVar4 == 0xE2`).
  Tooltip mechanism: WM_NCHITTEST in the common shell proc looks up the
  string and sends `SendMessageA(GetDlgItem(parent, 0x695), 0x4B2, 0, str)`;
  static `0x695` then repaints with the localized tooltip text. **No Win32
  TOOLTIPS_CLASS popup window is used** — tooltips appear only in the
  bottom-left status line.
