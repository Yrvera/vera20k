# LogicClass Slice 2 — Phase 1 (combat targeting/fire → logic order) — EXECUTION HANDOFF

**Status:** CODE COMPLETE + VERIFIED, **UNCOMMITTED (held by user choice).** Not committed because
`world/mod.rs` is co-edited with another session's `mapgen_rng` work; commit only once that's separable
or the user OKs bundling.
**Date:** 2026-05-29. Branch `dev`. HEAD at hold time = `a05886e` (another session's bridge work, on
top of Slice 1 `c7ae5bd`).

### Reality vs the plan below (READ FIRST — plan text below is the pre-implementation design)
- **Order authority was NOT `keys_sorted()` at combat/mod.rs:1212.** It is the attacker-snapshot
  `sort_by_key(stable_id)` at ~combat/mod.rs:1507. The fix changed THAT sort key to live-order index
  (+ threaded a `live_order: &[u64]` param), and made `tick_retaliation` collect in live order. The
  ":1212" `keys_sorted` was left as-is (re-sorted downstream — harmless). The `for_each_live_object`
  inline-walk is genuinely **Slice 5**, not this phase (pipeline stays collect→sort→apply, deaths
  deferred).
- **Files changed (mine):** `src/sim/combat/mod.rs` (sig param + sort), `src/sim/combat/combat_targeting.rs`
  (`tick_retaliation` live_order loop), `src/sim/world/mod.rs` (2 call sites: snapshot + arg),
  `src/sim/combat/combat_tests.rs` (9 existing calls get `&[]`; +1 new test).
- **New test:** `combat_resolves_in_live_object_order_not_stable_id` — asserts `fire_events[0].attacker_id`
  / first-resolver follows `live_order`; `&[]` reproduces stable-id order. Fails if the sort reverts.
- **Verified real (package is `vera20k`):** full lib suite `3322 passed; 11 failed; 4 ignored`. New test
  passes; NO combat test fails. The 11 failures are all non-combat (bridge ×4, miner ×1, civilian-sell ×2,
  mcv/harvester ×2, ore-growth ×1, ai ×1) AND the tree is contaminated by other sessions' edits to
  `rng.rs`/`world_hash.rs`/`world_orders.rs`/`snapshot.rs` + `mapgen_rng` hunks in `world/mod.rs` — so
  they are NOT certified mine-vs-theirs. Re-baseline against a clean tree before trusting that count.
- **Docs still stale:** study doc §8 Slice 2 still says Phase 1 not done (my update edit was interrupted);
  fix it at commit time with the real SHA.

## Decision (user said "go with the best, you choose")
- **Approach:** Snapshot reorder of all 3 combat sites → `live_object_order_snapshot()` (insertion/reveal order). Pure reorder. Keep the existing aliveness re-check in the fire pass. **Defer** the `for_each_live_object` same-pass removal/skip upgrade for `tick_combat_fire` as a separate, gated follow-on (it needs Simulation-level restructuring; it's a semantics change beyond reorder).
- **Process:** Implement directly (small, well-scoped). Update study doc §8 Slice 2 + commit message to record the reusable pattern. No separate /write-plan doc.

## Native ground truth (HIGH confidence, adversarially un-refuted this session via Ghidra)
Combat target-acquisition + weapon firing + retaliation ALL run **inline inside the single insertion/reveal-order per-object walk** (rung N over the live-object vector `0x0087F778`, count re-read each iter, `vtable+0x5C`). No separate targeting/fire/retaliation pass.
- Fire is immediate: unit earlier in vector damages a later one THIS tick; later one can retaliate THIS tick (same-pass read-after-write is real).
- Cross-object order = **insertion, NOT stable-id**.
- Heads: UnitClass__AI `0x007360C0` (→ FootClass__AI, UnitClass__TurretAI, UnitClass__Fire_At_Target inline); InfantryClass__AI `0x0051BAB0`; BuildingClass__Update `0x0043FB20`. Retaliation = ReceiveDamage records attacker, victim's own MissionClass dispatch issues counter in its own slot.

## The 3 sites to migrate (the "combat targeting/fire phase")
All currently use `entities.keys_sorted()` (stable-id / BTreeMap order):
1. `src/sim/combat/mod.rs` — `tick_target_acquisition` (defined line ~1095; keys_sorted line ~1116). Phase-1 collects `acquirers` (idle+armed), Phase-2 each picks best target (read-only select). Order ~non-observable for target choice but writes acquirer state → migrate for consistency.
2. `src/sim/combat/mod.rs` — `tick_combat_fire` (defined ~1135; keys_sorted line ~1212). **THE observable site.** Phase-1 collects `firers` (has attack_target && health>0), Phase-2 applies fire with a per-firer aliveness re-check (`still_alive`). Same-pass-sensitive but the aliveness check already approximates the common kill-ahead-of-cursor case.
3. `src/sim/combat/combat_targeting.rs` — `tick_retaliation` (defined 325; keys_sorted line 328). Collect `(retaliator_id, attacker_id)` then issue commands in 2nd loop.

**LEAVE:** `src/sim/movement/turret.rs:95` (`tick_turret_rotation`) — per-entity, order non-observable; belongs to a later turret/movement slice.

## Exact edit shape
The 3 functions take `&mut EntityStore`, NOT `&mut Simulation`, so they can't reach the logic vector themselves. **Thread the order in from the caller.**
- Call sites are in `src/sim/world/mod.rs` (advance_tick pipeline). MUST read them first to get exact signatures. Tick order: turrets+combat → retaliation+passengers; combat is `acquisition → fire`, retaliation runs later.
- Add a param `order: &[u64]` to each of the 3 functions.
- In advance_tick (has `&mut self` Simulation), compute `let combat_order = self.live_object_order_snapshot();` once and pass `&combat_order` to all 3 (recompute for retaliation if a later tick region — verify whether membership changed in between; snapshot at the combat region start is fine for acquisition+fire; retaliation may want a fresh snapshot).
- Inside each fn: replace `let keys: Vec<u64> = entities.keys_sorted();` with iterating the passed `order` (keep the per-id `entities.get(id)` filter + continue-on-None; safe against removed ids).
- **Set-change note (verify, don't skip):** logic order contains only revealed/active objects; keys_sorted contained ALL store entities incl. limbo. For combat this is set-equivalent (limbo units have no attack_target / can't acquire / handled by garrison building which IS in vector). Confirm no firing entity is absent from the logic vector.

## Acceptance / hash gate (DUAL — this is the reusable Slice 2 pattern)
1. **Hash-neutral on map-only fixture:** at load, allocation order == reveal order == section order == id order, so logic order == keys_sorted. Existing replay/`binary_frame_*` hash tests in `world_hash.rs` must stay GREEN unchanged.
2. **Discriminating tie-break test** (new, add to `src/sim/snapshot.rs` test module ~240-498, mirror its helpers — uses `GameEntity` test ctor + `reveal`/`register_live_object` + `set_logic_order_for_test`):
   `two_units_revealed_out_of_id_order_fire_in_reveal_order`: spawn A id=1 into limbo, spawn B id=2 active (reveal first), then reveal A → logic order [B(2),A(1)], stable-id [A(1),B(2)]. Both target a shared enemy T (or each other) with lethal same-tick damage. Assert under migrated path B fires before A; assert post-tick hash differs from the stable-id path.
3. Full sim suite: **baseline = same 11 pre-existing failures** as Slice 1 (movement×4, ai×1, ore_growth×1, production×4 — verify exact set). No NEW failures. Slice 1 baseline was 3319 passed pre-/ +acceptance tests.

## Verification (run FOREGROUND, bounded — do NOT bury cargo in a background workflow, per feedback_cargo_separate_verify_pass)
- `cargo check` (find sim crate name from Cargo.toml workspace members).
- Targeted: the new test + existing combat tests.
- Full sim suite for the hash gate. Then commit to `dev`.

## Divergence reference (id vs insertion)
`allocate_stable_id` (world/mod.rs:661) monotonic → id order = allocation order. Divergence sources (logic≠id): production completion→placement (primary), transport unload (board=conceal, unload=reveal→tail), paradrop, MCV redeploy, garrison eject, limbo→unlimbo. Map load is divergence-free.

## Tooling note for next window
Main-loop file reads were intermittently garbled (impossible all-blank tails, embedded line-number artifacts, duplicated grep lines). Workflow SUBAGENT reads were reliable. Before any Edit, re-read the target region cleanly and sanity-check (no blank tails); if garbled, re-read or delegate the edit to a single Agent. grep line numbers (MD5-corroborated) are trustworthy: keys_sorted at combat/mod.rs:1116, :1212; combat_targeting.rs:328.

## Source artifacts
- Study doc: `docs/research/LOGICCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` §8 (Slice 2 scope), §9 (death-to-limbo resolved).
- Slice 1 plan/pattern: `docs/plans/2026-05-29-logicclass-slice1-lifecycle-chokepoint-plan.md`.
- Workflow full result (native+adversarial+rust map+divergence): `<local>/AppData/Local/Temp/claude/<claude-project>/10cf3019-fd47-40c9-9cd9-751f05c90b2a/tasks/wqao2264m.output`.
- APIs: `src/sim/world/mod.rs:733` `live_object_order_snapshot()`, `:751` `for_each_live_object()`, `:691/697/703` reveal/conceal/unlimbo.
