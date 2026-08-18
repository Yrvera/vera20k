# Trace: Force_Track 0x47 Bib Step — Post-Dump Pre-Exit-Cell A*

**Mechanic:** `ReleaseDockedHarvester` step 8 — `Force_Track(0x47, building_center + (−0x80, +0x80), z)`  
**Scope:** Narrow — only the sub-cell arc before `Set_Destination`. A* path and `Set_Destination` are out of scope.  
**Date:** 2026-05-20  

> **Correction 2026-05-21 - trace invalidated for stock DockUnload**
>
> This trace's premise is superseded for stock `CMIN/HARV -> GAREFN/NAREFN`.
> Normal stock DockUnload completion does not call `ReleaseDockedHarvester` and
> does not issue `Force_Track(0x47)`; it exits through zero-link
> `Mission_Deploy_Building` state 4. Keep this trace only as a conditional
> reciprocal-link `ReleaseDockedHarvester` movement reference.
**Ghidra MCP:** OFFLINE — all claims cite existing docs only.  
**YR-active confirmation:** Superseded 2026-05-21. `ReleaseDockedHarvester` is conditional nonzero-link release; it does not fire every stock ore delivery.

---

## Stage 1 — What is Force_Track(0x47)?

**Claim (doc):** `RELEASEDOCKEDHARVESTER` §Step 8 states: vtable slot `+0x70` called as  
`(*loco_vtable + 0x70)(loco, 0x47, x - 0x80, y + 0x80, z)`.  
`0x47` = decimal 71.

**Verdict: PASS** — track index confirmed by cross-referencing our TURN_TRACKS table.  
`TURN_TRACKS[71]` (index 0x47):
```
normal_track: 15, short_track: 15, target_facing: 0xC0, flags: 0
```
(`src/sim/movement/drive_track.rs`, line 677–683)

This is a **TURN_TRACK index**, not a raw track index and not a facing byte. The doc (BUILDING_UNDOCKUNIT §3d) explicitly confirms: `0x47` is "a drive track index fed to Head_To, not a TechnoClass facing field write."

---

## Stage 2 — Does Track 15 Produce Visible Sub-Cell Motion?

**Question:** Is Force_Track(0x47) a position-changing operation or purely a "set up the track" metadata write?

**Track 15 geometry** (from `drive_track.rs` line 813–820, 3296–3379):
- **16 points**, `entry_index: 0`, `cell_cross_index: -1` (NO cell crossing).
- Start point: `(x=128, y=-128, facing=128)` — exactly the `(−0x80, +0x80)` lepton offset from building center.
- End point: `(x=16, y=−4, facing=188)`.
- Described as "special E — SE to S arc" — a smooth curve from ESE (facing 128 = East) sweeping southward toward S (facing 188-192 = South range, target_facing=0xC0=192=South-SW).

**Key finding:** `cell_cross_index: -1` means the miner does NOT move to a new cell during this arc. All 16 track points stay within sub-cell space. However, the miner's **sub-cell position and facing change continuously** along the arc.

**Observable motion:** YES, there is visible sub-cell motion. The miner starts at sub-position (128, -128) relative to the building center tile and sweeps toward (16, -4) over ~2–5 ticks (at typical miner speed ~7 leptons/tick). The body facing rotates from 128 (East) to ~188-192 (South) during this time.

**Verdict: NOT-IMPLEMENTED** — our code has no Force_Track step. `phase_departing` invokes `issue_move_command` directly; facing is set from the first A* path step direction, not from the track-15 arc.

Doc source: RELEASEDOCKEDHARVESTER_0x4595C0 §Step 8; BUILDING_UNDOCKUNIT_0x4593A0 §3b; drive_track.rs lines 677–683, 813–820, 3296–3379.

---

## Stage 3 — Our Code: What Does phase_departing Do?

From `src/sim/miner/miner_dock_sequence.rs` line 618–725:

```
DepositCooldown → Departing
  snap.miner.exit_cell cached on first entry (line 631)
  if !moving && !at_exit:
      issue_move_command(exit_cell)  ← straight to A*
      mt.bypass_grid = true
  return
```

**No Force_Track sub-step exists.** The comment at lines 688–692 explicitly notes:

> "Note: facing is intentionally NOT pinned here. `issue_move_command` already sets `facing_target` from the first path step, so the movement system rotates the unit toward the actual direction of travel as it leaves the pad. gamemd's `Force_Track(0x47, ...)` is a DRIVE-TRACK CURVE INDEX, not a facing byte — pinning facing to 0x47 would make the miner drive backwards."

This comment correctly identifies that `0x47` is a track index, NOT a facing byte. However, it does not address the sub-cell arc motion that Track 15 produces before A* fires. The comment's conclusion ("pinning facing to 0x47 would make the miner drive backwards") is about a facing-byte misread and is correct — but the absence of a Force_Track step is a separate matter.

**Verdict: NOT-IMPLEMENTED** — the brief sub-cell arc and facing sweep from Track 15 are absent.

---

## Stage 4 — Observable Player-Visible Difference

**What the player sees in gamemd:**  
Over ~2–5 ticks, the miner body sweeps from East-facing (0x80) through a smooth ESE→S arc to roughly South-facing (0xC0) while remaining within the same cell. The sub-cell position shifts ~112 leptons in X and ~124 leptons in Y. Then A* fires and the miner drives to the exit cell.

**What the player sees in our engine:**  
The miner is facing East (0x40 = `DOCK_FACING_EAST`, set in `phase_linked`). On the first Departing tick, `issue_move_command` fires immediately and `facing_target` is set from the first A* step (toward the exit cell, typically WSW or S direction). The miner body snaps toward that direction over the next few ticks via normal turret rotation.

**Observable delta:**
1. **Missing sub-cell arc**: In gamemd, the miner physically moves ~112 leptons west and ~124 leptons south within the bib tile before A* takes over. In ours, the miner is still centered on the pad cell when A* fires.
2. **Different facing sweep**: gamemd sweeps E→SE→S (via Track 15). Ours sweeps E→WSW/S directly (via normal turret rotation rate-limited by `turret_rot`).
3. **Timing slip**: A* fires 2–5 ticks later in gamemd (after Track 15 exhausts). In ours, A* fires immediately on the first Departing tick.

**Severity assessment:**  
Fires on every ore delivery (every ~1.5–3 minutes of game time per miner). The sub-cell arc lasts ~2–5 ticks (~133–333 ms at 15 fps). At normal zoom the arc is visible as a brief directional jitter before the exit drive begins. At maximum zoom-in it is clearly an extra smooth curved motion. Player-visibility: MEDIUM (noticeable at close zoom, less so at strategic zoom).

**Verdict: NOT-IMPLEMENTED**

---

## Stage 5 — Power_On (Step 7) and SetSpeedMultiplier (Step 9)

**Power_On (step 7):** `(*loco_vtable + 0x58)(loco)` — restores locomotor from stopped state after dock.  
**Our code:** `movement::issue_direct_move` and `issue_move_command` both set `movement_target`, which is the equivalent of starting locomotion. There is no explicit "Power_On" call because our drive system does not have a stopped/powered-off concept separate from having a movement target.  
**Verdict: UNCHECKED** — behavioral equivalence likely (movement begins on the same tick), but the exact locomotor power-on semantics are not modeled.

**SetSpeedMultiplier(1.0) (step 9):** restores full speed after dock override.  
**Our code:** `phase_linked` passes `snap.speed` to `issue_direct_move`. `snap.speed` is the miner's configured speed, not a reduced value. No dock-induced speed reduction is applied during the Linked/Unloading phases.  
**Verdict: UNCHECKED** — gamemd restores from a speed override; our code never applied a speed override, so the restore is vacuously correct. No observable difference expected.

---

## Stage 6 — TS-vs-YR Filter

`ReleaseDockedHarvester` (0x4595C0) is reached from `UnitClass::Mission_Deploy_Building` (0x0073D630) only on the nonzero reciprocal-link branch. This is not the normal stock ore-dump exit path; `Force_Track(0x47)` is conditional on that release context.

**Verdict: PASS** — this behavior is YR-active.

---

## Top 5 Player-Visible Failures

| # | Stage | Player-Observable Effect | Our File:Line | gamemd Evidence |
|---|-------|--------------------------|---------------|-----------------|
| 1 | Sub-cell arc | Miner exits pad with no SE sweep — jumps straight to A* direction | `miner_dock_sequence.rs:649–698` | RELEASEDOCKEDHARVESTER §Step 8; Track15 cell_cross_index=-1, 16-point arc |
| 2 | Facing sweep | Body facing goes E→WSW directly instead of E→SE→S arc (Track 15 target_facing=0xC0) | `miner_dock_sequence.rs:688–692` | TURN_TRACKS[71] target_facing=0xC0; BUILDING_UNDOCKUNIT §3d |
| 3 | A* timing | A* fires immediately; gamemd delays ~2–5 ticks until Track 15 exhausts | `miner_dock_sequence.rs:656–668` | RELEASEDOCKEDHARVESTER §Step 8 precedes §Step 10 |
| 4 | Sub-cell start position | Miner at pad-center when A* fires; gamemd has miner ~112×124 leptons into bib tile | `miner_dock_sequence.rs:631–637` | Track15 start=(128,-128), end=(16,-4); no cell-cross confirmed |
| 5 | Power_On semantics | No explicit locomotor power-on before movement; potential off-by-one if loco was truly stopped | `miner_dock_sequence.rs:649` | RELEASEDOCKEDHARVESTER §Step 7: `(*loco_vtable+0x58)` |

**Entries 1–4 are the same root cause** (missing Force_Track step); they are listed separately because they have distinguishable observable effects.

---

## Verdict Tally

PASS: 2 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 4

*(PASS: YR-active filter, track-index identity. UNCHECKED: Power_On semantics, speed-multiplier restore. NOT-IMPLEMENTED: sub-cell arc, facing sweep, A* timing, sub-cell start position — all stem from the missing Force_Track step.)*

---

## Implementation Note (for later)

To implement the Force_Track step, `phase_departing` would need to:
1. On first entry (when `exit_cell` is None), begin Track 15 via `begin_drive_track(15, flags=0, head_dx=-0x80, head_dy=+0x80, target_facing=0xC0)` and store the `DriveTrackState`.
2. Each subsequent tick, call `advance_drive_track` until the track is exhausted.
3. Only after Track 15 completes, fire `issue_move_command` to the exit cell.

The existing `DriveTrackState` / `begin_drive_track` / `advance_drive_track` machinery in `drive_track.rs` is already capable of running Track 15 — it is a matter of wiring the pre-A* arc into the Departing phase.

---

**Status: COMPLETE**
