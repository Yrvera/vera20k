# Frontend and match resolution lifecycle

Ordinary configured game resolution must not size the frontend. The saved-map
runtime check exposed overlapping Skirmish/Choose Map controls when Rust applied
`ScreenWidth=640`, `ScreenHeight=480` at startup. Retail normally runs those
screens at 800x600 and uses the configured pair after reading a scenario.

## Evidence and acceptance

Original `gamemd.exe` SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`:

- `OptionsClass::SetDefaults` `0x005FA3D0/0x005FA3D7` sets the independent
  frontend pair `+0x2C/+0x30` to 800/600. Global initializer `0x004E7E20`
  supplies `A8EB60`, giving shell pair `A8EB8C/90`; game pair is `A8EB84/88`.
- WinMain reads `Video/AllowModeToggle` with default false at
  `0x006BC0E6..0x006BC0FC`. False takes `0x006BDAF9`, using the shell pair for
  window creation and video setup. True takes the distinct 640x480 startup
  branch `0x006BD9C2`. Display failure also has a fallback branch.
- Skirmish `0x006AE2C0` and chooser `0x005E68A0` inherit the live frontend
  dimensions through common dialog hosting `0x0060C4A0`; neither promotes a
  configured 640x480 game to 800x600 on entry.
- `Start_Scenario` calls `Read_Scenario` at `0x00683D21`, then compares the
  game pair and changes mode at `0x00683DBB..0x00683DF3`. Loading resources,
  progress and their cleanup precede that return. Changing at loading start
  would incorrectly change loading artwork sizing.
- Victory restore `0x006857AE..0x006857E5` reads the shell pair without an
  AllowModeToggle guard. Its exact predicate requires both dimensions to differ;
  this observation must not be generalized to every exit caller. Ordinary
  640x480 and 1024x768 matches return to 800x600.

An independent read-only evidence reviewer and the implementation owner checked
the original sizing branches. This establishes static behavior, not native
rendered parity.

Acceptance for this ordinary-flow increment:

1. Game preferences 640x480, 800x600 and 1024x768 keep Main Menu → Single Player
   → Skirmish → Choose Map at 800x600, with drawing and hit testing aligned.
2. Loading artwork keeps frontend size; successful installation changes to the
   configured game size before camera/cursor/shroud setup. Native loading failure
   does not request game mode; the existing generic fallback instead installs
   its fallback map and enters game mode. Score entry and match return restore
   frontend size.
3. Launcher resolution edits retain the new game preference without resizing
   the frontend, and the next successful match installation consumes it.
4. Gameplay upscaling cannot change frontend layout or pointer mapping.
5. Sealed capture dimensions override both frontend and match transitions.

## Implementation boundary

`RetailOptionsLoad` carries an explicitly named startup shell size alongside the
retained profile. Existing two-pass game preference fallback/readback remains
intact. `PlatformState` retains the frontend return target and an optional sealed
capture size, while `PersistenceState.options_profile` remains the sole mutable
game preference. Common successful `apply_map_load_result` enters game mode;
individual Skirmish launch handlers no longer own that transition.

Window mode application reads back the actual client size even when winit
returns `None`: the installed winit 0.30.12 Windows implementation calls
SetWindowPos and returns `None`. Surface/depth sizing must not wait for the later
queued resize notification before tactical resource setup. Runtime checks must
confirm this handoff on the supported Windows target.

Frontend/result projections use window pixels. Loading retains tactical
projection for pending camera and shroud installation; loading artwork explicitly
uses GPU/window dimensions. This preserves the existing gameplay upscaler while
removing its accidental influence over frontend drawing.

## Validation state

The final library suite passed: **8,695 passed, 0 failed, 121 ignored** (18.73 s).
Clippy completed successfully with 1,150 repository warnings. Regressions cover
ordinary configured pairs, retained launcher preference, explicit mode-toggle
startup, frontend/upscaler separation, actual resize readback and capture startup
isolation. Fresh independent criticism found missing score-entry restoration;
the correction is included in this final candidate and was independently
rechecked against the original score caller.

Release SHA-256
`ef570d036d1db4e9b9ef3c828f2f24db6c302334e40ddf58d524b39d8bfcb276`
passed isolated production checks: a 640x480 game preference retained 800x600
frontend/chooser/loading, then applied 640x480 before tactical installation and
returned to 800x600 through Abort Mission. A 1024x768 preference with a 640x480
gameplay upscaler source retained frontend/chooser/loading at 800x600, applied
1024x768 before installation, accepted pointer selection and deployment, then
returned to 800x600 for score entry and Continue. Captures and surface logs were
retained locally. The ordinary AutoLocalStart route used its existing compatibility
bootstrap; these observations do not certify that bootstrap's native semantics.

The sealed 800x600 Skirmish capture passed its artifact validator and retained
the prior `fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`
frame hash. This is a Rust capture nonregression, not a native frame comparison.
An initial missing-data loading failure also retained frontend size; adding the
verified retail executable data to the isolated fixture and restarting enabled
the successful match checks.

Launcher options opened/returned without resizing the frontend, but its existing
clipped layout prevented manual resolution selection. The profile/launcher
storage boundary is source- and regression-tested; complete launcher UI acceptance
remains open. Runtime also exposed existing RA2MD.INI write failures from a
retained file mapping, and the pause menu's developer overlay. These ordinary
findings belong to the remaining shell/persistence work, not a resolved-parity
claim for this increment.

Fresh independent read-only criticism passed the final evidence, design, diff and
validation with the stated scope limits. Independently confirmed comments were
appended at `0x005FA350`, `0x006BB9A0`, `0x00683AB0` and `0x00685670`, preserving
existing comments and labels; the program was saved and all four readbacks matched.
This increment does not close all-shell acceptance. Explicit mode-toggle runtime, fullscreen/video
failure, unusual one-axis exit predicates and platform backends other than
Windows remain separately bounded coverage.
