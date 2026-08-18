# Generic Transport Cargo Insertion Order - Ghidra Research Report

**Address(es):** `0x004733A0` (`CargoClass::AddPassenger`), `0x00473430` (cargo head pop), `0x00739EC0` (`UnitClass::PerCellProcess` generic boarding branch), `0x0073D630` (`UnitClass::Mission_Deploy_Building` rollback context)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Generic vehicle transport passenger insertion/splice order that determines player-visible unload order after the already-verified cargo head-pop unload primitive.  
**Non-Scope:** Generic unload placement search, Scatter/garrison ejection, carryall/paradrop ordering, full radio protocol, and the exact semantic name of `Object+0x14` bit `0x4` beyond its direct use in `CargoClass::AddPassenger`.  
**Confidence:** High for ordinary generic transport order; Medium for the semantic meaning and active content frequency of pre-linked bit-`0x4` passenger tails.  
**Active in YR:** Yes for ordinary generic vehicle transport boarding/unload; Conditional for the pre-linked bit-`0x4` splice branch when the inserted passenger already heads a linked chain whose first tail node has `Object+0x14 & 0x4`.

## 0. Investigation Contract

Target question: After the verified generic transport unload primitive pops the cargo linked-list head, does `CargoClass::AddPassenger @ 0x004733A0` make player-visible generic vehicle transport unload FIFO, LIFO, or a special splice order?

Non-goals: Do not re-study generic unload placement except for head-pop/rollback context. Do not study CanBeOccupied sell/destruction ejection, infantry Scatter, carryall pickup/drop, paradrop payload order, or full building garrison behavior except where caller xrefs distinguish them from generic vehicle boarding.

Evidence needed to mark COMPLETE:

- Decompile plus assembly for `CargoClass::AddPassenger @ 0x004733A0`.
- Decompile plus assembly for the head-pop primitive `0x00473430`.
- Decompile plus xref/caller evidence proving generic vehicle boarding reaches `AddPassenger`.
- Decompile plus assembly for no-exit rollback re-add only where insertion order requires it.
- Current Rust comparison for `PassengerCargo::board` / `unload_first`.

Stop conditions:

- Ghidra MCP is read-only; no rename/comment/save/create operations.
- If Ghidra missed a boundary, inspect only read-only bytes/callers and record uncertainty.
- Write exactly this report plus the shared claims file.
- Stop when the open questions log has no un-deferred material questions for this slice and a final pass over `0x004733A0`, `0x00473430`, and the `0x00739EC0` boarding call added no new order-affecting branches.

## 1. Overview

Ordinary generic vehicle transport boarding is LIFO at the player-visible unload surface. The generic boarding branch in `UnitClass::PerCellProcess @ 0x00739EC0` calls the target transport's vtable `+0x394`, which resolves through `FUN_00710670` to `CargoClass::AddPassenger`; `AddPassenger` prepends the new passenger to `CargoClass+4` head, and the generic unload primitive pops that same head.

`AddPassenger` has one special-splice rule: if the passenger being inserted already has a `+0x30` linked tail and the first tail node has `Object+0x14` bit `0x4` set, the function keeps the contiguous bit-`0x4` tail immediately behind the inserted passenger and attaches the old cargo head after that tail. This is not FIFO and is not a sort of existing cargo; it only preserves a pre-existing chain on the inserted passenger.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x114` | `FootClass` / transport object | Embedded `CargoClass` used by generic transport cargo | `UnitClass::PerCellProcess` uses `LEA ECX,[ESI+0x114]` / vtable `+0x394`; `FUN_004DE710` uses `LEA EDI,[ESI+0x114]` | Yes |
| `+0x0` | `CargoClass` | Stored cargo count, recomputed by `AddPassenger` over inserted head plus contiguous bit-`0x4` tail; decremented by pop | `0x00473403..0x00473424`, `0x00473445` | Yes |
| `+0x4` | `CargoClass` | Cargo linked-list head | `0x004733F2..0x00473400`, `0x00473438..0x0047343B` | Yes |
| `+0x30` | passenger/object | Cargo next pointer / pre-linked tail pointer | `0x004733B6`, `0x004733D0`, `0x004733F5`, `0x004733FD`, `0x0047343E` | Yes |
| `+0x14` bit `0x4` | passenger/object | Special tail-continuation flag checked while inserting/recounting | `0x004733BD..0x004733C3`, `0x004733D7..0x004733DD`, `0x00473415..0x0047341B` | Conditional |
| `+0x5E0` | `TechnoType` / transport type | `Passengers`; positive value gates generic vehicle boarding and unload | boarding gate `0x0073A6B9..0x0073A6CB`; parser/unload context from prior report | Yes |
| `+0x5E4` | `UnitType` | `OpenTopped`; boarding sets in-open-transport state after cargo insertion | `0x0073A746..0x0073A75D`; prior generic unload clear context | Conditional on OpenTopped=yes |
| `+0x11C` | passenger | Back-reference to containing transport written after boarding, cleared during unload | `0x0073A762..0x0073A768`; prior unload report at `0x0073DBC9` | Yes |

## 3. Core Logic

### 3.1 `CargoClass::AddPassenger @ 0x004733A0`

Pseudocode, preserving order semantics:

1. If passenger is null, return without changing cargo.
2. Call passenger virtual `+0xD4` before insertion.
3. Inspect `passenger+0x30`.
4. If `passenger+0x30` is non-null and that first tail node has bit `0x4`, walk forward through contiguous tail nodes while each next node has bit `0x4`.
5. If that walk reaches the end of the contiguous flagged tail, write `tail_last+0x30 = old_cargo_head`.
6. Otherwise write `passenger+0x30 = old_cargo_head`.
7. Write `cargo_head = passenger`.
8. Set `cargo_count = 0`, then count the inserted head and any following contiguous bit-`0x4` tail; stop when next is null or next is not bit-`0x4`.

Assembly evidence:

- Null passenger guard: `0x004733A8..0x004733AA`.
- Pre-insertion virtual call: `0x004733AC..0x004733B0` calls `[passenger_vtable+0xD4]`.
- Tail test from inserted passenger, not old cargo head: `0x004733B6..0x004733CA` reads `[ESI+0x30]`, then `[EAX+0x14] >> 2 & 1`.
- Contiguous flagged-tail walk: `0x004733D0..0x004733EC`.
- Special splice attaches old cargo head after the flagged tail: `0x004733F2..0x004733F5` writes `[tail+0x30] = [cargo+4]`.
- Ordinary prepend attaches old cargo head directly after inserted passenger: `0x004733FA..0x004733FD` writes `[passenger+0x30] = [cargo+4]`.
- New head write: `0x00473400` writes `[cargo+4] = passenger`.
- Count recompute stops after the first following non-bit-`0x4` node: `0x00473403..0x0047342A`.

Active in YR: Yes for all cargo insertion callers, including generic vehicle boarding. The bit-`0x4` splice sub-branch is Conditional: it requires a non-null pre-existing `passenger+0x30` tail whose first node has bit `0x4`; ordinary single-passenger vehicle boarding does not require that condition.

### 3.2 Head-pop primitive `0x00473430`

`0x00473430` removes and returns the current cargo head:

- `0x00473430..0x00473435`: read `[CargoClass+4]`; return if null.
- `0x00473438..0x0047343B`: move old head's `[+0x30]` into `[CargoClass+4]`.
- `0x0047343E`: clear old head `[+0x30]`.
- `0x00473445`: decrement `[CargoClass+0]`.
- The function leaves the old head in `EAX`; caller `FUN_004DE710 @ 0x004DE710` saves it in `EBX` at `0x004DE722` and returns it at `0x004DE749`.

Active in YR: Yes. Prior report verified generic unload state 3 reaches this primitive through `FUN_004DE710`; this report rechecked the primitive's actual head-pop semantics.

### 3.3 Generic vehicle boarding reaches `AddPassenger`

The generic vehicle boarding path is inside `UnitClass::PerCellProcess @ 0x00739EC0`, reached when a passenger with mission `7` is in the same cell as a non-self object that can contain passengers:

- Same-cell object and mission gate: `0x0073A64A..0x0073A6A3` requires passenger mission `7`, non-null target object, not self, and equal cell coordinates.
- Acceptance/context gate: `0x0073A6A9..0x0073A6CB` calls a containment check and requires target type `+0x5E0 > 0` (`Passengers`).
- Radio/permission gate: `0x0073A6D1..0x0073A6E2` calls passenger virtual `+0x278` with message `0x0F` and the target; only return `1` boards.
- Passenger detachment/limbo preparation: `0x0073A6E8..0x0073A735` clears ghost/on-bridge/motion/contact state, frees mind-control if needed, and calls passenger virtual `+0xD4`.
- Insertion call: `0x0073A73B..0x0073A740` calls target vtable `+0x394` with the passenger. `FUN_00710670 @ 0x00710670`, the vtable implementation found from xrefs/data, performs `LEA ECX,[transport+0x114]` and calls `CargoClass::AddPassenger @ 0x004733A0` at `0x0071067B..0x00710682`.
- Open-topped/back-reference continuation: `0x0073A746..0x0073A768` optionally sets in-open-transport state and writes passenger `+0x11C = transport`.

Active in YR: Yes. This branch is keyed by live `Passengers > 0` unit types such as IFV/BFRT/LCRF/SAPC/YHVR and is the generic vehicle transport boarding path, not the garrison building branch.

### 3.4 Rollback re-add on generic unload failure

Only the order-relevant rollback was checked here. Prior unload report verified state 3 pops a cargo head, then tries placement. If placement cannot find a valid direction, it re-adds the same popped passenger through `CargoClass::AddPassenger`:

- Prior report evidence: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, failure branch `0x0073DC71..0x0073DC78` calls `CargoClass::AddPassenger @ 0x004733A0`.
- Order effect from this report: because the failed passenger was just popped and `0x00473430` cleared its `+0x30`, ordinary rollback prepends the same passenger back to cargo head, preserving retry order for the next unload attempt.
- If the failed passenger still has a pre-linked bit-`0x4` tail for some reason, `AddPassenger` would apply the same special-splice rule, but the ordinary pop primitive clears only the old head's `+0x30`.

Active in YR: Yes for no-exit/no-placement failure in generic transport unload; Conditional for any pre-linked tail on the re-added passenger.

## 4. INI Keys

| Key | Default/source | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `Passengers=` | `0` by default; positive on stock transports in `rules.ini` / `rulesmd.ini` | Gates generic vehicle boarding/unload through `Type+0x5E0 > 0` | parser/unload context from `GENERIC_TRANSPORT_MANUAL_UNLOAD_MAPPING_GHIDRA_REPORT.md`; boarding gate `0x0073A6B9..0x0073A6CB` | Yes |
| `SizeLimit=` | `0` default, stock transports set limits | Not used by `AddPassenger` itself; admission is already decided before insertion | Rust scan and no `SizeLimit` read in `0x004733A0` | Yes but out of insertion order |
| `OpenTopped=` | default false, BFRT true | Does not change cargo list order; only sets/clears open-transport passenger state around boarding/unload | `0x0073A746..0x0073A75D`, prior unload clear `0x0073DB85..0x0073DB98` | Conditional |
| `EnterTransportSound=` | stock transports define sounds | Played by wrapper after `AddPassenger`; does not affect order | `FUN_00710670 @ 0x0071068B..0x007106D0` | Conditional on sound id != -1 |

## 5. Integration Points

Direct xrefs to `CargoClass::AddPassenger @ 0x004733A0` include carryall, aircraft/paradrop, save/load or spawning helpers, garrison/building paths, generic vehicle boarding, generic unload rollback, and `UnitClass::PerCellProcess`. For this target, only two are order-critical:

- Generic vehicle boarding: `UnitClass::PerCellProcess @ 0x00739EC0`, call at `0x0073A73B..0x0073A740` through target vtable `+0x394`, implementation `FUN_00710670 @ 0x00710670`.
- Generic unload rollback: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, direct call at `0x0073DC71..0x0073DC78` after placement failure.

Negative caller distinction:

- `0x0073A237..0x0073A2E4` is a building/garrison-like branch keyed by target building type byte `+0x16AE`, not the generic vehicle transport branch; it calls `AddPassenger` directly on `EBX+0x114`.
- Carryall and aircraft payload callers were listed by xref but not re-studied because they do not determine generic vehicle transport unload order.

## 6. Current Rust Implementation Status

Observed Rust surface: `src/sim/passenger.rs`.

- `PassengerCargo.passengers` is documented as "boarding order (FIFO unload)" and `board()` appends with `Vec::push`.
- `unload_first()` removes index `0`, so current ordinary behavior is FIFO.
- `tick_boarding()` calls `cargo.board(pax_id, pax_size)` for both transports and garrisons; it does not distinguish vehicle transport LIFO from garrison order needs.
- `tick_unloading()` calls `cargo.unload_first()` after it has already selected an exit cell; it does not currently model binary pop-then-rollback ordering for failed placement. Prior unload report already covers the placement mismatch; this report only confirms the order consequence.

Current Rust delta: generic vehicle transport cargo order is mismatched for ordinary boarding. It should not be represented as FIFO if the same container semantics are used for generic vehicle transport unload. Because garrison fire/ejection uses occupant order separately, implementers should avoid blindly flipping all `PassengerCargo` consumers without checking garrison parity.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CargoClass::AddPassenger @ 0x004733A0` ordinary insertion | verified | decompile and assembly `0x004733A0..0x0047342C` | none |
| `CargoClass::AddPassenger` bit-`0x4` pre-linked-tail splice | verified mechanically | decompile and assembly `0x004733B6..0x004733F8` | exact semantic name/content frequency of bit `0x4` tail is not proven |
| `CargoClass::AddPassenger` count recompute | verified | assembly `0x00473403..0x0047342A` | whether count intentionally excludes old unflagged cargo tail after special splice is not separately named |
| `0x00473430` head pop | verified | decompile and assembly `0x00473430..0x00473447` | none |
| Generic vehicle boarding caller | verified | `UnitClass::PerCellProcess` decompile and assembly `0x0073A64A..0x0073A78C`; wrapper `0x00710670..0x00710682` | none for order |
| Generic unload rollback caller | verified for order | prior report plus call range `0x0073DC71..0x0073DC78`; `AddPassenger` mechanics in this report | placement details remain covered by prior report, not this one |
| Carryall / aircraft / paradrop AddPassenger callers | deferred | xrefs list `00415EB8`, `00416C4E`, `0041729E`, `0041A048`, `0041A0B7` | out-of-scope for generic vehicle transport |
| Building garrison direct AddPassenger branch | touched-not-exhausted | `0x0073A237..0x0073A2E4` | out-of-scope except as negative distinction |
| Current Rust `PassengerCargo` generic vehicle order | verified enough for handoff | `passenger.rs` scan, `board()` push + `unload_first()` remove(0) | exact garrison order must be handled by future implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this exhaustive-slice or coverage-map? -> exhaustive-slice for cargo insertion/unload order only` (evidence: target scope and primary functions `0x004733A0`, `0x00473430`, `0x00739EC0`)
- `[RESOLVED] OQ-2 - Does `AddPassenger` append, prepend, or sort ordinary passengers? -> ordinary path prepends new passenger to cargo head` (evidence: `0x004733FA..0x00473400`)
- `[RESOLVED] OQ-3 - Does the special splice inspect the existing cargo head? -> no; it inspects the inserted passenger's existing `+0x30` tail, then splices old head after that tail` (evidence: `0x004733B6..0x004733F8`)
- `[RESOLVED] OQ-4 - Does unload pop head or tail? -> head` (evidence: `0x00473430..0x00473447`)
- `[RESOLVED] OQ-5 - Does generic vehicle boarding reach `AddPassenger`? -> yes via `UnitClass::PerCellProcess` target vtable `+0x394` to `FUN_00710670` then `CargoClass::AddPassenger`` (evidence: `0x0073A73B..0x0073A740`, `0x0071067B..0x00710682`)
- `[RESOLVED] OQ-6 - Is the generic boarding branch active in YR? -> yes when target type `Passengers > 0` and same-cell boarding radio returns `1`` (evidence: `0x0073A6B9..0x0073A6E2`; stock `Passengers=` INI entries)
- `[RESOLVED] OQ-7 - Does OpenTopped affect cargo order? -> no; it runs after insertion and writes passenger open-transport state` (evidence: `0x0073A746..0x0073A75D`)
- `[RESOLVED] OQ-8 - Does no-exit rollback preserve the failed passenger as next to retry? -> yes for ordinary popped head, because pop clears `+0x30` and rollback prepends through `AddPassenger`` (evidence: `0x0047343E`, prior call `0x0073DC71..0x0073DC78`, insertion `0x004733FA..0x00473400`)
- `[RESOLVED] OQ-9 - Does `AddPassenger` itself enforce capacity or SizeLimit? -> no order-relevant capacity/size check in `0x004733A0`; admission occurs before insertion` (evidence: full decompile `0x004733A0`; boarding gate before call)
- `[RESOLVED] OQ-10 - Is the `0x0073A237` AddPassenger branch generic vehicle boarding? -> no, it is keyed by building type byte `+0x16AE` and uses direct `LEA ECX,[EBX+0x114]`` (evidence: `0x0073A237..0x0073A2E4`)
- `[DEFERRED] OQ-11 - What is the exact semantic name of `Object+0x14` bit `0x4`?` (category: bounded-cost-too-high; reason: not needed to decide ordinary generic vehicle FIFO/LIFO; next-step-if-pursued: audit all readers/writers of `+0x14 & 0x4`)
- `[DEFERRED] OQ-12 - Which stock YR runtime scenarios insert a passenger with a pre-linked bit-`0x4` tail into a generic vehicle?` (category: needs-runtime-debugger; reason: mechanical splice is verified, but live frequency requires runtime observation or writer audit; next-step-if-pursued: watch passenger `+0x30` before `0x004733A0` in standard vehicle boarding)
- `[DEFERRED] OQ-13 - Should garrison occupant ordering share the same Rust container semantics?` (category: requires-different-system-context; reason: garrison fire/ejection order has separate player-visible surfaces; next-step-if-pursued: synthesize garrison insertion/fire/ejection docs)
- `[DEFERRED] OQ-14 - Do aircraft/paradrop/carryall callers need the same special-splice representation?` (category: out-of-scope; reason: this target is generic vehicle transport; next-step-if-pursued: separate aircraft cargo order investigation)
- `[RESOLVED] OQ-15 - Did the final zero-add pass find another order-affecting branch in primary functions? -> no` (evidence: re-read `0x004733A0`, `0x00473430`, `0x0073A64A..0x0073A78C`, and rollback context)

Adversarial corner-case answers:

- Null passenger to `AddPassenger`: no change, return. Active in YR: Conditional defensive path. Evidence: `0x004733A8..0x004733AA`.
- Empty cargo before ordinary boarding: new passenger becomes head and count becomes `1`. Active in YR: Yes. Evidence: `0x004733FA..0x0047340C`.
- Cargo already has passengers before ordinary boarding: new passenger becomes head, old head becomes `new+0x30`, so next unload is the newest boarder. Active in YR: Yes. Evidence: `0x004733FA..0x00473400` plus head-pop `0x00473430`.
- Inserted passenger already has bit-`0x4` tail: new head stays first, flagged tail follows, old cargo starts after flagged tail. Active in YR: Conditional. Evidence: `0x004733B6..0x004733F8`.
- Placement failure after pop: popped ordinary passenger is re-added at head, so it remains next to retry rather than rotating behind later passengers. Active in YR: Yes. Evidence: `0x0047343E`, `0x0073DC71..0x0073DC78`, `0x00473400`.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary generic vehicle boarding prepends to cargo head; head-pop unload therefore makes ordinary generic vehicle unload LIFO | `0x0073A73B..0x0073A740`, `0x0071067B..0x00710682`, `0x004733FA..0x00473400`, `0x00473430..0x00473447` | mismatch: `board()` pushes and `unload_first()` removes first, FIFO | `src/sim/passenger.rs::PassengerCargo`, `tick_boarding`, `tick_unloading` | Generic vehicle transport cargo should unload newest ordinary boarder first | Board infantry A, then B into an APC; manual generic unload places B before A | Do not globally flip garrison order without checking garrison fire/ejection parity; proposed test: `generic_transport_unload_is_lifo_for_ordinary_boarding` |
| `AddPassenger` has a pre-linked-tail special splice: inserted passenger remains head, contiguous tail nodes with `Object+0x14 & 0x4` stay immediately behind it, old cargo head follows after tail | `0x004733B6..0x004733F8`; count loop `0x00473403..0x0047342A` | missing/unsupported; Rust cargo is flat IDs with no special linked-tail metadata | needed cargo insertion helper or cargo-chain representation if this case becomes relevant | Preserve a pre-linked special tail as an atomic head group before older cargo | Construct cargo `[old1, old2]`, insert `new -> flagged_tail1 -> flagged_tail2`; unload order is `new, flagged_tail1, flagged_tail2, old1, old2` | Do not implement as "insert behind existing flagged head"; the binary inspects the inserted passenger's tail, not current cargo; proposed test: `cargo_add_passenger_splices_prelinked_flagged_tail_before_old_head` |
| No-exit generic unload rollback re-adds the popped passenger through `AddPassenger`, preserving ordinary retry order at head | pop `0x00473430..0x00473447`; rollback call `0x0073DC71..0x0073DC78`; ordinary prepend `0x004733FA..0x00473400` | partial mismatch: current Rust skips pop when no free cell; prior placement report already covers broader order | `src/sim/passenger.rs::tick_unloading` failure path | If Rust models pop-before-placement, rollback must restore the failed passenger to cargo head, not append to tail | With two passengers and no valid exits, tick unload and then open one exit; the same would-be head passenger unloads first | Do not rotate blocked passengers behind later cargo; proposed test: `generic_transport_unload_failure_restores_popped_head_for_retry` |

Stale Docs / Follow-up Docs:

- Path: `docs/research/GENERIC_TRANSPORT_MANUAL_UNLOAD_MAPPING_GHIDRA_REPORT.md`
- Replacement wording: `Generic vehicle transport unload pops the cargo linked-list head. Ordinary generic vehicle boarding inserts each newly accepted passenger at the cargo head through CargoClass::AddPassenger @ 0x004733A0, so ordinary player-visible generic transport unload is LIFO (newest boarder unloads first), not FIFO. AddPassenger has a conditional pre-linked-tail splice: if the passenger being inserted already has a +0x30 tail whose first node has Object+0x14 bit 0x4 set, the contiguous flagged tail remains immediately behind the inserted passenger and the old cargo head is attached after that tail.`

## 10. Negative Facts / Do Not Do

- Do not keep describing ordinary generic vehicle transport cargo as FIFO after this report; binary boarding prepends and unload pops head. Active in YR: Yes. Evidence: `0x004733FA..0x00473400`, `0x00473430..0x00473447`.
- Do not implement the special splice as "insert behind an existing leading passenger flagged bit `0x4`"; `AddPassenger` checks the inserted passenger's existing `+0x30` tail, not the old cargo head. Active in YR: Conditional. Evidence: `0x004733B6..0x004733F8`.
- Do not append a failed no-exit unloaded passenger to cargo tail; rollback uses `AddPassenger`, which ordinary-prepends. Active in YR: Yes. Evidence: `0x0073DC71..0x0073DC78`, `0x004733FA..0x00473400`.
- Do not use `SizeLimit` or capacity logic to infer order; `CargoClass::AddPassenger` does no size/capacity check in the inspected function. Active in YR: Yes. Evidence: `0x004733A0` full decompile.
- Do not merge generic vehicle transport order with CanBeOccupied garrison order without separate proof; the `0x0073A237..0x0073A2E4` direct `AddPassenger` branch is distinct from the generic vehicle branch at `0x0073A64A..0x0073A78C`. Active in YR: Conditional by target type. Evidence: both disassembly ranges.

## 11. Remaining Uncertainty

- Exact semantic name and writer set for `Object+0x14` bit `0x4` remains unresolved; the mechanical splice rule is verified, but the report does not prove how often stock YR generic vehicle boarding presents a pre-linked flagged tail.
- Whether Rust should model the special splice immediately depends on whether any implemented stock scenario can create that pre-linked chain before vehicle boarding. Ordinary vehicle cargo order does not depend on this uncertainty.
- Garrison occupant ordering should be reconciled with existing garrison fire/ejection reports before changing shared `PassengerCargo` semantics globally.

## Sources

- Ghidra decompile/disassembly: `CargoClass::AddPassenger @ 0x004733A0`, `0x004733A0..0x0047342C`.
- Ghidra decompile/disassembly: cargo head pop `0x00473430`, `0x00473430..0x00473447`.
- Ghidra decompile/disassembly: `FUN_004DE710 @ 0x004DE710`, `0x004DE710..0x004DE74D`.
- Ghidra decompile/disassembly: `FUN_00710670 @ 0x00710670`, `0x00710670..0x007106DA`.
- Ghidra decompile/disassembly: `UnitClass::PerCellProcess @ 0x00739EC0`, relevant ranges `0x0073A237..0x0073A2E4` and `0x0073A64A..0x0073A78C`.
- Prior context report: `docs/research/GENERIC_TRANSPORT_MANUAL_UNLOAD_MAPPING_GHIDRA_REPORT.md`.
- Current Rust scan: `src/sim/passenger.rs`.

## Status

COMPLETE for generic vehicle cargo insertion order after verified head-pop unload. The result is ordinary LIFO with a conditional inserted-passenger pre-linked-tail splice, not FIFO.
