# RulesClass SplashList ReadINI Defaults Bounds - Ghidra Research Report

**Address(es):** `RulesClass::Constructor @ 0x00665650`, `RulesClass__ReadCombatDamage @ 0x0066BBB0`, `SplashList` read site `0x0066C18A..0x0066C287`, `AnimClass::AI` consumers `0x00423D29..0x00423D35` and `0x00423DD2..0x00423DD8`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active YR `[CombatDamage] SplashList=` reader/default behavior, dynamic-vector storage offsets, item lookup behavior, missing/blank key behavior, and the exact bounds assumptions made by the already-verified `AnimClass` bouncer water consumer.
**Non-Scope:** full bouncer impact behavior, full `RulesClass` loader/source-layer merge, final drawing of water splash anim rows, `VoxelAnimClass` water behavior, and unrelated `[CombatDamage]` keys.
**Confidence:** High for reader/vector/consumer mechanics; Medium for global source-layer priority because this report does not re-audit the full INI loader.
**Active in YR:** Yes. The reader is `RulesClass__ReadCombatDamage`; stock YR `ini/rulesmd.ini` and base `ini/rules.ini` both define `[CombatDamage] SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`, and `AnimClass::AI` actively consumes the resulting vector for bouncer/meteor water impacts.

Working notes:
- `Target question`: Verify active YR reader/default behavior for `[CombatDamage] SplashList=` into `RulesClass` vector fields used by `AnimClass` bouncer water branch.
- `Non-goals`: Do not re-investigate bouncer water predicate/row order beyond connecting reader fields to consumer; do not implement Rust; do not inspect unrelated `CombatDamage` keys except source priority context.
- `Evidence needed to mark COMPLETE`: INI source/default, binary reader address/storage offsets, dynamic-vector count/buffer layout, consumer read proof at `AnimClass::AI`, empty/missing/bounds behavior or explicit unresolved binary edge, Rust parser handoff.
- `Stop conditions`: Stop after parser and consumer evidence agree or after any parser edge that cannot be resolved read-only is recorded as Remaining Uncertainty with exact follow-up.

## 1. Overview

`SplashList=` is a live `[CombatDamage]` rules key parsed into a `DynamicVectorClass<AnimTypeClass*>` at `RulesClass+0xBC0`. `RulesClass__ReadCombatDamage` reads a comma-tokenized string, resolves each non-sentinel token through `AnimTypeClass::FindOrAllocate`, appends successful pointers, and copies the temporary vector into the rules object.

There is no hardcoded `H2O_EXP3,H2O_EXP2,H2O_EXP1` fallback string in `gamemd.exe`. The stock default comes from INI text; both base RA2 and YR rules files in this repo carry the same list.

## 2. Class Layout / Key Offsets

| Field | Offset | Type / layout | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `Rules.SplashList` | `+0xBC0` | `DynamicVectorClass<AnimTypeClass*>` | owning vector object | constructor decompile `0x00665650`; reader `0x0066C22F..0x0066C269` | Yes |
| `Rules.SplashList.buffer` | `+0xBC4` | `AnimTypeClass**` | backing pointer read by consumers | `DynamicVectorClass__CopyFrom @ 0x00525060`; consumer reads `0x00423D2F`, `0x00423DD2` | Yes |
| `Rules.SplashList.capacity` | `+0xBC8` | `int` | allocated capacity copied by vector helper | `DynamicVectorClass__Constructor @ 0x00525250`; copy helper `0x00525060` | Yes |
| `Rules.SplashList.is_allocated` | `+0xBCD` | `byte` | whether backing storage is owned/freeable | `VectorClass__Clear @ 0x005251C0`; `DynamicVectorClass__Constructor @ 0x00525250` | Yes |
| `Rules.SplashList.count` | `+0xBD0` | `int` | active element count; meteor indexes `count - 1` | add helper `0x005253B0`; consumer read `0x00423D29` | Yes |
| `Rules.SplashList.grow_amount` | `+0xBD4` | `int` | default growth step, constructor sets `10` | `DynamicVectorClass__Constructor @ 0x00525250`; constructor decompile `0x00665650` | Yes |

Important offset correction: prior docs that name only `+0xBC4/+0xBD0` are correct for consumer reads, but the vector object starts at `+0xBC0`. `+0xBC4` is not the list itself; it is the buffer pointer inside the list object.

## 3. Core Logic

### 3.1 Constructor default state

Active in YR: Yes.

`RulesClass::Constructor @ 0x00665650` initializes the `+0xBC0` vector as empty. The decompile shows the `RulesClass` vector member at `param_1[0x2F0]` (`0x2F0 * 4 = 0xBC0`) being constructed with a dynamic-vector vtable, `param_1[0x2F4] = 0` (`+0xBD0` count), and `param_1[0x2F5] = 10` (`+0xBD4` grow amount). This means no compiled-in splash anim list exists before INI read.

Material finding: if no layer ever supplies `SplashList=`, the binary-side vector can remain empty. Active in YR: Conditional for modified data, because stock YR supplies the key.

### 3.2 ReadString call and missing/blank behavior

Active in YR: Yes.

At `0x0066C18A..0x0066C1A6`, `RulesClass__ReadCombatDamage` calls `CCINIClass__ReadString` with:

- section pointer `CombatDamage` (`PTR_s_CombatDamage_007F0C84`);
- key string `SplashList` (`0x0083B1FC`);
- default string pointer `0x00889F64`;
- destination stack buffer `ESP+0x48`;
- max length `0x80`.

Assembly evidence:

- `0x0066C18A` loads destination buffer;
- `0x0066C18E` pushes `0x80`;
- `0x0066C194` pushes default string `0x00889F64`;
- `0x0066C199` pushes `0x0083B1FC` (`SplashList`);
- `0x0066C19F` passes the active INI object in `ECX`;
- `0x0066C1A1` calls `CCINIClass__ReadString`;
- `0x0066C1A6..0x0066C1A8` tests return length and branches to preserve-existing path when zero.

`CCINIClass__ReadString @ 0x00528A10` returns the trimmed string length, not a pure found/not-found boolean. It copies the found value or default string into the destination, trims it, then returns `strlen(trimmed_buffer)`. Therefore:

- missing key with empty default string returns `0`;
- explicit `SplashList=` with blank/whitespace value also returns `0` after trim;
- both cases take the `iVar2 == 0` branch at `0x0066C1A6..0x0066C1A8`.

The zero-length branch copies the existing `RulesClass+0xBC0` vector into the temp vector (`0x0066C22F..0x0066C23A`) and then copies it back to `RulesClass+0xBC0` (`0x0066C23F..0x0066C269`). So missing or blank `SplashList=` preserves the existing vector; it does not clear it and does not synthesize a default list.

### 3.3 Present non-empty behavior and item lookup

Active in YR: Yes.

If `ReadString` returns nonzero, the parser constructs a temporary dynamic vector with capacity `0` and grow amount `10`, tokenizes the stack buffer, and resolves each non-empty token:

- `0x0066C1AE..0x0066C1B6`: construct temp vector;
- `0x0066C1BB..0x0066C1D7`: first `strtok` call and null check;
- `0x0066C1D9..0x0066C1DC`: skip loop if token starts with NUL;
- `0x0066C1DE..0x0066C1E5`: pass token to `AnimTypeClass__FindOrAllocate @ 0x00428B80`;
- `0x0066C1E7..0x0066C1F6`: append only if returned pointer is non-null;
- `0x0066C1FB..0x0066C20C`: subsequent `strtok(NULL, delimiter)` calls;
- `0x0066C20E..0x0066C228`: copy temp vector and clear its backing storage;
- `0x0066C23F..0x0066C269`: copy temp into `RulesClass+0xBC0`, then copy count/grow/extra fields.

`AnimTypeClass__FindOrAllocate @ 0x00428B80` is not a pure lookup. It rejects only the sentinel strings `<none>` (`0x00817474`) and `<noname>` (`0x00817694`), searches `g_AnimTypes_Array`, and allocates a new `AnimTypeClass` on miss. Therefore a mod typo in `SplashList=` can create an anim type stub instead of being silently dropped, unless allocation fails or the token is one of those sentinels.

`DynamicVectorClass__Add @ 0x005253B0` appends at `data[count]`, increments count, and grows capacity by `grow_amount` when needed. The default grow amount is `10`; the first append from capacity `0` allocates capacity `10`.

### 3.4 Copy semantics and source priority within the reader

Active in YR: Yes.

`RulesClass__ReadCombatDamage` does not know about `rules.ini` vs `rulesmd.ini` itself. It reads from the already-active `CCINIClass` object. Its per-key priority behavior is:

- non-empty present value replaces the existing vector with the parsed token list;
- missing key preserves the existing vector;
- blank/trim-empty value also preserves the existing vector because `ReadString` returns length `0`.

Source-layer implication: the full game loader decides how base, YR patch, and later scenario/map layers are merged into the active `CCINIClass`. This report did not re-audit that global loader. For the stock data checked here, base `ini/rules.ini:722` and YR `ini/rulesmd.ini:902` both define the same `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1`, so the active stock default is unambiguous.

### 3.5 Consumer bounds assumptions

Active in YR: Conditional on bouncer/meteor water impact reaching the already-verified water branch.

`AnimClass::AI` does no null/count guard before reading `SplashList`.

Meteor consumer:

- `0x00423D29`: reads count from `RulesClass+0xBD0`;
- `0x00423D2F`: reads buffer from `RulesClass+0xBC4`;
- `0x00423D35`: reads `buffer[count * 4 - 4]`;
- this assumes `count > 0` and `buffer != null`.

Non-meteor consumer:

- `0x00423DD2`: reads buffer from `RulesClass+0xBC4`;
- `0x00423DD8`: reads `buffer[0]`;
- this assumes at least one valid element.

There is no branch that substitutes `Wake`, `H2O_EXP3`, or any other fallback when the vector is empty. A future Rust implementation must either preserve the same precondition for parity tests or deliberately model invalid-data crash/undefined behavior separately.

## 4. INI Keys

| Key | Section | Stock base value | Stock YR value | Binary reader/use | Active in YR |
|---|---|---|---|---|---|
| `SplashList` | `[CombatDamage]` | `H2O_EXP3,H2O_EXP2,H2O_EXP1` at `ini/rules.ini:722` | `H2O_EXP3,H2O_EXP2,H2O_EXP1` at `ini/rulesmd.ini:902` | read at `0x0066C18A..0x0066C287`; consumed at `0x00423D29..0x00423D35`, `0x00423DD2..0x00423DD8` | Yes |
| `Wake` | `[General]` | `WAKE1` at `ini/rules.ini:519` | `WAKE1` at `ini/rulesmd.ini:525` | not part of this parser; connected consumer read at `RulesClass+0x94` in bouncer water branch | Yes, connected only |

## 5. Integration Points

- `RulesClass__ReadCombatDamage @ 0x0066BBB0` is the only `SplashList` string xref found by `batch_string_anchor_report`.
- `RulesClass::Constructor @ 0x00665650` creates the initial empty vector before INI parsing.
- `AnimClass::AI @ 0x00423AC0` consumes `RulesClass+0xBC4/+0xBD0` in the bouncer/meteor water branch.
- `VoxelAnimClass` also has SplashList consumers, but this report only uses it as a stale-doc warning surface and does not claim voxel water row parity.

## 6. Current Rust Implementation Status

- `src/rules/ruleset.rs` parses `[General] Wake=` into `GeneralRules::wake` (`rg` hit around lines `245`, `996`, `1194`).
- `src/rules/ruleset.rs` and `src/rules/combat_damage.rs` parse several `[CombatDamage]` surfaces, but `rg "SplashList|splash_list" src` found no SplashList parse/use.
- No current Rust field was found for `RulesClass.SplashList` as `Vec<AnimRef>` or equivalent.
- The future generic `AnimClass` bouncer runtime will need this data before it can implement the already-verified water splash branch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `SplashList` string xref inventory | verified | `batch_string_anchor_report`: sole documented function `RulesClass__ReadCombatDamage` | none |
| `RulesClass` initial vector state | verified | decompile `RulesClass::Constructor @ 0x00665650` | exact constructor assembly range for this member not isolated; decompile is clear |
| `CCINIClass__ReadString` return semantics | verified | decompile `0x00528A10`; call branch `0x0066C1A6..0x0066C1A8` | none |
| Missing/blank key behavior | verified | reader zero-length branch `0x0066C1A6..0x0066C23F` plus `ReadString` trim/strlen return | none |
| Present non-empty parse loop | verified | reader `0x0066C1AE..0x0066C228` | delimiter memory byte not separately dumped; INI format and `strtok` call chain support comma-token behavior |
| Item lookup behavior | verified | `AnimTypeClass__FindOrAllocate @ 0x00428B80` | none for sentinels/find-or-allocate |
| Vector layout/count/buffer offsets | verified | helpers `0x00525250`, `0x005253B0`, `0x00525060`; consumer reads | none |
| Consumer bounds checks | verified | `0x00423D29..0x00423D35`; `0x00423DD2..0x00423DD8` | invalid-data runtime outcome not executed under debugger |
| Full INI source-layer merge order | touched-not-exhausted | stock INI files checked; reader consumes active `CCINIClass` | full loader/layer audit out of scope |
| Rust parser status | verified by source scan | `rg "SplashList|splash_list" src`; `src/rules/ruleset.rs`; `src/rules/combat_damage.rs` | none for absence finding |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `SplashList` a live YR rules key? -> Yes, sole string xref is `RulesClass__ReadCombatDamage`, and stock YR data defines the key.` (evidence: `0x0083B1FC`; `0x0066C18A..0x0066C287`; `ini/rulesmd.ini:902`)
- `[RESOLVED] OQ-02 - What field receives the parsed list? -> `RulesClass+0xBC0` dynamic vector; buffer at `+0xBC4`, count at `+0xBD0`.` (evidence: `0x0066C23F..0x0066C269`; helpers `0x00525060`, `0x005253B0`; consumers `0x00423D29`, `0x00423DD2`)
- `[RESOLVED] OQ-03 - Is there a hardcoded retail fallback list? -> No evidence of one; constructor starts empty and reader uses INI/default-string/preserve-existing semantics.` (evidence: constructor `0x00665650`; reader `0x0066C18A..0x0066C287`; string search found no `H2O_EXP*` binary strings)
- `[RESOLVED] OQ-04 - What is the stock list? -> Both base and YR files define `H2O_EXP3,H2O_EXP2,H2O_EXP1`.` (evidence: `ini/rules.ini:722`; `ini/rulesmd.ini:902`)
- `[RESOLVED] OQ-05 - What happens if the key is missing? -> Existing vector is preserved; if this is the initial constructor state, that means empty.` (evidence: `ReadString @ 0x00528A10`; zero branch `0x0066C1A6..0x0066C23F`)
- `[RESOLVED] OQ-06 - What happens if the key is present but blank/whitespace? -> It also preserves the existing vector because `ReadString` returns trimmed length `0`.` (evidence: `0x00528BB6..0x00528BE2`; `0x0066C1A6..0x0066C1A8`)
- `[RESOLVED] OQ-07 - How are tokens resolved? -> Each non-empty token is passed to `AnimTypeClass__FindOrAllocate`; returned non-null pointers are appended.` (evidence: `0x0066C1D9..0x0066C1F6`; `0x00428B80`)
- `[RESOLVED] OQ-08 - Are unknown token names ignored? -> No, not normally; `FindOrAllocate` allocates a new type on miss unless the token is `<none>`/`<noname>` or allocation fails.` (evidence: `0x00428B80`)
- `[RESOLVED] OQ-09 - Does consumer guard empty vector? -> No, meteor uses `buffer[count-1]` and non-meteor uses `buffer[0]` without checks.` (evidence: `0x00423D29..0x00423D35`; `0x00423DD2..0x00423DD8`)
- `[RESOLVED] OQ-10 - Which entry does stock non-meteor use? -> first entry, `H2O_EXP3`.` (evidence: `0x00423DD2..0x00423DD8`; `ini/rulesmd.ini:902`)
- `[RESOLVED] OQ-11 - Which entry does stock meteor use? -> last entry, `H2O_EXP1`.` (evidence: `0x00423D29..0x00423D35`; `ini/rulesmd.ini:902`)
- `[RESOLVED] OQ-12 - Is the key under `[General]`? -> No for this reader; `SplashList` is read from `[CombatDamage]`.` (evidence: `0x0066C193..0x0066C1A1`; `ini/rulesmd.ini:902`)
- `[RESOLVED] OQ-13 - Does Rust parse it? -> No current `SplashList`/`splash_list` source hit exists.` (evidence: `rg "SplashList|splash_list" src`)
- `[DEFERRED] OQ-14 - Full base/YR/map INI layer merge order for all rule sources.` (category: out-of-scope; reason: this slice proves the key reader behavior once the active `CCINIClass` exists; next-step-if-pursued: trace rules/art load order and scenario override application)
- `[DEFERRED] OQ-15 - Runtime crash signature for invalid empty SplashList at consumer.` (category: needs-runtime-debugger; reason: static consumer has no guard, but exact fault/state corruption should be observed under modified INI if needed; next-step-if-pursued: run debug build of gamemd with `SplashList` absent from all layers)

## 9. Visual/UI Composition Ledger

This report has no final draw-composition surface. It verifies visual asset references and constructor inputs consumed by the bouncer water branch.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| parser | `RulesClass__ReadCombatDamage @ 0x0066C18A..0x0066C287` | active rules load | `H2O_EXP3,H2O_EXP2,H2O_EXP1` names from INI | list order preserved by vector append | later `AnimClass::DrawIt` | Yes | data source |
| consumer | `AnimClass::AI @ 0x00423DD2..0x00423DD8` | non-meteor water impact | first list element | bouncer parent coords, z+3 per parent report | later `AnimClass::DrawIt` | Conditional | water splash row type |
| consumer | `AnimClass::AI @ 0x00423D29..0x00423D35` | meteor water impact | last list element | bouncer parent coords, z+3 per parent report | later `AnimClass::DrawIt` | Conditional | meteor splash row type |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `[CombatDamage] SplashList=` is parsed as an ordered `AnimTypeClass*` vector; stock value is `H2O_EXP3,H2O_EXP2,H2O_EXP1` | `0x0066C18A..0x0066C287`; `ini/rulesmd.ini:902` | missing parse/use | `src/rules/ruleset.rs`; possible `src/rules/combat_damage.rs`; future generic `AnimClass` runtime | store ordered anim references resolved like other anim names; preserve exact order | retail rules parse yields list `[H2O_EXP3,H2O_EXP2,H2O_EXP1]` | Do not hardcode the list outside rules parsing | `rules_combatdamage_splashlist_parses_ordered_anim_refs` |
| Missing or blank `SplashList` preserves existing vector rather than clearing or synthesizing stock defaults | `CCINIClass__ReadString @ 0x00528A10`; branch `0x0066C1A6..0x0066C23F` | unchecked/missing | rules merge/parser behavior | when layering INI data, absent or blank in a later layer must not delete a prior list if modeling binary reader passes | base list plus later blank override still leaves base list | Do not treat blank as an empty override unless a separate loader proof contradicts this reader | `rules_combatdamage_splashlist_missing_or_blank_preserves_existing_list` |
| Consumer has no empty-list guard: non-meteor reads first element; meteor reads last element using count | `0x00423DD2..0x00423DD8`; `0x00423D29..0x00423D35` | future runtime missing | future bouncer/meteor `AnimClass` runtime | water branch should use first/last list entries exactly and tests should assert precondition behavior for invalid data | stock non-meteor gets `H2O_EXP3`, stock meteor gets `H2O_EXP1`; invalid empty data is not silently replaced | Do not use middle entry, random entry, or Wake fallback for empty SplashList | `anim_bouncer_water_uses_first_and_last_splashlist_entries_without_fallback` |

### Negative Facts / Do Not Do

- Do not read `SplashList` from `[General]`; active reader uses `[CombatDamage]`.
- Do not hardcode `H2O_EXP3,H2O_EXP2,H2O_EXP1` as a binary fallback. It is stock INI data, not an embedded default string.
- Do not treat `+0xBC4` as the vector object; it is the buffer pointer. The vector object begins at `+0xBC0`.
- Do not clear the list on missing or trim-empty `SplashList=` when modeling this reader's pass; the zero-length path preserves the existing vector.
- Do not add safe fallback selection in the bouncer water consumer unless deliberately modeling invalid-data behavior outside exact stock parity; gamemd performs no visible bounds guard.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ANIMCLASS_BOUNCER_WATER_SPLASH_BRANCH_GHIDRA_REPORT.md` replacement for its deferred parser note:
  `RulesClass__ReadCombatDamage @ 0x0066C18A..0x0066C287 reads [CombatDamage] SplashList= into the DynamicVectorClass<AnimTypeClass*> at RulesClass+0xBC0. The buffer pointer is +0xBC4 and active count is +0xBD0. Missing or trim-empty SplashList preserves the existing vector; stock H2O_EXP3,H2O_EXP2,H2O_EXP1 comes from rules.ini/rulesmd.ini, not a hardcoded binary fallback. AnimClass::AI reads first entry for non-meteor water impacts and last entry for meteor water impacts with no empty-list guard.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md` line claiming `[General] SplashList=` should be replaced with:
  `SplashList is a [CombatDamage] rules key, read by RulesClass__ReadCombatDamage into RulesClass+0xBC0; [General] Wake= is the separate wake animation field.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/VOXELANIMCLASS_GHIDRA_REPORT.md` wording saying "count at some offset" should be replaced with:
  `The SplashList DynamicVectorClass begins at RulesClass+0xBC0; +0xBC4 is the AnimTypeClass** buffer pointer, +0xBC8 is capacity, +0xBD0 is active count, and +0xBD4 is grow amount.`

## Sources

- Ghidra read-only decompile: `RulesClass::Constructor @ 0x00665650`
- Ghidra read-only decompile/disassembly context: `RulesClass__ReadCombatDamage @ 0x0066BBB0`, SplashList site `0x0066C18A..0x0066C287`
- Ghidra read-only decompile: `CCINIClass__ReadString @ 0x00528A10`
- Ghidra read-only decompile: `AnimTypeClass__FindOrAllocate @ 0x00428B80`
- Ghidra read-only decompile: `DynamicVectorClass__Constructor @ 0x00525250`, `DynamicVectorClass__Add @ 0x005253B0`, `DynamicVectorClass__CopyFrom @ 0x00525060`, `VectorClass__Clear @ 0x005251C0`
- Ghidra read-only decompile/disassembly context: `AnimClass::AI @ 0x00423AC0`, consumer reads `0x00423D29..0x00423D35`, `0x00423DD2..0x00423DD8`
- INI checked: `ini/rulesmd.ini:902`, `ini/rules.ini:722`, `ini/rulesmd.ini:525`, `ini/rules.ini:519`
- Prior report referenced: `C:/Users/enok/Documents/ra2-rust-game/docs/research/ANIMCLASS_BOUNCER_WATER_SPLASH_BRANCH_GHIDRA_REPORT.md`
