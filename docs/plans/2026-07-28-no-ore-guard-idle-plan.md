# No-Ore → Guard Idle Shape Implementation Plan

> **For Claude:** Execute task-by-task.

**Goal:** Replace the WaitNoOre auto-rescan loop with the native parking shape: a no-ore harvester sets the sticky per-house "ore short" flag and parks in Guard permanently (player action resumes it); a no-refinery miner parks in Guard and auto-resumes only for AI houses once a dock-type building is owned; a full miner whose refinery search merely fails stays in ReturnToRefinery polling.

**Architecture:** sim-only. New hashed `HouseState.ore_short` field (schema change → SNAPSHOT_VERSION 30→31, harness re-baseline). The miner FSM keeps `WaitNoOre` as the "parked in Guard" model (mission verb queued via `mission_queue_exact`; the component-gated dispatch residual is unchanged — Track B1 later). Cadence: parked dispatches use `[Guard] Rate` (27-29f stock).

**Design Doc:** none (approved inline: "ok do the no-ore guard idle shape"); grounding below substitutes.

## Grounding Summary

Byte-verified this session (fresh asm windows) + today's lane docs:
- Mission_Harvest state 4 tail (asm 0x0073eea6..0x0073ef77, read this session): IsUseless branch queues 0x14/0xF but **falls through** (JZ/JMP → 0x0073eef0) to the common tail whose `Queue_Mission(5=Guard,0)` at 0x0073ef71 runs on EVERY state-4 dispatch, immediately before the Rate epilogue — the repair/hunt queues are clobbered same-dispatch (single queued slot, mission-harvest-cadence.md §5). Net state-4 effect: queue Guard, Rate-delay. RepairBay/Hunt behavior is dead in practice → NOT implemented.
- No-ore transition (cadence doc §3 state 0): status 4 + IsUseless + `OwnerHouse+0x242 = 1` (Harvester types) + return 105 — our 105 dispatch return already landed in the cadence slice.
- `house+0x242` (HOUSE_ORE_SHORT_FLAG report): set-once, never cleared; readers = AI_Choose_Unit (AI build gate — deferred with the AI) and Mission_Guard_Harvester.
- Guard/Sticky handler 0x00740810 (guard-harvester-shuttle.md §2): AI-owned branch — first Dock-listed type with owned-count>0 → if Harvester && ore-short clear → `Queue(0xA,0)`, return 1; ore-short set → suppressed → FootClass::Mission_Guard fallback at `[Guard] Rate` 27-29f. Player-owned branch — no dock-count auto-resume (only a Teleporter-gated adjacent-refinery affordance + a storage-full team check — both deferred, documented).
- Preamble P3/P4 (cadence doc §3): P4 "house owns no instance of any dock type → Queue(5,0), return 1" fires at EVERY Harvest dispatch entry — catches refinery-loss in any state. State 2 with no dock found → Rate-delay, STAYS state 2 (never parks).
- Rust: player commands already re-cursor parked miners (world_commands.rs:779 ForcedReturn, :1038 MoveToOre) — no stranding. `hash_houses` (world_hash.rs:357) hashes explicit fields. `mission_queue_exact` (authority.rs:431) + `EntityReadyInputProvider` pattern at miner_dock_sequence.rs:1429. `mission_base_frames` reads any mission's `Rate` ([Guard]=.030 → 27).

## Key Technical Decisions

- Model "parked in Guard" as `MinerState::WaitNoOre` + queued Guard mission verb (no commence; promotion at the host pickup): **high** — native shape; keeps the B1 residual honest while making mission state + passive-acquire consumers native-correct.
- Skip RepairBay(0x14)/Hunt(0xF): **high** — byte-proven clobbered same-dispatch (asm this session).
- `ore_short` per-house, hashed, sticky (never cleared in play): **high** — exhaustive whole-program scan in the flag report.
- Park cadence = `[Guard] Rate` via `mission_base_frames(rules, Guard, 27)` + jitter draw: **high** — shuttle lane §3.
- Preamble P4 (no owned dock-type refinery → park) implemented at `process_miner` entry; P3's player gate nuance NOT applied to P4 (doc lists none): **medium** — cadence doc P4 wording; flagged for review.
- AI-only auto-resume (is_human == false as IsPlayerControl proxy): **medium** — 0x0050b730 semantics vs our is_human; dormant until AI lands (feedback: no AI yet). Flagged.
- handle_return/forced no-refinery-found fallback: STAY in current state (Rate-delay), do not park: **high** — native state 2 "no dock2 → Rate-delay" stays; P4 owns parking.
- SNAPSHOT_VERSION 30→31 + hash the new field in `hash_houses`: **high** — behavior-bearing state must be hashed; explicit-field pattern.

## Open Questions

- Resolved: state-4 control flow (this session's asm); resume paths (shuttle lane); player re-task (world_commands cursor writes).
- Deferred: CMIN player-adjacent-refinery auto-start affordance (§2.6) and the storage-full team check — niche, documented residuals. Weeder one-shot `this+0x6B8` — weeder is retail-dead.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/house_state.rs` | `ore_short: bool` field (serde default) |
| Modify | `src/sim/world/world_hash.rs` | hash it in `hash_houses` |
| Modify | `src/sim/snapshot.rs` | SNAPSHOT_VERSION 31 |
| Modify | `src/sim/miner/miner_system.rs` | P4 preamble, no-ore flag set, handle_wait_no_ore rewrite, return-fallback change, cadence table |
| Modify | `src/sim/miner/miner_tests.rs` | rewrite rescan tests, new park/resume tests |
| Maybe | `src/sim/world/global_parity_harness_tests.rs` | re-baseline (schema change moves all constants) |

## Sim Checklist

- [x] No floats (bool flag; integer frames); new state IS hashed (that's the point); no forbidden deps; RNG: WaitNoOre dispatches keep exactly one jitter draw each (cadence epilogue — stream position changes only via the base, not the count).

## Player-Experience Critical Items

| Class | Item | Verification |
|---|---|---|
| MILESTONE | No-ore miners park permanently (retail) instead of auto-resuming on ore regrowth; player re-task must still work | park test + world_commands cursor paths (existing tests) |
| MILESTONE | Refinery-destroyed miners park via P4 instead of looping WaitNoOre→SearchOre | P4 test |
| COMPOUNDING | Parked mission = Guard → armed HARV defends itself via passive-acquire (native) | mission-verb assertion in park test |
| RESIDUAL | CMIN adjacent-refinery auto-start + storage-full team path deferred | documented |

## Tasks

### Task 1: HouseState field + hash + snapshot bump
- `house_state.rs`: after `waypoint_edge`, add:
```rust
    /// Native house+0x242 "ore short": set once when any of this house's
    /// harvesters fails its ore search with nothing to fall back on; NEVER
    /// cleared during play (exhaustive-scan-verified). Suppresses the
    /// Guard-handler auto-resume; second native reader is the AI build gate
    /// (deferred with the AI opponent).
    #[serde(default)]
    pub ore_short: bool,
```
  Initialize `ore_short: false` at every `HouseState` literal constructor site (grep `HouseState {`).
- `world_hash.rs` `hash_houses`: add `house.ore_short.hash(hasher);` beside the economy fields.
- `snapshot.rs:85`: `const SNAPSHOT_VERSION: u32 = 31;` (comment: house ore_short field).
- Verify: `cargo check -p vera20k --lib`.

### Task 2: miner_system.rs behavior
1. **P4 preamble** in `process_miner`, after the forced_drive_track early-return, before the match:
```rust
    // Native Mission_Harvest preamble P4: a miner whose house owns no
    // dock-eligible refinery parks in Guard (queue only; the host pickup
    // flips the mission) — fires at every dispatch entry, catching
    // refinery loss in any state. Return 1 (per-frame) like native; the
    // parked WaitNoOre dispatches then run at the [Guard] cadence.
    if snap.state != MinerState::Dock
        && snap.state != MinerState::WaitNoOre
        && !house_owns_dock_refinery(sim, rules, snap)
    {
        queue_guard_park(sim, snap);
        return;
    }
```
   Gates: skip when `Dock` (mid-dock sequences own their teardown; refinery death there is handled by the dock code) and when already parked. `house_owns_dock_refinery` = any live Structure, same owner (`eq_ignore_ascii_case`), `is_refinery_type` + `harvester_can_dock_at`, `!dying`, `health>0` (building_up counts as owned — native counts instances, not operability). `queue_guard_park`: `snap.state = MinerState::WaitNoOre;` + `mission_queue_exact(entity_id, MissionId::from_known(MissionType::Guard), 0, now, &EntityReadyInputProvider)` (ignore result, same as dock exit).
2. **No-ore transition** (`handle_search_ore` miss): keep `state = WaitNoOre` and the 105 dispatch return; DROP the `rescan_cooldown.arm`; add: set owner-house `ore_short = true` (all miner kinds here are Harvester-typed: War + Chrono; slave never dispatches here) via `house_state_for_owner_mut`. Queue Guard here too (native state 4 queues at +105; queueing at entry is the one-dispatch-early simplification — NO: keep native timing: do NOT queue here; `handle_wait_no_ore` queues on its first dispatch at +105).
3. **`handle_wait_no_ore` rewrite** (needs `sim: &mut Simulation`, `rules`, and snap):
```rust
fn handle_wait_no_ore(sim: &mut Simulation, rules: &RuleSet, snap: &mut MinerSnapshot) {
    // Parked-in-Guard model (native state 4 + the Guard/Sticky override
    // 0x00740810). Every parked dispatch re-queues Guard (native state 4
    // queues 5 before its Rate epilogue; queue is a single overwrite slot).
    let now = sim.session.binary_frame;
    let _ = sim.mission_queue_exact(
        snap.entity_id,
        crate::sim::mission::MissionId::from_known(crate::sim::mission::MissionType::Guard),
        0,
        now,
        &crate::sim::mission::authority::EntityReadyInputProvider,
    );
    // Auto-resume = the shuttle's AI-owned branch: house owns a dock-type
    // refinery AND the sticky ore-short flag is clear. Player-owned houses
    // have no dock-count auto-resume (native IsPlayerControl split); the
    // CMIN adjacent-refinery affordance is a documented residual.
    let owner = sim.interner.resolve(snap.owner).to_string();
    let house_is_human = crate::sim::house_state::house_state_for_owner(
        &sim.houses, &owner, &sim.interner,
    ).is_none_or(|h| h.is_human);
    let ore_short = crate::sim::house_state::house_state_for_owner(
        &sim.houses, &owner, &sim.interner,
    ).is_some_and(|h| h.ore_short);
    if !house_is_human && !ore_short && house_owns_dock_refinery(sim, rules, snap) {
        snap.state = MinerState::SearchOre;
        let _ = sim.mission_queue_exact(
            snap.entity_id,
            crate::sim::mission::MissionId::from_known(crate::sim::mission::MissionType::Harvest),
            0,
            now,
            &crate::sim::mission::authority::EntityReadyInputProvider,
        );
    }
}
```
   (Adapt getter names to the real `house_state` API; resolve owner once. `rescan_cooldown` is no longer read — leave the field for save-compat, stop arming it everywhere.)
4. **handle_return / handle_forced_return fallbacks**: `find_nearest_refinery == None` → do nothing (stay in the current state; the state-2 Rate epilogue paces retries). Delete the `WaitNoOre` transitions + `rescan_cooldown.arm` there (P4 owns real parking).
5. **Cadence table** (`apply_native_dispatch_cadence`): split `WaitNoOre` out of the `[Harvest]`-Rate arm into a `[Guard]`-Rate arm: `guard_rate_dispatch_delay` = `mission_base_frames(rules, MissionType::Guard, GUARD_RATE_FALLBACK_FRAMES /* 27 */)` + jitter(0..=2) (one draw, same stream). SearchOre→WaitNoOre keeps 105.
6. Update the module-header cadence map comment (WaitNoOre → Guard cadence; P4 preamble).
- Verify: `cargo check`, then `cargo test -p vera20k --lib miner` (expect rescan-test failures → Task 3).

### Task 3: Tests
- Rewrite the two `rescan_cooldown` contract tests (miner_tests.rs ~2384-2450) to the park contract (no auto-rescan; state stays WaitNoOre; cite the shuttle lane).
- New:
  - `no_ore_miner_parks_in_guard_and_sets_ore_short`: miner + own refinery, NO ore anywhere → 1 tick → WaitNoOre + house.ore_short == true; +106 ticks → mission queued/current == Guard; +200 ticks with ore inserted nearby → STILL WaitNoOre (sticky suppression; human house).
  - `ai_house_parked_miner_resumes_when_refinery_exists_and_not_ore_short`: is_human=false house fixture; miner with cargo, no refinery → P4 parks it; spawn refinery → within ~30 ticks state leaves WaitNoOre (SearchOre) and mission queued Harvest.
  - `full_miner_with_refineries_but_failed_selection_stays_returning`: refinery present but all `building_up` → handle_return None → state stays ReturnToRefinery (not parked; P4 sees an owned instance).
  - `refinery_loss_parks_miner_via_preamble`: harvesting miner, kill the only refinery (dying=true... P4's owned check excludes dying → parks) → WaitNoOre + Guard queued, harvest interrupted.
- Full suite; fix fallout (harness constants WILL move — schema change; re-baseline all three + FINAL_STREAM_STATES only if the stream moved, with a documented reason: "v31 house ore_short field; harness scenario value false everywhere; composition-only for the streams"). Record the literal `test result:` line.

### Task 4: Commit + live verify
- Commit: `miner: native no-ore/no-refinery Guard parking; house ore_short (v31)`.
- Rebuild; `RA2_QUICKPLAY=minerloop.map` — normal loop must be unaffected (ore exists); then confirm parking: after the session's ore field depletes fully (or via a quick observation of the log for WaitNoOre never flapping back), kill instance. Minimum bar: the normal loop unchanged + no WaitNoOre→SearchOre flapping in the log.

## Sources & References
- asm read this session: 0x0073eea6..0x0073ef8c (state-4 tail, Guard queue 0x0073ef71).
- docs/scans/trace-swarm-20260728/guard-harvester-shuttle.md; mission-harvest-cadence.md §3/§5; docs/research/miner/HOUSE_ORE_SHORT_FLAG_AND_DOCK_KEY_GHIDRA_REPORT.md.
- Code: miner_system.rs (process_miner, handle_search_ore, handle_wait_no_ore, handle_return, apply_native_dispatch_cadence), house_state.rs:57, world_hash.rs:357, snapshot.rs:85, authority.rs:431, world_commands.rs:779/:1038.
- Prior commits: 43318830, f57be00f, 8c74f28f.
