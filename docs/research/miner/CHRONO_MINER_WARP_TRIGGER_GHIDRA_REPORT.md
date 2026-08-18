# Chrono Miner Inbound Warp Trigger — Ghidra Research Report

**Address(es):** `0x007192F0` (`TeleportLocomotionClass::StateMachineTick`, the real
`ILocomotion::Process` slot for Teleport), `0x00719400` (Ghidra label
`TeleportLocomotionClass__InitiateWarp` — **not a real function**, see §1), `0x00718100`
(`TeleportLocomotionClass::HeadToCoord`), `0x004D94B0` (`FootClass::Set_Destination_Internal`),
`0x00741970` (`TechnoClass::Set_Destination`), `0x004B0500` (`DriveLocomotionClass::Process`),
`0x0073E5E0` (`UnitClass::Mission_Harvest`), `0x004D9290` (`FootClass::Mission_Enter`),
`0x004D8FB0` (`FootClass::Receive_Radio`)

**Investigation Mode:** exhaustive-slice, read-only (Ghidra MCP, gamemd.exe, `testProsjekt`,
image base `0x400000`)

**Claimed Scope:** WHERE and under what predicate a chrono miner (`CMIN`, `Teleporter=yes`)
switches from Drive to Teleport locomotion for the inbound (ore→refinery) trip. Confirms
outbound (refinery→ore) never warps. Does NOT re-derive the warp visual pipeline or unload
state machine.

**Trigger:** Prior report `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` proved the
`TechnoClass::Set_Destination` (0x741970) Teleporter predicate does **not** explain the classic
warp for `UnitClass::Mission_Harvest`'s state-2 fallback call, because NavCom (old destination)
is provably NULL at that call site, which always takes the "prefer Drive" default branch. This
report was dispatched to find the actual downstream trigger, per the prior report's own
"UNVERIFIED-pending-reinvestigate" flag pointing at Drive/Teleport locomotion internals.

---

## 1. Verified Facts (HIGH confidence, asm/xref/offset-verified)

### 1.1 The warp fires inside `TeleportLocomotionClass::StateMachineTick` (0x007192F0), NOT a
separate "InitiateWarp" function

`get_function_by_address 0x007192f0` returns body range **0x007192F0 - 0x00719BED**.
`get_function_by_address 0x00719400` (Ghidra label `TeleportLocomotionClass__InitiateWarp`)
returns body range **0x00719400 - 0x0071978F** — entirely contained inside StateMachineTick's
range. `get_xrefs_to 0x00719400` returns **zero references** (no CALL or jump anywhere in the
binary targets 0x719400). `decompile_function 0x00719400` shows classic mid-function-entry
decompiler artifacts (`unaff_ESI`, `unaff_EBP`, `unaff_EBX`, `unaff_retaddr` — uninitialized
register placeholders the decompiler emits when it cannot determine incoming register state
because the address is not a true function entry), and its body is word-for-word identical in
structure and constants (`RulesClass+0xbf8/0xbfc/0xc00` ChronoDelay math, `AnimClass::Constructor`,
`VocClass::PlayAt`, `WarpAttachClass::Detach`, `CrateClass::PickupDispatch`,
`Set_Destination_Internal(0,1)`) to the inline phase-0 code inside `StateMachineTick`'s own
decompile.

**Conclusion:** `0x00719400` is a Ghidra function-boundary/label error — mid-function code
inside `TeleportLocomotionClass::StateMachineTick` was mislabeled as an independent
"InitiateWarp" function. There is no separately-callable "InitiateWarp" — the warp-initiation
logic is inline phase-0 code inside `StateMachineTick`. This is independently corroborated by
a parallel slot in this same swarm run (`CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`,
logged `.swarm-claims.md` 2026-07-19T10:11+0000), which found the same raw byte pattern at
`0x7196e3`/`0x719a44` — both inside the 0x7192F0-0x719BED range — while investigating an
unrelated question, and flagged them as "inside TeleportLocomotionClass__InitiateWarp/
StateMachineTick" without noticing the containment (their find is consistent with, not
contradictory to, this finding).

Active in YR: Yes — StateMachineTick is the live per-tick handler for every active/piggybacked
`TeleportLocomotionClass` instance, and CMIN is a stock YR `Teleporter=yes` unit.

### 1.2 `StateMachineTick` is the true `ILocomotion::Process` slot for Teleport — vtable offset
`+0x40` matches `DriveLocomotionClass::Process` exactly

Verified via `read_memory` dump of both classes' ILocomotion-family vtables and manual
little-endian pointer decode:

- Teleport's vtable base is `0x007F5000` (confirmed: `get_xrefs_to 0x00718100` returns
  `From 007f5044 [DATA]`, and `0x007F5000 + 0x44 = 0x007F5044`). At base+0x40
  (`0x007F5040`) the table holds `0x007192F0` = `StateMachineTick`. At base+0x44
  (`0x007F5044`) it holds `0x00718100` = `HeadToCoord`.
- Drive's vtable base is `0x007E7EB0` (derived from a read window starting at `0x007e7ea0`;
  base confirmed by four independently-matching shared-`LocomotionClass`-baseclass function
  pointers landing at the same relative offsets as Teleport's table: `Link_To_Object`
  `0x0055A710` at both bases+0xC, `Can_Enter_Cell` `0x0055ABF0` at both bases+0x1C,
  `Is_To_Have_Shadow` `0x0055ABE0` at both bases+0x20, and the IUnknown QI/AddRef/Release
  triad occupying both bases+0x0/+0x4/+0x8). At base+0x40 (`0x007E7EF0`) the table holds
  `0x004B0500` = `DriveLocomotionClass::Process` — this exact address was independently
  confirmed via `get_xrefs_to 0x004b0500` → `From 007e7ef0 [DATA]`. At base+0x44
  (`0x007E7EF4`) it holds `0x004AFD40` = `DriveLocomotionClass::Set_Destination`.

**Conclusion:** vtable slot `+0x40` is `ILocomotion::Process` (the per-tick handler) for both
classes; `+0x44` is `ILocomotion::Head_To_Coord` (the movement-command entry point) for both
classes. `DriveLocomotionClass::Process` (0x4B0500, already well-documented in prior reports)
and `TeleportLocomotionClass::StateMachineTick` (0x7192F0) occupy the identical interface slot.
The Ghidra-labeled `TeleportLocomotionClass__Process` (0x718B70) is a **different, unrelated
function** — verified via `get_function_callers name:TeleportLocomotionClass__Process` →
only caller is `TeleportLocomotionClass::HeadToCoord` via a direct `UNCONDITIONAL_CALL` at
`0x7181ac` (not a vtable dispatch), and it is absent from the vtable dump entirely. It is an
internal coordinate/cell-placement helper HeadToCoord calls synchronously, not the per-tick
Process. This is a second, independent label-drift finding in the current Ghidra project.

Active in YR: Yes for both vtables, unconditionally — this is base engine plumbing.

### 1.3 The exact warp-arming predicate inside `StateMachineTick`

`decompile_function 0x007192f0` (full body read, this session):

```
piVar1 = owner;                                  // param_1[2] = owner FootClass/TechnoClass*
if (owner+0x271 != 0 && WarpPhase==0 && owner+0x280==0) { defer to base tick; return; }
if (WarpPhase==0 && owner+0x280 != 0) { WarpPhase = owner+0x280; return; }   // external phase-jump trigger, unrelated to this arming question
if (ChronoInTransit(owner+0x27C) == 0) {
    if (WarpPhase < 1) {                          // WarpPhase == 0
        if (Is_Moving() /* own vtable+0x10 */) {
            // >>> THE ENTIRE WARP-INITIATION SEQUENCE (StopAllTargeting, bullet retarget,
            //     ChronoDelay math from RulesClass+0xbf8/0xbfc/0xc00, owner+0x271=BeingWarped=1,
            //     WarpOut anim, sounds, WarpAttachClass::Detach, CrateClass::PickupDispatch,
            //     Set_Destination_Internal(0,1) clear, WarpIn anim) fires HERE, this tick.
        } else {
            // NOT moving: re-issue Set_Destination_Internal on owner, re-issue own Head_To (+0x48); defer.
        }
    }
}
```

`Is_Moving()` (own vtable+0x10, `TeleportLocomotionClass::Is_Moving @ 0x718080`) reflects
whether `HeadToCoord` (vtable+0x44, `0x718100`) was called on **this Teleport instance** with a
target coordinate different from `g_NullCoord_Teleport_*` — verified via `decompile_function
0x00718100`: on a valid non-null target it writes `*(param_1+0x30) = 1` and stores the pending
coordinate, which is the flag `Is_Moving()` reads.

**Conclusion:** the warp fires on whichever tick Teleport's own `Is_Moving` flag is true AND
`ChronoInTransit==0` AND `WarpPhase==0`. Teleport's `Is_Moving` is set **only** by a call to
Teleport's own `HeadToCoord`, never by Drive's `Set_Destination`.

Active in YR: Yes, unconditionally for any `Teleporter=yes` unit including stock CMIN.

### 1.4 `Set_Destination_Internal` dispatches Head_To_Coord to the ACTIVE locomotor only

`decompile_function 0x004D94B0` (fresh read, this session): the dispatch call is
`(**(code**)(*(int*)param_1[0x19d] + 0x44))(x,y,z)`, i.e. `active_locomotor_vtable+0x44(coord)`,
where `param_1[0x19d]` = `FootClass+0x674` (the active-locomotor pointer, same field the prior
SET_DESTINATION_GATE report identified). Per §1.2, `+0x44` is the `Head_To_Coord` slot for
whichever class is active. There is no second call in this function to a piggybacked/inactive
locomotor's `+0x44`.

**Conclusion:** Teleport's `HeadToCoord` — and hence its `Is_Moving` flag, and hence §1.3's
warp trigger — is invoked **only on ticks where Teleport is already the active locomotor at the
moment `Set_Destination_Internal` runs.** When Drive is active (the normal state during any
ground-drive leg, including the classic long-distance ore→refinery drive), new destinations
route to Drive's `Set_Destination` (0x4AFD40) instead and never touch Teleport's `Is_Moving`.

Active in YR: Yes, unconditionally — this is the single dispatch point for all `Set_Destination`
calls on any `FootClass`.

### 1.5 Chaining to the already-verified flag predicate

Combining 1.3+1.4 with the already-HIGH-confidence `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`
§1.3/1.4 predicate inside `TechnoClass::Set_Destination` (0x741970, re-read in full this
session, decompile matches the prior report's asm-verified predicate exactly): Teleport can only
become active (and thus only then can a subsequent `HeadToCoord` arm the warp) when a
`Set_Destination` call resolves **flag==0**, which requires: **the OLD destination (NavCom,
queried before this call overwrites it) has RTTI==BuildingClass(6) with
`BuildingTypeClass+0x16B3` (`DockUnload=yes`) set, AND the NEW destination is an empty,
unit-free `CellClass`** (`CellClass::FindFirstUnit` returns NULL on it).

**This chain fully explains why `Mission_Harvest`'s own fallback `Set_Destination` call
(state 2, far/refused branch) never produces a warp**: re-verified this session via a fresh
full decompile of `0x0073E5E0` — state 2's entire body, including the fallback-destination
call, is gated by `if (param_1[0x169] != 0) goto default;` (NavCom must already be NULL to
reach the fallback code at all), so the OLD destination passed into that specific
`Set_Destination` call is always NULL, never a `DockUnload` building — flag stays 1
("prefer Drive") every time, exactly as the prior report found.

Active in YR: Yes.

---

## 2. Where the arming condition is structurally plausible (PARTIAL, not fully closed this session)

Tracing forward from §1.5: the flag==0 condition needs an OLD NavCom that is a `DockUnload`
building. Three candidate call sites were located and partially read this session, consistent
with (but not fully proving) the accepted-CAN_DOCK negotiation being where this occurs:

- `FootClass::Mission_Enter @ 0x004D9290` (Mission state 7, entered via
  `Enter_Mission(7,0)` from `Mission_Harvest` state 3 on HELLO acceptance): for a
  `Teleporter=yes` unit whose CAN_DOCK(0x0E) radio to the current NavCom target succeeds, it
  explicitly clears NavCom to NULL then immediately calls `Set_Destination(same_target, 1)`
  again (`param_1[0x169]=0; vtable+0x480(iVar4,1)`) — decompiled this session, but the
  significance of the forced NULL-then-reassert and the role of the literal `1` third argument
  to `TechnoClass::Set_Destination` were not resolved.
- `FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` (MOVE_TO_CELL, the accepted-cell
  payload sent by the refinery per `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`):
  decompiled this session — confirmed it calls `vtable+0x480(payload_cell, 1)`, i.e.
  `Set_Destination(accepted_cell)`. **If** NavCom at that moment is still the refinery
  building (RTTI==6, DockUnload=yes) from an earlier stage, this call satisfies §1.5's flag==0
  predicate exactly and would arm Teleport for a short warp onto the accepted dock pad. This
  was NOT independently confirmed — I did not trace what NavCom holds at this exact call in a
  live sequence.
- `TechnoClass::Set_Destination` (0x741970) itself contains a third, separate mechanism (a
  "Dock= type list re-target" block, later in the function, after the Teleporter block) that
  re-checks the OLD destination against the unit's `Dock=` type list (`TechnoTypeClass+0x3F8`
  count/array) and can redirect `param_2` to a cell near the old dock target when a new HELLO to
  it is refused/busy. This is a distinct code path from the Teleporter piggyback block and was
  read but not fully traced this session; it may be an alternate or additional source of a
  DockUnload-building-typed NavCom.

**These candidates make it structurally very plausible that the observable inbound warp is a
short final-approach hop (staging/queue cell → accepted dock pad), not a single warp spanning
the miner's full ore-to-refinery distance** — since Drive is confirmed (§1.4, and the
already-exhaustive `DRIVELOCOMOTION_BLOCKED_DELAY_TIMER_CHRONO_MINER_GHIDRA_REPORT.md` /
`DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`, both of which
found zero teleport-related calls in Drive's own tick) to be what carries the miner across the
bulk of the return distance, near or far. This reframing is a significant candidate correction
to the project's standing intuition that "far return = the classic long-range warp" — but it is
**not proven** this session; see §5 Remaining Uncertainty. Do not implement around this
reframing without closing the loop.

---

## 3. Inbound-only confirmation (re-verified, HIGH confidence)

Fresh `decompile_function 0x0073E5E0` this session, state 0 (SEEK, outbound refinery→ore):
never calls the `CellClass`-fallback `Set_Destination` pattern that state 2 uses; it calls
`FootClass__Search_For_Tiberium_Short_And_Move` / `FootClass__Search_For_Tiberium_And_Move`
(pathfinding/drive helpers). This reconfirms `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`
§3 and `traces/CHRONO_MINER_ORE_ACQUISITION_WARP_VS_DRIVE_SWARM_20260520_TRACE.md`: outbound
never presents the code shape needed to reach any warp-arming call. Combined with §1's finding
that arming requires a `DockUnload=yes` building as the OLD NavCom (never true when seeking an
ore cell), outbound cannot warp through this mechanism either. **`feedback_chrono_teleport_direction.md`
("Chrono miner warps ONLY inbound... Outbound is a normal drive") is CONFIRMED, not refuted.**

Active in YR: Yes.

---

## 4. Implementation Handoff

- **Verified behavior:** Teleport's per-tick warp check lives in `TeleportLocomotionClass::StateMachineTick`
  (0x7192F0, vtable slot `ILocomotion::Process` +0x40), gated on `ChronoInTransit==0 && WarpPhase==0
  && Is_Moving()`; `Is_Moving` is set only by Teleport's own `HeadToCoord` (vtable +0x44), which
  `Set_Destination_Internal` calls only on the currently-active locomotor.
  **Rust delta:** do not model "chrono miner warps because the new destination is far" as a
  standalone distance check. Model it as: a locomotor-active-state machine (Drive primary during
  ground legs, Teleport becomes active only via the exact flag==0 predicate) where only a
  `HeadToCoord` call made while Teleport is active can arm a warp; the warp then fires on a
  later tick of that active-locomotor's per-tick handler, not synchronously inside
  `Set_Destination`.
  **Surface:** `src/sim/movement/locomotor.rs`, `src/sim/movement/teleport_movement.rs`,
  `src/sim/miner/miner_system.rs`.
  **Acceptance scenario:** a fixture where Drive is active and a new destination is set with
  NavCom==NULL must NOT arm any teleport, regardless of distance to the new destination — only
  Drive should receive the movement command and no `Is_Moving`/warp-phase state should be set on
  the (piggybacked) Teleport locomotor.
  **Test name suggestion:** `set_destination_with_null_navcom_never_arms_teleport_regardless_of_distance`.
  **Risk:** high — this directly overturns the "long-range Set_Destination call = warp" framing
  present in several older docs (`CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14/§21,
  `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3-4, `CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`
  §1's "the Teleporter decision skips Drive piggyback and the miner warps" line) and the current
  Rust miner architecture's "far-return teleport helper" framing described in
  `CHRONO_MINER_LOCOMOTION_ARCHITECTURE_SYSTEM_MODEL_SYNTHESIS.md` §4B/§6. Do not patch Rust
  from this report alone — §2's open link (exactly which call arms Teleport, and whether it is
  a short dock-pad hop rather than a long-range warp) must close first, or the "far distance
  triggers instant warp" player-visible behavior could regress if it turns out to be correct
  after all and this report's candidate reframing is wrong.

- **Verified behavior:** `TeleportLocomotionClass__InitiateWarp` (0x719400) and
  `TeleportLocomotionClass__Process` (0x718B70) are both Ghidra label/identity errors — neither
  is a real, independently-callable per-tick warp entry point.
  **Rust delta:** none directly (Rust does not reference these addresses), but any future
  research citing "InitiateWarp is called from X" is citing a phantom function; redirect such
  citations to `StateMachineTick` (0x7192F0) phase-0 inline code.
  **Surface:** `docs/research/` corpus only.
  **Acceptance scenario:** N/A (documentation-hygiene finding).
  **Test name suggestion:** N/A.
  **Risk:** low direct risk, but citing 0x719400 as a caller-having function will keep
  producing "No callers found" dead ends for future RE sessions until corrected.

- **Verified behavior:** outbound (refinery→ore) structurally cannot reach any warp-arming call;
  chrono warp is inbound-only.
  **Rust delta:** none required; re-confirms existing project fact
  (`feedback_chrono_teleport_direction.md`).
  **Surface:** N/A (confirmation only).
  **Risk:** none.

---

## 5. Negative Facts / Do Not Do

- Do NOT cite `0x00719400` as a real function or claim anything "calls InitiateWarp." It has
  zero xrefs and its claimed body is fully inside `StateMachineTick`'s body — verified via
  `get_function_by_address` on both addresses plus `get_xrefs_to 0x00719400`.
- Do NOT cite `0x00718B70` (Ghidra label `TeleportLocomotionClass__Process`) as the per-tick
  `ILocomotion::Process` for Teleport. It is called only synchronously by `HeadToCoord`
  (0x7181ac, direct call) and is absent from the ILocomotion vtable at `0x007F5000`. The real
  per-tick slot is `StateMachineTick` (0x7192F0), verified via exact vtable-offset match
  (+0x40) against `DriveLocomotionClass::Process`.
  address instead.
- Do NOT model "new destination is an empty cell and current locomotor is Teleport" as
  sufficient to warp on any Set_Destination call reached with NavCom==NULL — the SET_DESTINATION_GATE
  report and this report both independently confirm NavCom==NULL always takes the
  "prefer/become Drive" branch, never "keep Teleport."
  address is 0x741970's asm-verified default.
- Do NOT treat `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14/§21's or
  `CHRONO_MINER_SYSTEM_OVERVIEW.md` §3-4's "empty cell → FindFirstBuilding NULL → stays
  Teleport → warps" framing as current ground truth. That framing was already refuted for the
  `Set_Destination`-predicate level by `CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`, and
  this report additionally shows the ACTUAL warp-arming step (Teleport's own `HeadToCoord`) is
  gated on Teleport already being active, which the predicate only grants under the narrow
  flag==0 condition — not "any empty-cell destination."
- Do NOT implement a Rust fix around the §2 "short dock-pad hop" reframing yet — it is a
  plausible, structurally-consistent hypothesis based on partial tracing (Receive_Radio case
  0x12 confirmed calling Set_Destination on the accepted cell; Mission_Enter's Teleporter-specific
  NavCom-clear-and-reassert pattern confirmed to exist), not a closed proof of what NavCom holds
  at that exact moment in a live sequence.

---

## 6. Remaining Uncertainty

- Exactly which call in the accepted-CAN_DOCK sequence (Mission_Enter's initial CAN_DOCK
  negotiation, vs. the `0x12` MOVE_TO_CELL receive handler, vs. the `TechnoClass::Set_Destination`
  "Dock= list re-target" block) is the one whose OLD NavCom is a `DockUnload` building at the
  moment it fires, and therefore actually resolves flag==0 and arms Teleport. This requires
  either a live NavCom-value trace across the Mission_Harvest state-3 → Mission_Enter →
  Receive_Radio(0x12) sequence, or careful step-by-step disassembly of `FootClass::Mission_Enter`
  (0x4D9290)'s `Filter_AbstractType_InMap()` and radio-0xE negotiation branches (only partially
  read this session — the "else" branch covering the CAN_DOCK negotiation and its Teleporter
  special case were read; the `Filter_AbstractType_InMap()`-driven branch and the "Dock= list
  re-target" block's exact interaction with the Teleporter block were not fully resolved).
- Whether the observable chrono warp is a short hop (staging cell → accepted dock pad, a few
  cells) or can still span the full ore-to-refinery distance in some path not found this
  session. Given Drive is confirmed to carry the miner for the entire distance whenever NavCom
  is NULL at Set_Destination time (§1.5), and no code path was found in this session's traversal
  of `Mission_Harvest`, `Mission_Enter`, `Receive_Radio(0x12)`, or `TechnoClass::Set_Destination`
  that assigns a DockUnload-building NavCom followed immediately by a *distant* empty cell, the
  short-hop hypothesis is currently better supported by the evidence gathered — but this is not
  a completed proof, and the RulesClass+0xbf8/0xbfc/0xc00 ChronoDelay-timer math inside
  `StateMachineTick` (min-delay clamps, distance-scaled lock timer) does support warps of
  arbitrary distance mechanically, so a long-range path may simply not have been found yet.
- The role of the literal `param_3=1` argument passed to `TechnoClass::Set_Destination` in both
  `Mission_Enter`'s Teleporter-clear-and-reassert call and `Receive_Radio(0x12)`'s call was not
  resolved — it may gate a "force"/"skip normal negotiation" behavior relevant to the predicate,
  but its consumption inside 0x741970 was not traced byte-for-byte this session.

---

## Sources

- Ghidra (read-only, this session): `get_function_by_address 0x007192f0`, `0x00719400`,
  `0x004b0500`; `get_xrefs_to 0x00719400`, `0x00718100`, `0x004b0500`; `get_function_callers
  name:DriveLocomotionClass__Process`, `name:TeleportLocomotionClass__Process`,
  `name:TeleportLocomotionClass__InitiateWarp`, `name:TeleportLocomotionClass__StateMachineTick`,
  `name:TeleportLocomotionClass__TimerCheck`; `decompile_function 0x007192f0`, `0x00718100`,
  `0x00718b70`, `0x004b0500`, `0x004D94B0`, `0x00741970` (full, fresh reads this session),
  `0x0073E5E0` (full, fresh read this session), `0x004D9290`, `0x004D8FB0`; `read_memory
  0x007f5000` (128 bytes, Teleport ILocomotion vtable), `0x007e7ea0` (128 bytes, Drive
  ILocomotion vtable); `search_functions name_pattern:Locomot`, `name_pattern:FootClass__AI`,
  `name_pattern:Mission_Enter`.
- `docs/research/miner/CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md` (primary upstream
  finding this report extends).
- `docs/research/miner/DRIVELOCOMOTION_BLOCKED_DELAY_TIMER_CHRONO_MINER_GHIDRA_REPORT.md`,
  `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`
  (exhaustive prior scans of Drive's own tick that found no teleport-related calls, consistent
  with this report's finding that Drive never arms Teleport).
- `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`,
  `docs/research/miner/CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`,
  `docs/research/miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md` (dock-sequence
  context for §2's candidate call sites).
- `docs/research/miner/CHRONO_MINER_LOCOMOTION_ARCHITECTURE_SYSTEM_MODEL_SYNTHESIS.md`,
  `docs/research/miner/CHRONO_MINER_LOCOMOTION_DISCREPANCY_MAP_20260525.md`,
  `docs/research/miner/traces/CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md`,
  `docs/research/miner/traces/CHRONO_MINER_ORE_ACQUISITION_WARP_VS_DRIVE_SWARM_20260520_TRACE.md`
  (prior synthesis/trace corpus consulted; several superseded framings flagged in §5).
- `docs/research/.swarm-claims.md` (parallel-slot corroboration of the 0x7196e3/0x719a44
  byte-range containment inside StateMachineTick, `CHRONO_WARP_TECHNOCLASS_0X218_READER_GHIDRA_REPORT.md`,
  logged 2026-07-19T10:11+0000).
- `ini/rulesmd.ini:7396` (`[CMIN] Teleporter=yes`), `:11726`/`:12519` (`DockUnload=yes`).

**Status: PARTIAL.** §1 (the warp-arming mechanism: StateMachineTick, the vtable-slot identity
correction, the Head_To_Coord/active-locomotor dependency, and the chained explanation of why
Mission_Harvest's direct call can't arm it) is COMPLETE at HIGH confidence, asm/xref-verified.
§2 (which exact call in the accepted-dock sequence supplies the DockUnload-building NavCom that
satisfies the arming predicate) is NOT closed — flagged UNVERIFIED-pending-reinvestigate with
concrete next-step candidates named.
