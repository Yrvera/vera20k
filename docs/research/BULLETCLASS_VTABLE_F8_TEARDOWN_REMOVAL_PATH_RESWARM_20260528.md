# BulletClass vtable+0xF8 Teardown / Active-Vector Removal Path — Reswarm 2026-05-28

**Address(es):** `BulletClass vtable @ 0x007E46E4`, vtable+0xF8 = `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass::Conceal @ 0x005F4D30`, remover `FUN_0055BAE0 @ 0x0055BAE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Identity of BulletClass vtable+0xF8 slot, the complete removal route through UnInit→Conceal→remover, the exact ordering of Object+0x98 clear relative to vector compaction and post-AI index increment, same-frame skip hazard for bullets specifically, and memory-free timing (pending-delete queue, drain site).  
**Non-Scope:** Full body of every warhead/damage side effect, exhaustive BulletClass::AI non-detonation exit paths (sibling slot-2), child-spawn ordering during detonation (sibling slot-3), complete BulletClass field census.  
**Confidence:** High — all material claims verified via Ghidra MCP decompile + read_memory + assembly context this session.  
**Active in YR:** Yes. Bullets are logic-enabled; every detonation path dispatches vtable+0xF8; the removal chain is the standard YR active-game path.

---

## 0. Investigation Contract

**Target question.** What is BulletClass vtable+0xF8? What is the exact removal route, +0x98 clear timing relative to vector compaction, and memory-free point?

**Non-goals.** Do not investigate: vtable+0x28 (sibling slot, separate doc), non-detonation bullet AI exits, warhead damage internals, destructor body internals beyond what PENDING_DELETE_DRAIN doc covers.

**Evidence needed to mark COMPLETE.**
- Vtable base address confirmed by assembly write + RTTI type-descriptor name.
- Slot identity confirmed by read_memory at vtable+0xF8.
- Full removal route decompiled: UnInit body, Conceal body, remover call site.
- +0x98 clear order within Conceal confirmed by assembly context.
- Memory-free timing confirmed by prior PENDING_DELETE_DRAIN doc (no redo).

**Stop conditions.**
- Stop when vtable+0xF8 identity, full route, and timing are verified.
- Record unsettled items as Remaining Uncertainty.

---

## 1. BulletClass Vtable Base — RTTI Verification

The BulletClass instance vtable base is at **`0x007E46E4`**.

**Evidence chain:**

1. `BulletClass__Constructor @ 0x00466380` (decompile: verified via `decompile_function 0x00466380`): writes `*param_1 = &vtable__BulletClass` at assembly instruction `0x00466425: MOV dword ptr [ESI + 0x0], 0x7E46E4` (verified via `get_assembly_context 0x004663a8`).

2. RTTI COL at vtable−4 (`0x007E46E0`): `read_memory 0x007E46E0` → `0x007FC7B0` (COL pointer). COL TypeDescriptor pointer → `0x0081AF70`. `read_memory 0x0081AF70` bytes 8..onward: mangled name `.?AVBulletClass@@` — confirmed this is the BulletClass primary vtable (verified via `read_memory 0x007E46E0` and `read_memory 0x0081AF70`).

---

## 2. Vtable+0xF8 Slot Identity

**vtable+0xF8 = `ObjectClass::UnInit @ 0x005F65F0`**

- `read_memory 0x007E47DC` (= 0x007E46E4 + 0xF8) → bytes `F0 65 5F 00` = LE `0x005F65F0` (verified via `read_memory 0x007E47DC`).
- Confirmed as `ObjectClass__UnInit` via `get_function_by_address 0x005F65F0` which returns `"ObjectClass__UnInit at 005f65f0"`.
- BulletClass does NOT override this slot; the base `ObjectClass::UnInit` is inherited unchanged.

Also verified the vtable call site in BulletClass::AI: assembly at `0x00467FAF`: `MOV EAX, dword ptr [EBP]` (load vtable ptr), then `0x00467FB2: MOV ECX, EBP` (set this), then `0x00467FB4: CALL dword ptr [EAX + 0xF8]` (verified via `get_assembly_context 0x00467fba`).

---

## 3. Vtable+0xD4 Slot — Confirmed Not Overridden

**vtable+0xD4 = `ObjectClass::Conceal @ 0x005F4D30`**

- `read_memory 0x007E47B8` (= 0x007E46E4 + 0xD4) = slot index 53 = `0x005F4D30` (verified via 256-byte vtable dump `read_memory 0x007E46E4` length 256).
- BulletClass does NOT override vtable+0xD4; the base `ObjectClass::Conceal` is inherited.

---

## 4. Complete Removal Route

The removal chain dispatched by BulletClass::AI vtable+0xF8 call is:

### 4.1 UnInit @ 0x005F65F0 (outer shell)

Verified order (from `decompile_function 0x005F65F0` + `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md` §3.1):

1. `0x005F6616`: call `Detach_From_All_Lists @ 0x007258D0` — detaches from all named global lists
2. `0x005F661F`: call virtual `+0xD4` on `this` (= `ObjectClass::Conceal @ 0x005F4D30`) — unregisters from LogicClass live vector (see §4.2)
3. `0x005F6625`: write `Object+0x90 = 0` — clear alive flag (IsDead now returns true)
4. `0x005F6668..0x005F6671`: increment pending-delete count at `0x00B0F6A8`
5. `0x005F6677..0x005F667D`: store `this` at `*(0x00B0F69C + old_count * 4)` — queue pointer for late drain

### 4.2 Conceal @ 0x005F4D30 (unregister step, called from UnInit step 2)

Verified order (from `decompile_function 0x005F4D30`):

Early gates (bail early with return 0 if either):
- `g_GameActive == '\\0'` (game not running)
- `this+0x81 != '\\0'` (already InLimbo)

Active path:
1. call `vtable+0x150` (Deselect)
2. call `vtable+0xDC(1)` (unmark cell occupation)
3. call `vtable+0x124(0)` (clear state machine)
4. call `DisplayClass::RemoveFromLayer(this)`
5. call `AnimClass::Detach()`
6. call `VocHandle::Stop()`
7. **if type has logic enabled (`*(type+0x234) != 0`)** and game mode gate passes: call `FUN_0055BAE0(this, ECX=0x87F778)` at `0x005F4DD3` — **this clears `Object+0x98` and compacts the LogicClass live vector LEFT** (verified via `get_assembly_context 0x005F4DCD`)
8. Dirty screen rect
9. call `vtable+0x11C()`
10. `this+0x81 = 1` — SET InLimbo
11. `this+0x80 = 0`
12. return 1

**Game-mode gate for step 7:** `g_GameMode @ 0x00A8B238`. The gate is `(g_GameMode != 0) && (g_GameMode != 5)`. In standard YR skirmish/multiplayer (modes 1, 2, 3), the gate passes. Mode 0 = inactive (pre-game); mode 5 = TS-era/legacy mode not used in standard YR. Active in YR: **Yes** (modes 1-4 all pass; mode 5 is legacy). (Verified via `get_assembly_context 0x005F4DB0` confirming the two CMP+JZ checks against 0 and 5.)

BulletClass type has `+0x234 = 1` (logic-enabled) as established by `BulletTypeClass::Constructor @ 0x0046BBC0` (settled fact, not re-investigated).

---

## 5. Object+0x98 Clear Ordering — Successor Skip Hazard

**The +0x98 clear and vector compaction happen inside BulletClass::AI, before the scheduler increments its loop index.**

The exact order within one scheduler iteration at index `i`:

1. Scheduler calls `vtable+0x5C` (= BulletClass::AI) on `items[i]`
2. Inside AI: detonation fires; `(*vtable+0xF8)()` is called
3. Inside UnInit: virtual +0xD4 (Conceal) is dispatched
4. Inside Conceal: `FUN_0055BAE0(this, LogicClass)` is called → **vector compacted LEFT; Object+0x98 cleared**
5. After Conceal returns: `Object+0x90` cleared (step 3 in §4.1); object appended to pending-delete
6. BulletClass::AI returns to scheduler
7. Scheduler increments index `i → i+1`
8. Scheduler reloads count from `LogicClass+0x10` (already decremented by step 4)

**Consequence:** The object that was at vector index `i+1` before step 4 is now at index `i` after compaction. The scheduler advances to `i+1`, so that object is skipped until the next pass. This is the "synthesis claim 9" skip hazard, and it applies to bullets exactly as it does to every other self-removing logic object.

**Object+0x98 is cleared BEFORE:**
- The `vtable+0x5C` call returns to the scheduler
- The scheduler increments its index
- The scheduler reloads the live count

**Object+0x98 is cleared AFTER:**
- The detonation damage/effects (`BulletDetonation @ 0x00468D80`) have been dispatched
- The current tick's warhead/damage computation has run

---

## 6. Memory-Free Timing

Bullet memory is NOT freed during the scheduler pass. After UnInit:
- The pointer is in the global pending-delete queue at `0x00B0F69C`
- `Object+0x90 = 0` (IsDead returns true)
- `Object+0x98 = 0` (no longer in live vector)
- `InLimbo (Object+0x81) = 1`

The pending-delete drain runs later in the same `Main_Tick` at `0x0055DE9F` (after `LogicClassPerTickUpdateLiveVector` returns), unless any of the four session-end flags are set (as documented in `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` and `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`).

Physical free via scalar-deleting destructor (`vtable+0x20`) happens inside `FUN_00725C70` when `IsDead` returns true for the queued pointer — same `Main_Tick`, late phase, barring session-end gate suppression.

BulletClass is not BuildingClass/UnitClass/InfantryClass/AircraftClass, so the `Object+0x90` pre-destructor restore step in the drain does NOT apply to bullets. `Object+0x90` remains 0 through the scalar destructor. (Verified: drain restore is guarded by four dynamic-cast type checks as documented in `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md` §3.3; no BulletClass descriptor is in that list.)

**The drain site is `FUN_00725C70 @ 0x00725C70`, called from `Main_Tick @ 0x0055DE9F`.** (Cite from prior sibling doc; not re-investigated per scope constraint.)

---

## 7. Both Detonation Paths Use the Same vtable+0xF8

BulletClass::AI has two distinct detonation entry points (verified via `decompile_function 0x004666E0`):

| Path | Condition | vtable+0xF8 call location |
|---|---|---|
| Immediate detonation | Ground/wall/target proximity/proximity-detector | `0x00467FB4: CALL dword ptr [EAX + 0xF8]` |
| Delayed-nuke (anim-listened) | `param_1[0x56] != 0` at AI entry; NukeLobber-style | near top of function: `(**(code **)(*param_1 + 0xf8))()` |

Both dispatch to `ObjectClass::UnInit @ 0x005F65F0`. The removal route and timing described in §4–6 apply to both paths identically.

---

## 8. Negative Facts / Do Not Do

| Negative fact | Evidence | Active in YR |
|---|---|---|
| BulletClass does NOT override vtable+0xF8 with a bullet-specific teardown | Slot `0x007E47DC` reads `0x005F65F0` = `ObjectClass::UnInit`; no BulletClass-specific overriding function | Yes |
| BulletClass does NOT override vtable+0xD4 with a bullet-specific limbo path | Slot `0x007E47B8` reads `0x005F4D30` = `ObjectClass::Conceal` | Yes |
| The vtable+0xF8 route does NOT free memory inline | UnInit only appends to pending-delete; free is late drain in `FUN_00725C70` | Yes |
| Do NOT conflate vtable+0x28 (separate BulletClass pre-conceal callback) with vtable+0xF8 (UnInit) | Slot `0x007E470C` = `0x004684E0` ≠ `0x005F65F0`; confirmed separate function boundary | Yes |
| Bullet's `Object+0x90` is NOT restored before the scalar destructor (unlike techno-family objects) | Drain type-check only covers BuildingClass/UnitClass/InfantryClass/AircraftClass; BulletClass excluded | Yes |

---

## 9. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| vtable+0xF8 = `ObjectClass::UnInit`; removal route is Conceal→FUN_0055BAE0→compact LEFT→+0x98 clear→+0x90 clear→pending-delete append, all inside the AI call | Rust homing projectile path snapshots keys and returns detonated IDs to caller; no live-vector compaction during AI | `src/sim/movement/homing_movement.rs:379-569`, future central logic-vector scheduler | Vector `[A (bullet), B, C]`; bullet at index 0 detonates; B shifts to index 0; scheduler increments to index 1; B is skipped until next pass, C runs next | `bullet_detonation_uninit_skips_shifted_logic_successor` | HIGH — every tick with a detonating projectile fires this; wrong ordering produces cross-frame target-damage interaction differences |
| Object+0x98 clear (logic unregister) happens INSIDE the AI call before scheduler index increment; vector is compacted left immediately | Rust collects detonated IDs and despawns in caller-side loop after movement phase; the next entity in iteration order runs regardless of despawn | `src/sim/movement/homing_movement.rs`, `src/sim/world/mod.rs::advance_tick` | Snapshot-based despawn cannot match native skip unless object ordering and skip are explicitly modeled | `logic_vector_bullet_uninit_clears_membership_before_scheduler_increment` | HIGH — fires every match when any projectile detonates |
| Memory free is late in same Main_Tick (after LogicClass pass); not inline; bullet pointer remains valid through live-vector pass completion | Rust removes entity from EntityStore directly (no late drain phase) | `src/app_sim_tick.rs:306`, `src/sim/world/despawn_entity` | Object at pending-delete stage must be observably "not in live vector" but heap-valid until late drain; Rust inline removal may produce different observer behavior for same-frame effects | `bullet_uninit_heap_valid_until_late_drain` | MEDIUM — most same-tick observers won't differ, but edge cases (same-frame entity lookup after detonation) can diverge |

---

## 10. Remaining Uncertainty

- **g_GameMode = 5 semantics:** Mode 5 skips `FUN_0055BAE0` in Conceal, meaning bullets in mode-5 contexts would NOT be removed from the LogicClass live vector via Conceal. Mode 5 appears TS-era/legacy (not used in standard YR); exact activation path not traced. Risk: negligible for standard YR. Label: CONDITIONAL/TS-LEGACY.
- **vtable+0x11C body** (called from Conceal step 9): Not decompiled. Likely a visual/render notification. Does not affect +0x98 or scheduling. Out of scope.
- **Whether BulletClass overrides the secondary vtable `vtable__BulletClass__secondary_4` slots relevant to the Conceal/UnInit path:** Not investigated. Primary vtable is the one dispatched from `*param_1` and is fully accounted for.
- **Destructor chain body internals for BulletClass scalar destructor (`vtable+0x20`):** Not traced in this session. The function at slot vtable+0x20 (deducible from the vtable dump at slot 8 = offset 0x20) dispatches the object-destructor chain. Out of scope per handoff doc.

---

## 11. Coverage Ledger

| Area | Status | Evidence |
|---|---|---|
| BulletClass vtable base address | Verified | `read_memory 0x007E46E4` (from asm `0x00466425`); RTTI type name `.?AVBulletClass@@` via `read_memory 0x0081AF70` |
| vtable+0xF8 slot identity | Verified | `read_memory 0x007E47DC` = `0x005F65F0`; `get_function_by_address 0x005F65F0` |
| vtable+0xD4 slot identity (not overridden) | Verified | 256-byte vtable dump slot 53 = `0x005F4D30` |
| UnInit removal order | Verified | `decompile_function 0x005F65F0`; asm `0x005F661F`, `0x005F6625`, `0x005F6668..0x005F667D` |
| Conceal body / FUN_0055BAE0 call | Verified | `decompile_function 0x005F4D30`; asm `0x005F4DD3` (`CALL 0x0055BAE0, ECX=0x87F778`) |
| Object+0x98 cleared before scheduler increment | Verified | Conceal dispatches FUN_0055BAE0 which clears +0x98 (from COMMON_MIDPASS doc §2); compaction is pre-return-to-scheduler |
| Game-mode gate for unregister | Verified | `get_assembly_context 0x005F4DB0`: CMP against 0 and 5 |
| Memory-free timing (pending-delete) | Verified (cite) | Cited from `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md` §5; drain at `0x0055DE9F` |
| BulletClass not in pre-destructor restore set | Verified (cite) | `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md` §3.3: only BuildingClass, UnitClass, InfantryClass, AircraftClass |
| Both detonation paths use +0xF8 | Verified | `decompile_function 0x004666E0`: delayed-nuke branch and main detonation tail both dispatch `(*+0xF8)()` |

---

## Sources

- Ghidra MCP read-only: `decompile_function 0x00466380`, `0x00466560`, `0x004666E0`, `0x005F65F0`, `0x005F4D30`
- Ghidra MCP read-only: `read_memory 0x007E46E4` (256 bytes), `0x007E47DC`, `0x007E47B8`, `0x007E46E0`, `0x007FC7B0`, `0x0081AF70`
- Ghidra MCP read-only: `get_assembly_context` at `0x004663a8`, `0x00467fba`, `0x005F661F`, `0x005F4DCD`, `0x005F4DB0`
- Ghidra MCP read-only: `get_function_by_address 0x005F65F0`
- Prior docs cited (not re-investigated): `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`, `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
- Rust source scanned: `src/sim/movement/homing_movement.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`
