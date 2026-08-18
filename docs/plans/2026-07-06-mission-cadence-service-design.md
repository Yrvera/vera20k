# Mission-Cadence Service (W1) — Design

> Status: **DESIGN — awaiting approval.** No code written. Produced by /brainstorm 2026-07-06.
> Research bundle: `scratchpad/w1-arch-map.md`, `w1-gamemd-ledger.md`, `w1-substrate-fit.md`
> (this session). Source scan: `docs/gap-scans/2026-07-02-disparity-scan-miner.md` (G5, G6, M1,
> M5, L1, L9, L10, L20, S12). Roadmap: `docs/plans/2026-07-02-miner-parity-roadmap.md §W1`.

## Goal

Give the parsed-but-unconsumed `MissionControl` Rate table a consumer on the miner dispatch
path, so the miner FSM + refinery radio handshake are **mission-timer-paced** (gamemd's
~14–16 frame cadence) instead of running every sim tick — and restore gamemd's per-dispatch
`RandomRanged(0,2)` draw pattern for lockstep-stream parity.

## Architecture Context

- **Miner tick** runs in Phase 7 (production) of `World::advance_tick`
  (`world/mod.rs:2549…` → `production_economy.rs:21` → `miner_system::tick_miners_with_overlay_registry`,
  `miner_system.rs:98`). It snapshots every live, non-dying, non-Slave miner in native
  LogicVector order and dispatches `process_miner` (`miner_system.rs:253`, 8 states). **No
  per-miner cadence gate exists** — Harvest/WaitNoOre self-gate on a `MissionTimer`, every
  other state runs unconditionally (arch-map §2).
- **Dock phase machine** (`miner_dock_sequence.rs:716`, `RefineryDockPhase`): `Approach`
  re-sends `HELLO` **every tick with no timer** (`:826`); `MissionEnter` is `dock_enter_retry`-gated;
  `Pivoting`/`Unloading` are `mission_deploy_timer`-gated; `bus_enter_dock(0x18)` fires *before*
  the due gate (`:992`); `tick_unload_accumulator` runs unconditionally at the tail.
- **`MissionControl`** (`mission/control.rs:74`) is a `BTreeMap<MissionType, MissionControlEntry>`,
  one entry per dispatched mission id, `rate_frames`/`aa_rate_frames` pre-converted to integer
  frames at parse (`Rate × 900`, `control.rs:20/24`). Built on `RuleSet.mission_control`
  (`ruleset.rs:1695/1898`). **Zero non-test readers** (substrate-fit §3, grep-confirmed).
- **Mission/radio substrate has SHIPPED** (the 25-day `project_substrate_migration_program`
  memory was stale): `MissionCom` (`mission/mod.rs:190`, has a `timer: MissionTimer`) is
  authoritative + hashed on `GameEntity` (`game_entity.rs:500`); transitions commit via the
  pure verb API (`verb.rs:71…`); `object_ai_stage` (`techno_ai.rs:68`, pre-movement) is a live
  per-object dispatch host for non-miner Units.
- **`MissionTimer`** (`mission/timer.rs:41`) already models gamemd's exact due semantics:
  inclusive `now - start >= duration`, `SENTINEL=u32::MAX`=always-due, frame-anchored on
  `sim.session.binary_frame`. The miner already runs this pattern (`harvest_timer`,
  `rescan_cooldown`, `dock_enter_retry`, `mission_deploy_timer`) — **the primitive is a direct
  port; W1 wires consumers, it does not invent timer math** (ledger §25).

## Impact Analysis

**Touches:** `sim/mission/control.rs` (add ftol conversion + an accessor), `sim/miner/mod.rs`
(timer seeding), `sim/miner/miner_system.rs` (G5 accepted-HELLO arm; approach-HELLO gate),
`sim/miner/miner_dock_sequence.rs` (14/14 constants → table; L9/L10 draws; L20 gate),
`sim/rules/ruleset.rs` (pass `mission_control` to the miner path if not already reachable).

**Depends on this:** the W2 dock-protocol pass (M4/M5/M6) builds on this cadence seam; W3 dock
search re-runs each state-2 evaluation (needs the timer boundary). Do W1 first.

**Blast radius / risk:**
- **State-hash NOT neutral.** Two effects change the hash: (a) cadence shifts miner timing;
  (b) the L9/L10 fixes add `RandomRanged(0,2)` draws that reorder the RNG stream, which cascades
  into every downstream draw. **W1 requires ONE documented golden re-baseline** (one session at
  a time, per CLAUDE.md). This is not avoidable and must be planned, not discovered.
- **Determinism-critical:** the added RNG draws must land on the **correct gamemd RNG instance**
  (`Scen->Random` vs `g_MainRng`) — currently UNKNOWN (ledger U1). Landing them on the wrong
  instance is internally deterministic but drifts vs gamemd. See "Gates" below.
- **Slave path (S12) is OUT of W1 scope** — the "every 10 frames" limit is scan-asserted, not
  Ghidra-verified; `SlaveManager (0x006AFD60)` has no research doc (roadmap W11 gates on RE
  first). Deferred, not cut (ledger §23, U4).

## Chosen Approach — **A: hybrid Rate-consumer seam on the miner dispatch timers, shadow-first**

Cadence VALUE is owned centrally (`mission_control.rate_frames(mission)`, finally consuming the
table); gate STATE + CHECK stay per-object on the miner's existing `MissionTimer`s at their
current Phase-7 sites. No new phase, no reorder, no new iteration order. Landed in three
sub-steps, gated as noted, behind a shadow-first divergence log before the authoritative flip.

**Step A — data (L1) [stock-identical]:** add a truncating conversion to model `Math::ftol`
(`rate_frames` via `(minutes * 900.0) as u32` truncate, not `.round()`), and replace the miner
`ENTER_RETRY_BASE_FRAMES=14` / `MISSION_DEPLOY_UNLOAD_BASE_FRAMES=14` hardcodes
(`miner_dock_sequence.rs:53/56`) with `mission_control.rate_frames(Enter)` /
`rate_frames(Unload)`. **Keep `MISSION_DEPLOY_FACING_WAIT_FRAMES=5` a literal** — it is gamemd's
real `return 5` facing-not-ready constant, not a Rate, and consumes no RNG (ledger §8).

**Step B — cadence gates (G5, G6, L20) [changes hash]:**
- G5: at the accepted close-HELLO, replace `dock_enter_retry.clear()` (SENTINEL/always-due,
  `miner_system.rs:975`) with `arm(binary_frame, enter_rate + RandomRanged(0,2))` so the first
  CAN_DOCK waits the Enter cadence (the 15–17 frame window), not next tick.
- G6: gate the `phase_approach` re-HELLO (`:826`) behind a Harvest-mission-cadence timer armed
  `[Harvest] Rate×900 + RandomRanged(0,2)` per the harvest epilogue — one HELLO per due dispatch,
  not per tick.
- L20: move `bus_enter_dock(0x18)` (`:992`) to fire *after* the due-dispatch gate — one 0x18
  per 0x0E pass, not every arrived tick.

**Step C — RNG draws (L9, L10) [changes hash; GATED on U1]:**
- L9: draw one `RandomRanged(0,2)` on the accepted FaceSync→MissionQueued handoff
  (`miner_dock_sequence.rs:999`) — restores the missing per-dock-cycle draw.
- L10: draw one `RandomRanged(0,2)` at the state-4 exit epilogue (`:1274`) + apply its 0–2 frame
  jitter.

## Tiny-Detail Ledger (constraint set carried to /write-plan)

Every item cites the research bundle / doc / `[ini:]`. Full detail + sources in
`scratchpad/w1-gamemd-ledger.md`.

1. **Rate → frames = `Math::ftol(Rate × 900.0)` truncate-toward-zero**, computed at dispatch
   (gamemd `0x004D9473`); Rust `.round()` (`control.rs:25`) is the L1 drift — stock-identical
   (`.016×900=14.4→14` both ways), diverges only for modded rates with `≥.5` fraction. [ledger §5–7]
2. **Stock rates:** `[Enter]/[Unload]/[Harvest] Rate=.016 → 14`; `[Guard] Rate=.030 / AARate=.016
   → 27 / 14` `[ini: rulesmd.ini:30510/30557/30529/30503-04]`. [ledger §4]
3. **Enter epilogue re-arm = `ftol(Rate×900) + RandomRanged(0,2)` = 14/15/16 frames**; the base is
   computed FIRST, then the draw, then `ADD`, before `Mission_Dispatch` stores the return. [ledger §10,16]
4. **Mission_Enter sends exactly ONE `CAN_DOCK(0x0E)` per due dispatch** — no per-tick resend loop;
   the retry gate is entirely the `+0xC8/+0xD0` timer. [ledger §9]
5. **Harvest (mission 10) uses the SAME epilogue** — `[Harvest] Rate×900 + RandomRanged(0,2)`; every
   state-2/3 harvest pass draws one. HELLO→first CAN_DOCK ≈ **15–17 frames** (harvest cadence +
   ~1-frame commence-to-dispatch). [ledger §11,12,17]
6. **`Queue_Mission(7,0)` does NOT dispatch synchronously** — it sets queued mission, `Commence`
   clears `+0xD0=0` (immediately due next dispatch); first CAN_DOCK is the next `Mission_Dispatch`
   pass. Rust's always-due clear collapses this to next-tick (G5). [ledger §13,14]
7. **The `5` facing-wait is a literal `return 5`, NOT a Rate, draws NO RNG** — keep it hardcoded.
   [ledger §8, `MISSION_0X10_RETURN_DELAY §1`]
8. **One `RandomRanged(0,2)` per Enter / per Harvest / per accepted Unload dispatch; the
   facing-not-ready `return 5` branch draws none.** Inclusive `0..2`, callee `@0x0065C7E0`. [ledger §16–18]
9. **L9:** the accepted FaceSync→MissionQueued handoff is still a Mission_Enter dispatch in gamemd
   and draws one `RandomRanged(0,2)`; Rust skips it (`miner_dock_sequence.rs:999-1001`) — 1 missing
   draw per dock cycle. The un-accepted branch (`:1003`) already draws correctly. [ledger §19]
10. **L10:** state-4 exit draws one `Random(0,2)` + applies 0–2 frame jitter; Rust does neither
    (`:1274-1275`). [ledger §20]
11. **L20:** `EnterDock(0x18)` is one-per-0x0E-pass (only inside a `0x0E→0x12(==0x14)` handshake),
    ~every 14–16 frames; Rust fires it every arrived tick (`:992`, before the `:995` gate). [ledger §22]
12. **Refused CAN_DOCK is timer-gated too** — `BREAK(3)` + queue mission 0 → same epilogue → next
    attempt in 14–16 frames; a release does NOT reset a waiter's timer (same-frame takeover only if
    the waiter is later in the pass AND already due). [ledger §15]
13. **AARate absent/zero copies Rate** (`control.rs:108-114` matches). Guard is the only miner-adjacent
    mission with a distinct AARate; whether the miner path ever consumes AARate is UNKNOWN-U6. [ledger §3, U6]
14. **Frame basis = `sim.session.binary_frame`** (committed LATE, `mod.rs:1969/1982-85`), never
    `session.tick`; `MissionTimer.due` inclusive, `SENTINEL` always-due. [substrate-fit §6]
15. **Keyless-mission default diverges** (gamemd 14 vs Rust `rate_frames=0`, `control.rs:59`) —
    immaterial for the four miner missions (all have explicit stock sections); confirm no keyless
    miner-path dispatch before relying on the Rust default. [ledger U5]

## Design

### Components
- **`MissionControl::rate_frames_ftol(mission)`** (or fix `rate_to_frames` to truncate): the
  gamemd-exact frame value. Single source of the 14/27/… cadences. (Step A / L1)
- **Miner timer seeding**: `MinerConfig` (or the dock-sequence arm sites) reads
  `rules.mission_control.rate_frames(Enter/Unload)` for the base cadence; the `+RandomRanged(0,2)`
  jitter is applied at arm time via the existing `schedule_enter_retry` seam
  (`miner_dock_sequence.rs:84`). (Step A/B)
- **Approach-HELLO cadence gate**: a per-miner `MissionTimer` armed with the Harvest cadence,
  checked at the top of `phase_approach` before re-HELLO. Reuse an existing miner timer or a new
  field on `Miner`; `MissionCom.timer` is the substrate's long-term home but the miner keeps its
  own timers for now. (Step B / G6)

### Interfaces / Contracts
- The gate reads `RuleSet.mission_control` (already on `RuleSet`) — no new plumbing; `rules` is in
  scope at every miner tick site.
- All arms/checks off `sim.session.binary_frame`. The gate draws **zero RNG itself**; the only new
  draws are the *intentional* cadence-jitter/L9/L10 draws, sourced from the same
  `sim.miner_jitter_rng()` the current jitter uses (pending U1).

### Data Flow
`RuleSet.mission_control.rate_frames(mission)` → arm value → `Miner.<timer>.arm(binary_frame, rate
+ jitter)` → per-tick `.due(binary_frame)` gate at the dispatch decision → HELLO/CAN_DOCK/0x18
emitted only when due. Movement/combat/ore stay in their own phases, **un-gated** (they consume
the RNG stream and must not be reordered by the cadence gate).

### Error Handling / Determinism
- Gate throttles the **mission-AI decision only**, never a phase that consumes scenario/main RNG
  out of order (the single highest-risk point — substrate-fit §6).
- Native LogicVector iteration order preserved (no new due-set / no HashMap).
- Shadow-first: compute due/not-due + the would-be draws and **log divergence without acting**
  (mirroring the `object_ai_stage` S1/S2 dispatch shadow, `techno_ai.rs:88-115`); prove the
  divergence log matches expectations, then flip authoritative with ONE re-baseline.

### Testing Strategy
- **Unit:** `rate_frames_ftol` truncation vs `.round()` at a modded boundary (e.g. `.0206→18`, not
  19); `[Enter]/[Unload]` → 14, `[Guard]` → 27/14 from a real `mission_control`.
- **Cadence:** drive a full close-return cycle; assert the first CAN_DOCK lands at
  `binary_frame + rate + jitter` (G5), not next tick; assert one HELLO per due window (G6), one
  `0x18` per 0x0E pass (L20).
- **RNG-stream:** assert the per-dock-cycle draw count increases by exactly 1 at the L9 handoff and
  1 at the L10 exit (count-level certification; instance/order vs gamemd is U1-gated).
- **Regression ratchet:** the `global_skirmish_replay_is_deterministic_and_baseline_stable`
  harness — expect ONE intended re-baseline; document the SHA bump.

## Architectural Decisions
- **Follows** the shipped substrate pattern (per-object `MissionTimer` gate at the native tick
  site, verb-API mission commits, shadow-first rollout) — does not introduce a new scheduler.
- **Reuses** the miner's existing timers rather than migrating miners onto `MissionCom.timer` now
  (that migration is a separate substrate slice; noted as the long-term home).
- **Central value, per-object state** — `MissionControl` supplies the number; each miner owns its
  own timer. Avoids a global "due-set" (a non-gamemd primitive + second iteration order).

## Gates
- **U1 — RNG instance per draw: RESOLVED 2026-07-06 (Ghidra).** All mission-cadence epilogue
  `RandomRanged(0,2)` draws use **`Scen->Random`** — the `ScenarioClass::Random` member at `+0x218`
  of `g_ScenarioClass_Instance` (`0x00a8b230`, confirmed via `list_globals`), through a per-callsite
  `__thiscall` ECX (`Random__RandomRanged @0x0065c7e0` is thiscall — it mutates a ring buffer inside
  its `this`). Verified identical at the **Enter** epilogue (`0x004d948e MOV EAX,[0x00a8b230]` ->
  `0x004d948c LEA ECX,[EAX+0x218]` -> `CALL 0x0065c7e0`) and the **Harvest-10** epilogue
  (`0x0073ef8e`/`0x0073ef97`, same global + `+0x218`); the Unload path shares the same
  `GetMissionTimerEntry -> ftol(Rate*900) -> RandomRanged(0,2) -> sum` epilogue. The L9/L10 draws are
  part of these same Enter/Unload dispatches -> also `Scen->Random`.
  **Plan requirement:** route ALL miner cadence-jitter + the L9/L10 draws through the scenario RNG
  instance. Verify `sim.miner_jitter_rng()` maps to Scen->Random (per `reference_rng_instance_routing_truth`,
  the gameplay/mission RNG is Scen->Random); if it currently routes to a different instance, that is
  itself a lockstep bug to fix in this slice.
- **U-consumer — Rate gates the WHOLE mission handler: RESOLVED 2026-07-06 (Ghidra).**
  `MissionClass::Mission_Dispatch @0x005b3060` (verified via `decompile_function`) runs
  `ObjectClass::AI()` **unconditionally**, then gates on the mission timer — `if (start != -1 &&
  elapsed >= duration)` DUE (inclusive), else `return` (skip). When due it calls the **whole mission
  handler** via vtable (case 7 Enter `+0x240`, case 10 Harvest `+0x224`, case 0x10 Unload `+0x23c`)
  and writes `start = current frame`, `duration = handler's return value`. So the timer gates the
  entire mission handler (not a re-scan sub-step), and the handler's return IS the next cadence
  (= `ftol(Rate*900) + RandomRanged(0,2)`).
  **Plan requirement:** the miner cadence gate wraps the whole mission-AI decision (the HELLO /
  CAN_DOCK / 0x18 dispatch); per-tick ObjectClass-level work (facing interpolation, movement,
  animation) stays UNgated — matching `ObjectClass::AI` running every tick before the gate.
- **U4 — slave 10-frame limit (S12):** still unverified; deferred to W11 (SlaveManager RE). Not in W1.

## Alternatives Considered
- **B — central `object_ai_stage` cadence gate for ALL missions** (the full master-TODO #6 seam):
  closes W1 + general Unit cadence, but larger blast radius, and the miner runs in Phase 7 (not
  `object_ai_stage`) so it needs extra routing; over-scoped for W1's miner findings. Revisit as the
  general-cadence slice after W1 proves the pattern.
- **C — minimal (L1 data-drive + L9/L10 draws only), leave G5/G6/L20:** rejected — leaves the
  highest-visibility findings (G5/G6: ~13–15 frames early per haul, contested-dock winners flip)
  unfixed, which is the entire point of W1. Not a valid scope cut (no prerequisite block, not
  TS-legacy).
