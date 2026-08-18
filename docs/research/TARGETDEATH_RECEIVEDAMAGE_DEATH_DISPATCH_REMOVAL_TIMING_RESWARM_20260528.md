# TARGET-DEATH ReceiveDamage — Death Dispatch & Active-Vector Removal Timing

**Target question:** When HP≤0, which death handler fires, is the active-vector removal
SYNCHRONOUS (inside ReceiveDamage) or DEFERRED (next AI tick or later), and what is the
scheduler cursor consequence?

**Report type:** RE investigation — Ghidra MCP read-only
**Date:** 2026-05-28
**Non-goals:** Damage formula (covered by DAMAGE_MATH / RECEIVE_DAMAGE docs); InfantryClass
and BuildingClass per-class deep dives (slots 2/3/4); save/load active-vector rebuild;
AircraftClass ReceiveDamage.
**Evidence needed to mark COMPLETE:** vtable+0xDC, +0xE0, +0xE4 (UnitClass) identity
verified from memory; vtable+0xF8 (UnInit) slot verified; call site in UnitClass::ReceiveDamage
verified; scheduler consequence derived from established MIDPASS mechanics.
**Stop conditions:** All four items above satisfied, or Ghidra MCP unavailable.

---

## 1. ObjectClass::ReceiveDamage — HP≤0 Death Dispatch

**Address:** 0x005F5390 — verified via `decompile_function 0x005F5390`

When `this->Health` drops below 1, the following three vtable calls fire in order (asm
range 0x005F578D–0x005F57AF, verified via `get_assembly_context 0x005F577C`):

| Offset | Assembly address | Purpose |
|--------|-----------------|---------|
| vtable+0xE4 | 0x005F578D `CALL [EDX+0xE4]` | Enemy-kill credit (fires when attacker ≠ owner's house AND attacker pointer non-null) |
| vtable+0xE0 | 0x005F579A `CALL [EAX+0xE0]` | Ally/self-kill credit (fires otherwise) |
| vtable+0xDC | 0x005F57AF `CALL [EDX+0xDC]`, `PUSH 0x1` | Cell cleanup — called ALWAYS with arg=1 |

After vtable+0xDC the code reads `[ESI+0x90]` (IsAlive, 0x005F57B5). IsAlive is still
**true** at this point — vtable+0xDC does NOT clear IsAlive. ReceiveDamage then fires
trigger/tag processing and returns `local_14 = 4` (killed). Active-vector membership
(+0x98) is **unchanged** at ReceiveDamage return.

Active in YR: Yes.

---

## 2. UnitClass vtable+0xDC, +0xE0, +0xE4, +0xF8 Identity

**Verified via `read_memory 0x007F5D4C` (UnitClass vtable base 0x007F5C70 confirmed by
ADDRESS_MAP.md; slots computed as base+offset):**

| Slot | vtable address | Function | Address |
|------|---------------|----------|---------|
| +0xDC | 0x007F5D4C | `FootClass::Destroy` | 0x004D9720 |
| +0xE0 | 0x007F5D50 | `UnitClass::OnEnterCell_Triggers` (kill-credit/trigger dispatch; mislabeled) | 0x00744720 |
| +0xE4 | 0x007F5D54 | `FUN_00703230` (enemy kill credit, XP, score) | 0x00703230 |
| +0xF8 | 0x007F5D68 | `FootClass::UnInit` | 0x004DE5D0 |
| +0xD4 | 0x007F5D44 | `UnitClass::Limbo` (override of ObjectClass::Conceal) | 0x007440B0 |

Verified via `read_memory 0x007F5D40` (16 bytes) and `read_memory 0x007F5D60` (64 bytes),
cross-checked with `get_function_by_address` for each resolved address.

### vtable+0xDC = FootClass::Destroy — NOT UnInit

`FootClass::Destroy` (0x004D9720, decompiled via `decompile_function 0x004D9720`):
- param2=1 path (what ReceiveDamage passes): calls `FUN_006ea870` (trail cleanup if needed),
  then `vtable+0x274(3)` (locomotor stop), then `ObjectClass::Destroy(1)`.
- `ObjectClass::Destroy` (0x005F5280, decompiled): calls `Detach_From_All_Lists` (cell-list
  cleanup) and a conditional Deselect; does **not** call UnInit, vtable+0xD4 (Conceal/Limbo),
  or clear IsAlive (+0x90).

**Net effect of vtable+0xDC call:** cell-list detach only. Object remains in LogicClass
active vector. IsAlive unchanged (true). InLimbo unchanged. +0x98 membership unchanged.

Active in YR: Yes.

---

## 3. UnitClass::ReceiveDamage — Synchronous vtable+0xF8 Call

**Address:** 0x00737C90 — verified via `read_memory 0x007F5DDC` (UnitClass vtable+0x16C =
0x007F5DDC → 0x00737C90) and `get_function_by_address 0x00737C90`.

`UnitClass::ReceiveDamage` overrides vtable+0x16C. It calls `FootClass::ReceiveDamage`
(which chains up through TechnoClass → ObjectClass::ReceiveDamage) and processes the
result. When result==4 (killed), the function:

1. Calls `UnitClass__Death_Explosion()` if not a special water/revival case.
2. Calls `vtable+0x124(0)` (health-bar/pip update).
3. **Calls `vtable+0xF8()` directly** — FootClass::UnInit → ObjectClass::UnInit → vtable+0xD4
   (UnitClass::Limbo → FootClass::Limbo → ObjectClass::Conceal → FUN_0055BAE0 compacting
   removal from active vector) → clear IsAlive (+0x90) → append to PendingDeleteList.

Verified via `decompile_function 0x00737C90`. The relevant path at the end of the function:

```c
if (puVar10[0xd95] == '\0') {            // not Amphibious/sinking
    if (*(char*)(...+1) == '\0') {        // not special-revival flag
        vtable+0xF8();                    // ← UnInit CALLED HERE, synchronous
        return param_8;
    }
} else {
    cVar2 = vtable+0x3DC();
    if (cVar2 == '\0') {
        vtable+0xF8();                    // ← UnInit CALLED HERE, synchronous
        return param_8;
    }
}
```

**Active-vector removal is SYNCHRONOUS within the UnitClass::ReceiveDamage call for the
standard kill path.** The removal happens inside A's `vtable+0x5C` (AI) call, not on a
separate tick.

The call chain that effects removal:
`UnitClass::ReceiveDamage` → `vtable+0xF8` → `FootClass::UnInit` (0x004DE5D0) →
`ObjectClass::UnInit` (0x005F65F0) → `vtable+0xD4` (UnitClass::Limbo → ObjectClass::Conceal)
→ `FUN_0055BAE0` (compacting-left vector remove + clear +0x98) → IsAlive +0x90 = 0 →
PendingDeleteList append.

Active in YR: Yes.

### Exception: Amphibious sinking / death-explosion timer path

When `puVar10[0xd95] != '\0'` (Amphibious flag) AND `vtable+0x3DC()` returns true: vtable+0xF8
is NOT called inline. Instead, UnitClass::AI (0x007360C0) handles sinking via the `param_1[0x3cd]`
branch (sinking animation countdown), and calls `vtable+0xF8` when altitude drops below -400:

```c
if (iVar7 < -400) {
    vtable+0xE0(0);
    vtable+0xF8();   // UnInit, deferred to AI tick
    return;
}
```

Verified via `decompile_function 0x007360C0`. Active in YR for Amphibious units only; this is a
live YR code path (Amphibious=yes exists on stock units e.g. Terror Drones / MAD Tanks) —
not TS-only.

Also in UnitClass::AI, there is a separate death-explosion timer path (`param_1[0x1b6]`) that
calls `vtable+0xF8`. This appears to be for units with a death-explosion countdown (set during
the kill), and fires when the timer expires. For most stock units this fires within the same
pass or the next tick. **This path is not the primary synchronous path — it is the sinking /
explosion-delay deferred path.**

---

## 4. Scheduler Cursor Consequence

Established facts from MIDPASS and LIFECYCLE docs (not re-derived here):
- Scheduler in `LogicClass::PerTickUpdate` walks active vector forward by index, calling
  `vtable+0x5C`, reloading count each step. No index repair on removal.
- `FUN_0055BAE0` compacts left: entry at iB removed, iB+1..N-1 shift to iB..N-2, count
  decremented.

**Case A: B at iB > iA (cursor), A kills B during A's AI tick.**
- vtable+0xF8 fires synchronously inside A's vtable+0x5C call.
- At removal time: cursor = iA. Entry B at iB removed. iB+1..N-1 shift left by 1.
  Count reloaded on next iteration as N-1.
- Cursor increments to iA+1 next. Eventually reaches new-iB (which contains old-iB+1).
  Old-iB+1 is processed normally. **No entry is skipped.**

**Case B: B at iB < iA (cursor), B was already ticked this pass before A fires.**
- vtable+0xF8 fires synchronously inside A's vtable+0x5C call.
- At removal time: cursor = iA. Entry B at iB removed. All entries from iB..iA-1 shift left.
  Entry that was at iA is now at iA-1. The cursor variable still holds iA.
- A's vtable+0x5C finishes. Cursor increments to iA+1 (which is now old-iA+2).
  **Old-iA+1 is skipped this pass.** One entry is dropped from this tick.

**Rule:** B's synchronous removal causes skip of 1 entry when B was at a lower index
(already ticked) than the current cursor. No skip when B is at a higher index (not yet
ticked).

In standard play, A fires at B from any relative index. The B-already-ticked case fires
roughly half the time when two live objects are anywhere in the vector.

---

## 5. Per-Class ReceiveDamage Vtable Slots (handoff for slots 2/3/4)

Verified via `read_memory` at each class vtable + 0x16C:

| Class | vtable base | vtable+0x16C address | ReceiveDamage address | Label |
|-------|------------|---------------------|----------------------|-------|
| UnitClass | 0x007F5C70 (ADDRESS_MAP) | 0x007F5DDC | 0x00737C90 | `UnitClass__ReceiveDamage` |
| InfantryClass | 0x007EB058 (ADDRESS_MAP) | 0x007EB1C4 | 0x00517FA0 | no Ghidra boundary; slot 2 |
| BuildingClass | 0x007E3EBC (ANIMCLASS_SPAWN_PATHS doc) | 0x007E4028 | 0x00442230 | `BuildingClass__ReceiveDamage` |
| FootClass (via TechnoClass chain) | — | — | 0x00701900 | `TechnoClass__ReceiveDamage` |

BuildingClass result: slot-4 swarm already confirmed BuildingClass active-vector removal
IS synchronous via Limbo → Conceal chain inside BuildingClass::ReceiveDamage (see
TARGETDEATH_BUILDINGCLASS_DESTRUCTION_REMOVAL_OWNER_RESWARM_20260528.md).

---

## 6. Rust Delta

Current `src/sim/combat/mod.rs` around line 985–1006 (`dying = true` path vs `entities.remove`):

**SHP/infantry path:** sets `entity.dying = true`, defers removal to animation system.
Active-vector removal (via `unregister_live_object`) is not called at kill time — deferred.

**Structure/voxel path (lines 1003–1007):** calls `occupancy.remove`, `entities.clear_radio_contacts_for`, then `entities.remove(dead_id)`. Does NOT call `unregister_live_object`. The entity is removed from EntityStore but its slot may linger in `self.logic` vector until `unregister_live_object` is called by some other path (e.g., `despawn_entity`).

**Native contract:** for UnitClass, vtable+0xF8 (UnInit) is called synchronously inside
ReceiveDamage → Conceal → FUN_0055BAE0 removes from logic vector in one call chain.
`PendingDeleteList` defers physical memory free but the logic vector slot is gone immediately.

**DRIFT:** Rust voxel/structure path calls `entities.remove` without `unregister_live_object`.
If `logic` vector still holds the ID, the logic vector has a dangling reference (same issue
as documented in slot-4 BuildingClass report). The slot-4 fix (route through `World::despawn_entity`)
applies here equally.

**DRIFT:** The "dying animation" path defers active-vector removal to the animation system
rather than removing synchronously at kill time. This changes which subsequent objects get
skipped by the scheduler, compared to the synchronous removal native contract.

---

## 7. Implementation Handoff

**Handoff 1:**
- Verified behavior: `UnitClass::ReceiveDamage` calls `vtable+0xF8` (UnInit → Conceal →
  compacting remove from active vector) SYNCHRONOUSLY on lethal damage, within the same
  `vtable+0x5C` call that delivered the damage. Not deferred to a later AI tick.
- Rust delta: `combat/mod.rs` `entities.remove(dead_id)` for non-animated kills does not
  call `unregister_live_object`, leaving the logic vector with a dangling slot.
- Affected surface: `src/sim/combat/mod.rs` ~1000–1010, `src/sim/world/mod.rs` `despawn_entity`.
- Acceptance scenario: Kill a voxel unit via weapon fire; verify logic vector does NOT contain
  its ID on the next tick.
- Proposed test name: `test_synchronous_kill_clears_logic_vector_slot`
- Risk: Logic-vector slot accumulation over long matches → objects at those positions run
  AI on non-existent IDs; stale-slot behavior depends on BTreeMap / Vec ordering.

**Handoff 2:**
- Verified behavior: When B (index iB < cursor iA) is removed synchronously during A's AI,
  the entry that was at cursor iA shifts to iA-1. The scheduler (no index repair) increments
  cursor to iA+1, skipping old-iA+1 this pass.
- Rust delta: Rust `advance_tick` is phased; the logic vector is iterated with a sorted snapshot
  in `live_object_order_snapshot` (already flagged DRIFT). If Rust removes B from the logic
  vector mid-iteration, the iteration behavior depends on whether the iterator is a snapshot
  (no skip) or a live walk (skip). Native is a live walk — skip occurs.
- Affected surface: `src/sim/world/mod.rs` `live_object_order_snapshot`, the tick-phase
  iteration in `advance_tick`.
- Acceptance scenario: Construct a 3-unit fixture (A at i=1, B at i=0, C at i=2). A kills B
  during A's tick. Verify C's AI does NOT run this tick (skip), matching native.
- Proposed test name: `test_synchronous_removal_before_cursor_skips_next_entry`
- Risk: Firing order changes of one tick across matched objects could affect combat-result
  reproducibility in multiplayer.

**Handoff 3:**
- Verified behavior: `FootClass::Destroy` (vtable+0xDC, called from ObjectClass::ReceiveDamage)
  does NOT clear IsAlive (+0x90) and does NOT remove from the logic vector. It only calls
  `Detach_From_All_Lists` (cell-list / map-list cleanup) and `vtable+0x274(3)` (locomotor stop).
- Rust delta: Rust does not have an equivalent two-stage cleanup (cell-detach via Destroy then
  logic-vector removal via UnInit). The Rust `despawn_entity` does both in one call.
- Affected surface: ordering of cell-occupancy remove vs logic-vector remove inside
  `despawn_entity`.
- Acceptance scenario: After kill, entity must not appear in cell occupancy OR logic vector
  by the time B's ID would have been iterated — confirmed by `test_synchronous_kill_clears_logic_vector_slot`.
- Proposed test name: (covered by Handoff 1 test)
- Risk: Low for correctness in isolation; matters for determinism when other systems query
  cell occupancy vs logic membership between stages.

---

## 8. Negative Facts / Do-Not-Do

1. **Do NOT model `param_1[0x1b6]` (death-explosion timer) as the primary active-vector
   removal path for standard unit kills.** The primary path is `UnitClass::ReceiveDamage` →
   vtable+0xF8 synchronously. The timer path in `UnitClass::AI` fires only for amphibious
   sinking and explosion-delay units (separate deferred case). Verified: vtable+0xF8 call
   at end of `decompile_function 0x00737C90`.

2. **Do NOT treat vtable+0xDC (FootClass::Destroy) as the active-vector removal step.**
   It is cell-list cleanup only. Verified: `decompile_function 0x004D9720` shows no
   vtable+0xD4 or UnInit call; `decompile_function 0x005F5280` (`ObjectClass::Destroy`)
   shows only `Detach_From_All_Lists`.

3. **Do NOT skip the `unregister_live_object` call when killing a voxel/structure entity.**
   `entities.remove(dead_id)` in `combat/mod.rs` is incomplete — it must be paired with
   `unregister_live_object` or routed through `despawn_entity`. The logic vector is a
   separate structure from EntityStore. Verified: `FUN_0055BAE0` is called via Conceal,
   not via EntityStore destruction.

4. **Do NOT assume that because result==4 is returned, the object is already gone from the
   vector.** Only `UnitClass::ReceiveDamage` (and BuildingClass per slot-4) call vtable+0xF8
   synchronously. The removal happens inside the ReceiveDamage call, within the caller's
   `vtable+0x5C` AI tick. The object does NOT get one final AI tick after being killed by the
   synchronous path.

5. **Do NOT fire a deferred "cleanup next tick" event for standard unit deaths.** The native
   contract is synchronous removal within the kill call. PendingDeleteList defers only the
   physical `operator_delete` (memory free), not the logic-vector vacate. Verified:
   `decompile_function 0x005F65F0` shows vtable+0xD4 (Conceal) before +0x90=0 before
   PendingDeleteList append.

---

## 9. Remaining Uncertainty

- **InfantryClass::ReceiveDamage (0x00517FA0)** has no Ghidra function boundary. Whether it
  calls vtable+0xF8 synchronously or uses a different path (e.g. dying-flag + deferred to
  `InfantryClass::AI`) is unknown. Slot 2 scope.
- **Whether `UnitClass__Death_Explosion` (called before vtable+0xF8 in UnitClass::ReceiveDamage)
  can trigger collateral kills** that recurse into another ReceiveDamage synchronously
  (nested kill during kill) and whether the compaction interacts with the outer cursor has
  not been traced.
- **The `param_1[0x1b6]` initialization site** (when and how the death-explosion timer is set
  from -1 to 0) was not pinpointed. It does not affect the synchronous kill path (which calls
  vtable+0xF8 regardless of that timer), but matters for the deferred sinking case.
- **AircraftClass::ReceiveDamage** not covered by this slot.

---

## 10. Source Ledger

Live Ghidra (this session, read-only):
- `decompile_function 0x005F5390` — ObjectClass::ReceiveDamage
- `decompile_function 0x00701900` — TechnoClass::ReceiveDamage
- `decompile_function 0x005F65F0` — ObjectClass::UnInit
- `get_assembly_context 0x005F577C` — death-dispatch assembly (vtable+0xDC/+0xE0/+0xE4)
- `read_memory 0x007F5D40` + `0x007F5D60` — UnitClass vtable slots +0xD0..+0x11C
- `get_function_by_address` — resolved all vtable slot addresses
- `decompile_function 0x004D9720` — FootClass::Destroy
- `decompile_function 0x005F5280` — ObjectClass::Destroy
- `decompile_function 0x007360C0` — UnitClass::AI
- `decompile_function 0x00737C90` — UnitClass::ReceiveDamage (key finding)
- `decompile_function 0x00702D40` — TechnoClass::RecordKill
- `decompile_function 0x007440B0` — UnitClass::Limbo
- `read_memory 0x007E4028` + `get_function_by_address 0x00442230` — BuildingClass vtable+0x16C
- `read_memory 0x007EB1C4` — InfantryClass vtable+0x16C

Research docs consulted:
- `LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS.md` — lifecycle contracts (cited, not re-derived)
- `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` — scheduler mechanics (cited)
- `TARGETDEATH_BUILDINGCLASS_DESTRUCTION_REMOVAL_OWNER_RESWARM_20260528.md` — Building path
- `ADDRESS_MAP.md` — vtable bases
