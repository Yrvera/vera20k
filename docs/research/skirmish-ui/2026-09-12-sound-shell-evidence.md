# Active in-game Sound shell: native evidence

Date: 2026-09-12. Native evidence and bounded comparison packet for the integrated
B8 Sound increment. Independent source/evidence review passed after the shared
button-frame correction recorded below. Final production validation is recorded in the
[integrated report](2026-09-12-sound-shell.md).

## Source and active caller

Original retail `gamemd.exe`, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
The original instructions and resource bytes were read independently; Ghidra names
were leads. Callback `6B6300` was decoded directly from the pinned image with
Capstone because it was not a defined Ghidra function. CSF values were read from
retail `langmd.mix/ra2md.csf` with raw values retained during inspection.

BBB callback `4E1FE0` admits Sound control `52D` on notification0 while
`A8E9A0==1`, sets pause state6, and accepts/closes BBB. Dispatcher
`48C9EB..48C9FE` calls `6B6230`, then sets state5 to reopen BBB. The Sound owner
selects resource **B8** when active (`6B6258..627C`), otherwise inactive D6;
it installs callback **6B6300**, creates/registers through `622650`, and pumps
`623120` until its result changes. This is the shared modeless-dialog owner loop,
not a nested Windows `DialogBox`.

BBB enables its Sound button using `407000`, which reads `87E728!=0`. Sound
initialization uses the same predicate for the three sliders, two checkboxes and
music list. Ordinary investigation assumes an available audio device.

## Original B8 resource

The original PE resource tree identifies RT_DIALOG B8, language1033, VA
**BEFE4C**, **800 bytes**, SHA256
`fe780c2fcb6242d7ed0f238116ae240bdb60267a848b1c70bc4bddda11a58430`.
Standard DLGTEMPLATE: style `40000040`, extended style0, fourteen controls,
DLU bounds `(0,0,533,369)`, font8 MS Sans Serif. All child extended styles are0.
The raw-pixel column supplies the established 6x13 dialog base units and MulDiv
rounding; Windows font metrics were not emulated in this packet.

| ID | Class | Style | Resource caption | DLU x,y,w,h | Raw pixel x,y,w,h |
|---|---|---|---|---|---|
| 686 | Button | 5000000B | GUI:Back | 425,346,108,23 | 638,562,162,37 |
| 694 | Static | 50020001 | GUI:SoundOptions | 425,1,108,10 | 638,2,162,16 |
| 52F | msctls_trackbar32 | 50010018 | Slider1 | 157,60,175,13 | 236,98,263,21 |
| 532 | msctls_trackbar32 | 50010018 | Slider2 | 157,82,175,13 | 236,133,263,21 |
| FFFF | Static | 50000202 | GUI:MusicVolume | 61,59,90,15 | 92,96,135,24 |
| FFFF | Static | 50020202 | GUI:SoundVolume | 61,81,90,15 | 92,132,135,24 |
| 530 | ListBox | 50010151 | empty | 157,129,175,99 | 236,210,263,161 |
| 531 | Button | 5001000B | GUI:Play | 81,246,83,15 | 122,400,125,24 |
| 535 | Button | 5001000B | GUI:Stop | 249,246,83,15 | 374,400,125,24 |
| FFFF | Static | 50020202 | GUI:VoiceVolume | 61,103,90,15 | 92,167,135,24 |
| 536 | msctls_trackbar32 | 50010018 | Slider2 | 157,104,175,13 | 236,169,263,21 |
| 533 | Button/checkbox | 50008003 | GUI:Shuffle | 81,187,70,14 | 122,304,105,23 |
| 534 | Button/checkbox | 50008003 | GUI:Repeat | 81,214,70,14 | 122,348,105,23 |
| 695 | Static | 50000200 | GUI:Blank | 2,355,303,12 | 3,577,455,20 |

The anonymous statics share ID FFFF; retain each resource entry rather than
keying the descriptor solely by numeric control ID.

## Native composition and placement

`60C540` includes B8 in background mode1. Common active painting uses `72F540`,
the same full themed in-game frame as BBB/B5/B6. `60C0C0` routes ordinary B8
controls through `60B7A0`; `608500` contains no B8 special rectangle, and
`608CD0` recognizes its title694 rather than Play/Stop. `60B950` supplies no
ordinary B8 post-adjustment. Thus sliders, labels, list, checkboxes and Play/Stop
receive signed offsets `(-80,-60)`, `(0,0)`, `(112,84)` at640x480,800x600,1024x768
respectively, with `60B7A0`'s final coordinate clamp. The reproducible `sound_shell_layout` comparison now retains the raw resource
and executes all33 ordinary placements through original60B7A0 at these sizes.

Back686 uses `609730` -> `60B350`: shared right anchor
`(W-147,SIDE3.y-button_height,button_width,button_height)`. Title694 uses
`608CD0` -> `60B1D0`, the shared title anchor. Footer695 uses `601360` ->
`60B550`, yielding `(10,H-21,455,20)`.

Play531 and Stop535 use **MNBTTN type3**, not SIDEBTTN. Original selector
`609FC7..609FE2` recognizes this B8 pair; `60A330` assigns type3 after the
sidebar-button exceptions. Retail `MNBTTN.SHP`, resolved from
`ra2md.mix/localmd.mix`, has canvas126x25 and three frames. Their ordinary
control rectangles remain125x24; do not infer an image rescale or rectangle
expansion from the asset size alone. Reuse the established type3 paint path.

The three volume captions are ordinary statics: style lowbits2 chooses alignment
`12h` at `615A81..615AE8`, horizontal right and top aligned despite style200h.
`602490` enrolls title694 in the shared type1 title effect but not these anonymous
labels; `602B90` has no B8 alternate static classification.

All sliders preserve the **numeric plaque and 50px reserved width**.
`6B6382..647F` sends `4AE=0` to suppress generic click sounds. It does not send
`4AC=0`, which would disable that plaque. The expanded `tools/storage_oracle/launcher_trackbar.*` comparison now includes
B8 width263, reserve50 and maximum10: eleven valid positions and280 pointer
coordinates. The original seventeen geometry cases are unchanged. Independent
read-only replay passed; this is geometry/input coverage, not full slider paint.

## Live audio, Back and keyboard dismissal

Initialization sets each slider range0..10 and position
`trunc(f32_volume * 10.0 + 0.5)`. Literal original doubles are
`7E44A8=10.0`, `7E1738=0.5`, `7E3860=0.1`. `B0B800` suppresses notifications
during initialization. Afterwards HSCROLL `6B662F..671E` reads the sending
slider and applies position times0.1 immediately; there is no code5-only gate.

| Slider | Setter | Retained Options field | Live effect |
|---|---|---|---|
| Music52F | 5FA4A0 | +40 / A8EBA0 | music gain, no preview beep |
| Sound532 | 5FA510 | +38 / A8EB98 | sound gain, GenericBeep at local gain1 |
| Voice536 | 5FA590 | +3C / A8EB9C | voice gain, GenericBeep at local gain equal to new voice volume |

Sound/Voice HSCROLL calls pass preview=true. Their original preview branches
read Rules+710 and invoke `750920`; this is GenericBeep, distinct from the
suppressed trackbar GenericClick at Rules+70C. Store -> output -> cue ordering
matches the already implemented launcher preview mechanism.

Back686 accepts only notification0 (`6B68AF..68BD`). It reapplies music, sound
and voice from their slider positions with preview=false (`6B68C3..69A5`), then
sets result1. If music volume equals0, `6B6905..6939` invokes Queue(current)
then Stop(false). No audio rollback or WriteToINI occurs in the Sound owner or
callback. The dispatcher reopens BBB; its subsequent accepted close performs
the existing full Options persistence transaction.

**Current-track correction:** B8 reads **Theme+4 retained (`A83D14`), falling
back to Theme+8 pending (`A83D18`)**. This literal sequence occurs during list
selection `6B65CE..65DF`, Play `6B67F8..6810`, and zero-volume Back
`6B6918..692D`. It is not active(+0) falling back to retained(+4).

Callback6B6300 handles neither IDOK1 nor IDCANCEL2. Therefore the shared default
Enter/Escape notifications do not close B8. The registered modeless-dialog
`IsDialogMessageA` path was independently established for its shared owners;
Microsoft documents the default IDOK/IDCANCEL notifications:
[dialog programming considerations](https://learn.microsoft.com/en-us/windows/win32/dlgbox/dlgbox-programming-considerations),
[IsDialogMessageA](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-isdialogmessagea).
This does not establish every focused slider/list/checkbox keyboard action.

## Music catalog, selection and commands

Initialization `6B6518..6613` clears ListBox530 and enumerates the Theme catalog
in original index order. `721140` admits available, Normal entries and enforces
Side where present; the Scenario gate applies only in campaign mode0. Stock
skirmish skips the campaign gate. Original `THEMEMD.INI` resolves from
`ra2md.mix/localmd.mix`, size2718, with no loose override in the inspected
installation. A separate playlist must not bypass Theme admission.

Every admitted row stores its original Theme index with message19A. The row
format at `840070` is `%02d - %s [%d:%02d]`; the display ordinal starts1 and
increments among admitted rows. Name comes from entry+200 (`7209B0`), populated
by `720480` -> `529160` from the localized `Name=` CSF key. That reader supplies
a 64-wide-character destination and terminates it.

Duration is **WAV-derived seconds**, not the commented INI `Length` field.
`7207F0` checks Sound.WAV availability, parses its header through `408610`,
computes milliseconds through `408560`, integer-divides by1000 and stores a
float at entry+284. `720E50` converts that value to integer seconds for the
minutes/seconds formatter. The preserved native comparison below covers all
ordinary stock tracks and bounded PCM arithmetic.

Initial selection matches retained-else-pending when present, otherwise row0;
messages186 and197 select and set the top index. ListBox530 uses `618D40`, not
the saved-file browser's multi-column control. Its497 initialization sets row
height to GAME.FNT cell height+2 (19px for the established17px stock cell).
Row click selects. Double-click posts notification2 for530; B8 does not handle
that ID as Play, so do not invent double-click playback.

- Play531 (`6B67B1..6828`) reads selected row/item data and sets Options.IsScore
  at `A8EBA5` to1. If the selected original Theme index differs from native
  current, it calls Stop(true) then Queue(selected).
- Stop535 (`6B69C8..69E5`) calls Stop(true), Queue(-3), then sets IsScore to0.
  Preserve the existing Theme fade/pump ownership rather than direct playback
  replacement or immediate output stop.
- Shuffle533 (`6B6749..679D`) writes Options+46 and Theme+12 immediately through
  `5FA440`. Checking it unchecks Repeat534 and calls `5FA470(false)`.
- Repeat534 (`6B683C..6890`) writes Options+44 and Theme+10 through `5FA470`.
  Checking it unchecks Shuffle533 and calls `5FA440(false)`.
- Both options unchecked is allowed. Their initial checks come from retained
  Options state, not an invented default.

## Footer

`6040B0`'s B8 branch supplies the following keys to the shared hover-driven695
update. All text below was read from original English CSF without normalization
changes. Empty/non-help areas clear the footer through the common path.

| Control | Key suffix after STT:SoundOpt | Original text |
|---|---|---|
| 52F | SliderMusic | Controls the music volume level. |
| 532 | SliderSound | Controls the sound volume level. |
| 536 | SliderVoice | Controls the voice volume level. |
| 530 | ListScore | Lists the available music tracks. |
| 533 | CBoxShuffle | Shuffle the order of the music tracks that play. |
| 534 | CBoxRepeat | Repeat a single music track. |
| 531 | ButtonPlay | Play a music track. |
| 535 | ButtonStop | Stop a music track. |
| 686 | ButtonBack | Return to the previous screen. |

## Integrated Rust ownership

B8 reuses InGameShellFrame, shared title/Back/footer anchors, physical placement,
checkbox/trackbar mechanisms and MNBTTN painting. SoundState owns projected
controls, original-index music rows and interaction; BBB -> B8 -> BBB is explicit.

`src/app/persistence/options/audio.rs` owns shared profile -> output -> optional
GenericBeep dispatch. AppState implements the output operations; both launcher
preview and B8 use them. The private launcher adapter was removed. B8 Back uses
cue-silent setters; BBB acceptance applies/persists before opening the child.

ThemeEntry carries localized name keys and native duration metadata. Sound's app
adapter projects admitted original-index rows and calls narrow playback wrappers
in audio_runtime.rs. ThemeRuntime retains queue/fade/current-song authority and
its continuing audio pump. Both callers use the independently checked retained-
else-pending current query; score-zero queues that query then stops(false).
The previous active-else-retained launcher query was corrected.

## Preserved native audio prerequisite comparison

`tools/storage_oracle/sound_theme_metadata.py` executes original descriptor
block `408751..4087C6`, complete `408560` including original CRT multiplication
and division, and seconds-store block `7208D3..7208F2`. Original file I/O and
chunk parsing are substituted with supplied fmt/data metadata and valid scratch
registers. `run_checked` requires completion and the relevant interior visits.
The fixture does not decode audio or claim full native WAV-parser parity.

Original `408795..4087B6` reads and then overwrites the header average byte rate.
PCM rate is `(bits>>3) * channels * sample_rate`; IMA sets decoded bytes/sample2
and computes `(2 * channels * sample_rate)>>2`. `408560` calculates
`floor(data_bytes*1000/rate)`; `7208D3..7208EC` divides by1000 before storing float.
All eight ordinary retail songs are IMA0x11, stereo22050Hz, block1024, bits4:
header average22201, native computed rate22050. Native displayed duration differs
from the header-rate/playback duration by one or two seconds on these files.
The common20-byte fmt payload is
`1100020022560000b9560000000404000200f903`.

| WAV stem | Declared data bytes | Native milliseconds | Native display |
|---|---:|---:|---|
| BrainFre | 5329920 | 241719 | 4:01 |
| Drok | 3447808 | 156363 | 2:36 |
| Deceiver | 5176320 | 234753 | 3:54 |
| PhatAtta | 5249024 | 238050 | 3:58 |
| Bully | 5638144 | 255698 | 4:15 |
| Defend | 5883904 | 266843 | 4:26 |
| Tactics | 6609920 | 299769 | 4:59 |
| TranceLV | 4606976 | 208933 | 3:28 |

Each original WAV SHA256 and exact header through its data-size field is retained
in the JSON. Asset-browser winner checks independently confirmed thememd.mix for
all eight, with no loose override. Original THEMEMD.INI bytes were also read from
ra2md.mix/localmd.mix:2718 bytes, SHA256
`b7cf74e6bbac7a0656e4739e9158bd6e9988e867d19bb20c54706b755be23085`;
its eight Normal entries map to these WAV stems and have no Side restriction.

The comparison adds sixteen PCM inputs: mono/stereo,8/16-bit,22050/44100Hz,
with declared payload one byte before and exactly at60seconds. Four supplied
slot states execute each original launcher/B8 current query, including cases
where active differs and retained is absent. The reference sidecar states all
preconditions and exclusions. Canonical payload SHA256:
`445681154dc4366839506262f8986a3966d533174c280c50b490fe2d8712474a`.

Default read-only verification passed:
`python -m tools.storage_oracle.sound_theme_metadata`.
Connected Rust regression tests:

- `assets::wav_file::tests::metadata_duration_matches_original_instructions_and_retail_headers`
- `audio::theme::tests::catalog_reads_thememd_only_and_scans_wav_availability`
- `audio::theme::tests::current_song_matches_both_original_callers`
- `audio::theme::tests::score_zero_preserves_pending_request_when_nothing_is_retained_or_active`
- `audio::theme::tests::sound_play_and_stop_preserve_theme_queue_and_fade_authority`

`assets/wav_file.rs` shares the borrowed RIFF/fmt/data parser with
`audio/sfx.rs::decode_wav`; playback keeps its sample decoder and mono-to-stereo
conversion. Declared data length is distinct from the bounded payload slice;
metadata never decodes all music just to obtain a list label. Truncated fmt
records fail safely. The native duration calculation is explicitly separate
from playback length. Existing decoder tests remain required validation.

## Ordinary acceptance and limits

Fresh criticism corrected a shared type3 paint interpretation: original
`612F36..612F5B` selects MNBTTN frame1 for held state and frame2 for timer
highlighting. Disabled text is handled after this selection at `612F5F`.
The same independently traced handler covers B8 Play/Stop, RMG105 controls
620/621, and messages CE/120/121 used by validation, saved-file prompts and
main-menu Quit. All affected Rust consumers now use one ordinary type3 mapping
(released0, held1); timer highlighting remains a separately tracked residual.
The four-combination `owner_button_frame` fixture executes the original frame
selection, including held precedence over highlighting. Historical reports and
the incorrect frame phrase in Ghidra609E20 were explicitly corrected.

Independently approved Ghidra plates6B6230,408560,55FAA0,612B70,609E20 and61D950
were saved and read back exactly, preserving names and the unanalysed raw
callback boundary. They distinguish bounded arithmetic/instruction evidence
from whole-dialog parity.

Integration evidence independently checked before implementation:

- BBB Sound notification0 at `4E2370..4E238F` sets state6 and accepted result1.
  The owner at `4E1D9A..4E1DAB` applies all parent controls through `4E1DE0`
  and writes `5FAD10` **before** creating B8. B8 Back reconstructs BBB from
  the retained profile. `4E2201..4E2229` disables Sound when `407000` reports
  no common audio device (`87E728 == 0`).
- B8 list530 uses the same `618D40`/`61C690` mechanism as saved browsers,
  with no `4A6` column setup. Its plain-string branch `619A30..619B42`
  paints ordinary text AC18A4 at inner-left+2/top, and selected background
  AC4604 without changing the text color. Only complete19px rows paint;
  the263x161 control has eight complete rows. The initial `LB_SETTOPINDEX`
  request is the selected row, clamped to count minus eight.
- List mouse-down selects and plays GenericClick; double-click emits
  notification2 which B8 ignores. Wrapper `610CA0:6118BF..6118DE` rejects
  focus for both list618D40 and button612B70 despite some resource controls
  having WS_TABSTOP. Trackbar61D950 consumes key/character messages; its
  explicit original-procedure forwarding at `61E3C2..61E443` is for
  WM_GETDLGCODE. Enter and Escape do not close B8.

The integrated Rust change reuses one Options audio setter dispatch for D5
and B8, one Theme playback owner, and shared native list geometry/painting
with saved browsers. Sound state owns control positions, row identities and
interaction only. `tools/storage_oracle/sound_shell_layout.py` retains the raw
B8 resource and33 complete original60B7A0 placements at the three ordinary
resolutions. It supplies font metrics and bounded geometry hooks; it does not
execute whole-dialog painting or focus. Final Rust/runtime receipts and the fresh independent scoped PASS are in the integrated report.

Required ordinary journeys: open Sound from BBB with audio available; show all
fourteen resource controls; adjust all three sliders across their discrete
range with live gains and correct previews; select a different admitted music
row and Play; Stop without unintended auto-restart; toggle mutually exclusive
Shuffle/Repeat while permitting neither; Back to BBB with retained values;
reopen and verify selection/options; verify subsequent BBB persistence.

At640x480,800x600,1024x768 and each supported retail theme, verify full frame,
title/Back/footer anchors, ordinary signed placement, numeric plaques, MNBTTN
Play/Stop, list rows and localized captions/help. Enter/Escape must not invent
a close action. A row double-click must not invent Play. Zero-volume Back must
preserve its literal queue/current/stop ordering.

Before full B8 parity claims, preserve raw-resource and original executable
placement comparisons, run the candidate state/authority and decoder regression
tests, and collect production render/input evidence. The current packet does not exhaust list paint/scrollbar
details, focused-control keyboard behavior, localized name truncation,
unavailable-device paths, campaign gating, inactive D6, malformed/overflow WAV
inputs, or full Theme score-zero admission. These limits are explicit follow-up work, not
evidence that those mechanisms match. The prerequisite owner independently supplied the WAV/Theme implementation and
comparison packet; the integration owner supplied app/UI changes and validation.
A separate fresh critic reviewed the combined change and original evidence.

The pre-existing BBB Game Controls button capture keeps held art visible when
dragged outside until release; the action still cancels correctly. This ordinary
visual residual remains required common-input/Keyboard follow-up. B5/B6/B8 use
the shared hover-aware pressed predicate; the critic confirmed this distinction.

## Production rail-width correction

The first live B8 capture exposed a renderer-cache mismatch: numeric controls
always used the cached128x21 rail frame even though B8 input/thumb/plaque used263.
The corrected shared selector covers every current production caller: numeric128
(Skirmish and D5 audio),225(RMG Players),263(B8 audio), plain180(D5) and192(BBB).
It uses the existing size-driven frame builder; no split-frame bitmap is stretched.
Original61E1B9..61E269 supplies rail width=control width minus reserve, then value
x=rail width+2 and value width=reserve minus2;6208F0 expands each border by2.
B8 therefore produces267x25 art, with213px rail and48px value box at x215.
The fresh critic independently decoded those original instructions and confirmed
RMG105's03EB resource atBFC028: DLU179,163,150,13, pixels269,265,225,21 with supplied
6x13 metrics. Native596E50 sets range2..8 without plaque suppression.
The final selector must retain this ordinary sibling; the initial explicit-width
fix omitted225 and was corrected before any rebuilt runtime claim.
Emitted-sprite tests cover numeric128/225/263 sizes and origins; a pixel test
checks B8's divider at215/216 in its267px canvas. This fixes a production gap that
the bounded slider arithmetic oracle could not detect.
