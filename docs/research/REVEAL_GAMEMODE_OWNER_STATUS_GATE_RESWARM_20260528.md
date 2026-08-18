# Reveal GameMode / UniqueID Registration Gate - Reswarm 2026-05-28

**Address(es):** `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectClass::Conceal @ 0x005F4D30`, `AbstractClass::IRTTITypeInfo::GetID @ 0x00410220`, `Main_Game @ 0x0052D9A0`, `ScenarioClass::Full_Init @ 0x00686B20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact semantic names and active-YR meanings of the `ObjectClass::Reveal` / `ObjectClass::Conceal` logic-registration gate involving `g_GameMode == 0`, `g_GameMode == 5`, and secondary vtable `this+4` slot `+0x10` returning `-2`.
**Non-Scope:** whole-`Reveal` ordering, `FUN_0055BAA0`/`FUN_0055BAE0` mechanics, full game-mode enum audit beyond values needed by this gate, and full inventory of all writers that can create `UniqueID == -2`.
**Confidence:** High for branch semantics, game-mode meanings for `0` and `5`, and the secondary-vtable method identity; Medium for the broader class set that can hold `UniqueID == -2`.
**Active in YR:** Yes / Conditional. The gate is active in standard YR object reveal/conceal. `g_GameMode == 0` and `g_GameMode == 5` are live standard paths; the `UniqueID == -2` skip is conditional on non-0/non-5 modes.

## 0. Working Notes

**Target question:** Resolve the semantic names and active-YR meanings of `g_GameMode == 0`, `g_GameMode == 5`, and secondary vtable `this+4` slot `+0x10` returning `-2` in the `ObjectClass::Reveal @ 0x005F4EC0` logic-registration gate.

**Non-goals:** Do not re-investigate the whole `Reveal` function, do not re-prove the logic helper/vector scheduler, do not edit Rust, and do not audit all `g_GameMode` values or all possible `UniqueID == -2` producers.

**Evidence needed to mark COMPLETE:** decompile plus assembly context for the `Reveal`/`Conceal` gate, direct decompile/assembly/xref evidence identifying the secondary vtable slot, and binary route/initializer evidence naming `g_GameMode == 0` and `g_GameMode == 5` in active YR.

**Stop conditions:** Stop once the three gate terms are named or explicitly bounded, the branch skip semantics are stated for Reveal and Conceal, and Rust-facing handoff implications are recorded.

## 1. Overview

The `Reveal`/`Conceal` gate is not checking owner status. The secondary call is `IRTTITypeInfo::GetID(this+4)`, and the compared value is `AbstractClass+0x10 UniqueID`.

The branch means: campaign/single-player (`g_GameMode == 0`) and offline Skirmish (`g_GameMode == 5`) bypass the `UniqueID == -2` exclusion and register/unregister eligible logic objects. Other modes call `GetID`; when the object's UniqueID is `-2`, `Reveal` skips `FUN_0055BAA0` and `Conceal` skips `FUN_0055BAE0`.

## 2. Key Names / Offsets

| Item | Exact meaning | Evidence | Active in YR |
|---|---|---|---|
| `g_GameMode @ 0x00A8B238` | session/game-mode integer | direct xrefs; `Main_Game`; `ScenarioClass::Full_Init` | Yes |
| `g_GameMode == 0` | campaign / single-player scenario mode | `ScenarioClass::Full_Init` reads campaign/scenario mission data only in mode 0; `Main_Game` campaign case sets mode 0 | Yes |
| `g_GameMode == 5` | offline Skirmish setup/play mode | `Main_Game` route `0x0B` writes `g_GameMode=5`; `g_GameMode==5` calls `FUN_006AE2C0` offline setup dialog; MPModes combo population gates on mode 5 | Yes |
| `this+4` secondary vtable | `IRTTITypeInfo` subobject inherited from `AbstractClass` | `ObjectClass` layout; vtable data xrefs to `0x00410220` | Yes |
| secondary vtable `+0x10` | `IRTTITypeInfo::GetID` | decompile `0x00410220`; vtable xrefs | Yes |
| `GetID` return value | `AbstractClass+0x10 UniqueID`, read as `*(this+4+0x0C)` | decompile `0x00410220`; constructor `0x00410170`; assigner `0x00410230` | Yes |
| `-2` compared at the gate | special UniqueID sentinel value; not owner/house status | branch assembly `0x005F5030..0x005F5036` and `0x005F4DC5..0x005F4DCB`; Anim destructor also branches on `GetID()==-2` | Conditional |

## 3. Core Logic

### 3.1 Reveal registration gate

Inside the already-verified successful/alive/type-eligible branch, `ObjectClass::Reveal` uses this logic:

```text
if type.logic_enabled:
    if game_mode == 0 or game_mode == 5:
        register_logic_object()
    else:
        if this.IRTTITypeInfo.GetID() != -2:
            register_logic_object()
```

Material details:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `g_GameMode` is read once at `0x005F501B`; `0` jumps directly to registration. | assembly context `0x005F501B..0x005F5022` | High | Yes |
| `g_GameMode == 5` also jumps directly to registration. | assembly context `0x005F5024..0x005F5027` | High | Yes |
| Only when mode is neither `0` nor `5` does `Reveal` call secondary vtable `this+4 +0x10`. | assembly `0x005F5029..0x005F5030`; decompile `ObjectClass::Reveal` | High | Conditional |
| Return `-2` skips `FUN_0055BAA0`; any other value reaches `FUN_0055BAA0(param_1, 0)`. | assembly `0x005F5033..0x005F5040` | High | Conditional |
| The secondary call is `AbstractClass::IRTTITypeInfo::GetID`, not a house owner query. | `0x00410220` returns `*(param_1+0x0C)`; object layout places `IRTTITypeInfo` at `+4` and `UniqueID` at object `+0x10` | High | Yes |

### 3.2 Conceal unregistration gate

`ObjectClass::Conceal` mirrors the same mode/sentinel test for removal:

```text
if type.logic_enabled:
    if game_mode != 0 and game_mode != 5:
        if this.IRTTITypeInfo.GetID() == -2:
            skip_unregister()
    unregister_logic_object()
```

Evidence:

- `0x005F4DB0` reads `g_GameMode`.
- `0x005F4DB5` tests mode `0`; `0x005F4DB9` tests mode `5`.
- `0x005F4DBE..0x005F4DC5` calls secondary vtable `+0x10`.
- `0x005F4DC8..0x005F4DCB` skips `FUN_0055BAE0` only on `-2`.
- `0x005F4DCD..0x005F4DD3` calls `FUN_0055BAE0`.

Active in YR: Conditional. `Conceal` is active generally; the `-2` skip only matters outside campaign/offline Skirmish and only for objects whose `UniqueID` is exactly `-2`.

### 3.3 `g_GameMode == 0`

`g_GameMode == 0` is the campaign/single-player branch for this gate, not standard offline Skirmish.

Evidence:

- `Main_Game` campaign-selection cases set `g_GameMode = 0` after scenario selection and write campaign-side fields before entering gameplay.
- `ScenarioClass::Full_Init @ 0x00686B20` snapshots `g_GameMode == 0` at entry and takes campaign-specific paths: reads `MISSIONMD.INI`, reads `[Basic] Player`, applies campaign side/mix selection, and skips the non-campaign selected-MPModes setup branch.
- Assembly contexts `0x00686B29..0x00686B3D`, `0x00686B6A..0x00686BB6`, `0x006873C8..0x00687581`, and `0x00687868..0x00687932` show the mode-0 splits.

Active in YR: Yes. This is the standard YR campaign/single-player scenario path.

### 3.4 `g_GameMode == 5`

`g_GameMode == 5` is standard offline Skirmish setup/play in the active shell route inspected here, not replay playback.

Evidence:

- `Main_Game @ 0x0052D9A0`, route case `0x0B`, writes `g_GameMode = 5`; assembly at `0x0052E10F`.
- The same `Main_Game` switch reaches case `g_GameMode == 5` and calls `FUN_006AE2C0`.
- `FUN_006AE2C0` creates the offline Skirmish setup dialog path and returns true only on Start button `0x617`; false/back resets mode in `Main_Game`.
- Existing MPModes reports independently verify offline Skirmish mode combo population uses `g_GameMode == 5` at `0x005D6130`.

Active in YR: Yes. This is the standard offline Skirmish path.

## 4. Branch Semantics

| Mode / ID state | Reveal behavior | Conceal behavior | Active in YR |
|---|---|---|---|
| `g_GameMode == 0` | bypass `GetID`; register if prior alive/type gates pass | bypass `GetID`; unregister if type gate passes | Yes, campaign/single-player |
| `g_GameMode == 5` | bypass `GetID`; register if prior alive/type gates pass | bypass `GetID`; unregister if type gate passes | Yes, offline Skirmish |
| `g_GameMode != 0 && != 5`, `UniqueID != -2` | register | unregister | Conditional, network/other nonlocal modes |
| `g_GameMode != 0 && != 5`, `UniqueID == -2` | skip registration | skip unregistration | Conditional; exact producer inventory deferred |

Important ordering: the `UniqueID == -2` check is inside the existing type-level `ObjectType+0x234` gate and after the already-settled alive/display ordering in `Reveal`. It is not a replacement for `Object+0x98`, `IsAlive`, `InLimbo`, or type logic eligibility.

## 5. Current Rust Implementation Status

Static scan only; no Rust was modified.

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs` | `live_object_order: Vec<u64>`, `register_live_object`, `unregister_live_object`, `live_object_order_snapshot` | No explicit native `ObjectType+0x234`, `g_GameMode`, `UniqueID == -2`, or `Object+0x98` membership gate |
| `src/sim/world/world_spawn.rs` | spawn paths call `register_live_object` directly after inserting entities | Registration can happen without the native reveal registration gate |
| `src/sim/game_entity.rs` / `src/sim/components.rs` | Rust stable IDs and owners exist as gameplay data | No separate `AbstractClass.UniqueID == -2` sentinel semantics found in this slice |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Reveal` game-mode/ID branch | verified | decompile `0x005F4EC0`; assembly `0x005F501B..0x005F5040` | none |
| `Conceal` matching branch | verified | decompile `0x005F4D30`; assembly `0x005F4DB0..0x005F4DD3` | none |
| secondary vtable `+0x10` identity | verified | `0x00410220`, vtable layout docs/xrefs | none |
| `UniqueID` initialization and normal assignment | verified | `0x00410170`, `0x00410230`, `0x0068BCB0` | none for this gate |
| `g_GameMode == 0` meaning | verified | `Main_Game`; `ScenarioClass::Full_Init` mode-0 branches | none |
| `g_GameMode == 5` meaning | verified | `Main_Game` route `0x0B`; `FUN_006AE2C0`; MPModes gate docs | none |
| all producers of `UniqueID == -2` | touched-not-exhausted | `AnimClass` destructor shows `GetID()==-2` is a live branch; direct producer not exhausted | separate UniqueID sentinel/source inventory |
| current Rust gate equivalence | touched-not-exhausted | static scan of `world/mod.rs`, `world_spawn.rs` | implementation design/tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-RGOS-001 - What is `this+4`? -> AbstractClass's `IRTTITypeInfo` secondary subobject.` (evidence: `OBJECTCLASS_GHIDRA_REPORT.md`; `ABSTRACTCLASS_GHIDRA_REPORT.md`; vtable xrefs)
- `[RESOLVED] OQ-RGOS-002 - What is secondary vtable `+0x10`? -> `IRTTITypeInfo::GetID @ 0x00410220`.` (evidence: decompile `0x00410220`; vtable data xrefs)
- `[RESOLVED] OQ-RGOS-003 - What does `GetID` return? -> object `AbstractClass+0x10 UniqueID`.` (evidence: `0x00410220` reads `param_1+0x0C` where `param_1=this+4`; constructors initialize/assign `+0x10`)
- `[RESOLVED] OQ-RGOS-004 - Is this an owner/house status check? -> No; no owner pointer or HouseClass method is reached by this call.` (evidence: `0x00410220`; `0x005F5029..0x005F5036`)
- `[RESOLVED] OQ-RGOS-005 - What does `g_GameMode==0` mean here? -> campaign/single-player scenario mode.` (evidence: `Main_Game`; `ScenarioClass::Full_Init`)
- `[RESOLVED] OQ-RGOS-006 - What does `g_GameMode==5` mean here? -> offline Skirmish setup/play mode.` (evidence: `Main_Game` route `0x0B`, `FUN_006AE2C0`, MPModes population)
- `[RESOLVED] OQ-RGOS-007 - What does Reveal do when mode is 0 or 5? -> It bypasses the UniqueID check and registers when prior gates pass.` (evidence: `0x005F501B..0x005F5040`)
- `[RESOLVED] OQ-RGOS-008 - What does Reveal do when mode is not 0/5 and ID is -2? -> It skips `FUN_0055BAA0`.` (evidence: `0x005F5030..0x005F5036`)
- `[RESOLVED] OQ-RGOS-009 - Does Conceal mirror the branch? -> Yes; mode 0/5 bypass, otherwise ID `-2` skips `FUN_0055BAE0`.` (evidence: `0x005F4DB0..0x005F4DD3`)
- `[RESOLVED] OQ-RGOS-010 - Is the branch active in standard YR code paths? -> Yes; Reveal/Conceal are active, and modes 0/5 are standard YR campaign/skirmish paths.` (evidence: cited route and lifecycle docs)
- `[DEFERRED] OQ-RGOS-011 - Which exact constructors/runtime paths can assign `UniqueID == -2`?` (category: `requires-different-system-context`; reason: this gate only requires resolving the method and branch semantics; a full sentinel-source inventory spans constructors/load/destructor and global abstract directories; next-step-if-pursued: trace writes to `AbstractClass+0x10` and all `GetID()==-2` consumers.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Reveal registers logic-eligible objects in campaign (`0`) and offline Skirmish (`5`) without checking `UniqueID == -2`. | `0x005F501B..0x005F5040`; `Main_Game`; `ScenarioClass::Full_Init`; `FUN_006AE2C0` | Missing: direct spawn registration has no native reveal gate or game-mode semantics | `src/sim/world/mod.rs::register_live_object`; `src/sim/world/world_spawn.rs` | Future reveal API must treat campaign and offline Skirmish as bypass modes for the UniqueID sentinel after alive/type gates pass. | In offline Skirmish mode, reveal a logic-enabled object with simulated native UniqueID sentinel `-2`; assert it still enters live order. | `reveal_skirmish_bypasses_unique_id_minus_two_logic_gate` | Do not copy stale docs that call mode 5 replay or mode 0 skirmish for this gate. |
| In modes other than `0`/`5`, Reveal skips `FUN_0055BAA0` when `IRTTITypeInfo::GetID()` returns `-2`; Conceal skips `FUN_0055BAE0` under the same condition. | `0x005F5030..0x005F5036`; `0x005F4DC5..0x005F4DCB`; `0x00410220` | Missing: no separate `AbstractClass.UniqueID` sentinel or object-local logic membership gate | future native lifecycle/session-mode model | Add the mode-sensitive sentinel check around registration and unregistration, separate from owner/house data. | In a nonlocal/network-mode simulation, reveal/conceal a logic-enabled object with native UniqueID sentinel `-2`; assert live order and membership byte/list are not changed by the reveal/conceal gate. | `reveal_network_unique_id_minus_two_skips_logic_membership` | Do not implement this as an Owner/House status check. |
| The branch is inside `ObjectType+0x234` and existing alive/reveal success gates; it is not a general scheduler filter. | `ObjectClass::Reveal` decompile and assembly `0x005F4FEF..0x005F5040`; parent ordering report | Rust registers from spawn paths without expressing native reveal success/type gate | `src/sim/world/world_spawn.rs`; future `reveal_object` lifecycle surface | Registration should occur only after native reveal success, alive/type gates, and mode/UniqueID gate all pass. | Spawn a stored/limbo object, fail reveal placement, then attempt reveal in campaign/skirmish/network variants; assert only successful eligible reveals alter live order. | `reveal_registration_gate_runs_after_success_alive_and_type_checks` | Do not let `EntityStore` insertion imply LogicClass membership. |

## 9. Negative Facts / Do Not Do

- Do not name the secondary `this+4 +0x10` call as owner status. Active in YR: Yes; evidence `0x00410220` reads UniqueID, not owner/HouseClass state.
- Do not treat `g_GameMode == 5` as replay for this gate. Active in YR: Yes; evidence `Main_Game` writes `5` before `FUN_006AE2C0`, and MPModes population gates offline Skirmish on `5`.
- Do not treat `g_GameMode == 0` as standard offline Skirmish. Active in YR: Yes; evidence `ScenarioClass::Full_Init` mode-0 campaign branches and mode-5 offline Skirmish route.
- Do not apply the `UniqueID == -2` skip in campaign or offline Skirmish; native bypasses the check in both modes. Active in YR: Yes; evidence `0x005F5020..0x005F5027`.
- Do not collapse `UniqueID`, Rust stable ID, owner house, and `Object+0x98` into one field. Active in YR: Yes; evidence `0x00410220`, `0x0055BAA0`, and ObjectClass layout show distinct fields.

## 10. Remaining Uncertainty

- Exact producer inventory for `AbstractClass.UniqueID == -2` remains open. This report proves the `GetID` identity and branch effect, and finds another active consumer in `AnimClass` destructor, but does not map all assignments/writers.
- Exact behavior of this gate in every non-0/non-5 network/session variant is not runtime-sampled. Static branch semantics are verified; frequency and player-visible cases require a separate mode/session trace.

## 11. Stale Docs / Replacement Wording

- `docs/research/timing/logic-vs-render-loop.md`: replace `g_GameMode == 5 (replay)` with `g_GameMode == 5 (offline Skirmish in the active Main_Game route; do not use this value as replay without a separate replay-specific proof)`.
- `docs/research/timing/multiplayer-frame-step.md`: replace `In g_GameMode == 0 (skirmish)` with `In g_GameMode == 0 (campaign/single-player scenario mode; offline Skirmish is g_GameMode == 5 in Main_Game)`.
- `docs/research/ADDRESS_MAP.md`: replace `GameMode (0=SP,1=Skirm,2=LAN,3=WOL,4=TCP)` with `GameMode (0=campaign/single-player; 5=offline Skirmish in active YR Main_Game route; 3/4 are network branches; values 1/2 need separate confirmation before naming)`.
- `docs/research/OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`: replace OQ-REVEAL-016 with `RESOLVED by REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md: the gate checks g_GameMode==0 campaign/single-player and g_GameMode==5 offline Skirmish as bypass modes; otherwise it calls AbstractClass::IRTTITypeInfo::GetID(this+4), compares AbstractClass+0x10 UniqueID to -2, and skips logic register/unregister only when the UniqueID is -2. This is not an owner/House status check.`

## Sources

- Ghidra read-only decompile/assembly:
  - `ObjectClass::Reveal @ 0x005F4EC0`, assembly `0x005F501B..0x005F5040`
  - `ObjectClass::Conceal @ 0x005F4D30`, assembly `0x005F4DB0..0x005F4DD3`
  - `AbstractClass::IRTTITypeInfo::GetID @ 0x00410220`
  - `AbstractClass::Constructor_Full @ 0x00410170`
  - `AbstractClass::AssignUniqueID @ 0x00410230`
  - `Heap::GetNextID-ish @ 0x0068BCB0`
  - `Main_Game @ 0x0052D9A0`, route/write context `0x0052E10F`
  - `FUN_006AE2C0` offline Skirmish setup launcher
  - `ScenarioClass::Full_Init @ 0x00686B20`
  - `AnimClass::Destructor @ 0x004228E0` as corroborating `GetID()==-2` consumer
- Prior docs referenced:
  - `docs/research/OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
  - `docs/research/POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`
  - `docs/research/ABSTRACTCLASS_GHIDRA_REPORT.md`
  - `docs/research/OBJECTCLASS_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_NATIVE_SINGLE_PLAYER_ROUTE_TO_0X102_RECHECK_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`
- Rust source scanned read-only:
  - `src/sim/world/mod.rs`
  - `src/sim/world/world_spawn.rs`
  - `src/sim/game_entity.rs`
  - `src/sim/components.rs`
