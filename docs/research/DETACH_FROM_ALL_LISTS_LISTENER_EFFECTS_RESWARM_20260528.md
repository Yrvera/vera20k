# Detach_From_All_Lists Listener Effects - Re-swarm Research Report

**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass` pointer-expiry base `0x005F5230`, `TechnoClass::PointerExpired @ 0x007077C0`, `FootClass::PointerExpired @ 0x004D9960`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Rust-relevant, non-bullet pointer-expired/list invalidation effects reached by `ObjectClass::UnInit -> Detach_From_All_Lists` before virtual Conceal and before `Object+0x90` alive-clear. This ranks top active listeners/helpers and records the rest as uncertainty.  
**Non-Scope:** BulletClass target invalidation body, full derived destructor side effects, pending-delete drain/destructor timing, full radio BREAK/limbo broadcast internals, and every listener body in every RTTI-specific listener vector.  
**Confidence:** High for ordering, object-registered branch, named helper effects, and foot/techno/aircraft/building listener bodies cited below; Medium for semantic labels of some unlabeled object fields.  
**Active in YR:** Yes. `ObjectClass::UnInit` calls `Detach_From_All_Lists` on normal object removal paths; object-derived vtables use the pointer-expiry slot `+0x28`, and the object-registered branch is not behind a TS-only gate.

## Investigation Gate

Target question: Which non-bullet listener/list effects reached by `Detach_From_All_Lists @ 0x007258D0` during `ObjectClass::UnInit` matter to Rust death/despawn ordering before Conceal/alive-clear?

Non-goals: Do not redo BulletClass retargeting, pending-delete drain, or destructor side effects. Do not expand into every listener body if the census fans out; rank active Rust-relevant effects and state uncertainty.

Evidence needed to mark COMPLETE: decompile plus disassembly range for `ObjectClass::UnInit` and `Detach_From_All_Lists`; decompile plus xref/caller/vtable evidence for top listener bodies; prior-doc cross-check for disk laser, radio, aircraft cached dock, and mind control; current Rust surface scan.

Stop conditions: Stop after the object-registered branch and high-value class listener bodies (`Object`, `Techno`, `Foot`, `Unit/Infantry/Aircraft/Building`, `Anim`, `DiskLaser`, `SpawnManager`, tactical/current globals). Record remaining RTTI-specific listener vectors as follow-up uncertainty.

## Overview

`ObjectClass::UnInit` calls `Detach_From_All_Lists` before virtual Conceal and before clearing `Object+0x90`. The scoped branch for object-registered removals first clears global current/UI-mode pointers, then calls every generic object remove-listener's virtual `+0x28`, then runs several global cleanup helpers: vector/backref removal, spawn-retreat removal, reverse disk-laser detach, tactical pointer-expiry callback, and type/list cleanup.

This is not equivalent to Rust deleting an entity and letting later systems discover missing IDs. Native listeners see the expiring object while it is still unconcealed and still alive according to `Object+0x90`.

## Core Binary Findings

### `ObjectClass::UnInit` Entry Order

Active in YR: Yes. Decompile of `0x005F65F0` plus disassembly range `0x005F65F0..0x005F668F` confirms the settled parent order: bomb defuse, passenger/EMP hook, `Detach_From_All_Lists`, virtual `+0xD4` Conceal/Limbo, `Object+0x90 = 0`, pending-delete append. This report uses only the `Detach_From_All_Lists` slice and does not reopen BulletClass or pending-delete drain.

### `Detach_From_All_Lists` Object Branch

Active in YR: Yes. Decompile of `0x007258D0` plus disassembly ranges `0x007258D0..0x00725A0F` and `0x00725970..0x00725A6F` confirm:

1. Calls target virtual `+0x2C` to get RTTI/WhatAmI.
2. If `DAT_0088098C == target`, clears that current/placement pointer.
3. If `g_UIModeLock == target`, clears `g_UIModeLock` and calls the UI image/placement refresh helper `0x004A8BF0`.
4. For object-registered targets (`AbstractFlags +0x14 bit 1`), iterates `DAT_00B0F724`/`DAT_00B0F730` forward and calls each listener vtable `+0x28(target, 1)`.
5. Calls `FUN_00439150`, `SpawnRetreat__Remove`, reverse-loops `g_DiskLaserClass_Array_Count` and calls `DiskLaserClass::DetachFromObject`, calls no-op `0x00413490` for RTTI `0x0F/1/2`, calls `FUN_00733160`, calls `g_Tactical->vtable+0x28(target, 1)` if tactical exists, then calls `FUN_0055B880`.

The object-registered branch is the `ObjectClass::UnInit` material branch for Unit/Infantry/Aircraft/Building/Bullet/Anim-style objects. Other RTTI-specific listener vectors exist, but they are type/team/tag/trigger/house/factory-oriented and are not claimed as fully censused here.

### Current / UI / Tactical Pointers

Active in YR: Yes.

- `DAT_0088098C` is cleared if it equals the expiring object. Prior docs associate this with currently placed/selected production object state (`HOUSECLASS_GHIDRA_REPORT.md`, `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`).
- `g_UIModeLock @ 0x00880990` is cleared if it equals the expiring object, then `0x004A8BF0` clears/refreshes placement image state. Evidence: `Detach_From_All_Lists @ 0x007258D0`, `FUN_004A8BF0` decompile, callers of `0x004A8BF0`.
- `g_Tactical @ 0x00887324` receives virtual `+0x28(target, 1)` after disk-laser/spawn-retreat cleanup and before `FUN_0055B880`. The vtable slot read at `vtable__Tactical +0x28` points to `0x006DA560`; Ghidra lacks a function boundary there, so the exact body is a Remaining Uncertainty, but the call ordering and target are verified.

### Generic Object Listener Ownership / Ordering

Active in YR: Yes. The generic vector `DAT_00B0F724`/`DAT_00B0F730` is an object/remove-listener observer vector, not a per-frame update queue. Evidence: `Detach_From_All_Lists` iterates it only when the expiring target has `AbstractFlags+0x14 bit 1` and calls listener `+0x28`; `HOUSECLASS_CONSTRUCTOR_DETAILED.md` shows constructors append `this` into several hierarchy registration/listener vectors including `DAT_00B0F724`; `LABEL_AUDIT_LOG.md` records the same correction.

Ordering is forward iteration from index `0` to `count-1`, then helper cleanup. The iteration uses the current count in the loop condition; this report did not prove mutation safety if a listener changes the same vector during the callback.

### Base `ObjectClass` Pointer-Expiry Body

Active in YR: Yes for object-derived listeners that inherit or chain to it. Decompile of `FUN_005F5230` plus disassembly `0x005F5230..0x005F52AF` shows:

- If expired pointer equals `Object+0x34`, decrement the expired object's `+0x2C` counter and clear `Object+0x34`.
- If removal flag is nonzero and expired pointer equals `Object+0x30`, replace `Object+0x30` with the expired object's `+0x30` chain pointer.
- If expired pointer equals `Object+0x88`, clear `Object+0x88`.

Exact semantic labels for `+0x30/+0x34/+0x88` are not proven in this report. The Rust implication is that native pointer expiry may update chain pointers and counters, not only set target fields to null.

### `TechnoClass::PointerExpired @ 0x007077C0`

Active in YR: Yes. Vtable read `vtable__TechnoClass +0x28 -> 0x007077C0`; Unit/Infantry/Aircraft/Building listener bodies chain through it. Decompile plus disassembly `0x007077C0..0x007078BF` and caller evidence from `FootClass::PointerExpired` and `BuildingClass` confirm the following Rust-relevant effects:

- Calls radio/contact cleanup (`0x0065AAC0`), which chains to Object pointer expiry and clears matching radio contact slots when removal flag is nonzero.
- When removal flag is nonzero, calls cargo/list helper `0x004734B0`, which removes the expired object from a linked list with `next` at `+0x30` and decrements a count.
- Clears many direct Techno pointer fields when they equal the expiring object, including target/destination-like, owner/house-like, and helper object pointers.
- If a capture manager exists at Techno `+0x2BC` and removal flag is nonzero, calls `0x00471F90`; that reverse-scans MC nodes and deletes the node whose victim pointer equals the expired object. This removes a destroyed victim from the controller's manager; controller death release remains `FootClass::UnInit -> CaptureManagerClass::FreeAll`, not this callback.
- If a spawn manager exists, calls `SpawnManagerClass::PointerExpired`.
- Calls temporal helper `0x0071AB60` when temporal state exists; if source/target pointers match, it clears temporal target state and may call `TemporalClass::DetachFromTarget` or command the owner through virtual `+0x484`.
- Clears deploy/chrono relation pointers around `+0x2AC/+0x2B0` by calling `BuildingClass::DeployUnit_ChronoWarp(1)` before nulling reciprocal fields when the expired object matches.
- Removes matching objects from two dynamic vectors near the end of the body.

The body contains sensor/house gating and some mission-specific branches. Their exact field names were not exhaustively assigned, but the pointer-clearing/removal side effects are verified.

### `FootClass::PointerExpired @ 0x004D9960`

Active in YR: Yes for foot-family listeners. Vtable read `vtable__FootClass +0x28 -> 0x004D9960`; Unit/Infantry/Aircraft override bodies chain through it. Decompile plus disassembly `0x004D9960..0x004D9ADF` show:

- Chains to `TechnoClass::PointerExpired`.
- Clears transport/passenger pointer `+0x5D4` if it equals the expired object.
- Clears or chains multiple movement/nav/attach fields (`+0x694`, `+0x598/+0x5A4`, `+0x5CC/+0x5D0`) when they reference the expired object.
- If a nested helper/listener pointer at `+0x694` exists and has count at `+0x6C > 0`, it calls that helper's virtual `+0x28(expired, 1)`.
- Replaces a target-object pointer at `+0x5CC` with the expired object's cell when the expired pointer matched, then clears the object pointer. This is analogous to "object target becomes cell target" behavior outside bullets.
- Removes matching expired pointers from two dynamic vectors.

This body is a major Rust gap because it handles non-bullet target/destination/transport invalidation before the dying object is concealed or marked not alive.

### Unit / Infantry / Aircraft / Building Overrides

Active in YR: Yes for their stock classes.

- `UnitClass` vtable `+0x28 -> 0x007446E0`: chains to `FootClass::PointerExpired`, then clears fields `+0x6C8` and `+0x6C4` if they equal the expired object. Evidence: vtable read `0x007F5C98`, decompile `0x007446E0`.
- `InfantryClass` vtable `+0x28 -> 0x0051AA10`: chains to `FootClass::PointerExpired`, then clears field `+0x6C0` if it equals the expired object. Evidence: vtable read `0x007EB080`, decompile `0x0051AA10`.
- `AircraftClass` vtable `+0x28 -> 0x0041B660`: chains to `FootClass::PointerExpired`, clears `Aircraft+0x6CC CachedDock` and `+0x6C4` if they equal the expired object. Evidence: vtable read `0x007E22CC`, decompile `0x0041B660`; `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md` also cites decompile plus disassembly `0x0041B66E..0x0041B67F`. Active for stock ORCA/BEAG airfield docking.
- `BuildingClass` vtable `+0x28 -> 0x0044E8F0`: chains to `TechnoClass::PointerExpired`, clears several building fields (`+0x540`, `+0x548`, Factory, `+0x600`, LightSource), may recreate building anim slots when certain anim pointers expire, clears matching anim-slot pointers, Type pointer, Upgrades pointers, and removes the expired object from two dynamic vectors, one of them map-editor gated. Evidence: vtable read `0x007E3EE4`, decompile `0x0044E8F0`.

### Anim Pointer-Expiry Body

Active in YR: Yes for AnimClass objects. Vtable read `vtable__AnimClass +0x28 -> 0x00425150`; decompile shows:

- Chains to base Object pointer expiry.
- If owner pointer `Anim+0xCC` equals expired object and non-null, removes anim from display layer, calls expired owner's virtual `+0x60`, clears owner, sets byte `Anim+0x19B = 1`, and calls anim virtual `+0x124(0)`.
- Clears type/object pointer at `+0xC8`.
- If pointer fields `+0x17C` or `+0x180` equal expired object, clears `+0x17C` and calls anim virtual `+0xF8` to uninit the anim.

This is separate from full Anim destructor side effects; it is a pre-conceal listener effect caused by another object expiring.

### Disk Laser, Spawn Retreat, Spawn Manager

Active in YR: Yes.

- Disk laser: `Detach_From_All_Lists` reverse-loops `g_DiskLaserClass_Array_Count` and calls `DiskLaserClass::DetachFromObject @ 0x004A7900`. If the expiring object equals disk laser source `+0x24` or target `+0x28`, the function writes state `+0x30 = -1` and appends the disk laser object to the same pending-delete vector globals `0x00B0F69C/0x00B0F6A8`. Evidence: decompile `0x004A7900`; `DISK_LASER_CLASS_GHIDRA_REPORT.md` confirms active Floating Disc `DiskLaser=yes` usage and constructor ownership in `g_DiskLaserClass_Array`.
- Spawn retreat: `SpawnManagerClass::ClearAllTargets @ 0x006B7BB0` and `SpawnManagerClass::AI @ 0x006B7230` call `SpawnRetreat__Push` for spawned units returning/retreating; `Detach_From_All_Lists` calls `SpawnRetreat__Remove` for the expiring object before disk-laser detach. The exact `SpawnRetreat__Remove` body lacks a public symbol/function boundary in this report, but the pre-conceal call placement is verified.
- Spawn manager: `TechnoClass::PointerExpired` calls `SpawnManagerClass::PointerExpired @ 0x006B7C60` when manager pointer exists. That body clears current/queued target pointers at `+0x68/+0x6C`, marks spawn-control entries state `7` with current frame/timer fields when a spawned child expires, and if the owner pointer `+0x24` expires, kills all spawns and clears targets. Evidence: decompile and disassembly `0x006B7C60..0x006B7D5F`.

## Current Rust Implementation Status

| Surface | Current shape | Delta |
|---|---|---|
| `src/sim/combat/mod.rs` | `handle_entity_deaths` clears targets around death processing and uses `dying`/immediate removal paths. | Missing central native-order pre-conceal listener pass. |
| `src/sim/world/mod.rs::despawn_entity` | Clears radio contacts and removes entity/occupancy/order state by stable ID. | Too late for native pointer-expiry semantics that run while target is still alive/unconcealed. |
| `src/app_sim_tick.rs` | Death animation completion calls occupancy removal then `despawn_entity`. | Native listener invalidation is not coupled to app-side animation completion. |
| `src/sim/docking/aircraft_dock.rs` | Models slots plus FIFO queues; prior report notes no native FIFO queue proof. | Needs cached-dock pointer-expiry invalidation separate from reservation queues. |
| `src/sim/passenger.rs`, `src/sim/production/production_sell.rs`, paradrop cargo code | Cargo/passengers modeled as Rust vectors/roles. | No generic `Techno/Foot PointerExpired` equivalent that removes expired passengers or transport refs before conceal. |
| `src/sim/game_entity.rs` mind-control fields | Has simple mind-control booleans/flags. | No verified CaptureManager node removal/release stage tied to pre-conceal pointer expiry. |
| disk laser surface | No direct native `DiskLaserClass` sim surface found in the scan. | Floating Disc disk-laser effect cannot match source/target expiry until a disk-laser object/list lifecycle exists. |

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::UnInit -> Detach -> Conceal -> alive-clear` placement | verified | `0x005F65F0`, disasm `0x005F65F0..0x005F668F` | none |
| `Detach_From_All_Lists` object-registered branch | verified | `0x007258D0`, disasm `0x007258D0..0x00725A6F` | mutation safety if listener edits vector |
| Current/UI globals | verified | `0x007258D0`, `0x004A8BF0`, prior placement docs | exact label for `DAT_0088098C` |
| Generic object listener vector ownership/order | verified | `0x007258D0`; `HOUSECLASS_CONSTRUCTOR_DETAILED.md`; `LABEL_AUDIT_LOG.md` | exact class inventory of every registered listener |
| Object base pointer expiry | verified | `0x005F5230`, disasm `0x005F5230..0x005F52AF` | semantic names for `+0x30/+0x34/+0x88` |
| Techno pointer expiry | verified | `0x007077C0`, vtable read `0x007F4988 -> 0x007077C0` | semantic names for all cleared fields |
| Foot pointer expiry | verified | `0x004D9960`, vtable read `0x007E8CBC -> 0x004D9960` | exact mission labels for conditional target-to-cell branch |
| Aircraft CachedDock expiry | verified | `0x0041B660`; prior aircraft dock report | none for cached-dock clear |
| Building pointer expiry | verified | `0x0044E8F0`; vtable read `0x007E3EE4` | exact anim slot semantics |
| Anim pointer expiry | verified | `0x00425150`; vtable read `0x007E337C` | exact labels for `+0x17C/+0x180` |
| DiskLaser detach | verified | `0x004A7900`; disk laser report | exact pending-delete allocator failure edge not rederived |
| SpawnManager pointer expiry | verified | `0x006B7C60`; callers from `TechnoClass::PointerExpired` | exact spawn-control state names |
| SpawnRetreat remove body | touched-not-exhausted | call in `0x007258D0`; `SpawnRetreat__Push` callers in spawn manager | body/function boundary not resolved |
| Tactical `+0x28` body | touched-not-exhausted | call in `0x007258D0`; vtable read `0x007F4370 -> 0x006DA560` | Ghidra has no function boundary at body |
| RTTI-specific type/team/tag/trigger vectors | deferred | `0x007258D0` switch | out of object UnInit Rust target scope |

## Open Questions - Final State

- `[RESOLVED] OQ-001 - Does `Detach_From_All_Lists` run before Conceal/alive-clear? -> Yes.` (evidence: `0x005F65F0..0x005F668F`; Active in YR: Yes)
- `[RESOLVED] OQ-002 - What is the material object branch order? -> current/UI clears, generic object listener loop, vector/backref cleanup, spawn-retreat remove, disk-laser detach, no-op RTTI hook for `0x0F/1/2`, object-vector cleanup, tactical callback, type/list cleanup.` (evidence: `0x007258D0`; Active in YR: Yes)
- `[RESOLVED] OQ-003 - Is generic listener iteration ordered? -> Forward from index 0 to count-1 before later helpers.` (evidence: `0x007258D0`; Active in YR: Yes)
- `[RESOLVED] OQ-004 - Does native clear current placement/UI pointers pre-conceal? -> Yes, `DAT_0088098C` and `g_UIModeLock` are nulled before listener dispatch finishes.` (evidence: `0x007258D0`, `0x004A8BF0`; Active in YR: Yes)
- `[RESOLVED] OQ-005 - Do non-bullet object listeners clear targets/docks/cargo? -> Yes; Techno/Foot/Aircraft/Building/Anim listener bodies clear many object pointers, radio contacts, cargo links, cached dock, and vectors.` (evidence: `0x007077C0`, `0x004D9960`, `0x0041B660`, `0x0044E8F0`, `0x00425150`; Active in YR: Yes)
- `[RESOLVED] OQ-006 - Is mind-control cleanup through radio BREAK? -> No; victim-expiry node cleanup is through `TechnoClass::PointerExpired -> 0x00471F90`, while controller death release is `FootClass::UnInit -> CaptureManagerClass::FreeAll`.` (evidence: `0x007077C0`, `0x00471F90`, `0x004DE5D0`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`; Active in YR: Yes)
- `[RESOLVED] OQ-007 - Does aircraft cached dock clear through pointer expiry? -> Yes, `Aircraft+0x6CC` is cleared if it equals the expired object.` (evidence: `0x0041B660`, `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`; Active in YR: Yes for stock ORCA/BEAG airfield reload/docking)
- `[RESOLVED] OQ-008 - Does disk laser source/target expiry queue laser cleanup? -> Yes; matching source/target sets state -1 and appends disk laser to pending-delete vector.` (evidence: `0x004A7900`, `DISK_LASER_CLASS_GHIDRA_REPORT.md`; Active in YR: Yes for Floating Disc disk laser)
- `[RESOLVED] OQ-009 - Does spawn manager react to expired spawned/owner/target pointers? -> Yes; clears targets, moves controls to state 7, or kills all spawns if owner expires.` (evidence: `0x006B7C60`; Active in YR: Yes for spawn-manager weapons/units)
- `[DEFERRED] OQ-010 - What exact code runs inside tactical vtable `+0x28` at `0x006DA560`?` (category: `bounded-cost-too-high`; reason: call target verified but Ghidra has no function boundary and body was not needed to prove pre-conceal listener ordering; next-step-if-pursued: inspect raw bytes/callers and define a read-only function boundary in a separate mutating-approved RE session)
- `[DEFERRED] OQ-011 - What exact body is `SpawnRetreat__Remove`?` (category: `bounded-cost-too-high`; reason: call placement is verified but body boundary/name is not; next-step-if-pursued: resolve through `SpawnRetreat__Push` global storage and caller dataflow)
- `[DEFERRED] OQ-012 - Which exact concrete objects occupy every `DAT_00B0F724` slot at runtime?` (category: `needs-runtime-debugger`; reason: static constructors prove ownership pattern, not a stock-scenario runtime roster; next-step-if-pursued: set read-only runtime watch/log on vector appends)
- `[DEFERRED] OQ-013 - What does every RTTI-specific listener vector body do?` (category: `out-of-scope`; reason: user requested non-bullet object-death effects and allowed ranking top Rust-relevant listeners; next-step-if-pursued: separate type/team/tag/trigger/house/factory listener census)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Object death runs pointer-expiry listeners before Conceal and alive-clear; listeners can clear radio contacts, cargo links, nav/target pointers, MC nodes, cached docks, and vectors while target is still alive/unconcealed. | `0x005F65F0`; `0x007258D0`; `0x007077C0`; `0x004D9960` | Missing central pre-conceal listener invalidation stage | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, target/nav/passenger/radio systems | Add an UnInit-like stage that dispatches dependent reference invalidation before occupancy/display removal and before `alive=false`/delete queue. | Destroy a transport/targeted unit with cargo, radio contact, and attacker targets; dependent refs clear in the same pre-conceal phase while the target is still queryable as alive. Proposed test: `uninit_pointer_expiry_runs_before_conceal_and_alive_clear_for_non_bullet_refs` | Do not replace this with `entities.get(id).is_none()` fallbacks after despawn. |
| Aircraft `CachedDock` is cleared by aircraft pointer-expiry when the dock building expires, independent of voluntary radio release. | `0x0041B660`; `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md` | Airfield docks use slot/queue model; cached-dock pointer-expiry is not a generic listener effect | `src/sim/docking/aircraft_dock.rs`, `src/sim/aircraft/*`, `src/sim/world/mod.rs` | Model cached dock separately from pad queues and clear it during pre-conceal expiry of the dock building. | Destroy an airfield while an aircraft has it cached; aircraft loses cached dock before airfield occupancy/display removal and next dock search must revalidate/search. Proposed test: `aircraft_cached_dock_clears_on_pre_conceal_airfield_expiry` | Do not treat AirfieldDocks FIFO cancellation as equivalent to `Aircraft+0x6CC = NULL`. |
| CaptureManager node cleanup for a destroyed victim is through `TechnoClass::PointerExpired -> 0x00471F90`; controller death release is `FootClass::UnInit -> FreeAll` before base `ObjectClass::UnInit`. | `0x007077C0`; `0x00471F90`; `0x004DE5D0`; mind-control report | Mind-control state is not represented as native CaptureManager nodes/release order | future mind-control/CaptureManager surface, `src/sim/game_entity.rs` | Separate victim-expiry node removal from controller-death victim release; both happen before base conceal/alive-clear in their respective paths. | Destroy an MC victim: controller manager drops that victim node without radio BREAK; destroy controller: victims release before controller base UnInit. Proposed tests: `mind_control_victim_expiry_removes_capture_node_pre_conceal`, `mind_control_controller_uninit_freeall_before_base_object_uninit` | Do not implement MC release as a RadioClass BREAK side effect. |
| DiskLaser source/target expiry is global-list cleanup, not bullet targeting; matching source/target sets disk laser state -1 and queues it for delete before target Conceal. | `0x004A7900`; `DISK_LASER_CLASS_GHIDRA_REPORT.md`; `0x007258D0` | No native DiskLaser object/list lifecycle found in Rust scan | future disk-laser/Floating Disc weapon effect surface, render/effects integration | Track disk laser effects as removable objects/list entries; source or target expiry must mark the effect for cleanup in the pre-conceal phase. | Floating Disc disk laser active when source or target dies; laser stops/queues cleanup immediately before the target is concealed. Proposed test: `disk_laser_source_or_target_expiry_marks_laser_delete_before_conceal` | Do not model disk laser as a fire-and-forget beam with no source/target expiry hook. |
| Foot pointer-expiry can convert an object target pointer to a cell pointer and clear movement/nav/cargo fields; Unit/Infantry/Aircraft subclasses add extra clears. | `0x004D9960`; `0x007446E0`; `0x0051AA10`; `0x0041B660` | Current Rust target/nav cleanup is scattered in combat, movement, passenger, and docking systems | `src/sim/movement/*`, `src/sim/passenger.rs`, `src/sim/docking/*`, combat target state | Listener invalidation should update target refs, cached cells, cargo/transport refs, and subclass refs in one native-order pass. | A foot unit with movement/destination/transport/cached-dock refs to an expiring object keeps any native cell fallback but clears object refs before occupancy removal. Proposed test: `foot_pointer_expiry_preserves_cell_fallback_and_clears_object_refs_pre_conceal` | Do not null every target blindly; some native paths replace object target with a cell. |

## Negative Facts / Do Not Do

- Do not redo BulletClass target invalidation as part of this report; the bullet handler `0x004684E0` is already covered by the parent/prior reports.
- Do not treat `Conceal` as the owner of reference invalidation. The listener pass is before virtual `+0xD4`.
- Do not clear `alive` or physically remove the Rust entity before non-bullet listeners run; native listeners see the expiring object before `Object+0x90 = 0`.
- Do not implement mind-control cleanup as radio BREAK. Victim node removal and controller release are CaptureManager paths.
- Do not collapse aircraft cached dock, radio contact slots, and Rust airfield FIFO queues into one concept; native cached-dock pointer expiry is a separate field clear.

## Remaining Uncertainty

- Tactical vtable `+0x28` target `0x006DA560` body was not decoded because Ghidra has no function boundary in the read-only session.
- `SpawnRetreat__Remove` body/storage was not decoded; only its pre-conceal call placement is verified.
- Exact semantic names for many `TechnoClass::PointerExpired` and `FootClass::PointerExpired` fields remain unresolved, though equality-clear/list-removal effects are verified.
- Runtime roster and mutation behavior of `DAT_00B0F724` listeners were not measured with a debugger.
- RTTI-specific listener vectors for type/team/tag/trigger/house/factory branches were not censused because they are outside the object-death Rust target.

## Stale Docs / Follow-up Docs

- `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`: replace any wording implying death cleanup can be modeled as direct limbo/despawn with: "For object death, `ObjectClass::UnInit` runs `Detach_From_All_Lists` before Conceal and before `Object+0x90` alive-clear. Non-bullet listeners can clear radio contacts, cargo/transport links, cached dock pointers, mind-control nodes, spawn/disk-laser links, and target/navigation fields while the expiring object is still alive/unconcealed. Rust death/despawn paths that only clean references after occupancy removal or entity deletion are DRIFT/UNCHECKED."
- Death/despawn gap reports should add: "The missing pre-conceal listener stage is broader than bullets. It includes `TechnoClass::PointerExpired`, `FootClass::PointerExpired`, `AircraftClass::Detach`, `BuildingClass` pointer expiry, `AnimClass::Detach`, disk-laser detach, spawn-manager expiry, current/UI pointer clears, and tactical callback ordering."

## Sources

- Ghidra read-only decompile/disassembly: `ObjectClass::UnInit @ 0x005F65F0`, `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass` pointer-expiry base `0x005F5230`, `TechnoClass::PointerExpired @ 0x007077C0`, `FootClass::PointerExpired @ 0x004D9960`, Unit `+0x28 @ 0x007446E0`, Infantry `+0x28 @ 0x0051AA10`, Aircraft `+0x28 @ 0x0041B660`, Building `+0x28 @ 0x0044E8F0`, Anim `+0x28 @ 0x00425150`, `DiskLaserClass::DetachFromObject @ 0x004A7900`, `SpawnManagerClass::PointerExpired @ 0x006B7C60`, `SpawnManagerClass::ClearAllTargets @ 0x006B7BB0`, `CaptureManager` node removal helper `0x00471F90`, cargo/list helper `0x004734B0`, radio pointer-expiry helper `0x0065AAC0`, temporal helper `0x0071AB60`, `FootClass::UnInit @ 0x004DE5D0`.
- Vtable/memory reads: `vtable__TechnoClass +0x28 -> 0x007077C0`; `vtable__FootClass +0x28 -> 0x004D9960`; Unit/Infantry/Aircraft/Building/Anim `+0x28` reads; `vtable__Tactical +0x28 -> 0x006DA560`.
- Prior reports: `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`, `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`, `DISK_LASER_CLASS_GHIDRA_REPORT.md`, `HOUSECLASS_CONSTRUCTOR_DETAILED.md`, `LABEL_AUDIT_LOG.md`, `OBJECT_DERIVED_DESTRUCTOR_SIDE_EFFECTS_CENSUS_RESWARM_20260528.md`.
- Rust surfaces scanned: `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/sim/docking/aircraft_dock.rs`, `src/sim/passenger.rs`, `src/sim/game_entity.rs`, `src/sim/movement/*`, `src/sim/production/*`.
