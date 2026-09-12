# Launcher Options: retail layout and shared text admission

Status: final validation and independent scoped review passed. This increment covers
the launcher D5 parent, not the unfinished Keyboard/Network children or whole
shell-family acceptance.

The former egui card requested 720 logical points on an 800-pixel shell and
constrained its horizontal parent to 540 points. At 125% desktop DPI, the right
column collapsed and settings were clipped. The new production path uses the
retail physical rectangles, bitmap font and shell/control artwork. The retained
`OptionsDialogState` still owns values and ordered events; both the physical
view and assetless fallback drain the same app transaction for previews,
resolution changes, accepted settings and parent results.

## Acceptance for this increment

- At 800x600, all four sections, captions, values, title, warning and three
  right-column buttons fit the production frame at the available desktop DPI.
- Six sliders reach each endpoint. Thumb capture follows movement outside the
  control; a rail click changes once. Disabled audio rejects edits.
- Only checkbox icons toggle. Resolution opening, scrolling, selection and
  dismissal remain topmost, with ordered open/close cues and immediate selected
  dimensions. Main Menu persists settings. Keyboard/Network currently return
  fresh snapshots through their existing placeholder routes; their child screens
  remain required follow-up work.
- Shared text keeps the first partially clipped line and admits subsequent
  lines by consumed cell advance. Normal and animated text use the same rule.
  Inspect affected Skirmish and in-game text as well as launcher output.
- Independent original-evidence/diff/validation criticism passes before
  publication. A plausible Rust frame is not a native pixel comparison.

## Original evidence

Binary: retail `gamemd.exe`, x86, image base400000, SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

Owner55FC80 supplies dialogD5 and procedure55FDB0 to622650 at55FCB3–55FCC1.
The original RT_DIALOG/D5/language1033 resource is an extended533x369 DLU
template,31 controls, MS Sans Serif8. DLU conversion uses base6x13. The resource
was independently extracted directly from PE bytes and consumed all2018 bytes.
Hidden/disabled603 is excluded. Static captions and slider values are separate
controls, rather than combined labels.

| Control | Physical rectangle at800x600 |
| --- | --- |
| Detail52B | 134,86,180,21 |
| Difficulty50F | 138,208,180,21 |
| Scroll52A | 357,330,180,21 |
| Music52F / Sound532 / Voice536 | 123 /269 /416,470,128,21 |
| Resolution6ED face | 351,86,180,24 |
| Keyboard5CE / Network5CD | 644,199 /241,156,42 |
| Main Menu686 | 644,535,156,42 |
| Title694 | 635,9,162,17 |
| Warning71C | 670,47,92,54 |

608CD0/609730 perform ordinary right-column anchoring;60B950 applies title
y+7 and height+1. These are source-derived dimensions, not captured native HWND
rectangles. 603 is hidden and disabled in the resource. 6153E0 selects GAME.FNT
and yellow foreground; ordinary statics keep resource horizontal alignment and
top alignment. Checkbox6163A0 paints an18x18 icon, admits icon-local mouse down,
and starts vertically centered caption text at x+26.

60CF00's default background path selects MNSCRNL.SHP through SHELL.PAL. D5
is excluded from60CAF0's SDTP frame-1 highlight allowlist. Warning6038F7→6039EF
resolves original pointer833674→833688 to SDWRNANM.SHP, using SHELL2.PAL:
92x53 canvas,91 frames. 60A9C8–60AA0B initializes frame0 and a1ms startup timer;
6153E0's timer path then requests a recurring interval equal to the frame count.
Paint advances the frame; elapsed time alone does not select it. The Rust
presentation retains dirty content and acknowledges it after successful present.
Title reuses the shared30ms kind-1 reveal with step1 and leading span8.

Trackbar61D950 distinguishes two formulas. 61DA52–61DA79 establishes the usable
span `width - plaque_reserve - 13`. Initial setup,405 and406 project position
to thumb offset using `span * position / range`; drag/click quantize mouse using
`range + 1`, then project using `range`. The previous launcher thumb helper used
`range + 1` for both and stopped short. 4AC disables the plaque for the first
three sliders;4AE suppresses change sound for the audio sliders without changing
their default50-pixel plaque. Neither message resizes the control.

[The reproducible original-instruction oracle](../../../tools/storage_oracle/launcher_trackbar.py)
checks16 supplied geometry combinations,92 positions,2736 captured-drag pointer
samples and2464 admitted rail-click comparisons. Generation and default read-only
regeneration passed. Supplied width/reserve cross combinations are arithmetic
coverage, not a claim that every combination appears in D5.

Resolution617250 paints a24px face and sets GAME.FNT row height17+6=23.
6180CF–618205 sizes custom ComboDropWin from dropped-control allocation and
whole rows. The nominal120px allocation admits five rows; exact Win32
CB_GETDROPPEDCONTROLRECT border adjustments remain unmeasured. 60D540 selects
on mouse down, initializes top index0, and60E4E9–60E500 plays GUIComboCloseSound
before selection or outside dismissal. Scrollbar forwarding precedes that cue.

The shared line-admission discrepancy is independently established through
6153E0→775690/621040→433CA0/434CD0: the original HWND rectangle is the clip;
backing-surface +1 dimensions do not enlarge it. 434CD0 draws the first line
before a height test. At newline/wrap tails434EC2–434ED7 and435112–435127,
consumed cell advance stops further lines at the nonzero height limit. The old
Rust precheck discarded a line when its glyph did not fit completely. Both
plain and Path-A wrappers now admit the line and preserve the original scissor.
Retail GAME.FNT has17px cell advance and16 bitmap rows: an ordinary unwrapped,
top-aligned16px static already passed the old check. This correction concerns
partial lines and smaller clips; it is not evidence for the cause of every
previously missing label.

[The full-body line-submission oracle](../../../tools/storage_oracle/shell_text_lines.py)
executes434CD0 with original glyph lookup and supplied font metrics. Sixty-six
cases include48 newline/wrap cases covering one through four lines and
heights0/1/16/17/18/34, plus six saved-space word-suffix cases at widths18/24/30
and heights0/34, and12 no-space cases with two/three/four/six glyphs at
widths6/12/18 and unlimited height.
Only surface setup/teardown and glyph raster are substituted. Generation and
read-only comparison passed; Rust compares the emitted positions in both text
paths. This demonstrates bounded line admission, not native raster pixels.

The executable comparison exposed a pre-existing shared wrapping bug:
`A A` at width6 could produce a reversed byte span, and overflow after part of
a word could skip measuring that prefix on the next line. Original434F60–434F64
rewinds the scanner to the saved space;4350F5–435103 and43512D–435138 resume
after it. `BitFont::wrap_layout` now retains the break's character index and
width together and restarts after that break, deriving UTF-8 bounds from the
same index. The six additional native cases require those instructions to run.
Both shell text paths compare all66 native submission sequences. The former
vertical-centering test expected a fully fitting glyph; native line admission
instead submits both lines at y=-2/15 in a30px rect and leaves clipping intact.

The subsequent word-suffix comparison exposed a distinct hard-cut mismatch.
Original434F6F–434F78 moves the emission boundary back by one UTF-16 unit;
43512D–435138 resumes measurement at the overflowing unit. The deferred fitting
glyph stays in the next emitted span without being measured again. Rust now
keeps those two positions distinct. The bounded no-space fixtures show that
native can emit an empty first line at minimum width, or submit later glyphs
beyond the supplied width; clipping is a separate responsibility. This is not
a conventional word-wrapper substitution or a general Unicode equivalence claim.

612B70 type1 buttons keep the normal foreground when pressed. The shared
button text rectangle matches61358D–6135CD: unpressed top+1/right-2; pressed
left+2/top+5. Frames2 and4 represent idle and pressed. Frame3 is driven by a
separate timer flag, not ordinary pointer hover. No D5 primary-procedure sender
of that timer's4DC message was found; this increment keeps the ordinary2/4 path.

Fresh criticism found a lost-focus gesture issue. The host now cancels slider,
button and popup-drag capture at focus loss before accepting subsequent movement,
matching61E3AF–61E44E's absent-MK_LBUTTON cancellation requirement. A regression
checks reentry movement cannot change packed settings or emit preview events.

## Validation and remaining limits

The initial full suite reported8706 passed, two failed and121 ignored. The
failures were the wrapping defect and superseded vertical-centering expectation
described above. The next run passed8707 with one hard-cut mismatch and121
ignored; that source repair and expanded comparison are described above. The
final candidate full `cargo test -p vera20k --lib` passed8708 tests with zero
failures and121 ignored in30.58s (`launcher-options-full-tests-final2.log`).
`cargo clippy -p vera20k --lib` exited0 in1m22s with1152 warnings, including
existing warnings and nonblocking style suggestions (`launcher-options-clippy.log`).
The release build passed in4m39s with91 warnings. Fresh independent read-only
criticism of native evidence, design, final diff, documentation and validation
passed after the documented repairs. The approval covers this bounded increment.

Production release215ad210c994cd778741b244cf0ac9f5b18ee28be4e19750acb4df71180473e2
was inspected at125% desktop scale (800x600 physical client). All four sections,
six sliders, three checkboxes, title/warning and three buttons fit. Mouse drags
and rail clicks reached both endpoints of every slider; all three checkbox icons
toggled. The resolution popup showed this runtime's three available modes and
selected800x600 without resizing the shell. Keyboard and Network route through
their existing stubs and reconstruct the parent with saved values. Main Menu
exit/reentry also retained the settings; direct RA2MD.INI readback confirmed
resolution800x600, detail2, difficulty2, scroll0, three audio volumes1.0, tooltips
off, target lines off and hidden objects on. This release precedes the subsequent
word-wrap repair; final release validation follows below.

Local evidence is retained under `.local/seed-browser-runtime/`:
`launcher-options-native-initial.png`, `launcher-options-resolution-popup.png`,
`launcher-options-endpoints.png`, `launcher-options-reopened.png`, and
`RA2MD-launcher-endpoints.INI`. Captures establish VERA output, not retail pixel
equivalence or warning-animation cadence.

Final release SHA-256
`595932ce9a66adc14abc80727343f893949df8e62ca5c7213a757c0b82695724`
was inspected after process restart. All sections and persisted settings remained
visible and correct. Resolution popup opening and outside dismissal passed;
clicking the Main Menu button while the popup was open only dismissed the popup,
and the next click returned to the main menu. Final captures are
`launcher-options-final-restart.png` and `launcher-options-final-popup.png`.
The final executable's production Skirmish capture passed its diagnostic
validator and retained the exact previous frame SHA-256
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`.
The local bundle is `.local/launcher-options-skirmish-capture`; this is a Rust
visual nonregression check, not native pixel equivalence.

Neighbor checks on the earlier `215ad210...` release reached Skirmish, loading, the first800x600
tactical frame, Game Controls, its Escape return and Abort to the main menu.
Game Controls still lacks its panel and several captions, and the pause/abort
menus still use egui; those ordinary family gaps remain open. Captures
`launcher-neighbor-skirmish.png` and `launcher-neighbor-game-controls-gap.png`
record the observed scope without attributing all missing labels to clipping.

Independently confirmed Ghidra plate comments at61D950,434CD0 and55FC80 were
written, the intended `gamemd.exe` program saved, and all three comments read
back exactly. Existing function names were preserved.
Native pixel comparison, complete D5 entry/exit transition equivalence, and all
child-shell behavior remain open. This report does not certify those mechanisms.
