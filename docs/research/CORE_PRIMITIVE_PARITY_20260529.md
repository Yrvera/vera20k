# Core-Primitive Parity Pass - 2026-05-29

## Summary

Scanned seven core simulation primitives — `lepton`, `fixed`, `facing`, `rng`,
`tick`, `entitystore`, `cellgrid` — against gamemd.exe (live Ghidra) and the
docs/research corpus. The pass produced **24 findings**: **2 auto-fixed**
(PROVEN_TRIVIAL, applied and verified), **1 verifier-refused** downgrade
(FacingClass per-step magnitude — real and algebraically-exact fix, but blocked
by the determinism gate), **17 NEEDS_REVIEW drifts**, and **13 NEEDS_RESEARCH /
UNCHECKED items** (some titles overlap NEEDS_REVIEW where the same primitive is
both a confirmed drift and an unverified sibling concern). `cargo check` passed
cleanly (warnings only); all targeted module tests passed (sim::rng 8/8,
sim::entity_store 18/18, determinism_replay 3/3). No regressions introduced.

## Auto-fixed (PROVEN_TRIVIAL, applied + verified)

| System | File | Title | What changed | Evidence |
|---|---|---|---|---|
| rng | `src/sim/rng.rs` | RandomRanged rejection mask wrong for power-of-two spans — biases output AND changes draw count | In `next_range_u32_inclusive` (~L146) replaced `span.next_power_of_two().wrapping_sub(1)` with `u32::MAX >> span.leading_zeros()`. Old mask returned `N-1` for a power-of-two span `N=2^k` instead of `2N-1`, so the loop always accepted on the first draw, never reached the inclusive top (e.g. RandomRanged(0,4) could only yield 0..3, never 4), and consumed the wrong number of raw draws. New mask is one bit wider than the span's highest set bit (`2^(msb+1)-1`), matching the binary. Span guaranteed `1..=0x7FFFFFFE` here, so `leading_zeros` is `1..=31` and the shift never reaches 32. Added a WHY comment (no binary references) + 2 pinning tests. | Binary `Random__RandomRanged` @ 0x0065C7E0: `mask = ~(-1 << (iVar5+1))` where `iVar5` = index of highest set bit. span=4 (0b100) → iVar5=2 → mask `~(-1<<3)` = 0x7. Rust old: `4.next_power_of_two()=4` → mask 3. MISMATCH (7 vs 3). Live caller `InfantryClass__Scatter` @ 0x0051D0D0 calls `RandomRanged(0,4)` (power-of-two span) every scatter. Algebraic proof of correct mask: `2^(msb+1)-1 = u32::MAX >> span.leading_zeros()`. Tests: `random_ranged_power_of_two_span_matches_gamemd_draw_stream`, `random_ranged_power_of_two_span_can_return_inclusive_top`. |
| entitystore | `src/sim/entity_store.rs` | EntityStore doc falsely claims `insert()`/`remove()` keep `by_owner` index in sync automatically | Doc-only edit at two comment sites (the original fix proposal missed the second). Struct doc-comment now states the index is rebuilt by `rebuild_owner_index()` (once per tick) and that `insert()`/`remove()` do NOT update it, so any caller reading `ids_for_owner()` after a mid-tick mutation must rebuild first. The `by_owner` field comment now reads "Rebuilt by `rebuild_owner_index()`, not by insert/remove." Per `feedback_no_engine_refs_in_comments` the gamemd.exe reference was dropped ("Mirrors per-owner object membership"). Added contract test. | `entity_store.rs:28-30` (false claim) vs `insert()` 50-54 / `remove()` 57-59 (no `by_owner` mutation). `rebuild_owner_index` 142-149. Grep `ids_for_owner`: only test usages + `world/mod.rs:1418` rebuild; no production read site. Test: `insert_does_not_auto_sync_owner_index`. |

## Build/test gate result

`cargo check` passed cleanly — warnings only (dead-code / unused / non-snake-case),
no errors. Targeted tests for the touched modules all pass:

- `sim::rng` — 8/8 (incl. the two new power-of-two mask pinning tests)
- `sim::entity_store` — 18/18 (incl. radio-contact and owner-index cases)
- `determinism_replay` integration — 3/3 (`determinism_repeatability_same_inputs`,
  `fixed_step_invariance_across_frame_profiles`,
  `replay_playback_matches_live_hash_timeline`)

The RNG mask fix is covered by the two new tests (both green) and the determinism
timeline-hash test confirms no desync was introduced. **No failures to attribute** —
neither ours nor pre-existing/other-session.

## Needs your review (NEEDS_REVIEW + verifier-refused)

Ranked by player-visibility. Each carries a trigger-frequency note per project rule.

### 1. FacingClass::current() per-step magnitude wrong (verifier-refused → NEEDS_REVIEW)

- **System:** facing — **File:** `src/sim/movement/facing_class.rs` (L105-106)
- **Trigger frequency:** Fires on essentially every turret/body aim where `abs(diff)` is not an exact multiple of ROT — i.e. most rotations in normal play (ROT is `byte<<8` = multiple of 256, aim targets are arbitrary). High-frequency.
- **Drift:** Rust computes `signed_step = diff.signum() * rot_per_frame` (= `sign(diff)*ROT`) per step. gamemd's `RateTimer__Current` (0x004C93D0) uses `diff / step_size` per step, where `step_size = abs(diff)/ROT` (integer div), then `animated = current - (diff/step_size)*remaining`. Worked fixture: diff=12900, ROT=1280 → step_size=10; binary per-step 1290, Rust 1280; at remaining=10 binary gives 0 (=prev), Rust gives 100.
- **gamemd vs Rust:** binary `(diff/step_size)*remaining`; Rust `(sign(diff)*ROT)*remaining`.
- **Why not auto-fixed:** The finding is REAL and the proposed fix (`per_step = (diff as i32)/(step_size as i32)`, then `(current - per_step*remaining).rem_euclid(65536)`) is algebraically bit-identical to the binary for all diff/ROT. But `FacingClass::current()` output is hashed into world state (`world_hash.rs` L383-384, 75, 577) — it is determinism-critical. The determinism gate permits auto-confirm only when the proof is airtight AND the accompanying test is a determinism/world-hash test. The supplied test is a plain value-equality unit test that does not exercise the hash path, so the gate forces a downgrade. Apply under human review with a world-hash regression test alongside the unit test. (Module doc comment L4 also goes stale — cosmetic.)

### 2. Body rotation does not use FacingClass — uses ms-based integration

- **System:** facing — **File:** `src/sim/movement/facing_class.rs` (load-bearing consumer in `movement_step.rs`)
- **Trigger frequency:** Every moving/turning ground unit, every tick it changes heading. Very high-frequency.
- **Drift:** Body facing (`entity.facing`) is rotated by `rot_to_facing_delta(rot, tick_ms)` — a per-tick millisecond-integrated angular delta with a max_delta clamp. gamemd uses BodyFacing FacingClass (TechnoClass+0x370, default ROT=3) interpolated by the same binary-frame `RateTimer` timer keyed to `g_CurrentFrameCounter`, NOT ms-integrated.
- **gamemd vs Rust:** binary-frame FacingClass timer vs ms-integrated delta; ROT=3 render-smoother cap not modeled. Different body-rotation curves and timing.
- **Why not auto-fixed:** The corrective edit site is `movement_step.rs`, outside the three files in this scan; this is a consumer-gap, not a localized primitive fix. The FacingClass primitive exists and is correct (modulo finding 1) but is only wired to the turret.

### 3. Single RNG stream — gamemd has two phase-divergent gameplay streams

- **System:** rng — **File:** `src/sim/rng.rs`
- **Trigger frequency:** Every match, continuously — any tick with both a scatter/ore/sub-cell roll and a combat/audio/AI roll. Very high-frequency; affects full-match determinism and replay.
- **Drift:** All sim consumers draw from one shared `SimRng` cursor. gamemd seeds two `RandomClass` instances identically from one u32 then draws them independently: `Scen->Random` (scatter, sub-cell rotation, TIBTRE prob+direction, infantry random start frame, some HouseClass rolls, jumpjet bridge height) vs `g_MainRng` (combat/warhead, audio variant, lightning, tesla, laser, AI).
- **gamemd vs Rust:** two independent cursors (part of serialized ScenarioClass state, travels with save/replay) vs one interleaved sequence.
- **Why not auto-fixed:** Architectural — splitting the stream touches every RNG consumer and the serialization model; not a localized edit and would change the world hash.

### 4. Ore growth/spread runs AFTER combat+production in Rust; gamemd runs it FIRST in PerTickUpdate

- **System:** tick — **File:** `src/sim/world/mod.rs` (Phase 7)
- **Trigger frequency:** Every tick on any map with ore. Near-universal in skirmish.
- **Drift:** Rust grows/spreads ore at the END of the gameplay block, after combat read tiberium density (harvester reads, crater Reduce_Tiberium, miner docks) and after production consumed credits. gamemd runs `TiberiumClass__GrowthDriver_AllTypes()` then `SpreadDriver_AllTypes()` as the FIRST unconditional service calls of PerTickUpdate (0x0055B4D7), before any object AI, factory, or house update.
- **gamemd vs Rust:** combat/harvest reads in the same native tick see ore that already grew this tick; in Rust they see last tick's ore.
- **Why not auto-fixed:** Tick-order reorganization touching cross-phase data dependencies; design-level, not a one-line edit.

### 5. No unified live LogicClass object-vector AI pass (phase-split vs single count-reloading loop)

- **System:** tick — **File:** `src/sim/world/mod.rs`
- **Trigger frequency:** Every tick — governs same-tick side-effect visibility between objects. Universal.
- **Drift:** gamemd runs a single forward loop over the live object vector calling `vtable+0x5C` per object, reloading the count each iteration so tail-appended objects run same-pass and earlier objects' state is visible to later ones. Rust splits per-class systems across phases, many over `keys_sorted()` snapshots.
- **gamemd vs Rust:** object N's AI side effect visible to object N+1 this tick (binary) vs not (Rust phase-split).
- **Why not auto-fixed:** Core scheduler architecture; the LogicVector primitive exists (`logic_vector.rs`) but is not yet wired as the AI scheduler. Multi-session refactor.

### 6. Tick stages iterate `keys_sorted()` (stable-id order) instead of live-vector insertion order

- **System:** entitystore — **File:** `src/sim/entity_store.rs`
- **Trigger frequency:** Every tick, every per-entity pass. Universal.
- **Drift:** Production passes start with `entities.keys_sorted()` (ascending stable_id) split across fixed subsystem phases. gamemd walks the LogicClass-owned vector in INSERTION order, one object's full AI before the next, count reloaded each call. `for_each_live_object` (which models native order) is only used in snapshot tests.
- **gamemd vs Rust:** insertion-order single pass vs stable-id-order phased passes.
- **Why not auto-fixed:** Same root as finding 5; scheduler-level.

### 7. Direct `entities.remove()` at 5 sites bypasses `unregister_live_object`, leaking dead ids into hashed LogicVector

- **System:** entitystore — **File:** `src/sim/entity_store.rs`
- **Trigger frequency:** Every unit death routed through combat/other direct-remove paths — i.e. most deaths in a match. High-frequency, and it corrupts the determinism hash.
- **Drift:** Only `despawn_entity` (world/mod.rs:740-741) calls `unregister_live_object` before `remove`. Five sites call `remove()` directly and their callers don't scrub the id from `logic`. Stale ids accumulate in `LogicVector.order`, which is hashed (`world_hash.rs:43-47`), so they contribute to `state_hash` across the match. The module invariant at world/mod.rs:663-664 is false for these paths.
- **gamemd vs Rust:** binary compacts the vector on every object removal (never references a destroyed object) vs Rust leaks ids into the hashed order list.
- **Why not auto-fixed:** Several sites operate on a borrowed `&mut EntityStore` with no access to `Simulation::unregister_live_object`; correct fix needs a plumbing decision (where the scrub happens). Determinism-critical — needs review.

### 8. AOE damage falloff distance floored to whole cells before falloff; gamemd interpolates in raw leptons

- **System:** fixed — **File:** `src/util/fixed_math.rs` (consumer: `src/sim/combat/combat_aoe.rs` L173-174)
- **Trigger frequency:** Every splash/AOE weapon detonation — every artillery shell, missile, bomb. High-frequency in combat.
- **Drift:** `combat_aoe` computes `dist_leptons = isqrt_i64(...)` then `distance = SimFixed::from_num(dist_leptons / 256)` — integer-floors to whole cells BEFORE the falloff lerp, so targets at 128 and 255 leptons both collapse to distance=0 and take identical full damage. gamemd's `GetDamage` (FUN_00489180) interpolates in leptons with full per-lepton granularity (a target at 128 leptons gets t=0.5).
- **gamemd vs Rust:** per-lepton falloff vs per-cell-floored falloff. Consumer drift downstream of the fixed_math sqrt primitive.
- **Why not auto-fixed:** Consumer-side fix in `combat_aoe.rs`, outside this scan's three primitive files; touches damage output, needs review.

### 9. SIM_TICK_HZ = 45 contradicts its own doc comment ("matches RA2's native 15 fps")

- **System:** fixed — **File:** `src/util/fixed_math.rs` (L47-51)
- **Trigger frequency:** Latent — fires whenever a NEW consumer trusts the comment and uses an INI timing value directly against a 45 Hz tick (3x too fast). No current visible bug, but a documented landmine on a determinism-critical constant.
- **Drift:** Value is 45 but the comment claims 15 fps and "every sim tick equals one RA2 game frame ... INI timing values can be used directly without conversion." False at 45 Hz. Drive-track path compensates (`DRIVE_TRACK_SUBTICKS_PER_NATIVE_FRAME = 45/15 = 3`) but the constant's contract is stated wrong.
- **gamemd vs Rust:** gamemd logic is 15 fps binary-frame basis (INI Speed/ROF/Rate in 15 fps frames); Rust ticks at 45 Hz with per-consumer /3 compensation but a misleading contract.
- **Why not auto-fixed:** Whether any live consumer actually mis-scales needs a consumer audit before touching the doc-contract; flagged rather than silently reworded.

### 10. Square-root primitive uses precise integer Newton; gamemd uses float32-LUT Sqrt_Approx + ftol truncation

- **System:** fixed — **File:** `src/util/fixed_math.rs` (L226, L246)
- **Trigger frequency:** Every distance/range check — InRange, FindNearest, target acquisition — i.e. constantly. But the divergence is only ±1 lepton at specific large inputs.
- **Drift:** `isqrt_i64` / `int_distance_to_sim` compute precise integer floor sqrt. gamemd routes scalar distance through `Sqrt_Approx` (0x004CAC40): float32 mantissa-LUT (~12-13 bit, DAT_008650BC), then `ftol` truncate-toward-zero. The LUT can land the reconstructed value just below true sqrt so the int result is 1 less than the precise floor at specific inputs.
- **gamemd vs Rust:** float32-grade approximate sqrt + truncate vs precise integer floor. Differs by up to 1 lepton at the boundary set.
- **Why not auto-fixed:** Replacing the sqrt primitive with a LUT-faithful approximation changes range/targeting outputs widely and is determinism-critical; needs a dedicated implementation + the LUT table verified entry-by-entry.

### 11. fixed_sqrt / fixed_distance run fixed iteration count with no monotonic break

- **System:** fixed — **File:** `src/util/fixed_math.rs` (L168-175, L208-213)
- **Trigger frequency:** Wherever fractional fixed-point distance is consumed then truncated to int — fewer sites than the integer path, but still combat/movement math.
- **Drift:** 8 (resp. 16) Newton iterations with no `next >= guess` early break (unlike `isqrt_i64`), so the result can settle a fraction above/below true sqrt and oscillate by ±DELTA; a later `to_num::<i32>()` truncates a near-exact sqrt. gamemd produces `ftol(Sqrt_Approx(x))` — a single truncated integer from the float32 LUT.
- **gamemd vs Rust:** different representation (float32-grade vs I16F16) and different final integer when the LUT lands on the wrong side of an integer.
- **Why not auto-fixed:** Per-input divergence not proven bit-identical across the boundary set; same root as finding 10.

### 12. LightningStorm::Process runs Phase 4.5 (pre-combat) in Rust; gamemd runs it mid-ladder

- **System:** tick — **File:** `src/sim/world/mod.rs`
- **Trigger frequency:** Only while a Lightning Storm superweapon is active. Low-frequency, but high visibility when it fires (strike deaths feed retaliation/targeting).
- **Drift:** Rust applies lightning damage before combat (Phase 5), so strike deaths feed combat retaliation/targeting this tick. gamemd runs `LightningStorm__Process` after the laser/effect passes but its main-object-loop consumers (TechnoClass AI, retaliation) run later in the same native tick.
- **gamemd vs Rust:** superweapon damage placement relative to per-object AI differs.
- **Why not auto-fixed:** Tied to finding 5's scheduler reorganization.

### 13. Factory-then-House tail pass: gamemd runs both at the TAIL of PerTickUpdate; Rust scatters house work across phases

- **System:** tick — **File:** `src/sim/world/mod.rs`
- **Trigger frequency:** Every tick (house bookkeeping runs each tick). Universal but mostly internal-bookkeeping visibility.
- **Drift:** gamemd updates all factories, then all houses, as the last two service loops after tactical; house AI observes final post-factory state. Rust runs production in Phase 7 and spreads per-house state across power (Phase 4), production, superweapon grant, and defeat (Phase 8.5) — no single tail HouseClass::Update.
- **gamemd vs Rust:** ordered factories-then-houses tail vs scattered phased house mutation.
- **Why not auto-fixed:** Scheduler reorganization (finding 5 family).

### 14. AI decision/command placement: gamemd runs LogicClass::AI right after input (top of tick); Rust runs AI in Phase 8 (late)

- **System:** tick — **File:** `src/sim/world/mod.rs`
- **Trigger frequency:** Every tick the AI issues an order. Affects AI players every match; introduces a one-tick lag on AI-issued moves/builds.
- **Drift:** gamemd runs `LogicClass::AI` immediately after input, before per-object logic, so AI decisions influence the same tick. Rust applies due-commands at the top (consistent) but runs AI command generation+application at the very end (Phase 8), after movement/combat/production already ran — AI orders take effect a tick later.
- **gamemd vs Rust:** AI precedes object work (binary) vs AI follows object work (Rust). Code comment (world/mod.rs:1962-1964) acknowledges "AI placement is project-deferred."
- **Why not auto-fixed:** Acknowledged deferred design decision; scheduler-level.

### 15. Infantry sub-cell allocation ignores gamemd's approach-direction quadrant + RNG draw

- **System:** cellgrid — **File:** `src/sim/movement/bump_crush.rs` (allocator), consumed by `src/sim/pathfinding/cell_entry.rs`
- **Trigger frequency:** Every time an infantry enters a partially-occupied cell — common with infantry-heavy play. Visible as which corner the GI stands in.
- **Drift:** Rust always assigns the first free of `FUNCTIONAL_SUB_CELLS = [2,3,4]` in fixed order (NE, SW, SE), no approach input, no RNG. gamemd's `PlaceInfantryInCell` (0x00481180) computes an approach quadrant from the incoming vector (bit0 = approach.x_low>0x80, bit1 = approach.y_low>0x80, +1 if non-zero → {2,3,4}; <60 leptons → quadrant 0 = randomize via a g_RNG draw against `DAT_0081CC98`).
- **gamemd vs Rust:** approach-direction- and RNG-dependent slot selection vs fixed first-free order. Max-3-per-cell matches; selection order does not.
- **Why not auto-fixed:** Requires plumbing the approach vector + an RNG draw into the allocator — behavioral change touching placement output and RNG consumption; needs review.

### 16. LogicVector primitive is correct but not wired as the live per-object AI scheduler

- **System:** cellgrid — **File:** `src/sim/world/logic_vector.rs` (+ `for_each_live_object` in `world/mod.rs`)
- **Trigger frequency:** Every tick (it's the would-be scheduler). Same observable surface as findings 5/6.
- **Drift:** `LogicVector` (tail-append, retain-based compacting remove, verbatim snapshot, serde-as-Vec) and `for_each_live_object` faithfully model the native forward walk, but `for_each_live_object` is only exercised in passenger/tests. advance_tick still runs separate `keys_sorted()` subsystem passes, so same-pass append and self-removal index semantics aren't applied to the real AI tick.
- **gamemd vs Rust:** primitive matches; the wiring gap means same-pass append doesn't run this tick and self-unregister doesn't cause the native one-object skip.
- **Why not auto-fixed:** Wiring the scheduler is the finding-5 multi-session refactor.

### 17. Vision/fog refresh runs as an explicit pre-combat phase in Rust; gamemd has no global pre-object vision pass

- **System:** tick — **File:** `src/sim/world/mod.rs`
- **Trigger frequency:** Every tick. Potentially affects combat targeting near shroud/fog edges each tick.
- **Drift:** Rust computes full owner visibility (`recompute_owner_visibility_in_place`) as a discrete phase after movement, before combat (world/mod.rs:1582-1593), then combat reads `Some(&self.fog)`. gamemd maintains fog/shroud per-object (Map::Logic in Main_Tick before render, per-object Reveal); there is no global all-owner recompute slot before the main object loop.
- **gamemd vs Rust:** bulk pre-combat recompute vs incremental per-object reveal/conceal. Whether targeting results match within a tick is unproven.
- **Why not auto-fixed:** Needs verification that the bulk recompute produces identical per-tick targeting; flagged for review (overlaps NEEDS_RESEARCH).

## Needs further research (NEEDS_RESEARCH + UNCHECKED)

What to investigate next, by system:

- **lepton — `lepton_to_screen` Z lift scale.** util/lepton.rs uses 15px/level (z as level) while gamemd lifts by `leptons × 15/256`; `terrain.rs` `lepton_to_screen` already does the lepton form. The two Rust helpers disagree on Z scale. `LevelHeight` global (0x89DDB8) is BSS, computed at runtime by `HeightFactor_Init` (0x45B080) — exact lepton value (community 104) needs a debugger.
- **lepton — `HIGH_FLIGHT_THRESHOLD_LEPTONS = 1000`** is a self-labeled placeholder; gamemd splits at `HighFlightLevel*2` (`DAT_00ac13c8*2`, written at 0x5f37eb from an FMUL/ftol; INI FlightLevel=1500). Exact value requires a running debugger.
- **lepton — `BRIDGE_HEIGHT_DELTA_LEPTONS = 416`** placeholder (`BridgeHeight=4*104`); gamemd uses runtime `DAT_00b0eb24` (BSS), dependent on the same unverified LevelHeight constant.
- **lepton — `cell_delta_to_lepton_dir` diagonal length 362.038** has no gamemd equivalent (gamemd advances along the drive-track waypoint table 0x7E7A28, doesn't normalize a (256,256) vector). 362.038 vs floored 362 is a deterministic divisor difference that can accumulate per-tick step length. (UNCHECKED — also listed under NEEDS_REVIEW as a DRIFT-suspect.)
- **lepton — module doc claims `sub_x/sub_y` range 0..256** but offsets are an open primitive with no clamping. gamemd stores absolute leptons and derives in-cell residue `coord & 0xFF` (0..255, never 256). Verifying every Rust producer keeps `[0,256)` and re-normalizes carries spans the movement modules — out of this file's scope.
- **lepton — `subcell_lepton_offset(Some(1))` exposes dead sub-cell index 1 (64,64).** gamemd has the same 5-entry table at 0x89E9F0 but index 1 is DEAD (`GetSubCell` 0x4810A0 never returns 1; `PlaceInfantryInCell`/`IsSubCellFree` skip indices 0 and 1). Rust `FUNCTIONAL_SUB_CELLS=[2,3,4]` is correct but lepton.rs doesn't reject index 1. (DRIFT if index 1 ever leaks in.)
- **fixed — `facing_from_delta_int` / `_u16` use f32 atan2 in the sim layer** — a non-deterministic primitive violating the fixed-point contract; gamemd uses a different angle source. No gamemd `Desired_Facing` decompilation was done — UNCHECKED until bucket equivalence is proven.
- **fixed — INI-load conversions (`sim_from_f32/f64`, `from_num`) round to nearest; gamemd `ftol` truncates toward zero.** Needs a parser/consumer audit to confirm which fields (e.g. `cellSpread_leptons = ftol(CellSpread*256)`) are affected.
- **facing — SIN/COS facing table (`facing_table.rs`) not verified against gamemd's sine LUT.** No gamemd sine-table address located in this scan. Per burden-of-proof, an unverified table feeding movement math is DRIFT-suspect; needs entry-by-entry compare including negative-facing and the 64/128/192 cardinal exacts.
- **facing — `advance_drive_track` uses coordinate-based cell-jump detection** rather than gamemd's `Apply_Track_Delta`/`Process_Drive_Track` cell-cross index + residual semantics (a deliberate different mechanism per the L3750 comment). DRIFT-by-default until proven output-identical; needs a focused trace of cell-cross index consumption + residual budget arithmetic.
- **rng — `next_range_u32` exclusive wrapper** correctly delegates to inclusive `(0, max-1)`, matching gamemd for "pick one of N" callers, but per-caller intent across ~111 sites is unverified (some may need inclusive `RandomRanged(0,n)`); the power-of-two mask bug also flows through this path.
- **rng — `span >= 0x7FFFFFFF` guard** matches the binary exactly at 0x7FFFFFFF (no-draw return) but diverges for span in `0x80000000..=0xFFFFFFFE` (binary infinite-loops drawing-but-never-accepting; Rust short-circuits). No live YR caller passes span ≥ 0x7FFFFFFF (max 0x7FFFFFFE), so this is dead-input territory; flagged because it's unprovable as MATCH across the full input space.
- **tick — turret rotation runs AFTER combat in Rust** (drives next-frame facing); needs verification this matches gamemd's per-object Facing_Update ordering inside the single object loop rather than two separate global passes (facing updates relative to OTHER objects' fire decisions can differ).
- **tick — `MapClass::RecalcBridgeShroudFlags` fires every 120 frames** (`g_CurrentFrameCounter % 0x78 == 0`) early in PerTickUpdate (0x0055B29A); no equivalent 120-frame cadence found in advance_tick. Rust bridge state is event-driven (damage/repair). Whether the periodic shroud recalc is player-visible in standard YR needs confirmation, but it's a live unconditional call.
- **entitystore — same-pass append + compacting-removal index semantics** are modeled (snapshot.rs tests prove the model) but `for_each_live_object` has zero production callers. Not empirically diffed against a gamemd trace — UNCHECKED per burden-of-proof.
- **cellgrid — `runtime_can_enter_direction` 0-7 encoding** not verified against gamemd's `Can_Enter_Cell` predecessor argument (facing-byte vs index confusion is a recurring bug class). 0x004D9C60 / 0x0073F0A0 not decompiled this session — UNCHECKED.

## Notes

The working tree was already dirty from in-progress work before this pass started
(many `src/sim/**` and `src/app*` files were modified, plus a new
`src/sim/world/logic_vector.rs`). As a result, the two files this pass edited
(`src/sim/rng.rs` and `src/sim/entity_store.rs`) mix this pass's auto-fix edits
with prior uncommitted changes. When reviewing or committing, isolate the
RNG-mask change (`next_range_u32_inclusive` + its two new tests) and the
EntityStore doc/test change (`insert_does_not_auto_sync_owner_index`) from the
unrelated in-flight edits in the same files / sibling modules. The build/test
gate was run against the full dirty tree, so the green result reflects the
combined state, not this pass's edits in isolation.
