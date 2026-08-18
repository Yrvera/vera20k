# AnimClass Attached Owner / Detach Lifecycle - Ghidra Research Report

**Address(es):** `AnimClass::SetOwnerObject @ 0x00424B50`, `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`, `AnimClass::Detach @ 0x00425150`, `AnimClass::Destroy @ 0x004255B0`, `AnimClass::~AnimClass @ 0x004228E0`, `ObjectClass::UnInit @ 0x005F65F0`, `Detach_From_All_Lists @ 0x007258D0`, `Techno/Object detach callback @ 0x00710410`, `ObjectClass::DetachParachute @ 0x005F6DA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active YR `AnimClass` owner attachment, coordinate following, owner-expiry detach, anim-destroy detach order, owner callback fields, remove-listener behavior needed by temporal SQDG, and Rust-facing gaps in `WorldEffect`/descriptor-style bridges.
**Non-Scope:** full constructor caller taxonomy, complete `AnimClass::AI` frame/Next semantics, exact renderer/blitter output, save/load resurrection, full `PointerExpired` field taxonomy for every Techno/Foot subclass, and full building 21-slot active animation refresh.
**Confidence:** High for `SetOwnerObject`, coord following, detach ordering, owner callback fields, and direct caller-family list; Medium for the exact semantic names of owner vtable `+0x17C` and some helper families because names remain inferred from call shape.
**Active in YR:** Yes. The verified paths are reached by standard YR systems including non-building weapon muzzle anims, paradrop `PARACH`, `Behind=BEHIND`, temporal `SQDG`, building/terrain damage attached fires, EMP/psychic/capture visuals, and unit deploy/undeploy special anims.

## Working Notes Gate

Target question: How does active gamemd.exe attach `AnimClass` objects to owners, make them follow, detach them on owner or anim destruction, and which runtime spawn families require that identity rather than a free `WorldEffect` row?
Non-goals: Do not redo constructor row taxonomy, trailer/bouncer/Next lifecycle, draw blitter math, or unrelated pointer-expiry fields except where they prove attachment cleanup.
Evidence needed to mark COMPLETE: decompile plus assembly for `SetOwnerObject`, `GetCoords`, `AnimClass::Detach`, `Destroy`, `Destructor`, `ObjectClass::UnInit`/listener dispatch, owner callback field clears, and at least representative xref/caller evidence for attachment families.
Stop conditions: stop after attached-owner mechanism and Rust handoff are proven, with broader pointer-expiry, save/load, and building slot refresh follow-ups deferred.

## 1. Overview

An owner-attached `AnimClass` is not just a visual row with an owner id. `SetOwnerObject` converts the anim's stored coordinate into an owner-relative offset, stores the owner at `AnimClass+0xCC`, sets owner byte `Object+0x84`, removes/re-submits the anim to display layers, and later `GetCoords` returns `stored_offset + owner.GetCoords`.

Owner expiry is a separate path from ordinary anim expiry. When the owner is removed, `ObjectClass::UnInit` calls `Detach_From_All_Lists`, which dispatches `AnimClass::Detach @ 0x00425150` to live anim objects. That detach path removes the anim from display, calls the owner's cleanup callback, clears `AnimClass+0xCC`, sets `AnimClass+0x19B = 1`, and calls the anim mark/update slot `+0x124(0)`. Ordinary anim destruction instead calls owner callback `+0x60`, then `SetOwnerObject(NULL)`, then sound/uninit.

## 2. Key Offsets / Fields

| Field | Owner | Meaning | Active in YR | Evidence |
|---|---|---|---|---|
| `AnimClass+0x9C/+0xA0/+0xA4` | AnimClass | stored coordinate; when attached it is owner-relative offset | Yes | `SetOwnerObject @ 0x00424C51..0x00424C70`, `GetCoords @ 0x00422BE0` |
| `AnimClass+0xC8` | AnimClass | `AnimTypeClass*`; cleared if the type pointer itself expires | Conditional | `AnimClass::Detach @ 0x004251A3..0x004251AF` |
| `AnimClass+0xCC` | AnimClass | attached owner object pointer | Yes | `SetOwnerObject`, `GetCoords`, `Detach`, `Destroy` |
| `AnimClass+0xFC` | AnimClass | attached surface/cell height copied by several attach producers | Conditional | `TemporalClass::AI @ 0x00629DDE..0x00629DE5`, `ObjectClass::Unlimbo @ 0x005F5B1F..0x005F5B30` |
| `AnimClass+0x100` | AnimClass | instance ZAdjust; some attached producers mutate after attach | Conditional | `CaptureManager @ 0x00471F67..0x00471F71`, `TerrainClass::Catch_Fire @ 0x0071C67A..0x0071C683` |
| `AnimClass+0x17C/+0x180` | AnimClass | object pointers whose expiry destroys the anim | Conditional | `AnimClass::Detach @ 0x004251B1..0x004251E1` |
| `AnimClass+0x195` | AnimClass | loop/lifetime byte; force-stop helper writes zero for matching owner | Conditional | `FUN_00422B80 @ 0x00422B94..0x00422B9F` |
| `AnimClass+0x19B` | AnimClass | owner-expired detach marker set when owner pointer expires | Conditional | `AnimClass::Detach @ 0x0042518D..0x00425196` |
| `Object+0x84` | ObjectClass owner | "has attached anim" marker; also selects parachute fall-rate branch | Yes | `SetOwnerObject @ 0x00424BB6`, `0x00424C30`; parachute report |
| `Object+0x88` | ObjectClass owner | parachute anim pointer, cleared by callback if matching anim expires | Conditional | `ObjectClass::DetachParachute @ 0x005F6DA0`; `0x00710454..0x00710455` |
| `Techno/Object+0x12C` | Techno owner | BEHIND marker pointer, callback-cleared | Conditional | `FUN_00710410 @ 0x00710415..0x00710421`; `FUN_0070F1D0` |
| `Techno/Object+0x130` | Techno owner | unit deploy/undeploy special anim pointer, callback-cleared | Conditional | `FUN_00710410 @ 0x00710443..0x0071044E`; `FUN_00739AC0/FUN_00739CD0` |
| `Techno/Object+0x1D4` | Techno owner | transport/enter anim pointer, callback-cleared | Conditional | `FUN_00710410 @ 0x00710427..0x0071042F`; `BuildingClass::EnterTransport` |
| `Techno/Object+0x2C8` | Techno owner | capture/psychic-style attached anim pointer, callback-cleared | Conditional | `FUN_00710410 @ 0x00710435..0x0071043D`; `CaptureManager`, `PsychicDominator` |

## 3. Core Logic

### 3.1 `SetOwnerObject` attach/detach order

Active in YR: Yes, whenever a caller passes a non-null owner or `AnimClass::Destroy` passes null.

Detach-from-old-owner first:

1. If `AnimClass+0xCC` is non-null, snapshot byte `AnimClass+0x74`; when nonzero remove from display layer (`0x00424B67..0x00424B74`).
2. Scan `g_AnimClass_Array` for any other anim whose `+0xCC` equals this old owner, skipping `this` (`0x00424B79..0x00424BA7`).
3. If no other attached anim remains, call old owner vtable `+0x17C` and write owner `+0x84 = 0` (`0x00424BAB..0x00424BB6`).
4. Read current anim coords through vtable `+0x48`, clear `AnimClass+0xCC`, then write those absolute coords back through vtable `+0x1B4` (`0x00424BBD..0x00424BF0`).
5. If it was display-listed before, submit it again (`0x00424BF6..0x00424C00`).

Attach-to-new-owner second:

1. Read the anim's current absolute coords through vtable `+0x48` (`0x00424C0D..0x00424C22`).
2. Remove from display unconditionally (`0x00424C26..0x00424C2B`).
3. Write owner `+0x84 = 1`, then write `AnimClass+0xCC = new_owner` (`0x00424C30..0x00424C37`).
4. Read new owner coords through owner vtable `+0x48`.
5. Store `anim_absolute - owner_absolute` through anim vtable `+0x1B4` (`0x00424C49..0x00424C70`).
6. Submit to display layer (`0x00424C76..0x00424C7C`).

Important detail: `SetOwnerObject(NULL)` does not set `AnimClass+0x19B`; that byte is set by pointer-expiry detach, not ordinary explicit detach.

### 3.2 Coordinate following

Active in YR: Yes for every attached anim.

`AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0` checks `AnimClass+0xCC`. If non-null it calls base `ObjectClass::GetCoords` for the anim's stored offset, calls owner vtable `+0x48`, and returns component-wise sum:

```text
return anim.StoredCoord + owner.GetCoords()
```

Assembly proof: `0x00422BF1..0x00422C2E` calls base coords and owner coords, then adds X/Y/Z. If `+0xCC` is null, `0x00422C39..0x00422C5C` returns base object coords unchanged.

`AnimClass::GetLayer @ 0x00424CB0` also treats attachment as a draw-layer decision: if `+0xCC != 0`, return layer `2` regardless of `AnimType+0x364`; otherwise return type layer or fallback `3`.

### 3.3 Anim destruction order

Active in YR: Yes for normal animation completion and forced destruction.

`AnimClass::Destroy @ 0x004255B0`:

1. If `AnimClass+0xCC` exists, call owner vtable `+0x60(this_anim)` (`0x004255B6..0x004255C3`). For Techno/Object owners this callback is `FUN_00710410`, which clears known owner-side anim pointer fields.
2. Call `SetOwnerObject(NULL)` (`0x004255C6..0x004255CA`). This may clear owner `+0x84` only if no other live anim shares that owner.
3. Release sound handle, optionally play `StopSound`, then call `ObjectClass::UnInit` (`0x004255CF..0x0042561F`).

`AnimClass::~AnimClass @ 0x004228E0` repeats the owner-share scan if `g_GameActive != 0`, clears `+0xCC`, removes itself from class/listener vectors, releases handles, and finally compacts `g_AnimClass_Array`.

### 3.4 Owner expiry / uninit detach path

Active in YR: Yes for object removal. `ObjectClass::UnInit @ 0x005F65F0` calls `Detach_From_All_Lists @ 0x007258D0` before limbo/conceal, alive clear, and pending-delete append (`0x005F6612..0x005F6625`).

`Detach_From_All_Lists` dispatches pointer-expiry notifications. For live object-like targets it calls registered listeners' vtable `+0x28`; `AnimClass` vtable `+0x28` resolves to `AnimClass::Detach @ 0x00425150`.

`AnimClass::Detach`:

1. Calls inherited `ObjectClass` pointer-expired cleanup first (`0x0042515B..0x0042515F` -> `0x005F5230`).
2. If expired pointer equals `AnimClass+0xCC` and is non-null, remove the anim from display (`0x00425164..0x0042517A`).
3. Call the old owner's vtable `+0x60(this_anim)` while `AnimClass+0xCC` still points at the old owner (`0x0042517F..0x00425188`).
4. Clear `AnimClass+0xCC`, set `AnimClass+0x19B = 1`, and call anim vtable `+0x124(0)`, which resolves to `AnimClass::ProcessCloakMode -> ObjectClass::Mark(0)` (`0x0042518B..0x0042519D`; vtable read `0x007E3478 -> 0x004238B0`).
5. If expired pointer equals `AnimClass+0xC8`, clear the type pointer.
6. If expired pointer equals `AnimClass+0x17C` or `+0x180`, clear `+0x17C` and call vtable `+0xF8` to destroy the anim (`0x004251B1..0x004251E1`).

This proves owner death does not simply "drop the visual when entity disappears"; it sends a pointer-expiry detach event into each anim, clears owner-side pointers, marks the anim, and may let the anim continue ownerless depending on its state.

### 3.5 Owner callback fields

Active in YR: Yes for Techno/Object-derived owners whose vtable `+0x60` points to `FUN_00710410`.

`FUN_00710410` clears only fields whose value equals the anim pointer:

- `+0x12C = 0` when it equals the detached anim.
- `+0x1D4 = 0` when it equals the detached anim.
- `+0x2C8 = 0` when it equals the detached anim.
- `+0x130 = 0` when it equals the detached anim.
- Tail-calls `ObjectClass::DetachParachute @ 0x005F6DA0`, which clears `Object+0x88` when it equals the anim.

This callback is called by both normal anim destruction and owner-expiry detach before `AnimClass+0xCC` is cleared.

### 3.6 Remove listener vector for temporal SQDG

Active in YR: Conditional on temporal attack. `TemporalClass::AI` state 0 creates `SQDG`, stores it at `Temporal+0x44`, and appends the Temporal instance to `g_AnimClass_RemoveListeners` (`0x0062991C..0x0062996F`). `Detach_From_All_Lists` has a specific RTTI-4 branch that iterates `g_AnimClass_RemoveListeners` and calls listener vtable `+0x28` (`0x00725A16..0x00725A47`). Temporal cleanup later removes itself from that vector by find/left-compaction (`0x00629CA1..0x00629CF1`, `0x00629E0E..0x00629E5E`).

Rust therefore needs a listener/removal-notification concept for temporal/attached persistent anims; a free `WorldEffect` has no equivalent.

### 3.7 Attachment producer families verified in this slice

| Family | Attachment evidence | Active in YR |
|---|---|---|
| Non-building weapon muzzle `OccupantAnim`/weapon anim | `TechnoClassFireAtSpawnsBullet @ 0x006FF42D..0x006FF43A` calls `SetOwnerObject` only when `WhatAmI() != 6`; building branch writes `+0x100=-200` and skips attach | Yes / Conditional on weapon anim |
| Paradrop `PARACH` / `PARABOMB` | `ObjectClass::Unlimbo @ 0x005F5AF1..0x005F5AFE` stores owner pointer and calls `SetOwnerObject`; normal infantry uses `Rules+0xBBC` | Yes for normal paradrops |
| Behind marker | `FUN_0070F1D0 @ 0x0070F24C..0x0070F266` stores `Techno+0x12C`, calls `SetOwnerObject` | Yes when `CanBeHidden` and `[General] Behind` present |
| Temporal `SQDG` | `TemporalClass::AI @ 0x00629913..0x0062991C` creates/stores, tail `0x00629DAA..0x00629DC4` attaches to target if owner changed | Yes for Chrono Legionnaire temporal attack |
| Building damage side-effect debris/fire | `BuildingClass::ReceiveDamage @ 0x004428E4..0x004428F4` creates and attaches to building for `Warhead+0x14A` branches | Conditional on warhead/building damage result |
| Terrain catch-fire | `TerrainClass::Catch_Fire @ 0x0071C667..0x0071C675` attaches to terrain, then subtracts `0x14` from `+0x100` | Conditional; caller unresolved in prior doc |
| EMPulse attached sparks | `EMPulseClass::Apply @ 0x004C5876..0x004C5882` calls `SetOwnerObject` for each created anim | Conditional on EMPulse |
| Psychic Dominator per-victim anims | `PsychicDominator::MindControlArea @ 0x0053B306..0x0053B31C` attaches per affected unit; center anim is not owner-attached | Conditional on superweapon |
| CaptureManager capture anim | `CaptureManagerClass::CaptureUnit @ 0x00471F4C..0x00471F71` attaches to captured unit, then may write `+0x100=-1024` for building target case | Conditional on capture |
| Transport/enter anim | `BuildingClass::EnterTransport @ 0x0070FDFE..0x0070FE14` stores owner `+0x1D4`, calls `SetOwnerObject` | Conditional |
| Unit deploy/undeploy anims | `FUN_00739AC0 @ 0x00739BF3..0x00739C05`, `FUN_00739CD0 @ 0x00739DD7..0x00739DE9` store `+0x130`, call `SetOwnerObject` | Conditional on unit deploy animation metadata |

## 4. INI / Data Keys

| Key / source | Stock YR value | Effect | Active in YR | Evidence |
|---|---|---|---|---|
| `[General] Behind` | `BEHIND` | attached marker type for hidden objects | Yes | `rulesmd.ini:562`, `FUN_0070F1D0` |
| `[General] Parachute` | `PARACH` | normal infantry attached chute | Yes | `rulesmd.ini:564`, `ObjectClass::Unlimbo` |
| `[General] BombParachute` | `PARABOMB` | `WhatAmI()==8` attached chute | Conditional | `rulesmd.ini:565`, `ObjectClass::Unlimbo` |
| `SQDG` / `SQDG_*` | squid/temporal visual set | attached temporal grab anim uses `SQDG` lookup; directional SQDG entries exist in art/rules | Conditional | `TemporalClass::AI`, `rulesmd.ini:2236`, `artmd.ini:15786` |
| `DamageFireTypes` | `FIRE01,FIRE02,FIRE03` | building damage-fire slots are separate real `AnimClass*`; not this owner attach path unless side-effect debris uses `Rules+0xB78` | Conditional | prior damage-fire reports; `rulesmd.ini:519` |

No INI key directly toggles `SetOwnerObject`; attachment is a post-constructor callsite decision.

## 5. Integration Points

| Integration | Finding | Active in YR | Evidence |
|---|---|---|---|
| Constructor | Initializes `+0xCC=0`; attachment is always post-constructor | Yes | `AnimClass::Constructor @ 0x00421EA0` |
| Display layers | SetOwnerObject removes/re-submits around coord/owner changes | Yes | `0x00424B6E`, `0x00424BFA`, `0x00424C2B`, `0x00424C7C` |
| Draw layer | Any attached anim returns layer `2` from `GetLayer` | Yes | `0x00424CB0..0x00424CBF` |
| Owner death | `UnInit -> Detach_From_All_Lists -> AnimClass::Detach` before owner limbo/alive clear | Yes | `0x005F6612..0x005F6625`, `0x00425150` |
| Anim death | owner callback `+0x60`, then `SetOwnerObject(NULL)`, then `UnInit` | Yes | `0x004255B6..0x0042561F` |
| Owner-side fields | callback clears `+0x12C/+0x1D4/+0x2C8/+0x130/+0x88` only if equal | Conditional | `0x00710410`, `0x005F6DA0` |
| Temporal listener | Temporal SQDG uses `g_AnimClass_RemoveListeners` for removal notification | Conditional | `0x0062991C..0x0062996F`, `0x00725A16..0x00725A47` |

## 6. Current Rust Implementation Status

Rust has useful constructor-row storage but no native owner-attached object identity:

- `src/sim/components.rs:761..790` stores `AnimClassSpawnDescriptor` row fields, but no owner pointer, owner-relative offset, object identity, listener registration, display layer resubmission, or `+0x19B` detach marker.
- `src/sim/components.rs:817..855` `WorldEffect` is explicitly a temporary fixed-world one-shot. It cannot follow an owner, clear owner-side pointer fields, survive owner expiry as an ownerless marked anim, or dispatch removal listeners.
- `src/sim/components.rs:679..704` `AnimRuntime` captures some lifecycle state for garrison flashes but is not a global object with `SetOwnerObject`/`Detach`.
- `src/app_chute_anim.rs:1..24` models parachute visuals by polling `parachute_state`; it omits the native `PARACH` `AnimClass` pointer at owner `+0x88`, owner `+0x84` marker, callback clear, pointer-expiry detach, and generic anim listener behavior.
- `src/sim/movement/parachute_descent.rs:54..65` begins descent by state flag only; native creates an attached `PARACH` object and attachment marker in `ObjectClass::Unlimbo`.
- `src/sim/movement/teleport_movement.rs:41..77` emits teleport rows through `WorldEffect`; this is acceptable for free `WarpOut` rows only, not temporal `SQDG` or any owner-attached persistent anim.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SetOwnerObject` old-owner detach and new-owner attach | verified | decompile/disasm `0x00424B50` | exact name of owner `+0x17C` semantic |
| Owner-relative coordinate formula | verified | decompile/disasm `0x00422BE0` | none |
| Attached layer override | verified | decompile/disasm `0x00424CB0` | none |
| Anim normal destruction detach order | verified | decompile/disasm `0x004255B0` | none for attach order |
| Anim destructor fallback owner scan | verified | decompile/disasm `0x004228E0` | save/load destructor variants not exhausted |
| Owner expiry dispatch into AnimClass | verified | `ObjectClass::UnInit`, `Detach_From_All_Lists`, `AnimClass::Detach` | full listener roster is prior-doc scope |
| Owner callback field clears | verified | `FUN_00710410`, `ObjectClass::DetachParachute` | exact canonical field names |
| Temporal remove-listener vector | verified | `TemporalClass::AI`, `Detach_From_All_Lists` | listener vtable `+0x28` body of Temporal not named |
| Attachment producer families | verified for direct xrefs | `get_function_xrefs 0x00424B50` plus callsite context/decompiles | exhaustive semantic names for `FUN_006EF610`, unit deploy helpers |
| Generic owner death visual lifetime | touched-not-exhausted | `AnimClass::Detach` proves detached anim may continue ownerless | exact subsequent AI branch using `+0x19B` deferred |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which fields link an anim to an owner? -> `AnimClass+0xCC` stores owner pointer, owner `+0x84` marks any attached anim, and owner-side pointer slots may store specific anim pointers.` (evidence: `0x00424B50`, `0x00710410`)
- `[RESOLVED] OQ-02 - Does attachment convert coords to relative offsets? -> Yes, attach stores `anim_abs - owner_abs`; GetCoords later returns offset + owner coords.` (evidence: `0x00424C49..0x00424C70`, `0x00422BE0`)
- `[RESOLVED] OQ-03 - Does attached anim type layer still control layer? -> No, attached anims return layer 2 before reading `AnimType+0x364`.` (evidence: `0x00424CB0`)
- `[RESOLVED] OQ-04 - What does normal anim destroy do first? -> It calls owner `+0x60(this_anim)`, then `SetOwnerObject(NULL)`, then sound/uninit.` (evidence: `0x004255B6..0x0042561F`)
- `[RESOLVED] OQ-05 - How are owner-side anim pointer fields cleared? -> Techno/Object callback `0x00710410` clears `+0x12C/+0x1D4/+0x2C8/+0x130` and `DetachParachute` clears `+0x88` if equal to the anim.` (evidence: `0x00710410`, `0x005F6DA0`)
- `[RESOLVED] OQ-06 - What happens if the owner expires first? -> `ObjectClass::UnInit` dispatches pointer-expiry before limbo/alive clear; `AnimClass::Detach` removes layer, calls owner callback, clears `+0xCC`, sets `+0x19B=1`, and marks the anim.` (evidence: `0x005F6612..0x005F6625`, `0x00425150`)
- `[RESOLVED] OQ-07 - Is `SetOwnerObject(NULL)` equivalent to owner expiry? -> No; owner-expiry sets `+0x19B=1` and calls mark/update, while explicit detach converts to absolute coords and may re-submit.` (evidence: `0x0042518B..0x0042519D` vs `0x00424BBD..0x00424C00`)
- `[RESOLVED] OQ-08 - Do multiple attached anims keep owner `+0x84` true? -> Yes, detach scans `g_AnimClass_Array` and clears owner `+0x84` only when no other anim has the same owner.` (evidence: `0x00424B79..0x00424BB6`, `0x00422924..0x0042295B`)
- `[RESOLVED] OQ-09 - Which stock/common families attach? -> non-building weapon anims, paradrop chutes, BEHIND, temporal SQDG, building/terrain damage side effects, EMP, Psychic Dominator, CaptureManager, transport, and deploy/undeploy helpers all have direct `SetOwnerObject` xrefs.` (evidence: xrefs to `0x00424B50`)
- `[RESOLVED] OQ-10 - Are trailers owner-attached? -> No explicit owner transfer in trailer constructor row; child may use parent attached coordinates for spawn position but no `SetOwnerObject` call.` (evidence: prior trailer report `0x004242F6..0x0042431D`)
- `[RESOLVED] OQ-11 - Is temporal SQDG just a free world effect? -> No, it is stored at `Temporal+0x44`, registered as remove-listener, and attached to target.` (evidence: `0x00629913..0x0062996F`, `0x00629DAA..0x00629DC4`)
- `[RESOLVED] OQ-12 - Does Rust `WorldEffect` represent this? -> No; current fields are fixed world position/frame/delay/sound only.` (evidence: `src/sim/components.rs:817..855`)
- `[DEFERRED] OQ-13 - What exact AI behavior consumes `AnimClass+0x19B` after owner-expiry detach?` (category: bounded-cost-too-high; reason: this slice proves the write and mark call, but not all subsequent AI consumers; next-step-if-pursued: focused `AnimClass+0x19B` state-consumer investigation)
- `[DEFERRED] OQ-14 - Exact canonical meaning of owner vtable `+0x17C`.` (category: requires-different-system-context; reason: call/no-op vs Techno-specific check is proven but name is not; next-step-if-pursued: vtable-slot census for Object/Techno `+0x17C`)
- `[DEFERRED] OQ-15 - Full building 21-slot active animation refresh under radio/mark update.` (category: requires-different-system-context; reason: building animation slot lifecycle is adjacent and already has separate docs; next-step-if-pursued: update building active anim model)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Attached anim coords are stored owner-relative and `GetCoords` returns owner coords plus stored offset; attachment forces layer 2. | `0x00424C49..0x00424C70`, `0x00422BE0`, `0x00424CB0` | Missing from `WorldEffect`; parachute/BEHIND/temporal surfaces do not have generic owner-relative anim object. | Future generic anim object/runtime; `src/sim/components.rs`, `src/app_chute_anim.rs`, temporal/hidden visual systems | Add an owner-attached anim representation with owner id, relative coord, native layer override, and current absolute coords sampled from owner each draw/tick. | Attached `PARACH`/`BEHIND` follows a moving/falling owner without hardcoded screen lifts and renders on layer 2 regardless of art layer. Proposed test: `attached_anim_getcoords_adds_owner_offset_and_forces_ground_layer`. | Do not model attached anims as fixed `WorldEffect` rows or as render-only polling tied only to owner state. |
| Anim expiry calls owner callback `+0x60`, then `SetOwnerObject(NULL)`; owner byte `+0x84` clears only when no other live anim shares owner. | `0x004255B6..0x004255CA`, `0x00424B79..0x00424BB6`, `0x00710410` | Missing generic owner-side pointer slots and shared-owner scan. | Future anim pool/lifecycle; entity visual attachment fields | Track attached anim ownership globally enough to clear owner-side references only for matching anim and clear "has attached anim" only on last attached anim. | Owner with two attached anims destroys one; owner marker remains true and remaining anim still follows. Proposed test: `attached_anim_owner_marker_clears_only_after_last_attached_anim`. | Do not clear owner attachment state when any single attached anim expires if another still references owner. |
| Owner expiry detaches attached anims through pointer-expiry, not by simply deleting render effect: remove layer, callback clear, `+0xCC=0`, `+0x19B=1`, mark/update. | `ObjectClass::UnInit @ 0x005F6612..0x005F6625`, `AnimClass::Detach @ 0x00425164..0x0042519D` | Rust despawn/render polling removes effects when owner state disappears; no `AnimClass::Detach` equivalent. | `Simulation::despawn_entity` / future pointer-expiry listener pass; generic anim runtime | Dispatch owner-expired notifications to attached anims before owner removal finalization, allowing native detach/mark semantics and owner-side field clears. | Destroy owner with attached `BEHIND` or `PARACH`: owner pointer slots clear, anim is marked detached, and other pointer-expiry listeners run before owner alive clear. Proposed test: `owner_uninit_dispatches_attached_anim_detach_before_alive_clear`. | Do not just retain/drop effects based on owner existence; native has an ordered listener event. |
| Temporal SQDG uses a stored `AnimClass*` plus `g_AnimClass_RemoveListeners`; it is not a free sparkle row. | `0x00629913..0x0062996F`, `0x00629DAA..0x00629DC4`, `0x00725A16..0x00725A47` | Missing temporal attached anim and listener surface; teleport code emits only free `WorldEffect` rows. | temporal weapon/warp-attach system; generic anim listener registry | Model persistent attached temporal visual with removal listener registration and cleanup when anim or target expires. | CLEG temporal attack creates one attached SQDG on target; target death or anim expiry updates Temporal state through listener path. Proposed test: `temporal_sqdg_registers_anim_remove_listener_and_detaches_on_target_expiry`. | Do not reuse teleport `WarpOut`/fixed `WorldEffect` for SQDG. |

## 10. Negative Facts / Do Not Do

- Do not treat `AnimClassSpawnDescriptor` as equivalent to native `AnimClass`; it lacks owner pointer, owner-relative coords, global identity, listener registration, owner callback fields, `+0x19B`, and display re-submit behavior.
- Do not implement `SetOwnerObject(NULL)` and owner-expiry detach as the same path; owner expiry sets `+0x19B` and calls mark/update, explicit detach does coordinate conversion and possible re-submit.
- Do not clear owner `+0x84` while another anim still has the same `+0xCC`; native scans `g_AnimClass_Array`.
- Do not copy parent owner to trailer children; trailer children use parent coords but no `SetOwnerObject` call.
- Do not use `WorldEffect` for temporal `SQDG`, `PARACH`, `BEHIND`, non-building muzzle anims, or other owner-attached persistent visuals.

## 11. Stale Docs / Replacement Wording

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ANIM_CLASS_GHIDRA_REPORT.md`: replace "When attached, the anim's position follows the owner's position" with: "When `SetOwnerObject(owner)` attaches an anim, gamemd removes the anim from display, sets owner `+0x84`, stores owner at `AnimClass+0xCC`, rewrites the anim coordinate to `anim_abs - owner_abs`, and re-submits it. `AnimClass::GetCoords @ 0x00422BE0` later returns `stored_offset + owner.GetCoords`; attached anims also force `GetLayer` to return layer `2`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`: replace "chute disappears on landing/death" shorthand with: "Landing writes `Anim+0x195=0`; anim cleanup calls owner `+0x60`/`SetOwnerObject(NULL)` and clears owner `+0x88` through the callback. Owner death uses the separate pointer-expiry path `ObjectClass::UnInit -> Detach_From_All_Lists -> AnimClass::Detach`, which clears `Anim+0xCC`, sets `Anim+0x19B=1`, and marks the anim."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md`: replace any wording that equates temporal visual rows with generic teleport `WorldEffect` rows with: "Temporal `SQDG` is a stored, owner-attached `AnimClass` with remove-listener registration; it requires `SetOwnerObject`/listener semantics and is not representable as a fixed-position `WorldEffect`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/traces/NON_HARVESTER_SELF_TELEPORT_WARPOUT_ROWS_TRACE_20260528.md`: add after the descriptor gap: "The descriptor bridge is valid only for free constructor rows. Owner-attached producers require `SetOwnerObject` semantics: owner-relative coordinate storage, owner pointer callbacks, owner-expiry detach, shared-owner marker scan, and remove-listener dispatch."

## Sources

- Ghidra read-only decompile/disassembly: `AnimClass::SetOwnerObject @ 0x00424B50`, `AnimClass::GetCoords_WithOwnerOffset @ 0x00422BE0`, `AnimClass::GetLayer @ 0x00424CB0`, `AnimClass::Detach @ 0x00425150`, `AnimClass::Destroy @ 0x004255B0`, `AnimClass::~AnimClass @ 0x004228E0`, `FUN_00422B80 @ 0x00422B80`, `ObjectClass::UnInit @ 0x005F65F0`, `Detach_From_All_Lists @ 0x007258D0`, `FUN_00710410 @ 0x00710410`, `ObjectClass::DetachParachute @ 0x005F6DA0`, `AnimClass::ProcessCloakMode @ 0x004238B0`.
- Ghidra xrefs/callsite context for `AnimClass::SetOwnerObject @ 0x00424B50`: `TechnoClassFireAtSpawnsBullet`, `ObjectClass::Unlimbo`, `FUN_0070F1D0`, `TemporalClass::AI`, `BuildingClass::ReceiveDamage`, `TerrainClass::Catch_Fire`, `EMPulseClass::Apply`, `PsychicDominator::MindControlArea`, `CaptureManagerClass::CaptureUnit`, `BuildingClass::EnterTransport`, `FUN_00739AC0`, `FUN_00739CD0`, `FUN_006EF610`.
- Prior reports referenced: `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`, `ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md`, `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`, `BEHIND_HIDDEN_OBJECT_VISUAL_PATH_GHIDRA_REPORT.md`, `ANIMCLASS_AI_TRAILER_NEXT_INTERACTION_GHIDRA_REPORT.md`, `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/components.rs`, `src/app_chute_anim.rs`, `src/sim/movement/parachute_descent.rs`, `src/sim/movement/teleport_movement.rs`.
