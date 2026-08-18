# Mission-Cadence Service (W1) — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Do NOT batch Steps A/B/C
> into one commit — the hash-neutrality boundary between A (stock-identical) and B/C
> (hash-changing) is load-bearing for the re-baseline.

**Goal:** Give the parsed-but-unconsumed `MissionControl` Rate table a consumer on the miner
dispatch path so the miner FSM + refinery radio handshake are mission-timer-paced (gamemd's
~14–16 frame cadence), and restore gamemd's per-dispatch `RandomRanged(0,2)` draw pattern.

**Architecture:** Hybrid Rate-consumer seam — cadence VALUE owned centrally
(`RuleSet.mission_control.rate_frames(mission)`), gate STATE + CHECK per-object on the miner's
existing `MissionTimer`s at their current Phase-7 sites. No new phase, no reorder, no new
iteration order. Reuses the shipped substrate primitives (`MissionTimer`, `miner_jitter_rng`).

**Design Doc:** [docs/plans/2026-07-06-mission-cadence-service-design.md](2026-07-06-mission-cadence-service-design.md)

---

## Grounding Summary

- **Docs (verified this session):** `MissionControl` (`control.rs:74`) is fully parsed, keyed by
  `MissionType`, `rate_frames = Rate×900` at parse, with **zero non-test consumers** (grep-confirmed).
  The mission/radio substrate has SHIPPED (`MissionCom` authoritative+hashed; `MissionTimer` at
  `timer.rs:41` is a direct port of gamemd's inclusive `elapsed >= duration` / SENTINEL semantics).
  Full ledger: `scratchpad/w1-gamemd-ledger.md` (25 sourced items).
- **Ghidra (verified this session, both gates RESOLVED):**
  - `Mission_Dispatch @0x005b3060` runs `ObjectClass::AI()` unconditionally, then gates on the
    mission timer; when due it calls the **whole mission handler** and stores its return as the next
    duration. → **the timer gates the whole mission-AI decision, not a sub-step** (U-consumer).
  - All cadence `RandomRanged(0,2)` draws use **`Scen->Random`** (`ScenarioClass::Random @
    g_ScenarioClass_Instance+0x218`, `list_globals`-confirmed), thiscall per-callsite; verified
    identical at the Enter (`0x004d9492`) and Harvest-10 (`0x0073ef9d`) epilogues; Unload shares
    the epilogue (U1).
- **Repo pattern this mirrors:** the miner already runs the exact per-object frame-anchored gate —
  `harvest_timer`/`rescan_cooldown`/`dock_enter_retry: MissionTimer`, `.arm/.due(binary_frame)`
  (`miner_dock_sequence.rs:83-104`). A Rate gate is the SAME shape, seeded from
  `mission_control.rate_frames` instead of a hardcoded const.
- **INI keys (verified):** `[Enter]/[Unload]/[Harvest] Rate=.016 → 14`; `[Guard] Rate=.030 /
  AARate=.016 → 27/14` (`ini/rulesmd.ini:30510/30557/30529/30503-04`). Stock is unchanged by Step A.
- **RNG routing already correct:** `miner_jitter_rng()` (`world/mod.rs:628`) is documented +
  test-pinned (`rng_routing_tests.rs:161 assert_routes_scenario!`) as `Scen->Random`. **No routing
  fix needed** — the L9/L10 draws just call it.
- **Still unknown after grounding (→ Deferred):** exact HELLO→first-CAN_DOCK count (15/16/17, U3);
  slave 10-frame limit (S12/U4, out of scope → W11); keyless-mission default 0-vs-14 (U5, immaterial
  for the four miner missions); Guard AARate-vs-Rate selection on the miner path (U6).

## Key Technical Decisions

- **Reuse the miner's existing `dock_enter_retry`/`mission_deploy_timer`, seed from `MissionControl`**
  (not a new field, not `MissionCom.timer` yet) — **Confidence: high.** Source: repo pattern
  `miner_dock_sequence.rs:83-104`; design §Design. `MissionCom.timer` migration is a later slice.
- **`rate_to_frames` truncates (ftol), not `.round()`** — **Confidence: high (Ghidra).** Source:
  `Math__ftol @0x007c5f00` truncate-toward-zero; `control.rs:24-26`. Stock-identical (`14.4→14` both).
- **Keep `MISSION_DEPLOY_FACING_WAIT_FRAMES=5` a literal** (gamemd `return 5`, no Rate, no RNG) —
  **Confidence: high (doc).** Source: `MISSION_0X10_RETURN_DELAY §1`; ledger §7.
- **The L9/L10 draws go through `miner_jitter_rng()` (Scen->Random)** — **Confidence: high (Ghidra+repo).**
  Source: U1 resolution; `world/mod.rs:302/628`; `rng_routing_tests.rs:161`.
- **Re-baseline `GLOBAL_HARNESS_FINAL_HASH` once; do NOT bump `SNAPSHOT_VERSION`** — **Confidence:
  high.** W1 changes timer arm VALUES + adds RNG draws (deterministic, replay-stable) but adds no new
  serde-persisted authoritative schema, so `SNAPSHOT_VERSION` (snapshot.rs:71 = 25) stays; only the
  replay golden moves. Source: `global_parity_harness_tests.rs:83/175/298-301`.

## Open Questions

### Resolved during planning
- **U-consumer** (Rate gates whole handler vs re-scan): RESOLVED — whole handler (`Mission_Dispatch
  0x005b3060`). Gate wraps the whole mission-AI decision.
- **U1** (RNG instance): RESOLVED — `Scen->Random`; `miner_jitter_rng()` already routes there.

### Deferred to implementation / later
- **U3** — exact HELLO→first-CAN_DOCK count (15/16/17): the cadence math is fixed; the concrete
  observed number depends on live object order + the harvest draw. Assert the *mechanism* (cadence +
  1-frame commence step), not a hardcoded total. Runtime-debugger capture if a golden needs the exact N.
- **U4/S12** — slave 10-frame limit: OUT OF SCOPE (no SlaveManager doc; → W11 RE).
- **U5** — keyless-mission default (0 vs 14): the four miner missions all have explicit stock
  sections; add a debug-assert that `rate_frames(Enter/Unload/Harvest) != 0` at seam init.
- **U6** — Guard AARate: only relevant once Mission_Guard_Harvester (W10/M25) exists; not in W1.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/mission/control.rs` | L1: `rate_to_frames` truncate; keep `rate_frames(mission)` accessor |
| Modify | `src/sim/miner/miner_dock_sequence.rs` | seam: base cadences read the table; G5/G6/L20 gates; L9/L10 draws |
| Modify | `src/sim/miner/mod.rs` | (if needed) a `Miner` timer field for the approach-HELLO cadence (G6) |
| Modify | `src/sim/miner/miner_system.rs` | (if the G5 accept-path clear lives here) arm instead of clear |
| Modify | `src/sim/world/global_parity_harness_tests.rs` | re-baseline `GLOBAL_HARNESS_FINAL_HASH` (Step B/C) |
| Modify | `src/sim/miner/miner_tests.rs` | cadence + RNG-count tests |

## Interface Changes

- `MissionControl::rate_frames(mission)` (`control.rs:129`) gains its first production consumer
  (the miner seam). No signature change. The behaviour of `rate_to_frames` changes (round→truncate);
  the only caller is `from_ini`, so no external impact beyond the parsed values (stock-identical).
- No public API added; the seam is internal to `sim/miner`.

## Sim Checklist

- [x] All math fixed-point / integer — `rate_frames` is `u32` frames; the ftol conversion is a
      one-shot parse-time `f64` (like the existing `IncomeMult`/`DumpRate` parses), never in a tick path.
- [x] New state in the deterministic hash — no NEW persisted field; existing `MissionTimer`s already
      hashed. Arm-value + RNG-draw changes move the replay hash → one documented re-baseline.
- [x] No dependency on render/ui/sidebar/audio/net — all edits in `sim/`, reading `rules/`.
- [x] Tick ordering unchanged — the gate lives inside the existing Phase-7 miner tick; no reorder.
- [x] BTreeMap/LogicVector order preserved — the seam checks each miner's own timer inside the
      existing snapshot loop (`miner_system.rs:98`); no new iteration source.

## Risk Areas

- **RNG-stream reorder (highest risk):** adding the L9/L10 draws shifts every downstream draw. The
  gate itself must draw ZERO RNG; only the *intentional* cadence-jitter/L9/L10 draws are added, all on
  `miner_jitter_rng()` (Scen->Random). Safety net: `global_skirmish_replay_is_deterministic_and_baseline_stable`
  must still pass (deterministic) after the re-baseline; run it 2× to confirm stability.
- **Two G5 accept paths:** the close-return accept sets `dock_phase=MissionEnter` in `phase_approach`
  (`miner_dock_sequence.rs:841`) AND in a second accept path (`:972`); the abort-path `clear_enter_retry`
  (`:671/:701`) is CORRECT and must NOT be touched. Task B2 must distinguish accept-clear from abort-clear.
- **Existing miner tests bake the current cadence** (`miner_tests.rs` has 9 `dock_phase=MissionEnter`
  setups + the fallback/close-return timing tests). Expect several to need frame-count updates when
  cadence shifts — update them as part of Step B, don't skip.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| A1 | `ftol(Rate×900)` truncate | 1-frame drift for modded rates | Unit: `.0206→18` not 19; stock `.016→14` |
| A2 | Base cadence from `[Enter]/[Unload] Rate` | data-driven vs hardcode; stock 14/14 | Unit: table → 14/14; `[Guard]`→27/14 |
| A2 | Keep facing-wait = **5** literal | gamemd `return 5`, not a Rate, no RNG | Ghidra `MISSION_0X10_RETURN_DELAY §1` |
| B1 | Approach HELLO one-per-due-window | contested-dock winners flip if per-tick (every match) | Test: 1 HELLO / cadence window |
| B2 | First CAN_DOCK waits `rate+jitter`, not next tick | economy ~1s fast/haul (G5) | Test: first 0x0E at `frame+14..16` |
| B3 | `0x18` one-per-0x0E-pass | radio spam vs gamemd (L20) | Test: 1 `bus_enter_dock`/pass |
| C1 | +1 `RandomRanged(0,2)` at FaceSync→MissionQueued | jitter-stream desync (L9) | Test: draw count +1; Scen->Random |
| C2 | +1 `RandomRanged(0,2)` + jitter at state-4 exit | SearchOre 0–2f early + desync (L10) | Test: draw count +1; Scen->Random |
| C1/C2 | All draws on **Scen->Random** | lockstep-vs-gamemd RNG stream | `rng_routing_tests.rs` assert_routes_scenario |

---

## Tasks

### Step A — data / L1 (stock byte-identical, hash-neutral)

### Task A1: `rate_to_frames` truncates toward zero (ftol model)

**Why:** gamemd computes `Math::ftol(Rate×900)` (truncate); Rust `.round()` drifts for modded rates.
Stock-identical, so this lands hash-neutral before the behavioural steps.

**Files:** Modify `src/sim/mission/control.rs` (`rate_to_frames`, ~line 24; test ~line 155).

**Pattern:** mirrors the parse-time `f64→integer` truncation used elsewhere in rules parsing.

**Step 1 — change the conversion.** Read `control.rs:22-26`; replace the `.round()` with a
truncating cast (positive-only domain, so `as u32` == floor == ftol):
```rust
/// Convert an INI rate (minutes between processings) to integer frames,
/// modelling gamemd's `Math::ftol(Rate * 900)` truncate-toward-zero.
#[inline]
fn rate_to_frames(minutes: f64) -> u32 {
    (minutes * FRAMES_PER_MINUTE) as u32
}
```

**Step 2 — extend the test.** In `rate_to_frames_uses_900_per_minute` (control.rs:155), keep the
stock cases and add a truncation-boundary case that `.round()` would have failed:
```rust
    assert_eq!(rate_to_frames(0.0206), 18); // 18.54 -> ftol 18 (round would give 19)
    assert_eq!(rate_to_frames(0.016), 14);  // 14.4 -> 14 (stock, unchanged)
```

**Step 3 — verify:** `cargo test -p vera20k rate_to_frames_uses_900_per_minute` → PASS.

**Step 4 — commit:** `mission: L1 rate_to_frames truncates (ftol) instead of round`.

### Task A2: miner Enter/Unload base cadences read `MissionControl.rate_frames`

**Why:** wire the zero-consumer table into the seam; stock values (14/14) unchanged, so still
hash-neutral. This is the "consumer" that closes the L1 root cause.

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (`schedule_enter_retry` ~83,
`schedule_mission_deploy_delay` callers, and the `ENTER_RETRY_BASE_FRAMES` / `MISSION_DEPLOY_UNLOAD_BASE_FRAMES`
usages ~53/56). Read `rules` (already in scope at the dock-sequence sites via `&RuleSet`).

**Step 1 — source the base from the table.** Replace the hardcoded base reads with a helper that
looks up the mission Rate; keep the const as the fallback for a keyless mission (U5 guard). Example
for the Enter retry:
```rust
fn enter_base_frames(rules: &RuleSet) -> u8 {
    let f = rules.mission_control.rate_frames(MissionType::Enter);
    if f == 0 { ENTER_RETRY_BASE_FRAMES } else { f.min(u8::MAX as u32) as u8 }
}
```
and in `schedule_enter_retry`, use `enter_base_frames(rules)` for the base (thread `rules` into the
fn signature — it is already available at every caller). Do the same for the Unload base via
`rate_frames(MissionType::Unload)`. **Leave `MISSION_DEPLOY_FACING_WAIT_FRAMES = 5` untouched.**

**Step 2 — U5 guard.** Add a debug assert at the seam init (or a test) that
`rate_frames(Enter) != 0 && rate_frames(Unload) != 0` for the stock table.

**Step 3 — test.** New test in `miner_tests.rs`: build a ruleset with `[Enter] Rate=.016`,
`[Unload] Rate=.016`, `[Guard] Rate=.030 AARate=.016`; assert `enter_base_frames == 14`,
Unload base == 14, and (sanity) `mission_control.rate_frames(Guard) == 27`.

**Step 4 — verify:** `cargo test -p vera20k` (miner + control suites) → PASS, stock unchanged.

**Step 5 — commit:** `miner: W1-A wire Enter/Unload base cadence from MissionControl (L1)`.

### Step B — cadence gates (G5, G6, L20) — **HASH-CHANGING**

> After Step B the replay hash moves. Do the re-baseline in Task R2, not per-task. Each B task
> commits its code + test updates; R2 commits the golden.

### Task B1: gate the approach re-HELLO behind a Harvest-cadence timer (G6)

**Why:** `phase_approach` re-sends HELLO every tick (`miner_dock_sequence.rs:826`); gamemd sends one
per due Harvest dispatch (`[Harvest] Rate×900 + RandomRanged(0,2)` ≈ 14–16f).

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (`phase_approach`, HELLO send ~826);
possibly add a `Miner.approach_hello_timer: MissionTimer` field in `src/sim/miner/mod.rs` (mirror
`dock_enter_retry` init at mod.rs:334-ish and its serde/hash inclusion).

**Step 1 — read** `phase_approach` (the HELLO send around :815-841) to identify the current
unconditional re-HELLO and the accept transition (`dock_phase=MissionEnter` at :841).

**Step 2 — add the gate.** Before re-sending HELLO, `if !approach_hello_due(sim, snap) { return; }`;
on send, arm `approach_hello_timer` with `harvest_base_frames(rules) + RandomRanged(0,2)` via
`miner_jitter_rng()` (Scen->Random). `harvest_base_frames` mirrors `enter_base_frames` reading
`rate_frames(MissionType::Harvest)`.

**Step 3 — test:** drive an approach; assert exactly one HELLO per `harvest_base` window, not per tick.

**Step 4 — commit:** `miner: G6 gate approach re-HELLO on the Harvest mission cadence`.

### Task B2: accepted-HELLO arms the Enter cadence instead of clearing it (G5)

**Why:** the close-return accept sets `dock_phase=MissionEnter` with the retry timer **cleared**
(SENTINEL/always-due), so CAN_DOCK fires next tick. gamemd waits the Enter cadence (15–17f window).

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (accept path at :972 / :841) and/or
`src/sim/miner/miner_system.rs` (the close-return accept the design cites at ~:971-976).

**Step 1 — locate the ACCEPT-path clear.** Read both `dock_phase=MissionEnter` sites
(`miner_dock_sequence.rs:841, :972`) and the close-return in `miner_system.rs` (handle_return). Find
where the accept clears/leaves-always-due `dock_enter_retry`. **Do NOT touch the abort-path clears at
`:671`/`:701`** (those are correct — an aborted dock should be immediately due).

**Step 2 — arm instead of clear.** At the accept, replace the clear with
`schedule_enter_retry(sim, rules, snap)` (arms `enter_base_frames + RandomRanged(0,2)`), so the first
CAN_DOCK dispatch waits the cadence.

**Step 3 — test:** full close-return cycle; assert the first CAN_DOCK/`0x0E` fires at
`accept_frame + 14..=16`, not `accept_frame + 1`. Update any existing close-return timing tests.

**Step 4 — commit:** `miner: G5 accepted-HELLO waits the Enter cadence (no always-due collapse)`.

### Task B3: `bus_enter_dock(0x18)` fires after the due gate (L20)

**Why:** `bus_enter_dock` at `:992` fires **before** the `enter_retry_due` gate at `:995`, so `0x18`
re-sends every arrived tick; gamemd sends one per 0x0E pass.

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (`phase_face_sync`, :989-997; also the
already-aligned path at :928).

**Step 1 — read** `phase_face_sync` (:985-1005) to see the `bus_enter_dock` (:992) → `enter_retry_due`
(:995) order.

**Step 2 — reorder.** Move the `bus_enter_dock(sim, snap.entity_id, ref_sid)` call to *inside* the
due branch (after `if enter_retry_due(...)`), so `0x18` is sent once per due dispatch. Check the
`:928` aligned-path send is likewise gated (it is on the accepted 0x16→0x15 pass — confirm it is
one-per-pass).

**Step 3 — test:** waiting miner over a 14–16f window emits exactly one `bus_enter_dock`, not ~14.

**Step 4 — commit:** `miner: L20 EnterDock(0x18) one-per-0x0E-pass, not per arrived tick`.

### Step C — RNG draws (L9, L10) — **HASH-CHANGING, Scen->Random**

### Task C1: draw `RandomRanged(0,2)` at the accepted FaceSync→MissionQueued handoff (L9)

**Why:** gamemd's successful `0x16→0x15` handoff is still a Mission_Enter dispatch and draws one
`RandomRanged(0,2)`; Rust clears the timer with no draw (`miner_dock_sequence.rs:1000`) → 1 missing
draw per dock cycle.

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (accepted branch ~:995-1001).

**Step 1 — read** the accepted branch (:995-1004): the accepted path calls `clear_enter_retry`
(:1000, no draw); the un-accepted path (:1003) already draws via `schedule_enter_retry`.

**Step 2 — add the draw.** On the accepted handoff, draw one
`sim.miner_jitter_rng().next_range_u32_inclusive(0, ENTER_RETRY_JITTER_MAX_FRAMES)` (Scen->Random)
BEFORE transitioning to `MissionQueued` — the transition still proceeds, but the draw must happen to
keep the RNG stream aligned. (The draw value is consumed by the epilogue in gamemd; here it advances
the stream by exactly one.)

**Step 3 — test:** capture `miner_jitter_rng` draw count across one accepted dock cycle; assert it
increases by exactly 1 at this handoff vs the pre-fix baseline.

**Step 4 — commit:** `miner: L9 draw the RandomRanged(0,2) at the accepted FaceSync handoff`.

### Task C2: draw `RandomRanged(0,2)` + jitter at the state-4 exit (L10)

**Why:** gamemd's state-4 exit returns through a timer epilogue drawing `Random(0,2)`; Rust does
neither (`miner_dock_sequence.rs:1268`) → SearchOre resumes 0–2f early + 1 missing draw.

**Files:** Modify `src/sim/miner/miner_dock_sequence.rs` (state-4 exit ~:1260-1275).

**Step 1 — read** the state-4 exit (the `clear_enter_retry` at :1268 + the SearchOre resume).

**Step 2 — add the draw + jitter.** Draw `let jitter = miner_jitter_rng().next_range_u32_inclusive(0,
2)` and apply it as a 0–2 frame delay on the SearchOre resume (arm the resume timer with the jitter),
matching the epilogue.

**Step 3 — test:** draw count +1 at exit; SearchOre resumes `exit_frame + jitter`.

**Step 4 — commit:** `miner: L10 state-4 exit draws RandomRanged(0,2) + applies the jitter`.

### Rollout

### Task R1: shadow-verify the cadence (recommended, optional)

**Why:** de-risk the flip by proving the new cadence matches expectation before the golden moves.

**Step 1 —** temporarily log (behind `log::debug!`) the computed `(base, jitter, due_frame)` at each
gated site for one skirmish; confirm the values match the ledger (Enter/Unload 14+[0..2], Harvest
14+[0..2]). Remove the logging before R2. (No commit; scratch.)

### Task R2: re-baseline the replay golden + document

**Why:** Steps B/C deterministically move the replay hash once. Re-baseline the single golden.

**Files:** Modify `src/sim/world/global_parity_harness_tests.rs` (`GLOBAL_HARNESS_FINAL_HASH`, :83).

**Step 1 — run** `cargo test -p vera20k global_skirmish_replay_is_deterministic_and_baseline_stable`.
It fails with the new `left` hash (the intended move).

**Step 2 — re-baseline.** Set `GLOBAL_HARNESS_FINAL_HASH` to the reported `left` value; add a comment
`// re-baselined <date>: W1 mission-cadence (G5/G6/L20 + L9/L10 RNG draws) — intended stream move`.
**Do NOT bump `SNAPSHOT_VERSION`** (no new persisted authoritative schema).

**Step 3 — prove determinism.** Run the harness **twice**; both must PASS with the new constant
(same hash every run = deterministic).

**Step 4 — commit:** `test: W1 re-baseline replay golden for mission-cadence + RNG draws`.

### Task R3: fidelity + review

**Step 1 —** `/review-plan` was run pre-implementation; run `/fidelity-check` on the full-cargo
close-return cadence (first CAN_DOCK timing, one HELLO/window, one 0x18/pass) vs the ledger.

**Step 2 —** update `docs/plans/2026-07-02-miner-parity-roadmap.md §Status`: mark W1 landed; note the
golden re-baseline SHA; move S12 to W11-blocked.

## Sources & References

- **Design doc:** docs/plans/2026-07-06-mission-cadence-service-design.md
- **Research bundle (this session):** scratchpad/w1-gamemd-ledger.md (25 items), w1-arch-map.md,
  w1-substrate-fit.md.
- **Ghidra (verified this session):** `Mission_Dispatch 0x005b3060` (timer gates whole handler);
  `Mission_Enter 0x004d9290` epilogue; `RandomRanged 0x0065c7e0` (thiscall); Enter draw `0x004d9492`
  + Harvest draw `0x0073ef9d` both on `g_ScenarioClass_Instance(0x00a8b230)+0x218` = Scen->Random;
  `Math__ftol 0x007c5f00`.
- **Prior Ghidra reports:** REFINERY_ENTER_RETRY_TIMER_IMPLEMENTATION_VERIFICATION,
  WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK, CMIN_STATE2_CLOSE_FAR_RETURN..., FOOTCLASS_MISSION_ENTER_0X0E,
  MISSION_0X10_RETURN_DELAY, NEXT_DOCKER_SELECTION.
- **INI keys:** rulesmd.ini `[Enter]/[Unload]/[Harvest] Rate=.016`, `[Guard] Rate=.030 AARate=.016`.
- **Related code:** control.rs:74/129, miner_dock_sequence.rs:53-104/826-1005/1268, timer.rs:41,
  world/mod.rs:302/628, rng_routing_tests.rs:161, global_parity_harness_tests.rs:83/175, snapshot.rs:71.
- **Scan findings:** docs/gap-scans/2026-07-02-disparity-scan-miner.md G5, G6, M1, M5, L1, L9, L10, L20, S12.
