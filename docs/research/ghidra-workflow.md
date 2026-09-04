# Ghidra evidence and annotation

[ENGINE.md](../../ENGINE.md) owns scope, exactness and implementation authority.
This reference preserves the binary-specific pitfalls and mutation protocol.

## Establish the claim

Start at the production owner. Follow relevant callers/callees, input writers,
output consumers, initialization and teardown. Establish active-YR reachability
from gates, defaults and retail data; inherited TS code is not automatically active.
Stock traps include disabled fog-of-war (shroud is active), subterranean locomotion
and dormant `SpecialFlags` branches. Low-bridge `TubeClass` movement is active.

For new findings cite the address and reproducible decompile/memory read, including
the caller/data binding proving its role. Distinguish direct evidence, cited research
and inference. Resolve contradictions; name uninspected paths instead of overstating
coverage. Labels, YRpp and displayed signatures are navigation hints.

## Fragile interpretations

- **Pointer offsets:** check the actual base type. Integer-address
  `*(param_1 + 0x98)` uses bytes; `int *` indexing `param_1[0xac]` means
  `0xac * 4 = 0x2b0` bytes. An explicit cast/add to an integer address uses bytes.
- **Boundaries:** inspect adjacent instructions, incoming edges, tail calls and stack
  balance. Shared epilogues, hot/cold regions and thunks can mislead the decompiler.
- **Signatures:** recover receiver/ECX, stack arguments/cleanup and returns from callers.
  Account for hidden returns and adjusted `this`; derive widths/signedness from instructions.
- **Vtables:** prove owner/subobject through Complete Object Locator, TypeDescriptor,
  hierarchy and displacement; read the 32-bit slot, follow adjustor thunks and verify
  real callers' receiver shape. Positional labels are not identity proof.
- **Lifecycle:** distinguish vptr staging, deleting-destructor flags, subobject calls,
  array cookies and conditional frees from gameplay without losing lifecycle effects.
- **Runtime tables:** locate writers; zero-filled image bytes need not reflect initialized data.

Use assembly/bytes when pseudocode is ambiguous. Do not change types, prototypes or
boundaries merely to make output plausible.

## Visual/audio evidence and Rust provenance

Trace the full relevant paint/render handler through return, including layers after
helper calls: order, flags, assets/frames, rectangles, anchors, clipping and palettes.
Loaded is not drawn; unused for one role does not mean invisible. Inspect enough
variants to establish actual selection. Plausible palette previews, body-only voxels
and successful parsing are not full render proof. Follow audio admission/selection
through timing and playback. Reopen paths contradicted by runtime observations.

Use a nearby comment for each cohesive native-derived Rust behavior:

```rust
// gamemd: <verified owner>::<function> @ <address> — <behavior>.
// Source: <document/section or live body and caller evidence>.
```

Never invent placeholder identities. Use `FUN_<address>` plus the established role
and unknown owner when necessary. Architecture glue needs no fictitious provenance;
unproven internal behavior uses ENGINE's explicit UNCHECKED label.

## Access and write authority

For connection-refused, empty instances or no program, use local `ghidra-up` when
available, then retry. Avoid duplicate instances, routine re-imports and Auto Analyze.

Metadata synchronization requires a selected skill description promising it,
`--sync-ghidra-labels`, or a direct user request. Read-only requests and
`--no-sync-ghidra-labels` override defaults. Document correction alone grants no
Ghidra writes. Workers always keep Ghidra read-only; the authorized root/sole agent
writes serially after all readers stop. Without authority, report candidates.

Routine permission covers certain function/global labels, evidence comments and
proven missing memory references. Function creation/boundaries, prototypes, structs,
field/type edits, variable renames and byte patches need separate per-task authorization.

## Mutation protocol

Prove boundary, behavior, owner/receiver and caller/data binding before naming;
otherwise retain `FUN_*`/`DAT_*`. Use `ClassName__MethodName` only for proven identity.

For `add_memory_reference`, decode source bytes and exact target/operand, prove reference
kind and check for an existing equivalent. Bind `from_address`, `to_address`,
`operand_index`, `ref_type` and `USER_DEFINED` source type accordingly. Do not add
generic xrefs or delete analyzer-created references during routine mapping.

After **each mutation**, including comments: `save_program` → read back → continue.
Never batch saves; report failed/unapplied changes.

Analyzer changes affect interpretation, not executable bytes, actual slots or native
fields. Inspect local analysis settings and binary/database identity; do not assume
historical settings persist or bulk-sync labels from documents. Missing work after
restart calls for save/persistence investigation.