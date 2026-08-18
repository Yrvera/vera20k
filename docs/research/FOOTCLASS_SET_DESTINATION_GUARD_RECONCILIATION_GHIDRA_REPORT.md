# FootClass::Set_Destination_Internal Guard Reconciliation - Ghidra Research Report

**Address(es):** `0x004D94B0` primary; supporting `0x00741970`, `0x007425A0`, `0x00710000`, `0x0070FEE0`, `0x004D31E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** guard/side-effect bytes in `FootClass::Set_Destination_Internal @ 0x004D94B0` that affect NavCom handoff: `+0x6AC`, `+0x6AD`, `+0x6AE`, `+0x2B0`, `+0x2AC`, and stale semantic names.  
**Non-Scope:** full `TechnoClass::Set_Destination` preprocessing, full deploy/undeploy state machine, global writer enumeration for every adjacent byte, full NavQueue producer scan, and full arrival pipeline.  
**Confidence:** High for the bounded guard block and linked-object cleanup; Medium for best semantic name of `+0x6AD` because clear paths outside this slice were not globally exhausted.  
**Active in YR:** Yes. Foot-derived mobile objects reach `0x004D94B0` through Foot vtable slot `+0x480` and active `TechnoClass::Set_Destination @ 0x00741970` tail/direct calls. The disputed bytes are runtime Foot object fields initialized in `FootClass` constructor and read/written by active YR movement/deploy paths.

## Working Notes

Target question: Reconcile exact `FootClass::Set_Destination_Internal @ 0x004D94B0` guard/side-effect bytes relevant to NavCom handoff, especially `+0x6AC/+0x6AD/+0x6AE/+0x2B0/+0x2AC`.  
Non-goals: Do not redo all Set_Destination preprocessing, deploy, bridge, or NavQueue systems; do not implement Rust.  
Evidence needed to mark COMPLETE: primary decompile plus assembly context for the guard block, caller/liveness evidence, supporting writer evidence for `+0x6AC/+0x6AD`, Rust surface scan, and stale-doc replacement wording.  
Stop conditions: missing Ghidra function boundary, read-only Ghidra unavailable, guard block requiring broad deploy investigation, or evidence contradicting already-settled facts that cannot be resolved in this slot.

## 1. Overview

`0x004D94B0` clears `Foot+0x5A0` before every accepted or silently dropped call, rejects non-null destinations under three guards, optionally calls linked deploy/chrono cleanup when `+0x2AC` is present, then writes `Foot+0x5A4 = target`. The special null cleanup is not the same as the non-null `+0x2AC` path: it checks owner `+0x2B0`, clears the linked object's `+0x2AC`, clears owner `+0x2B0`, and sets owner `+0x6AE = 1`.

The best semantic reconciliation is: `+0x6AD` is a runtime Foot deploy/locomotor-piggyback active guard. It is not accurately described as only `IsDeploying`, only `IsDeployed`, or a `TechnoTypeClass` field. It is constructor-cleared, set by active `TechnoClass::PerformDeploy`, blocks non-null destination writes, gates null linked cleanup, and is read by other locomotor/deploy systems.

## 2. Key Offsets

| Offset | Verified behavior in this slice | Evidence | Active in YR |
|---|---|---|---|
| `+0x5A0` | Cleared before guard acceptance/rejection. | `0x004D94C7`: `MOV [EBP+0x5A0], EBX`; decompile `param_1[0x168] = 0`. | Yes; every call to `0x004D94B0`. |
| `+0x5A4` | NavCom write occurs only after non-null silent-drop guards and optional `+0x2AC` helper. | `0x004D9510`: `MOV [EBP+0x5A4], ESI`. | Yes; final Foot destination writer. |
| `+0x6AC` | One-shot skip-`Head_To_Coord`: if nonzero, clear to 0 and jump past target-coordinate fetch/locomotor `+0x44`. | `0x004D9607..0x004D9618`; writer `0x007425BF` in active `TechnoClass::Set_Destination` chrono/teleport branch. | Conditional; active for chrono/teleporter destination preprocessing that sets the byte. |
| `+0x6AD` | Non-null destination silent-drop guard; null linked-cleanup gate. | Reads at `0x004D94BE` and `0x004D9518`; constructor clear `0x004D3414`; writer `0x00710352`. | Conditional; active when deploy/locomotor-piggyback state is set. |
| `+0x6AE` | Set to 1 only in this guard block when null target, `+0x6AD != 0`, and owner `+0x2B0 != 0`. | `0x004D9538`: `MOV byte ptr [EBP+0x6AE], 0x1`. | Conditional; active for linked deploy/piggyback cleanup. |
| `+0x2AC` | Non-null destination path: if owner `+0x2AC != 0` and target non-null, call `BuildingClass::DeployUnit_ChronoWarp(1)` before NavCom write. Also the field cleared on the linked object during null cleanup. | `0x004D94F9..0x004D9509`; `0x004D952C` writes `[EAX+0x2AC]=0`. | Conditional; active linked deploy/chrono infrastructure. |
| `+0x2B0` | Null cleanup owner link. If present, its pointed object's `+0x2AC` is cleared, then owner `+0x2B0` is cleared. | `0x004D9522..0x004D9532`. | Conditional; active after linked object setup by deploy infrastructure. |

## 3. Core Guard Order

Verified order for the scoped guard block:

1. Load `+0x6AD`, set `EBX=0`, then clear `+0x5A0 = 0`.
2. If `+0x6AD != 0 && target != 0`, return to the common epilogue without writing `+0x5A4`, without clearing `+0x6AD`, and without resetting `+0x6AC`.
3. If `+0x82 != 0 && target != 0`, same silent drop.
4. If `+0x2E4 != 0 && target != 0`, same silent drop.
5. If owner `+0x2AC != 0 && target != 0`, call `BuildingClass::DeployUnit_ChronoWarp(1)`.
6. Write `+0x5A4 = target`.
7. If `target == 0 && +0x6AD != 0 && +0x2B0 != 0`, clear `(*+0x2B0)+0x2AC`, clear owner `+0x2B0`, set owner `+0x6AE = 1`.
8. Continue into null clear-navigation or non-null `Head_To_Coord` handling. In the non-null branch, `+0x6AC != 0` clears itself and skips the `Head_To_Coord` dispatch; `+0x6AC == 0` performs target coord fetch and locomotor vtable `+0x44`.

Handoff-critical evidence: decompile for `0x004D94B0` plus assembly contexts `0x004D94BE`, `0x004D94C7`, `0x004D94D7`, `0x004D94E9`, `0x004D94F9`, `0x004D9505`, `0x004D9510`, `0x004D9518`, `0x004D9522`, `0x004D9538`, `0x004D9607`, `0x004D96C2`; raw bytes read `0x004D94B0..0x004D96DF`.

## 4. Semantic Name Reconciliation

| Field | Prefer this wording | Avoid | Why |
|---|---|---|---|
| `Foot+0x6AC` | `skip_head_to_coord_once` / `deferred_head_to_coord_once` | `IsDeploying` | It is set by the chrono/teleporter preprocessing branch at `0x007425BF`, and the only scoped read in `0x004D94B0` clears it and skips the locomotor `Head_To_Coord` call once. |
| `Foot+0x6AD` | `deploy_or_locomotor_piggyback_active` | only `IsDeploying`, only `IsDeployed`, or `TechnoType+0x6AD` | Runtime Foot byte is constructor-cleared and set by `TechnoClass::PerformDeploy @ 0x00710352`; in this slice it blocks non-null destination writes and enables null linked cleanup. Same displacement in `TechnoTypeClass` is a different layout. |
| `Foot+0x6AE` | `post_deploy_link_cleanup_marker` / `returning_from_deploy_marker` | broad `IsUndeploying` unless separately proven | This slice only proves it is set after null destination clears the bidirectional `+0x2B0/+0x2AC` link while `+0x6AD` is set. |
| `Foot+0x2B0` | owner-side linked building/object pointer | `+0x2AC` in the null cleanup condition | `0x004D9522` reads owner `+0x2B0`; `0x004D952C` clears linked object's `+0x2AC`; `0x004D9532` clears owner `+0x2B0`. |
| `Foot+0x2AC` | owner-side locomotor/deploy target pointer for non-null helper; linked-object back-reference cleared by null cleanup | using one name for both owner and linked-object role without context | The same offset is used on different objects in a bidirectional link. The guard block proves direction by registers, not by name. |

## 5. Integration And YR Activity

- `TechnoClass::Set_Destination @ 0x00741970` is active in standard YR destination preprocessing and tail-calls/direct-calls `0x004D94B0`; the chrono/teleport preprocessing branch writes `Foot+0x6AC = 1` at `0x007425BF` after virtual calls `+0x1F0` and `+0x1E8(7,0)`. Active in YR: Conditional, for units using the chrono/teleporter branch.
- `TechnoClass::PerformDeploy @ 0x00710000` writes runtime Foot `+0x6AD = 1` at `0x00710352` after linked-object setup and vtable `+0x480(piVar1,1)` in the deploy/piggyback infrastructure. Active in YR: Conditional, for active deploy/locomotor-piggyback paths.
- `BuildingClass::DeployUnit_ChronoWarp @ 0x0070FEE0` consumes owner `+0x2AC`, clears linked `+0x2B0` at entry, can call linked object `+0x480(0,1)`, sets airborne recovery bytes on the linked object, and clears owner `+0x2AC`. Active in YR: Conditional, reached by the guarded helper call at `0x004D9509` and deploy cleanup infrastructure.
- `FootClass` constructor initializes `+0x6AC`, `+0x6AD`, and `+0x6AE` to 0 (`0x004D340E`, `0x004D3414`, `0x004D341A`). Active in YR: Yes for runtime Foot object construction.

## 6. Current Rust Implementation Status

Rust already has a partial gamemd-shaped owner destination:

- `src/sim/components.rs`: `NavigationState { nav_com_aux, nav_com, suspended_nav_com, nav_queue }`.
- `src/sim/game_entity.rs`: `GameEntity.navigation` is present and serialized.
- `src/sim/movement/navcom.rs`: `set_destination_internal_cell` clears `nav_com_aux`, writes `nav_com`, and sets Drive destination; `set_destination_internal_null` clears `nav_com` and Drive destination.
- `src/sim/movement/movement_commands.rs`: `can_accept_destination` rejects a broad local deploy state before pathfinding; normal Drive issue calls `navcom::set_destination_internal_cell`, but this helper does not yet model `+0x6AC`, `+0x6AD`, `+0x6AE`, `+0x2B0/+0x2AC`, or the precise "clear aux before silent drop" behavior.
- `src/sim/deploy.rs`: documentation says any deploy-state variant gates Set_Destination; this is broader than the scoped binary evidence for runtime `+0x6AD`, and should not be treated as proof that all local `DeployPhase` variants map byte-perfectly to `Foot+0x6AD`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x004D94B0` guard order | verified | decompile + assembly contexts listed in §3 | none for this slice |
| `+0x6AC` one-shot skip | verified | `0x004D9607..0x004D9618`; writer `0x007425BF` | global writer list not exhaustive |
| `+0x6AD` non-null silent drop/null cleanup gate | verified | `0x004D94BE..0x004D94D1`, `0x004D9518..0x004D9538` | exact full lifecycle/clear point deferred |
| `+0x6AE` scoped write | verified | `0x004D9538` | other readers/writers deferred |
| `+0x2B0/+0x2AC` null cleanup direction | verified | `0x004D9522..0x004D9532`; `0x0070FEE0` support | exact object type names outside this link deferred |
| Rust NavCom surface | verified from source scan | `components.rs`, `game_entity.rs`, `movement/navcom.rs`, `movement_commands.rs`, `deploy.rs` | implementation parity unchecked |
| Full deploy state machine | deferred | out of scope | separate deploy/piggyback lifecycle investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does `+0x5A0` clear before the non-null silent-drop guards? -> Yes; it clears after loading `+0x6AD` and before the first conditional return.` (evidence: `0x004D94BE..0x004D94D1`)
- `[RESOLVED] OQ-02 - Does `+0x6AD` silently drop non-null targets before NavCom write? -> Yes.` (evidence: `0x004D94BE..0x004D94D1`; NavCom write at `0x004D9510`)
- `[RESOLVED] OQ-03 - Is the null cleanup condition owner `+0x2AC` or owner `+0x2B0`? -> owner `+0x2B0`; linked object's `+0x2AC` is cleared.` (evidence: `0x004D9522..0x004D9532`)
- `[RESOLVED] OQ-04 - Does null cleanup set `+0x6AE`? -> Yes, after both linked-object and owner link clears.` (evidence: `0x004D9538`)
- `[RESOLVED] OQ-05 - Does `+0x6AC` mean generic deploying? -> No; in this function it is a one-shot skip of `Head_To_Coord`.` (evidence: `0x004D9607..0x004D9618`)
- `[RESOLVED] OQ-06 - Is runtime `+0x6AD` default zero? -> Yes, constructor writes zero using `BL` after `XOR EBX, EBX`.` (evidence: `0x004D3414`)
- `[RESOLVED] OQ-07 - Is runtime `+0x6AD` written by active deploy/piggyback code? -> Yes, `TechnoClass::PerformDeploy` writes it to 1.` (evidence: `0x00710352`)
- `[DEFERRED] OQ-08 - What exact code clears runtime `+0x6AD` after all deploy/piggyback paths?` (category: requires-different-system-context; reason: broad lifecycle scan outside this guard reconciliation; next-step-if-pursued: full writer/read xref audit for `Foot+0x6AD`)
- `[DEFERRED] OQ-09 - Do all Rust `DeployPhase` variants map to `Foot+0x6AD`?` (category: requires-different-system-context; reason: Rust deploy state and native deploy/piggyback byte lifecycle are broader than this slot; next-step-if-pursued: compare `src/sim/deploy.rs` against `TechnoClass::PerformDeploy` and clear paths)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Non-null guarded calls clear `NavCom_Aux` first, then silently return before `NavCom`/Drive destination write when `+0x6AD`, `+0x82`, or `+0x2E4` is set. | `0x004D94C7`, `0x004D94BE..0x004D94F3` | partial/mismatch: `can_accept_destination` returns before `navcom.rs` clears aux, and guards are broad local states rather than exact bytes | `src/sim/movement/movement_commands.rs`, `src/sim/movement/navcom.rs`, `src/sim/components.rs` | Route destination attempts through a Rust Set_Destination_Internal-equivalent that performs aux-clear and exact no-write/no-Drive-dispatch semantics for accepted guard fields | With stale `nav_com_aux` set and deploy/piggyback guard active, a move order clears aux but preserves old `nav_com` and Drive destination | Risk: dropping the command before the helper leaves stale aux or clears the wrong owner state |
| `+0x6AC` is one-shot: accepted non-null destination writes NavCom, clears `+0x6AC`, and skips `Head_To_Coord`/Drive destination dispatch exactly once. | `0x004D9607..0x004D9618`; writer `0x007425BF` | missing: Rust `navcom::set_destination_internal_cell` always calls Drive destination for Drive locomotor | `src/sim/movement/navcom.rs`, teleporter/chrono branch in `movement_commands.rs` | Add a separate deferred-head-to flag/state; when set, write owner NavCom but do not update Drive destination until the later native-equivalent path | Chrono/teleporter branch sets deferred flag, next destination call records NavCom and clears flag while Drive destination remains unchanged for that call | Risk: modeling this as "reject destination" loses NavCom and action-line endpoint |
| Null destination with `+0x6AD != 0 && +0x2B0 != 0` clears linked object's `+0x2AC`, clears owner `+0x2B0`, then sets owner `+0x6AE = 1`. | `0x004D9518..0x004D9538`; support `BuildingClass::DeployUnit_ChronoWarp @ 0x0070FEE0` | missing: Rust has no explicit linked deploy/piggyback object pair or `+0x6AE` marker | deploy/locomotor-piggyback state surfaces, `GameEntity`, future linked-object cleanup helper | Represent bidirectional deploy/piggyback link direction and perform cleanup during null destination in the binary order | Nulling destination during active linked deploy clears both sides of the link and records the post-cleanup marker before Drive stop | Risk: clearing only the owner's link leaves the linked building/unit with a stale back-reference |

Proposed test names:

- `test_set_destination_guard_clears_navcom_aux_but_silently_preserves_navcom`
- `test_skip_head_to_coord_once_writes_navcom_without_drive_destination`
- `test_null_destination_deploy_link_cleanup_clears_bidirectional_links`

## 10. Negative Facts / Do Not Do

- Do not treat `+0x6AC` as a generic `IsDeploying` flag. Evidence: `0x004D9607..0x004D9618` only proves a one-shot skip of `Head_To_Coord`, and `0x007425BF` writes it in a chrono/teleport preprocessing branch.
- Do not describe the null cleanup as checking owner `+0x2AC`. Evidence: `0x004D9522` reads owner `+0x2B0`; `0x004D952C` clears linked object's `+0x2AC`; `0x004D9532` clears owner `+0x2B0`.
- Do not clear or overwrite NavCom on non-null silent drops. Evidence: all three non-null guard returns branch to the epilogue before `0x004D9510`.
- Do not skip the `+0x5A0` clear just because the destination is rejected. Evidence: `0x004D94C7` precedes first guard return at `0x004D94D1`.
- Do not merge runtime Foot `+0x6AD` with `TechnoTypeClass+0x6AD` or Rust's whole `DeployPhase` enum without a separate lifecycle proof. Evidence: runtime constructor/write addresses are `0x004D3414` and `0x00710352`; same displacement in type layout is a different object.

## Stale Docs / Follow-up Docs

- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4 step 4 replacement: "If `target == NULL && Foot+0x6AD != 0 && Foot+0x2B0 != 0`, clear `(*(Foot+0x2B0))+0x2AC`, clear `Foot+0x2B0`, then set `Foot+0x6AE = 1`. This is distinct from the non-null owner `+0x2AC` helper path."
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4 `+0x6AC` replacement: "`Foot+0x6AC` is a one-shot `skip_head_to_coord_once` byte: when set, `Set_Destination_Internal` clears it and skips target coord fetch plus locomotor vtable `+0x44`; it does not reject or delay the NavCom write."
- `docs/research/FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` field table replacement for `+0x6AD`: "Runtime Foot `+0x6AD` is a deploy/locomotor-piggyback active guard: constructor clears it, `TechnoClass::PerformDeploy @ 0x00710352` sets it, `Set_Destination_Internal` silently rejects non-null destinations while set, and null destination uses it to drive linked-object cleanup. Do not use the name `IsDeploying` without this nuance."
- `docs/research/TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` row `0x6AD` replacement: "Runtime Foot `+0x6AD` is better named `deploy_or_locomotor_piggyback_active`; it is not the same as `TechnoTypeClass+0x6AD`, and exact clear paths require a separate lifecycle audit."
- `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md` row `+0x6AD` replacement: "Runtime Foot `+0x6AD` participates in deploy/locomotor-piggyback active state and blocks non-null Set_Destination_Internal calls; 'deploy complete' is too narrow for movement parity."

## Sources

- Ghidra read-only decompile: `0x004D94B0`, `0x00741970`, `0x00710000`, `0x0070FEE0`, `0x004D8F40`, `0x004D8F80`.
- Ghidra read-only assembly context: `0x004D94BE`, `0x004D94C7`, `0x004D94D7`, `0x004D94E9`, `0x004D94F9`, `0x004D9505`, `0x004D9510`, `0x004D9518`, `0x004D9522`, `0x004D9538`, `0x004D9607`, `0x004D96C2`, `0x007425BF`, `0x00710352`, `0x004D3414`.
- Ghidra read-only raw bytes: `read_memory 0x004D94B0 length 560`, `read_memory 0x007425A0 length 80`, `read_memory 0x00710320 length 96`, `read_memory 0x004D31E0 length 640`.
- Prior docs checked: `docs/research/FOOTCLASS_SET_DESTINATION_INTERNAL_NAVCOM_HEADTO_HANDOFF_GHIDRA_REPORT.md`, `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`, `docs/research/FOOTCLASS_COMPLETE_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`, `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md`, `docs/research/bridges/04-locomotion-height-tubes/BRIDGE_JUMPJET_ABORT_FLAG_WRITERS_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/components.rs`, `src/sim/game_entity.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/deploy.rs`.
