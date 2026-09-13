# Active Game Controls shell

Status: scoped independent review passed; the acceptance below is satisfied.
This is one increment of the [whole-shell acceptance](../../plans/2026-09-12-retail-shells-acceptance.md), not completion of the in-game family.

## Scoped acceptance

The ordinary offline Game Controls parent (`BBB`) should paint the active side's
complete retail shell, use physical window pixels at640x480,800x600,1024x768,
and retain its settings, pause and return authorities. Its two sliders have
plain192px rails; its three checkboxes and three buttons include their original
captions. Art, control placement, input coordinates and clipped font draws must
agree. Upscaling the battlefield must not upscale this shell or move its targets.
Back and the retained Escape route apply the existing close transaction and
return to the paused parent; resuming must preserve the battlefield camera.

Ordinary native feedback includes icon-only checkbox clicks, thumb capture versus
single rail clicks, position-change notifications and UI sound cues. Hidden
Visual Details controls remain hidden. Correct existing profile persistence and
simulation pause behavior remain authoritative.

The native pause parentB5, Keyboard/Sound children, title reveal/button timers,
complete keyboard/focus traversal and unavailable-device button states remain
required follow-ups. This increment does not claim those routes or whole-frame
native pixel equivalence. They must not be counted as completed by these checks.

## Native evidence

Original `gamemd.exe` SHA256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

- `4E1D00` choosesBBB while a scenario is active; `4E1FE0` configures its
  sliders, including `4AC=0`, disabling their50px value plaques. Range0..6
  displays6−stored speed. The parent handles `WM_HSCROLL` code5 and changes
  labels only after a changed position; pressing the current thumb alone does
  not replace the template “Faster” label.
- Common mode1 draw `621E90`, active call `621FD5`, invokes `72F540` with false.
  The latter clears the surface, then draws its selected background, credits,
  top, radar, side1, repeatedSIDE2B, side3/addon and closed lower strip. The false
  path drawsLENDCAP frame2 without BTTNBKGD repetitions; RENDCAP follows.
- `72FA10` loadsSIDE2B andSIDEBTTN through the side archive. `72FBC0` constructs
  converters; these generic pieces useSIDEBAR.PAL, including Yuri. Geometry
  `72FC60` deliberately usesSIDE2's header height while the painter usesSIDE2B.
  The atlas now has one side-specific owner for these buttons; the obsolete
  root-palette lookup in the Skirmish atlas was removed. Backgrounds separately
  useUIBKGD.PAL (Allied/Soviet) orUIBKGDY.PAL (Yuri), viaB0FBF0. The formerly
  unused background fields had incorrectly used the radar palette; their one
  existing owner now records and applies the separate background palette.
- `60B7A0` applies signed half-differences from800x600 to ordinary children,
  then clamps final x/y to zero. At640x480 the deltas are−80/−60. `60B000`
  snaps ordinary buttons from their original midpoint; `60B350` anchorsBack at
  `width−147, SIDE3.y−buttonHeight`. Active title `60B1D0`/`60B950` has no
  launcher nudge; footer `60B550` sits atx10, one pixel above the bottom.
- OriginalBBB resource atC01B18 supplies all17 controls and six interactive
  caption keys. Static `6153E0` uses top alignment with its horizontal style;
  checkbox `6163A0` shifts its caption's left edge26px. Button `612B70` uses
  text rectangle `(x,y+1,w−2,h−1)`, with pressed left/top shifts2/4 and fixed
  right/bottom edges; enabled foreground remainsAC18A4. `61374B..613771`
  plays the generic click on button-down.

## Reproducible comparisons and design

[Geometry oracle](../../../tools/storage_oracle/in_game_shell_geometry.py)
executes complete72FC60 with42 documented retail SHP inputs across three themes
and three resolutions. All16 rectangles, allocation sizes, repeat counts and
bottom anchors are compared. Nine additional full60B7A0 executions cover original
BBB caption714, slider529 and checkbox601. Only supplied window state, active
predicate and successful allocation are substituted; Windows font base units
6x13 are inputs. Payload SHA256:
`8dd165d683e8ad6c286f93cdc4e1c76719e80f729e099aa3367ac5f403511cda`.
The pure `ui::shell::in_game_shell` model and production child layout consume
these saved comparisons.

[Trackbar oracle](../../../tools/storage_oracle/launcher_trackbar.py) adds the
ordinaryBBB192/reserve0/range6 geometry while preserving all16 earlier fixtures.
The expanded fixture covers99 retained positions and2945 pointer samples;
Rust's BBB projection reads the new native results. It is an arithmetic
comparison, not a claim about Windows capture or final pixels.

The renderer selects a complete modal frame instead of recording battlefield
commands and then changing their shared camera buffer. The physical-sized shell
uses the existing RGB565 presentation boundary. The retained cursor remains in
tactical source coordinates; `window_cursor_position` projects it into physical
pixels for both paint and input. This avoids a second cursor owner and stale
coordinates on entry/resume. Each font draw retains its scissor.

Independent early criticism found missing button sound, conflicting OS/software
cursor visibility, and legacy egui input accumulating behind the native shell.
The candidate adds the cue, selects cursor visibility at presentation, and drains
inputs owned byBBB before another dialog can consume them. Independent re-review
accepted these corrections; production checks below exercise the affected routes.

## Validation and annotation state

Native geometry generation/default replay and syntax check passed. Expanded
trackbar regeneration/default replay passed, preserving all16 old fixtures exactly.
Its new payload SHA256 is
`6124143005a4a8f79f65fab82a2ba428485b744e0380e83edb16b48c4cb6f21e`. The first focused Rust build embedded the old
trackbar fixture while generation was in progress:26 tests passed and the one
new192-fixture lookup failed. The pre-palette full suite passed8712 tests, with122 ignored, in29.98s.
After the palette correction, the final full `cargo test -p vera20k --lib` passed
8712 tests with123 ignored in33.72s. Final `cargo clippy -p vera20k --lib` exited0
with1149 repository warnings. Logs are `.local/in-game-shell-full-tests-final.log`
and `.local/in-game-shell-clippy.log`.

The original-body findings for72FC60,72F540 and60B7A0 were independently confirmed
by the evidence worker and fresh critic. Three scoped Ghidra plate comments were
written, `gamemd.exe` explicitly saved, and all three read back exactly. Existing
function names were preserved; no executable bytes or types were changed.

The final release build exited0 (91 warnings). Its executable SHA256 is
`1ffd81fe49b219b4e40f3e88d6a328f005b7b2ddb69bd4f01f5b4851907dbff2`.
All following runtime checks used that executable in the owned
`.local/seed-browser-runtime` fixture; retail installation profiles were not edited.

The two additional ignored retail tests passed: `retail_in_game_shell_art_preserves_side_route_and_frame_transparency`
and `retail_modal_backgrounds_use_ui_palette_for_every_theme_and_size`.
They ran directly from the final library test executable while another owner was
compiling in its own target. Test executable SHA256:
`a7d0bfa0810ed1bcd3302c2f0cfd5171c5177067e99c610ad2408b52da915670`.
Actual output and identity are preserved in `.local/in-game-shell-retail-tests.log`.

Production mouse journeys covered Allied800x600, Soviet640x480 and Yuri1024x768.
Each entered through Main Menu → Single Player → Skirmish → match → pause →
Game Controls, rendered its side art and returned to a paused parent, then resumed
normal battlefield rendering. The fixture enabled battlefield upscaling; the
Allied and Yuri physical shell targets remained aligned with their painted controls.
Allied checks exercised both slider endpoints via rail clicks and thumb drags,
all three checkboxes, caption clicks that do not toggle icons, Back, reentry,
Escape, resumed camera position, and Abort returning to the main menu.
The profile recorded GameSpeed3, ScrollRate0, target lines/tooltips enabled and
hidden objects disabled. Reentry retained these settings; speed text reset to the
native template label until the next changed-position notification.
Yuri additionally changed the Game Speed rail and hidden-object icon before
Escape/resume; its profile recorded GameSpeed6 and ShowHidden=yes.
No queued menu click replay or second OS cursor was visible on these routes.

Saved production images include `bbb-allied-800-{before,entry,edited,reentry,parent-return,after}.png`,
`bbb-soviet-640-{entry,resumed}.png` and `bbb-yuri-1024-{entry,edited,resumed}.png`
under `.local/seed-browser-runtime`. These are production rendering/regression
checks, not a native full-frame pixel comparison. The three runtime cases cover
each theme and each size once; the native geometry oracle and retail background
regression separately cover all nine theme/size combinations.

The final executable's production Skirmish diagnostic passed its bundle validator.
Its frame SHA256 remained
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`,
identical to the previous merged launcher increment. The bundle is
`.local/in-game-shell-skirmish-capture` and receipt is
`.local/in-game-shell-skirmish-validation.log`. This is a neighboring Rust
regression check; its native parity certification remains NONE.

Fresh independent read-only criticism returned scoped PASS after inspecting the
original evidence, design, diff, corrected findings, final logs, all three themes
and return flows, release identity, diagnostic frame and saved Ghidra comments.
No blocking finding remained within this increment. The named follow-ups above
remain open and prevent closing the in-game family or whole-shell goal.
