# ReadyToCommence S5/L6 Blocker — Closure + Fear-Prone Sequence-Gate Correction — Ghidra Report

**Address(es):** loco `0x004afc20` (Drive Is_Moving_Now); `0x005200B0` (Infantry Fear_Decay_Handler); `0x00521B60` (Infantry ReadyToCommence); `0x00520AE0` (Infantry DoType_Sequencer); `0x00517A50` (Infantry ctor); `0x00517CC0` (InitFromType)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the design doc §0.3/§10.3/§7.7 "still-open" ReadyToCommence items (busy-flag setters, locomotor `slot+0x80` idle predicate, excluded-mission sets) — reconciled against two pre-existing reports, with the one load-bearing residual (loco `slot+0x80`) closed; PLUS the InfantryClass fear-prone Down/Up "27-30" gate, which this investigation found is **misread** in the design doc, the Plan-3 L4 plan, and the earlier Task-A fix.
**Non-Scope:** re-deriving the four leaf ReadyToCommence predicates (done in `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`); the Unit/Infantry busy-flag write-sites (done in `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md`); exact names of infantry sequences 27-30; aircraft `+0x6D2/+0x6D4` runtime flip sites.
**Confidence:** HIGH (all claims `read_memory`/`decompile_function`-verified this session).
**Active in YR:** Yes.

## 1. Overview

The design doc (`TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md`) §0.3/§10.3/§7.7 flagged the leaf `ReadyToCommence` busy-flag semantics and the locomotor `slot+0x80` "idle" predicate as **INFERRED, not traced — DRIFT until traced** (the gate for design Slice S5 / Plan-3 Slice L6). This investigation found those items are **already resolved by two HIGH-confidence reports written 2026-06-02** that the design doc §0.3 did not incorporate (stale). This report (a) reconciles §0.3 against those two reports, (b) closes the one genuinely body-unverified residual — the locomotor `slot+0x80` — and (c) corrects a **separate, load-bearing misread**: the InfantryClass fear-prone Down/Up "27-30" gate is on the infantry's **current animation sequence (`Doing`, +0x6C4)**, not CurrentMission and not the type index.

## 2. §0.3 Reconciliation — what the two existing reports already settle

The authoritative sources (both verified, both 2026-06-02):
- `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` — base + 4 leaf predicate bodies, vtable+0x200 slots COL-walked, excluded-mission sets, Building/Aircraft flag roles.
- `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md` — write-site lifecycles for UnitClass +0x6D1/+0x6E1/+0x6E2 and InfantryClass +0x2B4.

| §0.3 / §10.3 open item | Status | Resolved by |
|---|---|---|
| Base `ReadyToCommence` = `return 1` @ `0x004E0140` | RESOLVED | SUBCLASS report (vtable+0x200 slot reads; FootClass `0x007E8E94` = base stub, no override) |
| 4 leaf override addresses + predicate structure | RESOLVED | SUBCLASS report (Unit `0x00744270`, Infantry `0x00521B60`, Aircraft `0x0041B5E0`, Building `0x00454250`) |
| Building `+0x6DD` field role + SETTER | RESOLVED | SUBCLASS report: "construction-complete" flag; setter `OnConstructionComplete 0x00445F80`; **≠ ActuallyPlacedOnMap** |
| Aircraft `+0x6D2` / `+0x6D4` field roles + inits | RESOLVED (roles); runtime flip sites DEFERRED | SUBCLASS report: `+0x6D2` abort-in-progress (init 0), `+0x6D4` landed-ready (**init 1** @ ctor `0x00413D7B`) |
| Unit `+0x6D1` setter (dock-active) | RESOLVED | LIFECYCLES report: SET `Mission_Deploy_Building 0x0073D630` state-3 (~`0x0073E011`), CLEAR state-4 + path-abort |
| Unit `+0x6E1` / `+0x6E2` setters (deploy anims) | RESOLVED; only live for TypeData+0xe13 deploy-fly types (Siege Chopper), NOT standard vehicles | LIFECYCLES report: SET/CLEAR in `FUN_00739AC0` / `FUN_00739CD0` |
| Infantry `+0x2B4` (read in predicate) | RESOLVED + **CORRECTED** | LIFECYCLES report: it is the **attack-target POINTER** (not a counter); non-zero in Attack mission BLOCKS commence; SET/CLEAR via `TechnoClass__Set_ArchiveTarget 0x006FCDB0` |
| Locomotor `slot+0x80` "idle" predicate body | **RESOLVED THIS REPORT** (was the one body-unverified item) | §3 below |
| Excluded-mission set per leaf | RESOLVED | SUBCLASS report; see §4 correction (mission **6 = Sticky**, **0x15=21 = Rescue** — design "Sleep(6)" wording is wrong) |
| Infantry `+0x68D` setter | PARTIAL (cleared in `DoType_Sequencer`; set-site DEFERRED) | §3 |
| Infantry `+0x8D` ObjectClass byte semantic | DEFERRED (low priority — blocks only on invalid/in-transit state) | both reports + §7 |

**Net:** S5's per-`EntityCategory` `ready_to_commence()` hook and L6's commence gate are **implementable now** from the two existing reports' handoffs. The design doc's "DRIFT until traced" on these is stale and should be downgraded (see §10 stale-doc list).

## 3. Locomotor `slot+0x80` = `DriveLocomotionClass::Is_Moving_Now` (0x004afc20) — BODY VERIFIED

The Unit (`0x00744270`) and Infantry (`0x00521B60`) ReadyToCommence predicates call `(*(loco+0x674).vtable[0x80])()` and branch on the result. Both existing reports labelled it "IsMoving" by **usage inference only**. Verified this session to the body:

- Drive ILocomotion vtable base `0x007E7EB0` (confirmed: `read_memory 0x007E7EF0` = `0x004b0500` = `DriveLocomotionClass::Process`, the known `+0x40` slot).
- `slot+0x80` = `read_memory 0x007E7F30` = **`0x004afc20`**.
- `decompile_function 0x004afc20` (`DriveLocomotionClass__Is_Moving_Now`): returns **true** if `CDTimerClass::Remaining() != 0` (a movement timer still running) **OR** `(loco.vtable[0x10]() != 0 AND destination coord != g_NullCoord_Drive_{X,Y,Z} AND owner.vtable[0x538]() > 0)`. Otherwise **false**.

So `slot+0x80` is the ILocomotion `Is_Moving_Now` query (the usage-inferred role is correct, now body-confirmed). **Tiny detail:** it is not a single bool read — the dominant gate is the CDTimer remaining; the secondary path also requires a non-null destination sentinel AND a positive owner query. The "idle" state the predicate keys on is the negation. Infantry's locomotor is a different concrete class but implements the same ILocomotion `slot+0x80` contract (its body not separately decompiled — interface-slot role is stable; marked DEFERRED-low).

## 4. CORRECTION — InfantryClass fear-prone Down/Up "27-30" gate is on the current SEQUENCE, not the mission

**The design doc §6.2 / §2.6, the Plan-3 L4 plan (Task 2), AND the Task-A fix-#2 edit all state the gate is `CurrentMission ∈ {0x1B..0x1E}` (27-30). This is WRONG.** Verified via `decompile_function 0x005200B0`, `0x00520AE0`, `0x00521B60`, `0x00517A50`, `0x00517CC0`:

- `InfantryClass__Fear_Decay_Handler 0x005200B0` gates prone Down (`vtable[0x558](5,0,0)`) and Up (`vtable[0x558](7,0,0)`) on `param_1[0x1b1] ∉ {0x1b,0x1c,0x1d,0x1e}`. `param_1` is `int*`, so `param_1[0x1b1]` = byte offset **+0x6C4**. The handler **never reads CurrentMission (+0xAC / `param_1[0x2b]`)** for this gate.
- `+0x6C4` is **the infantry's current animation sequence ("Doing")**, NOT the type index and NOT the mission:
  - Constructor `0x00517A50`: `param_1[0x1b1] = 0xffffffff` (init **-1**); `param_1[0x1b0]` (+0x6C0) = the InfantryType pointer (param_2).
  - `InfantryClass__InitFromType 0x00517CC0` does **not** write +0x6C4 (it sets health +0x6c/+0x70, panic seed +0x2fc from `TypeData+0x680`/`+0x684`, +0x3d2). So +0x6C4 is **not** the type-array index.
  - `InfantryClass__DoType_Sequencer 0x00520AE0` `switch`es on `param_1[0x1b1]` (+0x6C4) as a **sequence index** into `*(TypeData+0xe3c) + (+0x6C4)*0x24` (the per-type SequenceData array, stride 0x24 = 36 bytes/entry) and transitions sequences via `vtable[0x558](newSeq,1,0)`. Cases include 0xb-0xf, 0x14/0x15/0x24, 0x1b→0x1c, 0x1f, 0x21, 0x22→0x23, 0x26, 0x28/0x29 (prone-fire). **This is the definitive proof +0x6C4 = current `Doing` sequence.**
- The InfantryClass `ReadyToCommence 0x00521B60` indexes the **same** +0x6C4 into `(&DAT_007eaf7c)[+0x6C4 * 4]` with a `== -1` sentinel — i.e., "is the current sequence one that permits commencing a new mission." The SUBCLASS report's "InfantryTypeClass index / g_InfantryTypeHasIdleSeq" label for this is **a mislabel** — the index is the current sequence, the table is per-sequence.

**Corrected behavior:** infantry suppress fear prone Down/Up transitions **while the current sequence (`Doing`, +0x6C4) ∈ {27,28,29,30}** — specific deploy/special animation sequences (0x1b transitions to 0x1c in the sequencer; exact sequence names DEFERRED). The earlier "mission 27-30 = ParadropOverfly/Deliberate/AttackMove/SpyplaneApproach" framing is void: those are mission codes, irrelevant here.

**Bonus confirmations for L4 (all verified this session):**
- Fearless gate = `TypeData+0xebc` (`if (*(char*)(TypeData+0xebc)==0) fear--`). Maps to Rust `can_decay_fear`.
- Thresholds 0x31 (49) / 0x32 (50): Down requires `0x31 < fear` (≥50); Up requires `fear < 0x32` (<50); Fraidycat scatter branch `LAB_005201dc` requires `0x32 < fear` (≥51) AND `TypeData+0xebf != 0` AND `+0x8d == 0`. **No "199" anywhere in the handler** (the design's "49/50/199" 199 is unsourced — confirmed absent).
- Panic countdown: `if (fear==0 && param_1[0xbf]==0) { if (vtable[0x2ac]()) param_1[0xbf] = *(TypeData+0x684); }`. `param_1[0xbf]` = byte offset **+0x2FC**; seeded from `TypeData+0x684` (or `+0x680` if not -1, per InitFromType). Matches L4 Task 4.
- The interrupt set is also enforced in `DoType_Sequencer` itself (`default` case: `if (seq ∉ {0x1b..0x1e})` → normal down/stand) — so the prone-suppression-during-27-30 logic appears in BOTH the fear handler and the sequencer, keyed on the current sequence.

## 5. Integration Points

`ReadyToCommence` (vtable+0x200) is called by `Queue_Mission 0x005B35E0` (only when `commence_now=true`) before `Commence 0x005B3570`. The fear handler + DoType_Sequencer run inside `InfantryClass::AI 0x0051BAB0` after `FootClass::AI`. The locomotor `Is_Moving_Now` is queried through the FootClass `+0x674` ILocomotion interface.

## 6. Current Rust Implementation Status

- `ready_to_commence()` per-`EntityCategory`: not yet landed (design Slice S5 / Plan-3 L6). The two existing reports' handoffs are the spec.
- Infantry fear: `tick_fear_decay_and_prone` (`src/sim/infantry.rs:101`), `tick_fear_for_entities` (`:130`), `InfantryRuntime{fear_level,is_prone}` (`src/sim/game_entity.rs:48`) — LIVE + hashed. The mission-exclusion gap (Plan-3 L4 Task 2) must be rewritten: it is a **current-sequence (`Doing`) gate**, and the Rust engine does not currently model an infantry `Doing` sequence enum at parity with gamemd → see §7 / §10.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| §0.3 leaf predicates + base | verified | SUBCLASS report (read in full) | none |
| §0.3 busy-flag setters (Bldg/Unit/Aircraft/+0x2B4) | verified | LIFECYCLES + SUBCLASS reports | none |
| Loco `slot+0x80` = Is_Moving_Now body | verified | `read_memory 0x007E7F30`, `decompile_function 0x004afc20` | infantry-loco impl of same slot not separately decompiled (interface contract stable) |
| Fear-prone 27-30 gate = current sequence (+0x6C4) | verified | `decompile_function 0x005200B0/0x00520AE0/0x00521B60/0x00517A50/0x00517CC0` | exact sequence NAMES for 27-30 |
| Panic countdown +0x2FC ← TypeData+0x680/+0x684 | verified | `decompile_function 0x005200B0/0x00517CC0` | vtable+0x2ac predicate identity |
| Fearless gate TypeData+0xebc; thresholds 49/50/51 | verified | `decompile_function 0x005200B0` | none ("199" confirmed absent) |
| Infantry +0x68D | touched-not-exhausted | cleared `=0` in `DoType_Sequencer 0x00520AE0` prone-fire path; read in predicate `0x00521B7B` | SET-site (where the action flag is raised) |
| Infantry +0x8D ObjectClass byte | not-touched | read in predicate + fear handler as block-if-nonzero | ObjectClass-level semantic |
| Aircraft +0x6D2/+0x6D4 runtime flip sites | deferred | roles+inits verified (SUBCLASS) | depart/land setter addresses |
| UnitClass FUN_004a51d0 spyplane-pad branch | deferred | SUBCLASS report YELLOW | not load-bearing for standard YR |

## 8. Open Questions — Final State

- `[RESOLVED]` Loco `slot+0x80` body → `DriveLocomotionClass::Is_Moving_Now` (timer-remaining OR loco-sub-predicate+non-null-dest+owner-query) (evidence: `decompile_function 0x004afc20`).
- `[RESOLVED]` What is InfantryClass +0x6C4 → current animation sequence (`Doing`), init -1, indexed into `TypeData+0xe3c` SequenceData (stride 0x24) (evidence: `decompile_function 0x00520AE0/0x00517A50/0x00517CC0`).
- `[RESOLVED]` Fear-prone 27-30 gate is on +0x6C4 (sequence), not CurrentMission (evidence: `decompile_function 0x005200B0` — never reads +0xAC).
- `[RESOLVED]` "199" fear threshold → does not exist in the handler (evidence: `decompile_function 0x005200B0`).
- `[RESOLVED]` Panic countdown field/seed → +0x2FC ← TypeData+0x684/+0x680 (evidence: `decompile_function 0x005200B0/0x00517CC0`).
- `[DEFERRED]` Exact sequence names for Doing ∈ {27,28,29,30} (category: bounded-cost-too-high; reason: needs the infantry DoType/`Sequence=` enum or the SequenceData array dump; next-step: dump `TypeData+0xe3c` entries 27-30 or locate the DoType enum).
- `[DEFERRED]` Infantry +0x68D set-site (category: bounded-cost-too-high; reason: cleared in sequencer, set-site elsewhere; next-step: xref the field write across infantry action handlers). Low priority — blocks commence only while an action is mid-flight.
- `[DEFERRED]` Infantry +0x8D ObjectClass semantic (category: requires-different-system-context; reason: ObjectClass-layer byte; next-step: trace ObjectClass +0x8d writers). Low priority — non-zero = invalid/in-transit state.
- `[DEFERRED]` Aircraft +0x6D2/+0x6D4 runtime depart/land flip sites (category: bounded-cost-too-high; next-step: trace airfield takeoff/landing handlers). Roles+inits already verified.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| §0.3 ReadyToCommence predicates + setters fully traced | two 2026-06-02 reports | unchecked (S5 not landed) | new `MissionCom.ready_to_commence()` per category | Implement per the two reports' handoffs | per-leaf tests in those reports | do NOT re-run the Ghidra dive — it is done |
| Loco `slot+0x80` = Is_Moving_Now | `decompile 0x004afc20` | n/a (read-only marker; S1 shadow already consumes `process_drive_locomotion_shell`) | `ready_to_commence()` Unit/Infantry "is moving" branch | Use the locomotor "is moving now" query, not a raw flag | unit mid-move does not promote a queued Move until idle (per SUBCLASS Unit handoff) | do NOT model as a single bool — it includes a movement-timer-remaining gate |
| Fear-prone Down/Up suppressed while current sequence ∈ {27,28,29,30} | `decompile 0x005200B0/0x00520AE0` | DRIFT — Rust has no infantry `Doing` sequence enum; Plan-3 L4 Task 2 spec is WRONG (says CurrentMission) | `tick_fear_decay_and_prone` (`infantry.rs:101`) | Gate on the infantry's **current sequence (`Doing`)**, NOT CurrentMission | infantry mid-deploy-sequence (Doing∈27-30) does not change prone state; on a normal sequence it does | do NOT gate on CurrentMission or on mission 27-30; do NOT name them ParadropOverfly/etc. |
| Panic countdown +0x2FC ← TypeData+0x684 on fear→0 | `decompile 0x005200B0/0x00517CC0` | DRIFT (L4 Task 4 unmodeled) | `InfantryRuntime` | frame-anchored seed on fear-hits-zero gated by vtable+0x2ac | defer unless scatter system re-enabled (L4 rec.) | do NOT pin a test until vtable+0x2ac identity traced |

## 10. Stale Docs / Follow-up Corrections (exact replacement wording)

1. **`TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §0.3 / §10.3 / §7.7 / §6.2 M5** — the "leaf ReadyToCommence busy-flag semantics INFERRED from constructor init, not traced setters — DRIFT until traced" claim is **STALE**. Replace with: *"RESOLVED — see `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` (predicates + Building/Aircraft flags) and `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md` (Unit/Infantry setter lifecycles). Loco `slot+0x80` = `DriveLocomotionClass::Is_Moving_Now 0x004afc20` (body-verified). Excluded-set: mission **6 = Sticky**, **21 (0x15) = Rescue** (NOT 'Sleep(6)')."*
2. **`TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §6.2 / §2.6 infantry-fear** and **`docs/plans/2026-06-02-ai-shell-leaves-commence-plan.md` L4 Task 2 + tests** — the "fear prone Down/Up suppressed while mission ∈ {27-30}" is **WRONG**. Replace with: *"suppressed while the infantry's current animation sequence (`Doing`, InfantryClass +0x6C4) ∈ {27,28,29,30} (deploy/special sequences; exact names TBD). It is a current-sequence gate, NOT a CurrentMission gate, and NOT the infantry type index."* Rename the L4 fear tests away from `_during_interrupt_missions` (the Task-A rename was built on the same misread).
3. **`READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` InfantryClass section** — the "+0x6C4 = InfantryTypeClass index / g_InfantryTypeHasIdleSeq[typeIndex]" label is a **mislabel**. +0x6C4 = current sequence (`Doing`); the table at `0x7eaf7c` is per-sequence ("does this sequence permit commence"), indexed by the current sequence, not by type.

## Sources

- Ghidra: `read_memory 0x007E7EF0/0x007E7F30`; `decompile_function 0x004afc20/0x005200B0/0x00521B60/0x00520AE0/0x00517A50/0x00517CC0`; `get_function_by_address 0x00517acc`; `search_functions InfantryClass__InitFromType`.
- Docs: `READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`, `READYTOCOMMENCE_UNIT_INFANTRY_FLAG_LIFECYCLES_GHIDRA_REPORT.md`, `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §0.3/§6.2/§10.3.
