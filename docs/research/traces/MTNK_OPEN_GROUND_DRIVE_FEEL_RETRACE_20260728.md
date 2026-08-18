# MTNK open-ground Drive feel retrace — 2026-07-28

**Status:** PARTIAL — current Rust and active-binary static evidence prove the
first visible facing break and several later lifecycle/cadence differences. A
controlled native run was not available, so the full native coordinate series,
literal arrival frame, and mission-byte timeline remain UNCHECKED.

## Exact fixture

- Stock YR `[MTNK]`, normal/unveteran owner, no crate, house, formation, damage,
  slope, bridge, or terrain-speed modifier.
- Flat clear Temperate ground; no blockers.
- Start at cell center `(40,40)`: world `(10368,10368)` leptons, subcell
  `(128,128)`, ground Z, idle/from rest, body facing north `0x00`.
- Normal player Move to exact center `(45,40)`: world `(11648,10368)` leptons.
- `F0` means the first Rust movement-processing sub-tick after command
  acceptance. Rust simulation is 45 Hz; its Drive budget gate runs at 15 Hz.

## Evidence and active-YR identity

- Research-index was used first. The highest-value anchors were
  `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`,
  `DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`,
  `DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md`, and the
  2026-07-20 AMCV/MTNK retraces.
- The active Ghidra program is the retail 32-bit PE
  `<ra2-install>/gamemd.exe`, SHA-256
  `1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`
  (verified via `get_current_program_info(program="gamemd.exe")` and local
  `Get-FileHash`).
- Drive identity was rechecked from bytes, not labels: ILocomotion vtable
  `0x007E7EB0`; `[vtable-4] -> COL 0x007FFDE8`; `COL+0x0C ->
  TypeDescriptor 0x00820248`; TypeDescriptor bytes spell
  `.?AVDriveLocomotionClass@@`; vtable slot `+0x40 -> 0x004B0500`
  (verified via `read_memory(0x007E7EAC,132)`,
  `read_memory(0x007FFDE8,24)`, and `read_memory(0x00820248,48)`).
- Fresh read-only decompiles of Drive `Process @ 0x004B0500`,
  `Process_Drive_Track @ 0x004B0F20`, `Process_Movement @ 0x004B2630`, and
  owner current-speed helper `0x004DB1A0` confirm the active standard-YR call
  chain (verified via one `batch_decompile` call for those four addresses).
- TurnTrack selector 2 is `{normal=4, short=9, target=0x40, flags=8}`
  (verified via `read_memory(0x007E7B40,12)`). RawTrack 4 is
  `{points=0x007E6790, chain_cursor=26, restart_anchor=11,
  occupation_handoff_cursor=19}` (verified via
  `read_memory(0x007E7A28,80)`). Points 0–13 were re-read directly via
  `read_memory(0x007E6790,168)`.
- This path is active in stock YR: `[MTNK]` explicitly uses Drive CLSID
  `{4A582741-9839-11d1-B709-00A024DDAFD1}` and
  `Accelerates=false` (`ini/rulesmd.ini:6603-6644`). No dormant TS gate is
  involved.
- Ghidra was read-only. No Cargo command or build mutation was run.

## Stock inputs and concrete speed

`[MTNK]` supplies `Speed=7`, `ROT=5`, Drive, `MovementZone=Normal`, and
`Accelerates=false`. No `SpeedType=` override means Track in current Rust
(`object_type.rs:919-933,1066-1078`; `locomotor_type.rs:130-152`).

Both sides produce an unmodified current-speed budget of `17` per native frame:

```text
parsed type speed = floor(7 * 256 / 100) = 17
Rust speed        = 17 * 15 = 255 leptons/second
Rust 15 Hz budget = floor(255 / 15) = 17
gamemd inputs     = house bonus 1, slope cache 1, no veteran-speed ability,
                    current fraction 1, no half-speed flag => 17
```

Rust conversion is at `src/util/fixed_math.rs:370-378`. The gamemd composition
and order are in `FootClass::GetCurrentSpeed @ 0x004DB1A0`; the Drive false
branch assigns its target fraction before that call in
`Process_Drive_Track @ 0x004B0F20`.

## Pipeline

```text
player Move -> queue Move mission / clear old intent -> A* path
-> owner NavCom + Drive destination/head-to -> TurnTrack 2 / RawTrack 4
-> Drive target/current fraction 1 -> 17+residual point budget
-> track coordinates/facing -> cell handoff / next track
-> arrival cleanup / deferred NavCom clear -> sim screen coords + VXL facing
```

## Entry-point coverage for this fixture

The only in-scope trigger is `Command::Move` for the selected MTNK:
`src/sim/world/world_commands.rs:146-175,254-284` routes to
`issue_move_command_with_layered`, which attaches the path and Drive state at
`src/sim/movement/movement_commands.rs:524-631`. Queued NavCom continuation,
mission-script retasks, formation commands, depot/miner movement, and forced
tracks are separate triggers and were not generalized into this scenario.

## End-to-end stage verdicts

| # | Stage | Current Rust output | Active `gamemd.exe` output | Verdict |
|---:|---|---|---|---|
| 1 | Stock rules / locomotor | `Speed=7`, `ROT=5`, Track, Drive, Normal, `Accelerates=false`. | Same stock merged inputs; Drive class identity proven above. | PASS |
| 2 | Command / Move mission acceptance | Queues Move, clears attack/order/dock intent, then creates movement state (`world_commands.rs:146-175,254-284`). | Exact command-to-mission field writes and relative tick were not captured for this fixture. | UNCHECKED |
| 3 | Destination ownership | Owner `nav_com=(45,40)` plus Drive destination/head-to `(11648,10368,0)` (`movement_commands.rs:550-560`; `navcom.rs:60-74,128-134`). | Separate owner NavCom plus Drive destination/head-to is the verified live mechanism with the same target center. | PASS |
| 4 | Flat path cells | Static empty-grid expectation is `(40,40)..(45,40)` in five east steps. | Literal native path queue was not runtime-captured. | UNCHECKED |
| 5 | No-active-track startup phase | Selects and creates the initial track during command dispatch (`movement_commands.rs:545-631`). | No-track `Process @ 0x004B0500` runs `Process_Movement @ 0x004B2630`, then processes the new track in that locomotor frame. | **FAIL** |
| 6 | TurnTrack / RawTrack choice | Facing `0x00 -> 0x40` selects TurnTrack 2, RawTrack 4, target `0x40`, flags 8 (`movement_commands.rs:579-591`; `drive_track.rs:3455-3508`). | Exact same table row and raw selector from live bytes. | PASS |
| 7 | Fresh cursor | `begin_drive_track` stores Raw `+0x08` anchor `11`, then stepping pre-increments before reading (`drive_track.rs:3621-3664,3741-3754`). | Fresh normal startup writes cursor `0`; budget loop reads current point before tail increment (`Process_Movement @ 0x004B2630`; `Process_Drive_Track @ 0x004B0F20`). | **FAIL** |
| 8 | `Accelerates=false` fraction | Flat-clear target is `1`; false branch directly stores current fraction `1` (`drive_locomotion.rs:76-112`; `movement_tick.rs:1166-1233`). | Directly calls owner `SetSpeedFraction(Drive+0x50)`; flat-clear healthy target is `1` (`0x004B0F20`, `0x004B2630`). | PASS |
| 9 | Fresh budget / residual arithmetic | First native-rate pass: `17+0`; strict `>7`, two costs of 7, residual `3`; subsequent passes carry Drive-owned residual (`movement_step.rs:42-69,753-772`; `drive_track.rs:3741-3779`). | Same `17+0`, strict `>7`, two paid points, residual `3`; retry calls add zero fresh speed (`0x004B0F20`). | PASS |
| 10 | First visible point/facing | Starts at cursor 11, consumes points 12 and 13, interpolates toward 14: subcell `(158,233)`, facing `0x0C`. | Starts at cursor 0 and consumes points 0 and 1; both headings are `0x00`. Exact native coordinate after residual interpolation is UNCHECKED, but facing is numerically unequal. | **FAIL** |
| 11 | Cell handoff / remaining budget | Detects crossing from transformed coordinates, breaks the point loop, and ends this entity's tick (`drive_track.rs:3750-3766`; `movement_step.rs:779-788`; `movement_tick.rs:1377-1505`). | Raw `+0x0C=19` is an explicit occupation-handoff event inside the continuing native point state machine (`0x004B0F20`); no general “coordinate crossing ends the frame” rule exists. | **FAIL** |
| 12 | Curve completion / next east track | Finishes from synthetic `points_count-1`, snaps center, creates the next track, and retries it with residual only (`drive_track.rs:3781-3804`; `movement_step.rs:801-864`). | Native pays a `(0,0)` sentinel point before selector/cursor reset; completion and any same-frame continuation occur inside `0x004B0F20`. | **FAIL** |
| 13 | Arrival / NavCom clear | End-of-path clears movement/track and sets Idle, but defers owner NavCom; pending clear runs on the next 45 Hz movement call (`movement_tick.rs:1931-1968`; `navcom.rs:94-125`; `movement_tick.rs:434-466,981-989`). | The next no-active-track Drive `Process` performs the arrival/`FootClass::Stop_Moving` path on the native logic-frame cadence (`0x004B0500`). | **FAIL** |
| 14 | Mission handoff after NavCom null | Current mission host later observes null NavCom / stopped locomotion and schedules arrival behavior (`techno_ai.rs:1363-1407`). | Literal mission-current, queued mission, timer, RNG, and arrival-call sequence was not captured for this fixture. | UNCHECKED |
| 15 | Render-to-screen | Sim `screen_x/screen_y` and `entity.facing` feed the VXL renderer directly at full byte-facing resolution (`app_instances/units.rs:188-197,255-289`; `unit_atlas.rs:35-48,1024-1027`). Thus the wrong first curve state is visible. | Native render consumes the native coordinate and locomotor-updated facing; each consumed point updates facing in `0x004B0F20`. | **FAIL** |

**Tally: PASS: 5 | FAIL: 7 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0.**

## Literal Rust first visible sample

RawTrack 4 point 13 is `(-228,109,facing 12)`. With east head-to offset
`(384,128)`, its discrete position is `(156,237)`. Residual `3` interpolates
toward point 14 by:

```text
delta 13->14 = (6,-10)
truncate_toward_zero(delta * 3 / 7) = (2,-4)
Rust F0 subcell = (158,233), facing = 0x0C
```

At cell `(40,40)`, `lepton_to_screen` maps the centered start to `(0,1215)` px
and Rust F0 to approximately `(-8.789,1222.910)` px
(`src/util/lepton.rs:116-144`). The next two 45 Hz sub-ticks hold that state
because the Drive budget gate is 15 Hz. Native consumes RawTrack points 0 and 1
for this same budget and both carry heading `0`, so the first rendered body
facing already differs even though the full native coordinate is UNCHECKED.

## Milestone failures

1. **Fresh curve starts eleven points late and skips another point.** This is
   the earliest visible break on every north-to-east move from rest: Rust shows
   facing `0x0C` on its first budget pass while native is still `0x00`.
2. **Cell handoff ends the Rust unit's tick.** Spendable residual can survive a
   crossing but is not consumed until a later Drive frame, producing ordinary
   open-ground pulsing/stutter.
3. **Track completion omits the paid native sentinel boundary.** The transition
   from the opening turn into straight east motion can occur on a different
   frame and with different residual.
4. **Startup ownership is in command dispatch instead of Drive Process.** This
   changes when path/track/facing state first becomes visible relative to the
   unit's normal locomotor/mission pass.
5. **Arrival NavCom clears on a 45 Hz sub-tick rather than the next native
   Drive logic frame.** Mission/arrival consumers can observe stopped/null state
   early after every completed move.

## Residuals and stop condition

- The complete native per-frame coordinate/facing/cursor/residual series through
  `(45,40)` was not captured; it remains UNCHECKED rather than inferred.
- Exact native command-to-mission writes, literal path queue, final arrival
  frame, and mission timer/RNG handoff remain UNCHECKED.
- No adjacent locomotor, obstacle, formation, slope, bridge, sound, or combat
  behavior was expanded here.
- Smallest decisive follow-up: capture entry/exit of `0x004B0F20` for this exact
  fixture, logging owner coordinate/facing, selector, cursor, residual,
  speed-fraction, NavCom, and mission fields each native frame through arrival,
  then compare to an equivalent Rust fixture.
