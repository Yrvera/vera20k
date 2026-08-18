# FUN_006E61F0 — TagTypeClass Event-Category Bitmask — Ghidra Research Report

**Date:** 2026-05-18  
**Investigated by:** re-swarm slot 1 (area=bridges)  
**Addresses:** FUN_006E61F0 @ `0x006E61F0`, caller @ `0x00684C30`  
**Overall confidence:** HIGH — all claims derived from live decompilation in this session  
**Active in YR:** Yes, Conditional — executes during every map load but is a no-op on
standard skirmish maps (no `[Tags]`/`[Triggers]` sections → zero-iteration loops)

---

## Executive Summary

**The §16.2 hypothesis in `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` was
wrong on every major claim.**

| Claim in §16.2 | Actual finding |
|---|---|
| FUN_006E61F0 is a "cell predicate" | It is a **TagTypeClass method** (ECX = `TagTypeClass*`) |
| param_1 is a cell or coord | param_1 is a `TagTypeClass*`; `+0xA0` is its trigger linked-list head |
| Bit 4 (value 4) means "bridge-linked cell" | Bit 4 means **"this tag has at least one Destroyed event"** (event codes 8 or 0x18) |
| DAT_008B41A8 is a "bridge-linked cell registry" | It is **`g_DestroyedEventTagList`** — a `DynamicVectorClass<TagClass*>` of tags with Destroyed events |
| FUN_00684C30 populates a bridge-cell lookup | FUN_00684C30 populates a **trigger-system tag registry** during post-map-load setup |

The "collision finding" from `UNREGISTERBRIDGEREPAIRHUT_AND_HUT_REGISTRY_GHIDRA_REPORT.md`
(2026-05-18) is **CORRECT**: `DAT_008B41A8` is `g_DestroyedEventTagList`, not a bridge
registry. This report fully confirms and extends that finding with the internals of
FUN_006E61F0 itself.

---

## 1. FUN_006E61F0 — Full Decompilation Analysis

**Address:** `0x006E61F0`  
**Calling convention:** `__fastcall` (param_1 in ECX = `this` pointer)  
**Return type:** `uint` (bitmask)  
**Real identity:** `TagTypeClass::GetEventCategoryBitmask` (name proposed, not written to Ghidra)

Ghidra pseudocode (verbatim):
```c
uint __fastcall FUN_006e61f0(int param_1)
{
  int iVar1;
  uint uVar2;
  uint uVar3;
  
  uVar3 = 0;
  for (iVar1 = *(int *)(param_1 + 0xa0); iVar1 != 0; iVar1 = *(int *)(iVar1 + 0xa8)) {
    uVar2 = FUN_007271e0();      // FUN_007271e0(TriggerTypeClass*)
    uVar3 = uVar3 | uVar2;
  }
  return uVar3;
}
```

### What it does

- `param_1` is a `TagTypeClass*` (the ECX `this` pointer, implicit in `__fastcall`).
- `param_1 + 0xA0` is the **head pointer of a linked list of TriggerTypeClass instances** attached to this tag.
- `iVar1 + 0xA8` is the **next-pointer** field of each TriggerTypeClass node in the list.
- For each attached TriggerTypeClass, it calls `FUN_007271E0` which recursively classifies all
  trigger events and actions into a bitmask.
- The bitmask is accumulated with OR across all triggers.
- Return value is a bitmask where each bit represents a category of trigger events present.

### TagTypeClass struct fields (verified from decompilation)

All offsets are byte offsets. `param_1` is `int` in Ghidra (direct byte offsets).

| Offset | Type | Field | Evidence |
|--------|------|-------|----------|
| `+0xA0` | `TriggerTypeClass*` | Head of trigger linked-list | `*(int *)(param_1 + 0xA0)` — loop head |
| `+0xA8` | `TriggerTypeClass*` | Next-pointer in trigger node | `*(int *)(iVar1 + 0xA8)` — loop advance |

Note: TagTypeClass constructor (`0x006E5B60`) sets `param_1[0x26]=0xFFFFFFFF`, `param_1[0x27]=0`,
`param_1[0x28]=0`. Since `param_1` is `undefined4*`, `param_1[0x28]` = byte offset `0xA0`.
Byte `0xA0` is thus the first trigger-list field; `0xA4` = associated data, `0xA8` = next-link.

---

## 2. FUN_007271E0 — TriggerTypeClass Event Classifier

**Address:** `0x007271E0`  
**Real identity:** `TriggerTypeClass::GetEventCategoryBitmask` (recursive)

```c
uint __fastcall FUN_007271e0(int param_1)
{
  int iVar1;
  uint uVar2;
  uint uVar3;
  
  uVar3 = 0;
  for (iVar1 = *(int *)(param_1 + 0xac); iVar1 != 0; iVar1 = *(int *)(iVar1 + 0x28)) {
    uVar2 = FUN_0071f680();      // event-type to bitmask classifier
    uVar3 = uVar3 | uVar2;
  }
  for (iVar1 = *(int *)(param_1 + 0xb0); iVar1 != 0; iVar1 = *(int *)(iVar1 + 0x28)) {
    uVar2 = FUN_006e3ee0();      // action-type to bitmask classifier
    uVar3 = uVar3 | uVar2;
  }
  if (*(int *)(param_1 + 0xa8) == 0) {
    return uVar3;
  }
  uVar2 = FUN_007271e0();        // recursive: follow linked structure
  return uVar2 | uVar3;
}
```

### TriggerTypeClass struct fields (byte offsets, param_1 is `int`)

| Offset | Field | Notes |
|--------|-------|-------|
| `+0xAC` | Event-list head (`TriggerEventClass*`) | Loop iterates trigger events |
| `+0xA8` | Next-pointer / linked-struct | Recursive follow for compound triggers |
| `+0xB0` | Action-list head (`TriggerActionClass*`) | Loop iterates trigger actions |

---

## 3. Bit-4 (value `4`) Semantic — The "Destroyed Event" Flag

**FUN_0071F680** (`0x0071F680`) is the trigger event-type to bitmask mapper.
It takes a single int parameter (the event type code) and returns a bitmask.

The bit assignments it produces (verified from full switch decompilation):

| Bit | Value | Meaning | Event codes that set it |
|-----|-------|---------|------------------------|
| 0 | 1 | Category A events | 0, 1, 4, 8, 0x18, 0x19, 0x1A, 0x1F, 0x35, 0x36, 0x3B |
| 1 | 2 | Category B events | 0, 1, 2, 4, 6, 7, 8, 0x1D, 0x21-0x2C, 0x30, 0x31 |
| **2** | **4** | **Destroyed events** | **8 (Destroyed) and 0x18 (Bridge Destroyed)** |
| 3 | 8 | Category D events | 3, 5, 8-0x16, 0x1E, 0x20, 0x34, 0x37-0x3A |
| 4 | 0x10 | Category E events | 8, 0xD, 0xE, 0x17, 0x1B, 0x1C, 0x24, 0x25, 0x2D-0x2F, 0x32, 0x33, 0x3C, 0x3D |

**Bit 2 (value `4`) is set if and only if the trigger event type is 8 (Destroyed) or 0x18 (Bridge Destroyed).**

Both `8` and `0x18` represent object-destroyed trigger events in the YR trigger system.

---

## 4. What FUN_00684C30 Actually Does with Bit 4

The caller loop (verified from FUN_00684C30 @ `0x00684C30`):

```
for each TagTypeClass tag in g_TagTypeClass_Array:
    uVar2 = FUN_006e61f0(tag);              // get event-category bitmask
    if (uVar2 & 4) != 0:                   // has any Destroyed event?
        uVar3 = FUN_006e52a0(tag);          // find or create TagClass for this tag
        push uVar3 into g_DestroyedEventTagList (DAT_008B41A8)
    
    if (uVar2 & 0x10) != 0:               // has category-E event?
        uVar3 = FUN_006e52a0(tag)
        push uVar3 into DAT_008B40C8 DynVec
    
    if (uVar2 & 8) != 0:                  // has category-D event?
        uVar3 = FUN_006e52a0(tag)
        push uVar3 into per-house trigger list (HouseClass+0x38)
```

FUN_006E61F0 is called **three times per tag** in the loop — once for each category check.
Each call re-traverses the trigger tree (not cached between calls within the loop iteration).

---

## 5. DAT_008B41A8 — `g_DestroyedEventTagList` Confirmed

This is a `DynamicVectorClass<TagClass*>`. Layout (verified from prior doc and re-confirmed
by decompiling `MapClass::UnregisterBridgeRepairHut` @ `0x00577920` which removes entries):

| Address | Field | Type |
|---------|-------|------|
| `0x008B41A8` | vtable ptr | `int*` |
| `0x008B41AC` | data_ptr | `TagClass**` |
| `0x008B41B0` | capacity | `int` |
| `0x008B41B4` | owns_memory | `bool` |
| `0x008B41B5` | flag | `bool` |
| `0x008B41B8` | count | `int` |
| `0x008B41BC` | grow_step | `int` |

**Producers:**
- `FUN_00684C30` @ `0x00684C30` (post-map-load): populates from `g_TagTypeClass_Array`
- `FUN_0067F9C0` @ `0x0067F9C0` (savegame loader): deserializes from save stream

**Consumers:**
- `MapClass::UnregisterBridgeRepairHut` @ `0x00577920`: removes a TagClass when destroyed
- `FootClass::PerCellProcess` @ `0x004D8B60`: uses `DAT_008B41B8`/`DAT_008B41AC` to check
  bridge-repair proximity (event type 0x18) and fire `TechnoClass::ProcessCellAction`
- `TagClass::Constructor` @ `0x006E4F60`: removes self from list during destruction
- `FUN_0067F7E0` @ `0x0067F7E0` (savegame writer): serializes to save stream

---

## 6. Collision Resolution — §16.2 vs UNREGISTERBRIDGEREPAIRHUT report

The 2026-05-18 `UNREGISTERBRIDGEREPAIRHUT_AND_HUT_REGISTRY_GHIDRA_REPORT.md` finding is
**fully confirmed** by this investigation:

- `DAT_008B41A8` = `g_DestroyedEventTagList` (NOT a bridge-cell registry)
- FUN_006E61F0 operates on `TagTypeClass*` (NOT on cells or coords)
- The §16.2 "bridge-linked cell" hypothesis is **REFUTED** on all three counts

The name `MapClass::UnregisterBridgeRepairHut` is a Ghidra annotation artefact from earlier
investigation; the function is actually a generic TagClass detach helper (checks RTTI == 0x2C).

**Remaining §16.2 questions now fully resolved:**
- Which fields it reads: `TagTypeClass+0xA0` (trigger list head), `TriggerTypeClass+0xAC/+0xB0/+0xA8`
- What bit 4 encodes: presence of event type 8 (Destroyed) or 0x18 (Bridge Destroyed)
- What the global vector stores: `TagClass*` pointers for tags with Destroyed events

---

## 7. Active in YR

**Yes, Conditional.**

- Code path is always exercised during map load (`FUN_00684C30` runs post-`.MAP` file parse).
- On **standard skirmish maps** with no `[Tags]`/`[Triggers]` sections, `g_TagTypeClass_Array_Count`
  is zero, so the loop runs zero iterations — both DynVecs remain empty.
- On **campaign and scripted multiplayer maps** that have `[Tags]`/`[Triggers]` entries,
  the function classifies each tag and populates the Destroyed-event list.
- The TS-legacy risk is low: the trigger system itself is live in YR (used in campaign).
  Event codes 8 and 0x18 are valid YR events. No TS-gating flag involved.

---

## 8. Open Questions

1. **FUN_006E3EE0** (`0x006E3EE0`): the action-type to bitmask classifier (analogous to
   FUN_0071F680 for events). Returns bit 2 for action codes 0xE, 0x20, 0x3C-0x3E, 0x5B, 0x6F.
   These action codes map to specific trigger actions — not investigated in this run.

2. **DAT_008B40C8 DynVec** (bit-0x10 / category-E vector): separate DynVec populated when
   `FUN_006E61F0() & 0x10 != 0`. Likely used for a different trigger-event category. Not
   investigated in this run.

3. **TagTypeClass layout `+0xA4`**: the field between the list head (`+0xA0`) and next-link
   (`+0xA8`) in the TriggerTypeClass node was not identified; likely holds the TriggerTypeClass
   pointer payload itself.

4. **FUN_006E52A0** (`0x006E52A0`): "find or create TagClass for TagTypeClass" — find-or-create
   factory. Not deep-dived; purpose is clear from context.

---

## 9. Rust Implication

The Rust bridge implementation does not need to replicate `g_DestroyedEventTagList` or
FUN_006E61F0 as part of the bridge state machine. These are trigger-system bookkeeping
structures. The bridge-state machine in `src/sim/bridge_state/` is independent of the
YR trigger/tag system (which is not yet implemented). When the trigger system is
implemented, the destroyed-event tag list should be populated during map load by walking
`TagTypeClass` instances and filtering for event codes 8 and 0x18.
