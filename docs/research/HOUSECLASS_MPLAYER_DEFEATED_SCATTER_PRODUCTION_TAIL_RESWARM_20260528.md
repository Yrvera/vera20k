# HouseClass MPlayer Defeated / Scatter / Production Tail - Re-Swarm Report

> **SUPERSEDED FOR `0x004FC6D0` (2026-08-29).** The report's movement-Scatter
> interpretation, coordinate labels, and proposed mission handoff are wrong.
> Active assembly proves a mutable live-Techno destruction loop using current
> health, incoming Temporal detach, Rules C4, and concrete vtable `+0x16C`
> `ReceiveDamage`. Use
> `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`
> for this callee and its callers. Factory/House ordering claims outside that
> callee remain historical evidence.

**Date:** 2026-05-28  
**Target:** `HOUSECLASS_MPLAYER_DEFEATED_SCATTER_PRODUCTION_TAIL`  
**Investigation Mode:** coverage-map  
**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, `HouseClass::Update @ 0x004F8440`, `HouseClass::MPlayer_Defeated @ 0x004FC0B0`, `HouseClass::ScatterAllUnits @ 0x004FC6D0`, `HouseClass::AI_ManageProduction @ 0x0050AF10`, `HouseClass::AI_ResumeProduction @ 0x0050B1D0`, `SuperClass::AI_Ready @ 0x006CBCA0`  
**Confidence:** High for scoped ordering, defeat/scatter entry effects, and superweapon manage/resume split points; Partial for full AI production formulas.  
**Active in YR:** Yes. These functions are reached from the active `LogicClass::PerTickUpdate` house array loop in standard YR gameplay; multiplayer defeat is conditional on `g_GameMode != 0`.

## Working Notes

Target question: Prove HouseClass tail-side effects relevant to PerTick factory->house order: `MPlayer_Defeated`, scatter side effects, and AI production management/resume gates inside `HouseClass::Update`; identify Rust-facing split points.
Non-goals: Full HouseClass AI chooser formulas, exact FactoryClass global array insertion order, sidebar/UI pixel rendering, and Rust implementation.
Evidence needed to mark COMPLETE: Decompile plus assembly/caller evidence for `PerTickUpdate -> HouseClass::Update` ordering, scoped `HouseClass::Update` callees, `MPlayer_Defeated`, scatter, manage/resume, plus current Rust surface scan.
Stop conditions: Stop once scoped side effects/order/split points are resolved or explicitly deferred; write only this report plus the shared claims file.

## 1. Overview

Active YR ticks all factories before any house update, then each house runs a dense tail in `HouseClass::Update`. The scoped tail order is: early per-house radar/superweapon/low-power checks, superweapon `AI_Ready` loop, multiplayer defeat detection, screen flash/cell action, periodic AI chooser, then a `House+0x1FC`-gated manage/resume pair.

The important correction for implementation handoff is that `0x0050AF10` / `0x0050B1D0` are not the global factory production step. In this slice they operate over the House superweapon DVC at `+0x258/+0x264`, suspend/deactivate/resume/add cameos, and can re-set `House+0x1FC`; the global factory loop has already happened before the house loop.

## 2. Key Offsets

| Offset | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `House+0x1F5` | defeated byte | `MPlayer_Defeated` writes `1` at function entry; `Update` reads before defeat check | Yes |
| `House+0x1FC` | dirty/manage-resume gate | `Update @ 0x004F926C..0x004F92FD`; `AI_ManageProduction` can write it again at `0x0050B1A5` | Yes |
| `House+0x258/+0x264` | `SuperClass*` array/count | `Update` loop `0x004F8E34..0x004F8E84`; manage/resume loops `0x0050AF47`, `0x0050B1D7` | Yes |
| `House+0x2F0`, `+0x5564`, `+0x5578`, `+0x558C` | defeat-count inputs for buildings/infantry/vehicles/aircraft | `Update` decompile defeat block and field map | Yes, multiplayer only |
| `HouseType+0x1A6` | `MultiplayPassive` exemption | `Update @ 0x004F8E86` branch and `MPlayer_Defeated` alive-house scans | Yes |
| `Rules+0xFA8` | scatter argument passed to vtable `+0x16C` | `ScatterAllUnits @ 0x004FC747..0x004FC766` | Yes |

## 3. Core Logic

### PerTick owner order

`LogicClass::PerTickUpdate @ 0x0055AFB0` runs tactical, then all factories, then all houses. The critical assembly is:

```text
0055b667: CALL dword ptr [EAX + 0x5c]        ; Tactical
0055b66a..0055b68b                          ; FactoryClass array loop
0055b680: CALL dword ptr [EDX + 0x5c]
0055b68d..0055b6b1                          ; HouseClass array loop
0055b6a6: CALL dword ptr [EDX + 0x5c]
```

Active in YR: Yes. This is the normal `PerTickUpdate` ladder; no TS-only flag gates the factory/house tail.

### HouseClass::Update scoped order

`HouseClass::Update @ 0x004F8440` establishes this local order:

1. `RecheckPower` can call `AI_AssessPower`, then forces `RecheckRadar`.
2. `RecheckRadar` calls `CheckSuperweaponReady` unless `DAT_00A8B538 != 0`, then always calls `CheckLowPower` (`0x004F84FB..0x004F850C`).
3. Superweapon loop calls `SuperClass::AI_Ready(sw, this == g_PlayerPtr)` for every `House+0x258/+0x264` entry (`0x004F8E34..0x004F8E84`).
4. Multiplayer defeat block runs only when `g_GameMode != 0`, `House+0x1F5 == 0`, `g_CurrentFrameCounter > 0`, and `HouseType+0x1A6 == 0`; if the count reaches zero it calls `ScatterAllUnits` then `MPlayer_Defeated` in that order (`0x004F8E86..0x004F8F82`).
5. AI chooser branches run later on `g_CurrentFrameCounter & 7 == 0` for non-current/non-passive houses (`0x004F8FE1..0x004F9265`).
6. If `House+0x1FC != 0`, `Update` clears it, optionally refreshes current-player objects/sidebar, then calls `AI_ManageProduction` and `AI_ResumeProduction` (`0x004F9265..0x004F92FD`).

Active in YR: Yes for normal house update; defeat subsection is Conditional: multiplayer/skirmish only (`g_GameMode != 0`), not already defeated, frame > 0, non-passive house.

### MPlayer_Defeated side effects

`HouseClass::MPlayer_Defeated @ 0x004FC0B0` immediately writes `House+0x1F5 = 1`. It then:

- conditionally recalculates alliances if IQ equals `Rules+0x1434`, the house is not human/current under the active-player test, `Rules+0x17E0 != 0`, and `g_GameMode != 0`;
- if `ScenarioFlags & 0x10`, clears rally point state when `House+0x53DC` or `House+0x53E0/0x53E2` is valid;
- if `g_GameMode != 0 && ScenarioFlags & 0x800`, iterates `UnitClass` global array and calls vtable `+0xF8` for active units owned by the defeated house;
- for local player, disables/reveals/defeat UI state, sets `DAT_00A8B538 = 1`, calls `MapClass::RevealEntireMap`, and plays/logs loss messaging unless observer;
- for non-local non-passive opponents, marks a per-house defeated-announcement byte at indexed `+0x241`, logs/plays opponent-defeated messaging unless observer;
- scans all houses twice, ignoring null/defeated/passive houses, to count alive and human houses and require bidirectional alliance bits before declaring game completion;
- sets global completion byte `DAT_00A8B8C1 = 1` when all remaining houses are mutually allied, then flags local player win/loss via `Flag_To_Win` or `Flag_To_Lose`.

Evidence: decompile `0x004FC0B0`; entry and completion assembly contexts at `0x004FC0B0`, `0x004FC591`, `0x004FC6A7`. Caller evidence: only `HouseClass::Update` calls `MPlayer_Defeated`.

Active in YR: Yes when called. Its standard `Update` caller is Conditional: multiplayer/skirmish defeat path.

### ScatterAllUnits side effects

`HouseClass::ScatterAllUnits @ 0x004FC6D0` loops the global `TechnoClass` array, not the house owned-object array. For each matching techno:

- it compares effective/current owner results from `FUN_0070F820` with the target house and has a capture-manager original-owner gate;
- it skips a duplicate pointer remembered in the local previous-scattered variable;
- it copies `Techno+0x6C` into a stack coordinate/target local;
- if `Techno+0x278 != 0`, it calls `FUN_0071AD40` before scatter;
- it calls the object's vtable `+0x16C` with stack target, `Rules+0xFA8`, and flags `0,1,1,0`;
- after a scatter call, it stores the scattered object pointer and does not increment the global array index on that iteration (`0x004FC76C..0x004FC771`); non-matching entries increment.

Evidence: decompile `0x004FC6D0`; assembly `0x004FC747..0x004FC766` shows the `Rules+0xFA8` and vtable call; `0x004FC76C..0x004FC777` shows duplicate-pointer storage and non-increment after scatter. Caller evidence: `HouseClass::Update` and `FUN_006E3180`.

Active in YR: Yes. In this report's target it is active through multiplayer defeat and delayed flag-to-win scatter paths; exact `FUN_006E3180` caller semantics are out of scope.

### Manage/resume gates are superweapon production-tail work

`HouseClass::AI_ManageProduction @ 0x0050AF10` first checks `g_GameActive`, then loops the house superweapon DVC. It only enters a superweapon row when enabled/activated-state combinations or `House+0x1F5` require management. It searches live buildings for matching superweapon slots (`Building+0x5EC` three entries and primary/secondary superweapon indices), combines building powered byte `Building+0x660` with house power ratio, applies an auxiliary type gate at `SuperWeaponType+0xE7` / `DAT_00A8B263`, then calls the type vtable `+0x40`, `SuperClass::Suspend(1/0)`, or `SuperClass::Deactivate`. On state change it may clear `DAT_008809A0`, refresh sidebar tab, and writes `House+0x1FC = 1`.

`HouseClass::AI_ResumeProduction @ 0x0050B1D0` early-outs if defeated. It loops the same superweapon DVC, searches the owned-object list in reverse for a building that grants that superweapon, rechecks the same type gate and power ratio, then calls `FUN_006CB560(0, current-player?, low-power?)`. For the current player it adds the cameo (`SidebarClass::AddCameo(0x1F, index)`), calls the type vtable `+0x40`, and refreshes the sidebar tab.

Evidence: decompile `0x0050AF10`, `0x0050B1D0`; assembly contexts `0x0050AF10`, `0x0050B10F..0x0050B14A`, `0x0050B1A5`, `0x0050B2D5..0x0050B314`, `0x0050B341..0x0050B353`; callers include `HouseClass::Update`, plus building/power paths.

Active in YR: Yes, Conditional on `g_GameActive` and `House+0x1FC` for the `Update` tail call. These are not TS-only branches.

## 4. Current Rust Status

| Surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs` | `tick_superweapons` runs around line 1427, production around 1690, AI around 1784, defeat around 1814. | DRIFT: native per-house superweapon ready/manage/resume and defeat are inside house update after all factories; Rust AI runs before defeat. |
| `src/sim/superweapon/mod.rs` | `tick_superweapons` combines charge/suspend and active Lightning Storm processing. | DRIFT: active Lightning Storm is global earlier; per-house ready/manage/resume belongs in house update tail. |
| `src/sim/production/production_queue.rs` | `tick_production_with_overlay_registry` iterates `queues_by_owner` owner/category pairs and immediately handles completion/delivery. | DRIFT/UNCHECKED: not native global `FactoryClass` array order; house tail should consume factory-visible state after all factories. |
| `src/sim/ai.rs` | simple deterministic command generator; queues production and attacks. | DRIFT for native house AI: not `HouseClass::AI_Choose_*`; must not run before native defeat gating. |
| `src/sim/world/mod.rs::check_defeat` / `src/sim/house_state.rs` | sets `is_defeated` after Rust AI phase; no native scatter/MPlayer side-effect bundle. | DRIFT: native scatter precedes defeated flag side effects and gates later AI/production tail in the same house update. |

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| All factories tick before any house update; house tail then runs per-house superweapon ready, defeat, AI chooser, and `+0x1FC` manage/resume. Active in YR: Yes. | `0x0055B667`, `0x0055B680`, `0x0055B6A6`; `0x004F8E4A`, `0x004F8F7B`, `0x004F8F82`, `0x004F92F6`, `0x004F92FD` | Rust has production, AI, defeat, and superweapons in separate staged phases. | `src/sim/world/mod.rs`, production/superweapon/AI modules | Add a native-order tail owner: factory-equivalent progress/completion before a per-house update pass. | `house_tail_factory_completion_visible_before_defeat_and_ready`: factory completion on frame N is visible to subsequent house ready/manage work on frame N. | Do not call this a broad "AI after production" phase; the per-house order matters. |
| Defeat path scatters first, then calls `MPlayer_Defeated`, before later AI chooser/manage/resume for that house. Active in YR: Conditional multiplayer. | `0x004F8E86..0x004F8F82`; caller list for `0x004FC0B0` | Rust `ai::tick_ai` can produce commands before `check_defeat`. | `src/sim/world/mod.rs::check_defeat`, future house-update pass, `src/sim/ai.rs` | Gate native AI/build/attack effects after the per-house defeat check; attach scatter and MPlayer side effects to defeat transition. | `defeated_house_cannot_queue_or_attack_same_house_tail`: house with zero qualifying objects in multiplayer is defeated before AI commands. | Do not just set `is_defeated`; native first runs scatter and then a large win/loss/UI/alliance side-effect bundle. |
| `ScatterAllUnits` is a global TechnoClass-array pass with duplicate-pointer guard and no index increment after a scatter call. Active in YR: Yes. | `0x004FC6D0`; `0x004FC747..0x004FC777` | Rust has no matching defeat scatter path. | future scatter/mission surface; `src/sim/world/mod.rs` defeat handling | Model scatter as mission/command handoff over native-equivalent active techno order, preserving no-extra-index step behavior if the list can mutate. | `defeat_scatter_global_order_no_double_scatter`: two same-house technos scatter in global order; a moved/mutated entry is not skipped or double-scattered relative to native. | Do not iterate only Rust house-owned object lists if global active order is the parity target. |
| `AI_ManageProduction`/`AI_ResumeProduction` in this tail are superweapon DVC manage/resume/cameo handlers gated by `House+0x1FC`, not global factory stepping. Active in YR: Yes/Conditional. | `0x004F9265..0x004F92FD`; `0x0050AF10`; `0x0050B1D0` | Rust combines charge/suspend with active storm early and has no house-tail cameo/resume split. | `src/sim/superweapon/mod.rs`, `src/sim/world/mod.rs` | Split active storm processing from per-house superweapon ready/manage/resume. House `+0x1FC` equivalent should be consumed in house tail and may be re-set by manage. | `superweapon_dirty_manage_can_requeue_resume_in_house_tail`: building/power change sets dirty; house tail suspend/deactivate/resume/cameo observes same-frame factory/building state. | Do not interpret `0x0050AF10` as FactoryClass production progress. |

## 6. Negative Facts / Do Not Do

- Do not place `MPlayer_Defeated` after native AI chooser/production-management effects; the call site is before those effects inside `HouseClass::Update`.
- Do not treat `HouseClass::ScatterAllUnits` as a per-house owned-vector loop; it scans the global `TechnoClass` array.
- Do not collapse active `LightningStorm::Process`, `SuperClass::AI_Ready`, and `AI_ManageProduction` into one early Rust superweapon tick.
- Do not use `House+0x1FC` as proof that `HouseClass::Update` owns global factory progress; `FactoryClass::AI` has already run globally before houses.
- Do not use stale `HOUSECLASS_GHIDRA_REPORT.md` wording that labels `+0x258/+0x264` as a production queue; verified field map and this decompile show the active DVC is superweapons.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| PerTick tactical -> factory -> house order | verified | `0x0055B667`, `0x0055B680`, `0x0055B6A6` | exact factory-array insertion order is separate slot |
| House superweapon ready loop | verified | `0x004F8E34..0x004F8E84`, `0x006CBCA0` | full sidebar/EVA visual details deferred |
| Multiplayer defeat call placement | verified | `0x004F8E86..0x004F8F82` | exact alternate game-mode value matrix deferred |
| `MPlayer_Defeated` scoped side effects | verified | `0x004FC0B0` decompile/caller | full UI/render side effects not expanded |
| `ScatterAllUnits` scoped side effects | verified | `0x004FC6D0`, `0x004FC747..0x004FC777` | exact `FUN_0070F820`/capture-manager meaning deferred |
| AI chooser formulas | touched-not-exhausted | `0x004F8FE1..0x004F9265` | out of scope; separate AI formula investigation |
| `AI_ManageProduction` superweapon manage | verified | `0x0050AF10` | exact type vtable `+0x40` semantics deferred |
| `AI_ResumeProduction` superweapon resume/cameo | verified | `0x0050B1D0` | exact `FUN_006CB560` internals deferred |
| Current Rust ordering | verified | `src/sim/world/mod.rs`, `src/sim/ai.rs`, `src/sim/superweapon/mod.rs`, `src/sim/production/production_queue.rs` rg scan | no Rust edits made |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the factory loop before the house loop? -> Yes, tactical vtable call, global factory loop, then global house loop.` (evidence: `0x0055B667..0x0055B6B1`)
- `[RESOLVED] OQ-2 - Where does per-house superweapon ready run? -> Inside `HouseClass::Update` before multiplayer defeat.` (evidence: `0x004F8E34..0x004F8E84`)
- `[RESOLVED] OQ-3 - Does defeat run before later AI/manage effects? -> Yes; scatter and `MPlayer_Defeated` precede AI chooser and `+0x1FC` manage/resume.` (evidence: `0x004F8F79..0x004F92FD`)
- `[RESOLVED] OQ-4 - What gates normal multiplayer defeat? -> `g_GameMode != 0`, not already defeated, frame > 0, non-passive house; count predicate differs under `DAT_00A8B262`.` (evidence: `0x004F8E86..0x004F8F82`)
- `[RESOLVED] OQ-5 - Does scatter use owned-object order? -> No, it scans global `TechnoClass` array.` (evidence: `0x004FC6D0`)
- `[RESOLVED] OQ-6 - What does `MPlayer_Defeated` write first? -> `House+0x1F5 = 1`.` (evidence: `0x004FC0B0`)
- `[RESOLVED] OQ-7 - What are the house-tail manage/resume split points? -> `House+0x1FC` gate clears then calls `0x0050AF10` and `0x0050B1D0`; manage may set `+0x1FC` again.` (evidence: `0x004F926C..0x004F92FD`, `0x0050B1A5`)
- `[RESOLVED] OQ-8 - Are manage/resume global factory production? -> No, both loop `House+0x258/+0x264` superweapon entries and building SW slots.` (evidence: `0x0050AF10`, `0x0050B1D0`)
- `[RESOLVED] OQ-9 - Does Rust run AI before defeat? -> Yes, `ai::tick_ai` phase precedes `check_defeat`.` (evidence: `src/sim/world/mod.rs` rg lines 1777, 1784, 1810, 1814)
- `[DEFERRED] OQ-10 - Exact AI chooser formulas and priorities` (category: out-of-scope; reason: target is tail placement and split points, not full AI build formulas; next-step-if-pursued: `/re-investigate HouseClass AI chooser formulas`)
- `[DEFERRED] OQ-11 - Exact `FUN_0070F820` and capture-manager semantics in scatter` (category: bounded-cost-too-high; reason: scatter entry effects are proven, helper ownership semantics require separate target; next-step-if-pursued: inspect `FUN_0070F820` and `CaptureManagerClass::SetOriginalOwner`)
- `[DEFERRED] OQ-12 - Exact sidebar/EVA rendering for superweapon cameos` (category: out-of-scope; reason: only sim/tail split points requested; next-step-if-pursued: trace sidebar tab refresh and cameo draw path)

## 9. Stale Docs / Follow-up Wording

- Replace any wording that says `HouseClass::Update` contains the factory production tick with: "`FactoryClass::AI` is a global factory-array pass before the global house-array pass. `HouseClass::Update` consumes the post-factory state and runs per-house superweapon ready, multiplayer defeat/scatter, AI chooser, and dirty-gated superweapon manage/resume work."
- Replace "`AI_ManageProduction` / `AI_ResumeProduction` are generic build production-management tail effects" with: "In the verified `0x0050AF10` / `0x0050B1D0` bodies, the dirty-gated tail work loops the house `SuperClass*` DVC, building superweapon grant slots, power state, suspend/deactivate/resume, and current-player cameo/sidebar refresh. Full build-choice production formulas remain in separate `AI_Choose_*` paths and are not proven here."
- Keep the existing correction that `House+0x258/+0x264` is the superweapon DVC, not a production queue.

## Sources

- Ghidra decompile/read-only: `0x0055AFB0`, `0x004F8440`, `0x004FC0B0`, `0x004FC6D0`, `0x0050AF10`, `0x0050B1D0`, `0x006CBCA0`, `0x00508DF0`, `0x00508F60`.
- Ghidra caller/callee evidence: `MPlayer_Defeated` caller `HouseClass::Update`; `ScatterAllUnits` callers `HouseClass::Update` and `FUN_006E3180`; `Update` callee list includes the scoped functions.
- Prior docs: `FACTORY_HOUSE_BULLET_ANIM_SAME_TICK_SYSTEM_MODEL_SYNTHESIS.md`, `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`, `HOUSECLASS_GHIDRA_REPORT.md`, `HOUSECLASS_VERIFIED_FIELD_MAP.md`, `docs/contracts/2026-05-28-factory-house-tail-order-implementation-contract.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/ai.rs`, `src/sim/superweapon/mod.rs`, `src/sim/production/production_queue.rs`, `src/sim/house_state.rs`.

## Status

COMPLETE for the requested coverage-map slice: HouseClass tail ordering, defeat/scatter entry side effects, manage/resume split points, and Rust-facing handoff. Remaining uncertainty is explicitly deferred above.
