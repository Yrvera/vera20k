# Lightning Storm RNG Classification - Ghidra Report

Target question: classify current Rust Lightning Storm RNG in `src/sim/superweapon/lightning_storm.rs` against gamemd/YR behavior, focusing scatter dx/dy, retry/fallback order, bolt animation pick, and damage/visual ordering.

Non-goals: full Lightning Storm rendering parity, AI launch targeting, charge/sidebar behavior, ambient lighting, warhead damage math beyond call ordering, and ElectricBolt/Tesla weapon visuals.

Evidence needed to mark COMPLETE: decompile plus assembly for the RNG-sensitive Lightning Storm entry points; caller evidence that the path is YR-live; Rust scan of the corresponding call sites; final GREEN/YELLOW/RED classification. Stop condition: once `LightningStorm__Process`, `CreateCloudBolt`, `GroundStrike`, and launch/start RNG fallback are classified for RNG bounds/order, stop without expanding into unrelated superweapons.

Status: COMPLETE for the bounded RNG classification. Overall verdict: RED.

## 1. Summary

Current Rust does not match gamemd Lightning Storm RNG order. The largest mismatch is structural: gamemd creates a cloud bolt first, then triggers a ground strike later when the cloud anim passes half its frames. Rust spawns the visible bolt, applies damage, emits warhead effects, and queues sound immediately.

Scatter placement is also RED. Gamemd uses exactly three attempts, `LightningCellSpread >> 1`, inclusive `RandomRanged(-spread, +spread)` for X then Y, checks in-bounds, and rejects candidates too close to any active cloud-bolt anim. Rust uses ten attempts plus an extra fallback draw/spawn, uses the full INI spread, clamps negative coordinates, and checks only the last stored bolt.

Bolt/cloud/sound animation picks are RED for call-order parity. Gamemd uses raw `Random__Next() % count` for `WeatherConClouds`, `WeatherConBolts`, and `LightningSounds`; Rust uses `next_range_u32(BOLT_ANIMS.len())`, hard-codes three bolt anim names, and does not consume the thunder-sound draw.

## 2. Evidence And Active-YR Check

| Item | Evidence | Active in YR |
|---|---|---|
| Launch path delegates Type=LightningStorm to `LightningStorm__Start` | Existing launch docs; `rulesmd.ini [LightningStormSpecial] Type=LightningStorm`, GAWEAT grants it | Yes |
| Per-tick path is `LightningStorm__Process @ 0x0053A6C0` | Ghidra decompile; assembly call to `GroundStrike` at `0x0053A81B` and `CreateCloudBolt` at `0x0053A95E`, `0x0053AA92` | Yes |
| Random helper is scenario-owned | Assembly uses `g_ScenarioClass + 0x218` before `Random__Next`/`Random__RandomRanged` calls | Yes |
| Rust surface scanned | `src/sim/superweapon/lightning_storm.rs:142-213`, Codegraph entry points `pick_scatter_cell`, `spawn_bolt` | Yes |

## 3. Load-Bearing Verified Facts

1. Center cloud creation runs before scatter on frames where both timers divide the global frame. Evidence: `Process` decompile checks `CurrentFrame % Rules+0x17A0` and calls `CreateCloudBolt` before the `Rules+0x17A4` scatter branch; assembly `0x0053A94A..0x0053A95E`, then `0x0053A96F..`. Active in YR: Yes.
2. Scatter frequency is based on global frame modulo `LightningScatterDelay`, not a per-storm decrementing timer. Evidence: `idiv [Rules+0x17A4]`, test EDX, assembly `0x0053A96F..0x0053A97A`. Active in YR: Yes.
3. Scatter attempts are exactly 3. Evidence: decompile `iStack_10 = 3`, decrement after failed candidate; no fourth fallback spawn. Active in YR: Yes.
4. Scatter spread is `Rules.LightningCellSpread >> 1`. Stock YR `10` becomes random offsets `-5..+5`, not `-10..+10`. Evidence: decompile `iVar9 = *(Rules+0x17A8) >> 1`; INI `rulesmd.ini:136`. Active in YR: Yes.
5. Scatter X draw is `RandomRanged(-spread, +spread)` and happens before Y. Evidence: assembly `0x0053A992..0x0053A9AB`, pushes `+spread` then `-spread`, calls `0x0065C7E0`; second call at `0x0053A9BB..0x0053A9C3`. Active in YR: Yes.
6. Candidate coordinates are added as signed 16-bit cell deltas to the target cell. Evidence: decompile adds returned `short` to packed cell low/high words. Active in YR: Yes.
7. Candidate must pass `Cell_in_bounds_check`; out-of-bounds candidates are discarded after consuming both RNG draws. Evidence: decompile calls `Cell_in_bounds_check(&uStack_14)` before `CreateCloudBolt`. Active in YR: Yes.
8. Separation checks against every active cloud-bolt anim in `DAT_00A9F9D4` count `DAT_00A9F9E0`, not just the immediately previous bolt. Evidence: decompile loops `iVar6 < DAT_00a9f9e0`, gets each anim coords, computes manhattan. Active in YR: Yes.
9. Separation rejects only when manhattan `< Rules.LightningSeparation`; equality is allowed. Evidence: decompile `if (iVar10 + iVar5 < *(Rules+0x17AC)) bVar1 = true`. Active in YR: Yes.
10. On three failed scatter attempts, gamemd returns without creating a scatter bolt and without extra RNG draws. Evidence: decompile decrements `iStack_10`; branch returns when `< 1`; no post-loop fallback call. Active in YR: Yes.
11. `CreateCloudBolt` has an anti-duplicate guard before consuming its cloud animation RNG. Evidence: decompile compares candidate lepton X/Y/Z to `DAT_00A9FA30/34/38` and returns before `Random__Next`; assembly `Random__Next` call at `0x0053A1F5`. Active in YR: Yes.
12. Cloud anim pick uses raw `Random__Next() % WeatherConCloudsCount`, not `RandomRanged`. Evidence: assembly `0x0053A1EF..0x0053A207` calls `0x0065C780`, then `div [Rules+0x2CC]`, vector `[Rules+0x2C0]`. Active in YR: Yes.
13. Ground strike is delayed until a tracked strike anim's current frame is greater than half its total frames. Evidence: `Process` decompile compares `total_frames / 2 < current_frame` then calls `GroundStrike`; assembly call at `0x0053A81B`. Active in YR: Yes.
14. Ground-strike bolt anim pick uses raw `Random__Next() % WeatherConBoltsCount`, not `RandomRanged`. Evidence: assembly `0x0053A33F..0x0053A35D` calls `0x0065C780`, divides by `[Rules+0x2E8]`, loads from `[Rules+0x2DC]`. Active in YR: Yes.
15. Thunder sound selection consumes raw RNG whenever `LightningSoundsCount > 0`, even stock count 1. Evidence: assembly `0x0053A45F..0x0053A493` tests `[Rules+0x744]`, calls `Random__Next`, divides by count. Active in YR: Yes.
16. Damage happens after bolt anim creation, duplicate guard, optional thunder-sound draw, and explosion anim creation. Evidence: assembly `0x0053A345` bolt RNG, `0x0053A47A` sound RNG, `0x0053A5AC` explosion helper, `0x0053A5D0` `Apply_area_damage`. Active in YR: Yes.
17. Terrain scorch/debris after damage uses `RandomRanged(2,4)` for count, then `RandomRanged(0, ScorchesCount-1)` for each scorch anim if terrain destruction criteria pass and infantry was not the pre-hit object. Evidence: assembly `0x0053A622..0x0053A62C`, `0x0053A650..0x0053A665`. Active in YR: Yes.
18. Invalid/default start target fallback uses `RandomRanged(0, MapHeightLimit)` then `RandomRanged(0, MapWidthLimit)` in a loop until in-bounds. The first draw uses `DAT_0087f918` (Y/height range, high word of packed cell) and the second uses `DAT_0087f914` (X/width range, low word of packed cell). Evidence: decompile `LightningStorm__Start` — `uVar3 = Random__RandomRanged(0, DAT_0087f918); uVar4 = Random__RandomRanged(0, DAT_0087f914); local_4 = CONCAT22(uVar3, uVar4)`; cross-confirmed via `MapClass__InitCellAttributes` xrefs showing `DAT_0087f918` iterates Y-axis rows and `DAT_0087f914` iterates X-axis columns. Active in YR: Yes for default-cell starts, not normal player target clicks. (corrected 2026-05-29: was "MapWidthLimit then MapHeightLimit"; binary shows Height/Y first, Width/X second; ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; via decompile_function 0x00539EB0 + MapClass__InitCellAttributes xrefs)

## 4. Current Rust Classification

| Rust surface | Classification | Reason |
|---|---|---|
| `process` timers | RED | Per-storm decrementing timers, not global-frame modulo; immediate center `spawn_bolt` instead of `CreateCloudBolt` |
| `pick_scatter_cell` spread | RED | Uses full INI value; gamemd halves it before randomizing |
| `pick_scatter_cell` retry/fallback | RED | Rust uses 10 attempts and then consumes two extra fallback draws and always returns a cell; gamemd uses 3 attempts and may spawn no scatter bolt |
| `pick_scatter_cell` bounds | RED | Rust clamps negative coords to zero; gamemd rejects out-of-bounds candidates |
| `pick_scatter_cell` separation | RED | Rust checks only last bolt; gamemd checks all active cloud bolts |
| `spawn_bolt` anim pick | RED | Rust uses `next_range_u32(3)` and hard-coded `WCLBOLT1..3`; gamemd uses raw `Random__Next() % WeatherConBoltsCount` from Rules |
| `spawn_bolt` damage/visual order | RED | Rust applies all effects immediately; gamemd delays ground strike until cloud anim half-frame |
| `spawn_bolt` sound RNG | RED | Rust has no `LightningSounds` random draw; gamemd consumes raw RNG when sound list count > 0 |

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Scatter uses 3 attempts, no fallback spawn | `0x0053A97A..0x0053AA92` decompile/assembly | mismatch | `pick_scatter_cell`, `process` | Failed candidates consume X/Y draws; after third failure, no scatter bolt | `lightning_scatter_three_failed_attempts_spawns_no_bolt_and_no_fallback_draws` | Do not keep `MAX_SCATTER_RETRIES=10` or post-loop fallback |
| Scatter random range is inclusive `RandomRanged(-(CellSpread>>1), +(CellSpread>>1))` | `0x0053A992..0x0053A9C3`; `rulesmd.ini:136` | mismatch | `pick_scatter_cell` | Stock offsets are `-5..+5` from `LightningCellSpread=10` | `lightning_scatter_halves_cell_spread_before_rng` | Do not interpret INI comment as full radius |
| Scatter rejects out-of-bounds and too-close-to-any-cloud candidates | `Process` decompile cloud-vector loop and `Cell_in_bounds_check` | mismatch | Lightning storm state model | Track active cloud bolts or equivalent separation candidates | `lightning_scatter_rejects_candidate_near_existing_cloud_bolt` | Do not clamp negative cells to zero |
| Cloud anim phase consumes raw `Random__Next() % WeatherConCloudsCount` before delayed strike | `0x0053A1F5..0x0053A213` | missing | storm visual/effect lifecycle | Add cloud bolt phase and strike trigger timing | `lightning_center_cloud_pick_consumes_raw_next_before_ground_strike` | Do not use `RandomRanged` for anim-vector picks |
| Ground bolt anim consumes raw `Random__Next() % WeatherConBoltsCount` | `0x0053A345..0x0053A35D` | mismatch | `spawn_bolt` or future `ground_strike` | Use Rules `WeatherConBolts` list and raw modulo call | `lightning_ground_bolt_anim_uses_raw_next_mod_weather_con_bolts_count` | Do not hard-code three anim names |
| Thunder sound consumes raw `Random__Next() % LightningSoundsCount` before damage when count > 0 | `0x0053A45F..0x0053A493` | missing | `spawn_bolt` / sound event path | Preserve RNG draw even if list count is 1 | `lightning_ground_strike_consumes_sound_rng_before_damage` | Do not push generic sound after damage without RNG |
| Damage happens after ground visual setup, not at scatter/center selection tick | `0x0053A81B`, `0x0053A5D0` | mismatch | `process`, `spawn_bolt` | Delay damage until strike anim reaches half frames | `lightning_cloud_bolt_does_not_damage_until_half_anim_frame` | Do not collapse cloud and ground phases for parity path |

## 6. Negative Facts / Do Not Do

- Do not convert Lightning Storm anim list picks to `RandomRanged`; gamemd uses raw `Random__Next() % count`.
- Do not keep `BOLT_ANIMS` as the behavioral source; gamemd reads `WeatherConBolts` from Rules.
- Do not add a fourth fallback scatter candidate; gamemd may simply skip the scatter bolt for that tick.
- Do not clamp scatter coordinates into the map; out-of-bounds candidates fail and cost their RNG draws.
- Do not compare scatter separation only to the last strike; gamemd checks all active cloud bolts.
- Do not apply Lightning damage in the same tick that the center/scatter candidate is selected.
- Do not ignore the thunder sound RNG draw just because stock `LightningSounds` has one entry.

## 7. Remaining Uncertainty

- Exact Rust data model for cloud-bolt tracking is an implementation design question, not a binary uncertainty.
- Zero-count modded lists (`WeatherConClouds`, `WeatherConBolts`, `LightningSounds`, `Scorches`) were not runtime-debugged. The binary divides by the count on the live paths shown here; stock YR has non-empty weather lists.
- This report did not fully classify scorch/smudge visual parity after Lightning damage; it only records the RNG calls that occur after the damage result enables that branch.

## 8. Stale Docs / Follow-Up Wording

- `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md` section 14 is stale: replace "No `src/sim/superweapon/` module exists" with "A partial `src/sim/superweapon/lightning_storm.rs` implementation exists, but Lightning Storm RNG/lifecycle is RED against gamemd; see `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`."
- `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md` can add: "Current Rust must not use `next_range_u32(BOLT_ANIMS.len())` for WeatherConBolts; gamemd uses raw `Random__Next() % WeatherConBoltsCount` in `GroundStrike`."
- No shared claims file was updated in this slot.

## Sources

- Ghidra decompile: `LightningStorm__Process @ 0x0053A6C0`, `LightningStorm__CreateCloudBolt @ 0x0053A140`, `LightningStorm__GroundStrike @ 0x0053A300`, `LightningStorm__Start @ 0x00539EB0`.
- Assembly from `gamemd.exe`: `0x0053A1F5`, `0x0053A345`, `0x0053A47A`, `0x0053A5D0`, `0x0053A62C`, `0x0053A665`, `0x0053A95E`, `0x0053A9AB`, `0x0053A9C3`, `0x0053AA92`, `0x00539EF9`, `0x00539F14`.
- Current Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/superweapon/lightning_storm.rs`.
- INI: `ini/rulesmd.ini:130-138`, `ini/rulesmd.ini:533`, `ini/rulesmd.ini:710`, `ini/rulesmd.ini:30898-30908`.
- Existing docs: `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`, `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md`, `RANDOM_RANDOMRANGED_0065C7E0_GHIDRA_REPORT.md`.
