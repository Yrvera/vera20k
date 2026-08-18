# BulletClass Construction, Pool, Registry, and Pre-Fire Membership Init

**Address(es):** `BulletClass::Allocate @ 0x0046B050`, `BulletClass::Init @ 0x004664C0`, `BulletClass::Constructor @ 0x00466380`, `BulletClass::Destructor @ 0x00466560`, `ObjectClass::Constructor @ 0x005F3900`, `COM ClassFactory::CreateInstance @ 0x006C5086`; globals `g_BulletClass_Array @ 0x00A8ED40`, `g_BulletClass_Array_Count @ 0x00A8ED44`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** BulletClass allocation mechanism (pool vs. heap), g_BulletClass_Array registry existence/purpose, and the exact initial state of lifecycle bytes (InLimbo +0x81, IsAlive +0x90, IsInLogic +0x98) between construction and `BulletClass::Fire`  
**Non-Scope:** full BulletClass::Fire trajectory math, detonation, homing, or LogicClass::PerTickUpdate scheduler internals  
**Confidence:** High — all claims cite live Ghidra decompile or assembly context  
**Active in YR:** Yes — every weapon discharge that creates a projectile runs this pipeline

---

## 1. Working Notes Gate

**Target question:** Is BulletClass allocated from a recycled object pool or plain heap? Does `g_BulletClass_Array` exist, and is it the scheduler? What are the exact lifecycle byte values at construction before `Fire`?

**Non-goals:** Full trajectory setup in `Fire`, LogicClass scheduler internals, save/load stream format, full destructor chain.

**Evidence needed to mark COMPLETE:** decompile of Allocate (0x0046B050), Constructor (0x00466380), Destructor (0x00466560), ObjectClass::Constructor (0x005F3900); assembly context at lifecycle byte store addresses (0x005F392a, 0x005F3930, 0x005F398d); xref scan of g_BulletClass_Array (0x00A8ED40) to identify all usage sites.

**Stop conditions:** Stop after allocation mechanism, registry purpose, and lifecycle byte initial values are verified; record save-game serialization detail as out of scope.

---

## 2. Overview

BulletClass is allocated via **COM `CoCreateInstance`**, which wraps a plain `operator_new(0x160)` — there is no recycled free-list pool. Each allocation appends `this` to `g_BulletClass_Array` (a growable pointer vector at `0x00A8ED40`); this array is the **save-game iteration registry**, not the AI scheduler. At construction, before `BulletClass::Fire` is called: `InLimbo (+0x81) = 1`, `IsAlive (+0x90) = 1`, `IsInLogic (+0x98) = 0`. The active-vector registration happens inside `BulletClass::Fire → ObjectClass::Reveal → FUN_0055BAA0`, not in the constructor.

**Active in YR:** Yes.

---

## 3. Allocation Mechanism — COM + operator_new, No Pool

**Finding:** BulletClass uses **COM `CoCreateInstance`** (CLSCTX_INPROC_SERVER|CLSCTX_LOCAL_SERVER = 7) to allocate. The COM factory at `0x006C5086` calls `operator_new(0x160)` directly and then `BulletClass::Constructor`. There is no recycled free-list or fixed-size pool array.

**Evidence:** Decompile of `COM ClassFactory::CreateInstance @ 0x006C5086`:
```c
pvVar1 = operator_new(0x160);          // 352 bytes per instance
piVar2 = BulletClass__Constructor();   // sets vtable, chains up
iVar3 = QI(piVar2, param_3, param_4); // COM QueryInterface
```
(verified via `decompile_function 0x006C5086`)

**Evidence:** Decompile of `BulletClass::Allocate @ 0x0046B050`:
```c
HRESULT hr = CoCreateInstance(&CLSID_BulletClass, NULL, 7, &IID_BulletClass, &block);
if (FAILED(hr)) return NULL;
BulletClass__Init(/* args forwarded */);
return block;
```
(verified via `decompile_function 0x0046B050`)

**Allocation order determinism:** Each call to `CoCreateInstance` → `operator_new` returns a fresh heap address; allocation order follows call order (TechnoClass::Fire_At call sequence). The COM CLSID is a game-startup-registered in-process class. No pool reuse, so EntityStore stable-id order in Rust is a structural difference — see Implementation Handoff §6.

**Active in YR:** Yes — `BulletClass::Allocate` is called by `TechnoClassFireAtSpawnsBullet @ 0x006FDD50`, `BuildingClass__Mission_Missile @ 0x0044C980`, `FUN_0041BC30`, `FUN_006E38F0`, `FUN_0070D690`, `House__LaunchNukeDown @ 0x006E3410`, `SuperClass__Launch @ 0x006CC390` (verified via `get_function_callers 0x0046B050`).

---

## 4. g_BulletClass_Array Registry — Purpose and Non-Scheduler Proof

**Finding:** `g_BulletClass_Array @ 0x00A8ED40` is a growable COM-managed pointer vector. The constructor appends `this` to it; the destructor removes `this` from it by linear-scan left-compaction. Its purpose is **save-game serialization iteration**, not per-tick scheduling.

**Evidence — constructor append (decompile_function 0x00466380):**
```c
if (DAT_00a8ed48 <= g_BulletClass_Array_Count) {
    // grow-or-abort logic
}
iVar1 = g_BulletClass_Array_Count * 4;
g_BulletClass_Array_Count = g_BulletClass_Array_Count + 1;
*(undefined4 **)(g_BulletClass_Array + iVar1) = param_1;  // append this
```
(verified via `decompile_function 0x00466380`)

**Evidence — destructor removal (decompile_function 0x00466560):**
```c
iVar1 = (**(code **)(DAT_00a8ed40 + 0x10))(&local_4); // find index
g_BulletClass_Array_Count = g_BulletClass_Array_Count + -1;
// left-compact remaining entries
```
(verified via `decompile_function 0x00466560`)

**Evidence — save-game usage:** XRef scan of `0x00A8ED40` shows reads from `FUN_0067d300` (body `0x0067d300–0x0067e43a`). Assembly at `0x0067e00c` loads `EDX = 0xa8ed40` and calls `FUN_006802f0`, which is an `OleSaveToStream`-pattern serializer — the same pattern used immediately before it for `g_AnimTypes_Array`, `g_TubeArray`, and other per-class registries. (verified via `get_xrefs_to 0x00A8ED40` + `get_assembly_context 0x0067e00c`)

**Negative fact — NOT the AI scheduler:** The AI scheduler is the LogicClass active vector at `0x87F778` (already settled; `FUN_0055BAA0` tail-appends there). `g_BulletClass_Array` has zero xrefs inside `LogicClass::PerTickUpdate @ 0x0055AFB0` or `FUN_0055BAA0`. The two systems are disjoint: g_BulletClass_Array tracks all allocated bullets for save/load; the active vector tracks only revealed bullets for per-tick AI.

**Active in YR:** Yes (save-game runs in YR).

---

## 5. Initial Lifecycle Byte State — Verified Assembly

All three lifecycle bytes are set inside `ObjectClass::Constructor @ 0x005F3900`, which is called as part of the `BulletClass::Constructor` chain (`BulletClass::Constructor` at `0x00466384` calls `ObjectClass::Constructor`).

**Exact assembly stores (verified via `get_assembly_context 0x005F3900`):**

| Byte | Offset | Assembly address | Immediate | Value at construction |
|------|--------|-----------------|-----------|----------------------|
| InLimbo | +0x81 | `0x005F392A: MOV byte ptr [ESI + 0x81], AL` | AL = 1 | **1** (in limbo) |
| IsAlive | +0x90 | `0x005F3930: MOV byte ptr [ESI + 0x90], AL` | AL = 1 | **1** (alive) |
| IsInLogic | +0x98 | `0x005F398D: MOV byte ptr [ESI + 0x98], BL` | BL = 0 | **0** (not in active vector) |

IsVisible (+0x99) is set to 1 by `0x005F3936: MOV byte ptr [ESI + 0x99], AL` (AL=1).

**Invariant before Fire:** a freshly constructed bullet is alive, in limbo, and NOT in the logic active vector. It is appended to `g_BulletClass_Array` (save/load registry) but NOT to the scheduler vector.

**Active in YR:** Yes — ObjectClass::Constructor runs for every BulletClass instance.

---

## 6. Registration Handoff — Constructor vs. Fire

**Finding:** The active-vector append (LogicClass membership, i.e., scheduling) does NOT happen in the constructor. It happens via the `BulletClass::Fire → ObjectClass::Reveal → FUN_0055BAA0` chain.

**Evidence:**  
- Decompile of `BulletClass::Fire @ 0x00468670` shows the first real call is `uVar6 = ObjectClass__Reveal()` and it returns immediately if that returns 0. (verified via `decompile_function 0x00468670`, Ghidra label `BulletClassFireRevealArmAndSubmit`)
- `FUN_0055BAA0` is the function that sets `Object+0x98 = 1` and tail-appends to `0x87F778` — already settled.

**Pre-Fire Conceal call:** `TechnoClass::Fire_At` explicitly calls `ObjectClass::Conceal` (vtable+0xD4 → `0x005F4D30`) between allocation and the `Fire` call. This re-asserts InLimbo=1 if the constructor left any uncertainty. (verified: BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md §3, step 3; also decompile of Fire shows Reveal is the entry gate)

**Active in YR:** Yes.

---

## 7. BulletClass::Init (0x004664C0) — Lifecycle Bytes Untouched

`BulletClass::Init` writes: Type (+0xAC), Owner (+0xB0), Target (+0x10C), TargetSpeed (+0x110), Warhead (+0x128), Bright (+0xE0), Damage (+0x6C), HouseColorIndex (+0x114), RockerScale (+0x150 = 0x100), BounceAnim (+0x154 = 0), IsWaitingForAnim (+0x158 = 0), AnimTimer (+0x12D from BulletTypeClass+0x2F6).

**It does NOT write InLimbo, IsAlive, or IsInLogic.** Those values set by ObjectClass::Constructor remain unchanged through Init. (verified via `decompile_function 0x004664C0` — no stores to +0x81, +0x90, or +0x98)

**Active in YR:** Yes.

---

## 8. Implementation Handoff

### H-1 — No Pool, EntityStore Key-Order Differs from gamemd Heap-Address Order

**Verified behavior:** gamemd allocates via `operator_new` each time; address order = allocation call order within `TechnoClass::Fire_At` sequence. No pool reuse, no stable recycled addresses.  
**Rust delta:** EntityStore assigns monotonically incrementing stable IDs; ordering is deterministic but differs from gamemd's heap-address interleave.  
**Affected surface:** Any system that iterates `g_BulletClass_Array` or the active-vector in address order (save/load, detonation callbacks, multi-bullet spread ordering).  
**Acceptance scenario:** Two Grizzly tanks firing simultaneously at the same target produce two bullets whose AI runs in the same relative order in gamemd and Rust. Multi-bullet spread weapons (e.g., FlakTrooper) fire sub-bullets in the same sequence.  
**Proposed test name:** `test_bullet_spawn_order_deterministic_multi_source`  
**Risk:** Low for single-source weapons; low-to-medium for spread/cluster weapons where sub-bullet spawn ordering inside `BulletClass::SpawnShrapnel` may affect RNG consumption order.

### H-2 — IsInLogic=0 at Construction; Active-Vector Entry via Fire Only

**Verified behavior:** `Object+0x98 = 0` after constructor; `Object+0x81 = 1` (InLimbo). Bullet is NOT in scheduler between `Allocate` and `Fire`.  
**Rust delta:** If Rust inserts a projectile entity into the per-tick update roster during spawn rather than at the Fire equivalent, the bullet gets an AI tick before its velocity/source coordinates are fully set.  
**Affected surface:** `src/sim/combat/combat_weapon.rs` — the site that creates projectile entities.  
**Acceptance scenario:** A newly spawned bullet does not receive a movement/AI tick on the same frame it is created.  
**Proposed test name:** `test_bullet_no_ai_tick_before_fire`  
**Risk:** High if violated — a spurious early AI tick can move the bullet one step before trajectory is committed, causing a 1-tick positional offset that is player-visible on fast projectiles.

### H-3 — g_BulletClass_Array Is Save/Load Registry, Not Scheduler

**Verified behavior:** Array is written in constructor, cleared in destructor, read by save-game serializer at `0x0067e00c`. The LogicClass active vector at `0x87F778` is entirely separate.  
**Rust delta:** Rust EntityStore is the equivalent of g_BulletClass_Array (lifetime ownership). The per-tick update loop driven by EntityStore iteration is the equivalent of the LogicClass active vector. These are already two distinct concepts in Rust.  
**Affected surface:** `src/sim/combat/mod.rs` — projectile entity lifetime vs. update roster.  
**Acceptance scenario:** A bullet that exists in EntityStore but has not yet been "revealed" (pre-Fire) does not appear in the per-tick update sweep.  
**Proposed test name:** `test_bullet_entity_store_vs_active_roster_disjoint`  
**Risk:** Medium — a design that uses EntityStore iteration directly as the AI scheduler would give pre-Fire bullets a tick, violating H-2.

---

## 9. Negative Facts / Do Not Do

1. **Do NOT model a recycled object pool for BulletClass.** Allocation is plain `operator_new(0x160)` inside COM factory. No free-list, no fixed-size arena. (evidence: `decompile_function 0x006C5086`)

2. **Do NOT treat g_BulletClass_Array as the AI scheduler.** It has zero xrefs inside `LogicClass::PerTickUpdate @ 0x0055AFB0`. It is a save/load iteration registry. (evidence: `get_xrefs_to 0x00A8ED40` shows only constructor, destructor, init, and `FUN_0067d300` save routine)

3. **Do NOT insert a new bullet into the logic active vector at constructor or Init time.** IsInLogic=0 is the correct post-constructor state. Active-vector entry is gated on `BulletClass::Fire → Reveal`. (evidence: `ObjectClass::Constructor @ 0x005F398D` stores BL=0 to +0x98)

4. **Do NOT clear InLimbo at constructor time.** InLimbo=1 is the correct post-constructor state. `TechnoClass::Fire_At` calls `ObjectClass::Conceal` between Allocate and Fire, reinforcing limbo. (evidence: `ObjectClass::Constructor @ 0x005F392A` stores AL=1 to +0x81)

5. **Do NOT reference `gamemd.exe` addresses or binary offsets in Rust source code comments** (per MEMORY.md: `feedback_no_engine_refs_in_comments.md`).

---

## 10. Remaining Uncertainty

- **g_BulletClass_Array grow-function identity:** The vector's grow callback is `DAT_00A8ED40 + 8` (a function pointer in the COM-vector control block). Its exact realloc strategy (doubling? +10?) is not verified. Practically irrelevant for Rust since Rust uses EntityStore, not this array.

- **ObjectClass::Constructor append to three additional arrays (`DAT_00B0F724`, `DAT_00B0F674`, `DAT_00B0F678`):** The `ObjectClass::Constructor` decompile shows appends to three more global arrays beyond `g_ObjectClass_Array`. Their purposes are not in scope for this report. At least one is the Detach listener roster (covered in DETACH_LISTENER_ROSTER_MUTATION_RULES_RESWARM_20260528.md).

- **CoCreateInstance CLSID/IID registration site:** The COM class factory registration that maps `DAT_007E96E0` (CLSID_BulletClass) to `0x006C5086` (CreateInstance) is not traced to a game-init function. Not relevant to Rust (no COM used).

---

## 11. Relation to Existing Research Docs

- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` §Constructor Call Chain: accurate, this report adds exact assembly addresses and byte values for lifecycle fields.
- `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md` §4: accurate on COM+CoCreateInstance. This report adds the non-pool confirmation and save-game evidence for g_BulletClass_Array.
- `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md` §2: lifecycle summary matches. No conflict.
- `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`: no conflict; complementary.
