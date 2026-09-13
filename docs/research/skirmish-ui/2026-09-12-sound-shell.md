# In-game Sound shell

Date: 2026-09-12. Status: final validation and fresh independent scoped review passed. The whole-shell goal remains open.

The active B8 Sound child now opens from Game Controls with the retail full-screen
themed frame, three numeric volume sliders, localized music list and durations,
Play/Stop, Shuffle/Repeat and Back. The parent applies and persists its controls
before entry. Sound changes use the existing Options and Theme owners; Back
reapplies slider positions without preview cues and reconstructs the parent.

## Evidence and design

The [native evidence packet](2026-09-12-sound-shell-evidence.md) records active
callers, raw B8 resource, original instruction addresses, asset identities and
bounded comparisons. Ghidra names were leads; original instructions and data
establish the claims. Enter and Escape remain in B8. Selecting or double-clicking
a music row does not invent Play. Play and Stop retain the Theme queue/fade owner,
including while simulation is paused. Shuffle and Repeat exclude each other when
checked, while both may remain unchecked.

The eight stock music labels use THEMEMD names and the native WAV duration
calculation. Retail IMA headers report an average byte rate different from the
rate recomputed by the original parser; using the header would give every stock
row a one- or two-second error. Shared borrowed WAV parsing now supplies both
metadata and the existing sample decoder without decoding music for labels.

Refactoring follows proven shared consumers: launcher and Sound use one volume
setter/preview dispatch; saved browsers and Sound share list geometry and backing
paint; playback retains one Theme authority. Sound state owns only projected
control values, row identities and interaction. Native current-song queries use
the retained track, falling back to the pending request, in both launcher and B8.

Fresh independent criticism corrected the old MNBTTN interpretation. Frame1 is
held; frame2 is timer highlighting. One shared binding now covers Sound, RMG,
validation, saved-file prompts and main-menu Quit. Historical reports explicitly
identify the superseded interpretation. Independently approved Ghidra plates at
6B6230,408560,55FAA0,612B70,609E20 and61D950 were saved and independently read back;
existing names and raw callback6B6300's analysis boundary were preserved.

## Validation

Final library suite: **8,739 passed, 123 ignored**,20.94s,
`.local/sound-shell-full-tests-final3.log`. Clippy for this revised candidate passed with1,149 warnings,25.94s
(`.local/sound-shell-clippy-final.log`). Earlier test logs are not final
receipts: the first needed two test-call updates, and one later run embedded a
stale slider fixture while native regeneration was still running. The final run
started after generation and all source fixes finished.

Independent read-only native replays passed:

- `sound_shell_layout`: raw B8 resource and33 original ordinary placements.
- `sound_theme_metadata`:24 WAV metadata inputs and four paired caller queries.
- `owner_button_frame`: all four supplied pressed/highlight combinations.
- `launcher_trackbar`:18 geometries, including B8 width263/reserve50/max10;
  the prior seventeen geometry cases are unchanged.

The first live screen exposed a separate rail-cache defect: wide Sound and RMG
controls reused narrow Skirmish border art. The shared renderer now selects actual
numeric128/225/263 and plain180/192 geometry through the existing border builder.
The critic independently confirmed all five current production sizes and caught
the initially omitted RMG225 case. Emission and divider-pixel tests protect this
rendering boundary; the full suite above includes both new regressions.

These establish bounded resource, geometry and instruction comparisons. They do
not certify full native frames, Windows focus or audio-device output. Production
runtime evidence is recorded below.

## Production validation

The final release build passed with92 warnings in4m00s
(`.local/sound-shell-release-final.log`). Its executable SHA256 is
`9c21a7db6caa4417147873cccf797d99a1e75f3c753c0e13ec27b327ad9bf4e5`.
The same executable was used for the following isolated retail-asset fixture runs.

Allied800x600 covered the full Sound journey: parent ToolTips change persisted
before entry; launcher Sound3 appeared correctly; all eight music names/durations
and full-width rails were visible. Double-clicking Drok selected it without Play;
Play subsequently submitted Drok to the physical music player at20:05:13Z. Stop
produced no further playback submission through the remaining checks. Sound
volume dragged3->10, Voice10->0 then a rail click selected5, Music10->0; Shuffle
then Repeat demonstrated mutual exclusion, and Repeat could be cleared to leave
neither. Enter and Escape remained in B8. Back retained values without an INI
write; the next parent acceptance persisted Score0/Sound1/Voice0.5, neither flag,
and InGameMusic=no. Repeated entry retained those values, and the full
B8->BBB->B5->Resume chain returned to the live match.

Soviet640x480 covered compact themed placement, retained values and Back to BBB.
Yuri1024x768 covered the larger themed placement, retained values and Back to BBB.
All three interactive game instances were closed after validation. Local captures are under
`.local/seed-browser-runtime`: `sound-allied-800-{entry,controls,retained,resumed}.png`
and `sound-soviet-640-{entry,parent}.png`, `sound-yuri-1024-{entry,parent}.png`.

Shared neighbor checks on that executable covered launcher volume input and
persistence, main-menu Quit/Cancel, RMG's225px Players rail input, and saved-seed
list scrolling/Back. Captures are `sound-final-{launcher,quit-prompt,rmg-rail,seed-scroll}.png`.
The independent critic inspected the Allied and neighbor captures without finding
a new defect. The final same-executable production Skirmish capture passed its
validator at `.local/sound-shell-skirmish-capture`; frame SHA256 remains
`fe7d42bfebaca25a608e1be3485a2f2f576beca75867ae4961a73a546c42f8fa`.
The receipt explicitly reports parity certification NONE. Fresh independent read-only criticism passed after examining original evidence,
architecture, final diff, all three themed captures, shared consumers and actual
validation. The critic independently reran the Skirmish validator and confirmed
the six saved Ghidra comments. No confirmed finding remains in this increment.

These are Rust production captures, not native frame goldens. Audio submission
logs establish player submission, not acoustically captured output or beep
audibility; ordered setter/preview tests cover the dispatch. Completed-gesture
captures do not certify intermediate held-button frames, which have separate
native frame comparisons and interaction tests.

## Remaining whole-scope work

The pre-existing Game Controls held-button visual stays pressed when dragged
outside until release; its action cancels correctly. The critic accepted this as
explicit common-input/Keyboard follow-up, required before whole-scope acceptance.
B5/B6/B8 already use the shared hover-aware predicate. Timer highlighting, full
transitions/focus, unavailable-device behavior, campaign gating and full native
frame comparison remain qualified by the evidence packet and the
[whole-shell acceptance inventory](../../plans/2026-09-12-retail-shells-acceptance.md).

Earlier executable35488cd177669b9574e8cbccf3bba391796e3fbab0a6cad728fea4545ca75d9a
and captures `sound-allied-800-rail-defect.png` / `sound-discovery-retained.png`
retain the known narrow-rail defect. They are discovery evidence only and are
superseded for final acceptance by the rebuilt executable and captures above.
