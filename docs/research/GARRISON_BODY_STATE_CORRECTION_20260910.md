# Garrison body state correction — 2026-09-10

The old reports below incorrectly treated BuildingClass+0x534 as a health-derived
damage flag. It is the building body animation state. Healthy completed occupied
buildings select body frame 2, not 0. This explains the unchanged bunker in the
user recording (2026-09-10 18:49, approximately 10–16 seconds).

## Native evidence

Saved Ghidra project `testProsjekt`, `/gamemd.exe`, x86:LE:32:default, image base
0x00400000. Executable SHA256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`
is checked by the executable oracle loader.

- Constructor 0x0043B740 initializes +0x534 to -1.
- GrandOpening 0x00447780 stages its argument in +0x538 and writes +0x534
  at 0x004477C1 when uninitialized, starting state 0, or in map-editor mode.
  The same state indexes the type's animation triples at +0xF04, stride 12.
- Active Guard handler pushes **1** at 0x0044995D and calls GrandOpening at
  0x00449961. Construction starts with **0** at 0x00449B36 and requests **1**
  on completion at 0x00449AC9. These are animation states, not health tiers.
- GetCurrentFrame 0x0043EF90 checks +0x534 at 0x0043EFC6; nonzero enters the
  CanBeOccupied branch at 0x0043F02C. Occupants select 2; damage then adds 1
  at ConditionRed, or ConditionYellow for TechLevel > 0. Civilian TechLevel
  -1 collapses 3 to 1. DrawBody calls it at 0x0043D7B5/0x0043D7CB/0x0043D8D4.
- SetDamagedState 0x00451EE0 operates a separate IsDamaged flag and animation
  slot variants; it does not define +0x534.

Rust already handles build-up and build-down before completed body selection.
The fix removes only the false HP gate. Keep the health-derived overlay flag;
it has separate consumers. The atlas already requests body frames 0..3 for
CanBeOccupied structures. No new animation or material asset is needed.

## Reproducible comparison and limits

`python -m tools.garrison_oracle.body_frame --check` executes the original
GetCurrentFrame, original BuildingClass vtable and original health/type/occupant
accessors for 54 idle-state cases. See its JSON and provenance sidecar. Inputs
cover empty/occupied, TechLevel -1/0/5, and health around both stock thresholds
at strength 2000 (CABUNK01 retail strength). The Rust production selector is
checked by `completed_garrison_body_frames_match_native_oracle`.

This comparison supplies initialized fixture objects and rules; it does not
execute the full boarding lifecycle or certify final GPU pixels. No general
BState scheduler or all-input floating-point parity is claimed.

Supersedes the healthy-frame-zero conclusions in
[GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md](GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md)
and [GARRISON_VISUAL_OCCUPANTANIM_RESWARM_20260527.md](GARRISON_VISUAL_OCCUPANTANIM_RESWARM_20260527.md).
Their separate shot-flash and live animation-slot findings are not re-established here.

## Validation on Apple M4 / Metal

- Native oracle `--check`: passed, 54 cases; focused Rust comparison: passed.
- Full `cargo test -p vera20k --lib`: 8587 passed, 7 failed, 119 ignored.
  Four untouched capture tests reject macOS `/var` symlinks before staging;
  three untouched tests assume Windows path semantics on Unix. Independent
  read-only review traced these failures and found no garrison dependency.
- `cargo clippy -p vera20k --lib` and release build: passed with existing warnings.
- Production release visual smoke: isolated `Garrison Check.app`, CABUNK01 and
  an Allied GI on a temporary authored map. Occupied bunker visibly showed
  reinforced artwork and a blue ownership band. Selecting it and pressing D
  ejected the GI and restored the plain concrete empty artwork. The user also
  confirmed the fix visually. This is a Rust runtime check, not a native GPU
  screenshot comparison. Original gameplay and retail files were not replaced.
- Fixture setup residuals: the first minimal map without a player building
  ended immediately; adding the player's construction yard kept it active.
  An initial 640x480 startup hit an unrelated scissor-rectangle validation
  error; the isolated profile used 1024x768 for the successful smoke.
