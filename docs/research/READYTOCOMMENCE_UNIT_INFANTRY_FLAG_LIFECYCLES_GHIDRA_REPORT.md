# ReadyToCommence Gate Flag Lifecycles — UnitClass +0x6D1/+0x6E1/+0x6E2 and InfantryClass +0x2B4

**Date:** 2026-06-02
**Confidence:** HIGH for all four offsets — write sites verified via `decompile_function` and
`get_assembly_context`; caller liveness verified via `get_function_callers`.
**Slot:** Re-Swarm round-2, slot-2

---

## Investigation Charter

**Target question:** When are UnitClass +0x6D1, +0x6E1, +0x6E2 set/cleared, and when is
InfantryClass +0x2B4 set/cleared? The prior report (READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md)
identified the READ sites; this document supplies the WRITE sites to make the Rust gate implementable.

**Non-goals:** Re-decoding ReadyToCommence readers, AircraftClass/BuildingClass flags,
unrelated locomotor logic.

**Evidence needed to mark COMPLETE:** For each offset: write-site address(es), access width
(byte/dword), trigger event, and caller liveness verified via `get_function_callers`.

**Stop conditions:** All four offsets have verified writers and semantics confirmed.

---

## Settled Facts — Pre-existing (Not Re-derived)

From READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md:

- `UnitClass::ReadyToCommence` (0x00744270) reads `+0x6D1`, `+0x6E1`, `+0x6E2` as BYTE
  flags. Any non-zero blocks (returns 0 = not ready).
- `InfantryClass::ReadyToCommence` (0x00521B60) reads `+0x2B4` as DWORD. Non-zero when
  `CurrentMission == Attack (1)` BLOCKS (returns 0 = not ready).
- Both offsets initialized to 0 in constructors.

---

## UnitClass +0x6D1 — "Dock-Active / Refinery-Unload Occupancy Flag"

**Type:** BYTE at `this+0x6D1`
**Access width:** 1 byte, verified via assembly `MOV byte ptr [ESI+0x6D1], 0/1`
**Active in YR:** Yes — fires every harvester unload cycle at a refinery

### SET site (= 1)

**Function:** `UnitClass__Mission_Deploy_Building` (`0x0073D630`)
**Assembly address:** `0x0073E011` (`MOV byte ptr [ESI+0x6D1], 0x1` is represented as
`*(undefined1 *)((int)param_1 + 0x6d1) = 1` in the decompile at the FSM state transition)
**Trigger:** Inside the `HasPathType` branch when the flag is currently clear:
`if (param_1[0x2f] != 3)` — entering FSM state 3 (harvester is on the refinery pad,
beginning the unload sequence). Sets +0x6D1=1, initializes frame counters (+0x3E/+0x40/+0x41/+0x42/+0x43),
then transitions state-machine to step 3.
Verified via `decompile_function 0x0073d630`, annotation in function header, and
`get_assembly_context 0x0073dee7` (clean path: `if (*(char*)((int)param_1 + 0x6d1) == '\\0') { ... *(undefined1*)((int)param_1 + 0x6d1) = 1; ... param_1[0x2f] = 3; }`).

**Context:** Preceded by `PathType__Has_Valid_Steps()` returning true; rate-timer gate
`((*puVar10 >> 7) + 1 & 0x1FE) == 0x80` gates this to fire once per harvester-cycle period.

### CLEAR site (= 0) — two sites

**Site 1:** `UnitClass__Mission_Deploy_Building` (`0x0073D630`), FSM state `case 4` path:
`*(undefined1 *)((int)param_1 + 0x6d1) = 0` — clears when unload is complete and
harvester is ready to depart, before calling `Queue_Mission(Harvest/10)` and
`Commence`. Verified in the decompile for both the `e0f != 0` path (slave miner variant)
and the normal harvester path.
Approximate assembly address: `~0x0073E100` / `~0x0073E190` (two symmetric clear sites
in the two harvest-type branches of state 4).

**Site 2:** `UnitClass__Mission_Deploy_Building` (`0x0073D630`), `PathType::Has_Valid_Steps`
returning false (no path):
`*(undefined1 *)((int)param_1 + 0x6d1) = 0` at `~0x0073DEF0`.
Verified via `get_assembly_context 0x0073dee7` showing the MOV clear before the locomotor
`IsMoving` check and the `ReadyToCommence` vtable call.

### Semantic Summary

+0x6D1 = **dock-active / refinery-pad occupancy flag**. It is set when the harvester
enters the refinery unload FSM state 3 (actively unloading) and cleared when unloading
completes (state 4 exit) OR when the harvester has no valid path steps (aborting).
While +0x6D1 is set, `ReadyToCommence` returns 0, preventing the harvester from accepting
a new mission assignment until the unload cycle finishes.

**Verified via:** `decompile_function 0x0073d630`, `get_assembly_context 0x0073dee7`,
`get_function_callers 0x00739ac0`.

---

## UnitClass +0x6E1 — "Deploy-Begin Animation Active Flag"

**Type:** BYTE at `this+0x6E1`
**Access width:** 1 byte
**Active in YR:** Conditional — only for unit types with `TypeData+0xe13` (IsFlying/CanFly?)
flag set. This applies to aircraft-vehicle hybrids (e.g., Siege Chopper). Not active for
standard ground vehicles.

### SET site (= 1)

**Function:** `FUN_00739ac0` (`0x00739AC0`) — "deploy-begin animation helper"
**Assembly address:** `0x00739C62`: `MOV byte ptr [ESI + 0x6e1], BL` where `BL = 1` (set at
`0x00739B17: MOV EBX, 0x1` early in the function).
**Trigger:** The function is called from `UnitClass__Mission_Deploy_Building` (0x0073D630)
inside the `TypeData+0xe13 != 0` branch. The SET fires when:
- +0x1B8 (is-currently-deploying flag) == 0 (not yet deploying)
- +0x4D (pending-exit flag) == 0
- TypeData deploy-anim type (`TypeData+0x6BC`) != 0
- +0x130 (anim-object slot) == 0 (no anim running yet)
Then constructs an AnimClass for the deploy-begin animation and sets +0x6E1=1.

Verified via `decompile_function 0x00739ac0` and `get_assembly_context 0x00739b77` /
`0x00739c44` (animation construction then `MOV byte ptr [ESI + 0x6e1], BL`).

### CLEAR site (= 0)

**Function:** `FUN_00739ac0` (`0x00739AC0`)
**Assembly address:** `0x00739B70`: `MOV byte ptr [ESI + 0x6e1], 0x0`
**Trigger:** When the animation has completed: frame counter `param_1[0xF8] >= (anim.StartFrame + anim.NumFrames - 1)`. Sets +0x1B8=1 (deploying-active state) and clears +0x6E1=0.

Verified via `get_assembly_context 0x00739b6a` showing the compare `CMP ECX, EAX` (frame
test) followed immediately by `MOV byte ptr [ESI + 0x6e1], 0x0` at 0x00739B70.

### Callers

`FUN_00739ac0` is called only by `UnitClass__Mission_Deploy_Building` (`0x0073D630`).
Verified via `get_function_callers 0x00739ac0`.

### Semantic Summary

+0x6E1 = **deploy-begin animation in-progress flag**. Set when the unit-type deploy
animation starts playing; cleared when that animation completes. Only active for unit
types that have a deploy animation (TypeData+0x6BC anim-type pointer non-null AND
TypeData+0xe13 flag set). While +0x6E1 is set, `ReadyToCommence` returns 0.

---

## UnitClass +0x6E2 — "Deploy-Reverse Animation Active Flag"

**Type:** BYTE at `this+0x6E2`
**Access width:** 1 byte
**Active in YR:** Same condition as +0x6E1 — only for types with TypeData+0xe13 flag set.

### SET site (= 1)

**Function:** `FUN_00739cd0` (`0x00739CD0`) — "deploy-reverse animation helper"
**Assembly address:** `0x00739E46`: `MOV byte ptr [ESI + 0x6e2], 0x1`
**Trigger:** Called from `UnitClass__Mission_Deploy_Building` for types with +0x1B8 == 1
(is-deploying active). The SET fires when:
- +0x6E2 == 0 (reverse anim not yet running)
- TypeData deploy-anim type (`TypeData+0x6BC`) != 0
- +0x130 (anim-object slot) == 0 (no anim running)
Then constructs the deploy animation in REVERSE (last param `1` to AnimClass constructor)
and sets +0x6E2=1.

Verified via `decompile_function 0x00739cd0` and `get_assembly_context 0x00739e04`
(→ `0x00739E46: MOV byte ptr [ESI + 0x6e2], 0x1`).

### CLEAR site (= 0)

**Function:** `FUN_00739cd0` (`0x00739CD0`)
**Assembly address:** `0x00739D28`: `MOV byte ptr [ESI + 0x6e2], 0x0`
**Trigger:** Reverse animation completes: frame counter >= `(anim.StartFrame + anim.NumFrames - 2)`.
Also clears +0x6E0 and +0x1B8, then calls `vtable[0x480]` (MoveToCell) if TypeData+0x6AD
(IsFlying) is set. Verified via `get_assembly_context 0x00739cd7` showing `0x00739D28:
MOV byte ptr [ESI + 0x6e2], 0x0`.

### Callers

`FUN_00739cd0` is called only by `UnitClass__Mission_Deploy_Building` (`0x0073D630`).
Verified via `get_function_callers 0x00739cd0`.

### Semantic Summary

+0x6E2 = **deploy-reverse animation in-progress flag** (also called "un-deploy" or
"fold-up" animation). Set when the deploy-reverse animation starts; cleared when it
completes. Mirror of +0x6E1 for the reverse direction. While +0x6E2 is set,
`ReadyToCommence` returns 0.

---

## InfantryClass +0x2B4 — "Attack Target Pointer"

**Type:** DWORD at `this+0x2B4` — a **pointer** (entity pointer, not a counter)
**Access width:** 4 bytes (dword), verified from `TechnoClass__Constructor` `param_1[0xad] = 0`
and `TechnoClass__Set_ArchiveTarget` `param_1[0xad] = (int)piVar4`.
**Active in YR:** Yes — fires for all infantry (and all TechnoClass subclasses that go through
the SetTarget path) whenever a target is acquired or cleared.

### Correction of Prior Report Label

The READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES report labeled this "Infantry
combat-lock counter (prevent mission switch mid-attack)." This is WRONG in two ways:
1. It is a **pointer** (entity pointer), not a counter.
2. `+0x2B4 != 0` in attack mission BLOCKS ReadyToCommence (returns 0 = not ready). The
   prior report's description was inverted.

**Correct semantic:** +0x2B4 = current **fire/attack target pointer**. When infantry is in
Attack mission AND has a target assigned (+0x2B4 != 0), `ReadyToCommence` returns 0,
preventing mission promotion until the target is cleared (null). When +0x2B4 == 0 (no
target) during Attack mission, ReadyToCommence proceeds to the type-check and can return 1.

This also appears in:
- `InfantryClass__IdleDispatch` (0x0051CBA0): `if (param_1[0xad] == 0)` → idle/walk DoType;
  else → fire DoType. Confirms +0x2B4 is the fire-target pointer.
- `InfantryClass__DoType_Sequencer` (0x00520AE0): checks non-zero for shoot-while-moving.
- `FootClass__Mission_Attack` (0x004D4DC0): checked to determine move-vs-attack behavior.
- `TechnoClass__Passive_Target_Acquire` (0x00709480): saves old `param_1[0xad]` and detects
  change after auto-acquire, setting +0x143 flag.
- `TechnoClass__StopAllTargeting` (0x0070D4A0): iterates all TechnoClass instances;
  for each where `piVar1[0xad] == param_1` (matching the dying entity), calls
  `vtable[0x1F8]()` and `vtable[0x3C8](0)` to stop attack and clear target.

### SET site

**Function:** `TechnoClass__Set_ArchiveTarget` (`0x006FCDB0`)
**Assembly address:** `LAB_006fcf38`: `param_1[0xad] = (int)piVar4`
**Trigger:** Called when a new attack target is assigned. The function resolves the final
target pointer (may follow mind-control chain, apply passenger rules, etc.) and writes the
resolved pointer to +0x2B4.
**Path:** `InfantryClass__SetTarget` override (`FUN_0051B1F0` at `0x0051B1F0`) calls
`TechnoClass__Set_ArchiveTarget(param_2)` after handling animation-state cleanup. This
is the InfantryClass vtable+0x3C8 slot (verified via `read_memory 0x007EB420` = `0x0051B1F0`
which calls `TechnoClass__Set_ArchiveTarget`).

**Additional write path:** `TechnoClass__Set_ArchiveTarget` also clears +0x2B4 to 0 in two cases:
- When `piVar4` (the resolved target) is null (e.g., `SetTarget(null)` call)
- When the target resolves to null via the passenger/mind-control filter

Verified via `decompile_function 0x006fcdb0` showing `param_1[0xad] = (int)piVar4` at
`LAB_006fcf38`, and `get_function_callers 0x006fcdb0` showing callers: `FUN_0051b1f0`
(InfantryClass SetTarget override) and `BuildingClass__ToggleGate`.

### CLEAR site

**Function:** `TechnoClass__Set_ArchiveTarget` (`0x006FCDB0`) — same function, null path
**Trigger:** Any call with null target (`SetTarget(0)`), or target-resolution resulting in null.
Also via `TechnoClass__StopAllTargeting` which calls `vtable[0x3C8](0)` (SetTarget(0))
for all entities that were targeting the removed entity.

**Initialization:** `TechnoClass__Constructor` sets `param_1[0xad] = 0` (verified from
decompile body showing sequential zero assignments at the +0xAC-+0xB8 range).

**When cleared relative to ReadyToCommence:**
Once +0x2B4 is cleared to 0, the next `ReadyToCommence` call for Attack mission no longer
blocks at the +0x2B4 check. This allows `Commence()` to promote the queued mission.

### Callers of TechnoClass__StopAllTargeting (liveness)

- `Apply_area_damage` (0x00489280) — area damage kills trigger target clearing
- `BuildingClass__Place_OccupyMap` (0x00441F60)
- `FUN_00581140` (0x00581140) — likely entity death/limbo
- `HouseClass__Sell_Building_At_Cell` (0x004FCE80) — selling triggers target clearing
- `TechnoClass__ChangeOwner` (0x007014A0)
- `TeleportLocomotionClass__StateMachineTick` (0x007192F0)

All are live YR paths. Verified via `get_function_callers 0x0070d4a0`.

---

## Implementation Handoff

### 1. UnitClass +0x6D1 — Dock-Active Flag → Rust Delta

**Full chain:** `Mission_Deploy_Building` FSM state 3 entry → SET +0x6D1 = 1 → `ReadyToCommence`
returns 0 → `Queue_Mission(Harvest, commence=true)` silently waits → FSM state 4 exit CLEAR
+0x6D1 = 0 → next tick `ReadyToCommence` returns 1 → `Commence()` → harvester departs.

**Rust delta:** Add `dock_active: bool` to the UnitEntity harvester state (or a field in the
`MissionCom` component). In the harvest-unload FSM state 3 entry: `dock_active = true`.
In state 4 exit: `dock_active = false`. The `ready_to_commence()` for UnitClass checks
`!dock_active` (among other locomotor flags).

**Affected surface:** `UnitClass__Mission_Deploy_Building` port in `sim/missions/harvest.rs`
(or equivalent); `ready_to_commence()` in unit entity logic.

**Acceptance scenario:** A harvester entering the refinery unload sequence should NOT
accept a new `Queue_Mission(Move, commence=true)` until `dock_active` is cleared. A
player-issued move command mid-unload is queued but does not promote.

**Proposed test:** `test_unit_dock_active_blocks_readytocommence` — create a harvester in
dock-active state, call `queue_mission(Move, commence=true)`, verify `current_mission`
stays at Deploy/Harvest; clear `dock_active`, verify mission promotes on next tick.

**Risk:** MEDIUM — requires the harvester FSM to correctly track the two clear sites (state 4
completion path AND the path-abort clear). Missing either produces a stuck harvester.

---

### 2. InfantryClass +0x2B4 — Target Pointer → Rust Delta (Corrected)

**Full chain:** `SetTarget(enemy_unit)` → `TechnoClass__Set_ArchiveTarget` writes +0x2B4 =
enemy_ptr → `ReadyToCommence` for Attack mission returns 0 (has target = don't interrupt) →
enemy dies / command cleared → `StopAllTargeting` or `SetTarget(0)` → +0x2B4 = 0 →
`ReadyToCommence` returns 1 (proceeds to type-check) → `Commence()`.

**Rust delta:** The `+0x2B4` field maps to `entity.target: Option<EntityId>` (the existing
attack target). In `ready_to_commence()` for infantry: when `current_mission == Attack`,
block (`return false`) if `target.is_some()`. This is OPPOSITE of what the prior
report implied — the infantry CANNOT be re-tasked mid-attack when it has a live target.

**Affected surface:** `ready_to_commence()` in infantry entity logic; `set_target()` in the
combat system.

**Acceptance scenario:** Infantry with a current target (`target.is_some()`) and
`current_mission == Attack` should NOT promote a queued `Queue_Mission(Guard)`. Once the
target is cleared (`set_target(None)`), the queued mission should promote.

**Proposed test:** `test_infantry_readytocommence_blocks_with_target` — assign Attack mission
to infantry, set a target entity, queue `Guard` mission with `commence=true`, verify no
promotion; clear target, verify Guard promotes.

**Risk:** HIGH — the prior report had this backwards. Implementing the old description would
produce infantry that ARE promotable mid-attack (inverted gate). The corrected logic means
the infantry finishes its current engagement before accepting a re-task.

---

### 3. UnitClass +0x6E1/+0x6E2 — Deploy Animation Flags → Rust Delta

**Full chain:** `Mission_Deploy_Building` calls `FUN_00739ac0` → `+0x6E1 = 1` (begin-anim
playing) → `ReadyToCommence` returns 0 → animation completes → `+0x6E1 = 0`, `+0x1B8 = 1`
→ `FUN_00739cd0` called → `+0x6E2 = 1` (reverse-anim playing) → animation completes →
`+0x6E2 = 0` → `ReadyToCommence` proceeds.

**Rust delta:** Only relevant for unit types with `TypeData.deploy_anim != None` AND
`TypeData.is_flying_unit == true` (TypeData+0xe13). For standard ground vehicles these
flags are never set. Add `deploy_anim_active: bool` and `undeploy_anim_active: bool`
to the deploy-capable unit component. Set in the deploy FSM, clear on animation
completion.

**Affected surface:** The deploy FSM for Siege Chopper / other flying-deploy units.

**Proposed test:** `test_unit_deploy_anim_flags_block_readytocommence` — set
`deploy_anim_active = true`, verify `ready_to_commence` blocks; clear, verify unblocks.

**Risk:** LOW for standard ground vehicles (flags never set). MEDIUM for deploy-capable
types (Siege Chopper) — requires correct animation-frame-count tracking.

---

## Negative Facts / Do Not Do

1. **Do NOT treat +0x2B4 as a counter.** The field is a 4-byte entity pointer.
   INC/DEC patterns do not exist in any caller. The only writes are pointer assignments
   (0 or valid entity address). Verified via `decompile_function 0x006fcdb0` showing only
   `param_1[0xad] = (int)piVar4` and `param_1[0xad] = 0` assignment patterns.

2. **Do NOT treat +0x2B4 != 0 as "ENABLES" readiness.** In Attack mission, +0x2B4 != 0
   BLOCKS readiness (JNZ to return-0 at `0x00521BEF`). Verified via `get_assembly_context
   0x00521BED` showing `TEST EAX, EAX; JNZ 0x00521c0b` (return 0).

3. **Do NOT apply +0x6E1/+0x6E2 gate to standard ground vehicles.** The flag SET path is
   gated behind `TypeData+0xe13 != 0`. Standard tanks/miners never enter `FUN_00739ac0`
   or `FUN_00739cd0` in practice. Verified from `decompile_function 0x00739ac0` first
   check: `if (*(char *)(param_1[0x1b1] + 0xe13) != '\\0')`. (Note: `+0x1b1` is the
   UnitTypeClass pointer in `param_1[0x6c4]`.)

4. **Do NOT implement the +0x6D1 SET as part of the deploy animation path.** The +0x6D1
   flag is set by the HARVESTER UNLOAD FSM (state 3 of `Mission_Deploy_Building`), not
   by any animation helper. It is structurally separate from +0x6E1/+0x6E2.
   Verified: `FUN_00739ac0`/`FUN_00739cd0` never write +0x6D1; the explicit SET at
   ~0x0073E011 is in the PathType branch of the main FSM.

5. **Do NOT remove the type-check gate just because target is cleared.** For
   InfantryClass ReadyToCommence, even after +0x2B4 is cleared to 0, the function still
   checks `g_InfantryTypeHasIdleSeq[typeIndex]`. Infantry with no idle sequences cannot
   gate through regardless of target state. Both conditions must be satisfied.
   Verified from `decompile_function 0x00521b60` sequence.

---

## Remaining Uncertainty

None for the primary mandate (write-site lifecycle for all four offsets).

Minor open items (out of scope, low priority):

1. **+0x6E0** (the byte just before +0x6E1): also written in `FUN_00739ac0` and
   `FUN_00739cd0` with similar timing. Its role in `ReadyToCommence` was NOT verified
   (the prior report does not list it as a gate). It may be a related display/animation
   sync flag. Low priority — not part of the ReadyToCommence gate.

2. **`TypeData+0xe13` exact semantic**: Described here as "IsFlying/CanFly" based on
   context (the entire path is gated by it and the reverse-anim calls `TypeData+0x6AD`
   which IS labeled "IsFlying" in other docs). Not formally verified in this session.
   Sufficient for implementation purposes — the flag is what gates the deploy-anim path.

3. **+0x2B4 NULL-clear timing relative to target-death vs. explicit user-clear:** Both
   paths go through `TechnoClass__Set_ArchiveTarget` (`SetTarget(0)`), but the exact
   frame-order (does the infantry's target get cleared before or after it fires one last
   shot?) was not traced. This ordering matters for "last-shot vs. promotion" edge cases.

---

## Cross-References

- Prior report: `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`
  (provides READ sites; this doc provides WRITE sites)
- Mission state machine: `docs/research/MISSIONCLASS_STATE_MACHINE.md`
- Harvest/dock system: `docs/research/NEXT_DOCKER_SELECTION_UNDER_SATURATION_GHIDRA_REPORT.md`
