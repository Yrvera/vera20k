# Infantry Fear/Prone/Crawls Runtime - Implementation Plan

> Execute this plan task by task. Keep each task narrow, and run the listed
> verification after the relevant task group. Do not mix unrelated GI/deploy
> cleanup into this change.

**Goal:** Full generic infantry fear/prone parity for normal play, with GI as
the primary stock validation case.

**Design Doc:** [docs/plans/2026-05-16-infantry-fear-prone-design.md](2026-05-16-infantry-fear-prone-design.md)

## Grounding Summary

- GI has `Crawls=yes` in `artmd.ini [GI]`.
- Binary verified `Crawls=+0xEBD`, `Fearless=+0xEBC`, and
  `Fraidycat=+0xEBF`.
- Binary verified `InfantryClass::GetMovementSpeed`: prone with `Crawls=yes`
  uses ceiling two-thirds speed; prone with `Crawls=no` uses
  `speed + speed / 2`.
- Binary verified fear behavior:
  - first normal hit sets fear to 100;
  - first fraidycat hit sets fear to 300;
  - repeated/no-damager fear adds 50, 25, or 12 by health threshold and clamps
    at 300;
  - fear decay happens before Down/Up checks;
  - Down starts when post-decay fear is 50 or higher;
  - Up starts when post-decay fear is below 50;
  - `Fearless=yes` and veteran FEARLESS ability block fear application;
  - the decay handler checks type `Fearless=yes` for the decrement and does not
    call the veteran FEARLESS ability query.
- Repo currently has prone sequence kinds but no runtime prone bit.
- Direct and AOE damage currently use `animation_is_prone`, which is explicitly
  marked as temporary.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Add rule/runtime object flags |
| Modify | `src/rules/art_data.rs` | Parse art `Crawls=` |
| Modify | `src/rules/ruleset.rs` | Merge art `Crawls` into infantry object types |
| Modify | `src/sim/game_entity.rs` | Add `InfantryRuntime` state |
| Add | `src/sim/infantry.rs` or `src/sim/infantry/mod.rs` | Fear/prone helpers |
| Modify | `src/sim/mod.rs` | Export infantry module |
| Modify | `src/sim/world/mod.rs` | Tick fear decay/prone transitions before combat |
| Modify | `src/sim/combat/mod.rs` | Apply fear on landed damage; use runtime prone |
| Modify | `src/sim/combat/combat_aoe.rs` | Use runtime prone |
| Modify | `src/sim/movement/movement_tick.rs` | Apply prone speed multiplier |
| Modify | `src/sim/animation.rs` | Drive prone/crawl/fire-prone from runtime |
| Modify | `src/sim/world/world_hash.rs` | Include fear/prone in deterministic state hash |
| Modify/Add | tests | Parser, fear math, combat, AOE, movement, animation |

## Key Technical Decisions

| Decision | Confidence | Source |
|----------|------------|--------|
| Use sim-owned `InfantryRuntime` instead of animation as prone source of truth | High | Binary `InfantryClass` has separate fear/prone state; repo combat currently labels animation prone as temporary |
| Parse `Fearless`/`Fraidycat` from rules and `Crawls` from art | High | Binary offsets `+0xEBC`, `+0xEBF`, `+0xEBD`; stock INI locations |
| Run fear decay/prone transitions before combat | Medium | Closest available repo tick slot after deploy state and before combat reads prone |
| Apply damage fear when damage actually lands | Medium | Current combat pipeline batches damage; avoids mid-batch state mutation |
| Use post-decay Down/Up thresholds | High | `InfantryClass__Fear_Decay_Handler @ 0x005200b0` decrements before threshold checks |
| Use ceiling two-thirds for `Crawls=yes` speed | High | `InfantryClass__GetMovementSpeed @ 0x00521d80` integer formula |

## Sources & References

- `docs/research/GI_GHIDRA_REPORT.md`
- `ini/artmd.ini [GI] Crawls=yes`
- `ini/rulesmd.ini [AudioVisual] ConditionRed=25%`
- `ini/rulesmd.ini [AudioVisual] ConditionYellow=50%`
- `gamemd.exe 0x005200b0 InfantryClass__Fear_Decay_Handler`
- `gamemd.exe 0x00518c00 InfantryClass__SetFear`
- `gamemd.exe 0x00521d80 InfantryClass__GetMovementSpeed`
- `gamemd.exe 0x00521c10 InfantryClass__Panic_SetFear300`
- `src/sim/combat/mod.rs`
- `src/sim/combat/combat_aoe.rs`
- `src/sim/animation.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/world/mod.rs`
- `src/sim/world/world_hash.rs`
- `src/rules/object_type.rs`
- `src/rules/art_data.rs`
- `src/rules/ruleset.rs`

## Task 1 - Parse Infantry Flags

1. Add these `ObjectType` fields:
   - `fearless: bool`
   - `fraidycat: bool`
   - `crawls: bool`
2. Parse `Fearless=` and `Fraidycat=` in `ObjectType::from_ini_section`.
3. Default `crawls` to false in `ObjectType::from_ini_section`; it is filled by
   art merge.
4. Update literal `ObjectType` test helpers if compilation requires it.
5. Add parser tests for `Fearless=yes` and `Fraidycat=yes`.

Verification:

```powershell
cargo test object_type --lib
```

## Task 2 - Parse and Merge Art `Crawls=`

1. Add `crawls: bool` to `ArtEntry`.
2. Parse `section.get_bool("Crawls").unwrap_or(false)` in `ArtRegistry::from_ini`.
3. Extend `RuleSet::merge_art_data`:
   - continue applying foundation/dock/add/remove occupancy only to buildings;
   - for infantry, resolve the same art entry by `obj.image` then `obj.id`;
   - assign `obj.crawls = entry.crawls`.
4. Add tests proving `Crawls=yes` from art metadata reaches the merged infantry
   `ObjectType` and does not affect unrelated buildings.

Verification:

```powershell
cargo test art_data --lib
cargo test ruleset --lib
```

## Task 3 - Add Sim Runtime State

1. Add `InfantryRuntime` near other component/runtime structs:

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
   pub struct InfantryRuntime {
       pub fear_level: u16,
       pub is_prone: bool,
   }
   ```

2. Add `infantry: Option<InfantryRuntime>` to `GameEntity` with serde default.
3. Initialize it for infantry spawns in both map-spawn and production-spawn
   paths.
4. Keep non-infantry entities at `None`.

Verification:

```powershell
cargo test world_spawn --lib
cargo check
```

## Task 4 - Add `sim::infantry` Fear/Prone Logic

Create `src/sim/infantry.rs` with deterministic helpers and unit tests.

Required behavior:

1. `is_fear_application_blocked(obj, entity)` returns true for `obj.fearless`
   or veteran FEARLESS ability. If the veteran ability query does not exist yet,
   isolate the stub in one helper.
2. `can_decay_fear(obj)` returns false only for type `Fearless=yes`. Do not use
   the veteran FEARLESS ability query for decay.
3. `apply_panic_force` sets fear to 300 unless application is blocked.
4. `apply_fear_from_damage`:
   - no effect for non-infantry, dead targets, zero damage, or blocked fear;
   - first normal hit sets fear to at least 100;
   - first fraidycat hit sets fear to 300;
   - repeated/no-damager branch adds 50, 25, or 12 by post-damage health ratio
     and clamps to 300.
5. `tick_fear_decay_and_prone`:
   - if fear is positive and type is not `Fearless=yes`, decrement by 1 first;
   - after the decrement step, starts Down and sets `is_prone=true` when fear is
     50 or higher;
   - after the decrement step, starts Up and sets `is_prone=false` when fear is
     below 50;
   - does not start Down/Up during deploy/deployed/deploying/undeploying
     sequences or while the unit is dying.
6. `prone_speed_multiplier` returns exact integer factors:
   - standing: 1;
   - prone and `crawls=true`: ceiling `2 * speed / 3`;
   - prone and `crawls=false`: `speed + speed / 2`.
7. `is_prone_for_damage` reads only `InfantryRuntime::is_prone`.

Required tests:

- normal first hit -> 100;
- fraidycat first hit -> 300;
- repeated green-health hit adds 12;
- repeated yellow/red health hit adds 25/50;
- fear clamps at 300;
- type `Fearless=yes` and veteran FEARLESS block first hit, repeated hit, and
  panic force;
- type `Fearless=yes` blocks fear decrement, but veteran FEARLESS does not block
  the decrement of already-existing fear;
- standing fear 50 decrements to 49 and does not start Down;
- standing fear 51 decrements to 50 and starts Down;
- prone fear 50 decrements to 49 and starts Up;
- speed multiplier rounding is exact, including odd/non-divisible values.

Verification:

```powershell
cargo test infantry --lib
```

## Task 4.5 - Hash Infantry Runtime State

1. Add `GameEntity::infantry` state to `Simulation::state_hash` in
   `src/sim/world/world_hash.rs`.
2. Hash presence, `fear_level`, and `is_prone`.
3. Add a state-hash test proving two otherwise identical sims diverge when only
   fear level differs and when only prone state differs.

Verification:

```powershell
cargo test world_hash --lib
```

## Task 5 - Wire Fear Tick Into World Order

1. Export the infantry module from `src/sim/mod.rs`.
2. In `Simulation::advance_tick`, call the fear tick after
   `deploy::tick_deploy_state` and before combat/capture/order-intent precombat
   work.
3. Pass only sim/rules data. Do not introduce render/audio/UI dependencies.
4. The tick should collect animation requests or directly set animation through
   the same low-level sequence-change path used elsewhere, while preserving
   deterministic entity iteration order.

Verification:

```powershell
cargo test infantry --lib
cargo test animation --lib
```

## Task 6 - Replace Combat's Animation Proxy

1. In `src/sim/combat/mod.rs`, replace the target-prone snapshot with
   `infantry::is_prone_for_damage`.
2. In `src/sim/combat/combat_aoe.rs`, replace `animation_is_prone` with the same
   runtime predicate.
3. In combat Phase 4, after landed damage is subtracted and before death
   handling, call `apply_fear_from_damage` for the damaged target.
4. Do not apply fear when Iron Curtain/Force Shield invulnerability nullifies
   the damage.
5. Preserve `last_attacker_id` behavior.

Verification:

```powershell
cargo test combat::combat_tests::test_prone_infantry_takes_scaled_direct_damage --lib
cargo test combat::combat_tests::test_prone_infantry_takes_scaled_aoe_damage --lib
cargo test combat --lib
```

## Task 7 - Apply Prone Movement Speed

1. In `movement_tick.rs`, after the existing current-speed and terrain-speed
   calculations, resolve the entity's object type.
2. If the entity has `InfantryRuntime { is_prone: true, .. }`, apply the
   `prone_speed_multiplier` result.
3. Keep all math integer/fixed-point. Do not use floating point in sim logic.
4. Add tests for:
   - GI-style `Crawls=yes` prone movement slowing to ceiling two-thirds,
     including a non-divisible speed value;
   - non-crawling prone infantry using `speed + speed / 2`;
   - standing infantry unchanged.

Verification:

```powershell
cargo test movement --lib
```

## Task 8 - Drive Animation From Runtime Prone State

1. Update `tick_animations` so infantry runtime state participates in sequence
   choice:
   - prone + moving -> `Crawl`;
   - prone + attacking -> `FireProne` or `SecondaryProne`;
   - prone + idle -> `Prone`;
   - standing after Up -> normal stand/walk/fire choices.
2. Preserve transition sequences:
   - `Down` should transition to `Prone`;
   - `Up` should transition to `Stand`;
   - normal movement/attack logic must not overwrite active Down/Up before the
     transition completes.
3. Keep deploy/deployed/deploying/undeploying sequence priority above fear/prone
   transitions.
4. Update or add animation tests for Down, Up, Crawl, Prone, and FireProne.

Verification:

```powershell
cargo test animation --lib
```

## Task 9 - Fraidycat Flee Behavior

1. Inspect existing garrison/enter-order helpers.
2. If a clean API exists, implement fraidycat high-fear flee by issuing the same
   enter-building intent used by normal garrison movement.
3. If a clean API does not exist, document the exact missing mission/order API
   in `docs/gap-scans/2026-05-16-disparity-scan-gi-unit.md` and keep this
   change focused on core prone/fear/Crawls parity.

Verification when implemented:

```powershell
cargo test garrison --lib
cargo test infantry --lib
```

## Task 10 - GI End-to-End Regression Test

Add one focused end-to-end test using stock-like GI data:

1. GI starts standing and not prone.
2. Damage lands.
3. Fear rises.
4. Fear tick starts Down and sets runtime `is_prone=true`.
5. Movement while prone uses the `Crawls=yes` ceiling two-thirds speed branch.
6. A later shot uses prone damage scaling through runtime state.
7. Fear decay eventually starts Up and clears runtime `is_prone=false`.
8. Deployed GI does not auto-undeploy because of fear.

Verification:

```powershell
cargo test gi --lib
cargo test infantry --lib
cargo test combat --lib
cargo test movement --lib
cargo test animation --lib
cargo test world_hash --lib
```

## Final Verification

Run the targeted set first:

```powershell
cargo test infantry --lib
cargo test object_type --lib
cargo test ruleset --lib
cargo test combat --lib
cargo test movement --lib
cargo test animation --lib
cargo test world_hash --lib
```

Then run the broader library suite if the local branch is otherwise healthy:

```powershell
cargo test --lib
```

## Non-Goals

- Do not hardcode GI behavior by type id.
- Do not replace or redesign the mission system.
- Do not change deploy-fire weapon selection.
- Do not alter render/audio/UI layering.
- Do not remove existing animation sequence helpers unless every caller has a
  runtime-state replacement.
