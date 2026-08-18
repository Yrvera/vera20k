# Bridge Collapse Trigger Event 0x1F -- Ghidra Research Report

**Address(es):** `0x00575EE0`, `0x006E53A0`, `0x007264C0`, `0x0071E940`, `0x0071F680`, `0x004D8B60`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `RepairBridgeSegment` bridge-collapse event `0x1F` broadcast, attached-tag gate, iteration footprint, trigger-only side effects, and Rust skirmish stub safety.  
**Non-Scope:** engineer repair event `0x30` except negative comparison; full trigger enum/name table; exact trigger action execution semantics beyond dispatcher queuing; unrelated bridge mutation/audio.  
**Confidence:** High for numeric event delivery and side effects; Medium for standard-map corpus absence because MIX-packed map corpus was not decoded in this slot.  
**Active in YR:** Conditional. The code is live in YR bridge-collapse paths, but produces trigger effects only when a map attaches tags/triggers to affected cells.

Working notes required before investigation:

- Target question: What does `RepairBridgeSegment` / `ProcessCellAction(0x1F, 0, DAT_00ABD480, 0, 0)` do on bridge destruction, and can Rust safely stub it for skirmish?
- Non-goals: Do not expand into bridge repair event `0x30`, exact audio, full trigger runtime, or full bridge collapse state machine.
- Evidence needed to mark COMPLETE: decompile plus assembly context for `0x00575EE0` call arguments and gates; decompile dispatcher/evaluator proving trigger-only behavior; caller/liveness evidence; current Rust surface scan; map-data check or bounded uncertainty for skirmish bindings.
- Stop conditions: stop after the event `0x1F` broadcast semantics and Rust handoff are resolved; defer full trigger enum/name lookup if it exceeds this slice.

## 1. Overview

`RepairBridgeSegment` is misnamed. On the bridge-destruction endpoint path it walks a span footprint and, for each cell that has an attached tag pointer at `CellClass+0x3C`, calls `TechnoClass::ProcessCellAction` with event ID `0x1F`, null source, `DAT_00ABD480`, and two zero flags.

The called function is not a bridge mutator. It is a trigger-event broadcaster over the attached tag/action list. It may queue scripted trigger actions if the cell tag's trigger conditions match event `0x1F`, but it does not damage objects, mutate bridge overlays, rebuild zones, play built-in bridge audio, or recursively trigger more bridge destruction.

## 2. Key Offsets / Arguments

| Item | Offset / value | Meaning | Active in YR |
|---|---:|---|---|
| Cell attached tag/object pointer | `CellClass+0x3C` | Non-null gate before event `0x1F` delivery | Yes, conditional on map tags |
| Event delivered by span broadcaster | `0x1F` | Numeric trigger event passed as first stack argument to `0x006E53A0` | Yes |
| Source object argument | `0` | Bridge collapse broadcaster passes no source object | Yes |
| Coordinate/context argument | `DAT_00ABD480` | Passed by value from global; prior report observed zero/sentinel | Yes |
| Flags | `0, 0` | Final two dispatcher arguments | Yes |
| Dispatcher re-entry guards | `TechnoClass+0x34/+0x35` | Early-out when set; `+0x35` is set during dispatch and cleared before return | Yes |
| Attached tag/list head | `TechnoClass+0x24/+0x28` | No attached tag means immediate no-op | Yes, conditional |

## 3. Core Logic

### 3.1 `RepairBridgeSegment @ 0x00575EE0`

Verified behavior:

- Normalizes the two endpoint coords so iteration starts at the lower coordinate on the varying axis.
- Chooses EW-style iteration when `start.y == end.y`; otherwise NS-style iteration.
- Loops while current coord is not equal to the normalized end coord. This is exclusive of the end coord.
- Per axis step, tests four cells total: the current span cell plus three cells in the perpendicular fan.
- Each tested cell runs a fixed map-grid lookup with stride `0x200` and valid index range `0..=0x3FFFF`; invalid/null lookup uses the default cell sentinel.
- For each tested cell, only `cell+0x3C != 0` gates the event call.
- The event call uses the same argument shape at all seven call sites: `ProcessCellAction(0x1F, 0, DAT_00ABD480, 0, 0)`.

Evidence:

- Decompile of `0x00575EE0` shows `while current != end`, `cell+0x3C` tests, four tested cells per step, and seven calls to `TechnoClass__ProcessCellAction(0x1f,0,DAT_00abd480,0,0)`.
- Assembly context at `0x00575F95`, `0x00576007`, `0x0057606C`, `0x005760CC`, `0x00576137`, `0x0057619C`, `0x005761DE` shows `MOV ECX,[cell+0x3c]`, `TEST ECX,ECX`, conditional skip, then pushes `0, DAT_00ABD480, 0, 0, 0x1F` and calls `0x006E53A0`.

Active in YR: Yes. The callers below are live bridge endpoint/edge update paths in normal YR bridge destruction; effect remains conditional on map tags.

### 3.2 Endpoint Callers

Fresh decompiles verify four direct endpoint callers:

- `MapClass__FindBridgeEndpoints_EW_High @ 0x0057DAF0`
- `MapClass__FindBridgeEndpoints_NS_High @ 0x0057DC20`
- `MapClass__FindBridgeEndpoints_EW_Low @ 0x0057C870`
- `MapClass__FindBridgeEndpoints_NS_Low @ 0x0057C990`

Each walks outward until leaving the high overlay band `0xCD..0xE8` or low overlay band `0x4A..0x65`, then calls `RepairBridgeSegment` with computed span endpoints.

Active in YR: Yes. These functions are reached when bridge destruction reaches terminal destroyed states; this report does not re-prove every upstream collapse caller because that was already settled by prior bridge reports.

### 3.3 Dispatcher `0x006E53A0`

`TechnoClass__ProcessCellAction` is better described as `FireTriggerAction(eventType, source, coord, flags...)` for this path:

- Early-outs if map editor is active or `TechnoClass+0x34/+0x35` guard bytes are set.
- Early-outs if `TechnoClass+0x24` attached tag pointer is null.
- Sets `TechnoClass+0x35 = 1` while processing.
- Iterates the linked trigger/action entries from `TechnoClass+0x28`.
- Calls `TriggerActionEntry__EvaluateConditions` with `param_2` as the event type.
- If conditions match, may call `TriggerActionEntry__PlayVoiceForObjects` and `DynamicVectorClass__Add` depending on trigger action type `0`, `1`, or `2`.
- Clears `TechnoClass+0x35` before return.

No bridge state is mutated here.

Active in YR: Yes, conditional on attached tags/triggers. The dispatcher is the normal YR trigger system.

### 3.4 Condition Matching

`TriggerActionEntry__EvaluateConditions @ 0x007264C0` walks the trigger's condition list and calls `TriggerCondition__Evaluate @ 0x0071E940`. The latter has a match-only cluster that includes `case 0x1F`; for that case, if the live event argument does not equal the condition kind, it returns false. If it equals `0x1F` and the map editor flag is clear, evaluation can continue.

Active in YR: Yes, conditional on a trigger condition of kind `0x1F`.

### 3.5 Naming Caveat: `0x1F` vs `0x18`

The bridge span broadcaster's numeric event is definitely `0x1F`. However, a separate trigger-category classifier `FUN_0071F680 @ 0x0071F680` sets the destroyed-event registry bit `0x04` only for event codes `8` and `0x18`, while `0x1F` only contributes category bit `0x01`.

`FootClass__PerCellProcess @ 0x004D8B60` uses the destroyed-event tag registry `DAT_008B41A8` and, after reachability/proximity checks, fires `ProcessCellAction(0x18, ...)`. That is a separate trigger proximity/destroyed-event path and not the `RepairBridgeSegment` span broadcaster.

Therefore, Rust should preserve the numeric event as `0x1F` for this bridge-collapse span hook, but should avoid overconfident public naming such as `BridgeDestroyed` until the full trigger enum/name table is verified. A safer internal name is `BridgeSpanCollapseEvent31`.

Active in YR: Yes for both paths, conditional on tags. Evidence is fresh decompile of `0x0071F680` and `0x004D8B60`.

## 4. INI / Map Keys

No rules/art INI key controls this broadcast. It is controlled by scenario trigger data:

| Section | Role | Effect |
|---|---|---|
| `[CellTags]` | Binds cells to tag IDs | Populates `CellClass+0x3C` equivalent |
| `[Tags]` | Binds tag IDs to triggers | Supplies attached trigger/tag object |
| `[Triggers]` | Defines trigger metadata | Supplies trigger/action linkage |
| `[Events]` | Stores event condition kinds | Must include condition kind `31` for this exact hook |
| `[Actions]` | Trigger actions | Determines actual script effect if event matches |

Top-level installed skirmish map files scanned in `C:/Users/enok/Documents/Command and Conquer Red Alert II/` (`*.map`, `*.mmx`, `*.mpr`, `*.yrm`) had no `[Events]` condition kind `31`. Some skirmish maps do have triggers for ambient/audio timing; absence of event 31, not absence of triggers, is the relevant fact. MIX-packed internal map corpus was not decoded in this slot.

Active in YR: Conditional. Scenario triggers are live in YR; the event is dormant unless authored.

## 5. Integration Points

Bridge mutation order in the already-settled collapse cascade is:

1. Per-cell bridge destruction reaches terminal destroyed state.
2. Endpoint finder computes span endpoints.
3. `RepairBridgeSegment` broadcasts event `0x1F` to tagged cells across the span footprint.
4. Other already-documented bridge refresh/zone work happens outside this trigger dispatcher.

The event dispatcher itself queues trigger actions; it does not run bridge damage, object damage, movement, or zone rebuild logic.

Active in YR: Yes, conditional on bridge collapse and map tags.

## 6. Current Rust Implementation Status

Current Rust has an intentional stub:

- `src/sim/world/bridge_orchestrator.rs` calls `notify_bridge_span_collapse` after debris/rim refresh in both normal damage and hut collapse paths.
- `notify_bridge_span_collapse` currently ignores `sim` and `cells`.
- The module comment states this is an intentional no-op on skirmish.
- `src/sim/trigger_runtime.rs` supports only a small subset of scenario events/actions today and does not implement event `31` delivery from cell tags.
- `src/map/cell_tags.rs`, `src/map/tags.rs`, `src/map/triggers.rs`, `src/map/events.rs`, and `src/map/trigger_graph.rs` parse/link trigger structures enough for future support.

Current Rust delta: no gameplay mismatch for skirmish-only scope; missing campaign/scripted-map support for event `31`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RepairBridgeSegment @ 0x00575EE0` event call shape | verified | decompile `0x00575EE0`; assembly contexts at seven call sites | none for this slice |
| Attached-tag gate `CellClass+0x3C` | verified | decompile `0x00575EE0`; assembly `MOV ECX,[...+0x3C]` + `TEST/JZ` | none |
| Loop endpoint inclusivity | verified | decompile `0x00575EE0`, break on current == end before processing | none |
| Per-step footprint | verified | decompile `0x00575EE0`, four tested cells per iteration | exact direction offset values deferred; not needed for stub decision |
| Endpoint callers low/high EW/NS | verified | decompile `0x0057DAF0`, `0x0057DC20`, `0x0057C870`, `0x0057C990` | upstream collapse caller re-proof out of scope |
| `TechnoClass__ProcessCellAction @ 0x006E53A0` side effects | verified | decompile `0x006E53A0` | full trigger action executor out of scope |
| `TriggerActionEntry__EvaluateConditions @ 0x007264C0` | verified | decompile `0x007264C0` | none for event-match plumbing |
| `TriggerCondition__Evaluate @ 0x0071E940` includes `0x1F` match-only case | verified | decompile `0x0071E940` | exact public enum label deferred |
| `FUN_0071F680` category conflict check | verified | decompile `0x0071F680` | full enum/name table deferred |
| `FootClass__PerCellProcess` separate event `0x18` path | touched-not-exhausted | decompile `0x004D8B60` around destroyed-tag registry loop | full `0x18` semantics out of scope |
| Top-level skirmish map event-31 scan | verified-bounded | PowerShell parse of top-level installed map files | MIX-contained map corpus not decoded |
| Rust no-op hook | verified | `src/sim/world/bridge_orchestrator.rs` grep/read | campaign implementation deferred |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Is event `0x1F` actually passed from bridge destruction? -> Yes, seven call sites in `0x00575EE0` push `0x1F` before calling `0x006E53A0`.` (evidence: `0x00575F95`, `0x00576007`, `0x0057606C`, `0x005760CC`, `0x00576137`, `0x0057619C`, `0x005761DE`)
- `[RESOLVED] OQ-2 -- What gates delivery per cell? -> Only non-null `CellClass+0x3C` at each tested cell.` (evidence: decompile `0x00575EE0`; assembly context at seven call sites)
- `[RESOLVED] OQ-3 -- Is the endpoint inclusive? -> No, the loop breaks when current equals normalized end before processing that coordinate.` (evidence: decompile `0x00575EE0`)
- `[RESOLVED] OQ-4 -- Does `0x006E53A0` mutate bridge/gameplay state directly? -> No direct bridge/object mutation; it is an attached-trigger dispatcher with re-entry guards and action queueing.` (evidence: decompile `0x006E53A0`)
- `[RESOLVED] OQ-5 -- Is `0x1F` accepted by condition evaluation? -> Yes, `TriggerCondition__Evaluate` includes `case 0x1F` in the match-only cluster.` (evidence: decompile `0x0071E940`)
- `[RESOLVED] OQ-6 -- Is there a conflicting bridge/destroyed event ID? -> Separate classifier treats `0x18` as destroyed-event registry member; `RepairBridgeSegment` still delivers `0x1F`.` (evidence: decompile `0x0071F680`, `0x004D8B60`)
- `[RESOLVED] OQ-7 -- Is Rust's skirmish no-op acceptable? -> Yes for skirmish-only gameplay because the binary effect requires authored cell tags and event-31 trigger conditions, and current Rust has no event-31 trigger runtime.` (evidence: decompile `0x006E53A0`; Rust scan; top-level map scan)
- `[DEFERRED] OQ-8 -- What exact editor/public enum label should event `0x1F` use?` (category: `out-of-scope`; reason: full trigger enum/name lookup exceeds this narrow bridge-collapse hook; next-step-if-pursued: decompile trigger event name lookup and compare FA2/INI event tables)
- `[DEFERRED] OQ-9 -- Do MIX-packed internal retail maps contain event-31 bridge hooks?` (category: `requires-different-system-context`; reason: this slot did not decode MIX archives; next-step-if-pursued: use asset/MIX tooling to extract all shipped maps and run the `[Events]` kind-31 scan)
- `[DEFERRED] OQ-10 -- What do all trigger actions queued from event `0x1F` do?` (category: `out-of-scope`; reason: only delivery semantics were requested; next-step-if-pursued: investigate trigger action executor and action IDs used by campaign maps)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Bridge span collapse delivers numeric event `0x1F` only to cells with attached tag pointer `+0x3C`; four tested cells per axis step; endpoint exclusive | `0x00575EE0` decompile + seven assembly call sites | missing for campaign; skirmish no-op acceptable | `src/sim/world/bridge_orchestrator.rs`, future trigger runtime/cell-tag surface | Keep current no-op for skirmish; future campaign support should deliver event 31 to matching cell tags over the binary footprint | Tagged bridge test map collapses a span and queues one trigger action for each tagged tested cell, excluding the terminal endpoint | Do not fire globally for every destroyed bridge cell |
| Dispatcher `0x006E53A0` is trigger-only and does not mutate bridge state | `0x006E53A0`, `0x007264C0`, `0x0071E940` decompile | none for current skirmish behavior; missing trigger action queue for campaigns | `src/sim/trigger_runtime.rs`, `src/map/cell_tags.rs`, `src/map/trigger_graph.rs` | Event-31 delivery should evaluate attached tag conditions and enqueue actions; bridge mutation remains in bridge systems | A map with `[Events]` kind 31 and a simple action fires the action after collapse without changing extra bridge cells | Do not implement event 31 as damage, sound, zone rebuild, or recursive collapse |
| Event `0x18` destroyed-tag registry path is separate from `RepairBridgeSegment` event `0x1F` | `0x0071F680` and `0x004D8B60` decompile | unchecked/not implemented | future trigger runtime, not bridge orchestrator core | Use distinct event constants/names until enum table is verified | Trigger runtime has separate tests for event 31 bridge-span collapse and event 24/0x18 destroyed-proximity path when implemented | Do not merge `0x18` and `0x1F` under one `BridgeDestroyed` enum variant |

Acceptance test-name proposals:

- `bridge_span_collapse_event31_queues_only_tagged_cells_exclusive_endpoint`
- `bridge_event31_dispatch_does_not_mutate_bridge_state`
- `trigger_event18_and_event31_are_distinct_bridge_related_hooks`

### Negative Facts / Do Not Do

- Do not treat `RepairBridgeSegment` as a repair function. Evidence: it only calls `ProcessCellAction(0x1F,...)` on tagged cells; no repair walker or overlay writes in `0x00575EE0`.
- Do not broadcast event `0x1F` to every destroyed cell in Rust's `destroyed_set`. Evidence: binary walks normalized endpoints and a four-cell-per-step footprint, endpoint-exclusive.
- Do not fire event `0x1F` when no cell tag exists. Evidence: every call is guarded by `CellClass+0x3C != 0`.
- Do not attach built-in audio/EVA/bridge damage semantics to event `0x1F`. Evidence: `0x006E53A0` only evaluates trigger conditions, plays trigger-configured voice, and queues trigger actions.
- Do not conflate event `0x1F` with event `0x18`. Evidence: `0x0071F680` puts `0x18` but not `0x1F` in destroyed-event registry bit `0x04`; `FootClass__PerCellProcess` fires `0x18` from that registry separately.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md`: replace "TriggerEvent::BridgeDestroyed = 0x1F" with "The bridge span-collapse path delivers numeric event `0x1F`; exact public/editor enum label remains unverified. Keep it distinct from the separate destroyed-event registry path that uses `0x18`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` section 19 item 4: replace "what do actions 31 and 48 do? Likely tag-trigger fires" with "`0x1F` is verified as a bridge span-collapse trigger-event broadcast via `0x00575EE0 -> 0x006E53A0`, gated by `CellClass+0x3C`; it has no direct bridge mutation and may remain a skirmish no-op until campaign triggers are supported."

## Sources

- Ghidra decompile: `0x00575EE0`, `0x006E53A0`, `0x007264C0`, `0x0071E940`, `0x0057DAF0`, `0x0057DC20`, `0x0057C870`, `0x0057C990`, `0x0071F680`, `0x004D8B60`.
- Ghidra assembly context: `0x00575F95`, `0x00576007`, `0x0057606C`, `0x005760CC`, `0x00576137`, `0x0057619C`, `0x005761DE`, `0x0071F680`.
- Existing docs: `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, `FUN_006E61F0_BRIDGE_LINKED_PREDICATE_GHIDRA_REPORT.md`, `UNREGISTERBRIDGEREPAIRHUT_AND_HUT_REGISTRY_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/world/bridge_orchestrator.rs`, `src/sim/trigger_runtime.rs`, `src/map/events.rs`, `src/map/cell_tags.rs`, `src/map/trigger_graph.rs`.
- Map scan: PowerShell parse of top-level installed `*.map`, `*.mmx`, `*.mpr`, `*.yrm` under `C:/Users/enok/Documents/Command and Conquer Red Alert II/`; result: no `[Events]` condition kind `31` in top-level files.
