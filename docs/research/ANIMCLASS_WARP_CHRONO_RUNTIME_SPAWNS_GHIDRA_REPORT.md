# AnimClass Warp / Chrono Runtime Spawns -- Ghidra Research Report

**Address(es):** `0x00629E90`, `0x00629FD0`, `0x006297F0`, `0x007192F0`, `0x00719400`, `0x00719790`, `0x00421EA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Runtime `AnimClass` constructor rows reached from the requested warp/chrono visual entry points only: `WarpAttachClass__SpawnWarpAnims`, `WarpAttachClass__UpdateAttack`, `TemporalClass__AI` as the `UpdateAttack` indirect target, and the teleport locomotor phase code at `StateMachineTick` / `InitiateWarp` / `ClearPendingWarpPhase`.  
**Non-Scope:** Full chrono miner movement, full Chronosphere target selection, full temporal damage math, generic `AnimClass` callers outside these entry points, render/blitter parity, and audio beyond identifying sound-adjacent ordering.  
**Confidence:** High for constructor rows and path liveness; Medium for exact temporal visual asset role names beyond the resolved keys.  
**Active in YR:** Yes, conditional on active YR content using `Teleporter=yes` / `Locomotor=Teleport` and Chrono Legionnaire-style `Temporal=yes` plus `WarpAway=yes` type data.

Working notes gate:
- `Target question`: Which `AnimClass` rows are spawned by active YR warp/chrono runtime paths, with exact constructor arguments, ordering, attachment, and keys?
- `Non-goals`: Do not re-investigate chrono miner routing, Chronosphere area selection, generic anim rendering, or non-warp `AnimClass` callers.
- `Evidence needed to mark COMPLETE`: Decompile plus assembly context for every material constructor call; INI/default plus binary parser address for every rules key; Rust touchpoint scan; final open-question ledger with no open items.
- `Stop conditions`: Stop after requested entry points and their direct warp/chrono constructor callees are classified; defer unrelated parasite projectile/damage visuals and broad render parity.

## 1. Overview

Active YR teleport-locomotor warp visuals create standalone, global-pool `AnimClass` objects using `[General] WarpOut` (`RulesClass+0x33C`) at the unit's current coordinates. The requested teleport paths do not use `[General] WarpIn`, `[General] WarpAway`, or `[General] ChronoSparkle1` for those constructor rows.

The requested `WarpAttachClass` path is a separate temporal/parasite visual surface. Its temporal branch is active for Chrono Legionnaire-style attackers, but its constructor rows use `SQDG`, `[General] Wake` (`RulesClass+0x94`), and rubble anims (`RulesClass+0xBC4/+0xBD0`), not the teleport `[General] WarpOut` row.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `+0x9C/+0xA0/+0xA4` | `TechnoClass` | World coordinate copied into `AnimClass` spawn coords | `StateMachineTick @ 0x00719420..0x00719442`, `0x00719838..0x00719873`, `0x00719B41..0x00719B7C`; `TemporalClass__AI @ 0x006298FD..0x00629913` | Yes |
| `+0x270` | `TechnoClass` | Warping-out flag cleared in Chronosphere phase 2; set by other helpers, not an anim constructor argument | `StateMachineTick @ 0x007198DA..0x007198FA` | Yes, conditional |
| `+0x271` | `TechnoClass` | Being-warped flag set/cleared around teleport state; not passed to `AnimClass` | `InitiateWarp @ 0x00719573..0x0071958A`; `StateMachineTick @ 0x007198DA`, `0x00719BB0` | Yes |
| `+0x280` | `TechnoClass` | Pending warp phase; cleared immediately after `WarpOut` spawn in self/cleanup paths | `StateMachineTick @ 0x00719791..0x00719799`, `0x00719B7C..0x00719B88`; `InitiateWarp decompile` | Yes |
| `+0x44` | `TemporalClass` | Persistent temporal visual `AnimClass*` stored for owner tracking/removal | `TemporalClass__AI @ 0x00629913..0x0062991C`; decompile stores constructor return | Yes |
| `+0x48` | `TemporalClass` | Temporal visual attack state `0..4`; gates constructor rows | `TemporalClass__AI @ 0x006298B0..0x006298BC` | Yes |
| `+0x33/+0xCC` | `AnimClass` | Owner object pointer used by attached temporal anim after construction | `AnimClass__Constructor @ 0x00421EA0` initializes owner null; `TemporalClass__AI` later calls `AnimClass__SetOwnerObject` | Yes, temporal only |
| `+0x190` | `AnimClass` | Draw flags; all rows in this slice pass `0x600` | `AnimClass__Constructor @ 0x00421EA0`; assembly contexts below | Yes |

## 3. Core Logic

### Constructor Rows

| Path | Constructor row | Ordering | Attachment | Active in YR |
|---|---|---|---|---|
| `TeleportLocomotionClass__StateMachineTick` state 0 / `InitiateWarp` self-teleport departure | `new AnimClass(Rules+0x33C, current coords, delay=0, loops=1, flags=0x600, zAdjust=0, reverse=0)` | Before distance timer setup, `BeingWarped`, detaching temporal links, unmark/move/re-mark, sounds, and arrival spawn | No owner attachment | Yes, when active Teleport locomotor has a non-null target coord different from current coord |
| `TeleportLocomotionClass__StateMachineTick` state 0 / `InitiateWarp` self-teleport arrival | Same `Rules+0x33C` row at post-move current coords | After movement/marking, crate pickup, and before `PendingWarpPhase=0` | No owner attachment | Yes, same branch |
| `TeleportLocomotionClass__StateMachineTick` phase 2 | Same `Rules+0x33C` row at current coords before external warp relocation | Before unmark, out sound, `BeingWarped=1`, clearing `ChronoInTransit`/`WarpingOut`/bridge flag, and `Update_Position` | No owner attachment | Yes, Chronosphere/external warp path |
| `TeleportLocomotionClass__StateMachineTick` phase 5 | Same `Rules+0x33C` row at destination/current coords | After post-warp validation, mission/ghost/occupation setup, and timer arm from `Techno+0x284` | No owner attachment | Yes, Chronosphere/external warp path if target survives |
| `TeleportLocomotionClass__ClearPendingWarpPhase` body at `0x00719790` | Same `Rules+0x33C` row, then `Techno+0x280=0` | Cleanup path when state-0 destination is invalid/null | No owner attachment | Conditional: live only on no-destination cleanup |
| `TemporalClass__AI` state 0 | `new AnimClass(Find("SQDG"), target coords, delay=0, loops=1, flags=0x600, zAdjust=0, reverse=0)` | Starts temporal visual machine; return stored in `Temporal+0x44` and remove-listener array | Later attached to target via `SetOwnerObject(target)` | Yes, Chrono Legionnaire temporal attack branch |
| `WarpAttachClass__SpawnWarpAnims` | Three rows of `new AnimClass(Rules+0x94 Wake, randomized coords, delay=0, loops=1, flags=0x600, zAdjust=0, reverse=0)` | Called when temporal state 1 completes and after failed/survived state 4 cycles back | No direct owner attachment | Yes, conditional on temporal visual state transitions |
| `TemporalClass__AI` state 4 | Three rows of random `Rules+0xBC4[i]` rubble anims, `delay=2`, `loops=1`, `flags=0x600`, `zAdjust=-10`, `reverse=0` | Before erase/damage resolution | No direct owner attachment | Yes, conditional on temporal state 4 |
| `WarpAttachClass__UpdateAttack` non-temporal branch | Optional weapon/parasite anim from attacker data, `delay=0`, `loops=1`, `flags=0x600`, `zAdjust=0`, `reverse=0` | Normal non-temporal damage update | No chrono/warp-specific ownership proven | No for this target; this is not the warp/chrono visual path |

`AnimClass__Constructor @ 0x00421EA0` registers the object in `g_AnimClass_Array`, stores draw flags at `AnimClass+0x190`, stores z-adjust at `+0x100`, clamps loop count to at least one, and immediately calls `AnimClass__Middle` when `delay==0`. The rows above are therefore live global anim objects, not local render-only overlays.

### Ordering Details

- Self-teleport departure `WarpOut` at `0x00719442` is skipped if the source/destination equality guard fails. Active in YR: Yes, on non-null target differing from current coords.
- The harvester special case happens after timer and first `WarpOut` spawn: `WhatAmI()==1` plus type `+0xE0E Harvester=yes` forces timer duration `0` and clears `BeingWarped`, but it does not undo the already-created departure anim. Active in YR: Yes for Chrono Miner.
- Self-teleport arrival `WarpOut` is created after the unit has been unmarked, moved/marked at destination, sounds have been considered, facing/state set, crate pickup dispatched, and occupation set. Active in YR: Yes for normal self-teleport; player-observed chrono miner arrival behavior may need runtime verification because the binary row is present but current scenario traces have contradicted older summaries.
- External warp phase 2 creates the first external-path `WarpOut` before clearing `ChronoInTransit` and before `Update_Position`; phase 5 creates the second after post-warp validation only if the target remains in play. Active in YR: Yes for Chronosphere/ChronoWarp standard paths.
- Temporal visual state 0 creates `SQDG`, stores the returned anim pointer, and later reattaches it to the target if the target differs from the stored owner. Active in YR: Yes for standard Chrono Legionnaire temporal attacks.

## 4. INI Keys

| INI key | Binary field | Stock YR value | Used by this slice | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `[General] Wake` | `Rules+0x94` | `WAKE1` | Three temporal spark rows in `SpawnWarpAnims` | `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:525`; `WarpAttachClass__SpawnWarpAnims @ 0x00629FA3..0x00629FAC` | Yes, temporal branch |
| `[General] WarpIn` | `Rules+0x338` | `WARPIN` | Parsed, not used by requested constructor rows | `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:548` | No in this slice |
| `[General] WarpOut` | `Rules+0x33C` | `WARPOUT` | All teleport-locomotor warp anim rows | `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:549`; call contexts `0x00719439..0x00719442`, `0x00719864..0x00719873`, `0x00719B6D..0x00719B7C` | Yes |
| `[General] WarpAway` | `Rules+0x340` | `WARPAWAY` | Parsed, not used by requested teleport-locomotor rows | `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:550`; no requested call context reads `+0x340` | No in this slice |
| `[General] ChronoSparkle1` | `Rules+0x344` | `CHRONOSK` | Parsed, not used by requested constructor rows | `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:554`; no requested call context reads `+0x344` | No in this slice |
| `SQDG` hardcoded anim name | `0x0083665C` string | `SQDG` | Temporal persistent visual row, via `AnimTypeClass` lookup | PE string read at VA `0x0083665C`; `TemporalClass__AI @ 0x006298CC..0x00629913` | Yes, temporal branch |
| `RubbleAnims` vector | `Rules+0xBC4/+0xBD0` | rules-defined list | Temporal state-4 three small anim rows | `TemporalClass__AI decompile @ 0x006297F0`; existing `TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md` | Yes, temporal branch |

## 5. Integration Points

| Function | Role | Caller / tick proof | Active in YR |
|---|---|---|---|
| `TeleportLocomotionClass__StateMachineTick @ 0x007192F0` | Main teleport phase tick; contains constructor rows and inlined `InitiateWarp` body | Vtable/tick documented in `chronominer-locomotion/enum-state-machine-states.md`; decompiled live this pass | Yes |
| `TeleportLocomotionClass__InitiateWarp @ 0x00719400` | Labeled body for state-0 self-teleport spawn/order | Decompile plus assembly context at `0x00719420..0x00719799`; also inlined in `StateMachineTick` | Yes |
| `TeleportLocomotionClass__ClearPendingWarpPhase @ 0x00719790` | No-destination cleanup: spawn `WarpOut`, clear pending phase | Decompile plus assembly context `0x00719790..0x00719799`; prior doc notes register dispatch from state 0 | Conditional |
| `WarpAttachClass__UpdateAttack @ 0x00629FD0` | Dispatches to `TemporalClass__AI` only when owner type has temporal and warp-away flags; otherwise parasite/damage path | Decompile branch checks owner type `+0xCCE` and `+0xD97` | Conditional; standard CLEG path Yes |
| `TemporalClass__AI @ 0x006297F0` | Visual attack state machine behind `UpdateAttack` | Decompile live this pass; constructor rows at `0x00629913`, `0x00629E90`, state 4 rows | Conditional; standard CLEG path Yes |
| `WarpAttachClass__SpawnWarpAnims @ 0x00629E90` | Three randomized `Wake` anim rows around target coords | Direct calls from temporal states 1 and post-damage retry; decompile + assembly `0x00629FA3..0x00629FAC` | Conditional |

## 6. Current Rust Implementation Status

Current Rust has parsed rule fields for `warp_in`, `warp_out`, `warp_away`, `chrono_sparkle1`, and `wake` in `src/rules/ruleset.rs`, but it does not have a generic `AnimClass` object with constructor-equivalent registration, owner attachment, draw flags, z-adjust, delay, and loop-count semantics.

Teleport movement in `src/sim/movement/teleport_movement.rs:35` models a simplified `Relocate -> ChronoDelay` state. `src/sim/miner/miner_system.rs:1055` emits two `WorldEffect` rows using `rules.general.warp_out`, but only from the chrono miner far-return helper and not from the generic teleport locomotor phase machine. `src/app_instances/units.rs:246` correctly avoids unit tinting for chrono teleport, but the visible warp anims are still ad-hoc world effects rather than global `AnimClass` objects with constructor semantics.

No Rust surface found for the temporal `SQDG` owner-attached anim, temporal `Wake` spark rows, or temporal state-4 rubble rows in this scope.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass__Constructor @ 0x00421EA0` argument/storage behavior | verified | Decompile live this pass | Full render semantics outside scope |
| `TeleportLocomotionClass__InitiateWarp @ 0x00719400` departure/arrival rows | verified | Decompile + assembly `0x00719420..0x00719442`, `0x007196B0..0x00719799` | Runtime capture could confirm player-observed chrono miner arrival row timing |
| `TeleportLocomotionClass__StateMachineTick @ 0x007192F0` external phases 2/5 | verified | Decompile + assembly `0x00719838..0x00719873`, `0x00719B41..0x00719B7C` | Full Chronosphere target eligibility outside scope |
| `TeleportLocomotionClass__ClearPendingWarpPhase @ 0x00719790` | verified | Decompile + assembly `0x00719790..0x00719799` | Exact entry condition beyond state-0 invalid target remains inherited from locomotion docs |
| `WarpAttachClass__SpawnWarpAnims @ 0x00629E90` | verified | Decompile + assembly `0x00629FA3..0x00629FAC`; PE string/rules parser for `Wake` | Pixel role of `WAKE1` visual not inspected |
| `WarpAttachClass__UpdateAttack @ 0x00629FD0` temporal dispatch | verified | Decompile branch checks owner type `+0xCCE/+0xD97` before `TemporalClass__AI` | Names of those flags inherited from prior temporal docs |
| `TemporalClass__AI @ 0x006297F0` constructor rows | verified | Decompile + assembly `0x006298CC..0x00629913`; decompile state-4 rows | Exact art frame data not inspected |
| Rust teleport/world-effect implementation | verified | `src/sim/movement/teleport_movement.rs`, `src/sim/miner/miner_system.rs`, `src/app_instances/units.rs` | No code changes made |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- Are the requested teleport entry points live in YR? -> Yes for active Teleport locomotor and Chronosphere/external warp state-machine paths.` (evidence: `StateMachineTick @ 0x007192F0`; chrono docs; `Teleporter=yes` stock INI entries)
- `[RESOLVED] OQ-2 -- Which rules key backs teleport-locomotor spawn rows? -> `[General] WarpOut`, `Rules+0x33C`, stock `WARPOUT`.` (evidence: `RulesClass__ReadGeneral @ 0x0066D530`; `ini/rulesmd.ini:549`; assembly reads `+0x33C`)
- `[RESOLVED] OQ-3 -- Does teleport self-warp use `WarpIn`, `WarpAway`, or `ChronoSparkle1` constructor rows? -> No for requested rows; those fields are parsed but not read by these constructor sites.` (evidence: call contexts read `+0x33C`; parser reads `+0x338/+0x340/+0x344`)
- `[RESOLVED] OQ-4 -- Are teleport anims owner-attached? -> No owner pointer is passed or set at the spawn sites; they are ordinary global `AnimClass` rows.` (evidence: constructor args at `0x00719442`, `0x00719873`, `0x00719B7C`; constructor global registration)
- `[RESOLVED] OQ-5 -- Does `ClearPendingWarpPhase` spawn before clearing pending phase? -> Yes, `CALL 0x00421EA0` precedes `MOV [EAX+0x280],0`.` (evidence: `0x00719790..0x00719799`)
- `[RESOLVED] OQ-6 -- Does chrono miner harvester special case suppress the departure anim? -> No; harvester timer clear occurs after the first `WarpOut` constructor row.` (evidence: `0x00719442` before harvester check in `InitiateWarp` decompile)
- `[RESOLVED] OQ-7 -- What does `WarpAttachClass__SpawnWarpAnims` spawn? -> Three randomized `[General] Wake` rows, not `ChronoSparkle1` or `WarpAway`.` (evidence: `0x00629FA3..0x00629FAC`; PE string `Wake` at `0x0083CF08`; `ini/rulesmd.ini:525`)
- `[RESOLVED] OQ-8 -- Does `UpdateAttack` always use temporal visuals? -> No; it dispatches to temporal AI only when owner type fields `+0xCCE` and `+0xD97` are set; otherwise it follows non-temporal parasite/damage visuals.` (evidence: `WarpAttachClass__UpdateAttack @ 0x00629FD0`)
- `[RESOLVED] OQ-9 -- Is there an attached temporal anim row? -> Yes, state 0 creates `SQDG`, stores it at `Temporal+0x44`, and later calls `AnimClass__SetOwnerObject(target)`.` (evidence: `0x006298CC..0x0062991C`; `TemporalClass__AI` tail)
- `[RESOLVED] OQ-10 -- Are temporal state-4 anims chrono rules keys? -> No; they use random rubble anims from `Rules+0xBC4` with delay `2` and z-adjust `-10`.` (evidence: `TemporalClass__AI @ 0x006297F0`)
- `[RESOLVED] OQ-11 -- Does current Rust have a generic `AnimClass` equivalent? -> No; it uses ad-hoc `WorldEffect` and no owner-attached temporal anim surface was found.` (evidence: `src/sim/miner/miner_system.rs:1055`; Rust search)
- `[DEFERRED] OQ-12 -- Does an actual retail runtime trace show the chrono miner arrival `WarpOut` row in the same player-observed scenario?` (category: `needs-runtime-debugger`; reason: binary row is present but old player-observation traces conflict; next-step-if-pursued: trace `AnimClass__Constructor` args during a CMIN return warp in gamemd)
- `[DEFERRED] OQ-13 -- What exact SHP frames/palettes compose `SQDG`, `WAKE1`, and rubble rows?` (category: `out-of-scope`; reason: this slot verifies spawn paths and constructor args, not frame rendering; next-step-if-pursued: render-path/frame audit)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `StateMachineTick/InitiateWarp @ 0x00719442` | Active Teleport target coord differs from current coord | `[General] WarpOut` stock `WARPOUT`, frame starts via constructor/Middle | World coords `Techno+0x9C/+0xA0/+0xA4` | `AnimClass` normal draw path, not audited here | Yes | Teleport departure overlay |
| 2 | `StateMachineTick/InitiateWarp @ 0x00719790/0x00719791` | No-destination cleanup | `[General] WarpOut` | World coords current | Not audited | Conditional | Cleanup overlay |
| 3 | `StateMachineTick phase 2 @ 0x00719873` | External warp phase 2 | `[General] WarpOut` | World coords current | Not audited | Yes for external warp | External departure/pre-relocation overlay |
| 4 | `StateMachineTick phase 5 @ 0x00719B7C` | Target alive after validation | `[General] WarpOut` | World coords current/destination | Not audited | Yes for surviving external warp | External arrival overlay |
| 5 | `TemporalClass__AI state 0 @ 0x00629913` | Temporal AI state 0 | `SQDG` | Target coords | Not audited | Yes for temporal attack | Persistent attached target visual |
| 6 | `WarpAttachClass__SpawnWarpAnims @ 0x00629FAC` | Temporal state transition calls helper; loop count 3 | `[General] Wake` stock `WAKE1` | Randomized around target coords | Not audited | Conditional | Temporal spark/wake bursts |
| 7 | `TemporalClass__AI state 4` | Temporal state 4 | Random `RubbleAnims` | Randomized around target coords | Not audited | Conditional | Damage/erase burst |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `WARPOUT` | Yes | Yes in teleport rows | Yes | No | No | Yes | Yes | No | `Rules+0x33C`, call contexts |
| `WARPIN` | Yes | Not by requested rows | No in this slice | No | No | No | No | Yes in this slice | `Rules+0x338`, no call reads |
| `WARPAWAY` | Yes | Not by requested teleport rows | No in this slice | No | No | No | No | Yes in this slice | `Rules+0x340`, no call reads |
| `CHRONOSK` | Yes | Not by requested rows | No in this slice | No | No | No | No | Yes in this slice | `Rules+0x344`, no call reads |
| `WAKE1` | Yes | Yes in `SpawnWarpAnims` | Conditional temporal | No | No | Yes | Yes | No | `Rules+0x94`, `0x00629FAC` |
| `SQDG` | Yes if art present | Yes in temporal state 0 | Conditional temporal | No | No | Yes, attached | Persistent during temporal state | No | `0x0083665C`, `0x00629913` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Teleport-locomotor warp anim rows are generic `AnimClass` objects using `[General] WarpOut` (`Rules+0x33C`) with args `delay=0`, `loops=1`, `flags=0x600`, `zAdjust=0`, `reverse=0`, at departure/arrival/external phase coords. | `0x00719442`, `0x00719873`, `0x00719B7C`; parser `0x0066D530`; `ini/rulesmd.ini:549` | Partial: only chrono miner far return pushes ad-hoc `WorldEffect` rows in `src/sim/miner/miner_system.rs:1055`; no generic constructor/state-machine spawn surface | Needed sim/world visual-event or generic anim surface; current `world_effects` bridge if kept must preserve args/order | Emit anim rows from generic teleport state transitions in binary order, with harvester timer special case not suppressing departure spawn | `teleport_locomotor_spawns_warpout_anim_rows_with_constructor_args` | Do not map teleport rows to `WarpAway` or `WarpIn`; do not tint units as the anim substitute |
| `ClearPendingWarpPhase`/no-destination cleanup spawns `WarpOut` before clearing `Techno+0x280`. | `0x00719790..0x00719799` | Missing/unchecked: Rust simplified teleport state has no pending-phase cleanup row | `src/sim/movement/teleport_movement.rs` and any future pending warp phase model | Preserve side-effect order: spawn first, clear pending second | Force invalid pending destination and assert one `WarpOut` anim event occurs before pending state clears | `teleport_clear_pending_spawns_warpout_before_state_clear` | Do not treat cleanup as silent because destination is invalid |
| Temporal/Chrono Legionnaire visual branch is not teleport `WarpOut`: it creates attached `SQDG`, three `[General] Wake` rows, and state-4 rubble rows; `UpdateAttack` dispatches to this only for temporal+warp-away owner type. | `0x00629FD0`, `0x00629913`, `0x00629FAC`, `TemporalClass__AI state 4`; `ini/rulesmd.ini:525` | Missing: no temporal `SQDG` owner-attached anim or `Wake`/rubble visual rows found | Future temporal weapon/warping target system, generic `AnimClass` ownership/listener support | Implement temporal visual state machine separately from teleport locomotor anims; owner-attach persistent anim to target and remove it on temporal cleanup | CLEG attacks a warpable target: first visual row is attached `SQDG`, later helper emits exactly three `Wake` rows | `temporal_attack_uses_sqdg_attached_anim_and_wake_bursts_not_warpout` | Do not reuse teleport `WarpOut`/`ChronoSparkle1` for temporal sparks |

### Negative Facts / Do Not Do

- Do not implement self-teleport visuals with `[General] WarpAway`; the requested teleport constructor sites read `Rules+0x33C` (`WarpOut`), while `WarpAway` is parsed at `+0x340` and not read by these call sites. Evidence: `RulesClass__ReadGeneral @ 0x0066D530`; `0x00719439`, `0x0071986A`, `0x00719B73`.
- Do not implement `[General] WarpIn` as the arrival anim for these rows; arrival/external phase rows still read `Rules+0x33C`. Evidence: `0x00719790`, `0x00719B73`.
- Do not use `ChronoSparkle1` for `WarpAttachClass__SpawnWarpAnims`; the helper reads `Rules+0x94` (`Wake`) and loops three times. Evidence: `0x00629FA3..0x00629FAC`; PE string `Wake` at `0x0083CF08`; `ini/rulesmd.ini:525`.
- Do not replace temporal `SQDG` with a free-floating world effect; `TemporalClass__AI` stores the constructor return at `Temporal+0x44` and later calls `AnimClass__SetOwnerObject(target)`. Evidence: `0x00629913..0x0062991C`; `TemporalClass__AI` tail.
- Do not make harvester zero-delay suppress the departure `WarpOut`; the constructor call occurs before the harvester type check clears the timer and `BeingWarped`. Evidence: `0x00719442` before `InitiateWarp` harvester branch.

### Stale Docs / Follow-up Docs

- `docs/research/CHRONO_WARP_VISUAL_RENDERING.md` should replace any wording saying teleport/self-warp uses `WarpAway` or `WarpIn` with: "For the requested `TeleportLocomotionClass` constructor rows, gamemd reads `[General] WarpOut` at `RulesClass+0x33C` and constructs `AnimClass(type=WarpOut, coords=current, delay=0, loops=1, flags=0x600, zAdjust=0, reverse=0)`. `[General] WarpIn`, `[General] WarpAway`, and `[General] ChronoSparkle1` are parsed but are not used by these teleport-locomotor constructor sites."
- `docs/research/TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md` should replace "WarpAway animation (Rules+0x33C)" with: "`Rules+0x33C` is `[General] WarpOut`; the self-teleport and external warp constructor rows observed in this slice use `WarpOut`, not `WarpAway`."
- `docs/research/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md` should replace "WarpAway at departure/arrival" with: "Binary constructor rows use `[General] WarpOut` (`Rules+0x33C`) at the teleport-locomotor spawn sites; arrival-row player visibility for the chrono miner return scenario remains a runtime-trace follow-up if contradicted by captures."

## Sources

- Ghidra decompiled: `AnimClass__Constructor @ 0x00421EA0`; `WarpAttachClass__SpawnWarpAnims @ 0x00629E90`; `WarpAttachClass__UpdateAttack @ 0x00629FD0`; `TemporalClass__AI @ 0x006297F0`; `TeleportLocomotionClass__StateMachineTick @ 0x007192F0`; `TeleportLocomotionClass__InitiateWarp @ 0x00719400`; `TeleportLocomotionClass__ClearPendingWarpPhase @ 0x00719790`; `RulesClass__ReadGeneral @ 0x0066D530`.
- Ghidra assembly contexts: `0x00719420..0x00719442`, `0x00719790..0x00719799`, `0x00719838..0x00719873`, `0x00719B41..0x00719B7C`, `0x006298CC..0x00629913`, `0x00629FA3..0x00629FAC`, `0x0062A150..0x0062A165`.
- INI checked: `ini/rulesmd.ini:525`, `548`, `549`, `550`, `554`.
- Rust scanned: `src/rules/ruleset.rs`, `src/sim/movement/teleport_movement.rs`, `src/sim/miner/miner_system.rs`, `src/app_instances/units.rs`.
- Prior docs referenced: `docs/research/CHRONO_WARP_VISUAL_RENDERING.md`; `docs/research/TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md`; `docs/research/chronominer-locomotion/enum-state-machine-states.md`; `docs/research/chronominer-locomotion/fn-clear-pending-warp-phase.md`; `docs/research/RULESCLASS_FIELDS.csv`.
