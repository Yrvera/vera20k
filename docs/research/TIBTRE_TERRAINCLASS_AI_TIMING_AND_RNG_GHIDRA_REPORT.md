# TIBTRE TerrainClass AI Timing And RNG - Ghidra Research Report

**Address(es):** `0x0071C730` (`TerrainClass::AI`), `0x00426630` (`CDTimerClass::GetTimeRemaining`), `0x0065C780` (`Random::Next`), `0x0071DEA0` (`TerrainTypeClass::ReadINI_Full`), `0x0071BB90` (`TerrainClass` map instance constructor), `0x0071DA80` (`TerrainTypeClass` constructor)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** TIBTRE terrain-object probability roll, animation timer, midpoint spawn trigger, frame-count source, and RNG consumption order in `TerrainClass::AI`.
**Non-Scope:** `CellClass::SpreadTiberium` placement gates/type selection, `PlaceTiberium` overlay/queue effects, terrain-object lighting, death/damage behavior, AnimClass meteor/crystal ore spawning.
**Confidence:** High for the scoped AI/timer/RNG behavior. Retail stock SHP frame counts are confirmed by the follow-up retail-SHP report: all standard-theater `TIBTRE01/02/03` variants are 22 frames.
**Active in YR:** Yes. Stock `rulesmd.ini` has `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]` with `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`.

## Working Notes

Target question: Does `TerrainClass::AI` make current Rust's immediate TIBTRE spawn model parity-wrong for timing and deterministic RNG?
Non-goals: Do not re-investigate `SpreadTiberium` acceptance/type/density, terrain light keys, AnimClass bouncer ore spawning, or Rust implementation patches.
Evidence needed to mark COMPLETE: direct Ghidra evidence for probability order/denominator, RNG API, `IsAnimated`/idle gates, CDTimer semantics, midpoint comparison, frame-count source, reset behavior, and current Rust delta.
Stop conditions: all scoped questions resolved/deferred with evidence; no Ghidra mutations; write only this report plus `.swarm-claims.md`.

## 1. Overview

TIBTRE spawning is a two-stage terrain animation path. While idle, an animated terrain object rolls a raw `Random::Next()` value once per AI tick; on a successful probability check it arms a CDTimer-style animation. Ore is not placed on the hit tick: `CellClass::SpreadTiberium(1)` is called only on a later timer-expiry tick where the current animation frame equals half of the loaded SHP frame count.

Earlier Rust collapsed the sequence into `roll -> direction RNG -> spawn` on the same tick. As of the 2026-05-24 TIBTRE implementation pass, `src/sim/terrain_spawn.rs` has a stateful idle/active model with loaded frame counts and delayed midpoint spawning; this report's remaining Rust-facing value is to preserve the exact timing/RNG contract and avoid regressions.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|---|
| `TerrainClass` | `+0xAC` | int | current animation frame | `TerrainClass::AI @ 0x0071C730`; constructor `0x0071BB90` initializes to 0 | Yes |
| `TerrainClass` | `+0xB0` | byte | redraw/animation-dirty flag | AI writes 1 on frame advance and 0 when inactive | Yes |
| `TerrainClass` | `+0xB4` | int | CDTimer start frame | constructor and AI write `g_CurrentFrameCounter` | Yes |
| `TerrainClass` | `+0xBC` | int | CDTimer duration | AI writes `AnimationRate`; `CDTimerClass::GetTimeRemaining @ 0x00426630` reads timer `+0x08` | Yes |
| `TerrainClass` | `+0xC0` | int | animation-active duration mirror / idle gate | AI checks `== 0` before probability roll and `!= 0` before frame advance | Yes |
| `TerrainClass` | `+0xC4` | int | frame increment | constructor `0x0071BB90` sets to `1`; AI adds it to current frame | Yes |
| `TerrainClass` | `+0xC8` | ptr | `TerrainTypeClass*` | AI reads type fields through this pointer | Yes |
| `TerrainClass` | `+0xCD` | byte | wrap-early-return gate | constructor `0x0071BB90` sets to `0`; when nonzero, timer-expiry path checks `current_frame == frame_count - 1` and calls vtable+0xF8 before returning; **always 0 for stock TIBTRE** (corrected 2026-05-28: field was absent from table; live decompile of `0x0071C730` shows `CMP byte [ESI+0xCD], BL` gate at ~`0x0071C7FE`, followed by `MOVSX`/`CMP EAX, short[image+6]-1`; stock constructor confirmed to zero it — ROOT_CAUSE: OMISSION) | Conditional (inactive for stock TIBTRE) |
| `TerrainTypeClass` | `+0x2A0` | int | `AnimationRate` | `TerrainTypeClass::ReadINI_Full @ 0x0071DEA0`; stock TIBTRE value 3 | Yes |
| `TerrainTypeClass` | `+0x2A4` | float | `AnimationProbability` | `ReadINI_Full @ 0x0071DEA0`; stock TIBTRE value `.003` | Yes |
| `TerrainTypeClass` | `+0x2B1` | bool | `SpawnsTiberium` | midpoint gate in AI; parsed from INI | Yes |
| `TerrainTypeClass` | `+0x2B3` | bool | `IsAnimated` | probability and midpoint gates in AI; parsed from INI | Yes |
| `TerrainTypeClass` | `+0xA4` | ptr | loaded image/SHP data pointer | vtable slot `+0x9C` points to tiny getter at `0x0041CFA0`, raw bytes return `[ecx+0xA4]` | Yes |

## 3. Core Logic

1. `TerrainClass::AI @ 0x0071C730` calls `ObjectClass::AI` first. Active in YR: Yes. Evidence: call at `0x0071C738`.
2. Probability RNG is gated by `type.IsAnimated != 0` and `this+0xC0 == 0`. Active in YR: Yes for stock TIBTRE because `IsAnimated=yes`; ongoing animation consumes no new probability RNG. Evidence: `0x0071C745..0x0071C755`.
3. The probability draw uses raw `Random::Next @ 0x0065C780`, not `RandomRanged`. Active in YR: Yes. Evidence: call at `0x0071C761`; `Random::Next` decompile shows one generator step and no range rejection loop.
4. The raw random value is converted with signed absolute-value logic, divided by `1_000_000`, and the signed remainder is multiplied by the double constant at `0x007EF918` (`1.0e-6`). Active in YR: Yes. Evidence: assembly bytes at `0x0071C761..0x0071C785` include `CDQ`, `XOR`, `SUB`, signed `IDIV 0xF4240`, store of `EDX`, `FILD`, and `FMUL [0x007EF918]`.
5. The comparison is floating point and strict: `(remainder * 1.0e-6) < stored_float(AnimationProbability)`. Active in YR: Yes. Evidence: decompile at `0x0071C730`; raw x87 compare sequence at `0x0071C785..0x0071C79C`; parser casts `CCINIClass::ReadDouble` to float at `0x0071E04C..0x0071E073`.
6. On success, AI starts the animation but does not advance the frame or spawn ore on the same tick. Active in YR: Yes. Evidence: success writes current frame `+0xAC = 0`, start frame `+0xB4 = g_CurrentFrameCounter`, duration `+0xBC = type.AnimationRate`, active mirror `+0xC0 = type.AnimationRate` at `0x0071C79E..0x0071C7BF`; `CDTimerClass::GetTimeRemaining @ 0x00426630` sees elapsed `0 < duration` and returns nonzero for stock rate 3.
7. `CDTimerClass::GetTimeRemaining @ 0x00426630` expires only when `g_CurrentFrameCounter - start >= duration`; while elapsed is smaller, it returns remaining frames. Active in YR: Yes. Evidence: decompile of `0x00426630`; caller from `TerrainClass::AI` at `0x0071C7CA`.
8. On an expiry tick with `+0xC0 != 0`, AI sets redraw dirty, increments current frame by `+0xC4`, rearms the timer from the current frame counter, and keeps duration equal to `+0xC0`. Active in YR: Yes. Evidence: `0x0071C7D4..0x0071C7FE`; constructor `0x0071BB90` sets `+0xC4 = 1`.
8a. **After the frame advance, AI checks `+0xCD` (byte).** If nonzero, it reads the SHP frame count via `type->vtable[+0x9C]` and compares `current_frame == frame_count - 1`; if true, it calls `this->vtable[+0xF8]()` and returns early without reaching the midpoint or spawn check. For stock TIBTRE the constructor always initialises `+0xCD = 0`, so this branch never fires. Active in YR: Conditional (inactive for stock TIBTRE). Evidence: `CMP byte [ESI+0xCD]` check at ~`0x0071C7FE`; constructor `0x0071BB90` at `*(undefined1*)((int)param_1 + 0xCD) = 0`. (corrected 2026-05-28: was absent; binary shows the wrap-early-return branch via live decompile `0x0071C730` — ROOT_CAUSE: OMISSION)
9. TIBTRE spawn is gated again by `SpawnsTiberium != 0` and `IsAnimated != 0`. Active in YR: Yes for stock TIBTRE. Evidence: AI checks type `+0x2B1` then `+0x2B3` at `0x0071C84D..0x0071C861`.
10. The total frame count comes from loaded image data, not from `AnimationRate`: AI calls `type->vtable[+0x9C]`, then reads signed word `[image_data + 6]`. Active in YR: Yes. Evidence: AI image getter call and signed load at `0x0071C863..0x0071C871`; `TerrainTypeClass` constructor `0x0071DA80` installs vtable `0x007F5458` (corrected 2026-05-28: was `0x0071DAE9`, which is mid-body of the constructor, not the vtable-install point; constructor starts at `0x0071DA80` per `get_function_by_address` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT), whose slot `+0x9C` is `0x0041CFA0`; raw bytes at `0x0041CFA0` are `MOV EAX,[ECX+0xA4]; RET`.
11. The midpoint comparison uses signed integer division by 2 after loading the signed 16-bit frame count: `current_frame == frame_count / 2`. Active in YR: Yes. Evidence: `0x0071C871..0x0071C87C` uses the signed divide-by-two sequence before comparing with current frame.
12. On the midpoint tick, AI resets current frame and active duration to zero before calling `CellClass::SpreadTiberium(1)`. Active in YR: Yes. Evidence: reset starts at `0x0071C87E`; `CellClass::Get_Cell_At` is called at `0x0071C8C9`; `CellClass::SpreadTiberium(1)` is called at `0x0071C8D0`.
13. A failed idle probability roll consumes exactly one raw `Random::Next` and no direction RNG. Active in YR: Yes. Evidence: only probability call before the failed branch exits to the timer check; `CellClass::SpreadTiberium` is unreachable unless midpoint branch fires.
14. An ongoing animation consumes no probability RNG; direction RNG is delayed until the future midpoint call inside `SpreadTiberium`. Active in YR: Yes. Evidence: idle gate `+0xC0 == 0` precedes the probability call, while `SpreadTiberium(1)` is only called after midpoint reset at `0x0071C8C5`.
15. Edge case: if `AnimationRate` is zero, a successful probability branch writes `+0xC0 = 0`, so the object remains effectively idle and can roll again next tick; stock TIBTRE uses rate 3, so this is not the normal YR path. Active in YR: Conditional. Evidence: `AnimationRate` parsed to `+0x2A0`; AI copies it to `+0xC0`; stock `rulesmd.ini` TIBTRE value is 3.

## 4. INI Keys

| Key | Stock YR value | Binary read/use | Effect | Active in YR |
|---|---|---|---|---|
| `[TIBTRE01/02/03] IsAnimated` | `yes` | `ReadINI_Full @ 0x0071E022..0x0071E041`; AI probability gate `0x0071C745`, midpoint gate `0x0071C84D..0x0071C861` | Enables probability roll and midpoint spawn gate | Yes |
| `[TIBTRE01/02/03] AnimationRate` | `3` | `ReadINI_Full @ 0x0071E032..0x0071E057`; AI start `0x0071C7A7` | CDTimer duration between frame increments | Yes |
| `[TIBTRE01/02/03] AnimationProbability` | `.003` | `ReadINI_Full @ 0x0071E04C..0x0071E073`; AI compare `0x0071C785..0x0071C79C` | Idle per-tick animation-start probability | Yes |
| `[TIBTRE01/02/03] SpawnsTiberium` | `yes` | `ReadINI_Full @ 0x0071DF32`; midpoint gate `0x0071C84D..0x0071C861` | Allows midpoint `SpreadTiberium(1)` | Yes |

## 5. Integration Points

`TerrainClass::AI` is installed in the `TerrainClass` vtable; Ghidra xref to `0x0071C730` is the data vtable entry at `0x007F5288`. The object instance constructor `0x0071BB90` initializes the animation fields when terrain objects are created from the map loader `TerrainClass::Read_Map_Section @ 0x0071CA70`.

The standard-YR path is live because stock maps can instantiate `TIBTRE01..03`, stock rules mark those types as animated and spawning, and `TerrainClass::AI` contains no TS-only or `SpecialFlags` gate around the timing path. The `TiberiumSpreads` flag is outside this slice and belongs to the `SpreadTiberium` force-flag investigation.

## 6. Current Rust Implementation Status

As of the 2026-05-24 TIBTRE implementation pass, current Rust has the verified two-phase timing model in `src/sim/terrain_spawn.rs`:

| Rust surface | Current behavior | Remaining note |
|---|---|---|
| `TerrainSpawnerState` | stores type, native-shaped probability, animation rate, loaded frame count, midpoint frame, and idle/active phase | no timing-state mismatch known in this slice |
| `tick_terrain_spawners_stateful` | rolls raw-shaped probability only while idle, starts active animation on a hit, suppresses active rolls, and calls placement only at midpoint | placement internals are owned by the `SpreadTiberium` / `PlaceTiberium` docs |
| file header | documents the two-phase model | keep this contract if terrain spawning is refactored |
| seeding | accepts `terrain_frame_counts` from app/render-side asset loading while keeping `sim/` free of asset/render dependencies | stock frame-count handoff is implemented; modded asset coverage depends on atlas loading |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TerrainClass::AI` idle probability gate | verified | `0x0071C745..0x0071C79C` | none |
| raw RNG vs ranged RNG | verified | `0x0071C761` call to `0x0065C780`; `RandomRanged @ 0x0065C7E0` not called in AI | none |
| signed abs/modulo-million transform | verified | raw bytes `0x0071C761..0x0071C785` | rare `INT_MIN` reachability in RNG stream not proven |
| x87 strict float comparison | verified | `0x0071C785..0x0071C79C`; parser `0x0071E04C..0x0071E073` | none |
| animation start writes | verified | `0x0071C79E..0x0071C7BF` | none |
| CDTimer expiration boundary | verified | `CDTimerClass::GetTimeRemaining @ 0x00426630` | none |
| frame increment and rearm | verified | `0x0071C7D4..0x0071C7FE`; constructor `0x0071BB90` | none |
| `+0xCD` wrap-early-return branch | verified (inactive for stock TIBTRE) | live decompile `0x0071C730` shows `CMP byte [ESI+0xCD]` gate followed by `frame == frame_count-1` check; constructor `0x0071BB90` zeroes `+0xCD` | documented in corrected step 8a; Rust impact: none for stock TIBTRE, but full frame-wrap path not implemented |
| midpoint frame count source | verified | `0x0071C863..0x0071C87C`; vtable slot evidence `0x007F5458+0x9C -> 0x0041CFA0` | stock retail TIBTRE SHP frame counts confirmed as 22 in the retail-SHP report |
| midpoint reset and `SpreadTiberium(1)` call | verified | reset `0x0071C87E`; cell lookup `0x0071C8C9`; spread call `0x0071C8D0` | placement internals out-of-scope |
| current Rust two-phase model | verified | `src/sim/terrain_spawn.rs` `TerrainSpawnerState`, `tick_terrain_spawners_stateful`, `seed_terrain_spawners` | placement side effects still owned by later docs |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What is the entry point for TIBTRE timing? -> TerrainClass::AI @ 0x0071C730` (evidence: prior reports plus Ghidra decompile; vtable xref `0x007F5288`)
- `[RESOLVED] OQ-2 - Is the path live in YR? -> Yes for stock TIBTRE01/02/03 because rulesmd sets SpawnsTiberium/IsAnimated/AnimationRate/AnimationProbability and AI has no TS-only gate` (evidence: `rulesmd.ini`; `0x0071C730`; `0x0071DEA0`)
- `[RESOLVED] OQ-3 - Does a failed idle roll consume RNG? -> Yes, exactly one raw Random::Next` (evidence: `0x0071C761`; no later spawn branch)
- `[RESOLVED] OQ-4 - Does an ongoing animation consume RNG? -> No probability RNG while +0xC0 != 0` (evidence: `0x0071C745..0x0071C755`)
- `[RESOLVED] OQ-5 - Is the probability denominator inclusive/exclusive integer threshold? -> It is not an integer threshold in binary; it is signed remainder modulo 1,000,000 times double 1e-6, strict x87 less-than versus stored float probability` (evidence: `0x0071C761..0x0071C79C`, `0x007EF918`, `0x0071E04C..0x0071E073`)
- `[RESOLVED] OQ-6 - What starts the timer? -> A successful probability roll writes current frame 0, start frame current tick, duration=AnimationRate, active mirror=AnimationRate` (evidence: `0x0071C79E..0x0071C7BF`)
- `[RESOLVED] OQ-7 - Can start and midpoint happen in the same stock tick? -> No for stock rate 3 because elapsed 0 returns remaining 3` (evidence: `0x00426630`; stock `AnimationRate=3`)
- `[RESOLVED] OQ-8 - How does CDTimer expire? -> expired when elapsed >= duration` (evidence: `0x00426630`)
- `[RESOLVED] OQ-9 - What advances current frame? -> each expiry adds constructor-initialized +0xC4, value 1` (evidence: `0x0071C7D4..0x0071C7DF`; `0x0071BB90`)
- `[RESOLVED] OQ-10 - What is the midpoint comparison? -> current_frame == signed_word(image_data+6) / 2` (evidence: `0x0071C863..0x0071C87C`)
- `[RESOLVED] OQ-11 - Where does frame count come from? -> loaded image data pointer returned by TerrainType vtable slot +0x9C, getter returns +0xA4` (evidence: vtable `0x007F5458`, slot `+0x9C`, bytes at `0x0041CFA0`)
- `[RESOLVED] OQ-12 - What reset occurs after spawn? -> current frame and active mirror reset to 0; timer start set to current tick; duration set to 0 before calling SpreadTiberium(1)` (evidence: reset `0x0071C87E`; spread call `0x0071C8D0`)
- `[RESOLVED] OQ-13 - Does current Rust match the timing/RNG model? -> Current Rust now has a stateful two-phase model: raw-shaped probability while idle, active-roll suppression, frame-count midpoint, and no same-tick spawn. Placement side effects remain covered by separate docs.` (evidence: `src/sim/terrain_spawn.rs`)
- `[RESOLVED] OQ-14 - What are exact stock TIBTRE01/02/03 SHP frame counts? -> All checked standard-theater retail variants are 22 frames, so the midpoint target is frame 11.` (evidence: `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-15 - Can Random::Next produce signed INT_MIN in normal YR RNG streams?` (category: `needs-runtime-debugger`; reason: static code proves the signed edge behavior if the value occurs, not generator reachability under stock seeds; next-step-if-pursued: brute-force or runtime-log the RNG stream)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful probability starts animation; ore spawns only later when `current_frame == frame_count / 2` | `0x0071C79E..0x0071C8D0`; `0x00426630`; `0x0041CFA0` | implemented in current `TerrainSpawnerState` / `tick_terrain_spawners_stateful` | `src/sim/terrain_spawn.rs`, app-side frame-count handoff | preserve per-spawner animation state using `AnimationRate`, current frame, active timer, and SHP frame count | with probability forced to always hit, tick 0 arms animation, no ore appears until midpoint expiry, then exactly one `SpreadTiberium` attempt occurs | regression test `tibtre_probability_hit_delays_spawn_until_animation_midpoint`; risk: preserving average rate still breaks visible timing and RNG sequence |
| Probability RNG is raw `Random::Next` while idle only; direction RNG waits until midpoint | `0x0071C745..0x0071C761`; `0x0071C8D0`; `Random::Next @ 0x0065C780` | implemented for terrain-spawner probability/active suppression; placement RNG remains in the placement slice | `src/sim/rng.rs` consumer API and `tick_terrain_spawners_stateful` | consume one raw probability RNG only when idle; suppress new rolls during active animation; perform placement RNG only on midpoint tick | deterministic RNG trace with one TIBTRE and a hit: GameMD-compatible sequence is probability draw on tick H, no terrain RNG on ticks H+1..midpoint-1, direction/placement RNG at midpoint | regression test `tibtre_rng_sequence_suppresses_rolls_while_animation_active`; risk: using `RandomRanged(0,999999)` for probability changes RNG stream |
| Probability compare is float strict-less against parsed float, not `roll < micros` | `0x0071C761..0x0071C79C`; constant `0x007EF918`; parser `0x0071E04C..0x0071E073` | implemented as a raw-sample helper with strict comparison against cached micros-scaled probability | `TerrainSpawnerState` probability representation and comparison helper | preserve signed modulo-million and strict floating comparison behavior | stock `.003` should accept samples less than the parsed-float probability and reject equality | regression test `tibtre_probability_uses_float_strict_less_boundary`; risk: integer-only thresholds silently shift boundary behavior |

### Negative Facts / Do Not Do

- Do not collapse midpoint spawning into same-tick placement. Evidence: AI only calls `SpreadTiberium(1)` after CDTimer expiry and midpoint comparison at `0x0071C863..0x0071C8D0`.
- Do not keep rolling probability while the animation is active. Evidence: probability call is behind `+0xC0 == 0` at `0x0071C745..0x0071C761`.
- Do not use `RandomRanged(0,999999)` for the probability roll if deterministic parity is the target. Evidence: AI calls raw `Random::Next @ 0x0065C780`; ranged helper `0x0065C7E0` has rejection-loop behavior and is not called here.
- Do not use `AnimationRate` as the total animation frame count. Evidence: `AnimationRate` is the CDTimer duration; midpoint uses signed word `[image_data + 6]` from loaded image data via vtable slot `+0x9C`.
- Do not add AnimClass `TiberiumSpawnType` or `TiberiumSpreadRadius` to TIBTRE terrain spawning. Evidence: TIBTRE path is `TerrainClass::AI @ 0x0071C730 -> CellClass::SpreadTiberium @ 0x00483780`; AnimClass bouncer path is separate prior work.

### Stale Docs / Follow-up Docs

- Previous stale-source-comment guidance for `src/sim/terrain_spawn.rs` has been applied by the 2026-05-24 TIBTRE implementation pass: the Rust header now documents the two-phase model.
- This report has been refreshed with the retail frame-count result from `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`.

Type/source wording conflicts in older terrain docs belong to the placement/type-selection reports, not this timing/RNG slice.

## Sources

- Ghidra decompile/read-memory: `TerrainClass::AI @ 0x0071C730`, raw bytes `0x0071C730..0x0071C91F`
- Ghidra decompile: `CDTimerClass::GetTimeRemaining @ 0x00426630`
- Ghidra decompile: `Random::Next @ 0x0065C780`, `RandomRanged @ 0x0065C7E0`
- Ghidra decompile/read-memory: `TerrainTypeClass::ReadINI_Full @ 0x0071DEA0`, constructor `0x0071DA80`, instance constructor `0x0071BB90`
- Ghidra read-memory: `0x007EF918` double constant `1.0e-6`; `0x0041CFA0` image pointer getter
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/terrain_spawn.rs`
