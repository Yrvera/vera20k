# Object+0x98 Save/Load Final Byte Provenance - Reswarm 2026-05-28

**Address(es):** `FUN_0067D300`, `FUN_0067E440`, `FUN_0067E730`, `FUN_00551B20`, `FUN_00551B90`, `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`, `FUN_005F5E80`, `ObjectClass full ctor @ 0x005F3900`, `ObjectClass vtable-only ctor @ 0x005F3B50`, `FUN_006CF240`, `FUN_006CF350`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard savegame save/load wrapper, raw IPersist object persistence, `LogicClass` active-vector stream/load, post-load swizzle, immediate post-load refresh callees, and representative active-object class `Load` bodies that can alter `Object+0x98`.  
**Non-Scope:** replay startup/playback, full active-vector stream order beyond byte provenance, every object AI body, and runtime watchpoint frequency sampling.  
**Confidence:** High for standard wrapper/order, raw stream provenance, swizzle non-effect, and inspected class-specific final byte provenance; Medium for any uninspected Ghidra-misbounded class load body.  
**Active in YR:** Yes for standard save/load; Conditional per object class for class-specific byte reset/preserve behavior.

## 0. Working Notes

**Target question:** What is the final runtime provenance/value of `ObjectClass+0x98` after standard save/load for objects referenced by the `LogicClass` active vector?  
**Non-goals:** Do not redo full active-vector stream order; do not investigate replay except negative contrast; do not implement Rust or mutate Ghidra.  
**Evidence needed to mark COMPLETE:** decompile plus assembly/xref evidence for raw stream save/load, vector load, swizzle fixup, post-load refresh non-writers, and class `Load` bodies that preserve or overwrite `+0x98`.  
**Stop conditions:** Stop after the standard save/load path and representative class-specific post-raw-load constructors prove whether the byte is streamed, reconstructed, ignored, or class-specific; defer only runtime frequency sampling.

## 1. Overview

`Object+0x98` is raw-streamed by the IPersist `AbstractClass::Save/Load` body, not reconstructed by a hidden post-load active-vector registration pass. The standard load wrapper reloads the `LogicClass` active vector in saved stream order and later swizzles its pointer slots, but neither the vector load helper nor swizzle fixup writes object bytes or calls `FUN_0055BAA0`.

The final byte value is therefore class-specific. Object-derived load bodies that call only the vtable-only Object constructor after raw load preserve the raw-streamed `+0x98`; load bodies that call a full Foot/Building/Techno constructor chain after raw load overwrite `Object+0x98` to `0`.

## 2. Key Offsets / Fields

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `Object+0x98` | `LogicClass` active-vector membership guard used by add/remove helpers | `FUN_0055BAA0`; `FUN_0055BAE0`; full ctor write at `0x005F3900` decompile | Yes |
| `LogicClass+0x04/+0x10` | active-vector items/count | `FUN_00551B20`/`FUN_00551B90`; scheduler reports | Yes |
| `SwizzleManager +0x08/+0x14` | queued pointer slots | `FUN_006CF240`, `FUN_006CF350` | Yes |
| `SwizzleManager +0x20/+0x2C` | old-pointer to new-pointer map | `AbstractClass::Load -> FUN_006CF2C0`; `FUN_006CF350` | Yes |

## 3. Core Findings

### 3.1 Active vector is streamed and swizzled; the byte is not reconstructed

`FUN_0067D300` saves the `LogicClass` active vector with `FUN_00551B20`, and `FUN_0067E730` loads it with `FUN_00551B90`. `FUN_00551B90` reads a count, appends each saved pointer token to `LogicClass+0x04`, increments `LogicClass+0x10`, then registers every vector slot with `FUN_006CF240`.

`FUN_006CF240` queues a pointer-slot fixup and clears the slot to `0`. `FUN_006CF350` later sorts the old/new map and slot queue, then writes only `*(int *)slot_address = new_object_pointer`. It does not inspect class type, does not touch `Object+0x98`, and does not call `FUN_0055BAA0` or `FUN_0055BAE0`.

Active in YR: Yes. `FUN_0067E440` is the `"LOADING GAME: %s"` wrapper, calls `FUN_0067E730`, then `FUN_006CF230 -> FUN_006CF350`, then refresh helpers.

### 3.2 Raw IPersist body includes `Object+0x98`

`AbstractClass::Save @ 0x00410320` writes the saved `this` pointer, then writes a raw memory body of size returned by virtual slot `+0x30`. `AbstractClass::Load @ 0x00410380` reads the old `this`, registers old/new mapping through `FUN_006CF2C0`, then reads the raw class-sized body back into the new object.

`FUN_005F5E80` is the ObjectClass stream load body in the object-derived chain. It calls `AbstractClass::Load`, swizzle-registers pointer fields `+0x30/+0x34/+0x38/+0x18/+0x88`, initializes two `VocHandle`s, and clears `+0xA8`. It does not write `+0x98`.

Active in YR: Yes, through object IPersist stream load/save.

### 3.3 Full constructor reset versus vtable-only constructor preserve

The full `ObjectClass` constructor at `0x005F3900` writes `*(byte *)(this+0x98)=0` (`param_1[0x26]=0` in dword-typed decompile). Assembly context starts at `0x005F3900`; the decompile shows the explicit byte write. Active in YR: Yes for ordinary construction and for load bodies that call full derived constructors after raw load.

The vtable-only constructor at `0x005F3B50` calls `AbstractClass__Constructor_VtablesOnly`, installs ObjectClass vtables, and returns. It has no `+0x98` write. Assembly context `0x005F3B50..0x005F3B77` shows only vtable writes. Active in YR: Yes for several load bodies.

### 3.4 Class-specific final byte provenance

| Class/load body | Post-raw-load action | Final `Object+0x98` provenance/value | Evidence | Active in YR |
|---|---|---|---|---|
| `AnimClass::Load @ 0x00425280` | calls `FUN_005F5E80`, then `ObjectClass` vtable-only ctor `0x005F3B50` | raw-streamed byte preserved; saved active anim remains `1` unless saved byte was different | decompile; assembly `0x0042528C` raw load, `0x004252A6 CALL 0x005F3B50` | Yes for saved anim objects |
| `OverlayClass::Load @ 0x005FD8F0` | raw load, then vtable-only ctor | raw-streamed byte preserved | decompile; assembly `0x005FD8FC`, `0x005FD912 CALL 0x005F3B50` | Yes if overlays are persisted through this path |
| `TerrainClass` load helper `0x0071CDA0` | raw load, then vtable-only ctor | raw-streamed byte preserved | decompile; assembly `0x0071CE93 CALL 0x005F3B50` | Yes for terrain persistence |
| `VoxelAnimClass` load helper `0x0074A970` | raw load, then vtable-only ctor | raw-streamed byte preserved | decompile; assembly `0x0074A992 CALL 0x005F3B50` | Yes for saved voxel anims |
| `BuildingLightClass::Load @ 0x00436950` | raw load, then vtable-only ctor | raw-streamed byte preserved | decompile; assembly `0x00436972 CALL 0x005F3B50` | Conditional on spotlight objects |
| `AircraftClass::Load @ 0x0041B430` | calls `FootClass::Load`, then `FootClass::Constructor @ 0x004D3540` | overwritten to `0` by full chain `Foot -> Techno -> Radio -> Mission -> Object full ctor` | decompile; assembly `0x0041B4BE CALL 0x004DB3C0`, `0x0041B4DC CALL 0x004D3540`; `MissionClass` calls full `0x005F3900` | Yes for saved aircraft |
| Unit load-like body `0x00744470` (Ghidra label stale as `Draw_It`) | calls `FootClass::Load`, then `FootClass::Constructor` | overwritten to `0` by full constructor chain | decompile; data xref `0x007F5C84`; assembly `0x007444FE CALL 0x004DB3C0`, `0x0074451C CALL 0x004D3540` | Yes for saved units |
| `BuildingClass::Load @ 0x00453E20` | calls `FUN_0070BF50`, then `BuildingClass::Constructor @ 0x0043B680` | overwritten to `0` by full chain `Building -> Techno -> Radio -> Mission -> Object full ctor` | decompile; assembly `0x00453ECF CALL 0x0043B680`; `MissionClass` calls full `0x005F3900` | Yes for saved buildings |

Material consequence: after save/load, active-vector pointer membership and the object-local duplicate/removal byte are not guaranteed to be globally equivalent. Raw-preserve load bodies keep the saved byte; full-constructor load bodies clear it after raw load. There is no later standard wrapper pass that reconciles this mismatch.

## 4. Immediate Post-Load Refresh Checks

`FUN_0067E440` calls `FUN_00685120`, `FUN_006D03A0`, `FUN_006D04F0(1)`, `SidebarClass::InitSurface`, `TiberiumClass::InitGrowthQueues_All`, `TiberiumClass::InitSpreadQueues_All`, `RadarClass::RefreshRadar`, `FUN_006842F0`, and `FUN_0072DEF0` after swizzle. Prior post-load report decompiled these and found no call to `FUN_0055BAA0`; the refreshed direct caller list for `FUN_0055BAA0` remains ordinary reveal/direct lifecycle callers, not the save/load wrapper or refresh helpers.

Active in YR: Yes for the refresh path; No for `Object+0x98` reconstruction in that path.

## 5. Current Rust Status

Rust serializes `Simulation` through bincode. `Simulation::live_object_order` is serde-persisted, but there is no native-equivalent object-local `Object+0x98` byte. `register_live_object` deduplicates by scanning the vector; `unregister_live_object` retain-removes by ID; `live_object_order_snapshot` appends missing `EntityStore` IDs sorted after the saved order. `Simulation::rebuild_caches_after_load` rebuilds skipped caches but is not a native raw-load constructor/swizzle/volatile-reset model.

## 6. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| Standard save wrapper | verified | prior `FUN_0067D300`; `FUN_00551B20` | none |
| Standard load wrapper | verified | `FUN_0067E440`, `FUN_0067E730` decompile/callees | none |
| Active-vector load | verified | `FUN_00551B90` decompile | none |
| Swizzle slot fixup | verified | `FUN_006CF240`, `FUN_006CF350` decompile | none |
| Raw object stream body | verified | `AbstractClass::Save/Load`, `FUN_005F5E80` | none |
| Full Object ctor reset | verified | `ObjectClass ctor @ 0x005F3900` | none |
| Vtable-only Object ctor preserve | verified | `ObjectClass ctor @ 0x005F3B50`; load call xrefs | none |
| Class-specific final byte table | verified for listed classes | decompile plus assembly call ranges | unlisted/misbounded class bodies only if needed later |
| Replay contrast | not-touched by design | parent negative contrast | non-goal |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-O98-001 - Is `+0x98` streamed? -> Yes, through `AbstractClass::Save/Load` raw class-sized body.` (evidence: `0x00410320`, `0x00410380`)
- `[RESOLVED] OQ-O98-002 - Is the active vector streamed? -> Yes, `FUN_00551B20`/`FUN_00551B90` save/load `LogicClass` slots.` (evidence: `0x00551B20`, `0x00551B90`)
- `[RESOLVED] OQ-O98-003 - Does vector load set object bytes? -> No, it appends pointer tokens and queues slot swizzles only.` (evidence: `0x00551B90`, `0x006CF240`)
- `[RESOLVED] OQ-O98-004 - Does swizzle set `+0x98`? -> No, it writes resolved pointers into queued slots only.` (evidence: `0x006CF350`)
- `[RESOLVED] OQ-O98-005 - Is there a post-load `FUN_0055BAA0` pass? -> No in the standard wrapper/refresh callee set.` (evidence: `FUN_0067E440` callees; `FUN_0055BAA0` caller list)
- `[RESOLVED] OQ-O98-006 - Do all class load constructors wipe the byte? -> No; vtable-only constructor `0x005F3B50` preserves it, full constructor `0x005F3900` clears it.` (evidence: class load table)
- `[RESOLVED] OQ-O98-007 - Is final value class-specific? -> Yes; raw-preserve classes keep saved byte, full-constructor load classes clear it to `0`.` (evidence: class load table)
- `[DEFERRED] OQ-O98-008 - How often do stock saved active-vector members land in the reset-to-zero class bucket?` (category: `needs-runtime-debugger`; reason: static mechanism is resolved, but frequency/incidence needs savegame watchpoints or dumps; next-step-if-pursued: watch `+0x98` for one saved anim, unit, aircraft, building, and spotlight through `FUN_0067E440`.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|---|
| Native save/load persists the active vector separately and swizzles pointer slots; it does not rebuild from sorted object storage. | `FUN_00551B20`, `FUN_00551B90`, `FUN_006CF350`; Active in YR: Yes | `live_object_order` persists, but snapshot helper appends sorted missing entities | `src/sim/world/mod.rs::live_object_order_snapshot`, `src/sim/snapshot.rs` | Treat active order as first-class saved state; missing entries require explicit policy, not sorted repair parity. | Save with live order different from stable ID order; load; first live-order consumer sees exact saved order. | `save_load_live_order_preserves_saved_vector_without_sorted_repair` | High |
| `Object+0x98` final value after load is class-specific: raw-preserve load bodies keep saved byte; full-constructor load bodies clear to `0`. | class load table; Active in YR: Conditional per class | Rust has no object-local membership byte or class-specific volatile reset after snapshot load | future active membership state; `Simulation::rebuild_caches_after_load` | Separate vector membership from object-local duplicate/removal guard; if modeled, reset/preserve by native class-load provenance. | Save/load one raw-preserve object and one full-constructor object, then unregister each with native byte-gate semantics. | `save_load_membership_byte_is_class_specific_not_vector_derived` | High |
| Standard post-load swizzle/refresh does not call normal registration for saved active vector members. | `FUN_0067E440`, `FUN_006CF350`, `FUN_0055BAA0` caller list; Active in YR: Yes negative | `register_live_object` could be misused as a load repair | load/rebuild logic, active registration API | Do not call register on every loaded active entry as a repair; that would set byte semantics incorrectly. | Load a saved vector with a reset-byte class; no generic post-load pass flips its byte to active. | `load_does_not_reregister_saved_active_vector_members` | High |

## 9. Negative Facts / Do Not Do

- Do not say `ObjectClass__Save @ 0x005F6250` decides savegame persistence of `+0x98`; it is the CRC/checksum-style surface, while IPersist raw save is `AbstractClass::Save`. Active in YR: Yes.
- Do not rebuild active order by walking `EntityStore` or object arrays sorted by ID after load. Active in YR: Yes.
- Do not run a generic post-load `FUN_0055BAA0` equivalent for every saved active-vector member. Active in YR: No for standard savegame post-load.
- Do not collapse active-vector slot membership and `Object+0x98` after load; class-specific load bodies can preserve or clear the byte independently of vector slots. Active in YR: Yes.
- Do not assume every `ObjectClass__Constructor` label means full field initialization; `0x005F3B50` is vtable-only and preserves `+0x98`. Active in YR: Yes.

## 10. Remaining Uncertainty

None for the standard static provenance mechanism. Runtime debugger sampling would still be useful only to measure stock incidence of reset-to-zero classes inside the loaded active vector, not to identify the standard writer/reconstructor.

## 11. Stale Docs / Replacement Wording

- `docs/research/POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`: replace "Load-specific constructors can wipe raw-loaded `+0x98`" with "Final `Object+0x98` after raw load is class-specific: load bodies that call vtable-only `ObjectClass` constructor `0x005F3B50` preserve the raw-streamed byte, while load bodies that call full Foot/Building/Techno constructor chains reach full `ObjectClass` constructor `0x005F3900` and clear the byte to `0`."
- `docs/research/ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`: replace "Runtime final `Object+0x98` after save/load remains a separate watchpoint/debugger question" with "The standard static provenance is class-specific: raw-preserve Object-derived load bodies keep the saved byte; full-constructor load bodies clear it. Runtime watchpoints are only needed to sample stock incidence/frequency."
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`: replace "The native post-load active-vector rebuild owner remains a required follow-up" with "Save/load persists the active vector directly; the object-local membership byte is raw-streamed but may be preserved or reset by class-specific load bodies, with no generic post-load re-registration pass."

## Sources

- Ghidra read-only decompile/assembly: `FUN_0067D300`, `FUN_0067E440`, `FUN_0067E730`, `FUN_00551B20`, `FUN_00551B90`, `FUN_006CF240`, `FUN_006CF350`, `AbstractClass__Save @ 0x00410320`, `AbstractClass__Load @ 0x00410380`, `FUN_005F5E80`, `ObjectClass__Constructor @ 0x005F3900`, `ObjectClass__Constructor @ 0x005F3B50`, `AnimClass__Load @ 0x00425280`, `OverlayClass__Load @ 0x005FD8F0`, `TerrainClass load @ 0x0071CDA0`, `VoxelAnimClass load @ 0x0074A970`, `BuildingLightClass__Load @ 0x00436950`, `AircraftClass__Load @ 0x0041B430`, Unit load-like body `0x00744470`, `BuildingClass__Load @ 0x00453E20`, `FootClass__Constructor @ 0x004D3540`, `BuildingClass__Constructor @ 0x0043B680`, `MissionClass__Constructor`.
- Prior docs referenced: `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`, `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/snapshot.rs`, `src/app_input.rs`.

Status: COMPLETE.
