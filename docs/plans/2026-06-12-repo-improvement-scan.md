# Repo Improvement Scan — 2026-06-12

Six parallel read-only auditors (layering, determinism, structure, test gaps, code health,
infra). Findings below are agent-reported with file:line evidence; spot-verify the
load-bearing ones before acting. Ordered by priority.

## P1 — Live desync/replay-divergence vector: app layer draws from the lockstep RNG per render frame

- `src/app_building_anim.rs:183` calls `sim.anim_rng()` inside `tick_damage_fire_overlays`,
  which runs **per render frame** (`src/app_sim_tick.rs:189-192`), not per sim tick.
- `anim_rng()` returns `&mut self.scenario_rng` (`src/sim/world/mod.rs:619-621`) — the same
  stream consumed in-tick by scatter/subcell/smudge/ore/bridge, and its state is part of the
  world hash (`src/sim/world/world_hash.rs:73`).
- Because 0..8 sim ticks commit per frame, **frame pacing decides how the shared RNG stream
  interleaves** → two clients (or record vs replay) diverge. Triggers the first time any
  building drops below the damage-fire health threshold — every match with combat.
- Also: the spawn gate uses an f32 health ratio in app code (`app_building_anim.rs:113`).
- **Fix:** move the damage-fire spawn decision + RNG draws into the building-anims phase of
  `World::advance_tick`, emit an event for the app; keep only visual frame-advance app-side.
- Related (medium): render-paced anim state lives on sim entities and is mutated app-side
  with wall-clock dt (`app_building_anim.rs:35-72`, 28 `entities_mut()` calls across 8
  app files). Relocate to an app/render-side map keyed by stable_id.

## P2 — Untested high-criticality orchestrators (also invisible to the hash harness)

- **Engineer capture**: `tick_capture_orders` (`src/sim/world/world_orders.rs:184-270`) has
  zero behavioral tests; only the CABHUT edge case is tested. Mainline ownership-flip never asserted.
- **Aircraft missions**: `src/sim/aircraft/mod.rs` (840 lines) — `tick_aircraft_missions`
  appears in zero test files; no test scenario anywhere contains a fixed-wing aircraft.
- **Superweapons**: charge/suspend/resume/reset (`superweapon/mod.rs:70-178`) and force
  shield (`force_shield.rs`) untested; suspend-on-power-loss is player-visible every match.
- **Free harvester spawn** (`production_refinery.rs:20,90`) untested — and refinery
  exit/facing math is the documented recurring direction-bug class.
- **Guard auto-targeting** (`combat_targeting.rs:92` `acquire_best_target_for_entity`) and
  **movement reservation arbiter** (`movement_reservation.rs:13` — desync-class two-movers-one-cell
  logic) have no direct tests.
- The 600-tick parity harness scenario family contains **no aircraft, no superweapon launch,
  no capture** — the untested systems are exactly the ones outside hash-stability coverage too.
- **Fix:** one capture test file, one end-to-end Harrier sortie test, superweapon timer tests,
  a second harness scenario including all three, plus a multi-thousand-tick seeded command soak.

## P3 — No `[profile.release]`: shipped binary gets overflow-checks=false

- Cargo.toml has only `[profile.dev]`. Release defaults: overflow wraps silently; dev/test
  builds panic. Different arithmetic failure semantics between every tested build and the
  shipped one; a dev-vs-release lockstep game diverges at the first overflow.
- **Fix:** explicit `[profile.release]` with commented `overflow-checks = true` decision,
  `lto = "thin"`, deliberate codegen-units. Also: `[profile.dev.package."*"]` opt-level=1 is
  a no-op (same as dev) — bump deps to opt-level=3 or delete the section.

## P4 — Runtime LUTs built from f64 transcendentals feed hashed state

- `homing_movement.rs:130-153` (65536-entry cos/sin), `:258-270` (atan LUT), and
  `smudge_dispatch.rs:18-31` (256-entry unit vectors) are built at runtime from f64
  sin/cos/atan; results reach hashed state (homing positions, smudge grid).
- In-code "deterministic across platforms" comments are unproven — Rust transcendentals are
  not correctly-rounded; `.round()` can flip near ties. Same-binary Windows peers are safe
  today; headless-server/cross-toolchain replay is not.
- **Fix:** freeze all three as compile-time const tables (like the SIDEWINDER table already
  at `homing_movement.rs:91-93`).
- Verified safe by the auditor: f64 damage/radiation kernel (basic IEEE ops only, no FMA),
  all f32 render fields hash-excluded, zero wall-clock, zero external RNG, no iterated
  HashMaps, all 26 sort sites total-ordered.

## P5 — Structure debt (maintenance drag, no correctness risk)

- 146 of 476 files exceed the 600-line aim. Worst non-exempt: `rules/ruleset.rs` 4,000
  (six rule-group structs + registry; GeneralRules alone ~1,250 lines — split per the
  existing terrain_rules.rs pattern), `app.rs` 3,394 (163-field AppState, 8+ responsibilities;
  carve out the three shell-input blocks ≈ 1,100 lines), `sim/world/mod.rs` 2,779 with a
  ~772-line `advance_tick` (split into named per-phase fns), `sim/combat/mod.rs` ~723-line
  `tick_combat_with_fog`.
- 39 `app_*.rs` files (28.6k lines) flat at src/ root → fold into `src/app/` with 4-5
  clusters (shell/, sidebar/, overlays/, frame/); mechanical, zero sim risk.
- Reverse layering leaks: map→render (`map/terrain.rs:18` imports `render::batch::SpriteInstance`),
  rules/map→sim (~6 files: LandType, Axis/BridgeheadAnchorClass, SequenceSet, MissionControl,
  InternedId). Extract shared primitives to a lower-level module.

## P6 — Conventions have zero mechanical enforcement

- The #1 invariant (sim imports nothing from render/ui/sidebar/audio/net — currently CLEAN,
  verified by three greps) is convention-only. Add a #[test] that scans src/sim for
  forbidden imports.
- No clippy.toml (disallowed-types for HashMap/HashSet in sim), no deny.toml (approved-crates
  ban list). HashMap exists in 3 non-test sim files (combat/mod.rs, pathfinding/core.rs,
  production/factory.rs) — auditor found none iterated into game state, but nothing prevents it.
- CI: no clippy step, no `--locked`, no concurrency-cancel, no timeout-minutes. Toolchain
  pinned to channel "stable" only — pin a specific version (repo already has 1.93+ breakage
  evidence in Cargo.toml:77-79).
- `.gitignore:4` bare `config.toml` will silently swallow a future `.cargo/config.toml` —
  anchor to `/config.toml`.

## P7 — Parity items living in comments instead of the backlog

- **Idle scatter disabled** in `sim/world/mod.rs:2547-2567` — commented-out
  `scatter::tick_idle_scatter` call ("units were moving on their own... needs further RE").
  Player-visible in every crowded battle; tracked only as a comment block. Promote to a
  tracked parity gap, then delete the dead block.
- `world_commands.rs:1013` TODO(parity): selling-in-progress buildings not rejected —
  small, player-reachable command-validation fix.
- Pathfinding: 10 of 24 TODOs + 19 allow(dead_code) = a written-but-unwired hierarchical-zone
  layer (zone_hierarchy.rs, zone_search.rs, build_zone_map). Wire it or delete it — dead code
  drifting from live predicates will mislead the next pathfinding parity pass.
- 698 comment lines reference gamemd/FUN_/addresses despite the no-engine-refs rule —
  either retire the rule or strip the 107 address/FUN_ lines first (they rot fastest).

## Clean bills of health

- sim layering invariant intact (zero forbidden imports, incl. wgpu/egui/winit/rodio).
- Panic hygiene: only 34 unwrap/expect in non-test sim code, every hotspot verified guarded.
- Dependency hygiene: all nine approved pins match; extras are mainstream; repo lean
  (536 tracked files; one orphan 1.5MB PNG in docs/images, tracked-but-ignored, referenced by nothing).
- Determinism fundamentals: no wall-clock, no external RNG, sorts total-ordered, f32 strictly
  render-side and hash-excluded.
