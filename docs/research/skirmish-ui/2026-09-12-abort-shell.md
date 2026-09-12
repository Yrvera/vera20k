# Ordinary Skirmish Abort shell

This increment replaces the small Abort card with retail's full-screen B6.
It preserves the existing queued `ExitMatch` command and direct Resume return.
Acceptance is the ordinary mode5 journey at 640x480, 800x600 and 1024x768:
the themed frame, Quit and Resume buttons, central question, hover footer,
Escape staying in B6, default Enter resuming, and Quit returning to the shell
through the existing event path. Campaign Restart and network Observe/Leave
branches remain required separate work, not certified here.

## Original evidence

All addresses refer to original `gamemd.exe`, SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.
Fresh independent evidence and criticism rechecked original instructions,
resource bytes and `langmd.mix/ra2md.csf`; labels were treated as leads.

State3 reaches `48CA1C` → `4F1840`, which creates modeless resource B6 using
callback `4F18B0`. The callback has no existing Ghidra function object; its
original bytes were decoded read-only without changing analysis boundaries.
B6 is resource VA `BEFBC0`, 312 bytes, five controls, 533x369 DLU, 8-point
MS Sans Serif. Resource SHA-256 is
`0c612d63f0a1e3af4a1d393a8895791e6dc3e7f2ac474bd5d6565325249fd7b6`.

| Control | DLU rectangle | Ordinary mode5 behavior |
| --- | --- | --- |
| 686 button | 425,346,108,23 | `GUI:ResumeMission` |
| FFFF static | 99,165,230,19 | `GUI:AskAbortMission`: What would you like to do? |
| 6C9 button | 431,122,108,23 | Init replaces Blank with `GUI:Leave`: Quit |
| 712 button | 431,149,108,23 | Hidden |
| 695 static | 2,353,303,12 | Initially blank; hover help |

There is no title694 or central dialog panel. `60C540` assigns B6 background
mode1; `621FB1..621FDA` invokes active `72F540` with the ordinary bottom-strip
argument false. It shares the existing `InGameShellFrame` composition.

`608CD0:609387..6093AA` enrolls 6C9 and 712 in the upper action placement
(`524` is also recognized but absent from the resource). Quit uses row0 of
`60B000`: W−147, SIDE2.y, loaded button width/height. Resume uses `60B350`:
W−147, SIDE3.y−button height. Footer695 uses `60B550`: 10,H−21,455,20.
The anonymous question has no special exception in `608500`, `608CD0` or
`60B950`; `60C3D1..60C3F8` selects ordinary `60B7A0`. Native execution yields:

| Size | Question rectangle in physical pixels |
| --- | --- |
| 640x480 | 69,208,345,31 |
| 800x600 | 149,268,345,31 |
| 1024x768 | 261,352,345,31 |

`602490` does not enroll the question in animated effects. Plain static paint
`615A81..615AE8` selects flags0x11 from style low bits1: horizontal center and
top alignment. It does not apply the resource's SS_CENTERIMAGE flag.

`603E40` writes button help context at record+C0, not sprite state.
`6040B0:60498D..604AC0` selects `STT:ConfirmExitButtonLeave` (Abort and leave
the game.) or `STT:ConfirmExitButtonResume` (Resume mission.). Common `622B50`
hover dispatch localizes the label and updates695; empty space clears it.

## Input and exit authority

`4F1A40..4F1A90` accepts 686 and IDOK1 as result1, resuming directly at state0.
It ignores IDCANCEL2; Escape stays in B6. Quit6C9 uses result2 in mode5.
The result table at `48CCA8` reaches `48CAA8` → `6471A0`, queueing EXIT0x13.
Event execution `4C7903..4C7917` sets the graceful-exit flag at A83D48.
The Rust shell therefore delegates through `apply_in_game_modal_outcome` to
the existing command scheduler; it never directly tears down the match.

The buttons have style0x5000000B and no WS_TABSTOP. `60F9A0` installs wrapper
`610CA0` and owner-button procedure `612B70`. On WM_SETFOCUS,
`6118BF..6118DE` recognizes that procedure and restores previous focus through
SetFocus. This original wrapper contradicts an ordinary focused-button/Tab
model. Default Enter maps to IDOK/Resume; no speculative focused-button Space
or Tab behavior was added. Root and independent critic both checked the bytes.

The shared `ShellButtonInteraction` retains press/release capture for B5 and
B6, while each adapter owns hit geometry, eligibility and actions. Transition
and focus-loss cleanup reset the interaction. The native frame/input branch
isolates the shell from battlefield controls and the developer overlay.

Independent review found that capture identity alone left B5/B6 looking pressed
after dragging off a held button. The shared helper now requires both capture
and hover for the visual pressed state, retaining capture for a return drag.
Original `612B70` delegates mouse movement to the Windows button procedure;
its pushed state follows the cursor's presence inside the button. See
[Microsoft's button default processing](https://learn.microsoft.com/en-us/windows/win32/controls/button-messages#button-default-message-processing).

## Validation and limits

The preserved `tools/storage_oracle/abort_shell_layout.py` executes complete
original `60B7A0` for the three question rectangles and retains all B6 resource
bytes. It supplies 6x13 DLU metrics and bounded Win32 geometry hooks; it does
not emulate Windows font selection, painting, focus or EXIT scheduling.
Rust's `question_matches_original_resource_and_executed_placement` consumes
the saved native data. Native dispatcher enrollment is instruction evidence,
separate from the executed placement body.

Final-candidate library tests passed: **8,729 passed, 123 ignored**, 20.12 seconds
(`.local/abort-shell-full-tests-final.log`). The earlier pre-correction run is
not the final receipt. Native placement replay passed after the B6 case metadata
correction. Independently confirmed Ghidra plate `004F1840` was saved and read
back exactly, preserving its existing name and the callback's analysis boundary.

Library Clippy passed (1,149 warnings, 40.00s), and the release build passed
(92 warnings, 2m29s). Receipts are `.local/abort-shell-clippy.log` and
`.local/abort-shell-release.log`. The release executable SHA-256 is
`5b56b7c699efc1d5bf1ef7f0322de88693badf08a09886a5ea35890a747478b2`.
That same executable passed live ordinary-flow checks in an isolated retail-asset
fixture. Yuri at 1024x768 covered B5 Abort press dragged outside, B6 Quit press
dragged outside, Escape remaining in B6, default Enter and mouse Resume returning
directly to the running match, repeated entry, both localized hover messages and
the empty footer, and Quit returning to the 800x600 main menu without a score
screen. Allied at 800x600 covered themed placement and mouse Resume; Soviet at
640x480 covered compact placement and Quit returning to the main menu. The
shared held-button visual is regression-tested; the live drag tool reports the
completed gesture, so these captures do not certify its intermediate pressed frame.

Local captures are `.local/seed-browser-runtime/abort-yuri-1024-{entry,quit-help,
resume-help,resumed,exited}.png`, `abort-allied-800-entry.png` and
`abort-soviet-640-{entry,exited}.png`. They are Rust production output, not native
frame goldens. The game instances were closed after validation.
The immediate Quit-hover drag capture also contains the automation pointer at
the destination and a native pointer at an earlier drag sample. The following
Resume-hover capture has the updated single pointer. This transient capture
does not certify settled cursor composition; source review found one native
cursor pass and an explicitly hidden OS cursor.

The final same-executable production Skirmish capture passed its validator at
`.local/abort-shell-skirmish-capture`; frame SHA-256 remains
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`.
Its receipt explicitly reports parity certification NONE.
Fresh independent read-only criticism passed the ordinary Skirmish B6 scope
after inspecting original evidence, design, final diff, native fixture replay,
tests, Clippy, executable identity and three-theme captures. Escape dismissal
and drag-out pressed visuals were corrected; no confirmed ordinary B6 finding
remains. The independent reviewer also confirmed the saved Ghidra plate.
This is not a whole-frame
native parity claim. Button timers, complete shell transitions, match-start
pointer retention and other families remain open under the
[whole-shell acceptance inventory](../../plans/2026-09-12-retail-shells-acceptance.md).
