# Ghidra evidence and annotation reference

Read this before binary work. [ENGINE.md](../../ENGINE.md) defines the parity standard,
project scope and implementation authority. Use this reference for fragile evidence
questions and Ghidra operations, not as a compulsory investigation checklist.

## Establish behavior

Start from the production entry point for the requested scenario. Read the body,
relevant callers and load-bearing callees; follow inputs to their writers and outputs
to their consumers. Include initialization, state transitions and teardown when they
affect the claim. A helper's correctness does not prove it is reached by the game.

Use decompilation for navigation, then assembly/bytes when types, arithmetic, branching
or bindings are ambiguous. Ghidra labels, inferred signatures, YRpp and previous prose
are hints. A credible name does not establish receiver identity or behavior.
Do not fabricate addresses, offsets, enum values, vtable slots or native names.

For new findings, cite the address and relevant decompile/memory read inline, including
the caller/data binding that establishes the claimed role. Distinguish direct evidence
from inference and reuse of a named research document. Recheck uncertain or conflicting
inherited claims rather than silently promoting them to new verification. A challenge
to a claim warrants checking the evidence, not repeating the claim.

Use the evidence depth the claim needs. A narrow question can have a narrow answer;
an exhaustive investigation must account for the requested active paths and report
uninspected or unresolved ones. No finding/function quota proves completeness.
Unknowns stay unknown. Record useful annotation candidates as evidence is established.

## Active Yuri's Revenge versus inherited code

Confirm that the behavior is enabled and reached in the target game/mode/scenario.
Read relevant defaults, flags, cases, retail INIs and callers. Code present in
`gamemd.exe` may be inherited, dormant or repurposed; absence from one observed
path does not prove global inactivity. State the reachability scope of exclusions.

Known stock-skirmish traps include disabled fog-of-war behavior (shroud is active),
subterranean locomotion inherited from TS, and many `SpecialFlags` paths.
Low-bridge `TubeClass` movement must not be excluded merely because it resembles
subterranean plumbing. Recheck the specific branch when its activation matters.

Preserve downstream state, RNG, timing, ordering and lifecycle semantics even when
a detail looks internal. The scale exception permits different storage, not arbitrary
behavior changes. Verify defaults in the main checkout's retail INIs; YR does not
load RA2 base INIs as an implicit layer below the `*MD.INI` files.

## Reading suspicious decompilation

- **Pointer arithmetic:** determine the actual base type and access width.
  `*(param_1 + 0x98)` with an integer address means a byte displacement;
  `param_1[0xac]` with `int *` means `0xac * 4 = 0x2b0` bytes.
  `*(type *)((int)param_1 + 0x372)` has an explicit byte displacement.
  Do not memorize one class's current decompiler type as a universal rule.
- **Function boundaries:** inspect instructions on both sides, incoming edges,
  tail calls and stack balance. Shared epilogues, hot/cold regions and jump thunks
  need not be separate gameplay functions. A decompiler failure alone proves no boundary.
- **Calling convention:** recover ECX/receiver setup, stack arguments/cleanup and
  returns from callsites. Account for hidden return pointers and adjusted `this`.
  Resolve widths and signedness from consuming instructions, not just displayed C types.
- **Vtables:** for an owner/slot claim, locate the table's Complete Object Locator,
  TypeDescriptor and relevant hierarchy/subobject offset; read the 32-bit slot bytes,
  follow adjustor thunks and check the receiver used by real callers.
  Nearby labels and positional guesses are not identity evidence.
- **Construction/destruction:** staged vptr writes, deleting-destructor flags,
  subobject calls, array cookies and conditional frees can be compiler plumbing.
  Separate them from gameplay effects without dropping lifecycle consequences.
- **Runtime data:** static image bytes do not describe a table populated during startup.
  Find its writers before interpreting zero-filled memory or treating contents as constant.

When layers disagree, resolve the discrepancy between bytes, assembly, decompile,
callers, data references and object identity before writing a confident conclusion.
Do not repair types, prototypes or boundaries merely to make the output look plausible.

## Visual and audio claims

For visual parity, start at the relevant paint/render handler and follow composition
through return, including layers drawn after helper calls. Establish draw order, selected
assets/frames, source/destination rectangles, anchors, clipping, palette/conversion path,
and the flags enabling each layer. Inspect the relevant asset variants; if the selected
frame is unknown, inspect enough frames or native selection logic to resolve it.

Loaded is not drawn, and unused for one role does not mean invisible in every role.
An asset-browser preview can choose a plausible but incorrect palette; body-only voxel
output and successful parsing are not full render proof. For audio, trace event admission,
selection, timing and the actual playback consumer. If screenshots or runtime evidence
contradict the account, reopen the composition/trigger path.

## Provenance in Rust

Place one nearby comment per cohesive gamemd-derived behavior, not one per line:

```rust
// gamemd: <verified owner>::<verified function> @ <exact address> — <behavior>.
// Source: <research document/section or live decompile and caller evidence>.
```

The angle-bracket fields are placeholders, not names to invent. If only the address and
role are established, use `FUN_<address>` and describe the verified role; mark owner
identity unknown. Pure Rust architecture glue does not need a fictitious native owner.
An internal behavior without an established native equivalent is explicitly
`VERA-internal, gamemd equivalent UNCHECKED` and cannot support a parity claim.

## Ghidra access and mutation authority

On connection-refused, empty instances or no loaded program, use the local
`ghidra-up` skill when available to reconnect, then retry. Do not launch duplicate
Ghidra instances or re-import the binary as a routine recovery step.

Investigations may report annotation candidates. Applying them is authorized only by:

- a selected skill whose description explicitly promises Ghidra synchronization;
- `--sync-ghidra-labels`; or
- a direct user request to synchronize Ghidra.

`--no-sync-ghidra-labels` and read-only requests override those defaults. A request to
correct a document alone grants no Ghidra writes. Workers are always read-only in Ghidra.
After all readers stop, the authorized root/sole agent applies accepted candidates
serially. Without synchronization authority, a candidate report is a complete deliverable.

This permission covers only certain function/global labels, evidence comments and
proven missing memory references. Function creation/boundaries, prototypes, structs,
field/type edits, variable renames and byte patches require separate explicit per-task
authorization. Never infer permission for them from “fix labels.”

## Certain metadata, saved immediately

Before naming a function, prove its boundary, behavior, owner/receiver and relevant
caller/data binding from the live binary. Keep `FUN_*`/`DAT_*` when identity is uncertain.
Use `ClassName__MethodName` only for an established identity. A comment may describe a
proven partial fact while explicitly recording what is unknown.

For `add_memory_reference`, read the source instruction/table bytes, decode the exact
target and operand, choose the proved reference kind, and check for an existing
equivalent reference. Bind `from_address`, `to_address`, `operand_index` and `ref_type`
from that evidence; use `USER_DEFINED` source type. Do not add a generic reference merely
to make an xref appear, or delete analyzer-created references during routine mapping.

After **each mutation**: call `save_program`, read back the changed metadata, then
continue. This includes comments and separately authorized structural edits. Do not
batch saves at the end. Report failures and unapplied candidates accurately.

## Label drift and local analyzer state

Names, signatures, function boundaries and inferred structures can change when Ghidra
analyzers or scripts run. Relabeling does not move the executable's bytes, actual
vtable entries or native object fields. Distinguish changed interpretation from
changed binary identity; record the program/build being analyzed when comparing evidence.

Do not casually run Auto Analyze, bulk relabeling, or docs-to-Ghidra synchronization.
RTTI analysis and Microsoft demangling can change metadata; inspect local analysis
options instead of assuming their historical settings still hold. If work disappears
after restart, investigate save/persistence and database identity before inventing a
new account of native behavior.