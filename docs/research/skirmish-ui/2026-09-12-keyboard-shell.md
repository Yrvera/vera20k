# Shared Keyboard options A3 — acceptance and integration

Status: implementation and final independent read-only review passed for this increment. This does not close
all-shell acceptance. Branch feature/retail-keyboard-shell begins at
c91855df1bba9fc6e0219363edd20775662fdb02 (Sound PR357).

## Required ordinary journeys

- Main Menu → Options D5 → Keyboard A3 → Back or Escape → reconstructed D5.
  Parent values apply before entry; child entry does not add a profile write.
- Paused Game Controls BBB → Keyboard A3 → Back or Escape → fresh BBB.
  Parent values apply and persist before the child opens; the match stays paused.
- All87 registered commands and their localized categories, names, descriptions
  and parameter substitutions appear. Categories/commands sort by displayed
  caption. No initial command is selected. Select a command to focus capture.
- Type a key/chord; inspect current shortcut and conflicting owner; Assign
  immediately updates the existing dispatch table. Ordinary collisions transfer
  the key without a confirmation. Modifier-policy errors leave bindings intact.
  Selecting a command clears capture; Assign empty removes the first shortcut.
- Same-category acceptance preserves selection/pending capture/error. A changed
  category clears command details/current-owner but retains capture/error. Reset
  forces a category0 rebuild. Programmatic capture clear does not emit EN_CHANGE;
  rejection text survives selection changes until a physical capture change.
- Back saves the current table. Escape reloads CURRENT disk, not an entry copy.
  Reset All immediately deletes loose KeyboardMD.ini and reloads archive defaults;
  cancelling afterward does not restore the deleted file. Repeated entries and a
  new/loaded match retain the process binding authority. Startup-only forced keys
  are not reintroduced by dialog reload.
- Render both parent families with their own original frame art, title timer,
  native-sized category face, list, description group, hotkey field, Assign/Reset,
  Back and footer. Exercise640/800/1024 game sizes and all three side themes.
  Capture/name/description content, dropdown precedence, held/released buttons,
  field focus and modal input blocking must match their established behavior.
- Assign HealthNav and CursorCheat and exercise their ordinary gameplay effects,
  then verify manual selection, TypeSelect, normal tooltip delay and disabled
  tooltips remain correct. Exact adjusted-cost feedback remains a required shared
  pricing residual; no full command parity claim includes that missing model.

## Design and evidence

The app keeps one mutable HotkeyBindings table used by startup, A3 and gameplay.
The existing MatchInputState field is initialized once; new-map/load installation
preserves it. Its historical owner name is not a reason to create duplicate
preferences. Catalog metadata comes from all87 original registered objects.
UI state contains selected rows/control state and localized projections only.

The live-file persistence adapter deliberately bypasses immutable startup loose
asset snapshots. AssetManager exposes its registered-archive fallback so Reset
cannot resurrect a cached deleted customization. Launcher continuation waits for
A3 close; BBB uses the existing apply/write transaction before entry. Neither
modal dispatches input to the world beneath it.

Affected common systems are reused: the frame compositor selects side or launcher
art and carries a popup layer;207px category art uses the existing native border
builder; BBB buttons use shared capture/hover state. HealthNav extends the existing
selection-mode authority and stable-ID lifecycle. CursorCheat refreshes the normal
tooltip owner, including an already visible tooltip, instead of adding an overlay.

Evidence:
- [Original A3 routes, catalog and input](2026-09-12-keyboard-shell-evidence.md).
- [Frame/control owners](2026-09-12-keyboard-render-evidence.md).
- [HealthNav and CursorCheat](2026-09-12-keyboard-command-evidence.md).
- tools/storage_oracle/keyboard_bindings.py:87 metadata rows/73 assignment cases.
- tools/storage_oracle/keyboard_shell_layout.py:19-child EX resource/48 ordinary
  placements at640×480,800×600,1024×768.
- tools/storage_oracle/keyboard_key_names.py:original formatter plus explicitly
  separate current-Windows control/keyboard-layout probes.

## Validation and review

First library check passed (91 warnings,1m02s). The third library suite passed
8764 tests with123 ignored in30.44s (`.local/keyboard-tests-third.log`); this
intermediate result predates the terminal-exit fix. The final suite passed
8765 tests,0 failed,123 ignored in32.69s (`.local/keyboard-tests-final.log`).
Clippy passed with1153 warnings in1m34s (`.local/keyboard-clippy-final.log`).
Release build passed with92 warnings in3m48s (`.local/keyboard-release-first.log`);
source was unchanged after that release build.
The tested runtime SHA256 is
`52dc5fc8832ea9d9c058127d33a86f48585a6074946654f02e71bfc1310c1704`.

Live launcher validation used800×600 game settings at Windows125% scaling;
the captured window images are642×511. Back saved ToggleAlliance=587 (Ctrl+K),
and reopening retained it. Capturing G displayed Guard as the conflicting owner;
Assign transferred G to Alliance. Escape and reopen restored saved Ctrl+K.
Reset All immediately deleted the loose keyboard file and restored Alliance=A;
Escape afterward did not resurrect the deleted customization. Captures are
`.local/seed-browser-runtime/keyboard-launcher-{entry,assigned,collision,cancel-restored}.png`.

The terminal-exit check changed D5 ToolTips from false to true before opening
A3. Returning from A3 retained the parent value without writing it to disk.
After Reset All, an unsaved Ctrl+K edit was pending when Alt+F4 closed A3 and
process35776. Final disk state had RA2MD ToolTips=yes and no loose keyboard file;
see `.local/seed-browser-runtime/keyboard-terminal-pending.png`. This establishes
the observed final state. Exact profile-write count is supported by source
sequencing, not a runtime write trace. The terminal fix received independent
approval and its5FBEF0 documentation append passed independent readback.

Live Allied800 validation changed BBB UnitActionLines=yes to no and confirmed
persistence before A3 entry (`.local/keyboard-live-game-entry.ini`). HealthNav
was assigned F10=121 and CursorCheat F11=122; Back returned through BBB and B5
to Resume. HealthNav selected an empty critical category, then visible healthy
units on the third tap (`keyboard-health-{critical,healthy}.png`). This is a
bounded live category-cycle check, not a full mixed-health selection proof.
CursorCheat displayed coordinate(55,79), refreshed to(58,79), displayed sidebar
RepairMode, and cleared when disabled (`keyboard-cursor-{coordinate,refreshed,sidebar,disabled}.png`).
After loading the fixture through B7, F11 still worked
(`keyboard-neighbor-load.png`, `keyboard-loaded-binding-retained.png`).

The Team list arrow advanced one row and thumb dragging exposed its last nine
rows (`keyboard-team-scroll.png`). HealthNav rejected Shift+K while retaining
F10 (`keyboard-modifier-rejected.png`). Neighboring Sound entry/Back was checked
(`keyboard-neighbor-sound.png`); its eight-song list fit, so this provides no
live Sound scrollbar proof. The shared500ms initial/25ms subsequent held-repeat
behavior has native-body, implementation and regression evidence; the live
driver cannot hold a button for an arbitrary duration, so arrow-click and drag
captures do not establish that timing.

Soviet640 and Yuri1024 entry and Back restored their parents
(`keyboard-soviet-640-{entry,parent}.png`, `keyboard-yuri-1024-{entry,parent}.png`).
At Windows125% scaling these captures are514×416 and821×646 respectively;
Allied800 captures are642×511. Allied process25668, Soviet13892 and Yuri9140
were closed. All capture filenames in these game checks are under
`.local/seed-browser-runtime/`. The independent critic viewed these captures
without a finding. They are bounded journey checks, not exact full-frame parity.

The same release completed the final ordinary Skirmish diagnostic capture in
`.local/keyboard-final-skirmish`; process19116 exited. The bundle validator passed
and frame SHA256 remained
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`, identical to the
prior Sound increment. This is a Rust neighbor regression check; the manifest
explicitly retains parity certification NONE and unenrolled native inputs.

The fresh read-only critic identified category retention, programmatic capture
notification distinctions, a paused modifier leak and missing held-arrow repeat.
The owner corrected category/modifier state and added regressions; shared list
repeat is now corrected for A3 and B8. The prior saved-file browser still needs
pointer re-hit-testing during held-arrow repeat in the whole-scope input audit.
The fresh independent read-only critic issued SCOPED PASS after inspecting the
original evidence, design, final diff, validation, production captures and saved
Ghidra annotations. No actionable defect remained within this increment.
Inherited required gameplay gaps include
VeterancyNav, PreviousObject, NextObject, CombatantSelect and PlanningMode; displaying
and binding those entries does not establish their execution. Whole-scope shell
validation and the final omission/regression audit remain open.

## Independently confirmed Ghidra documentation

After reading original bodies/callers, the fresh critic approved the concrete
annotations and independently read them back after saving gamemd.exe:
5FBEF0 KeyboardOptionsDialog__Show;533D20 Load_Hotkey_Bindings (name retained,
startup-only caller comment corrected);61EF70 HotkeyControl__FormatShortcutName;
733380 TacticalSelection__CycleHealthCategory;724200 TooltipManager__ProcessEvent;
A8F7D8 g_bCursorCoordinateTooltipsEnabled, byte. The event-owner name includes
button/timer duties rather than misleadingly calling it mouse-move-only. Raw
callback5FB320 and Execute537EF0 analysis boundaries were not repaired or extended.
These annotations document original semantics; they are not Rust parity claims.
