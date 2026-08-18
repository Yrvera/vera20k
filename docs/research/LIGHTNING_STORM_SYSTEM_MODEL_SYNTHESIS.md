# Lightning Storm System Model Synthesis

**Date:** 2026-05-22  
**Skill:** `/synthesize-system-model lightning`  
**Output type:** model-synthesis  
**System name:** Lightning Storm superweapon

## Scope

Included surfaces:

- Weather Control Device / `LightningStormSpecial` launch path.
- `LightningStorm__Start`, `LightningStorm__Process`, `CreateCloudBolt`, `GroundStrike`.
- Cloud bolt to ground strike timing, scatter RNG, anim/sound RNG, damage ordering.
- Lightning Storm bridge impact-Z and `Apply_area_damage` layer selection.
- Current Rust implementation status only where it affects implementation safety.

Non-scope:

- ElectricBolt/Tesla/EBOLT weapon visuals.
- AI target selection for launching the superweapon.
- Full sidebar charge UI, except Type/launch binding context.
- Full scorch/smudge visual parity beyond RNG/order facts already captured.
- Nuke, Psychic Dominator, and Genetic Converter except where shared bridge AoE docs compare them.

## Evidence Ladder

- `BINARY_HIGH`: live Ghidra spot-checks in this synthesis for `0053A6C0`, `0053A300`, `0053A140`, `00539EB0`, plus prior exact-address reports.
- `RESEARCH_HIGH`: `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`, `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md`.
- `VERIFY_FINDING`: `AUDIT_LOG.md` 2026-05-22 bridge patch cluster confirms the bridge AoE doc was patched for stale Rust status.
- `DOC_SYNTHESIS`: `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md`, but its current-Rust section is stale.
- `INFERENCE`: naming/UX labels not directly rechecked in binary.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| `LightningStormSpecial` is Type index 2 and launched by GAWEAT. | `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md:113,383`; `rulesmd.ini:12291,12313,30898` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Launch case 2 delegates to `LightningStorm__Start`; storm logic lives in LS state machine. | `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md:20-24,384-386`; Ghidra `00539EB0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Only one storm is active; later starts queue via countdown/start globals. | `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md:34,252-279`; Ghidra `00539EB0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Active storm uses global-frame modulo timers, not per-storm decrementing timers. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:31`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Center cloud creation is checked before scatter on frames where both fire. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:31`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Lightning is two-phase: cloud anim first, ground strike/damage when strike-tracked anim passes half frames. | `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md:98-143,164-189`; Ghidra `0053A6C0`, `0053A140`, `0053A300` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Scatter uses exactly 3 attempts and no fallback spawn. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:32,39`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Scatter offset range is inclusive `RandomRanged(-(LightningCellSpread >> 1), +(LightningCellSpread >> 1))`, X then Y. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:33-34`; `rulesmd.ini:136`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Scatter rejects out-of-bounds cells after consuming both draws. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:36`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Scatter separation checks every active cloud-bolt anim; reject only when manhattan `< LightningSeparation`. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:38`; Ghidra `0053A6C0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Cloud, ground-bolt, and thunder sound picks use raw `Random__Next() % count`. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:41,43-44`; Ghidra `0053A140`, `0053A300` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Damage happens after ground visual setup, duplicate guard, optional thunder RNG, and explosion anim setup. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:45`; Ghidra `0053A300` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Lightning bridge impact-Z is `cell.Level * level_height + bridge_height` only when `Flags & 0x100`; this selects deck occupants through `Apply_area_damage`. | `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md:45-60,130,166,180`; Ghidra `0053A300` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Current Rust already threads bridge-adjusted impact Z through `AoELayerContext`. | `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md:138-146`; `src/sim/superweapon/lightning_storm.rs:241-255`; `src/sim/combat/combat_aoe.rs:89-91,206-224` | confirmed-current | high | n/a | IMPLEMENTATION_SAFE |
| Current Rust Lightning RNG/lifecycle is RED against gamemd. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:49-60`; `src/sim/superweapon/lightning_storm.rs:17-21,111-208,211-308` | confirmed-current | high | n/a | IMPLEMENTATION_SAFE as mismatch diagnosis |
| `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md` current-Rust section says no superweapon module exists. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:92`; `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md:710` | stale | high | n/a | DOC_PATCH_READY |
| Zero-count weather anim/sound mod behavior is fully known. | `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:84-88` | unknown | low | conditional | NEEDS_REINVESTIGATE |

## Current Model

Lightning Storm is a standard YR superweapon granted by GAWEAT through `LightningStormSpecial`. `SuperClass::Launch` case 2 is thin: it calls `LightningStorm__Start`, plays/clears launch-side state, and lets the global Lightning Storm state machine run the effect.

`Start` records target and owner, handles the active-vs-queued distinction, creates the radar event, sets `LS_Active`, records the start frame/duration, warns enemy houses, updates local-player flags, and calls shared superweapon lighting. If a storm is already active, a later launch is queued rather than co-running.

`Process` is global-frame based. During the active phase it checks duration by `LS_StartFrame + LS_Duration`, creates the center cloud bolt when `CurrentFrame % LightningHitDelay == 0`, then handles scatter when `CurrentFrame % LightningScatterDelay == 0`. On dual-fire frames, center creation occurs before scatter.

Bolts are two-phase. `CreateCloudBolt` computes bridge-aware visual Z, rejects exact duplicate X/Y/Z before consuming cloud RNG, picks from `WeatherConClouds` with raw `Random__Next() % count`, creates an anim, and tracks it in both the cloud vector and strike vector. `Process` later turns a strike-tracked anim into damage when `current_frame > total_frames / 2`. `GroundStrike` then creates the visible bolt from `WeatherConBolts`, applies duplicate guard and optional thunder RNG, creates the warhead visual, applies area damage, and optionally emits scorch/debris anims.

Scatter is strict: exactly three attempts, no post-loop fallback, signed cell offsets from `LightningCellSpread >> 1`, X draw before Y draw, out-of-bounds rejected after both draws, and too-close rejection against all active cloud bolt anims using manhattan distance `< LightningSeparation`.

Lightning damage uses `LightningDamage` and `LightningWarhead`. On structural bridge cells, `GroundStrike` constructs impact Z with the Lightning bridge addend and routes through normal `Apply_area_damage`, which selects only the bridge/deck object list. Lightning does not add bridge height for `0x400` at this site.

## Implementation-Safe Facts

- Use global frame modulo for center/scatter scheduling.
- Do not collapse cloud creation and ground damage into one tick.
- Preserve center-before-scatter order on shared timer frames.
- Use exactly three scatter attempts and allow no scatter spawn on failure.
- Use `LightningCellSpread >> 1`, not the full INI value, for scatter offset radius.
- Reject out-of-bounds scatter cells; do not clamp them.
- Compare scatter separation against every active cloud bolt, not just the last bolt.
- Use raw `Random__Next() % count` for `WeatherConClouds`, `WeatherConBolts`, and `LightningSounds`.
- Consume thunder sound RNG before damage whenever `LightningSoundsCount > 0`, including stock count 1.
- Apply damage only in `GroundStrike`, after the verified visual/sound setup order.
- Preserve the current Rust bridge-aware `AoELayerContext` threading for Lightning damage.

## Doc-Patch-Ready Facts

- `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md` section 14 is stale. A partial `src/sim/superweapon/lightning_storm.rs` exists, but the RNG/lifecycle parity is RED.
- `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md` can be tightened with the raw RNG/modulo facts from the RNG classification report, especially that `WeatherConBolts` must not be modeled as hard-coded `BOLT_ANIMS` plus `next_range_u32`.

## Stale Or Superseded Claims

- Old consolidated current-Rust status: "No `src/sim/superweapon/` module exists." Superseded by current repo files and the 2026-05-22 Rust scan.
- Earlier bridge AoE status saying Lightning did not thread `AoELayerContext` is stale. Current Rust computes bridge-adjusted impact Z and passes occupancy, terrain, and impact Z through AoE context.

## Cross-Doc Conflicts

- No unresolved binary conflict found for core Lightning Storm lifecycle/RNG/bridge AoE.
- The only active conflict is doc freshness: consolidated current-Rust status lags current repo and newer RNG/bridge reports.

## Needs Re-Investigation

- `/re-investigate Lightning Storm zero-count WeatherConClouds WeatherConBolts LightningSounds Scorches behavior` if modded empty-list behavior matters. Existing evidence covers stock non-empty lists and notes divide-by-count paths.
- `/re-investigate Lightning Storm scorch and smudge visual parity after terrain destruction` if the next task needs exact post-damage debris visuals, not just RNG order.
- `/synthesize-system-model EBolt Tesla electric bolt visuals` if "lightning" is intended to mean weapon electric bolts rather than the Lightning Storm superweapon.

## Do-Not-Implement Notes

- Do not use per-storm decrementing bolt timers for parity.
- Do not retain `MAX_SCATTER_RETRIES=10` or fallback scatter spawns.
- Do not interpret `LightningCellSpread=10` as a plus/minus 10-cell radius.
- Do not hard-code `WCLBOLT1..3` as the behavioral source when rules already expose `WeatherConBolts`.
- Do not ignore the stock thunder RNG draw because there is only one `LightningSounds` entry.
- Do not damage both bridge and ground occupants for one Lightning impact.
- Do not generalize Lightning's `0x100` bridge-height addend to `0x400` cells.

## Source Ledger

- Ghidra spot-checked during synthesis:
  - `LightningStorm__Process @ 0x0053A6C0`
  - `LightningStorm__GroundStrike @ 0x0053A300`
  - `LightningStorm__CreateCloudBolt @ 0x0053A140`
  - `LightningStorm__Start @ 0x00539EB0`
- `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md:4-8,20-24,67-79,98-143,164-233,252-289,298-310,384-426`
- `LIGHTNING_STORM_RNG_CLASSIFICATION_GHIDRA_REPORT.md:9-17,31-45,49-72,76-88,92-96`
- `SUPERWEAPON_BRIDGE_AOE_IMPACT_Z_THREADING_GHIDRA_REPORT.md:12,18-25,36-60,130,138-146,153-166,176-189,197`
- `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md:113,383,495,710,815-830`
- `AUDIT_LOG.md:179,186`
- `ini/rulesmd.ini:130-137,532-534,710-711,12291,12313,30898`
- `src/sim/superweapon/lightning_storm.rs:17-21,76-88,111-174,178-208,211-308`
- `src/sim/combat/combat_aoe.rs:35-58,89-91,105,206-224`
- `src/rules/ruleset.rs:427-446,1051-1061`
