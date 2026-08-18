# AnimClass AI First Safe Migration Slice - Ghidra Research Report

**Address(es):** `AnimClass::AI @ 0x00423AC0`, `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::Destroy @ 0x004255B0`, `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectClass::UnInit @ 0x005F65F0`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `FUN_0055BAA0 @ 0x0055BAA0`, `FUN_0055BAE0 @ 0x0055BAE0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** first safe Rust migration slice for `AnimClass::AI` under the native LogicClass live-object scheduler, limited to first-AI guard, same-pass child eligibility, parent cleanup/remove ordering, and current Rust `WorldEffect` / app animation surfaces.
**Non-Scope:** full bouncer physics, water splash visuals, damage formulas, draw traversal/depth, audio fidelity, save/load, complete constructor caller taxonomy, and Rust implementation.
**Confidence:** High for the scheduler/lifecycle boundary; Medium for the exact first production feature to enable because it depends on parent slot integration with the global object pass.
**Active in YR:** Yes for ordinary revealed `AnimClass` AI through `LogicClass`; Conditional for trailer/bounce/expire child branches depending on `artmd.ini`/modded `AnimTypeClass` fields.

## Working Notes Gate

`Target question`: What is the first safe Rust migration slice for `AnimClass::AI` under native `LogicClass` live-object scheduling, including first-AI guard, child tail append, parent cleanup/removal, and current `WorldEffect`/animation surfaces?

`Non-goals`: Do not implement Rust; do not re-investigate full `AnimClass::AI`; do not broaden into rendering order, bouncer physics internals, water splash assets, sound timing, or complete spawn taxonomy.

`Evidence needed to mark COMPLETE`: decompile plus assembly-context evidence for scheduler count reload, reveal/register, constructor first guard, trailer/bounce/expire child constructor sites, destroy/uninit/removal, and current Rust surface scan with migration handoff and test names.

`Stop conditions`: Stop once the migration slice can be stated without depending on unverified global render/damage behavior, and every remaining question is either resolved or deferred with a next step.

## 1. Overview

The first safe migration is not "make `WorldEffect` smarter." Native `AnimClass` objects are revealed live objects: constructor/register paths give them `LogicClass` membership, `LogicClass::PerTickUpdate` calls their `vtable+0x5C`, and child anims constructed inside a parent AI can be tail-appended and reached in the same pass.

The safe Rust slice is a scheduler-backed `AnimClass` shell that can be driven by `Simulation::for_each_live_object`: construct/reveal/register an anim object, run the already researched ordinary lifecycle first-AI guard, allow trailer/bouncer/expire constructor rows to tail-append through the same membership API, and route normal destroy through unregister/compact semantics. Production replacement of all visual `WorldEffect` paths should wait until draw/depth and spawn-taxonomy slices are integrated.

## 2. Class Layout / Key Offsets

| Offset | Field | Verified role | Active in YR |
|---:|---|---|---|
| `Object+0x90` | alive/active byte | constructor sets; AI checks before trailer; UnInit clears | Yes, evidence `0x004220B0`, `0x005F65F0` |
| `Object+0x98` | live LogicClass membership byte | set by `FUN_0055BAA0`, cleared by `FUN_0055BAE0` | Yes, evidence `0x0055BAA5..0x0055BAC6`, `0x0055BAE7..0x0055BB2E` |
| `Anim+0xAC` | current frame | first guard returns before ordinary frame advance | Yes, evidence `0x00423AC0` |
| `Anim+0xC8` | `AnimTypeClass*` | trailer, expire, next, and lifecycle fields are read from here | Yes, evidence `0x004242BA`, `0x00423DE7`, `0x004247F3` |
| `Anim+0x184` | constructor/start delay | decremented after first-AI guard | Yes, evidence `0x00423AC0` |
| `Anim+0x194` | bouncer instance byte | gates `vtable+0x1E8` bounce processing | Conditional, evidence `0x00423C24..0x00423C41` |
| `Anim+0x195` | loop remaining byte | lifecycle field; not the scheduler flag | Yes/Conditional, evidence `ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md` |
| `Anim+0x19B` | inactive/expired byte | suppresses ordinary work; destroy path follows | Yes, evidence `0x00423AC0` |
| `Anim+0x19C` | first-AI guard | constructor sets; first AI clears and returns | Yes, evidence `0x00421EA0`, `0x00423AC0` |
| `AnimType+0x300` | `BounceAnim` | child constructor row in `ProcessBounceResult` | Conditional, evidence `0x004239A7..0x004239CE` |
| `AnimType+0x304` | `ExpireAnim` | child constructor row before AI impact side effects | Conditional, evidence `0x00423DE7..0x00423E70` |
| `AnimType+0x308` | `TrailerAnim` | child constructor row before first-AI guard | Conditional, evidence `0x004242A6..0x0042431D` |
| `AnimType+0x30C` | `TrailerSeperation` | signed global-frame modulo gate | Conditional, evidence `ANIMCLASS_AI_TRAILER_NEXT_INTERACTION_GHIDRA_REPORT.md` |

## 3. Core Logic

### 3.1 Scheduler ownership

Active in YR: Yes.

`LogicClass::PerTickUpdate @ 0x0055AFB0` owns the ordinary live-object AI pass. Assembly context `0x0055B608..0x0055B619` loads `LogicClass+0x04[index]`, calls `vtable+0x5C`, reloads `LogicClass+0x10`, increments the cursor, and compares against the reloaded count. This proves tail appends can be seen in the same pass, and compacting removal can skip the object shifted into the just-processed index.

### 3.2 Constructor and reveal/register

Active in YR: Yes.

`AnimClass::Constructor @ 0x00421EA0` appends `this` to `g_AnimClass_Array` at `0x00422092..0x004220A7`, sets alive byte `+0x90`, initializes `+0x19B=0`, and sets first-AI guard `+0x19C=1`. For normal non-meteor/non-bouncer construction it calls `ObjectClass::Reveal`, which reaches `FUN_0055BAA0` at `0x005F5038..0x005F5040` when the type is logic-eligible. `FUN_0055BAA0` is membership-guarded by `Object+0x98`.

Constructor ordering is also verified: the normal and special-position `ObjectClass::Reveal` calls occur before loop-byte initialization and before the final `if (delay == 0) AnimClass::Middle()` branch. Therefore a normal zero-delay damage-fire animation is revealed/registered before its constructor-time `Middle` effects run. Evidence: read-only live call `mcp__ghidra_mcp__decompile_function(address="0x00421EA0")` on 2026-07-18, specifically the reveal branches preceding the loop-count writes and final `AnimClass__Middle()` call.

Material consequence: `g_AnimClass_Array` is a registry/lifetime list, not the AI scheduler. The first safe Rust slice must separate object storage/registry from live AI membership.

### 3.3 First-AI guard

Active in YR: Yes.

The first `AnimClass::AI` visit checks inactive state, then checks first-AI guard `+0x19C`. If set, AI clears the byte and returns before delay countdown, timer checks, frame advance, loop/end, `Next`, or ordinary destroy. This is active for zero-delay constructor rows too: delay-zero construction can call `Middle()` immediately, but the first AI visit still cannot advance playback.

Material consequence: same-pass scheduler eligibility is not same-pass visible advancement.

### 3.4 Child tail-append sites inside parent AI

Active in YR: Conditional.

Three child families matter for the first scheduler-backed slice:

| Child source | Constructor evidence | Row | Active in YR |
|---|---|---|---|
| `TrailerAnim` | `0x004242A6..0x0042431D` | `type=parent.Type+0x308`, `coords=parent.GetCoords`, `delay=1`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0` | Conditional; stock `artmd.ini` has trailer rows such as `DBRIS*`, `METLARGE`, `METSMALL` |
| `BounceAnim` | `0x004239A7..0x004239CE` | `type=parent.Type+0x300`, `coords=parent.GetCoords`, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0` | Conditional engine path; no stock `BounceAnim=` row found in `art.ini`/`artmd.ini` |
| `ExpireAnim` | `0x00423DE7..0x00423E70` | `type=parent.Type+0x304`, impact coords, `delay=0`, `loop=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0` | Conditional; stock `artmd.ini` has bouncer/meteor `ExpireAnim=` rows |

Because these constructors call the same `AnimClass::Constructor`/`Reveal` path, children are eligible for same-pass `vtable+0x5C` if appended before the live cursor exits. The child first-AI guard and delay still decide whether the visit performs visible work.

### 3.5 Parent cleanup and active-vector removal

Active in YR: Yes.

`AnimClass::Destroy @ 0x004255B0` detaches owner state, releases sound, optionally plays `StopSound`, and calls `ObjectClass::UnInit @ 0x005F65F0`. `ObjectClass::UnInit` dispatches virtual `+0xD4` before clearing alive byte and appending to pending-delete. The active-list remover `FUN_0055BAE0 @ 0x0055BAE0` checks `Object+0x98`, finds the object, decrements count, shifts later entries left, and clears membership.

Material consequence: parent self-destroy during its AI can remove the current object from the same live vector. The scheduler increments the cursor after return and does not repair for the compaction.

## 4. INI Keys

| Key | Source | Effect for this slice | Active in YR |
|---|---|---|---|
| `TrailerAnim=` | `art.ini`/`artmd.ini`, read into `AnimType+0x308` | child row emitted before first-AI guard/frame advancement | Conditional; stock examples found in `ini/artmd.ini` |
| `TrailerSeperation=` | read into `AnimType+0x30C` | signed global-frame modulo gate for trailer row | Conditional; stock examples `=1` and `=2` |
| `BounceAnim=` | read into `AnimType+0x300` | optional return-1 bounce child row | Conditional engine path; no stock row found by `rg "^BounceAnim="` |
| `ExpireAnim=` | read into `AnimType+0x304` | optional accepted-impact child row and impact side-effect gate | Conditional; stock bouncer/meteor rows exist |
| `Bouncer=` | read into `AnimType+0x35A` | constructor sets instance bouncer byte | Conditional; stock debris rows exist |
| `IsMeteor=` | read into `AnimType+0x356` | constructor sets instance bouncer/meteor byte | Conditional; stock meteor rows exist |
| `Next=` | read into `AnimType+0x2C8` | in-place type transition; no new object | Conditional; stock examples exist |

## 5. Integration Points

Active in YR: Yes for the scheduler; Conditional for each child branch.

- `ObjectClass::Reveal @ 0x005F4EC0` submits visible objects and calls `FUN_0055BAA0` at singleton `0x87F778` when the type and game-mode gates pass.
- `LogicClass::PerTickUpdate @ 0x0055AFB0` calls live objects at `vtable+0x5C`. For `AnimClass`, this dispatch reaches `AnimClass::AI @ 0x00423AC0`.
- `AnimClass::AI` can construct child anims before first-AI guard (`TrailerAnim`), during bouncer processing (`BounceAnim`), and during accepted impact (`ExpireAnim`).
- `AnimClass::Destroy` routes to `ObjectClass::UnInit`, which routes through conceal/removal before pending-delete frees memory later.

## 6. Current Rust Implementation Status

Current Rust has partial ingredients but not the native `AnimClass` object.

| Surface | Current status | Evidence |
|---|---|---|
| Logic membership primitives | Present: `register_live_object`, `unregister_live_object`, and `for_each_live_object` model append/reload/compaction semantics | `src/sim/world/mod.rs:680..770` |
| Global tick pipeline | Still staged; combat/retaliation consume snapshots, not the live cursor | `src/sim/world/mod.rs:1508..1985`, especially `1760`, `1972` |
| Generic `WorldEffect` | Retained visual vector with frame-count/rate-ms ticking; no live membership, no first-AI guard, no child append cursor, no `UnInit` | `src/sim/components.rs:823..923`; ticked by `retain_mut` in `src/sim/world/mod.rs:1467..1480` |
| Constructor row descriptor | Exists and preserves constructor fields for migrated spawns | `src/sim/components.rs:769..815` |
| App-side garrison `AnimRuntime` | Has first-AI guard, trailer-before-guard event tests, `Next` in-place tests, and normal-destroy negative test, but is app-retained and not a live object | `src/app_building_anim.rs:776..940`, `1048..1065`, `1382..1475` |
| Art metadata | Parses `Next`, `BounceAnim`, `ExpireAnim`, `TrailerAnim`, signed `TrailerSeperation`, random fields, draw offsets | `src/rules/art_data.rs:180..195`, `274..295` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `LogicClass` live vector count reload | verified | `0x0055B608..0x0055B619` | none for scheduler rule |
| `ObjectClass::Reveal -> FUN_0055BAA0` registration | verified | `0x005F5038..0x005F5040`, `0x0055BAA0` | exact type-gate matrix out of scope |
| `AnimClass::Constructor` registry/first guard | verified | `0x00421EA0`, `0x00422092..0x004220B0` | none for migration slice |
| First-AI guard clears and returns | verified | `0x00423AC0` | none for ordinary lifecycle |
| Trailer child constructor row | verified | `0x004242A6..0x0042431D`; sibling report | none for row/scheduler, sound timing deferred |
| Bounce child constructor row | verified | `0x004239A7..0x004239CE`; sibling report | full bounce physics out of scope |
| Expire child constructor row | verified | `0x00423DE7..0x00423E70`; sibling report | water/splash branch and damage formulas out of scope |
| Parent destroy/uninit/remove | verified | `0x004255B0`, `0x005F65F0`, `0x0055BAE0` | exact pending-delete free timing out of scope |
| Current Rust `WorldEffect` retained vector | verified | `src/sim/components.rs:823..923`, `src/sim/world/mod.rs:1467..1480` | implementation pending |
| Current Rust app `AnimRuntime` | verified | `src/app_building_anim.rs:776..1475` | not scheduler-backed |
| Full render/depth order for migrated anims | deferred | draw reports exist | required before broad production replacement |
| Save/load membership reconstruction | deferred | separate active-object reports | required for save/load parity |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which function owns ordinary AnimClass AI order? -> LogicClass::PerTickUpdate live object loop, not g_AnimClass_Array.` (evidence: `0x0055B608..0x0055B619`, `0x00422092..0x004220A7`)
- `[RESOLVED] OQ-02 - Is first-AI guard before visible frame advancement? -> Yes; it clears +0x19C and returns before delay/timer/frame logic.` (evidence: `0x00421EA0`, `0x00423AC0`)
- `[RESOLVED] OQ-03 - Can child anims appended by parent AI be same-pass eligible? -> Yes, conditionally under the live cursor count-reload rule.` (evidence: `0x0055B613`, child constructor calls `0x0042431D`, `0x004239CE`, `0x00423E70`)
- `[RESOLVED] OQ-04 - Does same-pass eligibility imply same-pass visible frame advancement? -> No; child first-AI guard and delay still gate visible work.` (evidence: `0x00423AC0`, trailer row `delay=1`)
- `[RESOLVED] OQ-05 - Does parent cleanup use compacting active-vector removal? -> Yes; destroy reaches UnInit/conceal/remove, and remover shifts later entries left.` (evidence: `0x004255B0`, `0x005F65F0`, `0x0055BB09..0x0055BB21`)
- `[RESOLVED] OQ-06 - Does Rust already have a live-vector primitive suitable for the slice? -> Yes, but staged systems mostly do not use it yet.` (evidence: `src/sim/world/mod.rs:680..770`, `src/sim/world/mod.rs:1760`, `1972`)
- `[RESOLVED] OQ-07 - Is current WorldEffect a native AnimClass object? -> No; it is a retained visual list with ms/frame ticking and no membership byte or child scheduling.` (evidence: `src/sim/components.rs:823..923`, `src/sim/world/mod.rs:1467..1480`)
- `[RESOLVED] OQ-08 - Can app garrison AnimRuntime be treated as the generic scheduler implementation? -> No; it has useful lifecycle logic but is app-retained outside LogicClass membership.` (evidence: `src/app_building_anim.rs:776..940`)
- `[DEFERRED] OQ-09 - Which production spawn family should be enabled first?` (category: requires-different-system-context; reason: constructor taxonomy and render/depth readiness decide whether muzzle, warhead, debris, or garrison visual should flip first; next-step-if-pursued: reconcile with spawn taxonomy and draw traversal reports)
- `[DEFERRED] OQ-10 - Exact retail same-pass visible effect for one stock meteor/debris vector index.` (category: needs-runtime-debugger; reason: static rule is proven but concrete index observation needs runtime logging; next-step-if-pursued: instrument active-vector indices around a stock debris/meteor impact)
- `[DEFERRED] OQ-11 - Save/load membership for new Rust AnimClass objects.` (category: requires-different-system-context; reason: this slice covers runtime migration, not persistence; next-step-if-pursued: consume active-object save/load reports)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Revealed `AnimClass` objects are live `LogicClass` members; scheduler reloads count after each `vtable+0x5C`. | `0x005F5038..0x005F5040`, `0x0055BAA0`, `0x0055B608..0x0055B619` | `WorldEffect` is a retained vector; `advance_tick` mostly uses staged snapshots. | `src/sim/world/mod.rs`, future generic `AnimClass` object/runtime, `src/sim/components.rs`. | First safe slice: add a scheduler-backed AnimClass shell that registers on reveal and is visited by `for_each_live_object`; keep broad production visual replacement disabled until render/depth integration. | Synthetic live order `parent, sibling`; parent trailer-spawns a child at tail before old count exits; child is visited later in the same pass but only clears first guard. Proposed test: `anim_scheduler_tail_appended_child_gets_first_guard_visit_same_pass`. | Do not drive generic anims from `world_effects.retain_mut`; it cannot observe append/remove cursor semantics. |
| First `AnimClass::AI` clears constructor guard and returns before delay/timer/frame work. | `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`; app mirror `src/app_building_anim.rs:927..930`. | App garrison runtime has this; generic `WorldEffect` does not. | Reusable AnimRuntime moved/bridged into sim-owned AnimClass shell. | A zero-delay constructor row calls start/Middle equivalent, then first scheduler visit does not increment `current_frame`, does not delete, and does not process `Next`. Proposed test: `animclass_ai_first_visit_clears_guard_without_frame_advance`. | Do not equate constructor `delay=0` or same-pass scheduler visit with visible frame advance. |
| Trailer/Bounce/Expire children constructed inside parent AI use normal constructor/reveal path and can tail-append before parent cleanup. | Trailer `0x004242A6..0x0042431D`; Bounce `0x004239A7..0x004239CE`; Expire `0x00423DE7..0x00423E70`; scheduler reload `0x0055B613`. | `AnimClassSpawnDescriptor` preserves rows, but descriptors become visual `WorldEffect`s or app events, not live children. | Generic child-spawn API from AnimClass AI into entity/live-object registration. | Parent emits children in native order; each child is allocated/stored, reveal-registers to tail, and can receive a first-guard AI visit in the same global pass. Proposed test: `animclass_ai_child_rows_tail_append_in_native_order`. | Do not batch children into "next tick effects"; do not copy parent lifecycle fields into child runtime. |
| Parent destroy reaches UnInit/conceal/removal; removal compacts left and scheduler does not repair cursor. | `0x004255B0`, `0x005F65F0`, `0x0055BAE0`, `0x0055BB09..0x0055BB21`, scheduler `0x0055B616..0x0055B619`. | `WorldEffect` removal is `retain_mut`; no generic AnimClass parent removal participates in live-object cursor. | Generic AnimClass destroy/uninit/despawn path plus LogicVector removal. | Live order `parent, sibling`; parent destroys itself during AI after spawning a child. The shifted sibling skip/child reachability matches compacting live-vector semantics. Proposed test: `animclass_self_destroy_compacts_live_vector_without_index_repair`. | Do not use `swap_remove`; do not erase storage without `unregister_live_object`. |
| Normal `AnimClass::Destroy` does not spawn `ExpireAnim`; `ExpireAnim` is an accepted-impact AI branch. | `AnimClass::Destroy @ 0x004255B0` body; accepted-impact branch `0x00423DE7..0x00423E70`; sibling bouncer report. | App runtime has a normal-destroy negative test; generic WorldEffect has no bouncer branch. | Future generic AnimClass bouncer runtime. | Non-bouncer anim with `ExpireAnim=` that reaches ordinary end destroys without spawning `ExpireAnim`; bouncer accepted impact with `ExpireAnim=` can spawn the child before destroy. Proposed test: `animclass_normal_destroy_does_not_emit_expireanim`. | Do not implement `ExpireAnim` as a generic "on destroy" hook. |

### First Safe Slice Recommendation

1. Build the sim-owned `AnimClass` live-object shell first, behind narrow tests, using existing `LogicVector` primitives and the app-side `AnimRuntime` lifecycle as source material.
2. Enable only constructor/reveal/register, first-AI guard, ordinary non-bouncer lifecycle visits, trailer child row emission, `Next` in-place mutation, normal destroy/unregister, and scheduler tests.
3. Leave bouncer impact damage, water splash, global production spawn replacement, and full renderer replacement disabled until their sibling contracts are integrated.

This slice is safe because it proves the hard scheduler boundary without requiring all visual/depth/damage branches to be correct on day one.

## 10. Negative Facts / Do Not Do

- Do not iterate `g_AnimClass_Array` as the ordinary AI pass. Active in YR: Yes. Evidence: constructor registry append `0x00422092..0x004220A7` is separate from `LogicClass` `vtable+0x5C` loop `0x0055B608..0x0055B619`.
- Do not treat `WorldEffect` as native-equivalent just because it stores `AnimClassSpawnDescriptor`. Active in YR comparison: Yes. Evidence: `WorldEffect::tick_with_start_sound` is ms/rate retained-vector logic at `src/sim/components.rs:896..923`, not live-object membership.
- Do not make spawned trailer/bounce/expire children universally wait until the next tick. Active in YR: Conditional. Evidence: scheduler reloads live count at `0x0055B613` after each object AI call.
- Do not claim same-pass child visit means same-pass visible frame advance. Active in YR: Yes. Evidence: first-AI guard branch in `AnimClass::AI @ 0x00423AC0`; trailer child row uses `delay=1`.
- Do not use `swap_remove` or sorted-ID iteration for the active object scheduler. Active in YR: Yes. Evidence: remover shifts entries left at `0x0055BB11..0x0055BB21`; Rust has matching primitive comments at `src/sim/world/mod.rs:749..770`.
- Do not implement `ExpireAnim` in `AnimClass::Destroy`. Active in YR: Yes/Conditional. Evidence: `Destroy @ 0x004255B0` reads `StopSound +0x2FC`, not `ExpireAnim +0x304`; the `ExpireAnim` constructor is in AI accepted-impact branch `0x00423DE7..0x00423E70`.
- Do not trust stale Ghidra header comments as findings. Active in YR evidence rule: Yes. Evidence: current local decompiler header for `AnimClass::Destroy` says it spawns ExpireAnim, but the function body at `0x004255B0` does not read `+0x304` or call `AnimClass::Constructor`.

## 11. Remaining Uncertainty

- Concrete stock vector index traces for meteor/debris same-pass visible outcomes require runtime logging; static scheduler semantics are verified.
- Which production path should be flipped from `WorldEffect` to scheduler-backed `AnimClass` first depends on slot-1/slot-3/slot-5 reconciliation and draw/depth readiness.
- Save/load persistence for Rust `AnimClass` live-object membership is not covered.
- Full bouncer/water/damage side effects are covered only as branch boundaries here; implementation still needs the sibling bouncer reports.

## 12. Stale Docs / Follow-up Docs

- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` Section 10 replacement wording:
  "Within native `LogicClass::PerTickUpdate`, ordinary revealed `AnimClass` AI is dispatched through the live `LogicClass` active-object vector (`vtable+0x5C`), not by iterating `g_AnimClass_Array`. `g_AnimClass_Array` is still appended by the constructor for registry/lifetime/owner-scan uses. Child anims constructed during a parent AI visit use the normal reveal/register path and are same-pass eligible under the live cursor count-reload rule; first-AI guard and delay decide whether the visit is visibly advancing."
- No other exact stale-doc replacement was proven in this slot.

## Sources

- Read-only Ghidra decompile: `AnimClass::AI @ 0x00423AC0`, `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::Destroy @ 0x004255B0`, `ObjectClass::Reveal @ 0x005F4EC0`, `ObjectClass::UnInit @ 0x005F65F0`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `FUN_0055BAA0 @ 0x0055BAA0`, `FUN_0055BAE0 @ 0x0055BAE0`.
- Read-only Ghidra assembly context: `0x0055B608`, `0x0055B610`, `0x0055B613`, `0x005F5038`, `0x005F5040`, `0x00422092`, `0x004220A7`, `0x00423C24`, `0x004239A7`, `0x004239CE`, `0x004242A6`, `0x0042431D`, `0x00423DE7`, `0x00423E70`, `0x0055BB09`.
- Prior reports: `ANIMCLASS_GLOBAL_REGISTRATION_SAMEPASS_SCHEDULER_GHIDRA_REPORT.md`, `ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`, `ANIM_CLASS_GHIDRA_REPORT.md`, `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `ANIMCLASS_AI_TRAILER_NEXT_INTERACTION_GHIDRA_REPORT.md`, `ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`.
- Rust scan: `src/sim/components.rs`, `src/sim/world/mod.rs`, `src/app_building_anim.rs`, `src/rules/art_data.rs`.

## Status

COMPLETE for the scoped first safe `AnimClass::AI` migration slice.
