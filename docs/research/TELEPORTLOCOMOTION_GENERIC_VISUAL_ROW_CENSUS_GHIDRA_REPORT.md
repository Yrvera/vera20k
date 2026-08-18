# TeleportLocomotion Generic Visual Row Census -- Ghidra Research Report

**Address(es):** `0x00718100`, `0x00718B70`, `0x007192F0`, `0x00719400`, `0x00719790`, `0x00719BF0`, `0x00421EA0`, `0x0066D530`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Active `TeleportLocomotionClass` visual-row producers and non-producers in standard YR: self-teleport, pending/cleanup tail, external Chronosphere/ChronoWarp phases, abort paths, coordinate sources, constructor row arguments, and ordering relative to `TechnoClass+0x280`.  
**Non-Scope:** Temporal weapon `WarpAttachClass` visuals, render/blitter pixel math, chrono miner routing/refinery target selection, full superweapon targeting, and generic `AnimClass` lifecycle outside constructor row evidence.  
**Confidence:** High for the TeleportLocomotion constructor census and ordering.  
**Active in YR:** Yes, conditional on stock units/superweapons that use `TeleportLocomotionClass` or set `TechnoClass+0x280` external warp state.

Working notes gate:
- `Target question`: Which active `TeleportLocomotionClass` paths produce visual `AnimClass` constructor rows, what assets/coords/args do they use, and what paths produce no visual rows?
- `Non-goals`: Do not investigate temporal weapon `WarpAttachClass` except as a negative boundary; do not re-open generic AnimClass rendering.
- `Evidence needed to mark COMPLETE`: Decompile plus assembly/disassembly address ranges for each constructor producer, binary call-site census for missed producers, INI/default source for assets, and Rust surface scan.
- `Stop conditions`: Stop when every constructor call in the TeleportLocomotion VA range is classified and abort/cleanup branches are either row-producing or negative.

## 1. Overview

The active `TeleportLocomotionClass` visual-row census contains exactly four direct `AnimClass__Constructor @ 0x00421EA0` call sites in the `0x00718000..0x0071A100` TeleportLocomotion code range: self-teleport departure, self-teleport/clear-pending tail, external phase 2 departure, and external phase 5 arrival. All four use `[General] WarpOut` from `RulesClass+0x33C`; none use `[General] WarpIn`, `[General] WarpAway`, `[General] ChronoSparkle1`, `ChronoBlast`, or temporal `SQDG`.

`ClearPendingWarpPhase @ 0x00719790` is a row-producing cleanup tail: it calls the anim constructor first, then clears `TechnoClass+0x280`. Other abort/cleanup helpers such as invalid destination handling, `Stop_Moving`, `TimerCheck`, and phase 7 completion do not spawn visual rows in this slice.

## 2. Key Offsets and Fields

| Offset | Owner | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `+0x9C/+0xA0/+0xA4` | `TechnoClass` | Current world coords copied into spawn row coords | Yes | `0x00719420..0x00719442`, `0x00719851..0x00719873`, `0x00719B56..0x00719B7C` |
| `+0x270` | `TechnoClass` | WarpingOut byte set in external phase 0 and cleared in phase 2 | Conditional | `0x007197D0`, `StateMachineTick @ 0x007198DA` |
| `+0x271` | `TechnoClass` | BeingWarped byte set during self/external warp and cleared by timer/state 7 | Yes | `StateMachineTick @ 0x00719573`, `0x007198DA`, `TimerCheck @ 0x00719C12` |
| `+0x27C` | `TechnoClass` | ChronoInTransit gate for external multi-phase path | Conditional | `StateMachineTick @ 0x007192F0`, phase 2 clears |
| `+0x280` | `TechnoClass` | Pending external warp phase; cleared after row at `0x00719791` and in phase 7 | Conditional | `0x00719791..0x00719799`, `0x00719BD2..0x00719BE2` |
| `+0x284` | `TechnoClass` | External warp lock duration copied to locomotor timer in phase 5 | Conditional | `StateMachineTick @ 0x00719B1D..0x00719B4C` |
| `+0x33C` | `RulesClass` | `[General] WarpOut` `AnimTypeClass*` used by every TeleportLocomotion row | Yes | `0x00719439`, `0x00719788`, `0x0071986A`, `0x00719B73`; `ini/rulesmd.ini:549` |

## 3. Constructor Row Census

Binary PE scan over `gamemd.exe` `.text` for calls targeting `AnimClass__Constructor @ 0x00421EA0` inside `0x00718000..0x0071A100` returned exactly: `0x00719442`, `0x00719791`, `0x00719873`, `0x00719B7C`. Active in YR: Yes/Conditional per row below. Evidence: local PE call-target scan plus read-only Ghidra decompile/disassembly contexts.

| Row | Function / address | Asset | Coords | Constructor args after type/coords | Ordering | Active in YR |
|---|---|---|---|---|---|---|
| 1 | `StateMachineTick` / split `InitiateWarp`, `CALL 0x00421EA0 @ 0x00719442` | `Rules+0x33C` `[General] WarpOut` | Current `Techno+0x9C/+0xA0/+0xA4` before relocation | defaulted stack row resolves to `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0` by constructor call convention | Before timer computation, `BeingWarped=1`, harvester zero-delay special case, detach, unmark/move/mark, sounds, and arrival row | Yes, when active Teleport locomotor has a valid non-null destination different from current coords |
| 2 | `ClearPendingWarpPhase` / self-teleport tail, `CALL 0x00421EA0 @ 0x00719791` | `Rules+0x33C` `[General] WarpOut` | Current coords after relocation for normal self-teleport; caller-prepared coords for clear-pending entry | `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0` | Immediately before `MOV [EAX+0x280],0` at `0x00719799` | Yes for normal self-teleport arrival; Conditional for pending cleanup |
| 3 | External phase 2, `CALL 0x00421EA0 @ 0x00719873` | `Rules+0x33C` `[General] WarpOut` | Current `Techno+0x9C/+0xA0/+0xA4` before external relocation | `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0` | Before unmark, `ChronoOutSound`, `BeingWarped=1`, clearing `+0x27C/+0x270/+0x8C`, and `Update_Position` | Conditional, active for Chronosphere/ChronoWarp external path phase 2 |
| 4 | External phase 5, `CALL 0x00421EA0 @ 0x00719B7C` | `Rules+0x33C` `[General] WarpOut` | Current/destination `Techno+0x9C/+0xA0/+0xA4` after placement/validation | `delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0` | After post-warp validation, mission/ghost/occupation setup, timer arm from `Techno+0x284`, and before phase increments to 6 | Conditional, active if external-warped object survives validation |

Assembly load-bearing contexts:
- `0x00719439..0x00719442`: `MOV EDX,[Rules+0x33c]`, `CALL 0x00421ea0`.
- `0x00719782..0x00719799`: load `Rules+0x33c`, call constructor at `0x00719791`, then clear `Techno+0x280` at `0x00719799`.
- `0x00719864..0x00719873`: load `Rules+0x33c`, call constructor before external phase 2 unmark/sound/flag clears.
- `0x00719B6D..0x00719B88`: load `Rules+0x33c`, call constructor, increment phase.

## 4. Negative Branch Census

| Branch / function | Visual row? | Active in YR | Evidence |
|---|---:|---|---|
| `HeadToCoord @ 0x00718100` guard aborts (`IsWarpingOut`, `IsWarpingIn`, deploying, undeploying) | No | Conditional | Decompile clears `Techno+0x5A4` and returns; no constructor call in function |
| `HeadToCoord` invalid destination after `Process` | No | Conditional | Decompile calls `vtable+0x480(0,1)` and returns when `DestCoord == NullCoord`; no constructor call |
| `Process @ 0x00718B70` destination resolution | No | Yes | Decompile validates/snap coords only; no constructor call |
| `Stop_Moving / ClearCoords @ 0x00718230` | No | Conditional | Existing decompile in `chronominer-locomotion/fn-accessors.md` clears coords/flags only |
| `TimerCheck @ 0x00719BF0` post-warp cooldown expiry | No | Yes | Decompile clears `Techno+0x271`, reacquires target/mission; no constructor call |
| External phase 0 (`0x007197D0`) | No | Conditional | Assembly sets `Techno+0x270=1`, timer duration `0x3C`, phase increment; no constructor call |
| External phase 1 / 6 wait states | No | Conditional | `StateMachineTick` calls `TimerCheck`; no constructor in wait states |
| External phase 7 cleanup | No | Conditional | Clears `BeingWarped`, ghost cell, `+0x280`, phase 0; no constructor call |

## 5. INI Keys

| INI key | Stock YR value | Binary field | Used by TeleportLocomotion row? | Active in YR | Evidence |
|---|---|---:|---|---|---|
| `[General] WarpOut` | `WARPOUT;WAKE2` parsed as `WARPOUT` by current Rust comment; native field stores the AnimType pointer | `Rules+0x33C` | Yes, every row | Yes | `ini/rulesmd.ini:549`; `RulesClass__ReadGeneral @ 0x0066D530`; row contexts above |
| `[General] WarpIn` | `WARPIN;WAKE2` | `Rules+0x338` | No | No in this slice | `ini/rulesmd.ini:548`; no constructor context reads `+0x338` |
| `[General] WarpAway` | `WARPAWAY;RING1` | `Rules+0x340` | No | No in this slice | `ini/rulesmd.ini:550`; no constructor context reads `+0x340` |
| `[General] ChronoSparkle1` | `CHRONOSK` | `Rules+0x344` | No | No in this slice | `ini/rulesmd.ini:554`; no constructor context reads `+0x344` |
| `[General] ChronoInSound` / `ChronoOutSound` | `ChronoMinerTeleport` | `Rules+0x218/+0x21C` | Sound only, not visual row | Yes | `StateMachineTick` decompile; `ini/rulesmd.ini:660..661` |

## 6. Current Rust Implementation Status

Rust now has `AnimClassSpawnDescriptor` and a teleport visual bridge in `src/sim/movement/teleport_movement.rs`. The bridge preserves `type`, coords, `delay=0`, `loop_count=1`, `draw_flags=0x600`, `z_adjust=0`, and `reverse=false` when spawning `rules.general.warp_out`. That matches the constructor row fields for self-teleport rows.

The remaining Rust delta is state coverage and mechanism: `tick_teleport_movement` models a simplified `Relocate -> ChronoDelay` machine and spawns two `WorldEffect` rows there, but it does not model native `Techno+0x280` pending external phases 0..7, `ClearPendingWarpPhase` as a generic state-machine row before clear, or native active `AnimClass` scheduler/lifetime semantics. `src/rules/ruleset.rs` parses `warp_in`, `warp_out`, `warp_away`, `chrono_sparkle1`, and sounds; this report proves only `warp_out` is used by TeleportLocomotion visual rows.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct constructor calls in `0x00718000..0x0071A100` | verified | PE call-target scan found exactly `0x00719442`, `0x00719791`, `0x00719873`, `0x00719B7C` | none |
| Self-teleport departure/arrival | verified | `StateMachineTick @ 0x007192F0`, `InitiateWarp @ 0x00719400`, assembly contexts | Runtime visual screenshot not needed for constructor census |
| `ClearPendingWarpPhase` ordering | verified | Decompile `0x00719790`; assembly `0x00719791` before `0x00719799` | none |
| External phases 0..7 row/non-row classification | verified | `StateMachineTick @ 0x007192F0`, assembly `0x00719873`, `0x00719B7C`, TimerCheck decompile | full Chronosphere target selection out of scope |
| Abort/invalid destination paths | verified | `HeadToCoord @ 0x00718100`, `Process @ 0x00718B70`, PE constructor-call census | none for visual rows |
| Temporal `WarpAttachClass` boundary | deferred | prior `ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md` | out of scope; separate temporal report owns it |
| Rust generic AnimClass scheduler/lifetime | touched-not-exhausted | `src/sim/components.rs`, `src/sim/movement/teleport_movement.rs` scan | separate implementation contract/fix |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- How many active TeleportLocomotion constructor row sites exist? -> Four direct calls in the TeleportLocomotion VA range.` (evidence: PE call-target scan; Ghidra contexts)
- `[RESOLVED] OQ-2 -- Which asset backs every TeleportLocomotion row? -> `[General] WarpOut`, `Rules+0x33C`.` (evidence: `0x00719439`, `0x00719788`, `0x0071986A`, `0x00719B73`)
- `[RESOLVED] OQ-3 -- Does arrival use `WarpIn`? -> No, the arrival/tail site reads `Rules+0x33C`, not `+0x338`.` (evidence: `0x00719788..0x00719791`)
- `[RESOLVED] OQ-4 -- Does any TeleportLocomotion row use `WarpAway` or `ChronoSparkle1`? -> No for this slice.` (evidence: constructor contexts read `+0x33C`; no call context reads `+0x340/+0x344`)
- `[RESOLVED] OQ-5 -- Is ClearPending visual-before-clear? -> Yes, constructor call precedes `Techno+0x280=0`.` (evidence: `0x00719791..0x00719799`)
- `[RESOLVED] OQ-6 -- Do invalid-destination/abort paths spawn a consolation row? -> No.` (evidence: `HeadToCoord @ 0x00718100`; constructor-call census)
- `[RESOLVED] OQ-7 -- Does harvester zero-delay suppress the departure row? -> No, departure constructor is before the harvester branch.` (evidence: `StateMachineTick` decompile)
- `[RESOLVED] OQ-8 -- Do wait/timer cleanup states spawn rows? -> No.` (evidence: `TimerCheck @ 0x00719BF0`; `StateMachineTick` state 1/6/7)
- `[DEFERRED] OQ-9 -- Exact player-visible composition/frame data for `WARPOUT.SHP`.` (category: `out-of-scope`; reason: spawn row census only; next-step-if-pursued: AnimClass draw/frame audit)
- `[DEFERRED] OQ-10 -- Temporal weapon `WarpAttachClass` visual rows.` (category: `out-of-scope`; reason: explicit negative boundary; next-step-if-pursued: verify temporal-specific report)

## 9. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset | Anchor | Active for target? | Role |
|---|---|---|---|---|---|---|
| 1 | `0x00719442` | active self-teleport with non-null different target | `[General] WarpOut` | source/current coords | Yes | self-teleport departure overlay |
| 2 | `0x00719791` | self-teleport after relocation, or clear-pending tail | `[General] WarpOut` | current/destination or caller-prepared coords | Yes/Conditional | arrival/cleanup overlay before `+0x280` clear |
| 3 | `0x00719873` | external phase 2 | `[General] WarpOut` | current coords before relocation | Conditional | external departure overlay |
| 4 | `0x00719B7C` | external phase 5 and object alive | `[General] WarpOut` | current/destination coords | Conditional | external arrival overlay |

Asset role matrix:

| Asset | Loaded/parsed | Drawn by TeleportLocomotion rows | Visible in target | Inactive in this slice | Evidence |
|---|---:|---:|---:|---:|---|
| `WARPOUT` | Yes | Yes | Yes | No | `Rules+0x33C`; four constructor contexts |
| `WARPIN` | Yes | No | No | Yes | `Rules+0x338`; no constructor context |
| `WARPAWAY` | Yes | No | No | Yes | `Rules+0x340`; no constructor context |
| `CHRONOSK` / `ChronoSparkle1` | Yes | No | No | Yes | `Rules+0x344`; no constructor context |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| All TeleportLocomotion visual rows use generic `AnimClass(type=Rules+0x33C WarpOut, coords=current, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`. | `0x00719442`, `0x00719791`, `0x00719873`, `0x00719B7C`; `ini/rulesmd.ini:549` | Partial: row fields exist in `TeleportVisuals`, but still bridge through `WorldEffect` rather than native AnimClass lifecycle | `src/sim/movement/teleport_movement.rs`, `src/sim/components.rs`, future generic anim runtime | Preserve row fields and insertion order for self and external paths | Self-teleport emits exactly two `warp_out` descriptors with native row fields and no `warp_in`/`warp_away` | `teleport_locomotor_visual_rows_use_warpout_constructor_args` | Do not collapse to generic `WorldEffect` defaults or infer asset by "in/out" name |
| Clear-pending tail spawns before clearing `Techno+0x280`. | `0x00719791..0x00719799` | Missing/unchecked: Rust has no native pending phase field/order | generic teleport state machine surface | Spawn row event must be observable before pending-state clear in the same tick | Invalid/external pending cleanup produces one `WarpOut` row, then clears pending phase | `teleport_clear_pending_spawns_warpout_before_pending_clear` | Do not make invalid destination cleanup silent |
| Abort/invalid destination, wait states, `TimerCheck`, and phase 7 cleanup spawn no visual rows. | `HeadToCoord @ 0x00718100`; `TimerCheck @ 0x00719BF0`; constructor-call census | Simplified Rust may accidentally emit rows only from `Relocate`; future external-state work must keep negatives explicit | `src/sim/movement/teleport_movement.rs` | Only the four verified row sites may produce TeleportLocomotion rows | Guard-aborted HeadToCoord and timer expiry produce no anim descriptors | `teleport_abort_and_timer_cleanup_do_not_spawn_visual_rows` | Do not add "cleanup sparkle" or `ChronoSparkle1` to fill perceived gaps |

### Negative Facts / Do Not Do

- Do not use `[General] WarpIn` for TeleportLocomotion arrival. The arrival/tail constructor reads `Rules+0x33C`, not `+0x338`. Active in YR: No for this slice. Evidence: `0x00719788..0x00719791`.
- Do not use `[General] WarpAway` for self-teleport or external warp rows. Active in YR: No for this slice. Evidence: all four constructor contexts read `+0x33C`.
- Do not use `[General] ChronoSparkle1` for TeleportLocomotion rows. Active in YR: No for this slice. Evidence: no constructor context reads `Rules+0x344`.
- Do not suppress departure `WarpOut` for Chrono Miner because it has zero post-warp delay. Active in YR: Yes. Evidence: departure row precedes the harvester branch in `StateMachineTick`.
- Do not attach these TeleportLocomotion rows to the owner object. Active in YR: No attachment for this slice. Evidence: constructor row sites pass ordinary coords and no later `AnimClass__SetOwnerObject` call is in the TeleportLocomotion path.

### Stale Docs / Replacement Wording

- `docs/research/chronominer-locomotion/enum-state-machine-states.md`: replace "state 5 ... spawns WarpIn anim at destination Location" with "state 5 spawns `[General] WarpOut` (`RulesClass+0x33C`) at the destination/current location; `WarpIn` is parsed but not used by this TeleportLocomotion constructor site."
- `docs/research/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md`: replace any "WarpAway at departure/arrival" wording with "TeleportLocomotion self-warp constructor sites use `[General] WarpOut` (`RulesClass+0x33C`) at both departure and arrival, with `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`."
- `docs/research/miner/traces/CHRONO_MINER_TOO_FAR_THRESHOLD_BRANCH_TRACE.md`: replace "Matches gamemd ... WarpAway at both endpoints" and "gamemd spawns WarpAway" with "gamemd spawns `[General] WarpOut` (`RulesClass+0x33C`) at both endpoints; current Rust should use the generic AnimClass-like descriptor fields rather than WorldEffect defaults."
- `docs/research/miner/traces/CHRONO_MINER_LOCOMOTION_DRIVE_PHASE_TRACE.md`: replace "Missing WarpAway animation on ore approach" with "If the TeleportLocomotion self-warp path is armed, the visual row is `[General] WarpOut`, not `WarpAway`; close/drive-only paths that never arm TeleportLocomotion produce no teleport visual row."
- `docs/research/miner/traces/MINER_STUCK_FINAL_APPROACH_ADJACENT_TO_ORE_TRACE.md`: replace "Phase 0 spawns WarpAway anim" with "Phase 0 spawns `[General] WarpOut` (`RulesClass+0x33C`) if and only if TeleportLocomotion is armed with a valid non-null destination."

## Sources

- Ghidra read-only decompile: `TeleportLocomotionClass__HeadToCoord @ 0x00718100`; `TeleportLocomotionClass__Process @ 0x00718B70`; `TeleportLocomotionClass__StateMachineTick @ 0x007192F0`; split `InitiateWarp @ 0x00719400`; split `ClearPendingWarpPhase @ 0x00719790`; `TimerCheck @ 0x00719BF0`; `AnimClass__Constructor @ 0x00421EA0`; `RulesClass__ReadGeneral @ 0x0066D530`.
- Ghidra assembly/disassembly contexts: `0x00719439..0x00719442`; `0x00719782..0x00719799`; `0x00719864..0x00719873`; `0x00719B6D..0x00719B88`; `0x007197D0..0x007197F3`.
- Binary PE call-target census: `gamemd.exe` direct calls to `0x00421EA0` inside `0x00718000..0x0071A100` = `0x00719442`, `0x00719791`, `0x00719873`, `0x00719B7C`.
- INI checked: `ini/rulesmd.ini:548..550`, `554`, `660..661`; base `ini/rules.ini` has same key meanings.
- Rust scanned: `src/sim/movement/teleport_movement.rs`, `src/sim/components.rs`, `src/rules/ruleset.rs`, `src/sim/world/mod.rs`, `src/sim/miner/miner_system.rs`.
