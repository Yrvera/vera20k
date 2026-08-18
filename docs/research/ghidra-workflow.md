# Ghidra decompilation workflow & label discipline

Reference material relocated from `ENGINE.md`, which keeps only the non-negotiable rules and
points here. The **Ghidra MCP server** is connected to `gamemd.exe`. Always prefer live
decompilation via MCP over static reports — it allows tracing xrefs, reading vtables,
inspecting memory, and following call chains interactively. Authenticity matters more than
speed. If tool calls fail with connection-refused, an empty `list_instances`, or "No program
loaded", Ghidra isn't running — the MCP connects, it cannot launch. Relaunch before retrying.

**CAUTION:** gamemd.exe contains massive amounts of inherited Tiberian Sun code that is
dead or dormant in YR. When decompiling, always trace callers to confirm the code path
is reachable in a normal YR skirmish. See "Tiberian Sun legacy" below.

## Evidence rules for reverse engineering

- **Asked to study, research, inspect, or investigate → analysis only**, no implementation
  unless explicitly requested.
- **Never invent** offsets, addresses, vtable slots, fields, enum values, or labels. Not
  verified this session → `UNKNOWN`/`UNCHECKED`. Always separate verified from inferred.
- Treat Ghidra names and decompiler symbols as **navigation hints only** — this project
  carries stale labels, and YRpp is not ground truth. Verify load-bearing claims from the
  function body, assembly, callsites, receiver/`this` pointer, argument flow, vtable slot
  bytes, data references, and active-YR reachability. Prefer address plus verified role over
  label name; record label drift when found.
- **Cite the decompile/read-memory call inline** for every address, offset, or slot written
  into a doc ("verified via `decompile_function 0x00520A60`"). Unverifiable claims go in a
  separate UNVERIFIED section, never mixed into the verified body. Prior agent prose without
  cited evidence is not verification.
- **Treat your own prior claims as unverified.** Asked "are you sure?" → re-check from the
  binary rather than restating. Audit in two passes: enumerate claims without editing, then
  verify and classify against current binary evidence.
- Verify plan anchors before dispatching parallel work — one wrong address contaminates every
  downstream report.
- **Record proven low-risk metadata as annotation candidates in the same session** —
  certainty-gated function/global labels, evidence comments, and proven missing memory
  references. Apply them only when synchronization is authorized below. Types, function
  boundaries, prototypes, structs, field edits, and variable renames require separate
  explicit per-task authorization. Inferred findings stay unlabelled; a confident wrong
  label is worse than none.

## Tiberian Sun legacy — the most common mistake

The binary inherited a large TS codebase; entire systems, struct fields, and branches are
dead or irrelevant in YR, and implementing TS-only logic is the single most frequent error
when working from decompilation. For any behavior found: is it gated behind a flag whose YR
default is off? Is the path reachable in a normal YR skirmish? Does it have a visible effect
in standard YR? When in doubt, test in the original or ask — don't implement speculatively.
Even where a path is live, its internals may be awkward TS plumbing; only observable output
matters. Always note whether a feature is active by default in YR.

**Known dormant, do NOT implement as default:**
- **Fog of war** — `[MultiplayerDialogSettings] FogOfWar` defaults to `false`. Once explored,
  a cell stays fully visible. Implement shroud only (black for unexplored).
- **Subterranean locomotion** — TS legacy, not in RA2 or YR; skip it in audits and scans. Do
  not conflate it with low-bridge `TubeClass` movement, which *is* active YR behavior.
- Many `SpecialFlags`-gated features and unused mission/trigger actions.

## The mapping pass — the standard opening move for porting a system

**Before porting a system, map its functions and globals. Then port.** The reading has to
happen anyway. Record candidates even when the current workflow is not authorized to sync;
an authorized sync makes those verified facts durable in the Ghidra database.

Evidence this is worth the serial cost, from the 2026-08-06 refinery/miner and draw-anchor
work: the occlusion projection behind the nearby-cell classifier, the search's selection
tail, and two path-search callers were each worked out independently by different lanes,
three or four times over, because none of them carried a name. In the same session, two
functions that *did* carry verified plate comments — `BuildingClass::GetCoords` and the
direction-table initialiser — were trusted-then-checked in seconds rather than re-derived,
and one of those comments prevented a live trap (the table is zero-filled in the image and
written at runtime, so a raw `read_memory` of it is meaningless).

**Bound the pass by entry points, not transitive closure.** Start from the functions the
port actually needs — the mission handler, the INI reader, the virtual the production loop
dispatches — and expand only where a callee is load-bearing. "The system" balloons
otherwise, and a pass that never finishes maps nothing.

**The mapped set is the scope.** "These are the functions and globals that make up the
harvest loop" is itself the artifact: it tells you when you have ported the *system* rather
than a slice. When synchronization is authorized, accepted labels and comments make that
set directly navigable in Ghidra; otherwise the candidate ledger preserves it for review.

**A name is a claim, so carry the confidence in the annotation.** The certainty gate below
is not relaxed by doing the pass earlier. What the pass produces:
- proved → a `ClassName__MethodName` candidate plus a dated plate-comment candidate stating
  what was verified and how;
- understood-but-unproved → keep `FUN_*`; a comment candidate may state only the verified
  partial fact, the uncertainty, and the specific call that would settle it;
- guessed → nothing. Silence beats a plausible wrong name; that is how
  `g_refinery_unload_adjacent_lookup_dx` (which is nothing of the sort — it is entry 6 of the
  eight-direction table) has been misleading readers.

**Write down the writer, not just the readers.** The 2026-08-06 coordinate-frame error — a
building's `Location` documented as the NW cell origin when the map loader stores the NW cell
*centre* — survived across several investigations because everyone read the consumers
(`GetCoords`, `Get_Cell_Packed`) and nobody opened the function that establishes the value.
For any field a port depends on, identify the function that writes it and include that
writer in the mapping ledger. Label it only through an authorized synchronization pass.

**The pass is candidate-first and any mutation is single-writer and serial.** Ghidra reads
can fan out; writes cannot. Workers report candidates only. An authorized root or standalone
investigator waits for every reader to stop, re-verifies accepted candidates, and applies
them one at a time under the save/readback discipline below.

## Synchronization authorization

Reporting annotation candidates is part of reverse-engineering work. Mutating Ghidra is
authorized only when at least one of these is true:

1. the selected skill's user-facing description explicitly promises Ghidra synchronization;
2. the invocation includes `--sync-ghidra-labels`; or
3. the user directly requests Ghidra synchronization in plain language.

`--no-sync-ghidra-labels` or any request for read-only work overrides those permissions.
Workers and parallel readers never mutate Ghidra. If synchronization is not authorized,
finish with a candidate ledger; do not treat that as incomplete work. If it is authorized,
the root or sole agent applies candidates serially after all readers stop. Skills should
reference this section instead of copying the full certainty gate into their own prompts.
The compatibility flag name `--sync-ghidra-labels` covers the whole low-risk metadata set:
verified function/global labels, evidence comments, and proven missing memory references.

## Annotation best practices

**Only label what is proved.** A wrong label is worse than `FUN_*` — it misleads every
future session. Before renaming a function, prove its boundary, behavior, owner/receiver,
and relevant caller or data binding from the live binary. Existing names, Rust comments,
docs, YRpp, and neighboring patterns are navigation only. If any identity or binding fact
is uncertain, leave it as `FUN_*` and note the address in `docs/research/` instead.

When you do label: rename via `rename_function_by_address` using `ClassName__MethodName`,
rename identified helpers, then `save_program` and read back. Do not create a missed
function boundary without explicit per-task authorization. Don't bulk-label vtable methods
positionally and don't label from guesses — decompile and bind first.

**Cross-reference discipline.** `add_memory_reference` is an annotation mutation, not a
shortcut around uncertain analysis. Add one only after reading the source instruction or
table-slot bytes, proving the exact target and operand, selecting the correct reference kind,
and confirming that Ghidra does not already hold the equivalent reference. Names, docs,
neighboring table patterns, and plausible control flow are not proof. If either endpoint or
the reference kind is uncertain, report the candidate and leave the database unchanged.
Routine mapping never deletes analyzer-created references.

For `add_memory_reference`, bind every argument from that proof: `from_address` is the
verified instruction/table-slot address, `to_address` is the exact decoded target,
`operand_index` is the operand that encodes it, `ref_type` matches the proved access kind,
and `source_type` is `USER_DEFINED`. Do not use a generic DATA/READ/CALL kind merely to
make an xref appear.

**Save discipline — call `save_program` immediately after every mutation.** Every
`rename_function_by_address`, `create_label`, `set_plate_comment`,
`set_decompiler_comment`, `set_disassembly_comment`, or `add_memory_reference` — plus any
separately authorized structural/type edit — must be
followed by `save_program` before moving on. Do NOT batch saves at the end of a session.
The MCP server can disconnect, the Ghidra UI can crash, and parallel sessions can clobber
unsaved state — any of these silently loses every label applied since the last save. The
pattern is: mutate → save → verify the rename appears (e.g., via `get_function_by_address`)
→ only then move on.

**Finding ReadINI functions:** search for a known INI key string → follow xref into the
ReadINI function → if its boundary is missing, record it unless function creation was
explicitly authorized → each `ReadBool/ReadInt/ReadString` call reveals an INI key name
and its struct offset.

`RTTI_VTable_Labeler.java` at `C:\Users\enok\ghidra_scripts\` is already run.

## Label drift — the RTTI analyzer / demangler are the source

Ghidra's **built-in auto-analyzers re-derive labels**, and they are the primary cause of
label drift across `docs/research/`. Two enabled-by-default analyzers do the damage:

- **`Windows x86 PE RTTI Analyzer`** — walks `.?AV` RTTI → COL → vtable and (re)labels classes, vtables, and virtual functions. Overlaps directly with `RTTI_VTable_Labeler.java`; you do not have to run the script to get re-labeling — Ghidra's own analyzer does it.
- **`Demangler Microsoft`** (with Apply Function Signatures/Calling Conventions) — (re)demangles symbol names and can rewrite signatures.

These run on **import** and on any explicit **"Auto Analyze" / "Analyze All"** — **not**
on simply opening the project or rebooting. So drift is event-driven: a past re-analyze
shifted function names / vtable-slot indices, and research docs that copied a name once
then went stale (doc says `Mission_Enter`, binary now says `PerCellProcess`). The
2026-05-28 corpus audit found this across dozens of docs (`RTTI_LABEL_DRIFT`), e.g.
`0x005196A0` = `InfantryClass::PerCellProcess`, not `Mission_Enter`.

**These analyzers have been disabled in the gamemd.exe analysis options and the program
saved, so labels are now frozen.** Implications:

- **Do not casually re-import gamemd or click "Analyze" / "Analyze All".** That is what re-fires the RTTI analyzer + demangler and reshuffles labels. There is no reason to re-analyze an already-analyzed, fully-labeled DB.
- **Authority order is binary → Ghidra → docs.** The binary is unchanging truth; docs are the durable record; Ghidra labels are mutable *scaffolding*. A wrong label is worse than `FUN_*`. When a label and a doc disagree, re-verify against the live binary — never sync docs→Ghidra wholesale (docs are downstream).
- **If labels look wrong after a restart**, suspect **unsaved state** (work not `Ctrl+S`'d before closing) or a re-import — not the docs. Verified 2026-05-28: saved labels persist identically across a full Ghidra restart, so a per-startup wipe is a save/persistence problem, not analysis drift.
- Record candidates as you decode. When synchronization is authorized, apply verified
  metadata one item at a time with immediate save/readback. Do not run bulk
  auto-labeling/relabeling against gamemd to "fix" drift — that re-creates it.

## Decompilation pitfall: param_1 pointer arithmetic

CRITICAL: When extracting struct field offsets from Ghidra decompilation, always check
the `param_1` type in the function signature:
- `param_1` is `int` → offsets like `*(param_1 + 0x98)` are **direct byte offsets**
- `param_1` is `int *` → indexing like `param_1[0xac]` means **byte offset = 0xac × 4 = 0x2B0**
- `*(type *)((int)param_1 + 0x372)` is always a **direct byte offset** regardless of param type

Getting this wrong produces silently incorrect struct layouts. WeaponTypeClass and
BulletTypeClass use `int` (safe). AnimTypeClass uses `int *` (must multiply by 4).

## Decoder ring: when the decompile looks structurally wrong

(Absorbed from the retired `re-decoder-ring` skill, 2026-08-02. Escalation guide for
suspicious output — function boundaries, calling conventions, RTTI identity, thunks,
ctor/dtor plumbing. The core rule is triangulation: no local name, decompiler
signature, or attractive pseudocode is authoritative by itself.)

**Evidence stack** — use as many layers as the claim requires; if two disagree,
explain the disagreement before recording anything: (1) raw bytes and instruction
boundaries, (2) assembly-level register/stack/branch behavior, (3) the decompiled
body treated as a *rendering* of the first two, (4) direct callers/callees including
argument and receiver flow, (5) data refs, imports, strings, globals, (6) RTTI /
Complete Object Locators / vtables / subobject displacement, (7) ctor/dtor and
allocation patterns, (8) active-YR reachability.

**Function boundaries.** Suspect a bad boundary when the prologue/epilogue is
inconsistent, a branch lands inside another function, stack balance only works
across the boundary, or callers target a nearby byte instead of the named entry.
Inspect bytes on both sides, follow every incoming edge including tail calls,
distinguish hot/cold splits and shared epilogues from separate functions, and treat
tiny jump-only bodies as thunks until the destination and receiver adjustment are
understood. Do not infer a new boundary solely because the decompiler failed.

**Calling conventions.** Recover the effective signature from callsites: track
ECX/receiver setup, stack pushes, return cleanup, and returned registers. Check
whether an apparent argument is actually a hidden return pointer, adjusted `this`,
vbase displacement, or compiler-generated state. Verify widths and signedness from
the consuming instructions, not the displayed C type; compare multiple callers —
one unusual wrapper can hide the normal convention. (Pointer-index scaling: see the
param_1 pitfall section above.)

**Vtable/slot claims.** Locate the table's COL, resolve the TypeDescriptor and
hierarchy to establish owner/subobject, account for COL offset and constructor
displacement, read the 32-bit entry at `slot_index * 4`, follow adjustor thunks to
the ultimate body, and confirm representative virtual callsites use the same
receiver shape. An address *near* a named table, or a label inherited from a prior
analyst, is not class identity proof.

**Ctor/dtor/thunk plumbing.** Compiler-generated object code often looks like
gameplay logic. Before assigning semantics check for staged vptr writes, scalar/
vector deleting-destructor flags, base-subobject calls with `this` adjustment,
array cookies and conditional frees, and jump thunks that only normalize the
receiver. Separate lifecycle plumbing from active game behavior in the handoff.
SEH/EH setup, unwind helpers, security cookies, and import thunks likewise distort
the decompile — identify runtime scaffolding before interpreting state writes as
game semantics.

**Label-drift test.** When a local symbol name is load-bearing, independently
answer: what is the receiver, which callers reach it in active YR, which fields
does the body consume, which vtable/COL or ctor evidence owns it, and does the
alleged role explain all representative callsites? If the name fails, cite the
address and verified role and record the drift explicitly — never silently reuse
the polluted name.
