# Bridge Repair Multi-Engineer Same-Tick -- Ghidra Research Report

**Address(es):** `InfantryClass::PerCellProcess @ 0x00519630`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass::Conceal @ 0x005F4D30`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** duplicate/same-window engineer entries into the same `BridgeRepairHut=yes` CABHUT: ordering, engineer consumption, duplicate SFX/EVA branch behavior, and whether repeated repair dispatch can apply more bridge mutation.  
**Non-Scope:** general engineer cursor/order creation, full C4-on-CABHUT timing, bridge hut death, low/high bridge walker tables beyond duplicate-entry effects, save/load, and runtime debugger observation of a hand-built two-engineer scenario.  
**Confidence:** High for branch ordering and duplicate branch semantics; Medium-High for exact same-frame two-engineer outcome because it composes verified PerCellProcess removal with the verified live LogicClass scheduler rather than a runtime breakpoint trace.  
**Active in YR:** Yes. The path is standard YR infantry per-cell processing for engineers on `CABHUT`, and the scheduler/removal paths are the standard active object tick/removal paths.

## Target Question

If two engineers enter or resolve into the same bridge repair hut in the same tick/window, does gamemd consume both, emit duplicate sound/EVA, and/or apply repair twice?

## Non-Goals

- Do not re-investigate the generic CABHUT cursor/action gate.
- Do not re-investigate C4 timing or hut-death bridge collapse.
- Do not re-decode every bridge repair walker table; only use walker behavior needed for duplicate-entry consequences.
- Do not modify Rust or Ghidra state.

## Evidence Needed To Mark Complete

- Verify whether `PerCellProcess` has a bridge-repair success latch, repaired-cell count gate, or hut-local duplicate suppression.
- Verify order of EVA/SFX, bridge dispatch, callbacks, and engineer disposal.
- Verify how the main object loop handles an engineer removed during its own AI.
- Verify whether `MultiEngineer` gates CABHUT bridge repair.
- Compare the verified behavior to current Rust surfaces and propose concrete tests.

## Stop Conditions

Stop after proving the per-engineer branch semantics and scheduler/remove interaction. Runtime debugger reproduction of a specific two-engineer map is useful but not required for this slice; if static evidence showed ambiguous or hidden global state, downgrade to PARTIAL.

## 1. Overview

The bridge-repair branch is per-engineer and has no hut-level duplicate latch. When an engineer reaches a CABHUT cell under mission `8`, `0x0B`, or `0x19`, gamemd plays EVA/SFX first, runs the repair dispatcher, calls bridge-repair observers, refreshes the hut, optionally fires the engineer's attached trigger action, then destroys/limbos the engineer.

The exact same-tick outcome for two engineers is governed by the live `LogicClass` object vector. The loop walks forward and does not snapshot entries; when engineer A destroys itself, `ObjectClass::Conceal` can unregister it from the logic vector and compact entries. If engineer B was immediately after A, B can be shifted into the already-processed index and skipped until a later tick. If B already ran, or if B is not shifted into the skipped slot, B can run in the same tick and independently repeat feedback, consumption, and repair dispatch.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `Infantry+0x5A4` / `param_1[0x169]` | primary target object for enter/capture mission | `0x00519630` decompile | Yes |
| `InfantryType+0xEC3` | engineer flag used by the CABHUT PerCellProcess branch | `0x00519B58..0x00519B66` assembly/decompile | Yes |
| `Building+0x520` | building type pointer | `0x00519B7C..0x00519B82` | Yes |
| `BuildingType+0x16B6` | `BridgeRepairHut=yes` | `0x00519B7C..0x00519B8A`; `rulesmd.ini [CABHUT]` | Yes |
| `Rules+0x248` | `RepairBridgeSound`, `-1` means no SFX | `0x00519BCE..0x00519C02`; `rulesmd.ini:721` | Yes |
| `LogicClass+0x04/+0x10` | live object-AI vector pointer/count | `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md` | Yes |
| `Object+0x98` | logic-list membership bit | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` | Yes |
| `Object+0x81` | in-limbo flag set by Conceal | `ObjectClass::Conceal @ 0x005F4D30` | Yes |

## 3. Core Logic

### 3.1 Per-Engineer CABHUT Branch

For missions `8`, `0x0B`, or `0x19`, `InfantryClass::PerCellProcess @ 0x00519630` checks the building in the current cell against `Target` / secondary target. If the unit type has the engineer flag and the target is a building with `BridgeRepairHut=yes`, it enters the bridge-repair branch.

Execution order is fixed:

1. If `HouseClass::IsHumanPlayer()` and `CreateRadarEvent(engineer_cell)` returns nonzero, call `VoxClass::PlayEVA(0xFFFFFFFF)`.
2. If `Rules+0x248 != -1`, copy `building.Location` and call `VocClass::PlayAt`.
3. Run the 5x5 low-vs-high discriminator.
4. Dispatch `ProcessBridgeDestruction_Low @ 0x00570050` or `ProcessBridgeDestruction_High @ 0x00573540`.
5. Iterate observer array `DAT_00A83DEC` descending from `DAT_00A83DF8 - 1`, calling each observer `vtable+0x28(building, 0)`.
6. Call building `vtable+0x2E0`.
7. If engineer `+0x34` attached tag exists, call `TechnoClass::ProcessCellAction(0x30, engineer, DAT_00A8F1E0, 0, 0)`.
8. Call engineer `vtable+0xF8` at `0x0051A02E`.

There is no check for "bridge cells were actually repaired" before the sound, observer, hut refresh, trigger action, or engineer disposal.

Assembly anchors:
- Branch gate: `0x00519B58..0x00519B8A` reads engineer flag, target RTTI, and `BuildingType+0x16B6`.
- Sound-before-dispatch: `0x00519BCE..0x00519C07` checks `Rules+0x248` and calls `0x007509E0` before the scan begins.
- Low/high dispatcher calls: `0x00519CF6` and `0x00519D12`.
- Observer loop: `0x00519D17..0x00519D36`, descending.
- Engineer disposal: `0x0051A010..0x0051A02E`.

Active in YR: Yes. This is the stock CABHUT engineer-enter path.

### 3.2 Duplicate Semantics If The Second Engineer Runs

If a second engineer reaches the same branch after the first one, the branch runs again from the top. The first engineer's bridge mutation does not set any inspected hut flag or global "already repaired this tick" latch in this branch.

This means:
- The second branch can play EVA/SFX again, subject only to the same local-human and `RepairBridgeSound` gates.
- The second engineer is consumed even if its repair dispatcher changes zero cells.
- The second dispatcher can advance bridge state again if the walker reaches remaining damaged overlays. The high walkers treat intact high overlays (`0xCD..0xD0` / `0xD6..0xD9`) as in-range loop cells but no-op cases; they can continue along the connected high bridge band and repair later damaged cells (`0xD1..0xD5`, `0xE7`, `0xDA..0xDE`, `0xE8`).

Evidence:
- `PerCellProcess @ 0x00519630` has no bridge-repair result test before `vtable+0xF8`.
- `ProcessBridgeDestruction_High @ 0x00573540` scans first high overlay in the 5x5 window and calls `MapClass__RepairBridge_High`.
- `MapClass__RepairBridgeWalker_NS_High @ 0x005800D0` and `EW_High @ 0x00580600` loop while `FUN_00580B70` reports overlay in `[0xCD..=0xE8]`; intact overlays fall through with no write, damaged overlays invoke `FUN_00598030` and rewrite three cells.

Active in YR: Yes, conditional on the second engineer's AI actually running after the first branch.

### 3.3 Same-Tick Ordering Is Live Logic-Vector Order

The main object scheduler is a forward walk over the live `LogicClass` vector. It loads `items[i]`, calls object `vtable+0x5C`, increments `i`, then reloads `LogicClass+0x10`. It does not snapshot the object list.

Engineer disposal calls `vtable+0xF8` (`ObjectClass::UnInit` in standard object paths), which calls `vtable+0xD4`; `ObjectClass::Conceal @ 0x005F4D30` calls `FUN_0055BAE0` under normal logic-enabled object gates. The remover compacts the vector by shifting later entries left.

Consequences for two engineers in the same scheduler pass:
- If B is immediately after A and A repairs/UnInit's first, B is shifted into A's old index; the scheduler then increments past that index, so B does not run in that pass.
- If B already ran earlier, both can repair in the same tick in B-then-A order.
- If A removes itself and some other object was immediately after A, that other object can be skipped while B later in the vector still runs this pass.
- Therefore "same command resolution window" does not imply "both branches execute on the same tick"; it depends on live vector order and compaction.

Evidence:
- Scheduler loop: `LogicClass::PerTickUpdate @ 0x0055B5FB..0x0055B619` from `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`.
- UnInit: `ObjectClass::UnInit @ 0x005F65F0` calls `Detach_From_All_Lists`, then virtual `+0xD4`, clears alive, queues pending delete.
- Conceal: `ObjectClass::Conceal @ 0x005F4D30` calls `FUN_0055BAE0` before setting `+0x81=1`.
- Remover: `FUN_0055BAE0..0x0055BB2F` shifts later entries left and clears `Object+0x98`.

Active in YR: Yes.

### 3.4 `MultiEngineer` Does Not Gate CABHUT Repair

`MultiEngineer=no` is stock YR data and is parsed for lobby/settings, but the CABHUT branch does not read the multiplayer flag. The only visible `DAT_00A8B26C` use in the relevant `PerCellProcess` region is in the generic non-CABHUT capture/damage branch after the `BridgeRepairHut` branch has already jumped to cleanup.

Evidence:
- `rulesmd.ini:3037` and `rules.ini:2517` set `MultiEngineer=no`.
- Prior audit: `units/allied/ENGINEER.md` records `MultiEngineer` parser-only/desupported for standard capture.
- Fresh decompile: `PerCellProcess @ 0x00519630` shows `DAT_00A8B26C` only in the generic branch after `LAB_00519D47`, not in the `BridgeRepairHut` branch at `0x00519B82..0x00519D42`.

Active in YR: No for CABHUT bridge repair. The parsed setting is live data/UI, but not a CABHUT repair gate.

## 4. INI Keys

| Key | Stock value | Effect here | Evidence | Active in YR |
|---|---|---|---|---|
| `[CABHUT] BridgeRepairHut=yes` | yes | Opens bridge-repair branch | `rulesmd.ini:16348`; `0x00519B82` | Yes |
| `[AudioVisual] RepairBridgeSound= BridgeRepaired` | `BridgeRepaired` | SFX gate via `Rules+0x248 != -1` | `rulesmd.ini:721`; `0x00519BD3` | Yes |
| `[BridgeRepaired]` sound | `urepair`, global | played per branch execution | `soundmd.ini:807`, `5355..5359`; call `0x00519C02` | Yes |
| `EVA_BridgeRepaired` | registered EVA event | local-human branch can play EVA | `evamd.ini:49`, `982..987`; `0x00519BA8..0x00519BC9` | Yes |
| `MultiEngineer=no` | no | no CABHUT effect | `rulesmd.ini:3037`; PerCellProcess branch absence | No for CABHUT |

## 5. Integration Points

| Integration | Finding | Evidence | Active in YR |
|---|---|---|---|
| `InfantryClass::AI` | Standard infantry AI eventually reaches movement/per-cell processing and capture mission handling. | `InfantryClass::AI @ 0x0051BAB0` decompile | Yes |
| `InfantryClass::PerCellProcess` | Owns CABHUT repair branch and engineer disposal. | `0x00519630` | Yes |
| `ProcessBridgeDestruction_Low/High` | Misnamed engineer repair dispatchers. | `0x00570050`, `0x00573540`; prior bridge docs | Yes |
| `ObjectClass::UnInit` / `Conceal` | Engineer disposal can compact the live logic vector. | `0x005F65F0`, `0x005F4D30`, `FUN_0055BAE0` | Yes |
| Current Rust bridge-repair tick | Uses a prebuilt sorted candidate snapshot; this can process both engineers in one tick regardless of live-vector skip semantics. | `src/sim/world/world_orders.rs:264..280` | Rust-facing |

## 6. Current Rust Implementation Status

Rust currently builds a sorted snapshot of eligible engineers, then loops it. That differs from gamemd's live object scheduler when the first engineer despawns and would shift/skip the next logic-vector entry.

Relevant surfaces:
- `src/sim/world/world_orders.rs:261..364`: `tick_bridge_repair_orders`.
- `src/sim/world/world_orders_bridge_repair_tests.rs:547`: `two_engineers_both_repair_same_tick`.
- `src/sim/bridge_state/walker.rs:65..115`, `138..156`, `243..315`: repair scan and walker dispatch.
- `src/app_sim_tick.rs:530..571`: app-layer SFX/EVA conversion.

Current deltas:
- Rust processes every pre-snapshotted candidate even if the previous candidate despawned; gamemd can skip the immediately shifted next object.
- Rust test asserts two events for the same tick unconditionally; gamemd requires vector-order-specific scenarios.
- Rust emits sound events before mutation, matching branch order for any engineer that actually reaches the branch.
- Rust consumes engineers even on no-change repair, matching the per-branch disposal semantics.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| CABHUT branch gate | verified | `0x00519B58..0x00519B8A` | none |
| SFX/EVA before mutation | verified | `0x00519BA8..0x00519C07` | exact `0xFFFFFFFF` EVA resolution remains inherited from prior docs |
| Low/high dispatch order | verified | `0x00519CF6`, `0x00519D12` | none |
| Observer loop and hut refresh | verified | `0x00519D17..0x00519D42` | observer identity outside scope |
| Engineer disposal | verified | `0x0051A010..0x0051A02E`; `0x005F65F0`; `0x005F4D30` | exact Infantry vtable memory not re-read because debugger memory server was unavailable; vtable slot identity is supported by prior audits |
| Duplicate suppression latch | verified absent in branch | `0x00519630` decompile | runtime breakpoint scenario not run |
| Second walker can advance later damage | verified | `0x005800D0`, `0x00580600` | low bridge duplicate variant not separately walked; same shape per prior docs |
| Logic vector ordering/skip | verified by scheduler/remover composition | scheduler and helper reports, `0x005F4D30` decompile | exact object-vector positions for a concrete map need runtime debugger |
| `MultiEngineer` CABHUT use | verified absent | `PerCellProcess @ 0x00519630`; `rulesmd.ini` | none for CABHUT |
| Current Rust comparison | verified by static source scan | Rust files listed above | no Rust tests were run; no Rust modified |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-BRME-001 -- Is the CABHUT branch live in YR? -> Yes, `InfantryClass::PerCellProcess` branches on `BuildingType+0x16B6` for stock `CABHUT`.` (evidence: `0x00519B82`; `rulesmd.ini:16348`)
- `[RESOLVED] OQ-BRME-002 -- Is sound/EVA before mutation? -> Yes, both feedback blocks precede scan and dispatcher calls.` (evidence: `0x00519BA8..0x00519C07`, dispatch at `0x00519CF6/0x00519D12`)
- `[RESOLVED] OQ-BRME-003 -- Is engineer consumption conditional on repaired cell count? -> No; the branch falls through to `vtable+0xF8` after feedback/dispatch/callbacks.` (evidence: `0x0051A010..0x0051A02E`)
- `[RESOLVED] OQ-BRME-004 -- Is there a hut-level duplicate suppression flag in the branch? -> No branch-local write/read acts as a duplicate latch before feedback/dispatch/disposal.` (evidence: `0x00519630` decompile)
- `[RESOLVED] OQ-BRME-005 -- Can a second branch emit a second SFX/EVA? -> Yes if it runs; feedback gates are local-human and `Rules+0x248`, not mutation-success or per-tick uniqueness.` (evidence: `0x00519BA8..0x00519C07`)
- `[RESOLVED] OQ-BRME-006 -- Can duplicate repair advance state twice? -> Yes if the second branch reaches a walker with remaining damaged overlays; intact overlays are no-op in-range cells, not hard stop cells.` (evidence: high walkers `0x005800D0`, `0x00580600`)
- `[RESOLVED] OQ-BRME-007 -- Does same tick always mean both engineers process? -> No; LogicClass live vector compaction can skip the immediately shifted next object.` (evidence: scheduler `0x0055B5FB..0x0055B619`; remover `0x0055BAE0..0x0055BB2F`)
- `[RESOLVED] OQ-BRME-008 -- Does `MultiEngineer` gate CABHUT repair? -> No evidence; CABHUT branch does not read it, and prior audit treats it as parser-only/desupported.` (evidence: `0x00519630`; `units/allied/ENGINEER.md`; `rulesmd.ini:3037`)
- `[DEFERRED] OQ-BRME-009 -- What exact vector indices occur for a hand-built two-engineer retail scenario?` (category: needs-runtime-debugger; reason: static binary proves the scheduler contract but not a concrete map's allocation order; next-step-if-pursued: breakpoint on `0x0055B608` and log both engineer pointers around CABHUT entry)
- `[DEFERRED] OQ-BRME-010 -- What exact EVA id does `PlayEVA(0xFFFFFFFF)` resolve to in this branch?` (category: out-of-scope; reason: this slot only needs duplicate count/order; next-step-if-pursued: audit VoxClass `0x00752700` sentinel handling)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Live scheduler order can skip the immediately shifted next engineer after first engineer self-removal. | `0x0055B5FB..0x0055B619`; `0x005F4D30`; `0x0055BAE0..0x0055BB2F` | mismatch: current bridge repair uses a prebuilt candidate snapshot | `src/sim/world/world_orders.rs::tick_bridge_repair_orders`; future scheduler surface | Model same-tick bridge repair through a live logic-order contract or explicitly account for removal-shift skip before asserting duplicate events. | Two engineers consecutive in logic order enter one CABHUT; first repairs and is consumed; second remains for next tick rather than emitting same-tick event. | Do not keep unconditional snapshot semantics as binary parity. Proposed test: `bridge_repair_consecutive_engineers_second_skipped_by_live_logic_order` |
| If a second engineer's branch does run, feedback and disposal repeat independently of mutation success. | `0x00519BA8..0x0051A02E` | mostly matches for branch-local semantics | `src/sim/world/world_orders.rs`; `src/app_sim_tick.rs` | Keep SFX/EVA event before repair dispatch and consume the engineer even on no-change/no remaining bridge mutation. | Non-consecutive logic order lets both engineers run in one tick; two feedback events are queued; both engineers are consumed. | Do not coalesce duplicate same-frame bridge repair sounds by hut id. Proposed test: `bridge_repair_nonconsecutive_engineers_emit_two_events_and_consume_both` |
| Duplicate repair dispatch can advance remaining damaged cells; it is not limited to one strip if a second branch reaches the walker. | High walkers `0x005800D0`, `0x00580600`; `ProcessBridgeDestruction_High @ 0x00573540` | unchecked/maybe partial: current tests do not assert second-pass tail mutation | `src/sim/bridge_state/walker.rs`; `world_orders_bridge_repair_tests.rs` | Ensure a second repair pass from the same hut scan can walk across already-intact in-range overlays and repair later damaged overlays. | Seed connected high span with intact first hit plus later destroyed cells; second processed engineer repairs later damaged cells. | Do not return just because the first scanned bridge overlay is already healthy. Proposed test: `bridge_repair_second_pass_walks_past_intact_overlay_to_remaining_damage` |

## Negative Facts / Do Not Do

- Do not make `MultiEngineer` affect CABHUT bridge repair; it is not read by this branch.
- Do not suppress duplicate `BridgeRepaired` SFX/EVA by hut id when a second engineer branch actually executes.
- Do not treat "same tick" as "all candidates in a prebuilt snapshot run"; gamemd's object scheduler is live and removal compacts it.
- Do not require a positive repaired-cell count before consuming the engineer.
- Do not stop the second repair walker at an intact high bridge overlay; intact overlays are in-range no-op cells that allow the loop to continue.

## Stale Docs / Follow-Up Docs

- `traces/MULTI_ENGINEER_SAME_TICK_BRIDGE_REPAIR_TRACE.md` should replace "same-frame object ordering and duplicate engineer-entry behavior could not be live-verified" with: "Static binary evidence verifies live-vector ordering: a self-removing first engineer can skip an immediately shifted next engineer, while any second engineer whose `PerCellProcess` actually runs repeats feedback/disposal and can run another bridge repair pass."
- The same trace's Rust fixture summary should not imply gamemd always consumes both engineers in one tick; make it conditional on logic-vector order.

## Remaining Uncertainty

Runtime debugger confirmation of one hand-built retail two-engineer scenario's exact vector indices remains deferred. No material branch semantics remain open for implementation handoff.

## Sources

- Ghidra decompiled: `0x00519630`, `0x0051BAB0`, `0x00570050`, `0x00573540`, `0x005800D0`, `0x00580600`, `0x005F65F0`, `0x005F4D30`.
- Ghidra assembly context: `0x00519B58..0x0051A02E`.
- Prior reports: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, `REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md`, `TECH_CABHUT_GHIDRA_REPORT.md`, `units/allied/ENGINEER.md`, `traces/MULTI_ENGINEER_SAME_TICK_BRIDGE_REPAIR_TRACE.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/soundmd.ini`, `ini/evamd.ini`.

**Status:** COMPLETE for static binary semantics and Rust handoff; runtime index reproduction deferred as non-blocking evidence.
