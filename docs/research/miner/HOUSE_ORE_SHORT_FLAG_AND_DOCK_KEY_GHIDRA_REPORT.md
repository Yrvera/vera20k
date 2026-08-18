# House "no-ore" flag (house+0x242) and TechnoTypeClass Dock= key read site

Swarm 2026-07-28T12:25, slot-5. Read-only Ghidra investigation, gamemd.exe @ image base 0x400000, project testProsjekt.

## Target question

A) Who reads `HouseClass + 0x242` (the byte set to 1 at 0x0073e911 when `UnitClass::Mission_Harvest` finds no ore), what does each reader do with it, is it ever cleared, and what is the Active-in-YR verdict per reader.

B) Where is the literal `"Dock"` INI key string read in `TechnoTypeClass::ReadINI`, which call reads it, and what is the confirmed data/count layout of the destination vector at `type+0x3E8`.

## Non-goals

Full decode of `HouseClass::AI_Choose_Unit` or `TechnoTypeClass::ReadINI`; other house flags; Dock= consumers beyond the read site; mission-harvest cadence (already covered by `mission-harvest-cadence.md`); Rust implementation changes.

## Evidence needed to mark COMPLETE

(A) Exhaustive `search_instructions` on the 0x242 displacement, decompile of every hit, receiver-type confirmation, writer/clear-site enumeration, reachability evidence (caller or table xref) per reader.
(B) `search_strings`/memory inspection to pin the "Dock" literal, decompile/assembly proof of the call that reads it, and cross-verification of the type+0x3E8 vector's data/count offsets from an independent consumer.

## Stop conditions

Ghidra unreachable → FAILED. Program-wide instruction scan exhaustive (all mnemonics, one displacement, whole program) with every hit resolved → COMPLETE for (A). Key string pinned + read call proven + vector layout cross-verified by an independent site → COMPLETE for (B).

---

## Part A — HouseClass+0x242 ("no ore found this pass") readers

`search_instructions` for operand substring `0x242]` across mnemonic="" (all mnemonics), whole program, returned **exactly 4 hits** out of 1,152,096 instructions scanned — verified via `search_instructions(operand_pattern="0x242]")`. This is exhaustive; no other addressing form (register, scale, or mnemonic) touches this displacement anywhere in the binary.

| Address | Function | Op | Role |
|---|---|---|---|
| 0x004f5771 | `HouseClass::Constructor` | `MOV byte ptr [EBP+0x242], BL` (BL=0 in this init block) | init/clear |
| 0x004feb7b | `HouseClass::AI_Choose_Unit` | `MOV AL, byte ptr [EBP+0x242]` | reader |
| 0x0073e911 | `UnitClass::Mission_Harvest` | `MOV byte ptr [ECX+0x242], 1` | writer (already established) |
| 0x00740922 | `UnitClass::Mission_Guard_Harvester` | `MOV AL, byte ptr [EDX+0x242]` | reader |

**Receiver identity.** `HouseClass::Constructor`'s decompile shows the same instruction inline with dozens of other `*(undefined1*)((int)param_1+0xNNN)=0` initializations of clearly House-owned byte fields (e.g. adjoining bytes 0x241/0x243 zeroed in the same block) — verified via `decompile_function 0x004f5771`. In `AI_Choose_Unit`, the field is accessed as `param_1->field_0x242` where `param_1` is declared `HouseClass*` in the `__fastcall` signature — verified via `decompile_function 0x004feb7b`. In `Mission_Guard_Harvester`, the byte is read at `param_1[0x87] + 0x242`, where `param_1[0x87]` (= unit+0x21c) is the unit's Owner-House pointer — the same field slot the known writer (`Mission_Harvest`) writes through (unit+0x21c → House) — verified via `decompile_function 0x00740922`.

### Reader 1 — `HouseClass::Constructor` (0x004f5771) — init, not a "clear"

Sets the byte to 0 once, at object construction, alongside a run of other per-house byte flags (0x241, 0x242, 0x243, 0x245...) all zeroed together. Verified via `decompile_function 0x004f5771`.

**No other write to this byte exists anywhere in the binary except the constructor's zero-init and the harvester's set-to-1 at 0x0073e911.** The exhaustive 4-hit scan above proves this: the flag, once set to 1 by a harvester finding no ore, is **never cleared during gameplay** — it only resets to 0 if the House object itself is destructed/reconstructed (e.g., new game/scenario load), not per-frame or per-mission-cycle.
**Active in YR: Yes** (constructor runs for every House object, including AI and human houses).

### Reader 2 — `HouseClass::AI_Choose_Unit` (0x004feb7b)

```
if ((RulesClass+0x1458 <= house->field_0x24c) && (house->field_0x242 == 0)) {
    ... (side/player-control checks) ...
    if (cVar2==0 && iVar6<iVar3 && candidate_type[+0x634] <= house->field_0x1d4) {
        house->field_0x5650 = candidate_type[+0xdf8];   // record a "build this type" selection
        return 0xf;                                      // early production-decision return
    }
}
```
Verified via `decompile_function 0x004feb7b`. The flag gates entry to a fast-path production decision: only when `house+0x242 == 0` ("no shortage observed yet") does the function consider assigning `house->field_0x5650` (a build-target/queue field) from a candidate type drawn from a RulesClass-provided array (RulesClass+0xb40/+0xb4c) filtered by an owner-side bitmask. **Proven fact:** the branch that can select a new unit-to-build is skipped whenever the ore-short flag is set. **Inferred, not proven:** that the candidate array specifically enumerates the harvester type (it is plausible given the flag's origin, and matches the TS-parallel "ore short → stop over-producing harvesters" AI advisory the parent notes describe, but the array's exact contents were not independently confirmed in this pass) — placed in Unverified below.

**Reachability:** `get_function_callers(0x004feb7b)` → `HouseClass::Update @ 0x004f8440` (direct CALL). `HouseClass::Update` itself shows no direct-CALL callers (`get_function_callers` empty), but `get_xrefs_to(0x004f8440)` finds a DATA reference at `0x007ea8fc` — consistent with a vtable slot (HouseClass's constructor installs `vtable__HouseClass` and several secondary vtables at param_1[0..3],[9..0xb], matching the surrounding constructor code) invoked via the engine's per-object virtual AI/Update dispatch, not a static call.
**Active in YR: Yes** — direct-call chain into `HouseClass::AI_Choose_Unit` is proven; `HouseClass::Update`'s own reachability is via vtable dispatch (data xref evidence, not a direct-call xref) — verified via `get_function_callers 0x004feb7b`, `get_xrefs_to 0x004f8440`.

### Reader 3 — `UnitClass::Mission_Guard_Harvester` (entry 0x00740810; the 0x242 read is at 0x00740922 inside it)

```
if (house_field[+0xe0f] != 0) → HouseClass::IsPlayerControl() == 0 (AI house):
    iterate this_type->Dock vector (type+0x3E8, see Part B) looking for an owned dock building
      with instances > 0; if found and
        (type[+0xe0e]==0 OR ownerHouse->field_0x242==0)
      → force Mission = 10 (return to active harvest/dock mission), else break (stay Guard).
```
Verified via `decompile_function 0x00740922` (function entry resolved via `get_function_by_address` → true entry 0x00740810). This ties the two hunts together in one function: a harvester sitting in the Guard mission will **not** be force-switched back into active harvesting if its type requires the check (`type+0xe0e` set) **and** its owner house has the no-ore flag set — i.e., the flag suppresses a redundant "try harvesting again" dispatch once the house already knows the local ore situation is exhausted.

**Reachability:** neither `get_xrefs_to` nor `get_function_callers` on the *correct* function entry (0x00740810) found a direct-call site at first; `get_function_callers` returned none. However `get_xrefs_to(0x00740810)` found a **DATA** reference at `0x007f5e8c`. `inspect_memory_content(0x007f5e70, 64)` around that address shows a contiguous array of valid code-segment pointers (`0x00744270, 0x005b2e10, 0x005b2e20, 0x005b2e30, 0x007447a0, 0x004d4b20, 0x004d4cb0, 0x00740810 [[at +0x1c, matches 0x007f5e8c]], 0x00744100, 0x0073e5e0, 0x0073efc0, 0x00740a90, ...`) — i.e. a mission-handler function-pointer table (classic per-class Mission dispatch array; several neighboring entries sit in the same 0x73xxxx–0x74xxxx harvester/mission code range as the known `Mission_Harvest` write site 0x0073e911). A `search_byte_patterns` for the literal little-endian bytes of the function's entry address (`10087400`) confirms exactly one occurrence in the whole binary — the table slot at 0x007f5e8c. Verified via `get_xrefs_to 0x00740810`, `search_byte_patterns 10087400`, `inspect_memory_content 0x007f5e70`.
**Active in YR: Conditional/Yes-by-table-membership** — the function is provably embedded as a live entry in a mission-dispatch function-pointer table (not dead/orphaned code, not merely a leftover unreferenced compilation unit); the indirect call site that indexes into this table (presumably `handler[missionID]()`) was not resolved by Ghidra's static analysis, so a direct code-to-code call edge is not shown. This is materially different from "no evidence of any reference" — the table-slot xref is the applicable evidence for a computed-jump dispatch idiom.

---

## Part B — the "Dock" key string in `TechnoTypeClass::ReadINI`

`search_strings` for regex `Dock` (defined-string search) returned only `DockingOffset%d`, `NumberOfDocks`, `DockUnload` — no bare `"Dock"` — confirming the parent's prior-pass finding that no standalone indexed string exists.

**Pinned key string:** `inspect_memory_content(0x0084418c, 16)` → hex `44 6F 63 6B 00 00 00 00 43 61 74 65 67 6F 72 79` = `"Dock\0"` immediately followed (no gap) by `"Category"` — i.e. an **inline literal-pool string never classified as a standalone String data item by Ghidra**, which is exactly why it was invisible to `search_strings`. `get_xrefs_to(0x0084418c)` returns exactly **one** xref: `From 0x00713180 in TechnoTypeClass__ReadINI [DATA]` — a `PUSH 0x84418c` instruction.

**The read call:** disassembly of 0x00713171–0x0071318e (`disassemble_bytes`) shows:
```
LEA EDX,[ESP+0x5c]          ; output buffer
PUSH 0x80                   ; buflen = 128
PUSH EDX                    ; buffer
PUSH 0x889f64                ; default value (verified: 8 zero bytes = "")
PUSH 0x84418c                 ; key = "Dock"
PUSH EBX                    ; section handle
MOV  ECX, ESI               ; this = ini reader object
CALL 0x00528a10              ; CCINIClass::ReadString
```
`decompile_function 0x00528a10` confirms the signature `CCINIClass__ReadString(this, section, key, default, buffer, buflen)` — a canonical Get_String(section, entry, default, buf, buflen) shape, argument order matching the push order above exactly. This is the read site for `Dock=`.
**Verified via:** `disassemble_bytes 0x00713120-0x007131a5`, `inspect_memory_content 0x0084418c` and `0x889f64`, `get_xrefs_to 0x0084418c`, `decompile_function 0x00528a10`.

**Tokenizer mechanism** (already partly known from the prior pass, re-confirmed here): the raw string is split with the CRT `strtok(buffer, ",")` — verified `decompile_function 0x007c9cc2` identifies it as `CRT__strtok`; the delimiter constant at `0x00817f70` is `",\0"` (verified via `inspect_memory_content 0x817f70`, **not** a second key string as its address proximity to the tokenizer loop might suggest). Each token is resolved via `BuildingTypeClass::FindOrAllocate` (0x004653c0 — already plate-commented in Ghidra as "canonical FindOrAllocate pattern... searches g_BuildingTypeClass_Array by name@+0x24... on miss: operator_new → BuildingTypeClass constructor") and appended to a local accumulator vector, which is then assigned into `this->Dock` (type+0x3E8) via a `DynamicVectorClass`-style copy/assign helper (0x0067b180 / 0x005ad660). If `ReadString` returns 0 (key absent or empty), the code takes the `JZ 0x0071321c` branch, which copy-constructs the accumulator from the **existing** `type+0x3E8` vector and reassigns it — a functional no-op, i.e. **a missing `Dock=` line leaves the vector unchanged** (empty, for a freshly-constructed type). Verified via `decompile_function 0x0067b180`, `decompile_function 0x004653c0`, `decompile_function 0x005ad660`, and the surrounding `disassemble_bytes` context.

### Vector layout confirmation (type+0x3E8)

Two independent sites agree on the layout:
1. **Construction-side** (`decompile_function 0x005ad660`, the copy-ctor helper used at the ReadINI site): data pointer at struct-offset +0x4, capacity/count-ish field at +0x8, flag bytes at +0xC/+0xD, and **two additional fields at +0x10 and +0x14 copied unconditionally** (`param_1[4]=param_2[4]; param_1[5]=param_2[5]`).
2. **Independent consumer** — `UnitClass::Mission_Guard_Harvester` (0x00740922 region) reads the *same* member directly: `if (0 < *(int*)(type+0x3f8)) { ... *(int*)(*(int*)(type+0x3ec) + i*4) ... }` — i.e. it uses **type+0x3EC as the data pointer** and **type+0x3F8 as the iteration count**, verified via `decompile_function 0x00740922`.

`0x3EC - 0x3E8 = 0x4` matches the copy-helper's data-pointer offset exactly. `0x3F8 - 0x3E8 = 0x10` matches the copy-helper's "extra field copied unconditionally" (param_1[4]) rather than the inner param_1[2] (`+0x8`) capacity slot — meaning the **logical/active count used by real consumers lives at the outer +0x10 field, not the inner allocation-capacity field at +0x8**. This **confirms the prior doc's line "data +0x3EC, count +0x3F8" is correct** (the instruction to re-verify rather than trust it is satisfied: it checks out against an independent, unrelated consumer site).
**Verified via:** `decompile_function 0x00740922` (consumer), `decompile_function 0x005ad660` (constructor-side field shape), cross-checked arithmetically.

---

## Implementation Handoff

- **Verified behavior:** `HouseClass+0x242` is a sticky (never-cleared-in-play) per-house "ore recently found absent" flag, set once by a harvester's no-ore idle transition, gating (a) the AI's fast-path harvester/candidate-type build selection in `AI_Choose_Unit`, and (b) whether a Guard-mission harvester force-resumes active harvesting in `Mission_Guard_Harvester`. **Rust delta:** current Rust rules/sim has no equivalent per-house "ore short" flag or consumer (per parent's stated context). **Affected surface:** AI harvester production throttling and harvester Guard-mission re-dispatch logic (both currently absent in Rust, per parent notes — no AI opponent yet per project memory, so (a) is not yet relevant; (b) affects human-controlled-harvester Guard behavior too since `IsPlayerControl()==0` gates the AI-specific branch but the general Dock-vector scan happens regardless of control). **Acceptance scenario:** a harvester with no reachable ore sits in Guard state near a dock it owns; it should not spuriously flip back into an active harvest attempt every tick once the house has already registered "no ore." **Proposed test name:** `test_harvester_guard_no_reforce_after_ore_short_flag`. **Risk:** low-medium — behavior is currently invisible in Rust (no flag exists), so this is an additive-parity gap, not a regression risk.
- **Verified behavior:** `Dock=` is read via `CCINIClass::ReadString(section, "Dock", "", buffer, 128)` then comma-tokenized with each token resolved through `BuildingTypeClass::FindOrAllocate` (which **allocates a new BuildingTypeClass if the name isn't found yet** — order-of-parsing matters) into `TechnoTypeClass+0x3E8`, data at +0x3EC, count at +0x3F8. **Rust delta:** Rust's `harvester_can_dock_at` already reads `Dock=` into `ObjectType` per parent notes; this confirms the binary's key/parse mechanism matches a simple comma-list-of-building-names model (no surprising extension format). **Affected surface:** `src/rules` Dock parsing. **Acceptance scenario:** `Dock=NAREFN,GAREFN` parses to the two type names in list order, matching stock INI. **Proposed test name:** `test_dock_key_parses_comma_list_in_order`. **Risk:** low — matches existing Rust behavior; no change indicated.
- **Verified behavior:** a missing `Dock=` key is a functional no-op (vector stays at its prior/default value, i.e., empty for a fresh type) rather than an error or a "default dock list." **Rust delta:** confirm Rust's default is an empty dock list when `Dock=` is absent (not inherited from some other default). **Acceptance scenario:** a `[SomeUnit]` with no `Dock=` line has an empty dock-type list. **Proposed test name:** `test_missing_dock_key_yields_empty_list`. **Risk:** low.

## Negative Facts / Do Not Do

- Do **not** treat `0x00817f70` as the "Dock" key string — it is the strtok delimiter constant (`",\0"`), not a key name, despite being pushed adjacent to the tokenizer loop. Verified via `inspect_memory_content 0x817f70`.
- Do **not** assume `HouseClass+0x242` is cleared/reset per-mission-cycle or per-frame — the exhaustive whole-program instruction scan (4 hits total) proves the only writes are the constructor's zero-init and the single `Mission_Harvest` set-to-1; there is no periodic clear. Verified via `search_instructions operand_pattern="0x242]"` (1,152,096 instructions scanned, 4 matches).
- Do **not** rely on `get_function_callers`/`get_xrefs_to` alone to declare a function "dead" when it returns empty — both `HouseClass::Update` (0x004f8440) and `UnitClass::Mission_Guard_Harvester` (0x00740810) show zero direct-call callers yet are proven live via DATA/vtable/jump-table xrefs (0x007ea8fc and 0x007f5e8c respectively). Verified via `get_xrefs_to` on both addresses plus `search_byte_patterns` confirming the exact function-entry bytes appear in a function-pointer table.
- Do **not** reuse the stale count-offset guess of "+0x3F0" that a naive reading of the generic `DynamicVectorClass::operator=` helper (0x0067b180, using its inner `param_1[2]` capacity slot) would suggest — the real consumer (`Mission_Guard_Harvester`) proves the logical count sits at the outer `+0x10` field (type+0x3F8), not the inner capacity field. Verified via `decompile_function 0x00740922` cross-checked against `decompile_function 0x005ad660`.
- Do not confuse `HouseClass::AI_Choose_Unit`'s candidate-type array (RulesClass+0xb40/+0xb4c) with the Dock vector — they are unrelated structures gated by the same house byte but serving different purposes (production candidate list vs. dock-building list).

## Remaining Uncertainty

- Whether the candidate-type array iterated in `HouseClass::AI_Choose_Unit` (RulesClass+0xb40, filtered by an owner-side bitmask at candidate+0x6cc) is specifically/exclusively the harvester type, or a broader "AI must-maintain" unit list that happens to include harvesters — not independently confirmed in this pass (Unverified/Yellow).
- The exact indirect-call site that indexes into the mission-dispatch table containing `Mission_Guard_Harvester` (0x007f5e8c) was not located; Ghidra's static analysis did not resolve the computed jump, so no assembly-level "this is the switch statement" proof exists, only the table-membership proof.
- Semantic meaning of `HouseClass+0x24c`, `+0x1d4`, `+0x5650`, and the RulesClass+0x1458/0xb40 fields used alongside the 0x242 check in `AI_Choose_Unit` were read (decompile-level) but not independently named/verified against other docs — treat any inferred label for them as navigation-only.

## Unverified (Yellow)

- That the `AI_Choose_Unit` gate specifically throttles harvester (re)production rather than some other unit-type category (see Remaining Uncertainty above) — plausible given the flag's harvester-only origin and the TS-parallel framing, but not proven by an independently-verified array-contents read in this pass.

## Stale-doc replacement candidates

None found that require correction — the parent's flagged-as-uncertain line ("type+0x3E8 vector, data +0x3EC, count +0x3F8") in `docs/scans/trace-swarm-20260728/mission-harvest-cadence.md` was **cross-verified as correct** by this pass (see Part B vector-layout section), so no replacement wording is proposed; if anything, that line's confidence should be upgraded from "re-verify" to "confirmed via independent consumer (`UnitClass::Mission_Guard_Harvester` @ 0x00740922)."

## Status: COMPLETE
