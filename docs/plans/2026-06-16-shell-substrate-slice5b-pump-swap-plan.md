# Shell-Substrate Slice 5 sub-step 5b — modal-pump `service_tick` swap + C2 assertion

> **For Claude:** Execute task-by-task. Each task is self-contained. PLAN ONLY until approved.

**Goal:** Route the in-game sim-advance freeze through the verified modal-pump contract (offline freezes, network advances) instead of the bare `state.paused` flag, and lock it with the contract-C2 acceptance assertion — completing the last open piece of Slice 5.

**Architecture:** App-layer only. `state.paused` already *is* the in-game Options (0xBBB) modal in this port; the freeze decision moves from `!state.paused` to the already-landed pure seam `modal_pump_should_advance_sim(SessionMode, reentrancy)`. `World::advance_tick` is untouched; `SessionMode` stays read-only to the app loop — `sim/` never sees it (the #1 invariant).

**Design / source docs:** docs/plans/2026-06-01-shell-substrate-slice5-plan.md §3, §3.2, §7.A; docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md sub-step 5b; docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md.

---

## Grounding Summary

- **Docs (verified):** `MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT` is ghidra/verified. §3.4: offline `g_GameMode ∈ {0 campaign, 5 skirmish}` → message+net pump, **no `Main_Tick`** (sim frozen); network `{3 LAN, 4 WOL}` → `Main_Tick` advances (when reentrancy/blockers clear). §10 handoff acceptance: *open Options in skirmish → `World.tick` delta 0, dialog responsive, **no catch-up burst on close***. §11 negative facts: **do NOT** claim the battlefield animates behind offline Options; **do NOT** assert it recomposites each pump frame; **do NOT** model the reentrancy byte as a user-pause flag.
- **Repo state (read this pass):** the pure seam already exists and is unit-tested — `SessionMode` (`app_sim_tick.rs:156-189`), `modal_pump_should_advance_sim` (`:199-201`), tests `modal_pump_tests` (`:1684-1743`). The live gate is `advance_in_game_runtime` (`:203-281`): `run_sim = frame_step || !state.paused` (`:204-211`). Its doc comment (`:196-198`) explicitly defers the `service_tick` wrapper to "when the in-game modal loop is wired" — **this slice**. `state.paused` *is* the in-game Options modal: the 0xBBB overlay renders only `if state.paused` (`app.rs:2916`, `:2959`); `InGameOptionsState` has no separate visibility flag. The redraw call-site `app.rs:2740` calls `update_elapsed_ms` then `advance_in_game_runtime` — `update_elapsed_ms` advances `last_update_time` **every** frame, so no wall-clock accumulates while frozen ⇒ no catch-up burst (preserved, not newly built).
- **Headless test pattern:** `Simulation::new()` (cheap, used throughout `sim/` tests), `sim.session.tick: u64` (`components.rs:965`), `sim.advance_tick(&[], None, &BTreeMap::new(), None, None, ms)` increments `session.tick` once per call unconditionally (the increment is committed at the tail of `advance_tick`, `world/mod.rs:1971-1984`).
- **INI:** none. The pump decision reads game mode only; no INI key controls it (report §6). RA2MD.INI persist is sub-step 4, out of scope here.
- **Still unknown:** nothing blocking. Network mode is dead code this build (offline client); the live session mode is constant-offline (see Decision 2).

## Key Technical Decisions

- **D1 — The "modal pump" maps to a per-frame *decision*, not a nested loop.** gamemd's `FUN_00623120` is a nested modal message loop; this port has one winit frame loop, and `advance_in_game_runtime` already does the per-frame net/input/anim/repaint work. So `service_tick` is realized by making that function's `run_sim` gate modal-aware — NOT by adding a second loop that would duplicate the anim/camera/repaint tail. **Confidence:** high. **Source:** MODAL_PUMP report §7/§10 (handoff names `app_sim_tick.rs`/`app.rs` as the surface), repo structure `app_sim_tick.rs:203-281`.
- **D2 — Live session mode is `Skirmish` (offline) for this build.** The client is offline-only; both `Campaign` and `Skirmish` freeze identically, so `current_session_mode` returns `SessionMode::Skirmish`. The network branch is real, unit-tested, and dead until multiplayer lands (when this helper reads the live `g_GameMode` equivalent). This is correct-for-now, not a placeholder. **Confidence:** high. **Source:** slice5-plan §3.2 ("returns offline for current play"), MODAL_PUMP §3.4.
- **D3 — Offline behavior is byte-identical to today.** For offline play, `modal_pump_should_advance_sim(Skirmish, false) == false == !state.paused` when paused, and `true` when not paused — so the swap changes nothing observable offline; it only adds the (dead) network branch and routes the decision through the verified contract. **Confidence:** high. **Source:** algebraic equivalence + `modal_pump_should_advance_sim` body (`:200`).

## Open Questions

### Resolved during planning
- *Where does `service_tick` live?* → D1: a decision helper inside `app_sim_tick.rs`, consumed by `advance_in_game_runtime`. `app.rs` is **not** modified.
- *Is the battlefield "frozen-as-last-blit" assertable?* → No (report §11: don't assert recomposite). The offline freeze is asserted via `World.tick` delta == 0 (sim frozen ⇒ renderer draws frozen state); the visual is the in-game STOP gate (Task 5).
- *Could the swap cause a catch-up burst on close?* → No. `update_elapsed_ms` (`:144-148`) advances `last_update_time` every frame regardless of the gate, so no elapsed time accumulates while frozen. Preserved by leaving the call-site untouched.

### Deferred to implementation
- None. (Network branch stays dead-but-tested; no live caller this build.)

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_sim_tick.rs` | Add `current_session_mode` + `service_tick_should_advance_sim` helpers; make `advance_in_game_runtime`'s `run_sim` gate modal-aware; refresh the `:196-198` doc comment; extend `modal_pump_tests` with the headless-World C2 assertion. |

No other file changes. `app.rs` redraw call-site untouched. `src/ui/skirmish_shell/state/tests.rs` (87 tests) untouched and GREEN.

## Interface Changes

- New **private** fn `current_session_mode(&AppState) -> SessionMode` (app-layer, not exported).
- New **private** fn `service_tick_should_advance_sim(&AppState) -> bool` (app-layer).
- No public API, trait, or struct change. `SessionMode` / `modal_pump_should_advance_sim` already `pub` and unchanged. `World::advance_tick` signature unchanged.

## Sim Checklist

- [x] No `f32/f64` introduced in sim logic — change is app-layer; test only *reads* `session.tick`.
- [x] No new state in the deterministic hash — no sim state added.
- [x] **No `sim/` dependency on `render/ui/sidebar/audio/net`** — `SessionMode` stays in the app layer; `advance_tick` is called by the test as an external consumer, exactly as existing `sim/` tests do.
- [x] Tick ordering unaffected — the gate decides *whether* to call `advance_fixed_simulation`, never reorders within `advance_tick`.
- [x] BTreeMap iteration order irrelevant here.

## Risk Areas

- **Highest blast radius:** `advance_in_game_runtime` is the in-game per-frame heartbeat. The edit is confined to the `run_sim` boolean; the unconditional per-frame tail (anims/camera/radar/batch-renderer, `:261-280`) and the frame-step path (`:205-218`) are untouched. Regression guard: D3 equivalence + the unchanged 87-test skirmish safety net + the C2 test.
- **Catch-up burst** (units jumping forward when Options closes): mitigated by leaving `update_elapsed_ms`/the call-site untouched (no accumulation while frozen). Verified by the in-game STOP gate (Task 5).
- **Mis-asserting recomposite:** the C2 test asserts ONLY `World.tick` delta (report §11) — it must not assert anything about battlefield redraw.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | Offline sim FREEZE behind in-game Options | Every time the player opens Options mid-skirmish, the battlefield must be frozen (no sim advance) — observable every match | C2 test (`World.tick` delta 0) + Task 5 in-game STOP (battlefield static, dialog responsive) |
| Task 2 | No catch-up burst on Options close | If the sim caught up the elapsed frozen time, units would teleport forward on resume — immediately visible | Task 5 in-game STOP: close Options after several seconds, confirm smooth resume |
| Task 2 | Network advances behind modal (dead code) | Correctness seam so multiplayer modals don't freeze the netgame when MP lands | C2 test (`World.tick` delta N for Lan/Wol) — unit-only, no live caller |

---

## Tasks

### Task 1: Add `current_session_mode` + the modal-pump service decision

**Why:** Provide the live session-mode source (D2) and the modal-aware advance decision (D1) the gate will consume. Pure, app-layer, no `sim/` touch.

**Files:**
- Modify: `src/app_sim_tick.rs` (add two fns immediately after `modal_pump_should_advance_sim`, ~`:201`)

**Pattern:** Mirrors the existing pure-seam style at `:191-201` (small, documented, app-layer).

**Step 1: Add the helpers**
```rust
// src/app_sim_tick.rs — insert after `modal_pump_should_advance_sim` (~line 201)

/// Live front-end session mode for the running client. This build is offline
/// only, and offline campaign and skirmish freeze the world identically behind a
/// modal, so it reports `Skirmish`. When networking lands, this reads the live
/// game-mode discriminator and maps it via `SessionMode::from_game_mode`.
fn current_session_mode(_state: &AppState) -> SessionMode {
    SessionMode::Skirmish
}

/// App-layer modal-pump service decision: should the fixed simulation advance
/// this frame? While the in-game Options modal is open (`state.paused` is the
/// 0xBBB modal in this port), the verified pump contract decides — offline
/// campaign/skirmish freeze, network LAN/WOL advance. With no modal open the
/// world always runs. Re-entrancy is always clear here: the single-threaded
/// frame loop never re-enters a fixed tick mid-advance.
fn service_tick_should_advance_sim(state: &AppState) -> bool {
    if state.paused {
        modal_pump_should_advance_sim(current_session_mode(state), false)
    } else {
        true
    }
}
```

**Step 2: Verify it compiles**
Run: `cargo check -p vera20k`
Expected: clean (helpers currently unused → allow the dead-code warning to exist transiently; Task 2 consumes them in the same change set, so do Task 1 + Task 2 before the check in Task 3).

**Step 3:** No commit yet (consumed by Task 2).

### Task 2: Swap the `run_sim` gate to the modal-pump decision

**Why:** Replace the bare `!state.paused` freeze with the verified contract (D1/D3). Offline behavior is unchanged; the network branch becomes reachable (dead this build).

**Files:**
- Modify: `src/app_sim_tick.rs:204-211` (the `run_sim` computation in `advance_in_game_runtime`) and the doc comment at `:196-198`.

**Step 1: Replace the `run_sim` computation**
Current (`:205-211`):
```rust
    let frame_stepping = state.debug_frame_step_requested;
    let run_sim = if frame_stepping {
        state.debug_frame_step_requested = false;
        true
    } else {
        !state.paused
    };
```
Replace the `else` arm so the freeze is modal-pump-driven:
```rust
    let frame_stepping = state.debug_frame_step_requested;
    let run_sim = if frame_stepping {
        state.debug_frame_step_requested = false;
        true
    } else {
        // Modal-pump contract: while the in-game Options modal is open
        // (`state.paused`), offline freezes and network advances; otherwise run.
        // Offline-identical to the prior `!state.paused`.
        service_tick_should_advance_sim(state)
    };
```

**Step 2: Refresh the deferred-wrapper doc comment** at `:196-198` (inside the `modal_pump_should_advance_sim` doc) so it no longer says the wrapper is pending. Change the trailing sentence:
```rust
/// ... Pure and total, so it is
/// unit-tested without an `AppState`. The live app-layer consumer is
/// `service_tick_should_advance_sim`, which reads the running session mode and
/// gates `advance_fixed_simulation` inside `advance_in_game_runtime`.
```

**Step 3:** No commit yet (verified in Task 3 with the test).

**Architecture note:** No `sim/` dependency added; `SessionMode` stays app-layer. Behavior-preserving offline (D3).

### Task 3: Extend the C2 acceptance test to a real headless `World`

**Why:** The existing `pumped_tick_delta_is_zero_offline_and_n_on_network` uses a stand-in counter and its comment defers the real-sim assertion to "the live `service_tick` swap" — this slice. Drive a real `Simulation` so the contract is proven against actual `session.tick` motion (plan §7.A; report §10 acceptance).

**Files:**
- Modify: `src/app_sim_tick.rs` — `modal_pump_tests` module (~`:1684-1743`). ADD a new test; leave the existing four tests unchanged.

**Step 1: Add the headless-World test** (after the existing `pumped_tick_delta_is_zero_offline_and_n_on_network`):
```rust
    #[test]
    fn pumped_world_tick_freezes_offline_and_advances_on_network() {
        use crate::sim::world::Simulation;
        use std::collections::BTreeMap;

        // C2 acceptance with a real headless World: drive `advance_tick` exactly
        // when the pump decision is true, and assert `session.tick` motion.
        // `advance_tick` commits one tick per call (no entities/rules needed).
        const FRAMES: u64 = 7;
        let pumped_world_delta = |mode: SessionMode| -> u64 {
            let mut sim = Simulation::new();
            let start = sim.session.tick;
            let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
            for _ in 0..FRAMES {
                if modal_pump_should_advance_sim(mode, false) {
                    sim.advance_tick(&[], None, &height_map, None, None, super::SIM_TICK_MS);
                }
            }
            sim.session.tick - start
        };

        // Offline modes freeze the world behind the modal: zero tick advance.
        assert_eq!(pumped_world_delta(SessionMode::Skirmish), 0);
        assert_eq!(pumped_world_delta(SessionMode::Campaign), 0);
        // Network modes advance one fixed tick per pumped frame (dead code this
        // build; proves the contract for when multiplayer lands).
        assert_eq!(pumped_world_delta(SessionMode::Lan), FRAMES);
        assert_eq!(pumped_world_delta(SessionMode::Wol), FRAMES);
    }
```
Note: `SIM_TICK_MS` is a crate const in this module's parent (`super::SIM_TICK_MS`); confirm the path resolves (the test module is nested in `app_sim_tick`). If `advance_tick`'s `tick_ms` is `u32`, pass `super::SIM_TICK_MS` directly (it already is `u32` — used at `:215`/`:240`).

**Step 2: Verify (bounded foreground pass — read the literal `test result:` line)**
Run: `cargo check -p vera20k`
Then: `cargo test -p vera20k --lib -- modal_pump`
Expected: `test result: ok.` with all `modal_pump_tests` (5 tests) passing, including the new one.

**Step 3:** No commit yet (full regression in Task 4).

### Task 4: Regression pass — safety net + shell stays green

**Why:** `advance_in_game_runtime` is the in-game heartbeat; confirm nothing regressed and the 87-test skirmish safety net is untouched/green.

**Files:** none (verification only).

**Step 1:** Run `cargo test -p vera20k --lib -- skirmish_shell` — expected `test result: ok.`, count unchanged from before this slice (the 87 `state/tests.rs` tests + render tests). Confirm `git status` shows NO modification to `src/ui/skirmish_shell/state/tests.rs`.

**Step 2:** Run `cargo clippy -p vera20k` and confirm `src/app_sim_tick.rs` introduces no new warnings (the two new fns are consumed, so no dead-code warning).

**Step 3:** No commit yet — STOP for the in-game gate (Task 5) first.

### Task 5: In-game STOP gate (manual acceptance — gamemd parity)

**Why:** The frozen-as-last-blit + no-catch-up-burst behavior is the player-visible parity bar and is not unit-assertable (report §11). Must be confirmed live before commit (slice cadence rule).

**Verify (run the app, offline skirmish):**
- Open in-game Options (pause) mid-skirmish. **Expected:** the battlefield is frozen (units do not move/animate behind the 0xBBB overlay); the Options dialog is responsive (sliders/checkboxes/buttons react).
- Leave Options open several seconds, then close (Back/OK). **Expected:** play resumes smoothly with **no catch-up burst** — units do not jump forward to "make up" the paused time.
- Confirm this matches gamemd.exe: pausing into in-game Options freezes the offline skirmish, identical resume feel.

If any of these fail, STOP and reassess — do not commit.

### Task 6: Commit to `dev`

**Why:** Land 5b as its own commit (slice cadence).

**Step 1:** `git add src/app_sim_tick.rs`
**Step 2:** Commit (subject mirrors the repo convention, e.g. `ui: Slice 5 sub-step 5b - modal-pump service_tick swap (offline freeze / network advance) + C2 World-tick assertion`), body noting: behavior-identical offline (decision == `!paused`), network branch dead-but-tested, `sim/` untouched, safety net unchanged.
**Step 3:** Confirm `git log --oneline -1` shows the commit on `dev`.

---

## Sources & References

- **Plan/design:** docs/plans/2026-06-01-shell-substrate-slice5-plan.md §3 (pump contract C2), §3.2 (`service_tick` in app layer; `g_GameMode` {3,4}/{0,5} writer-proofed), §7.A (C2 tick-counter acceptance); docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md sub-step 5b; docs/plans/2026-05-31-shell-substrate-plan.md §5 Slice 5 (C2).
- **Ghidra report (verified):** docs/research/MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT_GHIDRA_REPORT.md — §3.4 offline freeze vs network advance, §7 current Rust status, §10 implementation handoff (acceptance scenarios), §11 negative facts. (gamemd addresses live here, not in Rust comments.)
- **Repo touchpoints:** `src/app_sim_tick.rs` — `SessionMode` `:156-189`, `modal_pump_should_advance_sim` `:199-201`, `advance_in_game_runtime` `:203-281` (gate `:204-211`), `advance_fixed_simulation` `:284`, `modal_pump_tests` `:1684-1743`; `src/app.rs:2740` redraw call-site, `:2916`/`:2959` Options-overlay-on-`paused`; `src/sim/world/mod.rs:1971` `advance_tick`; `src/sim/components.rs:965` `session.tick`; headless test pattern `src/sim/combat/combat_turret_facing_tests.rs:20-40,69-81`.
- **INI:** none (pump decision reads game mode only — report §6).
