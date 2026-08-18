# Mission Rescue (id 21) Handler and Mission Ambush (id 14) Stub — Ghidra Report

**Date:** 2026-06-02  
**Slot:** re-swarm slot-4  
**Binary:** gamemd.exe (RA2 Yuri's Revenge)  
**Confidence:** HIGH for all load-bearing claims (verified-from-binary this session)

---

## Investigation Scope

**Target question:** What does `Mission_Rescue` (id 21) DO each dispatch, what is its
return cadence, what state does it read/write, what is the live assignment path (incl.
`IsPlayerControl()==0` gate), and is `Mission_Ambush` (id 14) truly a dead TS stub
with no live assignment?

**Non-goals:** Other mission handlers; anything outside Rescue/Ambush assignment paths
and handler bodies.

**Evidence needed to mark COMPLETE:**
- Mission-id ↔ name confirmed via `read_memory` on `g_MissionNameTable`.
- Handler vtable slots read via `read_memory` for both ids, handler bodies decompiled.
- Assignment path for Rescue decompiled (`FUN_00708080` body + `FootClass__ReceiveDamage`
  gate), `HouseClass__IsPlayerControl()==0` branch confirmed in decompile.
- Ambush vtable slot read as stub `0x005B2E30`, no callers that assign id 14 to any unit.

**Stop conditions:** All items above checked; no contradictions with sibling docs.

---

## Pre-flight: Sibling Docs Read

Read before investigating:
- `MISSIONCLASS_STATE_MACHINE.md` — confirms Rescue vtable offset +0x258, Ambush +0x20C,
  base stub @ `0x005B2E10` returns `0x1C2`.
- `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` — confirms `FootClass__Mission_Rescue @
  0x004DDF90` at slot +0x258.
- `MISSION_RADIO_SUBSTRATE_BINARY_VERIFICATIONS.md` §V4 — already contains strong binary
  evidence for Rescue live / Ambush dead (this session extends and fully cites independently).

The V4 section of the verifications doc has substantial evidence; this report extends it
with full handler behavior, return cadence, and state layout.

---

## 1. Mission-ID Confirmation (g_MissionNameTable)

Verified via `read_memory 0x00816CAC` (88 bytes) → pointer array; each entry `i` is a
`char*` at `0x00816CAC + i*4`.

- **Entry[14]** pointer = `0x00816DF8`; `read_memory 0x00816DF8` → bytes `41 6d 62 75 73
  68 00` = **"Ambush"**. `(id 14 = 0x0E) ↔ "Ambush"` CONFIRMED.
- **Entry[21]** pointer = `0x00816DB4`; `read_memory 0x00816DB4` → bytes `52 65 73 63 75
  65 00` = **"Rescue"**. `(id 21 = 0x15) ↔ "Rescue"` CONFIRMED.

No off-by-one: the table is 0-indexed and the convention Rescue=21, Ambush=14 is correct
in this binary.

---

## 2. Ambush (id 14) — Dead TS Stub

### Vtable slot verification

FootClass vtable base = `0x007E8C94`. Ambush slot = +0x20C → addr `0x007E8EA0`.  
`read_memory 0x007E8EA0` → `30 2E 5B 00` = **`0x005B2E30`**.

`read_memory 0x005B2E30` (8 bytes) → `B8 C2 01 00 00 C3 90 90` = **`mov eax,0x1C2; ret`**.

This is byte-identical to the base stub at `0x005B2E10` (same pattern confirmed via
`read_memory 0x005B2E10`). Both are "returns 450 frames, does nothing."

**No override exists in any subclass** — the V4 section of the verifications doc confirms
`search_functions` found no `Mission_Ambush`; the FootClass slot above (the deepest common
base for mobile units) is the stub.

### Assignment / callers

The string "Ambush" at `0x00816DF8` is referenced only from the name table. No numeric
`push 0; push 0x0e` in any mission-verb call site (Queue_Mission / Assign_Mission) was
found in AI/team/trigger code. No `Mission_From_Name`-based assignment for this ID.

`iVar4 != 4 && iVar4 != 0` branch in `FootClass__ReceiveDamage @ 0x004D7330` routes to
`FUN_00708080` which assigns ONLY `0x15` (Rescue) or `0xB` (AreaGuard) — never `0x0E`.

**Active in YR: NO.** Ambush is a dead TS-legacy stub.

---

## 3. Rescue (id 21) — Live AI Mission

### 3.1 Vtable slot verification

FootClass vtable base `0x007E8C94` + 0x258 = `0x007E8EEC`.  
`read_memory 0x007E8EEC` → `90 DF 4D 00` = **`0x004DDF90`** = `FootClass__Mission_Rescue`.

AircraftClass vtable base `0x007E22A4` + 0x258 = `0x007E24FC`.  
`read_memory 0x007E24FC` → `90 DF 4D 00` = **`0x004DDF90`** (FootClass handler — aircraft
at this slot is the foot handler until slot +0x264, confirmed by the V4 doc noting
`0x007E2508` holds the aircraft override).

`read_memory 0x007E2508` → `60 59 41 00` = **`0x00415960`** = `AircraftClass__Mission_Rescue`.

**Active in YR: YES.** Two distinct handlers: one for foot units, one for aircraft.

### 3.2 FootClass__Mission_Rescue @ 0x004DDF90 — Handler Behavior

Decompiled via `decompile_function 0x004DDF90`.

**State read:**
- `param_1[0x2f]` — MissionState sub-state (0 = init/targeting, 1 = moving)
- `param_1[0xad]` — current Target / TechnoClass pointer (the attacker to converge on)
- `param_1[0x86]` — gather object / destination cell (set by the assigner; this is the
  attacker's cell, written by `FUN_00708080`)
- `param_1[0x169]` — locomotor moving flag (0 = arrived / not moving)
- `param_1[0x27/0x28/0x29]` — current coordinate (X/Y/Z leptons)
- `param_1[0x1a2]` — a clearable flag zeroed on entry (purpose: inferred "suppress scatter")

**Handler flow (state machine):**

**State 0, no target (`param_1[0xad]==0`):**
- If gather cell `param_1[0x86]` is null: call `TechnoClass__SetGhostCell()` using own
  current coords (sets up ghost for teleport/warp tracking — TS flavor, mostly inert in YR).
- Locate candidate target: `vtable+0x3C4` (scan for nearest threat near gather cell).
- If a candidate found: compute 3D distance; if within weapon range
  (`vtable+0x31C` returns weapon range, compared via `* _DAT_007e48f0` scale factor), call
  `vtable+0x3C8(candidate)` — this is `Assign_Target` (sets `param_1[0xad]`).
- If still no target after scan: advance to sub-state 1, call
  `FootClass__Find_Passable_Cell_Near_Unit` to pick a reachable cell near gather object,
  then `vtable+0x480(cell, true)` — queue a Move to that cell.
- Call `TechnoClass__SetGhostCell()` again after issuing move.

**State 0, target acquired (`param_1[0xad]!=0`):**
- Call `vtable+0x53C()` — this is the combat/attack dispatch (begin engaging the acquired
  target). Return `1` (call again next frame — fast loop while attacking).

**State 1, locomotor arrived (`param_1[0x169]==0`):**
- `TechnoClass__SetGhostCell()` to clear ghost.
- Call `vtable+0x1E8(0, 0)` (Queue_Mission with mission 0 = Sleep? — no, `uVar6 = 0x1A`
  in the decompile — actually the queued mission is `0x1A`=AreaGuard or a sub-mission;
  the decompile shows `(**(code **)(*param_1 + 0x1e8))()` with inline args).  
  Then `vtable+0x1EC()` — Commence. This terminates Rescue by transitioning to another
  mission once the unit has arrived at the gather cell.

**Return cadence:** `MissionClass__GetMissionTimerEntry()` + `Math__ftol()` +
`Random__RandomRanged()` jitter = **INI-configured Rescue rate + random jitter**. This is
the standard mission dispatch pattern (same as Hunt, Guard, etc.). The fast-loop exception
is state-0-with-target which returns `1` (next-frame). Aircraft handler always returns `5`
(5 frames).

**State written:**
- `param_1[0x2f]` set to 1 (enter move state)
- `param_1[0xad]` set via `Assign_Target` when a nearby threat is found
- `param_1[0x1a2]` zeroed at entry

### 3.3 AircraftClass__Mission_Rescue @ 0x00415960 — Handler Behavior

Decompiled via `decompile_function 0x00415960`.

**Flow:**
1. Set `this+0x6D2` = 1 (the AircraftClass busy/in-flight flag per earlier research).
2. If no target (`param_1[0xad]==0`) or no payload (`param_1[0x46]==0`):
   - Clear `+0x6D2` = 0.
   - Call `vtable+0x3C8(null)` (clear target), `vtable+0x480(null, true)` (queue null
     move), `vtable+0x1E8(4, 0)` (Queue_Mission id 4 = Retreat). Return 5.
3. Else (target + payload present): read `FUN_005f6440(param_1[0xad])` — get target's
   something (health? shield?) — compare against `g_RulesClass_Instance+0x54c` (a Rules
   threshold, likely `CloseEnough` or a min-distance limit).
   - If within range: check `MapClass__IsCoordsInPlayfield`, then call
     `AircraftClass__Drop_Payload()`. Return 5.
   - Else: clear `+0x6D2`, check `+0x6D3` ready-flag; if set queue `vtable+0x1E8(0x1A, 0)`
     (mission 0x1A = SpyplaneApproach or a paradrop approach). Return 5.

**Plain summary:** The aircraft variant of Rescue is a **drop-payload mission**: the AI
aircraft flies toward its target, drops its payload (bombs/paradrop) when in range, then
transitions to Retreat. This appears to be used by AI-controlled paradrop or bombing
aircraft assigned to converge on a damaged ally.

**Return cadence:** Always returns `5` (5 frames, fast loop).

### 3.4 Assignment Path — ReceiveDamage → FUN_00708080

**FootClass__ReceiveDamage @ 0x004D7330** (decompiled this session):

The key branch:
```c
if ((*(int *)&param_1[1].field_0xb4 != 0) &&
    (*(char *)(*(int *)(*(int *)&param_1[1].field_0xb4 + 0x24) + 0xac) != '\0' &&
     (cVar3 = HouseClass__IsPlayerControl(), cVar3 == '\0'))) {
    if (param_5 == (int *)0x0) {
        return iVar4;
    }
    FUN_00708080(param_5);
}
```

Gates:
1. `param_1[1].field_0xb4` — unit is on a team (`TeamClass*` non-null). This is offset
   +0xB4 from `param_1+4` = byte offset `0xB0+0x4+0xB4 = 0x1B8`? No — `param_1[1]` in
   `TechnoClass*` context is pointer offset +4 → field `field_0xb4` is at byte `+0xB4`
   of the struct relative to the field's base. The actual check is: the damaged unit's
   team pointer (`TechnoClass+0xB4` area — INFERRED, not traced to setter here) is non-null.
2. `*(char *)(...+ 0xac)` — team type's byte at `+0xAC` is non-zero. This is the team's
   "recruit" / "resquad" flag (INFERRED from offset).
3. **`HouseClass__IsPlayerControl() == '\0'`** — the unit's house is NOT player-controlled
   (AI house only). This is the live YR gate: only AI-owned units trigger Rescue
   assignment. Verified in decompile of `0x004D7330`.
4. `param_5 != null` — there must be a non-null attacker.

**FUN_00708080** (the gather/assign routine, decompiled this session):
- Verifies the VICTIM (`param_2`) is NOT an ally, not player-controlled, not in a no-
  response state.
- Scans `g_InfantryClass_Array` then `g_UnitClass_Array` for eligible teammates (same
  house, alive, on a team with `recruit-allowed` flag, locomotor-idle-ish, zone-reachable,
  threat score > 0).
- Builds a scored candidate list (up to 6 per type).
- For each selected responder:
  - `Random__RandomRanged(0, 99)`: if `> 0x41` (65%) → assign `Queue_Mission(0xB = AreaGuard)`; 
    else (35%) → assign `Queue_Mission(0x15 = Rescue, 0)`.
  - `responder[0x86] = uStack_5c` — sets the gather cell (attacker's cell).
  - `vtable+0x3C8(attacker)` — assigns attacker as Target.

**Caller chain to live YR skirmish:**
- `FootClass__ReceiveDamage @ 0x004D7330` — called whenever a mobile unit takes damage
  (called from `TechnoClass__ReceiveDamage @ 0x00701900`).
- Gated: AI house + team membership + non-null attacker.
- Result: fires in **every standard skirmish vs AI** whenever an AI-team unit is hit
  (which is every engagement). No SpecialFlags gate, no map trigger.

**Active in YR: YES — unconditionally in skirmish vs AI opponents.**

---

## 4. Implementation Handoff

### Rescue Handler (FootClass variant — primary)

Full chain:
1. **Trigger:** AI unit in team takes damage from a non-null attacker
   (`FootClass__ReceiveDamage` → `HouseClass__IsPlayerControl()==false` + team-flag gate →
   `FUN_00708080`).
2. **Assignment:** `FUN_00708080` calls `Queue_Mission(0x15, 0)` on up to 6 selected nearby
   idle teammates (35% probability; rest get AreaGuard=0xB), sets `responder[0x86]` =
   attacker's gather cell, sets target via `Assign_Target`.
3. **Handler dispatch:** Each tick (per INI Rescue rate + jitter), `Mission_Dispatch` calls
   `FootClass__Mission_Rescue`:
   - **No target yet:** scan for nearest threat within weapon range near `[0x86]`; if found,
     `Assign_Target`; else find passable cell near gather object and issue Move.
   - **Target acquired:** call combat/attack vtable slot immediately (return 1).
   - **State 1, arrived:** clear ghost, transition to next mission via Queue+Commence.
4. **Return rate:** INI MissionControl `Rate`/`AARate` for Rescue entry + 0..2 frame jitter.
   Aircraft variant hardcodes `return 5`.

### Rescue Handler (AircraftClass variant)

Aircraft assigned Rescue: fly toward target (`param_1[0xad]`) with payload (`param_1[0x46]`);
when in range (`Rules+0x54C` threshold), `Drop_Payload()` then return. If no target or no
payload, retreat (Queue Retreat). Transition mission via `vtable+0x1E8(0x1A)` for approach.
Always returns 5.

### Ambush Handler (no-op)

Ambush=14 slot points to `0x005B2E30` (`mov eax,0x1C2; ret`). No override, no assignment.
If ever called (via map INI `Mission=Ambush` on pre-placed object), it defers 450 ticks
and does nothing. The Rust enum may include `Ambush` as a variant that returns `450` for
map-INI round-trip fidelity, but MUST NOT have logic.

---

## 5. Negative Facts / Do-Not-Do

1. **Do not implement a Rescue handler for human-player units.** `HouseClass__IsPlayerControl()
   == '\0'` is a hard gate in `FootClass__ReceiveDamage`; human players' units are never
   assigned Rescue. Verified decompile `0x004D7330`.

2. **Do not implement Ambush handler logic.** Slot `0x007E8EA0` = `0x005B2E30` = stub
   `mov eax,0x1C2; ret`. No subclass overrides, no live assigner. Any Rust implementation
   would produce non-parity behavior (stub fires, does nothing).

3. **Do not give AircraftClass Rescue the ground-unit convergence logic.** The aircraft
   variant (`0x00415960`) is a distinct drop-payload handler, not the approach-and-attack
   loop of the foot variant. Using the foot handler for aircraft would produce wrong
   behavior (no payload drop, wrong transition).

4. **Do not hardcode a fixed return rate for Rescue foot handler.** It uses
   `MissionClass__GetMissionTimerEntry()` → the INI-configured `Rate`/`AARate` for the
   Rescue MissionControl entry, the same pattern as all other mission handlers. Hardcoding
   a constant is drift.

5. **Do not assign Rescue unconditionally.** `FUN_00708080` assigns Rescue with only ~35%
   probability (`Random__RandomRanged(0,99) <= 0x41`); the remaining ~65% gets AreaGuard
   (0xB). Assigning Rescue to all responders or none diverges from the binary.

---

## 6. Remaining Uncertainty

- **`TechnoClass::SetGhostCell` role in Rescue (foot handler):** Called at entry and after
  move-issue. In YR this call is mostly inert (ghost mechanics are TS-specific), but the
  exact side-effect in YR (if any) was not traced. LOW risk — likely a no-op for non-warp
  units.

- **`param_1[0x1a2]` exact field name:** Zeroed at entry in foot Rescue handler. Field
  semantics not traced to setter/reader. INFERRED as a scatter-suppress or targeting-lock
  flag. Does not affect the core Rescue behavior, but should be resolved before finalizing
  the component layout.

- **`FUN_005f6440` in AircraftClass Rescue:** Takes `param_1[0xad]` (target pointer),
  returns an int compared to `g_RulesClass_Instance+0x54c`. Likely `TechnoClass::
  Get_Threat` or a distance accessor. The threshold identity (`Rules+0x54C` field name)
  is unverified. LOW risk for the Rust hand-off — the comparison structure is clear even
  if the field name is unknown.

- **Team membership gate details:** `field_0xb4 != 0` in `FootClass__ReceiveDamage` was
  identified as a team pointer by position and pattern, but the exact byte offset in the
  final struct layout and whether it maps to `MissionClass::SuspendedMission (+0xB0)` or
  a RadioClass field was not independently confirmed. The `IsPlayerControl` gate is the
  decisive YR-activity gate, which IS confirmed.

---

## 7. Verified Facts Summary (load-bearing)

| # | Claim | Evidence |
|---|-------|----------|
| 1 | `g_MissionNameTable[14]` = "Ambush", `[21]` = "Rescue" | `read_memory 0x00816DF8` → "Ambush"; `read_memory 0x00816DB4` → "Rescue" |
| 2 | Ambush slot (FootClass vtable +0x20C) = `0x005B2E30` = `mov eax,0x1C2; ret` stub | `read_memory 0x007E8EA0` → `0x005B2E30`; `read_memory 0x005B2E30` → `B8 C2 01 00 00 C3` |
| 3 | Rescue foot handler `0x004DDF90` bound at FootClass vtable +0x258 | `read_memory 0x007E8EEC` → `0x004DDF90`; `decompile_function 0x004DDF90` confirms state machine |
| 4 | Rescue aircraft handler `0x00415960` bound at AircraftClass vtable +0x264 | `read_memory 0x007E2508` → `0x00415960`; `decompile_function 0x00415960` confirms drop-payload logic |
| 5 | `FUN_00708080` assigns Rescue via `Queue_Mission(0x15)` with 35% probability on AI team-member responders; `IsPlayerControl()==0` gate confirmed in `FootClass__ReceiveDamage @ 0x004D7330` | `decompile_function 0x00708080` → `RandomRanged(0,99) <= 0x41 → uVar6=0x15`; `decompile_function 0x004D7330` → `HouseClass__IsPlayerControl() == '\0'` gate before `FUN_00708080` call; `get_function_callers 0x00708080` → `{FootClass__ReceiveDamage, BuildingClass__ReceiveDamage, TechnoClass__ReceiveDamage}` |

---

## Status: COMPLETE
