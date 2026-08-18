# AnimClass Global Object Registration/Lifetime - Re-swarm Slot 1

**Address(es):** `AnimClass::Constructor @ 0x00421EA0`, load constructor `0x00422720`, destructor `0x004228E0` (corrected 2026-05-29: was `0x00422900`; binary entry is `0x004228E0` via `get_function_by_address 0x004228E0` — RTTI_LABEL_DRIFT), `AnimClass::SetOwnerObject @ 0x00424B50`, `AnimClass::Destroy @ 0x004255B0`, `ObjectClass::Reveal @ 0x005F4EC0` (corrected 2026-05-29: was `0x005F4F60`; binary entry is `0x005F4EC0` via `get_function_by_address 0x005F4F60` — RTTI_LABEL_DRIFT), `ObjectClass::UnInit @ 0x005F65F0`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `TechnoClass::Fire_At @ 0x006FDD50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** native global `AnimClass` registration, AI/destroy/deferred-delete lifetime, owner attach/detach global-array scans, and whether ordinary ownerless occupied-building `OccupantAnim` flashes require a full Rust `AnimClass` object pool for parity.
**Non-Scope:** full draw traversal/layer sort contract, full `AnimClass::DrawIt` flags/translucency/shadow contract, `Tactical_AdjustForZ` internals, complete save/load object resurrection, bouncer/meteor/veins/particle ownership, and non-garrison generic anim implementation.
**Confidence:** High for the scoped pool-vs-embedded decision; Medium for save/load implications because this slot did not exhaust all serialization callers.
**Active in YR:** Yes. Stock occupied-building fire reaches `TechnoClass::Fire_At`, constructs `WeaponType+0x110 OccupantAnim`, and stock UC art sections exist in `artmd.ini`.

## Working Notes Gate

Target question: Is a full app-layer `AnimClass` object pool required for parity of ordinary garrison `OccupantAnim` flashes, or is an embedded per-flash runtime enough if it preserves native lifecycle/order?
Non-goals: Do not re-solve lifecycle fields, draw flags, translucency, shadow, or exact depth; sibling swarm slots own those.
Evidence needed to mark COMPLETE: constructor registration evidence, destruction/removal evidence, AI scheduler evidence, ownerless garrison path evidence, and Rust-facing handoff.
Stop conditions: stop after proving whether `g_AnimClass_Array` identity/order has player-visible effects for ordinary ownerless garrison flashes, with unrelated generic anim pool needs deferred.

## 1. Overview

Native `AnimClass` objects are real world objects, but two different containers matter. `g_AnimClass_Array` is the class registry used for construction/destruction, scans, save/load and owner-detach maintenance. Per-tick AI for revealed objects runs through the live `LogicClass` object vector (`vtable+0x5C`), not by a dedicated "for every g_AnimClass_Array entry" loop in the ordinary tick path.

For the ordinary occupied-building `OccupantAnim` path, the created anim is ownerless: the building branch writes `AnimClass+0x100 = -200` and does not call `SetOwnerObject`. That removes the strongest immediate reason to mirror native pointer identity: attached-owner scans. For this specific path, a shared app-layer `AnimRuntime` embedded in `GarrisonMuzzleFlash` is parity-sufficient if it preserves creation order, live tick/lifecycle order, same-object `Next` state, and native-equivalent draw ordering. A full global `AnimClass` pool is still the right long-term architecture for all generic anims, save/load, attached anims, and cross-system ownership scans, but this slot did not find a pool-only requirement for ordinary garrison flashes.

## 2. Key Offsets / Globals

| Item | Address / offset | Meaning in this slice | Evidence |
|---|---:|---|---|
| `g_AnimClass_Array` data | `0x00A8E9AC` | Dynamic vector of `AnimClass*` | constructor/destructor assembly |
| `g_AnimClass_Array_Capacity` | `0x00A8E9B0` | capacity checked before append | `0x00422058..0x0042207B` |
| growth flag / increment | `0x00A8E9B5`, `0x00A8E9BC` | dynamic vector growth controls | `0x00422067..0x0042208E` |
| `g_AnimClass_Array_Count` | `0x00A8E9B8` | live registry count | `0x0042205D`, `0x0042209B`, `0x00422AEE` |
| `AnimClass+0x90` | byte | active/alive byte set after registry append | constructor `0x004220AA..0x004220B0` |
| `AnimClass+0x98` | byte | object on-map / logic-vector insertion guard used by `FUN_0055BAA0` | `0x0055BAA0..0x0055BABB` |
| `AnimClass+0xCC` | pointer | owner object for attached anims | `SetOwnerObject`, destructor |
| `AnimClass+0x100` | int | instance z adjust; occupied building path writes `-200` | prior report and `Fire_At` |
| `AnimClass+0x195` | byte | loop remaining; also cleared by owner/global scans | `AnimClass::AI`, `FUN_00422B80`, `TechnoClass::AI_Update` |
| `PendingDeleteList` data | `0x00B0F69C` | deferred delete vector data | `ObjectClass::UnInit @ 0x005F6668..0x005F667D` |
| `PendingDeleteList` count | `0x00B0F6A8` | deferred delete count | `ObjectClass::UnInit` |

## 3. Core Logic

### Constructor registration

`AnimClass::Constructor @ 0x00421EA0` appends the new object pointer to `g_AnimClass_Array` before setting the object active byte.

Load-bearing assembly:

- `0x00422058..0x00422063`: load capacity from `0x00A8E9B0`, load count from `0x00A8E9B8`, compare.
- `0x00422067..0x0042208E`: dynamic-vector growth gate and call through vector vtable when full.
- `0x00422092..0x004220A7`: reload count, increment `0x00A8E9B8`, load `0x00A8E9AC`, store `ESI` at `array[old_count]`.
- `0x004220AA..0x004220B0`: load type pointer and set byte `this+0x90 = 1`.

The load/deserialization constructor `0x00422720` repeats the same append pattern (`g_AnimClass_Array_Count++`, `g_AnimClass_Array[old_count] = this`) at `0x0042289D..0x004228B2` (corrected 2026-05-29: start was `0x00422863` which has no instruction; binary append starts at `0x0042289D` via `get_assembly_context 0x004228B5` — RTTI_LABEL_DRIFT). End boundary `0x004228B5` is a post-append `LEA ECX,[ESI+0x1a0]` (first VocHandle init), correctly marking the boundary.

Active in YR: Yes. The ordinary occupied-shot constructor is called from live `TechnoClass::Fire_At`.

### Reveal and AI scheduling

Constructor registration alone is not the ordinary AI scheduler. For stock UC anims, constructor calls `ObjectClass::Reveal(coords, 0)` because the type is not bouncer/meteor. `ObjectClass::Reveal @ 0x005F4EC0` (corrected 2026-05-29: was `0x005F4F60`; binary entry is `0x005F4EC0` via `get_function_by_address 0x005F4F60` — RTTI_LABEL_DRIFT) can call `FUN_0055BAA0`, which checks byte `object+0x98`; when not already inserted it calls `DynamicVector__Insert @ 0x005519B0`.

Load-bearing assembly:

- `0x0055BAA0..0x0055BAAD`: load and test object byte `+0x98`; return success if already inserted.
- `0x0055BAB5..0x0055BABB`: push args and call `0x005519B0` to insert.
- `0x0055B5FB..0x0055B619`: `LogicClass::PerTickUpdate` walks vector data at `+0x04`, calls `vtable+0x5C`, increments index, then reloads vector count from `+0x10` before the loop comparison.

Implication: if `Fire_At` constructs a garrison anim before the logic-vector cursor has passed the appended object, it may receive `AnimClass::AI` in the same pass. That same-pass visit still does not advance the frame because sibling research verified constructor byte `+0x19C` first-AI guard.

Active in YR: Yes. This is the same live object-vector mechanism used by stock world objects; no TS-only gate was found for this path.

### Destroy and deferred deletion

`AnimClass::AI` marks finished anims and calls vtable `+0xF8`, which resolves to `AnimClass::Destroy @ 0x004255B0`. Destroy does not remove the object directly from `g_AnimClass_Array`; it detaches owner state, releases sound handles, optionally plays stop sound, then calls `ObjectClass::UnInit @ 0x005F65F0`.

Load-bearing assembly:

- `0x004255B6..0x004255C3`: if `OwnerObject` exists, call owner vtable `+0x60` with this anim.
- `0x004255C6..0x004255CA`: call `AnimClass::SetOwnerObject(NULL)`.
- `0x005F6668..0x005F667D`: `ObjectClass::UnInit` appends `ESI` to pending-delete list at `0x00B0F69C` and increments count `0x00B0F6A8`.

The actual class destructor `AnimClass::~AnimClass @ 0x004228E0` (corrected 2026-05-29: was `0x00422900`; binary entry confirmed via `get_function_by_address 0x004228E0` — RTTI_LABEL_DRIFT) removes the pointer from `g_AnimClass_Array` by find-index then compaction:

- `0x00422AD0..0x00422AD9`: call vector find through `DAT_00A8E9A8+0x10`.
- `0x00422AE1..0x00422AEE`: compare found index with `g_AnimClass_Array_Count`, decrement count.
- `0x00422AF6..0x00422B0C`: shift later array entries left one slot until index reaches the new count.

Active in YR: Yes. Normal completed garrison anims take the same `AnimClass::AI -> Destroy -> UnInit -> destructor` lifecycle.

### Owner attachment scans

`AnimClass::SetOwnerObject @ 0x00424B50` scans `g_AnimClass_Array` to determine whether any other anim still references the same owner before clearing the owner's attached-anim flag. The destructor also repeats this attached-owner scan when `g_GameActive != 0`.

Load-bearing assembly:

- `0x00424B79..0x00424BA7`: load count `0x00A8E9B8`, array `0x00A8E9AC`, walk entries, skip `this`, compare `other+0xCC` to `this+0xCC`.
- `0x00424BAB..0x00424BB6`: if no other attached anim matched, call owner vtable `+0x17C` and clear owner byte `+0x84`.
- `0x00422924..0x0042294C`: destructor uses the same scan shape over `g_AnimClass_Array`.

Ordinary occupied-building garrison flashes do not use this owner attachment path. The building branch in `Fire_At` does not call `SetOwnerObject` after writing `ZAdjust=-200`; prior `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md` and `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` both verify this.

Active in YR: Yes for attached anims; not active for ordinary occupied-building garrison flashes because they are ownerless.

## 4. INI Keys / Variant Set

| Section | Source | Relevant keys | Pool implication |
|---|---|---|---|
| `[UCFLASH]` | `ini/artmd.ini`, fallback `ini/art.ini` | `Layer=ground`, `Translucent=yes` | no owner attachment, no `Next`, no shadow/tiled/bouncer keys |
| `[UCCONS]` | `ini/artmd.ini` | `Layer=ground`, `Translucent=yes` | same |
| `[UCINIT]` | `ini/artmd.ini` | `Layer=ground`, `Translucent=yes` | same |
| garrison weapons | `ini/rulesmd.ini` | `OccupantAnim=UCCONS/UCINIT/UCFLASH` | active ordinary YR path |

No scoped stock UC section defines `Bouncer`, `IsMeteor`, `IsVeins`, `Next`, `TrailerAnim`, `ExpireAnim`, `Spawns`, `Damage`, `MakeInfantry`, or owner-related metadata that would make global pointer identity visible for ordinary garrison flashes.

## 5. Integration Points

| Stage | Native behavior | Evidence | Rust implication |
|---|---|---|---|
| Shot creation | `Fire_At` allocates `0x1C8`, calls full constructor, building branch writes `-200`, no owner attach | `0x006FDD50`; prior reports | app-layer flash can remain ownerless |
| Registry append | constructor appends to `g_AnimClass_Array` in append order | `0x00422092..0x004220A7` | preserve creation order in flash vector |
| Logic insertion | `ObjectClass::Reveal` inserts into live LogicClass vector if eligible | `0x005F4EC0` (corrected 2026-05-29 from `0x005F4F60`), `0x0055BAA0` | lifecycle tick order should be fixed-tick/object-order based, not render time |
| AI tick | live vector calls `vtable+0x5C` and reloads count after each call | `0x0055B608..0x0055B619` | appended anims can be visited same pass; first-AI guard still returns |
| Finished anim | `AnimClass::Destroy` calls `ObjectClass::UnInit`; destructor later compacts global array | `0x004255B0`, `0x005F65F0`, `0x004228E0` (corrected 2026-05-29 from `0x00422900`) | remove finished flash by native lifecycle decision; keep stable order for survivors |
| Attached-owner cleanup | global array scan checks `OwnerObject` sharing | `0x00424B50`, `0x00422900` | not needed for ownerless garrison flashes; needed for generic attached anims later |

## 6. Current Rust Implementation Status

Current Rust uses a specific app-layer flash list instead of any generic anim object pool:

- `src/app_building_anim.rs:702..764` spawns new `GarrisonMuzzleFlash` entries from pending fire events, extends `state.garrison_muzzle_flashes`, then advances/removes them.
- `src/app_building_anim.rs:767..774` advances by elapsed fixed-sim milliseconds and deletes at raw SHP frame count.
- `src/app_building_anim.rs:792..796` explicitly notes that generic `End/Loop/Next/Shadow` metadata is not represented yet.
- `src/sim/components.rs:675..701` stores fixed garrison flash fields, rate, frame count, and `z_adjust`, but no generic `AnimClass` lifecycle state.
- `src/app_instances/overlays.rs:485..527` draws flashes by iterating `state.garrison_muzzle_flashes` in vector order.
- `src/app_instances/overlays.rs:531..546` still maps z adjust through a float bias, not the native integer depth expression; sibling slots own exact draw/depth closure.

Net: Rust does not need a full pool to fix the ordinary garrison lifecycle gap, but it does need a reusable app-layer `AnimRuntime` or equivalent state machine embedded in the flash object. If that embedded runtime preserves insertion order, first-AI guard, native end/loop/next semantics, and deterministic survivor compaction, it covers the pool-visible behavior found in this slot for ownerless UC flashes.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Full constructor registry append | verified | decompile `0x00421EA0`; assembly `0x00422058..0x004220A7` | none |
| Load constructor registry append | verified | decompile `0x00422720`; append range `0x0042289D..0x004228B2` (corrected 2026-05-29 from `0x00422863..0x004228B5`) | save/load caller closure deferred |
| `ObjectClass::Reveal` logic-vector insertion | verified | decompile `0x005F4EC0` (entry; corrected 2026-05-29 from `0x005F4F60`); assembly `0x0055BAA0..0x0055BABB` | exact `SetVisibility` branch not re-expanded |
| LogicClass live vector AI order | verified | assembly `0x0055B608..0x0055B619`; prior scheduler docs | none for order shape |
| Destroy -> UnInit pending delete | verified | decompile `0x004255B0`, `0x005F65F0`; assembly `0x005F6668..0x005F667D` | pending-delete cleanup timing not exhausted |
| Destructor global-array removal/compaction | verified | decompile `0x004228E0` (entry; corrected 2026-05-29 from `0x00422900`); assembly `0x00422AD0..0x00422B0C` | none for removal shape |
| `SetOwnerObject` global owner scans | verified | decompile/assembly `0x00424B50`, `0x00424B79..0x00424BB6` | attached anim generic pool future |
| Garrison `Fire_At` ownerless building branch | verified-via-prior-doc-and-spot | `OCCUPANTANIM...REPORT.md`, `ANIMCLASS_SPAWN_PATHS...` | not re-expanded in this slot |
| Draw traversal/layer ordering | deferred | sibling slot target | slot 3 |
| Draw flags/translucency/shadow | deferred | sibling slot target | slot 4 |
| `Tactical_AdjustForZ` exact integer depth | deferred | sibling slot target | slot 5 |
| Save/load exact effect on mid-flash garrison anims | deferred | load constructor touched | focused save/load anim persistence investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does constructor append to a global AnimClass array? -> Yes, appends `this` at `g_AnimClass_Array[old_count]` and increments count.` (evidence: `0x00422092..0x004220A7`)
- `[RESOLVED] OQ-02 - Is this global array removed by Destroy directly? -> No; Destroy calls `ObjectClass::UnInit`; removal from `g_AnimClass_Array` occurs in the class destructor.` (evidence: `0x004255B0`, `0x005F65F0`, `0x00422900`)
- `[RESOLVED] OQ-03 - Does destructor preserve order of remaining anims? -> It compacts by shifting later entries left after the removed index.` (evidence: `0x00422AF6..0x00422B0C`)
- `[RESOLVED] OQ-04 - Is `g_AnimClass_Array` the ordinary per-tick AI loop? -> No evidence for that; revealed objects are inserted into the LogicClass live vector, which calls `vtable+0x5C` and reloads live count.` (evidence: `0x005F4F60`, `0x0055BAA0..0x0055BABB`, `0x0055B608..0x0055B619`)
- `[RESOLVED] OQ-05 - Does ordinary occupied-building garrison flash attach to the building owner object? -> No, the building branch writes `ZAdjust=-200` and does not call `SetOwnerObject`.` (evidence: `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`; `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-06 - Are `SetOwnerObject` global scans relevant to ownerless UC flashes? -> Not for the stock occupied-building branch because `OwnerObject` remains null.` (evidence: `0x00424B50`; prior `Fire_At` docs)
- `[RESOLVED] OQ-07 - Does global array insertion order matter indirectly? -> Yes for class registry scans and destructor compaction; for ownerless UC flashes this reduces to preserving creation/survivor order, not native pointer identity.` (evidence: `0x004220A1..0x004220A7`, `0x00422AF6..0x00422B0C`)
- `[RESOLVED] OQ-08 - Does same-pass AI require full pool? -> No; it requires live tick order and first-AI guard. A vector of embedded runtimes can model that for this ownerless path.` (evidence: `0x0055B608..0x0055B619`; sibling first-AI guard report)
- `[RESOLVED] OQ-09 - Is `Next` a reason for a full global pool in this path? -> No pool-only reason found; `Next` reuses the same object identity, which an embedded runtime can preserve inside one flash record.` (evidence: `AnimClass::AI @ 0x00423AC0`; sibling lifecycle report)
- `[DEFERRED] OQ-10 - Does save/load during a live garrison flash require a global anim object model?` (category: requires-different-system-context; reason: load constructor was touched but save/load callers were not exhausted; next-step-if-pursued: focused `AnimClass` save/load persistence slot)
- `[DEFERRED] OQ-11 - Does native draw traversal use global array order as a tie-breaker?` (category: requires-different-system-context; reason: sibling draw traversal slot owns LayerClass/display traversal; next-step-if-pursued: slot 3 render traversal report)
- `[DEFERRED] OQ-12 - Does exact depth/sort force native object pool ordering?` (category: requires-different-system-context; reason: depth helper and render comparator are sibling targets; next-step-if-pursued: slots 3 and 5)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full constructor appends each `AnimClass` to a global registry in creation order; destructor removes by left-compaction. | `0x00422092..0x004220A7`, `0x00422AD0..0x00422B10` | Rust has `state.garrison_muzzle_flashes` vector, which already preserves append order and `retain_mut` survivor order. | `src/app_building_anim.rs`, `src/sim/components.rs` | Keep garrison flashes in deterministic creation order and remove completed entries without reordering survivors. A full global pool is not required for this ownerless path. | Two garrison shots in the same fixed-tick batch produce two flash runtimes in event order; if the first expires first, the second remains and keeps its relative order. Proposed test: `garrison_anim_runtime_preserves_append_and_survivor_order`. | Do not introduce a broad native-object pool solely to fix ordinary garrison flashes. |
| Revealed anims tick through the live LogicClass vector, which reloads count after each `vtable+0x5C` call; `g_AnimClass_Array` is not the ordinary AI loop. | `0x005F4F60`, `0x0055BAA0..0x0055BABB`, `0x0055B608..0x0055B619` | Rust currently advances flashes once after fixed tick batching; lifecycle is not modeled as a first-AI guarded runtime. | `src/app_building_anim.rs`, future app-layer `AnimRuntime` | Model first-AI guarded native lifecycle on fixed sim ticks. Embedded runtime is sufficient if tick order matches event/order semantics. | Newly spawned UC flash receives an initial runtime visit that clears first-AI guard without advancing, then advances/deletes only on the next eligible fixed tick. Proposed test: `garrison_anim_runtime_first_ai_guard_blocks_same_visit_advance`. | Do not claim `g_AnimClass_Array` itself is the scheduler; do not advance by render wall time. |
| Occupied-building garrison flashes are ownerless: building branch writes `ZAdjust=-200` and does not call `SetOwnerObject`. | `TechnoClass::Fire_At @ 0x006FDD50`; prior reports | Rust flash stores `building_id` for app lookup but no native owner attachment behavior. | `src/sim/components.rs`, `src/app_building_anim.rs` | Keep native owner-attachment side effects out of garrison flash runtime unless implementing generic attached anims. `building_id` may remain an app lookup, not native `OwnerObject`. | Destroying/moving/changing owner of the building does not invoke attached-anim owner-detach semantics for this garrison flash path. Proposed test: `garrison_occupant_anim_does_not_require_owner_attachment_cleanup`. | Do not use `building_id` as native `OwnerObject` for global detach scans. |
| `SetOwnerObject` scans `g_AnimClass_Array` to preserve/clear owner attached-anim flags, but that scan is only relevant when `OwnerObject != NULL`. | `0x00424B79..0x00424BB6`, `0x00422924..0x0042294C` | No generic attached anim pool in Rust. | Future generic app-layer anim pool, not narrow garrison flash fix | Defer native-style global pool until implementing moving-unit muzzle flashes, parachutes, building active anims, attached effects, or save/load anim objects. | Non-garrison moving unit muzzle flash attached to a unit follows owner and detaches without clearing owner flag while another attached anim remains. Proposed future test: `attached_anim_owner_flag_survives_until_last_attached_anim`. | Do not solve attached-owner scans in the garrison flash patch. |

## 10. Negative Facts / Do Not Do

- Do not claim ordinary per-tick `AnimClass::AI` is driven by iterating `g_AnimClass_Array`; the live object vector dispatch at `0x0055B608..0x0055B619` is the verified scheduler shape.
- Do not treat `building_id` in Rust `GarrisonMuzzleFlash` as native `AnimClass+0xCC OwnerObject`; native occupied-building garrison flashes are ownerless.
- Do not build a full global `AnimClass` pool just to implement UC flash lifecycle; embedded runtime plus stable order is enough for this ownerless path.
- Do not ignore global pool needs for future generic anims: attached moving-unit muzzle flashes, `SetOwnerObject`, save/load, building active anims, parachutes, and bouncer/meteor/particle effects need broader architecture later.
- Do not let `Next` spawn a separate unrelated flash record for garrison lifecycle; native switches the same anim object's type/timing state.

## 11. Stale Docs / Replacement Wording

- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` around its tick-order section currently says the anim phase is "for each AnimClass in g_AnimClass_Array". Replacement wording: "AnimClass constructor appends to `g_AnimClass_Array` for class registry/maintenance, but ordinary per-tick AI for revealed anim objects is through the live `LogicClass` object vector (`ObjectClass::Reveal -> FUN_0055BAA0 -> DynamicVector__Insert`; `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` calls `vtable+0x5C` and reloads live count)."
- `docs/research/traces/GARRISON_SHOT_CADENCE_POSTFIX_TRACE.md` and any derivative wording that says first same-pass AI can advance a newly spawned `OccupantAnim` should use the sibling lifecycle replacement: "A same-pass AI visit is possible through the live LogicClass vector, but constructor byte `+0x19C` makes the first `AnimClass::AI` call clear the guard and return before frame advancement."

## Sources

- Ghidra decompiled/read-only: `AnimClass::Constructor @ 0x00421EA0`.
- Ghidra decompiled/read-only: load constructor `0x00422720`.
- Ghidra decompiled/read-only: `AnimClass::~AnimClass @ 0x004228E0` (corrected 2026-05-29 from `0x00422900`; confirmed via `get_function_by_address 0x004228E0`).
- Ghidra decompiled/read-only: `AnimClass::SetOwnerObject @ 0x00424B50`.
- Ghidra decompiled/read-only: `AnimClass::Destroy @ 0x004255B0`.
- Ghidra decompiled/read-only: `ObjectClass::Reveal @ 0x005F4EC0` (corrected 2026-05-29 from `0x005F4F60`; confirmed via `get_function_by_address 0x005F4F60`).
- Ghidra decompiled/read-only: `ObjectClass::UnInit @ 0x005F65F0`.
- Ghidra assembly context: `0x00422058..0x004220A7`, `0x00422AD0..0x00422B0C`, `0x00424B79..0x00424BB6`, `0x0055B608..0x0055B619`, `0x0055BAA0..0x0055BABB`, `0x005F6668..0x005F667D`.
- Prior docs checked: `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`, `docs/research/ANIM_CLASS_GHIDRA_REPORT.md`, `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `docs/research/TICK_AND_ANIMATION_SPEED_GHIDRA_REPORT.md`, `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`.
- Rust scanned: `src/app_building_anim.rs`, `src/sim/components.rs`, `src/app_instances/overlays.rs`.
