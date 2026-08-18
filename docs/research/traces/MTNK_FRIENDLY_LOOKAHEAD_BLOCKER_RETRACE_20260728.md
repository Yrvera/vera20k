# MTNK Friendly Lookahead Blocker Retrace — 2026-07-28

## Verdict

**FAIL for ordinary-play locomotion.** In this exact straight-east fixture, `(55,50)` is
not a turn-chain lookahead cell in either engine. East-to-east selects TurnTrack entry
`18`, RawTrack `1`, target facing `0x40`, flags `3`; RawTrack `1` has
`chain_index = -1`. The active gamemd path therefore checks `(55,50)` as the immediate
next cell in `DriveLocomotionClass::Process_Movement` after the preceding straight track
finishes. Current Rust instead begins and residual-retries the next straight track
directly from the old-track finish path without a live occupancy check. A moving allied
MTNK can therefore be driven into/through instead of producing the retail traffic wait
and repath cadence.

Verdict tally: **PASS: 2 | FAIL: 7 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1**

## Exact Scenario and Scope

- A: stock owned/allied `[MTNK]`, centered at `(50,50)`, facing east (`0x40`), normal
  Move to `(60,50)`, committed straight east route.
- B: stock allied `[MTNK]`, alive and moving, enters/occupies `(55,50)` before A starts
  the straight segment from `(54,50)` to `(55,50)`.
- Flat clear Temperate ground, level `0`, no veterancy/crates/house bonuses.
- Scope: selected cell, live `Can_Enter_Cell` byte, block branch, residual/speed state,
  facing/track continuity, retry cadence, resume/arrival presentation.
- Active standard YR: `[MTNK]` has `Speed=7`, `ROT=5`, `Crusher=yes`,
  `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}`, `MovementZone=Normal`, and
  `Accelerates=false` in `ini/rules.ini:6569-6606`; `rulesmd.ini` does not replace this
  stock type. Fresh read-only decompiles of `0x004B0500`, `0x004B0F20`,
  `0x004B2630`, and `0x0073F0A0` confirm the path is live Drive/Unit behavior, with no
  TS-only gate.

## Evidence

- Research-index first: exact prior anchor
  `docs/research/traces/DRIVE_TRACK_CHAIN_LOOKAHEAD_BLOCKER_TRACE_20260527.md`, plus
  `docs/research/pathfinding/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`,
  `docs/research/pathfinding/FOOTCLASS_PATHFINDING_AND_MOVEMENT.md`, and the runtime
  Can-Enter callsite reports.
- Fresh read-only Ghidra:
  - `decompile_function 0x004B0500`: active-track finish calls
    `Process_Movement(...,1,0)`, then `Process_Drive_Track(1)`.
  - `decompile_function 0x004B0F20`: active DriveTrack step/finish and chain gates.
  - `decompile_function 0x004B2630`: immediate next-cell `Can_Enter_Cell` call, code-2
    recursion, track selection, and block cleanup.
  - `decompile_function 0x0073F0A0`: active UnitClass return-code producer; a moving
    allied occupant yields code `2`.
  - `disassemble_bytes 0x004B35D0..0x004B3850`: code-2 state writes and timer gates.
  - `disassemble_bytes 0x004B39C0..0x004B3AE0`: urgency `1/2` selection and
    `FootClass::Find_Path @ 0x004D3920`.
- Current Rust source was read from dirty `dev`; unrelated app/RMG changes were
  preserved. No Cargo command was run.

## Pipeline

Move command/path → straight RawTrack 1 toward `(54,50)` → old track finishes →
select/check next cell `(55,50)` → classify moving ally as byte `2` → hold/repath while
preserving east facing → blocker clears → install/retry straight track → update
position/occupancy → cached screen coordinates → voxel render.

## Entry Points for the Scoped Block Event

1. **gamemd ordinary straight continuation** — `DriveLocomotionClass::Process @
   0x004B0500` finishes the active track, calls `Process_Movement @ 0x004B2630`, and
   probes the immediate next queued cell before selecting the next track.
2. **gamemd turning-chain lookahead** — `Process_Drive_Track @ 0x004B0F20` can probe a
   next-next cell at a raw track's chain point. It is **not reached here** because
   straight RawTrack 1 has `chain_index=-1`.
3. **Rust old-track finish continuation** — `advance_lepton_position` directly selects,
   begins, and residual-retries the next track at
   `src/sim/movement/movement_step.rs:801-858`.
4. **Rust turning-chain hook** — `movement_tick.rs:1507-1562` only queues the deferred
   chain check when `next_face != cur_face`; it is **not reached** for `0x40 → 0x40`.
5. **Rust ordinary deferred occupancy hook** — `movement_occupancy.rs:806-872` handles a
   code-2 result only if a `DeferredCellCheck` was produced. The direct straight-track
   continuation does not produce one.

Coverage result: gamemd covers the exact straight continuation. Rust has classifiers and
block handlers, but the live handoff that must invoke them is missing.

## Stage Trace

| Stage | gamemd concrete output | Current Rust concrete output | Verdict |
|---|---|---|---|
| 1. Stock input | MTNK `Speed=7`, `ROT=5`, Drive CLSID, `MovementZone=Normal`, `Accelerates=false`; east facing byte `0x40`. | Same merged INI is the rules source; current code uses facing byte `0x40` for east. Runtime parsed object dump was not produced. | UNCHECKED |
| 2. Straight track identity | E→E table index `2*8+2=18`; RawTrack `1`; target facing `0x40`; flags `3`; chain index `-1`. No turn-chain probe occurs. | `TURN_TRACKS[18]` is RawTrack `1`, target `0x40`, flags `3` (`drive_track.rs:321-327`); `RAW_TRACKS[1].chain_index=-1` (`:716-723`). | PASS |
| 3. Exact checked cell | After A reaches `(54,50)`, `Process_Movement` derives the next coordinate by east direction `2`: `(54,50)+(1,0)=(55,50)`, height `0`, null parent, arg5 `1`. | The straight finish reads the same path cell `(55,50)` at `movement_step.rs:810-824`, but only uses it to select/begin a track; no runtime entry tuple is built. | NOT-IMPLEMENTED |
| 4. Live result byte | `UnitClass::Can_Enter_Cell @ 0x0073F0A0` sees alive moving allied B and returns byte/code `2` (TemporaryBlock). | If called, `classify_blocker` would map friendly plus `movement_target.is_some()` to `TemporaryBlock`/`yr_code()=2` (`cell_entry.rs:607-635`), but this live path never calls it. | FAIL |
| 5. Immediate branch/order | Code `2` clears head-to state, sets `Foot+0x6B7=1` on first block, starts `Foot+0x668` for `Rules+0x1768=60`, and does not install the track. Evidence: `0x004B3607..0x004B3690`. | Old-track finish clears/replaces `drive_track`, begins RawTrack 1, then immediately retries it with residual-only budget (`movement_step.rs:801-858`). No wait state is written. | FAIL |
| 6. Scatter | Code `2` itself issues zero `Scatter_Objects` calls. Scatter is the distinct code-6 branch. | The live bypass issues zero scatter calls. If the ordinary code-2 handler were reached after 60 local ticks it would call `scatter_blocker`, but that helper refuses already-moving B (`movement_occupancy.rs:821-835`; `bump_crush.rs:739-742`). | PASS |
| 7. Retry cadence | While blockage timer remains, allowed retries call `Find_Path` with urgency `1`; after the 60-frame timer expires, urgency is `2`. `PathDelay=.01` gives a 9-frame path-call rate gate. Evidence: `0x004B3690..0x004B36EF` and `0x004B39D1..0x004B3A0E`. | Live path performs no blocked retry. Even if the ordinary handler were reached, it does nothing while `blocked_delay>0`, then jumps directly to scatter+urgency-2 repath (`movement_occupancy.rs:814-865`), omitting urgency-1 retries. | FAIL |
| 8. Timer units | `BlockagePathDelay=60` is 60 native 15 Hz frames = 4 seconds; `PathDelay=.01` is 9 native frames = 0.6 seconds. | Values are stored as `60` and `9` (`ruleset.rs:1489-1498`) but decremented once per 45 Hz sim tick (`movement_tick.rs:1078-1079`; `fixed_math.rs:47-51`), so the latent handler would expire them in about 1.33 s and 0.20 s. | FAIL |
| 9. Residual/speed handoff | Blocked continuation leaves no active next track; the later `Process_Drive_Track(1)` no-track guard clears Drive `+0x4C` residual to `0`. MTNK keeps the east orientation; no positional budget is consumed into `(55,50)`. | Residual is preserved from the old track and consumed immediately by the new RawTrack through `advance_drive_track_retry_after_selection` (`movement_step.rs:801-858`, `drive_track.rs:3733-3779`). Exact encounter residual was not captured. | FAIL |
| 10. Facing/track continuity | Facing stays east `0x40`; active track is `-1` while blocked. On clear, E→E straight track starts. | Target/current facing stays east `0x40`, but RawTrack 1 remains active and advances. Facing byte matches while track state does not. | FAIL |
| 11. Position, occupancy, and screen | A holds at the completed-cell position until a retry can enter; B remains the sole vehicle occupant of `(55,50)`. | A advances toward B. On DriveTrack cell jump, Rust moves A's occupancy into the target with no intervening CEC (`movement_tick.rs:1377-1460`), allowing same-cell vehicle overlap. `refresh_screen_coords` publishes the advanced position (`components.rs:51-59`), consumed by vehicle drawing (`app_instances/units.rs:197`). | FAIL |
| 12. Resume/arrival | Once B clears, a permitted retry returns code `0`, installs the next straight track, and A resumes toward `(60,50)`. Exact arrival frame depends on B's route and the timer phase. | A can continue without ever entering blocked state and can still reach `(60,50)`, but its path timing, occupancy order, and visible spacing already diverged. Exact arrival tick for both engines was not captured. | UNCHECKED |

## Milestone Failures

1. **Missing straight-track dynamic occupancy handoff.** This is the earliest root cause.
   The classifier is present, but ordinary straight Drive continuation bypasses it.
2. **No retail traffic pause.** A can visibly drive into/through a moving ally instead of
   holding spacing at the completed cell.
3. **Wrong recovery mechanism behind the bypass.** The latent Rust code-2 handler waits
   out the full blockage counter, tries a scatter that rejects moving B, then starts at
   urgency 2. Retail performs urgency-1 path retries during the grace period.
4. **Timer base is three times too fast.** Movement timers expressed in native frames are
   decremented at 45 Hz.
5. **Residual/track state continues through the blocker.** Retail clears the no-track
   residual; Rust spends it into the next track in the same sim tick.

These trigger whenever a moving friendly enters a vehicle's next straight path cell after
the route was committed—common in base exits, group movement, and narrow traffic—so the
severity is high for ordinary skirmish locomotion feel.

## Residuals

- Exact sub-lepton position and residual at the encounter tick require a synchronized
  gamemd/Rust runtime capture; no numerical equality is claimed.
- Exact arrival tick remains fixture-dependent because B's committed movement route was
  intentionally not generalized beyond “moving and occupying `(55,50)`.”
- Rust occupancy iteration's first-match approximation is adjacent; only one blocker
  exists here, so it does not affect this result.

## Status

**COMPLETE** for the exact scenario. Investigation only; no Rust, INI, Ghidra state, or
other document was modified.
