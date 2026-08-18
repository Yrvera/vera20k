# Pending Delete Drain / Destructor Timing - Reswarm 2026-05-28

**Address(es):** `ObjectClass::UnInit @ 0x005F65F0`, pending-delete drain `FUN_00725C70`, active caller `Main_Tick @ 0x0055DE9F`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** pending-delete queue storage/append, every static caller of the drain helper, the standard active YR drain point relative to the `LogicClass` live-vector tick, and the destructor/free sequence used for queued `ObjectClass` instances.  
**Non-Scope:** complete internals of every concrete destructor body, shutdown teardown ordering of every non-object class vector, and runtime-debugger validation of unusual modal/frame-skip flags.  
**Confidence:** High  
**Active in YR:** Yes. The standard active path is `Main_Tick -> LogicClassPerTickUpdateLiveVector -> ... -> frame increment block -> FUN_00725C70`.

## 1. Overview

`ObjectClass::UnInit` does not free an object. It clears `Object+0x90` to zero, appends the object pointer to a global pending-delete dynamic vector, and returns. The queued pointer is normally drained later in the same `Main_Tick`, after the `LogicClass` live-vector tick has returned and after the guarded current-frame increment block begins.

The drain helper is not part of the live-vector iteration. It scans the pending-delete vector, asks each queued object's virtual `+0x44` whether it is dead, removes all matching entries from the queue, calls the object's COM `Release` slot, conditionally restores `Object+0x90 = 1` for Building/Unit/Infantry/Aircraft-derived objects, then calls the object's scalar-deleting destructor `vtable+0x20` with delete flag `1`.

## 2. Class Layout / Key Offsets

| Field / global | Offset / address | Meaning | Evidence |
|---|---:|---|---|
| Pending-delete vector vtable/control | `0x00B0F698` | vector object/control pointer used for grow/find methods | `0x005F6651`, `0x00725C9A`, init at `0x0072586D` |
| Pending-delete data | `0x00B0F69C` | pointer array of queued objects | append `0x005F6677..0x005F667D`; drain read `0x00725C81` |
| Pending-delete capacity | `0x00B0F6A0` | current vector capacity | append capacity check `0x005F662C..0x005F6637`; init `0x0072585C` |
| Pending-delete allocated flag | `0x00B0F6A5` | free/grow ownership flag | append `0x005F6639..0x005F6645`; cleanup `0x007258A6..0x007258BD` |
| Pending-delete count | `0x00B0F6A8` | active queued pointer count | append increment `0x005F6668..0x005F6671`; drain loop `0x00725C71`, `0x00725D80` |
| Pending-delete growth step | `0x00B0F6AC` | grow amount, initialized to `10` | init `0x00725877`; append check `0x005F6647` |
| Object alive byte | `Object+0x90` | `0` after `UnInit`; `ObjectClass::IsDead` returns true when zero | `0x005F6625`, `0x005F6690`, `0x00725D67` |
| Object virtual `+0x44` | vtable offset `0x44` | `ObjectClass::IsDead`: returns `Object+0x90 == 0` | decompile `0x005F6690`; drain call `0x00725C8F` |
| Object virtual `+0x20` | vtable offset `0x20` | scalar-deleting destructor, called with flag `1` by drain | drain `0x00725D78..0x00725D7A`; examples `0x005F6DC0`, `0x00459F20`, `0x00746E80` |
| Object virtual `+0x08` | vtable offset `0x08` | COM `IUnknown::Release`; inherited stub returns `1` | `OBJECTCLASS_GHIDRA_REPORT.md`; decompile `0x00410310`; drain `0x00725CE8` |

## 3. Core Logic

### 3.1 Append side: `ObjectClass::UnInit @ 0x005F65F0`

The last verified swarm's immediate order holds and the append target is exact:

1. `0x005F6616`: call `Detach_From_All_Lists`.
2. `0x005F661F`: call virtual `+0xD4` (`Conceal`/limbo path for ordinary objects).
3. `0x005F6625`: write `Object+0x90 = 0`.
4. `0x005F662C..0x005F6666`: if `count >= capacity`, grow the global vector unless owned/non-growable or grow amount `< 1`.
5. `0x005F6668..0x005F6671`: read old `count`, increment `0x00B0F6A8`.
6. `0x005F6677..0x005F667D`: store `this` into `*(0x00B0F69C + old_count * 4)`.

The append has no duplicate suppression. If the same pointer is queued more than once, later drain removes all matching entries before destruction.

### 3.2 Queue initialization / teardown

The pending-delete vector is constructed by the startup initializer at `0x00725850`, referenced by data at `0x008152F8`. Assembly evidence:

- `0x00725857`: `0x00B0F69C = 0`.
- `0x0072585C`: `0x00B0F6A0 = 0`.
- `0x00725861`: byte at `0x00B0F6A4 = 1`.
- `0x00725868`: byte at `0x00B0F6A5 = 0`.
- `0x0072586D`: vector vtable/control `0x00B0F698 = 0x007E91EC`.
- `0x00725877`: grow amount `0x00B0F6AC = 10`.
- `0x00725881`: count `0x00B0F6A8 = 0`.
- `0x00725886`: registers cleanup through `0x007C978A`.

The cleanup thunk at `0x00725890`, referenced by data from `0x00725852`, switches the vector vtable/control to `0x007E920C`, frees the data pointer with `operator delete @ 0x007C8B3D` only when the data pointer is non-null and the ownership flag is nonzero, then clears data/capacity/ownership.

### 3.3 Drain side: `FUN_00725C70`

Pseudocode shape from decompile plus assembly:

1. Read pending count from `0x00B0F6A8`; if count `<= 0`, return (`0x00725C71..0x00725C7B`).
2. Iterate forward with index `ESI`, rechecking `ESI < 0x00B0F6A8` after each item (`0x00725D80..0x00725D86`).
3. Load queued object pointer from `*(0x00B0F69C + ESI*4)` (`0x00725C81..0x00725C89`).
4. Call virtual `+0x44` (`0x00725C8F`). If it returns zero, do not delete; increment `ESI` and leave the entry queued (`0x00725C92..0x00725C94`, `0x00725D7F`).
5. If `+0x44` returns nonzero, repeatedly find that pointer in the vector via vector vtable `+0x10`, decrement count, and compact entries left until no match remains (`0x00725C9A..0x00725CDF`).
6. Call COM virtual `+0x08` / `Release` on the object (`0x00725CE1..0x00725CEB`). The inherited implementation returns `1`, so ordinary objects continue into destruction.
7. Dynamic-cast checks test the object against four type descriptors: BuildingClass `0x00818D60`, UnitClass `0x00842D80`, InfantryClass `0x00825508`, and AircraftClass `0x00817B90` (`0x00725CF3..0x00725D61`). If any check succeeds, write `Object+0x90 = 1` before deletion (`0x00725D63..0x00725D67`).
8. If the object pointer is non-null, call virtual `+0x20` with argument `1` (`0x00725D6E..0x00725D7A`). This is the scalar-deleting destructor path; concrete scalar destructors run their class destructor chain and then call `operator delete @ 0x007C8B3D` when bit 0 of the flag is set.

The important small detail is step 7: `UnInit` clears `Object+0x90` to zero so `IsDead` returns true for the drain; immediately before freeing Techno-family object classes, the drain may restore `Object+0x90` to one. Destructors for those classes can therefore observe the object as alive again during teardown. This is not equivalent to leaving a Rust entity permanently `dying=true` until physical removal.

### 3.4 Destructor/free examples

The drain's deletion is virtual, so the exact leaf destructor depends on the object's vtable:

| Concrete path | Evidence | Sequence |
|---|---|---|
| Base `ObjectClass` | scalar dtor `0x005F6DC0`, object dtor `0x005F3B80` | `ObjectClass::Destructor`; if delete flag bit 0 set, `operator delete @ 0x007C8B3D` |
| `AnimClass` | dtor `0x00422900`; caller chain to `ObjectClass::Destructor` at `0x00422AB8` / `0x00422B10`; scalar slot reached virtually | detach/list cleanup, sound/voc detach, g_AnimClass array compaction, then `ObjectClass::Destructor`, then free through scalar dtor |
| `BuildingClass` | scalar dtor `0x00459F20` | `BuildingClass::Destructor`; delete flag bit 0 calls `0x007C8B3D` |
| `UnitClass` | scalar dtor `0x00746E80` (callee mislabeled as constructor in Ghidra, but scalar wrapper shape is clear) | leaf destructor body, then delete flag bit 0 calls `0x007C8B3D` |
| `TerrainClass` | scalar dtor `0x0071D350` | terrain destructor body, then delete flag bit 0 calls `0x007C8B3D` |
| `VoxelAnimClass` | scalar dtor `0x0074AB50` | `VoxelAnimClass::Destructor`, then delete flag bit 0 calls `0x007C8B3D` |

`ObjectClass::Destructor @ 0x005F3B80` itself removes the object from the pending-delete vector again by find/compact (`0x005F3BC7..0x005F3BEA`) before removing it from the global object array and other listener/abstract registries. The drain already removed all matching queue entries first, so this destructor-side removal is idempotent for normal drain.

## 4. INI Keys

No INI key directly gates pending-delete queue append, drain, destructor dispatch, or the active `Main_Tick` drain. The behavior is engine lifecycle infrastructure.

## 5. Integration Points

### Drain callers

Static xrefs to `FUN_00725C70`:

| Caller | Call site(s) | Active role |
|---|---|---|
| `Main_Tick` | `0x0055DE9F` | ordinary active-game drain, after `LogicClassPerTickUpdateLiveVector` and after frame-increment gate work |
| `ScenarioClass::Full_Init` | `0x00687C11` | map/load initialization drain after post-map init and before later scenario finalization work |
| `ReadMapOverlayPacks` | `0x005FD692` | drain after overlay pack construction/placement |
| `FUN_00534450` | many call sites from `0x005344C6` through `0x00534996` | global teardown/clear routine drains after each class-array deletion loop |

### Active tick timing

In standard active execution, `Main_Tick` calls `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` at `0x0055DC9E`, with `ECX = 0x87F778`. That live-vector function performs the main object `vtable+0x5C` loop before returning.

The pending-delete drain is later:

1. `0x0055DC9E`: call `LogicClassPerTickUpdateLiveVector`.
2. call `FUN_00637550`. (corrected 2026-05-29: doc previously listed `FUN_00647260` as step 2 and `FUN_00637550` as step 3 — order was WRONG; binary `decompile_function 0x0055DE00` shows `FUN_00637550` appears immediately after `LogicClassPerTickUpdateLiveVector`, before `FUN_00647260` — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
3. call `FUN_005d4430`. (corrected 2026-05-29: previously absent; binary shows this call between the two `FUN_00637550` calls — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
4. call `FUN_00647260`.
5. call `FUN_00637550` (second call).
6. call `Network_ServiceLoop`.
7. if any of `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48` are nonzero, branch to return without frame increment or drain.
8. increment `g_CurrentFrameCounter`.
9. optional call `FUN_00684290` if `0x00B07784` is nonzero and below the current frame.
10. call `FUN_0055E160`.
11. `0x0055DE9F`: call `FUN_00725C70` pending-delete drain.
12. call `FUN_00637270`, then clear `0x00ABCD58` and return.

Therefore, objects queued by an `UnInit` reached during the live-vector pass are not destructed inside that pass. The pointer can be removed from live membership by `Conceal`/`ObjectClass::Destructor`, but heap destruction/free is deferred until the late `Main_Tick` drain if the frame gate allows it.

### Same-tick vs later

For ordinary active ticks that pass the late frame gate, deletion is same `Main_Tick`, but late-phase: after the live vector and after network/service calls shown above. If the late gate branches to `0x0055DEC8`, the drain does not run on that `Main_Tick`, and queued pointers remain in the global pending-delete vector for the next drain call. Static evidence proves the gate and skip; this report does not runtime-classify all four flag globals.

## 6. Current Rust Implementation Status

Current Rust does not model the native pending-delete queue as a separate late-tick destructor phase:

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/combat/mod.rs:804` `handle_entity_deaths` | collects death effects, clears targets, sets `dying=true` for animated entities or immediately removes non-animated entities from `EntityStore` | no `UnInit -> pending-delete -> late drain` split |
| `src/sim/combat/mod.rs:975` | calls `clear_targets_on_dead_entity` before Rust death marking/despawn | partially resembles notification, but not native pending-delete/destructor ordering |
| `src/sim/combat/mod.rs:985` | animated entities get `dying=true`, clear attack/movement/selection, and remain stored until animation completes | native objects are concealed, alive-cleared, queued, then normally destructed late in the same `Main_Tick`; death animation objects are separate lifecycle objects where applicable |
| `src/sim/combat/mod.rs:1000` | structures/vehicles are immediately removed from occupancy and `EntityStore` | native does not call scalar destructor inline in `UnInit`; free is late drain |
| `src/app_sim_tick.rs:292` | app ticks death animations after `Simulation::advance_tick`, then calls `sim.despawn_entity` for finished IDs | not equivalent to native late pending-delete drain; physical removal can be many animation ticks later |
| `src/sim/world/mod.rs:675` `despawn_entity` | removes occupancy/radio/entity and unregisters live object in one call | no global queue, no `IsDead` gate, no all-duplicate queue removal, no scalar-destructor-like class cleanup staging |
| `src/sim/world/mod.rs:618` | live-object unregister is a direct retain on Rust IDs | native drain is separate from live-vector iteration and heap destruction |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::UnInit` queue append target | verified | `0x005F662C..0x005F667D`; decompile `0x005F65F0` | none |
| pending-delete queue storage/init | verified | `0x00725857..0x00725881`; data xref from `0x008152F8` | none for storage shape |
| pending-delete queue cleanup thunk | verified | `0x00725890..0x007258C9`; data xref from `0x00725852` | none for queue cleanup shape |
| drain helper caller inventory | verified | `get_function_xrefs(0x00725C70)` returning `Main_Tick`, `ScenarioClass::Full_Init`, `ReadMapOverlayPacks`, and `FUN_00534450` sites | none |
| ordinary active tick drain placement | verified | `Main_Tick` decompile (`decompile_function 0x0055DE00`); drain confirmed at `0x0055DE9F`; post-live-vector sequence corrected 2026-05-29 (FUN_00637550 → FUN_005d4430 → FUN_00647260 → FUN_00637550 → Network_ServiceLoop, then gate, frame++, FUN_0055E160, drain) | exact semantic names for four skip flags are not claimed |
| drain loop / removal compaction | verified | decompile `0x00725C70`; assembly `0x00725C71..0x00725D86` | none |
| `Object+0x90` dead gate | verified | `ObjectClass::IsDead @ 0x005F6690`; drain call `0x00725C8F`; `UnInit` write `0x005F6625` | none |
| `Release` before destructor | verified | drain `0x00725CE8`; `AbstractClass::Release @ 0x00410310` returns `1` | whether any concrete class overrides COM `Release` was not exhaustively vtable-censused; ordinary ObjectClass-derived docs show inherited slot |
| Building/Unit/Infantry/Aircraft alive-byte restore | verified | dynamic-cast descriptor constants `0x00818D60`, `0x00842D80`, `0x00825508`, `0x00817B90`; write `0x00725D67`; helper decompiles `0x006E78E0..0x006E7B50` identify types | none |
| scalar-deleting destructor/free path | verified | drain `0x00725D78..0x00725D7A`; examples `0x005F6DC0`, `0x00459F20`, `0x00746E80`, `0x0071D350`, `0x0074AB50` | exhaustive body internals of every leaf destructor out-of-scope |
| Rust death/despawn touchpoints | verified | source scan paths and line refs in Section 6 | exact future design pending implementation contract |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What owns/stores the pending-delete queue? -> Global dynamic-vector-shaped storage at `0x00B0F698..0x00B0F6AC`, data pointer `0x00B0F69C`, count `0x00B0F6A8`, grow amount `10`.` (evidence: `0x005F662C..0x005F667D`, `0x00725857..0x00725881`)
- `[RESOLVED] OQ-02 - Where does `ObjectClass::UnInit` append? -> It appends `this` to `*(0x00B0F69C + old_count*4)` and increments `0x00B0F6A8`.` (evidence: `0x005F6668..0x005F667D`)
- `[RESOLVED] OQ-03 - Does append suppress duplicates? -> No duplicate check appears in `UnInit`; duplicate removal is handled by the drain's repeated find/compact loop.` (evidence: `0x005F662C..0x005F667D`, `0x00725C9A..0x00725CDF`)
- `[RESOLVED] OQ-04 - What function drains the queue? -> `FUN_00725C70`.` (evidence: xrefs to `0x00B0F69C/0x00B0F6A8`, decompile `0x00725C70`)
- `[RESOLVED] OQ-05 - Who calls the drain? -> `Main_Tick`, `ScenarioClass::Full_Init`, `ReadMapOverlayPacks`, and global teardown `FUN_00534450`.` (evidence: `get_function_xrefs(0x00725C70)`)
- `[RESOLVED] OQ-06 - Where is the ordinary active-game drain relative to live object AI? -> After `LogicClassPerTickUpdateLiveVector @ 0x0055DC9E`, after network/service helpers, inside the late frame-gated block at `0x0055DE9F`.` (evidence: `Main_Tick` decompile and assembly context)
- `[RESOLVED] OQ-07 - Is deletion same tick? -> For ordinary ticks that pass the late frame gate, yes, but late in the same `Main_Tick`; if the gate skips, queued entries remain for a later drain call.` (evidence: `0x0055DE4F..0x0055DEA4`)
- `[RESOLVED] OQ-08 - What gate decides whether a queued object is destroyed? -> Virtual `+0x44`; for ObjectClass this is `IsDead`, returning `Object+0x90 == 0`.` (evidence: `0x00725C8F`, `0x005F6690`)
- `[RESOLVED] OQ-09 - What destructor/free path runs? -> Drain calls COM `Release`; when it returns nonzero, it calls virtual `+0x20` with flag `1`, causing scalar destructor then `operator delete` in concrete wrappers.` (evidence: `0x00725CE8`, `0x00725D78..0x00725D7A`, `0x005F6DC0`, `0x00459F20`)
- `[RESOLVED] OQ-10 - Which classes get `Object+0x90` restored before destructor? -> BuildingClass, UnitClass, InfantryClass, AircraftClass dynamic-cast successes write `+0x90=1`.` (evidence: `0x00725CF3..0x00725D67`; RTTI helper xrefs/decompiles)
- `[RESOLVED] OQ-11 - Does `ObjectClass::Destructor` also touch the pending-delete queue? -> Yes; it find/compacts the same vector, idempotent after normal drain removal.` (evidence: `0x005F3BC7..0x005F3BEA`)
- `[RESOLVED] OQ-12 - Does Rust currently have an equivalent late pending-delete phase? -> No direct equivalent found; death/despawn paths either mark `dying=true`, remove immediately, or despawn after app animation completion.` (evidence: `src/sim/combat/mod.rs:804`, `src/app_sim_tick.rs:292`, `src/sim/world/mod.rs:675`)
- `[DEFERRED] OQ-13 - What are exact semantic names of the four late `Main_Tick` skip flags?` (category: requires-different-system-context; reason: the branch and its effect on drain timing are proven, but naming the globals requires a broader main-loop/state investigation; next-step-if-pursued: trace writers of `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48`)
- `[DEFERRED] OQ-14 - What are all leaf destructor body side effects for every possible ObjectClass-derived type?` (category: bounded-cost-too-high; reason: this report proves virtual dispatch/free timing and representative scalar wrappers; exhaustive leaf body inventory is a separate class-family lifecycle swarm; next-step-if-pursued: per-class destructor census from vtable `+0x20` data xrefs)

Deferred items are outside the claimed slice. They do not affect the proved queue owner, active drain point, or destructor/free dispatch mechanism.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `UnInit` queues object pointers and returns; scalar destructor/free is not inline. | `0x005F6625..0x005F667D` | mismatch: immediate removal or long `dying` animation residence replaces queue semantics | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs` | separate native-order uninit/conceal/dead-queue state from physical entity free/removal | kill a vehicle during its AI visit; it is concealed/unregistered as native, but heap/free-equivalent cleanup happens only in the late pending-delete phase | Do not call `despawn_entity` as the first or inline death operation for all object classes |
| The active pending-delete drain runs after the main live-vector tick, not during it. | `0x0055DC9E`, `0x0055DE9F`; `FUN_00725C70` xrefs | mismatch/unchecked: Rust removes entities inside combat/app phases without a late native drain phase | `Simulation::advance_tick`, future scheduler/lifecycle phase | add a deterministic late drain phase after native-equivalent live-object processing, respecting frame-gate semantics if modeled | object A destroys object B during live iteration; B is not physically freed until after the live-vector phase has returned | Do not mutate active iteration order by removing/freeing mid-pass unless native does so through live-vector removal only |
| Drain deletes only entries whose virtual `+0x44` says dead; otherwise entries remain queued. | `0x00725C8F..0x00725D80`, `0x005F6690` | missing: Rust has no pending queue with a dead gate | future lifecycle queue in `sim/world` or equivalent | queued object with restored/non-dead state survives the drain and remains queued for later reconsideration | Do not model pending-delete as an unconditional vector drain |
| Drain removes all duplicate queue entries for the same pointer before destructor. | `0x00725C9A..0x00725CDF` | missing | future lifecycle queue | duplicate UnInit/queue append of the same entity causes one destructor/free-equivalent action, not two | Do not use a naive Vec drain that can double-free/logically double-remove |
| Building/Unit/Infantry/Aircraft objects have `Object+0x90` restored to one immediately before scalar destructor. | `0x00725CF3..0x00725D67`; type descriptors `0x00818D60`, `0x00842D80`, `0x00825508`, `0x00817B90` | missing: Rust uses `dying`/health-dead state through physical cleanup paths instead of a native pre-destructor alive-byte restore | `GameEntity` lifecycle/destructor-equivalent cleanup | class-specific cleanup that queries alive state during finalization sees the native pre-destructor restored state for techno-family classes | Do not assume `dying=true`/health zero is visible throughout final cleanup for every class |
| Scalar-deleting destructor with flag `1` is the physical free path. | `0x00725D78..0x00725D7A`; wrappers `0x005F6DC0`, `0x00459F20`, `0x00746E80` | partially represented by `EntityStore::remove`, but no class-specific destructor ladder | `EntityStore`, class-specific cleanup systems, registries/live-order structures | run class-specific registry/list/attachment cleanup at the destructor-equivalent phase before removing storage | destroying an anim/object removes it from class registry and pending-delete idempotently before storage disappears | Do not collapse all class destructor effects into generic `entities.remove` |

### Stale Docs / Follow-up Docs

- `docs/research/OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`: replace the deferred note "pending-delete drain processor out of scope" with: "Follow-up `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md` verifies drain `FUN_00725C70`, active `Main_Tick` call at `0x0055DE9F` after `LogicClassPerTickUpdateLiveVector`, all static drain callers, duplicate queue compaction, `Release` then scalar-deleting destructor dispatch, and the Building/Unit/Infantry/Aircraft `Object+0x90=1` pre-destructor restore."
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`: add: "Objects queued by `ObjectClass::UnInit` normally remain heap-live until the late pending-delete drain in `Main_Tick` after the live object-vector tick. Physical free is not inline with conceal/limbo."
- `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`: replace "pending-delete cleanup timing not exhausted" with a reference to this report's `Main_Tick @ 0x0055DE9F` drain and `FUN_00725C70` delete sequence.

## Sources

- Ghidra decompiled/read-only: `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass::IsDead @ 0x005F6690`, pending-delete drain `FUN_00725C70`, `Main_Tick @ 0x0055DE00`, `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, `ObjectClass::Destructor @ 0x005F3B80`, `ObjectClass` scalar destructor `0x005F6DC0`, `AnimClass::Destructor @ 0x00422900`, `BuildingClass::ScalarDeletingDestructor @ 0x00459F20`, `UnitClass::ScalarDelDestructor @ 0x00746E80`, `TerrainClass` scalar destructor `0x0071D350`, `VoxelAnimClass` scalar destructor `0x0074AB50`.
- Ghidra xrefs/read-only: `get_function_xrefs(0x00725C70)`, `get_xrefs_to(0x00B0F69C)`, `get_xrefs_to(0x00B0F6A8)`, data xrefs to type descriptors `0x00818D60`, `0x00842D80`, `0x00825508`, `0x00817B90`.
- Ghidra assembly contexts/read-only: `0x005F662C..0x005F667D`, `0x00725C71..0x00725D86`, `0x0055DC9E`, `0x0055DE4F..0x0055DEA4`, `0x00725857..0x007258C9`.
- Prior docs referenced: `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, `OBJECTCLASS_GHIDRA_REPORT.md`, `ABSTRACTCLASS_GHIDRA_REPORT.md`, `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`.
- Rust files scanned: `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/sim/entity_store.rs`.
