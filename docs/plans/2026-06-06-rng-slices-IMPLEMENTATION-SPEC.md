# RNG-parity + death-window slices — IMPLEMENTATION SPEC (binary-verified)

**Date:** 2026-06-06. **Base:** `dev` @ `a592bfb6` (death-window substrate confirmed present → slices 4–5 NOT blocked).
**Source TODO:** `docs/plans/2026-06-05-rng-parity-and-death-window-slices-TODO.md`.
**Verify run:** 5 parallel read-only agents, Ghidra-grounded (workflow `wf_de7a094b-71e`). Full raw output:
`%LOCALAPPDATA%\Temp\claude\...\tasks\wwsgmj66l.output`.

This doc is the corrected, dev-accurate spec. Where it disagrees with the TODO, **trust this doc** — every claim
here was re-derived from the binary this session with the cited Ghidra call.

---

## CORRECTIONS TO THE TODO (read first — the TODO is wrong on these)

1. **Slice 1 needs TWO helpers, not one.** Lifetimes/insert use `abs(raw % n)`; jitter/spawn-offsets use **plain signed** `raw % n` (can be negative). Using abs everywhere silently kills negative offsets.
2. **Slice 1 — fire jitter doc is WRONG.** Binary `0x0062CB10` is `raw % 10 - 5` (plain signed) → range **-14..=4**, NOT the doc's abs/`-5..=4`. (`/audit` side-quest should patch `PARTICLE_RNG_CLASSIFICATION §3.4`.)
3. **Slice 1 — gas spawn offsets (S10/S11) draw RNG that gamemd NEVER draws.** `AI_Gas 0x0062E6D0` has zero `Random__Next`. Converting them faithfully translates a *phantom* draw → cannot reach parity. **DECISION NEEDED.**
4. **Slice 2 path is wrong:** wall code is `src/sim/overlay_grid.rs` (not `src/sim/map/overlay_grid.rs`).
5. **Slice 2 has NO dependency on Slice 1** and uses NO raw-modulo helper — both fixes reuse existing `next_range_u32_inclusive` (mask-reject = gamemd `RandomRanged 0x0065c7e0`).
6. **Slice 3 path is wrong:** `src/skirmish_launch.rs` (top-level `src/`, not under `src/sim/`).
7. **Slice 3 — no `color_random` field exists** and stock UI never randomizes color → the color draw is dormant/dead-code unless a field is threaded. **DECISION NEEDED.**
8. **Slice 4 — the reorder ALONE is a no-op.** `tick_ai` never reads `is_defeated`; moving `check_defeat` earlier changes nothing without ALSO adding a defeat-gate inside `tick_ai`. **This is required, not optional.**
9. **Slice 5 drain lines are stale.** dev: `:1723` end-of-tick (KEEP), `:1774` command-boundary (REMOVE), `:2288` end-of-P5 (REMOVE). Order is inverted vs the TODO.
10. **Slice 5 — AI is ALREADY fully dying-gated** (don't re-gate) and **particles need NO gate** (separate store). Only vision/power/production need gates added + one retaliation literal fix.

---

## DECISIONS NEEDED (before the affected slice)

- **D-S1 (gas phantom draw):** S10/S11 in `gas.rs:86-87`. Options: (a) gate/remove the gas periodic-spawn path (separate scope, OQ-PARTICLE-RNG-007); (b) convert-and-comment-YELLOW for now + track. Recommend surfacing, not silently converting.
- **D-S3 (color_random):** (a) add `color_random` field + UI plumbing now; (b) **[recommended]** implement the gamemd-faithful color-draw shape gated on a field that's always `false` now — dormant exactly like stock offline skirmish, UI plumbing as follow-up.
- **D-S4 (defeat gate):** confirm we add the `tick_ai` per-player `if is_defeated { continue; }` gate (required for gap 18 to actually close).

---

## SLICE 1 — Particle RNG raw-modulo  ·  rank 1  ·  hash-NOT-neutral

**Helpers (add to `src/sim/rng.rs` after `next_range_u32`):**
```rust
/// abs((next_u32() as i32) % n). One raw draw. gamemd particle-lifetime / fire-insert.
pub fn next_raw_abs_modulo(&mut self, n: u32) -> u32 {
    if n == 0 { return 0; }
    ((self.next_u32() as i32) % n as i32).unsigned_abs()
}
/// (next_u32() as i32) % n. One raw draw, can be negative. gamemd jitter / spawn-offset (CDQ;IDIV, no abs).
pub fn next_raw_modulo_signed(&mut self, n: u32) -> i32 {
    if n == 0 { return 0; }
    (self.next_u32() as i32) % n as i32
}
```

**Sites (dev-accurate; abs vs signed is load-bearing):**
| site | dev file:line | change | primitive |
|---|---|---|---|
| S1 | `particles/spawn.rs:96` | `next_range_u32(10)` → `next_raw_abs_modulo(10)` | abs (Railgun lifetime) |
| S2 | `particles/spawn.rs:99` | `next_range_u32(base)` → `next_raw_abs_modulo(base)` | abs (lifetime) |
| S3 | `particles/spawn.rs:229` | `next_range_u32(actual_range)` → `next_raw_abs_modulo(actual_range)` | abs (fire-insert offset) |
| S4 | `particles/fire.rs:65` | `next_range_u32(base)` → `next_raw_abs_modulo(base)` | abs (lifetime) |
| S5 | `particles/fire.rs:116` | `next_range_u32(10) as i32 - 5` → `next_raw_modulo_signed(10) - 5` | **signed** (jitter, -14..=4) |
| S6 | `particles/smoke.rs:88` | `next_range_u32(r+1) as i32` → `next_raw_modulo_signed(r+1)` | **signed** (off_x) |
| S7 | `particles/smoke.rs:89` | `next_range_u32(r+1) as i32` → `next_raw_modulo_signed(r+1)` | **signed** (off_y) |
| S8 | `particles/smoke.rs:178` | `next_range_u32(base)` → `next_raw_abs_modulo(base)` | abs (lifetime) |
| S9 | `particles/smoke.rs:213` | `next_range_u32(r) as i32` → `next_raw_modulo_signed(r)` | **signed** (symmetric offset; keep `if raw<1 {raw-r} else {raw+r}`) |
| S10 | `particles/gas.rs:86` | **D-S1 decision** | phantom (gamemd draws nothing) |
| S11 | `particles/gas.rs:87` | **D-S1 decision** | phantom |
| S12 | `particles/gas.rs:198` | `next_range_u32(base)` → `next_raw_abs_modulo(base)` | abs (lifetime) |

Keep `base = (max_ec as u32).max(1)`. Order draw1→X, draw2→Y for smoke spawn (matches binary `0x0062F0AC`).

**Binary evidence:** lifetime abs `decompile_function 0x0062B5E0`; fire jitter signed `0x0062CB10`; fire-insert abs `0x0062E4C0`; smoke offsets `0x0062ED40` + `get_assembly_context 0x0062F0AC`; gas zero-RNG `0x0062E6D0`; `Random__Next 0x0065C780`.

**Tests (`rng.rs mod tests` + each particle `mod tests`):** seed=1 raw stream `0x78B76ED5, 0x275D74AE, 0xDA63B931`. **CORRECTED GOLDENS (the Verify agent's decimal conversions were wrong — `0x78B76ED5`=2_025_287_381 not 2_025_721_557; trust the hex):** as i32 the stream is `+2_025_287_381, +660_436_142, -630_998_735`. `next_raw_abs_modulo(80)` = `[21, 62, 15]`; `next_raw_modulo_signed(10)` = `[1, 2, -5]`; fire jitter (`signed-5`) = `[-4, -3, -10]` (the -10 proves the -14..=4 range). Assert exactly-one-draw cursor advance per call (vs mask-reject's variable count). Regression guard: fail if any `particles/*` reintroduces `next_range_u32(`. **DONE — committed `6f23d10f`, full lib suite green (3734).** ⚠️ For Slices 2–3, compute goldens from the HEX/mask directly, not from any decimal in this doc.

**Out of scope (flag, don't implement):** R2 fire-insert index off-by-one (`count-off-1` vs Rust `count-1-off`); smoke child translucency `%6`; gas/smoke drift gates.

---

## SLICE 2 — Smudge 50/50 + wall-damage  ·  rank 2  ·  hash-NOT-neutral  ·  NO dep on Slice 1

**(a) `src/sim/combat/smudge_dispatch.rs:208-213`** — replace high-bit test with the ranged draw + threshold:
```rust
const SMUDGE_5050_RANGED_HIGH: u32 = 0x7FFF_FFFE;
const SMUDGE_SCORCH_ACCEPT_LT: u32 = 0x4000_0000;
fn rng_below_half_normalized(rng: &mut SimRng) -> bool {
    rng.next_range_u32_inclusive(0, SMUDGE_5050_RANGED_HIGH) < SMUDGE_SCORCH_ACCEPT_LT
}
```
Delete the stale `// One RNG advance, no modulo bias` comment (it documents the bug). Fixes draw count (`0x7FFFFFFF` rejection = extra advance) AND acceptance band.

**(b) `src/sim/overlay_grid.rs:364-369`** — inclusive range + `>=` boundary:
```rust
let roll = rng.next_range_u32_inclusive(0, flags.strength as u32) as u16; // [0, Strength]
if roll >= damage { return; }                                            // no-op when roll >= damage
```

**Binary evidence:** smudge `AnimClass::Start 0x00424F00` (consts `0x007e3570`≈2^-31, `0x007e1738`=0.5 → accept `<0x40000000`); `RandomRanged 0x0065c7e0` (mask-reject = `next_range_u32_inclusive`); wall `CellClass::DestroyOverlay 0x00480CB0` (`RandomRanged(0,Strength)` inclusive, `SETL` → no-op when `roll>=damage`).

**Tests:** smudge — seed=1 first masked draw `0x78B76ED5 ≤ 0x7FFFFFFE` → accept, 1 draw, returns false (crater); a value in `[0x40000000,0x7FFFFFFE]` returns false (discriminates new `<0x40000000` from old `<0x80000000`). Wall — `strength=400` seed=1 roll=**213**; `damage=213` → no-op, `damage=214` → advance; top value `strength` reachable. `state_hash` differs-then-stable.

---

## SLICE 3 — random_assignment SP color + order  ·  rank 3  ·  hash-NOT-neutral (pre-tick-0)  ·  SP only

**Site:** `src/skirmish_launch.rs:291-306` (`resolve_random_assignments`). Routing already correct Scen->Random (`random_assignment_rng` → `scenario_rng`).

**Prereq (D-S3):** add `pub color_random: bool` to `SkirmishLocalSlot` (`:251`) + `SkirmishAiSlot` (`:262`), threaded `false` from UI for now.

**New body (gamemd order — ALL humans country→color, THEN ALL AI country→color):**
```
// Phase A: humans (here: local) — COUNTRY then COLOR
if local.country_random { local.country = from_country_index(rng.next_range_u32_inclusive(0,9)); }
if local.color_random  { loop { let c=rng.next_range_u32_inclusive(0,7) as u8; if !color_in_use(c,&assigned){local.color_index=c; assigned.push(c); break;} } }
// Phase B: AI slots in order — COUNTRY then COLOR each
for opp in opponents { /* same country then color, push to assigned */ }
```
`assigned` seeded with all concrete (non-random) colors first. `color_in_use(candidate, &assigned)` = pure helper modeling `FUN_0069b600` + AI inline scan.

**Binary evidence:** `SessionClass__ProcessRandomAssignments 0x0069B8C0` (loop A humans `0x0069b8e0`, loop B AI `0x0069b9d6`; country `RandomRanged(0,9)`, color `RandomRanged(0,7)` with collision-retry `LAB_0069ba13`); collision helper `0x0069b600`; SP gate `param_1+4==0` (MP uses vtable+0x6c/+0x70 = out of scope). Observer slots (`-3`) draw nothing.

**Tests (`skirmish_launch.rs mod tests`):** assert draw ORDER country→color, humans→AI via a replayed `expected_rng` (`2*(N+M)` `RandomRanged` *calls* + retries — NOT a fixed raw-advance integer); color collision forces a redraw (extra `(0,7)` draw); seed=1 golden (country,color) table; tick-0 `state_hash` differs-then-stable. Regression: a color draw issues for every random-color slot.

---

## SLICE 4 — Presence FSM decouple + defeat-before-AI  ·  rank 4  ·  unblocks Slice 5

**Part (a) — ABANDONED (the spec was WRONG).** The proposed `derived_presence` change (`if dying { Dying }` first, or even membership-first `else if dying { Dying }`) is **not implementable**: `dying && !in_logic_vector` is shared by TWO states — a uninit'd corpse (presence field `Dying`, in `pending_delete`) AND a concealed-dead object mid-teardown (presence field `Limbo`, NOT in `pending_delete`, e.g. a C4-killed building). `derived_presence` sees only `GameEntity` fields and cannot distinguish them; deriving `Dying` broke **9 C4 teardown tests** (`entity N presence Limbo != derived Dying`). **`derived_presence` stays InCell/Limbo-only** (a clarifying doc note was added). The corpse presence-invariant is instead handled in **Slice 5** at the substrate-aware assert (`debug_assert_presence_consistent`), which DOES have `pending_delete` to tell the two apart.

**Part (b) — reorder + gate (defeat-before-AI):**
- Move the Phase-8.5 `check_defeat` block (dev `world/mod.rs:1683-1689`) to BEFORE the Phase-8 AI block (before dev `:1650`), inside `run_late_region`. Keep `if self.tick > 0`. Update the "DEPENDS ON … AI (commands applied)" comment.
- **REQUIRED gate** — in `src/sim/ai.rs` `tick_ai`, top of `for ai in ai_players.iter_mut()` (dev `:74`):
```rust
let owner_str = sim.interner.resolve(ai.owner);
if crate::sim::house_state::house_state_for_owner(&sim.houses, owner_str)
    .is_some_and(|h| h.is_defeated) { continue; }
```
(Verify exact accessor name `house_state_for_owner` against `house_state.rs`.)

**Binary evidence:** `ObjectClass::UnInit 0x005F65F0`, `IsDead 0x005F6690` (= `IsAlive==0`, always-true post-uninit), `ProcessPendingDelete 0x00725C70`, `Main_Tick 0x0055D360`; `HouseClass::Update 0x004f8440` (defeat `MPlayer_Defeated` precedes AI manage block).

**Hash:** Part (b) hash-neutral ONLY when no house's defeat status flips on a boundary tick; on a 0/0 house whose AI would spawn this tick, `is_defeated` flips → hash changes (pin new value). Divergence point: a 0-count house whose AI spawns a building the same tick its last structure/unit died.

**Tests:** `ai.rs tests` — `tick_ai_skips_defeated_house` (live house deploys MCV; defeated house issues nothing).

**Risk:** without the `tick_ai` gate the reorder is a no-op (most important finding).

**STATUS: DONE — committed `745603e2` (Part b only), verified green in an isolated worktree (3732 passed). Part (a) abandoned per above.**

---

## SLICE 5 — Dying-gates on raw-store consumers, then collapse to one drain  ·  rank 5  ·  hash-NOT-neutral  ·  HARD DEP on Slice 4

**Optional canonical gate helper** (`src/sim/game_entity.rs`, near `is_alive` `:794`):
```rust
/// Native IsAlive(+0x90) equivalent: false once uninit'd this tick (Dying corpse).
/// Distinct from is_alive() (health>0): a sold/captured structure keeps health but is dying.
pub fn is_active(&self) -> bool { !self.dying }
```
Gate field is **`entity.dying`** (NOT `health.current` — misses sold/captured structures).

**Consumers to gate (add `if entity.dying { continue; }` / `!e.dying &&`):**
- **Vision (none today):** `vision/mod.rs:487` (visibility reveal), `:538` (bounds), `world/mod.rs:1429` (SpySat/GapGen scan).
- **Power (none today):** `power_system.rs:78` (recalc), `:149` (owner-collect), `:236` (`has_active_radar`).
- **Production (none today):** `production_tech.rs:168,193,235,280,532,552` (override/BuildLimit/prereq/factory/multi-factory/producer-candidates).
- **Retaliation (literal fix):** `combat/combat_targeting.rs:356-358` — attacker-alive `is_some_and(|a| a.health.current > 0)` → `is_some_and(|a| !a.dying && a.health.current > 0)`.
- **AI:** NO change (already gated at `ai.rs:160,194,438,494,521,626`). **Particles:** NO change (separate store).

**Presence-invariant fix (MOVED here from Slice 4 Part a — REQUIRED before collapse).** After the early drains are removed, uninit'd corpses (presence field `Dying`, in `pending_delete`) survive to `debug_assert_presence_consistent`, where the old `presence == derived_presence()` check fails (derived is `Limbo` for `!in_logic_vector`). Fix the assert (NOT `derived_presence`) to be `pending_delete`-aware:
```rust
// in debug_assert_presence_consistent: a corpse enqueued for the end-of-tick
// drain is legitimately Dying (uninit set the field + queued it); everything
// else must match derived_presence().
let pending: std::collections::HashSet<u64> =
    self.substrate.pending_delete.iter().copied().collect();
for e in self.substrate.entities.values() {
    let expected = if pending.contains(&e.stable_id) { Presence::Dying }
                   else { e.derived_presence() };
    debug_assert_eq!(e.presence, expected, ...);
}
```
⚠️ FIRST re-check the actual order of `flush_pending_delete` vs `debug_assert_presence_consistent` in `run_late_region` on HEAD: the doc-comment at `mod.rs:~885` claims the flush runs in Phase 9 BEFORE the assert. If the *surviving* end-of-tick drain still precedes the assert, corpses are already gone at the assert and NO presence fix is needed — confirm before writing code.

**Collapse (do LAST, after gates land + a checkpoint commit + green suite):**
- Remove `world/mod.rs:1774` (+ comment `:1766-1773`) and `:2288` (+ comment `:2279-2287`).
- Keep `:1723` (end-of-tick); update its comment to "single in-tick drain; mid-tick consumers dying-gated."
- Leave `app_sim_tick.rs:316` (animation-end lifecycle, out of scope).
- ⚠️ **Line numbers above are dev-era and pre-P5d**; a concurrent P5d production refactor is actively editing `factory.rs`/`world_hash.rs`/`production_tech.rs` — RE-GREP every Slice-5 site on a stabilized tree, and expect production-tech gate sites to have moved.

**Binary evidence:** single drain `ProcessPendingDelete 0x00725C70` at `Main_Tick 0x0055D360` tail; `IsDead 0x005F6690` always-true post-uninit; gate-at-use (no proactive nulling of `last_attacker_id`/`capture_target`/`bunker_occupant`) per `SLICE6_DEFERRED_DELETE_DYING_WINDOW` §8; mutual same-tick death §3.6.

**Tests:** per-consumer exclusion (vision fog drops, power output/drain drops, prereq/BuildLimit re-evaluates) for an uninit'd-not-flushed corpse. **Kill-credit + last-attacker across the Dying window** (A kills B: within tick B resolvable+`dying`, `last_attacker_id==A`, freed at end-of-tick drain; mutual death both resolvable then freed). Slice-4 landmine: `unregister_live_object` `debug_assert_eq!(presence==InCell)` at `mod.rs:799` must not fire under Slice 4's change — pin as explicit debug-assert test. Regression: exactly ONE `flush_pending_delete()` remains in `advance_tick`/`run_late_region`. `state_hash` differs-then-stable (no hand-derived cursor count — pin the new deterministic hash).

**Recommended commit split:** (1) all dying-gates + retaliation fix → checkpoint, run suite; (2) remove the two drains → run suite. The drains were masking real consumer bugs — surfacing them is expected.

---

## STATUS — ALL SLICES DONE (2026-06-06)

| Slice | Commit | Result |
|---|---|---|
| 1 particle raw-modulo | `6f23d10f` | green |
| 2 smudge/wall RNG | `b134dcd8` | green |
| 3 random-color (dormant) | `f6b85496` | green |
| 4 defeat-before-AI | `745603e2` | green (Part a abandoned — see above) |
| 5 Phase 1 dying-gates | `344d2539` | green, hash-neutral |
| 5 Phase 2 drain collapse | `f61ad4c3` | green (3744 passed), hash-changing |

**Slice 5 reality vs spec:** the 5-agent re-audit found ~25 gate sites (the spec listed ~13) — all of movement, combat AoE, miner, aircraft, world_spawn were missing from the spec. Phase 2 surfaced ONE more consumer the audit itself missed (`production_sell::tick_repairs`), caught by an existing test backstop and gated. The presence-invariant fix the spec demanded was NOT needed: the surviving end-of-tick drain (run_late_region, called before the asserts) frees corpses before any assert. AI was already fully dying-gated; particles use a separate store.

## DEPENDENCY ORDER & HASH

`1 → 2` (independent in reality, but keep order) `→ 3` (independent) `→ 4 → 5` (4 unblocks 5). Slices 1,2,3,5 change the lockstep hash by design; Slice 4(a) is hash-neutral, 4(b) conditional. Capture a committed baseline before the hash-changing slices so "differs-then-stable" tests are trustworthy. Run cargo as a **separate foreground pass** (`cargo check -p vera20k`, then `cargo test -p vera20k <module>`); read the literal `test result:` line.
