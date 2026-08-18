# Ghidra Project Health Audit — gamemd.exe / testProsjekt

Date: 2026-07-12
Method: 5 parallel read-only Ghidra lanes + parent spot-checks. No mutating Ghidra calls,
no renames, no saves. Every claim below carries the verifying MCP call inline.

## Verdict

**The Ghidra project is fundamentally sound but carries three real, characterizable
"wrong-ground-truth" hazards that look authoritative and have already leaked into research
docs.** The setup itself (image base, segments, analysis completeness) is clean, and — critically
— the RTTI/vtable verification substrate the whole anti-pollution methodology depends on is
100% intact. The hazards are all in *applied project state that looks trustworthy but isn't*:
data-type-manager struct field **names**, a **clustered** batch of heuristic function labels,
and vtable-slot label **class prefixes**. None is "the decoding is broken"; all are "a specific
authoritative-looking surface lies, and you must verify it the way CLAUDE.md already prescribes."

Key reconciliation: a random 18-function label sample came back **clean** (Lane 2) while a
targeted duplicate-name sweep found **~80–90 badly-wrong labels** (Lane 5). That is the whole
story in one sentence — **drift is not uniform noise; it is concentrated in automated-labeling
batches** that random sampling misses and per-system doc audits keep hitting.

---

## Failure Class A — Persistent state that looks authoritative and is wrong (the dangerous ones)

### A1. The data-type manager's struct field NAMES are YRpp/Ares-imported, not binary-derived
`get_struct_layout TechnoClass` returns field names in verbatim YRpp/Ares convention
(`nSmoothedHealth`, `RockingSidewaysPerFrame`, `DisguisedAsHouse`, `MindControlledBy`). CLAUDE.md
explicitly forbids trusting YRpp as ground truth, yet `get_struct_layout` serves those names with
no "unverified" signal.

- **Offsets sampled are reliable** (11/11 matched the binary), but **names are not** (3 wrong in
  ~15 checked). Treat DTM field *names* as YRpp hints; *offsets* as strong-but-verify.
- **CellClass +0x11A "Height" is wrong.** The binary writes `row*width+col` (an iso sub-tile index)
  there — verified via `disassemble_function 0x0057B440` (`MOV byte ptr [ESI+0x11a],BL` in
  `MapClass::ApplyBridgeTile`). The **real elevation byte is the adjacent +0x11B "Level"**, which
  `CellClass::GetGroundHeight` (185 xrefs) actually reads — verified via
  `disassemble_function 0x0047b3a0` (`MOVSX EDX, byte ptr [ESI+0x11b]`). Classic one-field-off trap:
  trusting the name reads the wrong byte while its correctly-named neighbor holds the real value.
- **HouseClass power fields.** The real power accessors (`HasPowerOutput`/`HasPowerDrain`/
  `GetTotalPowerOutput`/`GetTotalPowerDrain`) read offsets 0x164/0x168, never the DTM-named
  `PowerOutput`@21412/`PowerDrain`@21416 (verified via `disassemble_function` on all four accessors).
  Reproducing the sidebar power bar from the DTM's named field reads the wrong data.
- **Mixed-header import artifact.** TechnoClass+0x70 (`nSmoothedHealth`) and its base
  ObjectClass+0x70 (`EstimatedHealth`) are two different DTM names for the identical inherited byte
  (verified via `get_struct_layout` on both).
- **Central classes are absent from the DTM entirely:** FootClass, UnitClass, and every `*TypeClass`
  (TechnoTypeClass, BuildingTypeClass, …) return empty from `get_struct_layout`/`search_data_types` —
  not wrong, just no safety net; 100% manual offset work for the classes the port leans on hardest.
- **Not everything flagged in wave-1 is actually wrong:** FactoryClass +0x34/+0x38 (TimeLeft/Duration)
  and siblings check out correct via `disassemble_function 0x004c9c70`; the earlier suspicion there
  does not hold.

**Impact:** `get_struct_layout` is the single most authoritative-looking, most-trusted surface, and
its field names have a high enough error rate that skipping verification on a load-bearing field is a
mistake. **Mitigation:** trust DTM *offsets* provisionally, verify field *names* against a real
accessor (`get_assembly_context` on the displacement) before writing them into a doc or the port.

### A2. Clustered function-label pollution from a heuristic idiom-matcher
An automated labeling pass named functions after an *internal idiom* they contain, not after what
the function *is*.

- **`CDFileClass__Constructor` is applied to 99 distinct addresses** (0x00401950 … 0x007b0490) —
  verified via `search_functions_enhanced name_pattern=CDFileClass__Constructor` (total=99). A genuine
  ctor is ~35 bytes; sampling found ~80% are 219 B–10.8 KB unrelated routines that merely *instantiate*
  a CDFileClass internally. Parent spot-check: 0x006951f0 has a 718-byte body
  (`get_function_by_address 0x006951f0`, body 0x006951f0–0x006954be) and is a CD-directory
  search/seed matcher (`decompile_function 0x006951f0`), not a constructor.
- **`CCFileClass__Constructor`** (5 addrs): ≥2 wrong, e.g. 0x00552d60 (5009 B) is a CD-check/
  language-select **UI draw screen** (`decompile_function 0x00552d60`), only caller
  `ScenarioClass__Full_Init`.
- ~80–90 badly-wrong labels concentrate in these two families.
- **Contrast — benign duplication:** `Blitter__Constructor` (×19), `GenericNode__Constructor` (×15),
  `MSAnim__Constructor` (×17), etc. are *correct* names; MSVC6 emits one physical ctor copy per
  translation-unit/template instantiation. The name is right, just ambiguous among N addresses.

**Impact:** anyone researching asset/CD/file-loading *by name* lands on ~4-in-5 wrong functions.
**Mitigation:** for any duplicate-name group above ~3–5 members, treat the name as an unverified hint —
pull body size and sanity-check against the known ~35–70 B ctor shape, or decompile, before citing.

### A3. vtable-slot label CLASS PREFIXES are unreliable once an intermediate class is in the chain
The vtable slot **data** is trustworthy; the `ClassName__Method` **prefix** on the resolved function
is not, past a class's own vtable length.

- UnitClass vtable+0x480 correctly holds `TechnoClass__Set_Destination` (0x00741970) and +0x484 holds
  `UnitClass__Scatter_Force` (0x00738970) — verified via `read_memory 0x007f60f0` + two
  `get_function_by_address`. Both in-bounds, both well-formed.
- But **TechnoClass's own vtable does not reach +0x480**: `read_memory 0x007f4de0` (TechnoClass
  vtable 0x007f4960 + 0x480) = 0x00709a30, a 2-byte pseudo-function (out-of-array garbage). So the
  "Set_Destination" slot is contributed by an intermediate class (FootClass, `vtable__FootClass @
  0x007e8c94`), **not** literally TechnoClass. A doc citing "TechnoClass::Set_Destination" as the
  *declaring* class asserts an ownership fact the vtable-size evidence contradicts.
- **Cross-sibling offset diffing is unsound past the shared prefix:** the same +0x480 offset lands on
  `TechnoClass__Set_Destination` (UnitClass), `UnitClass__EnterBuildingOrDock` (AircraftClass — a name
  that itself wrongly implies UnitClass ownership), and unnamed `FUN_0051aa40` (InfantryClass) — three
  unrelated virtuals (verified via `read_memory`+`get_function_by_address` on each vtable's +0x480).

**Impact:** false "override / no-override" conclusions when diffing sibling vtables byte-for-byte; wrong
declaring-class attributions in docs. **Mitigation:** bound each class's own vtable length empirically
before trusting a slot's class-prefix or comparing offsets across siblings; the RTTI hierarchy is the
authority, not the compound label.

### A4. One thiscall/fastcall convention mislabel (isolated in sample)
`TechnoClass__CloakingTick` @ 0x006FB740 is typed `__fastcall(ObjectClass*)`, but the body is
`__thiscall` (ECX=this, `RET 0x4`, no EDX second arg) — verified via `disassemble_function 0x006FB740`
+ `list_calling_conventions`. Only 1 of 4 sampled thiscall methods was mislabeled; likely isolated, not
swept exhaustively. **Impact:** a vtable-slot signature built from the printed C keyword under-reports
arity. **Mitigation:** read the RET-immediate + register usage from disassembly, never the printed
convention keyword, when reconciling vtable-slot signatures.

---

## Failure Class B — Inherent decompiler traps (mitigated by method, not fixable in setup)

### B1. Sub-object stride misread → phantom fields (reproduced exactly)
`decompile_function 0x005206B0` prints `param_1[1].field_0x16d` for a `TechnoClass*`; the real
instruction is `MOV AL,[EBP+0x68d]` (`disassemble_function 0x005206B0`), i.e. `0x520` (TechnoClass
size, `get_struct_layout TechnoClass`) `+ 0x16D = 0x68D`, a field **inside InfantryClass**, not
TechnoClass. Misreading the `[1]` index invents a phantom field at TechnoClass+0x16D (the reported
non-existent "IsAttacking"). **Mitigation:** whenever the decompiler prints `paramN[K].field_0xNNN`
with K≠0, the real offset is `K*sizeof(declared type) + 0xNNN`; resolve on the literal disassembled
displacement and treat the field as belonging to the actual runtime (derived) type.

### B2. Missing function boundaries on vtable adjustor thunks
`get_function_by_address 0x004B63B0` → "No function found," but `read_memory 0x004B6380` shows
well-formed x86 (an MI adjustor thunk: `SUB [ESP+4],imm; JMP`, no standard prologue). 6 entries of one
`ILocomotion`-family vtable (`.rdata` table @ 0x007e8258) lack Function objects while two neighbors have
them (six `get_function_by_address` calls). `find_code_gaps min_size=16` = 2919 gaps binary-wide (upper
bound; mostly padding/data, not all missed functions). **Root cause:** prologue-pattern function-start
detection can't latch onto no-prologue thunks reached only via indirect `CALL [vtable+off]`; re-running
auto-analysis would reproduce it. **Mitigation:** don't assume `get_function_by_address` succeeds for
every vtable slot — read raw table bytes and `disassemble` each target regardless.

### B3. Dead-parameter elision
`get_function_signature 0x006FB740` reports `param_count:0` though `RET 0x4` proves one stack arg.
SSA-based signature recovery drops ABI-relevant unused params. **Mitigation:** for a vtable slot's
canonical signature, check the RET-immediate across multiple overrides, not one override's param list.

---

## Failure Class C — Verified fine (do not "fix")

- **Setup is clean:** image base 0x400000, standard `.text`/`.rdata`/`.data`/`.rsrc` layout, no
  overlaps, analysis settled (`get_metadata`, `list_segments`, `analysis_status`).
- **RTTI/COL→TypeDescriptor→mangled-name substrate is 100% intact** for all 5 core classes
  (UnitClass `.?AVUnitClass@@`, AircraftClass, BuildingClass, InfantryClass, TechnoClass) — verified by
  walking `vtable-4 → COL → TypeDescriptor+8` for each. **This is the load-bearing good news: the
  verification method CLAUDE.md mandates actually works** — the authority you fall back to when a label
  is suspect is reliable.
- **thiscall recovery correct in the majority sampled** (only the one A4 mislabel found).
- **Template/inline-ctor duplicate names are legitimate**, not drift (MSVC6 per-TU duplication).
- **Tooling note (MCP, not project state):** `search_functions_enhanced`'s `calling_convention` filter
  is **inert** — it returns the unfiltered set regardless of value (verified with a real and a bogus
  convention name, both returning the baseline count). Don't rely on it to enumerate thiscall methods.

---

## What this means for the workflow

1. **The methodology is right; this audit shows *why* each CLAUDE.md rule exists.** "Verify from the
   binary, cite `get_assembly_context` for offsets, verify vtable owner via the mangled name, treat
   labels as hints" — every one of those maps to a Class-A/B hazard confirmed here. Keep doing it.
2. **`get_struct_layout` field names need the same skepticism as function labels** — this is the gap
   in current guidance. Offsets provisional-trust, names verify-before-use, whole DTM treated as
   YRpp-imported.
3. **Some wave-1 "RTTI_LABEL_DRIFT" fixes were doc-vs-current-Ghidra, not Ghidra-is-wrong.** Lane 2
   found all four wave-1 anchors currently correct in Ghidra (`CellClass__FindFirstUnit`,
   `SlaveManagerClass__AI_Update`, the `ILocomotion` thunk; no `TacticalClass__DrawObjects` at that
   address — the real one is `TacticalClass_Draw`@0x006d3d10). Interpretation: the docs carried stale
   names; current Ghidra is more correct for those specific functions. Worth confirming per-case.

## Remediation options (NOT executed — read-only audit; require user approval as they mutate Ghidra or docs)

- **Do not** bulk-fix the 99 `CDFileClass__Constructor` labels blindly — most need per-address
  decompilation to name correctly; low value vs. risk. Better: add a "duplicate-name ≥5 ⇒ unverified"
  note to the RE guidance.
- Optionally correct the DTM `CellClass.Height`→ rename/annotate to `IsoSubTileIndex` (mutating; single
  field; high leverage since it's queried often).
- Optionally add a short "get_struct_layout names are YRpp-imported hints" clause to CLAUDE.md /
  re-decoder-ring skill.

## Sources (all read-only)
Parent probes: `get_struct_layout CellClass/TechnoClass`, `analysis_status`,
`search_functions_enhanced`, `decompile_function 0x0057B440`, `read_memory 0x007f4de0/0x007f60f0`,
`get_function_by_address 0x006951f0`. Lane detail files (scratchpad): `slot-1-structs.md`,
`slot-2-labels.md`, `slot-3-decoder.md`, `slot-4-rtti.md`, `slot-5-dupes.md`.
