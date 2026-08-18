# Particle RNG Classification -- Ghidra Research Report

**Address(es):** `ParticleClass::Constructor @ 0x0062B5E0`, `ParticleSystemClass::AI_Smoke @ 0x0062ED40`, `ParticleSystemClass::AI_Gas @ 0x0062E6D0`, `ParticleSystemClass::AI_Fire @ 0x0062F9A0`, `ParticleClass::AI_Fire @ 0x0062CB10`, `ParticleClass::AI_Gas @ 0x0062BD50`, `ParticleClass::AI_Smoke @ 0x0062C540`, `SpawnParticleWithInsert @ 0x0062E4C0`
**Investigation Mode:** coverage-map
**Claimed Scope:** classify RNG calls in current Rust `src/sim/particles/{fire,smoke,gas,spawn}.rs` against verified gamemd particle RNG patterns.
**Non-Scope:** Spark/Railgun implementation, full particle damage application, full bridge collision modeling, renderer detail gates, and every constructor caller's gameplay semantics.
**Confidence:** High for classified RNG bounds/order in the scoped functions; Medium for unimplemented Rust deltas that depend on deferred non-RNG particle wiring.
**Active in YR:** Yes. Particle systems are constructed from standard YR paths (`TechnoClassFireAtSpawnsBullet`, `Apply_area_damage`, `TechnoClass::ReceiveDamage`, `TechnoClass::AI_Update`, `BuildingClass::UpdateGapGenerator_Tick`, `UnitClass::AI`) and tick through `ParticleSystemClass::AI` vtable dispatch.
**Status:** COMPLETE for classification of the scoped Rust RNG call sites.

## 0. Investigation Frame

Target question: do the scoped Rust particle RNG calls use the same gamemd/YR source helper, ranges, and consumption order after the `RandomRanged` rewrite?

Non-goals: do not implement; do not classify Spark/Railgun beyond noting shared constructor paths; do not chase all particle creation callers; do not resolve deferred movement/damage wiring.

Evidence needed to mark COMPLETE: decompile plus assembly/xref evidence for each handoff-critical RNG bound or helper choice; Rust file/line scan for every scoped `next_range_u32`; active-YR caller evidence for the particle system path.

Stop conditions: every scoped Rust RNG call classified GREEN/YELLOW/RED; report written; no Rust/INI/in-repo docs modified.

## 1. Overview

The scoped Rust particle code currently uses `SimRng::next_range_u32(n)` for lifetimes, spawn offsets, smoke child offsets, fire jitter, and fire insertion shuffling. After the recent RNG rewrite, that helper means gamemd `RandomRanged(0, n-1)` with mask-and-reject semantics.

gamemd particle code usually does something different: it calls `Random__Next()` directly and then applies `%`, bit masks, or sign normalization. Numeric output bounds often look similar, but RNG consumption and distribution differ because raw modulo never retries. These particle sites are therefore RED for call-order parity unless the Rust call is replaced with a raw-next modulo/bitmask helper.

## 2. Scoped Rust RNG Surfaces

| Rust surface | Current RNG call | Classification | Evidence |
|---|---:|---|---|
| `spawn.rs::spawn_particle` lifetime for Railgun | `next_range_u32(10)` | RED for helper choice; range value matches `abs(Random__Next() % 10)` | `src/sim/particles/spawn.rs:96`; `0x0062B842..0x0062B85C` |
| `spawn.rs::spawn_particle` lifetime for non-Railgun | `next_range_u32(max_ec)` | RED for helper choice; range value matches `abs(Random__Next() % MaxEC)` when MaxEC > 0 | `src/sim/particles/spawn.rs:99`; `0x0062B870..0x0062B889` |
| `fire.rs::make_particle` lifetime | `next_range_u32(max_ec)` | RED for helper choice; Rust clamps zero to one, gamemd constructor would divide by `MaxEC` | `src/sim/particles/fire.rs:65`; `0x0062B870..0x0062B889` |
| `smoke.rs::make_particle` lifetime | `next_range_u32(max_ec)` | RED for helper choice; same constructor mismatch | `src/sim/particles/smoke.rs:178`; `0x0062B870..0x0062B889` |
| `gas.rs::make_particle` lifetime | `next_range_u32(max_ec)` | RED for helper choice; same constructor mismatch | `src/sim/particles/gas.rs:198`; `0x0062B870..0x0062B889` |
| `smoke.rs` system spawn offset | two `next_range_u32(r + 1)` draws | RED for helper choice; numeric bounds are `0..SpawnRadius`, raw `% (SpawnRadius+1)` | `src/sim/particles/smoke.rs:88`; `0x0062F0AC..0x0062F0D5` |
| `gas.rs` system spawn offset | two `next_range_u32(r + 1)` draws | YELLOW/RED: same code pattern applies only to Smoke system AI; Gas system AI has no periodic self-spawn in the verified binary slice | `src/sim/particles/gas.rs:86`; `ParticleSystemClass::AI_Gas @ 0x0062E6D0` |
| `smoke.rs::symmetric_offset` | `next_range_u32(r)` | RED for helper choice; numeric formula matches the documented asymmetric offset if raw `% r` is used | `src/sim/particles/smoke.rs:213`; `0x0062EE4C..0x0062EE9E` |
| `fire.rs::tick_particle` jitter | `next_range_u32(10) - 5` | RED for helper choice; numeric range is `-5..=4`, but gamemd uses `Random__Next() % 10 - 5` | `src/sim/particles/fire.rs:116`; `0x0062CB41..0x0062CB51` |
| `spawn.rs::spawn_particle_with_insert` | `next_range_u32(actual_range)` | RED for helper choice; numeric range is `0..actual_range-1`, but gamemd uses `abs(Random__Next()) % actual_range` | `src/sim/particles/spawn.rs:229`; `0x0062E590..0x0062E59D` |

## 3. Verified Binary Facts

### 3.1 Particle Constructor Lifetime Randomization

Active in YR: Yes. `ParticleClass::Constructor` is called from standard particle spawn functions and direct smoke/gas child constructors. `ParticleSystemClass::Constructor` has stock-YR xrefs from weapon, area damage, damage smoke, gap generator, and refinery smoke paths.

For `ParticleType.BehavesLike == 4` (Railgun), constructor calls `Random__Next()` then signed-divides by constant `10`, takes an absolute signed remainder, and adds it to `ParticleType.MaxEC`. Evidence: decompile `0x0062B5E0`; assembly `0x0062B842..0x0062B85C` shows `CALL 0x0065C780`, `IDIV 0xA`, `XOR/SUB` absolute, `ADD AX,[type+0x2E0]`.

For all other particle types, constructor calls `Random__Next()` then signed-divides by `ParticleType.MaxEC`, takes an absolute signed remainder, and adds it to `MaxEC`. Evidence: decompile `0x0062B5E0`; assembly `0x0062B870..0x0062B889` shows `CALL 0x0065C780`, `IDIV [type+0x2E0]`, `XOR/SUB`, `ADD AX,[type+0x2E0]`.

Do not substitute `RandomRanged(0, MaxEC-1)`: its rejection loop changes the number of raw RNG draws whenever the masked value is outside the span. This matters heavily for stock `MaxEC=80`, `500`, and `1000`.

### 3.2 Smoke System Periodic Spawn Offsets

Active in YR: Yes for stock smoke systems (`BigGreySmokeSys`, `SmallGreySSys`, `DebrisSmokeSys`, `TestSmokeSys`) with `Spawns=yes` in `rulesmd.ini`.

`ParticleSystemClass::AI_Smoke` gates periodic spawn on `CurrentFrame % int(spawn_timer) == 0`, then calls `Random__Next()` twice. It computes `off_y = random % (SpawnRadius+1)` and `off_x = previous_random % (SpawnRadius+1)`, with no absolute normalization visible in the decompile for this path. Evidence: decompile `0x0062ED40`; assembly `0x0062F0AC..0x0062F0D5` shows two `CALL 0x0065C780`, `LEA EBP,[SpawnRadius+1]`, and two `IDIV EBP` remainders.

Rust's numeric bounds (`0..=SpawnRadius`) match only for non-negative remainders, but the helper choice is wrong after `next_range_u32` became `RandomRanged`. This call must preserve two raw draws in this order.

### 3.3 Smoke NextParticle Two-Child Offsets

Active in YR: Yes for any smoke particle with `NextParticle`; stock smoke particles do not define `NextParticle`, but the code path is live for modded smoke. It is still a particle-class behavior and not TS-only.

When a dying smoke particle has `NextParticle`, AI_Smoke computes `r = Radius >> 3`, draws `Random__Next() % r` for X and Y, then transforms each component with `if rem <= 0 then rem -= r else rem += r`. Two children are created at `(+dx,+dy)` and `(-dx,-dy)`, with extra translucency randomization via `Random__Next() % 6`. Evidence: decompile `0x0062ED40`; assembly `0x0062EE4C..0x0062EE9E` and `0x0062EF3B..0x0062F01D`.

Rust has the two-child shape and asymmetric transform, but `next_range_u32(r)` is the wrong helper. The child translucency randomization (`% 6` adds `0x19` unless remainder is zero) is also not modeled in the scoped Rust.

### 3.4 Fire Jitter

Active in YR: Yes for `[FireStream]` via `FireStreamSys`, `UseFireParticles=yes`, and the fire system AI dispatch.

`ParticleClass::AI_Fire` uses one raw `Random__Next()` draw, signed-divides by `10`, and subtracts `5`, producing jitter integers `-5..=4`. Evidence: decompile `0x0062CB10`; assembly `0x0062CB41..0x0062CB51` shows `CALL 0x0065C780`, `IDIV 0xA`, `SUB EDX,0x5`.

Rust has the correct visible integer range but the wrong helper: `next_range_u32(10)` is `RandomRanged(0,9)`, not raw modulo.

### 3.5 Fire SpawnParticleWithInsert Shuffle

Active in YR: Yes. `ParticleSystemClass::AI_Fire @ 0x0062F9A0` calls `FUN_0062E4C0(..., 4)` after its spawn cadence / target-moved checks. `FUN_0062E4C0` is the fire insertion helper.

The helper first constructs and appends a particle, clamps `actual_range = min(insert_range, count)`, draws `Random__Next()`, takes `abs(draw) % actual_range`, and moves the new particle within the recent tail. Evidence: decompile `0x0062E4C0`; assembly `0x0062E590..0x0062E59D` shows `CALL 0x0065C780`, `XOR/SUB` absolute, then `IDIV EDI`.

Rust's insertion topology is close, but the random offset must be raw `abs(Random__Next()) % actual_range`, not `RandomRanged(0, actual_range-1)`.

### 3.6 Gas/Smoke Per-Particle Drift RNG

Active in YR: Yes. Gas and smoke particle AI dispatches are reached from normal `ParticleClass::AI_Dispatch`.

Gas AI uses random drift only on even frames: one `Random__Next()` draw gated by `(abs(draw) & 7) == 0`, then a second draw chooses axis with `& 1`, and a third draw applies `abs(draw) % 3 - 1`; drift clamps to `[-2, 2]`. Evidence: decompile `0x0062BD50`; assembly `0x0062BD93..0x0062BDF1`.

Smoke AI uses random drift only on odd frames: one `Random__Next()` draw gated by `(abs(draw) & 3) == 0`, then a second draw chooses axis, and a third draw applies `abs(draw) % 3 - 1`; drift clamps to `[-5, 5]`. Evidence: decompile `0x0062C540`; assembly `0x0062C55E..0x0062C5B1`.

Rust currently defers per-particle random drift in both smoke and gas, so there is no incorrect helper at those call sites yet. The future implementation must not use `RandomRanged(0,3)`, `RandomRanged(0,7)`, or `RandomRanged(0,2)` for these gates.

### 3.7 One True RandomRanged Particle Constructor Site

Active in YR: Conditional. It applies when a particle type has color-list randomization data at `type+0x2D4..0x2D9`, used by Spark/Railgun-style color effects and possibly modded particles.

`ParticleClass::Constructor` calls `Random__RandomRanged(0, 0x7FFFFFFE)` at `0x0062BAC0` before color interpolation selection. Evidence: xref `Random__RandomRanged` from `ParticleClass__Constructor`; assembly `0x0062BAB4..0x0062BAEE` pushes `0x7FFFFFFE` and calls `0x0065C7E0`.

This is not the lifetime/jitter/spawn-offset helper. Do not generalize it to all particle RNG.

## 4. INI Keys

| Key / section | Stock values relevant to this report | Effect | Active in YR |
|---|---|---|---|
| `[ParticleSystems] BehavesLike=` | Smoke/Gas/Fire/Spark/Railgun | selects PSC AI dispatch | Yes |
| `[ParticleSystems] Spawns=` | smoke and fire stock systems use `yes`; `GasCloudSys` defaults no | enables periodic self-spawn in smoke/fire | Yes |
| `[ParticleSystems] SpawnFrames=` | Smoke 2/10, FireStream 4 | cadence for spawn gates | Yes |
| `[ParticleSystems] SpawnRadius=` | Big smoke 10, small/test 5, debris 3 | raw modulo spawn offset bound | Yes for smoke |
| `[Particles] MaxEC=` | FireStream 500, smoke 80, gas 50/448/1000 | lifetime base plus raw modulo extra | Yes |
| `[Particles] Radius=` | used by smoke NextParticle offset | `Radius >> 3` raw modulo bound | Conditional; stock smoke lacks NextParticle |
| `[Particles] WindEffect=` | stock gas/smoke mostly `0` | affects wind movement, not the random drift gates | Conditional |

## 5. Integration Points

`ParticleSystemClass::AI @ 0x0062FD60` dispatches by system `BehavesLike`: Smoke `0x0062ED40`, Gas `0x0062E6D0`, Fire `0x0062F9A0`, Spark, Railgun. Xrefs show the AI method is installed in the PSC vtable.

`ParticleClass::AI_Dispatch @ 0x0062CE40` dispatches by particle `BehavesLike`: Gas `0x0062BD50`, Smoke `0x0062C540`, Fire `0x0062CB10`, Spark, Railgun, then decrements particle lifetime and marks deletion when it reaches zero.

Constructor xrefs prove standard YR activity: `Apply_area_damage`, `TechnoClassFireAtSpawnsBullet`, `TechnoClass::ReceiveDamage`, `TechnoClass::AI_Update`, `BuildingClass::UpdateGapGenerator_Tick`, `UnitClass::AI`, and other standard paths call `ParticleSystemClass::Constructor @ 0x0062DC50`.

## 6. Current Rust Implementation Status

`src/sim/particles/spawn.rs` has the shared constructor-like lifetime draw and fire insertion draw. These are RED because they use `next_range_u32`, which is now RandomRanged semantics.

`src/sim/particles/fire.rs` has lifetime draw and fire jitter draw. Both are RED for helper choice; the visible jitter bounds are otherwise correct.

`src/sim/particles/smoke.rs` has lifetime draw, periodic spawn offsets, and smoke child symmetric offsets. All scoped RNG calls are RED for helper choice. Smoke child count/topology is otherwise consistent with prior docs, but child translucency randomization is missing.

`src/sim/particles/gas.rs` has lifetime draw and a smoke-like periodic spawn offset path. Lifetime is RED. The periodic gas self-spawn path is YELLOW/RED because verified gamemd `AI_Gas` did not show the same periodic spawn branch; if Rust keeps this behavior for gas, it is at least not binary-backed.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Particle constructor lifetime RNG | verified | `0x0062B842..0x0062B889` | none |
| Fire jitter RNG | verified | `0x0062CB41..0x0062CB51` | none |
| Fire insertion RNG | verified | `0x0062E4C0`, `0x0062E590..0x0062E59D` | exact Rust insertion index should be checked in implementation |
| Smoke periodic spawn offsets | verified | `0x0062ED40`, `0x0062F0AC..0x0062F0D5` | signed negative remainder behavior needs runtime/assembly audit if stock negative raw draws appear |
| Smoke NextParticle offset RNG | verified | `0x0062EE4C..0x0062EE9E` | stock path conditional because vanilla smoke lacks NextParticle |
| Smoke child translucency RNG | verified | `0x0062EF3B..0x0062F01D` | Rust implementation missing |
| Gas random drift RNG | verified | `0x0062BD93..0x0062BDF1` | Rust implementation missing |
| Smoke random drift RNG | verified | `0x0062C55E..0x0062C5B1` | Rust implementation missing |
| Gas periodic self-spawn | touched-not-exhausted | `0x0062E6D0`; Rust `gas.rs:86` | confirm whether Rust should remove/disable this path |
| Spark/Railgun RNG | deferred | `PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md` | out of scope for slot 6 |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-PARTICLE-RNG-001 -- Which helper does lifetime randomization use? -> Raw Random__Next modulo, not RandomRanged.` (evidence: `0x0062B842..0x0062B889`)
- `[RESOLVED] OQ-PARTICLE-RNG-002 -- Does fire jitter use RandomRanged? -> No, raw signed remainder `% 10 - 5`.` (evidence: `0x0062CB41..0x0062CB51`)
- `[RESOLVED] OQ-PARTICLE-RNG-003 -- Does fire insertion use RandomRanged? -> No, abs(raw) % actual_range.` (evidence: `0x0062E590..0x0062E59D`)
- `[RESOLVED] OQ-PARTICLE-RNG-004 -- Are smoke child offsets two-child and random? -> Yes, two children at signed symmetric offsets from raw `% (Radius >> 3)`.` (evidence: `0x0062EE4C..0x0062EE9E`)
- `[RESOLVED] OQ-PARTICLE-RNG-005 -- Are smoke/gas random drift gates implemented in Rust? -> No, both are deferred; future code must use bitmask/raw modulo.` (evidence: `src/sim/particles/smoke.rs:17`, `src/sim/particles/gas.rs:22`; `0x0062BD93..0x0062BDF1`, `0x0062C55E..0x0062C5B1`)
- `[RESOLVED] OQ-PARTICLE-RNG-006 -- Is there any ParticleClass RandomRanged call? -> Yes, color interpolation selection uses RandomRanged(0,0x7FFFFFFE), but it is not lifetime/spawn/jitter.` (evidence: `0x0062BAC0`)
- `[DEFERRED] OQ-PARTICLE-RNG-007 -- Should Rust gas periodic self-spawn exist?` (category: `requires-different-system-context`; reason: `AI_Gas` slice did not show a periodic branch, but system setup/caller semantics may still create gas particles externally; next-step-if-pursued: `trace GasCloudSys creation and first particle insertion from Apply_area_damage/warhead particle path`)
- `[DEFERRED] OQ-PARTICLE-RNG-008 -- Exact signed negative remainder behavior for smoke spawn offsets when Random__Next returns negative` (category: `needs-runtime-debugger`; reason: `decompiler shows direct IDIV remainder but stock Random__Next sign distribution should be checked before clamping decisions; next-step-if-pursued: `instrument seed cases or inspect Random__Next signed output contract`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Particle lifetime extra is `abs(Random__Next() % 10)` for Railgun and `abs(Random__Next() % MaxEC)` for all other types | `0x0062B842..0x0062B889` | mismatch: Rust uses `next_range_u32` / RandomRanged and clamps zero to one | `src/sim/particles/spawn.rs`, `fire.rs`, `smoke.rs`, `gas.rs` constructors | add/use raw-next modulo helper for particle lifetimes; preserve one draw/no rejection | deterministic seed where `RandomRanged` would reject must leave later RNG state different before fix and equal after fix | `test_particle_lifetime_uses_raw_next_modulo_without_rejection`; do not use RandomRanged |
| Fire jitter is `Random__Next() % 10 - 5` with one raw draw | `0x0062CB41..0x0062CB51` | mismatch: Rust range is correct but helper is RandomRanged | `src/sim/particles/fire.rs::tick_particle` | produce `-5..=4` from raw signed remainder and consume exactly one raw RNG draw | fire stream with pinned seed matches expected raw draw count after one particle AI tick | `test_fire_jitter_consumes_one_raw_modulo_draw`; do not preserve `next_range_u32(10)` |
| Fire insert shuffle uses `abs(Random__Next()) % actual_range` after appending | `0x0062E590..0x0062E59D`; caller `0x0062F9A0` passes `4` | mismatch: Rust uses RandomRanged for random_offset | `src/sim/particles/spawn.rs::spawn_particle_with_insert` | raw modulo offset; verify insertion index against binary formula in same patch | fire system with 5 spawned particles and pinned seed has expected order and RNG state | `test_fire_spawn_insert_uses_raw_abs_modulo_tail_shuffle`; avoid off-by-one tail insertion changes without checking `count-2` formula |
| Smoke periodic spawn offsets are two raw modulo draws with bound `SpawnRadius+1` | `0x0062F0AC..0x0062F0D5`; stock smoke `Spawns=yes` in `rulesmd.ini` | mismatch: Rust uses RandomRanged | `src/sim/particles/smoke.rs::tick_system` | consume exactly two raw draws for offsets before constructor draw | BigGreySmokeSys first spawn from pinned seed matches offset draw count and lifetime draw order | `test_smoke_spawn_offsets_use_two_raw_modulo_draws_before_constructor`; do not collapse offset and constructor RNG |
| Smoke NextParticle offset uses raw `% (Radius>>3)` for X then Y, and child translucency uses raw `% 6` separately | `0x0062EE4C..0x0062F01D` | partial: topology exists; helper wrong; translucency randomization missing | `src/sim/particles/smoke.rs::symmetric_offset`, `make_child` | raw modulo offsets, preserve two children, add `%6` translucency behavior if implementing visual fidelity | dying smoke particle with `Radius=16`, pinned seed, produces symmetric children and consumes offset/translucency draws in binary order | `test_smoke_nextparticle_offsets_and_translucency_use_raw_modulo_order`; do not replace with uniform inclusive ranges |

## 10. Negative Facts / Do Not Do

- Do not use `RandomRanged` / `next_range_u32` for particle lifetimes. Evidence: raw `Random__Next` plus `IDIV` at `0x0062B842..0x0062B889`.
- Do not use `RandomRanged(0,9)` for fire jitter. Evidence: raw `Random__Next() % 10 - 5` at `0x0062CB41..0x0062CB51`.
- Do not assume all particle RNG migrated to `RandomRanged`; the only scoped ParticleClass `RandomRanged` call is color-list selection at `0x0062BAC0`.
- Do not implement future gas/smoke random drift with uniform `RandomRanged` calls. Evidence: gas bit gates `&7`, smoke bit gates `&3`, then raw `%3 - 1`.
- Do not treat gas and smoke wind/random movement as identical. Evidence: gas AI/move uses different frame parity, drift clamp `[-2,2]`, and wind tables than smoke; see `0x0062BD50`, `0x0062C540`, `0x0062D2A0`, `0x0062D3F0`.

## 11. Remaining Uncertainty

- Gas periodic self-spawn in Rust is not backed by the decompiled `ParticleSystemClass::AI_Gas` slice; classify as YELLOW/RED until the gas creation path is traced.
- Smoke spawn raw negative remainders may require an exact signed-output check for `Random__Next`; do not silently clamp to unsigned behavior.
- Spark/Railgun RNG remains out of this slot except shared constructor lifetime and prior report notes.

## Sources

- Ghidra decompiled: `ParticleClass__Constructor @ 0x0062B5E0`, `ParticleSystemClass__SpawnParticle @ 0x0062E380`, `FUN_0062E4C0`, `ParticleSystemClass__AI_Smoke @ 0x0062ED40`, `ParticleSystemClass__AI_Gas @ 0x0062E6D0`, `ParticleSystemClass__AI_Fire @ 0x0062F9A0`, `FUN_0062BD50`, `FUN_0062C540`, `FUN_0062CB10`, `FUN_0062D2A0`, `FUN_0062D3F0`, `ParticleClass__AI_Dispatch @ 0x0062CE40`, `ParticleSystemClass__AI @ 0x0062FD60`.
- Ghidra assembly context: `0x0062B842..0x0062B889`, `0x0062BAAC..0x0062BAEE`, `0x0062CB41..0x0062CB51`, `0x0062E590..0x0062E59D`, `0x0062EE4C..0x0062F0D5`, `0x0062BD93..0x0062BDF1`, `0x0062C55E..0x0062C5B1`.
- Docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/PARTICLE_TIMING_SPARK_RAILGUN_NORMALIZED_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/particles/fire.rs`, `smoke.rs`, `gas.rs`, `spawn.rs`.
- INI scanned: `ini/rulesmd.ini`, `ini/rules.ini`.
