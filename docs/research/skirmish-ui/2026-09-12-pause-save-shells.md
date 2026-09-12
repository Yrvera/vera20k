# Pause and saved-game shells

This increment replaces the ordinary offline B5 pause menu and its Load B7,
Save 2B4 and Delete 2B5 children with the active retail shell composition.
Acceptance and original instruction/resource evidence are in the
[evidence report](2026-09-12-pause-save-shells-evidence.md). The checks below are
bounded to this increment; they do not certify the whole in-game family.

## Behavior and ownership

The parent exposes Game Controls, Load, Save, Delete, Abort and Resume.
Load/Delete availability and child enumeration use the same repository header
admission. Each child replaces its hidden parent, consumes its input, and keeps
the simulation paused. Back returns to B5; successful Load closes the chain.
B5 and the base browser ignore Escape, as their native modeless callbacks do.
Prompts consume Escape/initial Enter without sending them to the hidden parent.

Save starts with localized `GUI:SkirmishGame`. Description and file identity
remain separate. Blank descriptions produce the native warning; existing rows
require overwrite confirmation. Save failures stay in the browser with an error;
successful saves acknowledge before returning. Delete refreshes parent
availability and returns when the last row is removed. Load uses PreparedLoad's
existing validation and candidate-before-commit authority; errors leave the live
match intact and display the native error message.

`ui/shell/saved_file_input.rs` shares the established saved-seed input mechanism
with the game adapter. The generic browser carries opaque file identities, while
game admission accepts compatible empty descriptions and seed admission retains
its different native invalidation rule. Shared text/list/prompt rendering keeps
the two consumers aligned without sharing their persistence operations.
`InGameShellFrame` centralizes the themed background, controls and text passes
used by B5, Game Controls and the browsers. Physical geometry drives painting and
hit testing; the battlefield upscaler does not resize the shell.

Persistence returns explicit results to the shell. Explicit overwrite stages a
complete snapshot before replacing the selected file; failed publication leaves
the original and bookkeeping intact. New writes reject filename collisions.
These are VERA backend protections, not claims about retail SAV serialization.

## Review corrections

Independent review corrected differing parent/child admission, focus-loss gesture
cleanup, the localized New description and an apparent row marker. Native code
attempts to set a star at column x200, but the list only registers x2/255/315;
the setter rejects it. The renderer therefore paints no star. The unnecessary
snapshot-prefix decoder introduced for that interpretation was removed.
The successful-save copy targets temporary scratch, not the persistent default.

## Validation

- Final-source library suite: **8,726 passed, 123 ignored**, 19.15 seconds;
  local receipt `.local/pause-save-full-tests.log`.
- Library Clippy: exit 0, 1,149 warnings, 41.18 seconds;
  `.local/pause-save-clippy.log`.
- Native placement fixture: all 21 ordinary controls across the three resources
  and three stock resolutions; independently replayed. Payload SHA-256
  `969f6d3b485487f9c3e09890304057d60016dbc3f6ca4cf0d472c3c27b9dcac2`.
  Rust `ordinary_controls_match_original_resource_bytes_and_native_placement`
  consumes the preserved original bytes/results.
- Regression coverage includes opaque browser identity, empty-description
  admission and filesystem timestamps, prompt blocking, double-click ownership,
  scrollbar repeat/capture, explicit overwrite and failed publication.
- Release build: exit 0, 92 warnings, 2m16s; `.local/pause-save-release.log`.
  Executable SHA-256
  `3afb625cc434ddbe3d6f8e539a2e032db01558eff3f04a8b82635a006766b782`.
- The release's production Skirmish capture validates and retains frame SHA-256
  `fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`;
  `.local/pause-save-skirmish-validation.log`. This is a Rust neighboring-frame
  regression check; the capture receipt expressly grants no parity certification.
- Read-only native replays `saved_game_layout`, `seed_order`, and
  `saved_scrollbar` passed against the pinned binary. Run as Python modules with
  `VERA20K_GAMEMD_EXE` pointing to that binary.
- Ghidra plates `004F10E0`, `00558DD0`, and `005596A0` independently confirmed,
  saved, and read back exactly. Existing names were preserved; the old blanket
  invalid-description wording was corrected to the actual metadata flag.
- The same release passed the Allied 800x600 ordinary journey: empty-parent
  availability, base Escape, Save New and acknowledgment, overwrite cancel/yes,
  Load error/back/resume and successful restore, Delete cancel and last-row
  return, Save error and retained browser, and Game Controls return. Overwrite
  cancellation preserved SHA-256
  `07e3adef71050c437a9340c5423e0f38ba51031f2b886dc606520c402dec99c3`;
  confirmation replaced the same file with
  `6dd6ee6798ffe915c87f5141d15955847c86b22fdcbf61126b6702f595fbe99d`.
  Error tests used only owned fixtures: truncated snapshot bytes and a temporary
  file obstructing the save-directory path, both restored afterward.
- Soviet 640x480 B5 and all three browser layouts passed physical interaction
  and parent return checks. Stationary entry and movement checks leave no
  second cursor painted in the background. The captured input indicator does
  not certify full native cursor timing or match-start pointer retention.
- Saved-map neighbor: scrollbar arrow movement, double-click Load with generated
  preview, existing-row-to-editor selection, overwrite prompt and Escape cancel
  passed after the shared extraction. Wheel input remained a no-op.
- Yuri 1024x768 B5 and all three browser layouts passed physical interaction
  and parent return checks with the same release. All owned validation games
  were closed after capture.
- Captures are retained locally under `.local/seed-browser-runtime/pause-*.png`.
  Fresh independent read-only critic `pause_save_critic` returned **scoped PASS**
  after rechecking original evidence, design, final diff, receipts, captures and
  Ghidra readback; no actionable finding remains for this increment.
  The full suite covers shared editor
  and scrolling regressions; live checks are the named sample above, not every
  acceptance input combination at every size.

## Remaining scope

Cross-map and Single Player Load need the map/resource startup path; same-map
restoration cannot certify them. VERA retains its existing snapshot format and
New filename policy, not native `SAVE%04lX.SAV` compatibility. Saving/loading is
synchronous; retail progress panels remain open. Button timers, complete keyboard
focus traversal, full native frame comparisons, Abort's native child and Game
Controls' Keyboard/Sound children also remain open. These are bounded omissions,
not a whole-family pass. The [whole-shell acceptance inventory](../../plans/2026-09-12-retail-shells-acceptance.md)
continues to govern completion.
