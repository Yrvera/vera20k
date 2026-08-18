# AMCV open-ground Drive retrace — 2026-07-20

**Trace status:** PARTIAL — static Rust plus live read-only `gamemd.exe` evidence proves several first-order drifts, but no post-map-load native execution capture was available for the complete literal frame/arrival series.

## Exact fixture and assumptions

- Stock YR `[AMCV]`, normal/unveteran owner with no crate or house speed bonus.
- Start: flat clear Temperate cell `(40,40)`, center `(10368,10368)` leptons, ground Z, idle/from rest, body facing north `0x00`.
- Command: normal Move to exact center of `(45,40)`, `(11648,10368)`, with no units or blockers.
- Frame `F0` below means the first movement-processing frame after command acceptance. Rust samples are its 15 Hz Drive-budget passes; two intervening 45 Hz ticks still mutate its speed fraction.
- The older May trace was used only for navigation, never as current evidence.

## Stock inputs

Merged `rulesmd.ini` supplies `Speed=4`, `ROT=5`, Drive CLSID `{4A582741-9839-11d1-B709-00A024DDAFD1}`, and `MovementZone=Normal` (`ini/rulesmd.ini:6969-7000`). The section does not override acceleration keys, so the active defaults are `Accelerates=true`, `AccelerationFactor=0.03`, `DeaccelerationFactor=0.002`, and `SlowdownDistance=500` (`src/rules/object_type.rs:921-930`). `Speed=4` becomes `10` leptons/native frame and `150` leptons/second (`src/util/fixed_math.rs:370-378`).

## Live binary anchors (read-only)

- The retail `gamemd.exe` Drive vtable was identified from its Complete Object Locator and TypeDescriptor `.?AVDriveLocomotionClass@@`; vtable slot `+0x40` is `Process @ 0x004B0500`.
- Fresh decompiles of `Process @ 0x004B0500`, `Process_Movement @ 0x004B2630`, and `Process_Drive_Track @ 0x004B0F20` confirm the active standard-YR chain. A newly selected track is processed in the same native frame.
- `Process_Movement` writes the fresh track point cursor to **0**. `Process_Drive_Track` updates the speed fraction once per invocation, obtains an integer speed, adds residual, and consumes points only while `budget > 7`.
- TurnTrack entry 2 bytes at `0x007E7B40` decode to normal track `4`, short track `9`, target facing `64`, flags `8`.
- RawTrack 4 metadata at `0x007E7A68` decodes to point pointer `0x007E6790`, chain index `26`, metadata entry index `11`, and cell-cross index `19`. The first point is `(-256,245,facing 0)`.
- These are body/caller/table-byte findings, not trust in local Ghidra labels.

## End-to-end stage verdicts

| Stage | Current Rust for this fixture | Active `gamemd.exe` | Verdict |
|---|---|---|---|
| Rules and locomotor selection | Reads `Speed=4`, `ROT=5`, Drive, Normal and defaults above. | Same stock merged values. | PASS |
| Command/mission dispatch | Assigns `MissionType::Move`, clears attack/order/dock intent, then issues ground move (`world_commands.rs:149-170,255-280`). | Exact command-to-mission writes were not runtime-captured for this fixture. | UNCHECKED |
| Destination ownership | Sets owner `nav_com=(45,40)` and Drive destination/head-to to the target center (`navcom.rs:60-74,128-134`). | Separate owner NavCom plus Drive destination/head-to is the verified native mechanism. | PASS |
| Path cells | Static empty-grid expectation is `(40,40)..(45,40)` with five east steps; Rust A* was not executed here. | Literal native path queue was not runtime-captured. | UNCHECKED |
| First track choice | Facing pair `0 -> 64` selects TurnTrack 2 / RawTrack 4 / flags 8 / target 64 (`movement_commands.rs:495-520,584-591`). | Exact same table entry. | PASS |
| Fresh track cursor | `begin_drive_track` initializes from RawTrack `entry_index=11` (`drive_track.rs:3649-3657`). | `Process_Movement` explicitly initializes point index to `0`; metadata 11 is not the fresh cursor. | **FAIL** |
| Initial 90-degree turn | Curve owns facing immediately; no separate in-place turn. First Rust budget pass applies point 11 even with zero fresh budget. | Curve also owns the turn, but begins at point 0 and a zero budget leaves the centered north-facing unit unchanged. | **FAIL** |
| Acceleration cadence | Fraction mutates on every 45 Hz movement tick (`movement_tick.rs:1177-1212`), although point budget is gated to 15 Hz (`movement_step.rs:42-69`). | Fraction mutates once per native 15 Hz `Process_Drive_Track`. | **FAIL** |
| Point/residual progression | Costs 7 and uses strict `>7`, but increments before reading a point and detects cell crossing from transformed coordinates (`drive_track.rs:3741-3779`). | Processes from cursor 0 and uses RawTrack's verified index events; RawTrack 4 cell-cross index is 19. | **FAIL** |
| Braking | Within `<500`, subtracts `10*0.002=0.02` every 45 Hz tick, floor `0.3` (`drive_locomotion.rs:115-133`). | Same threshold/decrement/floor, once per native frame. | **FAIL** |
| Arrival ownership clear | Track finish defers NavCom clear, but the pending flag is consumed on the next 45 Hz tick (`movement_tick.rs:457-465,1898-1918`; `navcom.rs:94-106`). | NavCom clears on the next no-active-track native `Process`, i.e. the next 15 Hz frame. | **FAIL** |
| Render handoff | Renderer consumes sim `screen_x/screen_y` directly and uses `entity.facing` for the AMCV voxel key (`app_instances/units.rs:188-197,256-289`). | Native render sees native coordinate/facing state. | **FAIL** |
| Complete braking/arrival frame number | Rust outcome is necessarily earlier because of cadence drift; exact dirty-tree arrival frame was not executed. | Literal native positions and final arrival frame need a controlled runtime capture. | UNCHECKED |

**Tally: PASS 3 / FAIL 7 / UNCHECKED 3 / NOT-IMPLEMENTED 0.** A sampled trace cannot certify parity even for PASS rows.

## First DriveTrack point and early relative frames

Rust RawTrack 4 point 11 is `(-236,127,facing 5)`. For an east head-to offset `(384,128)`, it transforms to subcell `(148,255)`. Thus on Rust's first budget pass, even with fresh budget/residual `0/0`, it moves from `(128,128)` to `(148,255)` and changes body facing `0 -> 5`. At flat `(40,40)`, `lepton_to_screen` changes from `(0,1215)` to approximately `(-12.539,1223.613)` pixels (`src/util/lepton.rs:123-144`). Native remains centered/north on that zero-budget pass.

| Native-frame sample | Native speed fraction / position | Rust speed fraction / position | Result |
|---:|---|---|---|
| F0 | `0.03`, `(128,128)`, facing `0`, residual `0` | `0.03`, `(148,255)`, facing `5`, residual `0` | FAIL |
| F1 | `0.06`; centered while integer fresh budget remains 0 | `~0.12`, `(148,254)`, facing `5`, residual `1` | FAIL |
| F2 | `0.09`; centered while integer fresh budget remains 0 | `~0.21`, `(149,251)`, facing `5`, residual `3` | FAIL |
| F3 | `0.12`; first nonzero native residual/point interpolation is execution-dependent here | `~0.30`, `(150,248)`, facing `5`, residual `5` | FAIL / native literal UNCHECKED |
| F4 | literal native subcell/facing requires runtime capture | `~0.39`, point `12`, `(152,244)`, facing `8`, residual `1` | FAIL / native literal UNCHECKED |

At native-frame samples, Rust's nominal fractions are `.03,.12,.21,.30,...,.93,1.0` (full at F11); native is `.03,.06,.09,.12,...,.99,1.0` (full at F33). The complete F3-to-arrival native coordinate/facing/residual table is deliberately **UNCHECKED**, because `FootClass::GetCurrentSpeed` includes live owner modifiers and no controlled `gamemd.exe` run was captured.

## Top five root findings

1. **Wrong curve entry is the first visible break:** Rust treats RawTrack metadata `entry_index=11` as the initial cursor; native writes cursor `0`.
2. **Acceleration state runs three times too often:** Rust reaches full fraction at relative F11 instead of native F33.
3. **Cell transition authority is different:** Rust derives a boundary from coordinates; native RawTrack 4 carries the relevant index `19`, changing curve/cell timing and subsequent chaining.
4. **Braking also runs three times too often:** the correct `0.02` decrement is applied three times per native frame in Rust.
5. **The drift is presented directly:** sim subcell/facing feed the renderer with no compensating interpolation, while NavCom's deferred clear lasts only one 45 Hz tick.

## Smallest decisive follow-up

Capture a controlled native run for this fixture at the `0x004B0F20` entry/exit, recording owner coords/facing, Drive point index, current fraction, fresh integer speed, residual, track index, and NavCom each frame through arrival. Run the equivalent dirty Rust fixture with the same fields. This closes the remaining UNCHECKED literal series; it is not needed to establish the seven FAIL rows above.
