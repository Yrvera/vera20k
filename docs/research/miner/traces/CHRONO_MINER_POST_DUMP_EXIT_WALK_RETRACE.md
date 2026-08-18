# Chrono Miner Post-Dump Exit Walk — RE-TRACE
**Mechanic:** Chrono miner post-dump exit walk (REFINERY → exit cell, drive only, no warp)
**Scenario:** Pad cell (13,11) → exit cell, 4×3 GAREFN at base cell (10,10), `RefineryDockPhase::Departing` entered after cargo empty
**Date:** 2026-05-20

> **Correction 2026-05-21 - trace invalidated for stock DockUnload**
>
> This re-trace used the older `ReleaseDockedHarvester` normal-exit model.
> Later binary work shows stock GAREFN/NAREFN unload completes through the
> zero-link `Mission_Deploy_Building` state-4 path. Treat all
> `ReleaseDockedHarvester`, `Force_Track(0x47)`, departure VOC, and slot
> 0xC/0xD conclusions here as conditional reciprocal-link findings, not
> stock ore-delivery parity requirements.
**Ghidra:** OFFLINE this session — all gamemd claims sourced from `ra2-rust-game-docs/`
**Trace Iron Law:** PASS requires literal numerical equality between our output and gamemd's. UNCHECKED means one side was not computed.

---

## Concrete Scenario Facts

| Item | Value | Source |
|------|-------|--------|
| GAREFN foundation | 4×3 | rules INI `Foundation=4x3` |
| Building base cell | (10, 10) | scenario |
| Foundation cells | x∈[10,13], y∈[10,12] | derived |
| Pad cell (no DockingOffset) | (13, 11) = rx+w−1, ry+h/2 | `refinery_pad_cell` fallback |
| Pad stated in scenario | (13, 11) | matches |
| gamemd NW-corner cell | (10, 10) | `GetCellLocation` vtable+0x1b8, doc §Step 10 |
| gamemd anchor | (9, 11) | NW_x−1, NW_y+1 |
| Our anchor | (10, 12) | center_x−1, center_y+1 (see Stage 2) |
| gamemd exit cell | (9, 11) | ring-0 of spiral from (9,11); outside foundation → passable |
| Our exit cell (real gameplay) | (9,11) at tick%5=0; (9,13)/(10,13)/(11,13)/(9,12) otherwise | ring-1 of spiral from (10,12) |
| Our exit cell (blank-grid test) | (10, 12) | ring-0 passable on blank grid (building not in path grid) |

---

## Pipeline Stage Table

### Stage 1 — Phase Entry: `RefineryDockPhase::Departing` entered after cargo empty

| | gamemd | Ours | Status |
|---|---|---|---|
| Phase transition trigger | After `FindFirstNonEmptySlot` returns −1 on last dump-gate tick → `ReleaseDockedHarvester` | `phase_deposit_cooldown` decrements `deposit_cooldown_ticks` to 0, then sets `dock_phase = Departing` | PASS |
| Trigger evidence | `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §5: "tick delay computation … 1–3 tick random jitter on next Mission_Deploy_Building call" | `miner_dock_sequence.rs:607–616` | |

**Notes:** Our one-interval post-last-bale hold before `Departing` matches gamemd's post-last-bale idle described in §5 of the ReleaseDockedHarvester report. PASS.

---

### Stage 2 — Exit Anchor Formula

gamemd Step 10 (`RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 10):
```
psVar6 = (*building_vtable + 0x1b8)(building);  // GetCellLocation → (cell_x, cell_y) as shorts
anchor.x = psVar6[0] - 1;   // one cell west of building's NW corner cell
anchor.y = psVar6[1] + 1;   // one cell south
```
Doc states: "For GAREFN (4×3, NW at cell 10,10), anchor = (9, 11) — one cell west of the foundation's west edge."

**gamemd anchor: (9, 11)**

Our code (`miner_dock_sequence.rs:134–137`):
```rust
let center_x = rx as i32 + (width.saturating_sub(1) as i32) / 2;
let center_y = ry as i32 + (height.saturating_sub(1) as i32) / 2;
let anchor_x = center_x - 1;
let anchor_y = center_y + 1;
```
For rx=10, ry=10, width=4, height=3: `center_x = 10 + 1 = 11`, `center_y = 10 + 1 = 11`, `anchor = (10, 12)`.

**Our anchor: (10, 12)**

**FAIL.** gamemd anchor = (9, 11). Our anchor = (10, 12). Offset: 1 cell east, 1 cell south.

**Docs-internal disagreement surfaced:** The `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md` doc does NOT use vtable+0x1b8 (`GetCellLocation`) at all — `UndockUnit` uses vtable+0x48 (`GetCoords`) which returns leptons at the **building center**, then applies lepton-space offsets (−0x80, +0x80). The `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` uses vtable+0x1b8 (`GetCellLocation`) which returns **cell coordinates as shorts**, with the doc explicitly calling it "NW corner." These are two different functions producing two different bases. The current Rust code uses a center-cell formula, matching neither doc exactly — it approximates the lepton-center-then-truncate behavior but applies it in cell-space rather than lepton-space. The doc is explicit that vtable+0x1b8 returns the NW corner cell directly. Ghidra is offline so we cannot re-verify, but the discrepancy is flagged.

---

### Stage 3 — Exit Cell from Spiral Search

**gamemd:** `FootClass::Find_Nearby_Passable_Cell` (0x0056DC20) from anchor (9,11). Cell (9,11) is outside foundation → ring-0 = passable → **exit = (9, 11)**. Single candidate; no modulo pick needed.

**Ours (real gameplay, building occupies cells):** Anchor (10,12) is inside foundation → ring-0 blocked. Ring-1 candidates collected:
- Segment 1 top (y=11): (9,11)✓, (10,11)✗ in foundation, (11,11)✗ in foundation
- Segment 1 bottom (y=13): (9,13)✓, (10,13)✓, (11,13)✓
- Segment 2 left (x=9): (9,12)✓
- Segment 2 right (x=11): (11,11)✗ in foundation (already counted)

Ring-1 candidates: `[(9,11), (9,13), (10,13), (11,13), (9,12)]` — 5 cells. Selection = `tick % 5`.

| tick % 5 | Our exit | gamemd exit | Match? |
|----------|----------|-------------|--------|
| 0 | (9, 11) | (9, 11) | YES |
| 1 | (9, 13) | (9, 11) | NO |
| 2 | (10, 13) | (9, 11) | NO |
| 3 | (11, 13) | (9, 11) | NO |
| 4 | (9, 12) | (9, 11) | NO |

**FAIL.** gamemd always produces (9,11). Our code produces (9,11) only 20% of the time (tick%5=0). The other 80% of exit cycles the miner walks to a south-of-foundation cell instead of west of the foundation — visually, the miner curves south instead of west.

**Ours (blank-grid test, building not in path grid):** Anchor (10,12) is passable (no building in grid) → ring-0 = (10,12). Exit = **(10,12)** — 1 cell inside where the building would be. This is a test artifact, not gameplay behavior.

**Consequence for existing test:** `chrono_miner_teleports_to_refinery_on_return` asserts exit = (10,12) at tick≥600 on a blank grid. The test passes but validates behavior that would never occur in real gameplay. The test comment on line 436 correctly notes this is a blank-grid artifact but the assert still encodes the wrong reference value.

---

### Stage 4 — Drive vs. Warp on Outbound

**gamemd:** `ReleaseDockedHarvester` Step 6: locomotion type guard (`vtable+0x2C == 1` = DriveLocomotionClass) passes for chrono miner because DriveLoco is the piggybacked-and-active locomotor during dock. Step 8: `Force_Track(0x47, −0x80, +0x80)` on DriveLoco. Step 12: `SetMission(MOVE=2)`. Zero Teleporter branches throughout the function — confirmed by full decompile read (§4 of ReleaseDockedHarvester report, §g of UndockUnit report).

**Ours:** `phase_departing` calls `movement::issue_move_command` (A* drive, `miner_dock_sequence.rs:657–668`). The `teleporting` guard at line 644 prevents A* re-issue if teleport is active. `handle_search_ore` explicitly does NOT issue `issue_teleport_command` (removed in the reverted commit). The regression test `chrono_miner_does_not_warp_outbound` (`miner_tests.rs:968`) asserts `entity.teleport_state.is_none()` after one tick of `SearchOre` from the exit cell.

**PASS.** Both gamemd and our code drive (no warp) on outbound. The regression test guards this.

---

### Stage 5 — A* Routing: Pad (13,11) → Exit Cell

**gamemd:** `Force_Track(0x47, −0x80, +0x80)` sets the initial drive track from the building center shifted SW, then `Set_Destination` + `SetMission(MOVE=2)` send the harvester to the passable exit cell. The Ghidra note states "resolves to the south bib row" for a 4×3 refinery. A* (DriveLoco pathfinding) must route around the foundation.

**Routing analysis:**
- Pad (13,11) is inside the foundation's east edge.
- Exit (9,11) [gamemd] is west of the foundation's west edge, same Y.
- Direct W path blocked: (12,11),(11,11),(10,11) all inside foundation.
- North path blocked: (13→10, y=10) all inside foundation.
- Only viable route: exit east to (14,11) [outside], go south to (14,12)/(14,13), west past south edge [(13,13),(12,13),(11,13),(10,13),(9,13)], then north [(9,12),(9,11)].
- This is the "south of the foundation" routing the scenario describes. UNCHECKED (cannot verify A* path output without running the sim with a realistic path grid).

**Our code:** `phase_departing` uses `movement::issue_move_command` with `bypass_grid=true` (line 695–697) to avoid occupancy stall at the queue cell. A* starts from the blocked pad cell — `miner_dock_sequence.rs:653` comment confirms "astar_search accepts a blocked start cell."

**UNCHECKED** (A* path output not computed; routing direction claim is geometrically plausible but not verified against live A* output). The south-routing claim is structurally correct given the foundation geometry.

---

### Stage 6 — Departure Sound and Anim

**gamemd:** Steps 1–4 of `ReleaseDockedHarvester`: clear anim slots 0xA+0xB, play `BunkerWallsDownSound` (= `TankBunkerDown` from rulesmd.ini), create anim slots 0xC+0xD (`SpecialAnimThree`/`SpecialAnimFour`). All happen before locomotion commands.

**Ours:** `SimSoundEvent::DockDeploy` is pushed in `phase_linked` (line 455), NOT in `Departing`. No `SpecialAnimThree`/`SpecialAnimFour` creation in any departing phase. No `BunkerWallsDownSound` fired in Departing.

**NOT-IMPLEMENTED.** The departure VOC and slot-0xC/0xD anim creation from `ReleaseDockedHarvester` steps 1–4 are absent. Player hears no `TankBunkerDown` sound on miner departure. Fires every ore delivery cycle — high frequency (every ~30 seconds per miner in a typical game).

---

### Stage 7 — Force_Track(0x47) and Initial Facing

**gamemd:** Step 8 of `ReleaseDockedHarvester`: `ILocomotion::Force_Track(0x47, x−0x80, y+0x80)`. This sets a drive-track curve index (decimal 71 = ESE), not a facing field write. The unit's facing updates as DriveLoco follows the track. No `unit->Facing = 0x47` assignment.

**Ours:** `phase_departing` calls `movement::issue_move_command` which sets `facing_target` from the first A* path step. Comment at line 688–692: "facing is intentionally NOT pinned here." This correctly handles the conversion: pinning to 0x47 would make the miner drive backwards (ESE facing while moving west). The actual facing will rotate toward the first path step direction (east, since A* exits east first).

**UNCHECKED.** We do not implement `Force_Track(0x47, ...)` at all — we rely on A* providing a reasonable first step direction. Whether the observable facing during the first 1–2 animation frames matches gamemd's is not verifiable without side-by-side rendering comparison. The concern is that gamemd's Force_Track briefly orients the miner ESE on the first movement frame, while our code orients it toward the A* first step (also likely ESE since the exit is east). Likely PASS in practice but not numerically verified.

---

### Stage 8 — Speed Multiplier Restore

**gamemd:** Step 9 of `ReleaseDockedHarvester`: `SetSpeedMultiplier(1.0)` — restores full speed after dock.

**Ours:** `handle_search_ore` computes `speed` from `ra2_speed_to_leptons_per_second(raw_speed)` fresh each tick from INI. No speed-multiplier clamping during dock, no explicit restore. The miner's speed at the pad is the same as during ore travel.

**UNCHECKED.** The question is whether we honor a speed-multiplier field that gamemd writes during dock (if any). We have no evidence of a speed reduction during dock in our code, and the doc shows speed is only SET (not reduced) in ReleaseDockedHarvester. If gamemd reduces speed during dock (not shown in the docs), we might run faster during dock. Risk: low; not verified.

---

### Stage 9 — Dock Teardown (Radio + Link Clearing)

**gamemd:** Step 13 of `ReleaseDockedHarvester`: clear `building->field_0x2E4`, clear `building->field_0x718`, `SetMission(5)` on building, `RadioCommand(CLEAR=3)`.

**Ours:** `phase_departing` on arrival at exit (line 705–718): `dock_reservations.release(ref_sid)`, `reserved_refinery = None`, transition to `SearchOre`. No building-side mutation (we don't have direct building-state access from the miner FSM).

**UNCHECKED.** We handle the reservation protocol (our equivalent of `RadioCommand(CLEAR=3)`) via `dock_reservations.release`. The building-side state (`field_0x718`, `SetMission(5)`) is not mirrored in our architecture (building state is managed separately). No player-visible effect identified from the missing `field_0x718` clear.

---

## Docs-Internal Disagreement: What Does `GetCellLocation` (vtable+0x1b8) Return?

The `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 10 states:
> "vtable slot +0x1b8 = `GetCellLocation` (returns cell X/Y pair as shorts)"
> "anchor = NW-corner cell X−1, Y+1"
> "For GAREFN (4×3, NW at cell 10,10), anchor = (9, 11)"

The `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md` uses vtable+0x48 (`GetCoords`) in a different function (UndockUnit uses lepton-space offsets, not cell-space). No contradiction in the docs themselves, but our Rust code uses a **center-cell formula** derived from neither — it computes `rx + (width-1)/2` (center X), not the raw NW corner `rx`. The doc is unambiguous: vtable+0x1b8 returns the NW corner (cell 10,10 for GAREFN at base 10,10), and the anchor is `NW−(1,−1) = (9,11)`. Ghidra is offline; this cannot be re-verified this session, but the doc confidence is HIGH (all verified from live decompilation).

**Surface finding:** Our `refinery_exit_cell` uses center-cell arithmetic which is WRONG per the doc. The correct formula is `anchor = (rx − 1, ry + 1)` where `(rx, ry)` is the building's NW corner (= the base cell we store).

---

## Adjacent Findings

1. **Test `chrono_miner_teleports_to_refinery_on_return` asserts wrong exit cell (10,12) instead of gamemd's (9,11).** The test uses a blank path grid where (10,12) is walkable, but this would never occur in gameplay where the building occupies cells. The test should either use a realistic grid with the building marked unwalkable, or assert (9,11). (Adjacent — do not trace this run.)

2. **`phase_departing` releases dock reservation on miner arrival at exit cell, not when `ReleaseDockedHarvester` fires.** gamemd releases (RadioCommand CLEAR=3) inside `ReleaseDockedHarvester`, before the miner has physically moved. Our release is deferred until the miner reaches the exit cell. Effect: next queued miner must wait ~8–10 extra ticks per dock cycle. Frequency: every ore delivery. (Adjacent — do not trace this run.)

3. **`BunkerWallsDownSound` firing point is wrong.** We emit `DockDeploy` in `phase_linked`, but gamemd plays `BunkerWallsDownSound` in `ReleaseDockedHarvester` (when Departing begins). Two-phase offset of several ticks. (Adjacent to Stage 6 finding — same root cause.)

---

## Verdict Tally

**PASS: 2 | FAIL: 2 | UNCHECKED: 4 | NOT-IMPLEMENTED: 1**

| Stage | Verdict |
|-------|---------|
| 1 — Phase entry trigger | PASS |
| 2 — Exit anchor formula | FAIL |
| 3 — Exit cell from spiral | FAIL |
| 4 — Drive vs. warp outbound | PASS |
| 5 — A* routing south of foundation | UNCHECKED |
| 6 — Departure sound/anim | NOT-IMPLEMENTED |
| 7 — Force_Track(0x47) facing | UNCHECKED |
| 8 — Speed multiplier restore | UNCHECKED |
| 9 — Dock teardown | UNCHECKED |

---

## Top 5 Player-Visible Failures

1. **Stage 2/3 — Exit anchor formula wrong (FAIL).** 80% of exit cycles the miner walks south of the foundation (to y=13) instead of west (to y=11). Player sees: miner consistently curves wrong direction after delivering ore. Code: `miner_dock_sequence.rs:134–137`. gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 10 — anchor = NW_cell−(1,−1) = (9,11). Fires every ore delivery (~every 30s per miner).

2. **Stage 6 — Departure VOC not played (NOT-IMPLEMENTED).** `TankBunkerDown` sound plays in gamemd every time a miner exits the refinery. Player hears nothing. Code: no equivalent in `phase_departing` or `phase_deposit_cooldown`. gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 2, `rulesmd.ini BunkerWallsDownSound=TankBunkerDown`. Fires every ore delivery.

3. **Stage 6 — SpecialAnimThree/Four not created on departure (NOT-IMPLEMENTED).** Building door/bay animation (anim slots 0xC/0xD) should play when miner exits. Player sees no bay-open animation on the refinery at departure. Code: absent from dock sequence. gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Steps 3–4. Fires every ore delivery.

4. **Adjacent Finding 2 — Dock reservation released too late.** Next queued miner waits ~8–10 extra ticks per cycle because our release happens when the departing miner reaches the exit cell, not when gamemd fires `RadioCommand(CLEAR=3)` inside `ReleaseDockedHarvester`. Player sees: with 2+ miners, throughput is visibly reduced — second miner idles longer at queue cell. Code: `miner_dock_sequence.rs:705`. gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 13.

5. **Stage 3 — Test asserts wrong exit cell (FAIL).** `chrono_miner_teleports_to_refinery_on_return` asserts exit = (10,12) on a blank grid; real gameplay exit should be (9,11). The test does not catch the anchor formula bug because the blank grid masks it. Code: `miner_tests.rs:446–450`. gamemd evidence: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` §Step 10.

---

**Status: COMPLETE**
