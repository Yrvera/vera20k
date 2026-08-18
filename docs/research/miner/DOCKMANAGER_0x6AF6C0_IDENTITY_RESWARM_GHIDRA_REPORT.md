# 0x006AF6C0 Identity Certification (re-swarm, 2026-07-12)

## Verdict

**0x006AF6C0 = `SlaveManagerClass::AI_Update`.** It is the per-frame state
machine for the slave-infantry entries owned by a Yuri Slave Miner
(`SlaveManagerClass` instance hanging off the deployed `SMIN` unit). It is
**not** a refinery/harvester dock-queue processor and has no relation to
`BuildingClass+0x2E4` dock-slot reservation.

This is a **re-certification**, not a new finding — it independently
reproduces (via RTTI, not labels) the conclusion the target doc
(`DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`) already
self-corrected to in its 2026-05-11/2026-05-24 audit notes, and which two
other established docs (`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` §4,
`miner/SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` §"Slave AI State Machine")
independently converge on.

---

## Load-bearing verified facts

1. **Function identity (label).** `get_function_by_address("006AF6C0")` →
   `SlaveManagerClass__AI_Update`, body `006af6c0`–`006afd3e`. Plate comment
   (`get_plate_comment`) is the same string — noted per task constraint as a
   hint only, not relied on alone (see fact 2 for independent proof).

2. **Receiver class verified via RTTI COL walk (independent of any label).**
   Constructor at `006af1a0` (`disassemble_function`) contains
   `MOV dword ptr [ESI],0x7f31c8` at `006af1f2` — object's primary vtable =
   `0x7F31C8`. Walked the MSVC Complete Object Locator chain by raw bytes:
   - `read_memory(0x7F31C4, 4)` (vtable base − 4, the COL pointer slot) →
     bytes `00 97 80 00` → COL at `0x00809700`.
   - `read_memory(0x00809700, 20)` → signature=0, offset=0, cdOffset=0,
     `pTypeDescriptor` = bytes `10 FD 83 00` → `0x0083FD10`.
   - `read_memory(0x0083FD10, 40)` → mangled name bytes
     `2e 3f 41 56 53 6c 61 76 65 4d 61 6e 61 67 65 72 43 6c 61 73 73 40 40 00`
     = ASCII `.?AVSlaveManagerClass@@` — the canonical MSVC mangling of
     top-level class **`SlaveManagerClass`**. This is a byte-level RTTI
     proof, not a decompiler/label artifact.

3. **Caller set.** `get_function_callers("006AF6C0")` → exactly one:
   `SlaveManagerClass__AI @ 006af5f0` (confirmed also via
   `get_xrefs_to("006AF6C0")` → call site `006af631`, inside `006af5f0`'s
   body `006af5f0`–`006af642`). `006af5f0` is a 10-frame tick throttle
   (per prior doc's decompilation, not re-verified line-by-line here — out
   of scope). No other direct callers exist.

4. **Full dispatch chain to the receiver, verified end to end.**
   `decompile_function(0x007360C0)` (`UnitClass__AI`) contains:
   `if ((int *)param_1[0x9e] != (int *)0x0) { (**(code **)(*(int *)param_1[0x9e] + 0x5c))(); }`
   — i.e. if the unit's `SlaveManager` pointer (`unit+0x278`) is non-null,
   call vtable slot `+0x5C` on it. `read_memory(0x7F3224, 4)` (=
   `0x7F31C8`+`0x5C`, the RTTI-verified `SlaveManagerClass` vtable) → bytes
   `F0 F5 6A 00` = `0x006AF5F0`. So: `UnitClass::AI` → (vtable+0x5C on the
   RTTI-confirmed `SlaveManagerClass` instance) → `SlaveManagerClass::AI`
   (throttle) → `SlaveManagerClass::AI_Update` (0x6AF6C0). Every hop is
   receiver-class-verified, not label-inferred.

5. **Active in YR: Yes (stock, no TS/dead-flag gating).** `[SMIN]` in
   `ini/rulesmd.ini` (line 9042): `Name=Slave Miner`, `Owner=YuriCountry`,
   full voice/sound set (`SlaveMinerSelect`, `SlaveMinerHarvest`, etc.), no
   disabling flags. Companion slave-worker unit `[SLAV]` exists (line 5015).
   Tuning keys `SlaveMinerShortScan=8` / `SlaveMinerSlaveScan=14` /
   `SlaveMinerLongScan=48` (lines 313–315) are live stock values, not
   Ares/Phobos additions. The Yuri Slave Miner is a standard, buildable
   economy unit in a normal YR skirmish (Yuri faction) — this code path
   fires whenever a player deploys and uses one.

---

## Implementation handoff

No new Rust-facing implication from this task — it is a scope/identity
audit, not new behavioral decode. The behavior itself (per-slave state
machine states 0–6) is already documented in the target doc's body (post
self-correction) and, more authoritatively, in
`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` and
`miner/SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`. Any future slave-miner
implementation work should anchor on those two docs, not on the
dock-manager-named file.

---

## Negative facts / do not do

- Do **not** cite `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`
  as evidence for `CMIN/HARV → GAREFN/NAREFN` refinery dock admission,
  queue promotion, the `0x15` radio handoff, or release timing — the doc
  itself already disclaims this (its own 2026-05-24 status note), and this
  re-certification confirms the disclaimer is correct.
- Do **not** treat `BuildingClass+0x2E4` (harvester single-slot dock
  reservation) as related to `SlaveManagerClass` — they are unrelated
  structures on unrelated classes (`BuildingClass` vs a
  `SlaveManagerClass` instance referenced from `UnitClass+0x278`).
- Do **not** re-derive identity from the plate comment or Ghidra display
  label alone in future audits of this address — this session independently
  reproduced the same identity via raw-byte RTTI walk specifically because
  labels in this project have a known mislabel history (per task brief).
- Do **not** assume `0x6AF5F0` (the tick-throttle caller) has other callers
  besides `UnitClass::AI` — `get_function_callers` found none beyond the
  indirect vtable dispatch verified in fact 4 (no other xrefs surfaced).
- Chrono miner / Soviet ore miner dock logic is untouched by this function;
  it is Yuri-Slave-Miner-only (per target doc §"Chrono miner branch", not
  re-verified here — out of scope, flagged as inherited claim).

---

## Remaining uncertainty

- The exact semantics of the 20-byte per-slave entry fields and the 7-state
  machine body were not re-verified line-by-line in this session (out of
  scope per task brief — "do NOT fully decompile the whole state machine's
  internals"). That content already exists, cross-verified across two other
  docs; if it needs re-auditing, use `/verify-doc` on
  `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` or
  `miner/SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` directly, not this report.

---

## Is `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` misfounded?

**Its founding premise was misfounded** (it was written believing
0x6AF6C0 = refinery dock-queue processor), **but the file has already been
corrected in place** — its own top section ("CRITICAL FINDING: Address
Mismatch") and 2026-05-24 status note already state the reidentification to
`SlaveManagerClass::AI_Update` and explicitly disclaim use as refinery-dock
evidence. This re-swarm task's fresh RTTI/caller verification confirms that
correction is accurate.

**Residual problem: the filename, not the content.** The file is titled
`DOCKMANAGER_STATE_MACHINE_...` and lives under `miner/`, which will keep
misleading future doc search (`research_search`) toward "dock manager" hits
for a slave-miner-only function. Recommend **re-scoping (rename, not
rewrite)** to something like
`miner/SLAVE_MANAGER_AI_UPDATE_FUN_006AF6C0_GHIDRA_REPORT.md`, or — given
its content is now largely redundant with the two established docs above —
consider merging/retiring it in favor of
`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` (repo root) and
`miner/SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`, which already cover the
same function with equal or greater depth. This report does not perform
that rename (out of scope / read-only task) — flagging for the user/swarm
orchestrator to action.

---

## Status

**COMPLETE.** Identity, receiver class (RTTI-verified), and caller set are
certified. No code or doc mutation performed by this task.
