# Random::RandomRanged 0x0065C7E0 - Ghidra Research Report

**Address(es):** `0x0065C7E0` (primary helper), `0x0065C780` (same RNG state's raw next helper), `0x0065C6D0` (state initializer), `0x0065C660` (raw code region for an older 15-bit LCG ranged helper, non-target contrast only — note: no Ghidra function is carved out at this address, see correction note below)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `Random__RandomRanged @ 0x0065C7E0` contract: bound semantics, state ownership visible through representative callers, directly nearby seeding/initialization, signed edge behavior evident from the helper, and Rust deterministic implications.  
**Non-Scope:** exhaustive audit of all 415 direct callers; full scenario seed lifecycle after map/session setup; `FUN_00598030` float/rejection helper beyond noting it is a different RNG consumer already covered by bridge repair reports.  
**Confidence:** High for helper algorithm and representative scenario RNG ownership; Medium for seed lifecycle because only direct nearby setup paths were inspected.  
**Active in YR:** Yes. Representative live gameplay/UI callers load `ScenarioClass` from `0x00A8B230`, pass `Scenario + 0x218` as `this`, and call `0x0065C7E0` for bridge damage, ore/tiberium spawn, infantry random frame start, jumpjet bridge height randomization, mission retry jitter, and skirmish random assignments.

## 1. Overview

`Random__RandomRanged` returns a deterministic integer in the inclusive range between its two arguments, after sorting the two bounds if they are reversed. It draws from a 250-word scenario-owned RNG state, masks to the next power-of-two-minus-one envelope, and rejection-samples until the masked value is not greater than the inclusive span. A verified active caller in `UnitClass::Mission_Harvest` state 2 uses `Scenario+0x218.RandomRanged(0,2)` for the shared mission-delay return reached both when HARV already owns a destination and after a valid far-return destination is assigned.

This is a lockstep-critical helper: many player-visible calls use `(*gScenario + 0x218)`, so changing the generator, rejection behavior, reversed-bound behavior, or "no draw" cases changes later random outcomes even when the current roll's value looks harmless.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Evidence | Active in YR |
|--------|------|---------|----------|--------------|
| `Random + 0x00` | byte flag | If nonzero, raw draws return zero instead of advancing normally. Initializer writes zero after filling the state table. | `0x0065C82E`, `0x0065C833`, `0x0065C770` | Yes, but normal initialized state is zero. |
| `Random + 0x04` | `i32` index A | First circular index into 250-entry state table; incremented and wrapped at `250`. | `0x0065C837`, `0x0065C84D`, `0x0065C857..0x0065C866` | Yes. |
| `Random + 0x08` | `i32` index B | Second circular index into the same table; initializer sets it to `0x67` (`103`); incremented and wrapped at `250`. | `0x0065C6DD`, `0x0065C842`, `0x0065C858..0x0065C875` | Yes. |
| `Random + 0x0C` | `u32[250]` | State table. Raw draw XORs table[A] and table[B], stores result back into table[A], then returns table[A]. | `0x0065C837..0x0065C84B`, `0x0065C853..0x0065C87C` | Yes. |
| object size copied | `0xFD` dwords (`1012` bytes) | Full RNG object copy size used when copying initialized temp state into scenario/global RNG storage. | `0x0052FE2C`, `0x0052FE43`, `0x0052FE7B`, `0x0052FE97` | Yes on the inspected setup path. |

## 3. Core Logic

Material findings, separated from inference:

| Finding | Evidence | Confidence | Active in YR |
|---------|----------|------------|--------------|
| Bounds are inclusive after sorting: equal bounds return that exact value without a draw; reversed bounds are swapped before span computation. | `0x0065C7EB..0x0065C7F9`, return epilogue at `0x0065C885..0x0065C88A` | High | Yes. Representative callers pass ordered ranges; the reversed-bound contract is still in the live helper. |
| The span is `max - min`, not `max - min + 1`; the `+1` is used only to test whether the inclusive count overflows signed 31-bit handling. Accepted samples are `0..span`, then `min` is added. | `0x0065C7FC`, `0x0065C824`, `0x0065C87E..0x0065C886` | High | Yes. |
| The helper does rejection sampling, not modulo. It builds a bitmask covering the highest set bit of `span`, masks the raw RNG value, and retries while the masked value is greater than `span`. | `0x0065C801..0x0065C82B`, `0x0065C87E..0x0065C882` | High | Yes. |
| The high endpoint can be returned. Calls that need `N` choices pass `0, N-1` (for example `0,3`, `0,7`, `0,9`) and bridge damage passes `1, BridgeStrength`. | `0x00424102..0x00424110`, `0x0054C5C4..0x0054C5CE`, `0x0069B921..0x0069B930`, `0x0048A231..0x0048A245` | High | Yes. |
| Raw next for this helper is the 250-entry XOR lag state, not the nearby LCG at `0x0065C640`/`0x0065C660`. (corrected 2026-05-29: `0x0065C640` and `0x0065C660` are raw code *regions*, not Ghidra-carved functions — `get_function_by_address` returns "No function found" for both; the raw bytes via `read_memory 0x0065C640` confirm `0x0065C640` is the 15-bit LCG `state*0x41C64E6D+0x3039`, `SHR 0xA`, `AND 0x7FFF`, and `0x0065C660` is its ranged variant rejecting values >0x7FFF.) | `0x0065C837..0x0065C87C`; contrast `read_memory 0x0065C640` (raw region, not a function) | High | Yes for `0x0065C7E0`; the LCG code is a separate raw region. |
| For `span == 0x7FFFFFFE`, the helper builds mask `0x7FFFFFFF`, rejects only masked value `0x7FFFFFFF`, and therefore returns `0..0x7FFFFFFE` inclusive. | `0x0065C801..0x0065C882`; representative docs/callers use `RandomRanged(0, 0x7FFFFFFE)` for normalized probability gates. | High | Yes where those probability gates are used. |
| For `span == 0x7FFFFFFF`, signed overflow of `span + 1` takes the no-loop branch and returns `min + 0x80000000` without a draw. For spans with the sign bit set, the mask/retry path does not produce a normal range result. | `0x0065C801..0x0065C82B`; x86 signed `jle` and shift-count behavior | High for the edge behavior; no representative normal YR caller found in scope. | Conditional. Active helper behavior, but no scoped caller passes such a span. |
| If `Random + 0x00` is nonzero, the raw draw path sets the sample to zero without advancing, but the ranged helper still applies mask/rejection/add. Normal initializer clears this flag. | `0x0065C82E..0x0065C835`, `0x0065C770` | High | Conditional; normal initialized scenario RNG has flag zero. |

Pseudocode-level contract for ordinary spans (`0 <= max-min <= 0x7FFFFFFE`):

```text
lo = min(arg0, arg1)
hi = max(arg0, arg1)
if lo == hi: return lo, no RNG draw
span = hi - lo
mask = all low bits through highest set bit of span
repeat:
    raw = RandomNext250WordXorLag(this)
    sample = raw & mask
until sample <= span
return lo + sample
```

## 4. INI Keys

No INI keys belong to the helper itself. Representative callers feed it values read elsewhere:

| Key / source | Effect at representative caller | Evidence | Active in YR |
|--------------|---------------------------------|----------|--------------|
| `[CombatDamage] BridgeStrength` | Upper bound in `RandomRanged(1, BridgeStrength)` bridge tile damage gate. | `0x0048A231..0x0048A245`; existing bridge docs cite `rulesmd.ini` default. | Yes. |
| Rules debris / explosion counts | Callers pass `0, count - 1` for list indexing; helper's upper bound must be inclusive. | Existing bridge explosion docs and direct call pattern at `0x0065C7E0` call sites such as `0x00424110` for `0,3`. | Yes. |

## 5. Integration Points

Representative callers verified in this slice:

| Caller | Range | State owner | Behavior using result | Evidence | Active in YR |
|--------|-------|-------------|-----------------------|----------|--------------|
| Bridge tile damage branch | `1, Rules.BridgeStrength` | `(*0x00A8B230) + 0x218` | Strictly compares roll against damage; equality does not pass. | `0x0048A231..0x0048A24E`, `0x0048A28B..0x0048A2A8` | Yes. |
| Ore/tiberium spawn overlay variant | `0, 3` | `(*0x00A8B230) + 0x218` | Adds result to overlay type frame base. | `0x004240FA..0x00424130` | Yes. |
| Jumpjet bridge-height branch | `0, 7` | `(*0x00A8B230) + 0x218` | Uses random direction/value in a bridge-height-related branch. | `0x0054C5BE..0x0054C5CE` | Yes. |
| Infantry random start frame | `0, max(frame_count, 1) - 1` | `(*0x00A8B230) + 0x218` | Seeds `Techno/Infantry + 0xF8` frame counter when random-start flag is set. | `0x0051DA52..0x0051DA84` | Yes. |
| Mission Enter retry delay epilogue | `0, 2` | `(*0x00A8B230) + 0x218` | Adds 0..2 frame jitter to mission delay. | `0x004D947C..0x004D9497` | Yes. |
| HARV `Mission_Harvest` state-2 delay epilogue | `0, 2` | `(*0x00A8B230) + 0x218` | Adds 0..2 jitter to the current mission-control base delay. Both the existing-destination owner jump and the valid far-destination jump reach this epilogue. | `0x0073EB5A..0x0073EB62`, `0x0073EDB5..0x0073EDBB`, `0x0073EF77..0x0073EFA2` | Yes for stock HARV. |
| Skirmish random country/color assignment | `0,9` and `0,7` | `(*0x00A8B230) + 0x218` | Resolves random country/color placeholders, retrying colors for uniqueness. | `0x0069B921..0x0069B930`, `0x0069B949..0x0069B96F`, `0x0069B9F8..0x0069BA23` | Yes in skirmish/session setup. |

Visible initialization / seeding:

| Path | Finding | Evidence | Active in YR |
|------|---------|----------|--------------|
| Scenario constructor | Initializes `this + 0x218` with seed `0`; raw initializer sets index A `0`, index B `0x67`, fills 250 words, then clears flag byte. | `0x006832C8..0x006832D4`, `0x0065C6D0..0x0065C777` | Yes for scenario object construction. |
| Alternate scenario init/reset | Also initializes `this + 0x218` with seed `0`. | `0x00683564..0x0068356C` | Yes/Conditional depending on reset/load path. |
| Startup/setup seed path | (corrected 2026-05-29 via `decompile_function 0x0052FC20`) The whole time-seed branch is gated on `g_GameMode == 0 \|\| g_GameMode == 5`. Inside it the **primary** seed source is `GetSystemTime`: `wMilliseconds`, `wSecond` (and four right-shifts), `wMinute` (and four right-shifts), `wHour`, `wDay`, `wDayOfWeek`, `wMonth`, `wYear` are mixed in via `FUN_00661850`/`FUN_00661770`, then the mixed result is read back from `DAT_00a8ed98`. `GetTickCount` is only the **fallback**, taken when `DAT_00a8ed98 == 0` (it then writes the seed bytes via `FUN_00661c10(&DAT_00a8ed94, 4)` and calls `GetTickCount`). The chosen seed is stored in `0x00A8ED94`, then `FUN_0065c6d0(seed)` initializes a temporary 250-word RNG and `0xFD` dwords are copied into `(g_ScenarioClass_Instance)+0x218`; the initializer is run again and `0xFD` dwords are copied into global `0x00886B88`. (Outside the gate — non-`0`/`5` game modes — it skips the time mixing and just re-runs `FUN_0065c6d0(DAT_00a8ed94)` twice to repopulate both RNG instances from the already-stored seed.) | `decompile_function 0x0052FC20`; import table maps `0x007E1138` to `GetTickCount` | Conditional. Direct setup path exists; full session/map seed policy is beyond this helper slice. |

## 6. Current Rust Implementation Status

(corrected 2026-05-29 via Read of `src/sim/rng.rs`) Current Rust is now at full gamemd parity for the generator; the earlier "does not match" assessment is STALE. `SimRng` implements the 250-word XOR-lag state with `RNG_INDEX_B_SEED = 0x67`, sorted-inclusive rejection-sampled ranged draws, the signed-overflow `lo + 0x8000_0000` branch, full-state hash/serde, and a regression test against a binary-derived vector.

| Rust surface | Current behavior | Binary parity status | Evidence |
|--------------|------------------|----------------------|----------|
| `src/sim/rng.rs:9..21`, `:65..:118` | 250-word XOR-lag state (`RNG_TABLE_LEN = 250`), indices `index_a`/`index_b` with `RNG_INDEX_B_SEED = 0x67`, `disabled` flag short-circuiting to 0; `next_u32` XORs `state[a] ^ state[b]`, stores back into `state[a]`, returns it, then wraps both indices at 250. | Matches binary 250-word XOR-lag draw and initializer index seed. | `src/sim/rng.rs:9..21,65..118`; binary `0x0065C837..0x0065C882`, `0x0065C6DD`. |
| `src/sim/rng.rs:131..153` | `next_range_u32_inclusive(low, high)` sorts bounds, returns the bound without a draw when `lo == hi`, takes the `lo + 0x8000_0000` branch for `span >= 0x7FFF_FFFF`, otherwise masks to `next_power_of_two() - 1` and rejection-samples until `sample <= span`. | Matches binary: equality no-draw, reversed-bound swap, signed-overflow branch, mask-and-reject (not modulo). | `src/sim/rng.rs:131..153`; binary `0x0065C7EB..0x0065C882`. |
| `src/sim/world/mod.rs` (`Simulation` owns a single `SimRng`, seeded by `Simulation::new(seed)`) | One auditable sim-owned stream. | Ownership shape matches the representative `Scenario + 0x218` single-stream model; algorithm/edge cases now match. | Rust scan; representative callers all use `Scenario + 0x218`. |
| `src/sim/rng.rs:57..63` (`hash_state`) + `src/sim/world/world_hash.rs` | Hashes `disabled`, `index_a`, `index_b`, and the full 250-word `state`. | Matches the requirement to hash the complete parity state (indices + flag + 250 words), not just a `u64`. | `src/sim/rng.rs:57..63`. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `Random__RandomRanged @ 0x0065C7E0` ordinary ordered bounds | verified | `0x0065C7E0..0x0065C88A` | none |
| Equal bounds no-draw path | verified | `0x0065C7EB..0x0065C7EF`, `0x0065C889..0x0065C88A` | none |
| Reversed bound swap | verified | `0x0065C7EB..0x0065C7F9` | Search for live reversed callers deferred. |
| Rejection mask and retry loop | verified | `0x0065C801..0x0065C882` | none |
| Raw 250-word next helper semantics | verified | `0x0065C780..0x0065C7D0` and inline duplicate in `0x0065C837..0x0065C87C` | none |
| RNG object initializer | verified | `0x0065C6D0..0x0065C777` | Full lifecycle of all global Random instances deferred. |
| Scenario RNG ownership through representative callers | verified | `0x0048A23F`, `0x00424106`, `0x0054C5C8`, `0x0051DA79`, `0x0069B92A`, `0x0073EF8E..0x0073EF9D` | Exhaustive caller audit deferred by scope. |
| `GetSystemTime`-primary (GetTickCount-fallback) setup seed path | touched-not-exhausted | `decompile_function 0x0052FC20` (corrected 2026-05-29: GetSystemTime is the primary seed source, GetTickCount only fallback when DAT_00a8ed98==0, both gated on g_GameMode==0||5), IAT `0x007E1138` | Full new-game/load/replay seed policy requires a scenario setup investigation. |
| Signed/pathological span behavior | verified for helper | `0x0065C801..0x0065C82B` | No scoped live caller with `span >= 0x7FFFFFFF` found. |
| INI keys owned by helper | verified none | No string/key reads in helper; representative callers push already-loaded values. | Caller-specific INI ownership belongs in caller reports. |
| Rust `SimRng` comparison | verified at parity (corrected 2026-05-29) | `src/sim/rng.rs:9..21,65..153` | Rust now matches the binary generator; the earlier "Actual Rust patch not performed" note is STALE. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What exact bounds contract does 0x0065C7E0 implement? -> Inclusive sorted bounds; equality returns bound without draw; reversed arguments are swapped before drawing.` (evidence: `0x0065C7EB..0x0065C7F9`)
- `[RESOLVED] OQ-2 - Is the upper bound inclusive or exclusive? -> Inclusive; accepted sample can equal `span`, then `min` is added; representative callers pass count-1 for indexed lists.` (evidence: `0x0065C87E..0x0065C886`, `0x00424102..0x00424110`)
- `[RESOLVED] OQ-3 - Is this modulo-based? -> No; it masks and rejects values greater than span.` (evidence: `0x0065C801..0x0065C882`)
- `[RESOLVED] OQ-4 - Which RNG state does the helper advance? -> The `this` Random object: flag byte, two indices, 250-word table.` (evidence: `0x0065C837..0x0065C87C`)
- `[RESOLVED] OQ-5 - Do representative live gameplay callers use a global scenario RNG? -> Yes, they load `0x00A8B230` and use `+0x218`.` (evidence: `0x0048A238..0x0048A245`, `0x0051DA6F..0x0051DA7F`, `0x0069B921..0x0069B930`)
- `[RESOLVED] OQ-6 - Is there visible seed/init evidence nearby? -> Scenario constructor initializes `+0x218` with seed 0; setup path (gated on g_GameMode==0||5) seeds primarily from `GetSystemTime` (wMilliseconds..wYear mixed via FUN_00661850/FUN_00661770), falling back to `GetTickCount` only when DAT_00a8ed98==0, then copies 0xFD dwords into scenario RNG and into global 0x00886B88.` (corrected 2026-05-29 via `decompile_function 0x0052FC20`; evidence: `0x006832C8..0x006832D4`, `decompile_function 0x0052FC20`)
- `[RESOLVED] OQ-7 - What happens when low == high? -> Returns the bound and does not draw.` (evidence: `0x0065C7EB..0x0065C7EF`, `0x0065C889..0x0065C88A`)
- `[RESOLVED] OQ-8 - What happens when low > high? -> The helper swaps bounds and draws over the sorted inclusive range.` (evidence: `0x0065C7EB..0x0065C7F9`)
- `[RESOLVED] OQ-9 - What happens for span `0x7FFFFFFE`? -> Normal rejection range; only masked `0x7FFFFFFF` is retried.` (evidence: `0x0065C801..0x0065C882`)
- `[RESOLVED] OQ-10 - What happens for span `0x7FFFFFFF` or sign-bit spans? -> Helper's signed overflow/negative-span paths do not produce ordinary random range behavior; no representative normal caller in scope uses them.` (evidence: `0x0065C801..0x0065C82B`)
- `[RESOLVED] OQ-11 - Is `0x0065C660` the same helper? -> No, it is a separate 15-bit LCG ranged routine with `%`-like rejection over `0x7FFF`; target `0x0065C7E0` uses the 250-word XOR lag state.` (corrected 2026-05-29 via `get_function_by_address 0x0065C660`/`0x0065C640` (both "No function found") + `read_memory 0x0065C640`: these are raw code regions, not Ghidra-carved functions; the bytes confirm the LCG substance.) (evidence: `read_memory 0x0065C640`, `0x0065C7E0..0x0065C88A`)
- `[RESOLVED] OQ-12 - Does Rust currently match the generator? -> Yes (corrected 2026-05-29): `SimRng` implements the 250-word XOR-lag state, `RNG_INDEX_B_SEED = 0x67`, mask-and-reject ranged draws, the signed-overflow branch, and a regression test asserting the binary-derived seed-1 sequence 0x78B76ED5/0x275D74AE/0xDA63B931. The earlier "No; xorshift64*" answer is STALE.` (evidence: `src/sim/rng.rs:9..21,65..153,209..214`)
- `[RESOLVED] OQ-13 - Does Rust currently match reversed-bound semantics? -> Yes (corrected 2026-05-29): `next_range_u32_inclusive` sorts the bounds and draws over the inclusive range; the earlier "No; returns low" answer is STALE.` (evidence: `src/sim/rng.rs:131..153`, `0x0065C7EB..0x0065C7F9`)
- `[RESOLVED] OQ-16 - Does active stock HARV state 2 consume this RNG after destination ownership/handoff? -> Yes. The non-null destination branch at `0x0073EB62` and valid far-destination branch at `0x0073EDBB` both jump to `0x0073EF77`; the tail calls `RandomRanged(0,2)` on `Scenario+0x218` and adds it to the current mission-control base delay.` (evidence: live `disassemble_bytes` calls for `0x0073EB5A`, `0x0073EDB0`, `0x0073EF77` and `batch_decompile(0x005B3A00,0x0065C7E0)` on 2026-07-25)
- `[DEFERRED] OQ-14 - Which of the 415 direct callers pass reversed or pathological bounds?` (category: `out-of-scope`; reason: user explicitly requested representative callers only; next-step-if-pursued: run a caller-classification investigation over all direct call sites.)
- `[DEFERRED] OQ-15 - Exact replay/skirmish/map seed policy after setup path?` (category: `requires-different-system-context`; reason: helper slice found constructor/GetTickCount setup evidence but not full session serialization/loading policy; next-step-if-pursued: investigate ScenarioClass seed save/load and multiplayer handshake paths.)

### Remaining Uncertainty

- Exact seed lifecycle for saved games, replays, multiplayer sync, and map-load deterministic seed replacement remains outside this helper slice.
- No exhaustive scan was performed for live callers using reversed arguments or spans at/above `0x7FFFFFFF`.
- The normal meaning of `Random + 0x00` beyond "nonzero suppresses raw advancement and yields zero sample" was not traced.

## 9. Implementation Handoff

(corrected 2026-05-29 via Read of `src/sim/rng.rs`: the generator, ranged helper, and shared-stream ownership are now implemented at parity. The "Current Rust delta" rows below were marked "mismatch/partial" pre-parity and are STALE; the acceptance scenarios that have already landed are noted as DONE.)

| Verified behavior | Evidence | Current Rust delta (STALE — see correction note) | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| `RandomRanged` uses a 250-word XOR lag RNG with indices `+0x04`/`+0x08`, lag seed index `0x67`, and full-state copies of `0xFD` dwords. | `0x0065C6D0..0x0065C7D0`, `0x0065C837..0x0065C87C` | DONE (was: mismatch, Rust used xorshift64*): `SimRng` now uses the 250-word XOR-lag state, `RNG_INDEX_B_SEED = 0x67`, full-state serde/hash (`src/sim/rng.rs:9..21,65..118`) | `src/sim/rng.rs`, `src/sim/world/world_hash.rs`, save/replay RNG serialization | Done: gamemd-parity RNG state for sim-visible gameplay; full parity state hashed/serialized. | Met: `test_gamemd_raw_sequence_seed_one` asserts seed-1 yields 0x78B76ED5/0x275D74AE/0xDA63B931 (`src/sim/rng.rs:209..214`). | Do not regress to xorshift modulo for "close enough"; later RNG stream and player-visible variants diverge. |
| Ranged helper sorts bounds, returns equal bounds without draw, uses inclusive high endpoint, and rejection-samples rather than modulo. | `0x0065C7EB..0x0065C882` | DONE (was: mismatch for `high < low` and distribution): `next_range_u32_inclusive` sorts, no-draws on equality, masks to `next_power_of_two()-1` and rejection-samples (`src/sim/rng.rs:131..153`) | `src/sim/rng.rs::next_range_u32_inclusive`, all `next_range_u32` call sites that intend gamemd `RandomRanged(0,n)` | Done: binary-contract ranged helper; remaining audit work is confirming exclusive wrappers pass `0,count-1` correctly at each call site. | Met: `test_inclusive_range_degenerate` asserts `range(7,7)` consumes no draw and `range(7,3)` returns `3..=7` (`src/sim/rng.rs:188..198`). | Do not reintroduce `high <= low -> low`; it only matches equality, not reversed bounds. |
| Representative gameplay callers use the scenario RNG at `Scenario + 0x218`; many visual/gameplay outcomes share one stream. | `0x0048A23F`, `0x00424106`, `0x0051DA79`, `0x0069B92A` | DONE for the algorithm (was: partial, "algorithm is wrong"): `Simulation` owns one parity `SimRng`; remaining open item is auditing that gameplay call sites consume it in binary order | `src/sim/world/mod.rs`, bridge/ore/movement/combat/particle call sites that consume sim RNG | Keep one auditable sim-owned parity stream for gameplay-visible RandomRanged calls, with call-order tests around bridge damage/debris and ore growth. | Bridge damage `RandomRanged(1, BridgeStrength) < damage` and debris/frame variants advance the same stream in binary order; proposed test name: `bridge_random_ranged_uses_shared_scenario_stream`. | Do not create per-system RNG streams for gameplay parity unless a caller is proven to use a separate Random object. |
| HARV state-2 existing-destination and valid far-destination exits consume `Scenario+0x218.RandomRanged(0,2)` and return current mission base delay plus jitter. | `0x0073EB5A..0x0073EB62`, `0x0073EDB5..0x0073EDBB`, `0x0073EF77..0x0073EFA2`; `0x005B3A00`, `0x0065C7E0` decompiles | missing/unchecked at the callsite: the shared RNG algorithm exists, but current Rust miner dispatch does not visibly reproduce this draw plus scheduler delay | mission scheduler, `src/sim/miner/miner_system.rs`, `Simulation` RNG call order | Trace and implement the mission-dispatch delay as one mechanism: draw in native order and schedule the next HARV mission invocation from `base+jitter`. | Binary-derived state-2 dispatch oracle for both a pre-owned NavCom and a valid far target observes the expected RNG advance and next-dispatch delay. | Do not assert unchanged RNG for these full-dispatch paths; do not add a free-standing draw without closing the returned-delay consumer. |

### Negative Facts / Do Not Do

- Do not treat the second argument as exclusive. Evidence: equality no-draw return and callers passing `count - 1` (`0x0065C7EB..0x0065C7EF`, `0x00424102..0x00424110`). Active in YR: Yes.
- Do not implement `RandomRanged` as `% span`. Evidence: helper masks and retries while sample is greater than span (`0x0065C87E..0x0065C882`). Active in YR: Yes.
- Do not reintroduce a `high < low -> low` shortcut as the gamemd contract. Evidence: binary swaps reversed bounds (`0x0065C7EB..0x0065C7F9`). (corrected 2026-05-29 via Read of `src/sim/rng.rs`: the old shortcut is GONE — `next_range_u32_inclusive` at `src/sim/rng.rs:131..136` now sorts the bounds before drawing, matching the binary. This is now a regression guard, not an outstanding delta.) Active in YR: Conditional; helper behavior is live, but reversed live callers were not exhaustively searched.
- Do not model `0x0065C7E0` with the nearby LCG at `0x0065C640`/`0x0065C660` (raw code regions, not carved functions). Evidence: target inlines the 250-word XOR-lag draw (`0x0065C837..0x0065C87C`); LCG substance confirmed via `read_memory 0x0065C640` (corrected 2026-05-29). Active in YR: Yes.
- Do not assume `RandomRanged(0, 0x7FFFFFFF)` is a safe normalized-probability roll. Evidence: helper's signed `span + 1` branch skips normal sampling at exactly `0x7FFFFFFF`; existing callers use `0x7FFFFFFE`. Active in YR: Conditional; edge behavior exists, scoped live callers avoid it.
- Do not classify HARV state-2 destination ownership or valid far-destination handoff as RNG-neutral. Both reach the verified `Scenario+0x218.RandomRanged(0,2)` mission-delay tail.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/ADDRESS_MAP.md`: replace `| 0x0065C7E0 | DamageFireAnims (related) | - | DAMAGE_FIRE_ANIMS |` with `| 0x0065C7E0 | Random::RandomRanged | Inclusive sorted-bound deterministic RNG helper using ScenarioClass random state in representative gameplay callers | RANDOM_RANDOMRANGED_0065C7E0 |`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md`: after the sentence "`RandomRanged(1, strength)` uses inclusive bounds", add `Helper contract verified in RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md: equal bounds return without a draw, reversed bounds are sorted, ordinary spans use mask-and-reject sampling from Scenario+0x218, not modulo.`

## Sources

- Binary static disassembly of `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`:
  - `0x0065C7E0..0x0065C88A` - target helper.
  - `0x0065C780..0x0065C7D0` - raw 250-word RNG draw helper.
  - `0x0065C6D0..0x0065C777` - RNG initializer.
  - `0x0065C640..0x0065C6C7` - separate 15-bit LCG raw code region for contrast (not a Ghidra-carved function; verified via `read_memory 0x0065C640` and `get_function_by_address 0x0065C640`/`0x0065C660` returning "No function found", 2026-05-29).
  - Representative caller windows: `0x0048A231..0x0048A24E`, `0x00424102..0x00424130`, `0x0054C5BE..0x0054C5CE`, `0x0051DA52..0x0051DA84`, `0x004D947C..0x004D9497`, `0x0069B921..0x0069BA23`.
  - HARV state-2 caller rechecked 2026-07-25 with live `gamemd.exe`: owner jump `0x0073EB5A..0x0073EB62`, valid far-destination jump `0x0073EDB5..0x0073EDBB`, shared mission-delay tail `0x0073EF77..0x0073EFA2`; helper identities confirmed by `batch_decompile(0x005B3A00,0x0065C7E0)`.
  - Initialization windows: `0x006832C8..0x006832D4`, `0x00683564..0x0068356C`, `0x0052FDEE..0x0052FE51`.
- Import table: `0x007E1138` maps to `KERNEL32.GetTickCount`.
- Rust scan:
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/rng.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_hash.rs`
- Prior docs referenced for caller context only:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RUNTIME_DEEP_DIVE_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/REPAIRBRIDGEWALKER_BODIES_GHIDRA_REPORT.md`
