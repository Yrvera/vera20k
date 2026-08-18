# Mission Deploy Building Deploy-Facing Visible Dump Facing Ghidra Report

Date: 2026-05-26
System: miner / refinery dock / deploy unload
Investigation mode: exhaustive-slice, gap-only verification
Primary anchors: `UnitClass__Mission_Deploy_Building @ 0x0073D630`, `UnitClass__DrawExtras @ 0x0073CEC0`, `FUN_004DB0A0 @ 0x004DB0A0`, `MissionClass__Mission_Dispatch @ 0x005B3060`
Active YR verdict: active stock Yuri's Revenge path
Confidence: High for static binary mechanism and render-orientation source; Medium for exact same-frame screenshot ordering because no runtime capture was taken.

## Scope

This report verifies the narrow disagreement left after the radio `0x16` investigation:

- Does the stock engine still require an east-facing/rate-timer gate before starting the visible refinery dump?
- If yes, is that gate render/pixel relevant, or only internal bookkeeping?
- Should Rust remove pivoting entirely, keep it on radio `0x16`, or move it into mission `0x10` deploy-unload handling?

This report intentionally does not re-investigate far-return refinery search, cargo-credit drain math, unload state 3/4 refinery rediscovery, or missing-refinery stale-frame duration. Those are separate miner parity slices.

## Prior Reports Used

- `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md`
- `docs/research/miner/HARV_UNLOADING_CLASS_DISPLAY_TIMING_GHIDRA_REPORT.md`
- `docs/research/REFINERY_DOCK_DEPLOY_SOUND_ANIM_TIMING_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`

The existing reports were recent and high-confidence, so this pass only bridges the verified mission-facing gate to the verified render-orientation path.

## Key Offsets And Fields

| Field | Meaning in this slice |
| --- | --- |
| `Unit+0x388` | `RateTimer` / facing timer used by deploy-facing gate |
| `Unit+0x674` | active locomotor pointer |
| locomotor vtable `+0x3C` | live draw-facing query used by voxel draw |
| locomotor vtable `+0x4C` | command toward facing value `0x4000` |
| `Unit+0x6AF` | facing/locomotor sync flag checked before issuing vtable `+0x4C` |
| `Unit+0x6D1` | unload-active render latch |
| `Unit+0x6C4` | current `TypeClass` pointer, temporarily swapped for unloading draw |
| `Type+0xE0E` | `Harvester=yes` flag |
| `Type+0x6B8` | `UnloadingClass` pointer, e.g. `CMON`/`HORV` |
| `Unit+0xF8` | unload accumulator cleared at unload-start |
| `Unit+0x100..0x10C` | unload timing cluster written at unload-start |
| `Unit+0xBC` | mission substate; unload-start writes state `3` |
| mission vtable `+0x23C` | mission `0x10` dispatch target for `UnitClass__Mission_Deploy_Building` |

## Verified Mission Path

`MissionClass__Mission_Dispatch @ 0x005B3060` dispatches mission `0x10` through unit vtable `+0x23C`, reaching `UnitClass__Mission_Deploy_Building @ 0x0073D630`.

The relevant late path inside `UnitClass__Mission_Deploy_Building` is:

1. Validate the dock path with `PathType::Has_Valid_Steps`.
2. Check the facing timer:
   - call `RateTimer::Current` on `Unit+0x388`
   - compute `((current >> 7) + 1) & 0x1FE`
   - accept only when the result equals `0x80`
3. If not accepted:
   - if `Unit+0x6AF` is clear, call active locomotor vtable `+0x4C(0x4000)`
   - return mission delay `5`
4. If accepted, start unload with this observed write order:
   - `Unit+0xF8 = 0`
   - `Unit+0x6D1 = 1`
   - read global current frame counter
   - `Unit+0x10C = 1`
   - `Unit+0x100 = current frame`
   - `Unit+0x104 = stack-derived duration/value`
   - `Unit+0x108 = 1`
   - optionally invoke adjacent refinery anim slot `7`
   - `Unit+0xBC = 3`

No direct body-facing byte write was found in this unload-start block. The block gates on the existing rate-timer/facing state; it does not snap the unit to east when the dump begins.

## Verified Render Path

`UnitClass__DrawExtras @ 0x0073CEC0` makes the unload display visible by temporarily swapping the unit type pointer:

1. Check `Type+0xE0E` (`Harvester=yes`).
2. Check `Unit+0x6D1`.
3. Check `Type+0x6B8` (`UnloadingClass` pointer).
4. Temporarily write `Unit+0x6C4 = Type+0x6B8`.
5. Draw the unit using the normal unit draw path.
6. Restore the saved original `TypeClass` pointer.

For voxel draw, orientation is read from the live unit locomotor. `FUN_004DB0A0 @ 0x004DB0A0` checks `Unit+0x674`; when present, it calls locomotor vtable `+0x3C` and returns that value. If no locomotor exists, it returns fallback value `2`.

Therefore `CMON`/`HORV` unloading-class orientation is not a separate hardcoded east-facing render mode. It is the live unit/locomotor facing at draw time.

## Pixel-Parity Consequence

The mission-facing gate is player-visible. Once `Unit+0x6D1` is set, `CMON`/`HORV` can render through `DrawExtras`, and that rendering uses the live locomotor facing. If Rust starts unload while the unit's live facing still differs from the accepted stock facing window, the unloading voxel can rotate differently for the dump duration.

That is a pixel drift, not an internal-only difference.

The important correction is:

- radio `0x16` should not perform the ordinary first-call unload setup or body snap;
- mission `0x10` still must enforce the deploy-facing gate before the unload latch is set;
- Rust must not force `entity.facing = east` at unload-start as a substitute for the stock rate-timer/locomotor gate.

## INI Evidence

Relevant stock `rulesmd.ini` entries:

| Object | Relevant keys |
| --- | --- |
| `CMIN` | `Harvester=yes`, `ROT=5`, `UnloadingClass=CMON` |
| `HARV` | `Harvester=yes`, `ROT=5`, `UnloadingClass=HORV` |
| `GAREFN` | `DockUnload=yes`, `Refinery=yes` |
| `NAREFN` | `DockUnload=yes`, `Refinery=yes` |

Relevant stock art behavior from the existing sound/anim report:

- unload latch has no direct deploy sound call;
- slot `7` can be invoked in the binary, but stock `GAREFN`/`NAREFN` have no visible `PreProductionAnim`;
- per-bale refinery overlay comes later through slot `10` / special anim behavior and is facing-independent.

## Current Rust Implication

The earlier Approach 1 design needs one correction.

Correct part:

- ordinary radio `0x16` handling should be bounded to the verified stock behavior: parent receive-radio call, facing/rate-timer sync request through locomotor when unsynced, return `1`, and no first-call destination/write/unload-start/body-facing snap.

Correction:

- do not remove the deploy-facing gate from the overall unload path;
- move/keep that gate in mission `0x10` deploy-unload handling, matching `UnitClass__Mission_Deploy_Building`;
- when not ready, issue locomotor facing command `0x4000` only under the stock sync-flag condition and return/poll on mission delay `5`;
- set the unload display latch only after the gate accepts;
- do not hard set the entity body facing at unload-start;
- suppress stock `DockDeploy` sound emission at unload latch.

## Implementation Handoff

| Area | Required parity behavior |
| --- | --- |
| radio `0x16` | No first-call unload-start, no dock coordinate lookup, no destination write, no body snap. Only verified facing-sync request and later conditional `0x15` path. |
| mission `0x10` | Enforce path gate, then rate-timer facing gate before setting unload-active/display state. |
| not-ready cadence | Return mission delay `5`; do not poll every simulation tick if the mission scheduler should wait. |
| facing command | Use locomotor face command equivalent to vtable `+0x4C(0x4000)` only when the stock sync flag path allows it. |
| unload display latch | Activate the `Unit+0x6D1` equivalent only after the facing gate passes. |
| facing state | Do not set `entity.facing = 0x40` or equivalent at unload-start. The visible unloading class must inherit live locomotor facing. |
| sounds | No stock `DockDeploy` sound at unload latch. |
| refinery anim slot `7` | Keep conditional/mod-capable, but stock `GAREFN`/`NAREFN` should produce no visible slot-7 effect. |
| render verification | Acceptance should include a case where entering the unload phase from a non-east facing would visibly fail if Rust snaps or skips the gate. |

## Coverage Ledger

| Question | Status | Evidence |
| --- | --- | --- |
| Is mission `0x10` live for deploy-unload? | Resolved | `MissionClass__Mission_Dispatch @ 0x005B3060` vtable `+0x23C` call |
| Is there a path gate before unload-start? | Resolved | `UnitClass__Mission_Deploy_Building @ 0x0073D630`, `PathType::Has_Valid_Steps` path |
| Is there a facing/rate-timer gate before unload-start? | Resolved | `RateTimer::Current(Unit+0x388)`, formula `((current >> 7) + 1) & 0x1FE == 0x80` |
| What happens when the facing gate is not ready? | Resolved | optional locomotor vtable `+0x4C(0x4000)`, return delay `5` |
| Is unload-start a direct body-facing snap? | Resolved | no facing byte write found in unload-start write cluster |
| When does unloading visual become active? | Resolved | unload-start writes `Unit+0x6D1 = 1` before state `3` |
| What draws `CMON`/`HORV`? | Resolved | `UnitClass__DrawExtras @ 0x0073CEC0` temporary type swap |
| What facing does `CMON`/`HORV` use? | Resolved | live locomotor via `Unit+0x674` and locomotor vtable `+0x3C` |
| Is wrong facing pixel-visible? | Resolved | yes; unloading voxel orientation comes from live facing |
| Is there a stock deploy sound at unload latch? | Resolved | no direct stock deploy sound call in verified sound/anim report |
| Is exact same-frame screenshot timing proven? | Deferred | needs runtime capture/debugger; static mechanism is sufficient for implementation shape |

## Final Verdict

Approach 1 remains correct for radio `0x16`, but incomplete if interpreted as removing all pivot/facing gating from the unload path. Stock Yuri's Revenge separates the mechanisms:

- radio `0x16` does not start unload or snap the unit east on the first ordinary call;
- mission `0x10` deploy-unload later gates unload-start on the unit's facing/rate timer;
- once unload display is latched, the visible `CMON`/`HORV` voxel uses live locomotor facing.

For pixel parity, Rust should implement the mission `0x10` facing gate and remove the hard unload-start body snap. The visible dump must begin only after the stock gate accepts.

