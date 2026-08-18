# UnitClass +0x6C8 Convoy Link Lifecycle - Reswarm Ghidra Research Report

**Address(es):** `0x007463A0` (`UnitClass` owner-change wrapper), `0x00743270` (`ScenarioClass::Read_Units_Section`), `0x007353C0` (`UnitClass` constructor), `0x00744470` (UnitClass load-wrapper body; current Ghidra label is stale), `0x00744640` (`FootClass__Save_Convoy_State`), `0x007446E0` (`FootClass__Clear_Convoy_On_Delete`), `0x004AFE00` (`DriveLocomotionClass::Stop_Moving`), `0x004B0F20` (`DriveLocomotionClass::Process_Drive_Track`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact lifecycle of `UnitClass+0x6C8` and its directly participating state (`Unit+0x6D0`, save/load swizzle, scenario population, pointer-expiration clear, drive-locomotor readers, and `UnitClass::ChangeOwner` propagation).
**Non-Scope:** full team-script convoy movement, full DriveLocomotion pathfinding, campaign-map inventory of every `[Units]` convoy index, and runtime debugger watchpoints.
**Confidence:** High for binary field lifecycle and owner-transfer ordering; Medium for stock-map frequency because visible repo/INI data does not include campaign map contents embedded in MIX files.
**Active in YR:** Conditional. The code is live in standard YR: `[Units]` loading runs from `ScenarioClass__Full_Init`, Unit vtable `+0x3D4` dispatch is live for owner changes, and DriveLocomotion readers are live. Actual non-null `+0x6C8` state requires scenario save/map data or savegame data that links units; stock INI files do not set `IsTrain=`, so the train-gated stop propagation is normally dormant unless a map/mod sets that key.

## 1. Overview

`UnitClass+0x6C8` is a `UnitClass*`/foot-derived object pointer to the next unit in a singly linked convoy chain. It is initialized to null, populated primarily from a scenario `[Units]` convoy-next index after all units have been created, serialized and swizzle-registered on load, nulled when the referenced object expires, read by DriveLocomotion convoy logic, and propagated during `UnitClass` owner transfer before the parent `FootClass::ChangeOwner` runs.

The directly paired byte is `Unit+0x6D0`: native code sets it on the linked follower and clears it on the parent at owner-transfer entry. In `DriveLocomotionClass::Stop_Moving`, `+0x6D0` gates whether this unit should propagate a stop to its chain. This is not a separate Rust-style movement group id; it is a persisted per-unit byte tied to the native pointer chain.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Type | Verified purpose in this slice | Evidence | Active in YR |
|---:|---|---|---|---|---|
| `+0x6C4` | `UnitClass` | `UnitTypeClass*` | unit type pointer; saved beside convoy link and swizzle-registered on load | ctor `0x007353E6`; save `0x0074464E..0x00744673`; load wrapper `0x007445C7..0x007445D3` | Yes |
| `+0x6C8` | `UnitClass` | `UnitClass*` / object pointer | next convoy-linked unit pointer | ctor `0x007353EC`; scenario link `0x0074368C`; save `0x00744678`; load swizzle `0x007445D8`; owner transfer `0x007463B1`; clear on expire `0x007446F3` | Conditional on link data |
| `+0x6CC` | `UnitClass` | `int` | not part of the convoy pointer lifecycle in this slice; serialized by the same save helper; prior UnitClass docs identify it as flag-carrier house index | ctor `0x007353F2`; save `0x00744694` | Conditional |
| `+0x6D0` | `UnitClass` | byte | convoy follower / stop-propagation gate byte; set on linked unit during scenario link and owner-transfer propagation; cleared on current unit before owner-transfer propagation | ctor `0x007353F8`; scenario `0x00743692`; owner transfer `0x007463B9`, `0x007463D5`; save `0x007446A2`; stop gate `0x004AFE43` | Conditional |
| vtable `+0x28` | `UnitClass` | virtual | pointer-expired hook that clears `+0x6C8` if the expired object is the linked unit | data xref `0x007F5C98` -> `0x007446E0` | Yes |
| vtable `+0x34` | `UnitClass` | virtual | checksum/save helper serializing convoy-related fields | data xref `0x007F5CA4` -> `0x00744640` | Yes |
| vtable `+0x3D4` | `UnitClass` | virtual | owner-transfer wrapper; propagates owner to `+0x6C8` target before parent transfer | data xref `0x007F6044` -> `0x007463A0` | Yes when unit owner changes |
| `TechnoType+0xC94` | `TechnoTypeClass` | byte | `IsTrain`; gates `DriveLocomotionClass::Stop_Moving` traversal of `+0x6C8` | `0x00712277..0x00712284`; string `0x008444BC` = `IsTrain`; stop gate `0x004AFE36` | Conditional; stock visible INI has no assignments |
| `UnitType+0xE0C` | `UnitTypeClass` | byte | `Passive`; when set, skips the acceleration/deceleration branch that also contains convoy speed propagation in `Process_Drive_Track` | `UnitTypeClass__ReadINI @ 0x00747620`; `Passive` read/write in decompile; `0x004B0F20` reads owner type `+0xE0C` | Conditional; stock visible INI has no assignments |

## 3. Core Logic

### 3.1 Constructor default

`UnitClass::Constructor @ 0x007353C0` sets the Unit tail fields in a compact block:

| Address | Operation | Meaning |
|---|---|---|
| `0x007353E0` | `MOV dword ptr [ESI + 0x6C0], -1` | Unit sentinel/unused field |
| `0x007353E6` | `MOV dword ptr [ESI + 0x6C4], ECX` | store UnitType pointer |
| `0x007353EC` | `MOV dword ptr [ESI + 0x6C8], EBX` | `next_in_convoy = NULL` |
| `0x007353F2` | `MOV dword ptr [ESI + 0x6CC], -1` | sibling serialized int default |
| `0x007353F8` | `MOV byte ptr [ESI + 0x6D0], BL` | follower byte default false |
| `0x007353FE..0x00735404` | clears `+0x6D1/+0x6D2` | not the convoy pointer; serialized by same helper |

Tiny detail: the default link is a null pointer, not `-1`; the sibling `+0x6CC` defaults to `-1`. A future Rust save/load model must not collapse these into one "convoy id" field with a single sentinel value.

### 3.2 Scenario `[Units]` population

`ScenarioClass::Read_Units_Section @ 0x00743270` first creates every unit and stores the optional convoy-next index from the unit line into a temporary integer vector. After unit creation finishes, it makes a second pass over the global unit array:

| Address | Operation | Meaning |
|---|---|---|
| `0x00743664..0x0074367D` | load `g_UnitClass_Array_Count`, temp index array, and `g_UnitClass_Array` | starts the post-create link pass |
| `0x00743680..0x00743687` | compare stored index with `-1` and unit count | invalid index path |
| `0x00743689` | `MOV EAX,dword ptr [ESI + EAX*4]` | resolve target unit pointer from the index |
| `0x0074368C` | `MOV dword ptr [EDX + 0x6C8], EAX` | owner unit's `next_in_convoy = target` |
| `0x00743692` | `MOV byte ptr [EAX + 0x6D0], 1` | target unit becomes follower |
| `0x0074369B` | `MOV dword ptr [EDX + 0x6C8], EBX` | invalid/`-1` index clears the link |

The link pass is reached from standard scenario initialization: `ScenarioClass__Full_Init @ 0x00687AA7` calls `ScenarioClass__Read_Units_Section`. The code is live for all map loads, but the non-null link state is conditional on scenario data providing a valid next-unit index.

Tiny details:

- The link is resolved by global unit-array index, not stable object id, type, house, or map coordinate.
- The range check rejects `0xFFFFFFFF` and any index `>= g_UnitClass_Array_Count`.
- The follower byte is written on the target unit, not the source unit.
- The invalid path clears only the source `+0x6C8`; it does not clear any previously set follower byte on another unit in the same second pass.
- The temporary convoy index is read only after unit creation, so forward references are supported as long as the target index exists by the second pass.

### 3.3 Save/load and swizzle

`FootClass__Save_Convoy_State @ 0x00744640` serializes the unit type, the convoy target by the target object's abstract id, then sibling fields:

| Address | Operation | Meaning |
|---|---|---|
| `0x0074464E..0x00744673` | serialize `+0x6C4` type id/name data | unit type identity |
| `0x00744678` | read `+0x6C8` | convoy pointer |
| `0x0074467E..0x00744680` | null check | null link writes no target abstract id at this point |
| `0x00744682..0x0074468F` | call linked object's secondary-vtable `+0x10`, then `FUN_004A1D50` | writes target abstract id/name for linked unit |
| `0x00744694..0x0074469D` | serialize `+0x6CC` | sibling serialized int |
| `0x007446A2..0x007446D5` | serialize `+0x6D0`, `+0x6D1`, `+0x6D2`, `+0x687` bytes | persisted flags |

The load wrapper body at `0x00744470` calls `FootClass__Load`, restores UnitClass vtables, then swizzle-registers the two pointer slots:

| Address | Operation | Meaning |
|---|---|---|
| `0x007445C7..0x007445D3` | `FUN_006CF240(&DAT_00B0C110, this+0x6C4)` | register UnitType pointer slot |
| `0x007445D8..0x007445E4` | `FUN_006CF240(&DAT_00B0C110, this+0x6C8)` | register convoy pointer slot |
| `0x007445E9..0x007445EB` | clear `+0x6DC` | unrelated Unit runtime field |

Tiny detail: load does not rebuild the convoy chain by re-reading scenario indices. The loaded pointer slot is registered with the global swizzler. This matches the broader save/load model from recent lifecycle research: pointer fields are loaded then fixed by the swizzle pass.

### 3.4 Pointer-expiration clear

`FootClass__Clear_Convoy_On_Delete @ 0x007446E0` is bound in the Unit vtable (`0x007F5C98`). It first delegates to `FootClass__PointerExpired`, then clears the unit-tail pointers only when they equal the expired pointer:

| Address | Operation | Meaning |
|---|---|---|
| `0x007446EE` | call `FootClass__PointerExpired(expired, flags)` | parent cleanup first |
| `0x007446F3` | read `this+0x6C8` | convoy target pointer |
| `0x007446FB..0x007446FF` | compare to expired pointer, write null if equal | clear broken chain |
| `0x00744705..0x0074470D` | same check for `+0x6C4` | type pointer safety cleanup |

Negative finding: this function does not walk all predecessors in the global unit array and does not clear `+0x6D0` on the expired follower. It only clears this object's `+0x6C8` if that exact pointer expired. Any follower-byte repair depends on other paths, not this leaf.

### 3.5 DriveLocomotion stop propagation

`DriveLocomotionClass::Stop_Moving @ 0x004AFE00` walks the convoy chain only when all gates pass:

1. The locomotor destination/head-to coordinate at `Drive+0x3C..0x44` is not the global null coord.
2. The owner object's type has `TechnoType+0xC94` (`IsTrain`) set.
3. The owner object's `+0x6D0` byte is zero.
4. The owner object's `+0x6C8` pointer is non-null.

Assembly evidence:

| Address | Operation | Meaning |
|---|---|---|
| `0x004AFE36` | `MOV CL, byte ptr [EAX + 0xC94]` | read `IsTrain` |
| `0x004AFE43` | `MOV CL, byte ptr [EAX + 0x6D0]` | follower-byte gate |
| `0x004AFE4D` | `MOV ESI, dword ptr [EAX + 0x6C8]` | first linked unit |
| `0x004AFE57..0x004AFE74` | read follower locomotor `+0x674`, assert if null, call vtable `+0x48` | call follower `Stop_Moving` |
| `0x004AFE77` | `MOV ESI, dword ptr [ESI + 0x6C8]` | advance along chain |
| `0x004AFE81` | `CMP ESI, dword ptr [ESI + 0x6C8]` | loop guard: stop if a unit points to itself |

Tiny detail: the self-cycle guard is not a visited-set. A two-unit cycle would not be broken by this exact check. Native relies on scenario/link construction avoiding such cycles except self-link.

### 3.6 DriveLocomotion speed propagation

`DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` contains the other live reader. In the acceleration branch, after computing and applying the owner's speed fraction, it checks that the owner object reports abstract type `1` (UnitClass) and then walks `+0x6C8`:

| Address | Operation | Meaning |
|---|---|---|
| `0x004B121D..0x004B1220` | call owner vtable `+0x2C`, compare result with `1` | only UnitClass owners enter this chain propagation |
| `0x004B1228` | `MOV ESI, dword ptr [EAX + 0x6C8]` | first linked unit |
| `0x004B1232..0x004B1247` | call follower vtable `+0x544` with owner `+0x578` | propagate leader's speed fraction |
| `0x004B124D` | `MOV ESI, dword ptr [ESI + 0x6C8]` | advance |
| `0x004B1257` | `CMP ESI, dword ptr [ESI + 0x6C8]` | same self-cycle guard |

Important correction to older convoy prose: the `== 1` comparison in this block is not a mission-id check. It is the object's abstract type from virtual `+0x2C`; UnitClass's `What_Am_I` returns `1`. The related early branch reads `UnitType+0xE0C`, which `UnitTypeClass__ReadINI @ 0x00747620` maps to `Passive=`, not `IsTrain=`.

### 3.7 Owner-transfer propagation

`UnitClass__Transfer_Convoy_On_Owner_Change @ 0x007463A0` is the UnitClass virtual `+0x3D4` owner-transfer wrapper:

| Address | Operation | Meaning |
|---|---|---|
| `0x007463A8` | compare `newOwner` with `this->Owner` (`+0x21C`) | no-op guard |
| `0x007463B1` | `MOV ESI, dword ptr [EDI + 0x6C8]` | save linked unit pointer |
| `0x007463B9` | `MOV byte ptr [EDI + 0x6D0], 0` | clear current unit's follower byte before propagation |
| `0x007463C9` | `CALL dword ptr [EAX + 0x3D4]` with linked unit as `ECX`, args `(newOwner, 1)` | recursively transfer linked unit through its concrete owner-transfer wrapper |
| `0x007463CF` | `MOV dword ptr [EDI + 0x6C8], ESI` | restore/preserve original link after recursive call |
| `0x007463D5` | `MOV byte ptr [ESI + 0x6D0], 1` | reassert follower byte on linked unit |
| `0x007463E1` | `CALL FootClass__ChangeOwner(newOwner, 1)` | only now transfer this unit through the parent wrapper |
| `0x007463ED` | `XOR AL, AL` | same-owner no-op returns false |

Material consequence: owner transfer propagates from the current unit to `next_in_convoy` before the current unit itself runs `FootClass::ChangeOwner`. The link is preserved even if the linked unit's transfer path mutates its own state. This is a concrete pre-parent side effect and cannot be matched by a raw owner write.

Edge detail: there is no explicit null guard around the restored link after the recursive transfer because the restore happens only inside the `ESI != 0` branch. There is also no visited-set recursion guard here; malformed cyclic convoy chains can recurse through virtual `+0x3D4`.

## 4. INI Keys

| Key | Scope | Default / visible stock data | Binary effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|---|
| `IsTrain=` | `TechnoTypeClass+0xC94` | default false; no assignments found in visible repo `ini/rules.ini` or `ini/rulesmd.ini` | gates `DriveLocomotionClass::Stop_Moving` traversal of `Unit+0x6C8`; also broader train pass-through in movement docs | `0x00712277..0x00712284`; string memory `0x008444BC`; stop read `0x004AFE36` | Conditional; code live, stock visible data does not enable |
| `Passive=` | `UnitTypeClass+0xE0C` | default false; no assignments found in visible repo `ini/rules.ini` or `ini/rulesmd.ini` | when set, skips the acceleration/deceleration branch that contains chain speed propagation | `UnitTypeClass__ReadINI @ 0x00747620`; speed branch `0x004B0F20` decompile | Conditional |

No INI key directly creates `Unit+0x6C8`. The link is scenario/save data, not a rules key.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Scenario load | `ScenarioClass__Full_Init` calls `[Units]` reader; reader resolves convoy-next indices after all units are created | xref `0x00687AA7 -> 0x00743270`; link pass `0x00743664..0x007436AC` | Yes |
| Save/load | save serializes linked target id if non-null; load registers `+0x6C8` pointer slot with swizzler | `0x00744640`; `0x00744470` | Yes for saves with UnitClass |
| Pointer expiration | Unit vtable hook clears this unit's `+0x6C8` when the linked pointer expires | data xref `0x007F5C98`; body `0x007446E0` | Yes |
| Stop propagation | train-gated drive stop walks chain and calls each follower locomotor `Stop_Moving` | `0x004AFE00`; vtable data xref for Drive stop `0x007E7EF8` | Conditional on `IsTrain` and link |
| Speed propagation | accelerating UnitClass owner can propagate `+0x578` speed fraction down `+0x6C8` chain | `0x004B121D..0x004B125D`; callers from `DriveLocomotionClass::Process @ 0x004B0576`, `0x004B0AAA` | Conditional on drive movement and link |
| Owner transfer | Unit owner-change wrapper recursively transfers linked unit before parent transfer | data xref `0x007F6044`; body `0x007463A0` | Yes when a linked unit changes owner |
| Team convoy clear | separate helper walks `TeamClass+0x54` members via `+0x5D8`, not `Unit+0x6C8` | `TechnoClass__Clear_Convoy_Chain @ 0x006EC3A0`; xrefs `0x004B2EB6`, `0x006A2506`, `0x00516861` | Yes but separate system |

## 6. Current Rust Implementation Status

Rust has no confirmed native-equivalent `Unit+0x6C8` pointer-chain state.

| Rust surface | Current behavior observed | Delta |
|---|---|---|
| `src/sim/game_entity.rs` | `GameEntity` has owner, movement, navigation, locomotor, and many Unit-like fields, but no convoy next pointer / follower byte equivalent was found | missing native convoy chain state |
| `src/sim/components.rs` | `MovementTarget` has `group_id: Option<u32>` for command-level formation speed sync | mismatch: group id is command/runtime grouping, not the native persisted `Unit+0x6C8` pointer chain |
| `src/sim/world/world_commands.rs` | move command stamps `MovementTarget.group_id` onto selected units | mismatch for scenario/save convoy chains; does not create native linked list or follower byte |
| `src/sim/movement/movement_tick.rs` | `sync_formation_speeds` caps grouped movement to slowest member speed after the movement loop | mismatch: native speed propagation pushes leader `+0x578` down `+0x6C8` in DriveLocomotion's track processing, and stop propagation is train-gated |
| owner-transfer surfaces | prior reports found direct owner writes and no concrete virtual owner-transfer API | missing propagation from current unit to linked `+0x6C8` unit before parent owner transfer |
| map/scenario unit loading | no repo map files found via `rg --files -g '*.map' -g '*.yrm' -g '*.mpr'`; current map entity loading was not deeply audited here | unchecked for parsing convoy-next unit indices if map data exposes them |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass+0x6C8` constructor default | verified | `0x007353EC` | none |
| `Unit+0x6D0` constructor default | verified | `0x007353F8` | none |
| Scenario `[Units]` population / invalid clear | verified | `0x00743270`, `0x00743664..0x007436AC`; xref `0x00687AA7` | embedded retail campaign-map frequency not inventoried |
| Save serialization | verified | `0x00744640`, especially `0x00744678..0x0074468F` | exact stream record names outside scope |
| Load swizzle registration | verified | `0x00744470`, especially `0x007445C7..0x007445E4` | current Ghidra function label appears stale; behavior verified |
| Pointer-expiration clear | verified | `0x007446E0`, data xref `0x007F5C98` | no global predecessor repair exists in this leaf |
| Drive stop propagation | verified | `0x004AFE00`; assembly context `0x004AFE36..0x004AFE87` | no live stock sample with `IsTrain=yes` in visible INI |
| Drive speed propagation | verified | `0x004B0F20`; assembly context `0x004B121D..0x004B125D` | full DriveLocomotion branch behavior outside this slice |
| Owner-transfer propagation | verified | `0x007463A0`, data xref `0x007F6044` | malformed cyclic convoy behavior not runtime-tested |
| `IsTrain=` parser/default | verified | `0x00712277..0x00712284`; `0x008444BC` string; repo INI grep no assignments | embedded MIX/map overrides not scanned |
| `Passive=` parser/default | verified | `UnitTypeClass__ReadINI @ 0x00747620`; repo INI grep no assignments | embedded overrides not scanned |
| Team convoy clear vs Unit chain | verified-negative for `+0x6C8` | `0x006EC3A0`; xrefs at `0x004B2EB6`, `0x006A2506`, `0x00516861` | full TeamClass convoy movement remains separate |
| Binary displacement scan for `0x6C8` / `0x6D0` | verified as navigation aid | `search_byte_patterns c8 06 00 00`, `d0 06 00 00`; verified candidate functions above | unrelated class offsets with same displacement were not individually documented |
| Rust current state scan | verified for named surfaces | `rg group_id`, `rg convoy`, file reads of `game_entity.rs`, `components.rs`, `world_commands.rs`, `movement_tick.rs` | no Rust edited |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What is `Unit+0x6C8`? -> A nullable pointer to the next unit/object in a convoy chain, default null, populated from scenario/save data.` (evidence: `0x007353EC`, `0x0074368C`, `0x007445D8`, `0x00744678`)
- `[RESOLVED] OQ-02 - What field directly participates with `+0x6C8`? -> `Unit+0x6D0` is the follower/stop-propagation byte set on linked target and cleared on current unit during owner transfer.` (evidence: `0x00743692`, `0x007463B9`, `0x007463D5`, `0x004AFE43`)
- `[RESOLVED] OQ-03 - When is `+0x6C8` initialized? -> Constructor sets it to null before vtable setup and before global unit-array insertion completes.` (evidence: `0x007353EC`)
- `[RESOLVED] OQ-04 - How does map/scenario load populate it? -> `[Units]` reader stores a convoy-next index while parsing, then resolves index to pointer in a second pass over `g_UnitClass_Array`.` (evidence: `0x00743527..0x00743575`, `0x00743664..0x007436AC`)
- `[RESOLVED] OQ-05 - What invalid convoy-next index does native use? -> `0xFFFFFFFF` or any index `>= unit count` clears source `+0x6C8`.` (evidence: `0x00743680..0x0074369B`)
- `[RESOLVED] OQ-06 - Does scenario population set the follower byte on source or target? -> Target: after resolving `unit_array[index]`, native writes target `+0x6D0 = 1`.` (evidence: `0x00743689..0x00743692`)
- `[RESOLVED] OQ-07 - How is `+0x6C8` saved? -> If non-null, native serializes the linked object's abstract id/name through the linked object's secondary vtable `+0x10`.` (evidence: `0x00744678..0x0074468F`)
- `[RESOLVED] OQ-08 - How is `+0x6C8` loaded? -> The Unit load wrapper registers the `this+0x6C8` pointer slot with `FUN_006CF240` for swizzle fixup.` (evidence: `0x007445D8..0x007445E4`)
- `[RESOLVED] OQ-09 - How is the link cleared on deletion? -> Unit vtable pointer-expired hook clears this object's `+0x6C8` if it equals the expired pointer; it does not walk all units.` (evidence: `0x007446E0`, data xref `0x007F5C98`)
- `[RESOLVED] OQ-10 - Does `Stop_Moving` walk the chain? -> Yes, only when destination is non-null, type `IsTrain` is true, `+0x6D0` is false, and `+0x6C8` is non-null.` (evidence: `0x004AFE00`, `0x004AFE36..0x004AFE55`)
- `[RESOLVED] OQ-11 - Does speed propagation walk the chain? -> Yes, in DriveLocomotion's accelerating UnitClass branch, it calls follower vtable `+0x544` with owner `+0x578`, advancing via `+0x6C8`.` (evidence: `0x004B121D..0x004B125D`)
- `[RESOLVED] OQ-12 - Is the `== 1` gate in Process_Drive_Track a mission id? -> No; it is `What_Am_I`/abstract type. UnitClass returns `1`.` (evidence: `0x004B121D..0x004B1220`; `UnitClass__What_Am_I @ 0x00746E20` from UnitClass vtable report)
- `[RESOLVED] OQ-13 - What does owner transfer do with the link? -> Recursively calls linked unit's virtual `+0x3D4(newOwner,1)`, restores current `+0x6C8`, sets linked `+0x6D0=1`, then calls `FootClass::ChangeOwner` on current unit.` (evidence: `0x007463A0`)
- `[RESOLVED] OQ-14 - Does owner transfer guard against cycles? -> No explicit visited-set or cycle guard is present in `0x007463A0`; malformed cycles would recurse through virtual calls.` (evidence: `0x007463A0` disassembly)
- `[RESOLVED] OQ-15 - What gates train stop propagation? -> `TechnoType+0xC94`, parsed from `IsTrain=` and default false; visible repo INI has no assignments.` (evidence: `0x00712277..0x00712284`, `0x008444BC`, `rg IsTrain= ini`)
- `[RESOLVED] OQ-16 - What is `UnitType+0xE0C` in the speed branch? -> `Passive=`, not `IsTrain`; visible repo INI has no assignments.` (evidence: `UnitTypeClass__ReadINI @ 0x00747620`, `rg Passive= ini`)
- `[RESOLVED] OQ-17 - Is TeamClass convoy movement the same as `Unit+0x6C8`? -> No; `TechnoClass__Clear_Convoy_Chain` walks `TeamClass+0x54` members via member `+0x5D8` and sets member `+0x688`.` (evidence: `0x006EC3A0`)
- `[RESOLVED] OQ-18 - Does current Rust have equivalent state? -> No equivalent next-in-convoy pointer/follower byte was found; Rust has command `group_id` and speed sync instead.` (evidence: `src/sim/components.rs`, `src/sim/world/world_commands.rs`, `src/sim/movement/movement_tick.rs`)
- `[DEFERRED] OQ-19 - Which embedded retail campaign maps contain non-null convoy-next indices?` (category: `requires-different-system-context`; reason: map extraction from MIX/campaign inventory is outside this bounded binary lifecycle slice; next-step-if-pursued: run a map corpus extractor and parse `[Units]` field counts/convoy indices)
- `[DEFERRED] OQ-20 - What does a malformed cyclic chain do at runtime?` (category: `needs-runtime-debugger`; reason: binary lacks a visited-set in owner-transfer and only self-cycle guards in drive traversal; next-step-if-pursued: construct a debug map/save with cycle and watch recursion/traversal behavior)

The two deferred items do not block the lifecycle contract: binary population, clear, save/load, movement readers, and owner-transfer propagation for well-formed chains are exhausted.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Unit convoy link is a persisted pointer chain: `Unit+0x6C8 = next unit`, default null, follower byte on target at `+0x6D0 = 1`. | `0x007353EC`, `0x0074368C`, `0x00743692` | missing | `src/sim/game_entity.rs`; scenario/map unit loader surface | Add native-equivalent per-unit convoy link state and follower byte semantics, not only a movement group id. | Load a map/save with unit A linked to unit B: A stores B as next, B follower byte/equivalent is set, invalid index clears A link. | Do not model native convoy as unordered group membership only. |
| Scenario loader resolves convoy-next indices in a second pass after all units are created. | `0x00743270`, `0x00743664..0x007436AC` | unchecked/missing | map scenario `[Units]` parser and spawn pipeline | Preserve creation-first, link-second order; support forward references and reject `-1` / out-of-range. | A `[Units]` entry links to a later unit index; after load the pointer is valid and active for owner transfer. | Do not resolve by type/name/position while parsing the source unit line. |
| Save/load uses pointer serialization plus swizzle registration for `+0x6C8`; it does not rebuild links from scenario indices after load. | `0x00744640`; `0x007445D8..0x007445E4` | missing/unchecked | save/load snapshot model | Preserve convoy links in save state and restore by stable object reference/swizzle-equivalent. | Save a linked convoy, load it, then owner-change the leader: linked unit transfers first. | Do not discard links on save/load because movement group ids are transient. |
| Pointer-expiration hook clears this object's link if the target pointer expires, after parent pointer-expired cleanup. | `0x007446E0`, data xref `0x007F5C98` | missing | entity despawn / reference invalidation | When a linked target is deleted, predecessors must clear `next_in_convoy` through a pointer-expired style pass. | Destroy follower B, then stop/owner-change leader A: A no longer dereferences B. | Do not only remove B from EntityStore and leave stale stable ids in convoy links. |
| `UnitClass::ChangeOwner` propagates ownership to the linked unit before the current unit calls `FootClass::ChangeOwner`; it restores the link and reasserts linked `+0x6D0`. | `0x007463A0`, data xref `0x007F6044` | missing; prior reports found direct owner writes | future owner-transfer API; mind control / script owner transfer / unit capture paths | Unit owner transfer must dispatch concrete Unit wrapper and recursively transfer `next_in_convoy` first. | Mind-control a linked vehicle pair: follower changes owner before leader's parent transfer side effects, and the link remains after the call. | Do not raw-write `owner` or route all mobile objects directly to a FootClass/base owner helper. |
| Drive stop propagation is `IsTrain`-gated and follower-byte-gated, then walks `+0x6C8` calling each follower locomotor `Stop_Moving`. | `0x004AFE00`; `0x00712277..0x00712284`; `rg IsTrain= ini` | mismatch: Rust stop clears one entity and group speed sync is unrelated | `src/sim/world/world_commands.rs`; movement stop/locomotor state | If train/convoy state is implemented, stop command and locomotor stop must propagate through native chain under the same gates. | `IsTrain=yes` linked unit A->B with active head-to: stopping A calls B stop; if A's follower byte is set, propagation does not start from A. | Do not propagate stop for ordinary grouped move commands unless the native gates are present. |
| Drive speed propagation pushes leader speed fraction `+0x578` down the pointer chain during DriveLocomotion track processing. | `0x004B121D..0x004B125D` | mismatch: Rust `sync_formation_speeds` caps group speed to slowest member after movement loop | `src/sim/movement/movement_tick.rs`; drive-track speed model | Convoy-chain speed sync must happen in the DriveLocomotion-owned timing with leader speed fraction authority. | Linked accelerating leader updates follower speed on the same drive-track processing pass, independent of selection command group id. | Do not treat current `MovementTarget.group_id` slowest-speed cap as native `+0x6C8` parity. |
| TeamClass convoy clear is separate and walks team members, not Unit `+0x6C8`. | `0x006EC3A0`; call sites `0x004B2EB6`, `0x006A2506`, `0x00516861` | separate/unchecked | `src/sim/ai.rs`, team/script movement future work | Keep scenario unit convoy links distinct from TeamClass member-list convoy actions. | AI team path failure clears team targets/flags without rewriting Unit `next_in_convoy` unless another native path does so. | Do not merge TeamClass convoy member list with UnitClass pointer chain. |

### Stale Docs / Follow-up Docs

- `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` should be corrected where it says `Process_Drive_Track` checks `mission == 1`; replacement wording: "The `== 1` gate at `0x004B121D..0x004B1220` is the owner object's abstract type (`UnitClass`), not a mission id."
- `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` should not call `UnitType+0xE0C` `IsTrain`; replacement wording: "`UnitType+0xE0C` is parsed from `Passive=` in `UnitTypeClass::ReadINI`; `IsTrain=` is `TechnoType+0xC94`."
- `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md` deferred `Unit+0x6C8`; follow-up wording: "`UnitClass::ChangeOwner @ 0x007463A0` propagates owner transfer to `Unit+0x6C8` first, clears current `+0x6D0`, restores the link, sets linked `+0x6D0`, then calls `FootClass::ChangeOwner` on the current unit."

## Sources

- Ghidra read-only decompile/disassembly: `0x007353C0`, `0x00743270`, `0x00744470`, `0x00744640`, `0x007446E0`, `0x007463A0`, `0x004AFE00`, `0x004B0F20`, `0x006EC3A0`, `0x00747620`.
- Ghidra xrefs/data: `ScenarioClass__Read_Units_Section` xref from `0x00687AA7`; Unit vtable data xrefs `0x007F6044`, `0x007F5C98`, `0x007F5CA4`; Drive stop vtable data xref `0x007E7EF8`; Drive track callers `0x004B0576`, `0x004B0AAA`.
- Ghidra byte-pattern scans: `search_byte_patterns "c8 06 00 00"` and `"d0 06 00 00"` used to find displacement candidates; verified Unit/Drive candidates are listed above.
- Ghidra string/parser evidence: `0x00712277..0x00712284`; `read_memory 0x008444BC` -> `IsTrain`; `UnitTypeClass__ReadINI @ 0x00747620` maps `Passive=` to `UnitType+0xE0C`.
- Existing docs used as maps/checks: `CHANGEOWNER_SUBCLASS_WRAPPERS_RESWARM_20260528.md`, `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md`, `UNITCLASS_GHIDRA_REPORT.md`, `DRIVE_LOCOMOTION_CLASS.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`; no visible repo assignments for `IsTrain=` or `Passive=`.
- Rust scanned/read: `src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/world/world_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/command.rs`, broad `rg convoy|formation|group_id`.

## Status

COMPLETE for the requested `UnitClass+0x6C8` convoy-link lifecycle slice. The remaining work is implementation-facing, plus an optional map-corpus frequency scan if campaign usage priority is needed.
