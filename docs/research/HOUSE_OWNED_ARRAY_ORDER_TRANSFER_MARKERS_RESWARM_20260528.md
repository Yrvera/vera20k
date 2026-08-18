# House Owned Array Order Transfer Markers - Reswarm Research Report

**Address(es):** `0x006DD8B0`, `0x006E0CA0`, `0x006E0D00`, `0x0050D290`, `0x0050D2D0`, `0x0070BF50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** trigger actions `0x7B` and `0x7C` only: their helper entry points, the House owned array/list they walk, iteration order/direction, `Techno+0x2E0/+0x2CC` writes and clears, visible save/load treatment of those pointer slots, and Rust-facing requirements for temporary transfer/reclaim.
**Non-Scope:** direct owner-transfer trigger actions `0x0E/0x24`, team script opcode `0x14`, full `TechnoClass::ChangeOwner` internals beyond the relevant `+0x2E0` clear, full hijacker/mind-control semantics of the same fields, retail map frequency, editor-facing action names.
**Confidence:** High for action IDs, helper entry points, array offsets, reverse iteration, marker write/clear order, and load swizzle registration. Medium for the human semantic names "temporary source marker" and "temporary destination marker" because the same fields also participate in other Techno systems.
**Active in YR:** Conditional. The code is active in standard Yuri's Revenge when a loaded map trigger executes action `0x7B` or `0x7C`; no INI key gates this path.

## 1. Overview

Trigger actions `0x7B` and `0x7C` are a paired temporary ownership-transfer mechanism. `0x7B` transfers every object currently in a resolved source house's owned-object vector to the trigger owner/destination house, then records the source and destination in `Techno+0x2E0` and `Techno+0x2CC`. `0x7C` walks the current trigger owner's owned-object vector, reclaims only objects whose `+0x2E0` marker matches the resolved source house, transfers them back, and clears both markers.

This is order-sensitive and stateful. The native implementation walks the House-owned `DynamicVector<TechnoClass*>` from last entry to first entry, and each object is transferred through concrete virtual `+0x3D4`, not by raw owner pointer mutation.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Purpose in this slice | Evidence | Active in YR |
|---:|---|---|---|---|---|
| `ActionEntry+0x2C` | TriggerAction entry | `i32` | action ID switch key; cases `0x7B` and `0x7C` route to the helpers | `TriggerAction__Execute @ 0x006DD8B0` decompile | Conditional by map trigger |
| `ActionEntry+0x90` | TriggerAction entry | `i32` | house operand resolved before transfer/reclaim | `0x006E0CA0`, `0x006E0D00` decompile | Conditional |
| `House+0x68` | HouseClass | `DynamicVector<TechnoClass*>` header | owned objects vector object | `HOUSECLASS_CONSTRUCTOR_DETAILED.md`; `HOUSECLASS_GHIDRA_REPORT.md` | Yes |
| `House+0x6C` | HouseClass | `TechnoClass**` | owned objects backing array pointer read by transfer/reclaim | `0x0050D2A0`, `0x0050D2E0` assembly | Conditional |
| `House+0x78` | HouseClass | `i32` | owned objects count read before reverse iteration | `0x0050D299`, `0x0050D2D4` assembly | Conditional |
| `Techno+0x21C` | TechnoClass | `HouseClass*` | normal current owner pointer; `ChangeOwner` writes it late | `0x00701735` assembly | Yes |
| `Techno+0x2CC` | TechnoClass | pointer-sized dword | destination/new-house marker written by action `0x7B`, cleared by `0x7C`; also a reused Techno field in other systems | `0x0050D2BA`, `0x0050D303`, `0x0070C06C..0x0070C078` | Conditional |
| `Techno+0x2E0` | TechnoClass | pointer-sized dword | source/old-house marker written by action `0x7B`, compared and cleared by `0x7C`; also cleared inside `TechnoClass::ChangeOwner` before `0x7B` rewrites it | `0x0050D2B4`, `0x0050D2E6`, `0x0050D2FD`, `0x007017A0`, `0x0070C07D..0x0070C089` | Conditional |

House constructor evidence: `HOUSECLASS_CONSTRUCTOR_DETAILED.md` maps DVC #3 at `House+0x68`, with backing pointer at `+0x6C`, count at `+0x78`, grow at `+0x7C`, and names it `OwnedObjects (TechnoClass*[])`. `HOUSECLASS_GHIDRA_REPORT.md` independently names `+0x6C/+0x78` as `OwnedObjectsArray/OwnedObjectsCount`.

## 3. Core Logic

### 3.1 Trigger action entry points

`TriggerAction__Execute @ 0x006DD8B0` dispatches:

- case `0x7B` -> `FUN_006E0CA0(param_2,param_3,param_4,param_5)`;
- case `0x7C` -> `FUN_006E0D00(param_2,param_3,param_4,param_5)`.

Decompile evidence:

```text
case 0x7b:
  uVar6 = FUN_006e0ca0(param_2,param_3,param_4,param_5);
  return uVar6;
case 0x7c:
  uVar6 = FUN_006e0d00(param_2,param_3,param_4,param_5);
  return uVar6;
```

Assembly call-site evidence:

```text
0x006DFA26  CALL 0x006e0ca0
0x006DFA53  CALL 0x006e0d00
```

The surrounding assembly pushes the same four trigger-runtime arguments used by adjacent trigger actions; `ESI` is the action entry object (`ECX=ESI` before the call), and the caller returns immediately with the helper result.

### 3.2 Shared house operand resolution in `0x006E0CA0` / `0x006E0D00`

Both action helpers use the same resolution shape:

1. Read action operand from `ActionEntry+0x90` into `ESI`.
2. If the fourth runtime argument is null, return `0`.
3. If operand is `0x2325`, call `FUN_00726910()`.
4. Else if operand is `-1`, return `0`.
5. Else call `FUN_00510F60()`; if false, resolve via `HouseClass__Find_By_Country_Index()`, otherwise resolve via `FUN_00510ED0()`.
6. If resolved house is null, return `0`.
7. Call the House helper and return `1`.

Verified transfer action call setup:

```text
0x006E0CE6  MOV ECX,dword ptr [ESP + 0x8]  ; trigger owner / destination
0x006E0CEA  PUSH EAX                       ; resolved source house
0x006E0CEB  CALL 0x0050d290
0x006E0CF0  MOV AL,0x1
```

Verified reclaim action call setup:

```text
0x006E0D46  MOV ECX,dword ptr [ESP + 0x8]  ; current trigger owner / destination
0x006E0D4A  PUSH EAX                       ; resolved original/source house
0x006E0D4B  CALL 0x0050d2d0
0x006E0D50  MOV AL,0x1
```

This means `0x7B` is `destinationHouse.TransferUnitsFrom(resolvedSourceHouse)` and `0x7C` is `currentHouse.ReclaimUnitsFrom(resolvedSourceHouse)` in semantic terms.

### 3.3 Action `0x7B`: transfer source-owned objects to trigger owner

Function: `HouseClass__TransferUnitsTo @ 0x0050D290`.

Decompile:

```text
i = source[+0x78];
while (i = i - 1, -1 < i) {
    obj = source[+0x6C][i];
    obj->vtable[+0x3D4](destination, 0);
    obj[+0x2E0] = source;
    obj[+0x2CC] = destination;
}
```

Load-bearing assembly:

```text
0x0050D292  MOV EBP,dword ptr [ESP + 0xc]   ; source house arg
0x0050D297  MOV EBX,ECX                     ; destination/trigger owner
0x0050D299  MOV EDI,dword ptr [EBP + 0x78]  ; source owned count
0x0050D29C  DEC EDI
0x0050D29D  JS  0x0050d2c3                  ; empty vector exits
0x0050D2A0  MOV EAX,dword ptr [EBP + 0x6c]  ; source owned array
0x0050D2A3  PUSH 0x0                        ; announce flag false
0x0050D2A5  PUSH EBX                        ; new owner/destination
0x0050D2A6  MOV ESI,dword ptr [EAX + EDI*4] ; current object
0x0050D2A9  MOV ECX,ESI
0x0050D2AD  CALL dword ptr [EDX + 0x3d4]    ; concrete ChangeOwner wrapper
0x0050D2B3  DEC EDI
0x0050D2B4  MOV dword ptr [ESI + 0x2e0],EBP ; source marker, after ChangeOwner
0x0050D2BA  MOV dword ptr [ESI + 0x2cc],EBX ; destination marker, after source marker
0x0050D2C0  JNS 0x0050d2a0                  ; continue downward
```

Tiny details:

- Empty source owned vector exits before pushing `ESI`; `count=0` becomes `EDI=-1`, then `JS` exits.
- Iteration order is reverse index order: `count - 1`, `count - 2`, ..., `0`.
- The owned-array base pointer is reloaded from `source+0x6C` each iteration.
- Marker writes happen after `ChangeOwner`, not before.
- `TechnoClass::ChangeOwner` itself clears `Techno+0x2E0` at `0x007017A0`; `0x7B` intentionally writes `+0x2E0` again after that clear.
- Marker write order is `+0x2E0` first, then `+0x2CC`.
- The helper does not null-check each owned-array element before calling virtual `+0x3D4`.
- The helper does not test object alive, limbo, marked, class, or type flags. Membership in the House owned vector is the only selection filter inside this helper.

### 3.4 Action `0x7C`: reclaim only marked temporary transfers

Function: `HouseClass__ReclaimUnitsFrom @ 0x0050D2D0`.

Decompile:

```text
i = currentHouse[+0x78];
while (i = i - 1, -1 < i) {
    obj = currentHouse[+0x6C][i];
    if (obj[+0x2E0] == sourceHouse) {
        obj->vtable[+0x3D4](sourceHouse, 0);
        obj[+0x2E0] = 0;
        obj[+0x2CC] = 0;
    }
}
```

Load-bearing assembly:

```text
0x0050D2D2  MOV EBP,ECX                     ; current trigger owner/destination
0x0050D2D4  MOV EBX,dword ptr [EBP + 0x78]  ; current owned count
0x0050D2D7  DEC EBX
0x0050D2D8  JS  0x0050d30e                  ; empty vector exits
0x0050D2DC  MOV EDI,dword ptr [ESP + 0x14]  ; resolved original/source house
0x0050D2E0  MOV EAX,dword ptr [EBP + 0x6c]  ; current owned array
0x0050D2E3  MOV ESI,dword ptr [EAX + EBX*4]
0x0050D2E6  CMP dword ptr [ESI + 0x2e0],EDI ; marker match required
0x0050D2EC  JNZ 0x0050d309
0x0050D2F0  PUSH 0x0
0x0050D2F2  PUSH EDI                        ; restore owner/source
0x0050D2F3  MOV ECX,ESI
0x0050D2F5  CALL dword ptr [EDX + 0x3d4]
0x0050D2FB  XOR EAX,EAX
0x0050D2FD  MOV dword ptr [ESI + 0x2e0],EAX ; clear source marker
0x0050D303  MOV dword ptr [ESI + 0x2cc],EAX ; clear destination marker
0x0050D309  DEC EBX
0x0050D30A  JNS 0x0050d2e0
```

Tiny details:

- Empty current-owned vector exits before loading `EDI`.
- Iteration order is reverse index order over the current trigger owner's owned vector.
- `+0x2E0` is the only selection predicate. `+0x2CC` is not checked.
- Marker clears happen after the restoring `ChangeOwner` call.
- Clear order is `+0x2E0` then `+0x2CC`.
- Non-matching objects keep both marker fields unchanged.
- The helper does not verify that `+0x2CC` equals the current house. Reclaim depends on current-owned-vector membership plus source marker match.
- The helper does not count whether any object was reclaimed; the action wrapper returns `1` once the house operand resolves non-null, even if zero objects match.

### 3.5 Save/load visibility for `+0x2CC/+0x2E0`

The visible load-specific behavior is in `TechnoClass::Load_IStream @ 0x0070BF50`. After base raw load succeeds, it registers many pointer slots with the swizzle manager. `+0x2CC` and `+0x2E0` are explicitly registered:

```text
0x0070C06C  LEA EDX,[ESI + 0x2cc]
0x0070C072  PUSH EDX
0x0070C073  PUSH 0xb0c110
0x0070C078  CALL 0x006cf240
0x0070C07D  LEA EAX,[ESI + 0x2e0]
0x0070C083  PUSH EAX
0x0070C084  PUSH 0xb0c110
0x0070C089  CALL 0x006cf240
```

`TechnoClass::Constructor @ 0x006F2B40` initializes both fields to zero:

```text
param_1[0xb3] = 0;  // +0x2CC
param_1[0xb8] = 0;  // +0x2E0
```

`TechnoClass::Save_IStream @ 0x0070C250` is a thin forwarder to `ObjectClass::Save_IStream @ 0x0065AC40`, and prior save/load research shows the stream path goes through `AbstractClass::Save` raw object persistence plus pointer fixups. Therefore in a mid-temporary-transfer save, these pointer-valued marker fields are not reconstructed from House ownership later; they are raw persisted and then swizzled like other Techno pointer fields during load.

Do not confuse this with `TechnoClass__Save @ 0x0070C270`, which is a separate checksum/state-marshalling function and not the stream body for these fields.

## 4. INI Keys

None. This mechanism is map trigger action driven. It is activated by `[Actions]` entries that use action IDs `123` and `124` decimal (`0x7B` / `0x7C`), not by rules/art INI keys.

## 5. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `TriggerAction__Execute @ 0x006DD8B0` | top-level trigger action dispatcher | cases `0x7B`, `0x7C` decompile | Conditional by map |
| `FUN_006E0CA0` | action `0x7B` wrapper; resolves source house and calls transfer helper | decompile; call `0x006E0CEB` | Conditional |
| `FUN_006E0D00` | action `0x7C` wrapper; resolves source house and calls reclaim helper | decompile; call `0x006E0D4B` | Conditional |
| `HouseClass__TransferUnitsTo @ 0x0050D290` | reverse-walk source owned array, dispatch `+0x3D4`, write markers | decompile and assembly | Conditional |
| `HouseClass__ReclaimUnitsFrom @ 0x0050D2D0` | reverse-walk current owned array, compare marker, dispatch `+0x3D4`, clear markers | decompile and assembly | Conditional |
| `TechnoClass::ChangeOwner @ 0x007014A0` | class-dispatched owner transfer; clears `+0x2E0` before the transfer helper rewrites it | `0x007017A0`; prior ChangeOwner report | Yes |
| `TechnoClass::Load_IStream @ 0x0070BF50` | swizzle-registers marker slots after load | `0x0070C06C..0x0070C089` | Conditional by save/load |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior observed | Delta for this slice |
|---|---|---|
| `src/map/actions.rs:11` | `ActionEntry` preserves `kind: i32` and raw string params | parser can carry action IDs `123/124`, but no native house-operand resolver exists |
| `src/sim/trigger_runtime.rs:25..36` | supported action constants omit `123` and `124` | actions `0x7B/0x7C` currently no-op via default branch |
| `src/sim/trigger_runtime.rs:214..307` | `apply_action` only emits effects / changes variables for a narrow subset | no mutation surface for class-dispatched owner transfer or marker state |
| `src/sim/entity_store.rs:31..37` | owner index is `BTreeMap<owner, Vec<stable_id>>` sorted by stable ID after rebuild | not equivalent to native House owned-vector insertion/order unless proven; reverse stable-ID order would not be native reverse owned-vector order |
| `src/sim/entity_store.rs:140..149` | owner index rebuild iterates primary `BTreeMap` by ascending stable ID | a temporary transfer implemented from this index would walk a different order than native |
| `src/sim/game_entity.rs:130..150` | entity has only current `owner` | no `+0x2E0/+0x2CC` temporary source/destination marker state |
| `src/sim/game_entity.rs:317..325` | has garrison-specific original owner and passenger role state | not a substitute for action `0x7B/0x7C` markers |
| `src/sim/world/world_orders.rs` / `src/sim/passenger.rs` | known direct owner writes in capture/garrison paths from prior reports | same future class-dispatched owner-transfer API is needed before these trigger actions can be exact |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Trigger action `0x7B` dispatcher case | verified | `TriggerAction__Execute @ 0x006DD8B0`; call `0x006DFA26` | editor-facing name not binary-verified |
| Trigger action `0x7C` dispatcher case | verified | `TriggerAction__Execute @ 0x006DD8B0`; call `0x006DFA53` | editor-facing name not binary-verified |
| `0x006E0CA0` source-house wrapper | verified | decompile; assembly `0x006E0CE6..0x006E0CEB` | exact human meaning of sentinel `0x2325` remains contextual |
| `0x006E0D00` reclaim wrapper | verified | decompile; assembly `0x006E0D46..0x006E0D4B` | exact human meaning of sentinel `0x2325` remains contextual |
| House owned array identity | verified | `House+0x68/+0x6C/+0x78` docs plus helper assembly reads | full append/remove mechanics of the vector are outside this slot |
| `HouseClass__TransferUnitsTo @ 0x0050D290` | verified | decompile and assembly `0x0050D292..0x0050D2C0` | none for this slice |
| `HouseClass__ReclaimUnitsFrom @ 0x0050D2D0` | verified | decompile and assembly `0x0050D2D2..0x0050D30A` | none for this slice |
| Marker constructor defaults | verified | `TechnoClass::Constructor @ 0x006F2B40` decompile | none |
| Marker load swizzle behavior | verified | `TechnoClass::Load_IStream @ 0x0070BF50`; assembly `0x0070C06C..0x0070C089` | exact raw-save byte-owner handled by prior save/load docs, not re-proven with runtime stream here |
| Rust trigger runtime support | verified | `src/sim/trigger_runtime.rs:25..36`, `src/sim/trigger_runtime.rs:214..307` | no Rust edited |
| Rust native House-owned vector order | verified-missing | `src/sim/entity_store.rs:31..37`, `src/sim/entity_store.rs:140..149` | future implementation must add or prove native-equivalent ordering |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which trigger action IDs enter this mechanism? -> `0x7B` calls `FUN_006E0CA0`; `0x7C` calls `FUN_006E0D00`.` (evidence: `TriggerAction__Execute @ 0x006DD8B0`, calls `0x006DFA26`, `0x006DFA53`)
- `[RESOLVED] OQ-02 - Does either action raw-write owner? -> No. Both reach House helpers that dispatch object virtual `+0x3D4(...,0)`.` (evidence: `0x0050D2AD`, `0x0050D2F5`)
- `[RESOLVED] OQ-03 - Which House list is walked by `0x7B`? -> Source house `DynamicVector<TechnoClass*>` at `House+0x68`, using backing pointer `+0x6C` and count `+0x78`.` (evidence: `0x0050D299`, `0x0050D2A0`; `HOUSECLASS_CONSTRUCTOR_DETAILED.md`)
- `[RESOLVED] OQ-04 - Which House list is walked by `0x7C`? -> Current trigger owner/destination house `DynamicVector<TechnoClass*>` at the same `+0x68/+0x6C/+0x78`.` (evidence: `0x0050D2D2..0x0050D2E0`)
- `[RESOLVED] OQ-05 - What is transfer iteration order? -> Reverse array index order, `count-1` down to `0`.` (evidence: `0x0050D299..0x0050D2C0`)
- `[RESOLVED] OQ-06 - What is reclaim iteration order? -> Reverse array index order, `count-1` down to `0`.` (evidence: `0x0050D2D4..0x0050D30A`)
- `[RESOLVED] OQ-07 - What happens for an empty source/current owned vector? -> `count` is decremented to `-1` and signed-branch exits before object load.` (evidence: `0x0050D29C..0x0050D29D`, `0x0050D2D7..0x0050D2D8`)
- `[RESOLVED] OQ-08 - When does `0x7B` write markers relative to `ChangeOwner`? -> After the virtual `+0x3D4` call; `+0x2E0` then `+0x2CC`.` (evidence: `0x0050D2AD..0x0050D2BA`)
- `[RESOLVED] OQ-09 - What does `0x7B` store in each marker? -> `+0x2E0 = source house`, `+0x2CC = destination/trigger owner`.` (evidence: `0x0050D2B4`, `0x0050D2BA`)
- `[RESOLVED] OQ-10 - What predicate does `0x7C` use before reclaiming? -> Only `Techno+0x2E0 == resolved source house`.` (evidence: `0x0050D2E6..0x0050D2EC`)
- `[RESOLVED] OQ-11 - Does `0x7C` check `+0x2CC` before reclaim? -> No checked compare exists in the helper; current-owned-vector membership and `+0x2E0` match are the filters.` (evidence: `0x0050D2E0..0x0050D2F5`)
- `[RESOLVED] OQ-12 - When does `0x7C` clear markers? -> After the restoring `+0x3D4` call; clears `+0x2E0` then `+0x2CC`.` (evidence: `0x0050D2F5..0x0050D303`)
- `[RESOLVED] OQ-13 - Does `TechnoClass::ChangeOwner` interact with these markers? -> It clears `+0x2E0` at `0x007017A0`; action `0x7B` writes its source marker after the call, so the temporary marker survives the transfer helper.` (evidence: `0x007017A0`, `0x0050D2AD..0x0050D2B4`)
- `[RESOLVED] OQ-14 - Are the marker fields initialized? -> Constructor zeros both `+0x2CC` and `+0x2E0`.` (evidence: `TechnoClass::Constructor @ 0x006F2B40`)
- `[RESOLVED] OQ-15 - Are the marker pointer slots visible in load fixup? -> Yes. `TechnoClass::Load_IStream` swizzle-registers `this+0x2CC` and `this+0x2E0`.` (evidence: `0x0070C06C..0x0070C089`)
- `[RESOLVED] OQ-16 - Is this INI-gated? -> No relevant rules/art key exists; activation is by map trigger action IDs.` (evidence: trigger action switch and no INI reader in helper path)
- `[RESOLVED] OQ-17 - Does current Rust implement action IDs `123/124`? -> No. Supported action constants omit both and default branch does nothing.` (evidence: `src/sim/trigger_runtime.rs:25..36`, `src/sim/trigger_runtime.rs:306`)
- `[RESOLVED] OQ-18 - Does current Rust have marker fields? -> No general temporary transfer source/destination markers are present on `GameEntity`.` (evidence: `src/sim/game_entity.rs:130..150`, `src/sim/game_entity.rs:317..325`)
- `[RESOLVED] OQ-19 - Is Rust's owner index order native-equivalent? -> No proof. It is stable-ID sorted after rebuild, while native uses House owned-vector order and reverse traversal.` (evidence: `src/sim/entity_store.rs:31..37`, `src/sim/entity_store.rs:140..149`)
- `[DEFERRED] OQ-20 - What are the exact editor-facing action names for `0x7B/0x7C`?` (category: `requires-different-system-context`; reason: names are not embedded in the verified executable path; next-step-if-pursued: inspect FinalAlert/editor resources or map authoring metadata)
- `[DEFERRED] OQ-21 - Which stock missions use these actions?` (category: `requires-different-system-context`; reason: this slot proves engine semantics, not retail MIX/map usage frequency; next-step-if-pursued: extract retail maps and scan `[Actions]` for action IDs `123/124`)
- `[DEFERRED] OQ-22 - Full hijacker/mind-control interaction with `+0x2CC/+0x2E0`.` (category: `out-of-scope`; reason: this slot only proves trigger transfer/reclaim markers; next-step-if-pursued: investigate hijacker/mind-control field sharing and conflict order)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Trigger action `0x7B` resolves a source house, then reverse-walks that source house's native owned-object vector and calls concrete virtual `+0x3D4(destination,0)` for each object. | `0x006E0CA0`; `0x006E0CE6..0x006E0CEB`; `0x0050D292..0x0050D2AD` | missing | `src/sim/trigger_runtime.rs`; future owner-transfer API; future native House owned-object order | Implement action `123` only on top of class-dispatched owner transfer and native-equivalent House owned-vector order. | Source house has three objects in known native owned order; action `123` transfers them in reverse native order and class-specific wrappers run. | Do not iterate sorted stable IDs or raw-write `entity.owner`. |
| `0x7B` writes temporary markers after each transfer: `Techno+0x2E0 = source`, then `Techno+0x2CC = destination`. | `0x0050D2AD..0x0050D2BA`; `0x007017A0` | missing marker fields | `src/sim/game_entity.rs`; future Techno lifecycle state | Persist per-entity temporary transfer source/destination marker state separate from current owner. | After action `123`, transferred units record their original source house even though their current owner is the destination. | Do not write markers before owner transfer; `ChangeOwner` clears `+0x2E0`. |
| Trigger action `0x7C` reverse-walks the current trigger owner's native owned vector and reclaims only objects whose source marker matches the resolved house. | `0x006E0D00`; `0x006E0D46..0x006E0D4B`; `0x0050D2D2..0x0050D2F5` | missing | `src/sim/trigger_runtime.rs`; owner-transfer API; marker state | Implement action `124` as a paired reclaim, not as a bulk owner rewrite. | Destination house owns objects from two temporary source houses; reclaim for House A restores only objects whose marker is House A. | Do not reclaim every current-owner object; `+0x2CC` is not the native predicate. |
| Reclaim clears markers after the restoring owner transfer, in order `+0x2E0=0`, `+0x2CC=0`. | `0x0050D2F5..0x0050D303` | missing | future Techno lifecycle state | Clear marker state only after successful class-dispatched transfer back. | Reclaim action followed by a second reclaim for the same source does not transfer the same unit again. | Do not clear markers before dispatch or leave destination marker set after reclaim. |
| Save/load preserves and swizzles both marker pointer fields through `TechnoClass::Load_IStream`. | `0x0070C06C..0x0070C089`; save/load docs for raw object persistence | missing/unchecked | `src/sim/snapshot.rs`; future marker fields | Serialized Rust snapshots must preserve temporary transfer markers across save/load when these actions are implemented. | Save after action `123`, load, then run action `124`; the same marked objects reclaim correctly. | Do not reconstruct markers from current owner; native loads pointer slots. |
| Native House owned-vector order is the iteration authority for actions `0x7B/0x7C`. | `House+0x6C/+0x78` reads at `0x0050D2A0`, `0x0050D2E0`; House constructor docs | mismatch/unproven | `src/sim/entity_store.rs`; future House-owned-order storage | Maintain a native-equivalent per-house owned Techno order and support reverse traversal. | Two transfers with side effects that mutate House arrays still process in native reverse vector order. | Do not rely on `BTreeMap` or stable-ID sorted owner indexes as a parity substitute. |

### Stale Docs / Follow-up Docs

- `SCRIPT_OWNER_CHANGE_OPCODES_VIRTUAL_3D4_RESWARM_20260528.md` is directionally correct for actions `0x7B/0x7C`, but this report refines the implementation contract: `0x7C` does not compare `+0x2CC`; it only compares `+0x2E0` while walking the current trigger owner's owned vector. Marker fields are load-swizzled by `TechnoClass::Load_IStream`.
- `TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md` should not describe `+0x2E0` solely as a target/order state slot in contexts that include temporary trigger transfers. Replacement wording for this context: "`Techno+0x2E0` is cleared by `TechnoClass::ChangeOwner`, but action `0x7B` immediately rewrites it after `+0x3D4` as the temporary transfer source-house marker; `0x7C` uses this marker to reclaim."
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` labels `+0x2CC/+0x2E0` as hijacker/original-owner fields. That can remain true for the hijacker slice, but it should be amended to note verified reuse by House temporary transfer/reclaim actions `0x7B/0x7C`.

## Sources

- Ghidra read-only decompile: `TriggerAction__Execute @ 0x006DD8B0`, `FUN_006E0CA0`, `FUN_006E0D00`, `HouseClass__TransferUnitsTo @ 0x0050D290`, `HouseClass__ReclaimUnitsFrom @ 0x0050D2D0`, `TechnoClass::ChangeOwner @ 0x007014A0`, `TechnoClass::Constructor @ 0x006F2B40`, `TechnoClass::Load_IStream @ 0x0070BF50`, `TechnoClass::Save_IStream @ 0x0070C250`.
- Ghidra read-only assembly/context: `0x006DFA26`, `0x006DFA53`, `0x006E0CE6..0x006E0CEB`, `0x006E0D46..0x006E0D4B`, `0x0050D292..0x0050D2C0`, `0x0050D2D2..0x0050D30A`, `0x007017A0`, `0x0070C06C..0x0070C089`.
- Prior docs used as maps/checks: `SCRIPT_OWNER_CHANGE_OPCODES_VIRTUAL_3D4_RESWARM_20260528.md`, `HOUSECLASS_GHIDRA_REPORT.md`, `HOUSECLASS_CONSTRUCTOR_DETAILED.md`, `TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md`, `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`, `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`.
- Rust scanned/read: `src/map/actions.rs`, `src/sim/trigger_runtime.rs`, `src/sim/entity_store.rs`, `src/sim/game_entity.rs`.

## Status

COMPLETE for the requested bounded slice. The House array/list identity, reverse order/direction, marker writes/clears, action helper entry points, and visible load swizzle behavior are verified. Retail map frequency, editor-facing names, and full hijacker field-sharing interactions are explicitly deferred because they do not change this trigger transfer/reclaim contract.
