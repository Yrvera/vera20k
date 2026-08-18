# Ghidra Label Spot-Check - Random FactoryClass vtable sample

**Address(es):** `0x007E88D0`, `0x004CA230`, `0x004C98B0`, `0x004C9B20`, `0x004CA770`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Random label sample from current Ghidra globals: test whether `vtable_FactoryClass @ 0x007E88D0` and sampled slot labels match the body evidence.
**Non-Scope:** Full FactoryClass production mechanics, every vtable slot, all save/load and CRC semantics.
**Confidence:** High for the sampled labels.
**Active in YR:** Yes. `FactoryClass` is live production infrastructure; this report only verifies label roles, not full gameplay formulas.

## 1. Overview

Random global sampling landed on `vtable_FactoryClass @ 0x007E88D0`. The table label is correct: the FactoryClass constructor installs this pointer at object offset `+0x00`, and the vtable contains FactoryClass-specific production methods.

The useful label-drift finding is inside the table: slot 3 points at `0x004CA230`, which current Ghidra decompiles as `FactoryClass__Update`, but the body is a COM/IPersist-style `GetClassID` implementation. That label is misleading.

## 2. Class Layout / Key Offsets

| Address / field | Verified role | Evidence |
|---|---|---|
| `0x007E88D0` | FactoryClass primary vtable | constructor `0x004C98B0` writes it to `this+0x00` |
| `0x007E88B4` | FactoryClass secondary vtable at `this+0x04` | constructor write |
| `0x007E88AC` | FactoryClass secondary vtable at `this+0x08` | constructor write |
| `0x007E88A4` | FactoryClass secondary vtable at `this+0x0C` | constructor write |
| `FactoryClass+0x24` | production progress value | `FactoryClass__GetProgress @ 0x004CA120` returns it |
| `FactoryClass+0x58` | produced object pointer | `FactoryClass__IsComplete @ 0x004CA130` checks it |
| `FactoryClass+0x68` | special item id | `FactoryClass__IsComplete @ 0x004CA130` checks it against `-1` |

## 3. Core Logic

### Vtable label

`read_memory 0x007E88D0 len 96` decodes the first 24 primary-vtable dwords as:

| Slot | Offset | Target | Sampled role |
|---:|---:|---|---|
| 0 | `+0x00` | `0x00410260` | inherited QueryInterface-like shell |
| 1 | `+0x04` | `0x00410300` | inherited AddRef stub |
| 2 | `+0x08` | `0x00410310` | inherited Release stub |
| 3 | `+0x0C` | `0x004CA230` | mislabeled `FactoryClass__Update`; actually GetClassID-like |
| 4 | `+0x10` | `0x00410450` | inherited IsDirty |
| 5 | `+0x14` | `0x004CA270` | FactoryClass load-like stream method |
| 6 | `+0x18` | `0x004CA3C0` | FactoryClass save-like stream method |
| 7 | `+0x1C` | `0x004103E0` | inherited GetSizeMax |
| 8 | `+0x20` | `0x004CA770` | FactoryClass destructor / scalar-delete style wrapper |
| 13 | `+0x34` | `0x004CA430` | checksum/debug registration-style method |
| 23 | `+0x5C` | `0x004C9B20` | `FactoryClass__AI` |

The constructor at `0x004C98B0` calls `AbstractClass__Constructor_Full`, initializes FactoryClass production fields, writes all four FactoryClass vtable pointers, assigns an Abstract ID, and registers the object in `g_FactoryClass_Array`. That proves `vtable_FactoryClass` is not a random data-table label.

### Mislabeled sampled function: `0x004CA230`

Current decompile name: `FactoryClass__Update`.

Verified body behavior:

- reads output pointer from stack arg 2
- if null, returns `0x80004003`
- copies four dwords from `0x007E9820..0x007E982C` into that output pointer
- returns `0`
- stack cleanup is `RET 8`

Raw bytes at entry:

`8B 44 24 08 85 C0 75 08 B8 03 40 00 80 C2 08 00 ... 33 C0 C2 08 00`

The 16 source bytes at `0x007E9820` are:

`A8 D9 EC 34 B0 0A D2 11 AC A7 00 60 08 05 5B B5`

That shape is a GUID/CLSID copy routine, not a production update or tick. Because slot 3 in the inherited Abstract/IPersist-style vtable position is normally `GetClassID`, the safest verified role is:

`FactoryClass_GetClassID_like_0x004CA230`

### Other sampled slots

`0x004C9B20` decompiles as `FactoryClass__AI` and matches the label at a high level: it advances production when not suspended, uses the CDTimer, changes `Production_Value`, spends money, rolls back one progress step on insufficient funds, and marks completion at `0x36`.

`0x004CA770` is FactoryClass destructor-like infrastructure: it resets FactoryClass vtables, detaches from lists, removes from `g_FactoryClass_Array`, conditionally abandons production when `g_GameActive != 0`, frees queued-object storage if owned, calls `AbstractClass__Destructor_ResetVtables`, then has optional free behavior through the scalar-delete flag path. Current generic name `FactoryClass__vtable_8` is underspecified but not directionally wrong.

## 4. INI Keys

None for the label test. FactoryClass production behavior is data-driven elsewhere, but this spot-check only tests vtable/slot naming.

## 5. Integration Points

`FactoryClass__Constructor @ 0x004C98B0` installs the `0x007E88D0` primary vtable and appends the object to `g_FactoryClass_Array`. `FactoryClass__AI @ 0x004C9B20` is slot 23 (`+0x5C`) in that table, matching the non-object global loop reports that tick factories outside the main ObjectClass live vector.

## 6. Current Rust Implementation Status

No Rust change is implied by this label test. For future production research, avoid treating `0x004CA230` as an "Update" or gameplay tick function. The production tick role belongs to `FactoryClass__AI @ 0x004C9B20`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `vtable_FactoryClass @ 0x007E88D0` | verified | constructor `0x004C98B0` installs it; vtable bytes contain FactoryClass methods | full slot naming out of scope |
| `0x004CA230` current `FactoryClass__Update` label | verified-misleading | body copies GUID/CLSID bytes and returns HRESULT | rename/comment if editing Ghidra labels |
| `0x004C9B20` `FactoryClass__AI` label | verified-high-level | decompile shows production tick logic | exact production formula out of scope |
| `0x004CA770` slot 8 destructor wrapper | touched-not-exhausted | decompile shows destructor-like cleanup and optional free path | exact calling convention/flag register needs a focused destructor audit |
| all FactoryClass slots | deferred | random spot-check only | full vtable report if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Is sampled data label vtable_FactoryClass correct? -> Yes; FactoryClass constructor writes `0x007E88D0` to `this+0x00`.` (evidence: `0x004C98B0`)
- `[RESOLVED] OQ-002 - Is sampled slot 3 label FactoryClass__Update correct? -> No; body is GetClassID-like GUID copy with null-output HRESULT handling.` (evidence: `0x004CA230`, data `0x007E9820`)
- `[RESOLVED] OQ-003 - Is sampled AI label plausible? -> Yes; `0x004C9B20` is production tick logic and is slot `+0x5C` in the FactoryClass vtable.` (evidence: `0x007E88D0` bytes, `0x004C9B20` decompile)
- `[DEFERRED] OQ-004 - Are all FactoryClass vtable slot labels correct?` (category: out-of-scope; reason: random spot-check only; next-step-if-pursued: full FactoryClass vtable audit)

## 9. Visual/UI Composition Ledger

Not applicable.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x004CA230` is GetClassID-like, not FactoryClass update logic. | decompile/raw bytes at `0x004CA230`; GUID data at `0x007E9820` | none | future production research docs | Cite it as class-id/persistence infrastructure only. | A FactoryClass production report must not use `0x004CA230` as tick/update evidence. | Do not trust current decompile name `FactoryClass__Update` here. |
| Factory production tick label belongs to `0x004C9B20`. | vtable slot `+0x5C`, decompile body | unchecked in this report | `src/sim/production/production_queue.rs` if production parity work resumes | Use `0x004C9B20` and deeper production reports for build-progress semantics. | Build-progress parity tests should cite FactoryClass AI / CalcRate reports, not slot 3. | Do not conflate IPersist/COM vtable slots with gameplay virtuals. |

## Sources

- Ghidra MCP: `list_globals offset 1729` random sample
- Ghidra MCP: `read_memory 0x007E88D0 length 96`
- Ghidra MCP: `decompile_function 0x004C98B0`
- Ghidra MCP: `decompile_function 0x004CA230`
- Ghidra MCP: `read_memory 0x004CA230 length 96`
- Ghidra MCP: `read_memory 0x007E9820 length 16`
- Ghidra MCP: `decompile_function 0x004C9B20`
- Ghidra MCP: `decompile_function 0x004CA120`
- Ghidra MCP: `decompile_function 0x004CA130`
- Ghidra MCP: `decompile_function 0x004CA770`
