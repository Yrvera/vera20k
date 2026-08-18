# Slice 6 — Deferred-Delete Queue, One-Tick Dying Window & Detach Order — Ghidra Research Report

**Address(es):** `0x005F65F0` (`ObjectClass::UnInit`), `0x007258D0` (`Detach_From_All_Lists`, = the old `FUN_007258D0`), `0x00725C70` (`ProcessPendingDelete`), `0x0055D360` (`Main_Tick`), PendingDeleteList `DynamicVectorClass` @ `0x00B0F698`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the three residual questions for ObjectSubstrate Slice 6 — (Q1) the detach/observer-notify teardown order on death; (Q2) the `ProcessPendingDelete` drain and its position in the tick; (Q3) mutual-reference same-tick death determinism. EXTENDS the verified `ObjectClass::UnInit` chain in `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` §3.9 — does not re-verify it.
**Non-Scope:** the full `ObjectClass`/`TechnoClass` vtable map; the exact identity of the `vtable+0x44` drain-readiness predicate (deferred); precise per-field mapping of the Techno detach helpers (deferred).
**Confidence:** High (all three scope questions decompiled and traced from `Main_Tick`).
**Active in YR:** Yes — `Main_Tick` @ `0x0055D360` is the game loop; `UnInit`/`ProcessPendingDelete` are on every object death.

## 1. Overview

gamemd destroys an object in two phases. **Synchronously** in `ObjectClass::UnInit`: detach/notify observers, run Limbo (→ Conceal → occupancy-unmark + deselect), clear `IsAlive`, and **append the object pointer to a global pending-delete vector**. The actual destructor/free is **deferred** to `ProcessPendingDelete`, which runs once near the end of `Main_Tick`. Between `UnInit` and that end-of-tick drain, the object is **valid memory but `IsAlive=0`**, off all cell lists, unselectable — resolvable by pointer/id for the remainder of the tick. The drain is **conditional**: each queued object is freed only when its `vtable+0x44` readiness predicate returns nonzero, otherwise it is left in the queue for a future tick.

## 2. Key Offsets / Globals (verified)

| Symbol | Address / offset | Meaning | Evidence |
|--------|------------------|---------|----------|
| `IsAlive` | object `+0x90` (`param_1[0x24]`) | cleared to 0 in UnInit after Limbo | decompile `0x005F65F0` |
| AttachedBomb | object `+0x38` (`param_1[0xe]`) | gates `BombClass::Defuse` | decompile `0x005F65F0` |
| Foot flag | object `+0x14` bit 0 (`*(byte*)(param_1+5) & 1`) | gates `FootClass::EMPPassengers(0)` | decompile `0x005F65F0` |
| Techno flag | object `+0x14` bit 1 (`>>1 & 1`) | gates the Techno detach extras in `Detach_From_All_Lists` | decompile `0x007258D0` |
| Limbo slot | `vtable+0xD4` | called in UnInit; chains Conceal→Deselect (occupancy-unmark synchronous) | decompile `0x005F65F0` |
| NotifyOfRemoval slot | `vtable+0x28` | called on each RemoveListener in `Detach_From_All_Lists` | decompile `0x007258D0` |
| GetRTTIType slot | `vtable+0x2c` | RTTI key for the detach dispatch | decompile `0x007258D0` |
| Scalar-deleting dtor slot | `vtable+0x20` | called by `ProcessPendingDelete(1)` to actually free | decompile `0x00725C70` |
| **Drain-readiness slot** | `vtable+0x44` | predicate in `ProcessPendingDelete`; 0 ⇒ leave queued, nonzero ⇒ free now | decompile `0x00725C70` |
| PendingDeleteList vtable | `0x00B0F698` | `DynamicVectorClass` vtable ptr (grow via `+0x10`, add-helper via `+8`) | decompile `0x005F65F0`, `0x00725C70` |
| PendingDeleteList array base | `0x00B0F69C` | `int*` element buffer of queued object pointers | decompile `0x005F65F0`, `0x00725C70` |
| PendingDeleteList capacity | `0x00B0F6A0` | max elements before grow | decompile `0x005F65F0` |
| PendingDeleteList count | `0x00B0F6A8` | active length (append index) | decompile `0x005F65F0`, xrefs |
| PendingDeleteList cap-increment | `0x00B0F6AC` | grow step | decompile `0x005F65F0` |

## 3. Core Logic

### 3.1 `ObjectClass::UnInit` @ `0x005F65F0` (verified live — confirms & sharpens LIMBO §3.9)

```
if (AttachedBomb != 0)               BombClass::Defuse()
if (this && (byte[+0x14] & 1))        FootClass::EMPPassengers(0)     // transport pax get EMP'd, not killed
Detach_From_All_Lists(this)                                          // ← observer/notify dispatch (§3.2)
this->vtable[+0xD4]()                 // Limbo → Conceal → occupancy-unmark + Deselect (SYNCHRONOUS)
this[+0x90] = 0                       // IsAlive = 0
PendingDeleteList.Add(this)           // tail-append to DynamicVectorClass @ 0x00B0F698 (§3.3)
```

**Ordering (critic #6 / #9 answer):** detach/notify → Limbo/Conceal (occupancy-unmark, deselect) → `IsAlive=0` → enqueue, all **synchronous**. Only the *free* defers. When listeners are notified (step 3) the target is still `IsAlive=1`, still marked, still in its cell lists. `IsAlive` is cleared *after* Limbo.

**Label drift:** the old report's "`FUN_007258D0` (global cleanup) → DetachAll" is a **single** function — `FUN_007258D0` has been relabeled `Detach_From_All_Lists` and is the only call between `EMPPassengers` and Limbo. The prompt's "DetachAll @ 0x005F6612" is the call instruction to it inside UnInit. (Label verified by reading the UnInit body, not the name.)

### 3.2 `Detach_From_All_Lists` @ `0x007258D0` (verified) — it is an OBSERVER-NOTIFY dispatch, not a fixed field-null routine

This is the headline architectural finding. gamemd does **not** null a fixed set of cross-reference fields on death. It:

1. Clears two **UI/render-layer** singletons if they point at the dying object: `DAT_0088098c` (a "current object" global) and `g_UIModeLock` (→ `FUN_004a8bf0(0)`, a command-mode reset). *Above sim — not a sim link.*
2. Reads RTTI via `vtable+0x2c`, then walks the matching per-class **RemoveListeners registry**, calling each listener's `vtable+0x28` (`NotifyOfRemoval(target, param2)`):
   - RTTI `0xd` House → `g_HouseClass_RemoveListeners` + `FUN_0055b880`
   - RTTI `4` Anim → `g_AnimClass_RemoveListeners`
   - RTTI `0xc` Factory → `g_FactoryClass_RemoveListeners` + `MapClass__UnregisterBridgeRepairHut`
   - RTTI `0x22` Team, `0x26` Trigger, `0x2c` Tag (+`UnregisterBridgeRepairHut`+`FUN_0055b880`), `0x2f/0x30` TriggerType, `0x33`, `0x3c` Neuron — each walks its own listener array
   - RTTI `0x18` clears singleton `DAT_00a8ed78`
3. For Abstract-but-not-AbstractType objects: walks the master registry `DAT_00b0f674` (`NotifyOfRemoval`) + `FUN_00678850`.
4. For **Techno** objects (`byte[+0x14] bit 1`): walks `DAT_00b0f724` listeners, then runs extras in this order: `FUN_00439150` → `SpawnRetreat__Remove` → a `DiskLaserClass__DetachFromObject` loop (`g_DiskLaserClass_Array_Count`×) → if RTTI ∈ {Infantry `0xf`, Unit `1`, `2`} `FUN_00413490` → `FUN_00733160` → `g_Tactical->vtable[+0x28](target,1)` (render/map view notify) → `FUN_0055b880`.

**Port mapping (the design's `detach_all_links` set):** the port resolves `last_attacker_id` / `capture_target` / `bunker_occupant` / `garrison_original_owner` / `radio_contacts` by a **fixed scan** because it has no per-class observer registry. That is an acceptable Rust-native translation of the publish/subscribe mechanism *provided* the port performs the detach **at the same point** (synchronously in `uninit`, before conceal/occupancy-unmark and before clearing the alive flag). The exact gamemd helper→port-field mapping (`FUN_00439150`, `FUN_00413490`, `SpawnRetreat__Remove`) is deferred (see §8) — secondary because the port does not mirror the registry structure, only the timing/visibility contract.

### 3.3 PendingDeleteList append (verified) — `DynamicVectorClass<ObjectClass*>` @ `0x00B0F698`

```
if (capacity <= count) {                       // 0x00B0F6A0 <= 0x00B0F6A8
    if (flag==0 && capacity!=0) return;        // non-growable + already-allocated → DROP (no enqueue)
    if (cap_increment < 1)      return;        // no growth allowed → DROP
    if (vtable[+8](count+capacity_inc, 0) == 0) return;   // grow failed → DROP
}
array[count] = this;  count += 1;              // tail-append at 0x00B0F69C[count]
```

**Edge case (record):** on a grow failure / non-growable-full vector the object is **silently not enqueued** — it would never be freed by the drain. Unreachable in practice (the vector grows), but it is the literal behavior. The append is a plain tail-append; ordering in the queue = death order.

### 3.4 `ProcessPendingDelete` @ `0x00725C70` (verified) — the conditional end-of-tick drain

```
i = 0
while (i < count) {                            // count = 0x00B0F6A8
    obj = array[i]                             // 0x00B0F69C[i]
    if (obj->vtable[+0x44]() == 0) { i++; continue }   // NOT READY → leave queued, advance
    // READY: remove ALL occurrences of obj (vector find-remove via vtable[+0x10]), compacting + decrementing count
    if (obj->vtable[+8](obj) != 0) {           // RTTI/destroy gate (uses FUN_007caa05/FUN_007c8ad3 dynamic-cast helpers)
        obj[+0x90] = 1                          // sets a byte at +0x90 on the matched-RTTI branch (see note)
        obj->vtable[+0x20](1)                   // scalar-deleting destructor → frees memory
    }
}
```

- **Conditional free.** The `vtable+0x44` predicate gates the free. If it returns 0 the object stays in the queue and is retried at the next drain — so the valid-but-dead window is **"until the predicate clears at an end-of-tick drain," ≥1 tick**, not a fixed single tick. For the common case the predicate is ready and the object is freed at the first end-of-tick drain after death (same tick it died in).
- **Note on `obj[+0x90]=1`:** this write on the matched-RTTI branch is at the same byte UnInit cleared to 0 (`IsAlive`). Its purpose during the destroy path is a minor detail (likely a re-mark before the dtor walks); recorded, not load-bearing for the slice.
- Removal compacts the vector (shift-down) and removes *all* occurrences of the pointer (defensive against double-enqueue).

### 3.5 `Main_Tick` ordering @ `0x0055D360` (verified) — WHERE the drain runs (Q2)

The per-tick caller of `ProcessPendingDelete` is `Main_Tick`, call at `0x0055DE9F`, in this order:

```
... Process_Command() ...
Map__Logic()
RenderFrame_main()
(save/load desync-hash block over g_CurrentObjects)
LogicClassPerTickUpdateLiveVector()      // ← the main per-tick OBJECT UPDATE pass (deaths happen here)
... lightning/effects (FUN_004a9840 ×4), FUN_00637550, FUN_005d4430 ...
Network_ServiceLoop()
g_CurrentFrameCounter += 1
FUN_0055e160()
FUN_00725c70()                           // ← ProcessPendingDelete — END OF TICK (0x0055DE9F)
FUN_00637270()
```

So within a tick: objects that die during `LogicClassPerTickUpdateLiveVector` are enqueued and remain **valid-but-`IsAlive=0`** for the rest of that tick (any later same-tick system holding the pointer sees dead-limbo state, gating on `IsAlive`), then are freed at the end-of-tick drain (if ready). The object update pass itself is **live-count, not snapshot** (re-reads count after each vtable call — `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`).

### 3.6 Mutual-reference same-tick death (Q3 — verified by mechanism)

When A and B kill each other during the same `LogicClassPerTickUpdateLiveVector` pass: each `UnInit` enqueues its object (tail-append, death order) and sets its own `IsAlive=0` after its own Limbo. Neither free happens until the single end-of-tick drain. So for the remainder of the tick **both are resolvable-but-`IsAlive=0`** — a same-tick `last_attacker_id` lookup on either resolves to a valid, `Dying` object (not a freed/dangling pointer, not `None`). Determinism comes from (a) the deterministic object-pass order producing a deterministic enqueue order, and (b) the drain processing the queue in that order. No special-casing of mutual death exists; it falls out of the queue mechanism.

## 4. INI Keys

None. This is engine lifecycle plumbing with no INI-driven behavior.

## 5. Integration Points

- **Producer:** every lethal `ObjectClass::UnInit` (`0x005F65F0`), reached from damage→death, sell, despawn, etc.
- **Consumer/drain:** `ProcessPendingDelete` (`0x00725C70`) from `Main_Tick` (`0x0055DE9F`, end-of-tick), and from full-clear/scenario paths (`FUN_00534450` scenario teardown, `ScenarioClass__Full_Init` `0x00686B20`, `ReadMapOverlayPacks` `0x005FD2E0`).
- **Tick position:** drain runs after the object update pass and frame-counter increment, at the tail of `Main_Tick`.

## 6. Current Rust Implementation Status

Per the migration design (`ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` §8 Slice 6): the port currently frees on `uninit` immediately (no deferred queue, no one-tick Dying window), so same-tick references to a just-killed entity resolve to `None`. Slice 6 introduces the deferred queue + `Dying` window + a `detach_all_links` pass. The port pipeline's natural drain point is the **"building anims + cleanup"** stage of `World::advance_tick` (after AI / defeat detection, before the state hash) — the equivalent of gamemd's end-of-`Main_Tick` `ProcessPendingDelete`. This is a **hash-changing** behavior change (same-tick cross-reference resolution changes) → requires a `SNAPSHOT_VERSION` bump + new golden; **this report is the gamemd-side evidence artifact** the design's critic #4 requires.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `ObjectClass::UnInit` sequence | verified | decompile `0x005F65F0` | none |
| `Detach_From_All_Lists` observer dispatch + order | verified | decompile `0x007258D0` | per-helper field mapping (deferred) |
| PendingDeleteList layout + append + drop-edge | verified | decompile `0x005F65F0`, `0x00725C70` | none |
| `ProcessPendingDelete` drain + conditional free | verified | decompile `0x00725C70` | exact `vtable+0x44` predicate identity (deferred) |
| Drain position in tick | verified | decompile `0x0055D360`, callers of `0x00725C70` | none |
| Mutual-ref same-tick death determinism | verified | mechanism from `0x005F65F0` + `0x00725C70` | none |
| `vtable+0x44` readiness predicate (`ObjectClass::IsDead`) | verified | COL walk + `decompile_function 0x005F6690` | none — pure `IsAlive==0`; always true post-UnInit → free same tick |
| Techno detach helpers `FUN_00439150`/`FUN_00413490`/`SpawnRetreat__Remove`/`FUN_00733160` | verified | decompile `0x00439150`, `0x00413490` (empty), `0x00733160`, `SpawnRetreat__Remove` | none — all are subsystem-list removals, not 1:1 field-nulls (see §8) |

## 8. Open Questions — Final State

- `[RESOLVED]` Q1 detach order → `Detach_From_All_Lists` (observer-notify dispatch over per-RTTI RemoveListener registries + Techno extras) runs FIRST in `UnInit`, before Limbo/Conceal and before `IsAlive=0`; target is still alive+marked when listeners fire. (evidence: `decompile_function 0x005F65F0`, `0x007258D0`)
- `[RESOLVED]` Q2 `ProcessPendingDelete` → `FUN_00725C70`, called at end of `Main_Tick` (`0x0055DE9F`) after `LogicClassPerTickUpdateLiveVector`; conditional free gated on `vtable+0x44`, frees via `vtable+0x20(1)`. (evidence: `decompile_function 0x00725C70`, `0x0055D360`, `get_function_callers 0x00725C70`)
- `[RESOLVED]` Q3 mutual-ref death → both enqueued in deterministic death order, both valid-but-`IsAlive=0` until the single end-of-tick drain; determinism from object-pass order + queue order. (evidence: append `0x005F65F0` + FIFO/compact drain `0x00725C70`)
- `[RESOLVED]` label drift: `FUN_007258D0` == `Detach_From_All_Lists`; old report's two-step "FUN_007258D0 → DetachAll" is one function. (evidence: `decompile_function 0x005F65F0`)
- `[RESOLVED]` PendingDeleteList = `DynamicVectorClass` @ `0x00B0F698`; grow-or-drop edge documented. (evidence: `decompile_function 0x005F65F0`)
- `[RESOLVED]` `vtable+0x44` drain-readiness predicate = **`ObjectClass::IsDead`** @ `0x005F6690` — `return *(char*)(this+0x90) == 0` (true iff `IsAlive`/`+0x90` is 0). It is **not** a timer/animation gate. `UnInit` always sets `IsAlive=0` before enqueueing, so the predicate is **true for every queued object at the next drain → freed at the end of the same tick** it died in; the conditional is a safety/consistency check, not a multi-tick lingering mechanism. UnitClass does **not** override the slot (inherits the base). **Window confirmed: free at end of current tick, no common >1-tick lingering.** (evidence: COL walk `read_memory 0x007F5C6C` → COL `0x0080CC68`; `read_memory 0x0080CC74` → TypeDescriptor `0x00842D80`; `read_memory 0x00842D88` → mangled name `.?AVUnitClass@@`; slot `read_memory 0x007F5CB4` → `0x005F6690`; `decompile_function 0x005F6690`.)
- `[RESOLVED]` per-field mapping of the Techno detach helpers → they are **subsystem-list/registry removals, NOT 1:1 pointer-field nulls.** `FUN_00413490` (the Infantry/Unit/Aircraft extra) is an **empty no-op** (`decompile_function 0x00413490`). `FUN_00439150` removes the dying object from a subsystem list and nulls a back-ref at peer `+0x24` (`decompile_function 0x00439150`). `FUN_00733160` removes it from the tracker at `DAT_00b0fe6c` (`decompile_function 0x00733160`). `SpawnRetreat__Remove` removes it from the spawn/retreat manager list (`decompile_function SpawnRetreat__Remove`). The disk-laser loop detaches laser visuals. **Conclusion:** gamemd unregisters the dying object from *global registries/observer lists* (spawn, disk-laser, per-RTTI RemoveListeners, radio); it does **not** null the 1:1 cross-reference fields the port models (`last_attacker_id`, `capture_target`, `bunker_occupant`). Those are **gated-at-use on `IsAlive`** (the C1 stale-ref-degrades model). Port consequence: keep the radio-contact clear; do **not** add proactive nulling of the 1:1 fields (that would be drift) — rely on by-id None-degradation + `dying`-gating at the consumer.

- `[RESOLVED]` transport-with-cargo death: a dying Foot transport runs `FootClass::EMPPassengers` (mislabeled — `0x...`, called from `ObjectClass::UnInit`), which walks the cargo chain (`this+0x118`) and for each passenger calls `vtable+0xe0` (disconnect-from-cell) then `vtable+0xf8`. On InfantryClass, `vtable+0xf8` = `FootClass::UnInit` (`0x004DE5D0`) → `ObjectClass::UnInit` → enqueue. **So transport death destroys its passengers; each passenger enters its own Dying window and is freed at the same end-of-tick drain.** (evidence: `decompile_function FootClass__EMPPassengers`, COL walk InfantryClass `vtable@0x007EB058`, `read_memory 0x007EB150` → `0x004DE5D0`, `decompile_function 0x004DE5D0`)
- `[RESOLVED]` mind-control is **actively torn down** on the controller's death, NOT gated-at-use: `FootClass::UnInit` (`0x004DE5D0`) calls `CaptureManagerClass::FreeAll()` (if `this+0x2BC != 0`) **before** `ObjectClass::UnInit` — releasing all units the dier controls (they revert). This is a membership-style link (like radio), distinct from the gate-at-use 1:1 fields. Port consequence: a controller's death must revert its `mind_controlled` units (verify the port already does this in/around `uninit`; if not, it belongs in the detach step). (evidence: `decompile_function 0x004DE5D0`)
- `[RESOLVED]` building death adds no NEW cross-ref teardown for this slice: other entities referencing a dying building (capture_target, dock) follow the gate-at-use / `cleanup_dead(&alive)` rule; the building's own outgoing links vanish with the entity; `BuildingClass::Limbo` (`vtable+0xD4`) self-cleanup of its 8 slots is already documented in `BUILDING_DAMAGEFIRE_SLOT_CLEAR_DESTROY_LIFECYCLE_GHIDRA_REPORT.md`. (evidence: that report + resolved gate-at-use finding above)

This report fully answers Q1–Q3 plus the transport-cargo, mind-control, and building-death edges; all stated and discovered open questions are RESOLVED.

## 9. Visual/UI Composition Ledger

N/A — lifecycle plumbing, no visual surface. (Note: `Detach_From_All_Lists` clears the UI singletons `DAT_0088098c`/`g_UIModeLock` and notifies `g_Tactical` — those are above-sim render/UI concerns, out of `sim/` scope.)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| Death is two-phase: synchronous detach+conceal+occupancy-unmark+`IsAlive=0`+enqueue, deferred free | `0x005F65F0` | port frees immediately in `uninit` | `Simulation::uninit` / substrate uninit path | enqueue to a deferred-delete list; keep entity resolvable with a `Dying`/not-alive flag; conceal+occupancy-unmark stay synchronous | a unit killed mid-tick is absent from logic+occupancy but resolvable by id, `Dying`, for the rest of the tick | do NOT defer conceal/occupancy-unmark; only the slot-free defers |
| Drain runs once at end of tick, after the object update pass | `0x0055D360` call `0x0055DE9F` | no drain stage | `World::advance_tick` "building anims + cleanup" stage (before state hash) | flush the deferred-delete queue there; free entities still queued | `store.len()` correct after the cleanup stage; nothing references a freed id post-flush | do NOT flush mid-pass or before AI/defeat detection |
| Detach/notify happens before conceal and before `IsAlive=0` (target still alive+marked) | `0x007258D0` | port has no unified detach pass | new `detach_all_links(id)` in `uninit`, before conceal | null/clear cross-refs (`last_attacker_id`, `capture_target`, `bunker_occupant`, `garrison_original_owner`, `radio_contacts`) at that point | a killer's `last_attacker_id` set this tick resolves to the valid `Dying` victim; no dangling id after flush | do NOT clear refs after `IsAlive` is set / after conceal — order is observable |
| Same-tick mutual death: both valid-but-`Dying` until end-of-tick drain; deterministic by death order | `0x005F65F0`+`0x00725C70` | port frees immediately → one side sees `None` | deferred queue + `Dying` flag | both entities resolvable-but-`Dying` for the rest of the tick; freed in enqueue order at drain | two units that kill each other same tick: each resolves the other as `Dying`; deterministic across replay | mutual-ref death determinism test required (critic #9) |
| Drain is conditional on a readiness predicate (`vtable+0x44`); window is ≥1 tick | `0x00725C70` | n/a | deferred-delete drain | model the common case (free at end-of-tick) but do not hardcode "exactly one tick" — resolve the deferred predicate question before claiming exact-tick parity for lingering-death objects | (follow-up) | do NOT assume every dead object frees in exactly one tick without resolving the `vtable+0x44` predicate |

**Stale Docs / Follow-up Docs:** `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` §3.9 lists the UnInit step as "`FUN_007258D0` (global cleanup)" — update to `Detach_From_All_Lists` @ `0x007258D0` (observer-notify dispatch, not generic cleanup), and note the drain at `ProcessPendingDelete` (`0x00725C70`) is **conditional** (`vtable+0x44`), so "up to one tick" should read "≥1 tick, until the readiness predicate clears at an end-of-tick drain."

## Sources

- Ghidra (this session): `decompile_function 0x005F65F0` (UnInit), `0x007258D0` (Detach_From_All_Lists), `0x00725C70` (ProcessPendingDelete), `0x00534450` (scenario teardown — ruled out as the per-tick drain), `0x0055D360` (Main_Tick); `get_xrefs_to 0x00B0F6A8`, `0x00725C70`; `get_function_callers 0x00725C70`, `0x00534450`.
- Prior reports (extended): `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` §3.9/§7, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`, `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md` §3.8.
- Design: `ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` §8 Slice 6, critics #4/#6/#9.
