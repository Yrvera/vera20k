# fn-BuildingClass-IronCurtain

## Identity

| Field | Value |
|---|---|
| Address | `0x00457c90` |
| Name | `BuildingClass__IronCurtain` |
| Signature | `void __thiscall BuildingClass__IronCurtain(void* this, int duration, int source_house, int is_force_shield)` |
| Vtable slot | Byte offset `0x154` from vtable__BuildingClass base `0x007e4000` (slot 85). Verified: DATA xref from `0x007e4010` (bytes `90 7c 45 00` = `0x00457c90`); read_memory 0x007e4010. |
| Active in YR | Yes — BuildingClass vtable slot dispatched from IronCurtain super-weapon apply path. |
| Body range | `0x00457c90` – `0x00457cdd` |

Verified via `decompile_function 0x00457c90` and `get_function_by_address 0x00457c90`.

## Decompiled body (verbatim)

```c
void __thiscall BuildingClass__IronCurtain(void *this, int duration, int source_house, int is_force_shield)
{
  undefined4 uVar1;
  undefined4 local_8;

  if (*(char *)((int)this + 0x6df) != '\0') {
    *(undefined1 *)((int)this + 0x6df) = 0;
    uVar1 = g_CurrentFrameCounter;
    *(undefined4 *)((int)this + 0x540) = 0;
    *(undefined4 *)((int)this + 0x528) = uVar1;
    *(undefined4 *)((int)this + 0x52c) = local_8;
    *(undefined4 *)((int)this + 0x530) = 0;
  }
  TechnoClass__IronCurtain(this, duration, source_house, is_force_shield);
  return;
}
```

Verified via `decompile_function 0x00457c90`.

## Control flow

1. **Gate check**: read `this + 0x6df` (1 byte). If non-zero, enter the building-specific IC reset block.
2. **Reset block** (only when `+0x6df` was set):
   - Clear `+0x6df` to 0.
   - Read `g_CurrentFrameCounter` into a register.
   - Write `0` to `this + 0x540`.
   - Write current frame counter to `this + 0x528`.
   - Write `local_8` (Ghidra-named uninitialized stack slot at [ESP-8]) to `this + 0x52c`. See note below.
   - Write `0` to `this + 0x530`.
3. **Delegate**: call `TechnoClass__IronCurtain(this, duration, source_house, is_force_shield)` unconditionally.

## Key observations

### `+0x6df` gate byte

The byte at `BuildingClass + 0x6df` acts as a "reset pending" flag. When it is set, the building has some pre-IC state that needs to be cleared before the IC effect is applied. The purpose of this gate is not fully clear from this function alone — it is set elsewhere and cleared here.

**YELLOW — Unverified**: What sets `+0x6df`? Likely a building-specific production or undeploy state. The struct-decode task for BuildingClass IC fields (task #11) must identify this via additional xref analysis.

### `+0x52c` gets `local_8` — stack garbage or meaningful value?

`local_8` is at `[ESP - 8]` in Ghidra's frame model. In the Ghidra decompilation, `local_8` is listed as `undefined4` with no assignment shown — it appears to be whatever was on the stack before the prologue. This is almost certainly **uninitialized stack contents**, not a meaningful value being saved. The likely intent is that `+0x52c` is not the field being purposefully written here; the write may be dead or vestigial.

**YELLOW — Unverified**: Confirm by checking whether `+0x52c` is read downstream. Struct task #11 covers this.

### Fields modified in the reset block

| Offset | Size | Operation | Meaning (confirmed vs unverified) |
|---|---|---|---|
| `+0x6df` | 1 byte | Cleared to 0 | Gate flag — cleared on IC apply (verified) |
| `+0x540` | 4 bytes | Set to 0 | Unknown; task #11 |
| `+0x528` | 4 bytes | Set to `g_CurrentFrameCounter` | Frame timestamp; task #11 |
| `+0x52c` | 4 bytes | Set to `local_8` (likely stack garbage) | Likely vestigial; task #11 |
| `+0x530` | 4 bytes | Set to 0 | Unknown; task #11 |

### `TechnoClass__IronCurtain` delegate

Called unconditionally regardless of the gate check. The gate block only runs additional building-specific resets before the shared IC apply logic. The base `TechnoClass__IronCurtain` (at `0x0070e2b0`, verified via `get_function_callees 0x00457c90`) writes:
- `+0x18c` = current frame
- `+0x190` = (stack artifact, not source_house)
- `+0x194` = duration
- `+0x1a4` = 0
- `+0x1c4` = `is_force_shield` bool

## Callers

`get_function_callers 0x00457c90` returned empty (vtable-dispatched). `get_xrefs_to 0x00457c90` returned: DATA xref from `0x007e4010` — the BuildingClass primary vtable slot. This function is called exclusively through the vtable dispatch from the IronCurtain super-weapon apply path.

BuildingClass vtable base: `0x007e4000` (verified: constructor at `0x0043b740` sets `*this = 0x007e4000` per `get_assembly_context`). IronCurtain slot at byte offset `0x10` from vtable base = slot 4 in the overall table layout at that base, but the absolute vtable slot index in the full inheritance chain is 85 (offset `0x154` / 4).

## Out-of-scope references

- `BuildingClass+0x6df`, `+0x528`, `+0x52c`, `+0x530`, `+0x540` — covered by task #11 (decode-struct-BuildingClass_IC_fields)
- `g_CurrentFrameCounter` — covered by task #14
- `TechnoClass__IronCurtain` — covered by task #2 (completed)

## Active in YR: Yes

The vtable slot at `0x007e4010` is populated and the call chain from the super-weapon dispatch reaches it. No TS-legacy gate found.

## TS-legacy assessment: Not TS-legacy

The function is short, directly accessible via vtable, and the referenced fields are in the active BuildingClass layout range. No TS-only flags gate it.
