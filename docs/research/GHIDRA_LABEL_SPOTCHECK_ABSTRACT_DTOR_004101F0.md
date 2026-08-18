# Ghidra Label Spot-Check — AbstractClass destructor pair

**Address(es):** `0x004101F0`, `0x004105A0`, vtable `0x007E1F50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** One label-correctness test: whether `0x004101F0` is a scalar-deleting destructor or only the vtable-reset body.
**Non-Scope:** Full AbstractClass layout, ObjectClass destructor chain, pending-delete drain.
**Confidence:** High for this slice.
**Active in YR:** Yes as compiler-generated destructor/vtable infrastructure; gameplay side effects are in derived destructors and callers.

## 1. Overview

This spot-check tested one known label-pollution claim from `ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md`: the older claim that `0x004101F0` was the scalar-deleting destructor. The current Ghidra decompile names it `AbstractClass__Destructor_ResetVtables`, and the function body supports that current label.

Result: the old scalar-deleting-destructor label is wrong. `0x004101F0` only resets four AbstractClass vtable pointers and returns. The scalar-deleting wrapper is vtable slot 8 at `0x004105A0`.

## 2. Class Layout / Key Offsets

| Address / slot | Verified role | Evidence |
|---|---|---|
| `0x007E1F50` slot 0 | QueryInterface-like entry `0x00410260` | `read_memory 0x007E1F50 len 48` |
| `0x007E1F50` slot 8 / offset `+0x20` | scalar-deleting destructor pointer `0x004105A0` | vtable bytes decode to dword `A0 05 41 00` at slot 8 |
| `0x004101F0` | destructor body that resets vtables only | decompile + raw bytes |
| `0x004105A0` | scalar-deleting destructor wrapper | decompile + raw bytes |

## 3. Core Logic

`0x004101F0` decompiles to a fastcall-like helper that writes:

- `this+0x00 = 0x007E1F50`
- `this+0x04 = 0x007E1F34`
- `this+0x08 = 0x007E1F2C`
- `this+0x0C = 0x007E1F24`
- returns immediately

Raw bytes confirm four `MOV [ECX+offset], imm32` stores followed by `RET`:

`C7 01 50 1F 7E 00 ... C7 41 0C 24 1F 7E 00 C3`

There is no conditional flag test and no call to the free helper in this function.

`0x004105A0` decompiles to the scalar-deleting pattern:

- reads the delete flag byte from the stack
- resets the same four vtable pointers
- if `(flag & 1) != 0`, calls `FUN_007C8B3D(this)`
- returns `this`

Raw bytes show the flag test and free-helper call:

`8A 44 24 04 ... A8 01 ... 74 09 56 E8 71 85 3B 00 ... C2 04 00`

`0x007C8B3D` decompiles as a thin wrapper that calls `FUN_007C93E8`, consistent with the operator-delete helper role cited by existing destructor reports.

## 4. INI Keys

None. This is compiler/runtime object infrastructure, not data-driven behavior.

## 5. Integration Points

The primary AbstractClass vtable at `0x007E1F50` has slot 8 pointing at `0x004105A0`, not `0x004101F0`. That means calls through the destructor vtable slot reach the scalar-deleting wrapper.

`0x004101F0` is still a real destructor-body helper, but naming it "scalar-deleting destructor" is misleading because it has no optional deallocation behavior.

## 6. Current Rust Implementation Status

No direct Rust implementation should mirror this raw destructor mechanism. For Rust-facing substrate design, the important fact is negative: do not infer gameplay cleanup or free timing from `0x004101F0`; it only resets vtables. Pending-delete/free behavior belongs to the concrete destructor chain and scalar-deleting wrapper paths.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x004101F0` body | verified | `decompile_function`, `read_memory` | none for label role |
| `0x004105A0` body | verified | `decompile_function`, `read_memory` | none for label role |
| AbstractClass vtable slot 8 | verified | `read_memory 0x007E1F50 len 48` | none for slot pointer |
| `FUN_007C8B3D` free helper | touched-not-exhausted | decompiles to `FUN_007C93E8(param_1)` | exact allocator internals out of scope |
| Derived destructor chains | deferred | not needed for this label test | use object-derived destructor reports |

## 8. Open Questions — Final State

- `[RESOLVED] OQ-001 — Is current Ghidra's role for 0x004101F0 consistent with the function body? → Yes; current decompile label `AbstractClass__Destructor_ResetVtables` matches the four vtable stores and no free call.` (evidence: `0x004101F0`)
- `[RESOLVED] OQ-002 — Does the AbstractClass vtable point slot 8 at 0x004101F0? → No; slot 8 points at 0x004105A0.` (evidence: `read_memory 0x007E1F50 len 48`)
- `[RESOLVED] OQ-003 — Does 0x004105A0 have scalar-deleting behavior? → Yes; it tests bit 0 of the flag and calls `FUN_007C8B3D(this)` when set.` (evidence: `0x004105A0`)
- `[DEFERRED] OQ-004 — What exact allocator/free implementation is under FUN_007C93E8?` (category: out-of-scope; reason: not needed to classify the destructor label; next-step-if-pursued: inspect allocator/free reports or decompile `0x007C93E8`)

## 9. Visual/UI Composition Ledger

Not applicable.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x004101F0` only resets AbstractClass vtables and does not free memory. | `0x004101F0` decompile/raw bytes | none; Rust should not model raw vtable resets | future object-substrate docs/designs | Treat as destructor-body infrastructure only. | A substrate design must not use `0x004101F0` as evidence for pending-delete/free timing. | Do not call it a scalar-deleting destructor. |
| AbstractClass scalar-deleting destructor is vtable slot 8 at `0x004105A0`. | vtable `0x007E1F50` slot 8; `0x004105A0` body | none | future object-substrate docs/designs | Use `0x004105A0` when discussing AbstractClass slot-8 delete wrapper. | Vtable slot audit decodes slot 8 to `0x004105A0`, and body has optional free. | Do not infer slot-8 target from local function proximity or names. |

## Sources

- Ghidra MCP: `decompile_function 0x004101F0`
- Ghidra MCP: `decompile_function 0x004105A0`
- Ghidra MCP: `read_memory 0x007E1F50 length 48`
- Ghidra MCP: `read_memory 0x004101F0 length 64`
- Ghidra MCP: `read_memory 0x004105A0 length 96`
- Ghidra MCP: `decompile_function 0x007C8B3D`
- Existing docs searched: `ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md`, `ABSTRACTCLASS_GHIDRA_REPORT.md`
