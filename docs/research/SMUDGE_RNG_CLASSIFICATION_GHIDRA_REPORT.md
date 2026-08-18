# Smudge RNG Classification - Ghidra Research Report

**Address(es):** `0x004415F0` (`BuildingClass::DestructionEffects`), `0x00442D90` (`BuildingClass::SpawnSurvivors`), `0x00424F00` (`AnimClass::Start`), `0x0049F420` (random offset helper), `0x006B59A0` (`SpawnDebris`), `0x006B5C90` (`Debris_Smoke`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** RNG bounds, consumed draws, and call order for current Rust `src/sim/combat/smudge_dispatch.rs` functions `rng_below_half_normalized`, `try_dispatch_building_destruction_smudges`, `try_dispatch_building_survivor_smudges`, and `random_offset_at_radius`.  
**Non-Scope:** full smudge placement/rendering, full `SmudgeTypeClass::CanPlaceHere`, full infantry survivor/ejection parity, and all animation smudge trigger semantics beyond the shared 50/50 RNG helper.  
**Confidence:** High for the smudge RNG call shapes directly decompiled; Medium for Rust integration relative to non-smudge survivor RNG because this slot did not re-investigate all building death survivor paths.  
**Active in YR:** Yes. The checked paths are called from live building destruction and animation start paths in standard YR.

## 0. Working Notes Gate

Target question: Does current Rust consume the same RNG calls, ranges, and order as gamemd/YR for building destruction/survivor smudges and the shared scorch/crater 50/50 helper?

Non-goals: Do not investigate all smudge rendering/type placement, all survivor infantry spawning, or all combat death ordering outside the smudge dispatch surfaces.

Evidence needed to mark COMPLETE: Ghidra decompile plus assembly/xref context for the live binary call sites, focused Rust scan of the named functions, and implementation handoff with concrete Rust test names.

Stop conditions: Stop after classifying the named RNG surfaces; record broader survivor/passability integration questions as remaining uncertainty rather than expanding scope.

## 1. Overview

The building-center smudge path in current Rust is effectively GREEN after the `SimRng` parity rewrite: it uses `RandomRanged(0, W-2)` / `RandomRanged(0, H-2)` discard-equivalent calls, then `RandomRanged(0,99) < 50`. The apparently unconditional discarded calls do not consume RNG for `2x2` because gamemd equal-bound semantics are now no-draw.

The building-survivor smudge path is GREEN for RNG order after a passability gate: gamemd rolls `RandomRanged(0,99)`, branches on `< 50`, then calls `FUN_0049F420(0x80, 0)` in either branch. Rust rolls `next_range_u32(100)`, then calls `random_offset_at_radius`, then branches; because both branches call the offset helper, this preserves the same draw order for the smudge path.

The shared anim scorch/crater helper `rng_below_half_normalized` is RED. Gamemd does not test a raw RNG high bit. It calls `RandomRanged(0, 0x7FFFFFFE)`, rejects a masked `0x7FFFFFFF`, and checks `(result * 2^-31) < 0.5`, equivalent to `result < 0x40000000`.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass* 0x00A8B230 + 0x218` | Scenario-owned Random object used by the checked `RandomRanged` callers | assembly at `0x0042507A..0x0042508D`, `0x004432B0..0x004432BF`, Random report | Yes |
| `AnimType + 0x36B` | `Scorch` flag | decompile `0x00424F00`, assembly `0x00425060..0x00425078` | Yes |
| `AnimType + 0x36D` | `Crater` flag | decompile `0x00424F00`, assembly `0x00425070..0x00425078` | Yes |
| `AnimType + 0x36E` | `ForceBigCraters` flag | decompile `0x00424F00` | Yes |
| `SmudgeType + 0x2A1` | `Burn` selector for `SpawnDebris` | decompile `0x006B59A0` | Yes |
| `SmudgeType + 0x2A0` | `Crater` selector for `Debris_Smoke` | decompile `0x006B5C90` | Yes |
| `SmudgeType + 0x298/+0x29C` | footprint width/height in cells | decompile `0x006B59A0`, `0x006B5C90` | Yes |

## 3. Core Logic

### 3.1 Anim scorch/crater 50/50 helper

Verified binary behavior:

1. `AnimClass::Start @ 0x00424F00` checks altitude `< 0x1E`.
2. If `Scorch` is true and `Crater` is false, it calls `SpawnDebris` and returns.
3. If both `Scorch` and `Crater` are true, it calls `RandomRanged(0, 0x7FFFFFFE)`.
4. It multiplies the ranged result by double `0x007E3570` (`1/2^31`) and compares against double `0x007E1738` (`0.5`).
5. The scorch branch is taken only when `result < 0x40000000`; otherwise the crater branch runs.

Evidence: decompile `0x00424F00`; assembly `0x0042507A..0x004250A6` shows `PUSH 0x7ffffffe`, `PUSH 0`, `CALL 0x0065c7e0`, `FMUL [0x007e3570]`, `FCOMP [0x007e1738]`. Active in YR: Yes.

Current Rust delta: `rng_below_half_normalized` at `src/sim/combat/smudge_dispatch.rs:175` uses `rng.next_u32() < 0x80000000`. This has the same nominal probability but not the same accepted bit, not the same rare retry behavior, and not the same deterministic branch sequence for a given RNG stream.

### 3.2 Building center destruction smudge

Verified binary behavior:

1. `BuildingClass::DestructionEffects @ 0x004415F0` enters the smudge block only if foundation width `> 1` and height `> 1`.
2. If width `> 2`, it calls `RandomRanged(0, W - 2)` and discards the result.
3. If height `> 2`, it calls `RandomRanged(0, H - 2)` and discards the result.
4. It calls `RandomRanged(0, 99)`.
5. If roll `< 0x32`, it calls `SpawnDebris(coord, 100, 100, forceBig=1)`.
6. Otherwise it calls `Debris_Smoke(coord, 100, 100, forceBig=1)`.

Evidence: decompile `0x004415F0`; assembly `0x004417E4..0x00441819` for width discard and roll, `0x0044181E..0x00441888` for `< 50` branch to `SpawnDebris`, `0x00441888..0x004418E7` for crater branch. Active in YR: Yes.

Current Rust delta: `try_dispatch_building_destruction_smudges` uses `foundation_w < 2 || foundation_h < 2` early return, then `next_range_u32(foundation_w - 1)`, `next_range_u32(foundation_h - 1)`, and `next_range_u32(100) < 50`. With the settled `next_range_u32(n) = RandomRanged(0, n - 1)` and equal-bound no-draw behavior, this matches binary draw counts for `2x2` and larger foundations. The Rust comment/test wording still says `2x2` advances by three calls, which is misleading; the equal-bound calls do not advance RNG.

### 3.3 Building survivor smudge per foundation cell

Verified binary behavior:

1. `BuildingClass::SpawnSurvivors @ 0x00442D90` iterates foundation cells.
2. For the smudge subsection, it first calls `CellClass::CheckCellPassability`.
3. If passability fails, no smudge RNG is consumed for that cell.
4. If passability passes, it calls `RandomRanged(0, 99)`.
5. If roll `< 0x32`, it sets base cell center, calls `FUN_0049F420(0x80, 0)`, snaps the returned offset coord back to a cell center, then calls `SpawnDebris(100, 0)`.
6. Otherwise it does the same offset/snap flow and calls `Debris_Smoke(100, 0)`.

Evidence: decompile `0x00442D90`; assembly `0x0044329B..0x004432BF` for passability then `RandomRanged(0,99)`, `0x004432C4..0x00443358` for `<50` scorch path and offset call, `0x00443362..0x004433EF` for crater path and offset call. Active in YR: Yes.

Current Rust delta: `try_dispatch_building_survivor_smudges` uses `path_grid.is_walkable` as the gate, then `next_range_u32(100)`, then `random_offset_at_radius(0x80)`, then branch. The RNG order after the gate matches because both binary branches call `FUN_0049F420`; however, Rust gate equivalence to `CellClass::CheckCellPassability` was not proven in this slot. Any gate mismatch changes which cells consume the roll+offset pair.

### 3.4 Random offset helper

Verified binary behavior:

1. `FUN_0049F420(0x80, 0)` calls `Random__Next()` once and uses the low byte.
2. It derives an angle from `((byte << 8) as signed short) - 0x3FFF`.
3. It uses that angle to compute a fixed-magnitude offset, keeps Z unchanged, and only snaps to cell center if `flag != 0`.
4. Building survivor smudges pass `flag=0`; the caller later converts offset coords back to cell centers.

Evidence: decompile `0x0049F420`; caller assembly `0x004432ED..0x00443302` and `0x00443382..0x0044339A` show `PUSH 0x80`, `PUSH 0`, `CALL 0x0049f420`. Active in YR: Yes.

Current Rust delta: `random_offset_at_radius` consumes exactly one `next_u32()` and uses the low byte. This matches the draw count. The fixed-point table is a Rust approximation of the binary's trig/ftol helper; this slot did not prove every byte's `(dx,dy)` table bit-exact.

## 4. INI Keys

| INI key | Default | RNG effect | Evidence | Active in YR |
|---|---:|---|---|---|
| `[AnimType] Scorch` | false | Enables scorch path; when paired with `Crater`, gates the `RandomRanged(0,0x7FFFFFFE)` 50/50 call | `0x00424F00`, `artmd.ini` | Yes |
| `[AnimType] Crater` | false | Enables crater path and pairs with `Scorch` for 50/50 call | `0x00424F00`, `artmd.ini` | Yes |
| `[AnimType] ForceBigCraters` | false | No extra RNG; changes crater parameters to 300/300/true | `0x00424F00`, `artmd.ini` | Yes |
| `[SmudgeType] Burn` | false | Makes type eligible for later random pick inside `SpawnDebris` | `0x006B59A0`, `rulesmd.ini` | Yes |
| `[SmudgeType] Crater` | false | Makes type eligible for later random pick inside `Debris_Smoke` | `0x006B5C90`, `rulesmd.ini` | Yes |
| `[SmudgeType] Width/Height` | 1 | Filters candidate lists; if candidate list non-empty, later random pick uses `RandomRanged(0,count-1)` | `0x006B59A0`, `0x006B5C90` | Yes |

## 5. Integration Points

| Integration | Binary evidence | Current Rust surface | Status |
|---|---|---|---|
| Building death center smudge before survivor smudges | `0x004415F0` center smudge block precedes final `BuildingClass__SpawnSurvivors(param_1)` | `src/sim/combat/mod.rs:928..952` emits `BuildingCenter` before `BuildingSurvivor` events | Verified for event order |
| Center smudge draw sequence | `0x004417E4..0x00441819` | `src/sim/combat/smudge_dispatch.rs:199..206` | GREEN after parity RNG rewrite |
| Survivor smudge draw sequence after passability | `0x0044329B..0x004433EF` | `src/sim/combat/smudge_dispatch.rs:259..304` | GREEN for RNG sequence, YELLOW for passability gate equivalence |
| Anim both-scorch-and-crater 50/50 | `0x0042507A..0x004250A6` | `src/sim/combat/smudge_dispatch.rs:172..176` | RED |
| Candidate smudge type pick inside `try_place` equivalent | `0x006B59A0`, `0x006B5C90` random-pick candidate lists | `src/sim/smudge_grid.rs` not re-scanned in this slot | Deferred |

## 6. Current Rust Implementation Status

| Rust function | Classification | Evidence | Notes |
|---|---|---|---|
| `rng_below_half_normalized` | RED | Rust `src/sim/combat/smudge_dispatch.rs:175`; binary `0x0042507A..0x004250A6` | Must use ranged helper result `< 0x40000000`, not raw `next_u32() < 0x80000000`. |
| `try_dispatch_building_destruction_smudges` | GREEN for RNG behavior | Rust `src/sim/combat/smudge_dispatch.rs:199..206`; binary `0x004417E4..0x00441819` | Actual state matches because equal-bound calls consume no draw; comment/test wording is misleading for `2x2`. |
| `try_dispatch_building_survivor_smudges` | GREEN/YELLOW | Rust `src/sim/combat/smudge_dispatch.rs:259..304`; binary `0x0044329B..0x004433EF` | RNG order after gate matches; gate equivalence to binary `CheckCellPassability` remains unproven. |
| `random_offset_at_radius` | GREEN for draw count, YELLOW for exact offset table | Rust `src/sim/combat/smudge_dispatch.rs:37..42`; binary `0x0049F420` | One raw draw/low byte matches; trig rounding bit-exactness not exhausted. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AnimClass::Start` both `Scorch`+`Crater` RNG | verified | `0x00424F00`, `0x0042507A..0x004250A6` | Patch Rust helper. |
| `BuildingClass::DestructionEffects` center smudge RNG | verified | `0x004415F0`, `0x004417E4..0x004418E7` | None for RNG; comment/test cleanup recommended. |
| `BuildingClass::SpawnSurvivors` smudge RNG | verified | `0x00442D90`, `0x0044329B..0x004433EF` | Verify Rust passability gate equivalence separately. |
| `FUN_0049F420` draw count | verified | `0x0049F420`, call sites `0x00443302`, `0x0044339A` | Bit-exact offset table can be separately fixture-tested. |
| `SpawnDebris` / `Debris_Smoke` internal candidate random pick | touched-not-exhausted | `0x006B59A0`, `0x006B5C90` | This slot did not compare `SmudgeGrid::try_place` RNG count exhaustively. |
| Survivor infantry RNG interleaving | deferred | `0x00442D90` shows additional survivor RNG before smudge subsection | Requires focused survivor/ejection RNG integration audit. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the anim 50/50 helper raw high-bit or RandomRanged normalized? -> RandomRanged(0,0x7FFFFFFE) normalized by 1/2^31 and compared to 0.5.` (evidence: `0x0042507A..0x004250A6`)
- `[RESOLVED] OQ-2 - Does building center smudge always consume two discarded draws? -> No. Discarded draws are guarded by W>2 and H>2; Rust equal-bound no-draw makes current calls equivalent for W/H==2.` (evidence: `0x004417E4..0x00441805`, `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-3 - What is the building center roll range and threshold? -> RandomRanged(0,99), scorch when roll < 50, crater otherwise.` (evidence: `0x00441810..0x00441821`)
- `[RESOLVED] OQ-4 - Does survivor smudge consume offset before or after the 50/50 roll? -> After the roll; both branches call the offset helper, so roll then offset is invariant.` (evidence: `0x004432BF..0x00443358`, `0x00443362..0x004433EF`)
- `[RESOLVED] OQ-5 - Does survivor smudge consume RNG when passability fails? -> No smudge roll/offset after failed CheckCellPassability.` (evidence: decompile `0x00442D90`, call at `0x004432A3` before `0x004432BF`)
- `[RESOLVED] OQ-6 - How many RNG draws does the offset helper consume? -> One raw `Random__Next()`; low byte drives angle.` (evidence: `0x0049F420`)
- `[RESOLVED] OQ-7 - Is the building death center event before survivor smudge events in Rust and binary? -> Yes, for smudge event ordering.` (evidence: binary `0x004415F0` calls `BuildingClass__SpawnSurvivors` near end; Rust `src/sim/combat/mod.rs:928..952`)
- `[DEFERRED] OQ-8 - Is Rust `path_grid.is_walkable` exactly equivalent to binary `CellClass::CheckCellPassability` for survivor smudge gating?` (category: requires-different-system-context; reason: gate implementation lives outside this RNG-focused slice; next-step-if-pursued: trace `0x004834A0` and compare to `PathGrid::is_walkable`.)
- `[DEFERRED] OQ-9 - Does Rust interleave crewed survivor infantry RNG with smudge RNG exactly as `BuildingClass::SpawnSurvivors`?` (category: requires-different-system-context; reason: this slot was scoped to smudge_dispatch functions, not full survivor/ejection paths; next-step-if-pursued: run a survivor/ejection RNG integration audit.)
- `[DEFERRED] OQ-10 - Is the Rust fixed-point offset table byte-for-byte equal to binary trig/ftol outputs for all 256 bytes?` (category: bounded-cost-too-high; reason: draw count was enough for this classification, but visual scatter exactness needs a 256-vector fixture; next-step-if-pursued: generate binary-derived table or verify each byte against `0x0049F420`.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Both `Scorch=yes` and `Crater=yes` anims call `RandomRanged(0,0x7FFFFFFE)`, reject masked `0x7FFFFFFF`, and take scorch only when result `< 0x40000000`. | `0x0042507A..0x004250A6`; Random report `0x0065C801..0x0065C882` | mismatch | `src/sim/combat/smudge_dispatch.rs::rng_below_half_normalized` | Replace raw high-bit test with the parity ranged helper result/threshold. | A seeded RNG value whose raw high bit differs from the accepted 31-bit threshold chooses the binary branch and preserves retry behavior; proposed test name: `smudge_anim_both_flags_uses_randomranged_31bit_half_gate`. | Do not use `next_u32() < 0x80000000`; same probability is not same branch stream. |
| Building center smudge consumes W-discard only for W>2, H-discard only for H>2, then `RandomRanged(0,99) < 50`. | `0x004417E4..0x00441821` | none observed in actual RNG state after parity helper; misleading comment/test | `src/sim/combat/smudge_dispatch.rs::try_dispatch_building_destruction_smudges` tests/comments | Preserve equal-bound no-draw behavior; add explicit `2x2` and `3x3` RNG-state tests. | `2x2` consumes only the roll, `3x3` consumes W/H discards plus roll; proposed test names: `building_center_smudge_2x2_consumes_only_roll` and `building_center_smudge_3x3_consumes_two_discards_then_roll`. | Do not force unconditional raw draws for `2x2`; binary has no discarded draws when W/H are exactly 2. |
| Survivor smudge path consumes no smudge RNG on failed passability; on pass, it consumes `RandomRanged(0,99)` then one low-byte offset draw, then calls burn/crater placement. | `0x0044329B..0x004433EF`, `0x0049F420` | RNG order matches after gate; gate equivalence unchecked | `src/sim/combat/smudge_dispatch.rs::try_dispatch_building_survivor_smudges`; `PathGrid::is_walkable` | Keep roll-before-offset ordering; verify/adjust passability gate so cell inclusion matches binary. | A passable 2-cell foundation consumes exactly two pairs of roll+offset; an unpassable cell consumes none; proposed test name: `survivor_smudge_rng_skips_failed_passability_and_rolls_before_offset`. | Do not move offset draw before passability or use a gate that admits/rejects different cells. |

### Negative Facts / Do Not Do

- Do not treat `rng_below_half_normalized` as equivalent to a raw high-bit test; binary uses ranged 31-bit normalized output and rejects one value. Evidence: `0x00425080..0x004250A6`, `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`. Active in YR: Yes.
- Do not consume discarded building-center RNG for `2x2` foundations. Evidence: binary guards width/height discards with `2 < W` / `2 < H`; equal-bound no-draw is the only reason current Rust's calls do not desync. Active in YR: Yes.
- Do not skip `FUN_0049F420` offset draws for survivor smudges because final placement snaps to cell center; the draw is still consumed and can shift to a neighboring cell. Evidence: `0x00443302`, `0x0044339A`. Active in YR: Yes.
- Do not roll survivor smudge RNG before passability. Evidence: `CellClass__CheckCellPassability` precedes `RandomRanged(0,99)` at `0x0044329B..0x004432BF`. Active in YR: Yes.
- Do not assume smudge type candidate picking has no RNG; `SpawnDebris` and `Debris_Smoke` both call `RandomRanged(0,count-1)` when they allocate a smudge. Evidence: `0x006B59A0`, `0x006B5C90`. Active in YR: Yes.

### Remaining Uncertainty

- Exact equivalence of Rust `PathGrid::is_walkable` to binary `CellClass::CheckCellPassability` for survivor smudge gating remains unverified.
- Full interleaving between crewed survivor infantry RNG and smudge RNG was not audited in this slot.
- The `random_offset_at_radius` draw count is verified, but all 256 offset vectors were not binary-fixture-tested for exact `ftol` rounding parity.
- `SmudgeGrid::try_place` candidate filtering/random-pick count was touched through `SpawnDebris`/`Debris_Smoke` decompile but not exhaustively reconciled in this RNG call-site slot.

### Stale Docs / Follow-up Docs

- No standalone research-doc replacement required. The earlier `SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md` already states the correct `RandomRanged(0,0x7FFFFFFE)` 50/50 helper. The stale wording is in Rust source comments/tests, not a research doc: `src/sim/combat/smudge_dispatch.rs:172..176` and `:759..798`.

## Sources

- Ghidra `decompile_function 4415F0` - `BuildingClass::DestructionEffects`.
- Ghidra `decompile_function 442D90` - `BuildingClass::SpawnSurvivors`.
- Ghidra `decompile_function 424F00` - `AnimClass::Start`.
- Ghidra `decompile_function 49F420` - random offset helper.
- Ghidra `decompile_function 6B59A0` - `SpawnDebris`.
- Ghidra `decompile_function 6B5C90` - `Debris_Smoke`.
- Ghidra assembly context: `0x0042507A..0x004250A6`, `0x004417E4..0x004418E7`, `0x0044329B..0x004433EF`.
- `docs/research/SMUDGE_SPAWN_TRIGGERS_GHIDRA_REPORT.md`.
- `docs/research/RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`.
- `src/sim/combat/smudge_dispatch.rs`.
- `src/sim/combat/mod.rs`.
- `ini/artmd.ini`.
- `ini/rulesmd.ini`.
