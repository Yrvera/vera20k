# S5 Passive-Acquire Scan-Timer + scenario_rng Consume — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Builds directly on the
> uncommitted S4b changes in this session's working tree (SNAPSHOT_VERSION currently 25;
> `techno_common_post(sim, id, rules)` already rules-threaded; `damage_spark_spawn_threshold`,
> `MissionTimer`, `next_range_u32_inclusive`, `state()`-diff test idiom all present).

**Goal:** Reproduce gamemd's per-unit passive-acquire scan cadence and its per-scan `scenario_rng`
`n(0,2)` draw in `techno_common_post`, without setting TarCom (consume-only).

**Architecture:** Mirrors S4b exactly — a rules-threaded per-Unit helper in the S4a bracket host, a
hashed per-entity timer gating the draw, `state()`-diff tests, version bump, golden re-baseline.

**Design Doc:** `docs/plans/2026-06-11-s5-passive-acquire-consume-design.md`

---

## Grounding Summary

- **Docs:** `GRIZZLY_PASSIVE_TARGET_SCANNER_VTABLE_39C_GHIDRA_REPORT.md` (audited 2026-06-11, YELLOW —
  the audit *added* the per-scan RNG fact the doc omitted; see AUDIT_LOG). Design doc §"Why
  consume-only" carries the RNG analysis.
- **Ghidra (verified this session):** scan timer `+0x180/+0x188` + the `RandomRanged(0,2)` on
  `Scen->Random` re-arm at `0x00709820` top (`disassemble 0x00709820`); base `RulesClass+0xe04`
  (`GuardAreaTargetingDelay`) / `+0xe08` (`NormalTargetingDelay`) via `RulesClass__ReadGeneral`;
  AI_Update gate `mission∈{2,10,5}` (`0x006fa64e..0x006fa67d`); `vtable+0x4c4`=`0x004DF310`=`(+0x5c4==0x1d)`;
  `Calculate_Threat_Score 0x0070CD10` RNG-free; `Evaluate_Candidate 0x006F7CA0` has a conditional
  AI-only cloak-detect `RandomRanged(0,99)` (S5b).
- **Repo pattern:** S4b (`techno_ai.rs::techno_common_post`, the sparking-prob parse in `ruleset.rs`,
  the `MissionTimer` primitive in `sim/mission/timer.rs`, the `state()`-diff tests).
- **INI:** `[General] NormalTargetingDelay=27`, `GuardAreaTargetingDelay=36` (`rulesmd.ini:304-305`;
  not currently parsed).
- **Unknown after grounding:** `+0x5c4` field semantics (command-state; default-open, flagged); the
  `else`-branch `vtable+0x4cc` RNG behavior (deferred — only reached when `+0x5c4==29`).

## Key Technical Decisions

- **Passive-acquire path always uses `NormalTargetingDelay` (27).** — The AI_Update passive block
  gates mission∈{2,10,5}; mission 11 (AreaGuard, → GuardAreaTargetingDelay) reaches the scanner only
  via the AreaGuard mission handler (separate slice). **Confidence:** high — **Source:** Ghidra
  `0x006fa64e` gate + `0x0070983a` selector.
- **Scan timer keyed to `session.binary_frame`** (= `g_CurrentFrameCounter`), modeled as
  `MissionTimer`. **Confidence:** high — **Source:** `0x0070982e` (`+0x180=g_CurrentFrameCounter`),
  `rulesmd.ini:305` ("targeting rates in frames").
- **`passive_scan_consume` runs FIRST in `techno_common_post`, before the S4b damage-spark draw.** —
  gamemd order: passive-acquire (post-Mission_Dispatch) precedes the damage-particle block.
  **Confidence:** high — **Source:** `TechnoClass__AI_Update` decompile order.
- **`vtable+0x4c4`/`+0x5c4==29` scan-suppress term: default-open, not modeled.** — VERA has no
  equivalent command-state field; the term is a transient command-0x1d state. **Confidence:**
  medium (reachability for movers unverified) — **flag for /review-plan + S5b.**
- **Parse both delay keys now** (GuardArea used by the later AreaGuard slice) but the consume helper
  selects `if mission==AreaGuard {guard_area} else {normal}` — resolves to `normal` on this path,
  matching `0x00709820`'s own selector. **Confidence:** high.

## Open Questions

### Resolved During Planning
- vtable+0x4c4 → `(+0x5c4==0x1d)` (Ghidra `0x004DF310`).
- `+0x5c4` is a command-state set in `FootClass__Assign_Target_Command` (`0x004df134`).
- `MissionType::AreaGuard==11` (Rust `mission/mod.rs:47`).
- Passive path base = NormalTargetingDelay (mission never 11 in the AI_Update passive block).

### Deferred to Implementation
- Exact set of golden tests that shift (discovered by running the suite in Task 8).
- Whether `+0x5c4==29` is reachable for an opportunity-firing mover (verify in S5b; default-open now).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | Parse `NormalTargetingDelay`/`GuardAreaTargetingDelay` into `GeneralRules` |
| Modify | `src/sim/world/techno_ai.rs` | Un-gate `s4c_passive_acquire_eligible`; add `passive_scan_consume`; call it in `techno_common_post`; S5 tests |
| Modify | `src/sim/game_entity.rs` | Hashed `passive_scan_timer: MissionTimer` field + ctor init |
| Modify | `src/sim/world/world_hash.rs` | Fold `passive_scan_timer` into `hash_entities` |
| Modify | `src/sim/snapshot.rs` | `SNAPSHOT_VERSION` 25→26 + pin test |
| Modify | `src/sim/world/slice6_retask_tests.rs`, `global_parity_harness_tests.rs` (+ others found in Task 8) | Re-baseline golden hashes |

## Interface Changes
- `s4c_passive_acquire_eligible` loses its `#[cfg(any(test, debug_assertions))]` gate (becomes
  always-compiled). Consumers: the S4c debug shadow (unchanged) + the new release consume path.
- `GameEntity` gains a public `passive_scan_timer` field (serialized, hashed). Consumers: `new()`,
  `hash_entities`, snapshot serde.

## Sim Checklist
- [x] No float in sim logic — delays are `u32` frames; timer is integer-frame.
- [x] New state (`passive_scan_timer`) folded into the state hash.
- [x] No render/ui/sidebar/audio/net dependency (`techno_ai.rs` stays sim-internal).
- [x] Tick-ordering: draw runs in the existing `object_ai_stage`/`techno_common_post`, BEFORE the
  S4b draw; no new phase.
- [x] BTreeMap order: `techno_common_post` is driven by the LogicVector live order (unchanged).

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1 | `NormalTargetingDelay=27` default | Wrong base shifts every scan → desync | INI + `RulesClass__ReadGeneral` |
| 5 | `n(0,2)` on `scenario_rng`, exactly one per re-arm | Draw count/stream/instance = lockstep | `disasm 0x00709820`; `state()`-diff test |
| 5 | Re-arm `duration = base + roll`, `start = binary_frame`; fire on expiry only | Cadence sets WHICH frames draw | Ghidra timer block; tests |
| 5 | `passive_scan_consume` BEFORE S4b spark draw | RNG draw order within the tick | AI_Update order; Task 6 test |
| 5 | SENTINEL initial fires on first eligible tick | First-scan frame alignment | `0x006fa64e`; test |
| 5 | `vtable+0x4c4`/`+0x5c4==29` term **default-open (NOT modeled)** | Over-draw if reachable for movers | **flagged — /review-plan + S5b** |

---

## Tasks

### Task 1: Parse the two targeting-delay keys into `GeneralRules`

**Why:** The scan base must come from INI (default 27/36), not be hardcoded. Foundation for the draw.

**Files:** Modify `src/rules/ruleset.rs`

**Pattern:** Exactly the `condition_red_sparking_probability` parse added by S4b.

**Step 1:** In `struct GeneralRules`, after the sparking-threshold fields, add:
```rust
    /// `NormalTargetingDelay=` ([General]) — base frame delay between passive-acquire
    /// scans for missions Move/Guard/Harvest. Default **27**. The per-scan jitter
    /// `RandomRanged(0,2)` is added on top. Frames (= g_CurrentFrameCounter).
    pub normal_targeting_delay: u32,
    /// `GuardAreaTargetingDelay=` ([General]) — base scan delay for the Area Guard
    /// mission (11). Default **36**. Consumed by the AreaGuard mission handler (a
    /// later slice); the AI_Update passive-acquire path (missions {2,10,5}) always
    /// uses `normal_targeting_delay`.
    pub guard_area_targeting_delay: u32,
```

**Step 2:** In `impl Default for GeneralRules`, after the spark-threshold defaults, add:
```rust
            normal_targeting_delay: 27,
            guard_area_targeting_delay: 36,
```

**Step 3:** In `GeneralRules::from_ini`'s `Self { … }`, after the spark-threshold fields, add:
```rust
            normal_targeting_delay: general.get_i32("NormalTargetingDelay").unwrap_or(27).max(0) as u32,
            guard_area_targeting_delay: general.get_i32("GuardAreaTargetingDelay").unwrap_or(36).max(0) as u32,
```

**Step 4: Add test** (in the `ruleset.rs` tests module):
```rust
    #[test]
    fn targeting_delays_default_and_override() {
        let d = GeneralRules::default();
        assert_eq!(d.normal_targeting_delay, 27);
        assert_eq!(d.guard_area_targeting_delay, 36);
        let none = GeneralRules::from_ini(&IniFile::from_str("[Foo]\n"));
        assert_eq!(none.normal_targeting_delay, 27);
        assert_eq!(none.guard_area_targeting_delay, 36);
        let g = GeneralRules::from_ini(&ini_with_general(
            "NormalTargetingDelay=15\nGuardAreaTargetingDelay=20",
        ));
        assert_eq!(g.normal_targeting_delay, 15);
        assert_eq!(g.guard_area_targeting_delay, 20);
    }
```

**Step 5: Verify:** `cargo test -p vera20k --lib targeting_delays` → PASS.

**Step 6: Commit.**

### Task 2: Un-gate `s4c_passive_acquire_eligible` for release use

**Why:** The consume path (release) needs the eligibility predicate; it's currently debug-only.

**Files:** Modify `src/sim/world/techno_ai.rs`

**Step 1:** Remove the `#[cfg(any(test, debug_assertions))]` attribute immediately above
`fn s4c_passive_acquire_eligible(`. Leave the `debug_s4c_passive_acquire_shadow` method gated.

**Step 2:** Since `MissionType` is only imported under `#[cfg(any(test, debug_assertions))]` at the
top of the file, change that import to unconditional:
```rust
use crate::sim::mission::MissionType;
```
(remove its `#[cfg(any(test, debug_assertions))]` line; it is now used by the always-compiled
predicate).

**Step 3: Verify:** `cargo check -p vera20k` → compiles (no unused-cfg or unused-import warnings for
`MissionType` / `s4c_passive_acquire_eligible`).

**Step 4: Commit.**

### Task 3: Add the hashed `passive_scan_timer` field to `GameEntity`

**Why:** The `+0x180/+0x188` scan timer — per-entity authoritative state gating the draw.

**Files:** Modify `src/sim/game_entity.rs`

**Pattern:** The `damage_particle_live_until` field S4b added (same neighborhood, same serde idiom).

**Step 1:** In `struct GameEntity`, immediately after the `damage_particle_live_until` field, add:
```rust
    /// Sim-side model of gamemd's passive-acquire scan timer (`+0x180` start /
    /// `+0x188` duration, in `binary_frame` = g_CurrentFrameCounter units). When
    /// expired and the passive-acquire gate is open, the unit makes one
    /// `scenario_rng` scan draw and re-arms to `NormalTargetingDelay + n(0,2)`.
    /// SENTINEL (default) ⇒ fires on the first eligible tick (gamemd start==-1,
    /// duration==0). Hashed (it gates future scenario_rng draws). Consume-only:
    /// the scan body / TarCom-set is S5b.
    #[serde(default)]
    pub passive_scan_timer: crate::sim::mission::MissionTimer,
```

**Step 2:** In `GameEntity::new()`, immediately after `damage_particle_live_until: 0,`, add:
```rust
            passive_scan_timer: crate::sim::mission::MissionTimer::default(),
```
(`MissionTimer::default()` = SENTINEL start, duration 0 — fires first eligible tick.)

**Step 3: Verify:** `cargo check -p vera20k` → compiles. Confirm `MissionTimer` derives
`Serialize`/`Deserialize`/`Default` (it does — used by `MissionCom` and `BuildingGateRuntime`).

**Step 4: Commit.**

### Task 4: Fold `passive_scan_timer` into the state hash

**Why:** The timer gates future draws; a divergence desyncs the stream — must be hashed.

**Files:** Modify `src/sim/world/world_hash.rs`

**Pattern:** The explicit `start_frame`/`duration` fold used for `mission.timer` (`hash_mission_com`)
and `building_gate`.

**Step 1:** In `hash_entities`, immediately after the
`entity.damage_particle_live_until.hash(hasher);` line, add:
```rust
            // S5 passive-acquire scan timer (`+0x180/+0x188`-equivalent). Hashed
            // because it gates future scenario_rng scan draws. Explicit fold
            // (matches the mission.timer / building_gate timer idiom).
            entity.passive_scan_timer.start_frame.hash(hasher);
            entity.passive_scan_timer.duration.hash(hasher);
```

**Step 2: Verify:** `cargo check -p vera20k` → compiles (the fold itself shifts hashes; goldens are
re-baselined in Task 8).

**Step 3: Commit.**

### Task 5: Implement `passive_scan_consume` and call it first in `techno_common_post`

**Why:** The core — the gated, per-scan `scenario_rng` draw + timer re-arm, in gamemd order.

**Files:** Modify `src/sim/world/techno_ai.rs`

**Pattern:** S4b's `techno_common_post` damage-spark body (read entity facts → resolve type → gate →
draw from `sim.scenario_rng` → write hashed state).

**Step 1:** Add the helper (place it just above `techno_common_post`):
```rust
/// S5 passive-acquire scan-timer + `scenario_rng` consumption (consume-only).
///
/// gamemd `TechnoClass::AI_Update` (post-`Mission_Dispatch`): once the scan timer
/// (`+0x180/+0x188`, in `g_CurrentFrameCounter` frames) has expired AND the gate is
/// open (`vtable+0x4c4()==0` AND mission ∈ {Move 2, Harvest 10, Guard 5} AND the
/// OpportunityFire/Guard passive gate), `vtable+0x39C` re-arms the timer
/// (`start=frame`, `duration = NormalTargetingDelay + RandomRanged(0,2)`) and draws
/// `n(0,2)` on `Scen->Random` — UNCONDITIONALLY, target or not. This reproduces only
/// that draw + cadence; the scan body / TarCom-set / cloak-detect draw is S5b.
///
/// The `vtable+0x4c4` / `+0x5c4==29` scan-suppress term is NOT modeled (VERA has no
/// equivalent command-state field) — default-open; flagged for S5b.
fn passive_scan_consume(sim: &mut Simulation, id: u64, rules: &RuleSet) {
    let Some(e) = sim.substrate.entities.get(id) else {
        return;
    };
    let mission = e.mission.current;
    let timer = e.passive_scan_timer;
    let Some(obj) = rules.object(sim.interner.resolve(e.type_ref)) else {
        return;
    };
    // Eligibility gate (S4c predicate): mission ∈ {Move,Guard,Harvest} + weapon +
    // (OpportunityFire ∨ Guard). The `vtable+0x4c4`/`+0x5c4` term is default-open.
    let has_weapon = obj.primary.is_some() || obj.secondary.is_some();
    if !s4c_passive_acquire_eligible(mission, has_weapon, obj.opportunity_fire) {
        return;
    }
    // Timer gate: only scan (and draw) when the scan timer has expired.
    let frame = sim.session.binary_frame;
    if !timer.is_expired(frame) {
        return;
    }
    // The scan draw: one `n(0,2)` on scenario_rng, then re-arm. The AreaGuard base
    // (mission 11) is unreachable on the passive path (handled by the AreaGuard
    // mission handler), so this resolves to NormalTargetingDelay.
    let roll = sim.scenario_rng.next_range_u32_inclusive(0, 2);
    let base = if mission == MissionType::AreaGuard {
        rules.general.guard_area_targeting_delay
    } else {
        rules.general.normal_targeting_delay
    };
    let duration = base.saturating_add(roll);
    if let Some(e) = sim.substrate.entities.get_mut(id) {
        e.passive_scan_timer = crate::sim::mission::MissionTimer::armed(frame, duration);
    }
}
```

**Step 2:** Call it at the TOP of `techno_common_post`, before the S4b damage-spark logic:
```rust
fn techno_common_post(sim: &mut Simulation, id: u64, rules: Option<&RuleSet>) {
    let Some(rules) = rules else {
        return;
    };
    // S5 passive-acquire scan consumption runs FIRST (gamemd order:
    // passive-acquire precedes the damage-particle block in AI_Update).
    passive_scan_consume(sim, id, rules);

    // --- S4b damage-Spark draw (unchanged below) ---
    let Some(entity) = sim.substrate.entities.get(id) else {
        return;
    };
    // … existing S4b body …
```
(Restructure: `techno_common_post` already unwraps `rules` at the top; keep that unwrap, add the
`passive_scan_consume(sim, id, rules)` call right after it, then the existing S4b body continues with
the `entity` re-fetch. The S4b body's `let Some(rules) = rules else { return };` is now the shared
unwrap — ensure there is exactly one unwrap and both draws use it.)

**Step 3: Verify:** `cargo check -p vera20k` → compiles.

**Step 4: Commit.**

### Task 6: S5 consume tests (`state()`-diff exactness)

**Why:** Pin the 0/1 draw count per gate case, the cadence, the instance, and the draw ORDER.

**Files:** Modify `src/sim/world/techno_ai.rs` (tests module)

**Pattern:** S4b's `s4b_*` tests (build a minimal `RuleSet`, insert a typed unit, compare
`scenario_rng.state()` to an `expect` clone advanced by the exact draws).

**Step 1:** Add a helper rules builder with one armed OpportunityFire vehicle "TANKT":
```rust
    fn opp_fire_rules(normal_delay: &str) -> RuleSet {
        use crate::rules::ini_parser::IniFile;
        let text = format!(
            "[General]\n\
BuildSpeed=0.75\nMultipleFactory=0.7\nLowPowerPenaltyModifier=1.25\n\
MinLowPowerProductionSpeed=0.4\nMaxLowPowerProductionSpeed=0.85\n\
NormalTargetingDelay={normal_delay}\n\n\
[VehicleTypes]\n1=TANKT\n[InfantryTypes]\n[AircraftTypes]\n[BuildingTypes]\n\n\
[TANKT]\nPrimary=Gun\nOpportunityFire=yes\n\n[Gun]\nRange=5\n"
        );
        RuleSet::from_ini(&IniFile::from_str(&text)).expect("opp-fire rules parse")
    }

    fn insert_opp_unit(sim: &mut Simulation, id: u64, mission: MissionType) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("TANKT");
        let mut e = GameEntity::new(
            id, 5, 5, 0, 0, owner,
            crate::sim::components::Health { current: 100, max: 100 },
            type_ref, EntityCategory::Unit, 0, 5, true,
        );
        e.mission.current = mission;
        sim.substrate.entities.insert(e);
    }
```

**Step 2:** Add the tests (one draw on first eligible tick; none while timer live; none when
ineligible; re-arm value; scenario-not-main; draw-before-S4b-order). Concretely:
```rust
    #[test]
    fn s5_one_draw_on_first_eligible_tick() {
        // Eligible mover, SENTINEL timer ⇒ fires on the first tick: exactly one n(0,2),
        // timer re-armed to NormalTargetingDelay + roll at the current binary_frame.
        let rules = opp_fire_rules("27");
        let mut sim = Simulation::new();
        insert_opp_unit(&mut sim, 1, MissionType::Move);
        let frame = sim.session.binary_frame;
        let mut expect = sim.scenario_rng.clone();
        let main = sim.main_rng.state();
        passive_scan_consume(&mut sim, 1, &rules);
        let roll = expect.next_range_u32_inclusive(0, 2);
        assert_eq!(sim.scenario_rng.state(), expect.state(), "exactly one n(0,2)");
        assert_eq!(sim.main_rng.state(), main, "scenario stream only");
        let t = sim.substrate.entities.get(1).unwrap().passive_scan_timer;
        assert_eq!((t.start_frame, t.duration), (frame, 27 + roll), "re-armed base+roll");
    }

    #[test]
    fn s5_no_draw_while_timer_live() {
        let rules = opp_fire_rules("27");
        let mut sim = Simulation::new();
        insert_opp_unit(&mut sim, 1, MissionType::Move);
        passive_scan_consume(&mut sim, 1, &rules); // fires, arms to ~27
        let frozen = sim.scenario_rng.state();
        passive_scan_consume(&mut sim, 1, &rules); // same frame, not expired ⇒ no draw
        assert_eq!(sim.scenario_rng.state(), frozen, "live timer blocks the draw");
    }

    #[test]
    fn s5_no_draw_when_ineligible() {
        // No weapon ⇒ gate fails ⇒ zero draws (a no-Primary type).
        use crate::rules::ini_parser::IniFile;
        let text = "[General]\nBuildSpeed=0.75\nMultipleFactory=0.7\n\
LowPowerPenaltyModifier=1.25\nMinLowPowerProductionSpeed=0.4\nMaxLowPowerProductionSpeed=0.85\n\n\
[VehicleTypes]\n1=NOGUN\n[InfantryTypes]\n[AircraftTypes]\n[BuildingTypes]\n\n[NOGUN]\n";
        let rules = RuleSet::from_ini(&IniFile::from_str(text)).unwrap();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern("NOGUN");
        let mut e = GameEntity::new(1, 5, 5, 0, 0, owner,
            crate::sim::components::Health { current: 100, max: 100 },
            type_ref, EntityCategory::Unit, 0, 5, true);
        e.mission.current = MissionType::Move;
        sim.substrate.entities.insert(e);
        let scen = sim.scenario_rng.state();
        passive_scan_consume(&mut sim, 1, &rules);
        assert_eq!(sim.scenario_rng.state(), scen, "ineligible ⇒ no draw");
    }

    #[test]
    fn s5_draws_before_s4b_spark() {
        // Ordering: techno_common_post must run the passive-scan draw BEFORE the S4b
        // damage-spark draw. With an eligible mover that is ALSO a (synthetic) cyborg
        // below-yellow spark type, two draws happen; the FIRST must be the scan n(0,2).
        // (Build a type that is both opp-fire-eligible and emits_damage_spark.)
        // … see Task 6 note: assert the scenario stream after techno_common_post equals
        //   a reference advanced by [scan n(0,2)] THEN [spark roll], not the reverse.
    }
```
(The `s5_draws_before_s4b_spark` test needs a type that is simultaneously an `OpportunityFire`
infantry with `Cyborg=yes` + a Spark `DamageParticleSystems` + below-ConditionYellow health, so both
draws fire; assert the reference order is scan-then-spark. Write it concretely against the S4b
`cyborg_rules` helper extended with `Primary=Gun\nOpportunityFire=yes`.)

**Step 3: Verify:** `cargo test -p vera20k --lib s5_` → PASS (all).

**Step 4: Commit.**

### Task 7: Bump `SNAPSHOT_VERSION` 25 → 26

**Why:** New hashed field changes the serialized shape + the hash.

**Files:** Modify `src/sim/snapshot.rs`

**Step 1:** Change `const SNAPSHOT_VERSION: u32 = 25;` → `26`, extending the ladder comment with
"S5 added the hashed `passive_scan_timer` + its per-scan draws (25 → 26)."

**Step 2:** Update the pin test `snapshot_version_is_25` → `snapshot_version_is_26` (name + assert).

**Step 3: Verify:** `cargo test -p vera20k --lib snapshot_version` → PASS.

**Step 4: Commit.**

### Task 8: Golden re-baseline (the big one)

**Why:** Every opportunity-firing mover now draws `scenario_rng`; all such scenarios shift. Must
re-baseline AND prove the shift is the new draws only.

**Files:** Modify the golden-baseline consts found below (at minimum
`slice6_retask_tests.rs::SLICE6_BASELINE_HASH`, `global_parity_harness_tests.rs::GLOBAL_HARNESS_FINAL_HASH`,
plus any `rng_routing_tests` cursor pins and others surfaced by the run).

**Step 1:** `cargo test -p vera20k --lib 2>&1 | grep -nE "FAILED|left:|right:"` — enumerate every
failing hardcoded-hash test and capture each `left` (new) value.

**Step 2 (proof the shift is benign):** Temporarily comment the two `passive_scan_timer` fold lines
in `world_hash.rs` AND early-return at the top of `passive_scan_consume` (no draw). Re-run the failing
tests — they must pass at their OLD baselines (proving the only deltas are the new fold + the new
draws, no collateral). Restore both.
*Note:* unlike S4b, the draws are NOT zero here — so the baselines WILL change even with the fold
restored; this proof isolates "fold + intended draws" from any accidental perturbation. Document the
proof in each re-baselined const's comment.

**Step 3:** Paste each new `left` value into its const; extend each const's comment with "Re-baselined
for S5 (passive-acquire scan-timer fold + per-scan scenario_rng n(0,2) for opportunity-firing movers
— intended draw shift, proven isolated)."

**Step 4: Verify:** `cargo test -p vera20k --lib 2>&1 | grep "test result:"` → 0 failed.
Then `cargo test -p vera20k --tests 2>&1 | grep -cE "FAILED"` → 0.

**Step 5: Commit.**

### Task 9: Verify against gamemd

**Why:** Confirm cadence + draw + gate match the binary.

**Verify (no code):**
- A stock opportunity-firing mover (e.g. `[MTNK]` Grizzly on Move) draws exactly one `scenario_rng`
  `n(0,2)` every `27 + (0..2)` frames while eligible; none when not eligible. (Ghidra
  `0x00709820` + `0x006fa64e`.)
- The draw is on `scenario_rng` (`Scen->Random`, `[0x00A8B230]+0x218`), never `main_rng`.
- The passive-scan draw precedes the S4b damage-spark draw in a tick where both fire.
- `vtable+0x4c4`/`+0x5c4==29` default-open is the only unmodeled gate term — re-confirm at `/review-plan`
  whether command-0x1d is reachable for opportunity-firing movers (if yes, model it before merge).

## Sources & References
- **Design doc:** `docs/plans/2026-06-11-s5-passive-acquire-consume-design.md`
- **Ghidra report:** `docs/research/GRIZZLY_PASSIVE_TARGET_SCANNER_VTABLE_39C_GHIDRA_REPORT.md` (YELLOW; AUDIT_LOG 2026-06-11)
- **gamemd addresses:** `0x00709820` (Retaliate_And_Scan / timer re-arm + `n(0,2)`), `RulesClass+0xe04/+0xe08`
  (`RulesClass__ReadGeneral`), `0x006fa64e..0x006fa67d` (AI_Update gate), `0x004DF310` (`vtable+0x4c4`=`+0x5c4==0x1d`),
  `0x004df134` (`FootClass__Assign_Target_Command` sets `+0x5c4`), `0x0070CD10` (RNG-free score), `0x006F7CA0`
  (`Evaluate_Candidate` cloak-detect `n(0,99)` — S5b).
- **INI:** `rulesmd.ini:304-305` `NormalTargetingDelay=27` / `GuardAreaTargetingDelay=36`.
- **Related code:** S4b in `techno_ai.rs` (`techno_common_post`), `ruleset.rs` (sparking parse),
  `sim/mission/timer.rs` (`MissionTimer`), `sim/mission/mod.rs:47` (`AreaGuard=11`).
