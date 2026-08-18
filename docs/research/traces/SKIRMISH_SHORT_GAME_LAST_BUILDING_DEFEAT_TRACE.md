# Skirmish Short Game Last Building Defeat Trace

Scenario: standard offline Yuri's Revenge Skirmish Battle, Short Game enabled. After play begins, the human player loses the last building while still owning one ordinary non-building unit. Concrete values used for this trace: `OwnedBuildings=0`, surviving ordinary non-building units `=1`, counted ConYard-style instances `=0`, current frame `>0`.

## Pipeline

`Skirmish shell checkbox -> packed Short Game option -> per-house update defeat gate -> owned-count condition -> defeat propagation -> player-visible loss state`

## Active YR Evidence

- Standard YR default enables this option: `ini/rulesmd.ini:3039` has `ShortGame=yes`.
- Offline Skirmish Start packs checkbox `0x54E` into `DAT_00A8B262=1` when checked. Evidence: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- `HouseClass__Update @ 0x004F86F0` is active for standard non-campaign Skirmish because the defeat block is gated by `g_GameMode != 0`; Skirmish mode is documented as mode `5` in `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`.
- Live read-only Ghidra decompile of `0x004F86F0` confirms the Short Game branch reads `DAT_00A8B262`, requires `g_CurrentFrameCounter > 0`, and calls `HouseClass__ScatterAllUnits()` then `HouseClass__MPlayer_Defeated()` when the short condition is true.

## Stage Trace

### Stage 1 - Option Packing

Input: Short Game checkbox enabled.

gamemd: Start branch writes `DAT_00A8B262 = 1`.

Rust: `SkirmishLaunchOptions.short_game = true` flows into `GameOptions.short_game` in `src/skirmish_launch.rs:180` and is assigned to the simulation in `src/app_skirmish.rs:177`.

Verdict: PASS. Boolean value is equal: gamemd `1`, Rust `true/1`.

### Stage 2 - Defeat Check Timing Gate

Input: match has begun, current frame/tick is greater than zero, human house is not already defeated, house is not passive.

gamemd: `HouseClass__Update @ 0x004F86F0` gates the defeat branch with `g_GameMode != 0`, `IsDefeated == 0`, `g_CurrentFrameCounter > 0`, and `HouseType.MultiplayPassive == 0`.

Rust: `Simulation::tick` calls `check_defeat()` only when `self.tick > 0` in `src/sim/world/mod.rs:1583`, after combat and AI command application.

Verdict: UNCHECKED. Both sides require frame/tick greater than zero, but this trace did not compute exact same-frame ordering after the building death in both engines.

### Stage 3 - Short Game Defeat Condition

Input values after the last building dies: buildings `0`, ordinary non-building unit `1`, counted ConYard-style instances `0`.

gamemd formula when `DAT_00A8B262 != 0`: if `OwnedBuildings < 1` and the sum of three counted RulesClass `+0xB24` instance counts is `< 1`, call defeat. With the trace values: `0 < 1` and `0 + 0 + 0 < 1`, so defeat triggers. The ordinary non-building unit does not keep the player alive in this Short Game branch.

Rust formula: `total = house.owned_building_count + house.owned_unit_count` in `src/sim/world/mod.rs:636`; defeat only if `total == 0` in `src/sim/world/mod.rs:637`. With the trace values: `0 + 1 = 1`, so Rust does not defeat the player.

Verdict: FAIL. gamemd output is defeated `1`; Rust output is defeated `0`.

### Stage 4 - Defeat Propagation

Input: Stage 3 condition true in gamemd, false in Rust.

gamemd: `HouseClass__Update @ 0x004F86F0` calls `HouseClass__ScatterAllUnits()` and then `HouseClass__MPlayer_Defeated()` on the same update path.

Rust: because `total == 1`, `src/sim/world/mod.rs:637` does not enter the defeat branch, so `HouseState.is_defeated` remains false. There is also no Short Game-specific branch in `check_defeat`.

Verdict: NOT-IMPLEMENTED. The Short Game last-building transition is missing.

### Stage 5 - Player-Visible Result

Input: local human owns no buildings and one ordinary unit after play begins.

gamemd: local player is defeated under Short Game. `MPlayer_Defeated @ 0x004FC0B0` performs the local defeat path documented in `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`: defeated flag, input/sidebar/radar changes, map-clear behavior, defeat message/EVA, then win/loss decision.

Rust: the player remains alive because `is_defeated` is not set. No loss flag or defeat presentation is reachable from this scenario; `has_lost` has no writer in the current sim outside initialization.

Verdict: FAIL. The player can continue playing in Rust with only units, while YR defeats them.

### Stage 6 - Borrowed-Time / Final Game-End Timing

Input: local player has just been defeated.

gamemd: `Flag_To_Lose`/win-loss handling is documented in `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`; Skirmish mode `5` skips the multiplayer MaxAhead rounding branch used by LAN/WOL modes.

Rust: not reached in this scenario because the Short Game defeat condition is not implemented. Current Rust also does not set `has_lost` in `check_defeat`.

Verdict: UNCHECKED. The scenario fails earlier, so final loss-screen timing was not numerically compared.

## Failures And Missing Pieces

1. Stage 3: Short Game condition ignores ordinary remaining units in YR, but Rust requires total buildings plus units to be zero. Rust evidence: `src/sim/world/mod.rs:636`; gamemd evidence: `HouseClass__Update @ 0x004F86F0`.
2. Stage 4: Rust has no Short Game-specific transition to defeat after last building loss. Rust evidence: `src/sim/world/mod.rs:628`; gamemd evidence: `HouseClass__Update @ 0x004F86F0`.
3. Stage 5: the local player remains playable in Rust instead of receiving YR's immediate Short Game defeat handling. Rust evidence: no `has_lost` writer found outside initialization; gamemd evidence: `MPlayer_Defeated @ 0x004FC0B0`.

## Verdict Tally

PASS: 1 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

Status: COMPLETE
