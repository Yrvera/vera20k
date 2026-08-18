# Chrono Miner Pad Pivot to East — Trace Report

**Mechanic:** Linked → Pivoting → Unloading visual sequence  
**Scenario:** CMIN arrives on pad cell (13,11) of 4×3 GAREFN at (10,10) facing ~North (0x00).
`RefineryDockPhase::Linked` fires; body pivots to DOCK_FACING_EAST (0x40 8-bit); then Unloading.  
**Date:** 2026-05-20  
**Ghidra:** OFFLINE — doc-only. Citations from verified research docs.  

---

## Supersession 2026-05-26

This trace is stale for the stock YR unload-start facing contract. Its criticism
of Rust's direct `entity.facing = 0x40` snap remains valid, and radio `0x16`
still must not be modeled as direct unload-start or a body-facing write.

However, the stronger claim below that "the pivot does not exist in gamemd" is
not current. Later Ghidra-backed verification shows a real deploy-facing gate in
`UnitClass::Mission_Deploy_Building @ 0x0073D630`: before setting the unload
render latch, stock samples `RateTimer::Current(Unit+0x388)`, accepts only the
east-window expression `((current >> 7) + 1) & 0x1FE == 0x80`, and when not
ready can call active locomotor vtable `+0x4C(0x4000)` before returning mission
delay `5`.

Use `MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`
as the current authority. Correct Rust behavior is not "no facing gate"; it is
"no radio-`0x16` body snap, but mission `0x10` must gate unload-start on the
stock RateTimer/locomotor-facing state."

---

## Correction 2026-07-12 — exit mechanism, drain granularity, ROT value (re-audit)

Fresh live decompiles this session (`decompile_function(0x0073D630)` =
`UnitClass::Mission_Deploy_Building`, `decompile_function(0x004595C0)` =
`BuildingClass::ReleaseDockedHarvester`) independently confirm four more errors
in this trace beyond the 2026-05-26 facing-gate correction above:

1. **`ReleaseDockedHarvester`/`Force_Track(0x47)` is CONDITIONAL, not normal
   stock exit.** `Mission_Deploy_Building`'s outer gate is
   `if (param_1[0xB9] == 0) { <SizeLimit/harvester-dump path, incl. state 4
   via LAB_0073d672> } else { <call vtable+0x1BC, look up building, and only
   if found call BuildingClass::ReleaseDockedHarvester()> LAB_0073d672: <same
   harvester-dump path> }`. State 4 (the zero-link completion path reached via
   `goto LAB_0073d672` or by falling through the `param_1[0xB9]==0` branch)
   only clears `+0x6D1` and queues mission `10`/Harvest — it never calls
   `ReleaseDockedHarvester` or `Force_Track`. `ReleaseDockedHarvester` fires
   only when `param_1[0xB9]` (`unit+0x2E4`) is ALREADY non-zero at function
   entry, i.e. a reciprocal-link/interrupt precondition, not the normal
   drain-to-completion path. This matches (and is now independently confirmed
   from the binary, not just cited from) `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
   §2. See correction to the CRITICAL PRE-FINDING and Check 5 below.
2. **gamemd drains one whole resource-type SLOT per 14.4-frame threshold
   crossing, not one bale.** Inside the state-3 dump block, on threshold
   crossing: `iVar3 = StorageClass::FindFirstNonEmptySlot(); amount =
   StorageClass::GetAmount(iVar3); StorageClass::RemoveAmount(amount, iVar3)`
   — the removed amount IS the full current amount in that slot, not a
   fixed bale increment. A cargo of pure ore therefore produces exactly ONE
   `SetAnimSlotImage(10, …)`/particle-burst tick for the whole load, and mixed
   ore+gems produces exactly two (one per slot type), not one per bale. This
   contradicts Check 3's and Check 4's "gamemd drains one bale per 14.4-frame
   cycle" framing (sourced from REFERENCE_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md,
   not independently checked in this trace until now). Current Rust
   (`phase_unloading`, `src/sim/miner/miner_dock_sequence.rs:1165-1294`,
   read 2026-07-12) already implements whole-slot draining per crossing and
   documents this in its own comments — this trace's "bale granularity wrong"
   verdict on that specific point is itself wrong. See corrections to Checks
   3 and 4 below.
3. **CMIN's `ROT` is 5, not 10, and there is no `Harvester=yes forces ROT=10`
   rule.** `ini/rulesmd.ini:7378` reads `[CMIN] ROT=5`. Current
   `src/rules/object_type.rs:986` reads `turret_rot:
   section.get_i32("ROT").unwrap_or(0)` — a direct INI passthrough with no
   Harvester-specific override anywhere in the file. `dock_pivot_rot_byte()`
   in `miner_dock_sequence.rs:81-86` reads this same field, falling back to
   `10` only when the object type lookup itself fails (not applicable to
   CMIN). See correction to Check 1 below.
4. **Check 1's own arithmetic has an independent ×10 error.** Even taking the
   (wrong) `rot=10` input at face value, `10 × 256 × 15 × 66 = 2,534,400`, not
   `25,344,000` as stated — the doc's numerator is off by exactly a factor of
   10. `2,534,400 / 360,000 = ceil(7.04) = 8` units/tick, not `71`. Verified
   directly from the current `rot_to_facing_delta` formula in
   `src/sim/movement/turret.rs:33-44` (read 2026-07-12): `numerator = rot ×
   256 × 15 × tick_ms`, `denominator = 360 × 1000`, `delta =
   numerator.div_ceil(denominator)`. See correction to Check 1 below.

Root causes: (1)/(2) are `INFERENCE_HARDENED` findings from a sibling doc that
this trace cited without independent binary verification; (3) is a fabricated
INI/Rust claim not grounded in `ini/rulesmd.ini` or the current source
(no taxonomy label fits cleanly — closest is `OFFSET_RETYPED_WRONG` applied to
an INI key rather than a struct offset); (4) is a hand-computed arithmetic slip
(`OPERATOR_OR_ORDER_DRIFT` in the multiplication step), the exact class of
error CLAUDE.md's "golden values are machine-derived, never hand-computed"
rule exists to prevent.

Rust-side line citations throughout this trace (`miner_dock_sequence.rs:427-459,
484, 486-488, 491-492, 494, 573, 612`) are STALE — the file has been
substantially rewritten since 2026-05-20/26. As of 2026-07-12 the relevant
logic lives at: `dock_pivot_accepts`/`dock_pivot_rot_byte` (lines 77-86),
`sync_dock_facing` (1085-1108), `phase_pivoting` (1131-1163), `phase_unloading`
(1165-1294, includes the `BaleDepositEvent` push at 1277), `DOCK_FACING_EAST`
constant (52-53). The `RefineryDockPhase` enum no longer has `Linked` (aliased
to `FaceSync` for old saves) but still has `Pivoting`, whose doc comment now
already describes the rate-timer gate model, not a body-facing snap to
`DOCK_FACING_EAST` — current Rust behavior should NOT be assumed to still
match this trace's Check 1/2 "Our code" descriptions without a fresh read.

---

## CRITICAL PRE-FINDING: The Pivot Does Not Exist in gamemd

**DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §4.3 and §8 (verified 2026-05-19):**

> "No facing is set at any point during the dump phase. The unit's facing is whatever it was when
> it arrived at the dock cell. … No 'pivot to dock facing' operation exists in the binary. The
> unit just stops wherever it is, facing whatever direction it arrived from."

gamemd's sequence at the pad is:
1. Locomotor `Power_Off()` — unit stops, facing unchanged from approach direction.
2. `Mission_Deploy_Building` runs; dump timer initializes (step counter 0, CDTimer 1/1).
3. Dump loop fires SpecialAnim (slot 10 = `GAREFNOR`) on each ~14.4-frame threshold
   crossing (corrected 2026-07-12: one crossing drains one whole resource-type slot,
   not one bale — see "Correction 2026-07-12" above; `decompile_function(0x0073D630)`).
4. On dump complete (state 4): unit clears `+0x6D1` and queues mission `10`/Harvest.
   `ReleaseDockedHarvester`/`Force_Track(0x47, …)` is NOT part of this normal path — it
   is a conditional reciprocal-link helper, called only when `unit+0x2E4` is already
   non-zero at `Mission_Deploy_Building` entry (corrected 2026-07-12: was "on dump
   complete... at exit time"; binary shows the zero-link state-4 branch never calls it —
   `decompile_function(0x0073D630)`, `decompile_function(0x004595C0)` — root cause
   INFERENCE_HARDENED, see correction section above).

The facing `0x4000` mentioned in `HARVESTER_DOCK_UNLOAD.md §radio 0x16 "FACE_DOCK"` is
**wrong** — corrected by DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §9:
- Command 0x16 is TIMING_SYNC, not FACE_DOCK.
- `0x4000` is a RateTimer value passed to `locomotor->SetSpeed()` — not a facing angle.
- No facing write exists anywhere in the TIMING_SYNC handler.

(corrected 2026-07-18: the last two bullets are WRONG. Live decompile of
`UnitClass::Receive_Radio` case `0x16` (`decompile_function(0x00737430)`) shows: `if
(RateTimer::Current() != 0x4000) { locomotor_vtable[0x4C](locomotor, 0x4000); return 1; }` —
the identical vtable slot and literal argument `0x4000` that `Mission_Deploy_Building`'s
east-window gate also calls. Resolving that slot: `get_xrefs_to(0x007e7eb0)` (the
`DriveLocomotionClass` vtable base, confirmed via its Constructor/Destructor/Load install
sites) shows offset `+0x4C` (`0x007e7efc`) points to `DriveLocomotionClass::Do_Turn`
(`0x004b0ef0`), confirmed the reverse direction via `get_xrefs_to(0x004b0ef0)`. `Do_Turn`
(`decompile_function(0x004b0ef0)`) is a trivial one-line wrapper for `RateTimer::Set`
(`0x004c9220`, the same RateTimer/FacingClass interpolation-timer setter documented in
`TIMER_CLASSES_AND_ZONE_MAP_GHIDRA_REPORT.md`), not `SetSpeed()`. So radio `0x16` DOES write
toward a facing-turn target of `0x4000` (East in 16-bit facing convention, i.e. `0x40` in the
8-bit convention used elsewhere in this doc) via the locomotor's turn primitive — this
independently reproduces the 2026-05-22 verify-doc-swarm finding already on record in
`AUDIT_LOG.md` for this file ("UnitClass::Receive_Radio 0x16 ... call[s]
DriveLocomotionClass::Do_Turn(0x4000)") that the 2026-07-12 correction pass did not carry into
this specific paragraph. ROOT_CAUSE: INFERENCE_HARDENED — the "SetSpeed()"/"no facing write"
claims were inherited from a sibling doc's reading of DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md
§9 without confirming the vtable+0x4C callee identity.)

The `DOCK_FACING_EAST = 0x40` constant in our code and the entire Pivoting phase are
**fabricated behavior with no gamemd counterpart.** (Also superseded — see "Supersession
2026-05-26" at the top of this doc and the correction directly above: radio `0x16` and
`Mission_Deploy_Building` both drive the locomotor toward the same `0x4000` facing-turn target,
so "fabricated with no gamemd counterpart" does not hold for the RateTimer-gate mechanism
itself. The narrower claim that current Rust's specific `entity.facing = DOCK_FACING_EAST`
snap and a standalone `Pivoting` phase are not exact matches for that gate model may still be
real drift — see Check 1/Check 2, themselves already flagged as needing a fresh re-check.)

---

## Check 1 — Body Sprite Rotation Cadence

**Claim in our code (`miner_dock_sequence.rs:484`):**
```rust
let max_delta: u8 = turret::rot_to_facing_delta(rot, SIM_TICK_MS);
```
where `rot = obj.turret_rot.max(1)` and for CMIN:
- (corrected 2026-07-12: was "`rules/object_type.rs:861`: `turret_rot = 10` (because
  `Harvester=yes` forces ROT=10)" — no such override exists. `ini/rulesmd.ini:7378` reads
  `[CMIN] ROT=5`; current `src/rules/object_type.rs:986` reads `turret_rot:
  section.get_i32("ROT").unwrap_or(0)`, a direct passthrough with no Harvester-specific
  case anywhere in the file — read 2026-07-12, root cause: fabricated INI value not
  grounded in `ini/rulesmd.ini`)
- `SIM_TICK_HZ = 15`, so `SIM_TICK_MS = 66` ms

**Our `rot_to_facing_delta(5, 66)` computation (`src/sim/movement/turret.rs:40-42`,
read 2026-07-12):**
```
numerator = 5 × 256 × 15 × 66 = 1,267,200
denominator = 360 × 1000 = 360,000
delta = ceil(1,267,200 / 360,000) = ceil(3.52) = 4 units/tick
```
(corrected 2026-07-12: the original computation used `rot=10` and additionally had an
independent ×10 arithmetic error — `10 × 256 × 15 × 66 = 2,534,400`, not `25,344,000` —
giving a wrong `delta=71`; the correct multiplication even at `rot=10` is `delta=8`.
Root cause: hand-computed arithmetic, `OPERATOR_OR_ORDER_DRIFT` in the multiplication step.)

This pivots from 0x00 to 0x40 (= 64 facing units) over `ceil(64/4) = 16` ticks
(~1.07s at 66ms/tick) — a gradual, player-visible rotation, not an instant snap.

**gamemd behavior:** superseded 2026-05-26 (see top of doc) — there IS a facing/rate-timer
gate in `Mission_Deploy_Building` before unload-start, but it does not directly write a body
facing byte; the visible `CMON`/`HORV` unloading voxel instead follows live locomotor facing
at draw time. "No body rotation occurs at all during dock" is too strong; see
`MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`. Separately
(corrected 2026-07-12, see "Correction 2026-07-12" section above): facing is NOT normally
changed at exit via `Force_Track(0x47)` either — that call is conditional/reciprocal-link
only, not the normal stock completion path (`decompile_function(0x0073D630)`,
`decompile_function(0x004595C0)`).

**Verdict: FAIL, but not for the reason stated.** The `rot=10`/×10-arithmetic-error version
of this check overstated the pivot as an instant 1-tick snap; with the corrected `ROT=5` input
and corrected arithmetic, Rust's `sync_dock_facing`/`phase_pivoting` (current file, see
"Correction 2026-07-12" above) already models a gradual multi-tick facing convergence rather
than an instant snap, which is structurally closer to (though not proven identical to) the
gate-then-live-locomotor-facing model in the superseding Ghidra report. This check's original
"snaps immediately" framing and its `ReleaseDockedHarvester`-at-exit comparison point are both
wrong; a fresh Check 1 against current Rust and against
`MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md` is needed and is
out of scope for this correction pass.

**Evidence:** DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §4.3, §8 TICK-N+1 block (superseded,
see top-of-doc correction); `decompile_function(0x0073D630)` (2026-07-12, this session);
`ini/rulesmd.ini:7378`; `src/rules/object_type.rs:986`; `src/sim/movement/turret.rs:33-44`.

---

## Check 2 — Pivot Exit Condition / Snap Rule

**Our code (`miner_dock_sequence.rs:486-488`):**
```rust
let diff: i16 = turret::shortest_rotation(entity.facing, DOCK_FACING_EAST);
if diff.unsigned_abs() <= max_delta as u16 {
    entity.facing = DOCK_FACING_EAST;  // snap to 0x40
    entity.facing_target = None;
    snap.miner.unload_timer = (config.unload_tick_interval as i16).saturating_sub(10);
    snap.miner.dock_phase = RefineryDockPhase::Unloading;
}
```

`shortest_rotation(current, target)` correctly wraps into −128..127 range (`turret.rs:18-28`).
The snap rule (`abs(diff) ≤ max_delta`) correctly avoids overshoot — no error here on the
mechanics of the rotation function itself.

**gamemd behavior:** No snap-to-East exists. The transition from "stopped on pad" to "dumping"
is gated on the RateTimer reaching 0x4000 (locomotor arrival sync), not on body facing.
See DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §3.2.

**Verdict: FAIL — the condition being evaluated is wrong.** The math of `shortest_rotation`
and the overshoot-avoidance are mechanically correct, but the entire check gates the wrong
transition. gamemd transitions via rate-timer gate, not via facing gate.

**Evidence:** DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §4.2 (`if RateTimer != 0x4000:
return 5`), §8 TICK-N+2+ block.

---

## Check 3 — Unload Start: Animation Slot and Timing

**Our code (`miner_dock_sequence.rs:573`):**
```rust
sim.bale_events.push(BaleDepositEvent { building_id: ref_sid, tick: sim.tick });
```
One `BaleDepositEvent` fires per slot drain (per ResourceType, not per bale), which is intended
to trigger slot 10 (`SpecialAnim`) on the building side.

**gamemd behavior (REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Trigger 2; "per-bale"
corrected 2026-07-12 to "per-slot-drain" — see "Correction 2026-07-12" above):**
- Slot 10 (`SpecialAnim` = `GAREFNOR`) fires on every 14.4-frame threshold crossing via
  `BuildingClass__SetAnimSlotImage(10, low_health, 0)` inside `Mission_Deploy_Building` state 3
  (corrected 2026-07-12: not "on every bale" — each crossing drains one whole resource-type
  slot via `StorageClass::GetAmount`/`RemoveAmount` with the same full amount, verified via
  `decompile_function(0x0073D630)`; a pure-ore cargo produces exactly ONE such tick for the
  whole load, not one per bale).
- The particle emitter (`vtable+0x468`) fires BEFORE `SetAnimSlotImage(10)` within the same
  per-crossing block — i.e., particles spawn first (ordering itself CONFIRMED unchanged by
  `decompile_function(0x0073D630)`).
- Slot 7 (`PreProductionAnim`) fires one-shot on dock arrival (state 1 first-entry) — no-op
  for GAREFN (undefined in art), but the call happens unconditionally.
- Slot 8 (`ProductionAnim`) fires one-shot on cargo empty — also no-op for GAREFN.

**The unload animation starts the same tick as the transition to Unloading (state 3):**
`Mission_Deploy_Building` checks the per-crossing gate (`HarvesterDumpRate×900 ≤ step_counter`)
in state 3. Since `step_counter = 0` on first entry and the threshold is 14.4, the FIRST slot
drain does NOT fire immediately on the first state-3 tick — the counter must accumulate to 14.4
first.

**REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §8 "tiny detail"** (quoted verbatim from the
sibling doc; its "bale" terminology is the same pre-2026-07-12 framing corrected above —
read as "first slot drain", not literally "first bale"):
> "`unit+0x3E` is reset to 0 on slot-7 init (state 1 first-entry), not just after each bale.
> So the very first bale fires immediately on entering state 3 (counter is 0, gate is 14.4 ≤ 0
> → false). UNVERIFIED — depends on increment site."

Combined with §9.1 (corrected): the accumulator is `unit+0xF8`, incremented every
`field_0x108` frames (= 1 frame, since CDTimer rate is 1). So the first slot drain fires at
frame 15 (≈14.4 rounded up) after entering dump state.

**Our implementation:** We emit a `BaleDepositEvent` per slot (per ResourceType), which
(corrected 2026-07-12) is what gamemd itself does — see "Correction 2026-07-12" above.
`decompile_function(0x0073D630)` shows the state-3 dump block draining the FULL current amount
of one `StorageClass` slot per 14.4-frame threshold crossing (`GetAmount` then `RemoveAmount`
with the same value), not one bale at a time; the next crossing checks
`FindFirstNonEmptySlot` again for the next slot type.

**Verdict: PASS on drain granularity (corrected 2026-07-12) — the original "bale granularity
wrong" finding was itself wrong; current Rust's per-slot `BaleDepositEvent` already matches
the verified gamemd per-slot-crossing drain.** FAIL stands, unverified this session, on the
particle emitter: no per-crossing particle burst implementation was found in
`src/sim/miner/miner_dock_sequence.rs` (only a comment referencing it, line 1276, read
2026-07-12); whether it is implemented downstream in a render/particle module was not checked
in this pass (REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §7 "What's wrong" items 3 and 4).

**Evidence:** REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Trigger 2; DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §8 TICK-N+3+; `decompile_function(0x0073D630)` (2026-07-12, this session); `src/sim/miner/miner_dock_sequence.rs:1165-1294` (read 2026-07-12).

---

## Check 4 — First Bale Credit Timing (interval − 10 initialization)

**Our code (`miner_dock_sequence.rs:494`) — STALE as of 2026-07-12, this code no longer
exists in this form (see "Correction 2026-07-12" above):**
```rust
snap.miner.unload_timer = (config.unload_tick_interval as i16).saturating_sub(10);
```
With `unload_tick_interval = 144` (tenths of a tick), this seeds `unload_timer = 134` tenths.
Decrementing by 10 per tick: first bale at `ceil(134/10) = 14` ticks (not 14.4).

(Corrected 2026-07-12: current `src/sim/miner/miner_dock_sequence.rs` uses
`unload_tick_interval` as a whole-frame count (default `15`, not `144` tenths — see
`src/sim/miner/mod.rs:170-211`, read 2026-07-12) and `start_unload_deploy`
(`miner_dock_sequence.rs:1110-1125`) seeds `snap.miner.unload_accumulator = 0` with no `-10`
or other bias term. The specific "-10 offset" code criticized below no longer exists to be
wrong; whether current Rust implements ANY per-unit desync jitter at all was not re-checked
this session and remains open.)

**gamemd behavior (REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1 corrected finding, NOT
independently verified against `UnitClass::Unlimbo` in this session):**
> "Seed at Unlimbo time: `Random(0, 29)` uniform."

The `unit+0xF8` accumulator is claimed to be seeded with `Random(0, 29)` in
`UnitClass::Unlimbo` — not a fixed `-10` offset. This specific claim is UNVERIFIABLE in this
pass (no live decompile of `UnitClass::Unlimbo` was performed this session); it is carried
forward from the cited sibling doc only.

**The 14.4-frame dump rate itself:**
- gamemd: `HarvesterDumpRate(0.016) × 900.0 = 14.4` frames per **slot-drain crossing**
  (corrected 2026-07-12: not "frames/bale" — see "Correction 2026-07-12" above;
  `decompile_function(0x0073D630)`).
- Our code (stale, see above): `unload_tick_interval = 144` tenths, decrement 10/tick →
  average 14.4 ticks/crossing. Cadence number matches; the "/bale" unit label does not.
- The per-fleet desynchronization seed question (`Random(0, 29)` vs current Rust's `0`) is
  UNVERIFIABLE this session — see above.

**Verdict: PARTIAL — the specific code this check criticizes is STALE and no longer present;
the underlying jitter-seed question is UNVERIFIABLE this session, not confirmed FAIL.**
Do not cite this check's original "-10 offset" description as current Rust behavior.

**Evidence:** REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1 corrected (2026-05-19, not
independently reverified this session); DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §8 timer
initialization block; `src/sim/miner/miner_dock_sequence.rs:1110-1125`, `src/sim/miner/mod.rs:170-211`
(read 2026-07-12).

---

## Check 5 — Bay-Door / Pipes Anim (slots 0xA / 0xB) Retraction

**Task:** Verify slot indices 0xA (10) and 0xB (11), what they look like, when they retract.

**Slot assignment (REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §2):**

| Slot (dec) | Slot (hex) | INI Key | GAREFN art |
|-----------|------------|---------|------------|
| 10 | 0xA | `SpecialAnim` | `GAREFNOR` (per-slot-crossing, ONE-SHOT; corrected 2026-07-12 from "per-bale" — see "Correction 2026-07-12" above) |
| 11 | 0xB | — | undefined for GAREFN |

Slot 11 (`0xB`) is NOT `SpecialAnimTwo` in the standard 21-slot table for GAREFN.
`HARVESTER_DOCK_UNLOAD_SEQUENCE.md §4a State 1 (Undock/Exit)` notes:
> "Clears dock/unload animations (slots 8, 11)."

This refers to `BuildingClass::MissionRepairAndProduce` state 1, which clears slots 8 and 11
on the BUILDING side. `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §5.1` shows that
`ReleaseDockedHarvester` (0x4595C0) clears `ClearAnimSlot(0xA)`/`ClearAnimSlot(0xB)` and sets
`SetAnimSlotImage(0xC/0xD, ...)` — **CONFIRMED by fresh decompile this session
(`decompile_function(0x004595C0)`)** — but (corrected 2026-07-12, see "Correction 2026-07-12"
above) `ReleaseDockedHarvester` itself is called ONLY when `unit+0x2E4` is already non-zero at
`Mission_Deploy_Building` entry, which is NOT the normal stock `CMIN/HARV → GAREFN/NAREFN`
completion path. The normal zero-link state-4 completion (verified via
`decompile_function(0x0073D630)` this session) clears `+0x6D1` and queues mission `10`/Harvest;
it does not call `ReleaseDockedHarvester`, `ClearAnimSlot(0xA/0xB)`, or
`SetAnimSlotImage(0xC/0xD)`. Separately, the state-3 dump loop itself calls
`BuildingClass::ClearAnimSlot(this_00)` (single-argument form, gated on
`this_00->field_0x584 != 0`) at both the cargo-empty transition and the mission-changed-away
transition — this is a DIFFERENT call from `ReleaseDockedHarvester`'s explicit
`ClearAnimSlot(0xA)`/`ClearAnimSlot(0xB)` pair, and its exact slot-argument semantics were NOT
decoded this session (UNVERIFIABLE — the implicit second argument was not traced).

For GAREFN specifically, slot 0xB (11) has no defined art in stock artmd.ini —
`REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §5` does not list slot 11 with any INI key.
The note in the original question ("bay-door / pipes-down anim slot doesn't yet play")
appears correct — for stock GAREFN, slot 11 is undefined; whatever clears it, the clear is a
no-op for GAREFN's own art either way.

**What slot 0xA (10 / SpecialAnim) looks like during unloading:**
`GAREFNOR` — 20-frame one-shot at 200ms/frame (~4s total). Restarts every ~0.96s (14.4
frames) per slot-drain crossing (corrected 2026-07-12: not "new bales deposit" — a pure-ore
cargo produces exactly one restart for the whole load; see "Correction 2026-07-12" above).
Player sees the opening frames only when a second crossing preempts a still-playing anim;
after the last crossing the animation plays to completion (wind-down).

**When they retract (corrected 2026-07-12 — this is the CONDITIONAL reciprocal-link path,
NOT normal stock exit; see above):**
- Slot 10 (`GAREFNOR`): if the reciprocal-link path fires, cleared by `ClearAnimSlot(0xA)` in
  `ReleaseDockedHarvester`. For the NORMAL stock zero-link completion, no `ClearAnimSlot(0xA)`
  call was found in the verified state-4 path this session — the anim's fate on ordinary
  departure is UNVERIFIABLE from this pass (the `field_0x584`-gated single-arg
  `ClearAnimSlot(this_00)` call noted above may or may not cover it; not decoded).
- Slot 11: same caveat — only confirmed cleared via the conditional `ReleaseDockedHarvester`
  path (no-op for GAREFN's own art regardless).

**Our implementation status:**
- No slot-clearing calls implemented in `phase_departing` (current file, `miner_dock_sequence.rs`,
  read 2026-07-12) — given the correction above, this may now be closer to correct for the
  NORMAL stock path than this check originally assumed, since normal stock exit itself does not
  clearly call `ReleaseDockedHarvester`'s `ClearAnimSlot` pair either. Re-verification against
  the actual normal-path anim-clear mechanism (the `field_0x584`-gated call) is needed and is
  out of scope for this correction pass.
- `display_type_override` (UnloadingClass swap) is cleared in `phase_departing`
  (current file: `miner_dock_sequence.rs:1330`, read 2026-07-12; original citation
  "phase_deposit_cooldown line 612" is STALE — that phase is now a legacy pass-through, see
  "Correction 2026-07-12" above) — timing appears correct (before departure), but the
  building-side anim state synchronization is unverified this session.

**Verdict: PARTIAL, downgraded from FAIL (corrected 2026-07-12).** The original verdict assumed
`ReleaseDockedHarvester`'s anim-slot behavior IS the normal stock departure contract; that
premise is wrong (see above). What the normal stock departure actually does to slots 10/11 is
UNVERIFIABLE from this pass and needs a dedicated decode of the `field_0x584`-gated
`BuildingClass::ClearAnimSlot(this_00)` call before either a PASS or FAIL verdict can be given
for current Rust's `phase_departing`.

**Evidence:** REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Triggers 2/3; §5;
DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §5.1 steps 1-2 (superseded for "normal exit"
framing, see top-of-doc and "Correction 2026-07-12"); `decompile_function(0x0073D630)`,
`decompile_function(0x004595C0)` (2026-07-12, this session); `src/sim/miner/miner_dock_sequence.rs`
(read 2026-07-12).

---

## Summary Table

| Check | Result | Player-observable effect |
|-------|--------|--------------------------|
| 1. Body rotation cadence (ROT=5, corrected 2026-07-12) | **FAIL, but magnitude/verdict revised** — see Check 1 correction | Gradual ~16-tick (~1s) rotation, not the originally-claimed instant snap |
| 2. Pivot exit condition / snap rule | **FAIL** (unchanged; covered by top-of-doc 2026-05-26 Supersession) | Phase gate model wrong in detail; see current authority doc |
| 3. Unload anim slot start timing | **PARTIAL, revised 2026-07-12** — PASS on per-slot drain granularity (original "bale granularity wrong" was itself wrong), particle-burst implementation status unverified | GAREFNOR fires per slot-drain crossing, not per bale; particle-burst gap unconfirmed |
| 4. First bale credit timing (jitter seed) | **PARTIAL/UNVERIFIABLE, downgraded 2026-07-12 from FAIL** — the criticized `-10` code is STALE/gone; the `Random(0,29)`-at-Unlimbo claim itself was never independently verified | Unknown; open question, not a confirmed bug |
| 5. Bay-door/pipes anim slots 0xA/0xB | **PARTIAL, downgraded 2026-07-12 from FAIL/UNCHECKED** — normal-exit anim-clear mechanism misidentified; correct mechanism UNVERIFIABLE this pass | Unknown; needs a dedicated decode of the `field_0x584`-gated `ClearAnimSlot` call |

**Verdict tally (2026-05-20 original): PASS: 0 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0**
**Verdict tally (revised 2026-07-12, see "Correction 2026-07-12" section above): PASS: 0 |
FAIL: 2 (Checks 1, 2) | PARTIAL: 3 (Checks 3, 4, 5) | UNCHECKED: 0 | NOT-IMPLEMENTED: 0.** This
revised tally reflects only what was directly re-verified this session; it is not a fresh
end-to-end re-audit of current Rust and should not be read as final.

---

## Top 5 Player-Visible Failures

1. **Miner body pivots East during dock (stage: Linked→Pivoting→Unloading)**  
   Player sees: chrono miner body slowly or instantly rotates east after arriving on pad.
   gamemd: superseded 2026-05-26 — a real facing/rate-timer gate exists, live locomotor facing
   drives the visible `CMON`/`HORV` render; see top-of-doc Supersession and Check 1 correction.
   File: STALE citation — `phase_linked`/`miner_dock_sequence.rs:427-459` no longer exist;
   current equivalents are `phase_face_sync` (~1035), `phase_mission_queued` (~1081),
   `phase_pivoting` (~1131), read 2026-07-12.
   Evidence: DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §4.3, §8 (superseded).

2. **No GAREFNOR pipe animation per bale (stage: Unloading)**  
   Player sees: refinery "ore arriving" sprite (orange pipe sequence) never plays during unload.
   gamemd (corrected 2026-07-12): GAREFNOR fires every ~14.4 frames per SLOT-DRAIN CROSSING,
   not per bale — a pure-ore cargo produces exactly one fire for the whole load. See
   "Correction 2026-07-12" above; `decompile_function(0x0073D630)`.
   File: STALE line citation; current `BaleDepositEvent` push is at
   `src/sim/miner/miner_dock_sequence.rs:1277` (read 2026-07-12) and already emits one event
   per slot drain — matching, not contradicting, the corrected gamemd model.
   Evidence: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §3 Trigger 2, §7 (per-bale wording
   corrected).

3. **No smoke particle bursts on slot-drain (stage: Unloading)**  
   Player sees: no smoke puffs from refinery's RefinerySmokeOffset positions during unload.
   gamemd (corrected 2026-07-12): two SmallGreySSys particle bursts fire from symmetric N/S
   offsets on every ~14.4-frame SLOT-DRAIN CROSSING, not "every bale".
   File: not implemented (no particle emission found in `miner_dock_sequence.rs`, read
   2026-07-12; downstream render/particle-module coverage not checked this session).
   Evidence: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §4.

4. **Facing held during dump (stage: Unloading) — re-verification needed**  
   Player sees: unclear without a fresh check. Original claim ("miner body faces East (0x40)
   during dump") cited a hardcoded `entity.facing = DOCK_FACING_EAST` line that is
   STALE — `start_unload_deploy` (`miner_dock_sequence.rs:1110-1125`, read 2026-07-12) does not
   contain that assignment; facing is instead driven through a `FacingClass` pivot structure
   (`sync_dock_facing`, ~1085-1108) whose write-through to `entity.facing` was not traced this
   session. gamemd: per the top-of-doc Supersession, the visible unloading voxel uses LIVE
   locomotor facing at draw time, gated by the rate-timer window, not a hardcoded East value.
   File: STALE citation (`miner_dock_sequence.rs:491-492` no longer exists).
   Evidence: DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md §4.3, Q10 (superseded);
   `MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`.

5. **Per-fleet slot-drain desynchronization seed (stage: Linked→Pivoting transition)**  
   Player sees: unknown — downgraded 2026-07-12 from a confirmed bug to an open question. The
   originally-criticized fixed `-10`/tenths code no longer exists in current Rust (which seeds
   `unload_accumulator = 0` with no bias term, `miner_dock_sequence.rs:1110-1125`); whether
   gamemd's claimed `Random(0,29)`-at-`Unlimbo` seed was ever independently verified from the
   binary is itself unconfirmed this session — see Check 4 correction above.
   File: STALE citation (`miner_dock_sequence.rs:494` no longer exists).
   Evidence: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1 corrected finding (2026-05-19, not
   independently reverified).

---

## Status

**PARTIAL** (original, 2026-05-20) — Ghidra is offline; all findings derived from existing verified docs (DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md 2026-05-19, REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md, HARVESTER_DOCK_UNLOAD.md, HARVESTER_DOCK_UNLOAD_SEQUENCE.md). The core finding — that the entire Pivoting phase is fabricated behavior — is documented with HIGH confidence from the 2026-05-19 Ghidra report. One unchecked item (slot 11 for GAREFN) is irrelevant to player-visible behavior for stock assets.

**Re-audit 2026-07-12 (see "Supersession 2026-05-26" and "Correction 2026-07-12"
sections above):** the "entire Pivoting phase is fabricated" framing is superseded — a real
facing/rate-timer gate exists in `Mission_Deploy_Building`, it is pixel-relevant, and current
Rust already implements the gate formula. The `ReleaseDockedHarvester`/`Force_Track(0x47)`
"normal exit" framing and the "one bale per 14.4 frames" drain-granularity framing were both
directly re-verified as WRONG this session (`decompile_function(0x0073D630)`,
`decompile_function(0x004595C0)`, this session, Ghidra online). CMIN's `ROT` value (5, not 10)
and an independent ×10 arithmetic error in Check 1 were also corrected, both grounded in
`ini/rulesmd.ini` and current Rust source read this session. Most Rust-side line citations in
this trace are STALE — `src/sim/miner/miner_dock_sequence.rs` has been substantially rewritten
since 2026-05-20/26. This trace should not be treated as a current status snapshot of Rust
behavior without a fresh read of the file; treat it as a historical record of what was true in
May 2026, now layered with dated corrections. **Status: PARTIAL (corrected)** — a full fresh
re-trace against current Rust and the now-canonical Ghidra reports
(`MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`,
`DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`) is recommended but out of
scope for this correction pass.
