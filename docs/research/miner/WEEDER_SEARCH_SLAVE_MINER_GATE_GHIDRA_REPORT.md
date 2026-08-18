# Weeder Search Variant (0x004ddb90) and Slave-Miner Host Gate in Mission_Harvest

Swarm 2026-07-28T12:25, slot 2. Read-only Ghidra MCP investigation. gamemd.exe, image base 0x400000.

## Working notes (required four lines)

- **Target question**: What does 0x004ddb90 do internally and how does it differ from
  0x004dcfe0? What exactly does the slave-miner preamble gate at the top of
  `UnitClass::Mission_Harvest` (0x0073e5e0) check (type+0x5EC, type+0x5ED, this+0x2D8), which
  INI keys write type+0x5EC/0x5ED, what is this+0x2D8, and what does 0x006b0db0 do?
- **Non-goals**: cadence/state timing (already covered in
  `docs/scans/trace-swarm-20260728/mission-harvest-cadence.md`), ore-tick storage math (already
  covered in `harvest-ore-tick.md`), full SlaveManagerClass method inventory, full
  TechnoTypeClass::ReadINI field map beyond the fields directly implicated here.
- **Evidence needed to mark COMPLETE**: decompile + assembly for 0x004ddb90 and 0x004dcfe0;
  decompile + assembly for the Mission_Harvest preamble; xref chain from the two struct-offset
  writes back to literal INI key strings; decompile of 0x006b0db0 and confirmation of its
  identity via caller cross-check; identification of this+0x2D8 via its allocation site.
- **Stop conditions**: Ghidra MCP unreachable (did not occur); a required function boundary
  missing (did not occur — all target functions were already defined).

## 0x004ddb90 vs 0x004dcfe0: internals and the real difference

Both are `__thiscall`, both early-return 0 if `param_1[0x169]` (an in-progress order/lock field)
is non-zero. Beyond that they are different helpers:

- **0x004ddb90** (`FootClass__Search_For_Tiberium_Short_And_Move`, 2 params: `this, scanRange`)
  calls **0x004dd9?? → FootClass__Scan_For_Tiberium_NoZone** (non-virtual, fixed target)
  passing `scanRange` straight through. `Scan_For_Tiberium_NoZone` spirals outward from the
  unit's own cell up to `scanRange` rings and tests each ring cell with
  **`FootClass__Is_Cell_Weedable`** — not a Tiberium/ore-bin check. It ignores Zone/connectivity
  (hence "NoZone" in the decompiled name) — verified via `decompile_function 0x004ddb90` and
  `decompile_function` of the callee (`FootClass__Scan_For_Tiberium_NoZone`), which shows the
  `FootClass__Is_Cell_Weedable` calls directly.
- **0x004dcfe0** (`FootClass__Search_For_Tiberium_And_Move`, 3 params: `this, scanRange, flag`)
  calls **a virtual method through the unit's own vtable slot +0x338**
  (`(**(code**)(*param_1+0x338))(&param_2, param_2)`), i.e. it is polymorphic per concrete
  FootClass subtype and is Zone-aware (that's the counterpart to the other function's explicit
  "NoZone"). It also threads an extra `flag` parameter through to that virtual call — verified
  via `decompile_function 0x004dcfe0`.

**Conclusion**: 0x004ddb90 is a genuine, distinct "look for weeds, not ore, ignore zones" search —
it is not a cosmetic rename of the harvester search, it calls a different terminal predicate
(`Is_Cell_Weedable`) than the harvester path (Tiberium-bin/virtual scan). Active in YR:
**Conditional** — see dead-code finding below.

### Call sites (verified via `disassemble_function 0x0073e5e0` + `get_xrefs_to`/`get_function_callers`)

Both 0x004ddb90 and 0x004dcfe0 have exactly one caller: `UnitClass__Mission_Harvest`
(0x0073e5e0), each called from two sites:

| Mission_Harvest site | Guard flag | Scan-range field | Calls |
|---|---|---|---|
| state-0 initial entry (after piggyback-locomotion release, ~0x0073e762) | `type+0xE0F` (Weeder) | `g_RulesClass_Instance+0x177c` (**TiberiumLongScan**) | Weeder!=0 → 0x004ddb90; Weeder==0 → 0x004dcfe0 |
| state-1 re-search (Harvest_Ore_Tick still hungry, ~0x0073ea8d) | `type+0xE0E` (Harvester) | `g_RulesClass_Instance+0x1778` (**TiberiumShortScan**) | Harvester!=0 → 0x004dcfe0; Harvester==0 → 0x004ddb90 |

RulesClass offset mapping verified via `get_xrefs_to` on the `"TiberiumLongScan"` (0x0083c2b4)
and `"TiberiumShortScan"` (0x0083c2c8) strings → both resolve into `RulesClass__ReadGeneral`;
`get_assembly_context` on those two xref instructions shows the read for `"TiberiumShortScan"`
is stored to `[ESI+0x1778]` and the read for `"TiberiumLongScan"` is stored to `[ESI+0x177c]`
(confirmed via `get_assembly_context xref_sources=006702b8,00670299`). This matches, and
confirms, the parent trace-swarm's prior framing (state-0 weeder path uses LongScan, state-1
re-search uses ShortScan).

### Dead-code finding (important, verified)

`grep -iE "Weeder\s*=\s*(yes|no)"` over the shipped `ini/rulesmd.ini` (both `[SMIN]`/`[YAREFN]`
and the whole file) returns **zero matches** — no shipped TechnoType sets `Weeder=` at all, so
the type-level default (`no`, per the file's own documentation block at ini line 3615/3664)
applies to every unit and building in retail rules. Because both 0x004ddb90 call sites are
gated on `type+0xE0F` (Weeder) or reachable only when Weeder was the reason state-1 was ever
entered without Harvester, **0x004ddb90 is unreachable from any stock TechnoType**, including
the slave miner (SMIN/YAREFN — see below). This directly contradicts the framing in the swarm
brief that the stock SMIN/YAREFN has `Weeder=yes`; it does not — verified via
`Grep "^Weeder=" ini/rulesmd.ini"` (no hits) and by reading the `[SMIN]`/`[YAREFN]` sections in
full (`ini/rulesmd.ini` lines 9042-9114 and 13234-13303 — neither sets `Harvester=` or `Weeder=`
at all).

**Active in YR: No** for 0x004ddb90 under stock rulesmd.ini — it is TS-legacy code (the "weeds"
mechanic does not exist in shipped RA2/YR balance) that survives in the shared engine but is
never exercised by any retail unit.

## The slave-miner host gate at the top of Mission_Harvest

Verified via `disassemble_function 0x0073e5e0` (top of function):

```
iVar8 = param_1[0x1b1]                      ; this->Class (TechnoTypeClass*), UnitClass+0x6C4
if (type[0x5ed]==0 || type[0x5ec]==0 || this[0x2D8]==0)
    -> fall through to Harvester/Weeder mission (existing cadence doc)
else
    ECX = this[0x2D8]; CALL 0x006b0db0; return Rate-delay
```

- `type+0x5EC` and `type+0x5ED` are single-byte flags tested with `TEST reg,reg` (bool
  semantics), confirmed at instruction level: `007143d0/007143e4` write/read `[EBP+0x5ec]` and
  `007143ea/007143fe` write/read `[EBP+0x5ed]` inside `TechnoTypeClass__ReadINI`.
- Both are the classic WW `field = ReadBool(section, "Key", field)` idiom. Backtracking the two
  key-name strings pushed immediately before each `CALL 0x005295f0`:
  - `[EBP+0x5ec]` ← key string at 0x00843cb8 = **`"ResourceGatherer"`** (verified via
    `read_memory 0x00843cb8`).
  - `[EBP+0x5ed]` ← key string at 0x00843ca4 = **`"ResourceDestination"`** (verified via
    `read_memory 0x00843ca4`).
  - This **overturns** the swarm brief's working hypothesis ("plausibly the Enslaves/slave-count
    pair") — type+0x5EC/0x5ED are `ResourceGatherer`/`ResourceDestination`, not `Enslaves`/
    `SlavesNumber`.
- Stock `ini/rulesmd.ini` sets `ResourceGatherer=yes` and `ResourceDestination=yes` on **both**
  `[SMIN]` (line 9105-9106) and `[YAREFN]` (line 13292-13293) — verified via direct file read.
  So the first two gate conditions are true for the real slave miner in both its vehicle and
  deployed-building form. **Active in YR: Yes** for this half of the gate.

### this+0x2D8 (`param_1[0xb6]`)

`this+0x2D8` is a **`SlaveManagerClass*` instance pointer**, a TechnoClass-level field (present
on both UnitClass and BuildingClass instances — not FootClass/UnitClass-specific). Verified two
ways:
1. `TechnoClass__Constructor` (0x006f2e6f) zero-initializes `[ESI+0x2d8]`; `BuildingClass__Sell`
   (0x0044aa99, right before its own call to 0x006b0db0) reads `this+0x2d8` the same way a
   Building instance would — confirms the field is shared across the TechnoClass hierarchy, not
   a UnitClass-only slot (`search_instructions operand_pattern="0x2d8],"` + `get_assembly_context`
   on the BuildingClass::Sell call site).
2. `TechnoClass__Init_Managers` (0x006f3f40) is the allocation site: it fetches the unit's type,
   and only when `type+0xD40 != 0` does it `operator_new(100)` and call
   `SlaveManagerClass__Constructor(this, type[0xD40], type[0xD44], type[0xD48], type[0xD4C])`,
   storing the result at `param_1[0xb6]` (= `this+0x2D8`). `type+0xD40/0xD44/0xD48/0xD4C` were
   independently confirmed (via `search_instructions` + `get_assembly_context` inside
   `TechnoTypeClass__ReadINI`) to be populated from the literal key strings `"Enslaves"`
   (0xD40), `"SlavesNumber"` (0xD44), `"SlaveReloadRate"` (0xD48), `"SlaveRegenRate"` (0xD4C) —
   i.e. exactly the four keys the swarm brief originally suspected, just gating the *existence*
   of the +0x2D8 SlaveManager object rather than the two boolean fields Mission_Harvest itself
   tests.
   
**Active in YR: Conditional** — `this+0x2D8` is non-null for the lifetime of any TechnoClass
instance whose type has a non-empty `Enslaves=` (SMIN/YAREFN both set `Enslaves=SLAV`), so for
the stock slave miner the gate's third condition is true from construction onward and the
SlaveManager branch is taken every Mission_Harvest tick.

## 0x006b0db0 — identity and summary

Ghidra's own label (`SlaveManagerClass__HandleReturnedSlaves`) matches the decompiled body:
callers are exactly `UnitClass__Mission_Harvest` (ECX = `this+0x2D8`) and `BuildingClass__Sell`
(ECX = same `this+0x2D8` offset) — verified via `get_function_callers 0x006b0db0` +
`get_assembly_context` on both call sites. Body summary (`decompile_function 0x006b0db0`):
reads the owner TechnoClass back-pointer at `SlaveManager+0x24`; if the owner's archive/attached
target (`+0x2c` virtual, `+0x5a4`) resolves, it computes a deploy cell via
`SlaveManagerClass__FindDeployCell`, sets the owner's mission to Guard(5) or Move(2) depending on
whether a valid cell was found, and iterates a `SlaveControl` array (`+0x3c`/`+0x48`) calling a
"recall" method (`+0x3d0`) on any slave not already in state 6. This is at-summary-level
consistent with the working label ("SlaveManager returned-slaves handling").

**Active in YR: Yes** (reachable and exercised by the stock SMIN/YAREFN every Mission_Harvest
tick, and by BuildingClass::Sell when a slave-miner-owning building is sold).

## Implementation Handoff

- **Verified behavior**: gamemd gates the slave-miner "returned slaves" tick on
  `type.ResourceGatherer && type.ResourceDestination && (this has a live SlaveManager, i.e.
  type.Enslaves is set)`, checked freshly every Mission_Harvest call — not a static
  classification made once at load time.
  **Rust delta**: `src/sim/miner/mod.rs::miner_kind_for_object` currently classifies
  `MinerKind::Slave` purely from `object.enslaves.is_some()` (mod.rs:511-515) and
  `src/sim/miner/harvest_mission.rs` early-returns on `MinerKind::Slave` before entering the
  Harvest state machine — for stock rules this is behaviorally equivalent (SMIN/YAREFN always
  set all three), but it is not the same gate gamemd evaluates.
  **Affected surface**: any modded TechnoType that sets `Enslaves=` without also setting both
  `ResourceGatherer=yes` and `ResourceDestination=yes` (or vice versa) would diverge from gamemd
  under the current Rust classification.
  **Acceptance scenario**: a modded unit with `Enslaves=X` but `ResourceGatherer=no` should, per
  gamemd, fall through to the ordinary Harvester/Weeder state machine (and likely hit the
  `Harvester==false && Weeder==false` → 450-delay idle path if neither flag is set) rather than
  the SlaveManager path.
  **Proposed test name**: `miner_kind_requires_resource_gatherer_and_destination_with_enslaves`.
  **Risk**: low for stock parity (no shipped unit exercises the divergence), moderate for
  mod-authoring fidelity.
- **Verified behavior**: 0x004ddb90 (weeder Tiberium-vs-weeds search) is unreachable under stock
  `rulesmd.ini` because no TechnoType sets `Weeder=yes`.
  **Rust delta**: none required — this is confirmation that the Rust engine correctly has no
  "weed" mechanic; no action needed unless a future mod ships `Weeder=yes`.
  **Affected surface**: none under current rules.
  **Acceptance scenario**: N/A (dead code path, informational only).
  **Proposed test name**: none (no behavior to test).
  **Risk**: none.
- **Verified behavior**: `this+0x2D8` (SlaveManager pointer) is a TechnoClass-level field set up
  once in `Init_Managers` from `type.Enslaves/SlavesNumber/SlaveReloadRate/SlaveRegenRate`, shared
  by both the SMIN unit and the YAREFN building (both call the same 0x006b0db0 handler from
  different owning methods).
  **Rust delta**: confirm `src/sim/slave_miner.rs` models an equivalent "returned slaves" handoff
  for both the vehicle and deployed-building forms (not just the unit side), matching
  `BuildingClass::Sell`'s use of the same handler when a slave-miner-owning building sells.
  **Affected surface**: building-sell path for YAREFN with live slaves.
  **Acceptance scenario**: selling a YAREFN with active slaves should trigger the same
  recall/reassign logic the unit-side Mission_Harvest tick would.
  **Proposed test name**: `sell_yarefn_recalls_active_slaves`.
  **Risk**: low-medium — untested surface, not confirmed present or absent in Rust by this slot
  (out of narrow scope; flagged as open question, not asserted as missing).

## Negative Facts / Do Not Do

- Do not attribute type+0x5EC/0x5ED to `Enslaves`/`SlavesNumber` — verified via
  `read_memory 0x00843cb8`/`0x00843ca4` that the literal key strings feeding those two offsets
  are `"ResourceGatherer"` and `"ResourceDestination"`.
- Do not assume the stock SMIN/YAREFN sets `Weeder=yes` — verified absent via `Grep` over
  `ini/rulesmd.ini` (`^Weeder=` and case-insensitive `Weeder\s*=\s*(yes|no|true|false)`, both zero
  matches) and by reading both full sections directly.
- Do not treat 0x004ddb90 as reachable in retail play — both of its two call sites in
  `Mission_Harvest` are gated on the Weeder flag (directly, or indirectly via "reached state-1
  without Harvester"), and no shipped TechnoType sets that flag.
- Do not conflate `RulesClass+0x1778` and `+0x177c` — `+0x1778` is `TiberiumShortScan` and
  `+0x177c` is `TiberiumLongScan`, confirmed via `get_xrefs_to`/`get_assembly_context` on the
  `RulesClass__ReadGeneral` reads, not by name-similarity guessing.
- Do not assume `this+0x2D8` is UnitClass/FootClass-specific — `BuildingClass__Sell` reads the
  identical offset before calling the same handler, confirming it's a TechnoClass-level field.

## Remaining Uncertainty

- The exact argument order into `SlaveManagerClass__Constructor` for `type+0xD44/0xD48/0xD4C`
  (SlavesNumber/SlaveReloadRate/SlaveRegenRate) was cross-checked by key-string proximity and
  read/write ordering inside `TechnoTypeClass__ReadINI`, but the four-argument
  `SlaveManagerClass__Constructor` body itself was not decompiled in this slot (out of narrow
  scope) — the mapping is high-confidence but not body-verified.
- Whether `src/sim/slave_miner.rs` and `BuildingClass::Sell`'s equivalent path already implement
  a Rust-side "recall slaves on sell" flow was not checked beyond a grep for key names; this slot
  did not audit that file's logic in depth (would expand scope beyond the two named functions and
  gate).
- `FootClass__Is_Cell_Weedable`'s own internals (what overlay/tile predicate it tests) were not
  decompiled — only its role as the terminal predicate distinguishing 0x004ddb90 from the
  harvester scan was verified.

## Stale-doc replacement wording

None found in tracked/published docs — the "Enslaves/slave-count pair" and "(Weeder=yes)"
framings were part of this swarm's own task brief (not a committed doc), so no existing doc file
needs correction. If a future doc asserts either of those two claims about the SMIN/YAREFN slave
miner, it should be corrected using the verified facts above (type+0x5EC/0x5ED =
ResourceGatherer/ResourceDestination; stock rulesmd.ini has no `Weeder=` on any TechnoType).
