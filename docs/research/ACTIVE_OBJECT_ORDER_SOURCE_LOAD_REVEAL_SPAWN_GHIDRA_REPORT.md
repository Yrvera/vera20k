# Active Object Order Source: Load / Reveal / Spawn - Ghidra Research Report

**Address(es):** `ScenarioClass::Full_Init @ 0x00686B20`, `ObjectClass::Reveal @ 0x005F4EC0`, `FUN_0055BAA0 @ 0x0055BAA0`, `TechnoClass::Unlimbo @ 0x006F6CA0`, map section loaders `0x0071CA70`, `0x00743270`, `0x0041B110`, `0x0051FB00`, `0x0044F820`, `0x006B4C80`, `ObjectClass::Load @ 0x005F5E80`, `ObjectClass::Save @ 0x005F6250`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** order-source evidence outside the already-proved scheduler loop: scenario/map-load construction order, reveal/unlimbo insertion into the LogicClass active-object vector, representative runtime reveal/spawn paths, and cheap save/load persistence facts.  
**Non-Scope:** re-proving `LogicClass::PerTickUpdate` loop mechanics, class-specific `vtable+0x5C` AI bodies, complete savegame object-stream order, replay restore, and every possible direct `ObjectClass::Reveal` caller.  
**Confidence:** High for map-load order, reveal tail append, and ObjectClass save/load omissions; Medium for the Rust delta because Rust was scanned statically; Low for full save/load reconstruction because the rebuild owner was not drained.  
**Active in YR:** Yes for standard Yuri's Revenge scenario start and runtime object reveal; conditional for save/load paths, which are active only during persistence restore.

## 0. Scope Gate

**Target question:** How is native `LogicClass` active-object vector order established outside the scheduler loop, especially during scenario load, `ObjectClass::Reveal`/`Unlimbo`, runtime spawn/reveal, and save/load restoration?

**Non-goals:** This report does not re-prove live count reload, compacting removal, or same-pass append behavior inside `LogicClass::PerTickUpdate`; those are already covered by `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md` and `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`.

**Evidence needed to mark COMPLETE:** decompile plus caller/xref or assembly context for the scenario loader order, each in-scope section loop's construct-then-unlimbo path, the reveal-to-logic-vector append call, and a save/load restoration owner that either serializes the active vector or rebuilds it in a proved order.

**Stop conditions:** Stop after the above is proven for map load/reveal/spawn and the cheap `ObjectClass` save/load path, or when full persistence requires a broader save/load stream investigation. This report stops at that boundary and marks save/load reconstruction partial.

## 1. Overview

The native active-object order is not a sorted ID walk. During scenario load, objects are constructed and unlimboed in a fixed loader sequence; `Unlimbo` reaches `ObjectClass::Reveal`, and `Reveal` appends eligible objects to the `LogicClass` vector tail through `FUN_0055BAA0`.

That means native order is source-order plus reveal timing. Rust's `EntityStore` stable-ID order can only be equivalent when stable IDs are allocated in exactly the same effective reveal order and no fallback sorted-key path participates.

## 2. Key Offsets / Containers

| Offset / address | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `LogicClass+0x04/+0x10` | active object pointer array and live count | prior scheduler report; `0x0055B608..0x0055B619` | Yes |
| `ObjectClass+0x81` | in-limbo gate checked by `ObjectClass::Reveal` | `ObjectClass::Reveal @ 0x005F4EC0` decompile | Yes |
| `ObjectClass+0x90` | alive/display submission gate after reveal | `ObjectClass::Reveal` decompile | Yes |
| `ObjectClass+0x98` | LogicClass membership byte used by `FUN_0055BAA0` / remover | helper report `0x0055BAA0`, `0x0055BAE0` | Yes |
| `ObjectTypeClass+0x234` | type-level logic-registration eligibility checked before append | `ObjectClass::Reveal @ 0x005F5030..0x005F5045` | Yes, conditional per type |

## 3. Core Ordering Findings

### 3.1 Scenario load section order

`ScenarioClass::Full_Init` processes object-bearing map sections in this verified order:

1. `[Terrain]` through `TerrainClass::Read_Map_Section @ 0x0071CA70`
2. vein/tiberium maintenance and queue initialization
3. `[Units]` through `ScenarioClass::Read_Units_Section @ 0x00743270`
4. `[Aircraft]` through `FUN_0041B110`
5. `[Infantry]` through `FUN_0051FB00`
6. `[Structures]` through `BuildingClass::ReadFromINI @ 0x0044F820`
7. `[Smudge]` through `FUN_006B4C80`

Evidence: decompile of `ScenarioClass::Full_Init @ 0x00686B20`; xrefs from `ScenarioClass::Full_Init` to the loaders at `0x00687A74`, `0x00687AA7`, `0x00687ABF`, `0x00687ACB`, `0x00687AEA`, `0x00687B0E`; assembly context around `0x00687A74..0x00687B13` shows that exact call order. Active in YR: Yes, this is the standard scenario initialization path after `ScenarioClass::Read_Scenario @ 0x00684620`.

### 3.2 Section-internal order

The in-scope section readers use the INI section count and key-at-index helpers, then loop upward from index `0`.

| Section | Loop owner | Construct/reveal path | Evidence | Active in YR |
|---|---|---|---|---|
| `[Terrain]` | `0x0071CA70` | key index loop constructs `TerrainClass`; constructor/unlimbo path reaches `ObjectClass::Reveal` | decompile `0x0071CA70`; `TerrainClass::Unlimbo @ 0x0071D000` calls `ObjectClass::Reveal` | Yes |
| `[Units]` | `0x00743270` | key index loop constructs `UnitClass`, then calls vtable `+0xD8` `UnitClass::Unlimbo` before the next entry | decompile `0x00743270`; assembly context around `0x007434F0..0x00743567` | Yes |
| `[Aircraft]` | `0x0041B110` | key index loop constructs `AircraftClass`, then calls vtable `+0xD8` before next entry | decompile `0x0041B110`; xref from `0x00687ABF` | Yes |
| `[Infantry]` | `0x0051FB00` | key index loop constructs `InfantryClass`, checks sentinel cell, then calls vtable `+0xD8` | decompile `0x0051FB00`; xref from `0x00687ACB` | Yes |
| `[Structures]` | `0x0044F820` | key index loop constructs `BuildingClass`, then calls vtable `+0xD8` `BuildingClass::Unlimbo` | decompile `0x0044F820`; xref from `0x00687AEA` | Yes |
| `[Smudge]` | `0x006B4C80` | key index loop constructs `SmudgeClass`, whose constructor calls `ObjectClass::Reveal` when coords are not sentinel | decompile `0x006B4C80` and `SmudgeClass::Constructor @ 0x006B4A50` (call site to `ObjectClass::Reveal` at `0x006B4B14`) | Yes | <!-- corrected 2026-05-28: was SmudgeClass::Constructor @ 0x006B4B14; binary shows constructor entry is 0x006B4A50, 0x006B4B14 is the call site within the constructor — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> |

Material consequence: native load order is not global sort-by-ID or all-units/all-buildings from a merged list. It is loader-sequence order, then section key order, then reveal call timing inside each entry.

### 3.3 Unlimbo and reveal are the active insertion point

For techno objects, `FootClass::Unlimbo @ 0x004D7170` and `BuildingClass::Unlimbo @ 0x00440580` both call `TechnoClass::Unlimbo @ 0x006F6CA0`. `TechnoClass::Unlimbo` calls `ObjectClass::Reveal` first; only after reveal success does it continue with playfield status, fog, owner counters, facing, mission/locomotion setup, and other side effects.

`ObjectClass::Reveal @ 0x005F4EC0` calls `FUN_0055BAA0` with `ECX=0x87F778` and pushes `0` for the unique-scan argument. Assembly context at `0x005F5038..0x005F5040` shows `PUSH 0`, `MOV ECX,0x87F778`, `CALL 0x0055BAA0`. Active in YR: Yes; this is the standard `Unlimbo`/runtime reveal path and the same LogicClass singleton used by the tick scheduler.

Therefore, when a map unit/building/aircraft/infantry entry succeeds in `Unlimbo`, its active-vector insertion point is its reveal call, not its constructor alone. Failed `Unlimbo` paths delete/uninit the object and do not create a live active-vector entry.

### 3.4 Runtime spawn/reveal sources

Representative runtime paths use the same reveal append mechanism:

| Runtime source | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| fired bullets | `BulletClass::Fire` calls `ObjectClass::Reveal`, then display submission; logic-enabled bullets can append to the active vector tail | `BulletClassFireRevealArmAndSubmit @ 0x00468670` (call site to `ObjectClass::Reveal` at `0x00468684`); prior AAHeatSeeker2 latency report | Yes | <!-- corrected 2026-05-28: was @ 0x00468684; binary shows function entry 0x00468670, 0x00468684 is the call site — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->
| anims/particles/voxel anims | constructors call `ObjectClass::Reveal` directly for eligible world objects | xrefs to `ObjectClass::Reveal` from `AnimClass::Constructor`, `ParticleClass::Constructor`, `ParticleSystemClass::Constructor`, `VoxelAnimClass::Constructor` | Yes, conditional by object/type path |
| terrain/smudge map constructors | constructors or unlimbo call `ObjectClass::Reveal` during map load | `TerrainClass::Unlimbo @ 0x0071D000`; `SmudgeClass::Constructor @ 0x006B4A50` (call site at `0x006B4B14`) | Yes | <!-- corrected 2026-05-28: was SmudgeClass::Constructor @ 0x006B4B14; binary shows constructor entry 0x006B4A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->
| building light/open-transport helper paths | direct callers of `FUN_0055BAA0` exist outside `ObjectClass::Reveal` | xrefs to `FUN_0055BAA0` from `BuildingLightClass::Constructor @ 0x00435B01`, `TechnoClass::SetInOpenTransport @ 0x00710492`, `FUN_00437050`, `FUN_0075F8B0` | Yes for the functions, but exact gameplay conditions are outside this slot |

Runtime append order is therefore the chronological order of successful reveal/register calls. It is not automatically tied to creation ID if an object is constructed in limbo and revealed later.

### 3.5 Cheap save/load findings

`ObjectClass::Save @ 0x005F6250` serializes selected object fields including links, `+0x6C`, several bytes around `+0x74..+0x90`, and coordinates `+0x9C/+0xA0/+0xA4`. It does not serialize `ObjectClass+0x98`.

`ObjectClass::Load @ 0x005F5E80` calls `AbstractClass::Load`, registers several pointers for swizzling, initializes two `VocHandle`s, and clears `+0xA8`; it does not call `ObjectClass::Reveal` or `FUN_0055BAA0`.

Derived load functions inspected (`AircraftClass::Load @ 0x0041B430`, `BuildingClass::Load @ 0x00453E20`, `AnimClass::Load @ 0x00425280`, `OverlayClass::Load @ 0x005FD8F0`, `TerrainClass` load helper `0x0071CDA0`, `VoxelAnimClass` load helper `0x0074A970`) restore class vtables, global class-array entries, swizzled references, and class-specific state. This pass did not prove the later active-vector rebuild owner.

Active in YR: Conditional - active for save/load restore. The negative fact is high confidence for `ObjectClass` field persistence; the full order of post-load active-vector reconstruction remains deferred.

## 4. INI Keys / Data Sources

No gameplay INI key directly sorts the active-object vector. The order sources found here are structural:

| Data source | Effect | Evidence | Active in YR |
|---|---|---|---|
| Map section order in `ScenarioClass::Full_Init` | establishes coarse object load/reveal groups | `0x00687A74..0x00687B13` | Yes |
| INI section key order via `FUN_00526960` / `FUN_00526CC0` | establishes per-section iteration order | decompile of section readers | Yes |
| `ObjectTypeClass+0x234` | gates whether `ObjectClass::Reveal` attempts LogicClass registration | `0x005F5030..0x005F5040` | Yes, conditional |
| Savegame stream order | likely important for restoration, but not proven here | derived `Load` functions touched | Deferred |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/world/world_spawn.rs:41` | `spawn_from_map_with_resolved` iterates the supplied `MapEntity` slice (insert at :259, `reveal` at :260) | mismatches if `MapEntity` slice order is not native section order `[Terrain]`, `[Units]`, `[Aircraft]`, `[Infantry]`, `[Structures]`, `[Smudge]` with native key order |
| `src/sim/world/world_spawn.rs:301` | runtime `spawn_object_at_height` inserts and immediately reveals at lines 437-438 | matches tail-append shape only when spawn means successful reveal/unlimbo; mismatches limbo-created objects that should not yet be active |
| `src/sim/world/world_spawn.rs:464` | `spawn_object_limbo_at_height` inserts into `EntityStore` but does NOT register (comment at :588-589: "Limbo objects are NOT registered… mirroring ObjectClass+0x98") | FIXED (was DRIFT): construction/storage alone is no longer active-vector membership; registration deferred to reveal/unlimbo <!-- corrected 2026-05-29: verified Read world_spawn.rs:464-592 — insert at :587, no register; ROOT_CAUSE: STALE_RUST_DELTA --> |
| `src/sim/world/mod.rs:680` | `register_live_object` gates on `e.in_logic_vector` (the `+0x98` membership byte), tail-appends, sets the flag; idempotent. Paired with compacting `unregister_live_object` (:689) and `reveal`/`conceal`/`unlimbo` primitives | matches duplicate-prevented tail append AND now models the object-local `+0x98` membership byte via `in_logic_vector` <!-- corrected 2026-05-29: verified Read mod.rs:680-717 — +0x98 byte semantics now present; ROOT_CAUSE: STALE_RUST_DELTA --> |
| `src/sim/world/mod.rs:745` | `live_object_order_snapshot` returns `self.logic.snapshot()` verbatim (comment :740: "No sorted-ID fallback (was DRIFT)") | FIXED (was DRIFT): native active vector has no sorted-ID fallback; Rust no longer appends sorted `EntityStore` IDs <!-- corrected 2026-05-29: verified Read mod.rs:740-747 — verbatim snapshot, no fallback; ROOT_CAUSE: STALE_RUST_DELTA --> |
| `src/sim/passenger.rs:355` | garrison reconciliation consumes `live_object_order_snapshot` | no longer affected by sorted-ID fallback (removed); still affected by map-load order mismatches |
| `src/sim/world/mod.rs:1508` | `advance_tick` uses phased systems and pre-collected/sorted entity keys in many places | broader scheduler mismatch already covered by slot 1/parent reports |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::Reveal -> FUN_0055BAA0` tail registration | verified | decompile `0x005F4EC0`; assembly `0x005F5038..0x005F5040`; xref to `0x0055BAA0` | none |
| scenario loader coarse object order | verified | `ScenarioClass::Full_Init @ 0x00686B20`; xrefs/calls `0x00687A74..0x00687B13` | none for in-scope sequence |
| `[Terrain]` reader loop | verified | `TerrainClass::Read_Map_Section @ 0x0071CA70`; `TerrainClass::Unlimbo @ 0x0071D000` | constructor internals not fully drained |
| `[Units]` reader loop | verified | `ScenarioClass::Read_Units_Section @ 0x00743270` | full convoy/follower resolution beyond order source not covered |
| `[Aircraft]` reader loop | verified | `FUN_0041B110`; xref from `0x00687ABF` | function name remains generic |
| `[Infantry]` reader loop | verified | `FUN_0051FB00`; xref from `0x00687ACB` | no further infantry subcell parity in this report |
| `[Structures]` reader loop | verified | `BuildingClass::ReadFromINI @ 0x0044F820`; xref from `0x00687AEA` | upgrade side-spawns touched, not exhausted |
| `[Smudge]` reader loop | verified | `FUN_006B4C80`; `SmudgeClass::Constructor @ 0x006B4A50` (call site `0x006B4B14`) | smudge logic-eligibility not fully classified | <!-- corrected 2026-05-28: was @ 0x006B4B14; constructor entry is 0x006B4A50 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->
| direct non-`Reveal` `FUN_0055BAA0` callers | touched-not-exhausted | xrefs to `0x00435B01`, `0x00437070`, `0x00710492`, `0x0075F95F` | class/gameplay condition mapping |
| ObjectClass save/load membership persistence | verified partial | `ObjectClass::Save @ 0x005F6250`; `FUN_005F5E80` (inferred `ObjectClass::Load`) | full post-load active-vector rebuild owner |
| savegame stream reconstruction order | deferred | derived `Load` functions touched | needs save/load stream owner trace |
| Rust stable-ID equivalence | touched-not-exhausted | `src/sim/world/world_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/passenger.rs` | implementation design/tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-AOOS-001 - What inserts a normal revealed object into the active object vector? -> ObjectClass::Reveal calls FUN_0055BAA0 on 0x87F778 with unique flag 0.` (evidence: `0x005F5038..0x005F5040`)
- `[RESOLVED] OQ-AOOS-002 - Is map load active in standard YR? -> Yes; ScenarioClass::Read_Scenario calls ScenarioClass::Full_Init, which executes the map/object loaders.` (evidence: `0x00684620`; `0x00686B20`)
- `[RESOLVED] OQ-AOOS-003 - What is the coarse scenario object load order? -> Terrain, vein/tiberium maintenance, Units, Aircraft, Infantry, Structures, Smudge.` (evidence: `0x00687A74..0x00687B13`)
- `[RESOLVED] OQ-AOOS-004 - Do unit entries reveal before the next unit entry? -> Yes; the section loop constructs one unit and calls vtable +0xD8 before continuing.` (evidence: `0x00743270` decompile)
- `[RESOLVED] OQ-AOOS-005 - Do building entries reveal through TechnoClass::Unlimbo? -> Yes for normal building placement; BuildingClass::Unlimbo calls TechnoClass::Unlimbo, which calls ObjectClass::Reveal first.` (evidence: `0x00440580`; `0x006F6CA0`)
- `[RESOLVED] OQ-AOOS-006 - Are runtime bullets on the same reveal/append path? -> Yes; BulletClass::Fire calls ObjectClass::Reveal.` (evidence: xref `0x00468684`; prior AAHeatSeeker2 report)
- `[RESOLVED] OQ-AOOS-007 - Does constructor/global class-array order alone define active order? -> No; active membership happens at reveal/register time.` (evidence: `ObjectClass::Reveal`; `FUN_0055BAA0`; derived load/constructor scans)
- `[RESOLVED] OQ-AOOS-008 - Does ObjectClass::Save serialize +0x98? -> No observed write/read of +0x98 in scoped ObjectClass save fields.` (evidence: `0x005F6250` decompile)
- `[RESOLVED] OQ-AOOS-009 - Does ObjectClass::Load register into LogicClass? -> No; it loads base object state and swizzle pointers, with no call to ObjectClass::Reveal or FUN_0055BAA0.` (evidence: `0x005F5E80`; xrefs to `0x005F5E80`)
- `[RESOLVED] OQ-AOOS-010 - Can Rust sorted stable IDs be assumed equivalent? -> No; native active order is tail append by reveal timing, not sorted IDs.` (evidence: `0x005F5040`; `0x00687A74..0x00687B13`; Rust `live_object_order_snapshot`)
- `[DEFERRED] OQ-AOOS-011 - What exact owner rebuilds the active vector after savegame load?` (category: `requires-different-system-context`; reason: derived `Load` bodies and `ObjectClass::Load` do not prove the later restore owner; next-step-if-pursued: trace the IPersist stream object enumeration and post-load `Init`/rehydration pass.)
- `[DEFERRED] OQ-AOOS-012 - Are every direct FUN_0055BAA0 caller's gameplay conditions active in ordinary YR?` (category: `bounded-cost-too-high`; reason: xrefs prove callable shapes but class-condition mapping is a separate lifecycle slice; next-step-if-pursued: investigate `0x00437050`, `0x00710492`, `0x0075F8B0` individually.)
- `[DEFERRED] OQ-AOOS-013 - How do replay restore and multiplayer resync recreate active order?` (category: `out-of-scope`; reason: not needed for map-load/reveal/spawn comparison; next-step-if-pursued: trace replay/load systems after save/load owner is known.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Scenario load active order is fixed loader sequence plus section key order, then reveal timing: terrain before units before aircraft before infantry before structures before smudges for the inspected object sections. | `ScenarioClass::Full_Init @ 0x00686B20`; assembly `0x00687A74..0x00687B13`; section-reader decompiles | `spawn_from_map_with_resolved` trusts incoming `MapEntity` slice order and reveals each entity immediately | `src/sim/world/world_spawn.rs:41`; map parser/object assembly surface | Preserve native section-group ordering and per-section key order when building `live_object_order`; do not merge all parsed objects and sort by stable ID/cell/type. | Load a map with one unit, one aircraft, one infantry, one building, and one smudge whose parsed/cell order is intentionally different; active order must match native section order. | `map_load_live_object_order_follows_native_section_sequence` | Do not use stable-ID sort as proof of native map-load order unless the IDs were allocated from the verified native section sequence. |
| Successful `Unlimbo` reaches `ObjectClass::Reveal`; `Reveal` appends eligible objects to `LogicClass` through `FUN_0055BAA0` with unique flag 0. | `TechnoClass::Unlimbo @ 0x006F6CA0`; `ObjectClass::Reveal @ 0x005F5038..0x005F5040`; helper report | `spawn_object_at_height` immediately reveals after insertion (correct for successful reveal/unlimbo). Create-in-storage IS now split from register: `spawn_object_limbo_at_height` (:464) inserts without registering. STILL NOT enforced: the `IsAlive` / `ObjectTypeClass+0x234` eligibility / Mark(PUT)-success reveal gate-chain — Rust `reveal` unconditionally appends, with no type-level eligibility or placement-success gate. | `src/sim/world/world_spawn.rs:301`, `:464`; `Simulation::reveal/register_live_object` at `src/sim/world/mod.rs:680` | Add the `+0x234` type-eligibility and reveal-success gate so `reveal` only appends when native `ObjectClass::Reveal` would; keep limbo create-without-register. | Create object in limbo, assert it is stored but absent from live order; reveal it later and assert tail append at reveal time. | `limbo_object_registers_only_on_reveal_tail_append` | Do not conflate object construction, `EntityStore` insertion, and active-object membership. <!-- corrected 2026-05-29: create/register split now done (limbo no-register at world_spawn.rs:587-589); preserved remaining gate-chain gap; ROOT_CAUSE: STALE_RUST_DELTA --> |
| `ObjectClass::Save`/`Load` do not persist or restore the logic-membership byte `+0x98`, and `ObjectClass::Load` does not call reveal/register. | `ObjectClass::Save @ 0x005F6250`; `ObjectClass::Load @ 0x005F5E80`; derived load bodies touched | Rust save/load behavior for `live_object_order` not audited. The sorted-ID fallback that would have masked missing restoration has been REMOVED (`live_object_order_snapshot` returns the active order verbatim), so post-load reconstruction is no longer silently papered over. | future save/load surfaces; `Simulation::live_object_order_snapshot` at `src/sim/world/mod.rs:745` | Persist/rebuild active order from the native restore owner once verified; do not reconstruct by sorted stable IDs as a placeholder for parity. | Save a scenario with active objects whose creation IDs differ from reveal order; after load, first passenger/garrison reconciliation must see native restored active order. | `save_load_restores_live_object_order_not_sorted_ids` | Do not serialize Rust `live_object_order` as the final parity answer until the native save/load rebuild order is proved. |

## 9. Negative Facts / Do Not Do

- Do not assume `EntityStore` sorted key order is equivalent to gamemd active-object order. Active in YR: Yes; evidence `ObjectClass::Reveal -> FUN_0055BAA0` tail append and scenario loader call order.
- Do not register limbo-created objects in the live AI order merely because they exist in storage. Active in YR: Yes; evidence active membership is tied to reveal/register, while `ObjectClass::Load` and constructors alone do not prove active membership.
- Do not merge map objects across INI sections and sort them by cell, type, or stable ID before seeding live order. Active in YR: Yes; evidence `ScenarioClass::Full_Init` hard call sequence.
- Do not reuse building `Unlimbo` as the save/load restoration mechanism. Active in YR: Yes; prior building report says save/load does not go through `Unlimbo`; this report confirms `ObjectClass::Load` has no reveal/register call.
- Do not treat direct xrefs to `FUN_0055BAA0` as all ordinary map-load order sources without mapping their class conditions; several are touched but not exhausted.

## 10. Remaining Uncertainty

- Full savegame active-vector reconstruction order is unresolved. The key verified fact is negative: `ObjectClass` save/load does not serialize or restore `+0x98` and does not directly register. A follow-up should start from the save/load stream owner and post-load `Init` pass rather than from `ObjectClass::Load` alone.
- Direct non-`ObjectClass::Reveal` callers of `FUN_0055BAA0` were not individually classified for ordinary gameplay frequency.
- Overlay-pack loading order was not drained because the target was active-object vector order; overlays may be objects, but their logic eligibility and registration conditions need a separate overlay lifecycle slice if they become relevant.

## 11. Stale Docs / Replacement Wording

- Any wording that says "Rust stable-ID order is equivalent to gamemd active object order when IDs are deterministic" should be replaced with: "gamemd active-object order is the `LogicClass` vector tail-append order produced by successful reveal/register calls. During scenario load this is seeded by the native loader sequence and per-section INI key order; during runtime it is reveal timing. Stable IDs are equivalent only if allocated from that exact effective reveal order and no sorted fallback participates."
- Any wording that says "save/load can restore active order from `ObjectClass` fields" should be replaced with: "`ObjectClass::Save`/`Load` do not persist or restore the `+0x98` LogicClass membership byte and `ObjectClass::Load` does not call reveal/register. The native post-load active-vector rebuild owner remains a required follow-up before implementing parity save/load order."

## Sources

- Ghidra decompile/read-only:
  - `ObjectClass::Reveal @ 0x005F4EC0`
  - `FUN_0055BAA0 @ 0x0055BAA0` via prior helper report and xrefs
  - `TechnoClass::Unlimbo @ 0x006F6CA0`
  - `FootClass::Unlimbo @ 0x004D7170`
  - `BuildingClass::Unlimbo @ 0x00440580`
  - `TerrainClass::Unlimbo @ 0x0071D000`
  - `OverlayClass::Unlimbo @ 0x005FD270`
  - `ScenarioClass::Full_Init @ 0x00686B20`
  - `ScenarioClass::Read_Units_Section @ 0x00743270`
  - `FUN_0041B110` (`[Aircraft]` reader)
  - `FUN_0051FB00` (`[Infantry]` reader)
  - `BuildingClass::ReadFromINI @ 0x0044F820`
  - `FUN_006B4C80` (`[Smudge]` reader)
  - `FUN_005F5E80` (inferred `ObjectClass::Load`; Ghidra label not set) <!-- corrected 2026-05-28: was ObjectClass::Load @ 0x005F5E80; binary label is FUN_005f5e80 (unlabeled); behavioral description confirmed correct via decompile_function 0x005F5E80 — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
  - `ObjectClass::Save @ 0x005F6250`
- Assembly/xref evidence:
  - `ObjectClass::Reveal` call to `FUN_0055BAA0`: `0x005F5038..0x005F5040`
  - scenario loader order: `0x00687A74..0x00687B13`
  - xrefs to `ObjectClass::Reveal` and `FUN_0055BAA0` from Ghidra MCP
- Prior research:
  - `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
  - `docs/research/BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`
- Rust scan:
  - `src/sim/world/world_spawn.rs`
  - `src/sim/world/mod.rs`
  - `src/sim/passenger.rs`
